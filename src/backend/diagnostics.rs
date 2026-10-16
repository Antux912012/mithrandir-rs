use std::env;
use crate::ai::SafetyGuard;
use crate::backend::distro::DistroDetector;
use crate::backend::host_bridge::HostBridge;
use crate::backend::monitor::SystemMonitor;

pub struct DiagnosticsCollector;

impl DiagnosticsCollector {
    /// Gathers a comprehensive system diagnostics snapshot with automated PII redaction.
    pub fn gather_system_snapshot() -> String {
        let distro = DistroDetector::detect();
        let kernel = HostBridge::run_command("uname", &["-r"])
            .map(|(_, out, _)| out.trim().to_string())
            .unwrap_or_else(|_| "Unknown Kernel".to_string());

        let desktop = env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| env::var("DESKTOP_SESSION"))
            .unwrap_or_else(|_| "Unknown Desktop".to_string());

        let mem = SystemMonitor::get_memory_info();
        let disks = SystemMonitor::get_disks_info();
        let root_disk = disks.iter().find(|d| d.mount_point == "/").cloned();

        let failed_units = HostBridge::run_command("systemctl", &["--failed", "--no-pager"])
            .map(|(_, out, _)| {
                let lines: Vec<&str> = out.lines().take(15).collect();
                lines.join("\n")
            })
            .unwrap_or_else(|_| "None".to_string());

        let journal_errors = HostBridge::run_command("journalctl", &["-p", "3", "-xb", "-n", "20", "--no-pager"])
            .map(|(_, out, _)| {
                let lines: Vec<&str> = out.lines().take(20).collect();
                if lines.is_empty() {
                    "No critical errors found in current boot journal.".to_string()
                } else {
                    lines.join("\n")
                }
            })
            .unwrap_or_else(|_| "Could not access system journal.".to_string());

        let root_summary = if let Some(d) = root_disk {
            format!("{:.1}GB free on root ({:.1}% used of {:.1}GB)", d.available_gb, d.percent, d.total_gb)
        } else {
            "Disk details unavailable".to_string()
        };

        let raw_report = format!(
            "You are a GNU/Linux system diagnostics assistant. Analyze this system state:\n\
            - OS: {}\n\
            - Kernel: {}\n\
            - Desktop: {}\n\
            - Memory: {:.0}MB used of {:.0}MB ({:.1}%)\n\
            - Disk: {}\n\
            - Failed systemd units:\n{}\n\n\
            - Recent system errors:\n{}\n\n\
            Provide a brief diagnosis of any anomalies and propose a concrete shell fix command formatted strictly as: PROPOSED_COMMAND: <safe shell command>",
            distro.pretty_name,
            kernel,
            desktop,
            mem.used_mb,
            mem.total_mb,
            mem.percent,
            root_summary,
            failed_units,
            journal_errors
        );

        SafetyGuard::redact_pii(&raw_report)
    }
}
