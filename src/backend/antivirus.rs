use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFinding {
    pub file_path: String,
    pub virus_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamStatus {
    pub is_installed: bool,
    pub version: String,
    pub definitions_date: String,
}

pub struct AntivirusEngine;

impl AntivirusEngine {
    pub fn get_status() -> ClamStatus {
        let mut status = ClamStatus {
            is_installed: false,
            version: "Not Installed".to_string(),
            definitions_date: "Unknown".to_string(),
        };

        if let Ok(out) = Command::new("clamscan").arg("--version").output() {
            if out.status.success() {
                status.is_installed = true;
                status.version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
        }

        let def_path = Path::new("/var/lib/clamav/daily.cvd");
        if def_path.exists() {
            if let Ok(meta) = def_path.metadata() {
                if let Ok(mtime) = meta.modified() {
                    status.definitions_date = format!("{:?}", mtime);
                }
            }
        }

        status
    }

    pub fn scan_directory(target_path: &str) -> Result<Vec<ThreatFinding>, String> {
        let out = Command::new("clamscan")
            .args(["-r", "--no-summary", target_path])
            .output()
            .map_err(|e| e.to_string())?;

        let mut findings = Vec::new();
        let stdout = String::from_utf8_lossy(&out.stdout);

        for line in stdout.lines() {
            if line.ends_with("FOUND") {
                // e.g. /path/to/file: Eicar-Signature FOUND
                let parts: Vec<&str> = line.split(": ").collect();
                if parts.len() >= 2 {
                    let file = parts[0].to_string();
                    let virus = parts[1].trim_end_matches(" FOUND").to_string();
                    findings.push(ThreatFinding {
                        file_path: file,
                        virus_name: virus,
                    });
                }
            }
        }

        Ok(findings)
    }

    pub fn update_definitions() -> Result<(), String> {
        let status = Command::new("pkexec")
            .args(["freshclam"])
            .status()
            .map_err(|e| e.to_string())?;

        if status.success() {
            Ok(())
        } else {
            Err("Freshclam update returned non-zero exit code".to_string())
        }
    }
}
