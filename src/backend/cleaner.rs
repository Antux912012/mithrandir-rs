use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub freed_bytes: u64,
    pub freed_mb: f64,
    pub freed_files: usize,
}

pub struct CleanerEngine;

impl CleanerEngine {
    pub fn is_path_safe(target: &Path) -> bool {
        let canonical = match target.canonicalize() {
            Ok(c) => c,
            Err(_) => target.to_path_buf(),
        };

        let forbidden: &[&str] = &[
            "/", "/bin", "/boot", "/dev", "/etc", "/home", "/lib", "/lib64",
            "/opt", "/proc", "/root", "/run", "/sbin", "/sys", "/usr", "/var",
        ];

        for f in forbidden {
            if canonical == Path::new(f) {
                return false;
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            if canonical == Path::new(&home) {
                return false;
            }
        }

        true
    }

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

    pub fn get_categories() -> Vec<CleanCategory> {
        let mut list = Vec::new();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

        let cats = [
            ("user_cache", "User Application Cache", "Temporary application cache in ~/.cache", PathBuf::from(&home).join(".cache")),
            ("flatpak_cache", "Flatpak Applications Cache", "Application caches in ~/.var/app", PathBuf::from(&home).join(".var/app")),
            ("snap_cache", "Snap Package Cache", "Snap cache data in ~/snap", PathBuf::from(&home).join("snap")),
            ("thumbnails", "Thumbnail Cache", "Cached image and video thumbnails", PathBuf::from(&home).join(".cache/thumbnails")),
            ("trash", "Trash Bin", "Deleted files in user Trash", PathBuf::from(&home).join(".local/share/Trash")),
            ("tmp", "Temporary Files", "Scratch files in /tmp", PathBuf::from("/tmp")),
            ("package_cache", "Package Manager Cache", "Cached APT/DNF archives", PathBuf::from("/var/cache/apt/archives")),
        ];

        for (id, name, desc, path) in cats {
            let (size_bytes, file_count) = if id == "tmp" {
                Self::scan_user_tmp()
            } else {
                Self::scan_path(&path)
            };

            list.push(CleanCategory {
                id: id.to_string(),
                name: name.to_string(),
                description: desc.to_string(),
                path,
                size_bytes,
                file_count,
                size_mb: ((size_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
            });
        }

        list
    }

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

    pub fn clean_category(cat_id: &str) -> CleanResult {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let (bytes, count) = match cat_id {
            "user_cache" => Self::clean_path(&PathBuf::from(&home).join(".cache")),
            "flatpak_cache" => Self::clean_path(&PathBuf::from(&home).join(".var/app")),
            "snap_cache" => Self::clean_path(&PathBuf::from(&home).join("snap")),
            "thumbnails" => Self::clean_path(&PathBuf::from(&home).join(".cache/thumbnails")),
            "trash" => Self::clean_path(&PathBuf::from(&home).join(".local/share/Trash")),
            "tmp" => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let current_uid = unsafe { libc::getuid() };
                    let tmp = Path::new("/tmp");
                    let files: Vec<PathBuf> = WalkDir::new(tmp)
                        .min_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            if let Ok(meta) = e.path().symlink_metadata() {
                                meta.is_file() && meta.uid() == current_uid
                            } else {
                                false
                            }
                        })
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
                #[cfg(not(unix))]
                {
                    (0, 0)
                }
            }
            "package_cache" => {
                let _ = Command::new("pkexec").args(["apt-get", "clean"]).status();
                (0, 0)
            }
            _ => (0, 0),
        };

        CleanResult {
            freed_bytes: bytes,
            freed_mb: ((bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
            freed_files: count,
        }
    }
}
