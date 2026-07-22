use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use nostr::event::UnsignedEvent;
use nostr_portable_protocol as protocol;
use nostr_portable_protocol::NostrSigner;

use crate::AppState;

fn get_socket_path() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("HOME must be set to run the IPC server");
    let dir = PathBuf::from(&home).join(".nostr-portable-identity");
    fs::create_dir_all(&dir).expect("failed to create IPC directory");
    dir.join("ipc.sock")
}

#[derive(serde::Deserialize)]
struct IpcRequest {
    #[serde(default)]
    id: String,
    method: String,
    #[serde(default)]
    origin: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(serde::Serialize)]
struct IpcResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn handle_request(app: &AppHandle, req: IpcRequest) -> IpcResponse {
    let id = Some(req.id);
    let state = app.state::<AppState>();

    let _identity: nostr_portable_permissions::ClientIdentity = if req.origin.is_empty() || req.origin == "unknown" {
        nostr_portable_permissions::ClientIdentity::Unknown("ipc".into())
    } else {
        nostr_portable_permissions::ClientIdentity::BrowserOrigin(req.origin.clone())
    };

    match req.method.as_str() {
        "getPublicKey" => {
            let signer = state.signer.lock().unwrap();
            match signer.get_public_key() {
                Ok(pk) => IpcResponse {
                    id,
                    result: Some(serde_json::json!({ "pubkey": pk.to_hex() })),
                    error: None,
                },
                Err(e) => IpcResponse {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        "signEvent" => {
            let event_val = match req.params.get("event") {
                Some(v) => v.clone(),
                None => {
                    return IpcResponse {
                        id,
                        result: None,
                        error: Some("missing 'event' parameter".into()),
                    }
                }
            };

            let unsigned: UnsignedEvent = match serde_json::from_value(event_val) {
                Ok(e) => e,
                Err(e) => {
                    return IpcResponse {
                        id,
                        result: None,
                        error: Some(format!("invalid event: {}", e)),
                    }
                }
            };

            let kind = req
                .params
                .get("kind")
                .and_then(|k| k.as_u64())
                .map(|k| nostr::event::Kind::from(k as u16))
                .unwrap_or(nostr::event::Kind::TextNote);

            let content = req
                .params
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            let tags: Vec<Vec<String>> = req
                .params
                .get("tags")
                .and_then(|t| serde_json::from_value(t.clone()).ok())
                .unwrap_or_default();

            let signer = state.signer.lock().unwrap();
            match signer.sign_event(protocol::SignEventRequest {
                event: unsigned,
                kind,
                content,
                tags,
            }) {
                Ok(event) => IpcResponse {
                    id,
                    result: Some(serde_json::to_value(&event).unwrap()),
                    error: None,
                },
                Err(e) => IpcResponse {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        "nip44Encrypt" => {
            let pubkey = match req.params.get("pubkey").and_then(|p| p.as_str()) {
                Some(pk) => pk.to_string(),
                None => {
                    return IpcResponse {
                        id,
                        result: None,
                        error: Some("missing 'pubkey' parameter".into()),
                    }
                }
            };
            let plaintext = match req.params.get("plaintext").and_then(|p| p.as_str()) {
                Some(pt) => pt.to_string(),
                None => {
                    return IpcResponse {
                        id,
                        result: None,
                        error: Some("missing 'plaintext' parameter".into()),
                    }
                }
            };

            let signer = state.signer.lock().unwrap();
            match signer.nip44_encrypt(protocol::Nip44EncryptRequest {
                recipient_pubkey: pubkey,
                plaintext,
            }) {
                Ok(result) => IpcResponse {
                    id,
                    result: Some(serde_json::json!({ "ciphertext": result })),
                    error: None,
                },
                Err(e) => IpcResponse {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        "nip44Decrypt" => {
            let pubkey = match req.params.get("pubkey").and_then(|p| p.as_str()) {
                Some(pk) => pk.to_string(),
                None => {
                    return IpcResponse {
                        id,
                        result: None,
                        error: Some("missing 'pubkey' parameter".into()),
                    }
                }
            };
            let ciphertext = match req.params.get("ciphertext").and_then(|c| c.as_str()) {
                Some(ct) => ct.to_string(),
                None => {
                    return IpcResponse {
                        id,
                        result: None,
                        error: Some("missing 'ciphertext' parameter".into()),
                    }
                }
            };

            let signer = state.signer.lock().unwrap();
            match signer.nip44_decrypt(protocol::Nip44DecryptRequest {
                sender_pubkey: pubkey,
                ciphertext,
            }) {
                Ok(result) => IpcResponse {
                    id,
                    result: Some(serde_json::json!({ "plaintext": result })),
                    error: None,
                },
                Err(e) => IpcResponse {
                    id,
                    result: None,
                    error: Some(e.to_string()),
                },
            }
        }

        "getStatus" => {
            let signer = state.signer.lock().unwrap();
            let status = signer.status();
            IpcResponse {
                id,
                result: Some(serde_json::to_value(&status).unwrap()),
                error: None,
            }
        }

        _ => IpcResponse {
            id,
            result: None,
            error: Some(format!("unknown method: {}", req.method)),
        },
    }
}

fn handle_connection(app: AppHandle, mut stream: UnixStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));

    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => return,
        Ok(_) => {}
        Err(e) => {
            let resp = IpcResponse {
                id: None,
                result: None,
                error: Some(format!("read error: {}", e)),
            };
            let _ = writeln!(&mut stream, "{}", serde_json::to_string(&resp).unwrap());
            return;
        }
    }

    let request: IpcRequest = match serde_json::from_str(&line.trim()) {
        Ok(req) => req,
        Err(e) => {
            let resp = IpcResponse {
                id: None,
                result: None,
                error: Some(format!("invalid JSON: {}", e)),
            };
            let _ = writeln!(&mut stream, "{}", serde_json::to_string(&resp).unwrap());
            return;
        }
    };

    let response = handle_request(&app, request);
    let json = serde_json::to_string(&response).unwrap();
    let _ = writeln!(&mut stream, "{}", json);
}

pub fn start_ipc_server(app: AppHandle) {
    let socket_path = get_socket_path();

    let _ = fs::remove_file(&socket_path);

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind IPC socket at {:?}: {}", socket_path, e);
            return;
        }
    };

    let _ = fs::set_permissions(&socket_path, std::os::unix::fs::PermissionsExt::from_mode(0o600));

    log::info!("IPC server listening on {:?}", socket_path);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let app = app.clone();
                    thread::spawn(move || {
                        handle_connection(app, stream);
                    });
                }
                Err(e) => {
                    log::error!("IPC connection error: {}", e);
                }
            }
        }
    });
}
