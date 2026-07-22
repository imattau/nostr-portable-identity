mod android;
mod commands;
mod ios;

#[cfg(desktop)]
mod ipc_server;

use std::sync::Mutex;
use std::time::Duration;

use nostr_portable_permissions::PermissionStore;
use nostr_portable_protocol::ApprovalRequest;
use nostr_portable_signer_core::SignerService;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use commands::*;

pub struct AppState {
    pub signer: Mutex<SignerService>,
    pub pending_approval: Mutex<Option<ApprovalRequest>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            signer: Mutex::new(SignerService::new(None, PermissionStore::new(), Duration::from_secs(300))),
            pending_approval: Mutex::new(None),
        }
    }
}

fn open_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Settings - Nostr Portable Identity")
        .inner_size(500.0, 600.0)
        .resizable(true)
        .build();
}

fn open_unlock_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Nostr Portable Identity")
        .inner_size(450.0, 500.0)
        .resizable(false)
        .build();
}

#[cfg(desktop)]
fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show = tauri::menu::MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let settings = tauri::menu::MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = tauri::menu::Menu::with_items(app, &[&show, &settings, &quit])?;

    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        tauri::image::Image::new(&[], 0, 0)
    });
    let _tray = TrayIconBuilder::new()
        .tooltip("Nostr Portable Identity")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => open_unlock_window(app),
            "settings" => open_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                open_unlock_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new());

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                Some(vec!["--autostart"]),
            ))
            .plugin(tauri_plugin_updater::Builder::default().build());
    }

    builder = builder
        .setup(|app| {
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                setup_tray(app.handle())?;
                ipc_server::start_ipc_server(handle);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_public_key,
            unlock_vault,
            lock_signer,
            sign_text_note,
            create_vault,
            vault_info,
            get_pending_approval,
            submit_approval,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
