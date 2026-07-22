use crate::{EncryptedVault, Error, VaultProvider};

/// A vault provider that proxies signing operations to a remote
/// NIP-46 signer via relay communication.
///
/// This allows the local signer agent to act as a client to a
/// remote NIP-46 bunker (e.g., a hardware device or another
/// signer instance running on a different machine).
///
/// The remote provider stores metadata about the connection
/// rather than an encrypted key — the actual key lives on the
/// remote signer.
pub struct Nip46RemoteProvider {
    /// The relay URL where the remote signer listens
    relay: String,
    /// The remote signer's public key (hex)
    remote_pubkey: String,
    /// Human-readable name for this remote identity
    name: String,
    /// Whether the remote signer is currently reachable
    connected: bool,
}

impl Nip46RemoteProvider {
    pub fn new(relay: &str, remote_pubkey: &str, name: &str) -> Self {
        Self {
            relay: relay.to_string(),
            remote_pubkey: remote_pubkey.to_string(),
            name: name.to_string(),
            connected: false,
        }
    }

    pub fn relay(&self) -> &str {
        &self.relay
    }

    pub fn remote_pubkey(&self) -> &str {
        &self.remote_pubkey
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Serialize connection info for persistence
    pub fn to_connection_info(&self) -> String {
        serde_json::json!({
            "type": "nip46-remote",
            "relay": self.relay,
            "remote_pubkey": self.remote_pubkey,
            "name": self.name,
        })
        .to_string()
    }

    /// Parse connection info from stored JSON
    pub fn from_connection_info(json: &str) -> Result<Self, serde_json::Error> {
        let v: serde_json::Value = serde_json::from_str(json)?;
        Ok(Self {
            relay: v["relay"].as_str().unwrap_or("").to_string(),
            remote_pubkey: v["remote_pubkey"].as_str().unwrap_or("").to_string(),
            name: v["name"].as_str().unwrap_or("Remote").to_string(),
            connected: false,
        })
    }
}

impl VaultProvider for Nip46RemoteProvider {
    fn is_available(&self) -> bool {
        self.connected
    }

    fn vault_id(&self) -> Result<String, Error> {
        if !self.connected {
            return Err(Error::NotAvailable("remote signer not connected".into()));
        }
        Ok(format!("remote:{}::{}", self.remote_pubkey, self.name))
    }

    fn load_encrypted_vault(&self) -> Result<EncryptedVault, Error> {
        Err(Error::NotAvailable(
            "remote signer does not provide encrypted vault — use connect handshake".into(),
        ))
    }

    fn write_encrypted_vault(&self, _vault: &EncryptedVault) -> Result<(), Error> {
        Err(Error::NotAvailable(
            "cannot write to remote signer vault".into(),
        ))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_provider_initial_state() {
        let provider = Nip46RemoteProvider::new(
            "wss://relay.nostr.example",
            "abc123def456",
            "My Hardware Signer",
        );
        assert!(!provider.is_available());
        assert_eq!(provider.relay(), "wss://relay.nostr.example");
        assert_eq!(provider.remote_pubkey(), "abc123def456");
    }

    #[test]
    fn test_remote_provider_availability() {
        let mut provider = Nip46RemoteProvider::new(
            "wss://relay.example",
            "pubkey",
            "test",
        );
        assert!(!provider.is_available());
        provider.set_connected(true);
        assert!(provider.is_available());
    }

    #[test]
    fn test_connection_info_roundtrip() {
        let provider = Nip46RemoteProvider::new(
            "wss://relay.nostr.example",
            "abc123",
            "Remote Signer",
        );
        let json = provider.to_connection_info();
        let restored = Nip46RemoteProvider::from_connection_info(&json).unwrap();
        assert_eq!(restored.relay, "wss://relay.nostr.example");
        assert_eq!(restored.remote_pubkey, "abc123");
        assert_eq!(restored.name, "Remote Signer");
        assert!(!restored.connected);
    }

    #[test]
    fn test_load_encrypted_vault_returns_error() {
        let provider = Nip46RemoteProvider::new("wss://relay.example", "pk", "test");
        assert!(provider.load_encrypted_vault().is_err());
    }
}
