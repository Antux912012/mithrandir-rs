use std::collections::HashSet;
use std::path::Path;
use crate::backend::HostBridge;

pub const PROTECTED_EXTENSIONS: &[&str] = &[
    "sqlite",
    "sqlite-shm",
    "sqlite-wal",
    "db",
    "jsonlz4",
    "baklz4",
    "ini",
];

pub const PROTECTED_FILENAMES: &[&str] = &[
    "prefs.js",
    "user.js",
    "search.json.mozlz4",
    "containers.json",
    "handlers.json",
    "logins.json",
    "cert9.db",
    "key4.db",
    "lock",
    ".parentlock",
];

pub const PROTECTED_DIRS: &[&str] = &[
    "bookmarkbackups",
    "sessionstore-backups",
    "zen-sessions-backup",
    "extensions",
    "chrome",
];

/// Queries running Flatpak application IDs via `flatpak ps`.
pub fn get_running_flatpaks() -> HashSet<String> {
    let mut running = HashSet::new();
    if let Ok((code, stdout, _)) = HostBridge::run_command("flatpak", &["ps", "--columns=application"]) {
        if code == 0 {
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed != "APPLICATION" {
                    running.insert(trimmed.to_string());
                }
            }
        }
    }
    running
}

/// Checks if a file or directory inside `app_cache_root` should be protected from deletion.
pub fn is_protected_path(path: &Path, app_cache_root: &Path) -> bool {
    // 1. Never delete the root cache folder itself
    if path == app_cache_root {
        return true;
    }

    // 2. Strict boundary check: path must stay within app_cache_root
    // Check symlink targets if path is a symlink
    if path.is_symlink() {
        if let Ok(target) = std::fs::read_link(path) {
            let resolved = if target.is_relative() {
                path.parent().unwrap_or(app_cache_root).join(&target)
            } else {
                target
            };

            let norm_resolved = match resolved.canonicalize() {
                Ok(c) => c,
                Err(_) => resolved,
            };
            let norm_root = match app_cache_root.canonicalize() {
                Ok(c) => c,
                Err(_) => app_cache_root.to_path_buf(),
            };

            if !norm_resolved.starts_with(&norm_root) {
                // Symlink target points outside ~/.var/app/<app>/cache -> preserve!
                return true;
            }
        } else {
            return true;
        }
    }

    // 3. Filename match
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if PROTECTED_FILENAMES.contains(&file_name) {
            return true;
        }
    }

    // 4. Extension match
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        if PROTECTED_EXTENSIONS.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // 5. Directory component match
    if let Ok(rel) = path.strip_prefix(app_cache_root) {
        for component in rel.components() {
            let name = component.as_os_str().to_string_lossy();
            if PROTECTED_DIRS.contains(&name.as_ref()) {
                return true;
            }
        }
    }

    false
}
