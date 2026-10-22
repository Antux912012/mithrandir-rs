use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Align, Box as GtkBox, Button, FlowBox, Image, Label, ListBox, Orientation,
    ProgressBar, ScrolledWindow, SelectionMode, Window,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{
    ActionRow, Clamp, PreferencesGroup, PreferencesPage, PreferencesWindow, SpinRow, SwitchRow,
    Toast, ToastOverlay,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use crate::backend::monitor::{
    BatteryInfo, CpuInfo, DiskPartition, GpuInfo, MemoryInfo, NetworkRate, SystemMonitor,
    SystemSummary,
};
use crate::backend::smart_boost::{BoostStatus, SmartBoostEngine};

struct TelemetrySnapshot {
    summary: SystemSummary,
    cpu: CpuInfo,
    mem: MemoryInfo,
    disks: Vec<DiskPartition>,
    battery: BatteryInfo,
    net: NetworkRate,
    gpus: Vec<GpuInfo>,
    boost_status: BoostStatus,
}

#[derive(Clone)]
struct MetricTile {
    container: GtkBox,
    val_label: Label,
    progress: ProgressBar,
    sub_label: Label,
}

impl MetricTile {
    fn new(title: &str, icon_name: &str) -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .width_request(160)
            .height_request(160)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(4)
            .margin_end(4)
            .build();
        container.add_css_class("card");

        // Header with icon and title
        let header = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .margin_top(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let icon = Image::from_icon_name(icon_name);
        header.append(&icon);

        let title_lbl = Label::builder().label(title).build();
        title_lbl.add_css_class("heading");
        header.append(&title_lbl);
        container.append(&header);

        // Main value label
        let val_label = Label::builder().label("--").vexpand(true).build();
        val_label.add_css_class("title-1");
        container.append(&val_label);

        // Progress bar
        let progress = ProgressBar::builder()
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(4)
            .build();
        container.append(&progress);

        // Subtitle label
        let sub_label = Label::builder().label("--").margin_bottom(12).build();
        sub_label.add_css_class("caption");
        sub_label.add_css_class("dim-label");
        container.append(&sub_label);

        Self {
            container,
            val_label,
            progress,
            sub_label,
        }
    }

    fn set_data(&self, val_text: &str, fraction: f64, sub_text: &str) {
        self.val_label.set_label(val_text);
        self.progress.set_fraction(fraction.clamp(0.0, 1.0));
        self.sub_label.set_label(sub_text);
    }

    fn set_tooltip_text(&self, text: Option<&str>) {
        self.container.set_tooltip_text(text);
    }
}

thread_local! {
    static POWER_SWITCH_ROW: std::cell::RefCell<Option<SwitchRow>> = std::cell::RefCell::new(None);
}

pub struct DashboardPage;

impl DashboardPage {
    pub fn update_ui_power_saver(active: bool) {
        POWER_SWITCH_ROW.with(|cell| {
            if let Some(ref row) = *cell.borrow() {
                if row.is_active() != active {
                    row.set_active(active);
                }
            }
        });
    }
    fn create_info_row(title: &str, subtitle: &str, icon_name: &str) -> ActionRow {
        let row = ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();
        row.add_prefix(&Image::from_icon_name(icon_name));
        row
    }

    pub fn build(toast_overlay: Option<ToastOverlay>) -> ScrolledWindow {
        let scroller = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .kinetic_scrolling(true)
            .build();

        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(24)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        // 1. Microsoft PC Manager Style Smart Boost Banner
        let card = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .build();
        card.add_css_class("smart-boost-card");

        // Top row: Icon, Title/Description, Personalise button
        let top_row = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let flash_icon = Image::from_icon_name("media-flash-symbolic");
        flash_icon.set_pixel_size(24);
        top_row.append(&flash_icon);

        let title_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let lbl_title = Label::builder()
            .label("Smart Boost")
            .xalign(0.0)
            .build();
        lbl_title.add_css_class("title-3");
        title_box.append(&lbl_title);

        let lbl_desc = Label::builder()
            .label("Personalized one-click system performance booster and standby memory trimmer")
            .xalign(0.0)
            .build();
        lbl_desc.add_css_class("caption");
        lbl_desc.add_css_class("dim-label");
        title_box.append(&lbl_desc);

        top_row.append(&title_box);

        let pers_btn = Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Personalise Smart Boost Rules")
            .build();
        pers_btn.add_css_class("flat");
        top_row.append(&pers_btn);

        card.append(&top_row);

        // Stats row: RAM Usage + Temporary Files + Boost Button
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
        let ram_bar = ProgressBar::new();
        ram_box.append(&ram_label);
        ram_box.append(&ram_bar);
        stats_row.append(&ram_box);

        let temp_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .hexpand(true)
            .build();
        let clutter_label = Label::builder()
            .label("Temporary Files: -- MB")
            .xalign(0.0)
            .build();
        clutter_label.add_css_class("heading");
        let clutter_sub = Label::builder()
            .label("Standby cache and clutter cleanable")
            .xalign(0.0)
            .build();
        clutter_sub.add_css_class("caption");
        clutter_sub.add_css_class("dim-label");
        temp_box.append(&clutter_label);
        temp_box.append(&clutter_sub);
        stats_row.append(&temp_box);

        let boost_btn = Button::builder()
            .label("🚀 Boost")
            .valign(Align::Center)
            .build();
        boost_btn.add_css_class("suggested-action");
        boost_btn.add_css_class("pill");
        stats_row.append(&boost_btn);

        card.append(&stats_row);
        root_box.append(&card);

        // Personalise Smart Boost Dialog state & handler
        {
            let auto_enabled = Rc::new(Cell::new(true));
            let ram_thresh = Rc::new(Cell::new(80.0f64));
            let temp_thresh = Rc::new(Cell::new(1024.0f64));
            let notify_enabled = Rc::new(Cell::new(true));

            pers_btn.connect_clicked(move |btn| {
                let dialog = PreferencesWindow::builder()
                    .title("Personalise Smart Boost")
                    .modal(true)
                    .default_width(500)
                    .default_height(420)
                    .build();

                if let Some(root) = btn.root().and_then(|r| r.downcast::<Window>().ok()) {
                    dialog.set_transient_for(Some(&root));
                }

                let page = PreferencesPage::new();
                let group = PreferencesGroup::builder()
                    .title("Automatic Smart Boost Conditions")
                    .description("Mithrandir will automatically boost performance when thresholds are met")
                    .build();

                let auto_row = SwitchRow::builder()
                    .title("Enable Automatic Smart Boost")
                    .subtitle("Continuously monitors RAM and cache clutter in the background")
                    .active(auto_enabled.get())
                    .build();
                {
                    let auto_enabled = auto_enabled.clone();
                    auto_row.connect_active_notify(move |w| {
                        auto_enabled.set(w.is_active());
                    });
                }
                group.add(&auto_row);

                let ram_row = SpinRow::builder()
                    .title("Auto-Boost RAM Threshold (%)")
                    .subtitle("Trigger boost when system memory usage exceeds this percentage")
                    .adjustment(&Adjustment::new(ram_thresh.get(), 50.0, 95.0, 5.0, 5.0, 0.0))
                    .build();
                {
                    let ram_thresh = ram_thresh.clone();
                    ram_row.connect_value_notify(move |w| {
                        ram_thresh.set(w.value());
                    });
                }
                group.add(&ram_row);

                let temp_row = SpinRow::builder()
                    .title("Auto-Boost Temp Clutter Threshold (MB)")
                    .subtitle("Trigger boost when cleanable temporary files exceed this size")
                    .adjustment(&Adjustment::new(temp_thresh.get(), 250.0, 5000.0, 250.0, 250.0, 0.0))
                    .build();
                {
                    let temp_thresh = temp_thresh.clone();
                    temp_row.connect_value_notify(move |w| {
                        temp_thresh.set(w.value());
                    });
                }
                group.add(&temp_row);

                let notify_row = SwitchRow::builder()
                    .title("Show Notification on Boost")
                    .subtitle("Display desktop toast when Smart Boost reclaims memory and files")
                    .active(notify_enabled.get())
                    .build();
                {
                    let notify_enabled = notify_enabled.clone();
                    notify_row.connect_active_notify(move |w| {
                        notify_enabled.set(w.is_active());
                    });
                }
                group.add(&notify_row);

                page.add(&group);
                dialog.add(&page);
                dialog.present();
            });
        }

        // 2. Real-Time System Sensors (9 Square MetricTile Cards)
        let tile_group = PreferencesGroup::builder()
            .title("Real-Time System Sensors")
            .description("Live telemetry updating every 3 seconds (Stacer-grade sensor suite)")
            .build();

        let flow_box = FlowBox::builder()
            .valign(Align::Start)
            .max_children_per_line(5)
            .min_children_per_line(2)
            .selection_mode(SelectionMode::None)
            .column_spacing(16)
            .row_spacing(16)
            .build();

        let tile_cpu = MetricTile::new("CPU Load", "utilities-system-monitor-symbolic");
        let tile_mem = MetricTile::new("Memory", "drive-harddisk-symbolic");
        let tile_disk = MetricTile::new("Root Disk", "drive-multidisk-symbolic");
        let tile_temp = MetricTile::new("Temperature", "weather-clear-symbolic");
        let tile_battery = MetricTile::new("Battery", "battery-symbolic");
        let tile_net_dl = MetricTile::new("Download", "network-receive-symbolic");
        let tile_net_ul = MetricTile::new("Upload", "network-transmit-symbolic");
        let tile_gpu0 = MetricTile::new("GPU 0", "video-display-symbolic");
        let tile_gpu1 = MetricTile::new("GPU 1", "video-display-symbolic");

        flow_box.append(&tile_cpu.container);
        flow_box.append(&tile_mem.container);
        flow_box.append(&tile_disk.container);
        flow_box.append(&tile_temp.container);
        flow_box.append(&tile_battery.container);
        flow_box.append(&tile_net_dl.container);
        flow_box.append(&tile_net_ul.container);
        flow_box.append(&tile_gpu0.container);
        flow_box.append(&tile_gpu1.container);

        tile_group.add(&flow_box);
        root_box.append(&tile_group);

        // 3. Hardware & Power Management
        let power_group = PreferencesGroup::builder()
            .title("Hardware & Power Management")
            .description("Hardware-level power saving policies for CPU and PCIe bus")
            .build();

        let is_nixos = crate::os_info::detect_os().distro == crate::os_info::LinuxDistro::NixOS
            || crate::os_info::is_immutable_system();

        if is_nixos {
            let nix_banner = adw::Banner::builder()
                .title("NixOS detected: Please enable power saver services in your configuration.nix")
                .revealed(true)
                .build();
            power_group.add(&nix_banner);
        }

        let power_saver_row = SwitchRow::builder()
            .title("Battery Power Saver")
            .subtitle("Disable CPU Turbo Boost (intel_pstate), set EPP to power, and enable PCIe ASPM powersupersave")
            .active(crate::power::PowerManager::is_power_saver_active())
            .build();
        power_saver_row.add_prefix(&Image::from_icon_name("battery-symbolic"));

        power_saver_row.connect_active_notify(move |row| {
            let active = row.is_active();
            if crate::power::PowerManager::is_power_saver_active() != active {
                let _ = crate::power::PowerManager::set_power_saver(active);
                crate::tray::sync_tray_power_saver(active);
            }
        });

        POWER_SWITCH_ROW.with(|cell| {
            *cell.borrow_mut() = Some(power_saver_row.clone());
        });

        power_group.add(&power_saver_row);
        root_box.append(&power_group);

        // 4. System Information on Two Columns
        let sys_group = PreferencesGroup::builder()
            .title("System Information")
            .description("Host configuration and hardware overview")
            .build();

        let initial_summary = SystemMonitor::get_system_summary();

        let two_col_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(16)
            .homogeneous(true)
            .build();

        let col1_list = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .build();
        col1_list.add_css_class("boxed-list");
        col1_list.append(&Self::create_info_row(
            "Distribution",
            &initial_summary.distro,
            "computer-symbolic",
        ));
        col1_list.append(&Self::create_info_row(
            "Kernel",
            &initial_summary.kernel,
            "emblem-system-symbolic",
        ));
        col1_list.append(&Self::create_info_row(
            "Architecture",
            &initial_summary.arch,
            "cpu-symbolic",
        ));
        col1_list.append(&Self::create_info_row(
            "Desktop",
            &initial_summary.desktop,
            "preferences-desktop-display-symbolic",
        ));
        two_col_box.append(&col1_list);

        let col2_list = ListBox::builder()
            .selection_mode(SelectionMode::None)
            .build();
        col2_list.add_css_class("boxed-list");
        let cpu_display = if initial_summary.cpu_model.chars().count() <= 32 {
            initial_summary.cpu_model.clone()
        } else {
            let truncated: String = initial_summary.cpu_model.chars().take(32).collect();
            format!("{}...", truncated)
        };
        col2_list.append(&Self::create_info_row(
            "Processor",
            &cpu_display,
            "cpu-symbolic",
        ));
        let uptime_row = Self::create_info_row(
            "System Uptime",
            &initial_summary.uptime_str,
            "alarm-symbolic",
        );
        col2_list.append(&uptime_row);
        col2_list.append(&Self::create_info_row(
            "Session",
            &initial_summary.desktop,
            "dialog-information-symbolic",
        ));
        two_col_box.append(&col2_list);

        sys_group.add(&two_col_box);
        root_box.append(&sys_group);

        let clamp = Clamp::builder()
            .maximum_size(960)
            .child(&root_box)
            .build();
        scroller.set_child(Some(&clamp));

        // Connect Boost Button (Debounced + Asynchronous + Toast Feedback)
        let is_boosting = Arc::new(AtomicBool::new(false));
        {
            let is_boosting = is_boosting.clone();
            let ram_label = ram_label.clone();
            let ram_bar = ram_bar.clone();
            let clutter_label = clutter_label.clone();
            let toast_overlay = toast_overlay.clone();

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
                let toast_overlay = toast_overlay.clone();

                glib::timeout_add_local(Duration::from_millis(100), move || {
                    if let Ok(res) = rx.try_recv() {
                        btn.set_sensitive(true);
                        btn.set_label("🚀 Boost");
                        is_boosting.store(false, Ordering::SeqCst);

                        if res.already_running {
                            if let Some(ref overlay) = toast_overlay {
                                overlay.add_toast(Toast::new("Smart Boost is already running."));
                            }
                        } else {
                            let status = SmartBoostEngine::get_boost_status();
                            ram_label.set_label(&format!("RAM Usage: {:.1}%", status.ram_percent));
                            ram_bar.set_fraction((status.ram_percent / 100.0).clamp(0.0, 1.0));
                            clutter_label.set_label(&format!("Temporary Files: {}", status.temp_size_str));

                            if let Some(ref overlay) = toast_overlay {
                                let msg = format!(
                                    "Boost Complete! Freed {:.1} MB RAM and {:.1} MB temporary files.",
                                    res.ram_reclaimed_mb, res.disk_freed_mb
                                );
                                overlay.add_toast(Toast::new(&msg));
                            }
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
            let (tx, rx) = mpsc::channel::<TelemetrySnapshot>();

            std::thread::spawn(move || {
                loop {
                    let summary = SystemMonitor::get_system_summary();
                    let cpu = SystemMonitor::get_cpu_info();
                    let mem = SystemMonitor::get_memory_info();
                    let disks = SystemMonitor::get_disks_info();
                    let battery = SystemMonitor::get_battery_info();
                    let net = SystemMonitor::get_network_rate();
                    let gpus = SystemMonitor::get_gpus_info();
                    let boost_status = SmartBoostEngine::get_boost_status();

                    let snapshot = TelemetrySnapshot {
                        summary,
                        cpu,
                        mem,
                        disks,
                        battery,
                        net,
                        gpus,
                        boost_status,
                    };

                    if tx.send(snapshot).is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(3));
                }
            });

            glib::timeout_add_local(Duration::from_millis(300), move || {
                while let Ok(snap) = rx.try_recv() {
                    // 1. Update Smart Boost Banner
                    ram_label.set_label(&format!("RAM Usage: {:.1}%", snap.boost_status.ram_percent));
                    ram_bar.set_fraction((snap.boost_status.ram_percent / 100.0).clamp(0.0, 1.0));
                    clutter_label.set_label(&format!("Temporary Files: {}", snap.boost_status.temp_size_str));

                    // 2. CPU Tile
                    tile_cpu.set_data(
                        &format!("{:.1}%", snap.cpu.usage_percent),
                        (snap.cpu.usage_percent as f64) / 100.0,
                        "Total Load",
                    );

                    // 3. Memory Tile
                    tile_mem.set_data(
                        &format!("{:.1}%", snap.mem.percent),
                        snap.mem.percent / 100.0,
                        &format!("{:.0} / {:.0} MB", snap.mem.used_mb, snap.mem.total_mb),
                    );

                    // 4. Root Disk Tile
                    if let Some(root) = snap
                        .disks
                        .iter()
                        .find(|d| d.mount_point == "/")
                        .or_else(|| snap.disks.first())
                    {
                        tile_disk.set_data(
                            &format!("{:.1}%", root.percent),
                            root.percent / 100.0,
                            &format!("{:.1} GB Free", root.available_gb),
                        );
                    }

                    // 5. Temperature Tile
                    let cpu_temp = snap.cpu.temp_c.unwrap_or(0.0) as f64;
                    let temp_fraction = if cpu_temp > 0.0 { cpu_temp / 100.0 } else { 0.0 };
                    tile_temp.set_data(
                        &format!("{:.1}°C", cpu_temp),
                        temp_fraction,
                        "CPU Package",
                    );

                    // 6. Battery Tile
                    if snap.battery.present && snap.battery.design_capacity_wh > 0.0 {
                        let health_pct = snap.battery.capacity_percent;
                        let real_wh = snap.battery.real_capacity_wh;
                        let design_wh = snap.battery.design_capacity_wh;
                        let frac = if design_wh > 0.0 {
                            real_wh / design_wh
                        } else {
                            0.0
                        };
                        tile_battery.set_data(
                            &format!("{:.1}%", health_pct),
                            frac,
                            &format!("{:.1} / {:.1} Wh", real_wh, design_wh),
                        );
                        let status_text = if !snap.battery.status.is_empty()
                            && snap.battery.status != "Unknown"
                        {
                            format!(" · {}", snap.battery.status)
                        } else {
                            String::new()
                        };
                        tile_battery.set_tooltip_text(Some(&format!(
                            "Battery Health: {:.1}%\nReal Capacity: {:.2} Wh\nDesign Capacity: {:.2} Wh{}",
                            health_pct, real_wh, design_wh, status_text
                        )));
                    } else {
                        tile_battery.set_data("N/A", 0.0, "No Battery");
                        tile_battery.set_tooltip_text(Some("No battery detected on this system"));
                    }

                    // 7. Network Telemetry Tiles
                    let dl_frac = (snap.net.rx_bytes_sec / (20.0 * 1024.0 * 1024.0)).min(1.0);
                    let ul_frac = (snap.net.tx_bytes_sec / (10.0 * 1024.0 * 1024.0)).min(1.0);
                    tile_net_dl.set_data(&snap.net.rx_str, dl_frac, "Incoming Rate");
                    tile_net_ul.set_data(&snap.net.tx_str, ul_frac, "Outgoing Rate");

                    // 8. GPU 0 & GPU 1 Tiles
                    if let Some(gpu0) = snap.gpus.first() {
                        let name0: String = gpu0.name.chars().take(16).collect();
                        tile_gpu0.set_data(
                            &format!("{:.0}%", gpu0.load_percent),
                            (gpu0.load_percent as f64) / 100.0,
                            &name0,
                        );
                    } else {
                        tile_gpu0.set_data("N/A", 0.0, "No GPU Detected");
                    }

                    if let Some(gpu1) = snap.gpus.get(1) {
                        let name1: String = gpu1.name.chars().take(16).collect();
                        tile_gpu1.set_data(
                            &format!("{:.0}%", gpu1.load_percent),
                            (gpu1.load_percent as f64) / 100.0,
                            &name1,
                        );
                    } else {
                        tile_gpu1.set_data("N/A", 0.0, "Integrated / None");
                    }

                    // 9. Update System Uptime Row
                    uptime_row.set_subtitle(&snap.summary.uptime_str);
                }
                glib::ControlFlow::Continue
            });
        }

        scroller
    }
}
