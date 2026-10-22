Name:           mithrandir-rs
Version:        0.4.1
Release:        1%{?dist}
Summary:        Modern GNU/Linux System Cleaner, Optimizer, Telemetry & Diagnostics in Rust

License:        GPL-3.0-or-later
URL:            https://github.com/Antux912012/mithrandir-rs

Requires:       gtk4 >= 4.12
Requires:       libadwaita >= 1.5
Requires:       glib2
Recommends:     papirus-icon-theme

%description
Mithrandir-RS is a blazing-fast, Libadwaita-based GNU/Linux maintenance utility
written in Rust. Features include instant one-click Smart Boost, real-time hardware
and memory telemetry, multi-threaded cache cleaner, process manager, systemd services
controller, package uninstaller, anti-malware quarantine vault, and local/cloud AI diagnostics.

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

install -m 755 %{_sourcedir}/mithrandir-rs %{buildroot}%{_bindir}/mithrandir-rs
install -m 644 %{_sourcedir}/data/io.github.antonio.MithrandirRS.desktop %{buildroot}%{_datadir}/applications/io.github.antonio.MithrandirRS.desktop
install -m 644 %{_sourcedir}/data/io.github.antonio.MithrandirRS.gschema.xml %{buildroot}%{_datadir}/glib-2.0/schemas/io.github.antonio.MithrandirRS.gschema.xml
install -m 644 %{_sourcedir}/data/io.github.antonio.MithrandirRS.metainfo.xml %{buildroot}%{_datadir}/metainfo/io.github.antonio.MithrandirRS.metainfo.xml
cp -r %{_sourcedir}/data/icons/hicolor/* %{buildroot}%{_datadir}/icons/hicolor/

%post
/usr/bin/glib-compile-schemas %{_datadir}/glib-2.0/schemas &> /dev/null || :
/usr/bin/update-desktop-database &> /dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &> /dev/null || :

%postun
/usr/bin/glib-compile-schemas %{_datadir}/glib-2.0/schemas &> /dev/null || :
/usr/bin/update-desktop-database &> /dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &> /dev/null || :

%files
%{_bindir}/mithrandir-rs
%{_datadir}/applications/io.github.antonio.MithrandirRS.desktop
%{_datadir}/glib-2.0/schemas/io.github.antonio.MithrandirRS.gschema.xml
%{_datadir}/metainfo/io.github.antonio.MithrandirRS.metainfo.xml
%{_datadir}/icons/hicolor/*/*/*.*

%changelog
* Thu Oct 22 2026 Antonio <antonio@localhost> - 0.4.1-1
- Release 0.4.1: Native Fedora RPM packaging and full multi-distro compatibility.
