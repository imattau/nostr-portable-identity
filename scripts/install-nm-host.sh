#!/usr/bin/env bash
# Install the native messaging host for Chromium-based browsers
set -euo pipefail

NM_HOST_BIN="${1:-./target/release/nostr-portable-nm-host}"
NM_HOST_DIR="/usr/lib/nostr-portable-identity"
EXTENSION_ID="${2:-}"  # e.g., "abcdefghijklmnopabcdefghijklmnop"

if [ ! -f "$NM_HOST_BIN" ]; then
    echo "Error: Native messaging host binary not found at: $NM_HOST_BIN"
    echo "Build it first: cargo build -p nostr-portable-nm-host --release"
    exit 1
fi

if [ -z "$EXTENSION_ID" ]; then
    echo "Warning: No extension ID provided."
    echo "The manifest will need to be edited before the browser will accept it."
    echo "Usage: $0 <path-to-binary> <extension-id>"
fi

# Install the binary
echo "Installing native messaging host binary..."
sudo mkdir -p "$NM_HOST_DIR"
sudo cp "$NM_HOST_BIN" "$NM_HOST_DIR/nostr-portable-nm-host"
sudo chmod 755 "$NM_HOST_DIR/nostr-portable-nm-host"

# Generate and install the manifest
MANIFEST_DIR_CHROME="${XDG_CONFIG_HOME:-$HOME/.config}/google-chrome/NativeMessagingHosts"
MANIFEST_DIR_CHROMIUM="${XDG_CONFIG_HOME:-$HOME/.config}/chromium/NativeMessagingHosts"
MANIFEST_DIR_BRAVE="${XDG_CONFIG_HOME:-$HOME/.config}/BraveSoftware/Brave-Browser/NativeMessagingHosts"
MANIFEST_DIR_EDGE="${XDG_CONFIG_HOME:-$HOME/.config}/microsoft-edge/NativeMessagingHosts"
MANIFEST_DIR_VIVALDI="${XDG_CONFIG_HOME:-$HOME/.config}/vivaldi/NativeMessagingHosts"

MANIFEST_NAME="com.nostr.portable.identity.json"

generate_manifest() {
    local dir="$1"
    mkdir -p "$dir"
    cat > "$dir/$MANIFEST_NAME" << EOF
{
  "name": "com.nostr.portable.identity",
  "description": "Nostr Portable Identity Native Messaging Host",
  "path": "$NM_HOST_DIR/nostr-portable-nm-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://${EXTENSION_ID}/"
  ]
}
EOF
    echo "  Installed manifest: $dir/$MANIFEST_NAME"
}

echo "Installing native messaging manifests..."

if [ -d "$MANIFEST_DIR_CHROME" ] || [ ! -d "$MANIFEST_DIR_CHROMIUM" ]; then
    generate_manifest "$MANIFEST_DIR_CHROME"
fi

for dir in "$MANIFEST_DIR_CHROMIUM" "$MANIFEST_DIR_BRAVE" "$MANIFEST_DIR_EDGE" "$MANIFEST_DIR_VIVALDI"; do
    generate_manifest "$dir"
done

# Firefox support (uses a globally installed manifest)
FIREFOX_MANIFEST_DIR="/usr/lib/mozilla/native-messaging-hosts"
if [ -d "$FIREFOX_MANIFEST_DIR" ] || command -v firefox &>/dev/null; then
    sudo mkdir -p "$FIREFOX_MANIFEST_DIR"
    sudo bash -c "cat > '$FIREFOX_MANIFEST_DIR/$MANIFEST_NAME' << EOF
{
  \"name\": \"com.nostr.portable.identity\",
  \"description\": \"Nostr Portable Identity Native Messaging Host\",
  \"path\": \"$NM_HOST_DIR/nostr-portable-nm-host\",
  \"type\": \"stdio\",
  \"allowed_extensions\": [\"nostr-portable-identity@example.com\"]
}
EOF"
    echo "  Installed Firefox manifest: $FIREFOX_MANIFEST_DIR/$MANIFEST_NAME"
fi

echo ""
echo "Native messaging host installed successfully."
echo "Binary: $NM_HOST_DIR/nostr-portable-nm-host"
echo ""
echo "Next steps:"
echo "  1. Install the Chrome extension from the Chrome Web Store"
echo "  2. Ensure the extension ID matches the one in the manifests"
echo "  3. Restart your browser"
