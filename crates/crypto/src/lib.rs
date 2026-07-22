pub use nostr::{
    key::{SecretKey, Keys, PublicKey},
    event::{Event, UnsignedEvent, EventBuilder, EventId, Tag, Tags, Kind},
    message::SubscriptionId,
    nips::nip44::{self, Version},
    nips::nip49::{EncryptedSecretKey, KeySecurity, self},
    nips::nip19::{FromBech32, ToBech32},
    types::Timestamp,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("nostr error: {0}")]
    Nostr(String),
    #[error("secret key decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("secret key encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("bech32 encoding error: {0}")]
    Bech32(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Nostr(s)
    }
}

pub fn generate_keys() -> Keys {
    Keys::generate()
}

pub fn parse_keys(nsec: &str) -> Result<Keys, Error> {
    Keys::parse(nsec).map_err(|e| Error::InvalidKey(format!("{:?}", e)))
}

pub fn decrypt_ncryptsec(ncryptsec: &str, passphrase: &str) -> Result<Keys, Error> {
    let encrypted = EncryptedSecretKey::from_bech32(ncryptsec)
        .map_err(|e| Error::DecryptionFailed(format!("{:?}", e)))?;
    let secret = encrypted
        .decrypt(passphrase)
        .map_err(|e| Error::DecryptionFailed(format!("{:?}", e)))?;
    let keys = Keys::new(secret);
    Ok(keys)
}

pub fn encrypt_ncryptsec(keys: &Keys, passphrase: &str) -> Result<String, Error> {
    let encrypted = EncryptedSecretKey::new(
        keys.secret_key(),
        passphrase,
        16,
        KeySecurity::Medium,
    ).map_err(|e| Error::EncryptionFailed(format!("{:?}", e)))?;
    let encoded = encrypted.to_bech32()
        .map_err(|e| Error::Bech32(format!("{:?}", e)))?;
    Ok(encoded)
}

pub fn nip44_encrypt(keys: &Keys, recipient: &PublicKey, content: &str) -> Result<String, Error> {
    nip44::encrypt(keys.secret_key(), recipient, content, Version::V2)
        .map_err(|e| Error::EncryptionFailed(format!("{:?}", e)))
}

pub fn nip44_decrypt(keys: &Keys, sender: &PublicKey, payload: &str) -> Result<String, Error> {
    nip44::decrypt(keys.secret_key(), sender, payload)
        .map_err(|e| Error::DecryptionFailed(format!("{:?}", e)))
}

pub fn sign_event(keys: &Keys, unsigned: UnsignedEvent) -> Result<Event, Error> {
    unsigned.sign_with_keys(keys).map_err(|e| Error::Nostr(format!("{:?}", e)))
}

pub fn get_public_key(keys: &Keys) -> PublicKey {
    keys.public_key()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keys() {
        let keys = generate_keys();
        let pk = get_public_key(&keys);
        assert_eq!(keys.public_key(), pk);
    }

    #[test]
    fn test_parse_nsec_roundtrip() {
        let keys = generate_keys();
        let nsec = keys.secret_key().to_bech32().unwrap();
        let parsed = parse_keys(&nsec).unwrap();
        assert_eq!(keys.public_key(), parsed.public_key());
    }

    #[test]
    fn test_ncryptsec_roundtrip() {
        let keys = generate_keys();
        let passphrase = "test-password-123";
        let encrypted = encrypt_ncryptsec(&keys, passphrase).unwrap();
        assert!(encrypted.starts_with("ncryptsec1"));

        let decrypted = decrypt_ncryptsec(&encrypted, passphrase).unwrap();
        assert_eq!(keys.public_key(), decrypted.public_key());
    }

    #[test]
    fn test_ncryptsec_wrong_password() {
        let keys = generate_keys();
        let encrypted = encrypt_ncryptsec(&keys, "correct-password").unwrap();
        let result = decrypt_ncryptsec(&encrypted, "wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_and_verify_event() {
        let keys = generate_keys();
        let unsigned = EventBuilder::text_note("Hello from test")
            .build(keys.public_key());
        let event = sign_event(&keys, unsigned).unwrap();
        assert!(event.verify().is_ok());
        assert_eq!(event.content, "Hello from test");
    }

    #[test]
    fn test_nip44_roundtrip() {
        let alice = generate_keys();
        let bob = generate_keys();
        let plaintext = "secret message";

        let ciphertext = nip44_encrypt(&alice, &bob.public_key(), plaintext).unwrap();
        let decrypted = nip44_decrypt(&bob, &alice.public_key(), &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
