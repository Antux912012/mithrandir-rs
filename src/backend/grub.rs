use std::fs;
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrubConfig {
    pub timeout: i32,
    pub default_entry: String,
    pub cmdline: String,
}

pub struct GrubManager;

impl GrubManager {
    pub fn read_config() -> GrubConfig {
        let mut config = GrubConfig {
            timeout: 5,
            default_entry: "0".to_string(),
            cmdline: "quiet splash".to_string(),
        };

        if let Ok(content) = fs::read_to_string("/etc/default/grub") {
            for line in content.lines() {
                let l = line.trim();
                if let Some(val) = l.strip_prefix("GRUB_TIMEOUT=") {
                    let v = val.trim_matches('"');
                    if let Ok(t) = v.parse::<i32>() {
                        config.timeout = t;
                    }
                } else if let Some(val) = l.strip_prefix("GRUB_DEFAULT=") {
                    config.default_entry = val.trim_matches('"').to_string();
                } else if let Some(val) = l.strip_prefix("GRUB_CMDLINE_LINUX_DEFAULT=") {
                    config.cmdline = val.trim_matches('"').to_string();
                }
            }
        }

        config
    }

    pub fn save_and_update(timeout: i32, default_entry: &str) -> Result<(), String> {
        let script = format!(
            "sed -i 's/^GRUB_TIMEOUT=.*/GRUB_TIMEOUT={}/' /etc/default/grub && \
             sed -i 's/^GRUB_DEFAULT=.*/GRUB_DEFAULT=\"{}\"/' /etc/default/grub && \
             (update-grub || grub2-mkconfig -o /boot/grub/grub.cfg)",
            timeout, default_entry
        );

        let status = Command::new("pkexec")
            .args(["sh", "-c", &script])
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("GRUB update exited with code {:?}", status.code()))
        }
    }
}
