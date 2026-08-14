#!/usr/bin/env bash
# Run LocalLingo dev server with Rust/Cargo on PATH.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found." >&2
  echo "Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
  echo "Then run: source \"\$HOME/.cargo/env\"" >&2
  exit 1
fi

cd "$ROOT"
exec bash "$ROOT/scripts/run.sh"
