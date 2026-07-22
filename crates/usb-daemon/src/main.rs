use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::{Parser, Subcommand};
use nostr_portable_permissions::PermissionStore;
use nostr_portable_signer_core::SignerService;
use nostr_portable_vault::UsbFileVaultProvider;

#[derive(Parser)]
#[command(
    name = "nostr-portable-usb-daemon",
    about = "Runs the IPC server from a USB vault",
    long_about = "Runs a headless signer daemon from a Nostr Portable Identity USB drive.\n\n\
When run directly (no arguments), it auto-detects the USB vault and starts the IPC\n\
server so browser extensions can connect via the native messaging host.\n\n\
Use 'install' once to enable auto-start when the USB is plugged in."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, default_value = "300", global = true)]
    timeout: u64,
}

#[derive(Subcommand)]
enum Command {
    /// Run the daemon (auto-detects USB vault)
    Run,
    /// Enable auto-start when this USB drive is plugged in
    Install,
    /// Disable auto-start
    Uninstall,
}

const VAULT_FILE: &str = "NOSTR-SIGNER/nostr-vault.json";

fn vault_exists(path: &str) -> bool {
    let vault_path = PathBuf::from(path).join(VAULT_FILE);
    vault_path.exists()
}

fn find_vaults() -> Vec<String> {
    let mut found = Vec::new();

    #[cfg(target_os = "windows")]
    {
        for letter in 'D'..='Z' {
            let root = format!("{}:\\", letter);
            if vault_exists(&root) {
                found.push(root);
                if found.len() >= 5 {
                    return found;
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    for base in ["/media", "/mnt", "/Volumes"] {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(s) = path.to_str() {
                        if vault_exists(s) {
                            found.push(s.to_string());
                            if found.len() >= 5 {
                                return found;
                            }
                        }
                    }
                }
            }
        }
    }

    found
}

fn resolve_path() -> Result<String, String> {
    let vaults = find_vaults();
    match vaults.len() {
        0 => Err("No USB vault found. Plug in your Nostr Portable Identity USB drive.".into()),
        1 => {
            let path = vaults.into_iter().next().unwrap();
            println!("Detected vault at: {}", path);
            Ok(path)
        }
        _ => {
            eprintln!("Multiple vaults found. Please specify one with --path:");
            for v in &vaults {
                eprintln!("  {}", v);
            }
            Err("Multiple vaults detected.".into())
        }
    }
}

fn lock_file_path(base: &PathBuf) -> PathBuf {
    base.join("NOSTR-SIGNER").join(".daemon.lock")
}

fn write_lock_file(path: &PathBuf) -> Result<(), String> {
    let lock = lock_file_path(path);
    if lock.exists() {
        return Err("Another daemon is already running for this USB.".into());
    }
    std::fs::write(&lock, "1").map_err(|e| format!("Failed to write lock file: {}", e))
}

fn remove_lock_file(path: &PathBuf) {
    let _ = std::fs::remove_file(&lock_file_path(path));
}

fn run(vault_path: &PathBuf, timeout: u64) {
    let provider = UsbFileVaultProvider::new(vault_path);

    if let Err(e) = write_lock_file(vault_path) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let service = SignerService::new(
        Some(Box::new(provider)),
        PermissionStore::new(),
        Duration::from_secs(timeout),
    );

    println!("Starting IPC server...");
    log::info!("USB daemon started for vault at: {}", vault_path.display());

    let service = Arc::new(Mutex::new(service));
    nostr_portable_signer_core::ipc_server::start_ipc_server(service.clone());

    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        log::info!("Shutting down...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(500));
    }

    remove_lock_file(vault_path);
    log::info!("Daemon stopped.");
}

fn install_service() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Cannot get binary path: {}", e))?;

    #[cfg(target_os = "linux")]
    {
        let content = format!(
            r#"[Unit]
Description=Nostr Portable Identity USB Daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart={} run
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#,
            exe.display()
        );

        let config_dir = dirs::config_dir().ok_or_else(|| "Cannot find config directory".to_string())?;
        let service_dir = config_dir.join("systemd").join("user");
        std::fs::create_dir_all(&service_dir).map_err(|e| format!("Cannot create service dir: {}", e))?;
        let service_path = service_dir.join("nostr-portable-usb-daemon.service");
        std::fs::write(&service_path, &content).map_err(|e| format!("Cannot write service file: {}", e))?;

        std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .map_err(|e| format!("systemctl daemon-reload failed: {}", e))?;

        std::process::Command::new("systemctl")
            .args(["--user", "enable", "nostr-portable-usb-daemon.service"])
            .status()
            .map_err(|e| format!("systemctl enable failed: {}", e))?;

        std::process::Command::new("systemctl")
            .args(["--user", "start", "nostr-portable-usb-daemon.service"])
            .status()
            .map_err(|e| format!("systemctl start failed: {}", e))?;

        println!("Auto-start enabled.");
        println!("The daemon will run at login and detect your USB vault automatically.");
        println!("The binary at {} stays on the USB — nothing was copied.", exe.display());
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.nostr.portable.identity.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>run</string>
    </array>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
            exe.display()
        );

        let launch_agent_dir = dirs::home_dir()
            .ok_or_else(|| "Cannot find home directory".to_string())?
            .join("Library")
            .join("LaunchAgents");
        std::fs::create_dir_all(&launch_agent_dir).map_err(|e| format!("Cannot create LaunchAgents dir: {}", e))?;
        let plist_path = launch_agent_dir.join("com.nostr.portable.identity.daemon.plist");
        std::fs::write(&plist_path, &content).map_err(|e| format!("Cannot write plist: {}", e))?;

        std::process::Command::new("launchctl")
            .args(["load", &plist_path.to_string_lossy()])
            .status()
            .map_err(|e| format!("launchctl load failed: {}", e))?;

        println!("Auto-start enabled.");
        println!("The binary at {} stays on the USB — nothing was copied.", exe.display());
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let exe_str = dunce::canonicalize(&exe)
            .unwrap_or(exe)
            .to_string_lossy()
            .to_string();

        std::process::Command::new("schtasks")
            .args([
                "/create",
                "/tn", "NostrPortableUsbDaemon",
                "/tr", &format!("\"{}\" run", exe_str),
                "/sc", "onlogon",
                "/delay", "0000:30",
                "/rl", "highest",
                "/f",
            ])
            .status()
            .map_err(|e| format!("schtasks failed: {}", e))?;

        println!("Auto-start enabled.");
        println!("At each login, the daemon will start and look for a USB vault.");
        println!("The binary at {} stays on the USB — nothing was copied.", exe_str);
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Auto-start is not supported on this platform.".into())
    }
}

fn uninstall_service() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "nostr-portable-usb-daemon.service"])
            .status();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "disable", "nostr-portable-usb-daemon.service"])
            .status();

        if let Some(config_dir) = dirs::config_dir() {
            let service_path = config_dir.join("systemd").join("user").join("nostr-portable-usb-daemon.service");
            let _ = std::fs::remove_file(&service_path);
        }

        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();

        println!("Auto-start disabled.");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let plist_path = home.join("Library").join("LaunchAgents").join("com.nostr.portable.identity.daemon.plist");
            let _ = std::process::Command::new("launchctl")
                .args(["unload", &plist_path.to_string_lossy()])
                .status();
            let _ = std::fs::remove_file(&plist_path);
        }
        println!("Auto-start disabled.");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", "NostrPortableUsbDaemon", "/f"])
            .status();
        println!("Auto-start disabled.");
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Auto-start is not supported on this platform.".into())
    }
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Install) => {
            if let Err(e) = install_service() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Command::Uninstall) => {
            if let Err(e) = uninstall_service() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Command::Run) | None => {
            let vault_path = match resolve_path() {
                Ok(p) => PathBuf::from(p),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            run(&vault_path, cli.timeout);
        }
    }
}
