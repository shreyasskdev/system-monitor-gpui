use super::SystemMonitorView;
use crate::state::MemoryCalculationMode;
use crate::theme::{ThemeColors, ThemePreset};
use gpui_kit::*;

impl SystemMonitorView {
    pub(crate) fn render_settings_view(
        &self,
        colors: &ThemeColors,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let frequencies: [(u64, &'static str); 4] = [
            (250, "250ms (Ultra)"),
            (500, "500ms (Fast)"),
            (1000, "1000ms (1s)"),
            (2000, "2000ms (2s Default)"),
        ];

        let mut freq_buttons = div().flex().flex_wrap().gap_2();
        for (ms, label) in frequencies {
            let active = self.config.refresh_ms == ms;
            freq_buttons = freq_buttons.child(
                div()
                    .id(SharedString::from(format!("freq-{}", ms)))
                    .px_3()
                    .py_2()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .border_1()
                    .border_color(if active { colors.accent } else { colors.border })
                    .bg(if active {
                        colors.bg_elevated
                    } else {
                        colors.bg_app
                    })
                    .hover(|s| s.bg(colors.bg_hover))
                    .text_xs()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_refresh_rate(ms, cx);
                    }))
                    .child(label),
            );
        }

        // Side-by-side horizontal row for Per-App Memory Accounting Strategy!
        let mut mode_cards = div().w_full().min_w_0().flex().gap_3();
        for mode in [
            MemoryCalculationMode::FastRss,
            MemoryCalculationMode::RealTruePss,
        ] {
            let active = self.config.memory_mode == mode;
            mode_cards = mode_cards.child(
                div()
                    .id(SharedString::from(format!("mode-{:?}", mode)))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .p_3()
                    .rounded(px(7.0))
                    .cursor_pointer()
                    .border_1()
                    .border_color(if active { colors.accent } else { colors.border })
                    .bg(if active {
                        colors.bg_elevated
                    } else {
                        colors.bg_app
                    })
                    .hover(|s| s.bg(colors.bg_hover))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_memory_mode(mode, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs()
                                    .child(mode.label()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .whitespace_nowrap()
                                    .text_color(if active {
                                        colors.accent
                                    } else {
                                        colors.text_muted
                                    })
                                    .child(if active { "ACTIVE" } else { "Select" }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.text_muted)
                            .child(mode.description()),
                    ),
            );
        }

        let mut theme_buttons = div().flex().flex_wrap().gap_2();
        for preset in ThemePreset::ALL {
            let active = self.theme == preset;
            theme_buttons = theme_buttons.child(
                div()
                    .id(SharedString::from(format!("theme-{}", preset.name())))
                    .px_3()
                    .py_2()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .border_1()
                    .border_color(if active { colors.accent } else { colors.border })
                    .bg(if active {
                        colors.bg_elevated
                    } else {
                        colors.bg_app
                    })
                    .hover(|s| s.bg(colors.bg_hover))
                    .text_xs()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.theme = preset;
                        cx.notify();
                    }))
                    .child(preset.name()),
            );
        }

        let inner_settings = self.settings_scroller.apply_rubber_band(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .bg(colors.bg_panel)
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(colors.border)
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_xs()
                                .child("1. REFRESH FREQUENCY"),
                        )
                        .child(freq_buttons),
                )
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .bg(colors.bg_panel)
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(colors.border)
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_xs()
                                .child("2. PER-APP MEMORY STRATEGY (SIDE-BY-SIDE)"),
                        )
                        .child(mode_cards)
                        .child(
                            div()
                                .id("toggle-group-apps-settings")
                                .w_full()
                                .min_w_0()
                                .px_3()
                                .py_2()
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .border_1()
                                .border_color(colors.accent)
                                .bg(colors.bg_elevated)
                                .hover(|s| s.bg(colors.bg_hover))
                                .text_xs()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_group_by_app(cx);
                                }))
                                .child(if self.config.group_by_app {
                                    "Group Subprocesses by App Name: ON (Click for raw PIDs)"
                                } else {
                                    "Group Subprocesses by App Name: OFF (Showing raw PIDs)"
                                }),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .bg(colors.bg_panel)
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(colors.border)
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_xs()
                                .child("3. COLOR THEMES"),
                        )
                        .child(theme_buttons),
                ),
        );

        div()
            .id("settings-scroll-container")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .track_scroll(&self.settings_scroller.handle)
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                let dy = f32::from(event.delta.pixel_delta(px(24.0)).y);
                this.settings_scroller.on_wheel(dy);
                cx.notify();
            }))
            .p_4()
            .child(inner_settings)
    }
}
