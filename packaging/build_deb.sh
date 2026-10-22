#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

VERSION=$(grep -m1 '^version' "${ROOT_DIR}/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')
DEB_PACKAGE="${ROOT_DIR}/mithrandir-rs_${VERSION}_amd64.deb"

# If not running in Debian container and dpkg-deb is missing, run inside podman builder
if ! command -v dpkg-deb &>/dev/null; then
  if command -v podman &>/dev/null; then
    echo "==> dpkg-deb not found on host. Running build inside Debian container..."
    podman run --rm -v "${ROOT_DIR}:/workspace:Z" -w /workspace localhost/mithrandir-debian-builder:latest \
      ./packaging/build_deb.sh
    exit 0
  else
    echo "ERROR: dpkg-deb not found and podman is unavailable." >&2
    exit 1
  fi
fi

echo "==> Building Mithrandir-RS Debian Package (v${VERSION})..."

# Ensure binary built on Debian exists
if [[ ! -f "${ROOT_DIR}/target-debian/release/mithrandir-rs" ]]; then
  echo "==> Compiling native Debian release binary with GTK4 and Libadwaita..."
  cargo build --release --features gui --target-dir "${ROOT_DIR}/target-debian"
fi

PKG_DIR="${ROOT_DIR}/target/pkg-deb"
rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/bin"
mkdir -p "${PKG_DIR}/usr/share/applications"
mkdir -p "${PKG_DIR}/usr/share/glib-2.0/schemas"
mkdir -p "${PKG_DIR}/usr/share/metainfo"
mkdir -p "${PKG_DIR}/usr/share/icons/hicolor"
mkdir -p "${PKG_DIR}/lib/udev/rules.d"

echo "==> Staging Debian package files..."
cp "${ROOT_DIR}/packaging/debian/control" "${PKG_DIR}/DEBIAN/control"
sed -i "s/^Version:.*/Version: ${VERSION}/" "${PKG_DIR}/DEBIAN/control"

install -m 755 "${ROOT_DIR}/target-debian/release/mithrandir-rs" "${PKG_DIR}/usr/bin/mithrandir-rs"
install -m 644 "${ROOT_DIR}/data/io.github.antonio.MithrandirRS.desktop" "${PKG_DIR}/usr/share/applications/io.github.antonio.MithrandirRS.desktop"
install -m 644 "${ROOT_DIR}/data/io.github.antonio.MithrandirRS.gschema.xml" "${PKG_DIR}/usr/share/glib-2.0/schemas/io.github.antonio.MithrandirRS.gschema.xml"
install -m 644 "${ROOT_DIR}/data/io.github.antonio.MithrandirRS.metainfo.xml" "${PKG_DIR}/usr/share/metainfo/io.github.antonio.MithrandirRS.metainfo.xml"
install -m 644 "${ROOT_DIR}/data/99-mithrandir-power.rules" "${PKG_DIR}/lib/udev/rules.d/99-mithrandir-power.rules"
cp -r "${ROOT_DIR}/data/icons/hicolor/"* "${PKG_DIR}/usr/share/icons/hicolor/"

echo "==> Packaging .deb with dpkg-deb..."
dpkg-deb --build --root-owner-group "${PKG_DIR}" "${DEB_PACKAGE}"

echo "==> Debian package generated successfully at:"
echo "    ${DEB_PACKAGE}"
