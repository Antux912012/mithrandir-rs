use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, CheckButton, Label, Orientation, ScrolledWindow};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup};
use std::cell::RefCell;
use std::rc::Rc;
use crate::backend::cleaner::CleanerEngine;

struct CleanerRowWidgets {
    cat_id: String,
    check: CheckButton,
    size_label: Label,
}

pub struct CleanerPage;

impl CleanerPage {
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

        // Top Action Buttons
        let btn_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .halign(Align::End)
            .build();

        let scan_btn = Button::builder()
            .label("Scan System")
            .css_classes(vec!["suggested-action"])
            .build();

        let clean_btn = Button::builder()
            .label("Clean Selected")
            .css_classes(vec!["destructive-action"])
            .sensitive(false)
            .build();

        btn_box.append(&scan_btn);
        btn_box.append(&clean_btn);
        root_box.append(&btn_box);

        // Group
        let group = PreferencesGroup::builder()
            .title("Cleaning Categories")
            .description("Select items to scan and safely clean across System, Flatpaks, and Snaps")
            .build();
        root_box.append(&group);

        let initial_cats = CleanerEngine::get_categories();
        let rows_state: Rc<RefCell<Vec<CleanerRowWidgets>>> = Rc::new(RefCell::new(Vec::new()));

        for cat in &initial_cats {
            let row = ActionRow::builder()
                .title(&cat.name)
                .subtitle(&cat.description)
                .build();

            let check = CheckButton::builder()
                .active(true)
                .valign(Align::Center)
                .build();
            row.add_prefix(&check);

            let size_label = Label::builder()
                .label("Not scanned")
                .css_classes(vec!["dim-label"])
                .valign(Align::Center)
                .build();
            row.add_suffix(&size_label);

            group.add(&row);

            rows_state.borrow_mut().push(CleanerRowWidgets {
                cat_id: cat.id.clone(),
                check,
                size_label,
            });
        }

        // Scan button action
        let scan_btn_clone = scan_btn.clone();
        let clean_btn_clone = clean_btn.clone();
        let rows_state_clone = rows_state.clone();

        scan_btn.connect_clicked(move |_| {
            scan_btn_clone.set_sensitive(false);
            scan_btn_clone.set_label("Scanning...");

            let (tx, rx) = async_channel::unbounded::<Vec<crate::backend::cleaner::CleanCategory>>();
            let s_btn = scan_btn_clone.clone();
            let c_btn = clean_btn_clone.clone();
            let r_state = rows_state_clone.clone();

            glib::MainContext::default().spawn_local(async move {
                if let Ok(results) = rx.recv().await {
                    s_btn.set_sensitive(true);
                    s_btn.set_label("Scan System");
                    c_btn.set_sensitive(true);

                    let rows = r_state.borrow();
                    for r in rows.iter() {
                        if let Some(cat) = results.iter().find(|c| c.id == r.cat_id) {
                            r.size_label.set_label(&format!("{:.1} MB ({} files)", cat.size_mb, cat.file_count));
                        }
                    }
                }
            });

            std::thread::spawn(move || {
                let scanned = CleanerEngine::get_categories();
                let _ = tx.send_blocking(scanned);
            });
        });

        // Clean button action
        let clean_btn_for_clean = clean_btn.clone();
        let rows_state_for_clean = rows_state.clone();

        clean_btn.connect_clicked(move |_| {
            clean_btn_for_clean.set_sensitive(false);
            clean_btn_for_clean.set_label("Cleaning...");

            let selected_ids: Vec<String> = rows_state_for_clean
                .borrow()
                .iter()
                .filter(|r| r.check.is_active())
                .map(|r| r.cat_id.clone())
                .collect();

            let (tx, rx) = async_channel::unbounded::<f64>();
            let c_btn = clean_btn_for_clean.clone();
            let r_state = rows_state_for_clean.clone();

            glib::MainContext::default().spawn_local(async move {
                if let Ok(freed) = rx.recv().await {
                    c_btn.set_sensitive(false);
                    c_btn.set_label("Clean Selected");

                    let rows = r_state.borrow();
                    for r in rows.iter() {
                        if r.check.is_active() {
                            r.size_label.set_label("Cleaned (0 MB)");
                        }
                    }
                    crate::backend::indicator::NotificationManager::send_notification(
                        "🧹 Clean Complete",
                        &format!("Freed {:.1} MB of system clutter.", freed),
                    );
                }
            });

            std::thread::spawn(move || {
                let mut freed_mb = 0.0;
                for id in selected_ids {
                    let res = CleanerEngine::clean_category(&id);
                    freed_mb += res.freed_mb;
                }
                let _ = tx.send_blocking(freed_mb);
            });
        });

        let clamp = Clamp::builder()
            .maximum_size(840)
            .child(&root_box)
            .build();

        scrolled.set_child(Some(&clamp));
        scrolled
    }
}
