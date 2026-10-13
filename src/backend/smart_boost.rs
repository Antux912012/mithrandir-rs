use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use crate::backend::cleaner::CleanerEngine;
use crate::backend::monitor::SystemMonitor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoostStatus {
    pub ram_percent: f64,
    pub ram_used_mb: f64,
    pub ram_total_mb: f64,
    pub temp_bytes: u64,
    pub temp_size_mb: f64,
    pub temp_count: usize,
    pub temp_size_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoostResult {
    pub success: bool,
    pub ram_reclaimed_mb: f64,
    pub disk_freed_mb: f64,
    pub current_ram_percent: f64,
}

pub struct SmartBoostEngine;

impl SmartBoostEngine {
    pub fn get_temporary_files_size() -> (u64, usize) {
        let home = std::env::var("HOME").unwrap_or_default();
        let thumb_path = Path::new(&home).join(".cache/thumbnails");
        let (thumb_bytes, thumb_count) = CleanerEngine::scan_path(&thumb_path);
        let (tmp_bytes, tmp_count) = CleanerEngine::scan_user_tmp();

        (thumb_bytes + tmp_bytes, thumb_count + tmp_count)
    }

    pub fn get_boost_status() -> BoostStatus {
        let mem = SystemMonitor::get_memory_info();
        let (temp_bytes, temp_count) = Self::get_temporary_files_size();
        let temp_size_mb = (temp_bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0;

        let temp_size_str = if temp_size_mb >= 1024.0 {
            format!("{:.2} GB", temp_size_mb / 1024.0)
        } else if temp_size_mb > 0.0 {
            format!("{:.1} MB", temp_size_mb)
        } else {
            "0 MB".to_string()
        };

        BoostStatus {
            ram_percent: mem.percent,
            ram_used_mb: mem.used_mb,
            ram_total_mb: mem.total_mb,
            temp_bytes,
            temp_size_mb,
            temp_count,
            temp_size_str,
        }
    }

    pub fn execute_boost(clean_temp: bool, drop_caches: bool) -> BoostResult {
        let mem_before = SystemMonitor::get_memory_info();
        let (temp_before, _) = Self::get_temporary_files_size();

        // 1. Clean temporary files & thumbnails
        if clean_temp {
            let home = std::env::var("HOME").unwrap_or_default();
            let thumb_path = Path::new(&home).join(".cache/thumbnails");
            CleanerEngine::clean_path(&thumb_path);

            // Clean user files in /tmp
            let current_uid = unsafe { libc::getuid() };
            if let Ok(entries) = fs::read_dir("/tmp") {
                for entry in entries.filter_map(|e| e.ok()) {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if let Ok(meta) = entry.metadata() {
                            if meta.is_file() && meta.uid() == current_uid {
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }

        // 2. Drop inactive pagecache & slab caches
        if drop_caches {
            let _ = Command::new("sync").status();
            // Attempt to drop cache (silently ignored if root permissions not held)
            let _ = Command::new("sh")
                .args(["-c", "echo 3 > /proc/sys/vm/drop_caches 2>/dev/null || true"])
                .status();
            let _ = Command::new("sh")
                .args(["-c", "echo 1 > /proc/sys/vm/compact_memory 2>/dev/null || true"])
                .status();
        }

        std::thread::sleep(std::time::Duration::from_millis(300));
        let mem_after = SystemMonitor::get_memory_info();
        let (temp_after, _) = Self::get_temporary_files_size();

        let ram_reclaimed = (mem_before.used_mb - mem_after.used_mb).max(0.0);
        let disk_freed_mb = ((temp_before.saturating_sub(temp_after)) as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0;

        BoostResult {
            success: true,
            ram_reclaimed_mb: (ram_reclaimed * 10.0).round() / 10.0,
            disk_freed_mb,
            current_ram_percent: mem_after.percent,
        }
    }
}
