#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

echo "==> Building Release Binary with GNOME Sdk..."
flatpak run --user --devel --filesystem=home --command=sh org.gnome.Sdk//50 -c \
  "export PATH=\$HOME/.cargo/bin:\$PATH && cargo build --release --features gui"

echo "==> Building Flatpak in OSTree repository..."
flatpak-builder --force-clean --repo=repo-rs build-dir-rs io.github.antonio.MithrandirRS.json

echo "==> Exporting Flatpak Single-File Bundle..."
flatpak build-bundle repo-rs "${ROOT_DIR}/mithrandir-rs_0.4.0.flatpak" io.github.antonio.MithrandirRS master

echo "==> Flatpak bundle generated at ${ROOT_DIR}/mithrandir-rs_0.4.0.flatpak"
