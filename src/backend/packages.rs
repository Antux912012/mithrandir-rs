use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub size_str: String,
    pub description: String,
    pub pkg_type: String, // "native", "flatpak", "snap", "python"
}

pub struct PackageManager;

impl PackageManager {
    pub fn list_packages(pkg_type: &str) -> Vec<PackageItem> {
        match pkg_type {
            "native" => Self::list_native_packages(),
            "flatpak" => Self::list_flatpak_packages(),
            "snap" => Self::list_snap_packages(),
            "python" => Self::list_python_packages(),
            _ => Vec::new(),
        }
    }

    fn list_native_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();

        // Debian/Ubuntu dpkg-query
        if let Ok(out) = Command::new("dpkg-query")
            .args(["-W", "-f=${Package}\t${Version}\t${Installed-Size}\t${binary:Summary}\n"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 4 {
                    let name = parts[0].to_string();
                    let version = parts[1].to_string();
                    let size_kb = parts[2].parse::<f64>().unwrap_or(0.0);
                    let size_str = if size_kb > 1024.0 {
                        format!("{:.1} MB", size_kb / 1024.0)
                    } else {
                        format!("{:.0} KB", size_kb)
                    };
                    let desc = parts[3].to_string();

                    list.push(PackageItem {
                        id: name.clone(),
                        name,
                        version,
                        size_str,
                        description: desc,
                        pkg_type: "native".to_string(),
                    });
                }
            }
        }

        // Limit to reasonable list if huge, but keep alphabetized
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    fn list_flatpak_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();
        if let Ok(out) = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,size,description"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 4 {
                    list.push(PackageItem {
                        id: parts[0].to_string(),
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        size_str: parts[2].to_string(),
                        description: parts[3].to_string(),
                        pkg_type: "flatpak".to_string(),
                    });
                }
            }
        }
        list
    }

    fn list_snap_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();
        if let Ok(out) = Command::new("snap").args(["list"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for (idx, line) in text.lines().enumerate() {
                if idx == 0 {
                    continue; // Header
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    list.push(PackageItem {
                        id: parts[0].to_string(),
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                        size_str: "-".to_string(),
                        description: format!("Snap revision {}", parts[2]),
                        pkg_type: "snap".to_string(),
                    });
                }
            }
        }
        list
    }

    fn list_python_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();
        if let Ok(out) = Command::new("pip").args(["list", "--format=json"]).output() {
            if let Ok(text) = String::from_utf8(out.stdout) {
                if let Ok(val) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                    for item in val {
                        let name = item["name"].as_str().unwrap_or("").to_string();
                        let ver = item["version"].as_str().unwrap_or("").to_string();
                        list.push(PackageItem {
                            id: name.clone(),
                            name,
                            version: ver,
                            size_str: "pip".to_string(),
                            description: "Python Library".to_string(),
                            pkg_type: "python".to_string(),
                        });
                    }
                }
            }
        }
        list
    }

    pub fn uninstall_package(pkg: &PackageItem) -> Result<(), String> {
        let status = match pkg.pkg_type.as_str() {
            "native" => Command::new("pkexec")
                .args(["apt-get", "remove", "-y", &pkg.id])
                .status(),
            "flatpak" => Command::new("flatpak")
                .args(["uninstall", "-y", &pkg.id])
                .status(),
            "snap" => Command::new("pkexec")
                .args(["snap", "remove", &pkg.id])
                .status(),
            "python" => Command::new("pip")
                .args(["uninstall", "-y", &pkg.id])
                .status(),
            _ => return Err("Unknown package type".to_string()),
        };

        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("Uninstallation failed with exit code {:?}", s.code())),
            Err(e) => Err(format!("Failed to execute uninstaller: {}", e)),
        }
    }
}
