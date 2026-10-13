use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ProgressBar, ScrolledWindow};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use crate::backend::{SmartBoostEngine, SystemMonitor};

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

        // 1. Smart Boost Hero Card
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
            .label("High-speed standby pagecache trimmer and clutter cleanup in Rust")
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

        // 2. Real-Time Telemetry Cards
        let sensors_title = Label::builder()
            .label("System Telemetry")
            .xalign(0.0)
            .margin_top(12)
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
            .spacing(8)
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
            .spacing(8)
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

        root_box.append(&tiles_box);
        scroller.set_child(Some(&root_box));

        // Connect Boost Button
        let is_boosting = Arc::new(AtomicBool::new(false));
        {
            let is_boosting = is_boosting.clone();
            let boost_btn = boost_btn.clone();
            let ram_label = ram_label.clone();
            let ram_bar = ram_bar.clone();
            let clutter_label = clutter_label.clone();

            boost_btn.connect_clicked(move |btn| {
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

                glib::timeout_add_local(Duration::from_millis(50), move || {
                    if let Ok(_res) = rx.try_recv() {
                        btn.set_sensitive(true);
                        btn.set_label("🚀 Boost");
                        is_boosting.store(false, Ordering::SeqCst);

                        let status = SmartBoostEngine::get_boost_status();
                        ram_label.set_label(&format!("RAM Usage: {:.0}%", status.ram_percent));
                        ram_bar.set_fraction((status.ram_percent / 100.0).clamp(0.0, 1.0));
                        clutter_label.set_label(&format!("Temporary Files: {}", status.temp_size_str));

                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            });
        }

        // Real-Time Telemetry Refresh Loop (Every 3 seconds)
        {
            let ram_label = ram_label.clone();
            let ram_bar = ram_bar.clone();
            let clutter_label = clutter_label.clone();
            let cpu_val = cpu_val.clone();
            let cpu_sub = cpu_sub.clone();
            let mem_val = mem_val.clone();
            let mem_sub = mem_sub.clone();

            let update_metrics = move || {
                let status = SmartBoostEngine::get_boost_status();
                ram_label.set_label(&format!("RAM Usage: {:.0}%", status.ram_percent));
                ram_bar.set_fraction((status.ram_percent / 100.0).clamp(0.0, 1.0));
                clutter_label.set_label(&format!("Temporary Files: {}", status.temp_size_str));

                let cpu = SystemMonitor::get_cpu_info();
                cpu_val.set_label(&format!("{:.1}%", cpu.usage_percent));
                cpu_sub.set_label(&format!("{} ({} cores)", cpu.brand, cpu.core_count));

                let mem = SystemMonitor::get_memory_info();
                mem_val.set_label(&format!("{:.1}%", mem.percent));
                mem_sub.set_label(&format!("{:.0} MB / {:.0} MB", mem.used_mb, mem.total_mb));

                glib::ControlFlow::Continue
            };

            // Run initial update
            update_metrics();

            glib::timeout_add_seconds_local(3, update_metrics);
        }

        scroller
    }
}
