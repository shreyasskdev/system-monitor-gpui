mod header;
mod processes;
mod scroller;
mod settings;
mod telemetry;

pub use scroller::BouncyScroller;

use crate::state::{MemoryCalculationMode, MonitorConfig, SortColumn, SystemMetrics};
use crate::theme::ThemePreset;
use gpui_kit::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dashboard,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LayoutPreset {
    SidebarSplit,
    QuadGrid,
    TopRibbon,
}

impl LayoutPreset {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SidebarSplit => "Sidebar Split",
            Self::QuadGrid => "2x2 Quad Grid",
            Self::TopRibbon => "4-Card Ribbon",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessFilter {
    All,
    HighCpu,
    HighMem,
}

pub struct LayoutSettings {
    pub preset: LayoutPreset,
    pub show_cpu: bool,
    pub show_memory: bool,
    pub show_gpu: bool,
    pub show_network: bool,
    pub cpu_columns: usize,
    pub compact_rows: bool,
    pub drawer_open: bool,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            preset: LayoutPreset::SidebarSplit, // Default is Sidebar Split!
            show_cpu: true,
            show_memory: true,
            show_gpu: true,
            show_network: true,
            cpu_columns: 2,
            compact_rows: false,
            drawer_open: false,
        }
    }
}

pub struct SystemMonitorView {
    pub(crate) metrics_lock: Arc<Mutex<SystemMetrics>>,
    pub(crate) config_lock: Arc<Mutex<MonitorConfig>>,
    pub(crate) snapshot: SystemMetrics,
    pub(crate) config: MonitorConfig,
    // Delta-time smooth animation values
    pub(crate) smooth_cpu: Vec<f32>,
    pub(crate) smooth_global_cpu: f32,
    pub(crate) smooth_mem: f64,
    pub(crate) smooth_swap: f64,
    pub(crate) smooth_disk: f64,
    pub(crate) smooth_gpus: Vec<f32>,
    pub(crate) smooth_rx_ratio: f64,
    pub(crate) smooth_tx_ratio: f64,
    pub(crate) rx_history: Vec<f32>,
    pub(crate) tx_history: Vec<f32>,
    // iOS Bouncy Scrollers
    pub(crate) process_scroller: BouncyScroller,
    pub(crate) sidebar_scroller: BouncyScroller,
    pub(crate) settings_scroller: BouncyScroller,
    pub(crate) focus_handle: FocusHandle,
    // Search & Selection state
    pub(crate) search_query: String,
    pub(crate) search_active: bool,
    pub(crate) proc_filter: ProcessFilter,
    pub(crate) selected_pid: Option<u32>,
    pub(crate) status_banner: Option<String>,
    // Layout & Theme
    pub(crate) layout: LayoutSettings,
    pub(crate) active_tab: ActiveTab,
    pub(crate) theme: ThemePreset,
    pub(crate) sort_col: SortColumn,
    pub(crate) sort_desc: bool,
    pub(crate) viewport_width: f32,
}

impl SystemMonitorView {
    pub fn new(
        metrics_lock: Arc<Mutex<SystemMetrics>>,
        config_lock: Arc<Mutex<MonitorConfig>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_snapshot = metrics_lock.lock().map(|m| m.clone()).unwrap_or_default();
        let initial_config = config_lock.lock().map(|c| c.clone()).unwrap_or_default();
        let smooth_cpu = initial_snapshot.cpu_usage.clone();
        let smooth_gpus = initial_snapshot.gpus.iter().map(|g| g.usage_pct).collect();
        let focus_handle = cx.focus_handle();

        let weak_this = cx.weak_entity();
        let mut async_cx = cx.to_async();
        cx.foreground_executor()
            .spawn(async move {
                let mut last_frame = Instant::now();
                loop {
                    async_cx
                        .background_executor()
                        .timer(Duration::from_millis(16))
                        .await;

                    let now = Instant::now();
                    let dt = now
                        .duration_since(last_frame)
                        .as_secs_f32()
                        .clamp(0.005, 0.08);
                    last_frame = now;

                    let alpha = (1.0 - (-12.0 * dt).exp()) as f64;
                    let alpha_f32 = alpha as f32;

                    let should_continue = weak_this.update(&mut async_cx, |view, cx| {
                        let mut needs_redraw = false;

                        if let Ok(latest) = view.metrics_lock.try_lock() {
                            if latest.tick != view.snapshot.tick {
                                view.snapshot = latest.clone();

                                let rx_pt = if view.snapshot.net_rx_bytes_per_sec == 0 {
                                    0.04
                                } else {
                                    (((view.snapshot.net_rx_bytes_per_sec as f64 + 1.0).log10()
                                        / 7.72)
                                        .clamp(0.08, 1.0))
                                        as f32
                                };
                                let tx_pt = if view.snapshot.net_tx_bytes_per_sec == 0 {
                                    0.04
                                } else {
                                    (((view.snapshot.net_tx_bytes_per_sec as f64 + 1.0).log10()
                                        / 7.72)
                                        .clamp(0.08, 1.0))
                                        as f32
                                };

                                if view.rx_history.len() >= 32 {
                                    view.rx_history.remove(0);
                                }
                                view.rx_history.push(rx_pt);

                                if view.tx_history.len() >= 32 {
                                    view.tx_history.remove(0);
                                }
                                view.tx_history.push(tx_pt);

                                needs_redraw = true;
                            }
                        }

                        if view.process_scroller.step_physics(dt) {
                            needs_redraw = true;
                        }
                        if view.sidebar_scroller.step_physics(dt) {
                            needs_redraw = true;
                        }
                        if view.settings_scroller.step_physics(dt) {
                            needs_redraw = true;
                        }

                        if view.smooth_cpu.len() != view.snapshot.cpu_usage.len() {
                            view.smooth_cpu = view.snapshot.cpu_usage.clone();
                            needs_redraw = true;
                        } else {
                            for (smooth, &target) in view
                                .smooth_cpu
                                .iter_mut()
                                .zip(view.snapshot.cpu_usage.iter())
                            {
                                let diff = target - *smooth;
                                if diff.abs() > 0.15 {
                                    *smooth += diff * alpha_f32;
                                    needs_redraw = true;
                                } else {
                                    *smooth = target;
                                }
                            }
                        }

                        let g_diff = view.snapshot.global_cpu - view.smooth_global_cpu;
                        if g_diff.abs() > 0.1 {
                            view.smooth_global_cpu += g_diff * alpha_f32;
                            needs_redraw = true;
                        }

                        let m_diff = view.snapshot.memory_usage - view.smooth_mem;
                        if m_diff.abs() > 0.001 {
                            view.smooth_mem += m_diff * alpha;
                            needs_redraw = true;
                        }

                        let target_swap = if view.snapshot.swap_total > 0 {
                            view.snapshot.swap_used as f64 / view.snapshot.swap_total as f64
                        } else {
                            0.0
                        };
                        if (target_swap - view.smooth_swap).abs() > 0.001 {
                            view.smooth_swap += (target_swap - view.smooth_swap) * alpha;
                            needs_redraw = true;
                        }

                        let target_disk = if view.snapshot.disk_total > 0 {
                            view.snapshot.disk_used as f64 / view.snapshot.disk_total as f64
                        } else {
                            0.0
                        };
                        if (target_disk - view.smooth_disk).abs() > 0.001 {
                            view.smooth_disk += (target_disk - view.smooth_disk) * alpha;
                            needs_redraw = true;
                        }

                        if view.smooth_gpus.len() != view.snapshot.gpus.len() {
                            view.smooth_gpus =
                                view.snapshot.gpus.iter().map(|g| g.usage_pct).collect();
                            needs_redraw = true;
                        } else {
                            for (sg, gpu) in
                                view.smooth_gpus.iter_mut().zip(view.snapshot.gpus.iter())
                            {
                                let d = gpu.usage_pct - *sg;
                                if d.abs() > 0.15 {
                                    *sg += d * alpha_f32;
                                    needs_redraw = true;
                                } else {
                                    *sg = gpu.usage_pct;
                                }
                            }
                        }

                        let log_ratio = |bps: u64| -> f64 {
                            if bps == 0 {
                                0.0
                            } else {
                                ((bps as f64 + 1.0).log10() / 7.72).clamp(0.03, 1.0)
                            }
                        };

                        let target_rx = log_ratio(view.snapshot.net_rx_bytes_per_sec);
                        let target_tx = log_ratio(view.snapshot.net_tx_bytes_per_sec);

                        if (target_rx - view.smooth_rx_ratio).abs() > 0.0005 {
                            view.smooth_rx_ratio += (target_rx - view.smooth_rx_ratio) * alpha;
                            needs_redraw = true;
                        } else {
                            view.smooth_rx_ratio = target_rx;
                        }

                        if (target_tx - view.smooth_tx_ratio).abs() > 0.0005 {
                            view.smooth_tx_ratio += (target_tx - view.smooth_tx_ratio) * alpha;
                            needs_redraw = true;
                        } else {
                            view.smooth_tx_ratio = target_tx;
                        }

                        if needs_redraw {
                            cx.notify();
                        }
                    });

                    if should_continue.is_err() {
                        break;
                    }
                }
            })
            .detach();

        Self {
            metrics_lock,
            config_lock,
            snapshot: initial_snapshot,
            config: initial_config,
            smooth_cpu,
            smooth_global_cpu: 0.0,
            smooth_mem: 0.0,
            smooth_swap: 0.0,
            smooth_disk: 0.0,
            smooth_gpus,
            smooth_rx_ratio: 0.0,
            smooth_tx_ratio: 0.0,
            rx_history: vec![0.04; 32],
            tx_history: vec![0.04; 32],
            process_scroller: BouncyScroller::new(),
            sidebar_scroller: BouncyScroller::new(),
            settings_scroller: BouncyScroller::new(),
            focus_handle,
            search_query: String::new(),
            search_active: false,
            proc_filter: ProcessFilter::All,
            selected_pid: None,
            status_banner: None,
            layout: LayoutSettings::default(),
            active_tab: ActiveTab::Dashboard,
            theme: ThemePreset::ZedAyuMirage,
            sort_col: SortColumn::Memory,
            sort_desc: true,
            viewport_width: 1240.0,
        }
    }

    pub(crate) fn set_refresh_rate(&mut self, ms: u64, cx: &mut Context<Self>) {
        self.config.refresh_ms = ms;
        if let Ok(mut guard) = self.config_lock.lock() {
            guard.refresh_ms = ms;
        }
        cx.notify();
    }

    pub(crate) fn set_memory_mode(&mut self, mode: MemoryCalculationMode, cx: &mut Context<Self>) {
        self.config.memory_mode = mode;
        if let Ok(mut guard) = self.config_lock.lock() {
            guard.memory_mode = mode;
        }
        cx.notify();
    }

    pub(crate) fn toggle_group_by_app(&mut self, cx: &mut Context<Self>) {
        self.config.group_by_app = !self.config.group_by_app;
        if let Ok(mut guard) = self.config_lock.lock() {
            guard.group_by_app = self.config.group_by_app;
        }
        cx.notify();
    }

    pub(crate) fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.config.paused = !self.config.paused;
        if let Ok(mut guard) = self.config_lock.lock() {
            guard.paused = self.config.paused;
        }
        cx.notify();
    }

    pub(crate) fn toggle_sort(&mut self, col: SortColumn, cx: &mut Context<Self>) {
        if self.sort_col == col {
            self.sort_desc = !self.sort_desc;
        } else {
            self.sort_col = col;
            self.sort_desc = true;
        }
        cx.notify();
    }

    pub(crate) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        if self.search_active {
            match key {
                "backspace" => {
                    self.search_query.pop();
                    cx.notify();
                }
                "escape" | "enter" => {
                    if key == "escape" {
                        self.search_query.clear();
                    }
                    self.search_active = false;
                    cx.notify();
                }
                "space" => {
                    self.search_query.push(' ');
                    cx.notify();
                }
                _ => {
                    if !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt {
                        if let Some(ch) = event.keystroke.key_char.as_ref() {
                            self.search_query.push_str(ch);
                            cx.notify();
                        } else if key.len() == 1 {
                            self.search_query.push_str(key);
                            cx.notify();
                        }
                    }
                }
            }
        } else if key == "/" {
            self.search_active = true;
            self.active_tab = ActiveTab::Dashboard;
            cx.notify();
        } else if key == "space" {
            self.toggle_pause(cx);
        }
    }
}

impl Render for SystemMonitorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.viewport_width = f32::from(window.viewport_size().width);
        let colors = self.theme.colors();

        let body = match self.active_tab {
            ActiveTab::Dashboard => {
                if self.layout.preset == LayoutPreset::SidebarSplit {
                    div()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .flex()
                        .p_3()
                        .gap_3()
                        .child(self.render_telemetry_deck(&colors, cx))
                        .child(self.render_process_table(&colors, cx))
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .p_3()
                        .gap_3()
                        .child(self.render_telemetry_deck(&colors, cx))
                        .child(self.render_process_table(&colors, cx))
                        .into_any_element()
                }
            }
            ActiveTab::Settings => self.render_settings_view(&colors, cx).into_any_element(),
        };

        let status_right = self.status_banner.clone().unwrap_or_else(|| {
            "Shortcuts: '/' Search | 'Space' Pause | Click any process to inspect/kill".to_string()
        });

        div()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(colors.bg_app)
            .text_color(colors.text)
            .child(self.render_top_bar(&colors, cx))
            .children(
                if self.layout.drawer_open && self.active_tab == ActiveTab::Dashboard {
                    Some(self.render_layout_editor_bar(&colors, cx))
                } else {
                    None
                },
            )
            .child(body)
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_1()
                    .bg(colors.bg_panel)
                    .border_t_1()
                    .border_color(colors.border)
                    .text_xs()
                    .text_color(colors.text_muted)
                    .child(format!(
                        "Apps/Procs: {} | GPUs: {} | Cores: {} | Sample #{}",
                        self.snapshot.processes.len(),
                        self.snapshot.gpus.len(),
                        self.snapshot.cpu_usage.len(),
                        self.snapshot.tick
                    ))
                    .child(status_right),
            )
    }
}
