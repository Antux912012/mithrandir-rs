use libadwaita as adw;
use adw::prelude::*;
use adw::{PreferencesPage, PreferencesWindow, SwitchRow};

pub struct PreferencesDialog;

impl PreferencesDialog {
    pub fn build(parent: &adw::ApplicationWindow) -> PreferencesWindow {
        let win = PreferencesWindow::builder()
            .transient_for(parent)
            .modal(true)
            .title("Preferences")
            .default_width(560)
            .default_height(420)
            .build();

        let page = PreferencesPage::new();

        let bg_group = adw::PreferencesGroup::builder()
            .title("Background Service & Integration")
            .description("Configure continuous system optimization and top bar presence")
            .build();

        let keep_bg_row = SwitchRow::builder()
            .title("Keep Running in Background")
            .subtitle("Hide window on close instead of exiting")
            .active(true)
            .build();
        bg_group.add(&keep_bg_row);

        let indicator_row = SwitchRow::builder()
            .title("Show GNOME Top Bar Applet")
            .subtitle("Display quick-boost indicator in status area")
            .active(true)
            .build();
        bg_group.add(&indicator_row);

        page.add(&bg_group);

        let boost_group = adw::PreferencesGroup::builder()
            .title("Smart Boost Automation")
            .description("Automated memory trimming and cache pruning")
            .build();

        let auto_boost_row = SwitchRow::builder()
            .title("Automated Background Optimization")
            .subtitle("Periodically trim standby pagecache when RAM usage is high")
            .active(true)
            .build();
        boost_group.add(&auto_boost_row);

        let notify_row = SwitchRow::builder()
            .title("Desktop Notifications")
            .subtitle("Notify when RAM is reclaimed or cache is cleaned")
            .active(true)
            .build();
        boost_group.add(&notify_row);

        page.add(&boost_group);
        win.add(&page);

        win
    }
}
