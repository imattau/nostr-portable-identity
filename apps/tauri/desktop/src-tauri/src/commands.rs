use tauri::{AppHandle, Emitter, State};

use nostr::event::EventBuilder;
use nostr_portable_permissions::{ClientIdentity, PermissionEntry, PermissionRule, PermissionStore};
use nostr_portable_protocol::{NostrSigner, SignEventRequest};
use nostr_portable_signer_core::{PermissionCheck, SignerService};
use nostr_portable_vault as vault;
use nostr_portable_vault::{UsbFileVaultProvider, VaultProvider};

use crate::AppState;

#[derive(serde::Serialize)]
pub struct SignerStatusResponse {
    pub state: String,
    pub vault_present: bool,
    pub public_key: Option<String>,
    pub unlocked_at: Option<u64>,
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> SignerStatusResponse {
    let signer = state.signer.lock().unwrap();
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
    let signer = state.signer.lock().unwrap();
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
        let mut signer = state.signer.lock().unwrap();
        signer.set_provider(provider);
    }
    {
        let mut signer = state.signer.lock().unwrap();
        signer.unlock(&passphrase).map_err(|e| e.to_string())?;
    }
    let _ = app.emit("unlocked", ());
    Ok(())
}

#[tauri::command]
pub fn lock_signer(state: State<'_, AppState>) -> Result<(), String> {
    let mut signer = state.signer.lock().unwrap();
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
        let signer = state.signer.lock().unwrap();
        match signer.evaluate_permission(
            &identity,
            "signEvent",
            Some(nostr::event::Kind::TextNote),
        ) {
            PermissionCheck::Allowed => {}
            PermissionCheck::Denied(reason) => return Err(reason),
            PermissionCheck::Ask(request) => {
                *state.pending_approval.lock().unwrap() = Some(request.clone());
                let _ = app.emit(
                    "approval-request",
                    serde_json::to_string(&request).unwrap(),
                );
                return Err("approval_required".into());
            }
        }
    }

    let (unsigned, kind) = {
        let signer = state.signer.lock().unwrap();
        let pk = signer.get_public_key().map_err(|e| e.to_string())?;
        let unsigned = EventBuilder::text_note(content.clone()).build(pk);
        let k = nostr::event::Kind::TextNote;
        (unsigned, k)
    };

    let event = {
        let signer = state.signer.lock().unwrap();
        let request = SignEventRequest {
            event: unsigned,
            kind,
            content: content.clone(),
            tags: vec![],
        };
        signer.sign_event(request).map_err(|e| e.to_string())?
    };

    let _ = app.emit("event-signed", event.id.to_hex());
    Ok(serde_json::to_string_pretty(&event).unwrap())
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
    Ok(serde_json::to_string(&vault).unwrap())
}

#[tauri::command]
pub fn vault_info(path: String) -> Result<String, String> {
    let provider = UsbFileVaultProvider::new(&path);
    let vault = provider.load_encrypted_vault().map_err(|e| e.to_string())?;
    Ok(serde_json::to_string_pretty(&vault).unwrap())
}

#[tauri::command]
pub fn get_pending_approval(state: State<'_, AppState>) -> Option<String> {
    let pending = state.pending_approval.lock().unwrap();
    pending.as_ref().map(|r| serde_json::to_string(r).unwrap())
}

#[tauri::command]
pub fn submit_approval(state: State<'_, AppState>, approved: bool) -> Result<(), String> {
    let mut pending = state.pending_approval.lock().unwrap();
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
            let mut signer = state.signer.lock().unwrap();
            *signer.permissions_mut() = permission_store;
        }
        Ok(())
    } else {
        Err("no pending approval request".into())
    }
}
