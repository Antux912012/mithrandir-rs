use mithrandir_rs::{CleanerEngine, SmartBoostEngine, SystemMonitor};
use std::path::Path;

#[test]
fn test_cleaner_path_safety() {
    assert!(!CleanerEngine::is_path_safe(Path::new("/")));
    assert!(!CleanerEngine::is_path_safe(Path::new("/etc")));
    assert!(!CleanerEngine::is_path_safe(Path::new("/usr")));
    assert!(!CleanerEngine::is_path_safe(Path::new("/var")));
    assert!(CleanerEngine::is_path_safe(Path::new("/tmp/mithrandir_test")));
}

#[test]
fn test_memory_monitor() {
    let mem = SystemMonitor::get_memory_info();
    assert!(mem.total_mb > 0.0);
    assert!(mem.percent >= 0.0 && mem.percent <= 100.0);
}

#[test]
fn test_cpu_monitor() {
    let cpu = SystemMonitor::get_cpu_info();
    assert!(cpu.core_count > 0);
    assert!(!cpu.brand.is_empty());
}

#[test]
fn test_smart_boost_status() {
    let status = SmartBoostEngine::get_boost_status();
    assert!(status.ram_total_mb > 0.0);
    assert!(!status.temp_size_str.is_empty());
}
