use super::{LayoutPreset, ProcessFilter, SystemMonitorView};
use crate::state::{
    MemoryCalculationMode, ProcessEntry, SortColumn, format_bytes, send_process_signal,
};
use crate::theme::ThemeColors;
use gpui_kit::*;

impl SystemMonitorView {
    pub(crate) fn render_process_table(&self, colors: &ThemeColors, cx: &mut Context<Self>) -> Div {
        let table_width = if self.layout.preset == LayoutPreset::SidebarSplit {
            (self.viewport_width - 340.0).max(300.0)
        } else {
            self.viewport_width
        };
        let show_rss_col = table_width >= 520.0;
        let show_virt_col = table_width >= 640.0;

        let query_lower = self.search_query.trim().to_lowercase();

        let mut procs: Vec<ProcessEntry> = self
            .snapshot
            .processes
            .iter()
            .filter(|p| {
                if !query_lower.is_empty() {
                    let match_name = p.name.to_lowercase().contains(&query_lower);
                    let match_pid = p.pid.to_string().contains(&query_lower);
                    if !match_name && !match_pid {
                        return false;
                    }
                }
                match self.proc_filter {
                    ProcessFilter::All => true,
                    ProcessFilter::HighCpu => p.cpu_usage >= 5.0,
                    ProcessFilter::HighMem => p.display_memory >= 100 * 1024 * 1024,
                }
            })
            .cloned()
            .collect();

        procs.sort_by(|a, b| {
            let ord = match self.sort_col {
                SortColumn::Pid => a.pid.cmp(&b.pid),
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Cpu => a
                    .cpu_usage
                    .partial_cmp(&b.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortColumn::Memory => a.display_memory.cmp(&b.display_memory),
            };
            if self.sort_desc { ord.reverse() } else { ord }
        });

        let selected_entry = self
            .selected_pid
            .and_then(|pid| procs.iter().find(|p| p.pid == pid).cloned());

        let arrow = |col: SortColumn| -> &'static str {
            if self.sort_col == col {
                if self.sort_desc { " ▼" } else { " ▲" }
            } else {
                ""
            }
        };

        let is_true_pss = self.config.memory_mode == MemoryCalculationMode::RealTruePss;
        let mem_header_label = if is_true_pss {
            format!("PSS{}", arrow(SortColumn::Memory))
        } else {
            format!("RSS{}", arrow(SortColumn::Memory))
        };

        let row_py = if self.layout.compact_rows {
            px(2.0)
        } else {
            px(4.0)
        };

        let mut rows_container = div().w_full().flex().flex_col();

        for (idx, proc_item) in procs.iter().enumerate() {
            let is_selected = self.selected_pid == Some(proc_item.pid);
            let row_bg = if is_selected {
                colors.bg_elevated
            } else if idx % 2 == 0 {
                colors.bg_panel
            } else {
                colors.bg_app
            };

            let app_label = if self.config.group_by_app && proc_item.process_count > 1 {
                format!("{} ({})", proc_item.name, proc_item.process_count)
            } else {
                proc_item.name.clone()
            };

            let row_pid = proc_item.pid;

            rows_container = rows_container.child(
                div()
                    .id(SharedString::from(format!("proc-row-{}", row_pid)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py(row_py)
                    .bg(row_bg)
                    .hover(|s| s.bg(colors.bg_hover))
                    .border_b_1()
                    .border_color(if is_selected {
                        colors.accent
                    } else {
                        colors.border
                    })
                    .text_xs()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selected_pid == Some(row_pid) {
                            this.selected_pid = None;
                        } else {
                            this.selected_pid = Some(row_pid);
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(50.0))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_color(colors.text_muted)
                            .child(proc_item.pid.to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(100.0))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .font_weight(FontWeight::MEDIUM)
                            .child(app_label),
                    )
                    .child(
                        div()
                            .w(px(54.0))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_right()
                            .text_color(if proc_item.cpu_usage > 20.0 {
                                colors.warning
                            } else {
                                colors.text
                            })
                            .child(format!("{:.1}%", proc_item.cpu_usage)),
                    )
                    .child(
                        div()
                            .w(px(72.0))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_right()
                            .font_weight(FontWeight::BOLD)
                            .text_color(colors.accent)
                            .child(format_bytes(proc_item.display_memory)),
                    )
                    .children(if show_rss_col {
                        Some(
                            div()
                                .w(px(68.0))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_right()
                                .text_color(colors.text_muted)
                                .child(format_bytes(proc_item.rss_memory)),
                        )
                    } else {
                        None
                    })
                    .children(if show_virt_col {
                        Some(
                            div()
                                .w(px(68.0))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_right()
                                .text_color(colors.text_muted)
                                .child(format_bytes(proc_item.virtual_memory)),
                        )
                    } else {
                        None
                    }),
            );
        }

        rows_container = self.process_scroller.apply_rubber_band(rows_container);

        // Clean ASCII/Geometric Search Prompt (No missing emoji boxes!)
        let search_display = if self.search_query.is_empty() {
            if self.search_active {
                "/ Filter... (Esc)".to_string()
            } else {
                "/ Search ('/')".to_string()
            }
        } else {
            format!("/ {}", self.search_query)
        };

        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(colors.bg_panel)
            .rounded(px(7.0))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            // Compact Top Process Toolbar
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .bg(colors.bg_app)
                    .border_b_1()
                    .border_color(colors.border)
                    .text_xs()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id("proc-search-input")
                                    .w(px(145.0))
                                    .h(px(22.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .rounded(px(4.0))
                                    .bg(colors.bg_panel)
                                    .border_1()
                                    .border_color(if self.search_active {
                                        colors.accent
                                    } else {
                                        colors.border
                                    })
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .text_color(
                                        if self.search_query.is_empty() && !self.search_active {
                                            colors.text_muted
                                        } else {
                                            colors.text
                                        },
                                    )
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.search_active = true;
                                        window.focus(&this.focus_handle, cx);
                                        cx.notify();
                                    }))
                                    .child(search_display),
                            )
                            .children(if !self.search_query.is_empty() {
                                Some(
                                    div()
                                        .id("clear-search-btn")
                                        .h(px(22.0))
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .bg(colors.bg_elevated)
                                        .border_1()
                                        .border_color(colors.border)
                                        .hover(|s| s.bg(colors.bg_hover))
                                        .text_color(colors.danger)
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.search_query.clear();
                                            this.search_active = false;
                                            cx.notify();
                                        }))
                                        .child("×"),
                                )
                            } else {
                                None
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .flex_shrink_0()
                            .child(
                                div()
                                    .id("flt-all")
                                    .h(px(20.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded(px(4.0))
                                    .bg(if self.proc_filter == ProcessFilter::All {
                                        colors.bg_elevated
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .whitespace_nowrap()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.proc_filter = ProcessFilter::All;
                                        cx.notify();
                                    }))
                                    .child("All"),
                            )
                            .child(
                                div()
                                    .id("flt-cpu")
                                    .h(px(20.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded(px(4.0))
                                    .bg(if self.proc_filter == ProcessFilter::HighCpu {
                                        colors.bg_elevated
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .whitespace_nowrap()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.proc_filter = ProcessFilter::HighCpu;
                                        cx.notify();
                                    }))
                                    .child(">5% CPU"),
                            )
                            .child(
                                div()
                                    .id("flt-mem")
                                    .h(px(20.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded(px(4.0))
                                    .bg(if self.proc_filter == ProcessFilter::HighMem {
                                        colors.bg_elevated
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .whitespace_nowrap()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.proc_filter = ProcessFilter::HighMem;
                                        cx.notify();
                                    }))
                                    .child(">100M"),
                            )
                            .child(
                                div()
                                    .id("subbar-group-toggle")
                                    .h(px(20.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded(px(4.0))
                                    .bg(colors.bg_elevated)
                                    .border_1()
                                    .border_color(colors.border)
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .whitespace_nowrap()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_group_by_app(cx);
                                    }))
                                    .child(if self.config.group_by_app {
                                        "Grouped"
                                    } else {
                                        "PIDs"
                                    }),
                            )
                            .child(
                                div()
                                    .pl_1()
                                    .whitespace_nowrap()
                                    .text_color(colors.text_muted)
                                    .child(format!("#{}", procs.len())),
                            ),
                    ),
            )
            // Compact Sortable Column Headers
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .bg(colors.bg_elevated)
                    .border_b_1()
                    .border_color(colors.border)
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .child(
                        div()
                            .id("sort-pid")
                            .w(px(50.0))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_sort(SortColumn::Pid, cx);
                            }))
                            .child(format!("PID{}", arrow(SortColumn::Pid))),
                    )
                    .child(
                        div()
                            .id("sort-name")
                            .flex_1()
                            .min_w(px(100.0))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_sort(SortColumn::Name, cx);
                            }))
                            .child(format!("Process{}", arrow(SortColumn::Name))),
                    )
                    .child(
                        div()
                            .id("sort-cpu")
                            .w(px(54.0))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_right()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_sort(SortColumn::Cpu, cx);
                            }))
                            .child(format!("CPU{}", arrow(SortColumn::Cpu))),
                    )
                    .child(
                        div()
                            .id("sort-mem")
                            .w(px(72.0))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_right()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_sort(SortColumn::Memory, cx);
                            }))
                            .child(mem_header_label),
                    )
                    .children(if show_rss_col {
                        Some(
                            div()
                                .w(px(68.0))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_right()
                                .text_color(colors.text_muted)
                                .child("RSS"),
                        )
                    } else {
                        None
                    })
                    .children(if show_virt_col {
                        Some(
                            div()
                                .w(px(68.0))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_right()
                                .text_color(colors.text_muted)
                                .child("Virt"),
                        )
                    } else {
                        None
                    }),
            )
            // Scrollable Process List
            .child(
                div()
                    .id("process-table-scroll-viewport")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.process_scroller.handle)
                    .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                        let dy = f32::from(event.delta.pixel_delta(px(24.0)).y);
                        this.process_scroller.on_wheel(dy);
                        cx.notify();
                    }))
                    .child(rows_container),
            )
            // Compact Selected Process Inspector Bar
            .children(selected_entry.map(|sel| {
                let target_pid = sel.pid;
                let target_name = sel.name.clone();
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .bg(colors.bg_elevated)
                    .border_t_1()
                    .border_color(colors.accent)
                    .text_xs()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(colors.accent)
                                    .child(format!("{} ({})", sel.name, sel.pid)),
                            )
                            .child(div().text_color(colors.text_muted).child(format!(
                                "PSS {} · RSS {} · Virt {}",
                                format_bytes(sel.display_memory),
                                format_bytes(sel.rss_memory),
                                format_bytes(sel.virtual_memory)
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id("btn-sigterm")
                                    .px_2()
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(colors.warning)
                                    .text_color(colors.bg_app)
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let ok = send_process_signal(target_pid, false);
                                        this.status_banner = Some(if ok {
                                            format!("SIGTERM -> {} ({})", target_name, target_pid)
                                        } else {
                                            format!("SIGTERM failed ({})", target_pid)
                                        });
                                        cx.notify();
                                    }))
                                    .child("TERM"),
                            )
                            .child(
                                div()
                                    .id("btn-sigkill")
                                    .px_2()
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(colors.danger)
                                    .text_color(colors.bg_app)
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let name = sel.name.clone();
                                        let ok = send_process_signal(target_pid, true);
                                        this.status_banner = Some(if ok {
                                            format!("SIGKILL -> {} ({})", name, target_pid)
                                        } else {
                                            format!("SIGKILL failed ({})", target_pid)
                                        });
                                        cx.notify();
                                    }))
                                    .child("KILL"),
                            ),
                    )
            }))
    }
}
