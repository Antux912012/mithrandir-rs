# Mithrandir-RS (Rust Edition) 🦀

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
| **System Dashboard & Telemetry** | Full | Full (CPU, GPU, Network, Temp, Architecture) | ✅ Parity |
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

---

## 📦 Installation

Pre-built standalone packages are ready in the repository root:

### Debian / Ubuntu (`.deb`)
```bash
sudo dpkg -i mithrandir-rs_0.4.0_amd64.deb
```

### Flatpak Bundle (`.flatpak`)
```bash
flatpak install --user mithrandir-rs_0.4.0.flatpak
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
# Build both .deb and .flatpak packages
make all

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
