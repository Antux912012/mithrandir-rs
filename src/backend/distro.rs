use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistroFamily {
    Debian,
    Fedora,
    Arch,
    Suse,
    Alpine,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroInfo {
    pub id: String,
    pub pretty_name: String,
    pub family: DistroFamily,
}

pub struct DistroDetector;

impl DistroDetector {
    /// Detects current GNU/Linux distribution by inspecting os-release files.
    pub fn detect() -> DistroInfo {
        let candidates = [
            "/etc/os-release",
            "/usr/lib/os-release",
            "/run/host/etc/os-release",
            "/run/host/usr/lib/os-release",
        ];

        let mut id = "linux".to_string();
        let mut pretty_name = "GNU/Linux".to_string();
        let mut id_like = String::new();

        for path in &candidates {
            if let Ok(content) = fs::read_to_string(Path::new(path)) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("ID=") {
                        id = line.trim_start_matches("ID=").trim_matches('"').to_lowercase();
                    } else if line.starts_with("PRETTY_NAME=") {
                        pretty_name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string();
                    } else if line.starts_with("ID_LIKE=") {
                        id_like = line.trim_start_matches("ID_LIKE=").trim_matches('"').to_lowercase();
                    }
                }
                break;
            }
        }

        let combined = format!("{} {}", id, id_like);
        let family = if combined.contains("ubuntu") || combined.contains("debian") || combined.contains("linuxmint") || combined.contains("pop") {
            DistroFamily::Debian
        } else if combined.contains("fedora") || combined.contains("rhel") || combined.contains("centos") || combined.contains("nobara") {
            DistroFamily::Fedora
        } else if combined.contains("arch") || combined.contains("manjaro") || combined.contains("endeavouros") || combined.contains("cachyos") {
            DistroFamily::Arch
        } else if combined.contains("suse") || combined.contains("opensuse") {
            DistroFamily::Suse
        } else if combined.contains("alpine") {
            DistroFamily::Alpine
        } else {
            DistroFamily::Unknown
        };

        DistroInfo {
            id,
            pretty_name,
            family,
        }
    }

    /// Returns package manager command to install ClamAV and FreshClam.
    pub fn get_clamav_install_command() -> (&'static str, Vec<&'static str>) {
        let info = Self::detect();
        match info.family {
            DistroFamily::Debian => ("sh", vec!["-c", "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y clamav clamav-daemon clamav-freshclam"]),
            DistroFamily::Fedora => ("dnf", vec!["install", "-y", "clamav", "clamav-update", "clamd"]),
            DistroFamily::Arch => ("pacman", vec!["-Sy", "--noconfirm", "clamav"]),
            DistroFamily::Suse => ("zypper", vec!["--non-interactive", "install", "clamav"]),
            DistroFamily::Alpine => ("apk", vec!["add", "clamav", "clamav-libunrar"]),
            DistroFamily::Unknown => ("sh", vec!["-c", "apt-get update && apt-get install -y clamav clamav-freshclam || dnf install -y clamav clamav-update || pacman -Sy --noconfirm clamav"]),
        }
    }
}
