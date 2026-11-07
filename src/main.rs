mod state;
mod theme;
mod ui;

use gpui_kit::*;
use state::{MonitorConfig, SystemMetrics, spawn_collector};
use std::sync::{Arc, Mutex};
use ui::SystemMonitorView;

fn main() {
    let metrics = Arc::new(Mutex::new(SystemMetrics::default()));
    let config = Arc::new(Mutex::new(MonitorConfig::default()));

    // Start background system telemetry worker
    spawn_collector(Arc::clone(&metrics), Arc::clone(&config));

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1240.0), px(820.0)),
                cx,
            ))),
            ..Default::default()
        };

        let metrics_clone = Arc::clone(&metrics);
        let config_clone = Arc::clone(&config);

        gpui_kit::open_window(window_options, cx, move |_, cx| {
            cx.new(|cx| SystemMonitorView::new(metrics_clone, config_clone, cx))
        })
        .expect("Failed to open GPUI Kit window");
    });
}
