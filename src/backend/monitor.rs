use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use crate::backend::distro::DistroDetector;
use crate::backend::host_bridge::HostBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_mb: f64,
    pub used_mb: f64,
    pub free_mb: f64,
    pub available_mb: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub brand: String,
    pub core_count: usize,
    pub temp_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskPartition {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub available_gb: f64,
    pub used_gb: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub load_percent: f32,
    pub temp_c: Option<f32>,
    pub mem_used_mb: Option<f64>,
    pub mem_total_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRate {
    pub rx_kbps: f64,
    pub tx_kbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSummary {
    pub hostname: String,
    pub distro: String,
    pub kernel: String,
    pub desktop: String,
    pub uptime_str: String,
}

static SYSTEM_MUTEX: OnceLock<Mutex<System>> = OnceLock::new();
static NET_PREV: OnceLock<Mutex<Option<(Instant, u64, u64)>>> = OnceLock::new();

fn get_system() -> &'static Mutex<System> {
    SYSTEM_MUTEX.get_or_init(|| {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Mutex::new(sys)
    })
}

pub struct SystemMonitor;

impl SystemMonitor {
    pub fn get_memory_info() -> MemoryInfo {
        let mut lock = get_system().lock().unwrap();
        lock.refresh_memory();

        let total_bytes = lock.total_memory();
        let used_bytes = lock.used_memory();
        let available_bytes = lock.available_memory();
        let free_bytes = lock.free_memory();

        let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
        let used_mb = used_bytes as f64 / (1024.0 * 1024.0);
        let available_mb = available_bytes as f64 / (1024.0 * 1024.0);
        let free_mb = free_bytes as f64 / (1024.0 * 1024.0);

        let percent = if total_mb > 0.0 {
            (used_mb / total_mb) * 100.0
        } else {
            0.0
        };

        MemoryInfo {
            total_mb: (total_mb * 10.0).round() / 10.0,
            used_mb: (used_mb * 10.0).round() / 10.0,
            free_mb: (free_mb * 10.0).round() / 10.0,
            available_mb: (available_mb * 10.0).round() / 10.0,
            percent: (percent * 10.0).round() / 10.0,
        }
    }

    pub fn get_cpu_info() -> CpuInfo {
        let mut lock = get_system().lock().unwrap();
        lock.refresh_cpu_usage();

        let usage_percent = lock.global_cpu_usage();
        let cpus = lock.cpus();
        let core_count = cpus.len();
        let brand = cpus
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Processor".to_string());

        let temp_c = Self::get_cpu_temperature();

        CpuInfo {
            usage_percent: (usage_percent * 10.0).round() / 10.0,
            brand,
            core_count,
            temp_c,
        }
    }

    pub fn get_cpu_temperature() -> Option<f32> {
        // Read from /sys/class/thermal or /sys/class/hwmon
        let thermal_dirs = [
            "/sys/class/thermal/thermal_zone0/temp",
            "/sys/class/thermal/thermal_zone1/temp",
            "/sys/class/hwmon/hwmon0/temp1_input",
            "/sys/class/hwmon/hwmon1/temp1_input",
        ];

        for path in &thermal_dirs {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(val) = content.trim().parse::<f32>() {
                    let temp = if val > 1000.0 { val / 1000.0 } else { val };
                    if temp > 0.0 && temp < 125.0 {
                        return Some((temp * 10.0).round() / 10.0);
                    }
                }
            }
        }
        None
    }

    pub fn get_disks_info() -> Vec<DiskPartition> {
        let disks = Disks::new_with_refreshed_list();
        disks
            .iter()
            .map(|d| {
                let total = d.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let available = d.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let used = (total - available).max(0.0);
                let percent = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };

                DiskPartition {
                    name: d.name().to_string_lossy().into_owned(),
                    mount_point: d.mount_point().to_string_lossy().into_owned(),
                    total_gb: (total * 10.0).round() / 10.0,
                    available_gb: (available * 10.0).round() / 10.0,
                    used_gb: (used * 10.0).round() / 10.0,
                    percent: (percent * 10.0).round() / 10.0,
                }
            })
            .collect()
    }

    pub fn get_gpu_info() -> Option<GpuInfo> {
        // 1. Check NVIDIA via nvidia-smi
        if let Ok((code, stdout, _)) = HostBridge::run_command(
            "nvidia-smi",
            &["--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"],
        ) {
            if code == 0 {
                let line = stdout.lines().next()?;
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 5 {
                    let name = parts[0].to_string();
                    let load = parts[1].parse::<f32>().unwrap_or(0.0);
                    let temp = parts[2].parse::<f32>().ok();
                    let mem_used = parts[3].parse::<f64>().ok();
                    let mem_total = parts[4].parse::<f64>().ok();

                    return Some(GpuInfo {
                        name,
                        load_percent: load,
                        temp_c: temp,
                        mem_used_mb: mem_used,
                        mem_total_mb: mem_total,
                    });
                }
            }
        }

        // 2. AMD GPU fallback
        let amd_busy = "/sys/class/drm/card0/device/gpu_busy_percent";
        if Path::new(amd_busy).exists() {
            if let Ok(content) = fs::read_to_string(amd_busy) {
                if let Ok(load) = content.trim().parse::<f32>() {
                    return Some(GpuInfo {
                        name: "AMD Radeon Graphics".to_string(),
                        load_percent: load,
                        temp_c: None,
                        mem_used_mb: None,
                        mem_total_mb: None,
                    });
                }
            }
        }

        None
    }

    pub fn get_network_rate() -> NetworkRate {
        let lock = NET_PREV.get_or_init(|| Mutex::new(None));
        let mut prev = lock.lock().unwrap();
        let now = Instant::now();

        let mut cur_rx = 0u64;
        let mut cur_tx = 0u64;

        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let iface = parts[0].trim_end_matches(':');
                    if iface != "lo" {
                        let rx = parts[1].parse::<u64>().unwrap_or(0);
                        let tx = parts[9].parse::<u64>().unwrap_or(0);
                        cur_rx += rx;
                        cur_tx += tx;
                    }
                }
            }
        }

        let mut rate = NetworkRate { rx_kbps: 0.0, tx_kbps: 0.0 };

        if let Some((last_time, last_rx, last_tx)) = *prev {
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.0 {
                rate.rx_kbps = ((cur_rx.saturating_sub(last_rx)) as f64 / 1024.0 / elapsed * 10.0).round() / 10.0;
                rate.tx_kbps = ((cur_tx.saturating_sub(last_tx)) as f64 / 1024.0 / elapsed * 10.0).round() / 10.0;
            }
        }

        *prev = Some((now, cur_rx, cur_tx));
        rate
    }

    pub fn get_system_summary() -> SystemSummary {
        let distro = DistroDetector::detect();
        let hostname = HostBridge::run_command("hostname", &[])
            .map(|(_, out, _)| out.trim().to_string())
            .unwrap_or_else(|_| "localhost".to_string());

        let kernel = HostBridge::run_command("uname", &["-r"])
            .map(|(_, out, _)| out.trim().to_string())
            .unwrap_or_else(|_| "Linux".to_string());

        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_else(|_| "Desktop".to_string());

        let uptime_str = if let Ok(content) = fs::read_to_string("/proc/uptime") {
            if let Some(secs_str) = content.split_whitespace().next() {
                if let Ok(secs) = secs_str.parse::<f64>() {
                    let hours = (secs / 3600.0).floor() as u64;
                    let mins = ((secs % 3600.0) / 60.0).floor() as u64;
                    format!("{}h {}m", hours, mins)
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        SystemSummary {
            hostname,
            distro: distro.pretty_name,
            kernel,
            desktop,
            uptime_str,
        }
    }
}
