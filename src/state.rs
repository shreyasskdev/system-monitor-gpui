use std::collections::{HashMap, HashSet};
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemoryCalculationMode {
    FastRss,
    RealTruePss,
}

impl MemoryCalculationMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FastRss => "Fast RSS (statm)",
            Self::RealTruePss => "True PSS (smaps_rollup)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::FastRss => {
                "Instant lookup via /proc/<pid>/statm. Zero overhead, includes shared libs."
            }
            Self::RealTruePss => {
                "Exact Proportional Set Size via /proc/<pid>/smaps_rollup. Splits shared libs."
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortColumn {
    Pid,
    Name,
    Cpu,
    Memory,
}

#[derive(Clone, Debug)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub display_memory: u64,
    pub rss_memory: u64,
    pub virtual_memory: u64,
    pub process_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct GpuMetrics {
    pub id: String,
    pub name: String,
    pub usage_pct: f32,
    pub vram_used: u64,
    pub vram_total: u64,
    pub temp_c: Option<f32>,
    pub freq_mhz: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct ZramMetrics {
    pub active: bool,
    pub disksize_bytes: u64,
    pub orig_data_bytes: u64,
    pub compr_data_bytes: u64,
    pub ram_used_bytes: u64,
    pub algorithm: String,
}

impl ZramMetrics {
    pub fn compression_ratio(&self) -> Option<f64> {
        if self.ram_used_bytes > 64 * 1024 && self.orig_data_bytes > self.ram_used_bytes {
            Some(self.orig_data_bytes as f64 / self.ram_used_bytes as f64)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SystemMetrics {
    pub tick: u64,
    pub host_name: String,
    pub kernel_version: String,
    pub uptime_secs: u64,
    pub load_avg: (f64, f64, f64),
    pub cpu_brand: String,
    pub cpu_usage: Vec<f32>,
    pub global_cpu: f32,
    pub cpu_temp_c: Option<f32>,
    pub memory_usage: f64,
    pub memory_used: u64,
    pub memory_available: u64,
    pub memory_total: u64,
    pub memory_free: u64,
    pub memory_cached_buffers: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub zram: ZramMetrics,
    pub disk_used: u64,
    pub disk_total: u64,
    pub gpus: Vec<GpuMetrics>,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub net_total_rx: u64,
    pub net_total_tx: u64,
    pub battery_pct: Option<f32>,
    pub battery_status: String,
    pub processes: Vec<ProcessEntry>,
}

#[derive(Clone, Debug)]
pub struct MonitorConfig {
    pub refresh_ms: u64,
    pub memory_mode: MemoryCalculationMode,
    pub group_by_app: bool,
    pub paused: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            refresh_ms: 2000,                            // Default: 2s (2000ms)
            memory_mode: MemoryCalculationMode::FastRss, // Default: Fast RSS
            group_by_app: true,
            paused: false,
        }
    }
}

fn shorten_cpu_brand(raw: &str) -> String {
    let cleaned = raw
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace("11th Gen ", "")
        .replace("12th Gen ", "")
        .replace("13th Gen ", "")
        .replace("14th Gen ", "")
        .replace("Intel ", "")
        .replace("AMD ", "")
        .replace("Processor", "");
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn shorten_gpu_name(raw: &str) -> String {
    raw.replace("NVIDIA GeForce ", "")
        .replace("AMD Radeon ", "")
        .replace("Intel Integrated Graphics (iGPU)", "Intel iGPU")
        .replace("Integrated / System GPU", "System GPU")
        .trim()
        .to_string()
}

fn read_linux_pss_bytes(pid: u32) -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let path = format!("/proc/{}/smaps_rollup", pid);
        let content = fs::read_to_string(path).ok()?;
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("Pss:") {
                let kb_str = rest.trim().split_whitespace().next()?;
                let kb: u64 = kb_str.parse().ok()?;
                return Some(kb * 1024);
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

#[cfg(target_os = "linux")]
fn read_linux_true_memory() -> Option<(u64, u64, u64, u64)> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = None;
    let mut avail_kb = None;
    let mut free_kb = None;
    let mut buffers_kb = 0u64;
    let mut cached_kb = 0u64;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("MemFree:") {
            free_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok());
        } else if let Some(rest) = line.strip_prefix("Buffers:") {
            buffers_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("Cached:") {
            cached_kb = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
        }
    }

    let total = total_kb? * 1024;
    let avail = avail_kb? * 1024;
    let free = free_kb.unwrap_or(0) * 1024;
    let cached_buffers = (buffers_kb + cached_kb) * 1024;
    Some((total, avail, free, cached_buffers))
}

fn read_linux_zram(swap_used: u64) -> ZramMetrics {
    let mut zram = ZramMetrics::default();

    #[cfg(target_os = "linux")]
    {
        for idx in 0..4 {
            let base = format!("/sys/block/zram{}", idx);
            let Ok(disksize_str) = fs::read_to_string(format!("{}/disksize", base)) else {
                continue;
            };
            let disksize = disksize_str.trim().parse::<u64>().unwrap_or(0);
            if disksize == 0 {
                continue;
            }

            zram.active = true;
            zram.disksize_bytes = zram.disksize_bytes.saturating_add(disksize);

            if let Ok(mm_stat) = fs::read_to_string(format!("{}/mm_stat", base)) {
                let parts: Vec<u64> = mm_stat
                    .split_whitespace()
                    .filter_map(|s| s.parse::<u64>().ok())
                    .collect();
                if parts.len() >= 3 {
                    zram.orig_data_bytes = zram.orig_data_bytes.saturating_add(parts[0]);
                    zram.compr_data_bytes = zram.compr_data_bytes.saturating_add(parts[1]);
                    zram.ram_used_bytes = zram.ram_used_bytes.saturating_add(parts[2]);
                }
            }

            if zram.algorithm.is_empty() {
                if let Ok(comp_alg) = fs::read_to_string(format!("{}/comp_algorithm", base)) {
                    if let Some(start) = comp_alg.find('[') {
                        if let Some(end) = comp_alg[start + 1..].find(']') {
                            zram.algorithm = comp_alg[start + 1..start + 1 + end].to_string();
                        }
                    } else {
                        zram.algorithm = comp_alg.trim().to_string();
                    }
                }
            }
        }
    }

    if zram.active && zram.orig_data_bytes < swap_used {
        zram.orig_data_bytes = swap_used;
    }

    if zram.algorithm.is_empty() && zram.active {
        zram.algorithm = "zstd".to_string();
    }

    zram
}

fn read_linux_battery() -> (Option<f32>, String) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("BAT") {
                    let cap_path = entry.path().join("capacity");
                    let status_path = entry.path().join("status");
                    let pct = fs::read_to_string(cap_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<f32>().ok());
                    let status = fs::read_to_string(status_path)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    if pct.is_some() {
                        return (pct, status);
                    }
                }
            }
        }
        (None, "AC".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        (None, "AC".to_string())
    }
}

fn read_linux_gpus(components: &Components) -> Vec<GpuMetrics> {
    let mut gpus: Vec<GpuMetrics> = Vec::new();

    let mut fallback_temp = None;
    for comp in components.list() {
        let label = comp.label().to_lowercase();
        if label.contains("gpu") || label.contains("edge") || label.contains("amdgpu") {
            let t = comp.temperature();
            if fallback_temp.is_none() && t > 0.0 {
                fallback_temp = Some(t);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mut has_nvidia_smi = false;
        if let Ok(output) = Command::new("nvidia-smi")
            .args([
                "--query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,clocks.current.graphics",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 6 {
                        has_nvidia_smi = true;
                        let idx = parts[0];
                        let name = shorten_gpu_name(parts[1]);
                        let usage_pct = parts[2].parse::<f32>().unwrap_or(0.0);
                        let vram_used = parts[3].parse::<u64>().unwrap_or(0) * 1024 * 1024;
                        let vram_total = parts[4].parse::<u64>().unwrap_or(0) * 1024 * 1024;
                        let temp_c = parts[5].parse::<f32>().ok();
                        let freq_mhz = parts.get(6).and_then(|s| s.parse::<u32>().ok());

                        gpus.push(GpuMetrics {
                            id: format!("nvidia-{}", idx),
                            name,
                            usage_pct,
                            vram_used,
                            vram_total,
                            temp_c,
                            freq_mhz,
                        });
                    }
                }
            }
        }

        for card_idx in 0..8 {
            let card_dir = format!("/sys/class/drm/card{}", card_idx);
            let dev_dir = format!("{}/device", card_dir);
            let vendor_path = format!("{}/vendor", dev_dir);

            let Ok(vendor_raw) = fs::read_to_string(&vendor_path) else {
                continue;
            };
            let vendor = vendor_raw.trim();

            if vendor == "0x10de" && has_nvidia_smi {
                continue;
            }

            let mut usage_pct = fs::read_to_string(format!("{}/gpu_busy_percent", dev_dir))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .unwrap_or(0.0);

            let cur_freq = fs::read_to_string(format!("{}/gt_act_freq_mhz", card_dir))
                .or_else(|_| fs::read_to_string(format!("{}/gt_cur_freq_mhz", card_dir)))
                .or_else(|_| fs::read_to_string(format!("{}/gt/gt0/rps_act_freq_mhz", card_dir)))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok());

            let max_freq = fs::read_to_string(format!("{}/gt_max_freq_mhz", card_dir))
                .or_else(|_| fs::read_to_string(format!("{}/gt/gt0/rps_max_freq_mhz", card_dir)))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok());

            let min_freq = fs::read_to_string(format!("{}/gt_min_freq_mhz", card_dir))
                .or_else(|_| fs::read_to_string(format!("{}/gt/gt0/rps_min_freq_mhz", card_dir)))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(300);

            if usage_pct == 0.0 {
                if let (Some(cur), Some(max)) = (cur_freq, max_freq) {
                    if max > min_freq && cur > min_freq {
                        usage_pct = ((cur - min_freq) as f32 / (max - min_freq) as f32 * 100.0)
                            .clamp(0.0, 100.0);
                    }
                }
            }

            let vram_used = fs::read_to_string(format!("{}/mem_info_vram_used", dev_dir))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            let vram_total = fs::read_to_string(format!("{}/mem_info_vram_total", dev_dir))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);

            let mut card_temp = None;
            if let Ok(hwmons) = fs::read_dir(format!("{}/hwmon", dev_dir)) {
                for hm in hwmons.flatten() {
                    if let Ok(t_millideg) = fs::read_to_string(hm.path().join("temp1_input")) {
                        if let Ok(md) = t_millideg.trim().parse::<f32>() {
                            if md > 0.0 {
                                card_temp = Some(md / 1000.0);
                                break;
                            }
                        }
                    }
                }
            }

            let name = match vendor {
                "0x8086" => "Intel iGPU".to_string(),
                "0x1002" => format!("Radeon c{}", card_idx),
                "0x10de" => format!("NVIDIA c{}", card_idx),
                _ => format!("DRM c{}", card_idx),
            };

            gpus.push(GpuMetrics {
                id: format!("drm-card{}", card_idx),
                name,
                usage_pct,
                vram_used,
                vram_total,
                temp_c: card_temp.or(fallback_temp),
                freq_mhz: cur_freq,
            });
        }
    }

    gpus.sort_by(|a, b| a.id.cmp(&b.id));

    if gpus.is_empty() {
        gpus.push(GpuMetrics {
            id: "default-gpu".to_string(),
            name: "System GPU".to_string(),
            usage_pct: 0.0,
            vram_used: 0,
            vram_total: 0,
            temp_c: fallback_temp,
            freq_mhz: None,
        });
    }

    gpus
}

fn read_cpu_temp(components: &Components) -> Option<f32> {
    for comp in components.list() {
        let label = comp.label().to_lowercase();
        if label.contains("tctl")
            || label.contains("tdie")
            || label.contains("package")
            || label.contains("core 0")
            || label.contains("cpu")
        {
            let t = comp.temperature();
            if t > 0.0 {
                return Some(t);
            }
        }
    }
    components.list().first().and_then(|c| {
        let t = c.temperature();
        if t > 0.0 { Some(t) } else { None }
    })
}

pub fn send_process_signal(pid: u32, force_kill: bool) -> bool {
    let sig = if force_kill { "-9" } else { "-15" };
    Command::new("kill")
        .args([sig, &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn spawn_collector(
    metrics_lock: Arc<Mutex<SystemMetrics>>,
    config_lock: Arc<Mutex<MonitorConfig>>,
) {
    thread::spawn(move || {
        let mut sys = System::new_all();
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let mut components = Components::new_with_refreshed_list();
        let mut tick_counter: u64 = 0;

        let host_name = System::host_name().unwrap_or_else(|| "archy".to_string());
        let raw_kernel = System::kernel_version().unwrap_or_else(|| "Linux".to_string());
        let kernel_version = raw_kernel
            .split('-')
            .next()
            .unwrap_or(&raw_kernel)
            .to_string();

        sys.refresh_all();
        thread::sleep(Duration::from_millis(120));

        let mut last_net_sample = Instant::now();

        loop {
            let cycle_start = Instant::now();
            let config = config_lock.lock().map(|c| c.clone()).unwrap_or_default();

            if !config.paused {
                let elapsed_secs = last_net_sample.elapsed().as_secs_f64().max(0.05);
                last_net_sample = Instant::now();

                sys.refresh_cpu_all();
                sys.refresh_memory();
                sys.refresh_processes_specifics(
                    ProcessesToUpdate::All,
                    true,
                    ProcessRefreshKind::new().with_cpu().with_memory(),
                );
                disks.refresh();
                networks.refresh();
                components.refresh();

                #[cfg(target_os = "linux")]
                let (memory_total, memory_available, memory_free, memory_cached_buffers) =
                    read_linux_true_memory().unwrap_or_else(|| {
                        (
                            sys.total_memory(),
                            sys.available_memory(),
                            sys.free_memory(),
                            0,
                        )
                    });

                #[cfg(not(target_os = "linux"))]
                let (memory_total, memory_available, memory_free, memory_cached_buffers) = (
                    sys.total_memory(),
                    sys.available_memory(),
                    sys.free_memory(),
                    0,
                );

                let memory_used = memory_total.saturating_sub(memory_available);
                let memory_usage = if memory_total > 0 {
                    memory_used as f64 / memory_total as f64
                } else {
                    0.0
                };

                let swap_total = sys.total_swap();
                let swap_used = sys.used_swap();
                let zram = read_linux_zram(swap_used);

                let mut seen_mounts = HashSet::new();
                let mut disk_total = 0u64;
                let mut disk_avail = 0u64;
                for disk in disks.list() {
                    let dev = disk.name().to_string_lossy().into_owned();
                    if seen_mounts.insert(dev) {
                        disk_total = disk_total.saturating_add(disk.total_space());
                        disk_avail = disk_avail.saturating_add(disk.available_space());
                    }
                }
                let disk_used = disk_total.saturating_sub(disk_avail);

                let cpu_brand = sys
                    .cpus()
                    .first()
                    .map(|c| shorten_cpu_brand(c.brand()))
                    .unwrap_or_else(|| "CPU".to_string());
                let cpu_usage: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
                let num_cpus = cpu_usage.len().max(1) as f32;
                let global_cpu = cpu_usage.iter().sum::<f32>() / num_cpus;
                let cpu_temp_c = read_cpu_temp(&components);

                let mut rx_delta = 0u64;
                let mut tx_delta = 0u64;
                let mut net_total_rx = 0u64;
                let mut net_total_tx = 0u64;
                for (iface, data) in networks.list() {
                    if iface == "lo" {
                        continue;
                    }
                    rx_delta = rx_delta.saturating_add(data.received());
                    tx_delta = tx_delta.saturating_add(data.transmitted());
                    net_total_rx = net_total_rx.saturating_add(data.total_received());
                    net_total_tx = net_total_tx.saturating_add(data.total_transmitted());
                }
                let net_rx_bytes_per_sec = (rx_delta as f64 / elapsed_secs) as u64;
                let net_tx_bytes_per_sec = (tx_delta as f64 / elapsed_secs) as u64;

                let gpus = read_linux_gpus(&components);
                let (battery_pct, battery_status) = read_linux_battery();
                let uptime_secs = System::uptime();
                let l = System::load_average();
                let load_avg = (l.one, l.five, l.fifteen);

                let mut individual_procs: Vec<ProcessEntry> = Vec::new();
                for (&pid, proc_info) in sys.processes() {
                    if proc_info.thread_kind().is_some() {
                        continue;
                    }
                    let raw_pid = pid.as_u32();
                    let rss = proc_info.memory();
                    let vmem = proc_info.virtual_memory();
                    if rss == 0 && vmem == 0 {
                        continue;
                    }

                    let display_memory = match config.memory_mode {
                        MemoryCalculationMode::FastRss => rss,
                        MemoryCalculationMode::RealTruePss => {
                            read_linux_pss_bytes(raw_pid).unwrap_or(rss)
                        }
                    };

                    individual_procs.push(ProcessEntry {
                        pid: raw_pid,
                        name: proc_info.name().to_string_lossy().into_owned(),
                        cpu_usage: proc_info.cpu_usage(),
                        display_memory,
                        rss_memory: rss,
                        virtual_memory: vmem,
                        process_count: 1,
                    });
                }

                let processes = if config.group_by_app {
                    let mut grouped: HashMap<String, ProcessEntry> = HashMap::new();
                    for entry in individual_procs {
                        grouped
                            .entry(entry.name.clone())
                            .and_modify(|agg| {
                                if entry.pid < agg.pid {
                                    agg.pid = entry.pid;
                                }
                                agg.cpu_usage += entry.cpu_usage;
                                agg.display_memory =
                                    agg.display_memory.saturating_add(entry.display_memory);
                                agg.rss_memory = agg.rss_memory.saturating_add(entry.rss_memory);
                                agg.virtual_memory =
                                    agg.virtual_memory.saturating_add(entry.virtual_memory);
                                agg.process_count += 1;
                            })
                            .or_insert(entry);
                    }
                    grouped.into_values().collect()
                } else {
                    individual_procs
                };

                tick_counter = tick_counter.wrapping_add(1);

                if let Ok(mut guard) = metrics_lock.lock() {
                    *guard = SystemMetrics {
                        tick: tick_counter,
                        host_name: host_name.clone(),
                        kernel_version: kernel_version.clone(),
                        uptime_secs,
                        load_avg,
                        cpu_brand,
                        cpu_usage,
                        global_cpu,
                        cpu_temp_c,
                        memory_usage,
                        memory_used,
                        memory_available,
                        memory_total,
                        memory_free,
                        memory_cached_buffers,
                        swap_used,
                        swap_total,
                        zram,
                        disk_used,
                        disk_total,
                        gpus,
                        net_rx_bytes_per_sec,
                        net_tx_bytes_per_sec,
                        net_total_rx,
                        net_total_tx,
                        battery_pct,
                        battery_status,
                        processes,
                    };
                }
            }

            let target_ms = config.refresh_ms;
            while cycle_start.elapsed() < Duration::from_millis(target_ms) {
                thread::sleep(Duration::from_millis(50));
                if let Ok(latest_cfg) = config_lock.lock() {
                    if latest_cfg.memory_mode != config.memory_mode
                        || latest_cfg.group_by_app != config.group_by_app
                        || latest_cfg.paused != config.paused
                        || latest_cfg.refresh_ms < target_ms
                    {
                        break;
                    }
                }
            }
        }
    });
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1}G", b / GB)
    } else if b >= 100.0 * MB {
        format!("{:.0}M", b / MB)
    } else if b >= MB {
        format!("{:.1}M", b / MB)
    } else if b >= KB {
        format!("{:.0}K", b / KB)
    } else {
        format!("{}B", bytes)
    }
}

pub fn format_rate(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {:02}m", hours, mins)
    } else {
        format!("{}m {:02}s", mins, secs % 60)
    }
}
