use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ProgressBar, ScrolledWindow};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use crate::backend::monitor::{CpuInfo, DiskPartition, GpuInfo, MemoryInfo, NetworkRate, SystemMonitor, SystemSummary};
use crate::backend::smart_boost::{BoostStatus, SmartBoostEngine};

struct TelemetrySnapshot {
    summary: SystemSummary,
    cpu: CpuInfo,
    mem: MemoryInfo,
    disks: Vec<DiskPartition>,
    net: NetworkRate,
    gpu: Option<GpuInfo>,
    boost_status: BoostStatus,
}

pub struct DashboardPage;

impl DashboardPage {
    pub fn build() -> ScrolledWindow {
        let scroller = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .kinetic_scrolling(true)
            .build();

        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(20)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        // 1. System Architecture Summary Banner
        let summary_card = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .build();
        summary_card.add_css_class("metric-card");

        let summary_left = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .hexpand(true)
            .build();

        let initial_summary = SystemMonitor::get_system_summary();
        let host_lbl = Label::builder()
            .label(&format!("💻 {} | {}", initial_summary.hostname, initial_summary.distro))
            .xalign(0.0)
            .build();
        host_lbl.add_css_class("title-3");

        let host_sub = Label::builder()
            .label(&format!("Kernel: {} | Desktop: {} | Uptime: {}", initial_summary.kernel, initial_summary.desktop, initial_summary.uptime_str))
            .xalign(0.0)
            .build();
        host_sub.add_css_class("dim-label");

        summary_left.append(&host_lbl);
        summary_left.append(&host_sub);
        summary_card.append(&summary_left);
        root_box.append(&summary_card);

        // 2. Smart Boost Hero Card
        let card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .build();
        card.add_css_class("smart-boost-card");

        let title_row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let lbl_title = Label::builder()
            .label("⚡ Smart Boost (Rust Engine)")
            .xalign(0.0)
            .hexpand(true)
            .build();
        lbl_title.add_css_class("title-2");
        title_row.append(&lbl_title);

        let boost_btn = Button::builder()
            .label("🚀 Boost")
            .valign(Align::Center)
            .build();
        boost_btn.add_css_class("suggested-action");
        boost_btn.add_css_class("pill");
        title_row.append(&boost_btn);

        card.append(&title_row);

        let lbl_desc = Label::builder()
            .label("Instantaneous zero-overhead standby cache eviction and temporary clutter purging")
            .xalign(0.0)
            .build();
        lbl_desc.add_css_class("dim-label");
        card.append(&lbl_desc);

        // Stats Row
        let stats_row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(24)
            .margin_top(8)
            .build();

        let ram_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .hexpand(true)
            .build();
        let ram_label = Label::builder().label("RAM Usage: --%").xalign(0.0).build();
        ram_label.add_css_class("heading");
        let ram_bar = ProgressBar::builder().build();
        ram_box.append(&ram_label);
        ram_box.append(&ram_bar);
        stats_row.append(&ram_box);

        let clutter_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .hexpand(true)
            .build();
        let clutter_label = Label::builder().label("Temporary Files: --").xalign(0.0).build();
        clutter_label.add_css_class("heading");
        let clutter_sub = Label::builder()
            .label("Reclaimable standby cache")
            .xalign(0.0)
            .build();
        clutter_sub.add_css_class("dim-label");
        clutter_box.append(&clutter_label);
        clutter_box.append(&clutter_sub);
        stats_row.append(&clutter_box);

        card.append(&stats_row);
        root_box.append(&card);

        // 3. Real-Time Telemetry Cards
        let sensors_title = Label::builder()
            .label("Real-Time Telemetry & Hardware Gauges")
            .xalign(0.0)
            .margin_top(8)
            .build();
        sensors_title.add_css_class("title-3");
        root_box.append(&sensors_title);

        let tiles_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .build();

        // CPU Tile
        let cpu_card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .build();
        cpu_card.add_css_class("metric-card");
        let cpu_title = Label::builder().label("CPU Load").xalign(0.0).build();
        cpu_title.add_css_class("heading");
        let cpu_val = Label::builder().label("--%").xalign(0.0).build();
        cpu_val.add_css_class("title-1");
        let cpu_sub = Label::builder().label("Processor").xalign(0.0).build();
        cpu_sub.add_css_class("dim-label");
        cpu_card.append(&cpu_title);
        cpu_card.append(&cpu_val);
        cpu_card.append(&cpu_sub);
        tiles_box.append(&cpu_card);

        // RAM Tile
        let mem_card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .build();
        mem_card.add_css_class("metric-card");
        let mem_title = Label::builder().label("Memory").xalign(0.0).build();
        mem_title.add_css_class("heading");
        let mem_val = Label::builder().label("--%").xalign(0.0).build();
        mem_val.add_css_class("title-1");
        let mem_sub = Label::builder().label("Active RAM").xalign(0.0).build();
        mem_sub.add_css_class("dim-label");
        mem_card.append(&mem_title);
        mem_card.append(&mem_val);
        mem_card.append(&mem_sub);
        tiles_box.append(&mem_card);

        // Disk Tile
        let disk_card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .build();
        disk_card.add_css_class("metric-card");
        let disk_title = Label::builder().label("Root Storage").xalign(0.0).build();
        disk_title.add_css_class("heading");
        let disk_val = Label::builder().label("--%").xalign(0.0).build();
        disk_val.add_css_class("title-1");
        let disk_sub = Label::builder().label("Filesystem").xalign(0.0).build();
        disk_sub.add_css_class("dim-label");
        disk_card.append(&disk_title);
        disk_card.append(&disk_val);
        disk_card.append(&disk_sub);
        tiles_box.append(&disk_card);

        // Network Tile
        let net_card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .build();
        net_card.add_css_class("metric-card");
        let net_title = Label::builder().label("Network I/O").xalign(0.0).build();
        net_title.add_css_class("heading");
        let net_val = Label::builder().label("0 KB/s").xalign(0.0).build();
        net_val.add_css_class("title-1");
        let net_sub = Label::builder().label("Down: 0 | Up: 0").xalign(0.0).build();
        net_sub.add_css_class("dim-label");
        net_card.append(&net_title);
        net_card.append(&net_val);
        net_card.append(&net_sub);
        tiles_box.append(&net_card);

        // GPU Tile
        let gpu_card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .build();
        gpu_card.add_css_class("metric-card");
        let gpu_title = Label::builder().label("GPU Load").xalign(0.0).build();
        gpu_title.add_css_class("heading");
        let gpu_val = Label::builder().label("N/A").xalign(0.0).build();
        gpu_val.add_css_class("title-1");
        let gpu_sub = Label::builder().label("Graphics").xalign(0.0).build();
        gpu_sub.add_css_class("dim-label");
        gpu_card.append(&gpu_title);
        gpu_card.append(&gpu_val);
        gpu_card.append(&gpu_sub);
        tiles_box.append(&gpu_card);

        root_box.append(&tiles_box);
        scroller.set_child(Some(&root_box));

        // Connect Boost Button (Debounced + Asynchronous)
        let is_boosting = Arc::new(AtomicBool::new(false));
        {
            let is_boosting = is_boosting.clone();
            let boost_btn = boost_btn.clone();
            let ram_label = ram_label.clone();
            let ram_bar = ram_bar.clone();
            let clutter_label = clutter_label.clone();

            boost_btn.connect_clicked(move |btn| {
                if !SmartBoostEngine::check_debounce(1000) {
                    return;
                }
                if is_boosting.load(Ordering::SeqCst) {
                    return;
                }
                is_boosting.store(true, Ordering::SeqCst);
                btn.set_sensitive(false);
                btn.set_label("Boosting...");

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let res = SmartBoostEngine::execute_boost(true, true);
                    let _ = tx.send(res);
                });

                let is_boosting = is_boosting.clone();
                let btn = btn.clone();
                let ram_label = ram_label.clone();
                let ram_bar = ram_bar.clone();
                let clutter_label = clutter_label.clone();

                glib::timeout_add_local(Duration::from_millis(100), move || {
                    if let Ok(res) = rx.try_recv() {
                        btn.set_sensitive(true);
                        btn.set_label("🚀 Boost");
                        is_boosting.store(false, Ordering::SeqCst);

                        if !res.already_running {
                            let status = SmartBoostEngine::get_boost_status();
                            ram_label.set_label(&format!("RAM Usage: {:.0}%", status.ram_percent));
                            ram_bar.set_fraction((status.ram_percent / 100.0).clamp(0.0, 1.0));
                            clutter_label.set_label(&format!("Temporary Files: {}", status.temp_size_str));
                        }

                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            });
        }

        // Non-Blocking Background Telemetry Refresh Loop (Every 3 seconds)
        {
            let ram_label = ram_label.clone();
            let ram_bar = ram_bar.clone();
            let clutter_label = clutter_label.clone();
            let cpu_val = cpu_val.clone();
            let cpu_sub = cpu_sub.clone();
            let mem_val = mem_val.clone();
            let mem_sub = mem_sub.clone();
            let disk_val = disk_val.clone();
            let disk_sub = disk_sub.clone();
            let net_val = net_val.clone();
            let net_sub = net_sub.clone();
            let gpu_val = gpu_val.clone();
            let gpu_sub = gpu_sub.clone();
            let host_sub = host_sub.clone();

            let (tx, rx) = mpsc::channel::<TelemetrySnapshot>();

            // Background worker thread for telemetry
            std::thread::spawn(move || {
                loop {
                    let summary = SystemMonitor::get_system_summary();
                    let cpu = SystemMonitor::get_cpu_info();
                    let mem = SystemMonitor::get_memory_info();
                    let disks = SystemMonitor::get_disks_info();
                    let net = SystemMonitor::get_network_rate();
                    let gpu = SystemMonitor::get_gpu_info();
                    let boost_status = SmartBoostEngine::get_boost_status();

                    let snapshot = TelemetrySnapshot {
                        summary,
                        cpu,
                        mem,
                        disks,
                        net,
                        gpu,
                        boost_status,
                    };

                    if tx.send(snapshot).is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(3));
                }
            });

            glib::timeout_add_local(Duration::from_millis(500), move || {
                while let Ok(snap) = rx.try_recv() {
                    ram_label.set_label(&format!("RAM Usage: {:.0}%", snap.boost_status.ram_percent));
                    ram_bar.set_fraction((snap.boost_status.ram_percent / 100.0).clamp(0.0, 1.0));
                    clutter_label.set_label(&format!("Temporary Files: {}", snap.boost_status.temp_size_str));

                    let temp_str = if let Some(t) = snap.cpu.temp_c {
                        format!(" | {:.0}°C", t)
                    } else {
                        String::new()
                    };
                    cpu_val.set_label(&format!("{:.1}%", snap.cpu.usage_percent));
                    cpu_sub.set_label(&format!("{} ({} cores{})", snap.cpu.brand, snap.cpu.core_count, temp_str));

                    mem_val.set_label(&format!("{:.1}%", snap.mem.percent));
                    mem_sub.set_label(&format!("{:.0} MB / {:.0} MB", snap.mem.used_mb, snap.mem.total_mb));

                    if let Some(root) = snap.disks.iter().find(|d| d.mount_point == "/") {
                        disk_val.set_label(&format!("{:.0}%", root.percent));
                        disk_sub.set_label(&format!("{:.1} GB Free / {:.1} GB", root.available_gb, root.total_gb));
                    }

                    net_val.set_label(&format!("{:.1} KB/s", snap.net.rx_kbps + snap.net.tx_kbps));
                    net_sub.set_label(&format!("↓ {:.1} KB/s | ↑ {:.1} KB/s", snap.net.rx_kbps, snap.net.tx_kbps));

                    if let Some(gpu) = snap.gpu {
                        gpu_val.set_label(&format!("{:.0}%", gpu.load_percent));
                        let g_temp = gpu.temp_c.map(|t| format!(" | {:.0}°C", t)).unwrap_or_default();
                        gpu_sub.set_label(&format!("{}{}", gpu.name, g_temp));
                    } else {
                        gpu_val.set_label("N/A");
                        gpu_sub.set_label("Integrated/Unknown");
                    }

                    host_sub.set_label(&format!("Kernel: {} | Desktop: {} | Uptime: {}", snap.summary.kernel, snap.summary.desktop, snap.summary.uptime_str));
                }
                glib::ControlFlow::Continue
            });
        }

        scroller
    }
}
