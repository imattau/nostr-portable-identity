use nostr_portable_vault as vault;
use nostr_portable_vault::{EncryptedVault, VaultProvider};

/// iOS vault access through the system document picker (UIDocumentPickerViewController).
///
/// On iOS, direct file system access is restricted. Users select vault
/// files through the document picker, which returns a security-scoped URL.
/// The URL must be accessed via coordinated reads and the security scope
/// must be explicitly started and stopped.

#[derive(Debug, Clone)]
pub struct IosVaultUrl {
    /// The file URL from UIDocumentPickerViewController
    pub url: String,
    /// Whether the URL is security-scoped (requires start/stopAccessingSecurityScopedResource)
    pub is_security_scoped: bool,
}

/// iOS-specific vault provider using security-scoped file URLs.
pub struct IosVaultProvider {
    url: Option<String>,
    encrypted_vault: Option<EncryptedVault>,
}

impl IosVaultProvider {
    pub fn new() -> Self {
        Self {
            url: None,
            encrypted_vault: None,
        }
    }

    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

impl VaultProvider for IosVaultProvider {
    fn is_available(&self) -> bool {
        self.url.is_some() && self.encrypted_vault.is_some()
    }

    fn vault_id(&self) -> Result<String, vault::Error> {
        self.encrypted_vault
            .as_ref()
            .map(|v| format!("{}::{}", v.pubkey, v.name))
            .ok_or_else(|| vault::Error::NotFound("no vault loaded".into()))
    }

    fn load_encrypted_vault(&self) -> Result<EncryptedVault, vault::Error> {
        self.encrypted_vault
            .clone()
            .ok_or_else(|| vault::Error::NotFound("no vault loaded".into()))
    }

    fn write_encrypted_vault(&self, _vault: &EncryptedVault) -> Result<(), vault::Error> {
        Err(vault::Error::NotAvailable(
            "write not supported via iOS document picker URL".into(),
        ))
    }

    fn name(&self) -> &str {
        "iOS Document Vault"
    }
}

/// Bundle identifier for the iOS app
pub const IOS_APP_GROUP: &str = "group.com.nostr.portable.identity";

/// Shared container URL for app group (used for vault file sharing between
/// the main app and Safari extension)
pub fn shared_container_url(app_group: &str) -> String {
    format!("file:///private/var/mobile/Containers/Shared/AppGroup/{}/", app_group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_vault_provider_initial_state() {
        let provider = IosVaultProvider::new();
        assert!(!provider.is_available());
        assert!(provider.load_encrypted_vault().is_err());
    }

    #[test]
    fn test_shared_container_url() {
        let url = shared_container_url(IOS_APP_GROUP);
        assert!(url.contains(IOS_APP_GROUP));
    }
}
