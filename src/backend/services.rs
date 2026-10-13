use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub unit: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
    pub is_running: bool,
}

pub struct ServiceManager;

impl ServiceManager {
    pub fn list_services() -> Vec<ServiceInfo> {
        let output = Command::new("systemctl")
            .args(["list-units", "--type=service", "--all", "--no-legend", "--no-pager"])
            .output();

        let mut services = Vec::new();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let unit = parts[0].trim_start_matches('●').trim().to_string();
                    let load = parts[1].to_string();
                    let active = parts[2].to_string();
                    let sub = parts[3].to_string();
                    let desc = parts[4..].join(" ");
                    let is_running = active == "active" || sub == "running";

                    services.push(ServiceInfo {
                        unit,
                        load,
                        active,
                        sub,
                        description: desc,
                        is_running,
                    });
                }
            }
        }

        services.sort_by(|a, b| a.unit.cmp(&b.unit));
        services
    }

    pub fn manage_service(unit: &str, action: &str) -> Result<(), String> {
        // Try user systemctl first, fall back to pkexec systemctl
        let status = Command::new("systemctl")
            .args([action, unit])
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                let pk = Command::new("pkexec")
                    .args(["systemctl", action, unit])
                    .status()
                    .map_err(|e| e.to_string())?;
                if pk.success() {
                    Ok(())
                } else {
                    Err(format!("Failed to {} {}", action, unit))
                }
            }
        }
    }
}
