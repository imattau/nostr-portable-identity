use tauri::{AppHandle, Emitter, State};

use nostr::event::EventBuilder;
use nostr_portable_permissions::{ClientIdentity, PermissionEntry, PermissionRule, PermissionStore};
use nostr_portable_protocol::{ApprovalRequest, NostrSigner, SignEventRequest};
use nostr_portable_signer_core::{PermissionCheck, SignerService};
use nostr_portable_vault as vault;
use nostr_portable_vault::providers::local_file::LocalFileVaultProvider;
use nostr_portable_vault::{UsbFileVaultProvider, VaultProvider};

use crate::AppState;

fn lock_state(state: &AppState) -> Result<std::sync::MutexGuard<'_, SignerService>, String> {
    state.signer.lock().map_err(|_| "signer lock poisoned".to_string())
}

fn lock_pending(state: &AppState) -> Result<std::sync::MutexGuard<'_, Option<ApprovalRequest>>, String> {
    state.pending_approval.lock().map_err(|_| "pending approval lock poisoned".to_string())
}

#[derive(serde::Serialize)]
pub struct SignerStatusResponse {
    pub state: String,
    pub vault_present: bool,
    pub public_key: Option<String>,
    pub unlocked_at: Option<u64>,
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> SignerStatusResponse {
    let signer = state.signer.lock()
        .expect("signer lock poisoned");
    let status = signer.status();
    SignerStatusResponse {
        state: status.state,
        vault_present: status.vault_present,
        public_key: status.public_key,
        unlocked_at: status.unlocked_at,
    }
}

#[tauri::command]
pub fn get_public_key(state: State<'_, AppState>) -> Result<String, String> {
    let signer = lock_state(&state)?;
    signer
        .get_public_key()
        .map(|pk| pk.to_hex())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unlock_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    passphrase: String,
    _timeout: u64,
) -> Result<(), String> {
    let provider: Box<dyn VaultProvider> = Box::new(UsbFileVaultProvider::new(&path));
    {
        let mut signer = lock_state(&state)?;
        signer.set_provider(provider);
    }
    {
        let mut signer = lock_state(&state)?;
        signer.unlock(&passphrase).map_err(|e| e.to_string())?;
    }
    let _ = app.emit("unlocked", ());
    Ok(())
}

#[tauri::command]
pub fn lock_signer(state: State<'_, AppState>) -> Result<(), String> {
    let mut signer = lock_state(&state)?;
    SignerService::lock(&mut *signer).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sign_text_note(
    app: AppHandle,
    state: State<'_, AppState>,
    content: String,
) -> Result<String, String> {
    let identity = ClientIdentity::Unknown("tauri-app".into());

    {
        let signer = lock_state(&state)?;
        match signer.evaluate_permission(
            &identity,
            "signEvent",
            Some(nostr::event::Kind::TextNote),
        ) {
            PermissionCheck::Allowed => {}
            PermissionCheck::Denied(reason) => return Err(reason),
            PermissionCheck::Ask(request) => {
                let json = serde_json::to_string(&request)
                    .map_err(|e| format!("serialization error: {}", e))?;
                *lock_pending(&state)? = Some(request.clone());
                let _ = app.emit("approval-request", json);
                return Err("approval_required".into());
            }
        }
    }

    let (unsigned, kind) = {
        let signer = lock_state(&state)?;
        let pk = signer.get_public_key().map_err(|e| e.to_string())?;
        let unsigned = EventBuilder::text_note(content.clone()).build(pk);
        (unsigned, nostr::event::Kind::TextNote)
    };

    let event = {
        let signer = lock_state(&state)?;
        let request = SignEventRequest {
            event: unsigned,
            kind,
            content: content.clone(),
            tags: vec![],
        };
        signer.sign_event(request).map_err(|e| e.to_string())?
    };

    let _ = app.emit("event-signed", event.id.to_hex());
    serde_json::to_string_pretty(&event)
        .map_err(|e| format!("serialization error: {}", e))
}

#[tauri::command]
pub fn create_vault(
    path: String,
    name: String,
    passphrase: String,
    nsec: Option<String>,
) -> Result<String, String> {
    let provider = UsbFileVaultProvider::new(&path);
    let keys = match nsec {
        Some(nsec) => nostr_portable_crypto::parse_keys(&nsec).map_err(|e| e.to_string())?,
        None => nostr_portable_crypto::generate_keys(),
    };
    let vault = vault::create_vault(&provider, name, &keys, &passphrase)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&vault).map_err(|e| format!("serialization error: {}", e))
}

#[tauri::command]
pub fn vault_info(path: String) -> Result<String, String> {
    let provider = UsbFileVaultProvider::new(&path);
    let vault = provider.load_encrypted_vault().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&vault).map_err(|e| format!("serialization error: {}", e))
}

#[tauri::command]
pub fn get_pending_approval(state: State<'_, AppState>) -> Option<String> {
    let pending = state.pending_approval.lock()
        .expect("pending approval lock poisoned");
    pending.as_ref().and_then(|r| serde_json::to_string(r).ok())
}

#[tauri::command]
pub fn submit_approval(state: State<'_, AppState>, approved: bool) -> Result<(), String> {
    let mut pending = lock_pending(&state)?;
    if let Some(request) = pending.take() {
        if approved {
            let identity = ClientIdentity::Unknown("tauri-app".into());
            let mut permission_store = PermissionStore::new();
            permission_store.set_permission(
                identity.clone(),
                PermissionEntry {
                    method: request.method.clone(),
                    rule: PermissionRule::Allow,
                    kind_restriction: None,
                },
            );
            let mut signer = lock_state(&state)?;
            *signer.permissions_mut() = permission_store;
        }
        Ok(())
    } else {
        Err("no pending approval request".into())
    }
}

#[derive(serde::Serialize)]
pub struct IdentityInfo {
    pub name: String,
    pub provider_type: String,
    pub available: bool,
}

#[tauri::command]
pub fn create_local_vault(
    name: String,
    passphrase: String,
    nsec: Option<String>,
) -> Result<String, String> {
    let mut provider = LocalFileVaultProvider::new().map_err(|e| e.to_string())?;
    let keys = match nsec {
        Some(nsec) => nostr_portable_crypto::parse_keys(&nsec).map_err(|e| e.to_string())?,
        None => nostr_portable_crypto::generate_keys(),
    };
    let vault = provider.create(&name, &keys, &passphrase)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&vault).map_err(|e| format!("serialization error: {}", e))
}

#[tauri::command]
pub fn list_local_vaults() -> Result<Vec<IdentityInfo>, String> {
    let provider = LocalFileVaultProvider::new().map_err(|e| e.to_string())?;
    let names = provider.list_vaults().map_err(|e| e.to_string())?;
    let mut vaults = Vec::new();
    for name in &names {
        let active = provider.name();
        vaults.push(IdentityInfo {
            name: name.clone(),
            provider_type: "local".into(),
            available: name == active,
        });
    }
    Ok(vaults)
}

#[tauri::command]
pub fn switch_local_identity(name: String) -> Result<(), String> {
    let mut provider = LocalFileVaultProvider::new().map_err(|e| e.to_string())?;
    provider.set_active(&name);
    Ok(())
}
