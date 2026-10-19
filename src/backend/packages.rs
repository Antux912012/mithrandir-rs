use serde::{Deserialize, Serialize};
use crate::backend::distro::{DistroDetector, DistroFamily};
use crate::backend::host_bridge::HostBridge;

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
        let distro = DistroDetector::detect();
        let mut list = Vec::new();

        match distro.family {
            DistroFamily::Debian => {
                if let Ok((code, stdout, _)) = HostBridge::run_command(
                    "dpkg-query",
                    &["-W", "-f=${Package}\t${Version}\t${Installed-Size}\t${binary:Summary}\n"],
                ) {
                    if code == 0 {
                        for line in stdout.lines() {
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
                }
            }
            DistroFamily::Fedora => {
                if let Ok((code, stdout, _)) = HostBridge::run_command(
                    "rpm",
                    &["-qa", "--qf", "%{NAME}\t%{VERSION}-%{RELEASE}\t%{SIZE}\t%{SUMMARY}\n"],
                ) {
                    if code == 0 {
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.split('\t').collect();
                            if parts.len() >= 4 {
                                let name = parts[0].to_string();
                                let version = parts[1].to_string();
                                let size_b = parts[2].parse::<f64>().unwrap_or(0.0);
                                let size_str = format!("{:.1} MB", size_b / (1024.0 * 1024.0));
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
                }
            }
            DistroFamily::Arch => {
                if let Ok((code, stdout, _)) = HostBridge::run_command("pacman", &["-Qe"]) {
                    if code == 0 {
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                list.push(PackageItem {
                                    id: parts[0].to_string(),
                                    name: parts[0].to_string(),
                                    version: parts[1].to_string(),
                                    size_str: "native".to_string(),
                                    description: "Arch Linux Package".to_string(),
                                    pkg_type: "native".to_string(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {
                // Fallback attempt: dpkg or rpm
                if let Ok((code, stdout, _)) = HostBridge::run_command("dpkg-query", &["-W", "-f=${Package}\t${Version}\t${Installed-Size}\t${binary:Summary}\n"]) {
                    if code == 0 {
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.split('\t').collect();
                            if parts.len() >= 4 {
                                list.push(PackageItem {
                                    id: parts[0].to_string(),
                                    name: parts[0].to_string(),
                                    version: parts[1].to_string(),
                                    size_str: format!("{:.1} MB", parts[2].parse::<f64>().unwrap_or(0.0) / 1024.0),
                                    description: parts[3].to_string(),
                                    pkg_type: "native".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        list.sort_by_key(|a| a.name.to_lowercase());
        list
    }

    fn list_flatpak_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();
        if let Ok((code, stdout, _)) = HostBridge::run_command("flatpak", &["list", "--app", "--columns=application,version,size,description"]) {
            if code == 0 {
                for line in stdout.lines() {
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
        }
        list
    }

    fn list_snap_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();
        if let Ok((code, stdout, _)) = HostBridge::run_command("snap", &["list"]) {
            if code == 0 {
                for (idx, line) in stdout.lines().enumerate() {
                    if idx == 0 {
                        continue; // Skip header
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
        }
        list
    }

    fn list_python_packages() -> Vec<PackageItem> {
        let mut list = Vec::new();
        if let Ok((code, stdout, _)) = HostBridge::run_command("pip", &["list", "--format=json"]) {
            if code == 0 {
                if let Ok(val) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
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
        let distro = DistroDetector::detect();
        let res = match pkg.pkg_type.as_str() {
            "native" => match distro.family {
                DistroFamily::Debian => HostBridge::run_privileged(&["apt-get", "remove", "-y", &pkg.id]),
                DistroFamily::Fedora => HostBridge::run_privileged(&["dnf", "remove", "-y", &pkg.id]),
                DistroFamily::Arch => HostBridge::run_privileged(&["pacman", "-R", "--noconfirm", &pkg.id]),
                DistroFamily::Alpine => HostBridge::run_privileged(&["apk", "del", &pkg.id]),
                _ => HostBridge::run_privileged(&["apt-get", "remove", "-y", &pkg.id]),
            },
            "flatpak" => HostBridge::run_command("flatpak", &["uninstall", "-y", &pkg.id]),
            "snap" => HostBridge::run_privileged(&["snap", "remove", &pkg.id]),
            "python" => HostBridge::run_command("pip", &["uninstall", "-y", &pkg.id]),
            _ => return Err("Unknown package type".to_string()),
        }?;

        if res.0 == 0 {
            Ok(())
        } else {
            Err(format!("Uninstallation failed with exit code {}: {}", res.0, res.2))
        }
    }
}
