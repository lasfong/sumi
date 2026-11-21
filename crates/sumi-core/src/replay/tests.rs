use crate::data::{Timeframe, aggregate_ohlcv};
use crate::drawings::{
    Drawing, DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStore, DrawingStyle,
    DrawingTool, PaneDrawingProjection, hit_test_pane_drawings_causal,
};
use crate::model::Candle;
use crate::viewport::{HorzScaleOptions, PriceScale, PriceScaleOptions, TimeScale};

use super::{
    MAX_PLAYBACK_DT_SECS, MAX_STEPS_PER_FRAME, ReplayController, ReplaySpeed, ReplayStatus,
    ReplayStepUnit,
};

fn build_1m_series(count: usize) -> Vec<Candle> {
    // Starts Monday 2026-03-02 09:00:00 UTC (`1772442000`, exact 1h/15m/5m boundary).
    let start_ts: i64 = 1_772_442_000;
    (0..count)
        .map(|i| {
            let base = 1_000.0 + (i as f64) * 0.5 + ((i % 7) as f64 - 3.0) * 1.2;
            let open = base;
            let close = base + if i % 2 == 0 { 1.5 } else { -1.1 };
            // Inject dramatic future spikes in the second half so any DEF-001 leakage is unmistakable.
            let spike = if i >= count / 2 { 250.0 } else { 2.0 };
            let high = open.max(close) + spike;
            let low = (open.min(close) - if i >= count / 2 { 180.0 } else { 1.8 }).max(1.0);
            Candle {
                timestamp: start_ts + (i as i64) * 60,
                date_label: String::new(),
                open,
                high,
                low,
                close,
                volume: 100.0 + (i as f64) * 3.0,
            }
        })
        .collect()
}

#[test]
fn def_001_zero_lookahead_slicing_across_all_higher_timeframes() {
    let base = build_1m_series(180);
    let mut replay = ReplayController::new();
    // Cut mid-way through a 15m / 1h bucket (`index = 37`, minute `:37`, well before spike at `90`).
    let cut_ts = replay
        .start_at_base_index(&base, 37)
        .expect("cutoff timestamp");
    assert_eq!(cut_ts, base[37].timestamp);
    assert_eq!(replay.status, ReplayStatus::Paused);
    assert!(replay.is_active());

    for tf in [
        Timeframe::M1,
        Timeframe::M5,
        Timeframe::M15,
        Timeframe::H1,
        Timeframe::H4,
        Timeframe::D1,
        Timeframe::W1,
    ] {
        let materialized = replay
            .materialize_candles(&base, Timeframe::M1, tf)
            .expect("materialize causal series");
        let expected_prefix =
            aggregate_ohlcv(&base[..=37], Timeframe::M1, tf).expect("prefix aggregation");
        assert_eq!(
            materialized, expected_prefix,
            "DEF-001 violation on {tf:?}: materialized candles diverged from strict causal prefix"
        );
        // Verify no future spike (`>= 1200.0`) from `i >= 90` leaked into any bar's OHLC!
        for bar in &materialized {
            assert!(
                bar.high < 1_200.0 && bar.low > 900.0,
                "future OHLC extremum leaked into {tf:?}: {bar:?}"
            );
        }
    }
}

#[test]
fn active_bar_vs_base_bar_stepping_and_mid_bucket_completion() {
    let base = build_1m_series(90); // 6 full 15m buckets (`0..14`, `15..29`, `30..44`, ...)
    let mut replay = ReplayController::new();
    // Start mid-bucket at `index = 20` (inside second `15m` bucket `15..=29`).
    replay.start_at_base_index(&base, 20);
    replay.step_unit = ReplayStepUnit::ActiveBar;

    // Step 1 (`ActiveBar`): completes current partial `15m` bucket (`20 -> 29`).
    let t1 = replay
        .step_forward(&base, Timeframe::M15, 1)
        .expect("complete partial 15m bucket");
    assert_eq!(t1.prev_base_index, 20);
    assert_eq!(t1.new_base_index, 29);

    // Step 2 (`ActiveBar`): advances one full `15m` bucket (`29 -> 44`).
    let t2 = replay
        .step_forward(&base, Timeframe::M15, 1)
        .expect("advance next full 15m bucket");
    assert_eq!(t2.prev_base_index, 29);
    assert_eq!(t2.new_base_index, 44);

    // Switch to `BaseBar` (`Tick`) mode: advances single `1m` tick (`44 -> 45`), opening 4th `15m` bucket with 1 tick!
    replay.step_unit = ReplayStepUnit::BaseBar;
    let t3 = replay
        .step_forward(&base, Timeframe::M15, 1)
        .expect("advance 1m base tick");
    assert_eq!(t3.new_base_index, 45);
    let m15_partial = replay
        .materialize_candles(&base, Timeframe::M1, Timeframe::M15)
        .unwrap();
    assert_eq!(m15_partial.len(), 4);
    assert!((m15_partial[3].volume - base[45].volume).abs() < 1e-9);

    // Step backward in `ActiveBar` mode from `45` (first tick of bucket `45..59`) lands on `44` (close of bucket `30..44`).
    replay.step_unit = ReplayStepUnit::ActiveBar;
    let back1 = replay
        .step_backward(&base, Timeframe::M15, 1)
        .expect("step back to previous 15m close");
    assert_eq!(back1.new_base_index, 44);
    assert_eq!(replay.status, ReplayStatus::Paused);

    // Stepping backward past `start_base_index` (`20`) updates `start_base_index` cleanly.
    let back2 = replay
        .step_backward(&base, Timeframe::M15, 2)
        .expect("step back 2 buckets");
    assert_eq!(back2.new_base_index, 14);
    assert_eq!(replay.start_base_index, 14);
}

#[test]
fn cut_selection_while_active_never_exposes_future_bars() {
    let base = build_1m_series(120);
    let mut replay = ReplayController::new();
    replay.start_at_base_index(&base, 44);
    assert!(replay.toggle_play_pause(&base));
    assert_eq!(replay.status, ReplayStatus::Playing);

    // Activating `Jump to Bar` while already in Replay must pause playback WITHOUT dropping `is_active()`!
    replay.enter_cut_selection();
    assert!(replay.selecting_cut_bar);
    assert!(replay.is_active());
    assert_eq!(replay.status, ReplayStatus::Paused);
    assert_eq!(replay.cutoff_timestamp(&base), Some(base[44].timestamp));

    let visible = replay
        .materialize_candles(&base, Timeframe::M1, Timeframe::M15)
        .unwrap();
    assert_eq!(visible.len(), 3); // Strictly buckets `0..14`, `15..29`, `30..44` — zero future leak!

    // Resolving a clicked bar while active clamps to `max_allowed_base_index`.
    let resolved = ReplayController::resolve_cut_base_index(
        &base,
        &visible,
        Timeframe::M15,
        1, // bucket `15..29`
        Some(replay.base_cursor_index),
    )
    .expect("resolve cut index");
    assert_eq!(resolved, 29);

    replay.cancel_cut_selection();
    assert!(!replay.selecting_cut_bar);
    assert_eq!(replay.base_cursor_index, 44);
}

#[test]
fn playback_accumulator_respects_speed_burst_cap_and_end_of_data() {
    let base = build_1m_series(25);
    let mut replay = ReplayController::new();
    replay.start_at_base_index(&base, 10);
    replay.speed = ReplaySpeed::X20; // 20 steps/s => 0.05s per step
    replay.step_unit = ReplayStepUnit::BaseBar;
    assert!(replay.toggle_play_pause(&base));

    // Sub-interval dt (`0.02s < 0.05s`) accumulates without stepping.
    assert!(
        replay
            .advance_playback_dt(0.02, &base, Timeframe::M1)
            .is_none()
    );
    assert!(
        replay
            .advance_playback_dt(f64::NAN, &base, Timeframe::M1)
            .is_none()
    );
    assert!(
        replay
            .advance_playback_dt(-1.0, &base, Timeframe::M1)
            .is_none()
    );

    // Huge stall (`5.0s`) is clamped by `MAX_PLAYBACK_DT_SECS` (`0.25s`) and `MAX_STEPS_PER_FRAME` (`8`).
    let step = replay
        .advance_playback_dt(5.0, &base, Timeframe::M1)
        .expect("clamped playback burst");
    let delta = step.new_base_index - step.prev_base_index;
    assert!((1..=MAX_STEPS_PER_FRAME).contains(&delta));
    let max_expected = ((0.02 + MAX_PLAYBACK_DT_SECS) / 0.05).floor() as usize;
    assert_eq!(delta, max_expected.min(MAX_STEPS_PER_FRAME));

    // Advance to final bar (`24`) -> transitions automatically to `Completed`.
    let _ = replay.step_forward(&base, Timeframe::M1, 100);
    assert_eq!(replay.base_cursor_index, 24);
    assert_eq!(replay.status, ReplayStatus::Completed);
    assert!(!replay.toggle_play_pause(&base));
}

#[test]
fn sanitize_deserialized_pauses_playing_and_clamps_indices() {
    let base = build_1m_series(30);
    let mut replay = ReplayController::new();
    replay.status = ReplayStatus::Playing;
    replay.selecting_cut_bar = true;
    replay.start_base_index = 500;
    replay.base_cursor_index = 999;
    replay.speed = ReplaySpeed::X5;
    replay.step_unit = ReplayStepUnit::BaseBar;
    let serialized = serde_json::to_string(&replay).expect("serialize replay");
    let mut restored: ReplayController =
        serde_json::from_str(&serialized).expect("deserialize replay");
    restored.sanitize_deserialized(&base);
    assert!(!restored.selecting_cut_bar);
    assert_eq!(restored.base_cursor_index, 29);
    assert_eq!(restored.start_base_index, 29);
    assert_eq!(restored.status, ReplayStatus::Completed);

    // Mid-series `Playing` restores as `Paused` so workspaces never advance unexpectedly on launch.
    replay.base_cursor_index = 12;
    replay.start_base_index = 5;
    replay.sanitize_deserialized(&base);
    assert_eq!(replay.status, ReplayStatus::Paused);
    assert_eq!(replay.base_cursor_index, 12);
    assert_eq!(replay.start_base_index, 5);
}

#[test]
fn causal_drawing_visibility_masks_future_pre_replay_and_preserves_replay_projections() {
    let base = build_1m_series(60);
    let mut store = DrawingStore::new();

    // 1. Author a pre-replay TrendLine spanning bars `10..45` (`created_at_timestamp == None`).
    let pre_replay_trend = Drawing::new(
        1,
        DrawingPaneTarget::Main,
        DrawingGeometry::TrendLine {
            start: DrawingAnchor::with_timestamp(10.0, 1005.0, base[10].timestamp as f64),
            end: DrawingAnchor::with_timestamp(45.0, 1030.0, base[45].timestamp as f64),
        },
        DrawingStyle::default_for_tool(DrawingTool::TrendLine),
    );
    store.drawings.push(pre_replay_trend);
    store.selected_id = Some(1);

    // Enter replay at bar `25` (`cutoff < base[45].timestamp`): pre-replay future drawing is masked & deselected!
    let cutoff_25 = base[25].timestamp;
    store.set_replay_cutoff_timestamp(Some(cutoff_25), 26);
    assert_eq!(store.selected_id, None);
    assert!(!store.drawings[0].is_causally_visible(Some(cutoff_25), 26));

    // Hit-testing at bar `10` must also ignore the masked pre-replay drawing!
    let mut ts = TimeScale::with_data(26, 800.0, HorzScaleOptions::default());
    ts.set_bar_spacing(10.0);
    ts.set_right_offset(15.0);
    let mut ps = PriceScale::new("right", PriceScaleOptions::default());
    ps.set_height(400.0);
    ps.set_auto_scale(false);
    ps.set_raw_price_range(950.0, 1100.0, 1000.0);
    let proj = PaneDrawingProjection::new(&ts, &ps, 800.0, 400.0, 1000.0);

    let p10 = proj
        .anchor_to_local(DrawingAnchor::new(10.0, 1005.0))
        .expect("local point");
    assert!(
        hit_test_pane_drawings_causal(
            &store.drawings,
            DrawingPaneTarget::Main,
            &proj,
            p10,
            None,
            Some(cutoff_25),
            26,
        )
        .is_none()
    );

    // 2. Author a forward projection *during* replay at `cutoff_25` extending into right-offset whitespace (`bar 35`)!
    store.active_tool = DrawingTool::Ray;
    let p20 = proj
        .anchor_to_local(DrawingAnchor::new(20.0, 1010.0))
        .unwrap();
    let p35 = proj
        .anchor_to_local(DrawingAnchor::new(35.0, 1040.0))
        .unwrap();
    let _ = store.pointer_down(DrawingPaneTarget::Main, p20, false, &proj, &base[..=25]);
    store.pointer_move(DrawingPaneTarget::Main, p35, false, &proj, &base[..=25]);
    let _ = store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p35),
        false,
        &proj,
        &base[..=25],
    );
    assert_eq!(store.drawings.len(), 2);
    let replay_ray = &store.drawings[1];
    assert_eq!(replay_ray.created_at_timestamp, Some(cutoff_25));
    // Visible at `cutoff_25` despite `through.bar_index > 25`!
    assert!(replay_ray.is_causally_visible(Some(cutoff_25), 26));
    if let DrawingGeometry::Ray { through, .. } = &mut store.drawings[1].geometry {
        // Clear initial extrapolated timestamp if any, or verify refresh updates it once bar 35 materializes.
        through.bar_index = 35.0;
        through.timestamp = None;
    }

    // Step replay forward to bar `50`: both the pre-replay drawing (`end=45 <= 50`) and replay ray are visible,
    // and `refresh_materialized_anchor_timestamps` binds the exact timestamp of bar `35`!
    store.refresh_materialized_anchor_timestamps(26, &base[..=50], Timeframe::M1);
    store.set_replay_cutoff_timestamp(Some(base[50].timestamp), 51);
    assert!(store.drawings[0].is_causally_visible(Some(base[50].timestamp), 51));
    assert!(store.drawings[1].is_causally_visible(Some(base[50].timestamp), 51));
    if let DrawingGeometry::Ray { through, .. } = store.drawings[1].geometry {
        assert_eq!(through.timestamp, Some(base[35].timestamp as f64));
    } else {
        panic!("expected Ray");
    }

    // Rewind earlier than `cutoff_25` (`bar 15`): the replay-authored ray hides causally!
    store.set_replay_cutoff_timestamp(Some(base[15].timestamp), 16);
    assert!(!store.drawings[1].is_causally_visible(Some(base[15].timestamp), 16));
}

#[test]
fn critic_r07_regression_causal_undo_redo_clear_unlocked_whitespace_drag_and_dense_hit_test() {
    use crate::drawings::FibDirection;

    // Build 100 bars with an overnight session gap between bar 24 and bar 25 (`+65,700s`).
    let mut base = Vec::with_capacity(100);
    let mut ts = 1_700_000_000_i64;
    for i in 0..100 {
        if i == 25 {
            ts += 65_700;
        } else if i > 0 {
            ts += 60;
        }
        let o = 1_000.0 + (i as f64) * 1.5;
        base.push(Candle {
            timestamp: ts,
            date_label: String::new(),
            open: o,
            high: o + 4.0,
            low: o - 3.0,
            close: o + 1.0,
            volume: 200.0,
        });
    }

    let mut ts_scale = TimeScale::new(HorzScaleOptions::sumi_default());
    ts_scale.set_width(1_000.0);
    ts_scale.set_total_bars(100);
    ts_scale.fit_content();
    let mut ps_scale = PriceScale::new("main", PriceScaleOptions::sumi_main_price());
    ps_scale.set_height(600.0);
    ps_scale.set_auto_scale(false);
    ps_scale.set_raw_price_range(950.0, 1_250.0, 1_000.0);
    let proj = PaneDrawingProjection::new(&ts_scale, &ps_scale, 1_000.0, 600.0, 1_000.0);

    // 1. `[CRITICAL-01]` Scenario A & B: Future Fib deleted before replay (`Undo` during replay)
    //    + Fib created at bar 80 (`Undo` -> rewind to bar 40 -> `Redo`) MUST NEVER stay selected!
    let mut store = DrawingStore::new();
    let live_future_fib_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::FibonacciRetracement {
                start: DrawingAnchor::with_timestamp(80.0, 1_100.0, base[80].timestamp as f64),
                end: DrawingAnchor::with_timestamp(92.0, 1_150.0, base[92].timestamp as f64),
                direction: FibDirection::StartToEnd,
                extend_right: false,
            },
            None,
        )
        .unwrap();
    store.selected_id = Some(live_future_fib_id);
    assert!(store.delete_selected());

    // Enter replay at bar 80, author a replay Fib (`created_at_timestamp = T_80`), then `Undo` it:
    store.set_replay_cutoff_timestamp(Some(base[80].timestamp), 81);
    store.active_tool = DrawingTool::FibonacciRetracement;
    let p65 = proj
        .anchor_to_local(DrawingAnchor::new(65.0, 1_090.0))
        .unwrap();
    let p78 = proj
        .anchor_to_local(DrawingAnchor::new(78.0, 1_130.0))
        .unwrap();
    let _ = store.pointer_down(DrawingPaneTarget::Main, p65, false, &proj, &base[..=80]);
    store.pointer_move(DrawingPaneTarget::Main, p78, false, &proj, &base[..=80]);
    let _ = store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p78),
        false,
        &proj,
        &base[..=80],
    );
    assert!(store.undo(), "undo replay Fib at T_80");

    // Rewind to bar 40 (`T_40 < T_80`):
    store.set_replay_cutoff_timestamp(Some(base[40].timestamp), 41);
    // Redo the T_80 Fib -> must be inserted into store, but NOT selected (`DEF-001`)!
    assert!(store.redo());
    assert!(store.selected_drawing().is_none());
    assert_eq!(store.selected_id, None);
    // Undo twice so the pre-replay deleted future Fib (`bar 80..92`) is restored at T_40 -> also NOT selected!
    assert!(store.undo());
    assert!(store.undo());
    assert!(store.selected_drawing().is_none());
    assert_eq!(store.selected_id, None);

    // `has_unlocked_drawings()` must NOT leak hidden future unlocked drawings, and `clear_unlocked()`
    // must NOT wipe hidden future drawings!
    assert!(
        !store.has_unlocked_drawings(),
        "only future hidden drawing exists at T_40; has_unlocked_drawings must be false"
    );
    assert_eq!(store.clear_unlocked(), 0);
    assert_eq!(
        store.drawings.len(),
        1,
        "hidden future drawing must survive clear_unlocked during earlier replay"
    );

    // 2. `[HIGH-01]`: Dragging a pre-replay drawing (`created_at_timestamp == None`) into right-edge
    //    whitespace (`bar_index = 30.0 > 24.0`) at `bar 24` must keep it visible mid-drag!
    store.set_replay_cutoff_timestamp(None, 100);
    let pre_tl_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::with_timestamp(10.0, 1_015.0, base[10].timestamp as f64),
                end: DrawingAnchor::with_timestamp(20.0, 1_030.0, base[20].timestamp as f64),
            },
            None,
        )
        .unwrap();
    store.set_replay_cutoff_timestamp(Some(base[24].timestamp), 25);
    ts_scale.set_total_bars(25);
    ts_scale.set_bar_spacing(10.0);
    ts_scale.set_right_offset(12.0);
    let proj_24 = PaneDrawingProjection::new(&ts_scale, &ps_scale, 1_000.0, 600.0, 1_000.0);
    store.active_tool = DrawingTool::Select;
    let p_end_20 = proj_24
        .anchor_to_local(DrawingAnchor::new(20.0, 1_030.0))
        .unwrap();
    let p_ws_30 = proj_24
        .anchor_to_local(DrawingAnchor::new(30.0, 1_045.0))
        .unwrap();
    let _ = store.pointer_down(
        DrawingPaneTarget::Main,
        p_end_20,
        false,
        &proj_24,
        &base[..=24],
    );
    store.pointer_move(
        DrawingPaneTarget::Main,
        p_ws_30,
        false,
        &proj_24,
        &base[..=24],
    );
    let edited_tl = store.drawings.iter().find(|d| d.id == pre_tl_id).unwrap();
    assert!(
        edited_tl.is_causally_visible(Some(base[24].timestamp), 25),
        "pre-replay drawing dragged into right whitespace must remain causally visible mid-drag"
    );
    let _ = store.pointer_up(
        DrawingPaneTarget::Main,
        Some(p_ws_30),
        false,
        &proj_24,
        &base[..=24],
    );

    // 3. `[MEDIUM-03]`: Step forward to bar 25 (across overnight gap) and backward to bar 24:
    //    in-place `remap_timeframe` must NOT bake the overnight gap into `end.timestamp`!
    store.remap_timeframe(&base[..=25], Timeframe::M1, &base[..=25], Timeframe::M1);
    let c_5m_at_24 = aggregate_ohlcv(&base[..=24], Timeframe::M1, Timeframe::M5).unwrap();
    store.remap_timeframe(&base[..=24], Timeframe::M1, &c_5m_at_24, Timeframe::M5);
    let remapped_tl = store.drawings.iter().find(|d| d.id == pre_tl_id).unwrap();
    let [_, end_5m] = remapped_tl.geometry.anchors();
    let last_5m_idx = (c_5m_at_24.len() - 1) as f64;
    assert!(
        end_5m.bar_index > last_5m_idx && end_5m.bar_index < last_5m_idx + 4.0,
        "whitespace anchor 6x1m ahead must map ~1.2..2.0 5m bars ahead, not +220 bars across overnight gap: {end_5m:?}"
    );
    assert_eq!(end_5m.timestamp, None);

    // 4. `[HIGH-02]`: Dense >32 drawing stack in `hit_test_pane_drawings_causal` must ignore masked top drawings!
    let mut dense = DrawingStore::new();
    let visible_bottom_id = dense
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::with_timestamp(5.0, 1_020.0, base[5].timestamp as f64),
            },
            None,
        )
        .unwrap();
    for i in 0..36 {
        let mut d = Drawing::new(
            100 + i,
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::with_timestamp(5.0, 1_020.0, base[5].timestamp as f64),
            },
            DrawingStyle::default_for_tool(DrawingTool::HorizontalLine),
        );
        // Make 34 visible decoys + 2 top-most future masked drawings (`created_at_timestamp > cutoff`):
        if i >= 34 {
            d.created_at_timestamp = Some(base[80].timestamp);
        } else {
            d.created_at_timestamp = Some(base[10].timestamp);
        }
        dense.drawings.push(d);
    }
    let cursor_1020 = proj_24
        .anchor_to_local(DrawingAnchor::new(5.0, 1_020.0))
        .unwrap();
    let winner = hit_test_pane_drawings_causal(
        &dense.drawings,
        DrawingPaneTarget::Main,
        &proj_24,
        cursor_1020,
        None,
        Some(base[24].timestamp),
        25,
    )
    .expect("must hit visible decoy, not future top drawing");
    assert_ne!(winner.drawing_id, 134);
    assert_ne!(winner.drawing_id, 135);
    assert!(winner.drawing_id == visible_bottom_id || (100..134).contains(&winner.drawing_id));
}
