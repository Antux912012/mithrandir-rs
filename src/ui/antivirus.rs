use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, ListBox, Orientation, ProgressBar,
    ScrolledWindow, TextView, WrapMode,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Banner, Clamp, ExpanderRow, PreferencesGroup};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::backend::antivirus::AntivirusEngine;

pub struct AntivirusPage;

impl AntivirusPage {
    pub fn build() -> ScrolledWindow {
        let scrolled = ScrolledWindow::builder()
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

        let clam_status = AntivirusEngine::get_status();

        let banner = Banner::builder()
            .title(&if clam_status.is_installed {
                format!("ClamAV Engine Ready | {} (Definitions: {})", clam_status.version, clam_status.definitions_date)
            } else {
                "ClamAV Engine Not Installed. Click 'Install ClamAV' below to setup antivirus protection.".to_string()
            })
            .revealed(true)
            .build();
        root_box.append(&banner);

        // 1. Engine & Definition Control Group
        let engine_group = PreferencesGroup::builder()
            .title("ClamAV Engine & Definitions Management")
            .description("Keep virus signatures up-to-date and manage real-time scanning tools")
            .build();

        let engine_row = ActionRow::builder()
            .title("Virus Engine Lifecycle")
            .subtitle(if clam_status.is_installed {
                "Engine is ready for active malware scans"
            } else {
                "Engine is not installed on this system"
            })
            .build();

        let install_clam_btn = Button::builder()
            .label("Install ClamAV")
            .css_classes(vec!["suggested-action"])
            .valign(Align::Center)
            .visible(!clam_status.is_installed)
            .build();

        let update_defs_btn = Button::builder()
            .label("Update Definitions (FreshClam)")
            .valign(Align::Center)
            .sensitive(clam_status.is_installed)
            .build();

        engine_row.add_suffix(&install_clam_btn);
        engine_row.add_suffix(&update_defs_btn);
        engine_group.add(&engine_row);

        // Terminal Console Expander
        let console_expander = ExpanderRow::builder()
            .title("Anti-Malware Terminal Console")
            .subtitle("View live installer, freshclam definition update, and scan activity logs")
            .build();

        let console_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let console_progress = ProgressBar::builder()
            .visible(false)
            .pulse_step(0.1)
            .build();

        let console_view = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(WrapMode::Word)
            .monospace(true)
            .height_request(130)
            .build();

        let console_scroll = ScrolledWindow::builder()
            .child(&console_view)
            .vexpand(true)
            .build();

        console_box.append(&console_progress);
        console_box.append(&console_scroll);
        console_expander.add_row(&ActionRow::builder().child(&console_box).build());
        engine_group.add(&console_expander);
        root_box.append(&engine_group);

        // 2. Scan Actions Group
        let scan_group = PreferencesGroup::builder()
            .title("Malware Scanner")
            .description("Scan your home directory, downloads, or custom paths for malicious signatures")
            .build();

        let quick_row = ActionRow::builder()
            .title("Quick Scan")
            .subtitle("Scans Downloads and temporary user directories (~/Downloads)")
            .build();

        let quick_btn = Button::builder()
            .label("Quick Scan")
            .css_classes(vec!["suggested-action"])
            .valign(Align::Center)
            .sensitive(clam_status.is_installed)
            .build();
        quick_row.add_suffix(&quick_btn);
        scan_group.add(&quick_row);

        let full_row = ActionRow::builder()
            .title("Full User Scan")
            .subtitle("Deep scan across user home folder (~)")
            .build();

        let full_btn = Button::builder()
            .label("Deep Scan")
            .valign(Align::Center)
            .sensitive(clam_status.is_installed)
            .build();
        full_row.add_suffix(&full_btn);
        scan_group.add(&full_row);

        let custom_row = ActionRow::builder()
            .title("Custom Directory Scan")
            .subtitle("Enter path to scan custom directory")
            .build();

        let custom_entry = Entry::builder()
            .placeholder_text("/path/to/folder")
            .hexpand(true)
            .valign(Align::Center)
            .build();

        let custom_btn = Button::builder()
            .label("Scan Custom")
            .valign(Align::Center)
            .sensitive(clam_status.is_installed)
            .build();

        custom_row.add_suffix(&custom_entry);
        custom_row.add_suffix(&custom_btn);
        scan_group.add(&custom_row);

        let stop_scan_btn = Button::builder()
            .label("Stop Active Scan")
            .css_classes(vec!["destructive-action"])
            .valign(Align::Center)
            .sensitive(false)
            .build();
        let stop_row = ActionRow::builder()
            .title("Scan Lifecycle")
            .subtitle("Cancel any currently running scan operation")
            .build();
        stop_row.add_suffix(&stop_scan_btn);
        scan_group.add(&stop_row);

        root_box.append(&scan_group);

        // 3. Detection Log Group
        let findings_group = PreferencesGroup::builder()
            .title("Detection Log")
            .description("No active scan performed yet")
            .build();

        let findings_list = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        findings_group.add(&findings_list);
        root_box.append(&findings_group);

        // 4. Quarantine Vault Group
        let quarantine_group = PreferencesGroup::builder()
            .title("Quarantine Vault")
            .description("Isolated threats with revoked execution permissions (0000)")
            .build();

        let quarantine_list = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        quarantine_group.add(&quarantine_list);
        root_box.append(&quarantine_group);

        let clamp = Clamp::builder()
            .maximum_size(900)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        // Shared Scan Cancel Flag
        let scan_cancel_flag = Arc::new(AtomicBool::new(false));

        // Helper to refresh quarantine list
        let refresh_quarantine = {
            let q_list = quarantine_list.clone();
            move || {
                while let Some(child) = q_list.first_child() {
                    q_list.remove(&child);
                }

                let entries = AntivirusEngine::list_quarantined();
                if entries.is_empty() {
                    let empty_row = ActionRow::builder()
                        .title("Vault Clean")
                        .subtitle("No files currently quarantined.")
                        .build();
                    q_list.append(&empty_row);
                } else {
                    for entry in entries {
                        let row = ActionRow::builder()
                            .title(&entry.virus_name)
                            .subtitle(format!("{} | SHA: {:.8}...", entry.original_path, entry.sha256))
                            .build();

                        let restore_btn = Button::builder()
                            .label("Restore")
                            .valign(Align::Center)
                            .build();

                        let delete_btn = Button::builder()
                            .label("Delete")
                            .css_classes(vec!["destructive-action"])
                            .valign(Align::Center)
                            .build();

                        let id_r = entry.id.clone();
                        let ql_r = q_list.clone();
                        restore_btn.connect_clicked(move |_| {
                            let _ = AntivirusEngine::restore_file(&id_r);
                            // Refresh
                            let entries_now = AntivirusEngine::list_quarantined();
                            while let Some(c) = ql_r.first_child() {
                                ql_r.remove(&c);
                            }
                            for en in entries_now {
                                let r = ActionRow::builder().title(&en.virus_name).subtitle(&en.original_path).build();
                                ql_r.append(&r);
                            }
                        });

                        let id_d = entry.id.clone();
                        let ql_d = q_list.clone();
                        delete_btn.connect_clicked(move |_| {
                            let _ = AntivirusEngine::delete_quarantined(&id_d);
                            let entries_now = AntivirusEngine::list_quarantined();
                            while let Some(c) = ql_d.first_child() {
                                ql_d.remove(&c);
                            }
                            for en in entries_now {
                                let r = ActionRow::builder().title(&en.virus_name).subtitle(&en.original_path).build();
                                ql_d.append(&r);
                            }
                        });

                        row.add_suffix(&restore_btn);
                        row.add_suffix(&delete_btn);
                        q_list.append(&row);
                    }
                }
            }
        };

        refresh_quarantine();

        // Connect 1-Click Installer Button
        {
            let ce = console_expander.clone();
            let cp = console_progress.clone();
            let cv = console_view.clone();
            let b = banner.clone();

            install_clam_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                ce.set_expanded(true);
                cp.set_visible(true);

                let (tx, rx) = async_channel::unbounded::<(bool, String)>();
                let btn_c = btn.clone();
                let cv_c = cv.clone();
                let cp_c = cp.clone();
                let b_c = b.clone();

                glib::MainContext::default().spawn_local(async move {
                    while let Ok((done, line)) = rx.recv().await {
                        let buf = cv_c.buffer();
                        let mut end = buf.end_iter();
                        buf.insert(&mut end, &format!("{}\n", line));

                        if done {
                            btn_c.set_sensitive(true);
                            cp_c.set_visible(false);
                            let st = AntivirusEngine::get_status();
                            if st.is_installed {
                                b_c.set_title(&format!("ClamAV Engine Ready | {}", st.version));
                            }
                            break;
                        }
                    }
                });

                std::thread::spawn(move || {
                    let tx_log = tx.clone();
                    let res = AntivirusEngine::install_clamav_streaming(move |line| {
                        let _ = tx_log.send_blocking((false, line));
                    });
                    let done_msg = match res {
                        Ok(code) => format!("Installer finished with exit code {}.", code),
                        Err(e) => format!("Installation error: {}", e),
                    };
                    let _ = tx.send_blocking((true, done_msg));
                });
            });
        }

        // Connect Update Definitions (FreshClam) Button
        {
            let ce = console_expander.clone();
            let cp = console_progress.clone();
            let cv = console_view.clone();
            let b = banner.clone();

            update_defs_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                ce.set_expanded(true);
                cp.set_visible(true);

                let (tx, rx) = async_channel::unbounded::<(bool, String)>();
                let btn_c = btn.clone();
                let cv_c = cv.clone();
                let cp_c = cp.clone();
                let b_c = b.clone();

                glib::MainContext::default().spawn_local(async move {
                    while let Ok((done, line)) = rx.recv().await {
                        let buf = cv_c.buffer();
                        let mut end = buf.end_iter();
                        buf.insert(&mut end, &format!("{}\n", line));

                        if done {
                            btn_c.set_sensitive(true);
                            cp_c.set_visible(false);
                            let st = AntivirusEngine::get_status();
                            b_c.set_title(&format!("ClamAV Engine Ready | {} (Definitions: {})", st.version, st.definitions_date));
                            break;
                        }
                    }
                });

                std::thread::spawn(move || {
                    let tx_log = tx.clone();
                    let res = AntivirusEngine::update_definitions_streaming(move |line| {
                        let _ = tx_log.send_blocking((false, line));
                    });
                    let done_msg = match res {
                        Ok(code) => format!("FreshClam finished with exit code {}.", code),
                        Err(e) => format!("FreshClam error: {}", e),
                    };
                    let _ = tx.send_blocking((true, done_msg));
                });
            });
        }

        // Scan Runner Closure
        let run_scan = {
            let cp = console_progress.clone();
            let ce = console_expander.clone();
            let cv = console_view.clone();
            let fg = findings_group.clone();
            let fl = findings_list.clone();
            let ssb = stop_scan_btn.clone();
            let scf = scan_cancel_flag.clone();
            let rq = refresh_quarantine.clone();

            move |path: String| {
                scf.store(false, Ordering::SeqCst);
                cp.set_visible(true);
                ce.set_expanded(true);
                ssb.set_sensitive(true);
                fg.set_description(Some(&format!("Scanning {} in progress...", path)));

                while let Some(child) = fl.first_child() {
                    fl.remove(&child);
                }

                let (tx, rx) = async_channel::unbounded::<(bool, String, Option<Result<Vec<crate::backend::antivirus::ThreatFinding>, String>>)>();
                let cancel_clone = scf.clone();
                let p_scan = path.clone();

                let cp_c = cp.clone();
                let cv_c = cv.clone();
                let fg_c = fg.clone();
                let fl_c = fl.clone();
                let ssb_c = ssb.clone();
                let rq_c = rq.clone();

                glib::MainContext::default().spawn_local(async move {
                    while let Ok((done, line, result_opt)) = rx.recv().await {
                        let buf = cv_c.buffer();
                        let mut end = buf.end_iter();
                        buf.insert(&mut end, &format!("{}\n", line));

                        if done {
                            cp_c.set_visible(false);
                            ssb_c.set_sensitive(false);

                            if let Some(Ok(findings)) = result_opt {
                                if findings.is_empty() {
                                    fg_c.set_description(Some("Scan complete: Clean! No threats detected."));
                                } else {
                                    fg_c.set_description(Some(&format!("⚠️ {} threats detected!", findings.len())));
                                    for t in findings {
                                        let row = ActionRow::builder()
                                            .title(&t.virus_name)
                                            .subtitle(&t.file_path)
                                            .build();

                                        let q_btn = Button::builder()
                                            .label("Quarantine")
                                            .css_classes(vec!["suggested-action"])
                                            .valign(Align::Center)
                                            .build();

                                        let f_path = t.file_path.clone();
                                        let v_name = t.virus_name.clone();
                                        let rq_click = rq_c.clone();
                                        let row_c = row.clone();

                                        q_btn.connect_clicked(move |btn| {
                                            btn.set_sensitive(false);
                                            let _ = AntivirusEngine::quarantine_file(&f_path, &v_name);
                                            row_c.set_subtitle(&format!("{} (QUARANTINED)", f_path));
                                            rq_click();
                                        });

                                        row.add_suffix(&q_btn);
                                        fl_c.append(&row);
                                    }
                                }
                            } else if let Some(Err(e)) = result_opt {
                                fg_c.set_description(Some(&format!("Scan terminated: {}", e)));
                            }
                            break;
                        }
                    }
                });

                std::thread::spawn(move || {
                    let tx_log = tx.clone();
                    let res = AntivirusEngine::scan_directory_streaming(&p_scan, cancel_clone, move |line| {
                        let _ = tx_log.send_blocking((false, line, None));
                    });
                    match res {
                        Ok(findings) => {
                            let _ = tx.send_blocking((true, "Scan completed.".to_string(), Some(Ok(findings))));
                        }
                        Err(e) => {
                            let _ = tx.send_blocking((true, format!("Scan error: {}", e), Some(Err(e))));
                        }
                    }
                });
            }
        };

        // Quick Scan Button
        {
            let rs = run_scan.clone();
            quick_btn.connect_clicked(move |_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                rs(format!("{}/Downloads", home));
            });
        }

        // Deep Scan Button
        {
            let rs = run_scan.clone();
            full_btn.connect_clicked(move |_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                rs(home);
            });
        }

        // Custom Scan Button
        {
            let rs = run_scan.clone();
            let ce = custom_entry.clone();
            custom_btn.connect_clicked(move |_| {
                let path = ce.text().trim().to_string();
                if !path.is_empty() {
                    rs(path);
                }
            });
        }

        // Stop Scan Button
        {
            let scf = scan_cancel_flag.clone();
            stop_scan_btn.connect_clicked(move |btn| {
                scf.store(true, Ordering::SeqCst);
                btn.set_sensitive(false);
            });
        }

        scrolled
    }
}
