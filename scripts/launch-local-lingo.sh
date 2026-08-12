#!/usr/bin/env bash
# Launch LocalLingo from the desktop entry (release binary preferred).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

RELEASE="$ROOT/src-tauri/target/release/local-lingo"
DEBUG="$ROOT/src-tauri/target/debug/local-lingo"

if [[ -x "$RELEASE" ]]; then
  exec "$RELEASE" --show-settings
fi

if [[ -x "$DEBUG" ]]; then
  exec "$DEBUG" --show-settings
fi

exec "$ROOT/scripts/dev.sh"
