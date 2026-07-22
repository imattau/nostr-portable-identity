/// NIP-46 remote signing protocol integration for iOS.
///
/// NIP-46 defines a remote signing protocol where a signer (our app)
/// communicates with Nostr clients over relays. On iOS, this is the
/// preferred integration method since:
/// - iOS apps can't directly launch other apps reliably
/// - Relay-based communication works across app sandboxes
/// - The client app sends encrypted bunker requests via relays
/// - The signer reads from the relay, processes, and responds
///
/// This module provides the client side of NIP-46 — connecting to a
/// relay, handling bunker requests, and responding.

use nostr::event::{Event, EventBuilder, Kind};
use nostr::key::{Keys, PublicKey};
use serde::{Deserialize, Serialize};

/// A parsed NIP-46 bunker request received from a relay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerRequest {
    /// The request ID (used to match response)
    pub id: String,
    /// Method being called (e.g., "sign_event", "get_public_key")
    pub method: String,
    /// Parameters as JSON array of strings
    pub params: Vec<String>,
}

/// A NIP-46 bunker response to send back through the relay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Connection settings for a NIP-46 bunker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerConnection {
    /// The relay URL (wss://...)
    pub relay: String,
    /// The bunker public key (hex)
    pub bunker_pubkey: String,
    /// Our secret key (nsec, stored only during session)
    pub secret: Option<String>,
}

impl BunkerRequest {
    /// Parse a NIP-46 request from an event's content field
    pub fn from_event(event: &Event) -> Result<Self, serde_json::Error> {
        serde_json::from_str(&event.content)
    }
}

impl BunkerResponse {
    /// Serialize to JSON for sending as event content
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Supported NIP-46 methods
pub const METHOD_GET_PUBLIC_KEY: &str = "get_public_key";
pub const METHOD_SIGN_EVENT: &str = "sign_event";
pub const METHOD_NIP44_ENCRYPT: &str = "nip44_encrypt";
pub const METHOD_NIP44_DECRYPT: &str = "nip44_decrypt";
pub const METHOD_CONNECT: &str = "connect";

/// Build a NIP-46 connect response event
pub fn build_connect_response(
    _bunker_pubkey: &PublicKey,
    client_pubkey: &PublicKey,
) -> Event {
    let content = serde_json::json!({
        "id": "connect",
        "result": "ack",
    })
    .to_string();

    // Returns an unsigned event — the caller must sign it
    EventBuilder::new(Kind::NostrConnect, content)
        .build(*client_pubkey)
        .sign_with_keys(&Keys::generate())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bunker_request_roundtrip() {
        let req = BunkerRequest {
            id: "req-1".into(),
            method: "sign_event".into(),
            params: vec!["param1".into(), "param2".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: BunkerRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "req-1");
        assert_eq!(parsed.method, "sign_event");
        assert_eq!(parsed.params.len(), 2);
    }

    #[test]
    fn test_bunker_response_roundtrip() {
        let resp = BunkerResponse {
            id: "req-1".into(),
            result: Some("signed-event-hex".into()),
            error: None,
        };
        let json = resp.to_json().unwrap();
        assert!(json.contains("signed-event-hex"));
    }

    #[test]
    fn test_bunker_request_from_event() {
        let content = r#"{"id":"req-1","method":"get_public_key","params":[]}"#;
        let keys = Keys::generate();
        let event = EventBuilder::new(Kind::NostrConnect, content)
            .build(keys.public_key())
            .sign_with_keys(&keys)
            .unwrap();
        let req = BunkerRequest::from_event(&event).unwrap();
        assert_eq!(req.method, "get_public_key");
    }

    #[test]
    fn test_bunker_connection_serialization() {
        let conn = BunkerConnection {
            relay: "wss://relay.nostr.example".into(),
            bunker_pubkey: "abc123".into(),
            secret: None,
        };
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("wss://relay.nostr.example"));
    }
}
