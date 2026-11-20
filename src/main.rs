#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn ensure_workstation_working_dir() {
    if let Ok(env_dir) = std::env::var("SUMI_DATA_DIR") {
        let p = std::path::PathBuf::from(env_dir.trim());
        if p.is_dir() {
            let _ = std::env::set_current_dir(p);
            return;
        }
    }
    if std::path::Path::new("data").is_dir()
        || std::path::Path::new("ARCHIVE").exists()
        || std::path::Path::new("Cargo.toml").exists()
    {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        // 1. Prefer repo/workspace root when launched inside a checkout or `dist/Sumi Workstation.app`.
        let mut cursor = exe.parent();
        while let Some(dir) = cursor {
            if dir.join("Cargo.toml").exists()
                || dir.join("ARCHIVE").exists()
                || dir.join("data").is_dir()
            {
                let _ = std::env::set_current_dir(dir);
                return;
            }
            cursor = dir.parent();
        }
        // 2. Standalone `.app` copied outside the repo (e.g. `/Applications` or `/tmp`):
        //    resolve bundled `Contents/Resources` when present, or `~/Library/Application Support/Sumi Workstation`.
        if let Some(resources) = exe
            .parent()
            .and_then(|p| p.parent())
            .map(|c| c.join("Resources"))
            && resources.join("data").is_dir()
        {
            let _ = std::env::set_current_dir(resources);
            return;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let app_support = std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Sumi Workstation");
        if app_support.is_dir() {
            let _ = std::env::set_current_dir(app_support);
            return;
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let win_support = std::path::PathBuf::from(appdata).join("Sumi Workstation");
        if win_support.is_dir() {
            let _ = std::env::set_current_dir(win_support);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_initial_cli_file_arg() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok();
    for arg in std::env::args_os().skip(1) {
        let s = arg.to_string_lossy();
        if s.is_empty() || s.starts_with('-') {
            continue;
        }
        let raw = std::path::PathBuf::from(&arg);
        let abs = if raw.is_absolute() {
            raw
        } else if let Some(ref base) = cwd {
            base.join(raw)
        } else {
            raw
        };
        let ext_ok = abs.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "csv" | "tsv" | "txt" | "json"
            )
        });
        if abs.is_file() || ext_ok {
            return Some(abs);
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();
    // Resolve any CLI data file path BEFORE `ensure_workstation_working_dir()` potentially chdirs.
    let cli_file_arg = resolve_initial_cli_file_arg();
    ensure_workstation_working_dir();

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
        .with_title("Sumi Trading Workstation — v1.1")
        .with_inner_size([initial_width, initial_height])
        .with_min_inner_size([640.0, 420.0]);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Sumi Trading Workstation",
        native_options,
        Box::new(move |cc| {
            let mut app = sumi_workstation::SumiWorkstationApp::new(cc);
            if let Some(ref path) = cli_file_arg {
                let path_str = path.to_string_lossy();
                let _ = app.import_file_from_path(&path_str);
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
