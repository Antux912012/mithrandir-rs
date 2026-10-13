use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Image, Label, ListBox, ListBoxRow, Orientation, Separator, Stack};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar, ToolbarView, WindowTitle};
use crate::ui::dashboard::DashboardPage;
use crate::ui::theme::ThemeManager;

pub struct MithrandirWindow;

impl MithrandirWindow {
    pub fn build(app: &adw::Application) -> ApplicationWindow {
        ThemeManager::apply_theme();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Mithrandir (Rust)")
            .default_width(1080)
            .default_height(720)
            .build();

        let toolbar_view = ToolbarView::new();
        let header_bar = HeaderBar::new();

        let title_widget = WindowTitle::new("Mithrandir (Rust)", "System Optimizer & Diagnostics");
        header_bar.set_title_widget(Some(&title_widget));

        // Hamburger icon
        let menu_btn = Button::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Navigation Menu")
            .build();
        header_bar.pack_start(&menu_btn);

        toolbar_view.add_top_bar(&header_bar);

        // Main Layout: Sidebar + Separator + Stack
        let main_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .build();

        // Sidebar
        let sidebar_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .width_request(200)
            .build();
        sidebar_box.add_css_class("sidebar");

        let list_box = ListBox::builder().build();
        list_box.add_css_class("navigation-sidebar");

        // Dashboard Row
        let row_dash = ListBoxRow::new();
        let row_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(10)
            .margin_end(10)
            .build();

        let icon = Image::from_icon_name("utilities-system-monitor-symbolic");
        let lbl = Label::builder().label("Dashboard").hexpand(true).xalign(0.0).build();
        row_box.append(&icon);
        row_box.append(&lbl);
        row_dash.set_child(Some(&row_box));
        list_box.append(&row_dash);

        sidebar_box.append(&list_box);
        main_box.append(&sidebar_box);

        let sep = Separator::new(Orientation::Vertical);
        main_box.append(&sep);

        // Stack
        let stack = Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let dashboard_page = DashboardPage::build();
        stack.add_titled(&dashboard_page, Some("dashboard"), "Dashboard");

        main_box.append(&stack);

        toolbar_view.set_content(Some(&main_box));
        window.set_content(Some(&toolbar_view));

        window
    }
}
