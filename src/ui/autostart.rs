use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Image, Label, Orientation, ScrolledWindow, Switch,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup, SwitchRow};
use std::cell::RefCell;
use std::rc::Rc;
use crate::backend::autostart::AutostartManager;

pub struct AutostartPage;

impl AutostartPage {
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

        // 1. Mithrandir Autostart Switch
        let m_group = PreferencesGroup::builder()
            .title("Mithrandir Background Launch")
            .description("Keep Mithrandir protecting and optimizing your system on boot")
            .build();

        let m_switch_row = SwitchRow::builder()
            .title("Start Mithrandir Automatically on Login")
            .subtitle("Launches in the background to monitor RAM, cache clutter, and sensors")
            .active(AutostartManager::is_mithrandir_autostart_enabled())
            .build();

        m_switch_row.connect_active_notify(|row| {
            let active = row.is_active();
            let _ = AutostartManager::set_mithrandir_autostart(active);
        });

        m_group.add(&m_switch_row);
        root_box.append(&m_group);

        // 2. Startup Applications Group
        let apps_header = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let title_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let lbl = Label::builder()
            .label("Startup Applications")
            .css_classes(vec!["title-3"])
            .xalign(0.0)
            .build();
        let sub_lbl = Label::builder()
            .label("Manage programs configured to launch automatically on login")
            .css_classes(vec!["caption", "dim-label"])
            .xalign(0.0)
            .build();
        title_box.append(&lbl);
        title_box.append(&sub_lbl);
        apps_header.append(&title_box);

        root_box.append(&apps_header);

        let apps_group = PreferencesGroup::builder().build();
        root_box.append(&apps_group);

        let apps_group_rc = Rc::new(RefCell::new(apps_group));

        let reload_entries = {
            let apps_group_rc = apps_group_rc.clone();

            move || {
                let group = apps_group_rc.borrow().clone();
                let entries = AutostartManager::list_entries();

                for e in entries {
                    let row = ActionRow::builder()
                        .title(&e.name)
                        .subtitle(&e.exec)
                        .build();

                    let icon_name = if !e.icon.is_empty() {
                        e.icon.clone()
                    } else {
                        "application-x-executable-symbolic".to_string()
                    };
                    let img = Image::from_icon_name(&icon_name);
                    img.set_pixel_size(24);
                    row.add_prefix(&img);

                    let sw = Switch::builder()
                        .active(e.enabled)
                        .valign(Align::Center)
                        .build();

                    let e_clone = e.clone();
                    sw.connect_active_notify(move |s| {
                        let _ = AutostartManager::toggle_entry(&e_clone, s.is_active());
                    });
                    row.add_suffix(&sw);

                    if e.is_user {
                        let del_btn = Button::builder()
                            .icon_name("user-trash-symbolic")
                            .valign(Align::Center)
                            .css_classes(vec!["flat"])
                            .tooltip_text("Remove entry")
                            .build();

                        let e_del = e.clone();
                        let group_del = group.clone();
                        let row_del = row.clone();

                        del_btn.connect_clicked(move |_| {
                            let _ = AutostartManager::remove_entry(&e_del);
                            group_del.remove(&row_del);
                        });
                        row.add_suffix(&del_btn);
                    }

                    group.add(&row);
                }
            }
        };

        reload_entries();

        let clamp = Clamp::builder()
            .maximum_size(920)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        scrolled
    }
}
