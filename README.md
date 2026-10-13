# Mithrandir-RS (Rust Edition) 🦀

**Mithrandir-RS** is a blazing-fast, modern GNU/Linux system cleaner, optimizer, telemetry monitor, and diagnostics utility rewritten in **Rust** with **GTK4** and **Libadwaita**.

Differentiated by its distinctive **Rust Terracotta `#B7410E`** visual theme, Mithrandir-RS brings native compiled speed, instant responsiveness, and minimal system footprint compared to the Python edition.

---

## ✨ Highlights & Features

- **Terracotta Theme (`#B7410E`)**: Distinctive Rust-branded color palette across HeaderBar, Navigation, cards, and app icons.
- **Instantaneous performance**: Native compiled binary with sub-20ms startup time.
- **Minimal memory footprint**: Runs at ~15–20 MB RSS (ideal for an always-on background optimizer).
- **Parallel multi-threaded scanning**: Powered by `rayon` to scan tens of thousands of cache entries across CPU cores in milliseconds.
- **Smart Boost**: Reclaims standby kernel pagecache buffers (`drop_caches`), compacts memory, and cleans temporary file clutter.
- **Hardware Telemetry**: Real-time CPU, RAM, disk, and sensor monitoring via `sysinfo`.
- **Parallel Cleaner Engine**: High-speed, protected multi-threaded file cleaner with strict root safety guards.
- **GNOME Integration**: D-Bus deep-linking, Libadwaita adaptive layouts, and desktop notifications.

---

## 📦 Installation

Pre-built standalone packages are ready in the repository root:

### Debian / Ubuntu (`.deb`)
```bash
sudo dpkg -i mithrandir-rs_0.1.0_amd64.deb
```

### Flatpak Bundle (`.flatpak`)
```bash
flatpak install --user mithrandir-rs_0.1.0.flatpak
```
Run via Flatpak:
```bash
flatpak run io.github.antonio.MithrandirRS
```

---

## 🚀 CLI Usage

Mithrandir-RS can be run as a full Libadwaita GUI app or invoked directly from the terminal and script hooks:

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

### Manual Cargo Build
```bash
# Backend CLI only
cargo build --release

# Full GTK4 + Libadwaita GUI (using GNOME 50 SDK)
flatpak run --devel --filesystem=home --command=sh org.gnome.Sdk//50 -c \
  "export PATH=\$HOME/.cargo/bin:\$PATH && cargo build --release --features gui"
```
