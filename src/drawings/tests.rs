use crate::data::generate_synthetic_ohlcv;
use crate::indicators::OscillatorPaneId;
use crate::model::Candle;
use crate::viewport::{HorzScaleOptions, PriceScale, PriceScaleMode, PriceScaleOptions, TimeScale};

use super::*;

fn setup_pane_scales(
    candle_count: usize,
    width: f64,
    height: f64,
    price_min: f64,
    price_max: f64,
    mode: PriceScaleMode,
    inverted: bool,
) -> (TimeScale, PriceScale) {
    let mut ts = TimeScale::with_data(candle_count, width, HorzScaleOptions::default());
    ts.set_bar_spacing(10.0);
    ts.set_right_offset(5.0);

    let mut ps = PriceScale::new("right", PriceScaleOptions::default());
    ps.set_height(height);
    ps.set_mode(mode);
    ps.set_inverted(inverted);
    ps.set_auto_scale(false);
    ps.set_raw_price_range(price_min, price_max, 100.0);
    (ts, ps)
}

#[test]
fn liang_barsky_clips_finite_segments_and_360_degree_rays_accurately() {
    let bounds = LocalRect::from_pane_size(1000.0, 500.0);

    // 1. Finite segment crossing from outside top-left to inside bottom-right.
    let seg = clip_parametric_line(
        LocalPoint::new(-200.0, -100.0),
        LocalPoint::new(600.0, 300.0),
        0.0,
        1.0,
        bounds,
    )
    .expect("crossing segment must clip into bounds");
    assert!((seg.start.x - 0.0).abs() < 1e-9);
    assert!((seg.start.y - 0.0).abs() < 1e-9);
    assert!((seg.end.x - 600.0).abs() < 1e-9);
    assert!((seg.end.y - 300.0).abs() < 1e-9);

    // 2. Rightward ray (`dx > 0`) clips cleanly to `x = 1000.0`.
    let right_ray = clip_parametric_line(
        LocalPoint::new(200.0, 250.0),
        LocalPoint::new(300.0, 200.0),
        0.0,
        f64::INFINITY,
        bounds,
    )
    .expect("rightward ray must clip");
    assert!((right_ray.start.x - 200.0).abs() < 1e-9);
    assert!((right_ray.start.y - 250.0).abs() < 1e-9);
    assert!((right_ray.end.y - 0.0).abs() < 1e-9);
    assert!((right_ray.end.x - 700.0).abs() < 1e-9);

    // 3. Leftward ray (`dx < 0` — legacy Sumi failed here!) clips cleanly to `x = 0.0`.
    let left_ray = clip_parametric_line(
        LocalPoint::new(400.0, 250.0),
        LocalPoint::new(300.0, 250.0),
        0.0,
        f64::INFINITY,
        bounds,
    )
    .expect("leftward ray must clip to x = 0");
    assert!((left_ray.start.x - 400.0).abs() < 1e-9);
    assert!((left_ray.end.x - 0.0).abs() < 1e-9);
    assert!((left_ray.end.y - 250.0).abs() < 1e-9);

    // 4. Pure vertical upward and downward rays (`dx == 0`).
    let up_ray = clip_parametric_line(
        LocalPoint::new(500.0, 300.0),
        LocalPoint::new(500.0, 200.0),
        0.0,
        f64::INFINITY,
        bounds,
    )
    .expect("upward vertical ray must clip to top = 0");
    assert!((up_ray.start.y - 300.0).abs() < 1e-9);
    assert!((up_ray.end.y - 0.0).abs() < 1e-9);
    assert!((up_ray.end.x - 500.0).abs() < 1e-9);

    // 5. Ray pointing away from the viewport from outside returns `None`.
    let away_ray = clip_parametric_line(
        LocalPoint::new(-50.0, 200.0),
        LocalPoint::new(-100.0, 200.0),
        0.0,
        f64::INFINITY,
        bounds,
    );
    assert!(away_ray.is_none());

    // 6. Extreme 100K-bar zoom coordinates (`1e7 px` offscreen) clip without overflow or NaN.
    let extreme = clip_parametric_line(
        LocalPoint::new(-5_000_000.0, -2_500_000.0),
        LocalPoint::new(5_000_000.0, 2_500_000.0),
        0.0,
        1.0,
        bounds,
    )
    .expect("extreme offscreen segment crossing origin must clip");
    assert!(extreme.start.is_finite() && extreme.end.is_finite());
    assert!((extreme.start.x - 0.0).abs() < 1e-6);
    assert!((extreme.start.y - 0.0).abs() < 1e-6);
}

#[test]
fn fibonacci_levels_match_canonical_archive_ratios_and_direction_reversal() {
    // Mirrors `ARCHIVE/frontend/src/features/drawings/__tests__/drawingGeometry.test.ts:106-127`.
    let start = DrawingAnchor::new(10.0, 100.0);
    let end = DrawingAnchor::new(30.0, 110.0);

    let v_start_to_end_618 = fibonacci_level_value(start, end, 0.618, FibDirection::StartToEnd);
    let v_end_to_start_618 = fibonacci_level_value(start, end, 0.618, FibDirection::EndToStart);
    assert!((v_start_to_end_618 - 106.18).abs() < 1e-9);
    assert!((v_end_to_start_618 - 103.82).abs() < 1e-9);

    let v_end_to_start_146 = fibonacci_level_value(start, end, 0.854, FibDirection::EndToStart);
    assert!((v_end_to_start_146 - 101.46).abs() < 1e-9);

    let (ts, ps) = setup_pane_scales(
        100,
        800.0,
        400.0,
        90.0,
        120.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 100.0);
    let fib = Drawing::new(
        1,
        DrawingPaneTarget::Main,
        DrawingGeometry::FibonacciRetracement {
            start: DrawingAnchor::new(60.0, 100.0),
            end: DrawingAnchor::new(85.0, 110.0),
            direction: FibDirection::StartToEnd,
            extend_right: true,
        },
        DrawingStyle::default_for_tool(DrawingTool::FibonacciRetracement),
    );
    let projected = proj.project_drawing(&fib).expect("fib projects");
    let ProjectedDrawingShape::Fibonacci { levels, bands, .. } = projected.shape else {
        panic!("expected Fibonacci shape");
    };
    assert_eq!(levels.len(), FIBONACCI_RATIOS.len());
    assert_eq!(bands.len(), FIBONACCI_RATIOS.len() - 1);
    assert!(levels.iter().all(|lvl| (lvl.x_right - 800.0).abs() < 1e-9));
}

#[test]
fn hit_test_precedence_prevents_fib_and_unselected_rect_hijack_and_supports_lock_selection() {
    let (ts, ps) = setup_pane_scales(
        100,
        800.0,
        400.0,
        90.0,
        120.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 100.0);

    let rect = Drawing::new(
        1,
        DrawingPaneTarget::Main,
        DrawingGeometry::Rectangle {
            corner_a: DrawingAnchor::new(40.0, 115.0),
            corner_b: DrawingAnchor::new(80.0, 95.0),
        },
        DrawingStyle::default_for_tool(DrawingTool::Rectangle),
    );
    let fib = Drawing::new(
        2,
        DrawingPaneTarget::Main,
        DrawingGeometry::FibonacciRetracement {
            start: DrawingAnchor::new(40.0, 95.0),
            end: DrawingAnchor::new(80.0, 115.0),
            direction: FibDirection::StartToEnd,
            extend_right: false,
        },
        DrawingStyle::default_for_tool(DrawingTool::FibonacciRetracement),
    );
    let mut locked_hline = Drawing::new(
        3,
        DrawingPaneTarget::Main,
        DrawingGeometry::HorizontalLine {
            anchor: DrawingAnchor::new(60.0, 105.0),
        },
        DrawingStyle::default_for_tool(DrawingTool::HorizontalLine),
    );
    locked_hline.locked = true;

    // 1. Empty interior between Fib 0.0 (95.0) and 0.236 (99.72) at bar 75 (away from diagonal) must NOT hit!
    let empty_fib_pt = proj
        .anchor_to_local(DrawingAnchor::new(75.0, 97.3))
        .unwrap();
    let fib_proj = proj.project_drawing(&fib).unwrap();
    assert!(
        hit_test_projected_drawing(&fib_proj, empty_fib_pt, true, 0).is_none(),
        "Fibonacci empty zone interior must not hijack chart pan"
    );

    // 2. Unselected Rectangle interior must NOT hit; once selected, interior hits with `SelectedBody`.
    let rect_interior = proj
        .anchor_to_local(DrawingAnchor::new(60.0, 108.0))
        .unwrap();
    let drawings = vec![rect.clone()];
    assert!(
        hit_test_pane_drawings(
            &drawings,
            DrawingPaneTarget::Main,
            &proj,
            rect_interior,
            None
        )
        .is_none()
    );
    let selected_hit = hit_test_pane_drawings(
        &drawings,
        DrawingPaneTarget::Main,
        &proj,
        rect_interior,
        Some(1),
    )
    .expect("selected rectangle interior should hit");
    assert_eq!(selected_hit.priority, HitPriority::SelectedBody);

    // 3. Handle on selected drawing outranks stroke of overlapping higher-Z drawing.
    let corner_pt = proj
        .anchor_to_local(DrawingAnchor::new(40.0, 115.0))
        .unwrap();
    let cross_trend = Drawing::new(
        4,
        DrawingPaneTarget::Main,
        DrawingGeometry::TrendLine {
            start: DrawingAnchor::new(35.0, 115.0),
            end: DrawingAnchor::new(85.0, 115.0),
        },
        DrawingStyle::default_for_tool(DrawingTool::TrendLine),
    );
    let overlap_drawings = vec![rect, cross_trend];
    let handle_win = hit_test_pane_drawings(
        &overlap_drawings,
        DrawingPaneTarget::Main,
        &proj,
        corner_pt,
        Some(1),
    )
    .expect("handle should win");
    assert_eq!(handle_win.drawing_id, 1);
    assert_eq!(handle_win.priority, HitPriority::SelectedHandle);

    // 4. Locked drawing can be selected via `Stroke` so the user can unlock it, without exposing handles.
    let hline_pt = proj
        .anchor_to_local(DrawingAnchor::new(60.0, 105.0))
        .unwrap();
    let locked_hit = hit_test_pane_drawings(
        &[locked_hline],
        DrawingPaneTarget::Main,
        &proj,
        hline_pt,
        Some(3),
    )
    .expect("locked horizontal line must still hit on stroke for selection");
    assert!(locked_hit.locked);
    assert_eq!(locked_hit.target, HitTarget::Body);
    assert_eq!(locked_hit.priority, HitPriority::Stroke);
}

#[test]
fn ohlc_magnet_snaps_within_threshold_and_breaks_ties_deterministically() {
    let candles = vec![
        Candle {
            timestamp: 1_750_000_000,
            date_label: "2026-06-15".to_owned(),
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 10.0,
        },
        Candle {
            timestamp: 1_750_086_400,
            date_label: "2026-06-16".to_owned(),
            open: 105.0,
            high: 112.0,
            low: 100.0,
            close: 100.0,
            volume: 10.0,
        },
    ];
    let (ts, ps) = setup_pane_scales(2, 400.0, 300.0, 90.0, 120.0, PriceScaleMode::Normal, false);
    let proj = PaneDrawingProjection::new(&ts, &ps, 400.0, 300.0, 100.0);

    // Cursor 4 px away from candle 0 high (`110.0`).
    let c0_high_pt = proj
        .anchor_to_local(DrawingAnchor::new(0.0, 110.0))
        .unwrap();
    let near_cursor = LocalPoint::new(c0_high_pt.x + 3.0, c0_high_pt.y - 2.0);
    let raw = proj.local_to_anchor(near_cursor).unwrap();

    let snapped = snap_to_visible_ohlc(raw, near_cursor, &candles, &proj, MagnetMode::Ohlc, 12.0);
    assert_eq!(snapped.bar_index, 0.0);
    assert!((snapped.value - 110.0).abs() < 1e-9);

    // Cursor 30 px away should NOT snap.
    let far_cursor = LocalPoint::new(c0_high_pt.x - 30.0, c0_high_pt.y - 30.0);
    let far_raw = proj.local_to_anchor(far_cursor).unwrap();
    let unsnapped =
        snap_to_visible_ohlc(far_raw, far_cursor, &candles, &proj, MagnetMode::Ohlc, 12.0);
    assert_eq!(unsnapped, far_raw);

    // Exact midpoint between candle 0 and candle 1 at `100.0` picks lower bar index (`0`) deterministically.
    let c0_open = proj
        .anchor_to_local(DrawingAnchor::new(0.0, 100.0))
        .unwrap();
    let c1_low = proj
        .anchor_to_local(DrawingAnchor::new(1.0, 100.0))
        .unwrap();
    let mid = LocalPoint::new((c0_open.x + c1_low.x) * 0.5, c0_open.y);
    let mid_raw = proj.local_to_anchor(mid).unwrap();
    let tie_winner = snap_to_visible_ohlc(mid_raw, mid, &candles, &proj, MagnetMode::Ohlc, 12.0);
    assert_eq!(tie_winner.bar_index, 0.0);
    assert!((tie_winner.value - 100.0).abs() < 1e-9);
}

#[test]
fn controller_supports_two_click_and_drag_creation_log_body_drag_and_bounded_undo_redo() {
    let candles = generate_synthetic_ohlcv(120);
    // Test in `Logarithmic` + `Inverted` mode to prove body translation preserves visual geometry!
    let (ts, ps) = setup_pane_scales(
        candles.len(),
        900.0,
        500.0,
        50.0,
        500.0,
        PriceScaleMode::Logarithmic,
        true,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 900.0, 500.0, 100.0);
    let mut store = DrawingStore::new();

    // 1. Two-click TrendLine placement (`click 1 -> move -> click 2`).
    store.set_tool(DrawingTool::TrendLine);
    let p_start = proj
        .anchor_to_local(DrawingAnchor::new(70.0, 120.0))
        .unwrap();
    let p_end = proj
        .anchor_to_local(DrawingAnchor::new(95.0, 260.0))
        .unwrap();
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p_start, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(
        !store.pointer_up(
            DrawingPaneTarget::Main,
            Some(p_start),
            false,
            &proj,
            &candles
        ),
        "first click without drag enters waiting-for-second-click phase"
    );
    store.pointer_move(DrawingPaneTarget::Main, p_end, false, &proj, &candles);
    assert!(store.preview_drawing().is_some());
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p_end, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(store.pointer_up(DrawingPaneTarget::Main, Some(p_end), false, &proj, &candles));
    assert_eq!(store.drawings.len(), 1);
    assert_eq!(store.active_tool, DrawingTool::Select);

    // 2. Body drag in Logarithmic + Inverted mode preserves exact pixel vector `(dx, dy)`!
    let mid_pt = LocalPoint::new((p_start.x + p_end.x) * 0.5, (p_start.y + p_end.y) * 0.5);
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, mid_pt, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let dragged_mid = LocalPoint::new(mid_pt.x + 35.0, mid_pt.y - 42.0);
    store.pointer_move(DrawingPaneTarget::Main, dragged_mid, false, &proj, &candles);
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(dragged_mid),
        false,
        &proj,
        &candles
    ));

    let moved_proj = proj.project_drawing(&store.drawings[0]).unwrap();
    assert!((moved_proj.anchors_local[0].x - (p_start.x + 35.0)).abs() < 1e-6);
    assert!((moved_proj.anchors_local[0].y - (p_start.y - 42.0)).abs() < 1e-6);
    assert!((moved_proj.anchors_local[1].x - (p_end.x + 35.0)).abs() < 1e-6);
    assert!((moved_proj.anchors_local[1].y - (p_end.y - 42.0)).abs() < 1e-6);

    // 3. Escape during an active handle drag rolls back cleanly without polluting undo history.
    let moved_p0 = moved_proj.anchors_local[0];
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, moved_p0, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    store.pointer_move(
        DrawingPaneTarget::Main,
        LocalPoint::new(moved_p0.x - 80.0, moved_p0.y + 60.0),
        false,
        &proj,
        &candles,
    );
    assert!(store.cancel_or_deselect());
    let rolled_back = proj.project_drawing(&store.drawings[0]).unwrap();
    assert!((rolled_back.anchors_local[0].x - moved_p0.x).abs() < 1e-6);

    // 4. Undo/Redo reverses body move, then creation, and restores both cleanly.
    assert!(store.undo());
    let restored_orig = proj.project_drawing(&store.drawings[0]).unwrap();
    assert!((restored_orig.anchors_local[0].x - p_start.x).abs() < 1e-6);
    assert!(store.undo());
    assert!(store.drawings.is_empty());
    assert!(store.redo());
    assert_eq!(store.drawings.len(), 1);
}

#[test]
fn rectangle_four_corner_resize_and_oscillator_negative_values_and_sanitization_work() {
    let candles = generate_synthetic_ohlcv(80);
    // MACD oscillator scale with negative values (`-15.0..+15.0`).
    let (ts, ps) = setup_pane_scales(
        candles.len(),
        800.0,
        200.0,
        -15.0,
        15.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 200.0, 0.0);
    let macd_pane = DrawingPaneTarget::Oscillator(OscillatorPaneId::Macd);
    let mut store = DrawingStore::new();

    let id = store
        .add_drawing(
            macd_pane,
            DrawingGeometry::Rectangle {
                corner_a: DrawingAnchor::new(20.0, -8.5),
                corner_b: DrawingAnchor::new(50.0, 6.25),
            },
            None,
        )
        .expect("negative MACD rectangle must be accepted");
    store.selected_id = Some(id);

    // Grab corner 1 (`(corner_b.bar = 50.0, corner_a.val = -8.5)`) and drag to `(60.0, -12.0)`.
    let corner_1 = proj
        .anchor_to_local(DrawingAnchor::new(50.0, -8.5))
        .unwrap();
    let target_corner_1 = proj
        .anchor_to_local(DrawingAnchor::new(60.0, -12.0))
        .unwrap();
    assert_eq!(
        store.pointer_down(macd_pane, corner_1, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    store.pointer_move(macd_pane, target_corner_1, false, &proj, &candles);
    assert!(store.pointer_up(macd_pane, Some(target_corner_1), false, &proj, &candles));

    let DrawingGeometry::Rectangle { corner_a, corner_b } = store.drawings[0].geometry else {
        panic!("expected Rectangle");
    };
    assert!((corner_a.bar_index - 20.0).abs() < 1e-6);
    assert!((corner_a.value - (-12.0)).abs() < 1e-6);
    assert!((corner_b.bar_index - 60.0).abs() < 1e-6);
    assert!((corner_b.value - 6.25).abs() < 1e-6);

    // Locked drawing cannot be deleted until unlocked.
    assert!(store.toggle_selected_lock());
    assert!(!store.delete_selected());
    assert_eq!(store.drawings.len(), 1);

    //Bounded undo cap (`64` steps) + `serde_json` persistence sanitization.
    for i in 0..80 {
        let _ = store.add_drawing(
            macd_pane,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(10.0 + f64::from(i), -5.0 + f64::from(i) * 0.1),
            },
            None,
        );
    }
    let mut undo_count = 0;
    while store.undo() {
        undo_count += 1;
    }
    assert_eq!(undo_count, controller::MAX_UNDO_STEPS);

    // Inject a corrupted NaN drawing + duplicate ID and verify `sanitize_deserialized` drops them.
    let valid_id = store.drawings[0].id;
    let mut dup = store.drawings[0].clone();
    dup.id = valid_id;
    let mut nan_drawing = store.drawings[0].clone();
    nan_drawing.id = 9999;
    nan_drawing.geometry = DrawingGeometry::HorizontalLine {
        anchor: DrawingAnchor::new(f64::NAN, 10.0),
    };
    store.drawings.push(dup);
    store.drawings.push(nan_drawing);
    let mut restored = store.clone();
    restored.active_tool = DrawingTool::Ray;
    restored.sanitize_deserialized();
    assert!(!restored.drawings.iter().any(|d| d.id == 9999));
    let count_valid_id = restored
        .drawings
        .iter()
        .filter(|d| d.id == valid_id)
        .count();
    assert_eq!(count_valid_id, 1);
    assert_eq!(restored.active_tool, DrawingTool::Select);
}

#[test]
fn critic_r05_regression_click_without_drag_preserves_geometry_and_redo_stack() {
    let candles = generate_synthetic_ohlcv(120);
    let (ts, ps) = setup_pane_scales(
        candles.len(),
        900.0,
        420.0,
        900.0,
        1500.0,
        PriceScaleMode::Logarithmic,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 900.0, 420.0, 1100.0);
    let mut store = DrawingStore::new();
    store.magnet_mode = MagnetMode::Ohlc;

    let tl_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(72.37, 1134.567890123),
                end: DrawingAnchor::new(98.61, 1348.987654321),
            },
            None,
        )
        .unwrap();
    // Create and undo a second drawing so `redo_stack` is non-empty!
    let _ = store.add_drawing(
        DrawingPaneTarget::Main,
        DrawingGeometry::HorizontalLine {
            anchor: DrawingAnchor::new(80.0, 1250.0),
        },
        None,
    );
    assert!(store.undo());
    assert!(
        store.can_redo(),
        "precondition: redo_stack must be populated"
    );

    let original_geom = store.drawings[0].geometry.clone();
    let proj_tl = proj.project_drawing(&store.drawings[0]).unwrap();
    let handle_center = proj_tl.handles[0].pos;

    // 1. Click off-center (+5.5 px, -3.0 px) on Handle 0 WITH MagnetMode::Ohlc + sub-threshold jitter (1.2 px):
    let off_center_press = LocalPoint::new(handle_center.x + 5.5, handle_center.y - 3.0);
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            off_center_press,
            false,
            &proj,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let jitter_pos = LocalPoint::new(off_center_press.x + 1.0, off_center_press.y - 0.6);
    store.pointer_move(DrawingPaneTarget::Main, jitter_pos, false, &proj, &candles);
    assert!(!store.pointer_up(
        DrawingPaneTarget::Main,
        Some(jitter_pos),
        false,
        &proj,
        &candles
    ));
    assert_eq!(
        store.drawings[0].geometry, original_geom,
        "off-center handle click must preserve geometry bit-for-bit"
    );
    assert!(
        store.can_redo(),
        "selection click on handle must NOT wipe redo_stack"
    );

    // 2. Click on Body midpoint with sub-threshold jitter: must NOT push 1-ULP Modify or clear redo_stack!
    let mid_body = LocalPoint::new(
        (proj_tl.anchors_local[0].x + proj_tl.anchors_local[1].x) * 0.5,
        (proj_tl.anchors_local[0].y + proj_tl.anchors_local[1].y) * 0.5,
    );
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, mid_body, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    store.pointer_move(DrawingPaneTarget::Main, mid_body, false, &proj, &candles);
    assert!(!store.pointer_up(
        DrawingPaneTarget::Main,
        Some(mid_body),
        false,
        &proj,
        &candles
    ));
    assert_eq!(store.selected_id, Some(tl_id));
    assert_eq!(
        store.drawings[0].geometry, original_geom,
        "body click in Log mode must not introduce 1-4 ULP drift"
    );
    assert!(
        store.can_redo(),
        "selection click on body must NOT wipe redo_stack"
    );

    // 3. With Magnet OFF, dragging handle from an off-center grab preserves `grab_offset` smoothly!
    store.magnet_mode = MagnetMode::Off;
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            off_center_press,
            false,
            &proj,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let drag_target = LocalPoint::new(off_center_press.x + 20.0, off_center_press.y + 15.0);
    store.pointer_move(DrawingPaneTarget::Main, drag_target, false, &proj, &candles);
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(drag_target),
        false,
        &proj,
        &candles
    ));
    let after_proj = proj.project_drawing(&store.drawings[0]).unwrap();
    assert!(
        (after_proj.handles[0].pos.x - (handle_center.x + 20.0)).abs() < 1e-6
            && (after_proj.handles[0].pos.y - (handle_center.y + 15.0)).abs() < 1e-6,
        "grab_offset must translate handle center by exact pointer delta without 6px snap jump"
    );
}

#[test]
fn critic_r05_regression_liang_barsky_no_phantom_edge_ray_tiny_ray_and_extreme_span() {
    let bounds = LocalRect::from_pane_size(1000.0, 500.0);

    // 1. Off-screen ray with `0 < |dx| <= 1e-12` that misses the box by ~5,000 px on the right must return None!
    let p0_miss = LocalPoint::new(999.0, -1.0e5);
    let p1_miss = LocalPoint::new(999.0 + 5.0e-13, -1.0e5 + 1.0e-11);
    assert!(
        clip_parametric_line(p0_miss, p1_miss, 0.0, f64::INFINITY, bounds).is_none(),
        "near-vertical ray missing x=1000 far below top edge must NOT synthesize a phantom edge ray"
    );

    // 2. Valid ray inside box with sub-1e-12 projected step (`dx = 0`, `dy = -5e-13`) must still project to `y = 0.0`!
    let p0_tiny = LocalPoint::new(400.0, 250.0);
    let p1_tiny = LocalPoint::new(400.0, 250.0 - 5.0e-13);
    let tiny_ray = clip_parametric_line(p0_tiny, p1_tiny, 0.0, f64::INFINITY, bounds)
        .expect("tiny non-zero ray inside pane must not collapse");
    assert!((tiny_ray.start.x - 400.0).abs() < 1e-9);
    assert!((tiny_ray.start.y - 250.0).abs() < 1e-9);
    assert!((tiny_ray.end.x - 400.0).abs() < 1e-9);
    assert!(
        (tiny_ray.end.y - 0.0).abs() < 1e-9,
        "upward ray must extend cleanly to top border y=0, got {}",
        tiny_ray.end.y
    );

    // 3. Extreme finite segment whose naive `p1.y - p0.y` overflows `f64::MAX` (`-Inf`) must NOT produce `0 * Inf = NaN`!
    let extreme_seg = clip_parametric_line(
        LocalPoint::new(500.0, 1.39e308),
        LocalPoint::new(500.0, -1.39e308),
        0.0,
        1.0,
        bounds,
    )
    .expect("vertical segment spanning ±1.39e308 across viewport must clip cleanly");
    assert!((extreme_seg.start.x - 500.0).abs() < 1e-9);
    assert!((extreme_seg.end.x - 500.0).abs() < 1e-9);
    let min_y = extreme_seg.start.y.min(extreme_seg.end.y);
    let max_y = extreme_seg.start.y.max(extreme_seg.end.y);
    assert!((min_y - 0.0).abs() < 1e-6 && (max_y - 500.0).abs() < 1e-6);
}

#[test]
fn critic_r05_regression_locked_rectangle_deselect_pan_passthrough_and_transitive_ties() {
    let candles = generate_synthetic_ohlcv(100);
    let (ts, ps) = setup_pane_scales(
        100,
        800.0,
        400.0,
        90.0,
        130.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 100.0);
    let mut store = DrawingStore::new();

    let rect_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Rectangle {
                corner_a: DrawingAnchor::new(30.0, 120.0),
                corner_b: DrawingAnchor::new(80.0, 98.0),
            },
            None,
        )
        .unwrap();
    store.selected_id = Some(rect_id);
    assert!(store.toggle_selected_lock());
    assert!(store.drawings[0].locked);

    // 1. Interior of a selected LOCKED rectangle must return `None` so clicks inside deselect cleanly
    //    and drag gestures inside pan the chart without interference!
    let interior_pt = proj
        .anchor_to_local(DrawingAnchor::new(55.0, 109.0))
        .unwrap();
    assert!(
        hit_test_pane_drawings(
            &store.drawings,
            DrawingPaneTarget::Main,
            &proj,
            interior_pt,
            store.selected_id,
        )
        .is_none(),
        "locked selected rectangle must NOT capture SelectedBody in its interior"
    );

    // 2. Clicking on the locked rectangle's border stroke selects it AND returns `PassThroughToViewport`!
    store.selected_id = None;
    let border_pt = proj
        .anchor_to_local(DrawingAnchor::new(55.0, 120.0))
        .unwrap();
    let outcome = store.pointer_down(DrawingPaneTarget::Main, border_pt, false, &proj, &candles);
    assert_eq!(outcome, PointerCaptureOutcome::PassThroughToViewport);
    assert_eq!(store.selected_id, Some(rect_id));
    assert_eq!(*store.phase(), DrawingInteractionPhase::Idle);

    // 3. Transitive near-tie chain across 3 lines at distances `1.7 px` (Z=0), `2.1 px` (Z=1), `2.4 px` (Z=2):
    //    True `min_dist = 1.7 px`; `Z=1` (`2.1 <= 2.2`) wins, while `Z=2` (`2.4 > 2.2`) must NOT chain-steal!
    let base_y = 200.0;
    let v_at_dy = |dy: f64| ps.local_y_to_price(base_y + dy, 100.0).unwrap();
    let mut chain_store = DrawingStore::new();
    let id_z0 = chain_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(50.0, v_at_dy(1.7)),
            },
            None,
        )
        .unwrap();
    let id_z1 = chain_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(50.0, v_at_dy(2.1)),
            },
            None,
        )
        .unwrap();
    let _id_z2 = chain_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(50.0, v_at_dy(2.4)),
            },
            None,
        )
        .unwrap();
    let hit = hit_test_pane_drawings(
        &chain_store.drawings,
        DrawingPaneTarget::Main,
        &proj,
        LocalPoint::new(400.0, base_y),
        None,
    )
    .unwrap();
    assert_ne!(id_z0, id_z1);
    assert_eq!(
        hit.drawing_id, id_z1,
        "highest Z within min_dist + 0.5px must win without non-transitive chain drift to Z=2"
    );
}

#[test]
fn critic_r05_regression_u64_max_id_compaction_and_cross_pane_drag_hover_isolation() {
    let candles = generate_synthetic_ohlcv(100);
    let (ts, ps_main) = setup_pane_scales(
        100,
        800.0,
        360.0,
        900.0,
        1300.0,
        PriceScaleMode::Normal,
        false,
    );
    let (_, ps_rsi) =
        setup_pane_scales(100, 800.0, 120.0, 0.0, 100.0, PriceScaleMode::Normal, false);
    let main_proj = PaneDrawingProjection::new(&ts, &ps_main, 800.0, 360.0, 1000.0);
    let rsi_proj = PaneDrawingProjection::new(&ts, &ps_rsi, 800.0, 120.0, 1.0);

    // 1. Corrupt `u64::MAX` drawing + near-`u64::MAX` drawing deserialization test:
    let mut store = DrawingStore::new();
    let mut bad_max = Drawing::new(
        u64::MAX,
        DrawingPaneTarget::Main,
        DrawingGeometry::HorizontalLine {
            anchor: DrawingAnchor::new(40.0, 1100.0),
        },
        DrawingStyle::default_for_tool(DrawingTool::HorizontalLine),
    );
    assert!(
        !bad_max.is_valid(),
        "u64::MAX is reserved for preview_drawing"
    );
    bad_max.id = u64::MAX - 2;
    assert!(bad_max.is_valid());
    store.drawings.push(bad_max);
    store.selected_id = Some(u64::MAX - 2);
    store.sanitize_deserialized();

    assert_eq!(store.drawings.len(), 1);
    assert_eq!(
        store.drawings[0].id, 1,
        "near-u64::MAX IDs must compact cleanly to 1..=len"
    );
    assert_eq!(store.selected_id, Some(1));
    let next_created = store
        .add_drawing(
            DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(50.0, 50.0),
            },
            None,
        )
        .unwrap();
    assert_eq!(next_created, 2);

    // 2. Cross-pane hover isolation while dragging a `Main` drawing across into `Rsi`:
    let main_pt = main_proj
        .anchor_to_local(DrawingAnchor::new(50.0, 1100.0))
        .unwrap();
    let rsi_pt = rsi_proj
        .anchor_to_local(DrawingAnchor::new(50.0, 50.0))
        .unwrap();
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            main_pt,
            false,
            &main_proj,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    store.pointer_move(
        DrawingPaneTarget::Main,
        LocalPoint::new(main_pt.x, main_pt.y + 10.0),
        false,
        &main_proj,
        &candles,
    );
    // Simulate pointer overshooting into RSI sub-pane mid-drag:
    store.pointer_move(
        DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
        rsi_pt,
        false,
        &rsi_proj,
        &candles,
    );
    assert!(
        store.hovered_hit().is_none(),
        "dragging a Main drawing across RSI must never leak hovered_hit onto RSI drawings"
    );
}
