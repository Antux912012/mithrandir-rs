#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

echo "==> Building Release Binary with GNOME Sdk..."
flatpak run --user --devel --share=network --filesystem=home --command=sh org.gnome.Sdk//50 -c \
  "export PATH=\$HOME/.cargo/bin:\$PATH && cargo build --release --features gui"

PKG_DIR="${ROOT_DIR}/target/pkg-deb"
rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/bin"
mkdir -p "${PKG_DIR}/usr/share/applications"
mkdir -p "${PKG_DIR}/usr/share/glib-2.0/schemas"
mkdir -p "${PKG_DIR}/usr/share/icons/hicolor"

echo "==> Staging Debian Package..."
cp "${ROOT_DIR}/packaging/debian/control" "${PKG_DIR}/DEBIAN/control"
install -m 755 "${ROOT_DIR}/target/release/mithrandir-rs" "${PKG_DIR}/usr/bin/mithrandir-rs"
install -m 644 "${ROOT_DIR}/data/io.github.antonio.MithrandirRS.desktop" "${PKG_DIR}/usr/share/applications/io.github.antonio.MithrandirRS.desktop"
install -m 644 "${ROOT_DIR}/data/io.github.antonio.MithrandirRS.gschema.xml" "${PKG_DIR}/usr/share/glib-2.0/schemas/io.github.antonio.MithrandirRS.gschema.xml"
cp -r "${ROOT_DIR}/data/icons/hicolor/"* "${PKG_DIR}/usr/share/icons/hicolor/"

echo "==> Packaging .deb..."
dpkg-deb --build --root-owner-group "${PKG_DIR}" "${ROOT_DIR}/mithrandir-rs_0.4.1_amd64.deb"

echo "==> Debian package generated at ${ROOT_DIR}/mithrandir-rs_0.4.1_amd64.deb"
