#!/usr/bin/env bash
# Install Tauri/Linux build dependencies for LocalLingo (Ubuntu/Debian).
set -euo pipefail

echo "Installing LocalLingo Linux build dependencies (Debian/Ubuntu)..."

sudo apt-get update
sudo apt-get install -y \
  build-essential \
  cmake \
  clang \
  libclang-dev \
  pkg-config \
  libssl-dev \
  libdbus-1-dev \
  libglib2.0-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libasound2-dev \
  libx11-dev \
  libxkbcommon-dev

echo ""
echo "Done. Run: npm start"
