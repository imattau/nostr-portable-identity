# Security Model

## What the signer protects against

- Leaving plaintext `nsec` in browser storage or application config
- Storing private keys permanently on multiple computers
- Accidental copying and pasting of private keys
- Ordinary filesystem backup exposure of plaintext keys
- Nostr websites directly accessing the private key
- Casual use of the identity without the USB vault and password

## What the signer does not fully protect against

- Malware running on the host system
- Password capture (keylogging, phishing)
- Process-memory scraping (decrypted keys reside in RAM)
- Malicious replacement of the signer application
- Copying the encrypted USB vault for offline password attacks
- Event manipulation on a compromised device

The signer is a **portable encrypted Nostr signer vault**, not a
hardware wallet. The private key is decrypted in host memory during
signing sessions.

## Design principles

### Keys at rest

Private keys are stored encrypted using NIP-49 (ncryptsec1 format).
The encryption uses scrypt key derivation with a user-provided
passphrase.

### Keys in use

The decrypted key material exists only in the Rust signer core
during an unlocked session. It is never exposed to the frontend
(JavaScript/TypeScript) layer.

### Minimal exposure

- No plaintext keys written to disk
- No passwords passed through command-line arguments
- Sensitive buffers should be cleared on lock
- Core dumps should be disabled where practical

### Session lifecycle

The signer locks automatically when:
- The configured inactivity timeout expires
- The USB vault becomes unavailable (drive removed)
- The computer is suspended
- The screen is locked
- The signer process restarts

### Permission model

Every signing operation is subject to permission evaluation:

| Rule | Behaviour |
|------|-----------|
| Allow | Operation proceeds without prompting |
| Deny | Operation is rejected |
| Ask | User must approve each request |
| RememberForKinds | Auto-allow for specified kinds, ask for others |

Permissions are stored per client identity (browser origin, executable
path, package ID).

### USB presence

The signer uses USB presence as a usability and access-control signal,
not as cryptographic proof. Before signing, it verifies:
- The vault file is accessible
- The vault identifier matches
- The encrypted vault has not unexpectedly changed
- The active identity matches the request

## Event policy

The signer interprets event kinds before presenting them for approval:

| Kind | Name | Risk |
|------|------|------|
| 1 | Text Note | Info |
| 5 | Deletion Request | Destructive |
| 3 | Contact List | Caution |
| Unknown | Unknown Event | Warning |

Unknown or unusual event kinds are shown with full event details and
a stronger warning.
