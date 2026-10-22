<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.antonio.MithrandirRS.svg" width="128" height="128" alt="Mithrandir-RS Application Icon" />
</p>

<h1 align="center">Mithrandir-RS (Rust Edition) 🦀</h1>

<p align="center">
  <strong>Native GTK4 & Libadwaita GNU/Linux System Optimizer, Deep Cleaner, and Performance Booster</strong><br>
  <em>Blazing-fast native execution, instant startup, and low memory overhead written in Rust.</em>
</p>

<p align="center">
  <a href="#-installation"><img src="https://img.shields.io/badge/version-0.4.2--preview-orange.svg" alt="Version" /></a>
  <a href="https://www.gnu.org/licenses/gpl-3.0"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg" alt="License: GPL-3.0-or-later" /></a>
  <a href="https://gtk.org/"><img src="https://img.shields.io/badge/GUI-GTK%204%20%7C%20Libadwaita-brightgreen.svg" alt="GTK4 & Libadwaita" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.80%2B-informational.svg" alt="Rust 1.80+" /></a>
  <a href="https://getfedora.org/"><img src="https://img.shields.io/badge/Fedora-Compatible-51A2DA?logo=fedora&logoColor=white" alt="Fedora Compatible" /></a>
  <a href="https://github.com/Antux912012/mithrandir-rs"><img src="https://img.shields.io/badge/Platform-GNU%2FLinux-orange.svg" alt="Platform" /></a>
</p>

---

> [!IMPORTANT]
> **Project Roadmap & Deprecation Notice:**  
> The standard Python edition of [**Mithrandir**](https://github.com/Antux912012/mithrandir) is planned to be deprecated as development focus transitions to **Mithrandir-RS**. All future feature improvements, system integrations, and performance optimizations will be delivered in this Rust codebase.  
> At a future milestone, Mithrandir-RS will graduate from its preview designation and be officially rebranded as either **Mithrandir** or **Radagast** 🧙‍♂️.

**Mithrandir-RS** is a blazing-fast, native system optimizer, deep cleaner, telemetry monitor, and hardware power saver written in **Rust** with **GTK4** and **Libadwaita**, engineered for instant startup, minimal memory footprint, and robust multi-distribution compatibility.

Differentiated by its distinctive **Rust Terracotta `#B7410E`** visual theme, Mithrandir-RS explores native compiled speed while progressive feature parity with the main project is being developed.

> [!NOTE]
> **What's New in v0.4.2:**
> - 🛡️ **Pure Rust System Tray & Background Daemon**: StatusNotifierItem (SNI) & DBusMenu implementation via `ksni` with zero GTK3 runtime collisions.
> - ⚡ **Hardware Power Saver Mode**: Bidirectional control over Intel P-State CPU Turbo (`no_turbo`), CPU EPP profiles, and PCIe ASPM deep powersaving synced with the desktop panel.
> - 🎩 **Cross-Distribution Packaging**: Standalone packages for Fedora/RHEL (`.rpm`), Debian/Ubuntu (`.deb`), Arch Linux (`AUR / PKGBUILD`), and universal Flatpak (`.flatpak`).
> - ⚙️ **Passwordless Udev Rules**: Automatic rule generator for unprivileged power switching (`wheel` on Fedora/Arch, `sudo` on Debian/Ubuntu).

---

## ⚡ Current Preview Status

| Feature / Area | Original [Mithrandir](https://github.com/Antux912012/mithrandir) | Mithrandir-RS (Rust) | Status |
| :--- | :--- | :--- | :--- |
| **System Dashboard & Telemetry** | Full | Full (CPU, Dual GPU, Network Rx/Tx, Thermal Zones, Battery Wh/Health, Architecture) | ✅ Parity |
| **Smart Boost (RAM & Cache trim)**| Full | Full (Debounced, Re-entrancy Guard, Cooldown) | ✅ Parity |
| **Multi-Threaded Cache Cleaner** | Full | Full (Thumbnails, Browser, Stremio, Dev, Trash, Journal) | ✅ Parity |
| **Process & Task Explorer** | Full (Stacer style) | Live accurate CPU% & RAM, Search, Sorting | ✅ Parity |
| **Startup Applications** | Full | Desktop autostart management | ✅ Parity |
| **Systemd Services Manager** | Full with unit logs | Unit inspection & control | ✅ Parity |
| **Package Uninstaller** | APT, DNF, Pacman, Flatpak, Snap, pip | Multi-distro package management & uninstaller | ✅ Parity |
| **System Tray & Background Mode** | AppIndicator (GTK3) | Pure Rust StatusNotifierItem (SNI) + DBusMenu (GTK4 clean, no GTK3 collision) | ✅ Parity |
| **Hardware Power Saver** | TLP / Power Profiles | Hardware CPU Turbo/EPP + PCIe ASPM Powersupersave & Tray Sync | ✅ Parity |
| **Anti-Malware (ClamAV)** | Full scanner & signatures | 1-Click Installer, FreshClam Lock Fix, Quarantine Vault | ✅ Parity |
| **BTRFS Snapshots** | Automated snapshots & restore | Snapshot management & subvolumes | ✅ Parity |
| **GRUB Bootloader** | Full timeout & kernel select | Timeout & boot configuration | ✅ Parity |
| **AI Diagnostics & Local LLM** | Ollama, NIM, OpenAI, in-app Qwen | Built-in offline assistant, Qwen 2.5 + llama.cpp engine | ✅ Parity |

### 🎩 Fedora & RHEL Compatibility Highlights

Mithrandir-RS includes dedicated compatibility support for Fedora and enterprise Red Hat derivatives:
- **Native `.rpm` Packaging**: Pre-built standalone `.rpm` packages and built-in `make rpm` target using `rpmbuild`.
- **Package Manager Cache Cleaner**: Scans and cleans both `/var/cache/libdnf5` and `/var/cache/dnf` caches.
- **Fedora GRUB2 Support**: Full support for Fedora's `/boot/grub2/grub.cfg` configuration scheme.
- **ClamAV Integration**: Automatic detection and installation of Fedora's `clamav`, `clamav-freshclam`, and `clamd` packages.
- **Default Btrfs Snapshot Support**: Integrates directly with Fedora's default Btrfs subvolumes and snapshot management.
- **Fedora-Aware Diagnostics**: Built-in AI diagnostic commands tailored for `dnf` and `systemd`.

### 🛡️ System Tray & Background Architecture (GTK4 + Pure Rust D-Bus)

Standard `libappindicator` links against GTK3 (`GtkMenu`), which causes fatal runtime symbol conflicts when loaded into a GTK4 process. Mithrandir-RS solves this natively:
- **Pure Rust D-Bus SNI (`ksni`)**: Implements `org.kde.StatusNotifierItem` and `com.canonical.dbusmenu` entirely in Rust without GTK3 C bindings.
- **Freedesktop Symbolic Icons**: 16x16 monochrome symbolic icon (`mithrandir-status-symbolic.svg`) with transparent background that automatically adapts to both dark and light desktop panels.
- **Seamless Window Interception**: Closing the window hides it (`set_hide_on_close(true)`) while keeping the `GApplication` event loop active via `app.hold()`.
- **Tray Menu Actions**:
  1. **Run Smartboost**: Triggers RAM and disk cache reclamation with desktop notifications and re-entrancy locking.
  2. **Battery Power Saver**: Hardware-level power optimization toggle synced bidirectionally between the GTK4 UI and panel menu.
  3. **Open Mithrandir**: Restores and focuses the main Libadwaita window.
  4. **Quit Mithrandir**: Cleanly shuts down background threads and exits the application.
- **Autostart & Background Launch**: Pass `--background` or `--minimized` to initialize the app in the background without popping up the main window.

### ⚡ Hardware Power Saver & Multi-Distro Adaptability (`src/os_info.rs`)

Mithrandir-RS includes hardware-level battery and power saving controls synced across the Libadwaita UI and the panel indicator:
- **Intel P-State & CPU EPP**: Toggles `intel_pstate/no_turbo` (`"1"` vs `"0"`) and dynamically switches CPU Energy Performance Preference (`energy_performance_preference`) between `"power"` and `"balance_performance"`.
- **PCIe ASPM**: Switches PCIe Active State Power Management policy to `"powersupersave"` for deep NVMe and bus power management.
- **Zero-Password Udev Optimization**: Generates `99-mithrandir-power.rules` dynamically targeting the correct administrative group (`wheel` for Fedora/Arch/openSUSE, `sudo` for Debian/Ubuntu).
- **SELinux & Immutability Awareness**: Native `/etc/os-release` detection (`src/os_info.rs`) automatically recognizes NixOS and immutable trees, gracefully skipping direct `/etc` modifications with clear user guidance.

---

## 📦 Installation

Pre-built standalone packages are ready in the repository root:

### Debian / Ubuntu (`.deb`)
```bash
sudo dpkg -i mithrandir-rs_0.4.2_amd64.deb
```

### Fedora / RHEL (`.rpm`)
```bash
sudo dnf install ./mithrandir-rs_0.4.2_x86_64.rpm
```

### Flatpak Bundle (`.flatpak`)
```bash
flatpak install --user mithrandir-rs_0.4.2.flatpak
```
Run via Flatpak:
```bash
flatpak run io.github.antonio.MithrandirRS
```

### Arch Linux / CachyOS (AUR)
```bash
# Using an AUR helper
yay -S mithrandir-rs
# Or building manually from packaging/aur
cd packaging/aur && makepkg -si
```

---

## 🚀 CLI Usage

Mithrandir-RS can be run as a full Libadwaita GUI app or invoked directly from the terminal:

```bash
# Launch the Libadwaita GUI
mithrandir-rs

# Launch minimized directly into the system tray / background
mithrandir-rs --background

# Perform instant one-click performance boost
mithrandir-rs --smart-boost

# View live system telemetry
mithrandir-rs --monitor

# Scan cleanable clutter and caches
mithrandir-rs --clean
```

---

## 🛠️ Building from Source

### Quick Build with Makefile
```bash
# Build .deb, .rpm, and .flatpak packages
make all

# Build only the Fedora RPM package
make rpm

# Build only the Debian package
make deb

# Build only the Flatpak bundle
make flatpak

# Run backend test suite
make test
```

---

## 🔮 Project Roadmap & Future Rebranding

- **Deprecation of Standard Mithrandir**: The legacy Python/GTK4 version ([Mithrandir](https://github.com/Antux912012/mithrandir)) will be deprecated in favor of this native Rust implementation.
- **Continuous Improvements**: Active feature development, deep hardware integrations, system cleaners, and package management will continue exclusively in Mithrandir-RS.
- **Future Rebranding**: Once feature maturity and full stability are consolidated, this project will be officially rebranded as either **Mithrandir** or **Radagast** 🧙‍♂️.

---

## 🔗 Related Projects

- **[Mithrandir (Original Python / Libadwaita Edition)](https://github.com/Antux912012/mithrandir)**: The previous Python reference implementation (entering deprecation).
