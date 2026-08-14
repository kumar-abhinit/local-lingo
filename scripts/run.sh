#!/usr/bin/env bash
# Main LocalLingo entry: check deps, install if approved, launch dev app.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

cd "$ROOT"

if [[ ! -d node_modules ]]; then
  echo "Installing npm dependencies..."
  npm install
fi

bash "$ROOT/scripts/setup-deps.sh" --check

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found on PATH." >&2
  echo 'Run: source "$HOME/.cargo/env"' >&2
  exit 1
fi

exec npm run tauri dev
