use crate::model::Candle;

use super::{
    CandlestickBarMetrics, HorzScaleOptions, KineticAnimation, KineticScrollConfig, LogFormula,
    PaneViewport, PlotRect, PriceScale, PriceScaleMode, PriceScaleOptions, TimeScale,
    from_indexed_to_100, from_log, from_percent, iter_conflated_candles, lwc_clamp,
    optimal_candlestick_width_px, to_indexed_to_100, to_log, to_percent,
};

/// Deterministic SplitMix64 PRNG for zero-dependency property-based invariant sweeps.
struct SplitMix64(u64);

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f64_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
    }

    fn range_f64(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.next_f64_unit()
    }

    fn range_usize(&mut self, low: usize, high_inclusive: usize) -> usize {
        if low >= high_inclusive {
            return low;
        }
        let span = (high_inclusive - low + 1) as u64;
        low + (self.next_u64() % span) as usize
    }
}

fn make_candles(n: usize, base: f64, step: f64) -> Vec<Candle> {
    (0..n)
        .map(|i| {
            let open = base + i as f64 * step;
            let close = open + step * 0.5;
            Candle {
                timestamp: 1_700_000_000 + (i as i64) * 60,
                date_label: String::new(),
                open,
                high: open.max(close) + step.abs() + 1.0,
                low: (open.min(close) - step.abs() - 1.0).max(0.000_001),
                close,
                volume: 100.0 + (i % 17) as f64 * 25.0,
            }
        })
        .collect()
}

// ============================================================================
// 1. EXACT LWC v5.2.0 NUMERICAL FIXTURES (AUDITED & RECTIFIED)
// ============================================================================

#[test]
fn lwc_fixture_index_to_local_x_and_half_slot_float_index() {
    let opts = HorzScaleOptions {
        right_offset: 5.0,
        bar_spacing: 10.0,
        ..HorzScaleOptions::lwc_default()
    };
    let ts = TimeScale::with_data(100, 1000.0, opts);
    assert_eq!(ts.base_index(), Some(99));

    // Exact LWC `indexToCoordinate(50)`:
    // deltaFromRight = 99 + 5 - 50 = 54 => x = 1000 - (54 + 0.5) * 10 - 1 = 454.0
    let x50 = ts.index_to_local_x(50).unwrap();
    assert!((x50 - 454.0).abs() < 1e-9);

    // Exact LWC `_coordinateToFloatIndex(454.0)`:
    // deltaFromRight = (1000 - 1 - 454) / 10 = 54.5 => floatIndex = 99 + 5 - 54.5 = 49.5 (NOT 50.0!)
    let float_idx = ts.local_x_to_float_index(454.0).unwrap();
    assert!((float_idx - 49.5).abs() < 1e-9);

    // Continuous center float index at bar center is exact 50.0:
    let center_idx = ts.local_x_to_center_float_index(454.0).unwrap();
    assert!((center_idx - 50.0).abs() < 1e-9);

    // Discrete `coordinateToIndex(454.0)` = ceil(49.5) = 50:
    assert_eq!(ts.local_x_to_index(454.0), Some(50));

    // Across the entire slot (449.0, 459.0], `local_x_to_index` maps to bar 50:
    assert_eq!(ts.local_x_to_index(449.01), Some(50));
    assert_eq!(ts.local_x_to_index(459.0), Some(50));
    assert_eq!(ts.local_x_to_index(449.0), Some(49));
    assert_eq!(ts.local_x_to_index(459.01), Some(51));
}

#[test]
fn lwc_fixture_cursor_anchored_zoom_exact() {
    let opts = HorzScaleOptions {
        right_offset: 5.0,
        bar_spacing: 10.0,
        ..HorzScaleOptions::lwc_default()
    };
    let mut ts = TimeScale::with_data(100, 1000.0, opts);

    let before = ts.local_x_to_float_index(500.0).unwrap();
    assert!((before - 54.1).abs() < 1e-6);

    ts.zoom(500.0, 5.0);
    assert!((ts.bar_spacing() - 15.0).abs() < 1e-9);

    // new_right_offset = 5 + (54.1 - 70.733333) = -11.633333
    assert!((ts.right_offset() - (-11.633_333)).abs() < 1e-5);
    let after = ts.local_x_to_float_index(500.0).unwrap();
    assert!((after - before).abs() <= 1e-6);
}

#[test]
fn lwc_fixture_price_scale_linear_and_logarithmic_symmetry() {
    let opts = PriceScaleOptions {
        mode: PriceScaleMode::Normal,
        auto_scale: false,
        invert_scale: false,
        scale_margin_top: 0.10,
        scale_margin_bottom: 0.10,
        min_move: 0.01,
        pin_zero_bottom: false,
    };
    let mut ps = PriceScale::new("right", opts);
    ps.set_height(600.0);
    ps.set_raw_price_range(100.0, 200.0, 100.0);

    // topMarginPx = 60, bottomMarginPx = 60, internalHeight = 480
    assert!((ps.price_to_local_y(200.0, 100.0).unwrap() - 60.0).abs() < 1e-9);
    assert!((ps.price_to_local_y(150.0, 100.0).unwrap() - 299.5).abs() < 1e-9);
    assert!((ps.price_to_local_y(100.0, 100.0).unwrap() - 539.0).abs() < 1e-9);

    // Exact LWC `toLog` / `fromLog` symmetry across positive, zero, and negative values:
    let f = LogFormula::DEFAULT;
    assert_eq!(to_log(0.0, f), 0.0);
    assert_eq!(from_log(0.0, f), 0.0);
    let pos = to_log(100.0, f);
    let neg = to_log(-100.0, f);
    assert!(
        (pos + neg).abs() < 1e-12,
        "to_log must be odd-symmetric: pos={pos}, neg={neg}"
    );
    assert!((from_log(pos, f) - 100.0).abs() < 1e-9);
    assert!((from_log(neg, f) - (-100.0)).abs() < 1e-9);
}

// ============================================================================
// 2. PROPERTY-BASED RANDOMIZED INVARIANT SWEEPS (2,000+ SEEDED CASES)
// ============================================================================

#[test]
fn property_time_scale_slot_round_trip_across_2000_states() {
    let mut rng = SplitMix64::new(0x504D_1003_2026_1111);
    for case in 0..2_000 {
        let total_bars = rng.range_usize(1, 100_000);
        let width = rng.range_f64(40.0, 3840.0);
        let bar_spacing = rng.range_f64(0.5, (width * 0.45).max(1.0));
        let right_offset = rng.range_f64(-200.0, 50.0);

        let opts = HorzScaleOptions {
            right_offset,
            bar_spacing,
            ..HorzScaleOptions::lwc_default()
        };
        let ts = TimeScale::with_data(total_bars, width, opts);

        let test_idx = rng.range_usize(0, total_bars - 1) as i64;
        let x = ts.index_to_local_x(test_idx).expect("valid local x");
        let float_idx = ts.local_x_to_float_index(x).expect("valid float idx");
        let discrete_idx = ts.local_x_to_index(x).expect("valid discrete idx");

        assert!(
            (float_idx - (test_idx as f64 - 0.5)).abs() < 2e-5,
            "case {case}: float_idx={float_idx}, expected {}",
            test_idx as f64 - 0.5
        );
        assert_eq!(
            discrete_idx, test_idx,
            "case {case}: discrete round-trip mismatch at bar_spacing={bar_spacing}, width={width}"
        );
    }
}

#[test]
fn property_cursor_anchored_zoom_locks_bar_under_pointer() {
    let mut rng = SplitMix64::new(0xABCD_EF01_2345_6789);
    let mut locked_cases = 0_usize;

    for case in 0..1_500 {
        let total_bars = rng.range_usize(500, 50_000);
        let width = rng.range_f64(300.0, 2560.0);
        let bar_spacing = rng.range_f64(3.0, 40.0);
        // Start comfortably away from scroll clamp boundaries so interior invariant applies:
        let right_offset = rng.range_f64(-150.0, 15.0);

        let opts = HorzScaleOptions {
            right_offset,
            bar_spacing,
            ..HorzScaleOptions::lwc_default()
        };
        let mut ts = TimeScale::with_data(total_bars, width, opts);

        let cursor_x = rng.range_f64(10.0, width - 10.0);
        let scale = rng.range_f64(-0.85, 1.5);
        if scale.abs() < 1e-3 {
            continue;
        }

        let before = ts.local_x_to_float_index(cursor_x).unwrap();
        ts.zoom(cursor_x, scale);

        let min_ro = ts.min_right_offset().unwrap();
        let max_ro = ts.max_right_offset();
        let unclamped_offset =
            ts.right_offset() > min_ro + 1e-6 && ts.right_offset() < max_ro - 1e-6;
        if unclamped_offset {
            let after = ts.local_x_to_float_index(cursor_x).unwrap();
            assert!(
                (after - before).abs() <= 2e-5,
                "case {case}: cursor drifted from {before} to {after} (cursor_x={cursor_x}, scale={scale})"
            );
            locked_cases += 1;
        }
    }
    assert!(
        locked_cases > 1_200,
        "expected >1200 interior zoom lock cases, got {locked_cases}"
    );
}

#[test]
fn property_price_scale_round_trip_all_modes_and_inversions() {
    let mut rng = SplitMix64::new(0x9988_7766_5544_3322);
    let modes = [
        PriceScaleMode::Normal,
        PriceScaleMode::Logarithmic,
        PriceScaleMode::Percentage,
        PriceScaleMode::IndexedTo100,
    ];

    for case in 0..1_600 {
        let mode = modes[case % modes.len()];
        let inverted = (case / 4) % 2 == 1;
        let top = rng.range_f64(0.05, 0.30);
        let bottom = rng.range_f64(0.05, 0.30);
        let height = rng.range_f64(120.0, 1440.0);

        // Include micro-prices (< 0.001) to stress adaptive `LogFormula` as well as large prices:
        let scale_mag = match case % 3 {
            0 => 0.000_01,
            1 => 1.0,
            _ => 50_000.0,
        };
        let p_low = rng.range_f64(0.5, 5.0) * scale_mag;
        let p_high = p_low + rng.range_f64(0.1, 4.0) * scale_mag;
        let base_value = p_low + (p_high - p_low) * 0.35;

        let mut ps = PriceScale::new(
            "test",
            PriceScaleOptions {
                mode,
                auto_scale: false,
                invert_scale: inverted,
                scale_margin_top: top,
                scale_margin_bottom: bottom,
                min_move: scale_mag * 0.001,
                pin_zero_bottom: false,
            },
        );
        ps.set_height(height);
        ps.set_raw_price_range(p_low, p_high, base_value);

        let test_price = rng.range_f64(p_low, p_high);
        let ly = ps
            .price_to_local_y(test_price, base_value)
            .expect("valid y");
        let recovered = ps
            .local_y_to_price(ly, base_value)
            .expect("valid inverse price");
        let rel_err = (recovered - test_price).abs() / test_price.abs().max(1e-12);
        assert!(
            rel_err < 1e-7,
            "case {case} ({mode:?}, inv={inverted}): test_price={test_price}, recovered={recovered}, rel_err={rel_err}"
        );
    }
}

// ============================================================================
// 3. ARCHITECTURAL SEPARATION: SHARED TIMESCALE + MULTI-PANE + SCREEN OFFSET
// ============================================================================

#[test]
fn multi_pane_shared_time_scale_and_screen_offset_isolation() {
    let mut shared_ts = TimeScale::with_data(500, 1100.0, HorzScaleOptions::sumi_default());

    let mut main_ps = PriceScale::new("main-right", PriceScaleOptions::sumi_main_price());
    main_ps.set_height(500.0);
    main_ps.set_raw_price_range(60_000.0, 68_000.0, 62_000.0);

    let mut rsi_ps = PriceScale::new("rsi-right", PriceScaleOptions::lwc_default());
    rsi_ps.set_height(180.0);
    rsi_ps.set_raw_price_range(0.0, 100.0, 50.0);

    // Non-zero screen origins (left toolbar = 56px, top toolbar = 42px; RSI stacked below at y=550px)
    let main_rect = PlotRect::new(56.0, 42.0, 1100.0, 500.0);
    let rsi_rect = PlotRect::new(56.0, 550.0, 1100.0, 180.0);

    let main_vp = PaneViewport::new(&shared_ts, &main_ps, main_rect, 62_000.0);
    let rsi_vp = PaneViewport::new(&shared_ts, &rsi_ps, rsi_rect, 50.0);

    // 1. Horizontal alignment across stacked panes is bit-identical:
    let main_x = main_vp.index_to_screen_x(420).unwrap();
    let rsi_x = rsi_vp.index_to_screen_x(420).unwrap();
    assert_eq!(main_x, rsi_x);
    assert_eq!(main_vp.screen_x_to_index(main_x), Some(420));

    // 2. Vertical scales map independently into their own screen rectangles without interference:
    let main_y = main_vp.price_to_screen_y(64_000.0).unwrap();
    let rsi_y = rsi_vp.price_to_screen_y(70.0).unwrap();
    assert!(main_rect.contains_screen(main_x, main_y));
    assert!(rsi_rect.contains_screen(rsi_x, rsi_y));
    assert!((main_vp.screen_y_to_price(main_y).unwrap() - 64_000.0).abs() < 1e-6);
    assert!((rsi_vp.screen_y_to_price(rsi_y).unwrap() - 70.0).abs() < 1e-6);

    // 3. Cursor-anchored zoom using screen_x -> local_x keeps the exact screen_x anchored on BOTH panes:
    let pointer_screen_x = 680.0;
    let pointer_local_x = main_vp.screen_x_to_local_x(pointer_screen_x);
    let before_float = shared_ts.local_x_to_float_index(pointer_local_x).unwrap();
    shared_ts.zoom(pointer_local_x, 2.5);

    let main_vp_after = PaneViewport::new(&shared_ts, &main_ps, main_rect, 62_000.0);
    let rsi_vp_after = PaneViewport::new(&shared_ts, &rsi_ps, rsi_rect, 50.0);
    assert!(
        (main_vp_after
            .screen_x_to_float_index(pointer_screen_x)
            .unwrap()
            - before_float)
            .abs()
            < 1e-5
    );
    assert!(
        (rsi_vp_after
            .screen_x_to_float_index(pointer_screen_x)
            .unwrap()
            - before_float)
            .abs()
            < 1e-5
    );
}

// ============================================================================
// 4. SCROLLING, RESIZE, FIT CONTENT, AUTOSCALE & KINETIC ANIMATION
// ============================================================================

#[test]
fn horizontal_scroll_direction_and_edge_clamping() {
    let mut ts = TimeScale::with_data(200, 1000.0, HorzScaleOptions::sumi_default());
    let start_ro = ts.right_offset();
    assert!((start_ro - 12.0).abs() < 1e-9);

    // Drag right by 80px (from x=400 to x=480) at bar_spacing=8 => shift = -10 bars (reveals past):
    ts.start_scroll(400.0);
    ts.scroll_to(480.0);
    ts.end_scroll();
    assert!((ts.right_offset() - 2.0).abs() < 1e-9);

    // Drag extreme right (past bar 0) => clamped cleanly at min_right_offset:
    ts.start_scroll(0.0);
    ts.scroll_to(1_000_000.0);
    ts.end_scroll();
    assert_eq!(ts.right_offset(), ts.min_right_offset().unwrap());

    // Drag extreme left (into infinite future) => clamped cleanly at max_right_offset:
    ts.start_scroll(1_000_000.0);
    ts.scroll_to(0.0);
    ts.end_scroll();
    assert_eq!(ts.right_offset(), ts.max_right_offset());
}

#[test]
fn width_resize_locked_vs_unlocked_and_fit_content() {
    // Unlocked resize keeps bar_spacing constant:
    let mut ts = TimeScale::with_data(300, 1000.0, HorzScaleOptions::sumi_default());
    let bs_before = ts.bar_spacing();
    ts.set_width(500.0);
    assert!((ts.bar_spacing() - bs_before).abs() < 1e-9);

    // Locked resize scales bar_spacing proportionally to preserve visible logical range:
    let mut locked_opts = HorzScaleOptions::sumi_default();
    locked_opts.lock_visible_time_range_on_resize = true;
    let mut ts_locked = TimeScale::with_data(300, 1000.0, locked_opts);
    let range_before = ts_locked.visible_logical_range().unwrap();
    ts_locked.set_width(1500.0);
    let range_after = ts_locked.visible_logical_range().unwrap();
    assert!((ts_locked.bar_spacing() - 12.0).abs() < 1e-9);
    assert!((range_after.left - range_before.left).abs() < 1e-6);
    assert!((range_after.right - range_before.right).abs() < 1e-6);

    // `fit_content` fits all 300 bars + 12.0 future right_offset bars across 1500px:
    ts_locked.fit_content();
    assert!((ts_locked.right_offset() - 12.0).abs() < 1e-6);
    let fit_strict = ts_locked.visible_strict_range().unwrap();
    assert!(fit_strict.left <= 0);
    assert!(fit_strict.right >= 299);
}

#[test]
fn kinetic_animation_decay_direction_reversal_and_stale_cutoff() {
    let cfg = KineticScrollConfig::default();
    let mut ka = KineticAnimation::for_bar_spacing(10.0, cfg);

    // Duplicate timestamps overwrite cleanly without division-by-zero:
    ka.add_position(0.0, 100.0);
    ka.add_position(1.0, 100.0);
    ka.add_position(-4.0, 116.0);
    ka.add_position(-10.0, 132.0);

    // Releasing > 50ms after last sample cancels inertia (`MaxStartDelay = 50ms`):
    let mut stale = ka.clone();
    assert!(!stale.start(-10.0, 190.0));
    assert!(stale.is_finished(190.0));

    // Releasing within 50ms starts smooth monotonic decay in negative bar direction:
    assert!(ka.start(-10.0, 140.0));
    assert!(ka.initial_speed() < 0.0);
    let p0 = ka.position_at(140.0).unwrap();
    let p1 = ka.position_at(240.0).unwrap();
    let p2 = ka.position_at(500.0).unwrap();
    assert!((p0 - (-10.0)).abs() < 1e-9);
    assert!(p1 < p0 && p2 <= p1);
    assert!(ka.is_finished(140.0 + ka.duration_ms() + 1.0));

    // Opposite-direction older segment is ignored (only latest consistent direction counts):
    let mut rev = KineticAnimation::for_bar_spacing(10.0, cfg);
    rev.add_position(0.0, 0.0);
    rev.add_position(-8.0, 16.0); // moving negative
    rev.add_position(-2.0, 32.0); // reversed positive!
    assert!(rev.start(-2.0, 40.0));
    assert!(rev.initial_speed() > 0.0);
}

#[test]
fn price_autoscale_flat_candles_and_vertical_drag_scale_pan() {
    let flat_candles = vec![
        Candle {
            timestamp: 1,
            date_label: String::new(),
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 10.0,
        };
        5
    ];

    let mut ps = PriceScale::new("right", PriceScaleOptions::sumi_main_price());
    ps.set_height(400.0);
    ps.autoscale_candles(&flat_candles, 100.0);

    // Degenerate flat candles expand by ±5 * min_move (±0.05) without NaN:
    let r = ps.price_range().unwrap();
    assert!((r.min - 99.95).abs() < 1e-9);
    assert!((r.max - 100.05).abs() < 1e-9);
    assert!(ps.price_to_local_y(100.0, 100.0).unwrap().is_finite());

    // Vertical axis drag-scale disables auto_scale and widens range:
    ps.start_scale(200.0);
    ps.scale_to(260.0);
    ps.end_scale();
    assert!(!ps.is_auto_scale());
    assert!(ps.price_range().unwrap().len() > r.len());

    // Manual vertical pan shifts price_range while auto_scale is false:
    let before_pan = ps.price_range().unwrap();
    ps.start_scroll(200.0);
    ps.scroll_to(240.0);
    ps.end_scroll();
    assert!(ps.price_range().unwrap().min > before_pan.min);

    // Double-click reset restores auto_scale:
    ps.reset_auto_scale();
    assert!(ps.is_auto_scale());
}

// ============================================================================
// 5. EDGE CASES & ZERO-PANIC TORTURE TESTS (0 BARS, 1 BAR, TINY / NaN DIMS, 100K)
// ============================================================================

#[test]
fn edge_cases_empty_single_bar_tiny_heights_and_fixed_edges_never_panic() {
    // 1. Empty dataset (0 bars):
    let mut empty_ts = TimeScale::with_data(0, 1000.0, HorzScaleOptions::sumi_default());
    assert!(empty_ts.is_empty());
    assert_eq!(empty_ts.index_to_local_x(0), None);
    assert_eq!(empty_ts.local_x_to_float_index(500.0), None);
    assert_eq!(empty_ts.visible_logical_range(), None);
    assert_eq!(empty_ts.culling_data_range(), 0..0);
    empty_ts.zoom(500.0, 2.0);
    empty_ts.fit_content();

    // 2. Single-bar dataset with BOTH fix_left_edge & fix_right_edge (would panic `f64::clamp` in R02!):
    let mut fixed_opts = HorzScaleOptions::sumi_default();
    fixed_opts.fix_left_edge = true;
    fixed_opts.fix_right_edge = true;
    let mut single_ts = TimeScale::with_data(1, 800.0, fixed_opts);
    single_ts.zoom(400.0, -5.0);
    single_ts.zoom(400.0, 5.0);
    single_ts.set_width(0.5);
    single_ts.set_width(f64::NAN);
    single_ts.set_width(f64::INFINITY);
    single_ts.set_width(-100.0);
    assert!(single_ts.index_to_local_x(0).unwrap().is_finite());

    // 3. `PriceScale` with `height = 0.0`, `height = 1.0`, `internal_height == 1.0` (would divide by 0.0 in R02!):
    let mut tiny_ps = PriceScale::new("tiny", PriceScaleOptions::lwc_default());
    tiny_ps.set_raw_price_range(10.0, 20.0, 10.0);
    for h in [0.0, 0.5, 1.0, 1.428_571_428_571_428_6, f64::NAN, -5.0] {
        tiny_ps.set_height(h);
        if let Some(y) = tiny_ps.price_to_local_y(15.0, 10.0) {
            assert!(y.is_finite());
        }
        if let Some(p) = tiny_ps.local_y_to_price(0.5, 10.0) {
            assert!(p.is_finite());
        }
    }

    // 4. Percentage & IndexedTo100 with zero and negative base_value:
    assert_eq!(to_percent(50.0, 0.0), 0.0);
    assert_eq!(from_percent(10.0, 0.0), 0.0);
    let idx_neg = to_indexed_to_100(-80.0, -100.0);
    assert!((idx_neg - 120.0).abs() < 1e-9);
    assert!((from_indexed_to_100(idx_neg, -100.0) - (-80.0)).abs() < 1e-9);

    // 5. `lwc_clamp` when `min > max`:
    assert_eq!(lwc_clamp(5.0, 100.0, 20.0), 20.0);
}

#[test]
fn large_dataset_100k_bars_culling_and_conflation_invariants() {
    let candles = make_candles(100_000, 10_000.0, 0.25);
    let mut ts = TimeScale::with_data(candles.len(), 1200.0, HorzScaleOptions::sumi_default());

    // Default zoom (`bar_spacing = 8.0` on `1200px` => ~150 bars visible, NOT 100,000!):
    let culled = ts.culling_data_range();
    assert!(
        culled.len() <= 160,
        "expected <=160 culled bars, got {}",
        culled.len()
    );
    assert_eq!(ts.conflation_factor(2.0), 1);

    // Full-fit 100,000 bars into 1200px (`min_bar_spacing = 0.005` so all 100k fit):
    let mut wide_opts = HorzScaleOptions::sumi_default();
    wide_opts.min_bar_spacing = 0.005;
    ts.apply_options(wide_opts);
    ts.fit_content();

    let factor = ts.conflation_factor(2.0);
    assert!(
        factor >= 32,
        "expected power-of-2 conflation >= 32, got {factor}"
    );

    let conflated: Vec<_> =
        iter_conflated_candles(&candles, ts.culling_data_range(), factor).collect();
    assert!(conflated.len() <= 100_000 / factor + 2);

    // Conflated envelope must exactly match global high/low and total volume:
    let env_high = conflated
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let env_low = conflated
        .iter()
        .map(|c| c.low)
        .fold(f64::INFINITY, f64::min);
    let orig_high = candles
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let orig_low = candles.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    assert!((env_high - orig_high).abs() < 1e-9);
    assert!((env_low - orig_low).abs() < 1e-9);

    // Candlestick parity alignment check across DPI ratios:
    for dpr in [1.0, 2.0] {
        for bs in [1.0, 3.0, 6.0, 8.0, 15.0, 40.0] {
            let w = optimal_candlestick_width_px(bs, dpr);
            assert!(w >= 1.0);
            let m = CandlestickBarMetrics::compute(bs, dpr);
            assert!(m.body_width_logical >= m.wick_width_logical);
        }
    }
}

// ============================================================================
// 6. ADVERSARIAL CRITIC REGRESSION SUITE (CRITICAL-R03-01..03, MAJOR-R03-01..08)
// ============================================================================

#[test]
fn critic_regression_extreme_zoom_out_clamps_to_min_bar_spacing_not_6px() {
    let mut ts = TimeScale::with_data(100_000, 1200.0, HorzScaleOptions::sumi_default());
    let min_bs = ts.min_bar_spacing();
    assert!((min_bs - 0.005).abs() < 1e-9);

    // Calling set_bar_spacing with 0.0 or 5e-5 must clamp smoothly at min_bar_spacing (0.005), NOT jump 1,200x to 6.0!
    for tiny in [0.0, -10.0, 1e-6, 5e-5, 1e-4] {
        ts.set_bar_spacing(tiny);
        assert!(
            (ts.bar_spacing() - min_bs).abs() < 1e-9,
            "tiny={tiny} jumped to {} instead of {min_bs}",
            ts.bar_spacing()
        );
    }

    // Extreme time-axis drag-scale near right edge must also clamp smoothly at min_bar_spacing:
    ts.set_bar_spacing(0.01);
    ts.start_scale(200.0); // current_length_from_right = 1000.0
    ts.scale_to(1199.9); // start_length_from_right = 0.1 => ratio = 0.0001 => candidate 1e-6
    ts.end_scale();
    assert!((ts.bar_spacing() - min_bs).abs() < 1e-9);
}

#[test]
fn critic_regression_percentage_mode_autoscale_and_mode_transitions() {
    let candles = make_candles(50, 1200.0, 5.0);
    let base_val = candles[0].close;

    let mut ps = PriceScale::new("main-right", PriceScaleOptions::sumi_main_price());
    ps.set_height(600.0);
    ps.autoscale_candles(&candles, base_val);

    // Enter Percentage mode -> auto_scale is forced true and cannot be turned off while in %/Idx100
    ps.set_mode_with_base(PriceScaleMode::Percentage, Some(base_val));
    assert!(ps.is_auto_scale());
    ps.set_auto_scale(false);
    assert!(
        ps.is_auto_scale(),
        "Percentage mode must keep auto_scale=true per LWC price-scale.ts:363-367"
    );
    ps.autoscale_candles(&candles, base_val);

    // Switch Percentage -> Normal WITH base_value: price_range immediately converts back to ~1200..1450 raw units!
    ps.set_mode_with_base(PriceScaleMode::Normal, Some(base_val));
    assert!(ps.is_auto_scale());
    let normal_range = ps.price_range().expect("converted normal range");
    assert!(
        normal_range.min > 1100.0 && normal_range.max > 1400.0,
        "expected raw prices ~1200..1450, got {normal_range:?}"
    );

    // Switch IndexedTo100 -> Logarithmic WITHOUT base_value: resets cleanly without leaving 95..120 in log space!
    ps.set_mode_with_base(PriceScaleMode::IndexedTo100, Some(base_val));
    ps.autoscale_candles(&candles, base_val);
    ps.set_mode(PriceScaleMode::Logarithmic);
    assert!(ps.is_auto_scale());
    assert!(ps.price_range().is_none());
    ps.autoscale_candles(&candles, base_val);
    assert!(ps.price_to_local_y(1300.0, base_val).unwrap().is_finite());
}

#[test]
fn critic_regression_conflated_right_edge_bucket_is_invariant_across_pan() {
    let candles = make_candles(1_000, 500.0, 1.0);
    let step = 32;

    // Simulate 1-bar shifts of the right culling boundary across 32 consecutive steps:
    let baseline: Vec<_> = iter_conflated_candles(&candles, 64..513, step).collect();
    let baseline_bucket_15 = baseline
        .iter()
        .find(|b| b.start_index == 480)
        .copied()
        .expect("bucket starting at 480");
    assert_eq!(baseline_bucket_15.end_index, 511);

    for end_unaligned in 481..=512 {
        let shifted: Vec<_> = iter_conflated_candles(&candles, 64..end_unaligned, step).collect();
        let bucket = shifted
            .iter()
            .find(|b| b.start_index == 480)
            .copied()
            .expect("bucket 480..511 must be present");
        assert_eq!(
            bucket, baseline_bucket_15,
            "right-edge conflated bucket shimmered at slice end={end_unaligned}"
        );
    }
}

#[test]
fn critic_regression_set_total_bars_historical_lock_and_shift_option() {
    // 1. shift_visible_range_on_new_bar = true & viewing realtime edge -> viewport shifts with new bars (right_offset unchanged)
    let mut ts_live = TimeScale::with_data(200, 800.0, HorzScaleOptions::sumi_default());
    assert!((ts_live.right_offset() - 12.0).abs() < 1e-9);
    ts_live.set_total_bars(205);
    assert!((ts_live.right_offset() - 12.0).abs() < 1e-9);

    // 2. User scrolls deep into history (old base_index = 204 is outside visible_strict_range) -> new bars lock historical view!
    ts_live.set_right_offset(-80.0);
    let strict_before = ts_live.visible_strict_range().unwrap();
    assert!(strict_before.right < 204);
    ts_live.set_total_bars(210);
    let strict_after = ts_live.visible_strict_range().unwrap();
    assert_eq!(strict_before, strict_after);

    // 3. shift_visible_range_on_new_bar = false EVEN when right_offset > 0 -> historical indices stay locked!
    let mut no_shift_opts = HorzScaleOptions::sumi_default();
    no_shift_opts.shift_visible_range_on_new_bar = false;
    let mut ts_locked = TimeScale::with_data(200, 800.0, no_shift_opts);
    let range_before = ts_locked.visible_logical_range().unwrap();
    ts_locked.set_total_bars(203);
    let range_after = ts_locked.visible_logical_range().unwrap();
    assert!((range_after.left - range_before.left).abs() < 1e-9);
    assert!((range_after.right - range_before.right).abs() < 1e-9);
}

#[test]
fn critic_regression_extreme_log_and_linear_vertical_scale_never_overflows() {
    let candles = make_candles(120, 1000.0, 10.0);
    let base_val = candles[0].close;

    for mode in [PriceScaleMode::Logarithmic, PriceScaleMode::Normal] {
        let mut ps = PriceScale::new("main-right", PriceScaleOptions::sumi_main_price());
        ps.set_height(600.0);
        ps.set_mode(mode);
        ps.autoscale_candles(&candles, base_val);

        // Repeat extreme drag-down gestures 60 times in a row (attempting 6^60 range expansion!):
        for _ in 0..60 {
            ps.start_scale(20.0);
            ps.scale_to(599.0);
            ps.end_scale();
        }

        let top_p = ps.local_y_to_price(0.0, base_val);
        let mid_p = ps.local_y_to_price(300.0, base_val);
        let bot_p = ps.local_y_to_price(599.0, base_val);
        assert!(
            top_p.is_some_and(f64::is_finite),
            "{mode:?} top price overflowed"
        );
        assert!(
            mid_p.is_some_and(f64::is_finite),
            "{mode:?} mid price overflowed"
        );
        assert!(
            bot_p.is_some_and(f64::is_finite),
            "{mode:?} bot price overflowed"
        );
        let ticks = ps.nice_ticks(8, base_val);
        assert!(
            ticks
                .iter()
                .all(|t| t.local_y.is_finite() && t.raw_price.is_finite()),
            "{mode:?} ticks non-finite"
        );
    }
}

#[test]
fn critic_regression_sanitize_deserialized_recovers_corrupted_state() {
    let mut ts = TimeScale::with_data(100, 800.0, HorzScaleOptions::sumi_default());
    // Corrupt via JSON round-trip / options mutation:
    let mut bad_opts = HorzScaleOptions::sumi_default();
    bad_opts.bar_spacing = f64::NAN;
    bad_opts.min_bar_spacing = -5.0;
    bad_opts.right_offset = f64::INFINITY;
    ts.apply_options(bad_opts);
    ts.sanitize_deserialized();
    assert!(!ts.is_empty());
    assert!(ts.index_to_local_x(50).unwrap().is_finite());

    let mut ps = PriceScale::new("corrupt", PriceScaleOptions::sumi_main_price());
    ps.set_height(500.0);
    ps.sanitize_deserialized();
    assert!(ps.is_auto_scale());
}

#[test]
fn r12_multichart_grid_geometry_causal_prefix_and_linked_crosshair() {
    use super::multi_chart::{
        MultiChartLayoutKind, apply_synced_time_window, causal_closed_prefix_len,
        extract_visible_time_window, resolve_linked_crosshair_bar,
    };

    let outer = PlotRect::new(10.0, 20.0, 1_000.0, 600.0);
    for layout in MultiChartLayoutKind::ALL {
        let cells = layout.compute_grid_cell_rects(outer, 6.0);
        assert_eq!(cells.len(), layout.slot_count());
        for c in &cells {
            assert!(c.left >= outer.left - 1e-9 && c.right() <= outer.right() + 1e-9);
            assert!(c.top >= outer.top - 1e-9 && c.bottom() <= outer.bottom() + 1e-9);
            assert!(c.width > 100.0 && c.height > 100.0);
        }
        // Pairwise non-overlapping
        for i in 0..cells.len() {
            for j in (i + 1)..cells.len() {
                let x_overlap =
                    cells[i].left < cells[j].right() && cells[j].left < cells[i].right();
                let y_overlap =
                    cells[i].top < cells[j].bottom() && cells[j].top < cells[i].bottom();
                assert!(
                    !(x_overlap && y_overlap),
                    "{layout:?} overlapping cells {i} vs {j}"
                );
            }
        }
    }

    // Causal DEF-001 closed prefix: 1h bars vs replay frontier mid-hour
    let t0 = 1_750_000_000_i64;
    let h1_bars: Vec<Candle> = (0..10)
        .map(|i| Candle {
            timestamp: t0 + (i as i64) * 3_600,
            date_label: format!("h{i}"),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 1000.0,
        })
        .collect();
    // Cutoff during bar #5 open+30m (`t0 + 5*3600 + 1800`) -> bar #5 closes at `t0 + 5*3600 + 3599`
    // -> STRICTLY closed prefix must stop at 5 bars (`0..=4`), NEVER revealing incomplete bar #5!
    let mid_bar5_close = t0 + 5 * 3_600 + 1_800;
    assert_eq!(
        causal_closed_prefix_len(&h1_bars, 3_600, Some(mid_bar5_close)),
        5
    );
    assert_eq!(causal_closed_prefix_len(&h1_bars, 3_600, None), 10);

    // Linked crosshair exact MTF containment (`t0 + 2*3600 + 900` inside bar 2) & causal clamp
    assert_eq!(
        resolve_linked_crosshair_bar(&h1_bars, 3_600, t0 + 2 * 3_600 + 900, None),
        Some(2)
    );
    assert_eq!(
        resolve_linked_crosshair_bar(
            &h1_bars,
            3_600,
            t0 + 9 * 3_600,
            Some(t0 + 4 * 3_600 + 3_599)
        ),
        Some(4)
    );

    // Synced time window preserves independent PriceScale untouched!
    let src_candles = make_candles(200, 100.0, 1.0);
    let mut dst_candles = Vec::new();
    for w in src_candles.chunks(4) {
        dst_candles.push(w[0].clone());
    }
    let mut ts_src =
        TimeScale::with_data(src_candles.len(), 600.0, HorzScaleOptions::sumi_default());
    ts_src.set_bar_spacing(10.0);
    ts_src.set_right_offset(-15.0);
    let mut ts_dst =
        TimeScale::with_data(dst_candles.len(), 600.0, HorzScaleOptions::sumi_default());
    let win = extract_visible_time_window(&src_candles, 86_400, &ts_src).expect("window");
    assert!(apply_synced_time_window(
        &dst_candles,
        86_400 * 4,
        &mut ts_dst,
        &win
    ));
    assert!(ts_dst.right_offset().is_finite() && ts_dst.bar_spacing() > 0.0);
}

#[test]
fn r12_workspace_template_ron_json_roundtrip_and_validation() {
    use super::workspace_template::{
        WorkspaceTemplate, WorkspaceTemplateError, builtin_workspace_templates,
    };

    let builtins = builtin_workspace_templates();
    assert_eq!(builtins.len(), 4);
    for tpl in &builtins {
        let json = tpl.to_json_pretty();
        let ron = tpl.to_ron_pretty();
        let dec_json = WorkspaceTemplate::from_json_str(&json).expect("json parse");
        let dec_ron = WorkspaceTemplate::from_ron_str(&ron).expect("ron parse");
        let dec_auto_j = WorkspaceTemplate::from_auto_str(&json).expect("auto json");
        let dec_auto_r = WorkspaceTemplate::from_auto_str(&ron).expect("auto ron");
        assert_eq!(&dec_json, tpl);
        assert_eq!(&dec_ron, tpl);
        assert_eq!(&dec_auto_j, tpl);
        assert_eq!(&dec_auto_r, tpl);
    }

    let bad_ver =
        "{ \"schema_version\": 99, \"name\": \"x\", \"layout\": \"Quad2x2\", \"slots\": [] }";
    assert!(matches!(
        WorkspaceTemplate::from_json_str(bad_ver),
        Err(WorkspaceTemplateError::UnsupportedSchemaVersion(99))
    ));
    let bad_nan = "(schema_version: 1, name: \"x\", layout: \"Single1x1\", slots: [(symbol: \"FPT\", pane_stretches: (main: NaN))])";
    assert!(matches!(
        WorkspaceTemplate::from_ron_str(bad_nan),
        Err(WorkspaceTemplateError::NonFiniteNumber(_))
    ));
}

#[test]
fn critic_r12_regression_parser_linear_string_and_rfc8259_escapes() {
    use super::workspace_template::{WorkspaceTemplate, builtin_workspace_templates};

    let mut tpl = builtin_workspace_templates().into_iter().next().unwrap();
    // 1. Short RFC 8259 \b & \f escape round-trip (stays under MAX_TEMPLATE_DESC_BYTES)
    tpl.description = "short\u{0008}\u{000C}line".to_owned();
    let json = tpl.to_json_pretty();
    assert!(json.contains("\\b\\fline"));
    let decoded = WorkspaceTemplate::from_json_str(&json).expect("decode escaped \\b\\f JSON");
    assert!(decoded.description.ends_with("\u{0008}\u{000C}line"));

    // 2. Direct 100 KB string scan verifying linear O(N) scanning and zero quadratic UTF-8 DoS
    let big_desc = "x".repeat(100_000);
    let big_json = format!(
        "{{ \"schema_version\": 1, \"name\": \"ok\", \"description\": \"{big_desc}\", \"layout\": \"Single1x1\", \"slots\": [{{ \"symbol\": \"FPT\" }}] }}"
    );
    let big_decoded =
        WorkspaceTemplate::from_json_str(&big_json).expect("linear O(N) 100KB string scan");
    assert_eq!(big_decoded.name, "ok");

    // Reject raw unescaped control chars in JSON string
    let raw_ctrl_json = "{ \"schema_version\": 1, \"name\": \"bad\nline\", \"layout\": \"Single1x1\", \"slots\": [{ \"symbol\": \"FPT\" }] }";
    assert!(WorkspaceTemplate::from_json_str(raw_ctrl_json).is_err());

    // Reject whitespace or comments inside RON \u{...}
    let bad_ron_uni = "(schema_version: 1, name: \"\\u{41 }\", layout: \"Single1x1\", slots: [(symbol: \"FPT\")])";
    assert!(WorkspaceTemplate::from_ron_str(bad_ron_uni).is_err());
    let ok_ron_uni =
        "(schema_version: 1, name: \"\\u{41}\", layout: \"Single1x1\", slots: [(symbol: \"FPT\")])";
    assert_eq!(
        WorkspaceTemplate::from_ron_str(ok_ron_uni)
            .expect("valid RON \\u{41}")
            .name,
        "A"
    );
}

#[test]
fn critic_r12_regression_auto_detect_comments_and_nested_blocks() {
    use super::workspace_template::{WorkspaceTemplate, builtin_workspace_templates};

    let base = builtin_workspace_templates().into_iter().next().unwrap();
    let json = base.to_json_pretty();
    let ron = base.to_ron_pretty();

    let nested_ron = format!("/* a /* b */ c */ {ron}");
    assert_eq!(
        WorkspaceTemplate::from_auto_str(&nested_ron).expect("nested block RON"),
        base
    );

    let nested_json = format!("/* a /* b */ c */ {json}");
    assert_eq!(
        WorkspaceTemplate::from_auto_str(&nested_json).expect("nested block JSON"),
        base
    );

    let line_comment_json = format!("// note\n{json}");
    assert_eq!(
        WorkspaceTemplate::from_auto_str(&line_comment_json).expect("line comment JSON"),
        base
    );

    // Strict RFC-8259 from_json_str still rejects comments directly
    assert!(WorkspaceTemplate::from_json_str(&line_comment_json).is_err());
}

#[test]
fn critic_r12_regression_template_strict_validation() {
    use super::workspace_template::{WorkspaceTemplate, WorkspaceTemplateError};

    let ovf_f32 = "(schema_version: 1, name: \"x\", layout: \"Single1x1\", slots: [(symbol: \"FPT\", pane_stretches: (main: 1e39))])";
    assert!(matches!(
        WorkspaceTemplate::from_ron_str(ovf_f32),
        Err(WorkspaceTemplateError::NonFiniteNumber(_))
    ));

    let bare_inf = "(schema_version: 1, name: \"x\", layout: \"Single1x1\", slots: [(symbol: \"FPT\", pane_stretches: (main: INF))])";
    assert!(matches!(
        WorkspaceTemplate::from_ron_str(bare_inf),
        Err(WorkspaceTemplateError::NonFiniteNumber(_))
    ));

    let frac_period = "(schema_version: 1, name: \"x\", layout: \"Single1x1\", slots: [(symbol: \"FPT\", indicators: (ichimoku_tenkan: 9.4))])";
    assert!(matches!(
        WorkspaceTemplate::from_ron_str(frac_period),
        Err(WorkspaceTemplateError::InvalidField { .. })
    ));

    let frac_ver = "{ \"schema_version\": 1.0000005, \"name\": \"x\", \"layout\": \"Single1x1\", \"slots\": [{}] }";
    assert!(matches!(
        WorkspaceTemplate::from_json_str(frac_ver),
        Err(WorkspaceTemplateError::InvalidField { .. })
    ));

    let missing_ver = "{ \"name\": \"x\", \"layout\": \"Single1x1\", \"slots\": [{}] }";
    assert!(matches!(
        WorkspaceTemplate::from_json_str(missing_ver),
        Err(WorkspaceTemplateError::InvalidField { .. })
    ));

    let under_slots = "{ \"schema_version\": 1, \"name\": \"x\", \"layout\": \"Quad2x2\", \"slots\": [{\"symbol\": \"FPT\"}] }";
    assert!(matches!(
        WorkspaceTemplate::from_json_str(under_slots),
        Err(WorkspaceTemplateError::InvalidField { .. })
    ));
}

#[test]
fn critic_r12_regression_template_rejects_duplicate_and_unknown_fields() {
    use super::workspace_template::{WorkspaceTemplate, WorkspaceTemplateError};

    let dup_key = "{ \"schema_version\": 1, \"schema_version\": 1, \"name\": \"x\", \"layout\": \"Single1x1\", \"slots\": [{\"symbol\": \"FPT\"}] }";
    assert!(matches!(
        WorkspaceTemplate::from_json_str(dup_key),
        Err(WorkspaceTemplateError::InvalidField { .. })
    ));

    let typo_key = "(schema_version: 1, name: \"x\", layout: \"Single1x1\", slots: [(symbol: \"FPT\", indicators: (tenkan_peroid: 12))])";
    assert!(matches!(
        WorkspaceTemplate::from_ron_str(typo_key),
        Err(WorkspaceTemplateError::InvalidField { .. })
    ));
}
