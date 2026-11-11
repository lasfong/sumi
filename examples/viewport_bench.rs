use std::fmt::Write as _;
use std::process::Command;
use std::time::Instant;

use egui::{Pos2, Rect, Vec2};
use sumi_workstation::chart::{
    ChartDisplayConfig, ChartViewportState, render_candlestick_chart,
    render_shared_pane_candlestick_chart, render_workstation_chart,
    render_workstation_chart_with_drawings,
};
use sumi_workstation::data::generate_synthetic_ohlcv;
use sumi_workstation::drawings::{
    DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStore, FibDirection, LocalPoint,
    MAGNET_THRESHOLD_PX, MagnetMode, PaneDrawingProjection, hit_test_pane_drawings,
    snap_to_visible_ohlc,
};
use sumi_workstation::indicators::{
    BlockExtrema, IndicatorCache, IndicatorConfig, OscillatorPaneId, SeriesBlockIndex,
};
use sumi_workstation::model::Candle;
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
                    DrawingPaneTarget::Oscillator(OscillatorPaneId::Atr) => {
                        atr_proj.project_drawing(d)
                    }
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
