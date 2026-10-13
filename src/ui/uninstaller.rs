use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, ListBox, Orientation, ScrolledWindow, SearchEntry, ToggleButton,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use crate::backend::packages::{PackageItem, PackageManager};

pub struct UninstallerPage;

impl UninstallerPage {
    pub fn build() -> ScrolledWindow {
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(20)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        // Filter Pills: Native, Flatpak, Snap, Python
        let filter_bar = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .halign(Align::Center)
            .build();

        let btn_native = ToggleButton::builder().label("Native (.deb)").active(true).css_classes(vec!["pill"]).build();
        let btn_flatpak = ToggleButton::builder().label("Flatpak").css_classes(vec!["pill"]).build();
        let btn_snap = ToggleButton::builder().label("Snap").css_classes(vec!["pill"]).build();
        let btn_python = ToggleButton::builder().label("Python (pip)").css_classes(vec!["pill"]).build();

        filter_bar.append(&btn_native);
        filter_bar.append(&btn_flatpak);
        filter_bar.append(&btn_snap);
        filter_bar.append(&btn_python);
        root_box.append(&filter_bar);

        // Search
        let search_entry = SearchEntry::builder()
            .placeholder_text("Search installed software...")
            .build();
        root_box.append(&search_entry);

        let group = PreferencesGroup::builder()
            .title("Installed Packages")
            .description("Select applications and packages to uninstall")
            .build();

        let list_box = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        group.add(&list_box);
        root_box.append(&group);

        let clamp = Clamp::builder()
            .maximum_size(920)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        let current_type = Rc::new(RefCell::new("native".to_string()));
        let packages_cache: Rc<RefCell<Vec<PackageItem>>> = Rc::new(RefCell::new(Vec::new()));
        let search_filter = Rc::new(RefCell::new(String::new()));

        let populate = {
            let list_box = list_box.clone();
            let packages_cache = packages_cache.clone();
            let search_filter = search_filter.clone();

            move || {
                while let Some(child) = list_box.first_child() {
                    list_box.remove(&child);
                }

                let query = search_filter.borrow().to_lowercase();
                let pkgs = packages_cache.borrow();

                let mut count = 0;
                for p in pkgs.iter() {
                    if !query.is_empty() {
                        let match_name = p.name.to_lowercase().contains(&query);
                        let match_desc = p.description.to_lowercase().contains(&query);
                        if !match_name && !match_desc {
                            continue;
                        }
                    }

                    if count >= 100 {
                        break;
                    }
                    count += 1;

                    let row = ActionRow::builder()
                        .title(&p.name)
                        .subtitle(&format!("v{} | {} | {}", p.version, p.size_str, p.description))
                        .build();

                    let uninst_btn = Button::builder()
                        .label("Uninstall")
                        .css_classes(vec!["destructive-action"])
                        .valign(Align::Center)
                        .build();

                    let pkg_item = p.clone();
                    let row_inner = row.clone();
                    let list_inner = list_box.clone();

                    uninst_btn.connect_clicked(move |b| {
                        b.set_sensitive(false);
                        b.set_label("Removing...");
                        let p_clone = pkg_item.clone();
                        let r_inner = row_inner.clone();
                        let l_inner = list_inner.clone();

                        std::thread::spawn(move || {
                            let _ = PackageManager::uninstall_package(&p_clone);
                        });
                        l_inner.remove(&r_inner);
                    });

                    row.add_suffix(&uninst_btn);
                    list_box.append(&row);
                }
            }
        };

        let load_type = {
            let current_type = current_type.clone();
            let packages_cache = packages_cache.clone();
            let populate = populate.clone();

            move |pkg_type: &str| {
                *current_type.borrow_mut() = pkg_type.to_string();
                let p_type = pkg_type.to_string();

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let items = PackageManager::list_packages(&p_type);
                    let _ = tx.send(items);
                });

                let p_cache = packages_cache.clone();
                let pop = populate.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if let Ok(items) = rx.try_recv() {
                        *p_cache.borrow_mut() = items;
                        pop();
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            }
        };

        // Connect pills
        let lt1 = load_type.clone();
        let b_flat = btn_flatpak.clone();
        let b_snap = btn_snap.clone();
        let b_py = btn_python.clone();
        btn_native.connect_toggled(move |b| {
            if b.is_active() {
                b_flat.set_active(false);
                b_snap.set_active(false);
                b_py.set_active(false);
                lt1("native");
            }
        });

        let lt2 = load_type.clone();
        let b_nat = btn_native.clone();
        let b_snap2 = btn_snap.clone();
        let b_py2 = btn_python.clone();
        btn_flatpak.connect_toggled(move |b| {
            if b.is_active() {
                b_nat.set_active(false);
                b_snap2.set_active(false);
                b_py2.set_active(false);
                lt2("flatpak");
            }
        });

        let lt3 = load_type.clone();
        let b_nat3 = btn_native.clone();
        let b_flat3 = btn_flatpak.clone();
        let b_py3 = btn_python.clone();
        btn_snap.connect_toggled(move |b| {
            if b.is_active() {
                b_nat3.set_active(false);
                b_flat3.set_active(false);
                b_py3.set_active(false);
                lt3("snap");
            }
        });

        let lt4 = load_type.clone();
        let b_nat4 = btn_native.clone();
        let b_flat4 = btn_flatpak.clone();
        let b_snap4 = btn_snap.clone();
        btn_python.connect_toggled(move |b| {
            if b.is_active() {
                b_nat4.set_active(false);
                b_flat4.set_active(false);
                b_snap4.set_active(false);
                lt4("python");
            }
        });

        let search_clone = search_filter.clone();
        let pop_clone = populate.clone();
        search_entry.connect_search_changed(move |e| {
            *search_clone.borrow_mut() = e.text().to_string();
            pop_clone();
        });

        // Initial load
        load_type("native");

        scrolled
    }
}
