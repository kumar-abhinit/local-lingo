#!/usr/bin/env bash
# Detect OS, probe missing build dependencies, optionally install with user approval.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-}"

detect_distro() {
  if [[ ! -f /etc/os-release ]]; then
    echo "unknown"
    return
  fi
  # shellcheck disable=SC1091
  source /etc/os-release
  local id="${ID:-unknown}"
  local like="${ID_LIKE:-}"

  case "$id" in
    fedora|rhel|centos|rocky|almalinux) echo "fedora" ;;
    debian|ubuntu|linuxmint|pop) echo "debian" ;;
    arch|manjaro|endeavouros) echo "arch" ;;
    opensuse*|sles) echo "opensuse" ;;
    *)
      if [[ "$like" == *fedora* ]] || [[ "$like" == *rhel* ]]; then
        echo "fedora"
      elif [[ "$like" == *debian* ]] || [[ "$like" == *ubuntu* ]]; then
        echo "debian"
      elif [[ "$like" == *arch* ]]; then
        echo "arch"
      elif [[ "$like" == *suse* ]]; then
        echo "opensuse"
      else
        echo "unknown"
      fi
      ;;
  esac
}

is_linux() {
  [[ "$(uname -s)" == "Linux" ]]
}

missing_cmds=()
missing_pkgs=()

probe_cmd() {
  local label="$1"
  local cmd="$2"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    missing_cmds+=("$label")
  fi
}

probe_pkgconfig() {
  local label="$1"
  local pkg="$2"
  if ! pkg-config --exists "$pkg" 2>/dev/null; then
    missing_pkgs+=("$label")
  fi
}

check_linux_deps() {
  missing_cmds=()
  missing_pkgs=()

  probe_cmd "cmake" cmake
  probe_cmd "clang" clang
  probe_cmd "pkg-config" pkg-config
  probe_cmd "gcc" gcc
  probe_cmd "g++" g++

  probe_pkgconfig "webkit2gtk-4.1" webkit2gtk-4.1
  probe_pkgconfig "glib-2.0" glib-2.0
  probe_pkgconfig "gtk+-3.0" gtk+-3.0
  probe_pkgconfig "alsa" alsa
  probe_pkgconfig "xkbcommon" xkbcommon
  probe_pkgconfig "openssl" openssl
}

check_toolchain() {
  TOOLCHAIN_MISSING=()
  if ! command -v node >/dev/null 2>&1; then
    TOOLCHAIN_MISSING+=("Node.js — https://nodejs.org/")
  fi
  if ! command -v npm >/dev/null 2>&1; then
    TOOLCHAIN_MISSING+=("npm (bundled with Node.js)")
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    TOOLCHAIN_MISSING+=("Rust (cargo) — install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh")
  fi
}

install_for_distro() {
  local distro="$1"
  case "$distro" in
    debian) bash "$ROOT/scripts/setup-linux.sh" ;;
    fedora) bash "$ROOT/scripts/setup-fedora.sh" ;;
    arch) bash "$ROOT/scripts/setup-arch.sh" ;;
    opensuse) bash "$ROOT/scripts/setup-opensuse.sh" ;;
    *)
      echo "Unsupported Linux distribution: $distro"
      echo "See: https://v2.tauri.app/start/prerequisites/"
      return 1
      ;;
  esac
}

prompt_install_rust() {
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi
  echo ""
  echo "Rust (cargo) is not installed."
  read -r -p "Install Rust via rustup now? [y/N]: " answer
  case "${answer:-N}" in
    y|Y|yes|YES)
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      # shellcheck disable=SC1091
      source "$HOME/.cargo/env"
      ;;
    *)
      echo "Install Rust manually, then re-run: npm start"
      exit 1
      ;;
  esac
}

run_check() {
  local distro
  distro="$(detect_distro)"

  if ! is_linux; then
    check_toolchain
    if ((${#TOOLCHAIN_MISSING[@]} > 0)); then
      echo "Missing toolchain:"
      for item in "${TOOLCHAIN_MISSING[@]}"; do
        echo "  - $item"
      done
      echo ""
      echo "See: https://v2.tauri.app/start/prerequisites/"
      exit 1
    fi
    return 0
  fi

  check_linux_deps
  check_toolchain

  local total=$(( ${#missing_cmds[@]} + ${#missing_pkgs[@]} + ${#TOOLCHAIN_MISSING[@]} ))

  if (( total == 0 )); then
    return 0
  fi

  echo ""
  echo "Missing dependencies detected (${distro}):"
  local i=1
  for item in "${missing_cmds[@]}" "${missing_pkgs[@]}"; do
    echo "  $((i++)). $item"
  done
  for item in "${TOOLCHAIN_MISSING[@]}"; do
    echo "  $((i++)). $item"
  done
  echo ""

  if [[ "${LOCAL_LINGO_SKIP_DEPS:-}" == "1" ]]; then
    echo "LOCAL_LINGO_SKIP_DEPS=1 — skipping install prompt."
    return 0
  fi

  if ((${#missing_cmds[@]} + ${#missing_pkgs[@]} > 0)); then
    read -r -p "Install system packages now? (requires sudo) [y/N]: " answer
    case "${answer:-N}" in
      y|Y|yes|YES)
        install_for_distro "$distro"
        ;;
      *)
        echo ""
        echo "Manual install:"
        case "$distro" in
          debian) echo "  ./scripts/setup-linux.sh" ;;
          fedora) echo "  ./scripts/setup-fedora.sh" ;;
          arch) echo "  ./scripts/setup-arch.sh" ;;
          opensuse) echo "  ./scripts/setup-opensuse.sh" ;;
          *) echo "  https://v2.tauri.app/start/prerequisites/" ;;
        esac
        exit 1
        ;;
    esac
  fi

  prompt_install_rust
}

if [[ "$MODE" == "--check" ]] || [[ "$MODE" == "--install" ]]; then
  run_check
else
  echo "Usage: $0 --check"
  exit 1
fi
