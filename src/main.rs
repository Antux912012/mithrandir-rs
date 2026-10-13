use mithrandir_rs::{CleanerEngine, SmartBoostEngine, SystemMonitor};
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
        let app = adw::Application::builder()
            .application_id("io.github.antonio.MithrandirRS")
            .build();

        app.connect_activate(|app| {
            let win = mithrandir_rs::ui::MithrandirWindow::build(app);
            win.present();
        });

        app.run();
    }

    #[cfg(not(feature = "gui"))]
    {
        println!("Mithrandir-RS v0.1.0 (GNU/Linux System Cleaner & Optimizer in Rust)");
        println!("Usage:");
        println!("  mithrandir-rs --smart-boost   One-click RAM and cache boost");
        println!("  mithrandir-rs --monitor       Real-time system telemetry");
        println!("  mithrandir-rs --clean         Scan system clutter and cache");
    }
}
