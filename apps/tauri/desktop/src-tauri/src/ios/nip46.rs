use nostr::key::PublicKey;
use nostr_portable_protocol::nip46::{
    Nip46Request, Nip46Response, METHOD_GET_PUBLIC_KEY, METHOD_SIGN_EVENT,
    METHOD_NIP44_ENCRYPT, METHOD_NIP44_DECRYPT,
};
use nostr_portable_protocol::{Nip44EncryptRequest, Nip44DecryptRequest, SignEventRequest, NostrSigner};
use nostr_portable_signer_core::SignerService;

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

/// Build a NIP-46 connect response.
/// Returns JSON with the signer's public key.
pub fn build_connect_response(signer_pk: &PublicKey) -> String {
    serde_json::json!({
        "id": "connect",
        "result": format!("ack:{}", signer_pk.to_hex()),
    })
    .to_string()
}

/// Dispatch a NIP-46 bunker request to the signer service.
/// Returns the response without going through the approval system
/// (approval is handled by the caller or the signer's own checks).
pub fn handle_bunker_request(signer: &SignerService, request: Nip46Request) -> Nip46Response {
    let id = request.id.clone();

    match request.method.as_str() {
        METHOD_GET_PUBLIC_KEY => match signer.get_public_key() {
            Ok(pk) => Nip46Response {
                id,
                result: Some(pk.to_hex()),
                error: None,
            },
            Err(e) => Nip46Response {
                id,
                result: None,
                error: Some(e.to_string()),
            },
        },

        METHOD_SIGN_EVENT => {
            let event_json = request.params.first().cloned().unwrap_or_default();
            let unsigned: nostr::event::UnsignedEvent = match serde_json::from_str(&event_json) {
                Ok(e) => e,
                Err(e) => {
                    return Nip46Response {
                        id,
                        result: None,
                        error: Some(format!("invalid event JSON: {}", e)),
                    }
                }
            };
            let content = unsigned.content.clone();
            let kind = unsigned.kind;
            let req = SignEventRequest {
                event: unsigned,
                kind,
                content,
                tags: vec![],
            };
            match signer.sign_event(req) {
                Ok(event) => Nip46Response {
                    id,
                    result: Some(serde_json::to_string(&event).unwrap_or_default()),
                    error: None,
                },
                Err(e) => Nip46Response {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        METHOD_NIP44_ENCRYPT => {
            let params: Vec<&str> = request.params.iter().map(|s| s.as_str()).collect();
            if params.len() < 2 {
                return Nip46Response {
                    id,
                    result: None,
                    error: Some("need pubkey and plaintext".into()),
                };
            }
            let req = Nip44EncryptRequest {
                recipient_pubkey: params[0].to_string(),
                plaintext: params[1].to_string(),
            };
            match signer.nip44_encrypt(req) {
                Ok(ciphertext) => Nip46Response {
                    id,
                    result: Some(ciphertext),
                    error: None,
                },
                Err(e) => Nip46Response {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        METHOD_NIP44_DECRYPT => {
            let params: Vec<&str> = request.params.iter().map(|s| s.as_str()).collect();
            if params.len() < 2 {
                return Nip46Response {
                    id,
                    result: None,
                    error: Some("need pubkey and ciphertext".into()),
                };
            }
            let req = Nip44DecryptRequest {
                sender_pubkey: params[0].to_string(),
                ciphertext: params[1].to_string(),
            };
            match signer.nip44_decrypt(req) {
                Ok(plaintext) => Nip46Response {
                    id,
                    result: Some(plaintext),
                    error: None,
                },
                Err(e) => Nip46Response {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        _ => Nip46Response {
            id,
            result: None,
            error: Some(format!("unknown method: {}", request.method)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use nostr_portable_permissions::PermissionStore;
    use nostr_portable_vault as vault;
    use nostr_portable_vault::providers::local_file::LocalFileVaultProvider;
    use tempfile::TempDir;

    fn unlocked_signer() -> SignerService {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        std::env::set_var("HOME", home.to_str().unwrap());
        let mut vault_provider = LocalFileVaultProvider::new().unwrap();
        let keys = nostr_portable_crypto::Keys::generate();
        vault_provider.create("test", &keys, "pass").unwrap();

        let mut service = SignerService::new(
            Some(Box::new(vault_provider)),
            PermissionStore::new(),
            Duration::from_secs(300),
        );
        service.unlock("pass").unwrap();
        // Leak TempDir to keep vault files alive for the test
        std::mem::forget(tmp);
        service
    }

    #[test]
    fn test_get_public_key() {
        let signer = unlocked_signer();
        let req = Nip46Request {
            id: "r1".into(),
            method: METHOD_GET_PUBLIC_KEY.into(),
            params: vec![],
        };
        let resp = handle_bunker_request(&signer, req);
        assert!(resp.error.is_none());
        assert!(resp.result.unwrap().len() == 64); // hex pubkey
    }

    #[test]
    fn test_sign_event() {
        let signer = unlocked_signer();
        let pk = signer.get_public_key().unwrap();
        let unsigned = nostr::event::EventBuilder::text_note("hello from nip46")
            .build(pk);
        let event_json = serde_json::to_string(&unsigned).unwrap();

        let req = Nip46Request {
            id: "r2".into(),
            method: METHOD_SIGN_EVENT.into(),
            params: vec![event_json],
        };
        let resp = handle_bunker_request(&signer, req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_unknown_method() {
        let signer = unlocked_signer();
        let req = Nip46Request {
            id: "r3".into(),
            method: "unknown_method".into(),
            params: vec![],
        };
        let resp = handle_bunker_request(&signer, req);
        assert!(resp.error.unwrap().contains("unknown method"));
    }

    #[test]
    fn test_bunker_connection() {
        let conn = BunkerConnection::new("ws://127.0.0.1:48630", "abc123");
        assert_eq!(conn.relay, "ws://127.0.0.1:48630");
        assert_eq!(conn.bunker_pubkey, "abc123");
    }

    #[test]
    fn test_method_constants() {
        assert_eq!(METHOD_SIGN_EVENT, "sign_event");
        assert_eq!(METHOD_NIP44_ENCRYPT, "nip44_encrypt");
    }
}
