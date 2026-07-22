#!/usr/bin/env bash
# Post-install script for Linux packages
# Called by: dpkg --postinst, rpm --postinst
set -euo pipefail

INSTALL_DIR="/usr/lib/nostr-portable-identity"
BIN_DIR="/usr/bin"
ICONS_DIR="/usr/share/icons/hicolor"
APPLICATIONS_DIR="/usr/share/applications"
AUTOSTART_DIR="/etc/xdg/autostart"

install_icon() {
    local size="$1"
    local icon_src="$INSTALL_DIR/icons/${size}x${size}.png"
    local icon_dst="$ICONS_DIR/${size}x${size}/apps/com.nostr.portable.identity.png"
    if [ -f "$icon_src" ]; then
        mkdir -p "$(dirname "$icon_dst")"
        cp "$icon_src" "$icon_dst"
    fi
}

echo "Nostr Portable Identity post-install"

# Create symlink
if [ -f "$INSTALL_DIR/nostr-portable-desktop" ]; then
    ln -sf "$INSTALL_DIR/nostr-portable-desktop" "$BIN_DIR/nostr-portable-desktop"
fi

# Install application icon
for size in 32 48 64 128 256; do
    install_icon "$size"
done
update-icon-caches /usr/share/icons/hicolor 2>/dev/null || true

# Install .desktop file
DESKTOP_FILE="$INSTALL_DIR/com.nostr.portable.identity.desktop"
if [ -f "$DESKTOP_FILE" ]; then
    cp "$DESKTOP_FILE" "$APPLICATIONS_DIR/"
    cp "$DESKTOP_FILE" "$AUTOSTART_DIR/"
fi

# Update desktop database
update-desktop-database "$APPLICATIONS_DIR" 2>/dev/null || true

echo "Installation complete."
echo "Start the application: nostr-portable-desktop"
