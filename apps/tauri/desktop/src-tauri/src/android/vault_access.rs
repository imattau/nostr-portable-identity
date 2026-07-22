use nostr_portable_vault as vault;
use nostr_portable_vault::{EncryptedVault, VaultProvider};

/// Android vault access using the Storage Access Framework (SAF).
///
/// Android SAF allows access to documents and files through a content
/// URI system rather than direct file paths. The user picks a vault
/// file through the system document picker, and we get a content:// URI.
///
/// For USB-C OTG drives, Android presents them through the same SAF
/// framework when the user selects them.

#[derive(Debug, Clone)]
pub struct SafVaultUri {
    /// The content:// URI from Android SAF
    pub uri: String,
    /// Display name for the vault
    pub display_name: String,
}

/// Represents the state of USB-C OTG vault access on Android
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtgState {
    /// No OTG device connected
    NotConnected,
    /// OTG device connected but no vault file found
    DeviceDetected,
    /// Vault file found on OTG device
    VaultFound(String),
}

/// Android-specific vault provider that operates on content URIs.
///
/// This is a stub that will be wired to native Android SAF APIs
/// through Tauri plugin commands or direct JNI calls.
pub struct AndroidVaultProvider {
    uri: Option<String>,
    encrypted_vault: Option<EncryptedVault>,
}

impl AndroidVaultProvider {
    pub fn new() -> Self {
        Self {
            uri: None,
            encrypted_vault: None,
        }
    }

    pub fn set_uri(&mut self, uri: String) {
        self.uri = Some(uri);
    }

    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }
}

impl VaultProvider for AndroidVaultProvider {
    fn is_available(&self) -> bool {
        self.uri.is_some() && self.encrypted_vault.is_some()
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
            "write not supported via SAF URI".into(),
        ))
    }

    fn name(&self) -> &str {
        "Android SAF Vault"
    }
}

/// Detect whether a content URI points to a NOSTR-SIGNER/nostr-vault.json path
pub fn is_vault_uri(uri: &str) -> bool {
    uri.contains("nostr-vault.json") || uri.contains("NOSTR-SIGNER")
}

/// NIP-55 package name for the Android signer app
pub const SIGNER_PACKAGE: &str = "com.nostr.portable.identity";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_uri_detection() {
        assert!(is_vault_uri("content://com.android.externalstorage/doc/document/primary:NOSTR-SIGNER/nostr-vault.json"));
        assert!(is_vault_uri("content://media/external/file/12345?name=nostr-vault.json"));
        assert!(!is_vault_uri("content://media/external/file/12345?name=photo.jpg"));
    }

    #[test]
    fn test_android_vault_provider() {
        let provider = AndroidVaultProvider::new();
        assert!(!provider.is_available());
        assert!(provider.load_encrypted_vault().is_err());
    }
}
