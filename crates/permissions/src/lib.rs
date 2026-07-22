use std::collections::HashMap;
use nostr::event::Kind;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("permission denied for method '{method}'")]
    Denied { method: String },
    #[error("method not found: {0}")]
    MethodNotFound(String),
    #[error("unknown client identity")]
    UnknownClient,
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClientIdentity {
    BrowserOrigin(String),
    ExecutablePath(String),
    PackageId(String),
    Unknown(String),
}

impl std::fmt::Display for ClientIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientIdentity::BrowserOrigin(origin) => write!(f, "browser:{}", origin),
            ClientIdentity::ExecutablePath(path) => write!(f, "app:{}", path),
            ClientIdentity::PackageId(id) => write!(f, "package:{}", id),
            ClientIdentity::Unknown(id) => write!(f, "unknown:{}", id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionRule {
    Allow,
    Deny,
    Ask,
    RememberForKinds(Vec<u16>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionEntry {
    pub method: String,
    pub rule: PermissionRule,
    pub kind_restriction: Option<Vec<u16>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPermissions {
    pub identity: ClientIdentity,
    pub entries: Vec<PermissionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionStore {
    clients: HashMap<String, ClientPermissions>,
}

pub enum PermissionResult {
    Allowed,
    Denied,
    Ask,
}

impl PermissionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_permission(&mut self, identity: ClientIdentity, entry: PermissionEntry) {
        let key = identity.to_string();
        let permissions = self.clients.entry(key).or_insert(ClientPermissions {
            identity,
            entries: Vec::new(),
        });
        if let Some(existing) = permissions.entries.iter_mut().find(|e| e.method == entry.method) {
            *existing = entry;
        } else {
            permissions.entries.push(entry);
        }
    }

    pub fn evaluate(
        &self,
        identity: &ClientIdentity,
        method: &str,
        kind: Option<Kind>,
    ) -> PermissionResult {
        let key = identity.to_string();
        let Some(client) = self.clients.get(&key) else {
            return PermissionResult::Ask;
        };

        let Some(entry) = client.entries.iter().find(|e| e.method == method) else {
            return PermissionResult::Ask;
        };

        if let Some(restricted_kinds) = &entry.kind_restriction {
            if let Some(k) = kind {
                if !restricted_kinds.contains(&u16::from(k)) {
                    return PermissionResult::Denied;
                }
            }
        }

        match &entry.rule {
            PermissionRule::Allow => PermissionResult::Allowed,
            PermissionRule::Deny => PermissionResult::Denied,
            PermissionRule::Ask => PermissionResult::Ask,
            PermissionRule::RememberForKinds(kinds) => {
                if let Some(k) = kind {
                    if kinds.contains(&u16::from(k)) {
                        PermissionResult::Allowed
                    } else {
                        PermissionResult::Ask
                    }
                } else {
                    PermissionResult::Ask
                }
            }
        }
    }

    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn browser_origin(origin: &str) -> ClientIdentity {
        ClientIdentity::BrowserOrigin(origin.to_string())
    }

    #[test]
    fn test_default_is_ask() {
        let store = PermissionStore::new();
        let identity = browser_origin("https://example.com");
        let result = store.evaluate(&identity, "signEvent", Some(Kind::TextNote));
        assert!(matches!(result, PermissionResult::Ask));
    }

    #[test]
    fn test_allow_method() {
        let mut store = PermissionStore::new();
        let identity = browser_origin("https://example.com");
        store.set_permission(
            identity.clone(),
            PermissionEntry {
                method: "getPublicKey".into(),
                rule: PermissionRule::Allow,
                kind_restriction: None,
            },
        );
        let result = store.evaluate(&identity, "getPublicKey", None);
        assert!(matches!(result, PermissionResult::Allowed));
    }

    #[test]
    fn test_deny_method() {
        let mut store = PermissionStore::new();
        let identity = browser_origin("https://example.com");
        store.set_permission(
            identity.clone(),
            PermissionEntry {
                method: "nip44Decrypt".into(),
                rule: PermissionRule::Deny,
                kind_restriction: None,
            },
        );
        let result = store.evaluate(&identity, "nip44Decrypt", None);
        assert!(matches!(result, PermissionResult::Denied));
    }

    #[test]
    fn test_kind_restriction() {
        let mut store = PermissionStore::new();
        let identity = browser_origin("https://example.com");
        store.set_permission(
            identity.clone(),
            PermissionEntry {
                method: "signEvent".into(),
                rule: PermissionRule::Allow,
                kind_restriction: Some(vec![1, 6, 7]),
            },
        );
        assert!(matches!(
            store.evaluate(&identity, "signEvent", Some(Kind::TextNote)),
            PermissionResult::Allowed
        ));
        assert!(matches!(
            store.evaluate(&identity, "signEvent", Some(Kind::EventDeletion)),
            PermissionResult::Denied
        ));
    }

    #[test]
    fn test_remember_for_kinds() {
        let mut store = PermissionStore::new();
        let identity = browser_origin("https://example.com");
        store.set_permission(
            identity.clone(),
            PermissionEntry {
                method: "signEvent".into(),
                rule: PermissionRule::RememberForKinds(vec![1, 7]),
                kind_restriction: None,
            },
        );
        assert!(matches!(
            store.evaluate(&identity, "signEvent", Some(Kind::TextNote)),
            PermissionResult::Allowed
        ));
        assert!(matches!(
            store.evaluate(&identity, "signEvent", Some(Kind::EventDeletion)),
            PermissionResult::Ask
        ));
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut store = PermissionStore::new();
        store.set_permission(
            browser_origin("https://nos.to"),
            PermissionEntry {
                method: "signEvent".into(),
                rule: PermissionRule::Ask,
                kind_restriction: None,
            },
        );
        let json = store.to_json().unwrap();
        let restored = PermissionStore::from_json(&json).unwrap();
        let result = restored.evaluate(
            &browser_origin("https://nos.to"),
            "signEvent",
            Some(Kind::TextNote),
        );
        assert!(matches!(result, PermissionResult::Ask));
    }

    #[test]
    fn test_different_identities_independent() {
        let mut store = PermissionStore::new();
        let a = browser_origin("https://app-a.com");
        let b = browser_origin("https://app-b.com");
        store.set_permission(
            a.clone(),
            PermissionEntry {
                method: "getPublicKey".into(),
                rule: PermissionRule::Allow,
                kind_restriction: None,
            },
        );
        assert!(matches!(store.evaluate(&a, "getPublicKey", None), PermissionResult::Allowed));
        assert!(matches!(store.evaluate(&b, "getPublicKey", None), PermissionResult::Ask));
    }
}
