use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, Orientation, ScrolledWindow, SpinButton,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup};
use crate::backend::grub::GrubManager;

pub struct GrubPage;

impl GrubPage {
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

        let cfg = GrubManager::read_config();

        let group = PreferencesGroup::builder()
            .title("GRUB Bootloader Configuration")
            .description("Customize system boot menu delay and default kernel")
            .build();

        // Timeout row
        let timeout_row = ActionRow::builder()
            .title("Boot Menu Timeout (seconds)")
            .subtitle("Number of seconds before automatically booting default selection")
            .build();

        let timeout_spin = SpinButton::with_range(0.0, 30.0, 1.0);
        timeout_spin.set_value(cfg.timeout as f64);
        timeout_spin.set_valign(Align::Center);
        timeout_row.add_suffix(&timeout_spin);
        group.add(&timeout_row);

        // Default entry row
        let default_row = ActionRow::builder()
            .title("Default Boot Entry")
            .subtitle("Index or name of the kernel to boot by default")
            .build();

        let default_entry = Entry::builder()
            .text(&cfg.default_entry)
            .valign(Align::Center)
            .build();
        default_row.add_suffix(&default_entry);
        group.add(&default_row);

        // Kernel Cmdline row
        let cmdline_row = ActionRow::builder()
            .title("Kernel Boot Parameters")
            .subtitle(&cfg.cmdline)
            .build();
        group.add(&cmdline_row);

        root_box.append(&group);

        // Save Button
        let btn_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::End)
            .build();

        let save_btn = Button::builder()
            .label("Save & Update GRUB")
            .css_classes(vec!["suggested-action"])
            .build();
        btn_box.append(&save_btn);
        root_box.append(&btn_box);

        let spin_clone = timeout_spin.clone();
        let entry_clone = default_entry.clone();
        let save_clone = save_btn.clone();

        save_btn.connect_clicked(move |_| {
            save_clone.set_sensitive(false);
            save_clone.set_label("Updating GRUB...");
            let timeout = spin_clone.value() as i32;
            let def = entry_clone.text().to_string();
            let s_btn = save_clone.clone();

            std::thread::spawn(move || {
                let _ = GrubManager::save_and_update(timeout, &def);
            });

            glib::timeout_add_local_once(std::time::Duration::from_millis(2000), move || {
                s_btn.set_sensitive(true);
                s_btn.set_label("Save & Update GRUB");
            });
        });

        let clamp = Clamp::builder()
            .maximum_size(880)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        scrolled
    }
}
