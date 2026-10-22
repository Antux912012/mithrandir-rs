use serde::{Deserialize, Serialize};
use crate::os_info::{detect_os, LinuxDistro};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistroFamily {
    Debian,
    Fedora,
    Arch,
    Suse,
    Alpine,
    Unknown,
}

impl From<LinuxDistro> for DistroFamily {
    fn from(distro: LinuxDistro) -> Self {
        match distro {
            LinuxDistro::Debian | LinuxDistro::Ubuntu => DistroFamily::Debian,
            LinuxDistro::Fedora => DistroFamily::Fedora,
            LinuxDistro::Arch | LinuxDistro::CachyOS => DistroFamily::Arch,
            LinuxDistro::OpenSuse => DistroFamily::Suse,
            LinuxDistro::Alpine => DistroFamily::Alpine,
            LinuxDistro::NixOS | LinuxDistro::Unknown => DistroFamily::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroInfo {
    pub id: String,
    pub pretty_name: String,
    pub family: DistroFamily,
}

pub struct DistroDetector;

impl DistroDetector {
    /// Detects current GNU/Linux distribution by querying centralized os_info.
    pub fn detect() -> DistroInfo {
        let details = detect_os();
        DistroInfo {
            id: details.id,
            pretty_name: details.pretty_name,
            family: DistroFamily::from(details.distro),
        }
    }

    /// Returns package manager command to install ClamAV and FreshClam.
    pub fn get_clamav_install_command() -> (&'static str, Vec<&'static str>) {
        let info = Self::detect();
        match info.family {
            DistroFamily::Debian => ("sh", vec!["-c", "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y clamav clamav-daemon clamav-freshclam"]),
            DistroFamily::Fedora => ("dnf", vec!["install", "-y", "clamav", "clamav-freshclam", "clamd"]),
            DistroFamily::Arch => ("pacman", vec!["-Sy", "--noconfirm", "clamav"]),
            DistroFamily::Suse => ("zypper", vec!["--non-interactive", "install", "clamav"]),
            DistroFamily::Alpine => ("apk", vec!["add", "clamav", "clamav-libunrar"]),
            DistroFamily::Unknown => ("sh", vec!["-c", "apt-get update && apt-get install -y clamav clamav-freshclam || dnf install -y clamav clamav-freshclam || pacman -Sy --noconfirm clamav"]),
        }
    }
}
