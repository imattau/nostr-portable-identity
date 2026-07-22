use serde::{Deserialize, Serialize};

/// NIP-55 defines how Android applications can request signing
/// operations from a dedicated signer application via Android Intents.
///
/// The signer (our Tauri Android app) acts as the target of these
/// intent-based requests.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip55SignEventRequest {
    pub id: String,
    pub kind: u16,
    pub content: String,
    pub tags: Vec<Vec<String>>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip55SignEventResponse {
    pub id: String,
    pub sig: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip55DecryptRequest {
    pub id: String,
    pub pubkey: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip55EncryptRequest {
    pub id: String,
    pub pubkey: String,
    pub plaintext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip55DecryptResponse {
    pub id: String,
    pub plaintext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip55EncryptResponse {
    pub id: String,
    pub ciphertext: String,
}

impl Nip55SignEventRequest {
    /// Parse a NIP-55 intent request from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON for intent extras
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Nip55SignEventResponse {
    /// Serialize to JSON for intent response
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Nip55DecryptRequest {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Nip55EncryptRequest {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Nip55DecryptResponse {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Nip55EncryptResponse {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// The NIP-55 intent action used by Android Nostr apps to request signing
pub const NIP55_SIGN_EVENT_ACTION: &str = "to.nostr.action.SIGN_EVENT";
pub const NIP55_DECRYPT_ACTION: &str = "to.nostr.action.DECRYPT";
pub const NIP55_ENCRYPT_ACTION: &str = "to.nostr.action.ENCRYPT";
pub const NIP55_GET_PUBLIC_KEY_ACTION: &str = "to.nostr.action.GET_PUBLIC_KEY";

/// Extra names for intent extras
pub const EXTRA_REQUEST: &str = "request";
pub const EXTRA_RESPONSE: &str = "response";
pub const EXTRA_PACKAGE_NAME: &str = "package_name";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_event_request_roundtrip() {
        let req = Nip55SignEventRequest {
            id: "req-1".into(),
            kind: 1,
            content: "Hello".into(),
            tags: vec![],
            created_at: 1784678400,
        };
        let json = req.to_json().unwrap();
        let parsed = Nip55SignEventRequest::from_json(&json).unwrap();
        assert_eq!(parsed.id, "req-1");
        assert_eq!(parsed.kind, 1);
        assert_eq!(parsed.content, "Hello");
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = Nip55SignEventResponse {
            id: "req-1".into(),
            sig: "abcd".into(),
            event_id: "ef01".into(),
        };
        let json = resp.to_json().unwrap();
        assert!(json.contains("req-1"));
        assert!(json.contains("abcd"));
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let enc = Nip55EncryptRequest {
            id: "req-2".into(),
            pubkey: "abc".into(),
            plaintext: "secret".into(),
        };
        let json = enc.to_json().unwrap();
        let parsed = Nip55EncryptRequest::from_json(&json).unwrap();
        assert_eq!(parsed.plaintext, "secret");

        let dec_resp = Nip55DecryptResponse {
            id: "req-2".into(),
            plaintext: "secret".into(),
        };
        let json2 = dec_resp.to_json().unwrap();
        assert!(json2.contains("secret"));
    }
}
