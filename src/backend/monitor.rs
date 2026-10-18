use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use crate::backend::distro::DistroDetector;
use crate::backend::host_bridge::HostBridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_mb: f64,
    pub used_mb: f64,
    pub free_mb: f64,
    pub available_mb: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub usage_percent: f32,
    pub brand: String,
    pub core_count: usize,
    pub temp_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskPartition {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub available_gb: f64,
    pub used_gb: f64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub load_percent: f32,
    pub temp_c: Option<f32>,
    pub mem_used_mb: Option<f64>,
    pub mem_total_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub present: bool,
    pub real_capacity_wh: f64,
    pub design_capacity_wh: f64,
    pub energy_now_wh: f64,
    pub capacity_percent: f64,
    pub status: String,
    pub model: String,
    pub vendor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRate {
    pub rx_kbps: f64,
    pub tx_kbps: f64,
    pub rx_bytes_sec: f64,
    pub tx_bytes_sec: f64,
    pub rx_str: String,
    pub tx_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSummary {
    pub hostname: String,
    pub distro: String,
    pub kernel: String,
    pub arch: String,
    pub cpu_model: String,
    pub desktop: String,
    pub uptime_str: String,
}

static SYSTEM_MUTEX: OnceLock<Mutex<System>> = OnceLock::new();
static NET_PREV: OnceLock<Mutex<Option<(Instant, u64, u64)>>> = OnceLock::new();

fn get_system() -> &'static Mutex<System> {
    SYSTEM_MUTEX.get_or_init(|| {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_all();
        sys.refresh_memory();
        Mutex::new(sys)
    })
}

pub struct SystemMonitor;

impl SystemMonitor {
    pub fn get_memory_info() -> MemoryInfo {
        let mut lock = get_system().lock().unwrap();
        lock.refresh_memory();

        let total_bytes = lock.total_memory();
        let used_bytes = lock.used_memory();
        let available_bytes = lock.available_memory();
        let free_bytes = lock.free_memory();

        let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
        let used_mb = used_bytes as f64 / (1024.0 * 1024.0);
        let available_mb = available_bytes as f64 / (1024.0 * 1024.0);
        let free_mb = free_bytes as f64 / (1024.0 * 1024.0);

        let percent = if total_mb > 0.0 {
            (used_mb / total_mb) * 100.0
        } else {
            0.0
        };

        MemoryInfo {
            total_mb: (total_mb * 10.0).round() / 10.0,
            used_mb: (used_mb * 10.0).round() / 10.0,
            free_mb: (free_mb * 10.0).round() / 10.0,
            available_mb: (available_mb * 10.0).round() / 10.0,
            percent: (percent * 10.0).round() / 10.0,
        }
    }

    pub fn get_cpu_info() -> CpuInfo {
        let mut lock = get_system().lock().unwrap();
        lock.refresh_cpu_usage();

        let usage_percent = lock.global_cpu_usage();
        let cpus = lock.cpus();
        let core_count = cpus.len();
        let brand = cpus
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Processor".to_string());

        let temp_c = Self::get_cpu_temperature();

        CpuInfo {
            usage_percent: (usage_percent * 10.0).round() / 10.0,
            brand,
            core_count,
            temp_c,
        }
    }

    pub fn get_cpu_temperature() -> Option<f32> {
        let mut temps = Vec::new();

        for zone in 0..8 {
            let path = format!("/sys/class/thermal/thermal_zone{}/temp", zone);
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(val) = content.trim().parse::<f32>() {
                    let t = if val > 200.0 { val / 1000.0 } else { val };
                    if (10.0..=115.0).contains(&t) {
                        temps.push(t);
                    }
                }
            }
        }

        for hwmon in 0..8 {
            for t_idx in 1..6 {
                let path = format!("/sys/class/hwmon/hwmon{}/temp{}_input", hwmon, t_idx);
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(val) = content.trim().parse::<f32>() {
                        let t = if val > 200.0 { val / 1000.0 } else { val };
                        if (10.0..=115.0).contains(&t) {
                            temps.push(t);
                        }
                    }
                }
            }
        }

        temps
            .into_iter()
            .reduce(f32::max)
            .map(|max_t| (max_t * 10.0).round() / 10.0)
    }

    pub fn get_disks_info() -> Vec<DiskPartition> {
        let disks = Disks::new_with_refreshed_list();
        disks
            .iter()
            .map(|d| {
                let total = d.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let available = d.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let used = (total - available).max(0.0);
                let percent = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };

                DiskPartition {
                    name: d.name().to_string_lossy().into_owned(),
                    mount_point: d.mount_point().to_string_lossy().into_owned(),
                    total_gb: (total * 10.0).round() / 10.0,
                    available_gb: (available * 10.0).round() / 10.0,
                    used_gb: (used * 10.0).round() / 10.0,
                    percent: (percent * 10.0).round() / 10.0,
                }
            })
            .collect()
    }

    pub fn get_gpus_info() -> Vec<GpuInfo> {
        let mut gpus = Vec::new();

        // 1. Only invoke nvidia-smi if the NVIDIA kernel module is loaded
        if Path::new("/proc/driver/nvidia/version").exists() && HostBridge::which("nvidia-smi").is_some() {
            if let Ok((code, stdout, _)) = HostBridge::run_command(
                "nvidia-smi",
                &[
                    "--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total",
                    "--format=csv,noheader,nounits",
                ],
            ) {
                if code == 0 && !stdout.trim().is_empty() {
                    for line in stdout.trim().lines() {
                        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 3 {
                            let name = parts[0].to_string();
                            let load = parts[1].parse::<f32>().unwrap_or(0.0);
                            let temp = parts[2].parse::<f32>().ok();
                            let mem_used = parts.get(3).and_then(|s| s.parse::<f64>().ok());
                            let mem_total = parts.get(4).and_then(|s| s.parse::<f64>().ok());

                            gpus.push(GpuInfo {
                                name,
                                load_percent: load,
                                temp_c: temp,
                                mem_used_mb: mem_used,
                                mem_total_mb: mem_total,
                            });
                        }
                    }
                }
            }
        }

        // 2. Check DRM cards (/sys/class/drm/card0, card1)
        for card_idx in [0usize, 1usize] {
            if gpus.len() > card_idx {
                continue;
            }

            let card_path = format!("/sys/class/drm/card{}", card_idx);
            let vendor_path = format!("{}/device/vendor", card_path);
            let Ok(vendor_str) = fs::read_to_string(&vendor_path) else {
                continue;
            };

            let vendor_id = vendor_str.trim().to_lowercase();
            let vendor_name = if vendor_id.contains("0x8086") {
                "Intel Graphics"
            } else if vendor_id.contains("0x1002") {
                "AMD Radeon"
            } else if vendor_id.contains("0x10de") {
                "NVIDIA"
            } else {
                "GPU"
            };

            let busy_path = format!("{}/device/gpu_busy_percent", card_path);
            let busy = fs::read_to_string(&busy_path)
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .unwrap_or(0.0);

            gpus.push(GpuInfo {
                name: vendor_name.to_string(),
                load_percent: busy,
                temp_c: None,
                mem_used_mb: None,
                mem_total_mb: None,
            });
        }

        gpus
    }

    pub fn get_gpu_info() -> Option<GpuInfo> {
        Self::get_gpus_info().into_iter().next()
    }

    pub fn get_battery_info() -> BatteryInfo {
        let power_dir = Path::new("/sys/class/power_supply");
        let mut devices: Vec<String> = Vec::new();

        if let Ok(entries) = fs::read_dir(power_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    devices.push(name.to_string());
                }
            }
        }
        devices.sort();

        let mut total_real_wh = 0.0;
        let mut total_design_wh = 0.0;
        let mut total_now_wh = 0.0;
        let mut found_any = false;
        let mut status = "Unknown".to_string();
        let mut model = String::new();
        let mut vendor = String::new();

        for dev in &devices {
            let dev_path = power_dir.join(dev);
            let type_str = fs::read_to_string(dev_path.join("type")).ok();
            let is_battery = if let Some(ref t) = type_str {
                t.trim().to_lowercase().contains("battery")
            } else {
                dev.to_lowercase().starts_with("bat")
            };

            if !is_battery {
                continue;
            }

            if let Ok(present_str) = fs::read_to_string(dev_path.join("present")) {
                if present_str.trim() == "0" {
                    continue;
                }
            }

            let mut real_wh = 0.0;
            let mut design_wh = 0.0;
            let mut now_wh = 0.0;

            let ef_str = fs::read_to_string(dev_path.join("energy_full")).ok();
            let efd_str = fs::read_to_string(dev_path.join("energy_full_design")).ok();
            let en_str = fs::read_to_string(dev_path.join("energy_now")).ok();

            if let (Some(ef), Some(efd)) = (ef_str, efd_str) {
                if let (Ok(ef_val), Ok(efd_val)) = (ef.trim().parse::<f64>(), efd.trim().parse::<f64>()) {
                    real_wh = ef_val / 1_000_000.0;
                    design_wh = efd_val / 1_000_000.0;
                    if let Some(en) = en_str {
                        if let Ok(en_val) = en.trim().parse::<f64>() {
                            now_wh = en_val / 1_000_000.0;
                        }
                    }
                }
            } else {
                let cf_str = fs::read_to_string(dev_path.join("charge_full")).ok();
                let cfd_str = fs::read_to_string(dev_path.join("charge_full_design")).ok();
                let cn_str = fs::read_to_string(dev_path.join("charge_now")).ok();
                let vmin_str = fs::read_to_string(dev_path.join("voltage_min_design")).ok();
                let vnow_str = fs::read_to_string(dev_path.join("voltage_now")).ok();

                let v_uv = vmin_str
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .or_else(|| vnow_str.and_then(|s| s.trim().parse::<f64>().ok()));

                if let (Some(cf), Some(cfd), Some(v)) = (cf_str, cfd_str, v_uv) {
                    if v > 0.0 {
                        if let (Ok(cf_val), Ok(cfd_val)) = (cf.trim().parse::<f64>(), cfd.trim().parse::<f64>()) {
                            real_wh = (cf_val * v) / 1e12;
                            design_wh = (cfd_val * v) / 1e12;
                            if let Some(cn) = cn_str {
                                if let Ok(cn_val) = cn.trim().parse::<f64>() {
                                    now_wh = (cn_val * v) / 1e12;
                                }
                            }
                        }
                    }
                }
            }

            if design_wh > 0.0 {
                found_any = true;
                total_real_wh += real_wh;
                total_design_wh += design_wh;
                total_now_wh += now_wh;

                if let Ok(st) = fs::read_to_string(dev_path.join("status")) {
                    let st_trimmed = st.trim();
                    if !st_trimmed.is_empty() {
                        status = st_trimmed.to_string();
                    }
                }
                if let Ok(md) = fs::read_to_string(dev_path.join("model_name")) {
                    let md_trimmed = md.trim();
                    if !md_trimmed.is_empty() {
                        model = md_trimmed.to_string();
                    }
                }
                if let Ok(vn) = fs::read_to_string(dev_path.join("manufacturer")) {
                    let vn_trimmed = vn.trim();
                    if !vn_trimmed.is_empty() {
                        vendor = vn_trimmed.to_string();
                    }
                }
            }
        }

        // UPower fallback if sysfs produced no battery data
        if !found_any && HostBridge::which("upower").is_some() {
            if let Ok((0, out, _)) = HostBridge::run_command("upower", &["-e"]) {
                for line in out.lines() {
                    let u_dev = line.trim();
                    if !u_dev.to_lowercase().contains("battery") {
                        continue;
                    }
                    if let Ok((0, info, _)) = HostBridge::run_command("upower", &["-i", u_dev]) {
                        let mut ef_wh: Option<f64> = None;
                        let mut efd_wh: Option<f64> = None;
                        let mut en_wh: Option<f64> = None;

                        let parse_wh = |val: &str| -> Option<f64> {
                            if !val.contains("Wh") {
                                return None;
                            }
                            let num_part = val.split_whitespace().next()?.replace(',', ".");
                            num_part.parse::<f64>().ok()
                        };

                        for info_line in info.lines() {
                            if let Some((k, v)) = info_line.split_once(':') {
                                let key = k.trim();
                                let val = v.trim();
                                match key {
                                    "energy-full" => ef_wh = parse_wh(val),
                                    "energy-full-design" => efd_wh = parse_wh(val),
                                    "energy" => en_wh = parse_wh(val),
                                    "state" if !val.is_empty() => {
                                        let mut chars = val.chars();
                                        if let Some(first) = chars.next() {
                                            status = format!("{}{}", first.to_uppercase(), chars.as_str());
                                        }
                                    }
                                    "model" if !val.is_empty() => model = val.to_string(),
                                    "vendor" if !val.is_empty() => vendor = val.to_string(),
                                    _ => {}
                                }
                            }
                        }

                        if let (Some(r_wh), Some(d_wh)) = (ef_wh, efd_wh) {
                            if d_wh > 0.0 {
                                total_real_wh += r_wh;
                                total_design_wh += d_wh;
                                if let Some(n_wh) = en_wh {
                                    total_now_wh += n_wh;
                                }
                                found_any = true;
                            }
                        }
                    }
                }
            }
        }

        if !found_any || total_design_wh <= 0.0 {
            return BatteryInfo {
                present: false,
                real_capacity_wh: 0.0,
                design_capacity_wh: 0.0,
                energy_now_wh: 0.0,
                capacity_percent: 0.0,
                status: "N/A".to_string(),
                model: String::new(),
                vendor: String::new(),
            };
        }

        let cap_pct = ((total_real_wh / total_design_wh) * 100.0 * 10.0).round() / 10.0;
        BatteryInfo {
            present: true,
            real_capacity_wh: (total_real_wh * 100.0).round() / 100.0,
            design_capacity_wh: (total_design_wh * 100.0).round() / 100.0,
            energy_now_wh: (total_now_wh * 100.0).round() / 100.0,
            capacity_percent: cap_pct,
            status,
            model,
            vendor,
        }
    }

    pub fn get_network_rate() -> NetworkRate {
        let lock = NET_PREV.get_or_init(|| Mutex::new(None));
        let mut prev = lock.lock().unwrap();
        let now = Instant::now();

        let mut cur_rx = 0u64;
        let mut cur_tx = 0u64;

        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                if let Some((iface_part, stats_part)) = line.split_once(':') {
                    let iface = iface_part.trim();
                    if iface == "lo"
                        || iface == "docker0"
                        || iface == "virbr0"
                        || iface.starts_with("veth")
                    {
                        continue;
                    }
                    let vals: Vec<&str> = stats_part.split_whitespace().collect();
                    if vals.len() >= 9 {
                        let rx = vals[0].parse::<u64>().unwrap_or(0);
                        let tx = vals[8].parse::<u64>().unwrap_or(0);
                        cur_rx += rx;
                        cur_tx += tx;
                    }
                }
            }
        }

        let mut rx_bytes_sec = 0.0;
        let mut tx_bytes_sec = 0.0;

        if let Some((last_time, last_rx, last_tx)) = *prev {
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.0 && cur_rx >= last_rx && cur_tx >= last_tx {
                rx_bytes_sec = (cur_rx - last_rx) as f64 / elapsed;
                tx_bytes_sec = (cur_tx - last_tx) as f64 / elapsed;
            }
        }

        *prev = Some((now, cur_rx, cur_tx));

        fn fmt_rate(rate_bytes: f64) -> String {
            if rate_bytes >= 1024.0 * 1024.0 {
                format!("{:.1} MB/s", rate_bytes / (1024.0 * 1024.0))
            } else if rate_bytes >= 1024.0 {
                format!("{:.1} KB/s", rate_bytes / 1024.0)
            } else {
                format!("{:.0} B/s", rate_bytes.floor())
            }
        }

        let rx_kbps = ((rx_bytes_sec / 1024.0) * 10.0).round() / 10.0;
        let tx_kbps = ((tx_bytes_sec / 1024.0) * 10.0).round() / 10.0;

        NetworkRate {
            rx_kbps,
            tx_kbps,
            rx_bytes_sec,
            tx_bytes_sec,
            rx_str: fmt_rate(rx_bytes_sec),
            tx_str: fmt_rate(tx_bytes_sec),
        }
    }

    pub fn get_system_summary() -> SystemSummary {
        let distro = DistroDetector::detect();
        let hostname = fs::read_to_string("/etc/hostname")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                HostBridge::run_command("hostname", &[])
                    .ok()
                    .map(|(_, out, _)| out.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "localhost".to_string());

        let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                HostBridge::run_command("uname", &["-r"])
                    .ok()
                    .map(|(_, out, _)| out.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let arch = HostBridge::run_command("uname", &["-m"])
            .ok()
            .and_then(|(code, out, _)| {
                let s = out.trim().to_string();
                if code == 0 && !s.is_empty() {
                    Some(s)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| std::env::consts::ARCH.to_string());

        let mut cpu_model = String::new();
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.contains("model name") {
                    if let Some((_, val)) = line.split_once(':') {
                        cpu_model = val.trim().to_string();
                        break;
                    }
                }
            }
        }
        if cpu_model.is_empty() {
            if let Ok(lock) = get_system().lock() {
                if let Some(first_cpu) = lock.cpus().first() {
                    let b = first_cpu.brand().trim();
                    if !b.is_empty() {
                        cpu_model = b.to_string();
                    }
                }
            }
        }
        if cpu_model.is_empty() {
            cpu_model = "Unknown".to_string();
        }

        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                std::env::var("DESKTOP_SESSION")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| "Unknown".to_string());

        let uptime_str = if let Ok(content) = fs::read_to_string("/proc/uptime") {
            if let Some(secs_str) = content.split_whitespace().next() {
                if let Ok(secs) = secs_str.parse::<f64>() {
                    let total_secs = secs as u64;
                    let hours = total_secs / 3600;
                    let mins = (total_secs % 3600) / 60;
                    format!("{}h {}m", hours, mins)
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        SystemSummary {
            hostname,
            distro: distro.pretty_name,
            kernel,
            arch,
            cpu_model,
            desktop,
            uptime_str,
        }
    }
}
