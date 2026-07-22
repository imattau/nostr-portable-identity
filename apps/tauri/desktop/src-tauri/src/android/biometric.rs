/// Biometric approval bridge for Android.
///
/// On Android, signing operations that require user approval use
/// the system BiometricPrompt (fingerprint, face, or device PIN).
///
/// This module provides the data structures and logic for
/// requesting and receiving biometric confirmation.
/// The actual BiometricPrompt UI is handled on the Kotlin/Java side.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiometricResult {
    Success,
    Failure,
    Cancelled,
    NotAvailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricRequest {
    /// Reason for biometric prompt (shown to user)
    pub reason: String,
    /// Optional subtitle for the prompt
    pub subtitle: Option<String>,
    /// Whether to require device PIN fallback
    pub allow_device_pin: bool,
}

impl BiometricRequest {
    pub fn new(reason: &str) -> Self {
        Self {
            reason: reason.to_string(),
            subtitle: None,
            allow_device_pin: true,
        }
    }

    pub fn with_subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }
}

/// Represents the result of a biometric authentication attempt
#[derive(Debug, Clone)]
pub enum AuthenticationResult {
    Authenticated,
    Failed(String),
    Cancelled,
    NotEnrolled,
    HardwareUnavailable,
}

impl AuthenticationResult {
    pub fn is_authenticated(&self) -> bool {
        matches!(self, AuthenticationResult::Authenticated)
    }
}

/// Determine the appropriate biometric reason based on signing operation
pub fn reason_for_method(method: &str, kind: Option<u16>) -> String {
    match method {
        "signEvent" => {
            let base = match kind {
                Some(1) => "Approve text note",
                Some(5) => "Approve deletion request",
                Some(3) => "Approve contact list replacement",
                _ => "Approve signing request",
            };
            base.to_string()
        }
        "nip44Decrypt" => "Authenticate to decrypt message".to_string(),
        "nip44Encrypt" => "Authenticate to encrypt message".to_string(),
        "getPublicKey" => "Authenticate to reveal public key".to_string(),
        _ => "Authenticate to continue".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biometric_request() {
        let req = BiometricRequest::new("Sign a text note")
            .with_subtitle("Nostr Portable Identity");
        assert_eq!(req.reason, "Sign a text note");
        assert_eq!(req.subtitle.unwrap(), "Nostr Portable Identity");
        assert!(req.allow_device_pin);
    }

    #[test]
    fn test_reason_for_method() {
        let r = reason_for_method("signEvent", Some(1));
        assert_eq!(r, "Approve text note");

        let r = reason_for_method("nip44Decrypt", None);
        assert_eq!(r, "Authenticate to decrypt message");
    }

    #[test]
    fn test_authentication_result() {
        let r = AuthenticationResult::Authenticated;
        assert!(r.is_authenticated());

        let r = AuthenticationResult::Failed("timeout".into());
        assert!(!r.is_authenticated());
    }
}
