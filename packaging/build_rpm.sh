#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

VERSION=$(grep -m1 '^version' "${ROOT_DIR}/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')
echo "==> Staging Mithrandir-RS RPM build (v${VERSION})..."

# Check if release binary already exists or needs compilation
if [[ ! -f "${ROOT_DIR}/target/release/mithrandir-rs" ]]; then
  echo "==> Building Release Binary with GTK4 and Libadwaita..."
  if command -v cargo &>/dev/null && pkg-config --exists "gtk4 >= 4.12" "libadwaita-1 >= 1.5" 2>/dev/null; then
    cargo build --release --features gui
  elif command -v podman &>/dev/null; then
    echo "==> Compiling using Fedora container builder..."
    podman run --rm -v "${ROOT_DIR}:/workspace:Z" -w /workspace localhost/mithrandir-fedora-builder \
      cargo build --release --features gui
  elif command -v flatpak &>/dev/null && flatpak info org.gnome.Sdk//50 &>/dev/null; then
    flatpak run --user --devel --share=network --filesystem=home --command=sh org.gnome.Sdk//50 -c \
      "export PATH=\$HOME/.cargo/bin:\$PATH && cargo build --release --features gui"
  else
    echo "ERROR: No supported build toolchain found (host cargo + dev headers, podman, or flatpak org.gnome.Sdk//50)." >&2
    exit 1
  fi
fi

RPM_TOPDIR="${ROOT_DIR}/target/pkg-rpm"
rm -rf "${RPM_TOPDIR}"
mkdir -p "${RPM_TOPDIR}"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

echo "==> Staging sources and metadata..."
cp "${ROOT_DIR}/target/release/mithrandir-rs" "${RPM_TOPDIR}/SOURCES/mithrandir-rs"
cp -r "${ROOT_DIR}/data" "${RPM_TOPDIR}/SOURCES/data"

# Copy and update spec file with active version
cp "${ROOT_DIR}/packaging/rpm/mithrandir-rs.spec" "${RPM_TOPDIR}/SPECS/mithrandir-rs.spec"
sed -i "s/^Version:.*/Version:        ${VERSION}/" "${RPM_TOPDIR}/SPECS/mithrandir-rs.spec"

echo "==> Building RPM package with rpmbuild..."
rpmbuild -bb \
  --define "_topdir ${RPM_TOPDIR}" \
  --define "_sourcedir ${RPM_TOPDIR}/SOURCES" \
  "${RPM_TOPDIR}/SPECS/mithrandir-rs.spec"

# Find built RPM and copy to repository root
GENERATED_RPM=$(find "${RPM_TOPDIR}/RPMS" -type f -name "*.rpm" | head -n 1)

if [[ -f "${GENERATED_RPM}" ]]; then
  cp "${GENERATED_RPM}" "${ROOT_DIR}/mithrandir-rs_${VERSION}_x86_64.rpm"
  echo "==> RPM package generated successfully at:"
  echo "    ${ROOT_DIR}/mithrandir-rs_${VERSION}_x86_64.rpm"
else
  echo "ERROR: RPM build did not produce an .rpm artifact." >&2
  exit 1
fi
