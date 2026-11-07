use super::{LayoutPreset, SystemMonitorView};
use crate::state::{format_bytes, format_rate, format_uptime};
use crate::theme::ThemeColors;
use gpui_kit::*;

impl SystemMonitorView {
    pub(crate) fn render_meter_bar(&self, fraction: f64, color: Rgba, colors: &ThemeColors) -> Div {
        let clamped = fraction.clamp(0.0, 1.0) as f32;
        div()
            .h(px(5.0))
            .w_full()
            .bg(colors.bg_app)
            .rounded(px(3.0))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .child(
                div()
                    .h_full()
                    .w(relative(clamped))
                    .bg(color)
                    .rounded(px(2.0)),
            )
    }

    fn render_sparkline(&self, history: &[f32], color: Rgba, colors: &ThemeColors) -> Div {
        let mut row = div()
            .h(px(14.0))
            .w_full()
            .px_1()
            .py(px(1.5))
            .bg(colors.bg_app)
            .rounded(px(3.0))
            .border_1()
            .border_color(colors.border)
            .flex()
            .items_end()
            .gap(px(1.5));

        for &val in history {
            let h_frac = val.clamp(0.08, 1.0);
            row = row.child(
                div()
                    .flex_1()
                    .h(relative(h_frac))
                    .rounded(px(1.0))
                    .bg(if val > 0.05 { color } else { colors.border }),
            );
        }
        row
    }

    pub(crate) fn render_cpu_card(&self, colors: &ThemeColors) -> Div {
        let cols = self.layout.cpu_columns.max(1);
        let total_cores = self.smooth_cpu.len();
        let per_col = (total_cores + cols - 1) / cols;

        let mut grid_row = div().flex().gap_2().w_full().min_w_0();

        for c in 0..cols {
            let start = c * per_col;
            let end = ((c + 1) * per_col).min(total_cores);
            if start >= total_cores {
                break;
            }
            let mut col_div = div().flex_1().min_w_0().flex().flex_col().gap(px(3.0));
            for idx in start..end {
                let usage = self.smooth_cpu[idx];
                let bar_color = if usage > 85.0 {
                    colors.danger
                } else if usage > 60.0 {
                    colors.warning
                } else {
                    colors.accent
                };

                col_div = col_div.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .min_w_0()
                        .child(
                            div()
                                .w(px(28.0))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_xs()
                                .text_color(colors.text_muted)
                                .child(format!("C{:02}", idx + 1)),
                        )
                        .child(div().flex_1().min_w_0().child(self.render_meter_bar(
                            (usage / 100.0) as f64,
                            bar_color,
                            colors,
                        )))
                        .child(
                            div()
                                .w(px(32.0))
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .text_xs()
                                .text_right()
                                .child(format!("{:.0}%", usage)),
                        ),
                );
            }
            grid_row = grid_row.child(col_div);
        }

        let temp_badge = self
            .snapshot
            .cpu_temp_c
            .map(|t| format!(" · {:.0}°C", t))
            .unwrap_or_default();

        div()
            .flex_shrink_0()
            .min_w_0()
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .bg(colors.bg_panel)
            .rounded(px(7.0))
            .border_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .pb_1()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .font_weight(FontWeight::BOLD)
                            .text_xs()
                            .child(format!("CPU · {}", self.snapshot.cpu_brand)),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_xs()
                            .text_color(colors.accent)
                            .child(format!("{:.1}%{}", self.smooth_global_cpu, temp_badge)),
                    ),
            )
            .child(grid_row)
    }

    pub(crate) fn render_memory_storage_card(&self, colors: &ThemeColors) -> Div {
        let usable_gib = self.snapshot.memory_total as f64 / (1024.0 * 1024.0 * 1024.0);
        let nominal_hw_gb = ((usable_gib / 4.0).round() as u64) * 4;

        let ram_subtitle = format!(
            "Avl {} · Cch {} · Fre {}",
            format_bytes(self.snapshot.memory_available),
            format_bytes(self.snapshot.memory_cached_buffers),
            format_bytes(self.snapshot.memory_free),
        );

        let (swap_title, swap_right) = if self.snapshot.zram.active {
            if let Some(ratio) = self.snapshot.zram.compression_ratio() {
                (
                    format!("zram ({})", self.snapshot.zram.algorithm),
                    format!(
                        "{}→{} ({:.1}x) / {}",
                        format_bytes(self.snapshot.zram.orig_data_bytes),
                        format_bytes(self.snapshot.zram.ram_used_bytes),
                        ratio,
                        format_bytes(self.snapshot.swap_total),
                    ),
                )
            } else {
                (
                    format!("zram ({})", self.snapshot.zram.algorithm),
                    format!(
                        "{} / {}",
                        format_bytes(self.snapshot.swap_used),
                        format_bytes(self.snapshot.swap_total),
                    ),
                )
            }
        } else {
            (
                "Swap".to_string(),
                format!(
                    "{} / {}",
                    format_bytes(self.snapshot.swap_used),
                    format_bytes(self.snapshot.swap_total),
                ),
            )
        };

        div()
            .flex_shrink_0()
            .min_w_0()
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .bg(colors.bg_panel)
            .rounded(px(7.0))
            .border_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .pb_1()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_xs()
                            .whitespace_nowrap()
                            .child(format!("MEM ({}G) & ZRAM", nominal_hw_gb)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .whitespace_nowrap()
                            .text_color(colors.success)
                            .child(format!(
                                "{:.0}% ({}/{})",
                                self.smooth_mem * 100.0,
                                format_bytes(self.snapshot.memory_used),
                                format_bytes(self.snapshot.memory_total)
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .text_xs()
                            .child(div().text_color(colors.text_muted).child("RAM"))
                            .child(div().whitespace_nowrap().child(ram_subtitle)),
                    )
                    .child(self.render_meter_bar(self.smooth_mem, colors.success, colors)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .text_xs()
                            .child(div().text_color(colors.text_muted).child(swap_title))
                            .child(
                                div()
                                    .whitespace_nowrap()
                                    .text_color(colors.warning)
                                    .child(swap_right),
                            ),
                    )
                    .child(self.render_meter_bar(self.smooth_swap, colors.warning, colors)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .text_xs()
                            .child(div().text_color(colors.text_muted).child("Disk"))
                            .child(div().whitespace_nowrap().child(format!(
                                "{} / {}",
                                format_bytes(self.snapshot.disk_used),
                                format_bytes(self.snapshot.disk_total),
                            ))),
                    )
                    .child(self.render_meter_bar(self.smooth_disk, colors.accent, colors)),
            )
    }

    pub(crate) fn render_gpu_card(&self, colors: &ThemeColors) -> Div {
        let bat_text = match self.snapshot.battery_pct {
            Some(pct) => format!("Bat {:.0}%", pct),
            None => self.snapshot.battery_status.clone(),
        };

        let mut gpu_sections = div().flex().flex_col().gap_1();

        for (idx, gpu) in self.snapshot.gpus.iter().enumerate() {
            let smooth_load = self.smooth_gpus.get(idx).copied().unwrap_or(gpu.usage_pct);
            let vram_ratio = if gpu.vram_total > 0 {
                gpu.vram_used as f64 / gpu.vram_total as f64
            } else {
                0.0
            };

            let mut meta_parts: Vec<String> = Vec::new();
            meta_parts.push(format!("{:.0}%", smooth_load));
            if let Some(freq) = gpu.freq_mhz {
                meta_parts.push(format!("{}MHz", freq));
            }
            if let Some(temp) = gpu.temp_c {
                meta_parts.push(format!("{:.0}°C", temp));
            }

            let vram_label = if gpu.vram_total > 0 {
                format!(
                    "VRAM {} / {}",
                    format_bytes(gpu.vram_used),
                    format_bytes(gpu.vram_total)
                )
            } else {
                "VRAM Shared RAM".to_string()
            };

            gpu_sections = gpu_sections.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .p_1()
                    .px_2()
                    .rounded(px(5.0))
                    .bg(colors.bg_app)
                    .border_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .text_xs()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child(format!("GPU{} · {}", idx + 1, gpu.name)),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .whitespace_nowrap()
                                    .text_color(colors.warning)
                                    .child(meta_parts.join(" · ")),
                            ),
                    )
                    .child(self.render_meter_bar(
                        (smooth_load / 100.0) as f64,
                        colors.warning,
                        colors,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_xs()
                            .child(div().text_color(colors.text_muted).child(vram_label))
                            .children(if gpu.vram_total > 0 {
                                Some(div().w(px(75.0)).child(self.render_meter_bar(
                                    vram_ratio,
                                    colors.accent,
                                    colors,
                                )))
                            } else {
                                None
                            }),
                    ),
            );
        }

        div()
            .flex_shrink_0()
            .min_w_0()
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .bg(colors.bg_panel)
            .rounded(px(7.0))
            .border_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb_1()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_xs()
                            .child(format!("GPUS ({})", self.snapshot.gpus.len())),
                    )
                    .child(div().text_xs().text_color(colors.success).child(bat_text)),
            )
            .child(gpu_sections)
    }

    pub(crate) fn render_network_card(&self, colors: &ThemeColors) -> Div {
        div()
            .flex_shrink_0()
            .min_w_0()
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .bg(colors.bg_panel)
            .rounded(px(7.0))
            .border_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb_1()
                    .border_b_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_xs()
                            .child("NET & HOST"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.accent)
                            .child(format!("Up {}", format_uptime(self.snapshot.uptime_secs))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_xs()
                            .child(div().text_color(colors.text_muted).child(format!(
                                "↓ RX ({})",
                                format_bytes(self.snapshot.net_total_rx)
                            )))
                            .child(
                                div()
                                    .text_color(colors.success)
                                    .font_weight(FontWeight::BOLD)
                                    .child(format_rate(self.snapshot.net_rx_bytes_per_sec)),
                            ),
                    )
                    .child(self.render_meter_bar(self.smooth_rx_ratio, colors.success, colors))
                    .child(self.render_sparkline(&self.rx_history, colors.success, colors)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_xs()
                            .child(div().text_color(colors.text_muted).child(format!(
                                "↑ TX ({})",
                                format_bytes(self.snapshot.net_total_tx)
                            )))
                            .child(
                                div()
                                    .text_color(colors.accent)
                                    .font_weight(FontWeight::BOLD)
                                    .child(format_rate(self.snapshot.net_tx_bytes_per_sec)),
                            ),
                    )
                    .child(self.render_meter_bar(self.smooth_tx_ratio, colors.accent, colors))
                    .child(self.render_sparkline(&self.tx_history, colors.accent, colors)),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(colors.text_muted).child(format!(
                        "{} ({})",
                        self.snapshot.host_name, self.snapshot.kernel_version
                    )))
                    .child(div().child(format!(
                        "Ld {:.2} {:.2} {:.2}",
                        self.snapshot.load_avg.0,
                        self.snapshot.load_avg.1,
                        self.snapshot.load_avg.2
                    ))),
            )
    }

    pub(crate) fn render_telemetry_deck(
        &self,
        colors: &ThemeColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut cards: Vec<Div> = Vec::new();
        if self.layout.show_cpu {
            cards.push(self.render_cpu_card(colors));
        }
        if self.layout.show_memory {
            cards.push(self.render_memory_storage_card(colors));
        }
        if self.layout.show_gpu {
            cards.push(self.render_gpu_card(colors));
        }
        if self.layout.show_network {
            cards.push(self.render_network_card(colors));
        }

        if cards.is_empty() {
            return div().into_any_element();
        }

        match self.layout.preset {
            LayoutPreset::TopRibbon => {
                let mut row = div().flex_shrink_0().flex().gap_2().w_full().min_w_0();
                for card in cards {
                    row = row.child(card.flex_1());
                }
                row.into_any_element()
            }
            LayoutPreset::SidebarSplit => {
                let sidebar_w = if self.viewport_width < 960.0 {
                    px(305.0)
                } else {
                    px(335.0)
                };
                let mut inner_col = div().w_full().flex().flex_col().gap_2();
                for card in cards {
                    inner_col = inner_col.child(card);
                }
                inner_col = self.sidebar_scroller.apply_rubber_band(inner_col);

                div()
                    .id("sidebar-telemetry-scroll")
                    .w(sidebar_w)
                    .h_full()
                    .min_h_0()
                    .flex_shrink_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.sidebar_scroller.handle)
                    .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                        let dy = f32::from(event.delta.pixel_delta(px(22.0)).y);
                        this.sidebar_scroller.on_wheel(dy);
                        cx.notify();
                    }))
                    .child(inner_col)
                    .into_any_element()
            }
            LayoutPreset::QuadGrid => {
                let mut container = div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .min_w_0();
                let mut iter = cards.into_iter();
                while let Some(first) = iter.next() {
                    let mut row = div()
                        .flex()
                        .gap_2()
                        .w_full()
                        .min_w_0()
                        .child(first.flex_1());
                    if let Some(second) = iter.next() {
                        row = row.child(second.flex_1());
                    }
                    container = container.child(row);
                }
                container.into_any_element()
            }
        }
    }
}
