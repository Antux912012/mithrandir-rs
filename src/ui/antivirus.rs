use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, ListBox, Orientation, ProgressBar, ScrolledWindow,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Banner, Clamp, PreferencesGroup};
use std::sync::mpsc;
use crate::backend::antivirus::AntivirusEngine;

pub struct AntivirusPage;

impl AntivirusPage {
    pub fn build() -> ScrolledWindow {
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(24)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let clam_status = AntivirusEngine::get_status();

        let banner = Banner::builder()
            .title(&if clam_status.is_installed {
                format!("ClamAV Engine Ready | {}", clam_status.version)
            } else {
                "ClamAV Engine Not Installed. Install clamav via package manager to enable virus scanning.".to_string()
            })
            .revealed(true)
            .build();
        root_box.append(&banner);

        // Scan Action Box
        let scan_group = PreferencesGroup::builder()
            .title("Malware Scanner")
            .description("Scan your home directory and downloads for suspicious files and signatures")
            .build();

        let quick_row = ActionRow::builder()
            .title("Quick Scan")
            .subtitle("Scans Downloads and temporary user directories (~/Downloads)")
            .build();

        let quick_btn = Button::builder()
            .label("Scan Now")
            .css_classes(vec!["suggested-action"])
            .valign(Align::Center)
            .sensitive(clam_status.is_installed)
            .build();
        quick_row.add_suffix(&quick_btn);
        scan_group.add(&quick_row);

        let full_row = ActionRow::builder()
            .title("Full User Scan")
            .subtitle("Comprehensive deep-scan across your home folder (~)")
            .build();

        let full_btn = Button::builder()
            .label("Deep Scan")
            .valign(Align::Center)
            .sensitive(clam_status.is_installed)
            .build();
        full_row.add_suffix(&full_btn);
        scan_group.add(&full_row);

        root_box.append(&scan_group);

        // Progress bar
        let progress = ProgressBar::builder()
            .visible(false)
            .pulse_step(0.1)
            .build();
        root_box.append(&progress);

        // Findings Group
        let findings_group = PreferencesGroup::builder()
            .title("Detection Log")
            .description("No threats detected")
            .build();

        let findings_list = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        findings_group.add(&findings_list);
        root_box.append(&findings_group);

        let clamp = Clamp::builder()
            .maximum_size(880)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        // Connect scan
        let run_scan = {
            let progress = progress.clone();
            let findings_group = findings_group.clone();
            let findings_list = findings_list.clone();

            move |path: String| {
                progress.set_visible(true);
                findings_group.set_description(Some("Scanning in progress..."));

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let res = AntivirusEngine::scan_directory(&path);
                    let _ = tx.send(res);
                });

                let p_bar = progress.clone();
                let f_group = findings_group.clone();
                let f_list = findings_list.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                    p_bar.pulse();
                    if let Ok(res) = rx.try_recv() {
                        p_bar.set_visible(false);
                        match res {
                            Ok(threats) => {
                                if threats.is_empty() {
                                    f_group.set_description(Some("Scan complete: Clean! No threats found."));
                                } else {
                                    f_group.set_description(Some(&format!("⚠️ {} threats detected!", threats.len())));
                                    for t in threats {
                                        let row = ActionRow::builder()
                                            .title(&t.virus_name)
                                            .subtitle(&t.file_path)
                                            .build();
                                        f_list.append(&row);
                                    }
                                }
                            }
                            Err(e) => {
                                f_group.set_description(Some(&format!("Scan error: {}", e)));
                            }
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            }
        };

        let rs1 = run_scan.clone();
        quick_btn.connect_clicked(move |_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/antonio".to_string());
            rs1(format!("{}/Downloads", home));
        });

        let rs2 = run_scan.clone();
        full_btn.connect_clicked(move |_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/antonio".to_string());
            rs2(home);
        });

        scrolled
    }
}
