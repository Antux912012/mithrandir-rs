use std::sync::atomic::{AtomicBool, Ordering};

static POWER_SAVER_STATE: AtomicBool = AtomicBool::new(false);

/// Hardware-level Power Saver controller managing:
/// - Intel P-State CPU Turbo Boost (`intel_pstate/no_turbo`)
/// - Energy Performance Preference EPP (`cpufreq/energy_performance_preference`)
/// - PCIe Active State Power Management ASPM (`pcie_aspm/parameters/policy`)
pub struct PowerManager;

impl PowerManager {
    /// Detects if hardware-level power saving policies are supported on this machine.
    pub fn is_supported() -> bool {
        std::path::Path::new("/sys/devices/system/cpu/intel_pstate/no_turbo").exists()
            || std::path::Path::new("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference").exists()
            || std::path::Path::new("/sys/module/pcie_aspm/parameters/policy").exists()
    }

    /// Reads sysfs hardware state at startup to determine if power saver mode is active.
    pub fn is_power_saver_active() -> bool {
        // 1. Intel P-State no_turbo check (1 = Turbo Disabled = Power Saver active)
        if let Ok(content) = std::fs::read_to_string("/sys/devices/system/cpu/intel_pstate/no_turbo") {
            if content.trim() == "1" {
                POWER_SAVER_STATE.store(true, Ordering::SeqCst);
                return true;
            }
        }

        // 2. CPU0 EPP check
        if let Ok(content) = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference") {
            let val = content.trim();
            if val == "power" || val == "powersave" {
                POWER_SAVER_STATE.store(true, Ordering::SeqCst);
                return true;
            }
        }

        // 3. PCIe ASPM check
        if let Ok(content) = std::fs::read_to_string("/sys/module/pcie_aspm/parameters/policy") {
            if content.contains("[powersupersave]") {
                POWER_SAVER_STATE.store(true, Ordering::SeqCst);
                return true;
            }
        }

        let cached = POWER_SAVER_STATE.load(Ordering::SeqCst);
        cached
    }

    /// Updates internal cached power saver flag.
    pub fn set_cached_state(active: bool) {
        POWER_SAVER_STATE.store(active, Ordering::SeqCst);
    }

    /// Toggles hardware power saving settings across CPU and PCIe bus.
    /// Attempts direct unprivileged sysfs write first (facilitated by udev rule).
    /// Falls back to a batched `pkexec` execution if unprivileged write is denied.
    pub fn set_power_saver(enable: bool) -> Result<(), String> {
        let no_turbo = if enable { "1" } else { "0" };
        let epp = if enable { "power" } else { "balance_performance" };
        let aspm = if enable { "powersupersave" } else { "default" };

        let mut needs_pkexec = false;
        let mut paths_to_write: Vec<(String, &'static str)> = Vec::new();

        let pstate_path = "/sys/devices/system/cpu/intel_pstate/no_turbo";
        if std::path::Path::new(pstate_path).exists() {
            paths_to_write.push((pstate_path.to_string(), no_turbo));
        }

        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("cpu") && fname[3..].chars().all(|c| c.is_ascii_digit()) {
                    let epp_path = entry.path().join("cpufreq/energy_performance_preference");
                    if epp_path.exists() {
                        paths_to_write.push((epp_path.to_string_lossy().to_string(), epp));
                    }
                }
            }
        }

        let aspm_path = "/sys/module/pcie_aspm/parameters/policy";
        if std::path::Path::new(aspm_path).exists() {
            paths_to_write.push((aspm_path.to_string(), aspm));
        }

        // Try direct unprivileged write
        for (path, val) in &paths_to_write {
            if let Err(e) = std::fs::write(path, val) {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    needs_pkexec = true;
                    break;
                }
            }
        }

        if !needs_pkexec && !paths_to_write.is_empty() {
            POWER_SAVER_STATE.store(enable, Ordering::SeqCst);
            return Ok(());
        }

        // Fallback to batched pkexec execution
        let script = format!(
            r#"
            if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
                echo "{no_turbo}" > /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || true
            fi
            for f in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do
                if [ -f "$f" ]; then
                    echo "{epp}" > "$f" 2>/dev/null || true
                fi
            done
            if [ -f /sys/module/pcie_aspm/parameters/policy ]; then
                echo "{aspm}" > /sys/module/pcie_aspm/parameters/policy 2>/dev/null || true
            fi
            "#,
            no_turbo = no_turbo,
            epp = epp,
            aspm = aspm
        );

        let output = std::process::Command::new("pkexec")
            .arg("sh")
            .arg("-c")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to invoke pkexec: {}", e))?;

        if output.status.success() {
            POWER_SAVER_STATE.store(enable, Ordering::SeqCst);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("pkexec error: {}", stderr.trim()))
        }
    }
}
