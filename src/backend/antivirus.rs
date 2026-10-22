use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::backend::distro::DistroDetector;
use crate::backend::host_bridge::HostBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFinding {
    pub file_path: String,
    pub virus_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamStatus {
    pub is_installed: bool,
    pub version: String,
    pub definitions_date: String,
    pub definitions_stale: bool,
    pub definitions_age_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub id: String,
    pub original_path: String,
    pub quarantined_path: String,
    pub virus_name: String,
    pub sha256: String,
    pub quarantined_at: String,
    pub size_bytes: u64,
}

pub struct AntivirusEngine;

impl AntivirusEngine {
    pub fn quarantine_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/io.github.antonio.MithrandirRS/quarantine")
    }

    pub fn quarantine_files_dir() -> PathBuf {
        Self::quarantine_dir().join("files")
    }

    pub fn manifest_file() -> PathBuf {
        Self::quarantine_dir().join("manifest.json")
    }

    pub fn get_status() -> ClamStatus {
        let mut is_installed = false;
        let mut version = "Not Installed".to_string();
        let mut definitions_date = "Unknown".to_string();
        let mut definitions_age_hours = 0u64;
        let mut definitions_stale = true;

        if let Ok((code, stdout, _)) = HostBridge::run_command("clamscan", &["--version"]) {
            if code == 0 && !stdout.trim().is_empty() {
                is_installed = true;
                version = stdout.trim().to_string();
            }
        }

        let def_candidates = [
            "/var/lib/clamav/daily.cvd",
            "/var/lib/clamav/daily.cld",
            "/var/lib/clamav/main.cvd",
            "/var/lib/clamav/main.cld",
        ];

        for p in &def_candidates {
            let path = Path::new(p);
            if path.exists() {
                if let Ok(meta) = path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if let Ok(dur) = SystemTime::now().duration_since(mtime) {
                            definitions_age_hours = dur.as_secs() / 3600;
                            definitions_stale = definitions_age_hours > 24;
                            definitions_date = format!("{}h ago", definitions_age_hours);
                            break;
                        }
                    }
                }
            }
        }

        ClamStatus {
            is_installed,
            version,
            definitions_date,
            definitions_stale,
            definitions_age_hours,
        }
    }

    pub fn install_clamav_streaming<F>(on_line: F) -> Result<i32, String>
    where
        F: FnMut(String),
    {
        let (cmd, args) = DistroDetector::get_clamav_install_command();
        HostBridge::spawn_streaming(cmd, &args, true, on_line)
    }

    pub fn update_definitions_streaming<F>(on_line: F) -> Result<i32, String>
    where
        F: FnMut(String),
    {
        // Resolve /var/log/clamav/freshclam.log lock contention by stopping service temporarily
        let script = "systemctl stop clamav-freshclam 2>/dev/null || true; \
                      systemctl stop freshclam 2>/dev/null || true; \
                      pkill -f freshclam 2>/dev/null || true; \
                      rm -f /var/log/clamav/freshclam.log.lock /var/log/freshclam.log.lock 2>/dev/null || true; \
                      freshclam; \
                      systemctl start clamav-freshclam 2>/dev/null || true; \
                      systemctl start freshclam 2>/dev/null || true";

        HostBridge::spawn_streaming("sh", &["-c", script], true, on_line)
    }

    pub fn scan_directory_streaming<F>(
        target_path: &str,
        cancel_flag: Arc<AtomicBool>,
        mut on_line: F,
    ) -> Result<Vec<ThreatFinding>, String>
    where
        F: FnMut(String),
    {
        let mut command = HostBridge::wrap_command("clamscan", &["-r", "--no-summary", target_path]);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn clamscan: {}", e))?;

        let stdout = child.stdout.take();
        let mut findings = Vec::new();

        if let Some(out) = stdout {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(out);
            for line in reader.lines().map_while(Result::ok) {
                if cancel_flag.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    return Err("Scan cancelled by user.".to_string());
                }

                on_line(line.clone());

                if line.ends_with("FOUND") {
                    let parts: Vec<&str> = line.split(": ").collect();
                    if parts.len() >= 2 {
                        findings.push(ThreatFinding {
                            file_path: parts[0].to_string(),
                            virus_name: parts[1].trim_end_matches(" FOUND").to_string(),
                        });
                    }
                }
            }
        }

        let _ = child.wait();
        Ok(findings)
    }

    // --- Quarantine Vault ---

    pub fn list_quarantined() -> Vec<QuarantineEntry> {
        let mpath = Self::manifest_file();
        if let Ok(content) = fs::read_to_string(&mpath) {
            if let Ok(entries) = serde_json::from_str::<Vec<QuarantineEntry>>(&content) {
                return entries;
            }
        }
        Vec::new()
    }

    fn save_manifest(entries: &[QuarantineEntry]) -> Result<(), String> {
        let mpath = Self::manifest_file();
        let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
        fs::write(mpath, json).map_err(|e| e.to_string())
    }

    pub fn quarantine_file(file_path: &str, virus_name: &str) -> Result<QuarantineEntry, String> {
        let p = Path::new(file_path);
        if !p.is_file() {
            return Err("Target file not found.".to_string());
        }

        let q_dir = Self::quarantine_files_dir();
        fs::create_dir_all(&q_dir).map_err(|e| e.to_string())?;

        let meta = p.metadata().map_err(|e| e.to_string())?;
        let size_bytes = meta.len();

        let sha256 = if let Ok((_, out, _)) = HostBridge::run_command("sha256sum", &[file_path]) {
            out.split_whitespace().next().unwrap_or("unknown").to_string()
        } else {
            "unknown".to_string()
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let id = format!("threat_{}_{}", now, meta.ino());
        let dest_file = q_dir.join(&id);

        fs::rename(p, &dest_file)
            .or_else(|_| fs::copy(p, &dest_file).and_then(|_| fs::remove_file(p)))
            .map_err(|e| format!("Failed to move file into quarantine vault: {}", e))?;

        // Revoke all execution and read permissions
        let mut perms = dest_file.metadata().map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o000);
        let _ = fs::set_permissions(&dest_file, perms);

        let entry = QuarantineEntry {
            id,
            original_path: file_path.to_string(),
            quarantined_path: dest_file.to_string_lossy().to_string(),
            virus_name: virus_name.to_string(),
            sha256,
            quarantined_at: format!("{}s epoch", now),
            size_bytes,
        };

        let mut list = Self::list_quarantined();
        list.push(entry.clone());
        Self::save_manifest(&list)?;

        Ok(entry)
    }

    pub fn restore_file(id: &str) -> Result<(), String> {
        let mut list = Self::list_quarantined();
        let idx = list.iter().position(|e| e.id == id).ok_or("Quarantine ID not found")?;
        let entry = list.remove(idx);

        let q_file = Path::new(&entry.quarantined_path);
        if q_file.exists() {
            let mut perms = q_file.metadata().map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o644);
            let _ = fs::set_permissions(q_file, perms);

            let orig = Path::new(&entry.original_path);
            if let Some(parent) = orig.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::rename(q_file, orig)
                .or_else(|_| fs::copy(q_file, orig).and_then(|_| fs::remove_file(q_file)))
                .map_err(|e| format!("Failed to restore file: {}", e))?;
        }

        Self::save_manifest(&list)
    }

    pub fn delete_quarantined(id: &str) -> Result<(), String> {
        let mut list = Self::list_quarantined();
        let idx = list.iter().position(|e| e.id == id).ok_or("Quarantine ID not found")?;
        let entry = list.remove(idx);

        let q_file = Path::new(&entry.quarantined_path);
        if q_file.exists() {
            let mut perms = q_file.metadata().map_err(|e| e.to_string())?.permissions();
            perms.set_mode(0o600);
            let _ = fs::set_permissions(q_file, perms);
            let _ = fs::remove_file(q_file);
        }

        Self::save_manifest(&list)
    }
}
