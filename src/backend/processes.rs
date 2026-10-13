use serde::{Deserialize, Serialize};
use sysinfo::{ProcessesToUpdate, System};

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
    pub fn list_processes() -> Vec<ProcessItem> {
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
                mem_mb,
                mem_percent,
                user: proc
                    .user_id()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "user".to_string()),
                cmd,
            });
        }

        // Default sort by memory usage descending
        list.sort_by(|a, b| b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(std::cmp::Ordering::Equal));
        list
    }

    pub fn kill_process(pid: u32, force: bool) -> Result<(), String> {
        let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
        let ret = unsafe { libc::kill(pid as i32, sig) };
        if ret == 0 {
            Ok(())
        } else {
            Err(format!("Failed to kill PID {}: errno {}", pid, unsafe { *libc::__errno_location() }))
        }
    }
}
