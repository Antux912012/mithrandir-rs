<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.antonio.MithrandirRS.svg" width="128" height="128" alt="Mithrandir-RS Application Icon" />
</p>

<h1 align="center">Mithrandir-RS (Rust Edition) 🦀</h1>

<p align="center">
  <strong>Native GTK4 & Libadwaita GNU/Linux System Optimizer, Deep Cleaner, and Performance Booster</strong><br>
  <em>Blazing-fast native execution, instant startup, and low memory overhead written in Rust.</em>
</p>

<p align="center">
  <a href="#-installation"><img src="https://img.shields.io/badge/version-0.4.1--preview-orange.svg" alt="Version" /></a>
  <a href="https://www.gnu.org/licenses/gpl-3.0"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg" alt="License: GPL-3.0-or-later" /></a>
  <a href="https://gtk.org/"><img src="https://img.shields.io/badge/GUI-GTK%204%20%7C%20Libadwaita-brightgreen.svg" alt="GTK4 & Libadwaita" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.80%2B-informational.svg" alt="Rust 1.80+" /></a>
  <a href="https://getfedora.org/"><img src="https://img.shields.io/badge/Fedora-Compatible-51A2DA?logo=fedora&logoColor=white" alt="Fedora Compatible" /></a>
  <a href="https://github.com/Antux912012/mithrandir-rs"><img src="https://img.shields.io/badge/Platform-GNU%2FLinux-orange.svg" alt="Platform" /></a>
</p>

---

> [!WARNING]
> **Mithrandir-RS is an EXPERIMENTAL preview release.**  
> This Rust port is under active early development. Many features and deep system integrations from the original Mithrandir are still missing or partially implemented.  
> If you are looking for the full-featured, mature, and stable version with complete system maintenance tools and package managers, please use the original [**Mithrandir (Python & Libadwaita Edition)**](https://github.com/Antux912012/mithrandir).

**Mithrandir-RS** is an experimental, blazing-fast exploration rewriting the core of Mithrandir in **Rust** with **GTK4** and **Libadwaita**, focusing on native execution speed, instantaneous startup times, and minimal memory overhead.

Differentiated by its distinctive **Rust Terracotta `#B7410E`** visual theme, Mithrandir-RS explores native compiled speed while progressive feature parity with the main project is being developed.

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

---

## 📦 Installation

Pre-built standalone packages are ready in the repository root:

### Debian / Ubuntu (`.deb`)
```bash
sudo dpkg -i mithrandir-rs_0.4.1_amd64.deb
```

### Fedora / RHEL (`.rpm`)
```bash
sudo dnf install ./mithrandir-rs_0.4.1_x86_64.rpm
```

### Flatpak Bundle (`.flatpak`)
```bash
flatpak install --user mithrandir-rs_0.4.1.flatpak
```
Run via Flatpak:
```bash
flatpak run io.github.antonio.MithrandirRS
```

---

## 🚀 CLI Usage

Mithrandir-RS can be run as a full Libadwaita GUI app or invoked directly from the terminal:

```bash
# Launch the Libadwaita GUI
mithrandir-rs

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

## 🔗 Related Projects

- **[Mithrandir (Original Python / Libadwaita Edition)](https://github.com/Antux912012/mithrandir)**: The primary, production-ready GNU/Linux system cleaner, optimizer, telemetry, and diagnostics suite.
