use serde::{Deserialize, Serialize};

/// A NIP-46 request received from a relay (bunker request).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46Request {
    pub id: String,
    pub method: String,
    pub params: Vec<String>,
}

/// A NIP-46 response sent back through the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46Response {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Connection parameters for initiating a NIP-46 bunker session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46ConnectRequest {
    pub relay: String,
    pub client_name: Option<String>,
    pub client_url: Option<String>,
    pub client_icon: Option<String>,
}

/// Response to a NIP-46 connect request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nip46ConnectResponse {
    pub bunker_pubkey: String,
    pub relay: String,
}

impl Nip46Request {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Nip46Response {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

pub const METHOD_GET_PUBLIC_KEY: &str = "get_public_key";
pub const METHOD_SIGN_EVENT: &str = "sign_event";
pub const METHOD_NIP44_ENCRYPT: &str = "nip44_encrypt";
pub const METHOD_NIP44_DECRYPT: &str = "nip44_decrypt";
pub const METHOD_CONNECT: &str = "connect";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_roundtrip() {
        let req = Nip46Request {
            id: "req-1".into(),
            method: "sign_event".into(),
            params: vec!["param1".into()],
        };
        let json = req.to_json().unwrap();
        let parsed = Nip46Request::from_json(&json).unwrap();
        assert_eq!(parsed.id, "req-1");
    }

    #[test]
    fn test_response_to_json() {
        let resp = Nip46Response {
            id: "req-1".into(),
            result: Some("result-hex".into()),
            error: None,
        };
        let json = resp.to_json().unwrap();
        assert!(json.contains("result-hex"));
    }

    #[test]
    fn test_nip46_request_to_json() {
        let req = Nip46Request {
            id: "req-1".into(),
            method: "get_public_key".into(),
            params: vec![],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("get_public_key"));
    }

    #[test]
    fn test_method_constants() {
        assert_eq!(METHOD_GET_PUBLIC_KEY, "get_public_key");
        assert_eq!(METHOD_CONNECT, "connect");
    }
}
