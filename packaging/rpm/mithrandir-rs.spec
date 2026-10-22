Name:           mithrandir-rs
Version:        0.4.4
Release:        1%{?dist}
Summary:        Native GTK4 & Libadwaita GNU/Linux System Optimizer, Deep Cleaner, and Hardware Power Saver in Rust

License:        GPL-3.0-or-later
URL:            https://github.com/Antux912012/mithrandir-rs

Requires:       gtk4 >= 4.12
Requires:       libadwaita >= 1.5
Requires:       glib2
Recommends:     papirus-icon-theme

%description
Mithrandir-RS is a blazing-fast, Libadwaita-based GNU/Linux maintenance utility
written in Rust. Features include instant one-click Smart Boost, real-time hardware
and memory telemetry, multi-threaded cache cleaner, hardware power saver mode
(Intel P-State CPU Turbo/EPP & PCIe ASPM), pure Rust D-Bus system tray panel icon,
process manager, systemd services controller, package uninstaller, anti-malware
quarantine vault, and local/cloud AI diagnostics.

%prep
# No-op: staged from pre-built artifacts in build_rpm.sh

%build
# No-op: binary already compiled

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/glib-2.0/schemas
mkdir -p %{buildroot}%{_datadir}/metainfo
mkdir -p %{buildroot}%{_datadir}/icons/hicolor
mkdir -p %{buildroot}%{_prefix}/lib/udev/rules.d

install -m 755 %{_sourcedir}/mithrandir-rs %{buildroot}%{_bindir}/mithrandir-rs
install -m 644 %{_sourcedir}/data/io.github.antonio.MithrandirRS.desktop %{buildroot}%{_datadir}/applications/io.github.antonio.MithrandirRS.desktop
install -m 644 %{_sourcedir}/data/io.github.antonio.MithrandirRS.gschema.xml %{buildroot}%{_datadir}/glib-2.0/schemas/io.github.antonio.MithrandirRS.gschema.xml
install -m 644 %{_sourcedir}/data/io.github.antonio.MithrandirRS.metainfo.xml %{buildroot}%{_datadir}/metainfo/io.github.antonio.MithrandirRS.metainfo.xml
install -m 644 %{_sourcedir}/data/99-mithrandir-power.rules %{buildroot}%{_prefix}/lib/udev/rules.d/99-mithrandir-power.rules
cp -r %{_sourcedir}/data/icons/hicolor/* %{buildroot}%{_datadir}/icons/hicolor/

%post
/usr/bin/glib-compile-schemas %{_datadir}/glib-2.0/schemas &> /dev/null || :
/usr/bin/update-desktop-database &> /dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &> /dev/null || :
echo "==> Mithrandir-RS v0.4.4 installed successfully!"
echo "==> Native GTK4/Libadwaita System Optimizer, Cleaner & Hardware Power Saver."
echo "==> Launch with 'mithrandir-rs' or start in system tray with 'mithrandir-rs --background'."

%postun
/usr/bin/glib-compile-schemas %{_datadir}/glib-2.0/schemas &> /dev/null || :
/usr/bin/update-desktop-database &> /dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &> /dev/null || :

%files
%{_bindir}/mithrandir-rs
%{_datadir}/applications/io.github.antonio.MithrandirRS.desktop
%{_datadir}/glib-2.0/schemas/io.github.antonio.MithrandirRS.gschema.xml
%{_datadir}/metainfo/io.github.antonio.MithrandirRS.metainfo.xml
%{_prefix}/lib/udev/rules.d/99-mithrandir-power.rules
%{_datadir}/icons/hicolor/*/*/*.*

%changelog
* Thu Oct 22 2026 Antonio <antonio@localhost> - 0.4.4-1
- Release 0.4.4: Major codebase optimizations, persistent preferences, zero-allocation process sorting, and dead-lock prevention.
* Thu Oct 22 2026 Antonio <antonio@localhost> - 0.4.2-1
- Release 0.4.2: Hardware Power Saver, pure Rust D-Bus system tray, and multi-distro adaptability.
* Thu Oct 22 2026 Antonio <antonio@localhost> - 0.4.1-1
- Release 0.4.1: Native Fedora RPM packaging and full multi-distro compatibility.
