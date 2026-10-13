use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtrfsSnapshot {
    pub id: String,
    pub path: String,
    pub timestamp: String,
}

pub struct BtrfsManager;

impl BtrfsManager {
    pub fn is_btrfs_available() -> bool {
        if let Ok(out) = Command::new("findmnt").args(["-n", "-o", "FSTYPE", "/"]).output() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            return s == "btrfs";
        }
        false
    }

    pub fn list_snapshots() -> Vec<BtrfsSnapshot> {
        let mut list = Vec::new();
        if let Ok(out) = Command::new("btrfs").args(["subvolume", "list", "/"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                // e.g. ID 256 gen 1234 top level 5 path @snapshots/2026-10-13
                if line.contains("path") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let id = parts.get(1).unwrap_or(&"?").to_string();
                    let path = parts.last().unwrap_or(&"").to_string();
                    list.push(BtrfsSnapshot {
                        id,
                        path: path.clone(),
                        timestamp: "Local Subvolume".to_string(),
                    });
                }
            }
        }
        list
    }

    pub fn create_snapshot(name: &str) -> Result<(), String> {
        let snap_path = format!("/.snapshots/{}", name);
        let status = Command::new("pkexec")
            .args(["btrfs", "subvolume", "snapshot", "/", &snap_path])
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("Snapshot creation returned exit code {:?}", s.code())),
            Err(e) => Err(format!("Failed to execute btrfs snapshot: {}", e)),
        }
    }
}
