use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Image, Label, ListBox, ListBoxRow, Orientation, Separator, Stack,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{AboutDialog, ApplicationWindow, HeaderBar, ToastOverlay, ToolbarView, WindowTitle};
use std::rc::Rc;
use crate::ui::antivirus::AntivirusPage;
use crate::ui::autostart::AutostartPage;
use crate::ui::btrfs::BtrfsPage;
use crate::ui::cleaner::CleanerPage;
use crate::ui::dashboard::DashboardPage;
use crate::ui::grub::GrubPage;
use crate::ui::preferences::PreferencesDialog;
use crate::ui::processes::ProcessesPage;
use crate::ui::services::ServicesPage;
use crate::ui::theme::ThemeManager;
use crate::ui::uninstaller::UninstallerPage;
use crate::ui::ai::AIPage;

pub struct MithrandirWindow;

impl MithrandirWindow {
    pub fn build(app: &adw::Application) -> ApplicationWindow {
        ThemeManager::apply_theme();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Mithrandir-RS")
            .default_width(1100)
            .default_height(740)
            .build();

        let toast_overlay = ToastOverlay::new();
        let toolbar_view = ToolbarView::new();
        let header_bar = HeaderBar::new();

        let title_widget = WindowTitle::new("Mithrandir-RS", "Dashboard");
        header_bar.set_title_widget(Some(&title_widget));

        // Hamburger icon to toggle sidebar
        let menu_btn = Button::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Toggle Navigation Menu")
            .build();
        header_bar.pack_start(&menu_btn);

        // Header Right: Preferences & Info Buttons
        let pref_btn = Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Preferences")
            .build();
        header_bar.pack_end(&pref_btn);

        let info_btn = Button::builder()
            .icon_name("dialog-information-symbolic")
            .tooltip_text("About Mithrandir-RS")
            .build();
        header_bar.pack_end(&info_btn);

        toolbar_view.add_top_bar(&header_bar);

        // Main Layout: Sidebar + Separator + Stack
        let main_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .build();

        // Sidebar
        let sidebar_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .width_request(220)
            .build();
        sidebar_box.add_css_class("sidebar");

        let list_box = ListBox::builder().build();
        list_box.add_css_class("navigation-sidebar");

        // Stack for subpages
        let stack = Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        // Instantiate all subpages
        let page_dashboard = DashboardPage::build(Some(toast_overlay.clone()));
        let page_cleaner = CleanerPage::build();
        let page_processes = ProcessesPage::build();
        let page_autostart = AutostartPage::build();
        let page_services = ServicesPage::build();
        let page_uninstaller = UninstallerPage::build();
        let page_antivirus = AntivirusPage::build();
        let page_btrfs = BtrfsPage::build();
        let page_grub = GrubPage::build();
        let page_ai = AIPage::build();

        stack.add_titled(&page_dashboard, Some("dashboard"), "Dashboard");
        stack.add_titled(&page_cleaner, Some("cleaner"), "Cleaner");
        stack.add_titled(&page_processes, Some("processes"), "Processes");
        stack.add_titled(&page_autostart, Some("autostart"), "Startup Apps");
        stack.add_titled(&page_services, Some("services"), "Services");
        stack.add_titled(&page_uninstaller, Some("uninstaller"), "Uninstaller");
        stack.add_titled(&page_antivirus, Some("antivirus"), "Anti-Malware");
        stack.add_titled(&page_btrfs, Some("btrfs"), "BTRFS Snapshots");
        stack.add_titled(&page_grub, Some("grub"), "GRUB Bootloader");
        stack.add_titled(&page_ai, Some("ai"), "AI Diagnostics");

        let pages = [
            ("dashboard", "Dashboard", "utilities-system-monitor-symbolic"),
            ("cleaner", "Cleaner", "edit-clear-all-symbolic"),
            ("processes", "Processes", "view-list-bullet-symbolic"),
            ("autostart", "Startup Apps", "system-run-symbolic"),
            ("services", "Services", "emblem-system-symbolic"),
            ("uninstaller", "Uninstaller", "system-software-install-symbolic"),
            ("antivirus", "Anti-Malware", "security-high-symbolic"),
            ("btrfs", "BTRFS Snapshots", "drive-harddisk-symbolic"),
            ("grub", "GRUB Bootloader", "preferences-system-symbolic"),
            ("ai", "AI Diagnostics", "system-search-symbolic"),
        ];

        let stack_rc = Rc::new(stack.clone());
        let title_rc = Rc::new(title_widget.clone());

        for (id, label_str, icon_name) in pages {
            let row = ListBoxRow::new();
            let row_box = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(12)
                .margin_top(8)
                .margin_bottom(8)
                .margin_start(12)
                .margin_end(12)
                .build();

            let icon = Image::from_icon_name(icon_name);
            let lbl = Label::builder()
                .label(label_str)
                .hexpand(true)
                .xalign(0.0)
                .build();

            row_box.append(&icon);
            row_box.append(&lbl);
            row.set_child(Some(&row_box));
            list_box.append(&row);

            let stack_clone = stack_rc.clone();
            let title_clone = title_rc.clone();
            let id_str = id.to_string();
            let title_str = label_str.to_string();

            // When selected
            row.connect_activate(move |_| {
                stack_clone.set_visible_child_name(&id_str);
                title_clone.set_subtitle(&title_str);
            });
        }

        let stack_for_nav = stack_rc.clone();
        let title_for_nav = title_rc.clone();
        list_box.connect_row_selected(move |_, row_opt| {
            if let Some(row) = row_opt {
                let idx = row.index();
                if idx >= 0 && (idx as usize) < pages.len() {
                    let (id, title_str, _) = pages[idx as usize];
                    stack_for_nav.set_visible_child_name(id);
                    title_for_nav.set_subtitle(title_str);
                }
            }
        });

        sidebar_box.append(&list_box);
        main_box.append(&sidebar_box);

        let sep = Separator::new(Orientation::Vertical);
        main_box.append(&sep);
        main_box.append(&stack);

        toolbar_view.set_content(Some(&main_box));
        toast_overlay.set_child(Some(&toolbar_view));
        window.set_content(Some(&toast_overlay));

        // Connect hamburger button to toggle sidebar
        let sidebar_clone = sidebar_box.clone();
        menu_btn.connect_clicked(move |_| {
            let vis = sidebar_clone.is_visible();
            sidebar_clone.set_visible(!vis);
        });

        // Connect Preferences
        let win_weak = window.downgrade();
        pref_btn.connect_clicked(move |_| {
            if let Some(win) = win_weak.upgrade() {
                let dlg = PreferencesDialog::build(&win);
                dlg.present();
            }
        });

        // Connect About dialog
        let win_weak_about = window.downgrade();
        info_btn.connect_clicked(move |_| {
            if let Some(win) = win_weak_about.upgrade() {
                let about = AboutDialog::builder()
                    .application_name("Mithrandir-RS")
                    .application_icon("io.github.antonio.MithrandirRS")
                    .developer_name("Antonio")
                    .version("0.4.2")
                    .comments("Blazing-fast GNU/Linux System Cleaner, Optimizer, Telemetry & Diagnostics in Rust (GTK4 & Libadwaita)")
                    .website("https://github.com/Antux912012/mithrandir-rs")
                    .license_type(gtk4::License::Gpl30Only)
                    .build();
                about.present(Some(&win));
            }
        });

        // Handle window close request: hide to background
        window.set_hide_on_close(true);
        window.connect_close_request(move |w| {
            w.set_visible(false);
            gtk4::glib::Propagation::Stop // Prevent window destruction, keep running in background!
        });

        window
    }
}
