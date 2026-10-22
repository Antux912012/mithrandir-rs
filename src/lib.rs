pub mod backend;
pub mod ai;
pub mod tray;

#[cfg(feature = "gui")]
pub mod ui;

pub use tray::{MithrandirTray, TrayAction};

pub use backend::{
    AntivirusEngine, AutostartManager, BackgroundDaemon, BatteryInfo, BtrfsManager, CleanCategory,
    CleanerEngine, DiagnosticsCollector, DistroDetector, DistroFamily, GrubManager,
    HostBridge, NotificationManager, PackageManager, ProcessManager, ServiceManager,
    SmartBoostEngine, SystemMonitor, ThreatFinding,
};

pub use ai::{
    AIClient, BuiltinLinuxAssistant, ChatMessage, ChatResponse, GgufModelInfo,
    LlamaCppStatus, LocalLLMManager, SafetyGuard, get_recommended_qwen_models,
};
