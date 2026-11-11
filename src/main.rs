#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();

    let initial_width = std::env::var("SUMI_WINDOW_WIDTH")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1280.0);
    let initial_height = std::env::var("SUMI_WINDOW_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(800.0);

    let is_automated_capture = std::env::var("SUMI_SCREENSHOT_PATH")
        .ok()
        .is_some_and(|v| !v.is_empty())
        || std::env::var("SUMI_CAPTURE_BATCH")
            .ok()
            .is_some_and(|v| !v.is_empty());

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(
            "Sumi Trading Workstation — VNINDEX · 1D (Milestone R04 Multi-Pane & Indicator Engine)",
        )
        .with_inner_size([initial_width, initial_height])
        .with_min_inner_size([640.0, 420.0]);

    if is_automated_capture {
        // Prevent background automated verification captures from stealing window/keyboard focus!
        viewport = viewport.with_active(false);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Sumi Trading Workstation",
        native_options,
        Box::new(|cc| Ok(Box::new(sumi_workstation::SumiWorkstationApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
