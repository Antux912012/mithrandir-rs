use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

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

pub struct SystemMonitor;

impl SystemMonitor {
    pub fn get_memory_info() -> MemoryInfo {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_memory();

        let total_bytes = sys.total_memory();
        let used_bytes = sys.used_memory();
        let available_bytes = sys.available_memory();
        let free_bytes = sys.free_memory();

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
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );
        sys.refresh_cpu_all();

        // Brief delay or second refresh for accurate delta if needed
        std::thread::sleep(std::time::Duration::from_millis(100));
        sys.refresh_cpu_all();

        let usage_percent = sys.global_cpu_usage();
        let cpus = sys.cpus();
        let core_count = cpus.len();
        let brand = cpus
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        CpuInfo {
            usage_percent: (usage_percent * 10.0).round() / 10.0,
            brand,
            core_count,
        }
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
}
