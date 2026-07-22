/// iOS biometric authentication bridge (Face ID / Touch ID).
///
/// On iOS, biometric authentication uses LocalAuthentication framework (LAContext).
/// The actual authentication is handled on the Swift/ObjC side via Tauri plugin
/// or direct FFI. This module provides the Rust-side data types and logic.

use serde::{Deserialize, Serialize};

/// Biometric authentication policy on iOS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IosBiometricPolicy {
    /// Face ID (iPhone X+) or Touch ID (iPhone 5s–8)
    DeviceOwnerAuthenticationWithBiometrics,
    /// Face ID/Touch ID + device passcode fallback
    DeviceOwnerAuthentication,
}

/// Result of an iOS biometric authentication attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IosBiometricResult {
    Success,
    Failure(String),
    Cancelled,
    BiometryNotAvailable,
    BiometryNotEnrolled,
    BiometryLockout,
}

/// Reason string displayed in the biometric prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricPrompt {
    /// Main reason (required, shown in the dialog)
    pub reason: String,
    /// Subtitle (optional, shown below reason on iOS 14+)
    pub subtitle: Option<String>,
    /// Whether to fall back to device passcode
    pub fallback_to_passcode: bool,
}

impl BiometricPrompt {
    pub fn new(reason: &str) -> Self {
        Self {
            reason: reason.to_string(),
            subtitle: None,
            fallback_to_passcode: true,
        }
    }

    pub fn with_subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }
}

impl IosBiometricResult {
    pub fn is_authenticated(&self) -> bool {
        matches!(self, IosBiometricResult::Success)
    }
}

/// Map signing operations to appropriate biometric prompt text
pub fn prompt_for_method(method: &str, kind: Option<u16>) -> BiometricPrompt {
    let reason = match method {
        "signEvent" => match kind {
            Some(1) => "Sign a text note",
            Some(5) => "Confirm event deletion",
            Some(3) => "Update contact list",
            _ => "Approve signing request",
        },
        "nip44Decrypt" => "Decrypt a private message",
        "nip44Encrypt" => "Encrypt a message",
        "getPublicKey" => "Reveal your public key",
        _ => "Authenticate",
    };
    BiometricPrompt::new(reason).with_subtitle("Nostr Portable Identity")
}

/// Determine which biometric policy to use based on the operation sensitivity
pub fn policy_for_risk(risk_level: &str) -> IosBiometricPolicy {
    match risk_level {
        "destructive" | "warning" => IosBiometricPolicy::DeviceOwnerAuthentication,
        _ => IosBiometricPolicy::DeviceOwnerAuthenticationWithBiometrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_for_text_note() {
        let p = prompt_for_method("signEvent", Some(1));
        assert_eq!(p.reason, "Sign a text note");
        assert!(p.fallback_to_passcode);
    }

    #[test]
    fn test_prompt_for_deletion() {
        let p = prompt_for_method("signEvent", Some(5));
        assert_eq!(p.reason, "Confirm event deletion");
    }

    #[test]
    fn test_prompt_for_decrypt() {
        let p = prompt_for_method("nip44Decrypt", None);
        assert_eq!(p.reason, "Decrypt a private message");
    }

    #[test]
    fn test_policy_for_risk() {
        assert_eq!(
            policy_for_risk("destructive"),
            IosBiometricPolicy::DeviceOwnerAuthentication
        );
        assert_eq!(
            policy_for_risk("info"),
            IosBiometricPolicy::DeviceOwnerAuthenticationWithBiometrics
        );
    }

    #[test]
    fn test_biometric_result() {
        assert!(IosBiometricResult::Success.is_authenticated());
        assert!(!IosBiometricResult::Cancelled.is_authenticated());
    }
}
