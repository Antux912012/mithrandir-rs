use mithrandir_rs::{
    AIClient, AntivirusEngine, BuiltinLinuxAssistant, ChatMessage, CleanerEngine,
    DiagnosticsCollector, DistroDetector, LocalLLMManager, SafetyGuard, SmartBoostEngine,
    SystemMonitor, get_recommended_qwen_models,
};
use std::fs;
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
fn test_cleaner_categories_coverage() {
    let cats = CleanerEngine::get_categories();
    assert!(cats.len() >= 8);
    let ids: Vec<&str> = cats.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&"thumbnails"));
    assert!(ids.contains(&"user_cache"));
    assert!(ids.contains(&"browser_cache"));
    assert!(ids.contains(&"stremio_cache"));
    assert!(ids.contains(&"dev_cache"));
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
fn test_system_summary() {
    let summary = SystemMonitor::get_system_summary();
    assert!(!summary.hostname.is_empty());
    assert!(!summary.kernel.is_empty());
    assert!(!summary.distro.is_empty());
    assert!(!summary.arch.is_empty());
    assert!(!summary.cpu_model.is_empty());
}

#[test]
fn test_sensor_telemetry_suite() {
    let net = SystemMonitor::get_network_rate();
    assert!(!net.rx_str.is_empty());
    assert!(!net.tx_str.is_empty());
    assert!(net.rx_bytes_sec >= 0.0);
    assert!(net.tx_bytes_sec >= 0.0);

    let bat = SystemMonitor::get_battery_info();
    assert!(!bat.status.is_empty());
    if bat.present {
        assert!(bat.design_capacity_wh > 0.0);
        assert!(bat.capacity_percent >= 0.0);
    }

    let gpus = SystemMonitor::get_gpus_info();
    for gpu in &gpus {
        assert!(!gpu.name.is_empty());
        assert!(gpu.load_percent >= 0.0);
    }
}

#[test]
fn test_smart_boost_status() {
    let status = SmartBoostEngine::get_boost_status();
    assert!(status.ram_total_mb > 0.0);
    assert!(!status.temp_size_str.is_empty());
}

#[test]
fn test_smart_boost_debounce() {
    assert!(SmartBoostEngine::check_debounce(500));
    // Immediate repeat within 500ms should be debounced
    assert!(!SmartBoostEngine::check_debounce(500));
}

#[test]
fn test_safety_pii_and_denylist() {
    let raw = "Log from user /home/antonio/test.log";
    std::env::set_var("USER", "antonio");
    let redacted = SafetyGuard::redact_pii(raw);
    assert!(!redacted.contains("/home/antonio"));
    assert!(redacted.contains("/home/[REDACTED_USER]"));

    let (safe_cmd, _) = SafetyGuard::is_command_safe("df -h /");
    assert!(safe_cmd);

    let (unsafe_cmd1, _) = SafetyGuard::is_command_safe("rm -rf /");
    assert!(!unsafe_cmd1);

    let (unsafe_cmd2, _) = SafetyGuard::is_command_safe("mkfs.ext4 /dev/sda1");
    assert!(!unsafe_cmd2);
}

#[test]
fn test_ai_url_normalization() {
    assert_eq!(AIClient::normalize_base_url("builtin://linux-mini"), "builtin://linux-mini");
    assert_eq!(AIClient::normalize_base_url("http://localhost:18080"), "http://127.0.0.1:18080/v1");
    assert_eq!(AIClient::normalize_base_url("http://127.0.0.1:18080/v1/chat/completions"), "http://127.0.0.1:18080/v1");
    assert_eq!(AIClient::normalize_base_url("http://localhost:11434"), "http://localhost:11434/v1");
    assert_eq!(AIClient::normalize_base_url("https://integrate.api.nvidia.com"), "https://integrate.api.nvidia.com/v1");
    assert_eq!(AIClient::normalize_base_url("https://generativelanguage.googleapis.com"), "https://generativelanguage.googleapis.com/v1beta");
    assert_eq!(
        AIClient::normalize_base_url("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"),
        "https://generativelanguage.googleapis.com/v1beta"
    );
    assert_eq!(AIClient::normalize_base_url("https://api.anthropic.com"), "https://api.anthropic.com/v1");
}

#[test]
fn test_gemini_payload_and_response() {
    let client = AIClient::new("https://generativelanguage.googleapis.com/v1beta", "AIzaTestKey", "");
    assert!(client.is_gemini());
    assert!(client.is_gemini_native());
    assert_eq!(client.resolve_model(None), "gemini-2.5-flash");
    assert_eq!(client.resolve_model(Some("models/gemini-2.5-pro")), "gemini-2.5-pro");

    let msgs = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are Mithrandir Linux assistant.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "Check disk usage".to_string(),
        },
    ];
    let payload = AIClient::build_gemini_payload(&msgs, 0.3);
    assert_eq!(
        payload["systemInstruction"]["parts"][0]["text"].as_str(),
        Some("You are Mithrandir Linux assistant.")
    );
    assert_eq!(
        payload["contents"][0]["parts"][0]["text"].as_str(),
        Some("Check disk usage")
    );

    let mock_resp = serde_json::json!({
        "candidates": [
            {
                "content": {
                    "parts": [
                        { "text": "Disk space looks fine.\nPROPOSED_COMMAND: df -h /" }
                    ],
                    "role": "model"
                }
            }
        ]
    });
    let parsed = AIClient::parse_gemini_response(&mock_resp).unwrap();
    assert!(parsed.contains("PROPOSED_COMMAND: df -h /"));
    assert_eq!(
        AIClient::extract_proposed_command(&parsed),
        Some("df -h /".to_string())
    );
}

#[test]
fn test_builtin_linux_assistant() {
    let msgs = vec![ChatMessage {
        role: "user".to_string(),
        content: "Diagnose system state: failed systemd units: None".to_string(),
    }];
    let resp = BuiltinLinuxAssistant::respond(&msgs);
    assert!(resp.contains("MITHRANDIR BUILT-IN OFFLINE LINUX DIAGNOSTICS"));
    assert!(resp.contains("PROPOSED_COMMAND:"));

    let proposed = AIClient::extract_proposed_command(&resp);
    assert!(proposed.is_some());
}

#[test]
fn test_qwen_gguf_catalog() {
    let models = get_recommended_qwen_models();
    assert_eq!(models.len(), 3);
    assert_eq!(models[0].id, "qwen2.5-1.5b-instruct-q4_k_m.gguf");
    assert!(models[0].url.contains("huggingface.co"));
}

#[test]
fn test_llamacpp_status_schema() {
    let st = LocalLLMManager::check_llamacpp_status();
    assert_eq!(st.port, 18080);
    assert_eq!(st.base_url, "http://127.0.0.1:18080/v1");
}

#[test]
fn test_distro_detection() {
    let d = DistroDetector::detect();
    assert!(!d.id.is_empty());
    assert!(!d.pretty_name.is_empty());
}

#[test]
fn test_diagnostics_snapshot_collection() {
    let snapshot = DiagnosticsCollector::gather_system_snapshot();
    assert!(snapshot.contains("GNU/Linux system diagnostics assistant"));
    assert!(snapshot.contains("- OS:"));
    assert!(snapshot.contains("- Memory:"));
}

#[test]
fn test_quarantine_vault_cycle() {
    let tmp_file = Path::new("/tmp/mithrandir_eicar_test.txt");
    let _ = fs::write(tmp_file, "X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*");

    let entry_res = AntivirusEngine::quarantine_file(tmp_file.to_str().unwrap(), "Eicar-Test-Signature");
    assert!(entry_res.is_ok());
    let entry = entry_res.unwrap();
    assert!(!tmp_file.exists());

    let list = AntivirusEngine::list_quarantined();
    assert!(list.iter().any(|e| e.id == entry.id));

    let restore_res = AntivirusEngine::restore_file(&entry.id);
    assert!(restore_res.is_ok());
    assert!(tmp_file.exists());

    let _ = fs::remove_file(tmp_file);
}
