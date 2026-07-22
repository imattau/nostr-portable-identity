use std::collections::HashMap;

use nostr_portable_vault as vault;
use nostr_portable_vault::VaultProvider;

use crate::Error;

/// Represents a registered identity with its associated vault provider.
#[derive(Clone)]
pub struct Identity {
    /// Human-readable label (e.g., "Primary", "Backup", "Hardware")
    pub label: String,
    /// The vault provider type for display purposes
    pub provider_type: String,
    /// Whether this identity is currently available (vault present)
    pub available: bool,
}

/// Manages multiple identities, each backed by a different VaultProvider.
///
/// This allows the signer to switch between:
/// - A USB drive vault
/// - A local file vault
/// - A remote NIP-46 signer
/// - A hardware signer
///
/// Only one identity is active at a time per signer session.
pub struct IdentityManager {
    /// All registered identities, keyed by label
    providers: HashMap<String, Box<dyn VaultProvider>>,
    /// The label of the currently active identity
    active: Option<String>,
}

impl IdentityManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active: None,
        }
    }

    /// Register a new identity with a vault provider
    pub fn register(&mut self, label: &str, provider: Box<dyn VaultProvider>) {
        self.providers.insert(label.to_string(), provider);
    }

    /// Remove an identity by label
    pub fn unregister(&mut self, label: &str) -> Option<Box<dyn VaultProvider>> {
        if self.active.as_deref() == Some(label) {
            self.active = None;
        }
        self.providers.remove(label)
    }

    /// Switch the active identity
    pub fn switch_to(&mut self, label: &str) -> Result<(), Error> {
        if self.providers.contains_key(label) {
            self.active = Some(label.to_string());
            Ok(())
        } else {
            Err(Error::Protocol(
                nostr_portable_protocol::Error::Internal(format!("identity '{}' not found", label)),
            ))
        }
    }

    /// Get the active provider
    pub fn active_provider(&self) -> Option<&dyn VaultProvider> {
        self.active
            .as_ref()
            .and_then(|label| self.providers.get(label))
            .map(|p| p.as_ref())
    }

    /// Get the active provider mutably
    pub fn active_provider_mut<'b>(&'b mut self) -> Option<&'b mut (dyn VaultProvider + 'static)> {
        if let Some(ref label) = self.active {
            self.providers.get_mut(label).map(|p| p.as_mut())
        } else {
            None
        }
    }

    /// Get a provider by label
    pub fn provider(&self, label: &str) -> Option<&dyn VaultProvider> {
        self.providers.get(label).map(|p| p.as_ref())
    }

    /// Get the active identity label
    pub fn active_label(&self) -> Option<&str> {
        self.active.as_deref()
    }

    /// List all registered identities
    pub fn list(&self) -> Vec<Identity> {
        self.providers
            .iter()
            .map(|(label, provider)| Identity {
                label: label.clone(),
                provider_type: provider.name().to_string(),
                available: provider.is_available(),
            })
            .collect()
    }

    /// Number of registered identities
    pub fn count(&self) -> usize {
        self.providers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr_portable_vault::providers::local_file::LocalFileVaultProvider;
    use nostr_portable_vault::providers::nip46_remote::Nip46RemoteProvider;
    use tempfile::TempDir;

    fn setup_local_provider(tmp: &TempDir) -> Box<dyn VaultProvider> {
        let home = tmp.path().join("home");
        std::env::set_var("HOME", home.to_str().unwrap());
        Box::new(LocalFileVaultProvider::new().unwrap())
    }

    #[test]
    fn test_empty_manager() {
        let mgr = IdentityManager::new();
        assert_eq!(mgr.count(), 0);
        assert!(mgr.active_label().is_none());
        assert!(mgr.active_provider().is_none());
    }

    #[test]
    fn test_register_and_switch() {
        let tmp = TempDir::new().unwrap();
        let mut mgr = IdentityManager::new();
        let provider = setup_local_provider(&tmp);
        mgr.register("local", provider);

        assert_eq!(mgr.count(), 1);
        assert!(mgr.active_label().is_none());

        mgr.switch_to("local").unwrap();
        assert_eq!(mgr.active_label(), Some("local"));
        assert!(mgr.active_provider().is_some());
    }

    #[test]
    fn test_switch_to_nonexistent() {
        let mut mgr = IdentityManager::new();
        let result = mgr.switch_to("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_identities() {
        let tmp = TempDir::new().unwrap();
        let mut mgr = IdentityManager::new();
        mgr.register("usb", setup_local_provider(&tmp));
        mgr.register("remote", Box::new(Nip46RemoteProvider::new(
            "wss://relay.example", "abc", "Remote",
        )));

        assert_eq!(mgr.count(), 2);

        mgr.switch_to("usb").unwrap();
        assert_eq!(mgr.active_label(), Some("usb"));

        mgr.switch_to("remote").unwrap();
        assert_eq!(mgr.active_label(), Some("remote"));
    }

    #[test]
    fn test_list_identities() {
        let tmp = TempDir::new().unwrap();
        let mut mgr = IdentityManager::new();
        mgr.register("main", setup_local_provider(&tmp));

        let identities = mgr.list();
        assert_eq!(identities.len(), 1);
        assert_eq!(identities[0].label, "main");
    }

    #[test]
    fn test_unregister_clears_active() {
        let tmp = TempDir::new().unwrap();
        let mut mgr = IdentityManager::new();
        mgr.register("main", setup_local_provider(&tmp));
        mgr.switch_to("main").unwrap();

        let removed = mgr.unregister("main");
        assert!(removed.is_some());
        assert!(mgr.active_label().is_none());
        assert_eq!(mgr.count(), 0);
    }
}
