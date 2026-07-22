/// iOS NIP-46 support using the protocol crate's shared types.
///
/// NIP-46 defines a remote signing protocol where a signer communicates
/// with Nostr clients over relays. On iOS, this is the preferred
/// integration method since apps can't directly launch other apps reliably.
///
/// This module re-exports and extends the protocol crate's NIP-46 types
/// with iOS-specific helpers.



use nostr::event::{Event, EventBuilder, Kind};
use nostr::key::Keys;

/// Connection settings for a NIP-46 bunker
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BunkerConnection {
    pub relay: String,
    pub bunker_pubkey: String,
    pub secret: Option<String>,
}

impl BunkerConnection {
    pub fn new(relay: &str, bunker_pubkey: &str) -> Self {
        Self {
            relay: relay.to_string(),
            bunker_pubkey: bunker_pubkey.to_string(),
            secret: None,
        }
    }
}

/// Build a NIP-46 connect response event.
///
/// The caller must provide the actual signer keys so the response
/// can be verified by the client.
pub fn build_connect_response(signer_keys: &Keys, client_pubkey: &nostr::key::PublicKey) -> Event {
    let content = serde_json::json!({
        "id": "connect",
        "result": "ack",
    })
    .to_string();

    EventBuilder::new(Kind::NostrConnect, content)
        .build(*client_pubkey)
        .sign_with_keys(signer_keys)
        .expect("failed to sign NIP-46 connect response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bunker_connection() {
        let conn = BunkerConnection::new("wss://relay.nostr.example", "abc123");
        assert_eq!(conn.relay, "wss://relay.nostr.example");
        assert_eq!(conn.bunker_pubkey, "abc123");
        assert!(conn.secret.is_none());
    }

    #[test]
    fn test_bunker_request_from_json() {
        use nostr_portable_protocol::nip46::Nip46Request;
        let req = Nip46Request::from_json(r#"{"id":"req-1","method":"get_public_key","params":[]}"#).unwrap();
        assert_eq!(req.method, "get_public_key");
    }

    #[test]
    fn test_method_constants() {
        assert_eq!(
            nostr_portable_protocol::nip46::METHOD_SIGN_EVENT,
            "sign_event"
        );
        assert_eq!(
            nostr_portable_protocol::nip46::METHOD_NIP44_ENCRYPT,
            "nip44_encrypt"
        );
    }
}
