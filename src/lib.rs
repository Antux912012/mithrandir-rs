pub mod backend;
pub mod cleaners;
pub mod ai;
pub mod tray;
pub mod power;
pub mod os_info;

#[cfg(feature = "gui")]
pub mod ui;

pub use tray::{MithrandirTray, TrayAction};
pub use power::PowerManager;
pub use os_info::{detect_os, get_admin_group, LinuxDistro, OsDetails};
pub use cleaners::{
    clean_default_flatpak_cache, clean_flatpak_cache, clean_flatpak_cache_with_running,
    get_running_flatpaks, is_protected_path, scan_default_flatpak_cache, scan_flatpak_cache,
};

pub use backend::{
    AntivirusEngine, AppSettings, AutostartManager, BackgroundDaemon, BatteryInfo, BtrfsManager, CleanCategory,
    CleanerEngine, DiagnosticsCollector, DistroDetector, DistroFamily, GrubManager,
    HostBridge, NotificationManager, PackageManager, ProcessManager, ServiceManager,
    SmartBoostEngine, SystemMonitor, ThreatFinding,
};

pub use ai::{
    AIClient, BuiltinLinuxAssistant, ChatMessage, ChatResponse, GgufModelInfo,
    LlamaCppStatus, LocalLLMManager, SafetyGuard, get_recommended_qwen_models,
};
