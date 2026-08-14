#!/usr/bin/env bash
# Install Tauri/Linux build dependencies for LocalLingo (Fedora/RHEL/CentOS).
set -euo pipefail

echo "Installing LocalLingo Linux build dependencies (Fedora/RHEL)..."

if command -v dnf >/dev/null 2>&1; then
  PKG=dnf
elif command -v yum >/dev/null 2>&1; then
  PKG=yum
else
  echo "error: neither dnf nor yum found" >&2
  exit 1
fi

sudo "$PKG" install -y \
  webkit2gtk4.1-devel \
  openssl-devel \
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
  librsvg2-devel \
  alsa-lib-devel \
  libX11-devel \
  libxkbcommon-devel \
  dbus-devel \
  glib2-devel

echo ""
echo "Done. Run: npm start"
