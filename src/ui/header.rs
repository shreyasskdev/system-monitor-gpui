use super::{ActiveTab, LayoutPreset, SystemMonitorView};
use crate::state::MemoryCalculationMode;
use crate::theme::{ThemeColors, ThemePreset};
use gpui_kit::*;

impl SystemMonitorView {
    pub(crate) fn render_top_bar(&self, colors: &ThemeColors, cx: &mut Context<Self>) -> Div {
        let is_dashboard = self.active_tab == ActiveTab::Dashboard;
        let is_settings = self.active_tab == ActiveTab::Settings;

        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .py_1()
            .bg(colors.bg_panel)
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(
                                if self.config.paused {
                                    colors.warning
                                } else {
                                    colors.accent
                                },
                            ))
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .child("MONITOR"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .bg(colors.bg_app)
                            .p(px(2.0))
                            .rounded(px(5.0))
                            .border_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .id("tab-dashboard")
                                    .h(px(20.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .cursor_pointer()
                                    .bg(if is_dashboard {
                                        colors.bg_elevated
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .text_color(if is_dashboard {
                                        colors.text
                                    } else {
                                        colors.text_muted
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.active_tab = ActiveTab::Dashboard;
                                        cx.notify();
                                    }))
                                    .child("Dashboard"),
                            )
                            .child(
                                div()
                                    .id("tab-settings")
                                    .h(px(20.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .cursor_pointer()
                                    .bg(if is_settings {
                                        colors.bg_elevated
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .hover(|s| s.bg(colors.bg_hover))
                                    .text_color(if is_settings {
                                        colors.text
                                    } else {
                                        colors.text_muted
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.active_tab = ActiveTab::Settings;
                                        cx.notify();
                                    }))
                                    .child("Settings"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_shrink_0()
                    .child(
                        div()
                            .id("pause-btn")
                            .h(px(22.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.0))
                            .bg(if self.config.paused {
                                colors.warning
                            } else {
                                colors.bg_app
                            })
                            .border_1()
                            .border_color(colors.border)
                            .hover(|s| s.bg(colors.bg_hover))
                            .text_xs()
                            .whitespace_nowrap()
                            .text_color(if self.config.paused {
                                colors.bg_app
                            } else {
                                colors.text
                            })
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_pause(cx);
                            }))
                            .child(if self.config.paused {
                                "Resume"
                            } else {
                                "Pause"
                            }),
                    )
                    .child(
                        div()
                            .id("layout-drawer-toggle")
                            .h(px(22.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.0))
                            .bg(if self.layout.drawer_open {
                                colors.bg_elevated
                            } else {
                                colors.bg_app
                            })
                            .border_1()
                            .border_color(if self.layout.drawer_open {
                                colors.accent
                            } else {
                                colors.border
                            })
                            .hover(|s| s.bg(colors.bg_hover))
                            .text_xs()
                            .whitespace_nowrap()
                            .text_color(colors.accent)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.layout.drawer_open = !this.layout.drawer_open;
                                this.active_tab = ActiveTab::Dashboard;
                                cx.notify();
                            }))
                            .child("Layout"),
                    )
                    .child(
                        div()
                            .id("quick-mem-mode-toggle")
                            .h(px(22.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.0))
                            .bg(colors.bg_app)
                            .border_1()
                            .border_color(colors.border)
                            .hover(|s| s.bg(colors.bg_hover))
                            .text_xs()
                            .whitespace_nowrap()
                            .text_color(colors.text)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                let next = match this.config.memory_mode {
                                    MemoryCalculationMode::FastRss => {
                                        MemoryCalculationMode::RealTruePss
                                    }
                                    MemoryCalculationMode::RealTruePss => {
                                        MemoryCalculationMode::FastRss
                                    }
                                };
                                this.set_memory_mode(next, cx);
                            }))
                            .child(format!(
                                "{} {}s",
                                match self.config.memory_mode {
                                    MemoryCalculationMode::FastRss => "RSS",
                                    MemoryCalculationMode::RealTruePss => "PSS",
                                },
                                if self.config.refresh_ms >= 1000 {
                                    format!("{}", self.config.refresh_ms / 1000)
                                } else {
                                    format!("0.{}", self.config.refresh_ms / 100)
                                }
                            )),
                    )
                    .child(
                        div()
                            .id("quick-theme-cycle")
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
                            .text_xs()
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                let idx = ThemePreset::ALL
                                    .iter()
                                    .position(|&t| t == this.theme)
                                    .unwrap_or(0);
                                this.theme = ThemePreset::ALL[(idx + 1) % ThemePreset::ALL.len()];
                                cx.notify();
                            }))
                            .child(self.theme.short_name()),
                    ),
            )
    }

    pub(crate) fn render_layout_editor_bar(
        &self,
        colors: &ThemeColors,
        cx: &mut Context<Self>,
    ) -> Div {
        let presets = [
            LayoutPreset::SidebarSplit,
            LayoutPreset::QuadGrid,
            LayoutPreset::TopRibbon,
        ];

        let mut preset_pills = div().flex().items_center().gap_1();
        for p in presets {
            let active = self.layout.preset == p;
            preset_pills = preset_pills.child(
                div()
                    .id(SharedString::from(format!("preset-{}", p.name())))
                    .h(px(20.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if active { colors.accent } else { colors.border })
                    .bg(if active {
                        colors.bg_elevated
                    } else {
                        colors.bg_app
                    })
                    .hover(|s| s.bg(colors.bg_hover))
                    .text_xs()
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.layout.preset = p;
                        cx.notify();
                    }))
                    .child(p.name()),
            );
        }

        let toggle_pill = |id: &str, label: &str, active: bool| {
            div()
                .id(SharedString::from(id.to_string()))
                .h(px(20.0))
                .px_2()
                .flex()
                .items_center()
                .rounded(px(4.0))
                .border_1()
                .border_color(if active {
                    colors.success
                } else {
                    colors.border
                })
                .bg(if active {
                    colors.bg_elevated
                } else {
                    colors.bg_app
                })
                .hover(|s| s.bg(colors.bg_hover))
                .text_xs()
                .whitespace_nowrap()
                .text_color(if active {
                    colors.text
                } else {
                    colors.text_muted
                })
                .cursor_pointer()
                .child(label.to_string())
        };

        div()
            .flex_shrink_0()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .py_1()
            .bg(colors.bg_elevated)
            .border_b_1()
            .border_color(colors.border)
            .child(preset_pills)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        toggle_pill("tog-cpu", "CPU", self.layout.show_cpu).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.layout.show_cpu = !this.layout.show_cpu;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        toggle_pill("tog-mem", "MEM", self.layout.show_memory).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.layout.show_memory = !this.layout.show_memory;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        toggle_pill("tog-gpu", "GPU", self.layout.show_gpu).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.layout.show_gpu = !this.layout.show_gpu;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        toggle_pill("tog-net", "NET", self.layout.show_network).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.layout.show_network = !this.layout.show_network;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("cycle-cpu-cols")
                            .h(px(20.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(colors.border)
                            .bg(colors.bg_app)
                            .hover(|s| s.bg(colors.bg_hover))
                            .text_xs()
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.layout.cpu_columns = match this.layout.cpu_columns {
                                    1 => 2,
                                    2 => 4,
                                    _ => 1,
                                };
                                cx.notify();
                            }))
                            .child(format!("{} Col", self.layout.cpu_columns)),
                    )
                    .child(
                        toggle_pill("tog-compact", "Dense", self.layout.compact_rows).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.layout.compact_rows = !this.layout.compact_rows;
                                cx.notify();
                            }),
                        ),
                    ),
            )
    }
}
