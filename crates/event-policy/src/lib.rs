use nostr::event::Kind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown event kind: {0}")]
    UnknownKind(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Info,
    Caution,
    Warning,
    Destructive,
}

#[derive(Debug, Clone)]
pub struct EventDescriptor {
    pub kind: u16,
    pub name: &'static str,
    pub description: &'static str,
    pub risk: RiskLevel,
}

impl EventDescriptor {
    pub fn describe(kind: Kind) -> Self {
        let k = u16::from(kind);
        match kind {
            Kind::TextNote => Self {
                kind: 1,
                name: "Text Note",
                description: "Publish a text note",
                risk: RiskLevel::Info,
            },
            Kind::Repost | Kind::GenericRepost => Self {
                kind: k,
                name: "Repost",
                description: "Repost an existing event",
                risk: RiskLevel::Info,
            },
            Kind::Reaction => Self {
                kind: 7,
                name: "Reaction",
                description: "Publish a reaction (like, emoji, etc.)",
                risk: RiskLevel::Info,
            },
            Kind::ChannelCreation => Self {
                kind: 40,
                name: "Channel Creation",
                description: "Create a new chat channel",
                risk: RiskLevel::Caution,
            },
            Kind::ChannelMessage => Self {
                kind: 42,
                name: "Channel Message",
                description: "Send a message to a channel",
                risk: RiskLevel::Info,
            },
            Kind::EncryptedDirectMessage => Self {
                kind: 4,
                name: "Encrypted Direct Message",
                description: "Send an encrypted direct message (legacy NIP-04)",
                risk: RiskLevel::Caution,
            },
            Kind::ContactList => Self {
                kind: 3,
                name: "Contact List",
                description: "Replace the account contact list",
                risk: RiskLevel::Caution,
            },
            Kind::EventDeletion => Self {
                kind: 5,
                name: "Deletion Request",
                description: "Publish a deletion request",
                risk: RiskLevel::Destructive,
            },
            Kind::Metadata => Self {
                kind: 0,
                name: "Profile Metadata",
                description: "Set account profile metadata",
                risk: RiskLevel::Caution,
            },
            Kind::RecommendRelay => Self {
                kind: 2,
                name: "Recommend Relay",
                description: "Recommend a relay server",
                risk: RiskLevel::Info,
            },
            Kind::Authentication => Self {
                kind: 22242,
                name: "Client Authentication",
                description: "Authenticate with a relay",
                risk: RiskLevel::Info,
            },
            Kind::Seal | Kind::GiftWrap | Kind::PrivateDirectMessage => Self {
                kind: k,
                name: "Private Message",
                description: "Private encrypted message (NIP-17/NIP-59)",
                risk: RiskLevel::Info,
            },
            _ => {
                if k >= 20000 && k <= 29999 {
                    Self {
                        kind: k,
                        name: "Ephemeral Event",
                        description: "Ephemeral event (not stored by relays)",
                        risk: RiskLevel::Info,
                    }
                } else if k >= 30000 && k <= 39999 {
                    Self {
                        kind: k,
                        name: "Parameterized Replaceable",
                        description: "Replaceable data event with d-tag identifier",
                        risk: RiskLevel::Caution,
                    }
                } else if k >= 10000 && k <= 19999 {
                    Self {
                        kind: k,
                        name: "Replaceable Data",
                        description: "Replaceable data event",
                        risk: RiskLevel::Caution,
                    }
                } else if k >= 1000 && k <= 9999 {
                    Self {
                        kind: k,
                        name: "Regular Event",
                        description: "Custom event kind",
                        risk: RiskLevel::Warning,
                    }
                } else {
                    Self {
                        kind: k,
                        name: "Unknown Event",
                        description: "Unknown event kind — full details will be shown",
                        risk: RiskLevel::Warning,
                    }
                }
            }
        }
    }
}

pub fn describe(kind: Kind) -> EventDescriptor {
    EventDescriptor::describe(kind)
}

pub fn describe_raw(kind: u16) -> EventDescriptor {
    EventDescriptor::describe(Kind::from(kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_note_descriptor() {
        let desc = describe(Kind::TextNote);
        assert_eq!(desc.kind, 1);
        assert_eq!(desc.risk, RiskLevel::Info);
        assert_eq!(desc.name, "Text Note");
    }

    #[test]
    fn test_event_deletion_descriptor() {
        let desc = describe(Kind::EventDeletion);
        assert_eq!(desc.kind, 5);
        assert_eq!(desc.risk, RiskLevel::Destructive);
    }

    #[test]
    fn test_unknown_kind() {
        let desc = describe(Kind::from(9999));
        assert_eq!(desc.risk, RiskLevel::Warning);
        assert_eq!(desc.name, "Regular Event");
    }

    #[test]
    fn test_ephemeral_kind() {
        let desc = describe(Kind::from(20001));
        assert_eq!(desc.risk, RiskLevel::Info);
        assert_eq!(desc.name, "Ephemeral Event");
    }

    #[test]
    fn test_describe_raw() {
        let desc = describe_raw(7);
        assert_eq!(desc.kind, 7);
        assert_eq!(desc.name, "Reaction");
    }

    #[test]
    fn test_parameterized_replaceable() {
        let desc = describe(Kind::from(30023));
        assert_eq!(desc.risk, RiskLevel::Caution);
        assert_eq!(desc.name, "Parameterized Replaceable");
    }

    #[test]
    fn test_replaceable() {
        let desc = describe(Kind::from(10000));
        assert_eq!(desc.risk, RiskLevel::Caution);
        assert_eq!(desc.name, "Replaceable Data");
    }
}
