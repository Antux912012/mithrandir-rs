use std::process::Command;
use crate::backend::smart_boost::SmartBoostEngine;

pub struct NotificationManager;

impl NotificationManager {
    pub fn send_notification(summary: &str, body: &str) {
        let _ = Command::new("notify-send")
            .args([
                "--app-name=Mithrandir-RS",
                "--icon=io.github.antonio.MithrandirRS",
                summary,
                body,
            ])
            .status();
    }
}

pub struct BackgroundDaemon;

impl BackgroundDaemon {
    pub fn run_auto_boost_check(ram_threshold: f64) -> bool {
        let status = SmartBoostEngine::get_boost_status();
        if status.ram_percent >= ram_threshold {
            let res = SmartBoostEngine::execute_boost(true, true);
            let msg = format!(
                "Reclaimed {:.1} MB RAM and freed {:.1} MB disk cache.",
                res.ram_reclaimed_mb, res.disk_freed_mb
            );
            NotificationManager::send_notification("⚡ Smart Boost Automated", &msg);
            true
        } else {
            false
        }
    }
}
