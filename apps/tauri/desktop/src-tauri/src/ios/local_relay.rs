use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;

use nostr_portable_protocol::nip46::{Nip46Request, Nip46Response, METHOD_CONNECT};
use nostr_portable_protocol::NostrSigner;

use crate::AppState;

use super::nip46::handle_bunker_request;

/// Default local relay port
pub const DEFAULT_RELAY_PORT: u16 = 48630;

/// Configuration for the local WebSocket relay
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub port: u16,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self { port: DEFAULT_RELAY_PORT }
    }
}

/// Local WebSocket relay that accepts NIP-46 bunker connections
/// from client apps on the same device.
pub struct LocalRelay {
    port: u16,
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl LocalRelay {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            port: config.port,
            running: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// Start the relay in a background thread.
    /// Blocks until the relay is bound and ready.
    pub fn start(&mut self, app_state: Arc<AppState>) -> Result<u16, String> {
        let port = self.port;
        let running = self.running.clone();
        let paused = self.paused.clone();
        let (tx_ready, rx_ready) = oneshot::channel::<u16>();

        let handle = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .build()
                .expect("failed to build tokio runtime");

            rt.block_on(async move {
                let addr: SocketAddr = ([127, 0, 0, 1], port).into();
                let listener = match TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("LocalRelay: failed to bind {}: {}", addr, e);
                        let _ = tx_ready.send(0);
                        return;
                    }
                };
                let actual_port = listener.local_addr().unwrap().port();
                let _ = tx_ready.send(actual_port);
                log::info!("LocalRelay: listening on 127.0.0.1:{}", actual_port);

                running.store(true, Ordering::SeqCst);

                while running.load(Ordering::SeqCst) {
                    if paused.load(Ordering::SeqCst) {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        continue;
                    }

                    let accept = tokio::time::timeout(
                        std::time::Duration::from_secs(1),
                        listener.accept(),
                    ).await;

                    let (stream, peer) = match accept {
                        Ok(Ok(conn)) => conn,
                        Ok(Err(e)) => {
                            log::error!("LocalRelay: accept error: {}", e);
                            continue;
                        }
                        Err(_) => continue, // timeout — recheck running flag
                    };

                    if !peer.ip().is_loopback() {
                        log::warn!("LocalRelay: rejected non-loopback connection from {}", peer);
                        continue;
                    }

                    let app_state = app_state.clone();
                    tokio::spawn(async move {
                        handle_connection(stream, peer, app_state).await;
                    });
                }

            });
        });

        // Block until the relay has bound
        let actual_port = rx_ready.blocking_recv().unwrap_or(0);
        if actual_port == 0 {
            self.running.store(false, Ordering::SeqCst);
            return Err("failed to bind relay socket".into());
        }
        self.port = actual_port;
        self.handle = Some(handle);
        Ok(actual_port)
    }

    /// Pause accepting new connections.
    /// Existing connections continue until they disconnect.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        log::info!("LocalRelay: paused (no new connections)");
    }

    /// Resume accepting new connections.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        log::info!("LocalRelay: resumed");
    }

    /// Stop the relay entirely.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        log::info!("LocalRelay: stopped");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

async fn handle_connection(stream: TcpStream, peer: SocketAddr, app_state: Arc<AppState>) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("LocalRelay: WS handshake failed from {}: {}", peer, e);
            return;
        }
    };

    log::info!("LocalRelay: WS connection from {}", peer);

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(Message::Text(t)) => t,
            Ok(Message::Close(_)) | Err(_) => break,
            _ => continue,
        };

        let request: Nip46Request = match serde_json::from_str(&msg) {
            Ok(req) => req,
            Err(e) => {
                let resp = Nip46Response {
                    id: "error".into(),
                    result: None,
                    error: Some(format!("invalid request: {}", e)),
                };
                let _ = write
                    .send(Message::Text(serde_json::to_string(&resp).unwrap()))
                    .await;
                continue;
            }
        };

        let response = process_request(request, &app_state).await;

        if let Ok(json) = serde_json::to_string(&response) {
            let _ = write.send(Message::Text(json)).await;
        }
    }

    log::info!("LocalRelay: WS connection closed from {}", peer);
}

async fn process_request(request: Nip46Request, app_state: &Arc<AppState>) -> Nip46Response {
    let req_id = request.id.clone();

    if request.method == METHOD_CONNECT {
        let client_pubkey = request.params.first().cloned().unwrap_or_default();
        let pk = nostr::key::PublicKey::parse(&client_pubkey);

        let signer = app_state.signer.lock().unwrap();
        match (pk, signer.get_public_key()) {
            (Ok(_client_pk), Ok(signer_pk)) => {
                let ack = super::nip46::build_connect_response(&signer_pk);
                drop(signer);
                Nip46Response {
                    id: req_id,
                    result: Some(ack),
                    error: None,
                }
            }
            (Err(e), _) => Nip46Response {
                id: req_id,
                result: None,
                error: Some(format!("invalid client pubkey: {}", e)),
            },
            (_, Err(e)) => Nip46Response {
                id: req_id,
                result: None,
                error: Some(format!("signer locked: {}", e)),
            },
        }
    } else {
        let signer = app_state.signer.lock().unwrap();
        handle_bunker_request(&signer, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use nostr_portable_permissions::PermissionStore;
    use nostr_portable_signer_core::SignerService;
    use nostr_portable_vault as vault;
    use nostr_portable_vault::providers::local_file::LocalFileVaultProvider;

    fn create_test_app_state() -> Arc<AppState> {
        let state = AppState::new();
        Arc::new(state)
    }

    fn create_unlocked_state() -> Arc<AppState> {
        use tempfile::TempDir;
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
        std::mem::forget(tmp);
        let state = AppState::new();
        *state.signer.lock().unwrap() = service;
        Arc::new(state)
    }

    #[test]
    fn test_relay_start_stop() {
        let app_state = create_test_app_state();
        let mut relay = LocalRelay::new(RelayConfig { port: 0 });
        assert!(!relay.is_running());

        let _port = relay.start(app_state).unwrap();
        assert!(relay.is_running());

        relay.stop();
        assert!(!relay.is_running());
    }

    #[test]
    fn test_relay_pause_resume() {
        let app_state = create_test_app_state();
        let mut relay = LocalRelay::new(RelayConfig { port: 0 });
        let _port = relay.start(app_state).unwrap();

        // Give the relay a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(relay.is_running());

        relay.pause();
        assert!(relay.is_paused());

        relay.resume();
        assert!(!relay.is_paused());

        relay.stop();
    }

    #[test]
    fn test_get_public_key_requires_unlock() {
        let app_state = create_test_app_state();
        let request = Nip46Request {
            id: "req-1".into(),
            method: "get_public_key".into(),
            params: vec![],
        };

        let signer = app_state.signer.lock().unwrap();
        let response = handle_bunker_request(&signer, request);
        drop(signer);

        assert!(response.error.is_some());
        assert!(response.error.unwrap().contains("locked"));
    }
}
