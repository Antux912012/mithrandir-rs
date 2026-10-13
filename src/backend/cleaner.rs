use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub size_mb: f64,
}

pub struct CleanerEngine;

impl CleanerEngine {
    /// Strictly protects critical system directories from accidental deletion.
    pub fn is_path_safe(target: &Path) -> bool {
        let canonical = match target.canonicalize() {
            Ok(c) => c,
            Err(_) => target.to_path_buf(),
        };

        let forbidden: &[&str] = &[
            "/",
            "/bin",
            "/boot",
            "/dev",
            "/etc",
            "/home",
            "/lib",
            "/lib64",
            "/opt",
            "/proc",
            "/root",
            "/run",
            "/sbin",
            "/sys",
            "/usr",
            "/var",
        ];

        for f in forbidden {
            if canonical == Path::new(f) {
                return false;
            }
        }

        // Also forbid deleting the user's home folder root
        if let Ok(home) = std::env::var("HOME") {
            if canonical == Path::new(&home) {
                return false;
            }
        }

        true
    }

    /// Recursively scans a directory in parallel using Rayon.
    pub fn scan_path(path: &Path) -> (u64, usize) {
        if !path.exists() {
            return (0, 0);
        }

        WalkDir::new(path)
            .into_iter()
            .par_bridge()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
            .fold(|| (0u64, 0usize), |(s, c), len| (s + len, c + 1))
            .reduce(|| (0, 0), |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2))
    }

    /// Returns standard cleanup categories available on GNU/Linux.
    pub fn get_categories() -> Vec<CleanCategory> {
        let mut list = Vec::new();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

        // 1. Thumbnails cache
        let thumb_path = PathBuf::from(&home).join(".cache/thumbnails");
        let (thumb_size, thumb_count) = Self::scan_path(&thumb_path);
        list.push(CleanCategory {
            id: "thumbnails".to_string(),
            name: "Thumbnail Cache".to_string(),
            description: "Cached previews of images and videos".to_string(),
            path: thumb_path,
            size_bytes: thumb_size,
            file_count: thumb_count,
            size_mb: ((thumb_size as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
        });

        // 2. /tmp user files
        let tmp_path = PathBuf::from("/tmp");
        let (tmp_size, tmp_count) = Self::scan_user_tmp();
        list.push(CleanCategory {
            id: "tmp".to_string(),
            name: "Temporary Files".to_string(),
            description: "Temporary system and user scratch files in /tmp".to_string(),
            path: tmp_path,
            size_bytes: tmp_size,
            file_count: tmp_count,
            size_mb: ((tmp_size as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
        });

        list
    }

    /// Scans only user-owned files in /tmp to prevent attempting to touch other users' files.
    pub fn scan_user_tmp() -> (u64, usize) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let current_uid = unsafe { libc::getuid() };
            let tmp = Path::new("/tmp");
            if !tmp.exists() {
                return (0, 0);
            }

            WalkDir::new(tmp)
                .min_depth(1)
                .into_iter()
                .par_bridge()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    if let Ok(meta) = e.path().symlink_metadata() {
                        meta.is_file() && meta.uid() == current_uid
                    } else {
                        false
                    }
                })
                .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .fold(|| (0u64, 0usize), |(s, c), len| (s + len, c + 1))
                .reduce(|| (0, 0), |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2))
        }
        #[cfg(not(unix))]
        {
            (0, 0)
        }
    }

    /// Cleans the specified path safely, deleting files in parallel.
    pub fn clean_path(path: &Path) -> (u64, usize) {
        if !Self::is_path_safe(path) || !path.exists() {
            return (0, 0);
        }

        let files: Vec<PathBuf> = WalkDir::new(path)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        files
            .into_par_iter()
            .map(|f| {
                let size = f.metadata().map(|m| m.len()).unwrap_or(0);
                if fs::remove_file(&f).is_ok() {
                    (size, 1usize)
                } else {
                    (0, 0)
                }
            })
            .reduce(|| (0, 0), |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2))
    }
}
