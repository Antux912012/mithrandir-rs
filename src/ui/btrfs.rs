use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, ListBox, Orientation, ScrolledWindow,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup};
use crate::backend::btrfs::BtrfsManager;

pub struct BtrfsPage;

impl BtrfsPage {
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

        let is_btrfs = BtrfsManager::is_btrfs_available();

        let status_group = PreferencesGroup::builder()
            .title("BTRFS Subvolumes & Snapshots")
            .description("Manage system restore points and subvolumes")
            .build();

        let status_row = ActionRow::builder()
            .title("Filesystem Status")
            .subtitle(if is_btrfs {
                "Root partition is formatted as BTRFS. Instant snapshots supported."
            } else {
                "Root partition is ext4/other. BTRFS snapshotting is inactive."
            })
            .build();
        status_group.add(&status_row);
        root_box.append(&status_group);

        // Actions
        let actions_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .halign(Align::End)
            .build();

        let create_btn = Button::builder()
            .label("Create Snapshot")
            .css_classes(vec!["suggested-action"])
            .sensitive(is_btrfs)
            .build();
        actions_box.append(&create_btn);
        root_box.append(&actions_box);

        // List Group
        let list_group = PreferencesGroup::builder()
            .title("Existing Snapshots")
            .build();

        let list_box = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        list_group.add(&list_box);
        root_box.append(&list_group);

        let snapshots = BtrfsManager::list_snapshots();
        if snapshots.is_empty() {
            let empty_row = ActionRow::builder()
                .title("No BTRFS snapshots detected")
                .subtitle("Create a snapshot before upgrading packages or editing system files")
                .build();
            list_box.append(&empty_row);
        } else {
            for s in snapshots {
                let row = ActionRow::builder()
                    .title(&s.path)
                    .subtitle(format!("Subvolume ID: {}", s.id))
                    .build();
                list_box.append(&row);
            }
        }

        let clamp = Clamp::builder()
            .maximum_size(880)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        scrolled
    }
}
