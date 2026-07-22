# Architecture

## Overview

Nostr Portable Identity is a cross-platform signing platform for Nostr.
It separates key storage, signing, permissions, and client communication
into independent layers connected through a shared Rust core.

## Layer Diagram

```text
Applications (browsers, desktop apps, mobile apps)
       |
       v
Client Adapters
  ├── NIP-07 (browser extension)
  ├── Local IPC (desktop native messaging)
  ├── NIP-46 (remote signing)
  └── NIP-55 (Android)
       |
       v
Signer Service API (NostrSigner trait)
       |
       v
Shared Rust Signer Core
  ├── Session management
  ├── Permission evaluation
  ├── Event policy interpretation
  └── Cryptographic operations
       |
       v
Vault Providers
  ├── USB file vault
  ├── Local file vault
  └── Future: hardware, smartcard, remote
```

## Crates

### `nostr-portable-crypto`

Re-exports from the `nostr` crate. Provides convenience wrappers for
key generation, NIP-49 encrypted key storage, NIP-44 encryption, and
event signing.

Dependencies: `nostr` (with nip44, nip49 features)

### `nostr-portable-vault`

Defines the `VaultProvider` trait for abstracting storage backends.
Provides `EncryptedVault` (NIP-49-based vault format) and
`UsbFileVaultProvider` (reads from `NOSTR-SIGNER/nostr-vault.json`).

### `nostr-portable-event-policy`

Interprets event kinds into human-readable descriptions with risk
levels (Info, Caution, Warning, Destructive). Used by approval
windows to explain what is being signed.

### `nostr-portable-permissions`

Per-client permission model. Supports Allow, Deny, Ask, and
RememberForKinds rules per method per client identity.

### `nostr-portable-protocol`

Defines the `NostrSigner` trait, IPC request/response types, and
NIP-46 type stubs. This is the API contract between the signer core
and all client adapters.

### `nostr-portable-signer-core`

Implements the `NostrSigner` trait as `SignerService`. Manages the
locked/unlocked state machine, auto-lock timing, vault availability
monitoring, and orchestrates vault → decrypt → permission check →
sign flows.

## Crate Dependency Graph

```
crypto        (nostr crate)
    ↑
vault
    ↑
event-policy  permissions
    ↑          ↑
    └── protocol ──┘
            ↑
      signer-core
```
