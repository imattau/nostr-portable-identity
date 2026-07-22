use crate::{EncryptedVault, Error, VaultProvider};

/// Smartcard / hardware signer abstraction.
///
/// This provider represents a future backend where the private key
/// lives on a hardware device (YubiKey, OpenPGP smartcard, Nitrokey,
/// Trezor, Ledger, etc.) and cannot be exported.
///
/// The device handles signing internally — the host only sends
/// data to be signed and receives the signature.
///
/// Currently a placeholder for the provider interface.
pub struct SmartcardProvider {
    /// Human-readable name
    name: String,
    /// Whether the smartcard is currently present
    card_present: bool,
    /// The public key exposed by the card (hex)
    pubkey: Option<String>,
}

/// Supported smartcard / hardware types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareType {
    YubiKeyPiv,
    OpenPgpCard,
    Trezor,
    Ledger,
    Nitrokey,
    Tpm,
    SecureEnclave,
}

impl SmartcardProvider {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            card_present: false,
            pubkey: None,
        }
    }

    pub fn set_card_present(&mut self, present: bool) {
        self.card_present = present;
    }

    pub fn set_pubkey(&mut self, pubkey: &str) {
        self.pubkey = Some(pubkey.to_string());
    }

    pub fn card_present(&self) -> bool {
        self.card_present
    }

    pub fn card_type_detected(&self) -> Option<HardwareType> {
        // In a real implementation, detect the card type via PC/SC
        None
    }
}

impl VaultProvider for SmartcardProvider {
    fn is_available(&self) -> bool {
        self.card_present && self.pubkey.is_some()
    }

    fn vault_id(&self) -> Result<String, Error> {
        match &self.pubkey {
            Some(pk) => Ok(format!("hardware:{}::{}", pk, self.name)),
            None => Err(Error::NotFound("no smartcard key loaded".into())),
        }
    }

    fn load_encrypted_vault(&self) -> Result<EncryptedVault, Error> {
        Err(Error::NotAvailable(
            "smartcard keys cannot be exported — use the device directly".into(),
        ))
    }

    fn write_encrypted_vault(&self, _vault: &EncryptedVault) -> Result<(), Error> {
        Err(Error::NotAvailable(
            "cannot write vault to smartcard".into(),
        ))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A provider backed by platform secure element / TPM.
///
/// On Linux: TPM 2.0
/// On macOS: Secure Enclave
/// On Windows: TPM 2.0 via Windows Hello / CNG
/// On Android: Android Keystore / TEE
/// On iOS: Secure Enclave
pub struct SecureElementProvider {
    name: String,
    available: bool,
}

impl SecureElementProvider {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            available: false,
        }
    }

    /// Check whether the platform secure element is available
    pub fn platform_available() -> bool {
        #[cfg(target_os = "macos")]
        { true } // macOS has Secure Enclave on all T2/Apple Silicon devices
        #[cfg(target_os = "linux")]
        { std::path::Path::new("/dev/tpm0").exists() }
        #[cfg(target_os = "windows")]
        { true } // Windows has TPM via CNG
        #[cfg(target_os = "android")]
        { true } // Android has KeyStore / TEE
        #[cfg(target_os = "ios")]
        { true } // iOS has Secure Enclave
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows", target_os = "android", target_os = "ios")))]
        { false }
    }
}

impl VaultProvider for SecureElementProvider {
    fn is_available(&self) -> bool {
        self.available
    }

    fn vault_id(&self) -> Result<String, Error> {
        Err(Error::NotAvailable("secure element provider not initialized".into()))
    }

    fn load_encrypted_vault(&self) -> Result<EncryptedVault, Error> {
        Err(Error::NotAvailable(
            "secure element keys cannot be exported".into(),
        ))
    }

    fn write_encrypted_vault(&self, _vault: &EncryptedVault) -> Result<(), Error> {
        Err(Error::NotAvailable(
            "cannot write vault to secure element".into(),
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
    fn test_smartcard_initial_not_present() {
        let provider = SmartcardProvider::new("YubiKey");
        assert!(!provider.is_available());
        assert!(!provider.card_present());
    }

    #[test]
    fn test_smartcard_with_key() {
        let mut provider = SmartcardProvider::new("YubiKey");
        provider.set_card_present(true);
        provider.set_pubkey("abcdef");
        assert!(provider.is_available());
    }

    #[test]
    fn test_smartcard_load_encrypted_fails() {
        let provider = SmartcardProvider::new("test");
        assert!(provider.load_encrypted_vault().is_err());
    }

    #[test]
    fn test_secure_element_provider() {
        let se = SecureElementProvider::new("TPM 2.0");
        assert!(!se.is_available());
        assert!(se.vault_id().is_err());
    }
}
