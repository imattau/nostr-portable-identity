use std::time::Duration;

use clap::{Parser, Subcommand};
use nostr_portable_crypto as crypto;
use nostr_portable_permissions::{ClientIdentity, PermissionEntry, PermissionRule, PermissionStore};
use nostr_portable_protocol::{NostrSigner, SignEventRequest};
use nostr_portable_signer_core::{PermissionCheck, SignerService};
use nostr_portable_vault as vault;
use nostr_portable_vault::{UsbFileVaultProvider, VaultProvider};

fn validate_path(path: &str) -> Result<String, String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| format!("invalid vault path '{}': {}", path, e))?;
    let s = canonical.to_string_lossy().to_string();
    if s.contains("..") {
        return Err("vault path contains directory traversal".into());
    }
    Ok(s)
}

fn prompt_passphrase(prompt: &str) -> String {
    rpassword::prompt_password(prompt).unwrap_or_else(|_| {
        eprint!("{}", prompt);
        let mut pass = String::new();
        std::io::stdin().read_line(&mut pass).ok();
        pass.trim().to_string()
    })
}

#[derive(Parser)]
#[command(name = "nostr-portable", about = "Nostr Portable Identity CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new encrypted vault (passphrase prompted via stdin)
    Create {
        #[arg(short, long)]
        path: String,
        #[arg(short, long, default_value = "Primary identity")]
        name: String,
        #[arg(short, long)]
        nsec: Option<String>,
    },
    /// Show vault information
    Info {
        #[arg(short, long)]
        path: String,
    },
    /// Unlock vault and start signing session (passphrase prompted via stdin)
    Unlock {
        #[arg(short, long)]
        path: String,
        #[arg(short, long, default_value = "300")]
        timeout: u64,
    },
    /// Sign a text note event
    Sign {
        content: String,
    },
    /// Get the public key
    Pubkey,
    /// Lock the signer
    Lock,
    /// Show signer status
    Status,
}

fn prompt_approval(request: &nostr_portable_protocol::ApprovalRequest) -> bool {
    use std::io::{self, Write};
    eprintln!();
    eprintln!("=== Approval Required ===");
    eprintln!("  Client:   {}", request.client_identity);
    eprintln!("  Method:   {}", request.method);
    eprintln!("  Details:  {}", request.description);
    eprintln!("  Risk:     {}", request.risk_level);
    eprintln!("=========================");
    loop {
        print!("Approve? [y/N] > ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" | "" => return false,
            _ => eprintln!("Please enter 'y' or 'n'"),
        }
    }
}

fn handle_permission_check(
    service: &SignerService,
    method: &str,
    kind: Option<nostr::event::Kind>,
    permissions: &mut PermissionStore,
) -> bool {
    let identity = ClientIdentity::Unknown("cli".into());
    loop {
        match service.evaluate_permission(&identity, method, kind) {
            PermissionCheck::Allowed => return true,
            PermissionCheck::Denied(reason) => {
                eprintln!("Error: {}", reason);
                return false;
            }
            PermissionCheck::Ask(request) => {
                if prompt_approval(&request) {
                    permissions.set_permission(
                        identity.clone(),
                        PermissionEntry {
                            method: method.to_string(),
                            rule: PermissionRule::Allow,
                            kind_restriction: kind.map(|k| vec![u16::from(k)]),
                        },
                    );
                    continue;
                } else {
                    return false;
                }
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Create { path, name, nsec } => {
            let _ = validate_path(path).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let passphrase = prompt_passphrase("Vault passphrase: ");
            let provider = UsbFileVaultProvider::new(path);
            let keys = match nsec {
                Some(nsec) => match crypto::parse_keys(nsec) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("Error: invalid nsec: {}", e);
                        std::process::exit(1);
                    }
                },
                None => crypto::generate_keys(),
            };
            match vault::create_vault(&provider, name.clone(), &keys, &passphrase) {
                Ok(vault) => {
                    println!("Vault created successfully!");
                    println!("  Name:     {}", vault.name);
                    println!("  Pubkey:   {}", vault.pubkey);
                    println!("  Path:     {}/NOSTR-SIGNER/nostr-vault.json", path);
                    println!("  Type:     {}", vault.vault_type);
                }
                Err(e) => {
                    eprintln!("Error creating vault: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Info { path } => {
            let _ = validate_path(path).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let provider = UsbFileVaultProvider::new(path);
            match provider.load_encrypted_vault() {
                Ok(vault) => {
                    println!("Vault Information:");
                    println!("  Name:     {}", vault.name);
                    println!("  Pubkey:   {}", vault.pubkey);
                    println!("  Version:  {}", vault.version);
                    println!("  Type:     {}", vault.vault_type);
                    println!("  Created:  {}", vault.created_at);
                    let preview = if vault.encrypted_key.len() > 30 {
                        &vault.encrypted_key[..30]
                    } else {
                        &vault.encrypted_key
                    };
                    println!("  Encrypted key: {}...", preview);
                }
                Err(e) => {
                    eprintln!("Error reading vault: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Unlock { path, timeout } => {
            let _ = validate_path(path).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
            let passphrase = prompt_passphrase("Vault passphrase: ");
            let provider = Box::new(UsbFileVaultProvider::new(path));
            let mut service = SignerService::new(
                Some(provider),
                PermissionStore::new(),
                Duration::from_secs(*timeout),
            );
            match service.unlock(&passphrase) {
                Ok(()) => {
                    println!("Vault unlocked. Session active (timeout: {}s).", timeout);
                    println!("Available commands: pubkey, sign <text>, lock, status");
                    interactive_loop(&mut service);
                }
                Err(e) => {
                    eprintln!("Error unlocking vault: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Command::Sign { .. } => {
            eprintln!("Error: 'sign' command must be used after 'unlock'");
            std::process::exit(1);
        }
        Command::Pubkey => {
            eprintln!("Error: 'pubkey' command must be used after 'unlock'");
            std::process::exit(1);
        }
        Command::Lock => {
            eprintln!("Error: 'lock' command must be used after 'unlock'");
            std::process::exit(1);
        }
        Command::Status => {
            eprintln!("Error: 'status' command must be used after 'unlock'");
            std::process::exit(1);
        }
    }
}

fn interactive_loop(service: &mut SignerService) {
    use std::io::{self, Write};

    let mut session_permissions = PermissionStore::new();

    loop {
        service.check_auto_lock();
        if service.state() == nostr_portable_signer_core::State::Locked {
            println!("\nSession expired (auto-lock).");
            break;
        }
        print!("> ");
        io::stdout().flush().ok();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        match parts[0] {
            "pubkey" | "pk" => {
                match service.get_public_key() {
                    Ok(pk) => println!("{}", pk.to_hex()),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "sign" => {
                let content = parts.get(1).unwrap_or(&"");
                if !handle_permission_check(service, "signEvent", Some(nostr::event::Kind::TextNote), &mut session_permissions) {
                    eprintln!("Signing not permitted.");
                    continue;
                }
                let pk = match service.get_public_key() {
                    Ok(pk) => pk,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        continue;
                    }
                };
                let unsigned = nostr::event::EventBuilder::text_note(content.to_string())
                    .build(pk);
                let request = SignEventRequest {
                    event: unsigned,
                    kind: nostr::event::Kind::TextNote,
                    content: content.to_string(),
                    tags: vec![],
                };
                match service.sign_event(request) {
                    Ok(event) => {
                        println!("Signed event:");
                        println!("  ID:      {}", event.id.to_hex());
                        println!("  Kind:    {}", u16::from(event.kind));
                        println!("  Content: {}", event.content);
                        println!("  Sig:     {}", event.sig.to_string());
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "lock" => {
                match service.lock() {
                    Ok(()) => {
                        println!("Signer locked.");
                        break;
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "status" | "st" => {
                let status = service.status();
                println!("State:        {}", status.state);
                println!("Vault:        {}", if status.vault_present { "present" } else { "missing" });
                if let Some(pk) = &status.public_key {
                    println!("Public key:   {}", pk);
                }
            }
            "help" | "h" | "?" => {
                println!("Commands:");
                println!("  pubkey|pk       - Show the public key");
                println!("  sign <text>     - Sign a text note");
                println!("  lock            - Lock the signer");
                println!("  status|st       - Show signer status");
                println!("  help|h|?        - Show this help");
                println!("  quit|exit       - Exit");
            }
            "quit" | "exit" | "q" => {
                let _ = service.lock();
                break;
            }
            _ => {
                eprintln!("Unknown command: {}. Type 'help' for available commands.", parts[0]);
            }
        }
    }
}
