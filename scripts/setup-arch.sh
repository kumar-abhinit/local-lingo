#!/usr/bin/env bash
# Install Tauri/Linux build dependencies for LocalLingo (Arch/Manjaro).
set -euo pipefail

echo "Installing LocalLingo Linux build dependencies (Arch)..."

sudo pacman -S --needed --noconfirm \
  base-devel \
  cmake \
  clang \
  pkg-config \
  openssl \
  curl \
  wget \
  file \
  webkit2gtk-4.1 \
  gtk3 \
  libappindicator-gtk3 \
  librsvg \
  alsa-lib \
  libx11 \
  libxkbcommon \
  dbus \
  glib2

echo ""
echo "Done. Run: npm start"
