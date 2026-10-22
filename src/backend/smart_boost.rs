use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::backend::cleaner::CleanerEngine;
use crate::backend::host_bridge::HostBridge;
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
    pub already_running: bool,
    pub ram_reclaimed_mb: f64,
    pub disk_freed_mb: f64,
    pub current_ram_percent: f64,
}

static IS_BOOSTING: AtomicBool = AtomicBool::new(false);
static LAST_AUTO_BOOST: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static LAST_TRIGGER: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

fn last_auto_boost_mutex() -> &'static Mutex<Option<Instant>> {
    LAST_AUTO_BOOST.get_or_init(|| Mutex::new(None))
}

fn last_trigger_mutex() -> &'static Mutex<Option<Instant>> {
    LAST_TRIGGER.get_or_init(|| Mutex::new(None))
}

pub struct SmartBoostEngine;

impl SmartBoostEngine {
    /// Enforces a debounce window on manual or tray boost actions. Returns true if allowed.
    pub fn check_debounce(min_interval_ms: u64) -> bool {
        let mut lock = last_trigger_mutex().lock().unwrap();
        let now = Instant::now();
        if let Some(last) = *lock {
            if now.duration_since(last) < Duration::from_millis(min_interval_ms) {
                return false;
            }
        }
        *lock = Some(now);
        true
    }

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
        // Prevent re-entrancy / notification cascades
        if IS_BOOSTING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            let mem = SystemMonitor::get_memory_info();
            return BoostResult {
                success: false,
                already_running: true,
                ram_reclaimed_mb: 0.0,
                disk_freed_mb: 0.0,
                current_ram_percent: mem.percent,
            };
        }

        let mem_before = SystemMonitor::get_memory_info();
        let (temp_before, _) = Self::get_temporary_files_size();

        // 1. Clean temporary files & thumbnails
        if clean_temp {
            let home = std::env::var("HOME").unwrap_or_default();
            let thumb_path = Path::new(&home).join(".cache/thumbnails");
            CleanerEngine::clean_path(&thumb_path);
            CleanerEngine::clean_user_tmp();
        }

        // 2. Drop inactive pagecache & slab caches
        if drop_caches {
            let _ = HostBridge::run_command("sync", &[]);
            let _ = HostBridge::run_privileged(&["sh", "-c", "echo 3 > /proc/sys/vm/drop_caches 2>/dev/null || true"]);
            let _ = HostBridge::run_privileged(&["sh", "-c", "echo 1 > /proc/sys/vm/compact_memory 2>/dev/null || true"]);
        }

        std::thread::sleep(Duration::from_millis(300));
        let mem_after = SystemMonitor::get_memory_info();
        let (temp_after, _) = Self::get_temporary_files_size();

        let ram_reclaimed = (mem_before.used_mb - mem_after.used_mb).max(0.0);
        let disk_freed_mb = ((temp_before.saturating_sub(temp_after)) as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0;

        IS_BOOSTING.store(false, Ordering::SeqCst);

        BoostResult {
            success: true,
            already_running: false,
            ram_reclaimed_mb: (ram_reclaimed * 10.0).round() / 10.0,
            disk_freed_mb,
            current_ram_percent: mem_after.percent,
        }
    }

    /// Evaluates whether an automated background boost is needed and enforces cooldown.
    pub fn evaluate_auto_boost(
        ram_threshold: f64,
        temp_threshold_mb: f64,
        cooldown_minutes: u64,
    ) -> Option<BoostResult> {
        let now = Instant::now();
        let cooldown = Duration::from_secs(cooldown_minutes * 60);

        {
            let lock = last_auto_boost_mutex().lock().unwrap();
            if let Some(last) = *lock {
                if now.duration_since(last) < cooldown {
                    return None;
                }
            }
        }

        let status = Self::get_boost_status();
        let ram_exceeded = status.ram_percent >= ram_threshold;
        let temp_exceeded = status.temp_size_mb >= temp_threshold_mb;

        if ram_exceeded || temp_exceeded {
            *last_auto_boost_mutex().lock().unwrap() = Some(now);
            let res = Self::execute_boost(temp_exceeded, ram_exceeded);
            if !res.already_running {
                return Some(res);
            }
        }

        None
    }
}
