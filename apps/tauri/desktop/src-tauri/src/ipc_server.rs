use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::thread;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(windows)]
use std::net::{TcpListener, TcpStream};

use tauri::{AppHandle, Manager};

use nostr::event::UnsignedEvent;
use nostr_portable_protocol as protocol;
use nostr_portable_protocol::NostrSigner;

use crate::AppState;

const IPC_PORT: u16 = 48631;

fn get_ipc_path() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .expect("HOME must be set to run the IPC server");
    let dir = PathBuf::from(&home).join(".nostr-portable-identity");
    fs::create_dir_all(&dir).expect("failed to create IPC directory");
    #[cfg(unix)]
    { dir.join("ipc.sock") }
    #[cfg(windows)]
    { dir.join("ipc.txt") } // Used as a marker file on Windows
}

/// Cross-platform listener: Unix socket on Linux/macOS, TCP on Windows
enum Listener {
    #[cfg(unix)]
    Unix(UnixListener),
    #[cfg(windows)]
    Tcp(TcpListener),
}

impl Listener {
    fn accept(&self) -> Result<Stream, String> {
        match self {
            #[cfg(unix)]
            Listener::Unix(l) => l.accept()
                .map(|(s, _)| Stream::Unix(s))
                .map_err(|e| format!("accept: {}", e)),
            #[cfg(windows)]
            Listener::Tcp(l) => l.accept()
                .map(|(s, _)| Stream::Tcp(s))
                .map_err(|e| format!("accept: {}", e)),
        }
    }
}

enum Stream {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    Tcp(TcpStream),
}

impl std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)] Stream::Unix(s) => s.read(buf),
            #[cfg(windows)] Stream::Tcp(s) => s.read(buf),
        }
    }
}

impl std::io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)] Stream::Unix(s) => s.write(buf),
            #[cfg(windows)] Stream::Tcp(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)] Stream::Unix(s) => s.flush(),
            #[cfg(windows)] Stream::Tcp(s) => s.flush(),
        }
    }
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
                None => return IpcResponse { id, result: None, error: Some("missing 'event' parameter".into()) },
            };

            let unsigned: UnsignedEvent = match serde_json::from_value(event_val) {
                Ok(e) => e,
                Err(e) => return IpcResponse { id, result: None, error: Some(format!("invalid event: {}", e)) },
            };

            let kind = req.params.get("kind")
                .and_then(|k| k.as_u64())
                .map(|k| nostr::event::Kind::from(k as u16))
                .unwrap_or(nostr::event::Kind::TextNote);
            let content = req.params.get("content")
                .and_then(|c| c.as_str()).unwrap_or("").to_string();
            let tags: Vec<Vec<String>> = req.params.get("tags")
                .and_then(|t| serde_json::from_value(t.clone()).ok()).unwrap_or_default();

            let signer = state.signer.lock().unwrap();
            match signer.sign_event(protocol::SignEventRequest { event: unsigned, kind, content, tags }) {
                Ok(event) => IpcResponse { id, result: Some(serde_json::to_value(&event).unwrap()), error: None },
                Err(e) => IpcResponse { id, result: None, error: Some(e.to_string()) },
            }
        }

        "nip44Encrypt" => {
            let pubkey = req.params.get("pubkey").and_then(|p| p.as_str()).unwrap_or("").to_string();
            let plaintext = req.params.get("plaintext").and_then(|p| p.as_str()).unwrap_or("").to_string();
            let signer = state.signer.lock().unwrap();
            match signer.nip44_encrypt(protocol::Nip44EncryptRequest { recipient_pubkey: pubkey, plaintext }) {
                Ok(result) => IpcResponse { id, result: Some(serde_json::json!({ "ciphertext": result })), error: None },
                Err(e) => IpcResponse { id, result: None, error: Some(e.to_string()) },
            }
        }

        "nip44Decrypt" => {
            let pubkey = req.params.get("pubkey").and_then(|p| p.as_str()).unwrap_or("").to_string();
            let ciphertext = req.params.get("ciphertext").and_then(|c| c.as_str()).unwrap_or("").to_string();
            let signer = state.signer.lock().unwrap();
            match signer.nip44_decrypt(protocol::Nip44DecryptRequest { sender_pubkey: pubkey, ciphertext }) {
                Ok(result) => IpcResponse { id, result: Some(serde_json::json!({ "plaintext": result })), error: None },
                Err(e) => IpcResponse { id, result: None, error: Some(e.to_string()) },
            }
        }

        "getStatus" => {
            let signer = state.signer.lock().unwrap();
            let status = signer.status();
            IpcResponse { id, result: Some(serde_json::to_value(&status).unwrap()), error: None }
        }

        _ => IpcResponse { id, result: None, error: Some(format!("unknown method: {}", req.method)) },
    }
}

fn handle_connection(app: AppHandle, mut stream: Stream) {
    let mut line = String::new();
    let mut reader = BufReader::new(&mut stream);
    match reader.read_line(&mut line) {
        Ok(0) => return,
        Ok(_) => {}
        Err(e) => {
            let resp = IpcResponse { id: None, result: None, error: Some(format!("read error: {}", e)) };
            let _ = writeln!(&mut stream, "{}", serde_json::to_string(&resp).unwrap());
            return;
        }
    }

    let request: IpcRequest = match serde_json::from_str(&line.trim()) {
        Ok(req) => req,
        Err(e) => {
            let resp = IpcResponse { id: None, result: None, error: Some(format!("invalid JSON: {}", e)) };
            let _ = writeln!(&mut stream, "{}", serde_json::to_string(&resp).unwrap());
            return;
        }
    };

    let response = handle_request(&app, request);
    let json = serde_json::to_string(&response).unwrap();
    let _ = writeln!(&mut stream, "{}", json);
}

pub fn start_ipc_server(app: AppHandle) {
    let listener = create_listener();
    let listener = match listener {
        Ok(l) => l,
        Err(e) => { eprintln!("Failed to start IPC server: {}", e); return; }
    };

    thread::spawn(move || {
        loop {
            match listener.accept() {
                Ok(stream) => { let app = app.clone(); thread::spawn(move || { handle_connection(app, stream); }); }
                Err(e) => log::error!("IPC connection error: {}", e),
            }
        }
    });
}

fn create_listener() -> Result<Listener, String> {
    #[cfg(unix)] {
        let path = get_ipc_path();
        let _ = fs::remove_file(&path);
        let listener = UnixListener::bind(&path).map_err(|e| format!("bind: {}", e))?;
        #[cfg(unix)]
        fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .map_err(|e| format!("chmod: {}", e))?;
        log::info!("IPC: Unix socket at {:?}", path);
        Ok(Listener::Unix(listener))
    }
    #[cfg(windows)] {
        let addr = format!("127.0.0.1:{}", IPC_PORT);
        let listener = TcpListener::bind(&addr).map_err(|e| format!("bind TCP {}: {}", addr, e))?;
        log::info!("IPC: TCP on {}", addr);
        Ok(Listener::Tcp(listener))
    }
}
