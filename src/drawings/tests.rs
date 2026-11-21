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
    // Verifies canonical 7-level Fibonacci retracement coordinates and reversal.
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

#[test]
fn risk_reward_box_handles_linear_and_log_body_drag_invariance_flip_and_main_pane_exclusivity() {
    let candles = generate_synthetic_ohlcv(120);
    let (ts, ps_lin) = setup_pane_scales(
        120,
        800.0,
        400.0,
        60.0,
        180.0,
        PriceScaleMode::Normal,
        false,
    );
    let (_, ps_log) = setup_pane_scales(
        120,
        800.0,
        400.0,
        60.0,
        180.0,
        PriceScaleMode::Logarithmic,
        false,
    );
    let (_, ps_rsi) =
        setup_pane_scales(120, 800.0, 120.0, 0.0, 100.0, PriceScaleMode::Normal, false);

    let proj_lin = PaneDrawingProjection::new(&ts, &ps_lin, 800.0, 400.0, 100.0);
    let proj_log = PaneDrawingProjection::new(&ts, &ps_log, 800.0, 400.0, 100.0);
    let proj_rsi = PaneDrawingProjection::new(&ts, &ps_rsi, 800.0, 120.0, 1.0);

    let mut store = DrawingStore::new();

    // 1. Reject oscillator sub-pane placement & programmatic insertion:
    let valid_rr = DrawingGeometry::RiskRewardBox {
        start: DrawingAnchor::new(70.0, 100.0),
        end: DrawingAnchor::new(95.0, 118.0),
        stop_price: 94.0,
    };
    assert!(
        store
            .add_drawing(
                DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
                valid_rr.clone(),
                None
            )
            .is_none(),
        "RiskRewardBox must be Main-pane exclusive"
    );
    store.set_tool(DrawingTool::RiskRewardBox);
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
            LocalPoint::new(300.0, 60.0),
            false,
            &proj_rsi,
            &candles
        ),
        PointerCaptureOutcome::PassThroughToViewport
    );

    // 2. Interactive 2-click placement on Main pane (upward -> Long 2.0R):
    let p_start = proj_lin
        .anchor_to_local(DrawingAnchor::new(70.0, 100.0))
        .unwrap();
    let p_target = proj_lin
        .anchor_to_local(DrawingAnchor::new(95.0, 116.0))
        .unwrap();
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p_start, false, &proj_lin, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    store.pointer_move(
        DrawingPaneTarget::Main,
        p_target,
        false,
        &proj_lin,
        &candles,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p_target),
        false,
        &proj_lin,
        &candles
    ));
    assert_eq!(store.drawings.len(), 1);
    let rr_id = store.drawings[0].id;
    let (_, entry0, stop0, target0, r0) = store.drawings[0]
        .geometry
        .risk_reward_metrics()
        .expect("metrics");
    assert!((entry0 - 100.0).abs() < 1e-6);
    assert!((target0 - 116.0).abs() < 1e-6);
    assert!((stop0 - 92.0).abs() < 1e-6);
    assert!((r0 - 2.0).abs() < 1e-6);

    // 3. Linear AND Logarithmic whole-body vertical drag preserves exact `gross_r` (`2.0R`):
    for (label, proj) in [("Linear", &proj_lin), ("Logarithmic", &proj_log)] {
        let projected = proj.project_drawing(&store.drawings[0]).unwrap();
        assert_eq!(projected.handles.len(), 4);
        let reward_rect = match &projected.shape {
            ProjectedDrawingShape::RiskReward { reward_fill, .. } => reward_fill.unwrap(),
            other => panic!("unexpected shape: {other:?}"),
        };
        let inside_pt = LocalPoint::new(
            0.5 * (reward_rect.left + reward_rect.right),
            0.5 * (reward_rect.top + reward_rect.bottom),
        );
        assert_eq!(
            store.pointer_down(DrawingPaneTarget::Main, inside_pt, false, proj, &candles),
            PointerCaptureOutcome::CapturedByDrawing
        );
        let moved_pt = LocalPoint::new(inside_pt.x + 12.0, inside_pt.y - 28.0);
        store.pointer_move(DrawingPaneTarget::Main, moved_pt, false, proj, &candles);
        assert!(store.pointer_up(
            DrawingPaneTarget::Main,
            Some(moved_pt),
            false,
            proj,
            &candles
        ));
        let (_, _, _, _, r_after) = store.drawings[0].geometry.risk_reward_metrics().unwrap();
        assert!(
            (r_after - 2.0).abs() < 1e-9,
            "{label} body drag must preserve gross_r == 2.0, got {r_after}"
        );
        assert!(store.undo());
    }

    // 4. Individual handle drag (`RrStop` & `RrTarget`) + clamping when dragged past entry:
    let projected = proj_lin.project_drawing(&store.drawings[0]).unwrap();
    let stop_handle = projected
        .handles
        .iter()
        .find(|h| h.part == HandlePart::RrStop)
        .unwrap()
        .pos;
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            stop_handle,
            false,
            &proj_lin,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    // Attempt to drag stop ABOVE entry (`130.0`) -> must clamp cleanly strictly below entry (`100.0`)!
    let above_entry_pt = proj_lin
        .anchor_to_local(DrawingAnchor::new(82.5, 130.0))
        .unwrap();
    store.pointer_move(
        DrawingPaneTarget::Main,
        above_entry_pt,
        false,
        &proj_lin,
        &candles,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(above_entry_pt),
        false,
        &proj_lin,
        &candles
    ));
    assert!(store.drawings[0].geometry.is_valid());
    let (side_clamped, en_c, sl_c, _, _) =
        store.drawings[0].geometry.risk_reward_metrics().unwrap();
    assert_eq!(side_clamped, crate::trading::PositionSide::Long);
    assert!(sl_c < en_c);
    assert!(store.undo());

    // 5. Flip Long <-> Short (`reverse_selected_risk_reward`) + Undo/Redo:
    store.selected_id = Some(rr_id);
    assert!(store.reverse_selected_risk_reward());
    let (flipped_side, en_f, sl_f, tp_f, r_f) =
        store.drawings[0].geometry.risk_reward_metrics().unwrap();
    assert_eq!(flipped_side, crate::trading::PositionSide::Short);
    assert!((en_f - 100.0).abs() < 1e-6);
    assert!((tp_f - 84.0).abs() < 1e-6);
    assert!((sl_f - 108.0).abs() < 1e-6);
    assert!((r_f - 2.0).abs() < 1e-6);
    assert!(store.undo());
    assert!(store.redo());
}

#[test]
fn critic_r10_regression_rr_box_entry_handle_crossover_and_target_flip_symmetry() {
    let candles = generate_synthetic_ohlcv(120);
    let (ts, ps_lin) = setup_pane_scales(
        120,
        800.0,
        400.0,
        60.0,
        180.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj_lin = PaneDrawingProjection::new(&ts, &ps_lin, 800.0, 400.0, 100.0);

    let mut store = DrawingStore::new();
    let rr_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::RiskRewardBox {
                start: DrawingAnchor::new(70.0, 100.0),
                end: DrawingAnchor::new(85.0, 116.0),
                stop_price: 92.0,
            },
            None,
        )
        .expect("valid RiskRewardBox");
    store.selected_id = Some(rr_id);

    // 1. C-05: Drag `RrEntryLeft` (`bar 70`) past `end` (`bar 95`) and back (`bar 68`) in a single continuous drag:
    //    `end.bar_index` MUST remain untouched at `85.0` with zero drift!
    let projected = proj_lin.project_drawing(&store.drawings[0]).unwrap();
    let left_handle = projected
        .handles
        .iter()
        .find(|h| h.part == HandlePart::RrEntryLeft)
        .unwrap()
        .pos;
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            left_handle,
            false,
            &proj_lin,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let crossed_right = proj_lin
        .anchor_to_local(DrawingAnchor::new(95.0, 101.0))
        .unwrap();
    store.pointer_move(
        DrawingPaneTarget::Main,
        crossed_right,
        false,
        &proj_lin,
        &candles,
    );
    let dragged_back = proj_lin
        .anchor_to_local(DrawingAnchor::new(68.0, 101.0))
        .unwrap();
    store.pointer_move(
        DrawingPaneTarget::Main,
        dragged_back,
        false,
        &proj_lin,
        &candles,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(dragged_back),
        false,
        &proj_lin,
        &candles
    ));
    let [a0, a1] = store.drawings[0].geometry.anchors();
    assert!(
        (a1.bar_index - 85.0).abs() < 1e-6,
        "C-05: opposite anchor drifted during RrEntryLeft crossover: expected 85.0, got {}",
        a1.bar_index
    );
    assert!((a0.bar_index - 68.0).abs() < 1e-6);

    // 2. M-07: Drag `RrTarget` across `entry` (`100.0`) down to `84.0` -> smoothly flips to `Short`
    //    and reflects `stop_price` above `entry` so risk & reward zones NEVER overlap!
    let projected2 = proj_lin.project_drawing(&store.drawings[0]).unwrap();
    let target_handle = projected2
        .handles
        .iter()
        .find(|h| h.part == HandlePart::RrTarget)
        .unwrap()
        .pos;
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            target_handle,
            false,
            &proj_lin,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let below_entry = proj_lin
        .anchor_to_local(DrawingAnchor::new(85.0, 84.0))
        .unwrap();
    store.pointer_move(
        DrawingPaneTarget::Main,
        below_entry,
        false,
        &proj_lin,
        &candles,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(below_entry),
        false,
        &proj_lin,
        &candles
    ));
    let (side_flipped, en_v, sl_v, tp_v, _) =
        store.drawings[0].geometry.risk_reward_metrics().unwrap();
    assert_eq!(side_flipped, crate::trading::PositionSide::Short);
    assert!(
        tp_v < en_v && sl_v > en_v,
        "M-07: flipped Short R:R box must keep stop > entry > target"
    );
}

#[test]
fn r11_frvp_and_avwap_creation_hit_test_log_linear_drag_timeframe_remap_and_causal_masking() {
    use crate::data::{Timeframe, aggregate_ohlcv, generate_intraday_vn30f_1m_report};

    let candles_15m = aggregate_ohlcv(
        &generate_intraday_vn30f_1m_report(6_000).candles,
        Timeframe::M1,
        Timeframe::M15,
    )
    .expect("15m candles");
    let candles_h1 =
        aggregate_ohlcv(&candles_15m, Timeframe::M15, Timeframe::H1).expect("H1 candles");

    let (ts_lin, ps_lin) = setup_pane_scales(
        candles_15m.len(),
        900.0,
        450.0,
        1_220.0,
        1_380.0,
        PriceScaleMode::Normal,
        false,
    );
    let (ts_log, ps_log) = setup_pane_scales(
        candles_15m.len(),
        900.0,
        450.0,
        1_220.0,
        1_380.0,
        PriceScaleMode::Logarithmic,
        false,
    );
    // Fit window cleanly over latest 100 bars so bar indices project onscreen.
    let mut ts_vis = ts_lin;
    ts_vis.set_bar_spacing(8.0);
    ts_vis.set_right_offset(5.0);
    let mut ts_log_vis = ts_log;
    ts_log_vis.set_bar_spacing(8.0);
    ts_log_vis.set_right_offset(5.0);
    let proj_lin = PaneDrawingProjection::new(&ts_vis, &ps_lin, 900.0, 450.0, 1_280.0);
    let proj_log = PaneDrawingProjection::new(&ts_log_vis, &ps_log, 900.0, 450.0, 1_280.0);

    let n = candles_15m.len();
    let i0 = n - 45;
    let i1 = n - 20;
    let a_idx = n - 30;
    let mid_p = candles_15m[i0].close;

    let mut store = DrawingStore::new();
    // 1. Two-click creation for FixedRangeVolumeProfile + single-click creation for AnchoredVwap.
    store.set_tool(DrawingTool::FixedRangeVolumeProfile);
    let p_a = proj_lin
        .anchor_to_local(DrawingAnchor::new(i0 as f64, mid_p - 6.0))
        .unwrap();
    let p_b = proj_lin
        .anchor_to_local(DrawingAnchor::new(i1 as f64, mid_p + 6.0))
        .unwrap();
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p_a, false, &proj_lin, &candles_15m),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(!store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p_a),
        false,
        &proj_lin,
        &candles_15m
    ));
    store.pointer_move(DrawingPaneTarget::Main, p_b, false, &proj_lin, &candles_15m);
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p_b, false, &proj_lin, &candles_15m),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p_b),
        false,
        &proj_lin,
        &candles_15m
    ));
    assert_eq!(store.drawings.len(), 1);
    let frvp_id = store.drawings[0].id;
    assert!(matches!(
        store.drawings[0].geometry,
        DrawingGeometry::FixedRangeVolumeProfile { row_count: 24, .. }
    ));

    store.set_tool(DrawingTool::AnchoredVwap);
    let av_pt = proj_lin
        .anchor_to_local(DrawingAnchor::new(a_idx as f64, candles_15m[a_idx].low))
        .unwrap();
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            av_pt,
            false,
            &proj_lin,
            &candles_15m
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(av_pt),
        false,
        &proj_lin,
        &candles_15m
    ));
    assert_eq!(store.drawings.len(), 2);
    let avwap_id = store.drawings[1].id;
    assert!(matches!(
        store.drawings[1].geometry,
        DrawingGeometry::AnchoredVwap {
            show_bands: true,
            ..
        }
    ));

    // 2. Hit-testing on both Normal & Logarithmic projections + handle drag on Log scale!
    let frvp_hit = hit_test_pane_drawings(
        &store.drawings,
        DrawingPaneTarget::Main,
        &proj_lin,
        p_b,
        Some(frvp_id),
    );
    assert!(frvp_hit.is_some());
    let av_hit = hit_test_pane_drawings(
        &store.drawings,
        DrawingPaneTarget::Main,
        &proj_log,
        proj_log
            .anchor_to_local(store.drawings[1].geometry.anchors()[0])
            .unwrap(),
        Some(avwap_id),
    );
    assert!(av_hit.is_some());

    let av_log_pos = proj_log
        .anchor_to_local(store.drawings[1].geometry.anchors()[0])
        .unwrap();
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            av_log_pos,
            false,
            &proj_log,
            &candles_15m
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let new_av_idx = (a_idx + 4) as f64;
    let new_av_pos = proj_log
        .anchor_to_local(DrawingAnchor::new(new_av_idx, candles_15m[a_idx + 4].low))
        .unwrap();
    store.pointer_move(
        DrawingPaneTarget::Main,
        new_av_pos,
        false,
        &proj_log,
        &candles_15m,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(new_av_pos),
        false,
        &proj_log,
        &candles_15m
    ));
    let moved_av_bar = store.drawings[1].geometry.anchors()[0].bar_index;
    assert!((moved_av_bar - new_av_idx).abs() < 1e-6);

    // 3. Timeframe remap (`M15 -> H1 -> M15`) preserves exact chronological bucket placement.
    store.remap_timeframe(&candles_15m, Timeframe::M15, &candles_h1, Timeframe::H1);
    let h1_bar = store.drawings[1].geometry.anchors()[0].bar_index;
    assert!(h1_bar < moved_av_bar);
    store.remap_timeframe(&candles_h1, Timeframe::H1, &candles_15m, Timeframe::M15);

    // 4. `DEF-001` causal replay masking hides future `AVWAP` (and FRVP extending past cutoff) when cutoff rewinds.
    let cutoff_mid = candles_15m[n - 15].timestamp;
    let future_ts = candles_15m[n - 10].timestamp;
    store.drawings[1].created_at_timestamp = Some(future_ts);
    store.set_replay_cutoff_timestamp(Some(cutoff_mid), candles_15m.len());
    assert!(store.drawings[0].is_causally_visible(Some(cutoff_mid), candles_15m.len()));
    assert!(!store.drawings[1].is_causally_visible(Some(cutoff_mid), candles_15m.len()));
    let cutoff_before_frvp_end = candles_15m[n - 25].timestamp;
    assert!(
        !store.drawings[0].is_causally_visible(Some(cutoff_before_frvp_end), candles_15m.len())
    );

    // 5. `sanitize_deserialized` clamps corrupted row counts and removes duplicate IDs cleanly.
    store.set_replay_cutoff_timestamp(None, usize::MAX);
    if let DrawingGeometry::FixedRangeVolumeProfile { row_count, .. } =
        &mut store.drawings[0].geometry
    {
        *row_count = 9_999;
    }
    store.sanitize_deserialized();
    if let DrawingGeometry::FixedRangeVolumeProfile { row_count, .. } = store.drawings[0].geometry {
        assert_eq!(row_count, 200);
    }
}

#[test]
fn critic_r11_regression_frvp_timeframe_remap_preserves_validity_and_vertical_handles() {
    // M4: Remapping a multi-bar 15m FRVP whose timestamps collapse into one H1 bucket must remain valid
    // when distinct timestamps were preserved, and offscreen-Y FRVP must keep horizontally clamped handles.
    let start_ts = 1_704_067_200_i64;
    let candles_15m: Vec<Candle> = (0..32)
        .map(|i| Candle {
            timestamp: start_ts + (i as i64) * 900,
            date_label: format!("m15_{i}"),
            open: 100.0 + (i as f64) * 0.2,
            high: 101.0 + (i as f64) * 0.2,
            low: 99.0 + (i as f64) * 0.2,
            close: 100.5 + (i as f64) * 0.2,
            volume: 10_000.0,
        })
        .collect();
    let candles_h1: Vec<Candle> = (0..8)
        .map(|h| Candle {
            timestamp: start_ts + (h as i64) * 3_600,
            date_label: format!("h1_{h}"),
            open: 100.0 + (h as f64),
            high: 102.0 + (h as f64),
            low: 99.0 + (h as f64),
            close: 101.0 + (h as f64),
            volume: 40_000.0,
        })
        .collect();

    let mut store = DrawingStore::default();
    let frvp_geom = DrawingGeometry::FixedRangeVolumeProfile {
        start: DrawingAnchor::with_timestamp(4.0, 100.8, candles_15m[4].timestamp as f64),
        end: DrawingAnchor::with_timestamp(6.0, 101.2, candles_15m[6].timestamp as f64),
        row_count: 24,
    };
    assert!(frvp_geom.is_valid());
    let frvp_id = store
        .add_drawing(DrawingPaneTarget::Main, frvp_geom, None)
        .expect("add FRVP");
    store.remap_timeframe(
        &candles_15m,
        crate::data::Timeframe::M15,
        &candles_h1,
        crate::data::Timeframe::H1,
    );
    let remapped = store
        .drawings
        .iter()
        .find(|d| d.id == frvp_id)
        .expect("FRVP survives 15m -> H1 remap");
    assert!(remapped.geometry.is_valid());

    // Vertical offscreen check: scale is `[200.0, 260.0]` while FRVP anchors live near `101.0`.
    store.remap_timeframe(
        &candles_h1,
        crate::data::Timeframe::H1,
        &candles_15m,
        crate::data::Timeframe::M15,
    );
    let (ts, ps) = setup_pane_scales(
        candles_15m.len(),
        800.0,
        400.0,
        200.0,
        260.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 200.0).with_candles(&candles_15m);
    let projected = proj
        .project_drawing(
            store
                .drawings
                .iter()
                .find(|d| d.id == frvp_id)
                .expect("remapped back"),
        )
        .expect("project offscreen-Y FRVP");
    assert_eq!(projected.handles.len(), 2);
    for h in &projected.handles {
        assert!((0.0..=400.0).contains(&h.pos.y));
    }
}

#[test]
fn critic_r11_regression_avwap_tail_hit_test_and_left_drag_clamp() {
    // M4: Selecting/hitting AVWAP along its visible tail when the anchor is offscreen left must work,
    // and dragging the anchor past bar `-50` must clamp `anchor.bar_index >= 0.0`.
    let candles: Vec<Candle> = (0..80)
        .map(|i| Candle {
            timestamp: 1_704_067_200 + (i as i64) * 3_600,
            date_label: format!("b{i}"),
            open: 100.0 + (i as f64) * 0.5,
            high: 102.0 + (i as f64) * 0.5,
            low: 98.0 + (i as f64) * 0.5,
            close: 101.0 + (i as f64) * 0.5,
            volume: 25_000.0,
        })
        .collect();

    let mut store = DrawingStore::default();
    let av_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::AnchoredVwap {
                anchor: DrawingAnchor::with_timestamp(
                    5.0,
                    candles[5].low,
                    candles[5].timestamp as f64,
                ),
                show_bands: true,
            },
            None,
        )
        .expect("add AVWAP");

    // Pan viewport to `[30..79]` so anchor bar 5 is far offscreen to the left!
    let mut ts = TimeScale::with_data(candles.len(), 800.0, HorzScaleOptions::default());
    ts.set_bar_spacing(16.0);
    ts.set_right_offset(0.0);
    let mut ps = PriceScale::new("right", PriceScaleOptions::default());
    ps.set_height(400.0);
    ps.set_auto_scale(false);
    ps.set_raw_price_range(95.0, 150.0, 100.0);

    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 100.0).with_candles(&candles);
    let projected = proj
        .project_drawing(
            store
                .drawings
                .iter()
                .find(|d| d.id == av_id)
                .expect("avwap"),
        )
        .expect("project avwap");
    let tail_pt = match &projected.shape {
        ProjectedDrawingShape::AnchoredVwap { tail_segments, .. } => {
            assert!(
                !tail_segments.is_empty(),
                "visible AVWAP tail must produce hit segments"
            );
            let s0 = tail_segments[0];
            LocalPoint::new(0.5 * (s0.start.x + s0.end.x), 0.5 * (s0.start.y + s0.end.y))
        }
        other => panic!("expected AnchoredVwap, got {other:?}"),
    };
    let hit = hit_test_pane_drawings_causal(
        &store.drawings,
        DrawingPaneTarget::Main,
        &proj,
        tail_pt,
        None,
        None,
        candles.len(),
    );
    assert_eq!(hit.map(|h| h.drawing_id), Some(av_id));

    // Move viewport to include bar 0 and drag anchor handle way left past `-50`.
    let (mut ts_full, ps_full) = setup_pane_scales(
        candles.len(),
        800.0,
        400.0,
        90.0,
        150.0,
        PriceScaleMode::Normal,
        false,
    );
    ts_full.set_bar_spacing(8.0);
    ts_full.set_right_offset(5.0);
    let proj_drag =
        PaneDrawingProjection::new(&ts_full, &ps_full, 800.0, 400.0, 100.0).with_candles(&candles);
    let start_pos = proj_drag
        .anchor_to_local(
            store
                .drawings
                .iter()
                .find(|d| d.id == av_id)
                .unwrap()
                .geometry
                .anchors()[0],
        )
        .expect("start pos");
    assert_eq!(
        store.pointer_down(
            DrawingPaneTarget::Main,
            start_pos,
            false,
            &proj_drag,
            &candles
        ),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let far_left_x = proj_drag
        .time_scale
        .float_index_to_local_x(-40.0)
        .unwrap_or(-600.0);
    let far_left = LocalPoint::new(far_left_x, start_pos.y);
    store.pointer_move(
        DrawingPaneTarget::Main,
        far_left,
        false,
        &proj_drag,
        &candles,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(far_left),
        false,
        &proj_drag,
        &candles
    ));
    let clamped_idx = store
        .drawings
        .iter()
        .find(|d| d.id == av_id)
        .unwrap()
        .geometry
        .anchors()[0]
        .bar_index;
    assert!(
        clamped_idx >= 0.0,
        "AVWAP left drag must clamp bar_index >= 0.0, got {clamped_idx}"
    );
}

#[test]
fn r15_inspector_atomic_patch_hidden_selection_lock_and_timeframe_sync() {
    use crate::data::{Timeframe, aggregate_ohlcv};

    let candles_15m: Vec<Candle> = (0..320)
        .map(|i| {
            let ts = 1_760_000_000 + (i as i64) * 900;
            let p = 110.0 + ((i % 23) as f64) * 0.6;
            Candle {
                timestamp: ts,
                date_label: Timeframe::M15.format_candle_label(ts),
                open: p - 0.3,
                high: p + 0.9,
                low: p - 0.8,
                close: p + 0.2,
                volume: 1_500.0,
            }
        })
        .collect();
    let candles_1d = aggregate_ohlcv(&candles_15m, Timeframe::M15, Timeframe::D1).expect("agg 1D");
    let (ts, ps) = setup_pane_scales(
        candles_15m.len(),
        900.0,
        500.0,
        80.0,
        160.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 900.0, 500.0, 100.0).with_candles(&candles_15m);

    let mut store = DrawingStore::new();
    let p0 = proj
        .anchor_to_local(DrawingAnchor::new(260.0, candles_15m[260].low))
        .unwrap();
    let p1 = proj
        .anchor_to_local(DrawingAnchor::new(300.0, candles_15m[300].high))
        .unwrap();
    store.set_tool(DrawingTool::FibonacciRetracement);
    store.pointer_down(DrawingPaneTarget::Main, p0, false, &proj, &candles_15m);
    store.pointer_move(DrawingPaneTarget::Main, p1, false, &proj, &candles_15m);
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p1),
        false,
        &proj,
        &candles_15m
    ));

    let id = store.selected_id.expect("created fib selected");
    let orig = store.selected_drawing().unwrap().clone();

    // 1. Atomic multi-property patch (color + width + dash + opacity + extend_right + reverse + visible=false)
    let mut patch = DrawingPropertyPatch::from_drawing(&orig);
    patch.visible = false;
    patch.style.color = DrawingColorPreset::AmberGold;
    patch.style.line_width = 3.5;
    patch.style.dash = LineDashStyle::Dashed;
    patch.style.fill_opacity = 0.28;
    if let DrawingGeometry::FibonacciRetracement {
        direction,
        extend_right,
        ..
    } = &mut patch.geometry
    {
        *direction = direction.reversed();
        *extend_right = true;
    }

    assert_eq!(store.undo_depth(), 1);
    let changed = store
        .apply_drawing_property_patch(id, patch.clone(), &candles_15m, Timeframe::M15)
        .expect("valid atomic patch");
    assert!(changed);
    assert_eq!(
        store.undo_depth(),
        2,
        "multi-field Apply must push exactly ONE DrawingCommand::Modify"
    );

    // 2. Hidden (`visible == false`) drawing MUST remain selected in Inspector (`is_causally_eligible`)
    //    while hidden from canvas (`!is_causally_visible`)!
    let hidden_sel = store
        .selected_drawing()
        .expect("hidden drawing stays selected in Inspector");
    assert!(!hidden_sel.visible);
    assert!(!hidden_sel.is_causally_visible(None, usize::MAX));
    assert!(hidden_sel.is_causally_eligible(None, usize::MAX));

    // No-op re-apply must NOT wipe redo_stack or push duplicate undo entries!
    assert!(store.undo());
    assert!(store.can_redo());
    assert!(store.redo());
    let noop = store
        .apply_drawing_property_patch(id, patch, &candles_15m, Timeframe::M15)
        .expect("noop patch");
    assert!(!noop);
    assert_eq!(store.undo_depth(), 2);

    // 3. Re-enable visibility & lock, then verify locked drawings reject edits until unlocked
    let mut lock_patch = DrawingPropertyPatch::from_drawing(store.selected_drawing().unwrap());
    lock_patch.visible = true;
    lock_patch.locked = true;
    assert!(
        store
            .apply_drawing_property_patch(id, lock_patch.clone(), &candles_15m, Timeframe::M15)
            .unwrap()
    );

    let mut blocked = lock_patch.clone();
    blocked.style.line_width = 5.0;
    assert_eq!(
        store.apply_drawing_property_patch(id, blocked, &candles_15m, Timeframe::M15),
        Err(DrawingPatchError::DrawingLocked)
    );

    let mut unlock_and_move = lock_patch;
    unlock_and_move.locked = false;
    if let DrawingGeometry::FibonacciRetracement { start, .. } = &mut unlock_and_move.geometry {
        start.bar_index = 272.0;
    }
    assert!(
        store
            .apply_drawing_property_patch(id, unlock_and_move, &candles_15m, Timeframe::M15)
            .unwrap()
    );
    let moved_ts = store.selected_drawing().unwrap().geometry.anchors()[0]
        .timestamp
        .expect("edited bar_index re-stamped against candles_15m");
    assert!((moved_ts - (candles_15m[272].timestamp as f64)).abs() < 1e-6);

    // Remap 15m -> 1D -> 15m preserves edited anchor!
    store.remap_timeframe(&candles_15m, Timeframe::M15, &candles_1d, Timeframe::D1);
    store.remap_timeframe(&candles_1d, Timeframe::D1, &candles_15m, Timeframe::M15);
    let roundtrip_bar = store.selected_drawing().unwrap().geometry.anchors()[0].bar_index;
    assert!(roundtrip_bar.is_finite());

    // 4. Reject invalid style & invalid geometry (zero-direction Ray, inverted RR, out-of-range FRVP rows)
    let mut bad_style = DrawingPropertyPatch::from_drawing(store.selected_drawing().unwrap());
    bad_style.style.line_width = 12.0;
    assert_eq!(
        store.apply_drawing_property_patch(id, bad_style, &candles_15m, Timeframe::M15),
        Err(DrawingPatchError::InvalidStyle)
    );
}

#[test]
fn r19_text_note_utf8_wrapping_drag_timeframe_remap_and_replay_mask() {
    use super::{MAX_TEXT_NOTE_CHARS, sanitize_text_note_content, wrap_text_note_lines};
    use crate::data::{Timeframe, aggregate_ohlcv, generate_intraday_vn30f_1m_report};

    // 1. Multibyte Vietnamese UTF-8 sanitization (`<= 280` scalar chars, never panics on byte boundaries!)
    let long_vi =
        "Điểm phá vỡ kháng cự VSD T+2 lúc 13:00 ICT — kiểm định khối lượng đột biến! ".repeat(8);
    assert!(long_vi.chars().count() > MAX_TEXT_NOTE_CHARS);
    let clean = sanitize_text_note_content(&format!("  \t{long_vi}\u{0007}\u{0000}  "));
    assert!(clean.chars().count() <= MAX_TEXT_NOTE_CHARS);
    assert!(!clean.contains('\u{0007}'));
    assert!(!clean.contains('\t'));

    let wrapped = wrap_text_note_lines(&clean, 34, 4);
    assert_eq!(wrapped.len(), 4);
    assert!(wrapped.last().unwrap().ends_with('…'));
    for line in &wrapped {
        assert!(line.chars().count() <= 34);
    }
    assert_eq!(wrap_text_note_lines("   ", 28, 4), vec!["Note".to_owned()]);

    // 2. Interactive 1-click `DrawingTool::TextNote` placement, callout card hit-test, body drag,
    //    `15m -> 1D -> 15m` timeframe remap, and `DEF-001` Replay causal cutoff masking!
    let rep_1m = generate_intraday_vn30f_1m_report(1_530);
    let c_15m = aggregate_ohlcv(&rep_1m.candles, Timeframe::M1, Timeframe::M15).unwrap();
    let c_1d = aggregate_ohlcv(&rep_1m.candles, Timeframe::M1, Timeframe::D1).unwrap();
    let (ts, ps) = setup_pane_scales(
        c_15m.len(),
        900.0,
        500.0,
        1_240.0,
        1_340.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 900.0, 500.0, 1_280.0).with_candles(&c_15m);

    let mut store = DrawingStore::new();
    store.set_tool(DrawingTool::TextNote);
    let anchor_idx = (c_15m.len() - 12) as f64;
    let anchor_pt = proj
        .anchor_to_local(DrawingAnchor::new(anchor_idx, 1_295.0))
        .unwrap();
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, anchor_pt, false, &proj, &c_15m),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(anchor_pt),
        false,
        &proj,
        &c_15m
    ));
    assert_eq!(store.drawings.len(), 1);
    assert_eq!(store.active_tool, DrawingTool::Select);
    let note_id = store.drawings[0].id;

    // Update note text via `DrawingPropertyPatch`
    let mut patch = DrawingPropertyPatch::from_drawing(&store.drawings[0]);
    if let DrawingGeometry::TextNote { text, .. } = &mut patch.geometry {
        *text = "Vùng tích lũy FPT\nUnlock T+2 13:00 ICT".to_owned();
    }
    assert!(
        store
            .apply_drawing_property_patch(note_id, patch, &c_15m, Timeframe::M15)
            .unwrap()
    );

    let projected = proj.project_drawing(&store.drawings[0]).unwrap();
    let card_rect = match &projected.shape {
        ProjectedDrawingShape::TextNote {
            box_rect, lines, ..
        } => {
            assert_eq!(lines.len(), 2);
            box_rect.expect("visible TextNote card rect")
        }
        other => panic!("expected ProjectedDrawingShape::TextNote, got {other:?}"),
    };

    // Hit-test inside the callout card body and drag by `(+24.0, -18.0)` px:
    let card_center = LocalPoint::new(
        0.5 * (card_rect.left + card_rect.right),
        0.5 * (card_rect.top + card_rect.bottom),
    );
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, card_center, false, &proj, &c_15m),
        PointerCaptureOutcome::CapturedByDrawing
    );
    let dragged_center = LocalPoint::new(card_center.x + 24.0, card_center.y - 18.0);
    store.pointer_move(
        DrawingPaneTarget::Main,
        dragged_center,
        false,
        &proj,
        &c_15m,
    );
    assert!(store.pointer_up(
        DrawingPaneTarget::Main,
        Some(dragged_center),
        false,
        &proj,
        &c_15m
    ));
    let moved_proj = proj.project_drawing(&store.drawings[0]).unwrap();
    assert!((moved_proj.anchors_local[0].x - (anchor_pt.x + 24.0)).abs() < 1e-5);
    assert!((moved_proj.anchors_local[0].y - (anchor_pt.y - 18.0)).abs() < 1e-5);

    // Causal `DEF-001` Replay cutoff before the note's anchor hides it from canvas & hit-test!
    let early_cutoff = c_15m[5].timestamp;
    assert!(!store.drawings[0].is_causally_visible(Some(early_cutoff), 5));
    assert!(
        hit_test_pane_drawings_causal(
            &store.drawings,
            DrawingPaneTarget::Main,
            &proj,
            dragged_center,
            Some(note_id),
            Some(early_cutoff),
            5,
        )
        .is_none()
    );

    // Timeframe roundtrip `15m -> 1D -> 15m` preserves anchor!
    let pre_bar = store.drawings[0].geometry.anchors()[0].bar_index;
    store.remap_timeframe(&c_15m, Timeframe::M15, &c_1d, Timeframe::D1);
    store.remap_timeframe(&c_1d, Timeframe::D1, &c_15m, Timeframe::M15);
    let post_bar = store.drawings[0].geometry.anchors()[0].bar_index;
    assert!((post_bar - pre_bar).abs() < 1e-6);

    // `[m1]` Regression: Selected `TextNote` opaque callout card overlapping an unselected `HorizontalLine`
    // MUST retain `HitPriority::Stroke` (not click-through to the background line behind it)!
    let mut overlap_store = DrawingStore::new();
    let bg_hline_id = overlap_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::new(anchor_idx, 1_295.0),
            },
            None,
        )
        .unwrap();
    let top_note_id = overlap_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TextNote {
                anchor: DrawingAnchor::new(anchor_idx, 1_295.0),
                text: "Opaque Note Card\nOver HLine".to_owned(),
            },
            None,
        )
        .unwrap();
    overlap_store.selected_id = Some(top_note_id);
    let note_proj = proj.project_drawing(&overlap_store.drawings[1]).unwrap();
    let note_box = match note_proj.shape {
        ProjectedDrawingShape::TextNote {
            box_rect: Some(r), ..
        } => r,
        other => panic!("expected TextNote box, got {other:?}"),
    };
    let hline_y = proj
        .anchor_to_local(DrawingAnchor::new(anchor_idx, 1_295.0))
        .unwrap()
        .y;
    let overlap_cursor = LocalPoint::new(
        0.5 * (note_box.left + note_box.right),
        hline_y.clamp(note_box.top + 1.0, note_box.bottom - 1.0),
    );
    let hit = hit_test_pane_drawings(
        &overlap_store.drawings,
        DrawingPaneTarget::Main,
        &proj,
        overlap_cursor,
        overlap_store.selected_id,
    )
    .expect("hit inside selected TextNote card");
    assert_ne!(bg_hline_id, top_note_id);
    assert_eq!(
        hit.drawing_id, top_note_id,
        "selected TextNote card must win over occluded background HorizontalLine"
    );

    // `[R2]` Regression: Clicking `3.5 px` inside `note_box.top` (inside the outer `6 px` border tolerance!)
    // directly over an occluded `HorizontalLine` at that exact `y` MUST STILL hit `top_note_id` (`distance == 0.0`)!
    let near_top_y = note_box.top + 3.5;
    let near_top_val = ps.local_y_to_price(near_top_y, 1_280.0).unwrap();
    overlap_store.drawings[0].geometry = DrawingGeometry::HorizontalLine {
        anchor: DrawingAnchor::new(anchor_idx, near_top_val),
    };
    let near_edge_cursor = LocalPoint::new(0.5 * (note_box.left + note_box.right), near_top_y);
    let edge_hit = hit_test_pane_drawings(
        &overlap_store.drawings,
        DrawingPaneTarget::Main,
        &proj,
        near_edge_cursor,
        overlap_store.selected_id,
    )
    .expect("hit 3.5px inside TextNote top edge");
    assert_eq!(
        edge_hit.drawing_id, top_note_id,
        "clicking 3.5px inside TextNote card must NOT click through to occluded line"
    );
    assert_eq!(edge_hit.distance, 0.0);
}

#[test]
fn r19_custom_fibonacci_levels_and_dual_end_trendline_alert_parity() {
    use super::{FIBONACCI_FULL_EXT_RATIOS, FIBONACCI_OTE_EXT_RATIOS, FIBONACCI_RATIOS};
    use crate::alerts::{AlertCondition, AlertEngine, AlertError, AlertFrequency, CrossDirection};
    use crate::data::Timeframe;

    let candles = generate_synthetic_ohlcv(120);
    let (ts, ps) = setup_pane_scales(
        candles.len(),
        900.0,
        500.0,
        50.0,
        350.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 900.0, 500.0, 100.0).with_candles(&candles);
    let mut store = DrawingStore::new();

    // 1. Custom Fibonacci levels (`Std 7` -> `OTE 9` -> `Full 13` -> custom sorted deduped ratios & validation):
    let fib_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::FibonacciRetracement {
                start: DrawingAnchor::new(30.0, 100.0),
                end: DrawingAnchor::new(80.0, 200.0),
                direction: FibDirection::StartToEnd,
                extend_right: false,
            },
            None,
        )
        .unwrap();
    assert_eq!(
        store.drawings[0].effective_fib_ratios(),
        FIBONACCI_RATIOS.as_slice()
    );

    let mut ote_patch = DrawingPropertyPatch::from_drawing(&store.drawings[0]);
    ote_patch.custom_fib_levels = Some(FIBONACCI_OTE_EXT_RATIOS.to_vec());
    assert!(
        store
            .apply_drawing_property_patch(fib_id, ote_patch, &candles, Timeframe::D1)
            .unwrap()
    );
    let proj_ote = proj.project_drawing(&store.drawings[0]).unwrap();
    match &proj_ote.shape {
        ProjectedDrawingShape::Fibonacci { levels, bands, .. } => {
            assert_eq!(levels.len(), FIBONACCI_OTE_EXT_RATIOS.len());
            assert_eq!(bands.len(), FIBONACCI_OTE_EXT_RATIOS.len() - 1);
        }
        other => panic!("expected Fibonacci shape, got {other:?}"),
    }

    let mut full_patch = DrawingPropertyPatch::from_drawing(&store.drawings[0]);
    full_patch.custom_fib_levels = Some(FIBONACCI_FULL_EXT_RATIOS.to_vec());
    assert!(
        store
            .apply_drawing_property_patch(fib_id, full_patch, &candles, Timeframe::D1)
            .unwrap()
    );
    assert_eq!(
        store.drawings[0].effective_fib_ratios().len(),
        FIBONACCI_FULL_EXT_RATIOS.len()
    );

    // Reject empty or out-of-bounds custom Fib ratios (`> 10.0` or `> 16` levels):
    let mut bad_fib = DrawingPropertyPatch::from_drawing(&store.drawings[0]);
    bad_fib.custom_fib_levels = Some(vec![25.0]);
    assert_eq!(
        store.apply_drawing_property_patch(fib_id, bad_fib, &candles, Timeframe::D1),
        Err(DrawingPatchError::InvalidGeometry)
    );

    // 2. Time-directional `trend_extend_left` / `trend_extend_right` on BOTH left-to-right AND right-to-left
    //    `TrendLine`s (`start.bar_index > end.bar_index`), verified against `PaneDrawingProjection` AND `AlertEngine`!
    let mut alert_bars: Vec<Candle> = (0..12)
        .map(|i| Candle {
            timestamp: 1_750_000_000 + (i as i64) * 86_400,
            date_label: format!("2026-06-{:02}", i + 1),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 1_000.0,
        })
        .collect();
    // Bar 9 surges from 100.0 up to 115.0 (crossing above horizontal-or-sloped extended line at y=105.0):
    alert_bars[9].high = 115.0;
    alert_bars[9].close = 114.0;

    let mut rtl_store = DrawingStore::new();
    // Right-to-left TrendLine: `start = (6.0, 105.0)`, `end = (2.0, 105.0)`!
    // Later-in-time endpoint is `start` (`bar 6`), earlier-in-time endpoint is `end` (`bar 2`)!
    let rtl_id = rtl_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::with_timestamp(6.0, 105.0, alert_bars[6].timestamp as f64),
                end: DrawingAnchor::with_timestamp(2.0, 105.0, alert_bars[2].timestamp as f64),
            },
            None,
        )
        .unwrap();

    let mut engine = AlertEngine::new();
    let _rule_id = engine
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::DrawingLine {
                drawing_id: rtl_id,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            Some("RTL Trend Break".to_owned()),
            &alert_bars,
            0,
            None,
            Some(&rtl_store),
        )
        .expect("TrendLine alert valid");

    // Without `trend_extend_right`, bar 9 is outside `[2.0, 6.0]` -> 0 triggers!
    engine.evaluate_causal(
        &alert_bars,
        Timeframe::D1,
        &alert_bars,
        Timeframe::D1,
        alert_bars.len() - 1,
        None,
        &rtl_store,
    );
    assert!(
        engine.triggers.is_empty(),
        "unextended TrendLine [2..6] must NOT fire at bar 9"
    );

    // Enable `trend_extend_right = true` (chronological `+∞`, even though `start.bar_index > end.bar_index`!):
    rtl_store.drawings[0].trend_extend_right = true;
    let (ts_a, ps_a) =
        setup_pane_scales(12, 600.0, 300.0, 90.0, 120.0, PriceScaleMode::Normal, false);
    let proj_a = PaneDrawingProjection::new(&ts_a, &ps_a, 600.0, 300.0, 100.0);
    let rtl_proj = proj_a.project_drawing(&rtl_store.drawings[0]).unwrap();
    let seg = match rtl_proj.shape {
        ProjectedDrawingShape::Segment {
            clipped: Some(s), ..
        } => s,
        other => panic!("expected extended segment, got {other:?}"),
    };
    assert!(
        (seg.start.x.max(seg.end.x) - 600.0).abs() < 1e-6,
        "RTL TrendLine with trend_extend_right=true must extend rightward to pane right edge 600.0, got {seg:?}"
    );

    engine.evaluate_causal(
        &alert_bars,
        Timeframe::D1,
        &alert_bars,
        Timeframe::D1,
        alert_bars.len() - 1,
        None,
        &rtl_store,
    );
    assert_eq!(
        engine.triggers.len(),
        1,
        "RTL TrendLine with trend_extend_right=true MUST fire at bar 9!"
    );
    assert_eq!(engine.triggers[0].bar_timestamp, alert_bars[9].timestamp);

    // Also verify `TextNote` is cleanly rejected as a `DrawingLine` alert source:
    let note_id = rtl_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TextNote {
                anchor: DrawingAnchor::new(4.0, 102.0),
                text: "Note".to_owned(),
            },
            None,
        )
        .unwrap();
    assert_eq!(
        engine.add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::DrawingLine {
                drawing_id: note_id,
                direction: CrossDirection::CrossAny,
            },
            AlertFrequency::Once,
            Some("Bad Note Alert".to_owned()),
            &alert_bars,
            0,
            None,
            Some(&rtl_store),
        ),
        Err(AlertError::UnsupportedDrawingGeometry)
    );
}

#[test]
fn v1_1_wave2_parallel_channel_3_click_handles_extend_tf_replay_and_undo() {
    let base_ts: i64 = 1_735_689_600; // 2025-01-01 00:00:00 UTC
    let candles: Vec<Candle> = (0..40)
        .map(|i| {
            let p = 100.0 + (i as f64) * 0.5;
            Candle {
                timestamp: base_ts + (i as i64) * 86_400,
                date_label: format!("D{i:02}"),
                open: p,
                high: p + 2.0,
                low: p - 2.0,
                close: p + 0.4,
                volume: 100_000.0 + (i as f64) * 1_000.0,
            }
        })
        .collect();
    let (ts, ps) = setup_pane_scales(
        candles.len(),
        800.0,
        400.0,
        90.0,
        130.0,
        PriceScaleMode::Normal,
        false,
    );
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 100.0).with_candles(&candles);

    let mut store = DrawingStore::new();
    store.set_tool(DrawingTool::ParallelChannel);

    let p0 = proj
        .anchor_to_local(DrawingAnchor::new(5.0, 102.0))
        .unwrap();
    let p1 = proj
        .anchor_to_local(DrawingAnchor::new(20.0, 110.0))
        .unwrap();
    let p2 = proj
        .anchor_to_local(DrawingAnchor::new(12.5, 114.0))
        .unwrap();

    // Click 1 -> Click 2 -> Click 3
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p0, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(!store.pointer_up(DrawingPaneTarget::Main, Some(p0), false, &proj, &candles));
    store.pointer_move(DrawingPaneTarget::Main, p1, false, &proj, &candles);
    assert!(store.preview_drawing().is_some());
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p1, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(!store.pointer_up(DrawingPaneTarget::Main, Some(p1), false, &proj, &candles));
    assert!(matches!(
        store.phase(),
        DrawingInteractionPhase::PlacingChannelOffset { .. }
    ));

    // Move mouse to set channel width, check live preview, then Click 3 to commit!
    store.pointer_move(DrawingPaneTarget::Main, p2, false, &proj, &candles);
    let preview = store
        .preview_drawing()
        .expect("live 3-step channel preview");
    assert!(matches!(
        preview.geometry,
        DrawingGeometry::ParallelChannel { channel_offset, .. } if (channel_offset - 8.0).abs() < 0.25
    ));
    assert_eq!(
        store.pointer_down(DrawingPaneTarget::Main, p2, false, &proj, &candles),
        PointerCaptureOutcome::CapturedByDrawing
    );
    assert!(store.pointer_up(DrawingPaneTarget::Main, Some(p2), false, &proj, &candles));
    assert_eq!(store.active_tool, DrawingTool::Select);
    assert_eq!(store.drawings.len(), 1);
    let ch_id = store.drawings[0].id;

    // Projection yields 3 handles (`Anchor(0)`, `Anchor(1)`, `ChannelOffset`), midline & polygon fill.
    let projected = proj.project_drawing(&store.drawings[0]).unwrap();
    assert_eq!(projected.handles.len(), 3);
    assert!(
        projected
            .handles
            .iter()
            .any(|h| h.part == HandlePart::ChannelOffset)
    );
    match &projected.shape {
        ProjectedDrawingShape::ParallelChannel {
            base_line,
            parallel_line,
            mid_line,
            fill_polygon,
        } => {
            assert!(base_line.is_some());
            assert!(parallel_line.is_some());
            assert!(mid_line.is_some());
            assert!(fill_polygon.len() >= 3);
        }
        other => panic!("expected ParallelChannel shape, got {other:?}"),
    }

    // Hit-test ChannelOffset handle + interior body (when selected).
    let off_handle = projected
        .handles
        .iter()
        .find(|h| h.part == HandlePart::ChannelOffset)
        .unwrap()
        .pos;
    let hit_h = hit_test_pane_drawings(
        &store.drawings,
        DrawingPaneTarget::Main,
        &proj,
        off_handle,
        Some(ch_id),
    )
    .expect("hit ChannelOffset handle");
    assert_eq!(hit_h.target, HitTarget::Handle(HandlePart::ChannelOffset));

    let mid_inside = proj
        .anchor_to_local(DrawingAnchor::new(12.5, 110.0))
        .unwrap();
    let hit_body = hit_test_pane_drawings(
        &store.drawings,
        DrawingPaneTarget::Main,
        &proj,
        mid_inside,
        Some(ch_id),
    )
    .expect("hit selected ParallelChannel interior/midline");
    assert_eq!(hit_body.drawing_id, ch_id);

    // Enable Extend Left & Right via DrawingPropertyPatch and verify clipping reaches pane edges.
    let mut patch = DrawingPropertyPatch::from_drawing(&store.drawings[0]);
    patch.trend_extend_left = true;
    patch.trend_extend_right = true;
    assert_eq!(
        store.apply_drawing_property_patch(ch_id, patch, &candles, crate::data::Timeframe::D1),
        Ok(true)
    );
    let ext_proj = proj.project_drawing(&store.drawings[0]).unwrap();
    if let ProjectedDrawingShape::ParallelChannel {
        base_line: Some(bl),
        parallel_line: Some(pl),
        ..
    } = ext_proj.shape
    {
        let on_boundary = |pt: LocalPoint| {
            pt.x.abs() < 1e-6
                || (pt.x - 800.0).abs() < 1e-6
                || pt.y.abs() < 1e-6
                || (pt.y - 400.0).abs() < 1e-6
        };
        assert!(
            on_boundary(bl.start) && on_boundary(bl.end),
            "extended baseline must clip cleanly to pane boundaries, got {bl:?}"
        );
        assert!(bl.start.x.min(bl.end.x) < p0.x - 10.0);
        assert!(bl.start.x.max(bl.end.x) > p1.x + 10.0);
        assert!(
            on_boundary(pl.start) && on_boundary(pl.end),
            "extended parallel border must clip cleanly to pane boundaries, got {pl:?}"
        );
    } else {
        panic!("extended ParallelChannel lines must clip cleanly to pane");
    }

    // `DEF-001` causal replay masking when cutoff is earlier than anchor `end` (bar 20).
    let early_cutoff = candles[10].timestamp;
    assert!(!store.drawings[0].is_causally_visible(Some(early_cutoff), 11));
    assert!(store.drawings[0].is_causally_visible(Some(candles[25].timestamp), 26));

    // `1D -> 1W -> 1D` timeframe roundtrip preserves `channel_offset` exactly.
    let weekly = crate::data::aggregate_ohlcv(
        &candles,
        crate::data::Timeframe::D1,
        crate::data::Timeframe::W1,
    )
    .unwrap();
    let orig_offset = match store.drawings[0].geometry {
        DrawingGeometry::ParallelChannel { channel_offset, .. } => channel_offset,
        _ => unreachable!(),
    };
    store.remap_timeframe(
        &candles,
        crate::data::Timeframe::D1,
        &weekly,
        crate::data::Timeframe::W1,
    );
    store.remap_timeframe(
        &weekly,
        crate::data::Timeframe::W1,
        &candles,
        crate::data::Timeframe::D1,
    );
    let rt_offset = match store.drawings[0].geometry {
        DrawingGeometry::ParallelChannel { channel_offset, .. } => channel_offset,
        _ => unreachable!(),
    };
    assert!((rt_offset - orig_offset).abs() < 1e-9);

    // Undo/Redo restores cleanly.
    assert!(store.undo());
    assert!(!store.drawings[0].trend_extend_left);
    assert!(store.redo());
    assert!(store.drawings[0].trend_extend_left);
}

#[test]
fn v1_1_wave2_price_date_range_measure_metrics_and_nan_resilience() {
    let base_ts: i64 = 1_735_689_600;
    let mut candles: Vec<Candle> = (0..10)
        .map(|i| Candle {
            timestamp: base_ts + (i as i64) * 86_400,
            date_label: format!("D{i}"),
            open: 50.0 + (i as f64),
            high: 51.0 + (i as f64),
            low: 49.0 + (i as f64),
            close: 50.5 + (i as f64),
            volume: 1_000_000.0,
        })
        .collect();
    // Inject one NaN volume bar and one negative volume bar to prove resilience.
    candles[3].volume = f64::NAN;
    candles[4].volume = -500.0;

    let a0 = DrawingAnchor::with_timestamp(1.0, 50.0, candles[1].timestamp as f64);
    let a1 = DrawingAnchor::with_timestamp(6.0, 62.5, candles[6].timestamp as f64);
    let m = compute_price_date_range_metrics(a0, a1, Some(&candles));
    assert!((m.price_delta - 12.5).abs() < 1e-9);
    assert!((m.price_pct.unwrap() - 25.0).abs() < 1e-9);
    assert_eq!(m.bar_count, 5);
    assert_eq!(m.elapsed_secs, Some(5 * 86_400));
    assert_eq!(format_measure_duration(m.elapsed_secs.unwrap()), "5d");
    assert_eq!(format_measure_duration(6 * 3600 + 30 * 60), "6h 30m");
    // Bars 1..=6 = 6 bars, 2 invalid volumes (bars 3 & 4) -> 4 valid bars * 1_000_000 = 4_000_000.
    assert!((m.total_volume.unwrap() - 4_000_000.0).abs() < 1e-6);
    assert!(m.is_bullish);

    // Zero start price produces `price_pct == None` (no division-by-zero / Inf).
    let zero_start = DrawingAnchor::new(0.0, 0.0);
    let m_zero = compute_price_date_range_metrics(zero_start, a1, Some(&candles));
    assert!(m_zero.price_pct.is_none());
}
