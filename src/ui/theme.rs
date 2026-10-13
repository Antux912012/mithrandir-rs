use gtk4::gdk::Display;
use gtk4::CssProvider;

pub struct ThemeManager;

impl ThemeManager {
    pub const ACCENT_COLOR: &'static str = "#B7410E";

    pub const CSS: &'static str = r#"
    /* Mithrandir-RS Terracotta / Rust Theme (#B7410E) */

    headerbar {
        background: linear-gradient(180deg, #6b2406 0%, #4a1803 100%);
        color: #ffffff;
        border-bottom: 1px solid rgba(183, 65, 14, 0.45);
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    }

    headerbar windowtitle, headerbar .title, headerbar .subtitle {
        color: #ffffff;
    }

    headerbar button {
        color: #ffffff;
    }

    headerbar button:hover {
        background: rgba(255, 255, 255, 0.15);
        color: #ffffff;
    }

    .sidebar {
        background: linear-gradient(180deg, #381202 0%, #220b01 100%);
        border-right: 1px solid rgba(183, 65, 14, 0.35);
    }

    .navigation-sidebar {
        background: transparent;
    }

    .navigation-sidebar row {
        color: #fed7aa;
        border-radius: 8px;
        margin: 2px 6px;
        transition: all 150ms ease;
    }

    .navigation-sidebar row:hover {
        background: rgba(183, 65, 14, 0.3);
        color: #ffffff;
    }

    .navigation-sidebar row:selected {
        background: #B7410E;
        color: #ffffff;
        font-weight: 600;
        box-shadow: 0 2px 6px rgba(183, 65, 14, 0.5);
    }

    .smart-boost-card {
        background: linear-gradient(135deg, rgba(183, 65, 14, 0.25) 0%, rgba(74, 24, 3, 0.35) 100%);
        border: 1px solid rgba(183, 65, 14, 0.55);
        border-radius: 12px;
        padding: 16px;
    }

    button.suggested-action {
        background-color: #B7410E;
        color: #ffffff;
        font-weight: bold;
    }

    button.suggested-action:hover {
        background-color: #d8531a;
    }

    progressbar > trough > progress {
        background-color: #B7410E;
        border-radius: 4px;
    }

    .metric-card {
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(183, 65, 14, 0.25);
        border-radius: 12px;
        padding: 14px;
    }
    "#;

    pub fn apply_theme() {
        if let Some(display) = Display::default() {
            let provider = CssProvider::new();
            provider.load_from_string(Self::CSS);
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
}
