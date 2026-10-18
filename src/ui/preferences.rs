use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow, SpinRow, SwitchRow};

pub struct PreferencesDialog;

impl PreferencesDialog {
    pub fn build(parent: &adw::ApplicationWindow) -> PreferencesWindow {
        let win = PreferencesWindow::builder()
            .transient_for(parent)
            .modal(true)
            .title("Preferences")
            .default_width(620)
            .default_height(520)
            .build();

        let page = PreferencesPage::new();

        // 1. Background Service & Integration
        let bg_group = PreferencesGroup::builder()
            .title("Background Service & Integration")
            .description("Configure continuous system optimization and desktop integration")
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

        // 2. Smart Boost Automation
        let boost_group = PreferencesGroup::builder()
            .title("Smart Boost Automation")
            .description("Configure automatic standby cache trimming and RAM thresholds")
            .build();

        let auto_boost_row = SwitchRow::builder()
            .title("Automated Background Optimization")
            .subtitle("Periodically trim standby pagecache when RAM usage is high")
            .active(true)
            .build();
        boost_group.add(&auto_boost_row);

        let ram_thresh_row = SpinRow::builder()
            .title("RAM Threshold (%)")
            .subtitle("Trigger optimization when system memory crosses threshold")
            .adjustment(&gtk4::Adjustment::new(80.0, 50.0, 95.0, 1.0, 5.0, 0.0))
            .build();
        boost_group.add(&ram_thresh_row);

        let cooldown_row = SpinRow::builder()
            .title("Cooldown Interval (minutes)")
            .subtitle("Minimum cooldown between automated boosts to prevent thrashing")
            .adjustment(&gtk4::Adjustment::new(15.0, 1.0, 120.0, 1.0, 5.0, 0.0))
            .build();
        boost_group.add(&cooldown_row);

        let notify_row = SwitchRow::builder()
            .title("Desktop Notifications")
            .subtitle("Notify when RAM is reclaimed or cache is cleaned")
            .active(true)
            .build();
        boost_group.add(&notify_row);
        page.add(&boost_group);

        // 3. Anti-Malware Protection
        let av_group = PreferencesGroup::builder()
            .title("Anti-Malware & ClamAV")
            .description("Virus definition update policies and definitions freshness")
            .build();

        let auto_fresh_row = SwitchRow::builder()
            .title("Auto-Update Before Scanning")
            .subtitle("Invoke FreshClam to fetch latest signatures before executing scans")
            .active(false)
            .build();
        av_group.add(&auto_fresh_row);

        let stale_hours_row = SpinRow::builder()
            .title("Definition Stale Threshold (hours)")
            .subtitle("Warn when local virus definitions are older than this value")
            .adjustment(&gtk4::Adjustment::new(24.0, 1.0, 168.0, 1.0, 12.0, 0.0))
            .build();
        av_group.add(&stale_hours_row);
        page.add(&av_group);

        // 4. AI Diagnostics Defaults
        let ai_group = PreferencesGroup::builder()
            .title("AI Diagnostics & Cloud / Local Inference")
            .description("Supports Google Gemini (Google AI Studio), Anthropic Claude, OpenAI, NVIDIA NIM, Ollama, LM Studio, and In-App llama.cpp")
            .build();

        let default_provider_row = ActionRow::builder()
            .title("Default AI Provider")
            .subtitle("Built-in Linux Mini-Assistant (Offline, Zero-Download)")
            .build();
        ai_group.add(&default_provider_row);

        let gemini_provider_row = ActionRow::builder()
            .title("Google Gemini (Google AI Studio)")
            .subtitle("gemini-2.5-flash on https://generativelanguage.googleapis.com/v1beta")
            .build();
        ai_group.add(&gemini_provider_row);

        let inapp_model_row = ActionRow::builder()
            .title("Recommended In-App Model")
            .subtitle("Qwen 2.5 1.5B Instruct GGUF on http://127.0.0.1:18080/v1")
            .build();
        ai_group.add(&inapp_model_row);
        page.add(&ai_group);

        win.add(&page);
        win
    }
}
