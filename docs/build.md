# Build and Packaging

## Prerequisites

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux build dependencies

```bash
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev

# For Tauri bundler (deb, AppImage)
sudo apt install dpkg-dev fakeroot
```

### Node.js

```bash
# Required for the Svelte frontend
# Install from https://nodejs.org or use nvm
node --version  # >= 18
npm --version   # >= 8
```

## Building

### Development build

```bash
# Build all Rust crates (works without Node.js)
cargo build

# Build the desktop app with frontend
cd apps/tauri/desktop
npm install
npm run tauri dev       # Run in development mode
```

### Release build

```bash
# Build the Rust workspace in release
cargo build --release

# Build the Tauri desktop app with bundling
cd apps/tauri/desktop
npm install
npm run tauri build     # Produces .deb, .AppImage in src-tauri/target/release/bundle/
```

The Tauri bundler will produce:
- `bundle/deb/` → `.deb` package (Debian/Ubuntu)
- `bundle/appimage/` → `.AppImage` (all Linux)
- `bundle/rpm/` → `.rpm` package (Fedora/RHEL) — requires `rpmbuild`

### Native messaging host

```bash
cargo build --release -p nostr-portable-nm-host
```

The binary is at `target/release/nostr-portable-nm-host`.

## Installing

### From built packages

```bash
# .deb
sudo dpkg -i apps/tauri/desktop/src-tauri/target/release/bundle/deb/nostr-portable-identity_*.deb

# .AppImage (no install required)
chmod +x apps/tauri/desktop/src-tauri/target/release/bundle/appimage/*.AppImage
./Nostr-Portable-Identity*.AppImage
```

### Manual install

```bash
# Install the desktop binary
sudo cp target/release/nostr-portable-desktop /usr/lib/nostr-portable-identity/
sudo ln -sf /usr/lib/nostr-portable-identity/nostr-portable-desktop /usr/bin/

# Install the native messaging host
sudo ./scripts/install-nm-host.sh target/release/nostr-portable-nm-host <extension-id>

# Run post-install
sudo ./scripts/postinstall.sh
```

## Native messaging host registration

The `scripts/install-nm-host.sh` script installs the native messaging host
manifest for Chromium, Chrome, Brave, Edge, and Firefox.

You need the extension ID from the Chrome Web Store to complete registration.

## Autostart

The `.desktop` file at `platform/desktop-entry/com.nostr.portable.identity.desktop`
is configured for XDG autostart. The post-install script copies it to
`/etc/xdg/autostart/`.

The Tauri autostart plugin also allows enabling/disabling autostart from
within the app.

## Update mechanism

The Tauri updater plugin is configured in `tauri.conf.json`:

```json
"plugins": {
  "updater": {
    "endpoints": [
      "https://releases.nostr-portable-identity.com/{{target}}/{{current_version}}"
    ],
    "pubkey": "<your-public-key>"
  }
}
```

To enable updates:

1. Set up a release server at the endpoint URL
2. Generate signing keys: `tauri signer generate -w ~/.tauri/updater.key`
3. Set the public key in `tauri.conf.json`
4. Sign release artifacts with `tauri signer sign`

## Platform-specific notes

### Linux

- Requires `libwebkit2gtk-4.1` (runtime dependency)
- The `.deb` package defines dependencies in `tauri.conf.json`
- Autostart uses XDG autostart (`/etc/xdg/autostart/`)

### macOS

- Build on macOS produces `.dmg` and `.app` bundles
- Minimum macOS 11.0 (Big Sur)
- Code signing and notarisation require an Apple Developer account

### Windows

- Build on Windows produces `.msi` (WiX) and `.exe` (NSIS) installers
- NSIS installer uses currentUser install mode
- Windows native messaging uses the same binary with stdio protocol
