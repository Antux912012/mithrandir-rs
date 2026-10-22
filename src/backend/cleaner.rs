use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::backend::host_bridge::HostBridge;

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
        Self::scan_path_excluding(path, &[])
    }

    pub fn scan_path_excluding(path: &Path, exclude_subdirs: &[&str]) -> (u64, usize) {
        if !path.exists() {
            return (0, 0);
        }

        WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 1 && e.file_type().is_dir() {
                    let fname = e.file_name().to_string_lossy();
                    !exclude_subdirs.contains(&fname.as_ref())
                } else {
                    true
                }
            })
            .par_bridge()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
            .fold(|| (0u64, 0usize), |(s, c), len| (s + len, c + 1))
            .reduce(|| (0, 0), |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2))
    }

    pub fn get_categories() -> Vec<CleanCategory> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let home_p = PathBuf::from(&home);

        let pkg_cache_path = if Path::new("/var/cache/libdnf5").exists() {
            PathBuf::from("/var/cache/libdnf5")
        } else if Path::new("/var/cache/dnf").exists() {
            PathBuf::from("/var/cache/dnf")
        } else if Path::new("/var/cache/apt/archives").exists() {
            PathBuf::from("/var/cache/apt/archives")
        } else if Path::new("/var/cache/pacman/pkg").exists() {
            PathBuf::from("/var/cache/pacman/pkg")
        } else {
            PathBuf::from("/var/cache")
        };

        let cats = [
            ("thumbnails", "Thumbnail Cache", "Cached image and video thumbnails", home_p.join(".cache/thumbnails")),
            ("user_cache", "User Application Cache", "Application cache files in ~/.cache (excluding browsers, dev, thumbnails)", home_p.join(".cache")),
            ("browser_cache", "Web Browser Caches", "Firefox, Chrome, Chromium, Brave, and Edge caches", home_p.join(".cache/mozilla")),
            ("stremio_cache", "Stremio Streaming Cache", "Torrent parts and media stream cache", home_p.join(".config/stremio/stremio-cache")),
            ("dev_cache", "Developer & Package Caches", "Pip, Cargo, and npm package cache directories", home_p.join(".cache/pip")),
            ("flatpak_cache", "Flatpak Applications Cache", "Safe application caches in ~/.var/app/*/cache (preserves configs, profiles & running apps)", home_p.join(".var/app")),
            ("snap_cache", "Snap Package Cache", "Snap cache data in ~/snap", home_p.join("snap")),
            ("trash", "Trash Bin", "Deleted files in user Trash", home_p.join(".local/share/Trash")),
            ("tmp", "Temporary Files", "Volatile user files in /tmp", PathBuf::from("/tmp")),
            ("package_cache", "Package Manager Cache", "Cached APT, DNF, or Pacman package archives", pkg_cache_path),
        ];

        let home_ref = home_p.clone();
        cats.into_par_iter()
            .map(|(id, name, desc, path)| {
                let (size_bytes, file_count) = if id == "tmp" {
                    Self::scan_user_tmp()
                } else if id == "user_cache" {
                    let exclusions = ["thumbnails", "mozilla", "google-chrome", "chromium", "BraveSoftware", "microsoft-edge", "pip"];
                    Self::scan_path_excluding(&path, &exclusions)
                } else if id == "browser_cache" {
                    let (s1, c1) = Self::scan_path(&home_ref.join(".cache/mozilla"));
                    let (s2, c2) = Self::scan_path(&home_ref.join(".cache/google-chrome"));
                    let (s3, c3) = Self::scan_path(&home_ref.join(".cache/chromium"));
                    (s1 + s2 + s3, c1 + c2 + c3)
                } else if id == "dev_cache" {
                    let (s1, c1) = Self::scan_path(&home_ref.join(".cache/pip"));
                    let (s2, c2) = Self::scan_path(&home_ref.join(".npm/_cacache"));
                    let (s3, c3) = Self::scan_path(&home_ref.join(".cargo/registry/cache"));
                    (s1 + s2 + s3, c1 + c2 + c3)
                } else if id == "flatpak_cache" {
                    crate::cleaners::scan_flatpak_cache(&path)
                } else if id == "package_cache" {
                    let mut total_s = 0;
                    let mut total_c = 0;
                    for p in &["/var/cache/libdnf5", "/var/cache/dnf", "/var/cache/apt/archives", "/var/cache/pacman/pkg"] {
                        let pb = Path::new(p);
                        if pb.exists() {
                            let (s, c) = Self::scan_path(pb);
                            total_s += s;
                            total_c += c;
                        }
                    }
                    (total_s, total_c)
                } else {
                    Self::scan_path(&path)
                };

                CleanCategory {
                    id: id.to_string(),
                    name: name.to_string(),
                    description: desc.to_string(),
                    path,
                    size_bytes,
                    file_count,
                    size_mb: ((size_bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0,
                }
            })
            .collect()
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
        Self::clean_path_excluding(path, &[])
    }

    pub fn clean_path_excluding(path: &Path, exclude_subdirs: &[&str]) -> (u64, usize) {
        if !Self::is_path_safe(path) || !path.exists() {
            return (0, 0);
        }

        let files: Vec<PathBuf> = WalkDir::new(path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 1 && e.file_type().is_dir() {
                    let fname = e.file_name().to_string_lossy();
                    !exclude_subdirs.contains(&fname.as_ref())
                } else {
                    true
                }
            })
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

    pub fn clean_user_tmp() -> (u64, usize) {
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

    pub fn clean_category(cat_id: &str) -> CleanResult {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let home_p = PathBuf::from(&home);

        let (bytes, count) = match cat_id {
            "thumbnails" => Self::clean_path(&home_p.join(".cache/thumbnails")),
            "user_cache" => {
                let exclusions = ["thumbnails", "mozilla", "google-chrome", "chromium", "BraveSoftware", "microsoft-edge", "pip"];
                Self::clean_path_excluding(&home_p.join(".cache"), &exclusions)
            }
            "browser_cache" => {
                let (b1, c1) = Self::clean_path(&home_p.join(".cache/mozilla"));
                let (b2, c2) = Self::clean_path(&home_p.join(".cache/google-chrome"));
                let (b3, c3) = Self::clean_path(&home_p.join(".cache/chromium"));
                (b1 + b2 + b3, c1 + c2 + c3)
            }
            "stremio_cache" => {
                let (b1, c1) = Self::clean_path(&home_p.join(".config/stremio/stremio-cache"));
                let (b2, c2) = Self::clean_path(&PathBuf::from("/tmp/stremio-cache"));
                (b1 + b2, c1 + c2)
            }
            "dev_cache" => {
                let (b1, c1) = Self::clean_path(&home_p.join(".cache/pip"));
                let (b2, c2) = Self::clean_path(&home_p.join(".npm/_cacache"));
                let (b3, c3) = Self::clean_path(&home_p.join(".cargo/registry/cache"));
                (b1 + b2 + b3, c1 + c2 + c3)
            }
            "flatpak_cache" => crate::cleaners::clean_flatpak_cache(&home_p.join(".var/app")),
            "snap_cache" => Self::clean_path(&home_p.join("snap")),
            "trash" => Self::clean_path(&home_p.join(".local/share/Trash")),
            "tmp" => Self::clean_user_tmp(),
            "package_cache" => {
                let _ = HostBridge::run_privileged(&["sh", "-c", "apt-get clean 2>/dev/null || dnf clean all 2>/dev/null || pacman -Scc --noconfirm 2>/dev/null || true"]);
                let _ = HostBridge::run_privileged(&["journalctl", "--vacuum-time=7d"]);
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
