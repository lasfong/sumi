use std::fmt::Write as _;
use std::process::Command;
use std::time::Instant;

use egui::{Pos2, Rect, Vec2};
use sumi_workstation::alerts::{AlertCondition, AlertEngine, AlertFrequency, CrossDirection};
use sumi_workstation::app::{
    EquityPaneSource, RightDockTab, ScannerDockTab, StrategyLabSubTab, SumiWorkstationApp,
};
use sumi_workstation::backtest::{
    BacktestConfig, BacktestRunOptions, BacktestRunResult, PortfolioUniverseFilter,
    PortfolioWeightingMode, PositionSizingMode, StopTakeConfig, StrategyDirectionMode,
    StrategyParams, StrategyPresetKind, SweepRankingMetric, WalkForwardConfig,
    run_all_presets_battle, run_backtest_with_options, run_multi_symbol_battle,
    run_parameter_sweep, run_portfolio_backtest, run_walk_forward_validation,
};
use sumi_workstation::chart::{
    ChartDisplayConfig, ChartReplayTradingOverlay, ChartViewportState, render_candlestick_chart,
    render_shared_pane_candlestick_chart, render_workstation_chart, render_workstation_chart_full,
    render_workstation_chart_with_drawings,
};
use sumi_workstation::data::{
    DataLoadOptions, Timeframe, aggregate_ohlcv, build_btcusdt_15m_json_document,
    generate_intraday_vn30f_1m_report, generate_synthetic_ohlcv, load_bundled_watchlist_universe,
    load_ohlcv_csv, load_ohlcv_json, resolve_benchmark_series_for_window,
};
use sumi_workstation::drawings::{
    DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStore, FibDirection, LocalPoint,
    MAGNET_THRESHOLD_PX, MagnetMode, PaneDrawingProjection, hit_test_pane_drawings,
    snap_to_visible_ohlc,
};
use sumi_workstation::indicators::{
    BlockExtrema, IndicatorCache, IndicatorConfig, OscillatorPaneId, SeriesBlockIndex,
    VolumeProfileConfig, compute_anchored_vwap, compute_relative_strength, compute_session_vwap,
    compute_volume_profile,
};
use sumi_workstation::journal::{
    ChecklistState, ExchangeVenue, MistakeTag, SetupTag, TradeJournalStore, TradePlanInput,
    VnPriceUnit, compute_trade_plan,
};
use sumi_workstation::model::Candle;
use sumi_workstation::replay::{ReplayController, ReplayStepUnit};
use sumi_workstation::scanner::{
    ScannerFilter, WatchlistSortColumn, evaluate_symbol_scan_and_signals, filter_and_sort_watchlist,
};
use sumi_workstation::trading::{OrderKind, OrderRequest, OrderSide, TradingSession};
use sumi_workstation::viewport::{
    HorzScaleOptions, KineticAnimation, PaneViewport, PlotRect, PriceScale, PriceScaleMode,
    PriceScaleOptions, TimeScale,
};

#[derive(Debug, Clone)]
struct TimingStats {
    iterations: usize,
    min_us: f64,
    median_us: f64,
    p95_us: f64,
    max_us: f64,
    mean_us: f64,
}

impl TimingStats {
    fn from_samples(mut samples_us: Vec<f64>) -> Self {
        samples_us.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = samples_us.len().max(1);
        let min_us = samples_us.first().copied().unwrap_or(0.0);
        let max_us = samples_us.last().copied().unwrap_or(0.0);
        let median_us = samples_us[n / 2];
        let p95_idx = ((n as f64 * 0.95).floor() as usize).min(n - 1);
        let p95_us = samples_us[p95_idx];
        let mean_us = samples_us.iter().sum::<f64>() / (n as f64);
        Self {
            iterations: n,
            min_us,
            median_us,
            p95_us,
            max_us,
            mean_us,
        }
    }
}

#[derive(Debug, Clone)]
struct RenderCaseResult {
    dataset_bars: usize,
    workload_id: &'static str,
    description: &'static str,
    scale_mode: &'static str,
    conflation_enabled: bool,
    bar_spacing: f64,
    conflation_factor: usize,
    culled_bars: usize,
    rendered_candles: usize,
    paint_shapes: usize,
    clipped_primitives: usize,
    total_vertices: usize,
    total_indices: usize,
    ui_build_stats: TimingStats,
    tessellate_stats: TimingStats,
    total_cpu_frame_stats: TimingStats,
}

#[derive(Debug, Clone)]
struct MathCaseResult {
    dataset_bars: usize,
    gen_time_ms: f64,
    candle_vec_bytes: usize,
    rss_kb_after_alloc: u64,
    pure_zoom_cull_autoscale_stats: TimingStats,
}

fn main() {
    let cpu_brand = cmd_stdout("sysctl", &["-n", "machdep.cpu.brand_string"])
        .unwrap_or_else(|| format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS));
    let mem_bytes = cmd_stdout("sysctl", &["-n", "hw.memsize"])
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let rustc_ver =
        cmd_stdout("rustc", &["--version"]).unwrap_or_else(|| "rustc (unknown)".to_owned());
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    if std::env::args().any(|a| a == "--r04") {
        run_r04_indicator_multipane_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r05") {
        run_r05_drawing_engine_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r06") {
        run_r06_data_timeframe_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r07") {
        run_r07_replay_trading_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r08") {
        run_r08_scanner_alerts_palette_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r09") {
        run_r09_backtest_lab_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r10") {
        run_r10_risk_journal_wfo_portfolio_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }
    if std::env::args().any(|a| a == "--r11") {
        run_r11_volume_profile_vwap_rs_benchmark(&cpu_brand, mem_bytes, &rustc_ver, profile);
        return;
    }

    println!("=== Sumi R03 Viewport & Renderer Benchmark ===");
    println!(
        "CPU: {cpu_brand} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let dataset_sizes = [10_000_usize, 50_000_usize, 100_000_usize];
    let mut math_results = Vec::new();
    let mut render_results = Vec::new();
    let mut prebuilt_datasets = Vec::with_capacity(dataset_sizes.len());

    // Phase 1: Measure clean dataset generation, heap footprint, RSS, and pure O(1)-culled viewport math
    // BEFORE allocating multi-megabyte unconflated epaint meshes!
    for &bar_count in &dataset_sizes {
        let t_gen = Instant::now();
        let candles = generate_synthetic_ohlcv(bar_count);
        let gen_time_ms = t_gen.elapsed().as_secs_f64() * 1_000.0;
        let candle_vec_bytes = candles.len() * std::mem::size_of::<Candle>()
            + candles
                .iter()
                .map(|c| c.date_label.capacity())
                .sum::<usize>();
        let rss_kb = current_rss_kb();

        let math_stats = bench_pure_viewport_math(&candles, 100, 1_000);
        math_results.push(MathCaseResult {
            dataset_bars: bar_count,
            gen_time_ms,
            candle_vec_bytes,
            rss_kb_after_alloc: rss_kb,
            pure_zoom_cull_autoscale_stats: math_stats,
        });
        prebuilt_datasets.push(candles);
    }

    // Phase 2: Measure end-to-end egui::Painter UI + epaint CPU tessellation across workloads
    for candles in &prebuilt_datasets {
        let bar_count = candles.len();

        // 1. Standard trading view (~150 visible bars, Linear)
        render_results.push(bench_headless_render(
            candles,
            "standard_8px",
            "Standard trading view (bar_spacing=8.0, ~150 visible bars)",
            PriceScaleMode::Normal,
            true,
            |vp| {
                vp.reset_to_default();
            },
            20,
            120,
        ));

        // 2. Dense view (~980 visible bars, Logarithmic)
        render_results.push(bench_headless_render(
            candles,
            "dense_1_2px_log",
            "Dense swing view (bar_spacing=1.2, ~980 visible bars, Log)",
            PriceScaleMode::Logarithmic,
            true,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(1.2);
            },
            20,
            120,
        ));

        // 3. Full-dataset fit WITH conflation
        render_results.push(bench_headless_render(
            candles,
            "fit_all_conflated",
            "Full-dataset fit WITH DPR-aware power-of-2 conflation",
            PriceScaleMode::Logarithmic,
            true,
            |vp| {
                vp.fit_all();
            },
            15,
            80,
        ));

        // 4. Full-dataset fit WITHOUT conflation (stress-testing raw egui::Painter tessellation cliff)
        let unconflated_iters = if bar_count >= 50_000 { 25 } else { 60 };
        render_results.push(bench_headless_render(
            candles,
            "fit_all_unconflated",
            "Full-dataset fit WITHOUT conflation (raw egui::Painter stress)",
            PriceScaleMode::Logarithmic,
            false,
            |vp| {
                vp.fit_all();
            },
            5,
            unconflated_iters,
        ));
    }

    // 5. Multi-pane workstation simulation (1 shared TimeScale + 3 independent PriceScale panes on 100K bars)
    let candles_100k = prebuilt_datasets.last().expect("100K dataset");
    let multipane_res = bench_multipane_3x(candles_100k, 15, 80);
    render_results.push(multipane_res);

    // Print concise table
    println!(
        "\n{:<8} | {:<22} | {:>6} | {:>7} | {:>7} | {:>8} | {:>9} | {:>9} | {:>9}",
        "Bars",
        "Workload",
        "Confl",
        "Culled",
        "Drawn",
        "Vertices",
        "UI (ms)",
        "Tess (ms)",
        "Total(ms)"
    );
    println!("{}", "-".repeat(108));
    for r in &render_results {
        println!(
            "{:<8} | {:<22} | {:>5}x | {:>7} | {:>7} | {:>8} | {:>9.3} | {:>9.3} | {:>9.3}",
            r.dataset_bars,
            r.workload_id,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
        );
    }

    let json_content = format_json_report(
        &cpu_brand,
        mem_bytes,
        &rustc_ver,
        profile,
        &math_results,
        &render_results,
    );
    let md_content = format_markdown_report(
        &cpu_brand,
        mem_bytes,
        &rustc_ver,
        profile,
        &math_results,
        &render_results,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write("docs/evidence/r03_viewport_benchmark.json", json_content)
        .expect("write benchmark json");
    std::fs::write("docs/evidence/r03_viewport_benchmark.md", md_content)
        .expect("write benchmark markdown");

    println!(
        "\nSaved docs/evidence/r03_viewport_benchmark.json and docs/evidence/r03_viewport_benchmark.md"
    );
}

fn bench_pure_viewport_math(candles: &[Candle], warmup: usize, iters: usize) -> TimingStats {
    let mut vp = ChartViewportState::new();
    let plot_rect = PlotRect::new(8.0, 44.0, 1178.0, 690.0);
    vp.time_scale.set_total_bars(candles.len());
    vp.time_scale.set_width(plot_rect.width);
    vp.main_price_scale.set_height(plot_rect.height);

    let mut samples_us = Vec::with_capacity(iters);
    for step in 0..(warmup + iters) {
        let scale = if step % 2 == 0 { 0.35 } else { -0.35 };
        let anchor_x = 300.0 + ((step % 11) as f64) * 55.0;

        let t0 = Instant::now();
        vp.time_scale.zoom(anchor_x, scale);
        let strict = vp.time_scale.strict_visible_data_range();
        let cull = vp.time_scale.culling_data_range();
        let slice = &candles[strict];
        let base_val = slice.first().map_or(1000.0, |c| c.close);
        vp.main_price_scale.autoscale_candles(slice, base_val);
        let bridge = PaneViewport::new(&vp.time_scale, &vp.main_price_scale, plot_rect, base_val);

        let mut checksum = 0.0_f64;
        for (offset, c) in candles[cull.clone()].iter().enumerate() {
            let idx = (cull.start + offset) as i64;
            if let (Some(sx), Some(sy)) = (
                bridge.index_to_screen_x(idx),
                bridge.price_to_screen_y(c.close),
            ) {
                checksum += sx + sy;
            }
        }
        std::hint::black_box(checksum);
        let elapsed_us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= warmup {
            samples_us.push(elapsed_us);
        }
    }
    TimingStats::from_samples(samples_us)
}

#[allow(clippy::too_many_arguments)]
fn bench_headless_render(
    candles: &[Candle],
    workload_id: &'static str,
    description: &'static str,
    scale_mode: PriceScaleMode,
    conflation_enabled: bool,
    mut setup_fn: impl FnMut(&mut ChartViewportState),
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    let ppp = 2.0_f32; // Retina HiDPI
    ctx.set_pixels_per_point(ppp);

    let mut vp = ChartViewportState::new();
    vp.set_conflation_enabled(conflation_enabled);
    vp.main_price_scale.set_mode(scale_mode);

    let cfg = ChartDisplayConfig {
        symbol: "VNINDEX",
        timeframe: "1D",
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    // Prime 2 frames so pixels_per_point and plot_rect (1178x756 logical px) are 100% settled
    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_candlestick_chart(ui, candles, &mut vp, cfg);
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);

    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 120.0 + ((step % 37) as f32) * 24.0;
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, 360.0))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_candlestick_chart(ui, candles, &mut vp, cfg);
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;

        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }
        std::hint::black_box(&primitives);

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("benchmark chart report");
    if workload_id.starts_with("fit_all") {
        assert_eq!(
            rep.culled_bar_count,
            candles.len(),
            "fit_all workload must span 100% of dataset bars"
        );
    }
    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn bench_multipane_3x(candles: &[Candle], warmup: usize, iters: usize) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 960.0)));
    let ppp = 2.0_f32;
    ctx.set_pixels_per_point(ppp);

    let mut shared_ts = TimeScale::new(HorzScaleOptions::sumi_default());
    let mut shared_kinetic: Option<KineticAnimation> = None;
    let mut panes = [
        (
            PriceScale::new("pane1-right", PriceScaleOptions::sumi_main_price()),
            PriceScale::new("pane1-vol", PriceScaleOptions::sumi_volume_overlay()),
        ),
        (
            PriceScale::new("pane2-right", PriceScaleOptions::sumi_main_price()),
            PriceScale::new("pane2-vol", PriceScaleOptions::sumi_volume_overlay()),
        ),
        (
            PriceScale::new("pane3-right", PriceScaleOptions::sumi_main_price()),
            PriceScale::new("pane3-vol", PriceScaleOptions::sumi_volume_overlay()),
        ),
    ];
    panes[1].0.set_mode(PriceScaleMode::Logarithmic);

    let cfg = ChartDisplayConfig {
        symbol: "VNINDEX",
        timeframe: "1D",
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let h = (ui.available_height() - 16.0) / 3.0;
                    for (idx, (main_ps, vol_ps)) in panes.iter_mut().enumerate() {
                        ui.push_id(idx, |ui| {
                            ui.allocate_ui(Vec2::new(ui.available_width(), h), |ui| {
                                let _ = render_shared_pane_candlestick_chart(
                                    ui,
                                    candles,
                                    &mut shared_ts,
                                    &mut shared_kinetic,
                                    main_ps,
                                    vol_ps,
                                    idx == 0,
                                    cfg,
                                );
                            });
                        });
                    }
                });
            },
        );
        init_out.textures_delta.clear();
    }
    shared_ts.set_bar_spacing(1.2);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut culled_sum = 0_usize;
    let mut drawn_sum = 0_usize;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut c_acc = 0_usize;
        let mut d_acc = 0_usize;
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let h = (ui.available_height() - 16.0) / 3.0;
                for (idx, (main_ps, vol_ps)) in panes.iter_mut().enumerate() {
                    ui.push_id(idx, |ui| {
                        ui.allocate_ui(Vec2::new(ui.available_width(), h), |ui| {
                            if let Some(r) = render_shared_pane_candlestick_chart(
                                ui,
                                candles,
                                &mut shared_ts,
                                &mut shared_kinetic,
                                main_ps,
                                vol_ps,
                                idx == 0,
                                cfg,
                            ) {
                                c_acc += r.culled_bar_count;
                                d_acc += r.rendered_candle_count;
                            }
                        });
                    });
                }
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;
        culled_sum = c_acc;
        drawn_sum = d_acc;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }
        std::hint::black_box(&primitives);

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id: "multipane_3x_dense",
        description: "3 stacked panes sharing 1 TimeScale on 100K bars (bar_spacing=1.2, ~2,940 total visible bars)",
        scale_mode: "Linear+Log",
        conflation_enabled: true,
        bar_spacing: 1.2,
        conflation_factor: 1,
        culled_bars: culled_sum,
        rendered_candles: drawn_sum,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn cmd_stdout(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn current_rss_kb() -> u64 {
    let pid = std::process::id().to_string();
    cmd_stdout("ps", &["-o", "rss=", "-p", &pid])
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn format_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    math: &[MathCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");

    let _ = writeln!(s, "  \"pure_viewport_math\": [");
    for (idx, m) in math.iter().enumerate() {
        let comma = if idx + 1 == math.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"iterations\": {}, \"gen_time_ms\": {:.4}, \"candle_heap_bytes\": {}, \"process_rss_kb\": {}, \"min_us\": {:.3}, \"median_us\": {:.3}, \"p95_us\": {:.3}, \"max_us\": {:.3}, \"mean_us\": {:.3} }}{comma}",
            m.dataset_bars,
            m.pure_zoom_cull_autoscale_stats.iterations,
            m.gen_time_ms,
            m.candle_vec_bytes,
            m.rss_kb_after_alloc,
            m.pure_zoom_cull_autoscale_stats.min_us,
            m.pure_zoom_cull_autoscale_stats.median_us,
            m.pure_zoom_cull_autoscale_stats.p95_us,
            m.pure_zoom_cull_autoscale_stats.max_us,
            m.pure_zoom_cull_autoscale_stats.mean_us,
        );
    }
    let _ = writeln!(s, "  ],");

    let _ = writeln!(s, "  \"egui_painter_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"conflation_enabled\": {}, \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"ui_p95_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"tessellate_p95_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.conflation_enabled,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.ui_build_stats.p95_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.tessellate_stats.p95_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    math: &[MathCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R03 — Reproducible Viewport & `egui::Painter` Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Target Resolution:** `1280×800` logical px (`2560×1600` physical Retina px at `pixels_per_point = 2.0`)"
    );
    let _ = writeln!(
        s,
        "- **Measurement Scope:** Measures deterministic CPU viewport math, `egui` UI pass (`render_candlestick_chart`), and full `epaint` CPU mesh tessellation (`Context::tessellate`). Metal GPU swapchain vsync presentation is measured separately via native GUI capture.\n"
    );

    let _ = writeln!(
        s,
        "## 1. Dataset Generation, Memory Footprint & Pure Viewport Math (`TimeScale` + `PriceScale` + Culling)\n"
    );
    let _ = writeln!(
        s,
        "| Dataset Bars | Gen Time (`ms`) | Series Heap (`MiB`) | Process RSS (`MiB`) | Pure Zoom+Cull+Autoscale+Project Median (`µs`) | P95 (`µs`) |"
    );
    let _ = writeln!(s, "| ---: | ---: | ---: | ---: | ---: | ---: |");
    for m in math {
        let _ = writeln!(
            s,
            "| `{}` | `{:.2}` | `{:.2}` | `{:.2}` | `{:.2} µs` | `{:.2} µs` |",
            m.dataset_bars,
            m.gen_time_ms,
            (m.candle_vec_bytes as f64) / (1024.0 * 1024.0),
            (m.rss_kb_after_alloc as f64) / 1024.0,
            m.pure_zoom_cull_autoscale_stats.median_us,
            m.pure_zoom_cull_autoscale_stats.p95_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End `egui::Painter` Build + `epaint` Tessellation Across 10K / 50K / 100K Bars\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | 60 FPS (`16.67ms`) | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let pass_60 = if tot_med_ms <= 16.67 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{pass_60}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

#[derive(Debug, Clone)]
struct IndicatorComputeCaseResult {
    dataset_bars: usize,
    cold_all6_stats: TimingStats,
    warm_cache_sync_stats: TimingStats,
    cache_heap_bytes: usize,
}

fn run_r04_indicator_multipane_benchmark(
    cpu_brand: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    println!("=== Sumi R04 Indicator Engine & 4-Pane Workstation Benchmark ===");
    println!(
        "CPU: {cpu_brand} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let dataset_sizes = [10_000_usize, 50_000_usize, 100_000_usize];
    let all_ind_cfg = IndicatorConfig {
        show_sma: true,
        sma_period: 20,
        show_ema: true,
        ema_period: 50,
        show_bbands: true,
        bb_period: 20,
        bb_std_mult: 2.0,
        show_rsi: true,
        rsi_period: 14,
        show_macd: true,
        macd_fast: 12,
        macd_slow: 26,
        macd_signal: 9,
        show_atr: true,
        atr_period: 14,
        ..IndicatorConfig::default()
    };

    let mut compute_results = Vec::new();
    let mut render_results = Vec::new();

    for &bar_count in &dataset_sizes {
        let candles = generate_synthetic_ohlcv(bar_count);

        // 1. Cold computation of ALL 6 indicators (10 output series total)
        let cold_iters = if bar_count >= 100_000 { 40 } else { 80 };
        let mut cold_samples = Vec::with_capacity(cold_iters);
        let mut warm_cache = IndicatorCache::new();
        for step in 0..(8 + cold_iters) {
            let mut tmp_cache = IndicatorCache::new();
            let t0 = Instant::now();
            tmp_cache.sync(&candles, (step + 1) as u64, &all_ind_cfg);
            std::hint::black_box(&tmp_cache.macd.histogram);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            if step >= 8 {
                cold_samples.push(us);
            }
            if step == 0 {
                warm_cache = tmp_cache;
            }
        }

        // 2. Warm dirty-cache `sync()` overhead during pan/zoom/hover frames
        let mut warm_samples = Vec::with_capacity(500);
        for _ in 0..500 {
            let t0 = Instant::now();
            warm_cache.sync(&candles, 1, &all_ind_cfg);
            std::hint::black_box(warm_cache.rsi.len());
            warm_samples.push(t0.elapsed().as_secs_f64() * 1_000_000.0);
        }

        let series_vecs = 10_usize; // sma, ema, bb(3), rsi, macd(3), atr
        let raw_series_bytes = series_vecs * bar_count * std::mem::size_of::<Option<f64>>();
        let block_index_bytes = series_vecs
            * bar_count.div_ceil(SeriesBlockIndex::BLOCK_SIZE)
            * std::mem::size_of::<Option<BlockExtrema>>();
        let cache_heap_bytes = raw_series_bytes + block_index_bytes;
        compute_results.push(IndicatorComputeCaseResult {
            dataset_bars: bar_count,
            cold_all6_stats: TimingStats::from_samples(cold_samples),
            warm_cache_sync_stats: TimingStats::from_samples(warm_samples),
            cache_heap_bytes,
        });

        // 3. Full 4-pane workstation UI + epaint tessellation workloads
        render_results.push(bench_r04_workstation_render(
            &candles,
            &all_ind_cfg,
            &warm_cache,
            "r04_4pane_std_8px",
            "4 panes (Main+SMA+EMA+BB + RSI + MACD + ATR), bar_spacing=8.0",
            PriceScaleMode::Normal,
            |vp| {
                vp.reset_to_default();
                vp.reset_pane_stretches();
            },
            15,
            100,
        ));

        render_results.push(bench_r04_workstation_render(
            &candles,
            &all_ind_cfg,
            &warm_cache,
            "r04_4pane_dense_1_2px",
            "4 panes + all 6 indicators, dense swing view (bar_spacing=1.2, Log)",
            PriceScaleMode::Logarithmic,
            |vp| {
                vp.reset_to_default();
                vp.reset_pane_stretches();
                vp.time_scale.set_bar_spacing(1.2);
            },
            15,
            100,
        ));

        render_results.push(bench_r04_workstation_render(
            &candles,
            &all_ind_cfg,
            &warm_cache,
            "r04_4pane_fit_conflated",
            "4 panes + all 6 indicators, full-dataset fit WITH bucket-aligned conflation",
            PriceScaleMode::Logarithmic,
            |vp| {
                vp.reset_pane_stretches();
                vp.fit_all();
            },
            15,
            80,
        ));

        if bar_count == 100_000 {
            render_results.push(bench_r04_workstation_render(
                &candles,
                &all_ind_cfg,
                &warm_cache,
                "r04_4pane_resized_sep",
                "4 panes + all 6 indicators after interactive separator stretch transfer",
                PriceScaleMode::Normal,
                |vp| {
                    vp.reset_to_default();
                    vp.reset_pane_stretches();
                    vp.apply_separator_drag_delta(None, OscillatorPaneId::Rsi, 450.0, -60.0);
                },
                15,
                100,
            ));
        }
    }

    println!("\n--- Cold All-6 Indicator Compute vs Warm Dirty-Cache Sync ---");
    for c in &compute_results {
        println!(
            "Bars={:<7} | Cold All-6 Median: {:>7.3} ms (P95 {:>7.3} ms) | Warm Sync Median: {:>6.3} µs | Cache Heap: {:.2} MiB",
            c.dataset_bars,
            c.cold_all6_stats.median_us / 1_000.0,
            c.cold_all6_stats.p95_us / 1_000.0,
            c.warm_cache_sync_stats.median_us,
            (c.cache_heap_bytes as f64) / (1024.0 * 1024.0),
        );
    }

    println!(
        "\n{:<8} | {:<24} | {:>6} | {:>7} | {:>7} | {:>8} | {:>9} | {:>9} | {:>9}",
        "Bars",
        "Workload",
        "Confl",
        "Culled",
        "Drawn",
        "Vertices",
        "UI (ms)",
        "Tess (ms)",
        "Total(ms)"
    );
    println!("{}", "-".repeat(110));
    for r in &render_results {
        println!(
            "{:<8} | {:<24} | {:>5}x | {:>7} | {:>7} | {:>8} | {:>9.3} | {:>9.3} | {:>9.3}",
            r.dataset_bars,
            r.workload_id,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
        );
    }

    let json_out = format_r04_json_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &compute_results,
        &render_results,
    );
    let md_out = format_r04_markdown_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &compute_results,
        &render_results,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write(
        "docs/evidence/r04_indicator_multipane_benchmark.json",
        json_out,
    )
    .expect("write r04 benchmark json");
    std::fs::write("docs/evidence/r04_indicator_multipane_benchmark.md", md_out)
        .expect("write r04 benchmark md");

    println!(
        "\nSaved docs/evidence/r04_indicator_multipane_benchmark.json and docs/evidence/r04_indicator_multipane_benchmark.md"
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_r04_workstation_render(
    candles: &[Candle],
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    workload_id: &'static str,
    description: &'static str,
    scale_mode: PriceScaleMode,
    mut setup_fn: impl FnMut(&mut ChartViewportState),
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut vp = ChartViewportState::new();
    vp.main_price_scale.set_mode(scale_mode);

    let cfg = ChartDisplayConfig {
        symbol: "VNINDEX",
        timeframe: "1D",
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_workstation_chart(ui, candles, &mut vp, cfg, indicators, cache);
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 140.0 + ((step % 35) as f32) * 24.0;
        let hover_y = if step % 2 == 0 { 220.0 } else { 520.0 }; // alternates hovering Main & Oscillator panes!
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_workstation_chart(ui, candles, &mut vp, cfg, indicators, cache);
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }
        std::hint::black_box(&primitives);

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("r04 benchmark report");
    assert_eq!(rep.pane_count, 4);
    assert_eq!(rep.active_overlay_count, 3);

    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r04_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    compute: &[IndicatorComputeCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R04\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"indicator_compute\": [");
    for (idx, c) in compute.iter().enumerate() {
        let comma = if idx + 1 == compute.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"cold_all6_median_ms\": {:.4}, \"cold_all6_p95_ms\": {:.4}, \"warm_cache_sync_median_us\": {:.4}, \"cache_heap_bytes\": {} }}{comma}",
            c.dataset_bars,
            c.cold_all6_stats.median_us / 1_000.0,
            c.cold_all6_stats.p95_us / 1_000.0,
            c.warm_cache_sync_stats.median_us,
            c.cache_heap_bytes,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"multipane_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r04_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    compute: &[IndicatorComputeCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R04 — Indicator Engine & 4-Pane Workstation Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r04`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** Main Candlestick + Volume + `SMA(20)` + `EMA(50)` + `BB(20, 2.0)` + 3 Stacked Oscillator Panes (`RSI(14)`, `MACD(12, 26, 9)`, `ATR(14)`) = **4 Panes, 10 Concurrent Series**\n"
    );
    let _ = writeln!(
        s,
        "## 1. Cold All-6 Indicator Computation vs Warm Dirty-Cache Sync\n"
    );
    let _ = writeln!(
        s,
        "| Dataset Bars | Cold All-6 Compute Median (`ms`) | Cold All-6 P95 (`ms`) | Warm Pan/Zoom Cache Sync (`µs`) | Indicator Cache Heap (`MiB`) |"
    );
    let _ = writeln!(s, "| ---: | ---: | ---: | ---: | ---: |");
    for c in compute {
        let _ = writeln!(
            s,
            "| `{}` | **`{:.3} ms`** | `{:.3} ms` | **`{:.3} µs`** | `{:.2} MiB` |",
            c.dataset_bars,
            c.cold_all6_stats.median_us / 1_000.0,
            c.cold_all6_stats.p95_us / 1_000.0,
            c.warm_cache_sync_stats.median_us,
            (c.cache_heap_bytes as f64) / (1024.0 * 1024.0),
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End 4-Pane `egui::Painter` Build + `epaint` Tessellation (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

#[derive(Debug, Clone)]
struct DrawingMathCaseResult {
    dataset_bars: usize,
    active_drawings: usize,
    project_stats: TimingStats,
    hit_test_stats: TimingStats,
    magnet_snap_stats: TimingStats,
    magnet_fit_stats: TimingStats,
}

fn build_r05_benchmark_drawings(
    candles: &[Candle],
    vis_left: usize,
    vis_right: usize,
) -> DrawingStore {
    let mut store = DrawingStore::new();
    let span = vis_right.saturating_sub(vis_left).max(20);
    let bar_at = |frac: f64| -> usize {
        (vis_left + ((span as f64) * frac).round() as usize).min(candles.len() - 1)
    };

    // 25 active drawings: 5 of each primitive distributed across Main, RSI, MACD, and ATR!
    for k in 0..5_usize {
        let f0 = 0.06 + (k as f64) * 0.16;
        let f1 = (f0 + 0.18).min(0.96);
        let i0 = bar_at(f0);
        let i1 = bar_at(f1);
        let p0 = candles[i0].low;
        let p1 = candles[i1].high;

        // 1. HorizontalLine (Main + RSI + MACD + ATR)
        let h_pane = match k {
            0 | 1 => DrawingPaneTarget::Main,
            2 => DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
            3 => DrawingPaneTarget::Oscillator(OscillatorPaneId::Macd),
            _ => DrawingPaneTarget::Oscillator(OscillatorPaneId::Atr),
        };
        let h_val = match k {
            0 | 1 => (p0 + p1) * 0.5,
            2 => 55.0,
            3 => -2.5,
            _ => 18.0,
        };
        let _ = store.add_drawing(
            h_pane,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(i0 as f64, h_val),
            },
            None,
        );

        // 2. TrendLine
        let tl_pane = if k == 4 {
            DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi)
        } else {
            DrawingPaneTarget::Main
        };
        let (tl_v0, tl_v1) = if k == 4 { (35.0, 68.0) } else { (p0, p1) };
        let _ = store.add_drawing(
            tl_pane,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(i0 as f64, tl_v0),
                end: DrawingAnchor::new(i1 as f64, tl_v1),
            },
            None,
        );

        // 3. 360° Ray (including leftward rays when k % 2 == 1)
        let (r_orig, r_thru) = if k % 2 == 0 {
            (
                DrawingAnchor::new(i0 as f64, p0),
                DrawingAnchor::new(i1 as f64, p1),
            )
        } else {
            (
                DrawingAnchor::new(i1 as f64, p1),
                DrawingAnchor::new(i0 as f64, p0),
            )
        };
        let _ = store.add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Ray {
                origin: r_orig,
                through: r_thru,
            },
            None,
        );

        // 4. Rectangle (Main + MACD negative zone)
        let rect_pane = if k == 3 {
            DrawingPaneTarget::Oscillator(OscillatorPaneId::Macd)
        } else {
            DrawingPaneTarget::Main
        };
        let (rc0, rc1) = if k == 3 {
            (
                DrawingAnchor::new(i0 as f64, -5.0),
                DrawingAnchor::new(i1 as f64, 4.0),
            )
        } else {
            (
                DrawingAnchor::new(i0 as f64, p0),
                DrawingAnchor::new(i1 as f64, p1),
            )
        };
        let _ = store.add_drawing(
            rect_pane,
            DrawingGeometry::Rectangle {
                corner_a: rc0,
                corner_b: rc1,
            },
            None,
        );

        // 5. FibonacciRetracement
        let id = store.add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::FibonacciRetracement {
                start: DrawingAnchor::new(i0 as f64, p0),
                end: DrawingAnchor::new(i1 as f64, p1),
                direction: if k % 2 == 0 {
                    FibDirection::StartToEnd
                } else {
                    FibDirection::EndToStart
                },
                extend_right: k == 0,
            },
            None,
        );
        if k == 0 {
            store.selected_id = id;
        }
    }
    assert_eq!(store.drawings.len(), 25);
    store
}

fn run_r05_drawing_engine_benchmark(
    cpu_brand: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    println!("=== Sumi R05 Interactive Drawing & Multi-Pane Benchmark ===");
    println!(
        "CPU: {cpu_brand} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let dataset_sizes = [10_000_usize, 50_000_usize, 100_000_usize];
    let all_cfg = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };

    let mut math_cases = Vec::with_capacity(dataset_sizes.len());
    let mut render_cases = Vec::new();

    for &bar_count in &dataset_sizes {
        let candles = generate_synthetic_ohlcv(bar_count);
        let mut cache = IndicatorCache::new();
        cache.sync(&candles, 1, &all_cfg);

        // 1. Pure Drawing Projection + Hit-Test + OHLC Magnet microbenchmarks
        let store_recent = build_r05_benchmark_drawings(
            &candles,
            candles.len().saturating_sub(150),
            candles.len() - 1,
        );
        let mut ts = TimeScale::with_data(candles.len(), 1178.0, HorzScaleOptions::sumi_default());
        ts.set_bar_spacing(8.0);
        let mut main_ps = PriceScale::new("main-right", PriceScaleOptions::sumi_main_price());
        main_ps.set_height(420.0);
        let vis_slice = &candles[candles.len().saturating_sub(150)..];
        let base_val = vis_slice[0].close;
        main_ps.autoscale_candles(vis_slice, base_val);

        let mut rsi_ps = PriceScale::new("rsi-right", PriceScaleOptions::sumi_oscillator());
        rsi_ps.set_height(100.0);
        rsi_ps.autoscale_raw_bounds(0.0, 100.0, 50.0);
        let mut macd_ps = PriceScale::new("macd-right", PriceScaleOptions::sumi_oscillator());
        macd_ps.set_height(100.0);
        macd_ps.autoscale_raw_bounds(-12.0, 12.0, 1.0);
        let mut atr_ps = PriceScale::new("atr-right", PriceScaleOptions::sumi_oscillator());
        atr_ps.set_height(100.0);
        atr_ps.autoscale_raw_bounds(0.0, 40.0, 1.0);

        let main_proj = PaneDrawingProjection::new(&ts, &main_ps, 1178.0, 420.0, base_val);
        let rsi_proj = PaneDrawingProjection::new(&ts, &rsi_ps, 1178.0, 100.0, 1.0);
        let macd_proj = PaneDrawingProjection::new(&ts, &macd_ps, 1178.0, 100.0, 1.0);
        let atr_proj = PaneDrawingProjection::new(&ts, &atr_ps, 1178.0, 100.0, 1.0);

        let mut proj_samples = Vec::with_capacity(500);
        for step in 0..550 {
            let t0 = Instant::now();
            let mut count = 0_usize;
            for d in &store_recent.drawings {
                let p = match d.pane {
                    DrawingPaneTarget::Main => main_proj.project_drawing(d),
                    DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi) => {
                        rsi_proj.project_drawing(d)
                    }
                    DrawingPaneTarget::Oscillator(OscillatorPaneId::Macd) => {
                        macd_proj.project_drawing(d)
                    }
                    DrawingPaneTarget::Oscillator(
                        OscillatorPaneId::Atr
                        | OscillatorPaneId::RelativeStrength
                        | OscillatorPaneId::Equity,
                    ) => atr_proj.project_drawing(d),
                };
                if p.is_some() {
                    count += 1;
                }
            }
            std::hint::black_box(count);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            if step >= 50 {
                proj_samples.push(us);
            }
        }

        let mut hit_samples = Vec::with_capacity(500);
        for step in 0..550 {
            let cx = 80.0 + ((step % 41) as f64) * 24.0;
            let cy = 40.0 + ((step % 13) as f64) * 22.0;
            let cursor = LocalPoint::new(cx, cy);
            let t0 = Instant::now();
            let h0 = hit_test_pane_drawings(
                &store_recent.drawings,
                DrawingPaneTarget::Main,
                &main_proj,
                cursor,
                store_recent.selected_id,
            );
            let h1 = hit_test_pane_drawings(
                &store_recent.drawings,
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
                &rsi_proj,
                LocalPoint::new(cx, 50.0),
                store_recent.selected_id,
            );
            let h2 = hit_test_pane_drawings(
                &store_recent.drawings,
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Macd),
                &macd_proj,
                LocalPoint::new(cx, 50.0),
                store_recent.selected_id,
            );
            let h3 = hit_test_pane_drawings(
                &store_recent.drawings,
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Atr),
                &atr_proj,
                LocalPoint::new(cx, 50.0),
                store_recent.selected_id,
            );
            std::hint::black_box((h0, h1, h2, h3));
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            if step >= 50 {
                hit_samples.push(us);
            }
        }

        let mut magnet_samples = Vec::with_capacity(500);
        for step in 0..550 {
            let cx = 120.0 + ((step % 37) as f64) * 26.0;
            let cy = 60.0 + ((step % 17) as f64) * 18.0;
            let cursor = LocalPoint::new(cx, cy);
            let raw = main_proj
                .local_to_anchor(cursor)
                .unwrap_or(DrawingAnchor::new(0.0, 1200.0));
            let t0 = Instant::now();
            let snapped = snap_to_visible_ohlc(
                raw,
                cursor,
                &candles,
                &main_proj,
                MagnetMode::Ohlc,
                MAGNET_THRESHOLD_PX,
            );
            std::hint::black_box(snapped);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            if step >= 50 {
                magnet_samples.push(us);
            }
        }

        let mut fit_ts =
            TimeScale::with_data(candles.len(), 1178.0, HorzScaleOptions::sumi_default());
        fit_ts.fit_content();
        let mut fit_ps = PriceScale::new("main-fit", PriceScaleOptions::sumi_main_price());
        fit_ps.set_height(420.0);
        let fit_base = candles[0].close;
        fit_ps.autoscale_candles(&candles, fit_base);
        let fit_proj = PaneDrawingProjection::new(&fit_ts, &fit_ps, 1178.0, 420.0, fit_base);
        let mut magnet_fit_samples = Vec::with_capacity(500);
        for step in 0..550 {
            let cx = 120.0 + ((step % 37) as f64) * 26.0;
            let cy = 60.0 + ((step % 17) as f64) * 18.0;
            let cursor = LocalPoint::new(cx, cy);
            let raw = fit_proj
                .local_to_anchor(cursor)
                .unwrap_or(DrawingAnchor::new(0.0, 1200.0));
            let t0 = Instant::now();
            let snapped = snap_to_visible_ohlc(
                raw,
                cursor,
                &candles,
                &fit_proj,
                MagnetMode::Ohlc,
                MAGNET_THRESHOLD_PX,
            );
            std::hint::black_box(snapped);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            if step >= 50 {
                magnet_fit_samples.push(us);
            }
        }

        math_cases.push(DrawingMathCaseResult {
            dataset_bars: bar_count,
            active_drawings: store_recent.drawings.len(),
            project_stats: TimingStats::from_samples(proj_samples),
            hit_test_stats: TimingStats::from_samples(hit_samples),
            magnet_snap_stats: TimingStats::from_samples(magnet_samples),
            magnet_fit_stats: TimingStats::from_samples(magnet_fit_samples),
        });

        // 2. End-to-End 4-Pane + 6-Indicator + 25-Drawing Workstation Render + Tessellation
        render_cases.push(bench_r05_workstation_render(
            &candles,
            &all_cfg,
            &cache,
            "r05_standard_8px_4pane_25draw",
            "4 panes + 6 indicators + 25 drawings (bar_spacing=8.0, Linear)",
            PriceScaleMode::Normal,
            candles.len().saturating_sub(150),
            candles.len() - 1,
            |vp| vp.reset_to_default(),
            20,
            100,
        ));
        render_cases.push(bench_r05_workstation_render(
            &candles,
            &all_cfg,
            &cache,
            "r05_dense_1_2px_4pane_25draw",
            "4 panes + 6 indicators + 25 drawings (bar_spacing=1.2, Log)",
            PriceScaleMode::Logarithmic,
            candles.len().saturating_sub(980),
            candles.len() - 1,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(1.2);
            },
            20,
            100,
        ));
        render_cases.push(bench_r05_workstation_render(
            &candles,
            &all_cfg,
            &cache,
            "r05_fit_all_conflated_4pane_25draw",
            "4 panes + 6 indicators + 25 drawings (100% full-fit conflated, Log)",
            PriceScaleMode::Logarithmic,
            0,
            candles.len() - 1,
            |vp| vp.fit_all(),
            15,
            80,
        ));
    }

    println!(
        "\n{:<8} | {:<34} | {:>5} | {:>8} | {:>9} | {:>9} | {:>9}",
        "Bars", "Workload", "Confl", "Vertices", "UI (ms)", "Tess (ms)", "Total(ms)"
    );
    println!("{}", "-".repeat(100));
    for r in &render_cases {
        println!(
            "{:<8} | {:<34} | {:>4}x | {:>8} | {:>9.3} | {:>9.3} | {:>9.3}",
            r.dataset_bars,
            r.workload_id,
            r.conflation_factor,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
        );
    }

    let json_out = format_r05_json_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &math_cases,
        &render_cases,
    );
    let md_out = format_r05_markdown_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &math_cases,
        &render_cases,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write("docs/evidence/r05_drawing_engine_benchmark.json", json_out)
        .expect("write r05 json");
    std::fs::write("docs/evidence/r05_drawing_engine_benchmark.md", md_out)
        .expect("write r05 markdown");
    println!(
        "\nSaved docs/evidence/r05_drawing_engine_benchmark.json and docs/evidence/r05_drawing_engine_benchmark.md"
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_r05_workstation_render(
    candles: &[Candle],
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    workload_id: &'static str,
    description: &'static str,
    scale_mode: PriceScaleMode,
    vis_left: usize,
    vis_right: usize,
    mut setup_fn: impl FnMut(&mut ChartViewportState),
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut vp = ChartViewportState::new();
    vp.main_price_scale.set_mode(scale_mode);
    let mut store = build_r05_benchmark_drawings(candles, vis_left, vis_right);

    let cfg = ChartDisplayConfig {
        symbol: "VNINDEX",
        timeframe: "1D",
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_workstation_chart_with_drawings(
                        ui, candles, &mut vp, cfg, indicators, cache, &mut store,
                    );
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 120.0 + ((step % 37) as f32) * 24.0;
        let hover_y = 140.0 + ((step % 19) as f32) * 28.0;
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_workstation_chart_with_drawings(
                    ui, candles, &mut vp, cfg, indicators, cache, &mut store,
                );
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;
        paint_shapes = full_out.shapes.len();

        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }
        std::hint::black_box(&primitives);

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("r05 render report");
    assert_eq!(rep.drawing_count, 25);
    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r05_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    math: &[DrawingMathCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R05\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"drawing_geometry_and_hit_test\": [");
    for (idx, m) in math.iter().enumerate() {
        let comma = if idx + 1 == math.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"active_drawings\": {}, \"project_median_us\": {:.3}, \"project_p95_us\": {:.3}, \"hit_test_4panes_median_us\": {:.3}, \"hit_test_4panes_p95_us\": {:.3}, \"ohlc_magnet_8px_median_us\": {:.3}, \"ohlc_magnet_8px_p95_us\": {:.3}, \"ohlc_magnet_fit_all_median_us\": {:.3}, \"ohlc_magnet_fit_all_p95_us\": {:.3} }}{comma}",
            m.dataset_bars,
            m.active_drawings,
            m.project_stats.median_us,
            m.project_stats.p95_us,
            m.hit_test_stats.median_us,
            m.hit_test_stats.p95_us,
            m.magnet_snap_stats.median_us,
            m.magnet_snap_stats.p95_us,
            m.magnet_fit_stats.median_us,
            m.magnet_fit_stats.p95_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"multipane_drawing_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"active_drawings\": 25, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r05_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    math: &[DrawingMathCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R05 — Interactive Drawing & Multi-Pane Workstation Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r05`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** Main Candlestick + Volume + `SMA(20)` + `EMA(50)` + `BB(20, 2.0)` + 3 Stacked Oscillator Panes (`RSI(14)`, `MACD(12, 26, 9)`, `ATR(14)`) + **25 Active Multi-Pane Drawings** (`5` `HorizontalLine`, `5` `TrendLine`, `5` $360^\\circ$ `Ray`, `5` `Rectangle`, `5` `FibonacciRetracement`)\n"
    );
    let _ = writeln!(
        s,
        "## 1. Pure `f64` Projection, Liang-Barsky Clipping, 4-Pane Hit-Testing & OHLC Magnet Throughput\n"
    );
    let _ = writeln!(
        s,
        "| Dataset Bars | Active Drawings | Project + Clip 25 Drawings (`µs`) | 4-Pane Hit-Test 25 Drawings (`µs`) | OHLC Magnet `bar_spacing = 8.0` (`µs`) | OHLC Magnet `fit_content` (`µs`) |"
    );
    let _ = writeln!(s, "| ---: | ---: | ---: | ---: | ---: | ---: |");
    for m in math {
        let _ = writeln!(
            s,
            "| `{}` | `{}` | **`{:.2} µs`** (P95 `{:.2} µs`) | **`{:.2} µs`** (P95 `{:.2} µs`) | **`{:.2} µs`** (P95 `{:.2} µs`) | **`{:.2} µs`** (P95 `{:.2} µs`) |",
            m.dataset_bars,
            m.active_drawings,
            m.project_stats.median_us,
            m.project_stats.p95_us,
            m.hit_test_stats.median_us,
            m.hit_test_stats.p95_us,
            m.magnet_snap_stats.median_us,
            m.magnet_snap_stats.p95_us,
            m.magnet_fit_stats.median_us,
            m.magnet_fit_stats.p95_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End 4-Pane + 6-Indicator + 25-Drawing Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

// ============================================================================
// Milestone R06 — CSV/JSON OHLCV Loader & Multi-Timeframe Aggregation Benchmark
// ============================================================================

#[derive(Debug, Clone)]
struct R06LoaderCaseResult {
    loader_id: &'static str,
    format: &'static str,
    raw_rows: usize,
    parsed_bars: usize,
    payload_bytes: usize,
    parse_stats: TimingStats,
}

#[derive(Debug, Clone)]
struct R06AggregationCaseResult {
    source_bars: usize,
    source_tf: &'static str,
    target_tf: &'static str,
    output_bars: usize,
    aggregate_stats: TimingStats,
    remap_25_drawings_stats: TimingStats,
}

fn build_r06_csv_document(candles: &[Candle]) -> String {
    let mut out = String::with_capacity(candles.len() * 64 + 64);
    out.push_str("time,open,high,low,close,volume\n");
    for c in candles {
        let dt = sumi_workstation::data::decompose_utc(c.timestamp);
        let _ = writeln!(
            out,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z,{:.2},{:.2},{:.2},{:.2},{:.0}",
            dt.year,
            dt.month,
            dt.day,
            dt.hour,
            dt.minute,
            dt.second,
            c.open,
            c.high,
            c.low,
            c.close,
            c.volume
        );
    }
    out
}

fn run_r06_data_timeframe_benchmark(
    cpu_brand: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    println!("=== Sumi R06 Data Loader & Multi-Timeframe Aggregation Benchmark ===");
    println!(
        "CPU: {cpu_brand} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let sizes = [10_000_usize, 50_000_usize, 100_000_usize];
    let mut loader_cases = Vec::new();
    let default_opts = DataLoadOptions::default();

    // 1. Streaming RFC-4180 CSV & RFC-8259 JSON Loader Throughput across 10K, 50K, 100K bars
    for &n in &sizes {
        let rep_1m = generate_intraday_vn30f_1m_report(n);
        let csv_doc = build_r06_csv_document(&rep_1m.candles);
        let json_doc = build_btcusdt_15m_json_document(n);
        let iters = if n >= 100_000 { 25 } else { 40 };
        let warmup = 5;

        let mut csv_samples = Vec::with_capacity(iters);
        let mut parsed_csv_bars = 0_usize;
        for step in 0..(warmup + iters) {
            let t0 = Instant::now();
            let loaded = load_ohlcv_csv(&csv_doc, &default_opts).expect("valid r06 csv");
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            parsed_csv_bars = loaded.candles.len();
            std::hint::black_box(&loaded.candles);
            if step >= warmup {
                csv_samples.push(us);
            }
        }
        loader_cases.push(R06LoaderCaseResult {
            loader_id: match n {
                10_000 => "csv_iso8601_10k",
                50_000 => "csv_iso8601_50k",
                _ => "csv_iso8601_100k",
            },
            format: "CSV (ISO-8601 1m)",
            raw_rows: n,
            parsed_bars: parsed_csv_bars,
            payload_bytes: csv_doc.len(),
            parse_stats: TimingStats::from_samples(csv_samples),
        });

        let mut json_samples = Vec::with_capacity(iters);
        let mut parsed_json_bars = 0_usize;
        for step in 0..(warmup + iters) {
            let t0 = Instant::now();
            let loaded = load_ohlcv_json(&json_doc, &default_opts).expect("valid r06 json");
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            parsed_json_bars = loaded.candles.len();
            std::hint::black_box(&loaded.candles);
            if step >= warmup {
                json_samples.push(us);
            }
        }
        loader_cases.push(R06LoaderCaseResult {
            loader_id: match n {
                10_000 => "json_kline_10k",
                50_000 => "json_kline_50k",
                _ => "json_kline_100k",
            },
            format: "JSON (Unix-ms kline)",
            raw_rows: n,
            parsed_bars: parsed_json_bars,
            payload_bytes: json_doc.len(),
            parse_stats: TimingStats::from_samples(json_samples),
        });
    }

    // Real Archive Fixtures (`CafeF.INDEX.clean.csv` & `SPY.csv`) if present in workspace
    if let Ok(cafef_csv) =
        std::fs::read_to_string("ARCHIVE/test-results/p0b/data/CafeF.INDEX.clean.csv")
    {
        let cafef_opts = DataLoadOptions {
            symbol_filter: Some("VNINDEX".to_owned()),
            ..DataLoadOptions::default()
        };
        let mut samples = Vec::with_capacity(80);
        let mut raw_rows = 0_usize;
        let mut parsed_bars = 0_usize;
        for step in 0..90 {
            let t0 = Instant::now();
            let rep = load_ohlcv_csv(&cafef_csv, &cafef_opts).expect("cafef csv");
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            raw_rows = rep.raw_row_count;
            parsed_bars = rep.candles.len();
            std::hint::black_box(&rep.candles);
            if step >= 10 {
                samples.push(us);
            }
        }
        loader_cases.push(R06LoaderCaseResult {
            loader_id: "archive_cafef_vnindex_csv",
            format: "CSV (CafeF Multi-Ticker)",
            raw_rows,
            parsed_bars,
            payload_bytes: cafef_csv.len(),
            parse_stats: TimingStats::from_samples(samples),
        });
    }

    if let Ok(spy_csv) =
        std::fs::read_to_string("ARCHIVE/research_repos/lightweight-charts-drawing/demo/SPY.csv")
    {
        let mut samples = Vec::with_capacity(80);
        let mut raw_rows = 0_usize;
        let mut parsed_bars = 0_usize;
        for step in 0..90 {
            let t0 = Instant::now();
            let rep = load_ohlcv_csv(&spy_csv, &default_opts).expect("spy csv");
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            raw_rows = rep.raw_row_count;
            parsed_bars = rep.candles.len();
            std::hint::black_box(&rep.candles);
            if step >= 10 {
                samples.push(us);
            }
        }
        loader_cases.push(R06LoaderCaseResult {
            loader_id: "archive_spy_5col_csv",
            format: "CSV (5-Col NYSE SPY)",
            raw_rows,
            parsed_bars,
            payload_bytes: spy_csv.len(),
            parse_stats: TimingStats::from_samples(samples),
        });
    }

    // 2. Multi-Timeframe Aggregation (`100,000` `1m` -> `5m`, `15m`, `1h`, `4h`, `1D`, `1W`)
    //    + 25-Drawing Exact Timestamp Remapping
    let base_100k_rep = generate_intraday_vn30f_1m_report(100_000);
    let candles_100k_1m = base_100k_rep.candles;
    let mut base_store_25 = build_r05_benchmark_drawings(
        &candles_100k_1m,
        candles_100k_1m.len().saturating_sub(600),
        candles_100k_1m.len() - 1,
    );
    // Stamp initial UTC timestamps on all 25 anchors at `1m`
    base_store_25.remap_timeframe(
        &candles_100k_1m,
        Timeframe::M1,
        &candles_100k_1m,
        Timeframe::M1,
    );

    let targets = [
        Timeframe::M5,
        Timeframe::M15,
        Timeframe::H1,
        Timeframe::H4,
        Timeframe::D1,
        Timeframe::W1,
    ];
    let mut agg_cases = Vec::new();
    for &target_tf in &targets {
        let mut agg_samples = Vec::with_capacity(80);
        let mut out_candles = Vec::new();
        for step in 0..90 {
            let t0 = Instant::now();
            let agg =
                aggregate_ohlcv(&candles_100k_1m, Timeframe::M1, target_tf).expect("valid rollup");
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            std::hint::black_box(&agg);
            if step == 0 {
                out_candles = agg;
            }
            if step >= 10 {
                agg_samples.push(us);
            }
        }

        let mut remap_samples = Vec::with_capacity(200);
        for step in 0..220 {
            let mut working_store = base_store_25.clone();
            let t0 = Instant::now();
            working_store.remap_timeframe(&candles_100k_1m, Timeframe::M1, &out_candles, target_tf);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            std::hint::black_box(&working_store.drawings);
            if step >= 20 {
                remap_samples.push(us);
            }
        }

        agg_cases.push(R06AggregationCaseResult {
            source_bars: candles_100k_1m.len(),
            source_tf: Timeframe::M1.label(),
            target_tf: target_tf.label(),
            output_bars: out_candles.len(),
            aggregate_stats: TimingStats::from_samples(agg_samples),
            remap_25_drawings_stats: TimingStats::from_samples(remap_samples),
        });
    }

    // 3. End-to-End 4-Pane + 6-Indicator + 25-Drawing Intraday & Aggregated Frame Budget (`< 4.0 ms` gate)
    let all_cfg = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };
    let mut cache_1m = IndicatorCache::new();
    cache_1m.sync(&candles_100k_1m, 1, &all_cfg);

    let candles_15m =
        aggregate_ohlcv(&candles_100k_1m, Timeframe::M1, Timeframe::M15).expect("15m agg");
    let mut cache_15m = IndicatorCache::new();
    cache_15m.sync(&candles_15m, 2, &all_cfg);
    let mut store_15m = base_store_25.clone();
    store_15m.remap_timeframe(
        &candles_100k_1m,
        Timeframe::M1,
        &candles_15m,
        Timeframe::M15,
    );

    let candles_1d =
        aggregate_ohlcv(&candles_100k_1m, Timeframe::M1, Timeframe::D1).expect("1d agg");
    let mut cache_1d = IndicatorCache::new();
    cache_1d.sync(&candles_1d, 3, &all_cfg);
    let mut store_1d = base_store_25.clone();
    store_1d.remap_timeframe(&candles_100k_1m, Timeframe::M1, &candles_1d, Timeframe::D1);

    let render_cases = vec![
        bench_r06_intraday_workstation_render(
            &candles_100k_1m,
            "VN30F1M",
            "1m",
            &all_cfg,
            &cache_1m,
            &base_store_25,
            "r06_100k_1m_recent_4pane_25draw",
            "100K 1m intraday window (bar_spacing=8.0, HH:MM/Day axis, 4 panes + 6 ind + 25 draw)",
            PriceScaleMode::Normal,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(8.0);
            },
            15,
            100,
        ),
        bench_r06_intraday_workstation_render(
            &candles_100k_1m,
            "VN30F1M",
            "1m",
            &all_cfg,
            &cache_1m,
            &base_store_25,
            "r06_100k_1m_fit_conflated_4pane_25draw",
            "100K 1m full-fit conflated (Logarithmic, 4 panes + 6 ind + 25 draw)",
            PriceScaleMode::Logarithmic,
            |vp| vp.fit_all(),
            15,
            80,
        ),
        bench_r06_intraday_workstation_render(
            &candles_15m,
            "VN30F1M",
            "15m",
            &all_cfg,
            &cache_15m,
            &store_15m,
            "r06_100k_to_15m_recent_4pane_25draw",
            "100K 1m aggregated to 15m + 25 remapped drawings (bar_spacing=8.0, Normal)",
            PriceScaleMode::Normal,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(8.0);
            },
            15,
            100,
        ),
        bench_r06_intraday_workstation_render(
            &candles_1d,
            "VN30F1M",
            "1D",
            &all_cfg,
            &cache_1d,
            &store_1d,
            "r06_100k_to_1d_fit_4pane_25draw",
            "100K 1m aggregated to 1D + 25 remapped drawings (fit_all, Logarithmic)",
            PriceScaleMode::Logarithmic,
            |vp| vp.fit_all(),
            15,
            100,
        ),
    ];

    println!("\n--- Streaming CSV & JSON Loader Throughput ---");
    for c in &loader_cases {
        let mb = (c.payload_bytes as f64) / (1024.0 * 1024.0);
        let med_ms = c.parse_stats.median_us / 1_000.0;
        let mb_per_s = if med_ms > 0.0 {
            mb / (med_ms / 1_000.0)
        } else {
            0.0
        };
        println!(
            "{:<26} | {:>7} bars ({:>6.2} MiB) | Median: {:>7.3} ms (P95 {:>7.3} ms) | {:>7.1} MiB/s",
            c.loader_id,
            c.parsed_bars,
            mb,
            med_ms,
            c.parse_stats.p95_us / 1_000.0,
            mb_per_s,
        );
    }

    println!("\n--- 100,000 x 1m Multi-Timeframe Aggregation + 25-Drawing Timestamp Remap ---");
    for a in &agg_cases {
        println!(
            "100K {} -> {:<3} ({:>6} bars) | Rollup Median: {:>7.3} ms (P95 {:>7.3} ms) | Remap 25 Drawings: {:>6.2} µs",
            a.source_tf,
            a.target_tf,
            a.output_bars,
            a.aggregate_stats.median_us / 1_000.0,
            a.aggregate_stats.p95_us / 1_000.0,
            a.remap_25_drawings_stats.median_us,
        );
    }

    println!(
        "\n{:<8} | {:<38} | {:>5} | {:>8} | {:>9} | {:>9} | {:>9}",
        "Bars", "Workload", "Confl", "Vertices", "UI (ms)", "Tess (ms)", "Total(ms)"
    );
    println!("{}", "-".repeat(104));
    for r in &render_cases {
        println!(
            "{:<8} | {:<38} | {:>4}x | {:>8} | {:>9.3} | {:>9.3} | {:>9.3}",
            r.dataset_bars,
            r.workload_id,
            r.conflation_factor,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
        );
    }

    let json_out = format_r06_json_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &loader_cases,
        &agg_cases,
        &render_cases,
    );
    let md_out = format_r06_markdown_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &loader_cases,
        &agg_cases,
        &render_cases,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write("docs/evidence/r06_data_timeframe_benchmark.json", json_out)
        .expect("write r06 json");
    std::fs::write("docs/evidence/r06_data_timeframe_benchmark.md", md_out)
        .expect("write r06 markdown");
    println!(
        "\nSaved docs/evidence/r06_data_timeframe_benchmark.json and docs/evidence/r06_data_timeframe_benchmark.md"
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_r06_intraday_workstation_render(
    candles: &[Candle],
    symbol: &'static str,
    timeframe: &'static str,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    initial_store: &DrawingStore,
    workload_id: &'static str,
    description: &'static str,
    scale_mode: PriceScaleMode,
    mut setup_fn: impl FnMut(&mut ChartViewportState),
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut vp = ChartViewportState::new();
    vp.main_price_scale.set_mode(scale_mode);
    let mut store = initial_store.clone();

    let cfg = ChartDisplayConfig {
        symbol,
        timeframe,
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_workstation_chart_with_drawings(
                        ui, candles, &mut vp, cfg, indicators, cache, &mut store,
                    );
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 140.0 + ((step % 35) as f32) * 24.0;
        let hover_y = if step % 2 == 0 { 220.0 } else { 520.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_workstation_chart_with_drawings(
                    ui, candles, &mut vp, cfg, indicators, cache, &mut store,
                );
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }
        std::hint::black_box(&primitives);

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("r06 render report");
    assert_eq!(rep.drawing_count, 25);
    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r06_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    loaders: &[R06LoaderCaseResult],
    aggregations: &[R06AggregationCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R06\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"loader_throughput\": [");
    for (idx, c) in loaders.iter().enumerate() {
        let comma = if idx + 1 == loaders.len() { "" } else { "," };
        let med_ms = c.parse_stats.median_us / 1_000.0;
        let mb = (c.payload_bytes as f64) / (1024.0 * 1024.0);
        let mb_s = if med_ms > 0.0 {
            mb / (med_ms / 1_000.0)
        } else {
            0.0
        };
        let _ = writeln!(
            s,
            "    {{ \"loader_id\": \"{}\", \"format\": \"{}\", \"raw_rows\": {}, \"parsed_bars\": {}, \"payload_bytes\": {}, \"median_ms\": {:.4}, \"p95_ms\": {:.4}, \"mib_per_sec\": {:.2} }}{comma}",
            c.loader_id,
            c.format,
            c.raw_rows,
            c.parsed_bars,
            c.payload_bytes,
            med_ms,
            c.parse_stats.p95_us / 1_000.0,
            mb_s,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"timeframe_aggregation\": [");
    for (idx, a) in aggregations.iter().enumerate() {
        let comma = if idx + 1 == aggregations.len() {
            ""
        } else {
            ","
        };
        let _ = writeln!(
            s,
            "    {{ \"source_bars\": {}, \"source_tf\": \"{}\", \"target_tf\": \"{}\", \"output_bars\": {}, \"aggregate_median_ms\": {:.4}, \"aggregate_p95_ms\": {:.4}, \"remap_25_drawings_median_us\": {:.3}, \"remap_25_drawings_p95_us\": {:.3} }}{comma}",
            a.source_bars,
            a.source_tf,
            a.target_tf,
            a.output_bars,
            a.aggregate_stats.median_us / 1_000.0,
            a.aggregate_stats.p95_us / 1_000.0,
            a.remap_25_drawings_stats.median_us,
            a.remap_25_drawings_stats.p95_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"workstation_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r06_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    loaders: &[R06LoaderCaseResult],
    aggregations: &[R06AggregationCaseResult],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R06 — CSV/JSON OHLCV Data Loader & Multi-Timeframe Aggregation Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r06`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** Pure-Rust Streaming RFC-4180 CSV & RFC-8259 JSON Loaders + `1m..1W` Upward Aggregation + `25` Timestamp-Locked Multi-Pane Drawings + 4-Pane 6-Indicator Adaptive Intraday/Daily Workstation\n"
    );

    let _ = writeln!(
        s,
        "## 1. Streaming RFC-4180 CSV & RFC-8259 JSON Loader Throughput (`10K`, `50K`, `100K` + Real Archive Fixtures)\n"
    );
    let _ = writeln!(
        s,
        "| Loader Case | Format | Raw Rows | Parsed Bars | Payload (`MiB`) | Parse Median (`ms`) | Parse P95 (`ms`) | Throughput (`MiB/s`) |"
    );
    let _ = writeln!(
        s,
        "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for c in loaders {
        let mb = (c.payload_bytes as f64) / (1024.0 * 1024.0);
        let med_ms = c.parse_stats.median_us / 1_000.0;
        let mb_s = if med_ms > 0.0 {
            mb / (med_ms / 1_000.0)
        } else {
            0.0
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` | `{}` | `{}` | `{:.2}` | **`{:.3} ms`** | `{:.3} ms` | **`{:.1} MiB/s`** |",
            c.loader_id,
            c.format,
            c.raw_rows,
            c.parsed_bars,
            mb,
            med_ms,
            c.parse_stats.p95_us / 1_000.0,
            mb_s,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. `100,000` `1m` Bars Multi-Timeframe Aggregation & 25-Drawing Exact Timestamp Remapping\n"
    );
    let _ = writeln!(
        s,
        "| Source (`100K`) | Target TF | Output Bars | Rollup Median (`ms`) | Rollup P95 (`ms`) | 25-Drawing Timestamp Remap (`µs`) |"
    );
    let _ = writeln!(s, "| :--- | :--- | ---: | ---: | ---: | ---: |");
    for a in aggregations {
        let _ = writeln!(
            s,
            "| `{} {}` | **`{}`** | `{}` | **`{:.3} ms`** | `{:.3} ms` | **`{:.2} µs`** (P95 `{:.2} µs`) |",
            a.source_bars,
            a.source_tf,
            a.target_tf,
            a.output_bars,
            a.aggregate_stats.median_us / 1_000.0,
            a.aggregate_stats.p95_us / 1_000.0,
            a.remap_25_drawings_stats.median_us,
            a.remap_25_drawings_stats.p95_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 3. End-to-End 4-Pane + 6-Indicator + 25-Drawing Intraday & Aggregated Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

// ============================================================================
// Milestone R07 — Causal Bar Replay & Paper Trading Execution Engine Benchmark
// ============================================================================

#[derive(Debug, Clone)]
struct R07DomainBenchCase {
    case_id: &'static str,
    description: &'static str,
    base_bars: usize,
    output_or_fills: usize,
    stats: TimingStats,
}

fn build_r07_benchmark_trading_session(
    base_candles: &[Candle],
    start_idx: usize,
    end_idx: usize,
) -> TradingSession {
    let mut session = TradingSession::default();
    session.reset_at(base_candles, start_idx);
    let total_span = end_idx.saturating_sub(start_idx).max(1);
    let stride = (total_span / 32).max(20);
    let mut cursor = start_idx;

    while cursor + stride <= end_idx {
        let mark = base_candles[cursor].close;
        if session.open_position().is_none() {
            let step_num = (cursor - start_idx) / stride;
            let side = if step_num.is_multiple_of(2) {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            let (sl, tp) = match side {
                OrderSide::Buy => (Some(mark * 0.992), Some(mark * 1.012)),
                OrderSide::Sell => (Some(mark * 1.008), Some(mark * 0.988)),
            };
            let _ = session.submit_order(
                OrderRequest {
                    side,
                    kind: OrderKind::Market,
                    quantity: 4.0,
                    stop_loss: sl,
                    take_profit: tp,
                },
                base_candles,
                cursor,
            );
            // Submit a working Limit add-on order as well so working order lines & matching run
            let limit_px = match side {
                OrderSide::Buy => mark * 0.996,
                OrderSide::Sell => mark * 1.004,
            };
            let _ = session.submit_order(
                OrderRequest {
                    side,
                    kind: OrderKind::Limit {
                        limit_price: limit_px,
                    },
                    quantity: 2.0,
                    stop_loss: sl,
                    take_profit: tp,
                },
                base_candles,
                cursor,
            );
        } else if ((cursor - start_idx) / stride).is_multiple_of(3) {
            let _ = session.flatten_position(base_candles, cursor);
        }

        let next_cursor = (cursor + stride).min(end_idx);
        session.advance_through_bars(base_candles, cursor + 1, next_cursor);
        cursor = next_cursor;
    }

    // Ensure an active bracketed position + working limit order are present at `end_idx` for HUD/line rendering
    let final_mark = base_candles[end_idx].close;
    if session.open_position().is_none() {
        let _ = session.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 5.0,
                stop_loss: Some(final_mark * 0.994),
                take_profit: Some(final_mark * 1.009),
            },
            base_candles,
            end_idx,
        );
    }
    let pending_count = session.pending_orders().count();
    if pending_count == 0 {
        let _ = session.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Limit {
                    limit_price: final_mark * 0.997,
                },
                quantity: 3.0,
                stop_loss: None,
                take_profit: None,
            },
            base_candles,
            end_idx,
        );
    }

    session
}

fn run_r07_replay_trading_benchmark(
    cpu_brand: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    println!("=== Sumi R07 Causal Bar Replay & Paper Trading Execution Engine Benchmark ===");
    println!(
        "CPU: {cpu_brand} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let base_100k_rep = generate_intraday_vn30f_1m_report(100_000);
    let candles_100k_1m = base_100k_rep.candles;
    let cut_start_idx = 90_000_usize;
    let cut_end_idx = 95_000_usize;

    let mut domain_cases = Vec::new();

    // 1A. Causal `materialize_candles` at cut_end_idx (`95,001` 1m -> `15m` zero-lookahead rollup)
    {
        let mut replay = ReplayController::new();
        replay.start_at_base_index(&candles_100k_1m, cut_end_idx);
        let mut samples = Vec::with_capacity(80);
        let mut out_len = 0_usize;
        for step in 0..90 {
            let t0 = Instant::now();
            let sliced = replay
                .materialize_candles(&candles_100k_1m, Timeframe::M1, Timeframe::M15)
                .expect("causal 15m materialize");
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            out_len = sliced.len();
            std::hint::black_box(&sliced);
            if step >= 10 {
                samples.push(us);
            }
        }
        domain_cases.push(R07DomainBenchCase {
            case_id: "replay_materialize_95k_1m_to_15m",
            description: "Causal zero-lookahead materialize (`95,001` 1m -> `15m`)",
            base_bars: cut_end_idx + 1,
            output_or_fills: out_len,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 1B. Causal `ActiveBar` (`15m`) step-forward + `TradingSession::advance_through_bars`
    {
        let base_session =
            build_r07_benchmark_trading_session(&candles_100k_1m, cut_start_idx, cut_end_idx);
        let mut samples = Vec::with_capacity(200);
        let mut fills_after = 0_usize;
        for step in 0..220 {
            let mut replay = ReplayController::new();
            replay.step_unit = ReplayStepUnit::ActiveBar;
            replay.start_at_base_index(&candles_100k_1m, cut_end_idx);
            let mut session = base_session.clone();
            let t0 = Instant::now();
            let tr = replay
                .step_forward(&candles_100k_1m, Timeframe::M15, 1)
                .expect("step forward 1x 15m");
            session.advance_through_bars(
                &candles_100k_1m,
                tr.prev_base_index + 1,
                tr.new_base_index,
            );
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            fills_after = session.fills().len();
            std::hint::black_box((replay.base_cursor_index, session.cash_balance()));
            if step >= 20 {
                samples.push(us);
            }
        }
        domain_cases.push(R07DomainBenchCase {
            case_id: "replay_step_15m_plus_order_matching",
            description: "`1x 15m` ActiveBar step + intrabar OHLC bracket/limit matching",
            base_bars: 15,
            output_or_fills: fills_after,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 1C. Full 5,000-bar `advance_through_bars` simulation (`90,000..=95,000` with ~64 commands)
    {
        let mut samples = Vec::with_capacity(120);
        let mut fill_count = 0_usize;
        for step in 0..135 {
            let t0 = Instant::now();
            let session =
                build_r07_benchmark_trading_session(&candles_100k_1m, cut_start_idx, cut_end_idx);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            fill_count = session.fills().len();
            std::hint::black_box(&session);
            if step >= 15 {
                samples.push(us);
            }
        }
        domain_cases.push(R07DomainBenchCase {
            case_id: "trading_sim_5000_1m_bars_full",
            description: "5,000-bar `1m` causal paper-trading run (~65 orders + SL/TP brackets)",
            base_bars: cut_end_idx - cut_start_idx + 1,
            output_or_fills: fill_count,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 1D. Deterministic `rewind_to` across 4,500 1m bars (`95,000 -> 94,500`, replaying `90,000..=94,500`)
    {
        let base_session =
            build_r07_benchmark_trading_session(&candles_100k_1m, cut_start_idx, cut_end_idx);
        let rewind_target = cut_end_idx - 500;
        let mut samples = Vec::with_capacity(120);
        let mut rewound_fills = 0_usize;
        for step in 0..135 {
            let mut session = base_session.clone();
            let t0 = Instant::now();
            session.rewind_to(&candles_100k_1m, cut_start_idx, rewind_target);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            rewound_fills = session.fills().len();
            std::hint::black_box((session.cash_balance(), session.closed_trades().len()));
            if step >= 15 {
                samples.push(us);
            }
        }
        domain_cases.push(R07DomainBenchCase {
            case_id: "trading_causal_rewind_4500_bars",
            description: "Bit-exact command-log `rewind_to` (`90,000..=94,500` rebuild)",
            base_bars: rewind_target - cut_start_idx + 1,
            output_or_fills: rewound_fills,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 2. End-to-End 4-Pane + 6-Indicator + 25-Drawing + Active Replay + Paper Trading Overlays (`< 4.0 ms` gate)
    let all_cfg = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };

    let mut replay_70k = ReplayController::new();
    let cutoff_ts = replay_70k
        .start_at_base_index(&candles_100k_1m, cut_end_idx)
        .expect("cutoff ts");
    let trading_70k =
        build_r07_benchmark_trading_session(&candles_100k_1m, cut_start_idx, cut_end_idx);

    let candles_70k_1m = replay_70k
        .materialize_candles(&candles_100k_1m, Timeframe::M1, Timeframe::M1)
        .expect("70k 1m slice");
    let mut cache_70k_1m = IndicatorCache::new();
    cache_70k_1m.sync(&candles_70k_1m, 1, &all_cfg);
    let mut store_70k_1m = build_r05_benchmark_drawings(
        &candles_70k_1m,
        candles_70k_1m.len().saturating_sub(500),
        candles_70k_1m.len() - 1,
    );
    store_70k_1m.remap_timeframe(
        &candles_70k_1m,
        Timeframe::M1,
        &candles_70k_1m,
        Timeframe::M1,
    );
    store_70k_1m.set_replay_cutoff_timestamp(Some(cutoff_ts), candles_70k_1m.len());

    let candles_70k_15m = replay_70k
        .materialize_candles(&candles_100k_1m, Timeframe::M1, Timeframe::M15)
        .expect("70k 15m slice");
    let mut cache_70k_15m = IndicatorCache::new();
    cache_70k_15m.sync(&candles_70k_15m, 2, &all_cfg);
    let mut store_70k_15m = store_70k_1m.clone();
    store_70k_15m.remap_timeframe(
        &candles_70k_1m,
        Timeframe::M1,
        &candles_70k_15m,
        Timeframe::M15,
    );
    store_70k_15m.set_replay_cutoff_timestamp(Some(cutoff_ts), candles_70k_15m.len());

    let render_cases = vec![
        bench_r07_workstation_render(
            &candles_70k_1m,
            "VN30F1M",
            "1m",
            &all_cfg,
            &cache_70k_1m,
            &store_70k_1m,
            &trading_70k,
            true,
            false,
            "r07_replay_1m_recent_4pane_25draw_trading",
            "95K/100K 1m Replay + 4 panes + 6 ind + 25 draw + Position/SL/TP/Fills (bar_spacing=8.0)",
            PriceScaleMode::Normal,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(8.0);
                vp.time_scale.set_right_offset(8.0);
            },
            15,
            100,
        ),
        bench_r07_workstation_render(
            &candles_70k_15m,
            "VN30F1M",
            "15m",
            &all_cfg,
            &cache_70k_15m,
            &store_70k_15m,
            &trading_70k,
            true,
            true,
            "r07_replay_15m_cut_scrubber_4pane_25draw_trading",
            "95K/100K -> 15m Replay + Cut Scrubber + 4 panes + 6 ind + 25 draw + Fills (bar_spacing=9.0)",
            PriceScaleMode::Normal,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(9.0);
                vp.time_scale.set_right_offset(8.0);
            },
            15,
            100,
        ),
        bench_r07_workstation_render(
            &candles_70k_1m,
            "VN30F1M",
            "1m",
            &all_cfg,
            &cache_70k_1m,
            &store_70k_1m,
            &trading_70k,
            true,
            false,
            "r07_replay_1m_fit_conflated_4pane_25draw_trading",
            "95K/100K 1m full-fit conflated + Replay Frontier + Trading Overlays (Logarithmic)",
            PriceScaleMode::Logarithmic,
            |vp| vp.fit_all(),
            15,
            80,
        ),
        bench_r07_full_app_ui_100k(15, 80),
    ];

    println!("\n--- Causal Bar Replay & Paper Trading Domain Throughput ---");
    for d in &domain_cases {
        println!(
            "{:<38} | base={:>6} | out/fills={:>5} | Median: {:>8.2} µs (P95 {:>8.2} µs)",
            d.case_id, d.base_bars, d.output_or_fills, d.stats.median_us, d.stats.p95_us,
        );
    }

    println!(
        "\n{:<8} | {:<48} | {:>5} | {:>8} | {:>9} | {:>9} | {:>9}",
        "Bars", "Workload", "Confl", "Vertices", "UI (ms)", "Tess (ms)", "Total(ms)"
    );
    println!("{}", "-".repeat(114));
    for r in &render_cases {
        println!(
            "{:<8} | {:<48} | {:>4}x | {:>8} | {:>9.3} | {:>9.3} | {:>9.3}",
            r.dataset_bars,
            r.workload_id,
            r.conflation_factor,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
        );
    }

    let json_out = format_r07_json_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );
    let md_out = format_r07_markdown_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write("docs/evidence/r07_replay_trading_benchmark.json", json_out)
        .expect("write r07 json");
    std::fs::write("docs/evidence/r07_replay_trading_benchmark.md", md_out)
        .expect("write r07 markdown");
    println!(
        "\nSaved docs/evidence/r07_replay_trading_benchmark.json and docs/evidence/r07_replay_trading_benchmark.md"
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_r07_workstation_render(
    candles: &[Candle],
    symbol: &'static str,
    timeframe: &'static str,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    initial_store: &DrawingStore,
    trading: &TradingSession,
    replay_active: bool,
    selecting_cut_bar: bool,
    workload_id: &'static str,
    description: &'static str,
    scale_mode: PriceScaleMode,
    mut setup_fn: impl FnMut(&mut ChartViewportState),
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut vp = ChartViewportState::new();
    vp.main_price_scale.set_mode(scale_mode);
    let mut store = initial_store.clone();

    let cfg = ChartDisplayConfig {
        symbol,
        timeframe,
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_workstation_chart_full(
                        ui,
                        candles,
                        &mut vp,
                        cfg,
                        indicators,
                        cache,
                        &mut store,
                        ChartReplayTradingOverlay {
                            replay_active,
                            selecting_cut_bar,
                            replay_status_label: Some("REPLAY PAUSED · 15m"),
                            trading_session: Some(trading),
                            ..Default::default()
                        },
                    );
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 180.0 + ((step % 30) as f32) * 24.0;
        let hover_y = if step % 2 == 0 { 220.0 } else { 520.0 };
        // Clone store only when `selecting_cut_bar` would not mutate it (we send PointerMoved only,
        // never PointerButton click, so `selecting_cut_bar` remains `true` across all iterations).
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_workstation_chart_full(
                    ui,
                    candles,
                    &mut vp,
                    cfg,
                    indicators,
                    cache,
                    &mut store,
                    ChartReplayTradingOverlay {
                        replay_active,
                        selecting_cut_bar,
                        replay_status_label: Some("REPLAY PAUSED · 15m"),
                        trading_session: Some(trading),
                        ..Default::default()
                    },
                );
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("r07 render report");
    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn bench_r07_full_app_ui_100k(warmup: usize, iters: usize) -> RenderCaseResult {
    let ctx = egui::Context::default();
    SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut app = SumiWorkstationApp::default();
    let rep_100k = generate_intraday_vn30f_1m_report(100_000);
    let csv_100k = build_r06_csv_document(&rep_100k.candles);
    app.load_from_csv_str(&csv_100k, &DataLoadOptions::default())
        .expect("load 100k csv into app");
    app.set_timeframe(Timeframe::M15).expect("15m rollup");
    app.indicators = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };
    app.show_replay_bar = true;
    app.show_trading_panel = true;

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        init_out.textures_delta.clear();
    }

    assert!(app.start_replay_at_base_index(68_000));
    app.populate_showcase_drawings();
    let mark = app.current_mark_price();
    app.ticket_qty_input = "5".to_owned();
    app.ticket_sl_input = format!("{:.2}", mark * 0.994);
    app.ticket_tp_input = format!("{:.2}", mark * 1.009);
    let _ = app.submit_ticket_order(OrderSide::Buy);
    let _ = app.step_replay_forward(8);
    app.viewport.time_scale.set_bar_spacing(8.5);
    app.viewport.time_scale.set_right_offset(8.0);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 180.0 + ((step % 25) as f32) * 20.0;
        let hover_y = if step % 2 == 0 { 250.0 } else { 520.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            app.render_workstation_ui(ui);
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = app.last_report.expect("full app r07 report");
    RenderCaseResult {
        dataset_bars: app.candles.len(),
        workload_id: "r07_full_workstation_ui_dock_ticket_4pane",
        description: "Full SumiWorkstationApp (Toolbar + Replay Dock + Right Trading Panel + 4 Panes + Overlays)",
        scale_mode: app.viewport.main_price_scale.mode().label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r07_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R07DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R07\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"replay_and_trading_domain\": [");
    for (idx, d) in domain.iter().enumerate() {
        let comma = if idx + 1 == domain.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"case_id\": \"{}\", \"description\": \"{}\", \"base_bars\": {}, \"output_or_fills\": {}, \"median_us\": {:.3}, \"p95_us\": {:.3}, \"max_us\": {:.3} }}{comma}",
            d.case_id,
            d.description.replace('"', "\\\""),
            d.base_bars,
            d.output_or_fills,
            d.stats.median_us,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"workstation_replay_trading_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r07_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R07DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R07 — Causal Bar Replay & Paper Trading Execution Engine Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r07`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** `100,000` `1m` Base Bars (`VN30F1M`) + Causal Replay Cut (`90,000..=95,000`) + Paper Trading Portfolio (`Market`/`Limit`/`SL`/`TP` Brackets + Execution Fill Markers) + 4 Panes (`Main` + `RSI(14)` + `MACD(12,26,9)` + `ATR(14)`) + 6 Indicators + 25 Causal Drawings + Replay Dock & Right Trading Panel\n"
    );

    let _ = writeln!(
        s,
        "## 1. Causal Replay Slicing, Step-Forward Matching, 5,000-Bar Simulation & Bit-Exact Rewind\n"
    );
    let _ = writeln!(
        s,
        "| Case ID | Description | Base Bars | Output Bars / Fills | Median (`µs`) | P95 (`µs`) | Max (`µs`) |"
    );
    let _ = writeln!(s, "| :--- | :--- | ---: | ---: | ---: | ---: | ---: |");
    for d in domain {
        let _ = writeln!(
            s,
            "| `{}` | {} | `{}` | `{}` | **`{:.2} µs`** | `{:.2} µs` | `{:.2} µs` |",
            d.case_id,
            d.description,
            d.base_bars,
            d.output_or_fills,
            d.stats.median_us,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End 4-Pane + 6-Indicator + 25-Drawing + Replay + Paper Trading Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

#[derive(Debug, Clone)]
struct R08DomainBenchCase {
    case_id: &'static str,
    description: &'static str,
    input_units: usize,
    output_items: usize,
    stats: TimingStats,
}

fn build_r08_benchmark_alert_engine(
    base_candles: &[Candle],
    active_candles: &[Candle],
    active_tf: Timeframe,
    arm_idx: usize,
    eval_idx: usize,
    drawings: &DrawingStore,
) -> AlertEngine {
    let mut engine = AlertEngine::new();
    let arm_close = base_candles[arm_idx].close;
    let arm_cut = Some(base_candles[arm_idx].timestamp);

    let _ = engine.add_alert(
        "VN30F1M",
        active_tf,
        AlertCondition::PriceLevel {
            price: arm_close * 1.002,
            direction: CrossDirection::CrossAbove,
        },
        AlertFrequency::Once,
        None,
        base_candles,
        arm_idx,
        arm_cut,
        Some(drawings),
    );
    let _ = engine.add_alert(
        "VN30F1M",
        active_tf,
        AlertCondition::PriceLevel {
            price: arm_close * 0.996,
            direction: CrossDirection::CrossBelow,
        },
        AlertFrequency::OncePerBarClose,
        None,
        base_candles,
        arm_idx,
        arm_cut,
        Some(drawings),
    );
    let _ = engine.add_alert(
        "VN30F1M",
        active_tf,
        AlertCondition::RsiThreshold {
            period: 14,
            level: 32.0,
            direction: CrossDirection::CrossAbove,
        },
        AlertFrequency::OncePerBarClose,
        None,
        base_candles,
        arm_idx,
        arm_cut,
        Some(drawings),
    );
    let _ = engine.add_alert(
        "VN30F1M",
        active_tf,
        AlertCondition::MacdHistogramZeroCross {
            fast: 12,
            slow: 26,
            signal: 9,
            direction: CrossDirection::CrossAny,
        },
        AlertFrequency::OncePerBarClose,
        None,
        base_candles,
        arm_idx,
        arm_cut,
        Some(drawings),
    );
    if let Some(ray_d) = drawings.drawings.iter().find(|d| {
        d.pane == DrawingPaneTarget::Main
            && matches!(
                d.geometry,
                DrawingGeometry::Ray { .. }
                    | DrawingGeometry::TrendLine { .. }
                    | DrawingGeometry::HorizontalLine { .. }
            )
    }) {
        let _ = engine.add_alert(
            "VN30F1M",
            active_tf,
            AlertCondition::DrawingLine {
                drawing_id: ray_d.id,
                direction: CrossDirection::CrossAny,
            },
            AlertFrequency::OncePerBarClose,
            None,
            base_candles,
            arm_idx,
            arm_cut,
            Some(drawings),
        );
    }

    let eval_cut = Some(base_candles[eval_idx].timestamp);
    engine.evaluate_causal(
        base_candles,
        Timeframe::M1,
        active_candles,
        active_tf,
        eval_idx,
        eval_cut,
        drawings,
    );
    engine
}

fn run_r08_scanner_alerts_palette_benchmark(
    cpu_brand: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    println!("=========================================================================");
    println!(" SUMI R08 WATCHLIST SCANNER, CAUSAL ALERTS & CMD+K PALETTE BENCHMARK");
    println!(" CPU      : {cpu_brand}");
    println!(
        " RAM      : {:.1} GiB",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    println!(" Toolchain: {rustc_ver} ({profile})");
    println!("=========================================================================\n");

    let mut domain_cases = Vec::new();

    // 1A. 13-Symbol Multi-Asset Watchlist Snapshot + Historical Signal Scan (`lookback = 80`)
    let universe = load_bundled_watchlist_universe();
    let total_universe_bars: usize = universe.iter().map(|e| e.candles.len()).sum();
    {
        let filter = ScannerFilter::default();
        let mut samples = Vec::with_capacity(80);
        let mut out_items = 0_usize;
        for step in 0..92 {
            let t0 = Instant::now();
            let mut rows = Vec::with_capacity(universe.len());
            let mut hits = Vec::new();
            for entry in &universe {
                let (row_opt, entry_hits) = evaluate_symbol_scan_and_signals(
                    &entry.instrument_id,
                    &entry.symbol,
                    entry.preset.short_label(),
                    entry.base_timeframe,
                    &entry.candles,
                    None,
                    80,
                    false,
                );
                if let Some(r) = row_opt {
                    rows.push(r);
                }
                hits.extend(entry_hits);
            }
            let sorted =
                filter_and_sort_watchlist(&rows, &filter, WatchlistSortColumn::ChangePct, false);
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            out_items = sorted.len() + hits.len();
            std::hint::black_box((sorted.len(), hits.len()));
            if step >= 12 {
                samples.push(us);
            }
        }
        domain_cases.push(R08DomainBenchCase {
            case_id: "scanner_13_symbols_watchlist_and_signals",
            description: "13-symbol multi-asset Watchlist snapshot + 80-bar Historical Signal scan + sort",
            input_units: total_universe_bars,
            output_items: out_items,
            stats: TimingStats::from_samples(samples),
        });
    }

    // Prepare 100K `1m` series (`90,000..=95,000` active causal window) for AlertEngine & Workstation benchmarks
    let rep_100k = generate_intraday_vn30f_1m_report(100_000);
    let candles_100k_1m = rep_100k.candles;
    let cut_start_idx = 90_000_usize;
    let cut_end_idx = 95_000_usize;

    let mut replay_95k = ReplayController::new();
    let cutoff_ts = replay_95k
        .start_at_base_index(&candles_100k_1m, cut_end_idx)
        .expect("cutoff ts");
    let candles_95k_1m = replay_95k
        .materialize_candles(&candles_100k_1m, Timeframe::M1, Timeframe::M1)
        .expect("95k 1m slice");
    let mut store_95k_1m = build_r05_benchmark_drawings(
        &candles_95k_1m,
        candles_95k_1m.len().saturating_sub(500),
        candles_95k_1m.len() - 1,
    );
    store_95k_1m.remap_timeframe(
        &candles_95k_1m,
        Timeframe::M1,
        &candles_95k_1m,
        Timeframe::M1,
    );
    store_95k_1m.set_replay_cutoff_timestamp(Some(cutoff_ts), candles_95k_1m.len());

    // 1B. Causal `AlertEngine::evaluate_causal` across 5,000 `1m` bars (`PriceLevel` + `RSI` + `MACD` + `DrawingLine`)
    let base_alerts = build_r08_benchmark_alert_engine(
        &candles_100k_1m,
        &candles_95k_1m,
        Timeframe::M1,
        cut_start_idx,
        cut_end_idx,
        &store_95k_1m,
    );
    {
        let mut samples = Vec::with_capacity(80);
        let mut trig_cnt = 0_usize;
        for step in 0..92 {
            let mut eng = base_alerts.clone();
            let t0 = Instant::now();
            eng.evaluate_causal(
                &candles_100k_1m,
                Timeframe::M1,
                &candles_95k_1m,
                Timeframe::M1,
                cut_end_idx,
                Some(cutoff_ts),
                &store_95k_1m,
            );
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            trig_cnt = eng.triggers.len();
            std::hint::black_box(trig_cnt);
            if step >= 12 {
                samples.push(us);
            }
        }
        domain_cases.push(R08DomainBenchCase {
            case_id: "alerts_eval_5000_1m_bars_5_rules",
            description: "Causal evaluation of 5 rules (`Price`×2 + `RSI` + `MACD` + `DrawingLine`) on 95K `1m` series",
            input_units: cut_end_idx - cut_start_idx + 1,
            output_items: trig_cnt,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 1C. Causal Replay Rewind (`95,000 -> 92,000`) re-arming `Once` rules & purging future triggers
    {
        let rewind_idx = 92_000_usize;
        let rewind_cut = candles_100k_1m[rewind_idx].timestamp;
        let candles_92k_1m = &candles_100k_1m[..=rewind_idx];
        let mut samples = Vec::with_capacity(80);
        let mut rewound_trigs = 0_usize;
        for step in 0..92 {
            let mut eng = base_alerts.clone();
            let t0 = Instant::now();
            eng.evaluate_causal(
                &candles_100k_1m,
                Timeframe::M1,
                candles_92k_1m,
                Timeframe::M1,
                rewind_idx,
                Some(rewind_cut),
                &store_95k_1m,
            );
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            rewound_trigs = eng.triggers.len();
            std::hint::black_box((rewound_trigs, eng.armed_count(Some(rewind_cut))));
            if step >= 12 {
                samples.push(us);
            }
        }
        domain_cases.push(R08DomainBenchCase {
            case_id: "alerts_causal_rewind_rearm_once",
            description: "Causal Replay rewind (`95,000 -> 92,000`) re-arming `Once` alerts & rebuilding trigger log",
            input_units: rewind_idx - cut_start_idx + 1,
            output_items: rewound_trigs,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 1D. `Cmd+K` Command Palette Candidate Generation & Fuzzy Ranking
    {
        let mut palette_app = SumiWorkstationApp::default();
        palette_app.ensure_watchlist_universe_loaded();
        palette_app.sync_scanner_caches();
        let queries = ["", "fpt", "15m", "macd", "alert 1295"];
        let mut samples = Vec::with_capacity(150);
        let mut cand_count = 0_usize;
        for step in 0..170 {
            palette_app.command_palette.query = queries[step % queries.len()].to_owned();
            let t0 = Instant::now();
            let cands = palette_app.build_command_palette_candidates();
            let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
            cand_count = cands.len();
            std::hint::black_box(cand_count);
            if step >= 20 {
                samples.push(us);
            }
        }
        domain_cases.push(R08DomainBenchCase {
            case_id: "command_palette_fuzzy_rank_cycle",
            description: "`Cmd+K` candidate generation & fuzzy ranking across symbol, timeframe, indicator & quick-alert queries",
            input_units: 42,
            output_items: cand_count,
            stats: TimingStats::from_samples(samples),
        });
    }

    // 2. End-to-End Render Workloads (`< 4.0 ms` Hybrid Gate across all 4!)
    let all_cfg = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };
    let mut cache_95k_1m = IndicatorCache::new();
    cache_95k_1m.sync(&candles_95k_1m, 1, &all_cfg);
    let trading_95k =
        build_r07_benchmark_trading_session(&candles_100k_1m, cut_start_idx, cut_end_idx);

    let render_cases = vec![
        bench_r08_chart_render(
            &candles_95k_1m,
            "VN30F1M",
            "1m",
            &all_cfg,
            &cache_95k_1m,
            &store_95k_1m,
            &trading_95k,
            &base_alerts,
            Some(cutoff_ts),
            "r08_chart_1m_recent_4pane_25draw_trading_alerts",
            "95K 1m Replay + 4 panes + 6 ind + 25 draw + Trading + Active/Triggered Alerts (bar_spacing=8.0)",
            PriceScaleMode::Normal,
            |vp| {
                vp.reset_to_default();
                vp.time_scale.set_bar_spacing(8.0);
                vp.time_scale.set_right_offset(8.0);
            },
            15,
            100,
        ),
        bench_r08_chart_render(
            &candles_95k_1m,
            "VN30F1M",
            "1m",
            &all_cfg,
            &cache_95k_1m,
            &store_95k_1m,
            &trading_95k,
            &base_alerts,
            Some(cutoff_ts),
            "r08_chart_1m_fit_conflated_4pane_trading_alerts",
            "95K 1m full-fit conflated + Replay + Trading + Batched Alert Diamonds & Lines (Logarithmic)",
            PriceScaleMode::Logarithmic,
            |vp| vp.fit_all(),
            15,
            80,
        ),
        bench_r08_full_workstation_ui(
            &candles_100k_1m,
            false,
            "r08_full_workstation_watchlist_alerts_4pane",
            "Full SumiWorkstationApp (Left 13-Sym Watchlist + Right Alerts Dock + Replay + 4 Panes + 100K Feed)",
            15,
            80,
        ),
        bench_r08_full_workstation_ui(
            &candles_100k_1m,
            true,
            "r08_full_workstation_cmd_k_modal_open",
            "Full SumiWorkstationApp + Floating `Cmd+K` Command Palette Modal (`alert 1295`) over 4-Pane Chart",
            15,
            80,
        ),
    ];

    println!("\n--- Multi-Symbol Scanner, Causal Alerts & Command Palette Domain Throughput ---");
    for d in &domain_cases {
        println!(
            "{:<42} | in={:>6} | out={:>4} | Median: {:>8.2} µs (P95 {:>8.2} µs)",
            d.case_id, d.input_units, d.output_items, d.stats.median_us, d.stats.p95_us,
        );
    }

    println!(
        "\n{:<8} | {:<48} | {:>5} | {:>8} | {:>9} | {:>9} | {:>9}",
        "Bars", "Workload", "Confl", "Vertices", "UI (ms)", "Tess (ms)", "Total(ms)"
    );
    println!("{}", "-".repeat(114));
    for r in &render_cases {
        println!(
            "{:<8} | {:<48} | {:>4}x | {:>8} | {:>9.3} | {:>9.3} | {:>9.3}",
            r.dataset_bars,
            r.workload_id,
            r.conflation_factor,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
        );
    }

    let json_out = format_r08_json_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );
    let md_out = format_r08_markdown_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write("docs/evidence/r08_scanner_alerts_benchmark.json", json_out)
        .expect("write r08 json");
    std::fs::write("docs/evidence/r08_scanner_alerts_benchmark.md", md_out)
        .expect("write r08 markdown");
    println!(
        "\nSaved docs/evidence/r08_scanner_alerts_benchmark.json and docs/evidence/r08_scanner_alerts_benchmark.md"
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_r08_chart_render(
    candles: &[Candle],
    symbol: &'static str,
    timeframe: &'static str,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    initial_store: &DrawingStore,
    trading: &TradingSession,
    alerts: &AlertEngine,
    replay_cutoff_timestamp: Option<i64>,
    workload_id: &'static str,
    description: &'static str,
    scale_mode: PriceScaleMode,
    mut setup_fn: impl FnMut(&mut ChartViewportState),
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut vp = ChartViewportState::new();
    vp.main_price_scale.set_mode(scale_mode);
    let mut store = initial_store.clone();
    let cfg = ChartDisplayConfig {
        symbol,
        timeframe,
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_workstation_chart_full(
                        ui,
                        candles,
                        &mut vp,
                        cfg,
                        indicators,
                        cache,
                        &mut store,
                        ChartReplayTradingOverlay {
                            replay_active: true,
                            selecting_cut_bar: false,
                            replay_status_label: Some("PAUSED"),
                            trading_session: Some(trading),
                            alert_engine: Some(alerts),
                            backtest_result: None,
                            equity_override: None,
                            show_equity_pane: false,
                            show_backtest_trades_on_chart: false,
                            focused_backtest_trade_id: None,
                            replay_cutoff_timestamp,
                            frontier_base_timestamp: replay_cutoff_timestamp
                                .or_else(|| candles.last().map(|c| c.timestamp)),
                            modal_open: false,
                            ..ChartReplayTradingOverlay::default()
                        },
                    );
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 180.0 + ((step % 30) as f32) * 24.0;
        let hover_y = if step % 2 == 0 { 220.0 } else { 520.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_workstation_chart_full(
                    ui,
                    candles,
                    &mut vp,
                    cfg,
                    indicators,
                    cache,
                    &mut store,
                    ChartReplayTradingOverlay {
                        replay_active: true,
                        selecting_cut_bar: false,
                        replay_status_label: Some("PAUSED"),
                        trading_session: Some(trading),
                        alert_engine: Some(alerts),
                        backtest_result: None,
                        equity_override: None,
                        show_equity_pane: false,
                        show_backtest_trades_on_chart: false,
                        focused_backtest_trade_id: None,
                        replay_cutoff_timestamp,
                        frontier_base_timestamp: replay_cutoff_timestamp
                            .or_else(|| candles.last().map(|c| c.timestamp)),
                        modal_open: false,
                        ..ChartReplayTradingOverlay::default()
                    },
                );
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("r08 chart render report");
    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn bench_r08_full_workstation_ui(
    candles_100k_1m: &[Candle],
    open_cmd_k_modal: bool,
    workload_id: &'static str,
    description: &'static str,
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut app = SumiWorkstationApp::default();
    let csv_100k = build_r06_csv_document(candles_100k_1m);
    app.load_from_csv_str(&csv_100k, &DataLoadOptions::default())
        .expect("load 100k csv into app");
    app.set_timeframe(Timeframe::M15).expect("15m rollup");
    app.indicators = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };

    assert!(app.start_replay_at_base_index(68_000));
    app.populate_showcase_drawings();
    let mark = app.current_mark_price();
    let _ = app.add_quick_price_alert(mark * 1.003, CrossDirection::CrossAbove);
    let _ = app.add_quick_price_alert(mark * 0.996, CrossDirection::CrossBelow);
    let _ = app.step_replay_forward(10);

    app.show_replay_bar = true;
    app.show_scanner_dock = !open_cmd_k_modal;
    app.scanner_tab = ScannerDockTab::Watchlist;
    app.show_trading_panel = true;
    app.show_alerts_panel = true;
    app.right_dock_tab = RightDockTab::Alerts;
    app.viewport.time_scale.set_bar_spacing(8.5);
    app.viewport.time_scale.set_right_offset(8.0);

    if open_cmd_k_modal {
        app.open_command_palette();
        app.command_palette.query = format!("alert {:.0}", mark * 1.005);
    }

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        init_out.textures_delta.clear();
    }

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 340.0 + ((step % 20) as f32) * 18.0;
        let hover_y = if step % 2 == 0 { 260.0 } else { 520.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            app.render_workstation_ui(ui);
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = app.last_report.expect("full app r08 report");
    RenderCaseResult {
        dataset_bars: app.candles.len(),
        workload_id,
        description,
        scale_mode: app.viewport.main_price_scale.mode().label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r08_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R08DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R08\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"scanner_alerts_and_palette_domain\": [");
    for (idx, d) in domain.iter().enumerate() {
        let comma = if idx + 1 == domain.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"case_id\": \"{}\", \"description\": \"{}\", \"input_units\": {}, \"output_items\": {}, \"median_us\": {:.3}, \"p95_us\": {:.3}, \"max_us\": {:.3} }}{comma}",
            d.case_id,
            d.description.replace('"', "\\\""),
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"workstation_r08_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r08_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R08DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R08 — Multi-Symbol Watchlist Scanner, Causal Alert Engine & `Cmd+K` Command Palette Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r08`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** `13`-Symbol Multi-Asset Watchlist Universe (`VN30` + `CafeF` + `SPY` + `BTCUSDT` + `VN30F1M`) + `100,000` `1m` Base Bars + Causal `AlertEngine` (`PriceLevel`, `RSI(14)`, `MACD(12,26,9)`, `DrawingLine`) + `Cmd+K` Command Palette + 4 Panes + 6 Indicators + 25 Drawings + Paper Trading\n"
    );

    let _ = writeln!(
        s,
        "## 1. Multi-Symbol Scanner, Causal Alert Evaluation/Rewind & Command Palette Throughput\n"
    );
    let _ = writeln!(
        s,
        "| Case ID | Description | Input Bars / Items | Output Rows / Events | Median (`µs`) | P95 (`µs`) | Max (`µs`) |"
    );
    let _ = writeln!(s, "| :--- | :--- | ---: | ---: | ---: | ---: | ---: |");
    for d in domain {
        let _ = writeln!(
            s,
            "| `{}` | {} | `{}` | `{}` | **`{:.2} µs`** | `{:.2} µs` | `{:.2} µs` |",
            d.case_id,
            d.description,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

#[derive(Debug, Clone)]
struct R09DomainBenchCase {
    case_id: &'static str,
    description: &'static str,
    input_units: usize,
    output_items: usize,
    stats: TimingStats,
}

fn run_r09_backtest_lab_benchmark(cpu_brand: &str, mem_bytes: u64, rustc_ver: &str, profile: &str) {
    println!(
        "=== Sumi R09 Strategy Backtest Lab, Equity/Drawdown Sub-Pane & Quantitative Tearsheet Benchmark ==="
    );
    println!(
        "CPU: {cpu_brand} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let universe = load_bundled_watchlist_universe();
    let candles_100k_1m = generate_intraday_vn30f_1m_report(100_000).candles;
    let fpt_feed = universe
        .iter()
        .find(|e| e.symbol == "FPT")
        .unwrap_or(&universe[0]);
    let btc_json = build_btcusdt_15m_json_document(2_400);
    let btc_15m = load_ohlcv_json(&btc_json, &DataLoadOptions::default())
        .expect("load btc 15m")
        .candles;
    let btc_4h = aggregate_ohlcv(&btc_15m, Timeframe::M15, Timeframe::H4).expect("btc 4h");

    let mut domain_cases = Vec::new();

    // 1. 100,000-bar `SmaEmaTrendCross` + `70/30` IS/OOS split + `SeriesBlockIndex` equity/underwater + Tearsheet (`< 15.0 ms` gate)
    let sma_cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 20,
            slow_ema: 50,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongShort,
        sizing: PositionSizingMode::PercentEquity(95.0),
        risk: StopTakeConfig {
            stop_loss_pct: Some(2.5),
            take_profit_pct: Some(6.0),
            trailing_stop_atr_mult: Some(2.4),
            min_holding_bars: 0,
            ..StopTakeConfig::default()
        },
        in_sample_split_pct: Some(0.70),
        ..BacktestConfig::default()
    };
    let mut sma_100k_samples = Vec::with_capacity(60);
    let mut sma_100k_trades = 0_usize;
    for step in 0..72 {
        let t0 = Instant::now();
        let res = run_backtest_with_options(
            "VN30F1M",
            Timeframe::M1,
            std::hint::black_box(&candles_100k_1m),
            std::hint::black_box(&sma_cfg),
            BacktestRunOptions::default(),
        );
        sma_100k_trades = res.trades.len();
        std::hint::black_box(res.tearsheet.total_return_pct);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 12 {
            sma_100k_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r09_backtest_100k_sma_ema_oos",
        description: "100,000-bar 1m SmaEmaTrendCross + 70/30 IS/OOS + SeriesBlockIndex + Tearsheet (<15ms gate)",
        input_units: candles_100k_1m.len(),
        output_items: sma_100k_trades,
        stats: TimingStats::from_samples(sma_100k_samples),
    });

    // 2. 100,000-bar `DonchianAtrBreakout` ($O(N)$ monotonic-deque prior channel + ATR trailing ratchet + OOS)
    let don_cfg = BacktestConfig {
        preset: StrategyPresetKind::DonchianAtrBreakout,
        params: StrategyParams {
            donchian_entry_period: 20,
            donchian_exit_period: 10,
            atr_period: 14,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongShort,
        sizing: PositionSizingMode::RiskPercentPerTrade(1.5),
        risk: StopTakeConfig {
            stop_loss_pct: Some(2.0),
            take_profit_pct: Some(5.0),
            trailing_stop_atr_mult: Some(2.2),
            min_holding_bars: 0,
            ..StopTakeConfig::default()
        },
        in_sample_split_pct: Some(0.70),
        ..BacktestConfig::default()
    };
    let mut don_100k_samples = Vec::with_capacity(60);
    let mut don_100k_trades = 0_usize;
    for step in 0..72 {
        let t0 = Instant::now();
        let res = run_backtest_with_options(
            "VN30F1M",
            Timeframe::M1,
            std::hint::black_box(&candles_100k_1m),
            std::hint::black_box(&don_cfg),
            BacktestRunOptions::default(),
        );
        don_100k_trades = res.trades.len();
        std::hint::black_box(res.tearsheet.max_drawdown_pct);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 12 {
            don_100k_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r09_backtest_100k_donchian_atr_oos",
        description: "100,000-bar 1m O(N) monotonic-deque DonchianAtrBreakout + ATR trail + 70/30 IS/OOS",
        input_units: candles_100k_1m.len(),
        output_items: don_100k_trades,
        stats: TimingStats::from_samples(don_100k_samples),
    });

    // 3. 12-variant parameter sweep on `FPT` (`VN30` `1D`)
    let fpt_cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        direction_mode: StrategyDirectionMode::LongOnly,
        in_sample_split_pct: Some(0.70),
        ..BacktestConfig::default()
    };
    let mut sweep_samples = Vec::with_capacity(90);
    let mut sweep_variants = 0_usize;
    for step in 0..108 {
        let t0 = Instant::now();
        let rep = run_parameter_sweep(
            "FPT",
            fpt_feed.base_timeframe,
            std::hint::black_box(&fpt_feed.candles),
            std::hint::black_box(&fpt_cfg),
            SweepRankingMetric::SharpeRatio,
            BacktestRunOptions::default(),
        );
        sweep_variants = rep.variants.len();
        std::hint::black_box(&rep);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 18 {
            sweep_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r09_sweep_12_grid_fpt_1d",
        description: "12-variant deterministic parameter sweep + IS/OOS robustness on FPT 1D",
        input_units: fpt_feed.candles.len() * 12,
        output_items: sweep_variants,
        stats: TimingStats::from_samples(sweep_samples),
    });

    // 4. 6-preset strategy battle on `BTCUSDT` `4h`
    let btc_cfg = BacktestConfig {
        preset: StrategyPresetKind::MacdMomentum,
        direction_mode: StrategyDirectionMode::LongShort,
        in_sample_split_pct: Some(0.70),
        ..BacktestConfig::default()
    };
    let mut preset_samples = Vec::with_capacity(90);
    let mut preset_rows = 0_usize;
    for step in 0..108 {
        let t0 = Instant::now();
        let rows = run_all_presets_battle(
            "BTCUSDT",
            Timeframe::H4,
            std::hint::black_box(&btc_4h),
            std::hint::black_box(&btc_cfg),
            SweepRankingMetric::SharpeRatio,
            BacktestRunOptions::default(),
        );
        preset_rows = rows.len();
        std::hint::black_box(&rows);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 18 {
            preset_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r09_6_preset_battle_btc_4h",
        description: "All 6 strategy presets battle on BTCUSDT 4h with IS/OOS split",
        input_units: btc_4h.len() * 6,
        output_items: preset_rows,
        stats: TimingStats::from_samples(preset_samples),
    });

    // 5. 13-symbol multi-asset Watchlist Universe Strategy Battle
    let mut battle_samples = Vec::with_capacity(80);
    let mut battle_rows = 0_usize;
    let total_univ_bars: usize = universe.iter().map(|e| e.candles.len()).sum();
    for step in 0..96 {
        let t0 = Instant::now();
        let rep = run_multi_symbol_battle(
            std::hint::black_box(&universe),
            Timeframe::D1,
            std::hint::black_box(&fpt_cfg),
            SweepRankingMetric::SharpeRatio,
            None,
        );
        battle_rows = rep.rows.len();
        std::hint::black_box(&rep);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 16 {
            battle_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r09_universe_13_symbol_battle",
        description: "13-symbol multi-asset Watchlist Universe strategy battle (VN30 + CafeF + SPY + BTC + VN30F1M)",
        input_units: total_univ_bars,
        output_items: battle_rows,
        stats: TimingStats::from_samples(battle_samples),
    });

    println!("\n[1] Strategy Backtest Engine, Parameter Sweep & Universe Battle Throughput");
    for d in &domain_cases {
        println!(
            "  {:<36} | in={:<8} out={:<5} | med={:>8.2} µs ({:>6.3} ms) | p95={:>8.2} µs",
            d.case_id,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us
        );
    }

    // Build 95,000-bar causal fixture with full backtest result, indicators, drawings, alerts & trading
    let mut replay_95k = ReplayController::new();
    replay_95k
        .start_at_base_index(&candles_100k_1m, 94_999)
        .expect("start 95k replay");
    let mat_95k = replay_95k
        .materialize_candles(&candles_100k_1m, Timeframe::M1, Timeframe::M1)
        .expect("materialize 95k");
    let cutoff_95k = replay_95k.cutoff_timestamp(&candles_100k_1m);

    let ind_all = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };
    let mut cache_95k = IndicatorCache::new();
    cache_95k.sync(&mat_95k, 1, &ind_all);

    let ind_eq_only = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: false,
        show_rsi: false,
        show_macd: false,
        show_atr: false,
        ..IndicatorConfig::default()
    };
    let mut cache_95k_eq = IndicatorCache::new();
    cache_95k_eq.sync(&mat_95k, 2, &ind_eq_only);

    let bt_95k = run_backtest_with_options(
        "VN30F1M",
        Timeframe::M1,
        &mat_95k,
        &sma_cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: cutoff_95k,
            incomplete_tail_bar: false,
        },
    );
    let focus_tid_95k = bt_95k.trades.last().map(|t| t.trade_id);

    let mut trading_95k = TradingSession::default();
    trading_95k.reset_at(&candles_100k_1m, 90_000);
    let _ = trading_95k.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 5.0,
            stop_loss: Some(candles_100k_1m[94_920].close - 4.0),
            take_profit: Some(candles_100k_1m[94_920].close + 8.0),
        },
        &candles_100k_1m,
        94_920,
    );
    trading_95k.advance_through_bars(&candles_100k_1m, 94_921, 94_999);

    let mut drawings_95k = build_r05_benchmark_drawings(
        &mat_95k,
        mat_95k.len().saturating_sub(500),
        mat_95k.len() - 1,
    );
    drawings_95k.remap_timeframe(&mat_95k, Timeframe::M1, &mat_95k, Timeframe::M1);
    drawings_95k.set_replay_cutoff_timestamp(cutoff_95k, mat_95k.len());

    let mut alerts_95k = build_r08_benchmark_alert_engine(
        &candles_100k_1m,
        &mat_95k,
        Timeframe::M1,
        90_000,
        94_999,
        &drawings_95k,
    );
    alerts_95k.evaluate_causal(
        &candles_100k_1m,
        Timeframe::M1,
        &mat_95k,
        Timeframe::M1,
        94_999,
        cutoff_95k,
        &drawings_95k,
    );

    let mut render_cases = Vec::new();
    render_cases.push(bench_r09_chart_full_case(
        &mat_95k,
        "VN30F1M",
        "1m",
        &ind_all,
        &cache_95k,
        &drawings_95k,
        &trading_95k,
        &alerts_95k,
        &bt_95k,
        focus_tid_95k,
        cutoff_95k,
        PriceScaleMode::Normal,
        |vp| {
            vp.time_scale.set_bar_spacing(8.0);
            vp.time_scale.set_right_offset(12.0);
        },
        "r09_95k_1m_recent_5pane_equity",
        "95K 1m causal bars, 5 active panes (Main+RSI+MACD+ATR+Equity) + Trade Links + Alerts + Trading",
        16,
        90,
    ));
    render_cases.push(bench_r09_chart_full_case(
        &mat_95k,
        "VN30F1M",
        "1m",
        &ind_eq_only,
        &cache_95k_eq,
        &drawings_95k,
        &trading_95k,
        &alerts_95k,
        &bt_95k,
        focus_tid_95k,
        cutoff_95k,
        PriceScaleMode::Logarithmic,
        |vp| {
            vp.fit_all();
        },
        "r09_95k_1m_full_fit_equity_conflated",
        "95K 1m full-fit conflated (>=32x), Main + Equity SeriesBlockIndex M4 baseline Mesh + Trade Links",
        16,
        90,
    ));
    render_cases.push(bench_r09_full_workstation_ui(
        &candles_100k_1m,
        StrategyLabSubTab::Tearsheet,
        true,
        "r09_workstation_1280x800_tearsheet_equity",
        "Full 1280x800 workstation UI (Watchlist + Main + RSI + Equity + Strategy Lab Tearsheet & IS/OOS)",
        16,
        90,
    ));
    render_cases.push(bench_r09_full_workstation_ui(
        &candles_100k_1m,
        StrategyLabSubTab::Sweep,
        false,
        "r09_workstation_1280x800_sweep_battle",
        "Full 1280x800 workstation UI (Main + MACD + Equity + Strategy Lab 12-Grid Sweep & 6-Preset Battle)",
        16,
        90,
    ));

    println!("\n[2] End-to-End Workstation R09 Render & Tessellation Budgets (1280x800 @ 2.0x)");
    for r in &render_cases {
        println!(
            "  {:<42} | confl={:>3}x | drawn={:>4} | verts={:>6} | ui={:>6.3}ms | tess={:>6.3}ms | total={:>6.3}ms (p95={:>6.3}ms)",
            r.workload_id,
            r.conflation_factor,
            r.rendered_candles,
            r.total_vertices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }

    let json = format_r09_json_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );
    let md = format_r09_markdown_report(
        cpu_brand,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );

    std::fs::create_dir_all("docs/evidence").expect("create docs/evidence");
    std::fs::write("docs/evidence/r09_backtest_lab_benchmark.json", json)
        .expect("write r09 benchmark json");
    std::fs::write("docs/evidence/r09_backtest_lab_benchmark.md", md)
        .expect("write r09 benchmark md");
    println!(
        "\nSaved docs/evidence/r09_backtest_lab_benchmark.json and docs/evidence/r09_backtest_lab_benchmark.md"
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_r09_chart_full_case<F: FnOnce(&mut ChartViewportState)>(
    candles: &[Candle],
    symbol: &str,
    timeframe: &str,
    indicators: &IndicatorConfig,
    cache: &IndicatorCache,
    template_store: &DrawingStore,
    trading: &TradingSession,
    alerts: &AlertEngine,
    backtest_result: &BacktestRunResult,
    focused_trade_id: Option<u64>,
    replay_cutoff_timestamp: Option<i64>,
    scale_mode: PriceScaleMode,
    setup_fn: F,
    workload_id: &'static str,
    description: &'static str,
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut vp = ChartViewportState::new();
    vp.time_scale.set_total_bars(candles.len());
    vp.main_price_scale.set_mode(scale_mode);
    vp.equity_stretch = 1.35;
    let mut store = template_store.clone();

    let cfg = ChartDisplayConfig {
        symbol,
        timeframe,
        show_grid: true,
        show_crosshair: true,
        show_volume: true,
    };

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = render_workstation_chart_full(
                        ui,
                        candles,
                        &mut vp,
                        cfg,
                        indicators,
                        cache,
                        &mut store,
                        ChartReplayTradingOverlay {
                            replay_active: true,
                            selecting_cut_bar: false,
                            replay_status_label: Some("PAUSED"),
                            trading_session: Some(trading),
                            alert_engine: Some(alerts),
                            backtest_result: Some(backtest_result),
                            equity_override: None,
                            show_equity_pane: true,
                            show_backtest_trades_on_chart: true,
                            focused_backtest_trade_id: focused_trade_id,
                            replay_cutoff_timestamp,
                            frontier_base_timestamp: replay_cutoff_timestamp
                                .or_else(|| candles.last().map(|c| c.timestamp)),
                            modal_open: false,
                            ..ChartReplayTradingOverlay::default()
                        },
                    );
                });
            },
        );
        init_out.textures_delta.clear();
    }
    setup_fn(&mut vp);

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut last_rep = None;
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 180.0 + ((step % 30) as f32) * 24.0;
        let hover_y = if step % 2 == 0 { 220.0 } else { 610.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                last_rep = render_workstation_chart_full(
                    ui,
                    candles,
                    &mut vp,
                    cfg,
                    indicators,
                    cache,
                    &mut store,
                    ChartReplayTradingOverlay {
                        replay_active: true,
                        selecting_cut_bar: false,
                        replay_status_label: Some("PAUSED"),
                        trading_session: Some(trading),
                        alert_engine: Some(alerts),
                        backtest_result: Some(backtest_result),
                        equity_override: None,
                        show_equity_pane: true,
                        show_backtest_trades_on_chart: true,
                        focused_backtest_trade_id: focused_trade_id,
                        replay_cutoff_timestamp,
                        frontier_base_timestamp: replay_cutoff_timestamp
                            .or_else(|| candles.last().map(|c| c.timestamp)),
                        modal_open: false,
                        ..ChartReplayTradingOverlay::default()
                    },
                );
            });
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = last_rep.expect("r09 chart render report");
    RenderCaseResult {
        dataset_bars: candles.len(),
        workload_id,
        description,
        scale_mode: scale_mode.label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn bench_r09_full_workstation_ui(
    candles_100k_1m: &[Candle],
    sub_tab: StrategyLabSubTab,
    show_scanner: bool,
    workload_id: &'static str,
    description: &'static str,
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    let ctx = egui::Context::default();
    SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut app = SumiWorkstationApp::default();
    let csv_100k = build_r06_csv_document(candles_100k_1m);
    app.load_from_csv_str(&csv_100k, &DataLoadOptions::default())
        .expect("load 100k csv into app");
    app.set_timeframe(Timeframe::M15).expect("15m rollup");
    app.indicators = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: !show_scanner,
        show_rsi: show_scanner,
        show_macd: !show_scanner,
        show_atr: false,
        ..IndicatorConfig::default()
    };

    assert!(app.start_replay_at_base_index(72_000));
    app.populate_showcase_drawings();
    app.show_replay_bar = true;
    app.show_scanner_dock = show_scanner;
    app.scanner_tab = ScannerDockTab::Watchlist;
    app.show_trading_panel = false;
    app.show_alerts_panel = false;
    app.strategy_lab_tab = sub_tab;
    app.open_strategy_lab();
    if sub_tab == StrategyLabSubTab::Sweep {
        app.run_active_parameter_sweep();
    }
    app.viewport.time_scale.set_bar_spacing(8.5);
    app.viewport.time_scale.set_right_offset(8.0);

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        init_out.textures_delta.clear();
    }

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 340.0 + ((step % 20) as f32) * 18.0;
        let hover_y = if step % 2 == 0 { 240.0 } else { 580.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            app.render_workstation_ui(ui);
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = app.last_report.expect("full app r09 report");
    RenderCaseResult {
        dataset_bars: app.candles.len(),
        workload_id,
        description,
        scale_mode: app.viewport.main_price_scale.mode().label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r09_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R09DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R09\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"backtest_lab_domain\": [");
    for (idx, d) in domain.iter().enumerate() {
        let comma = if idx + 1 == domain.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"case_id\": \"{}\", \"description\": \"{}\", \"input_units\": {}, \"output_items\": {}, \"median_us\": {:.3}, \"median_ms\": {:.4}, \"p95_us\": {:.3}, \"max_us\": {:.3} }}{comma}",
            d.case_id,
            d.description.replace('"', "\\\""),
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"workstation_r09_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r09_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R09DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R09 — Deterministic Strategy Backtest Lab, Synchronized Equity/Drawdown Sub-Pane & Quantitative Tearsheet Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r09`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** `100,000` `1m` Base Bars + Causal `run_backtest_with_options` (`Open[T+1]` fills, pessimistic `SL`-before-`TP`, post-close ATR trail, `70/30` `IS/OOS` split) + `12`-Grid Parameter Sweep + `13`-Symbol Universe Battle + 5 Synchronized Panes (`Main + RSI + MACD + ATR + Equity` 2-`Mesh` Baseline) + `O(log T)` Trade Links\n"
    );

    let _ = writeln!(
        s,
        "## 1. Strategy Backtest Engine, Parameter Sweep & Multi-Symbol Battle Throughput (`< 15.0 ms` 100K Gate)\n"
    );
    let _ = writeln!(
        s,
        "| Case ID | Description | Input Bars | Output Trades / Rows | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |"
    );
    let _ = writeln!(
        s,
        "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for d in domain {
        let _ = writeln!(
            s,
            "| `{}` | {} | `{}` | `{}` | **`{:.2} µs`** | **`{:.3} ms`** | `{:.2} µs` | `{:.2} µs` |",
            d.case_id,
            d.description,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

fn run_r10_risk_journal_wfo_portfolio_benchmark(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    use sumi_workstation::data::DataSourcePreset;
    use sumi_workstation::trading::PositionSide;

    println!(
        "=== Sumi R10 Vietnam Risk Planner, Trade Journal & Walk-Forward Portfolio Benchmark ==="
    );
    println!(
        "CPU: {cpu} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let universe = load_bundled_watchlist_universe();
    let candles_100k_1m = generate_intraday_vn30f_1m_report(100_000).candles;
    let candles_15m =
        aggregate_ohlcv(&candles_100k_1m, Timeframe::M1, Timeframe::M15).expect("15m rollup");
    let fpt_feed = universe
        .iter()
        .find(|e| e.symbol == "FPT")
        .unwrap_or(&universe[0]);

    let mut domain_cases = Vec::new();

    // 1. 1,000 HOSE Risk Planner evaluations (±7% band, 3-tier tick snap, board lot 100, T+2 weekend rollover, all-in friction)
    let base_plan_in = TradePlanInput {
        venue: ExchangeVenue::Hose,
        price_unit: VnPriceUnit::Auto,
        side: PositionSide::Long,
        account_equity: 250_000_000.0,
        available_cash: 250_000_000.0,
        leverage: 1.0,
        risk_pct: 1.0,
        entry_price: 132_450.0,
        stop_loss: 126_800.0,
        target_price: 146_900.0,
        reference_price: Some(131_500.0),
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        slippage_bps: 5.0,
        snap_to_ticks: true,
        entry_timestamp: Some(1_731_600_000),
        ..TradePlanInput::default()
    };
    let mut plan_1k_samples = Vec::with_capacity(80);
    let mut last_qty = 0_usize;
    for step in 0..96 {
        let t0 = Instant::now();
        for k in 0..1_000_u32 {
            let mut inp = base_plan_in.clone();
            inp.entry_price += f64::from(k % 17) * 50.0;
            if let Ok(out) = compute_trade_plan(std::hint::black_box(&inp)) {
                last_qty = out.planned_quantity as usize;
            }
        }
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 16 {
            plan_1k_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r10_hose_risk_planner_1k_evals",
        description: "1,000 HOSE Risk Planner evaluations (±7% band, 3-tier tick snap, lot 100, T+2, all-in friction)",
        input_units: 1_000,
        output_items: last_qty,
        stats: TimingStats::from_samples(plan_1k_samples),
    });

    // 2. 500-Entry Causal Trade Journal Analytics (SQN, Expectancy, Discipline %) + RFC-4180 CSV Export
    let mut j_store = TradeJournalStore::new();
    let sample_plan = compute_trade_plan(&base_plan_in).expect("valid sample plan");
    for i in 0..500_usize {
        let id = j_store.record_trade_plan(
            "FPT",
            &sample_plan,
            1_700_000_000 + (i as i64) * 3_600,
            SetupTag::ALL[i % SetupTag::ALL.len()],
            ((i % 5) + 1) as u8,
            ChecklistState::all_checked(),
            "=SUM(1,2) formula-sanitized journal note",
        );
        if let Some(e) = j_store.entry_mut(id) {
            e.status = sumi_workstation::journal::JournalEntryStatus::Closed;
            e.exit_timestamp = Some(e.entry_timestamp + 7_200);
            let sign = if i % 3 == 0 { -0.85 } else { 1.65 };
            e.realized_r = Some(sign);
            e.net_pnl = sign * 2_500_000.0;
            e.gross_pnl = e.net_pnl + 125_000.0;
            e.fees_and_taxes = 125_000.0;
            if i % 7 == 0 {
                e.mistake = MistakeTag::FomoChase;
                e.checklist.calm_state = false;
            }
        }
    }
    let cutoff_ts = Some(1_700_000_000 + 420 * 3_600);
    let mut jrn_samples = Vec::with_capacity(80);
    let mut csv_bytes = 0_usize;
    for step in 0..96 {
        let t0 = Instant::now();
        let analytics = j_store.compute_analytics(std::hint::black_box(cutoff_ts));
        let csv = j_store.export_csv(std::hint::black_box(cutoff_ts));
        std::hint::black_box(analytics.sqn);
        csv_bytes = csv.len();
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 16 {
            jrn_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r10_journal_analytics_and_csv_500",
        description: "500-entry DEF-001 causal Journal analytics (SQN, Discipline %, Attribution) + RFC-4180 CSV export",
        input_units: 500,
        output_items: csv_bytes,
        stats: TimingStats::from_samples(jrn_samples),
    });

    // 3. 4-Fold Rolling Walk-Forward Optimization (12-grid IS -> blind OOS compounding) on FPT (1D)
    let bt_cfg = BacktestConfig {
        preset: StrategyPresetKind::MacdMomentum,
        params: sumi_workstation::backtest::StrategyParams {
            macd_fast: 6,
            macd_slow: 16,
            macd_signal: 5,
            macd_trend_ema: 0,
            ..sumi_workstation::backtest::StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongOnly,
        sizing: PositionSizingMode::PercentEquity(95.0),
        lot_step: 100.0,
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        ..BacktestConfig::default()
    };
    let bt_cfg_15m = BacktestConfig {
        lot_step: 1.0,
        commission_bps: 1.5,
        tax_sell_bps: 0.0,
        ..bt_cfg.clone()
    };
    let wf_roll = WalkForwardConfig {
        fold_count: 4,
        in_sample_ratio: 0.70,
        anchored: false,
        ranking_metric: SweepRankingMetric::SharpeRatio,
    };
    let mut wfo_fpt_samples = Vec::with_capacity(48);
    let mut wfo_fpt_trades = 0_usize;
    for step in 0..56 {
        let t0 = Instant::now();
        let rep = run_walk_forward_validation(
            "FPT",
            fpt_feed.base_timeframe,
            std::hint::black_box(&fpt_feed.candles),
            std::hint::black_box(&bt_cfg),
            wf_roll,
            BacktestRunOptions::default(),
        );
        wfo_fpt_trades = rep.stitched_oos_trades.len();
        std::hint::black_box(rep.wfe_pct);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 8 {
            wfo_fpt_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r10_walk_forward_4fold_rolling_fpt_1d",
        description: "4-fold Rolling Walk-Forward (12-variant IS sweep -> stitched OOS curve & Pardo WFE) on FPT 1D",
        input_units: fpt_feed.candles.len(),
        output_items: wfo_fpt_trades,
        stats: TimingStats::from_samples(wfo_fpt_samples),
    });

    // 4. 4-Fold Anchored Walk-Forward on 6,667 15m bars (100K 1m causal rollup)
    let wf_anch = WalkForwardConfig {
        fold_count: 4,
        in_sample_ratio: 0.70,
        anchored: true,
        ranking_metric: SweepRankingMetric::SharpeRatio,
    };
    let mut wfo_15m_samples = Vec::with_capacity(24);
    let mut wfo_15m_trades = 0_usize;
    for step in 0..28 {
        let t0 = Instant::now();
        let rep = run_walk_forward_validation(
            "VN30F1M",
            Timeframe::M15,
            std::hint::black_box(&candles_15m),
            std::hint::black_box(&bt_cfg_15m),
            wf_anch,
            BacktestRunOptions::default(),
        );
        wfo_15m_trades = rep.stitched_oos_trades.len();
        std::hint::black_box(rep.param_stability_pct);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 4 {
            wfo_15m_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r10_walk_forward_4fold_anchored_15m",
        description: "4-fold Anchored Walk-Forward across 6,667 15m bars (100K 1m rollup)",
        input_units: candles_15m.len(),
        output_items: wfo_15m_trades,
        stats: TimingStats::from_samples(wfo_15m_samples),
    });

    // 5. Multi-Asset Portfolio Simulation: VN30 (8 HOSE equities, EqualWeight)
    let mut port_vn30_samples = Vec::with_capacity(48);
    let mut port_vn30_pts = 0_usize;
    for step in 0..56 {
        let t0 = Instant::now();
        let rep = run_portfolio_backtest(
            std::hint::black_box(&universe),
            Timeframe::D1,
            std::hint::black_box(&bt_cfg),
            PortfolioUniverseFilter::Vn30EquitiesOnly,
            PortfolioWeightingMode::EqualWeight,
            None,
        );
        port_vn30_pts = rep.portfolio_equity_curve.len();
        std::hint::black_box(rep.diversification_dd_reduction_pct);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 8 {
            port_vn30_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r10_portfolio_vn30_8_equal_weight",
        description: "8-symbol VN30 cash equity portfolio simulation (EqualWeight 1/N + diversification DD attribution)",
        input_units: 8,
        output_items: port_vn30_pts,
        stats: TimingStats::from_samples(port_vn30_samples),
    });

    // 6. Multi-Asset Portfolio Simulation: All 13 Universe Feeds (CausalInverseVolatility(20))
    let mut port_13_samples = Vec::with_capacity(32);
    let mut port_13_pts = 0_usize;
    for step in 0..38 {
        let t0 = Instant::now();
        let rep = run_portfolio_backtest(
            std::hint::black_box(&universe),
            Timeframe::D1,
            std::hint::black_box(&bt_cfg),
            PortfolioUniverseFilter::AllUniverseFeeds,
            PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 20 },
            None,
        );
        port_13_pts = rep.portfolio_equity_curve.len();
        std::hint::black_box(rep.portfolio_tearsheet.total_return_pct);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        if step >= 6 {
            port_13_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r10_portfolio_universe_13_causal_inv_vol",
        description: "13-feed multi-asset universe portfolio simulation (past-only CausalInverseVolatility(20))",
        input_units: universe.len(),
        output_items: port_13_pts,
        stats: TimingStats::from_samples(port_13_samples),
    });

    for d in &domain_cases {
        println!(
            "[DOMAIN {:42}] in={:6} out={:6} | med={:8.2} us ({:6.3} ms) | p95={:8.2} us",
            d.case_id,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
        );
    }

    // End-to-End Retina (2.0x, 1280x800) Workstation Frame Budget (< 4.0 ms Hybrid Gate)
    let render_cases = vec![
        bench_r10_full_workstation_ui(
            &candles_100k_1m,
            0,
            "r10_full_ui_vn_risk_journal_rr_box",
            "Full Workstation UI: FPT 1D + HOSE Risk Planner + selected dual-zone RiskRewardBox + Journal KPIs",
            16,
            100,
        ),
        bench_r10_full_workstation_ui(
            &candles_100k_1m,
            1,
            "r10_full_ui_wfo_stitched_oos_5panes",
            "Full Workstation UI: 100K 1m->15m + 5 panes (Main+RSI+MACD+ATR+Stitched WFO OOS Equity) + WFO sub-tab",
            16,
            100,
        ),
        bench_r10_full_workstation_ui(
            &candles_100k_1m,
            2,
            "r10_full_ui_portfolio_equity_5panes",
            "Full Workstation UI: FPT 1D + 5 panes + mapped 8-symbol VN30 Inv-Vol Portfolio Equity Curve + WFO sub-tab",
            16,
            100,
        ),
    ];

    for r in &render_cases {
        println!(
            "[RENDER {:42}] bars={:6} | drawn={:4} | shapes={:4} | ui={:.3}ms | tess={:.3}ms | total={:.3}ms (p95={:.3}ms)",
            r.workload_id,
            r.dataset_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }

    let _ = std::fs::create_dir_all("docs/evidence");
    let json_doc = format_r10_json_report(
        cpu,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );
    let md_doc = format_r10_markdown_report(
        cpu,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
    );
    std::fs::write(
        "docs/evidence/r10_risk_journal_wfo_benchmark.json",
        json_doc,
    )
    .expect("write r10 benchmark json");
    std::fs::write("docs/evidence/r10_risk_journal_wfo_benchmark.md", md_doc)
        .expect("write r10 benchmark md");
    println!(
        "\nSaved R10 benchmark artifacts to docs/evidence/r10_risk_journal_wfo_benchmark.{{json,md}}"
    );
    let _ = DataSourcePreset::Vn30Basket;
}

fn bench_r10_full_workstation_ui(
    candles_100k_1m: &[Candle],
    mode: u8,
    workload_id: &'static str,
    description: &'static str,
    warmup: usize,
    iters: usize,
) -> RenderCaseResult {
    use sumi_workstation::data::DataSourcePreset;

    let ctx = egui::Context::default();
    SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut app = SumiWorkstationApp::default();
    match mode {
        0 => {
            app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
            app.indicators = IndicatorConfig {
                show_sma: true,
                show_ema: true,
                show_bbands: true,
                show_rsi: true,
                show_macd: false,
                show_atr: false,
                ..IndicatorConfig::default()
            };
            let _ = app.import_backtest_trades_to_journal(10);
            let _ = app.snap_planner_ticks_and_sync_rr_box();
            let _ = app.draw_or_update_rr_box_from_planner();
            let _ = app.log_active_plan_to_journal();
            app.open_journal_tab();
            app.viewport.time_scale.set_bar_spacing(9.5);
            app.viewport.time_scale.set_right_offset(10.0);
        }
        1 => {
            let csv_100k = build_r06_csv_document(candles_100k_1m);
            app.load_from_csv_str(&csv_100k, &DataLoadOptions::default())
                .expect("load 100k csv");
            app.set_timeframe(Timeframe::M15).expect("15m rollup");
            app.indicators = IndicatorConfig {
                show_sma: true,
                show_ema: true,
                show_bbands: false,
                show_rsi: true,
                show_macd: true,
                show_atr: true,
                ..IndicatorConfig::default()
            };
            assert!(app.start_replay_at_base_index(72_000));
            app.open_strategy_lab();
            app.strategy_lab_tab = StrategyLabSubTab::WalkForward;
            app.equity_pane_source = EquityPaneSource::WalkForwardOos;
            app.show_backtest_trades = false;
            app.run_active_walk_forward();
            let _ = app.draw_or_update_rr_box_from_planner();
            app.viewport.time_scale.set_bar_spacing(8.5);
            app.viewport.time_scale.set_right_offset(8.0);
        }
        _ => {
            app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
            app.indicators = IndicatorConfig {
                show_sma: true,
                show_ema: true,
                show_bbands: false,
                show_rsi: true,
                show_macd: true,
                show_atr: true,
                ..IndicatorConfig::default()
            };
            app.portfolio_universe_filter = PortfolioUniverseFilter::Vn30EquitiesOnly;
            app.portfolio_weighting =
                PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 20 };
            app.open_strategy_lab();
            app.strategy_lab_tab = StrategyLabSubTab::WalkForward;
            app.equity_pane_source = EquityPaneSource::MultiAssetPortfolio;
            app.show_backtest_trades = false;
            app.run_active_walk_forward();
            app.run_active_portfolio_backtest();
            let _ = app.draw_or_update_rr_box_from_planner();
            app.viewport.fit_all();
        }
    }

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        init_out.textures_delta.clear();
    }

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 300.0 + ((step % 22) as f32) * 18.0;
        let hover_y = if step % 2 == 0 { 220.0 } else { 610.0 };
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events: vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))],
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            app.render_workstation_ui(ui);
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let rep = app.last_report.expect("full app r10 report");
    RenderCaseResult {
        dataset_bars: app.candles.len(),
        workload_id,
        description,
        scale_mode: app.viewport.main_price_scale.mode().label(),
        conflation_enabled: true,
        bar_spacing: rep.bar_spacing,
        conflation_factor: rep.conflation_factor,
        culled_bars: rep.culled_bar_count,
        rendered_candles: rep.rendered_candle_count,
        paint_shapes,
        clipped_primitives,
        total_vertices,
        total_indices,
        ui_build_stats: TimingStats::from_samples(ui_samples),
        tessellate_stats: TimingStats::from_samples(tess_samples),
        total_cpu_frame_stats: TimingStats::from_samples(total_samples),
    }
}

fn format_r10_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R09DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"milestone\": \"R10\",");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"r10_domain_throughput\": [");
    for (idx, d) in domain.iter().enumerate() {
        let comma = if idx + 1 == domain.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"case_id\": \"{}\", \"description\": \"{}\", \"input_units\": {}, \"output_items\": {}, \"median_us\": {:.3}, \"median_ms\": {:.4}, \"p95_us\": {:.3}, \"max_us\": {:.3} }}{comma}",
            d.case_id,
            d.description.replace('"', "\\\""),
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"workstation_r10_workloads\": [");
    for (idx, r) in render.iter().enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r10_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R09DomainBenchCase],
    render: &[RenderCaseResult],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R10 — Vietnam Market Microstructure Risk Planner, On-Chart Risk-Reward Tool, Structured Trade Journal & Rolling Walk-Forward Portfolio Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r10`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** Vietnam `HOSE / HNX / UPCOM / VN30F` Microstructure Risk Planner + Dual-Zone `RiskRewardBox` + `DEF-001` Causal `TradeJournalStore` (`SQN`, `Discipline %`, RFC-4180 CSV) + `K`-Fold Rolling/Anchored Walk-Forward (`Pardo WFE` + Stitched OOS Equity) + `8`/`13`-Asset Causal Portfolio Simulation\n"
    );

    let _ = writeln!(
        s,
        "## 1. Risk Planner, Trade Journal, Walk-Forward & Multi-Asset Portfolio Throughput\n"
    );
    let _ = writeln!(
        s,
        "| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |"
    );
    let _ = writeln!(
        s,
        "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for d in domain {
        let _ = writeln!(
            s,
            "| `{}` | {} | `{}` | `{}` | **`{:.2} µs`** | **`{:.3} ms`** | `{:.2} µs` | `{:.2} µs` |",
            d.case_id,
            d.description,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |"
    );
    for r in render {
        let tot_med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let tot_p95_ms = r.total_cpu_frame_stats.p95_us / 1_000.0;
        let hybrid_gate = if tot_med_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "TRIGGER_WGPU"
        };
        let pass_120 = if tot_med_ms <= 8.33 {
            "PASS"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | `{:.3}` | `{:.3}` | **`{:.3}`** | `{:.3}` | `{hybrid_gate}` | `{pass_120}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            tot_med_ms,
            tot_p95_ms,
        );
    }
    s
}

fn run_r11_volume_profile_vwap_rs_benchmark(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
) {
    println!(
        "=== Sumi R11 Order-Flow Volume Profile, Session/Anchored VWAP & Causal RS Benchmark ==="
    );
    println!(
        "CPU: {cpu} | RAM: {:.1} GiB | {rustc_ver} | profile={profile}",
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );

    let universe = load_bundled_watchlist_universe();
    let candles_50k_1m = generate_intraday_vn30f_1m_report(50_000).candles;
    let first_ts = candles_50k_1m.first().map_or(0, |c| c.timestamp);
    let last_ts = candles_50k_1m.last().map_or(0, |c| c.timestamp);
    let (_, bench_tf, bench_candles) =
        resolve_benchmark_series_for_window("VNINDEX", first_ts, last_ts, &universe)
            .expect("resolve VNINDEX benchmark");

    let mut domain_cases = Vec::new();
    let vp_cfg = VolumeProfileConfig {
        row_count: 64,
        value_area_pct: 0.70,
    };

    // 1. 500-bar Visible VRVP (64 rows, exact intrabar volume apportionment + 70% Value Area)
    let slice_500 = &candles_50k_1m[candles_50k_1m.len() - 500..];
    let mut vrvp_500_samples = Vec::with_capacity(120);
    let mut last_vp_bins = 0_usize;
    for step in 0..144 {
        let t0 = Instant::now();
        let res = compute_volume_profile(std::hint::black_box(slice_500), vp_cfg)
            .expect("500-bar volume profile");
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        last_vp_bins = res.bins.len();
        if step >= 24 {
            vrvp_500_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r11_vrvp_500_bars_64_rows",
        description: "Visible Range Volume Profile (`500` bars, `64` rows, 70% VA + POC)",
        input_units: 500,
        output_items: last_vp_bins,
        stats: TimingStats::from_samples(vrvp_500_samples),
    });

    // 2. 50,000-bar VRVP stress pass (64 rows)
    let mut vrvp_50k_samples = Vec::with_capacity(48);
    for step in 0..56 {
        let t0 = Instant::now();
        let res = compute_volume_profile(std::hint::black_box(&candles_50k_1m), vp_cfg)
            .expect("50k volume profile");
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        last_vp_bins = res.bins.len();
        if step >= 8 {
            vrvp_50k_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r11_vrvp_50k_bars_64_rows",
        description: "Full-History Volume Profile (`50,000` bars, `64` rows, 70% VA + POC)",
        input_units: candles_50k_1m.len(),
        output_items: last_vp_bins,
        stats: TimingStats::from_samples(vrvp_50k_samples),
    });

    // 3. 50,000-bar Session VWAP (`UTC+7` daily reset + ±1σ / ±2σ Welford variance bands)
    let mut svwap_samples = Vec::with_capacity(48);
    let mut last_sessions = 0_usize;
    for step in 0..56 {
        let t0 = Instant::now();
        let out = compute_session_vwap(
            std::hint::black_box(&candles_50k_1m),
            Timeframe::M1,
            25_200,
            1.0,
            2.0,
        );
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        last_sessions = out.session_starts.len();
        if step >= 8 {
            svwap_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r11_session_vwap_50k_1m_utc7",
        description: "Session VWAP (`50,000` 1m bars, `UTC+7` reset, ±1σ/±2σ Welford bands)",
        input_units: candles_50k_1m.len(),
        output_items: last_sessions,
        stats: TimingStats::from_samples(svwap_samples),
    });

    // 4. 50,000-bar Anchored VWAP (anchor at midpoint 25,000)
    let mut avwap_samples = Vec::with_capacity(48);
    let mut avwap_len = 0_usize;
    for step in 0..56 {
        let t0 = Instant::now();
        let out = compute_anchored_vwap(std::hint::black_box(&candles_50k_1m), 25_000, 1.0, 2.0);
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        avwap_len = out.vwap.iter().filter(|v| v.is_some()).count();
        if step >= 8 {
            avwap_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r11_anchored_vwap_50k_bars",
        description: "Anchored VWAP (`50,000` 1m bars, anchor `25,000`, ±1σ/±2σ bands)",
        input_units: candles_50k_1m.len(),
        output_items: avwap_len,
        stats: TimingStats::from_samples(avwap_samples),
    });

    // 5. 50,000-bar Causal Relative Strength (`DEF-001` binary search + 20-SMA)
    let mut rs_samples = Vec::with_capacity(48);
    let mut rs_valid = 0_usize;
    for step in 0..56 {
        let t0 = Instant::now();
        let out = compute_relative_strength(
            std::hint::black_box(&candles_50k_1m),
            Timeframe::M1,
            std::hint::black_box(&bench_candles),
            bench_tf,
            20,
            None,
        );
        let us = t0.elapsed().as_secs_f64() * 1_000_000.0;
        rs_valid = out.rs_line.iter().filter(|v| v.is_some()).count();
        if step >= 8 {
            rs_samples.push(us);
        }
    }
    domain_cases.push(R09DomainBenchCase {
        case_id: "r11_causal_rs_50k_vs_vnindex",
        description: "Causal Relative Strength vs `VNINDEX` (`50,000` 1m bars, `DEF-001` close alignment + `20`-SMA)",
        input_units: candles_50k_1m.len(),
        output_items: rs_valid,
        stats: TimingStats::from_samples(rs_samples),
    });

    println!("\n--- R11 Order-Flow / VWAP / Causal RS Kernels ---");
    for d in &domain_cases {
        println!(
            "  {:<36} | in={:>7} out={:>6} | med={:>8.2} µs ({:.3} ms) p95={:>8.2} µs",
            d.case_id,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us
        );
    }

    // Phase 2: Full Workstation 1280x800 (`2560x1600` Retina `2.0x`) UI + GPU Tessellation Frame Budgets
    let (r_fpt, r_fpt_p99) = bench_r11_full_workstation_ui(
        &candles_50k_1m,
        0,
        "r11_fpt_vrvp_frvp_vwap_compare_rs",
        "FPT 1D + VRVP + FRVP + VWAP(±1σ/2σ) + AVWAP + Compare VNINDEX + RS Pane",
        10,
        60,
    );
    let (r_50k_intraday, r_50k_intraday_p99) = bench_r11_full_workstation_ui(
        &candles_50k_1m,
        1,
        "r11_vn30f_50k_1m_session_vwap_vrvp_rs",
        "VN30F1M 50k 1m + Session VWAP(UTC+7) + AVWAP + VRVP(64) + FRVP + RS",
        10,
        60,
    );
    let (r_50k_conflated, r_50k_conflated_p99) = bench_r11_full_workstation_ui(
        &candles_50k_1m,
        2,
        "r11_vn30f_50k_conflated_vwap_vrvp_compare_rs",
        "VN30F1M 50k 1m Zoom-Out Conflated + VRVP + Session VWAP + Compare + RS",
        10,
        60,
    );
    let (r_50k_pan_zoom, r_50k_pan_zoom_p99) = bench_r11_full_workstation_ui(
        &candles_50k_1m,
        3,
        "r11_vn30f_50k_interactive_pan_zoom_avwap_compare_rs",
        "VN30F1M 50k 1m Interactive Pan+Zoom + VRVP + FRVP + VWAP + AVWAP + Compare VNINDEX + RS",
        10,
        60,
    );
    let render_cases = vec![r_fpt, r_50k_intraday, r_50k_conflated, r_50k_pan_zoom];
    let p99_ms_list = [
        r_fpt_p99,
        r_50k_intraday_p99,
        r_50k_conflated_p99,
        r_50k_pan_zoom_p99,
    ];

    println!("\n--- Workstation R11 Full-Frame Budget (< 4.0 ms target) ---");
    for (r, p99_ms) in render_cases.iter().zip(p99_ms_list.iter()) {
        let mean_ms = r.total_cpu_frame_stats.mean_us / 1_000.0;
        let med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        println!(
            "  {:<44} | bars={:>6} conf={}x drawn={:>5} verts={:>6} | mean={:.3} ms med={:.3} ms p99={:.3} ms",
            r.workload_id,
            r.dataset_bars,
            r.conflation_factor,
            r.rendered_candles,
            r.total_vertices,
            mean_ms,
            med_ms,
            p99_ms
        );
        assert!(
            mean_ms < 4.0 && *p99_ms < 4.0,
            "R11 frame budget exceeded on {}: mean={mean_ms:.3} ms, p99={p99_ms:.3} ms (must be < 4.0 ms)",
            r.workload_id
        );
    }

    let _ = std::fs::create_dir_all("docs/evidence");
    let json_doc = format_r11_json_report(
        cpu,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
        &p99_ms_list,
    );
    let md_doc = format_r11_markdown_report(
        cpu,
        mem_bytes,
        rustc_ver,
        profile,
        &domain_cases,
        &render_cases,
        &p99_ms_list,
    );
    std::fs::write(
        "docs/evidence/r11_volume_profile_vwap_rs_benchmark.json",
        json_doc,
    )
    .expect("write r11 benchmark json");
    std::fs::write(
        "docs/evidence/r11_volume_profile_vwap_rs_benchmark.md",
        md_doc,
    )
    .expect("write r11 benchmark md");
    println!(
        "\nSaved R11 benchmark artifacts to docs/evidence/r11_volume_profile_vwap_rs_benchmark.{{json,md}}"
    );
}

fn bench_r11_full_workstation_ui(
    candles_50k_1m: &[Candle],
    mode: u8,
    workload_id: &'static str,
    description: &'static str,
    warmup: usize,
    iters: usize,
) -> (RenderCaseResult, f64) {
    use sumi_workstation::data::DataSourcePreset;

    let ctx = egui::Context::default();
    SumiWorkstationApp::configure_dark_workstation_visuals(&ctx);
    let screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
    ctx.set_pixels_per_point(2.0);

    let mut app = SumiWorkstationApp::default();
    match mode {
        0 => {
            app.select_watchlist_instrument(DataSourcePreset::Vn30Basket, "FPT");
            app.indicators = IndicatorConfig {
                show_sma: true,
                show_ema: true,
                show_bbands: false,
                show_vwap: true,
                show_vrvp: true,
                vrvp_rows: 48,
                show_rsi: false,
                show_macd: false,
                show_atr: false,
                show_rs: true,
                rs_sma_period: 20,
                ..IndicatorConfig::default()
            };
            app.show_compare_overlay = true;
            app.set_compare_benchmark(Some("VNINDEX".to_owned()));
            let n = app.candles.len();
            if n >= 20 {
                let _ = app.add_frvp_drawing((n * 42) / 100, (n * 62) / 100, 32);
                let _ = app.add_anchored_vwap_drawing((n * 45) / 100, true);
            }
            app.sync_indicators_and_benchmarks();
            app.viewport.time_scale.set_width(1190.0);
            app.viewport.time_scale.set_bar_spacing(9.0);
            app.viewport.time_scale.set_right_offset(18.0);
        }
        1 => {
            let csv_50k = build_r06_csv_document(candles_50k_1m);
            app.load_from_csv_str(&csv_50k, &DataLoadOptions::default())
                .expect("load 50k csv");
            app.indicators = IndicatorConfig {
                show_sma: false,
                show_ema: true,
                show_bbands: false,
                show_vwap: true,
                show_vrvp: true,
                vrvp_rows: 64,
                show_rsi: false,
                show_macd: false,
                show_atr: false,
                show_rs: true,
                rs_sma_period: 20,
                ..IndicatorConfig::default()
            };
            let n = app.candles.len();
            if n >= 500 {
                let _ = app.add_anchored_vwap_drawing(n - 320, true);
                let _ = app.add_frvp_drawing(n - 260, n - 40, 36);
            }
            app.sync_indicators_and_benchmarks();
            app.viewport.time_scale.set_width(1190.0);
            app.viewport.time_scale.set_bar_spacing(8.5);
            app.viewport.time_scale.set_right_offset(16.0);
        }
        2 => {
            let csv_50k = build_r06_csv_document(candles_50k_1m);
            app.load_from_csv_str(&csv_50k, &DataLoadOptions::default())
                .expect("load 50k csv");
            app.indicators = IndicatorConfig {
                show_sma: true,
                show_ema: false,
                show_bbands: false,
                show_vwap: true,
                show_vrvp: true,
                vrvp_rows: 64,
                show_rsi: false,
                show_macd: false,
                show_atr: false,
                show_rs: true,
                rs_sma_period: 20,
                ..IndicatorConfig::default()
            };
            app.show_compare_overlay = true;
            app.set_compare_benchmark(Some("VNINDEX".to_owned()));
            let n = app.candles.len();
            if n >= 1000 {
                let _ = app.add_anchored_vwap_drawing(n / 2, true);
                let _ = app.add_frvp_drawing((n * 40) / 100, (n * 70) / 100, 32);
            }
            app.sync_indicators_and_benchmarks();
            app.viewport.time_scale.set_width(1190.0);
            app.viewport.fit_all();
        }
        _ => {
            let csv_50k = build_r06_csv_document(candles_50k_1m);
            app.load_from_csv_str(&csv_50k, &DataLoadOptions::default())
                .expect("load 50k csv");
            app.indicators = IndicatorConfig {
                show_sma: false,
                show_ema: false,
                show_bbands: false,
                show_vwap: true,
                show_vrvp: true,
                vrvp_rows: 64,
                show_rsi: false,
                show_macd: false,
                show_atr: false,
                show_rs: true,
                rs_sma_period: 14,
                ..IndicatorConfig::default()
            };
            app.show_compare_overlay = true;
            app.set_compare_benchmark(Some("VNINDEX".to_owned()));
            let n = app.candles.len();
            if n >= 2000 {
                let _ = app.add_anchored_vwap_drawing(n - 1_500, true);
                let _ = app.add_frvp_drawing(n - 1_200, n - 400, 32);
            }
            app.sync_indicators_and_benchmarks();
            app.viewport.time_scale.set_width(1190.0);
            app.viewport.time_scale.set_bar_spacing(1.6);
            app.viewport.time_scale.set_right_offset(80.0);
        }
    }

    for _ in 0..2 {
        let mut init_out = ctx.run_ui(
            egui::RawInput {
                screen_rect,
                ..Default::default()
            },
            |ui| app.render_workstation_ui(ui),
        );
        init_out.textures_delta.clear();
    }

    let mut ui_samples = Vec::with_capacity(iters);
    let mut tess_samples = Vec::with_capacity(iters);
    let mut total_samples = Vec::with_capacity(iters);
    let mut paint_shapes = 0_usize;
    let mut clipped_primitives = 0_usize;
    let mut total_vertices = 0_usize;
    let mut total_indices = 0_usize;

    for step in 0..(warmup + iters) {
        let hover_x = 300.0 + ((step % 22) as f32) * 20.0;
        let hover_y = if step == 0 || mode == 3 {
            240.0
        } else if step % 2 == 0 {
            220.0
        } else {
            620.0
        };
        let mut events = vec![egui::Event::PointerMoved(Pos2::new(hover_x, hover_y))];
        if mode == 3 {
            let phase = (step / 6) % 2;
            let pan_dx = if step % 3 == 2 {
                0.0
            } else if phase == 0 {
                -36.0
            } else {
                36.0
            };
            let zoom_dy = if step % 3 == 2 {
                if phase == 0 { -20.0 } else { 20.0 }
            } else {
                0.0
            };
            events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(pan_dx, zoom_dy),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            });
        }
        let raw_input = egui::RawInput {
            screen_rect,
            time: Some(step as f64 * 0.016),
            events,
            ..Default::default()
        };

        let t_frame = Instant::now();
        let t_ui = Instant::now();
        let mut full_out = ctx.run_ui(raw_input, |ui| {
            app.render_workstation_ui(ui);
        });
        let ui_us = t_ui.elapsed().as_secs_f64() * 1_000_000.0;

        paint_shapes = full_out.shapes.len();
        let t_tess = Instant::now();
        let primitives = ctx.tessellate(full_out.shapes, full_out.pixels_per_point);
        let tess_us = t_tess.elapsed().as_secs_f64() * 1_000_000.0;
        let total_us = t_frame.elapsed().as_secs_f64() * 1_000_000.0;
        full_out.textures_delta.clear();

        if step + 1 == warmup + iters {
            clipped_primitives = primitives.len();
            total_vertices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            total_indices = primitives
                .iter()
                .map(|p| match &p.primitive {
                    egui::epaint::Primitive::Mesh(m) => m.indices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
        }

        if step >= warmup {
            ui_samples.push(ui_us);
            tess_samples.push(tess_us);
            total_samples.push(total_us);
        }
    }

    let mut sorted_tot = total_samples.clone();
    sorted_tot.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p99_idx =
        ((sorted_tot.len() as f64 * 0.99).floor() as usize).min(sorted_tot.len().saturating_sub(1));
    let p99_ms = sorted_tot.get(p99_idx).copied().unwrap_or(0.0) / 1_000.0;

    let rep = app.last_report.expect("full app r11 report");
    (
        RenderCaseResult {
            dataset_bars: app.candles.len(),
            workload_id,
            description,
            scale_mode: app.viewport.main_price_scale.mode().label(),
            conflation_enabled: true,
            bar_spacing: rep.bar_spacing,
            conflation_factor: rep.conflation_factor,
            culled_bars: rep.culled_bar_count,
            rendered_candles: rep.rendered_candle_count,
            paint_shapes,
            clipped_primitives,
            total_vertices,
            total_indices,
            ui_build_stats: TimingStats::from_samples(ui_samples),
            tessellate_stats: TimingStats::from_samples(tess_samples),
            total_cpu_frame_stats: TimingStats::from_samples(total_samples),
        },
        p99_ms,
    )
}

fn format_r11_json_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R09DomainBenchCase],
    render: &[RenderCaseResult],
    p99_ms_list: &[f64],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(s, "{{");
    let overall_pass = render
        .iter()
        .zip(p99_ms_list.iter())
        .all(|(r, p99)| (r.total_cpu_frame_stats.mean_us / 1_000.0) < 4.0 && *p99 < 4.0);
    let _ = writeln!(s, "  \"milestone\": \"R11\",");
    let _ = writeln!(s, "  \"overall_pass\": {overall_pass},");
    let _ = writeln!(s, "  \"environment\": {{");
    let _ = writeln!(s, "    \"cpu\": \"{}\",", cpu.replace('"', "\\\""));
    let _ = writeln!(s, "    \"os\": \"{}\",", std::env::consts::OS);
    let _ = writeln!(s, "    \"arch\": \"{}\",", std::env::consts::ARCH);
    let _ = writeln!(s, "    \"ram_bytes\": {mem_bytes},");
    let _ = writeln!(s, "    \"rustc\": \"{}\",", rustc_ver.replace('"', "\\\""));
    let _ = writeln!(s, "    \"profile\": \"{profile}\",");
    let _ = writeln!(s, "    \"viewport_logical_px\": [1280, 800],");
    let _ = writeln!(s, "    \"pixels_per_point\": 2.0");
    let _ = writeln!(s, "  }},");
    let _ = writeln!(s, "  \"r11_domain_throughput\": [");
    for (idx, d) in domain.iter().enumerate() {
        let comma = if idx + 1 == domain.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"case_id\": \"{}\", \"description\": \"{}\", \"input_units\": {}, \"output_items\": {}, \"median_us\": {:.3}, \"median_ms\": {:.4}, \"p95_us\": {:.3}, \"max_us\": {:.3} }}{comma}",
            d.case_id,
            d.description.replace('"', "\\\""),
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }
    let _ = writeln!(s, "  ],");
    let _ = writeln!(s, "  \"workstation_r11_workloads\": [");
    for (idx, (r, p99_ms)) in render.iter().zip(p99_ms_list.iter()).enumerate() {
        let comma = if idx + 1 == render.len() { "" } else { "," };
        let _ = writeln!(
            s,
            "    {{ \"dataset_bars\": {}, \"workload_id\": \"{}\", \"scale_mode\": \"{}\", \"bar_spacing\": {:.5}, \"conflation_factor\": {}, \"culled_bars\": {}, \"rendered_candles\": {}, \"paint_shapes\": {}, \"clipped_primitives\": {}, \"total_vertices\": {}, \"total_indices\": {}, \"ui_median_ms\": {:.4}, \"tessellate_median_ms\": {:.4}, \"total_cpu_mean_ms\": {:.4}, \"total_cpu_median_ms\": {:.4}, \"total_cpu_p95_ms\": {:.4}, \"total_cpu_p99_ms\": {:.4} }}{comma}",
            r.dataset_bars,
            r.workload_id,
            r.scale_mode,
            r.bar_spacing,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.paint_shapes,
            r.clipped_primitives,
            r.total_vertices,
            r.total_indices,
            r.ui_build_stats.median_us / 1_000.0,
            r.tessellate_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.mean_us / 1_000.0,
            r.total_cpu_frame_stats.median_us / 1_000.0,
            r.total_cpu_frame_stats.p95_us / 1_000.0,
            p99_ms,
        );
    }
    let _ = writeln!(s, "  ]");
    let _ = writeln!(s, "}}");
    s
}

fn format_r11_markdown_report(
    cpu: &str,
    mem_bytes: u64,
    rustc_ver: &str,
    profile: &str,
    domain: &[R09DomainBenchCase],
    render: &[RenderCaseResult],
    p99_ms_list: &[f64],
) -> String {
    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        "# Milestone R11 — Order-Flow Volume Profile (`VRVP`/`FRVP`), Session/Anchored VWAP & Causal Relative Strength vs `VNINDEX` Benchmark Evidence\n"
    );
    let _ = writeln!(
        s,
        "- **Command:** `cargo run --release --example viewport_bench -- --r11`"
    );
    let _ = writeln!(
        s,
        "- **Hardware:** `{cpu}` (`{}-{}`), `{:.1} GiB` RAM",
        std::env::consts::ARCH,
        std::env::consts::OS,
        (mem_bytes as f64) / (1024.0 * 1024.0 * 1024.0)
    );
    let _ = writeln!(
        s,
        "- **Toolchain & Profile:** `{rustc_ver}` (`--{profile}`)"
    );
    let _ = writeln!(
        s,
        "- **Active Stack:** Intrabar `VRVP`/`FRVP` (Exact Buy/Sell conservation + 70% VA + POC) + `Session VWAP` (`UTC+7` reset, ±1σ/±2σ Welford bands) + `Anchored VWAP` (`DEF-001` causal) + `RS vs VNINDEX` + Rebased Compare Overlay\n"
    );

    let _ = writeln!(
        s,
        "## 1. Volume Profile, VWAP & Causal Relative Strength Kernel Throughput\n"
    );
    let _ = writeln!(
        s,
        "| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |"
    );
    let _ = writeln!(
        s,
        "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |"
    );
    for d in domain {
        let _ = writeln!(
            s,
            "| `{}` | {} | `{}` | `{}` | **`{:.2} µs`** | **`{:.3} ms`** | `{:.2} µs` | `{:.2} µs` |",
            d.case_id,
            d.description,
            d.input_units,
            d.output_items,
            d.stats.median_us,
            d.stats.median_us / 1_000.0,
            d.stats.p95_us,
            d.stats.max_us,
        );
    }

    let _ = writeln!(
        s,
        "\n## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)\n"
    );
    let _ = writeln!(
        s,
        "| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |"
    );
    let _ = writeln!(
        s,
        "| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |"
    );
    for (r, p99_ms) in render.iter().zip(p99_ms_list.iter()) {
        let mean_ms = r.total_cpu_frame_stats.mean_us / 1_000.0;
        let med_ms = r.total_cpu_frame_stats.median_us / 1_000.0;
        let gate = if mean_ms < 4.0 && *p99_ms < 4.0 {
            "PASS (<4ms)"
        } else {
            "EXCEEDS"
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` ({}) | `{}` | `{}x` | `{}` | `{}` | `{}` | `{}` | **`{:.3}`** | **`{:.3}`** | `{:.3}` | `{gate}` |",
            r.dataset_bars,
            r.workload_id,
            r.description,
            r.scale_mode,
            r.conflation_factor,
            r.culled_bars,
            r.rendered_candles,
            r.total_vertices,
            r.total_indices,
            mean_ms,
            med_ms,
            p99_ms,
        );
    }
    s
}
