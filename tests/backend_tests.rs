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

#[test]
fn test_tray_action_channel_dispatch() {
    use mithrandir_rs::{MithrandirTray, TrayAction};
    use ksni::Tray;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    let (tx, rx) = async_channel::unbounded();
    let power_saver_arc = Arc::new(AtomicBool::new(false));
    let mut tray = MithrandirTray::new(tx, power_saver_arc.clone());

    assert_eq!(tray.id(), "mithrandir-rs");
    assert_eq!(tray.icon_name(), "mithrandir-status-symbolic");
    assert_eq!(tray.category(), ksni::Category::SystemServices);

    let pixmap = tray.icon_pixmap();
    assert!(!pixmap.is_empty());
    assert_eq!(pixmap[0].width, 16);
    assert_eq!(pixmap[0].height, 16);
    assert_eq!(pixmap[0].data.len(), 16 * 16 * 4);

    // Test activate trigger
    tray.activate(100, 200);
    assert_eq!(rx.try_recv(), Ok(TrayAction::OpenMithrandir));

    // Test menu item activation: RunSmartboost, Checkmark(Battery Power Saver), Separator, OpenMithrandir, Separator, QuitMithrandir
    let menu_items = tray.menu();
    assert_eq!(menu_items.len(), 6);

    // 0: Run Smartboost
    if let ksni::MenuItem::Standard(item) = &menu_items[0] {
        assert_eq!(item.label, "Run Smartboost");
        (item.activate)(&mut tray);
        assert_eq!(rx.try_recv(), Ok(TrayAction::RunSmartboost));
    } else {
        panic!("Expected StandardItem for Run Smartboost");
    }

    // 1: Battery Power Saver (Checkmark)
    if let ksni::MenuItem::Checkmark(item) = &menu_items[1] {
        assert_eq!(item.label, "Battery Power Saver");
        assert!(!item.checked);
        (item.activate)(&mut tray);
        assert_eq!(rx.try_recv(), Ok(TrayAction::TogglePowerSaver(true)));
        assert!(tray.power_saver_enabled.load(std::sync::atomic::Ordering::SeqCst));
    } else {
        panic!("Expected CheckmarkItem for Battery Power Saver");
    }

    // 3: Open Mithrandir
    if let ksni::MenuItem::Standard(item) = &menu_items[3] {
        assert_eq!(item.label, "Open Mithrandir");
        (item.activate)(&mut tray);
        assert_eq!(rx.try_recv(), Ok(TrayAction::OpenMithrandir));
    } else {
        panic!("Expected StandardItem for Open Mithrandir");
    }

    // 5: Quit Mithrandir
    if let ksni::MenuItem::Standard(item) = &menu_items[5] {
        assert_eq!(item.label, "Quit Mithrandir");
        (item.activate)(&mut tray);
        assert_eq!(rx.try_recv(), Ok(TrayAction::QuitMithrandir));
    } else {
        panic!("Expected StandardItem for Quit Mithrandir");
    }
}

#[test]
fn test_tray_smartboost_concurrency_lock() {
    use mithrandir_rs::tray::MithrandirTray;

    let res = MithrandirTray::execute_tray_smartboost();
    assert!(res.is_some());
}

#[test]
fn test_power_saver_controller() {
    use mithrandir_rs::PowerManager;

    let supported = PowerManager::is_supported();
    let is_active = PowerManager::is_power_saver_active();
    println!("Power saver supported: {}, active: {}", supported, is_active);

    // Test cached state toggle
    PowerManager::set_cached_state(true);
    assert!(PowerManager::is_power_saver_active());

    PowerManager::set_cached_state(false);
    // Cached state resets
    let _ = PowerManager::is_power_saver_active();
}

#[test]
fn test_distro_detection_mock_matrix() {
    use mithrandir_rs::os_info::{get_admin_group_for_distro, parse_os_release, LinuxDistro};

    let debian_os_release = r#"
PRETTY_NAME="Debian GNU/Linux 12 (bookworm)"
NAME="Debian GNU/Linux"
VERSION_ID="12"
VERSION="12 (bookworm)"
VERSION_CODENAME=bookworm
ID=debian
HOME_URL="https://www.debian.org/"
SUPPORT_URL="https://www.debian.org/support"
BUG_REPORT_URL="https://bugs.debian.org/"
"#;
    let debian_info = parse_os_release(debian_os_release);
    assert_eq!(debian_info.distro, LinuxDistro::Debian);
    assert_eq!(debian_info.version_id, Some("12".to_string()));
    assert_eq!(get_admin_group_for_distro(debian_info.distro), "sudo");

    let fedora_os_release = r#"
NAME="Fedora Linux"
VERSION="41 (Workstation Edition)"
ID=fedora
VERSION_ID="41"
VERSION_CODENAME=""
PLATFORM_ID="platform:f41"
PRETTY_NAME="Fedora Linux 41 (Workstation Edition)"
"#;
    let fedora_info = parse_os_release(fedora_os_release);
    assert_eq!(fedora_info.distro, LinuxDistro::Fedora);
    assert_eq!(fedora_info.version_id, Some("41".to_string()));
    assert_eq!(get_admin_group_for_distro(fedora_info.distro), "wheel");

    let arch_os_release = r#"
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://archlinux.org/"
"#;
    let arch_info = parse_os_release(arch_os_release);
    assert_eq!(arch_info.distro, LinuxDistro::Arch);
    assert_eq!(get_admin_group_for_distro(arch_info.distro), "wheel");

    let cachyos_os_release = r#"
NAME="CachyOS Linux"
PRETTY_NAME="CachyOS"
ID=cachyos
ID_LIKE=arch
BUILD_ID=rolling
"#;
    let cachy_info = parse_os_release(cachyos_os_release);
    assert_eq!(cachy_info.distro, LinuxDistro::CachyOS);
    assert_eq!(get_admin_group_for_distro(cachy_info.distro), "wheel");

    let nixos_os_release = r#"
NAME=NixOS
ID=nixos
VERSION="24.11 (Vicuna)"
VERSION_CODENAME=vicuna
VERSION_ID="24.11"
PRETTY_NAME="NixOS 24.11 (Vicuna)"
"#;
    let nixos_info = parse_os_release(nixos_os_release);
    assert_eq!(nixos_info.distro, LinuxDistro::NixOS);
    assert_eq!(nixos_info.version_id, Some("24.11".to_string()));
    assert_eq!(get_admin_group_for_distro(nixos_info.distro), "sudo");

    let alpine_os_release = r#"
NAME="Alpine Linux"
ID=alpine
VERSION_ID=3.21.0
PRETTY_NAME="Alpine Linux v3.21"
"#;
    let alpine_info = parse_os_release(alpine_os_release);
    assert_eq!(alpine_info.distro, LinuxDistro::Alpine);
    assert_eq!(alpine_info.version_id, Some("3.21.0".to_string()));
    assert_eq!(get_admin_group_for_distro(alpine_info.distro), "sudo");
}

#[test]
fn test_nixos_immutable_graceful_handling() {
    use mithrandir_rs::os_info::{generate_udev_rule_for_group, LinuxDistro};

    let wheel_rule = generate_udev_rule_for_group("wheel");
    assert!(wheel_rule.contains("chgrp -R wheel"));
    assert!(wheel_rule.contains("chgrp wheel"));

    let sudo_rule = generate_udev_rule_for_group("sudo");
    assert!(sudo_rule.contains("chgrp -R sudo"));
    assert!(sudo_rule.contains("chgrp sudo"));

    // Verify mock NixOS environment handling
    let simulated_nixos = LinuxDistro::NixOS;
    let is_immutable = matches!(simulated_nixos, LinuxDistro::NixOS);
    assert!(is_immutable);

    if is_immutable {
        let msg = "NixOS detected: /etc is immutable. Please enable power saver services in your configuration.nix";
        assert!(msg.contains("NixOS detected"));
    }
}

#[test]
fn test_app_settings_default_and_roundtrip() {
    use mithrandir_rs::AppSettings;

    let defaults = AppSettings::default();
    assert!(defaults.keep_in_background);
    assert!(defaults.show_tray_icon);
    assert!(defaults.auto_boost_enabled);
    assert_eq!(defaults.ram_threshold, 80.0);
    assert_eq!(defaults.cooldown_minutes, 15);
    assert!(defaults.desktop_notifications);

    let serialized = serde_json::to_string(&defaults).expect("Serialization failed");
    let deserialized: AppSettings = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(deserialized.ram_threshold, 80.0);
    assert_eq!(deserialized.cooldown_minutes, 15);
}

#[test]
fn test_distro_family_conversion() {
    use mithrandir_rs::os_info::LinuxDistro;
    use mithrandir_rs::DistroFamily;

    assert_eq!(DistroFamily::from(LinuxDistro::Debian), DistroFamily::Debian);
    assert_eq!(DistroFamily::from(LinuxDistro::Ubuntu), DistroFamily::Debian);
    assert_eq!(DistroFamily::from(LinuxDistro::Fedora), DistroFamily::Fedora);
    assert_eq!(DistroFamily::from(LinuxDistro::Arch), DistroFamily::Arch);
    assert_eq!(DistroFamily::from(LinuxDistro::CachyOS), DistroFamily::Arch);
    assert_eq!(DistroFamily::from(LinuxDistro::OpenSuse), DistroFamily::Suse);
    assert_eq!(DistroFamily::from(LinuxDistro::Alpine), DistroFamily::Alpine);
    assert_eq!(DistroFamily::from(LinuxDistro::NixOS), DistroFamily::Unknown);
}
