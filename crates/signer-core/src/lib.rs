pub mod identity;

use std::time::{Duration, Instant};

use nostr::event::{Event, Kind};
use nostr::key::PublicKey;
use nostr_portable_crypto as crypto;
use nostr_portable_permissions as permissions;
use nostr_portable_permissions::{
    ClientIdentity, PermissionResult, PermissionStore,
};
use nostr_portable_protocol as protocol;
use nostr_portable_protocol::{ApprovalRequest, NostrSigner, SignEventRequest, SignerStatus};
use nostr_portable_vault as vault;
use nostr_portable_vault::VaultProvider;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("vault error: {0}")]
    Vault(#[from] vault::Error),
    #[error("crypto error: {0}")]
    Crypto(#[from] crypto::Error),
    #[error("permissions error: {0}")]
    Permissions(#[from] permissions::Error),
    #[error("protocol error: {0}")]
    Protocol(#[from] protocol::Error),
    #[error("signer is locked")]
    Locked,
    #[error("signer is already unlocked")]
    AlreadyUnlocked,
    #[error("no vault provider configured")]
    NoProvider,
    #[error("user rejected")]
    UserRejected,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("approval required: {0}")]
    ApprovalRequired(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Locked,
    Unlocked,
}

#[derive(Debug)]
pub enum PermissionCheck {
    Allowed,
    Denied(String),
    Ask(ApprovalRequest),
}

pub struct SignerService {
    provider: Option<Box<dyn VaultProvider>>,
    permissions: PermissionStore,
    state: State,
    unlocked_at: Option<Instant>,
    keys: Option<crypto::Keys>,
    auto_lock_interval: Duration,
}

impl SignerService {
    pub fn new(
        provider: Option<Box<dyn VaultProvider>>,
        permissions: PermissionStore,
        auto_lock_interval: Duration,
    ) -> Self {
        Self {
            provider,
            permissions,
            state: State::Locked,
            unlocked_at: None,
            keys: None,
            auto_lock_interval,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn unlock(&mut self, passphrase: &str) -> Result<(), Error> {
        if self.state == State::Unlocked {
            return Err(Error::AlreadyUnlocked);
        }
        let provider = self.provider.as_ref().ok_or(Error::NoProvider)?;
        let keys = vault::unlock_vault(provider.as_ref(), passphrase)?;
        self.keys = Some(keys);
        self.state = State::Unlocked;
        self.unlocked_at = Some(Instant::now());
        Ok(())
    }

    pub fn lock(&mut self) -> Result<(), Error> {
        self.keys = None;
        self.state = State::Locked;
        self.unlocked_at = None;
        Ok(())
    }

    pub fn check_auto_lock(&mut self) {
        if self.state != State::Unlocked {
            return;
        }
        if let Some(unlocked_at) = self.unlocked_at {
            if unlocked_at.elapsed() >= self.auto_lock_interval {
                let _ = self.lock();
            }
        }
    }

    pub fn check_vault_present(&self) -> bool {
        self.provider.as_ref().map_or(false, |p| p.is_available())
    }

    pub fn status(&self) -> SignerStatus {
        SignerStatus {
            state: match self.state {
                State::Locked => "locked".into(),
                State::Unlocked => "unlocked".into(),
            },
            vault_present: self.check_vault_present(),
            public_key: self.keys.as_ref().map(|k| k.public_key().to_hex()),
            unlocked_at: self.unlocked_at.map(|t| t.elapsed().as_secs()),
        }
    }

    pub fn permissions(&self) -> &PermissionStore {
        &self.permissions
    }

    pub fn permissions_mut(&mut self) -> &mut PermissionStore {
        &mut self.permissions
    }

    pub fn set_provider(&mut self, provider: Box<dyn VaultProvider>) {
        self.provider = Some(provider);
    }

    pub fn evaluate_permission(
        &self,
        identity: &ClientIdentity,
        method: &str,
        kind: Option<Kind>,
    ) -> PermissionCheck {
        match self.permissions.evaluate(identity, method, kind) {
            PermissionResult::Allowed => PermissionCheck::Allowed,
            PermissionResult::Denied => {
                PermissionCheck::Denied(format!("{} denied for {}", method, identity))
            }
            PermissionResult::Ask => {
                let request = ApprovalRequest {
                    id: uuid_v4(),
                    client_identity: identity.to_string(),
                    method: method.to_string(),
                    description: format!("{} request from {}", method, identity),
                    risk_level: "unknown".into(),
                    details: serde_json::json!({ "kind": kind.map(|k| u16::from(k)) }),
                };
                PermissionCheck::Ask(request)
            }
        }
    }

    fn require_unlocked(&self) -> Result<(), Error> {
        if self.state != State::Unlocked {
            return Err(Error::Locked);
        }
        Ok(())
    }

    fn ensure_vault_present(&self) -> Result<(), Error> {
        if !self.check_vault_present() {
            return Err(Error::Vault(vault::Error::NotAvailable("vault not present".into())));
        }
        Ok(())
    }
}

impl NostrSigner for SignerService {
    fn get_public_key(&self) -> Result<PublicKey, protocol::Error> {
        self.require_unlocked().map_err(|_| protocol::Error::Locked)?;
        self.keys.as_ref().map(|k| k.public_key()).ok_or(protocol::Error::Locked)
    }

    fn sign_event(&self, request: SignEventRequest) -> Result<Event, protocol::Error> {
        self.require_unlocked().map_err(|_| protocol::Error::Locked)?;
        self.ensure_vault_present().map_err(|_| protocol::Error::NoVault)?;

        let keys = self.keys.as_ref().ok_or(protocol::Error::Locked)?;

        crypto::sign_event(keys, request.event)
            .map_err(|e| protocol::Error::InvalidEvent(e.to_string()))
    }

    fn nip44_encrypt(&self, request: protocol::Nip44EncryptRequest) -> Result<String, protocol::Error> {
        self.require_unlocked().map_err(|_| protocol::Error::Locked)?;
        let keys = self.keys.as_ref().ok_or(protocol::Error::Locked)?;
        let recipient = PublicKey::parse(&request.recipient_pubkey)
            .map_err(|e| protocol::Error::InvalidKey(format!("{:?}", e)))?;
        crypto::nip44_encrypt(keys, &recipient, &request.plaintext)
            .map_err(|e| protocol::Error::EncryptionError(e.to_string()))
    }

    fn nip44_decrypt(&self, request: protocol::Nip44DecryptRequest) -> Result<String, protocol::Error> {
        self.require_unlocked().map_err(|_| protocol::Error::Locked)?;
        let keys = self.keys.as_ref().ok_or(protocol::Error::Locked)?;
        let sender = PublicKey::parse(&request.sender_pubkey)
            .map_err(|e| protocol::Error::InvalidKey(format!("{:?}", e)))?;
        crypto::nip44_decrypt(keys, &sender, &request.ciphertext)
            .map_err(|e| protocol::Error::DecryptionError(e.to_string()))
    }

    fn lock(&self) -> Result<(), protocol::Error> {
        Err(protocol::Error::Internal("use SignerService::lock() for mutable access".into()))
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("req-{}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_portable_permissions::{PermissionEntry, PermissionRule};
    use tempfile::TempDir;

    fn create_test_provider(tmp: &TempDir) -> Box<dyn VaultProvider> {
        let provider = vault::UsbFileVaultProvider::new(tmp.path());
        let keys = crypto::Keys::generate();
        vault::create_vault(&provider, "test".into(), &keys, "password").unwrap();
        Box::new(provider)
    }

    #[test]
    fn test_initial_state_locked() {
        let service = SignerService::new(None, PermissionStore::new(), Duration::from_secs(300));
        assert_eq!(service.state(), State::Locked);
    }

    #[test]
    fn test_unlock_lock_cycle() {
        let tmp = TempDir::new().unwrap();
        let provider = create_test_provider(&tmp);
        let mut service = SignerService::new(
            Some(provider),
            PermissionStore::new(),
            Duration::from_secs(300),
        );
        service.unlock("password").unwrap();
        assert_eq!(service.state(), State::Unlocked);
        SignerService::lock(&mut service).unwrap();
        assert_eq!(service.state(), State::Locked);
    }

    #[test]
    fn test_unlock_wrong_password() {
        let tmp = TempDir::new().unwrap();
        let provider = create_test_provider(&tmp);
        let mut service = SignerService::new(
            Some(provider),
            PermissionStore::new(),
            Duration::from_secs(300),
        );
        let result = service.unlock("wrong-password");
        assert!(result.is_err());
        assert_eq!(service.state(), State::Locked);
    }

    #[test]
    fn test_get_public_key_when_locked() {
        let service = SignerService::new(None, PermissionStore::new(), Duration::from_secs(300));
        let result = service.get_public_key();
        assert!(matches!(result, Err(protocol::Error::Locked)));
    }

    #[test]
    fn test_get_public_key_when_unlocked() {
        let tmp = TempDir::new().unwrap();
        let provider = create_test_provider(&tmp);
        let mut service = SignerService::new(
            Some(provider),
            PermissionStore::new(),
            Duration::from_secs(300),
        );
        service.unlock("password").unwrap();
        let pk = service.get_public_key().unwrap();
        assert!(!pk.to_hex().is_empty());
    }

    #[test]
    fn test_auto_lock_after_timeout() {
        let tmp = TempDir::new().unwrap();
        let provider = create_test_provider(&tmp);
        let mut service = SignerService::new(
            Some(provider),
            PermissionStore::new(),
            Duration::from_secs(0),
        );
        service.unlock("password").unwrap();
        assert_eq!(service.state(), State::Unlocked);
        service.check_auto_lock();
        assert_eq!(service.state(), State::Locked);
    }

    #[test]
    fn test_vault_present_check() {
        let tmp = TempDir::new().unwrap();
        let provider = create_test_provider(&tmp);
        let service = SignerService::new(
            Some(provider),
            PermissionStore::new(),
            Duration::from_secs(300),
        );
        assert!(service.check_vault_present());
    }

    #[test]
    fn test_vault_not_present() {
        let service = SignerService::new(None, PermissionStore::new(), Duration::from_secs(300));
        assert!(!service.check_vault_present());
    }

    #[test]
    fn test_evaluate_permission_allowed() {
        let mut store = PermissionStore::new();
        let identity = ClientIdentity::Unknown("cli".into());
        store.set_permission(
            identity.clone(),
            PermissionEntry {
                method: "signEvent".into(),
                rule: PermissionRule::Allow,
                kind_restriction: None,
            },
        );
        let service = SignerService::new(None, store, Duration::from_secs(300));
        let result = service.evaluate_permission(&identity, "signEvent", Some(Kind::TextNote));
        assert!(matches!(result, PermissionCheck::Allowed));
    }

    #[test]
    fn test_evaluate_permission_ask() {
        let store = PermissionStore::new();
        let identity = ClientIdentity::Unknown("cli".into());
        let service = SignerService::new(None, store, Duration::from_secs(300));
        let result = service.evaluate_permission(&identity, "signEvent", Some(Kind::TextNote));
        assert!(matches!(result, PermissionCheck::Ask(_)));
    }

    #[test]
    fn test_evaluate_permission_denied() {
        let mut store = PermissionStore::new();
        let identity = ClientIdentity::Unknown("cli".into());
        store.set_permission(
            identity.clone(),
            PermissionEntry {
                method: "signEvent".into(),
                rule: PermissionRule::Deny,
                kind_restriction: None,
            },
        );
        let service = SignerService::new(None, store, Duration::from_secs(300));
        let result = service.evaluate_permission(&identity, "signEvent", Some(Kind::TextNote));
        assert!(matches!(result, PermissionCheck::Denied(_)));
    }

    #[test]
    fn test_status_reporting() {
        let tmp = TempDir::new().unwrap();
        let provider = create_test_provider(&tmp);
        let mut service = SignerService::new(
            Some(provider),
            PermissionStore::new(),
            Duration::from_secs(300),
        );
        let status = service.status();
        assert_eq!(status.state, "locked");
        assert!(status.vault_present);

        service.unlock("password").unwrap();
        let status = service.status();
        assert_eq!(status.state, "unlocked");
        assert!(status.public_key.is_some());
    }
}
