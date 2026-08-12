#!/usr/bin/env bash
# Install LocalLingo into the user application menu (~/.local/share/applications).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APPS_DIR="$HOME/.local/share/applications"
ICONS_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"
DESKTOP_FILE="$APPS_DIR/local-lingo.desktop"
LAUNCH_SCRIPT="$ROOT/scripts/launch-local-lingo.sh"
SOURCE_DESKTOP="$ROOT/desktop/local-lingo.desktop"
SOURCE_ICON="$ROOT/src-tauri/icons/128x128.png"

if [[ ! -f "$SOURCE_DESKTOP" ]]; then
  echo "error: missing desktop template at $SOURCE_DESKTOP" >&2
  exit 1
fi

if [[ ! -f "$SOURCE_ICON" ]]; then
  echo "error: missing icon at $SOURCE_ICON" >&2
  exit 1
fi

chmod +x "$LAUNCH_SCRIPT"

mkdir -p "$APPS_DIR" "$ICONS_DIR"
cp "$SOURCE_ICON" "$ICONS_DIR/local-lingo.png"
sed "s|__LAUNCH_SCRIPT__|$LAUNCH_SCRIPT|g" "$SOURCE_DESKTOP" > "$DESKTOP_FILE"
chmod 644 "$DESKTOP_FILE"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR"
fi

echo "Installed desktop launcher:"
echo "  $DESKTOP_FILE"
echo "Search for \"LocalLingo\" in your app menu."
