use serde::{Deserialize, Serialize};
use sysinfo::{ProcessesToUpdate, System};
use crate::backend::host_bridge::HostBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub mem_mb: f64,
    pub mem_percent: f32,
    pub user: String,
    pub cmd: String,
}

pub struct ProcessManager;

impl ProcessManager {
    /// Lists active system processes with accurate instantaneous CPU% and RSS MB.
    pub fn list_processes() -> Vec<ProcessItem> {
        // Attempt fast and accurate host ps inspection
        if let Ok((code, stdout, _)) = HostBridge::run_command("ps", &["-eo", "pid,user,%cpu,%mem,rss,comm", "--sort=-%mem"]) {
            if code == 0 && !stdout.trim().is_empty() {
                let mut list = Vec::new();
                for (idx, line) in stdout.lines().enumerate() {
                    if idx == 0 {
                        continue; // Skip header
                    }
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        if let Ok(pid) = parts[0].parse::<u32>() {
                            let user = parts[1].to_string();
                            let cpu = parts[2].parse::<f32>().unwrap_or(0.0);
                            let mem_pct = parts[3].parse::<f32>().unwrap_or(0.0);
                            let rss_kb = parts[4].parse::<f64>().unwrap_or(0.0);
                            let mem_mb = (rss_kb / 1024.0 * 10.0).round() / 10.0;
                            let name = parts[5..].join(" ");

                            list.push(ProcessItem {
                                pid,
                                name: name.clone(),
                                cpu_percent: cpu,
                                mem_mb,
                                mem_percent: mem_pct,
                                user,
                                cmd: name,
                            });
                        }
                    }
                }
                if !list.is_empty() {
                    return list;
                }
            }
        }

        // Sysinfo fallback
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        sys.refresh_memory();

        let total_mem = sys.total_memory() as f64;
        let mut list = Vec::new();

        for (pid, proc) in sys.processes() {
            let mem_bytes = proc.memory();
            let mem_mb = (mem_bytes as f64) / (1024.0 * 1024.0);
            let mem_percent = if total_mem > 0.0 {
                ((mem_bytes as f64) / total_mem * 100.0) as f32
            } else {
                0.0
            };

            let cmd = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" ");

            list.push(ProcessItem {
                pid: pid.as_u32(),
                name: proc.name().to_string_lossy().to_string(),
                cpu_percent: proc.cpu_usage(),
                mem_mb: (mem_mb * 10.0).round() / 10.0,
                mem_percent,
                user: proc
                    .user_id()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "user".to_string()),
                cmd,
            });
        }

        list.sort_by(|a, b| b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(std::cmp::Ordering::Equal));
        list
    }

    pub fn kill_process(pid: u32, force: bool) -> Result<(), String> {
        let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
        let ret = unsafe { libc::kill(pid as i32, sig) };
        if ret == 0 {
            Ok(())
        } else {
            // Attempt privileged kill via HostBridge
            let flag = if force { "-9" } else { "-15" };
            let pid_str = pid.to_string();
            let (code, _, err) = HostBridge::run_privileged(&["kill", flag, &pid_str])?;
            if code == 0 {
                Ok(())
            } else {
                Err(format!("Could not terminate process {}: {}", pid, err))
            }
        }
    }
}
