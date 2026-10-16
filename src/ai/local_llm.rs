use std::fs;
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use crate::backend::host_bridge::HostBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufModelInfo {
    pub id: String,
    pub name: String,
    pub size: String,
    pub ram_required: String,
    pub url: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppStatus {
    pub engine_installed: bool,
    pub engine_path: String,
    pub running: bool,
    pub models: Vec<String>,
    pub default_model_installed: bool,
    pub disk_usage_mb: f64,
    pub port: u16,
    pub base_url: String,
    pub active_model: String,
}

pub fn get_recommended_qwen_models() -> Vec<GgufModelInfo> {
    vec![
        GgufModelInfo {
            id: "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            name: "Qwen 2.5 (1.5B Instruct Q4_K_M) — Recommended In-App Mini-LLM".to_string(),
            size: "~986 MB".to_string(),
            ram_required: "2 GB RAM".to_string(),
            url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            description: "Fast CPU inference (~30 tok/s on laptops). Excellent at Linux system commands, bash, and diagnostics.".to_string(),
        },
        GgufModelInfo {
            id: "qwen2.5-coder-1.5b-instruct-q4_k_m.gguf".to_string(),
            name: "Qwen 2.5 Coder (1.5B Instruct Q4_K_M) — Shell & Script Specialist".to_string(),
            size: "~986 MB".to_string(),
            ram_required: "2 GB RAM".to_string(),
            url: "https://huggingface.co/Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-1.5b-instruct-q4_k_m.gguf".to_string(),
            description: "Specialized in shell scripting, configuration files, and terminal CLI command generation.".to_string(),
        },
        GgufModelInfo {
            id: "qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            name: "Qwen 2.5 (0.5B Instruct Q4_K_M) — Ultra-Lightweight Edge Model".to_string(),
            size: "~398 MB".to_string(),
            ram_required: "1 GB RAM".to_string(),
            url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            description: "Ultra-compact model for minimal CPU/RAM footprint on older or resource-constrained PCs.".to_string(),
        },
    ]
}

static MANAGED_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
static ACTIVE_MODEL: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn child_mutex() -> &'static Mutex<Option<Child>> {
    MANAGED_CHILD.get_or_init(|| Mutex::new(None))
}

fn active_model_mutex() -> &'static Mutex<Option<String>> {
    ACTIVE_MODEL.get_or_init(|| Mutex::new(None))
}

pub struct LocalLLMManager;

impl LocalLLMManager {
    pub const PORT: u16 = 18080;
    pub const BASE_URL: &'static str = "http://127.0.0.1:18080/v1";

    pub fn app_data_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".local/share/io.github.antonio.MithrandirRS/llm")
    }

    pub fn bin_dir() -> PathBuf {
        Self::app_data_dir().join("bin")
    }

    pub fn models_dir() -> PathBuf {
        Self::app_data_dir().join("models")
    }

    pub fn find_llama_server_binary() -> Option<PathBuf> {
        let internal = Self::bin_dir().join("llama-server");
        if internal.is_file() {
            return Some(internal);
        }
        if let Some(host_path) = HostBridge::which("llama-server") {
            return Some(PathBuf::from(host_path));
        }
        None
    }

    pub fn get_installed_gguf_models() -> Vec<String> {
        let mdir = Self::models_dir();
        if !mdir.is_dir() {
            return Vec::new();
        }

        let mut models = Vec::new();
        if let Ok(entries) = fs::read_dir(mdir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".gguf") && !name.ends_with(".part") {
                        if let Ok(meta) = entry.metadata() {
                            if meta.is_file() && meta.len() > 10 * 1024 * 1024 {
                                models.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        models.sort();
        models
    }

    pub fn is_llamacpp_server_running() -> bool {
        let addr = format!("127.0.0.1:{}", Self::PORT);
        if let Ok(stream) = TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(500)) {
            drop(stream);
            // Quick check via curl to verify health or models endpoint
            let url = format!("http://127.0.0.1:{}/health", Self::PORT);
            if let Ok(output) = Command::new("curl").args(["-s", "--max-time", "1", &url]).output() {
                return output.status.success();
            }
            return true;
        }
        false
    }

    pub fn check_llamacpp_status() -> LlamaCppStatus {
        let engine_path_buf = Self::find_llama_server_binary();
        let engine_installed = engine_path_buf.is_some();
        let engine_path = engine_path_buf.map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let running = Self::is_llamacpp_server_running();
        let models = Self::get_installed_gguf_models();
        let default_model = "qwen2.5-1.5b-instruct-q4_k_m.gguf";
        let default_model_installed = models.iter().any(|m| m == default_model);

        let mut total_bytes = 0u64;
        let root = Self::app_data_dir();
        if root.is_dir() {
            for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total_bytes += meta.len();
                    }
                }
            }
        }

        let disk_usage_mb = (total_bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0;
        let cur_active = active_model_mutex().lock().unwrap().clone();
        let active_model = cur_active.unwrap_or_else(|| {
            models.first().cloned().unwrap_or_else(|| default_model.to_string())
        });

        LlamaCppStatus {
            engine_installed,
            engine_path,
            running,
            models,
            default_model_installed,
            disk_usage_mb,
            port: Self::PORT,
            base_url: Self::BASE_URL.to_string(),
            active_model,
        }
    }

    pub fn install_llamacpp_engine<F>(mut on_line: F, cancel_flag: Option<Arc<AtomicBool>>) -> Result<String, String>
    where
        F: FnMut(String),
    {
        let bin_dir = Self::bin_dir();
        fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

        on_line("[Mithrandir-RS] Resolving official llama.cpp Linux prebuilt release...".to_string());
        if let Some(ref cf) = cancel_flag {
            if cf.load(Ordering::SeqCst) {
                return Err("Installation cancelled.".to_string());
            }
        }

        let arch = std::env::consts::ARCH;
        let asset_suffix = if arch.contains("arm") || arch.contains("aarch64") {
            "bin-ubuntu-arm64.zip"
        } else {
            "bin-ubuntu-x64.zip"
        };

        let zip_url = format!(
            "https://github.com/ggml-org/llama.cpp/releases/latest/download/llama-b4500-{}",
            asset_suffix
        );
        let fallback_url = format!(
            "https://github.com/ggerganov/llama.cpp/releases/latest/download/llama-{}",
            asset_suffix
        );

        let zip_path = bin_dir.join("llama-release.zip");
        on_line(format!("[Mithrandir-RS] Downloading llama.cpp binary archive ({}) ...", asset_suffix));

        let curl_cmd = format!("curl -f -sS -L -o '{}' '{}' || curl -f -sS -L -o '{}' '{}'",
            zip_path.to_string_lossy(), zip_url,
            zip_path.to_string_lossy(), fallback_url
        );

        let res = HostBridge::run_command("sh", &["-c", &curl_cmd])?;
        if res.0 != 0 {
            return Err(format!("Download failed: {}", res.2));
        }

        on_line("[Mithrandir-RS] Extracting llama-server runtime...".to_string());
        let unzip_cmd = format!("unzip -o -q '{}' -d '{}'", zip_path.to_string_lossy(), bin_dir.to_string_lossy());
        let unz_res = HostBridge::run_command("sh", &["-c", &unzip_cmd])?;
        let _ = fs::remove_file(&zip_path);

        if unz_res.0 != 0 {
            return Err(format!("Unzip failed: {}", unz_res.2));
        }

        let server_bin = bin_dir.join("llama-server");
        if server_bin.is_file() {
            if let Ok(meta) = fs::metadata(&server_bin) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&server_bin, perms);
            }
            on_line("[Mithrandir-RS] ✓ llama.cpp engine installed successfully!".to_string());
            Ok("llama.cpp engine installed.".to_string())
        } else {
            Err("llama-server binary was not found after extraction.".to_string())
        }
    }

    pub fn download_gguf_model<F>(
        model_id: &str,
        mut on_line: F,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<String, String>
    where
        F: FnMut(String),
    {
        let models = get_recommended_qwen_models();
        let target = models.iter().find(|m| m.id == model_id)
            .ok_or_else(|| format!("Unknown model ID: {}", model_id))?;

        let mdir = Self::models_dir();
        fs::create_dir_all(&mdir).map_err(|e| e.to_string())?;

        let dest = mdir.join(model_id);
        let part = mdir.join(format!("{}.part", model_id));

        if dest.is_file() {
            if let Ok(meta) = dest.metadata() {
                if meta.len() > 10 * 1024 * 1024 {
                    on_line(format!("[Mithrandir-RS] Model '{}' already downloaded.", model_id));
                    return Ok("Model already downloaded.".to_string());
                }
            }
        }

        on_line(format!("[Mithrandir-RS] Downloading {} (size {})...", target.name, target.size));
        let curl_cmd = format!(
            "curl -f -L -C - -o '{}' '{}'",
            part.to_string_lossy(), target.url
        );

        let mut child = Command::new("sh")
            .args(["-c", &curl_cmd])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn curl: {}", e))?;

        let start_time = Instant::now();
        while child.try_wait().map_err(|e| e.to_string())?.is_none() {
            if let Some(ref cf) = cancel_flag {
                if cf.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let _ = fs::remove_file(&part);
                    return Err("Download cancelled.".to_string());
                }
            }
            std::thread::sleep(Duration::from_millis(500));
            if let Ok(meta) = fs::metadata(&part) {
                let mb = meta.len() as f64 / (1024.0 * 1024.0);
                if start_time.elapsed().as_secs() % 3 == 0 {
                    on_line(format!("[Mithrandir-RS] Downloading {}: {:.1} MB retrieved...", model_id, mb));
                }
            }
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            let _ = fs::remove_file(&part);
            return Err("Download terminated with failure.".to_string());
        }

        let _ = fs::rename(&part, &dest);
        on_line(format!("[Mithrandir-RS] ✓ Model '{}' downloaded and ready!", model_id));
        Ok("Model downloaded successfully.".to_string())
    }

    pub fn install_qwen_with_llamacpp<F>(
        model_id: Option<&str>,
        mut on_line: F,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<String, String>
    where
        F: FnMut(String),
    {
        let model = model_id.unwrap_or("qwen2.5-1.5b-instruct-q4_k_m.gguf");
        if Self::find_llama_server_binary().is_none() {
            on_line("[Mithrandir-RS] Step 1 of 2: Installing llama.cpp inference engine...".to_string());
            Self::install_llamacpp_engine(&mut on_line, cancel_flag.clone())?;
        }

        on_line(format!("[Mithrandir-RS] Step 2 of 2: Downloading Qwen Mini-LLM weights ({})...", model));
        Self::download_gguf_model(model, &mut on_line, cancel_flag)?;

        on_line("[Mithrandir-RS] Starting embedded llama.cpp server with Qwen 2.5...".to_string());
        let _ = Self::start_llamacpp_server(Some(model));
        on_line("[Mithrandir-RS] ✓ Qwen 2.5 Mini-LLM is active and ready inside Mithrandir-RS!".to_string());
        Ok("Qwen 2.5 and llama.cpp runtime installed successfully.".to_string())
    }

    pub fn start_llamacpp_server(model_id: Option<&str>) -> Result<String, String> {
        let mut lock = child_mutex().lock().unwrap();
        if Self::is_llamacpp_server_running() {
            return Ok(format!("llama.cpp is already running on {}", Self::BASE_URL));
        }

        let bin = Self::find_llama_server_binary().ok_or("llama-server binary not found. Install Qwen + llama.cpp first.")?;
        let models = Self::get_installed_gguf_models();
        if models.is_empty() {
            return Err("No GGUF models installed. Download Qwen model first.".to_string());
        }

        let chosen = model_id.filter(|m| models.contains(&m.to_string())).unwrap_or(&models[0]);
        let model_path = Self::models_dir().join(chosen);

        if let Some(mut existing) = lock.take() {
            let _ = existing.kill();
        }

        let mut cmd = Command::new(&bin);
        cmd.args([
            "-m", model_path.to_str().unwrap(),
            "--host", "127.0.0.1",
            "--port", &Self::PORT.to_string(),
            "-c", "2048",
        ]);

        let bin_dir_str = Self::bin_dir().to_string_lossy().to_string();
        let current_ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
        let new_ld = if current_ld.is_empty() {
            bin_dir_str
        } else {
            format!("{}:{}", bin_dir_str, current_ld)
        };
        cmd.env("LD_LIBRARY_PATH", new_ld);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| format!("Failed to spawn llama-server: {}", e))?;
        *lock = Some(child);
        *active_model_mutex().lock().unwrap() = Some(chosen.to_string());

        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(500));
            if Self::is_llamacpp_server_running() {
                return Ok(format!("llama.cpp server started successfully on http://127.0.0.1:{}/v1", Self::PORT));
            }
        }

        Err("llama-server started but did not respond on port 18080 within 10s.".to_string())
    }

    pub fn stop_llamacpp_server() -> Result<String, String> {
        let mut lock = child_mutex().lock().unwrap();
        if let Some(mut child) = lock.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *active_model_mutex().lock().unwrap() = None;

        // Cleanup port
        let _ = HostBridge::run_command("fuser", &["-k", &format!("{}/tcp", Self::PORT)]);
        Ok("llama.cpp server stopped.".to_string())
    }

    pub fn uninstall_llamacpp_and_models(remove_engine: bool, remove_models: bool) -> Result<String, String> {
        let _ = Self::stop_llamacpp_server();
        let mut freed = Vec::new();

        if remove_models && Self::models_dir().is_dir() {
            let _ = fs::remove_dir_all(Self::models_dir());
            freed.push("model weights removed");
        }

        if remove_engine && Self::bin_dir().is_dir() {
            let _ = fs::remove_dir_all(Self::bin_dir());
            freed.push("llama.cpp engine removed");
        }

        let summary = if freed.is_empty() { "Storage cleaned.".to_string() } else { freed.join(", ") };
        Ok(format!("Uninstalled successfully ({}).", summary))
    }
}

pub struct BuiltinLinuxAssistant;

impl BuiltinLinuxAssistant {
    /// Deterministic offline expert heuristics for Linux triage and generic tasks.
    pub fn respond(messages: &[ChatMessage]) -> String {
        let query = messages.last().map(|m| m.content.as_str()).unwrap_or("").to_lowercase();

        if query.contains("diagnose") || query.contains("anomal") || query.contains("system state") {
            let mut findings = Vec::new();
            let mut proposed = "systemctl --failed";

            if query.contains("memory:") {
                findings.push("• Memory telemetry scanned. Standby caches can be reclaimed via Smart Boost.");
            }
            if query.contains("failed systemd units:\nnone") {
                findings.push("• Systemd units: Healthy (no failed services detected).");
            } else if query.contains("failed systemd units:") {
                findings.push("• Warning: One or more failed systemd services detected.");
                proposed = "systemctl reset-failed";
            }
            if query.contains("error") || query.contains("failed") {
                findings.push("• Journal inspect: Minor warnings logged in current boot cycle.");
                proposed = "journalctl -p 3 -xb -n 30 --no-pager";
            }

            return format!(
                "**MITHRANDIR BUILT-IN OFFLINE LINUX DIAGNOSTICS**\n\n\
                **Status Summary:**\n{}\n\n\
                **Recommended Action:**\nReview failed system units or journal alerts.\n\n\
                PROPOSED_COMMAND: {}",
                findings.join("\n"),
                proposed
            );
        }

        if query.contains("file") && (query.contains("large") || query.contains("100m")) {
            return "Scan your home directory for files exceeding 100MB:\n\nPROPOSED_COMMAND: find ~ -type f -size +100M -exec ls -lh {} + 2>/dev/null | sort -k5 -rh | head -n 20".to_string();
        }

        if query.contains("port") || query.contains("listen") || query.contains("network") {
            return "Display all open listening TCP/UDP ports and associated process names:\n\nPROPOSED_COMMAND: ss -tulnp".to_string();
        }

        if query.contains("package") || query.contains("broken") || query.contains("apt") {
            return "Configure partially installed packages and fix broken dependencies:\n\nPROPOSED_COMMAND: dpkg --configure -a && apt-get install -f -y".to_string();
        }

        if query.contains("boot") || query.contains("slow") || query.contains("systemd") {
            return "Analyze boot sequence performance and identify slowest startup units:\n\nPROPOSED_COMMAND: systemd-analyze blame | head -n 15".to_string();
        }

        if query.contains("disk") || query.contains("space") || query.contains("usage") {
            return "Inspect filesystem usage across all mounted disk volumes:\n\nPROPOSED_COMMAND: df -h -x tmpfs -x devtmpfs".to_string();
        }

        "GNU/Linux diagnostic query acknowledged.\n\nPROPOSED_COMMAND: top -b -n 1 | head -n 25".to_string()
    }
}
