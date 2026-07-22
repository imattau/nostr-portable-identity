pub mod nip46;

use nostr::event::{Event, Kind, UnsignedEvent};
use nostr::key::PublicKey;
use nostr_portable_event_policy as event_policy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    #[error("signer is locked")]
    Locked,
    #[error("signer is not available")]
    Unavailable,
    #[error("no vault connected")]
    NoVault,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("user rejected the request")]
    UserRejected,
    #[error("invalid event: {0}")]
    InvalidEvent(String),
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("encryption error: {0}")]
    EncryptionError(String),
    #[error("decryption error: {0}")]
    DecryptionError(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignEventRequest {
    pub event: UnsignedEvent,
    pub kind: Kind,
    pub content: String,
    pub tags: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip44EncryptRequest {
    pub recipient_pubkey: String,
    pub plaintext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip44DecryptRequest {
    pub sender_pubkey: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub client_identity: String,
    pub method: String,
    pub description: String,
    pub risk_level: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub approved: bool,
    pub remember: bool,
}

pub trait NostrSigner: Send + Sync {
    fn get_public_key(&self) -> Result<PublicKey, Error>;
    fn sign_event(&self, request: SignEventRequest) -> Result<Event, Error>;
    fn nip44_encrypt(&self, request: Nip44EncryptRequest) -> Result<String, Error>;
    fn nip44_decrypt(&self, request: Nip44DecryptRequest) -> Result<String, Error>;
    fn lock(&self) -> Result<(), Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerStatus {
    pub state: String,
    pub vault_present: bool,
    pub public_key: Option<String>,
    pub unlocked_at: Option<u64>,
}

impl From<event_policy::Error> for Error {
    fn from(e: event_policy::Error) -> Self {
        Error::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_error_serialization() {
        let err = Error::Locked;
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, "\"locked\"");
    }

    #[test]
    fn test_signer_error_deserialization() {
        let json = "\"user_rejected\"";
        let err: Error = serde_json::from_str(json).unwrap();
        assert!(matches!(err, Error::UserRejected));
    }

    #[test]
    fn test_approval_request_serialization() {
        let req = ApprovalRequest {
            id: "req-1".into(),
            client_identity: "https://example.com".into(),
            method: "signEvent".into(),
            description: "Publish a text note".into(),
            risk_level: "Info".into(),
            details: serde_json::json!({}),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("req-1"));
        assert!(json.contains("signEvent"));
    }
}
