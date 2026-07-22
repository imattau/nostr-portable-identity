use std::path::PathBuf;
use std::fs;
use nostr_portable_crypto as crypto;
use crypto::{PublicKey, Timestamp};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("vault not available at path: {0}")]
    NotAvailable(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("crypto error: {0}")]
    Crypto(#[from] crypto::Error),
    #[error("vault version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u8, actual: u8 },
    #[error("invalid vault type: {0}")]
    InvalidVaultType(String),
    #[error("vault identifier mismatch")]
    IdentifierMismatch,
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedVault {
    pub version: u8,
    #[serde(rename = "type")]
    pub vault_type: String,
    pub name: String,
    pub pubkey: String,
    pub encrypted_key: String,
    pub created_at: u64,
}

impl EncryptedVault {
    pub const CURRENT_VERSION: u8 = 1;
    pub const EXPECTED_TYPE: &'static str = "nostr-portable-vault";

    pub fn new(
        name: String,
        pubkey: &PublicKey,
        encrypted_key: String,
    ) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            vault_type: Self::EXPECTED_TYPE.to_string(),
            name,
            pubkey: pubkey.to_hex(),
            encrypted_key,
            created_at: Timestamp::now().as_secs(),
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.version != Self::CURRENT_VERSION {
            return Err(Error::VersionMismatch {
                expected: Self::CURRENT_VERSION,
                actual: self.version,
            });
        }
        if self.vault_type != Self::EXPECTED_TYPE {
            return Err(Error::InvalidVaultType(self.vault_type.clone()));
        }
        Ok(())
    }
}

pub trait VaultProvider: Send + Sync {
    fn is_available(&self) -> bool;
    fn vault_id(&self) -> Result<String, Error>;
    fn load_encrypted_vault(&self) -> Result<EncryptedVault, Error>;
    fn write_encrypted_vault(&self, vault: &EncryptedVault) -> Result<(), Error>;
    fn name(&self) -> &str;
}

pub struct UsbFileVaultProvider {
    base_path: PathBuf,
    name: String,
}

impl UsbFileVaultProvider {
    const VAULT_DIR: &'static str = "NOSTR-SIGNER";
    const VAULT_FILE: &'static str = "nostr-vault.json";

    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        let resolved: PathBuf = base_path.into();
        Self {
            name: format!("USB vault at {}", resolved.display()),
            base_path: resolved,
        }
    }

    fn vault_path(&self) -> PathBuf {
        self.base_path.join(Self::VAULT_DIR).join(Self::VAULT_FILE)
    }

    pub fn vault_exists(&self) -> bool {
        self.vault_path().exists()
    }

    pub fn recommend_path() -> PathBuf {
        PathBuf::from("USB Drive").join(Self::VAULT_DIR).join(Self::VAULT_FILE)
    }
}

impl VaultProvider for UsbFileVaultProvider {
    fn is_available(&self) -> bool {
        self.vault_path().exists()
    }

    fn vault_id(&self) -> Result<String, Error> {
        let vault = self.load_encrypted_vault()?;
        Ok(format!("{}::{}", vault.pubkey, vault.name))
    }

    fn load_encrypted_vault(&self) -> Result<EncryptedVault, Error> {
        let path = self.vault_path();
        if !path.exists() {
            return Err(Error::NotAvailable(path.display().to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let vault: EncryptedVault = serde_json::from_str(&content)?;
        vault.validate()?;
        Ok(vault)
    }

    fn write_encrypted_vault(&self, vault: &EncryptedVault) -> Result<(), Error> {
        let dir = self.base_path.join(Self::VAULT_DIR);
        fs::create_dir_all(&dir)?;
        let path = dir.join(Self::VAULT_FILE);
        let content = serde_json::to_string_pretty(vault)?;
        fs::write(&path, content)?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub fn create_vault(
    provider: &dyn VaultProvider,
    name: String,
    keys: &crypto::Keys,
    passphrase: &str,
) -> Result<EncryptedVault, Error> {
    let encrypted_key = crypto::encrypt_ncryptsec(keys, passphrase)?;
    let pubkey = crypto::get_public_key(keys);
    let vault = EncryptedVault::new(name, &pubkey, encrypted_key);
    provider.write_encrypted_vault(&vault)?;
    Ok(vault)
}

pub fn unlock_vault(
    provider: &dyn VaultProvider,
    passphrase: &str,
) -> Result<crypto::Keys, Error> {
    let vault = provider.load_encrypted_vault()?;
    let keys = crypto::decrypt_ncryptsec(&vault.encrypted_key, passphrase)?;
    Ok(keys)
}

pub fn vault_exists_on(provider: &dyn VaultProvider) -> bool {
    provider.is_available()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::Keys;
    use tempfile::TempDir;

    #[test]
    fn test_encrypted_vault_validation() {
        let vault = EncryptedVault {
            version: 1,
            vault_type: "nostr-portable-vault".into(),
            name: "test".into(),
            pubkey: "abc".into(),
            encrypted_key: "ncryptsec1...".into(),
            created_at: 1000,
        };
        assert!(vault.validate().is_ok());
    }

    #[test]
    fn test_encrypted_vault_wrong_version() {
        let vault = EncryptedVault {
            version: 99,
            vault_type: "nostr-portable-vault".into(),
            name: "test".into(),
            pubkey: "abc".into(),
            encrypted_key: "ncryptsec1...".into(),
            created_at: 1000,
        };
        assert!(vault.validate().is_err());
    }

    #[test]
    fn test_encrypted_vault_wrong_type() {
        let vault = EncryptedVault {
            version: 1,
            vault_type: "wrong-type".into(),
            name: "test".into(),
            pubkey: "abc".into(),
            encrypted_key: "ncryptsec1...".into(),
            created_at: 1000,
        };
        assert!(vault.validate().is_err());
    }

    #[test]
    fn test_usb_vault_provider_write_and_read() {
        let tmp = TempDir::new().unwrap();
        let provider = UsbFileVaultProvider::new(tmp.path());

        let keys = Keys::generate();
        let _vault = create_vault(&provider, "test-identity".into(), &keys, "password").unwrap();

        assert!(provider.vault_exists());
        assert!(provider.is_available());

        let loaded = provider.load_encrypted_vault().unwrap();
        assert_eq!(loaded.name, "test-identity");
        assert_eq!(loaded.pubkey, keys.public_key().to_hex());
    }

    #[test]
    fn test_unlock_vault_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let provider = UsbFileVaultProvider::new(tmp.path());

        let keys = Keys::generate();
        create_vault(&provider, "test".into(), &keys, "password").unwrap();

        let unlocked = unlock_vault(&provider, "password").unwrap();
        assert_eq!(keys.public_key(), unlocked.public_key());
    }

    #[test]
    fn test_unlock_vault_wrong_password() {
        let tmp = TempDir::new().unwrap();
        let provider = UsbFileVaultProvider::new(tmp.path());

        let keys = Keys::generate();
        create_vault(&provider, "test".into(), &keys, "correct").unwrap();

        let result = unlock_vault(&provider, "wrong");
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_not_available() {
        let provider = UsbFileVaultProvider::new("/nonexistent/path");
        assert!(!provider.is_available());
        let result = provider.load_encrypted_vault();
        assert!(result.is_err());
    }
}
