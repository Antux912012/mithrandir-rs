use mithrandir_rs::{CleanerEngine, SmartBoostEngine, SystemMonitor};
#[cfg(feature = "gui")]
use mithrandir_rs::BackgroundDaemon;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--smart-boost") {
        println!("⚡ Executing Mithrandir Smart Boost (Rust Engine)...");
        let status = SmartBoostEngine::get_boost_status();
        println!("  Initial RAM: {:.1}% | Clutter: {}", status.ram_percent, status.temp_size_str);

        let res = SmartBoostEngine::execute_boost(true, true);
        println!("✨ Boost complete!");
        println!("  RAM Reclaimed: {:.1} MB", res.ram_reclaimed_mb);
        println!("  Disk Freed:    {:.1} MB", res.disk_freed_mb);
        println!("  Current RAM:   {:.1}%", res.current_ram_percent);
        return;
    }

    if args.iter().any(|a| a == "--monitor") {
        println!("📊 Mithrandir System Telemetry (Rust Engine)");
        let mem = SystemMonitor::get_memory_info();
        println!("  RAM:  {:.1}% ({:.1} MB / {:.1} MB)", mem.percent, mem.used_mb, mem.total_mb);

        let cpu = SystemMonitor::get_cpu_info();
        println!("  CPU:  {:.1}% ({} cores - {})", cpu.usage_percent, cpu.core_count, cpu.brand);

        let disks = SystemMonitor::get_disks_info();
        for d in disks {
            println!("  Disk ({}): {:.1}% ({:.1} GB / {:.1} GB)", d.mount_point, d.percent, d.used_gb, d.total_gb);
        }
        return;
    }

    if args.iter().any(|a| a == "--clean") {
        println!("🧹 Scanning cleanable categories...");
        let categories = CleanerEngine::get_categories();
        for cat in &categories {
            println!("  [{}]: {} ({:.1} MB, {} files)", cat.id, cat.name, cat.size_mb, cat.file_count);
        }
        return;
    }

    #[cfg(feature = "gui")]
    {
        use libadwaita as adw;
        use adw::prelude::*;

        let start_minimized = args
            .iter()
            .any(|a| a == "--background" || a == "--minimized" || a == "--keep-alive");

        // Start background optimization monitor thread (runs every 60s)
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                // Automatically triggers boost if RAM >= 85% with 15min cooldown
                BackgroundDaemon::run_auto_boost_check(85.0, 1024.0, 15, true);
            }
        });

        let app = adw::Application::builder()
            .application_id("io.github.antonio.MithrandirRS")
            .build();

        app.connect_shutdown(|_| {
            let _ = mithrandir_rs::LocalLLMManager::stop_llamacpp_server();
        });

        // Query initial power saver state and create thread-safe shared flag
        let initial_power_saver = mithrandir_rs::PowerManager::is_power_saver_active();
        let power_saver_arc = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(initial_power_saver));

        // Setup communication channel between tray thread and GTK4 main thread
        let (tx, rx) = async_channel::unbounded();

        // Spawn pure Rust StatusNotifierItem (SNI) + DBusMenu service (no GTK3 conflict)
        match mithrandir_rs::tray::MithrandirTray::spawn_service(tx, power_saver_arc.clone()) {
            Ok(handle) => {
                mithrandir_rs::tray::register_tray_handle(handle);
            }
            Err(e) => {
                eprintln!("Warning: Failed to register system tray icon: {}", e);
            }
        }

        // Hold guard keeps GApplication event loop active when all windows are hidden in background
        let hold_guard = std::rc::Rc::new(std::cell::RefCell::new(None));
        let hg_init = hold_guard.clone();

        app.connect_startup(move |app| {
            *hg_init.borrow_mut() = Some(app.hold());
        });

        let ps_arc_action = power_saver_arc.clone();

        app.connect_activate(move |app| {
            // Single-instance activation: restore existing window if already running
            if let Some(existing_win) = app.windows().first() {
                existing_win.set_visible(true);
                existing_win.present();
                return;
            }

            let win = mithrandir_rs::ui::MithrandirWindow::build(app);

            let rx_channel = rx.clone();
            let app_weak = app.downgrade();
            let win_weak = win.downgrade();
            let ps_arc = ps_arc_action.clone();

            gtk4::glib::spawn_future_local(async move {
                while let Ok(action) = rx_channel.recv().await {
                    match action {
                        mithrandir_rs::TrayAction::RunSmartboost => {
                            if let Some(res) = mithrandir_rs::tray::MithrandirTray::execute_tray_smartboost() {
                                let body = format!(
                                    "Reclaimed {:.1} MB RAM and freed {:.1} MB disk cache.",
                                    res.ram_reclaimed_mb, res.disk_freed_mb
                                );
                                mithrandir_rs::NotificationManager::send_notification("⚡ Smart Boost", &body);
                            }
                        }
                        mithrandir_rs::TrayAction::TogglePowerSaver(enable) => {
                            let res = mithrandir_rs::PowerManager::set_power_saver(enable);
                            match res {
                                Ok(_) => {
                                    let body = if enable {
                                        "Hardware Power Saver Enabled (Turbo Disabled, EPP Power, PCIe ASPM Powersupersave)"
                                    } else {
                                        "Hardware Power Saver Disabled (Performance Restored)"
                                    };
                                    mithrandir_rs::NotificationManager::send_notification("⚡ Power Management", body);
                                    mithrandir_rs::ui::DashboardPage::update_ui_power_saver(enable);
                                }
                                Err(e) => {
                                    eprintln!("Failed to toggle power saver: {}", e);
                                    ps_arc.store(!enable, std::sync::atomic::Ordering::SeqCst);
                                    mithrandir_rs::tray::sync_tray_power_saver(!enable);
                                    mithrandir_rs::ui::DashboardPage::update_ui_power_saver(!enable);
                                    let err_msg = format!("Failed to change power profile: {}", e);
                                    mithrandir_rs::NotificationManager::send_notification("⚠️ Power Saver Error", &err_msg);
                                }
                            }
                        }
                        mithrandir_rs::TrayAction::OpenMithrandir => {
                            if let Some(w) = win_weak.upgrade() {
                                w.set_visible(true);
                                w.present();
                            }
                        }
                        mithrandir_rs::TrayAction::QuitMithrandir => {
                            if let Some(a) = app_weak.upgrade() {
                                a.quit();
                            }
                        }
                    }
                }
            });

            if !start_minimized {
                win.present();
            }
        });

        let filtered_args: Vec<String> = args
            .into_iter()
            .filter(|a| a != "--background" && a != "--minimized" && a != "--keep-alive")
            .collect();

        app.run_with_args(&filtered_args);
    }

    #[cfg(not(feature = "gui"))]
    {
        println!("Mithrandir-RS v0.4.1 (GNU/Linux System Cleaner & Optimizer in Rust)");
        println!("Usage:");
        println!("  mithrandir-rs                 Launch Libadwaita GUI");
        println!("  mithrandir-rs --smart-boost   One-click RAM and cache boost");
        println!("  mithrandir-rs --monitor       Real-time system telemetry");
        println!("  mithrandir-rs --clean         Scan system clutter and cache");
        println!("  mithrandir-rs --background    Launch hidden in background with panel tray icon");
        println!("  mithrandir-rs --keep-alive    Run background daemon on login");
    }
}
