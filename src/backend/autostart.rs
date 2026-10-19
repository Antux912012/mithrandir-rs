use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutostartEntry {
    pub file_name: String,
    pub path: String,
    pub name: String,
    pub exec: String,
    pub comment: String,
    pub icon: String,
    pub enabled: bool,
    pub is_user: bool,
}

pub struct AutostartManager;

impl AutostartManager {
    fn get_user_autostart_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/antonio".to_string());
        PathBuf::from(home).join(".config/autostart")
    }

    pub fn is_mithrandir_autostart_enabled() -> bool {
        let path = Self::get_user_autostart_dir().join("io.github.antonio.MithrandirRS.desktop");
        if !path.exists() {
            return false;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                if line.trim() == "Hidden=true" || line.trim() == "X-GNOME-Autostart-enabled=false" {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn set_mithrandir_autostart(enabled: bool) -> Result<(), String> {
        let dir = Self::get_user_autostart_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("io.github.antonio.MithrandirRS.desktop");

        if enabled {
            let content = "[Desktop Entry]\n\
                           Type=Application\n\
                           Name=Mithrandir-RS\n\
                           Comment=Start Mithrandir-RS in background on login\n\
                           Exec=mithrandir-rs --keep-alive\n\
                           Icon=io.github.antonio.MithrandirRS\n\
                           Terminal=false\n\
                           Hidden=false\n\
                           X-GNOME-Autostart-enabled=true\n";
            fs::write(path, content).map_err(|e| e.to_string())?;
        } else {
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }

    pub fn list_entries() -> Vec<AutostartEntry> {
        let mut entries = Vec::new();
        let user_dir = Self::get_user_autostart_dir();
        let sys_dir = PathBuf::from("/etc/xdg/autostart");

        let mut seen = std::collections::HashSet::new();

        if user_dir.exists() {
            if let Ok(read_dir) = fs::read_dir(&user_dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if let Some(item) = Self::parse_desktop_file(&path, true) {
                            seen.insert(fname);
                            entries.push(item);
                        }
                    }
                }
            }
        }

        if sys_dir.exists() {
            if let Ok(read_dir) = fs::read_dir(&sys_dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if !seen.contains(&fname) {
                            if let Some(item) = Self::parse_desktop_file(&path, false) {
                                entries.push(item);
                            }
                        }
                    }
                }
            }
        }

        entries.sort_by_key(|a| a.name.to_lowercase());
        entries
    }

    fn parse_desktop_file(path: &Path, is_user: bool) -> Option<AutostartEntry> {
        let content = fs::read_to_string(path).ok()?;
        let mut name = String::new();
        let mut exec = String::new();
        let mut comment = String::new();
        let mut icon = String::new();
        let mut hidden = false;
        let mut autostart_enabled = true;

        for line in content.lines() {
            let l = line.trim();
            if l.starts_with("Name=") && name.is_empty() {
                name = l[5..].to_string();
            } else if l.starts_with("Exec=") && exec.is_empty() {
                exec = l[5..].to_string();
            } else if l.starts_with("Comment=") && comment.is_empty() {
                comment = l[8..].to_string();
            } else if l.starts_with("Icon=") && icon.is_empty() {
                icon = l[5..].to_string();
            } else if l == "Hidden=true" {
                hidden = true;
            } else if l == "X-GNOME-Autostart-enabled=false" {
                autostart_enabled = false;
            }
        }

        if name.is_empty() {
            name = path.file_stem()?.to_string_lossy().to_string();
        }

        let enabled = !hidden && autostart_enabled;

        Some(AutostartEntry {
            file_name: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            name,
            exec,
            comment,
            icon,
            enabled,
            is_user,
        })
    }

    pub fn toggle_entry(entry: &AutostartEntry, enable: bool) -> Result<(), String> {
        let user_dir = Self::get_user_autostart_dir();
        fs::create_dir_all(&user_dir).map_err(|e| e.to_string())?;
        let user_file = user_dir.join(&entry.file_name);

        let content = if user_file.exists() {
            fs::read_to_string(&user_file).unwrap_or_default()
        } else if Path::new(&entry.path).exists() {
            fs::read_to_string(&entry.path).unwrap_or_default()
        } else {
            String::new()
        };

        let mut lines: Vec<String> = content
            .lines()
            .filter(|l| !l.starts_with("Hidden=") && !l.starts_with("X-GNOME-Autostart-enabled="))
            .map(|s| s.to_string())
            .collect();

        if enable {
            lines.push("Hidden=false".to_string());
            lines.push("X-GNOME-Autostart-enabled=true".to_string());
        } else {
            lines.push("Hidden=true".to_string());
            lines.push("X-GNOME-Autostart-enabled=false".to_string());
        }

        fs::write(user_file, lines.join("\n") + "\n").map_err(|e| e.to_string())
    }

    pub fn add_entry(name: &str, exec: &str, comment: &str) -> Result<(), String> {
        let user_dir = Self::get_user_autostart_dir();
        fs::create_dir_all(&user_dir).map_err(|e| e.to_string())?;

        let safe_name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let file_path = user_dir.join(format!("{}.desktop", safe_name));

        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={}\n\
             Exec={}\n\
             Comment={}\n\
             Terminal=false\n\
             Hidden=false\n\
             X-GNOME-Autostart-enabled=true\n",
            name, exec, comment
        );

        fs::write(file_path, content).map_err(|e| e.to_string())
    }

    pub fn remove_entry(entry: &AutostartEntry) -> Result<(), String> {
        let user_dir = Self::get_user_autostart_dir();
        let user_file = user_dir.join(&entry.file_name);
        if user_file.exists() {
            fs::remove_file(user_file).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
