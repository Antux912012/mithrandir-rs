# Mithrandir (Rust Edition) 🦀

**Mithrandir-RS** is a blazing-fast, modern GNU/Linux system cleaner, optimizer, telemetry monitor, and diagnostics utility rewritten in **Rust** with **GTK4** and **Libadwaita**.

Inspired by tools like Microsoft PC Manager, Stacer, and BleachBit, but built natively with modern Linux desktop standards:
- **Instantaneous performance**: Native compiled machine code with < 30ms startup time.
- **Minimal memory footprint**: Runs at ~15–20 MB RSS (ideal for an always-on background optimizer).
- **Parallel multi-threaded scanning**: Powered by `rayon` to scan tens of thousands of cache entries across CPU cores in milliseconds.
- **GNOME HIG & Libadwaita**: Adaptive layouts, dynamic color palettes, and StatusNotifierItem top bar integration.

---

## ⚡ Architecture & Features

- **Smart Boost**: Reclaims standby kernel pagecache buffers (`drop_caches`), compacts memory, and cleans temporary file clutter.
- **System Telemetry**: Real-time CPU, RAM, disk, and sensor monitoring via `sysinfo`.
- **Parallel Cleaner Engine**: High-speed, protected multi-threaded file cleaner with strict root safety guards.
- **Background Service & GNOME Applet**: Native FreeDesktop `StatusNotifierItem` and `DBusMenu` panel integration via `zbus`.
- **Desktop Notifications**: Standard GNOME Shell notifications upon automated and manual performance boosts.

---

## 🚀 Usage

### Command Line Options

```bash
# Perform instant one-click performance boost
mithrandir-rs --smart-boost

# View live system telemetry
mithrandir-rs --monitor

# Scan cleanable clutter and caches
mithrandir-rs --clean
```

---

## 🛠️ Building & Developing

### Requirements
- Rust 1.80+ (`cargo`, `rustc`)
- For GUI build: `libgtk-4-dev` and `libadwaita-1-dev` (or via Flatpak `org.gnome.Sdk:50`)

```bash
# Build the core engine & CLI
cargo build --release

# Run integration tests
cargo test
```
