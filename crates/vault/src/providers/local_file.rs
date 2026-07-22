use std::env;
use std::fs;
use std::path::PathBuf;

use crate::{create_vault, EncryptedVault, Error, VaultProvider};

const APP_DIR: &str = ".nostr-portable-identity";
const VAULTS_DIR: &str = "vaults";
const DEFAULT_VAULT_NAME: &str = "default";

/// A vault provider that stores encrypted vaults in the user's home directory.
///
/// Location: ~/.nostr-portable-identity/vaults/<name>.json
///
/// This enables using the signer without a USB drive — the vault
/// is stored locally and encrypted at rest (NIP-49 ncryptsec).
pub struct LocalFileVaultProvider {
    vaults_dir: PathBuf,
    active_name: String,
}

impl LocalFileVaultProvider {
    /// Create a new local vault provider with the default directory.
    /// Creates the directory structure if it doesn't exist.
    pub fn new() -> Result<Self, Error> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| Error::NotAvailable("HOME not set".into()))?;

        let vaults_dir = PathBuf::from(home)
            .join(APP_DIR)
            .join(VAULTS_DIR);

        fs::create_dir_all(&vaults_dir).map_err(Error::Io)?;

        Ok(Self {
            vaults_dir,
            active_name: DEFAULT_VAULT_NAME.to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn new_with_home(home: PathBuf) -> Result<Self, Error> {
        let vaults_dir = home.join(APP_DIR).join(VAULTS_DIR);
        fs::create_dir_all(&vaults_dir).map_err(Error::Io)?;
        Ok(Self {
            vaults_dir,
            active_name: DEFAULT_VAULT_NAME.to_string(),
        })
    }

    /// Set the active vault name (supports multiple identities)
    pub fn set_active(&mut self, name: &str) {
        self.active_name = name.to_string();
    }

    /// List all available vaults in the store
    pub fn list_vaults(&self) -> Result<Vec<String>, Error> {
        let mut vaults = Vec::new();
        for entry in fs::read_dir(&self.vaults_dir).map_err(Error::Io)? {
            let entry = entry.map_err(Error::Io)?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(stem) = path.file_stem() {
                    vaults.push(stem.to_string_lossy().to_string());
                }
            }
        }
        Ok(vaults)
    }

    /// Get the vault path for a given name
    fn vault_path(&self, name: &str) -> PathBuf {
        self.vaults_dir.join(format!("{}.json", name))
    }

    fn active_path(&self) -> PathBuf {
        self.vault_path(&self.active_name)
    }

    /// Create a new identity vault with a local path.
    /// Automatically switches the active identity to the newly created vault.
    pub fn create(&mut self, name: &str, keys: &nostr_portable_crypto::Keys, passphrase: &str) -> Result<EncryptedVault, Error> {
        let provider = Self {
            vaults_dir: self.vaults_dir.clone(),
            active_name: name.to_string(),
        };
        let vault = create_vault(&provider, name.to_string(), keys, passphrase)?;
        self.active_name = name.to_string();
        Ok(vault)
    }
}

impl VaultProvider for LocalFileVaultProvider {
    fn is_available(&self) -> bool {
        // The local vault is always available as long as the vault file exists
        self.active_path().exists()
    }

    fn vault_id(&self) -> Result<String, Error> {
        let vault = self.load_encrypted_vault()?;
        Ok(format!("{}::{}", vault.pubkey, vault.name))
    }

    fn load_encrypted_vault(&self) -> Result<EncryptedVault, Error> {
        let path = self.active_path();
        let content = fs::read_to_string(&path)?;
        let vault: EncryptedVault = serde_json::from_str(&content)?;
        vault.validate()?;
        Ok(vault)
    }

    fn write_encrypted_vault(&self, vault: &EncryptedVault) -> Result<(), Error> {
        let path = self.active_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let content = serde_json::to_string_pretty(vault)?;
        fs::write(&path, content)?;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.active_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unlock_vault;
    use nostr_portable_crypto::Keys;
    use tempfile::TempDir;

    fn create_local_provider(tmp: &TempDir) -> LocalFileVaultProvider {
        let home = tmp.path().join("home");
        LocalFileVaultProvider::new_with_home(home).unwrap()
    }

    #[test]
    fn test_create_and_load_vault() {
        let tmp = TempDir::new().unwrap();
        let mut provider = create_local_provider(&tmp);
        let keys = Keys::generate();

        let vault = provider.create("test-id", &keys, "password").unwrap();
        assert_eq!(vault.name, "test-id");

        assert!(provider.is_available());
        let loaded = provider.load_encrypted_vault().unwrap();
        assert_eq!(loaded.pubkey, keys.public_key().to_hex());
    }

    #[test]
    fn test_unlock_vault() {
        let tmp = TempDir::new().unwrap();
        let mut provider = create_local_provider(&tmp);
        let keys = Keys::generate();

        provider.create("test-id", &keys, "secret").unwrap();
        let unlocked = unlock_vault(&provider, "secret").unwrap();
        assert_eq!(keys.public_key(), unlocked.public_key());
    }

    #[test]
    fn test_list_vaults() {
        let tmp = TempDir::new().unwrap();
        let mut provider = create_local_provider(&tmp);
        let keys = Keys::generate();

        provider.create("alice", &keys, "p1").unwrap();
        provider.create("bob", &keys, "p2").unwrap();

        let vaults = provider.list_vaults().unwrap();
        assert!(vaults.contains(&"alice".to_string()));
        assert!(vaults.contains(&"bob".to_string()));
    }

    #[test]
    fn test_set_active_identity() {
        let tmp = TempDir::new().unwrap();
        let mut provider = create_local_provider(&tmp);
        let keys = Keys::generate();

        provider.create("primary", &keys, "p1").unwrap();
        assert!(provider.is_available());
        assert_eq!(provider.name(), "primary");

        provider.create("secondary", &keys, "p2").unwrap();
        assert_eq!(provider.name(), "secondary");
        assert!(provider.is_available());

        provider.set_active("primary");
        assert_eq!(provider.name(), "primary");
        assert!(provider.is_available());
    }
}
