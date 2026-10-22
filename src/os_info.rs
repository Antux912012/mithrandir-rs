use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxDistro {
    Debian,
    Ubuntu,
    Fedora,
    Arch,
    CachyOS,
    OpenSuse,
    NixOS,
    Alpine,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsDetails {
    pub id: String,
    pub pretty_name: String,
    pub distro: LinuxDistro,
    pub version_id: Option<String>,
}

/// Parses /etc/os-release content natively into structured OS details.
pub fn parse_os_release(content: &str) -> OsDetails {
    let mut id = "linux".to_string();
    let mut pretty_name = "GNU/Linux".to_string();
    let mut id_like = Vec::new();
    let mut version_id = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("ID=") {
            id = val.trim_matches('"').trim_matches('\'').to_lowercase();
        } else if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
            pretty_name = val.trim_matches('"').trim_matches('\'').to_string();
        } else if let Some(val) = line.strip_prefix("ID_LIKE=") {
            id_like = val
                .trim_matches('"')
                .trim_matches('\'')
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
        } else if let Some(val) = line.strip_prefix("VERSION_ID=") {
            version_id = Some(val.trim_matches('"').trim_matches('\'').to_string());
        }
    }

    let distro = match id.as_str() {
        "debian" => LinuxDistro::Debian,
        "ubuntu" | "linuxmint" | "pop" => LinuxDistro::Ubuntu,
        "fedora" | "rhel" | "rocky" | "almalinux" | "centos" | "nobara" => LinuxDistro::Fedora,
        "arch" | "manjaro" | "endeavouros" => LinuxDistro::Arch,
        "cachyos" => LinuxDistro::CachyOS,
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" | "suse" => LinuxDistro::OpenSuse,
        "nixos" => LinuxDistro::NixOS,
        "alpine" => LinuxDistro::Alpine,
        _ => {
            if id_like.iter().any(|s| s == "fedora" || s == "rhel") {
                LinuxDistro::Fedora
            } else if id_like.iter().any(|s| s == "debian" || s == "ubuntu") {
                LinuxDistro::Debian
            } else if id_like.iter().any(|s| s == "arch") {
                LinuxDistro::Arch
            } else if id_like.iter().any(|s| s == "suse") {
                LinuxDistro::OpenSuse
            } else {
                LinuxDistro::Unknown
            }
        }
    };

    OsDetails {
        id,
        pretty_name,
        distro,
        version_id,
    }
}

/// Detects the current Linux distribution from /etc/os-release or host mount points.
pub fn detect_os() -> OsDetails {
    let candidates = [
        "/etc/os-release",
        "/usr/lib/os-release",
        "/run/host/etc/os-release",
        "/run/host/usr/lib/os-release",
    ];

    for path in &candidates {
        if let Ok(content) = fs::read_to_string(path) {
            return parse_os_release(&content);
        }
    }

    parse_os_release("")
}

/// Returns the primary administrative group ('wheel' vs 'sudo') for a given distro.
pub fn get_admin_group_for_distro(distro: LinuxDistro) -> &'static str {
    match distro {
        LinuxDistro::Arch | LinuxDistro::CachyOS | LinuxDistro::Fedora | LinuxDistro::OpenSuse => "wheel",
        _ => "sudo",
    }
}

/// Returns the administrative group for the active host.
pub fn get_admin_group() -> &'static str {
    let details = detect_os();
    get_admin_group_for_distro(details.distro)
}

/// Checks if the host has an immutable root filesystem (e.g., NixOS or ostree).
pub fn is_immutable_system() -> bool {
    let details = detect_os();
    details.distro == LinuxDistro::NixOS || std::path::Path::new("/run/current-system").exists()
}

/// Checks if SELinux is active (common on Fedora, RHEL, CentOS, openSUSE).
pub fn is_selinux_active() -> bool {
    std::path::Path::new("/sys/fs/selinux").exists()
}

/// Dynamically generates 99-mithrandir-power.rules content targeting the appropriate admin group.
pub fn generate_udev_rule() -> String {
    let group = get_admin_group();
    generate_udev_rule_for_group(group)
}

/// Generates udev rules for a specific administrator group.
pub fn generate_udev_rule_for_group(group: &str) -> String {
    format!(
        "# Mithrandir-RS Hardware Power Saver permissions\n\
        # Dynamically generated for group: {group}\n\
        SUBSYSTEM==\"cpu\", ACTION==\"add\", RUN+=\"/bin/sh -c 'chgrp -R {group} /sys/devices/system/cpu/intel_pstate /sys/devices/system/cpu/cpu*/cpufreq 2>/dev/null && chmod -R g+w /sys/devices/system/cpu/intel_pstate /sys/devices/system/cpu/cpu*/cpufreq'\"\n\
        SUBSYSTEM==\"module\", KERNEL==\"pcie_aspm\", ACTION==\"add\", RUN+=\"/bin/sh -c 'chgrp {group} /sys/module/pcie_aspm/parameters/policy 2>/dev/null && chmod g+w /sys/module/pcie_aspm/parameters/policy'\"\n",
        group = group
    )
}

/// Installs or updates the udev rule on the system.
/// Gracefully skips and notifies if run on NixOS or an immutable system.
pub fn install_udev_rule() -> Result<String, String> {
    let details = detect_os();
    if details.distro == LinuxDistro::NixOS || is_immutable_system() {
        return Err("NixOS detected: /etc is immutable. Please enable power saver services in your configuration.nix".to_string());
    }

    let rule_content = generate_udev_rule();
    let target_path = "/etc/udev/rules.d/99-mithrandir-power.rules";

    // Attempt direct write first
    if fs::write(target_path, &rule_content).is_ok() {
        return Ok(format!("Successfully installed udev rule to {}", target_path));
    }

    // Fall back to pkexec
    let script = format!(
        "mkdir -p /etc/udev/rules.d && cat << 'EOF' > {}\n{}\nEOF\nudevadm control --reload 2>/dev/null || true",
        target_path,
        rule_content
    );

    let output = std::process::Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(&script)
        .output()
        .map_err(|e| format!("Failed to run pkexec: {}", e))?;

    if output.status.success() {
        Ok(format!("Successfully installed udev rule to {}", target_path))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to install udev rule: {}", err.trim()))
    }
}
