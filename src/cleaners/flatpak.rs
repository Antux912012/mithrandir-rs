use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::cleaners::flatpak_guard::{get_running_flatpaks, is_protected_path};

/// Scans Flatpak application caches inside `base_var_app_dir` (typically `~/.var/app`).
/// Skips running applications and protected profile/configuration files.
pub fn scan_flatpak_cache(base_var_app_dir: &Path) -> (u64, usize) {
    scan_flatpak_cache_with_running(base_var_app_dir, &get_running_flatpaks())
}

/// Scans Flatpak application caches with an explicit set of running application IDs.
pub fn scan_flatpak_cache_with_running(base_var_app_dir: &Path, running_apps: &HashSet<String>) -> (u64, usize) {
    if !base_var_app_dir.exists() || !base_var_app_dir.is_dir() {
        return (0, 0);
    }

    let mut total_bytes = 0u64;
    let mut total_files = 0usize;

    if let Ok(entries) = fs::read_dir(base_var_app_dir) {
        for entry in entries.flatten() {
            let app_id = entry.file_name().to_string_lossy().to_string();
            if running_apps.contains(&app_id) {
                continue;
            }

            let cache_dir = entry.path().join("cache");
            if !cache_dir.exists() || cache_dir.is_symlink() || !cache_dir.is_dir() {
                continue;
            }

            for e in WalkDir::new(&cache_dir).follow_links(false).into_iter().flatten() {
                let p = e.path();
                if p == cache_dir {
                    continue;
                }
                if is_protected_path(p, &cache_dir) {
                    continue;
                }
                if e.file_type().is_file() || e.file_type().is_symlink() {
                    let len = e.metadata().map(|m| m.len()).unwrap_or(0);
                    total_bytes += len;
                    total_files += 1;
                }
            }
        }
    }

    (total_bytes, total_files)
}

/// Safely cleans Flatpak application caches inside `base_var_app_dir` (typically `~/.var/app`).
/// Strict guardrails:
/// 1. Only touches `~/.var/app/<app_id>/cache/`
/// 2. Skips running applications (logged as warnings)
/// 3. Preserves symlinks pointing outside cache root
/// 4. Protects database, configuration, and session files
pub fn clean_flatpak_cache(base_var_app_dir: &Path) -> (u64, usize) {
    clean_flatpak_cache_with_running(base_var_app_dir, &get_running_flatpaks())
}

/// Cleans Flatpak application caches with an explicit set of running application IDs.
pub fn clean_flatpak_cache_with_running(base_var_app_dir: &Path, running_apps: &HashSet<String>) -> (u64, usize) {
    if !base_var_app_dir.exists() || !base_var_app_dir.is_dir() {
        return (0, 0);
    }

    let mut freed_bytes = 0u64;
    let mut freed_files = 0usize;

    if let Ok(entries) = fs::read_dir(base_var_app_dir) {
        for entry in entries.flatten() {
            let app_id = entry.file_name().to_string_lossy().to_string();
            if running_apps.contains(&app_id) {
                eprintln!("Skipping running Flatpak app: {}", app_id);
                continue;
            }

            let cache_dir = entry.path().join("cache");
            if !cache_dir.exists() || cache_dir.is_symlink() || !cache_dir.is_dir() {
                continue;
            }

            // Walk bottom-up (contents_first=true) so files are deleted before parent directories
            for e in WalkDir::new(&cache_dir)
                .follow_links(false)
                .contents_first(true)
                .into_iter()
                .flatten()
            {
                let p = e.path();
                if p == cache_dir {
                    continue;
                }

                if is_protected_path(p, &cache_dir) {
                    continue;
                }

                if e.file_type().is_file() || e.file_type().is_symlink() {
                    let len = e.metadata().map(|m| m.len()).unwrap_or(0);
                    if fs::remove_file(p).is_ok() {
                        freed_bytes += len;
                        freed_files += 1;
                    }
                } else if e.file_type().is_dir() {
                    // Only prune directory if it became completely empty
                    let _ = fs::remove_dir(p);
                }
            }
        }
    }

    (freed_bytes, freed_files)
}

/// Helper that resolves default Flatpak cache directory: `~/.var/app`.
pub fn get_default_var_app_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".var/app")
}

pub fn scan_default_flatpak_cache() -> (u64, usize) {
    scan_flatpak_cache(&get_default_var_app_dir())
}

pub fn clean_default_flatpak_cache() -> (u64, usize) {
    clean_flatpak_cache(&get_default_var_app_dir())
}
