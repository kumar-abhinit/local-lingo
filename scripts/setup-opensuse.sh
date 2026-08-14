#!/usr/bin/env bash
# Install Tauri/Linux build dependencies for LocalLingo (openSUSE).
set -euo pipefail

echo "Installing LocalLingo Linux build dependencies (openSUSE)..."

sudo zypper install -y \
  webkit2gtk3-devel \
  libopenssl-devel \
  curl \
  wget \
  file \
  gcc \
  gcc-c++ \
  make \
  cmake \
  clang \
  pkg-config \
  gtk3-devel \
  libappindicator-gtk3-devel \
  librsvg-devel \
  alsa-devel \
  libX11-devel \
  libxkbcommon-devel \
  dbus-1-devel \
  glib2-devel

echo ""
echo "Done. Run: npm start"
