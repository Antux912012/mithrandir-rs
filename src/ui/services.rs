use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Image, ListBox, Orientation, ScrolledWindow, SearchEntry,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use crate::backend::services::{ServiceInfo, ServiceManager};

pub struct ServicesPage;

impl ServicesPage {
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

        let header_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .halign(Align::End)
            .build();

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search systemd services...")
            .hexpand(true)
            .build();

        let refresh_btn = Button::builder()
            .label("Refresh Services")
            .build();

        header_box.append(&search_entry);
        header_box.append(&refresh_btn);
        root_box.append(&header_box);

        let group = PreferencesGroup::builder()
            .title("System Services")
            .description("Monitor and manage systemd services on the host system")
            .build();

        let list_box = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        group.add(&list_box);
        root_box.append(&group);

        let clamp = Clamp::builder()
            .maximum_size(880)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        let all_services: Rc<RefCell<Vec<ServiceInfo>>> = Rc::new(RefCell::new(Vec::new()));
        let filter_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        let populate = {
            let list_box = list_box.clone();
            let all_services = all_services.clone();
            let filter_text = filter_text.clone();

            move || {
                while let Some(child) = list_box.first_child() {
                    list_box.remove(&child);
                }

                let query = filter_text.borrow().to_lowercase();
                let services = all_services.borrow();

                let mut count = 0;
                for s in services.iter() {
                    if !query.is_empty() {
                        let unit_match = s.unit.to_lowercase().contains(&query);
                        let desc_match = s.description.to_lowercase().contains(&query);
                        if !unit_match && !desc_match {
                            continue;
                        }
                    }

                    if count >= 80 {
                        break;
                    }
                    count += 1;

                    let row = ActionRow::builder()
                        .title(&s.unit)
                        .subtitle(format!("{} [{}]", s.description, s.active))
                        .build();

                    let icon_name = if s.is_running {
                        "emblem-default-symbolic"
                    } else {
                        "action-unavailable-symbolic"
                    };
                    let img = Image::from_icon_name(icon_name);
                    img.set_pixel_size(18);
                    row.add_prefix(&img);

                    let toggle_btn = Button::builder()
                        .label(if s.is_running { "Stop" } else { "Start" })
                        .css_classes(if s.is_running {
                            vec!["destructive-action"]
                        } else {
                            vec!["suggested-action"]
                        })
                        .valign(Align::Center)
                        .build();

                    let unit_str = s.unit.clone();
                    let is_running = s.is_running;
                    toggle_btn.connect_clicked(move |b| {
                        b.set_sensitive(false);
                        let action = if is_running { "stop" } else { "start" };
                        let _ = ServiceManager::manage_service(&unit_str, action);
                    });

                    row.add_suffix(&toggle_btn);
                    list_box.append(&row);
                }
            }
        };

        let refresh_data = {
            let all_services = all_services.clone();
            let populate = populate.clone();
            let refresh_btn = refresh_btn.clone();

            move || {
                refresh_btn.set_sensitive(false);
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let s = ServiceManager::list_services();
                    let _ = tx.send(s);
                });

                let all_services_inner = all_services.clone();
                let populate_inner = populate.clone();
                let ref_btn = refresh_btn.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if let Ok(services) = rx.try_recv() {
                        *all_services_inner.borrow_mut() = services;
                        populate_inner();
                        ref_btn.set_sensitive(true);
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            }
        };

        let filter_clone = filter_text.clone();
        let populate_clone = populate.clone();
        search_entry.connect_search_changed(move |e| {
            *filter_clone.borrow_mut() = e.text().to_string();
            populate_clone();
        });

        let ref_clone = refresh_data.clone();
        refresh_btn.connect_clicked(move |_| {
            ref_clone();
        });

        refresh_data();

        scrolled
    }
}
