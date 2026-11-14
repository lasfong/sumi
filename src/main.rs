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

    // Execute automated GPU verification batches & screenshots offscreen on Metal/wgpu
    // (zero window creation, zero Dock bounce, zero focus stealing, immune to display sleep).
    if let Ok(batch_name) = std::env::var("SUMI_CAPTURE_BATCH")
        && !batch_name.trim().is_empty()
    {
        if let Err(err) = sumi_workstation::SumiWorkstationApp::run_offscreen_wgpu_capture_batch(
            batch_name.trim(),
        ) {
            log::error!("Offscreen GPU batch capture '{batch_name}' failed: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    if let Ok(screenshot_path) = std::env::var("SUMI_SCREENSHOT_PATH")
        && !screenshot_path.trim().is_empty()
    {
        let telemetry_path = std::env::var("SUMI_TELEMETRY_PATH").ok();
        let scenario = std::env::var("SUMI_SCENARIO").ok();
        if let Err(err) = sumi_workstation::SumiWorkstationApp::run_offscreen_wgpu_single_capture(
            screenshot_path.trim(),
            telemetry_path.as_deref(),
            scenario.as_deref(),
            initial_width.round().max(320.0) as u32,
            initial_height.round().max(240.0) as u32,
        ) {
            log::error!("Offscreen GPU single capture '{screenshot_path}' failed: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title(
            "Sumi Trading Workstation — VNINDEX · 1D (Milestone R09 Strategy Backtest Lab & Equity Curve Engine)",
        )
        .with_inner_size([initial_width, initial_height])
        .with_min_inner_size([640.0, 420.0]);

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
