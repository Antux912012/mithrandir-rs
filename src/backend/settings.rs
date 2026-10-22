use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub keep_in_background: bool,
    pub show_tray_icon: bool,
    pub auto_boost_enabled: bool,
    pub ram_threshold: f64,
    pub cooldown_minutes: u64,
    pub desktop_notifications: bool,
    pub auto_freshclam: bool,
    pub stale_hours_threshold: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            keep_in_background: true,
            show_tray_icon: true,
            auto_boost_enabled: true,
            ram_threshold: 80.0,
            cooldown_minutes: 15,
            desktop_notifications: true,
            auto_freshclam: false,
            stale_hours_threshold: 24,
        }
    }
}

static SETTINGS_INSTANCE: OnceLock<Mutex<AppSettings>> = OnceLock::new();

impl AppSettings {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".config/io.github.antonio.MithrandirRS/settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppSettings>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, content);
        }
    }

    fn instance() -> &'static Mutex<AppSettings> {
        SETTINGS_INSTANCE.get_or_init(|| Mutex::new(Self::load()))
    }

    pub fn get() -> Self {
        Self::instance().lock().unwrap().clone()
    }

    pub fn update<F: FnOnce(&mut AppSettings)>(f: F) {
        let mut lock = Self::instance().lock().unwrap();
        f(&mut lock);
        lock.save();
    }
}
