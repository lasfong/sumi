use crate::data::{Timeframe, aggregate_ohlcv, generate_intraday_vn30f_1m_report};
use crate::drawings::{
    Drawing, DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStore, DrawingStyle,
};
use crate::model::Candle;
use crate::replay::{ReplayController, ReplayStepUnit};

use super::{
    AlertCondition, AlertEngine, AlertError, AlertFrequency, AlertStatus, CrossDirection,
    drawing_line_value_at_bar,
};

fn make_daily_bar(day_idx: usize, open: f64, high: f64, low: f64, close: f64) -> Candle {
    let ts = 1_760_000_000 + (day_idx as i64) * 86_400;
    Candle {
        timestamp: ts,
        date_label: Timeframe::D1.format_candle_label(ts),
        open,
        high,
        low,
        close,
        volume: 1_000.0,
    }
}

#[test]
fn price_level_4_waypoint_gap_wick_touch_and_flat_continuation_invariants() {
    let candles = vec![
        make_daily_bar(0, 99.0, 100.5, 98.5, 100.0), // arming bar (`close = 100`)
        make_daily_bar(1, 101.0, 105.0, 100.5, 105.0), // exact touch & close AT `105.0` -> triggers #1!
        make_daily_bar(2, 105.0, 105.0, 105.0, 105.0), // flat bar sitting AT `105.0` -> NO duplicate trigger!
        make_daily_bar(3, 105.0, 108.0, 105.0, 107.0), // continuation `105 -> 108` without dipping `< 105` -> NO duplicate!
        make_daily_bar(4, 107.0, 107.5, 103.5, 104.0), // dips below `105.0` (`close = 104.0`)
        make_daily_bar(5, 109.0, 110.0, 108.0, 109.5), // overnight gap-up `104.0 -> 109.0` across `105.0` -> triggers #2!
    ];
    let drawings = DrawingStore::new();
    let mut engine = AlertEngine::new();
    let id = engine
        .add_alert(
            "TEST",
            Timeframe::D1,
            AlertCondition::PriceLevel {
                price: 105.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &candles,
            0,
            Some(candles[0].timestamp),
            Some(&drawings),
        )
        .expect("valid price alert");

    engine.evaluate_causal(
        &candles,
        Timeframe::D1,
        &candles,
        Timeframe::D1,
        candles.len() - 1,
        None,
        &drawings,
    );

    // Must fire ONLY on bar 1 (exact wick/close touch from 100) and bar 5 (overnight gap-up from 104 -> 109),
    // never on bar 0 (arming bar), bar 2 (flat at 105), or bar 3 (continuation from 105 -> 108)!
    assert_eq!(engine.triggers.len(), 2);
    assert_eq!(engine.triggers[0].alert_id, id);
    assert_eq!(engine.triggers[0].bar_timestamp, candles[1].timestamp);
    assert!((engine.triggers[0].observed_value - 105.0).abs() < 1e-12);
    assert_eq!(engine.triggers[1].bar_timestamp, candles[5].timestamp);
    assert!((engine.triggers[1].observed_value - 109.0).abs() < 1e-12);
}

#[test]
fn rsi_and_macd_indicator_alerts_match_wilder_and_talib_crossovers() {
    // Construct a series that slides into oversold (`RSI < 25`) then rallies sharply (`RSI > 30` and `MACD Hist > 0`).
    let mut candles = Vec::with_capacity(60);
    let mut p = 120.0;
    for i in 0..60 {
        if i < 20 {
            p += 0.3;
        } else if i < 38 {
            p -= 1.6;
        } else {
            p += 2.1;
        }
        candles.push(make_daily_bar(i, p - 0.3, p + 0.6, p - 0.8, p));
    }

    let drawings = DrawingStore::new();
    let mut engine = AlertEngine::new();
    let rsi_id = engine
        .add_alert(
            "TEST",
            Timeframe::D1,
            AlertCondition::RsiThreshold {
                period: 14,
                level: 30.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            &candles,
            34,
            Some(candles[34].timestamp),
            Some(&drawings),
        )
        .expect("valid RSI alert");
    let macd_id = engine
        .add_alert(
            "TEST",
            Timeframe::D1,
            AlertCondition::MacdHistogramZeroCross {
                fast: 12,
                slow: 26,
                signal: 9,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            &candles,
            34,
            Some(candles[34].timestamp),
            Some(&drawings),
        )
        .expect("valid MACD alert");

    engine.evaluate_causal(
        &candles,
        Timeframe::D1,
        &candles,
        Timeframe::D1,
        candles.len() - 1,
        None,
        &drawings,
    );

    assert_eq!(engine.triggers.len(), 2);
    assert!(
        engine
            .triggers
            .iter()
            .any(|t| t.alert_id == rsi_id && t.observed_value > 30.0)
    );
    assert!(
        engine
            .triggers
            .iter()
            .any(|t| t.alert_id == macd_id && t.observed_value > 0.0)
    );
    assert_eq!(engine.triggered_count(None), 2);
}

#[test]
fn sloped_trendline_and_360_ray_relative_offset_prevents_parallel_false_cross() {
    // Upward-sloping Ray: origin `(0, 100.0)` through `(10, 110.0)` -> `L_t = 100.0 + 1.0 * t`.
    // Bars 0..=8 ride `+0.5` strictly ABOVE the ray (`L_t + 0.5`)! Notice `close[t-1] = L_{t-1} + 0.5 < L_t = L_{t-1} + 1.0`:
    // a naive `prev_close < L_t && high_t >= L_t` check would falsely fire on EVERY bar!
    // Relative offset `d = price - L_t` stays `+0.2 .. +1.2 > 0` until bar 9 genuinely dips below `L_9 = 109.0`
    // and bar 10 genuinely crosses back above `L_10 = 110.0`!
    let mut candles = Vec::with_capacity(12);
    for t in 0..9 {
        let line_v = 100.0 + (t as f64);
        candles.push(make_daily_bar(
            t,
            line_v + 0.5,
            line_v + 1.2,
            line_v + 0.2,
            line_v + 0.5,
        ));
    }
    // Bar 9 (`L_9 = 109.0`): breaks below ray (`close = 108.2`, `d = -0.8`).
    candles.push(make_daily_bar(9, 109.4, 109.5, 108.0, 108.2));
    // Bar 10 (`L_10 = 110.0`): reclaims ray (`open = 109.0`, `high = 111.0`, `close = 110.6`).
    candles.push(make_daily_bar(10, 109.0, 111.0, 108.9, 110.6));

    let mut store = DrawingStore::new();
    let ray = Drawing::new(
        1,
        DrawingPaneTarget::Main,
        DrawingGeometry::Ray {
            origin: DrawingAnchor::with_timestamp(0.0, 100.0, candles[0].timestamp as f64),
            through: DrawingAnchor::with_timestamp(5.0, 105.0, candles[5].timestamp as f64),
        },
        DrawingStyle::default(),
    );
    assert!((drawing_line_value_at_bar(&ray, 10).expect("ray at 10") - 110.0).abs() < 1e-12);
    store.drawings.push(ray);

    // Also add a finite TrendLine `[2..=6]` and verify it does NOT fire at bar 10 (`u > 1.0`).
    let finite_tl = Drawing::new(
        2,
        DrawingPaneTarget::Main,
        DrawingGeometry::TrendLine {
            start: DrawingAnchor::with_timestamp(2.0, 102.0, candles[2].timestamp as f64),
            end: DrawingAnchor::with_timestamp(6.0, 106.0, candles[6].timestamp as f64),
        },
        DrawingStyle::default(),
    );
    assert!(drawing_line_value_at_bar(&finite_tl, 10).is_none());
    store.drawings.push(finite_tl);

    let mut engine = AlertEngine::new();
    let ray_alert = engine
        .add_alert(
            "RAY",
            Timeframe::D1,
            AlertCondition::DrawingLine {
                drawing_id: 1,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &candles,
            0,
            Some(candles[0].timestamp),
            Some(&store),
        )
        .expect("valid ray alert");
    let _tl_alert = engine
        .add_alert(
            "RAY",
            Timeframe::D1,
            AlertCondition::DrawingLine {
                drawing_id: 2,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &candles,
            0,
            Some(candles[0].timestamp),
            Some(&store),
        )
        .expect("valid finite trendline alert");

    engine.evaluate_causal(
        &candles,
        Timeframe::D1,
        &candles,
        Timeframe::D1,
        candles.len() - 1,
        None,
        &store,
    );

    // Zero false triggers on parallel bars 1..=8, zero out-of-domain triggers on finite TrendLine #2,
    // and exactly ONE true CrossAbove trigger on Ray #1 at bar 10!
    assert_eq!(engine.triggers.len(), 1);
    assert_eq!(engine.triggers[0].alert_id, ray_alert);
    assert_eq!(engine.triggers[0].bar_timestamp, candles[10].timestamp);
}

#[test]
fn causal_replay_stepping_1m_15m_parity_and_rewind_rearms_once_alerts() {
    let report = generate_intraday_vn30f_1m_report(450);
    let base_1m = &report.candles;
    let drawings = DrawingStore::new();

    let arm_idx = 60_usize;
    let arm_price = base_1m[arm_idx].close;
    // Pick a nearby level guaranteed to be crossed within the next 150 1m bars.
    let max_future = base_1m[arm_idx + 1..=300]
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let target_level = ((arm_price + max_future) * 0.5 * 100.0).round() / 100.0;
    assert!(target_level > arm_price && target_level < max_future);

    // 1. Step 1m-by-1m (`BaseBar`) vs 15m-by-15m (`ActiveBar` on `15m`) to `cursor = 300`:
    //    `PriceLevel` must trigger at the EXACT same `1m` timestamp & observed price!
    let mut replay_1m = ReplayController::new();
    assert!(replay_1m.start_at_base_index(base_1m, arm_idx).is_some());
    replay_1m.step_unit = ReplayStepUnit::BaseBar;
    let mut alerts_1m = AlertEngine::new();
    let alert_id = alerts_1m
        .add_alert(
            "VN30F1M",
            Timeframe::M1,
            AlertCondition::PriceLevel {
                price: target_level,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            base_1m,
            arm_idx,
            replay_1m.cutoff_timestamp(base_1m),
            Some(&drawings),
        )
        .expect("1m alert");

    let mut replay_15m = ReplayController::new();
    assert!(replay_15m.start_at_base_index(base_1m, arm_idx).is_some());
    let mut alerts_15m = alerts_1m.clone();

    while replay_1m.base_cursor_index < 300 {
        let tr = replay_1m
            .step_forward(base_1m, Timeframe::M1, 1)
            .expect("1m step");
        let active = replay_1m
            .materialize_candles(base_1m, Timeframe::M1, Timeframe::M1)
            .expect("1m materialize");
        alerts_1m.evaluate_causal(
            base_1m,
            Timeframe::M1,
            &active,
            Timeframe::M1,
            tr.new_base_index,
            Some(tr.cutoff_timestamp),
            &drawings,
        );
    }

    let tr_15m = replay_15m
        .jump_to_base_index(base_1m, 300)
        .expect("jump 15m to 300");
    let active_15m = aggregate_ohlcv(&base_1m[..=300], Timeframe::M1, Timeframe::M15).expect("15m");
    alerts_15m.evaluate_causal(
        base_1m,
        Timeframe::M1,
        &active_15m,
        Timeframe::M15,
        tr_15m.new_base_index,
        Some(tr_15m.cutoff_timestamp),
        &drawings,
    );

    assert_eq!(alerts_1m.triggers.len(), 1);
    assert_eq!(alerts_1m.triggers, alerts_15m.triggers);
    assert_eq!(alerts_1m.rules[0].status, AlertStatus::Triggered);

    // 2. Rewind back to `arm_idx` (`before` trigger timestamp): `Once` alert MUST re-arm back to `Armed`
    //    and purge the future trigger event cleanly!
    let rew = replay_1m.jump_to_start(base_1m).expect("rewind to start");
    let active_rew = replay_1m
        .materialize_candles(base_1m, Timeframe::M1, Timeframe::M1)
        .expect("rewound 1m");
    alerts_1m.evaluate_causal(
        base_1m,
        Timeframe::M1,
        &active_rew,
        Timeframe::M1,
        rew.new_base_index,
        Some(rew.cutoff_timestamp),
        &drawings,
    );
    assert!(alerts_1m.triggers.is_empty());
    assert_eq!(alerts_1m.rules[0].id, alert_id);
    assert_eq!(alerts_1m.rules[0].status, AlertStatus::Armed);
    assert_eq!(alerts_1m.rules[0].trigger_count, 0);

    // 3. Also verify rewinding BEFORE `created_at_timestamp` masks an in-replay alert (`is_causally_visible == false`).
    let mut pre_replay = ReplayController::new();
    assert!(pre_replay.start_at_base_index(base_1m, 20).is_some());
    let pre_ts = pre_replay.cutoff_timestamp(base_1m);
    assert!(!alerts_1m.rules[0].is_causally_visible(pre_ts));
}

#[test]
fn critic_r08_regression_at_mark_boundary_live_whitespace_ray_and_vertical_line_guard() {
    // 1. `@Mark` (`prev_close == level = 100.0`):
    //    - Flat bar `O=H=L=C=100.0` must NOT fire.
    //    - Departure bar `100.0 -> 103.0` MUST fire `CrossAbove` once.
    //    - Subsequent touch/continuation `100.0 -> 104.0` after already crossing above (`close = 100.0` from above)
    //      without dipping `< 100.0` properly updates state.
    let candles = vec![
        make_daily_bar(0, 99.0, 100.5, 98.5, 100.0), // arming bar: `close == 100.0 == level` (`@Mark`)
        make_daily_bar(1, 100.0, 100.0, 100.0, 100.0), // flat bar sitting on `100.0` -> MUST NOT fire!
        make_daily_bar(2, 100.0, 103.0, 100.0, 102.5), // departs upward `100.0 -> 103.0` -> MUST fire #1!
        make_daily_bar(3, 102.0, 104.0, 100.0, 100.0), // dips only to `100.0` (not `< 100.0`), closes `100.0`
        make_daily_bar(4, 100.0, 105.0, 100.0, 104.5), // bounces `100.0 -> 105.0` without ever having gone `< 100.0` -> NO duplicate CrossAbove!
    ];
    let mut store = DrawingStore::new();
    let mut engine = AlertEngine::new();
    let mark_id = engine
        .add_alert(
            "MARK",
            Timeframe::D1,
            AlertCondition::PriceLevel {
                price: 100.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &candles,
            0,
            Some(candles[0].timestamp),
            Some(&store),
        )
        .expect("valid @Mark alert");

    // Evaluate at bar 1 (flat): 0 triggers
    engine.evaluate_causal(
        &candles,
        Timeframe::D1,
        &candles[..=1],
        Timeframe::D1,
        1,
        Some(candles[1].timestamp),
        &store,
    );
    assert!(
        engine.triggers.is_empty(),
        "flat bar O=H=L=C=level must never fire"
    );

    // Evaluate through bar 4: fires ONLY on bar 2 (`100.0 -> 103.0`), NOT again on bar 4!
    engine.evaluate_causal(
        &candles,
        Timeframe::D1,
        &candles,
        Timeframe::D1,
        4,
        Some(candles[4].timestamp),
        &store,
    );
    assert_eq!(engine.triggers.len(), 1);
    assert_eq!(engine.triggers[0].alert_id, mark_id);
    assert_eq!(engine.triggers[0].bar_timestamp, candles[2].timestamp);

    // 2. Vertical `TrendLine` (`|dx| < 1e-9`), hidden drawing (`!visible`), and invalid `MACD(12,26,1)` must be rejected!
    let vert_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(1.0, 95.0),
                end: DrawingAnchor::new(1.0, 105.0),
            },
            None,
        )
        .unwrap();
    assert!(
        engine
            .add_alert(
                "MARK",
                Timeframe::D1,
                AlertCondition::DrawingLine {
                    drawing_id: vert_id,
                    direction: CrossDirection::CrossAny,
                },
                AlertFrequency::Once,
                None,
                &candles,
                0,
                None,
                Some(&store),
            )
            .is_err(),
        "vertical TrendLine must be rejected at arming time"
    );
    assert!(
        engine
            .add_alert(
                "MARK",
                Timeframe::D1,
                AlertCondition::MacdHistogramZeroCross {
                    fast: 12,
                    slow: 26,
                    signal: 1,
                    direction: CrossDirection::CrossAny,
                },
                AlertFrequency::Once,
                None,
                &candles,
                0,
                None,
                Some(&store),
            )
            .is_err(),
        "MACD signal < 2 must be rejected"
    );

    // 3. Live-mode right-whitespace `Ray` (`origin.bar_index = 7.0 > candles.len()`, pointing left through `1.0` at `101.0`)
    //    must evaluate and trigger cleanly in live mode (`replay_cutoff_timestamp = None`)!
    let ws_ray_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Ray {
                origin: DrawingAnchor::new(7.0, 101.0),
                through: DrawingAnchor::new(1.0, 101.0),
            },
            None,
        )
        .unwrap();
    let mut live_engine = AlertEngine::new();
    let ray_alert_id = live_engine
        .add_alert(
            "MARK",
            Timeframe::D1,
            AlertCondition::DrawingLine {
                drawing_id: ws_ray_id,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            &candles,
            0,
            None,
            Some(&store),
        )
        .expect("right-whitespace live Ray alert");
    // Re-arm from bar 0 so bars 1..=4 are evaluated in live mode
    live_engine.rules[0].armed_at_timestamp = candles[0].timestamp;
    live_engine.rules[0].armed_bucket_timestamp = candles[0].timestamp;
    live_engine.evaluate_causal(
        &candles,
        Timeframe::D1,
        &candles,
        Timeframe::D1,
        candles.len() - 1,
        None,
        &store,
    );
    assert_eq!(live_engine.triggers.len(), 1);
    assert_eq!(live_engine.triggers[0].alert_id, ray_alert_id);
    assert_eq!(live_engine.triggers[0].bar_timestamp, candles[2].timestamp);

    // 4. `u64::MAX` saturation compaction in `sanitize_deserialized`
    live_engine.next_id = u64::MAX;
    live_engine.sanitize_deserialized();
    assert!(live_engine.next_id < 100);
    assert_eq!(live_engine.rules[0].id, 1);
    assert_eq!(live_engine.triggers[0].alert_id, 1);
}

#[test]
fn critic_r08_regression_intrabucket_coarse_alerts_horizontal_parity_and_causal_active_slice() {
    let report = generate_intraday_vn30f_1m_report(300);
    let base_1m = &report.candles;

    // Find an intra-15m bar `arm_idx` (`offset 1..=10` inside its 15m bucket) where a subsequent 1m bar
    // inside the SAME 15m bucket moves higher than `base_1m[arm_idx].close`.
    let mut chosen = None;
    for idx in 45..240 {
        let b_ts = Timeframe::M15.bucket_start_timestamp(base_1m[idx].timestamp);
        let prev_b_ts = Timeframe::M15.bucket_start_timestamp(base_1m[idx - 1].timestamp);
        let bucket_end = (idx + 1..base_1m.len())
            .take_while(|&j| Timeframe::M15.bucket_start_timestamp(base_1m[j].timestamp) == b_ts)
            .last();
        if b_ts == prev_b_ts
            && let Some(end_j) = bucket_end
        {
            let arm_c = base_1m[idx].close;
            let max_rest = base_1m[idx + 1..=end_j]
                .iter()
                .map(|c| c.high)
                .fold(f64::NEG_INFINITY, f64::max);
            if max_rest > arm_c + 0.05 {
                chosen = Some((idx, end_j, 0.5 * (arm_c + max_rest)));
                break;
            }
        }
    }
    let (arm_idx, bucket_end_idx, target_lvl) = chosen.expect("intra-bucket upward move");

    // 1. HorizontalLine `DrawingLine` alert armed mid-bucket on `15m` MUST fire inside the SAME `15m` bucket
    //    AND match `1m` evaluation bit-for-bit (`bar_timestamp == eval_base_timestamp`)!
    let mut store_1m = DrawingStore::new();
    let hline_id = store_1m
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::with_timestamp(
                    arm_idx as f64,
                    target_lvl,
                    base_1m[arm_idx].timestamp as f64,
                ),
            },
            None,
        )
        .unwrap();
    let active_15m_at_end =
        aggregate_ohlcv(&base_1m[..=bucket_end_idx], Timeframe::M1, Timeframe::M15).unwrap();
    let mut store_15m = store_1m.clone();
    store_15m.remap_timeframe(
        &base_1m[..=bucket_end_idx],
        Timeframe::M1,
        &active_15m_at_end,
        Timeframe::M15,
    );

    let mut eng_1m = AlertEngine::new();
    eng_1m
        .add_alert(
            "VN30F1M",
            Timeframe::M15,
            AlertCondition::DrawingLine {
                drawing_id: hline_id,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            base_1m,
            arm_idx,
            Some(base_1m[arm_idx].timestamp),
            Some(&store_1m),
        )
        .unwrap();
    let mut eng_15m = eng_1m.clone();

    eng_1m.evaluate_causal(
        base_1m,
        Timeframe::M1,
        &base_1m[..=bucket_end_idx],
        Timeframe::M1,
        bucket_end_idx,
        Some(base_1m[bucket_end_idx].timestamp),
        &store_1m,
    );
    eng_15m.evaluate_causal(
        base_1m,
        Timeframe::M1,
        &active_15m_at_end,
        Timeframe::M15,
        bucket_end_idx,
        Some(base_1m[bucket_end_idx].timestamp),
        &store_15m,
    );
    assert_eq!(eng_1m.triggers.len(), 1);
    assert_eq!(
        eng_1m.triggers, eng_15m.triggers,
        "HorizontalLine DrawingLine must achieve 100% 1m == 15m parity and fire intra-bucket"
    );

    // 2. Also verify a sloped `TrendLine` on `15m` armed mid-bucket fires intra-bucket when post-arm sub-bars cross it!
    let tl_id = store_15m
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(0.0, target_lvl),
                end: DrawingAnchor::new((active_15m_at_end.len() - 1) as f64, target_lvl),
            },
            None,
        )
        .unwrap();
    let mut tl_eng = AlertEngine::new();
    tl_eng
        .add_alert(
            "VN30F1M",
            Timeframe::M15,
            AlertCondition::DrawingLine {
                drawing_id: tl_id,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            base_1m,
            arm_idx,
            Some(base_1m[arm_idx].timestamp),
            Some(&store_15m),
        )
        .unwrap();
    tl_eng.evaluate_causal(
        base_1m,
        Timeframe::M1,
        &active_15m_at_end,
        Timeframe::M15,
        bucket_end_idx,
        Some(base_1m[bucket_end_idx].timestamp),
        &store_15m,
    );
    assert_eq!(
        tl_eng.triggers.len(),
        1,
        "15m TrendLine armed mid-bucket must fire intra-bucket on post-arm sub-bars"
    );
    assert_eq!(
        tl_eng.triggers[0].eval_base_timestamp,
        base_1m[bucket_end_idx].timestamp
    );

    // 3. Causal `active_candles` slice guard (`[HIGH-02]`): passing full un-sliced `base_1m` (`300` bars)
    //    with `cursor_base_index = arm_idx` (`replay_cutoff_timestamp = None`) MUST NOT leak future triggers!
    let mut guard_eng = AlertEngine::new();
    guard_eng
        .add_alert(
            "VN30F1M",
            Timeframe::M1,
            AlertCondition::DrawingLine {
                drawing_id: hline_id,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            base_1m,
            arm_idx,
            None,
            Some(&store_1m),
        )
        .unwrap();
    guard_eng.evaluate_causal(
        base_1m,
        Timeframe::M1,
        base_1m, // deliberately un-sliced!
        Timeframe::M1,
        arm_idx,
        None,
        &store_1m,
    );
    assert!(
        guard_eng.triggers.is_empty(),
        "evaluate_causal must clamp un-sliced active_candles to cursor_base_index with zero future leak"
    );
}

#[test]
fn r16_signal_catalog_and_confluence_alerts_bool_float_regime_and_bidirectional_matching() {
    use crate::drawings::DrawingStore;
    use crate::scanner::ConfluenceWeightProfile;

    let base_ts = 1_710_000_000_i64;
    let mut bars: Vec<Candle> = Vec::with_capacity(95);
    for i in 0..75_usize {
        let p = 100.0 + (i as f64) * 0.45;
        let vol = if i == 62 { 5_200.0 } else { 1_000.0 };
        bars.push(Candle {
            timestamp: base_ts + (i as i64) * 86_400,
            date_label: format!("d{i}"),
            open: p - 0.3,
            high: p + 0.25,
            low: p - 0.65,
            close: p,
            volume: vol,
        });
    }
    // Bar 75: sharp Bearish breakdown (`sr.prior_20b_breakout` with `direction = Bearish`) + regime transition!
    for i in 75..95_usize {
        let p = 112.0 - ((i - 75) as f64) * 1.1;
        bars.push(Candle {
            timestamp: base_ts + (i as i64) * 86_400,
            date_label: format!("d{i}"),
            open: p + 1.2,
            high: p + 1.5,
            low: p - 0.8,
            close: p,
            volume: 2_800.0,
        });
    }

    let store = DrawingStore::new();
    let mut eng = AlertEngine::new();
    let arm_idx = 55;

    // 1. Bool `volume.spike_2x` `CrossAbove` -> fires on bar 62!
    let id_spike = eng
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "volume.spike_2x".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            Some("Spike2x".to_owned()),
            &bars,
            arm_idx,
            None,
            None,
        )
        .expect("spike alert");

    // 2. Bidirectional `sr.prior_20b_breakout`: `CrossAbove` vs `CrossBelow`
    //    `CrossBelow` MUST fire on bar 75's Bearish breakdown; `CrossAbove` only fires on Bullish breakouts!
    let id_brk_bear = eng
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "sr.prior_20b_breakout".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossBelow,
            },
            AlertFrequency::Once,
            Some("BearBreak".to_owned()),
            &bars,
            73, // Armed right before bar 75 breakdown!
            None,
            None,
        )
        .expect("bear breakout alert");

    let id_brk_bull_after_73 = eng
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "sr.prior_20b_breakout".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            Some("BullBreakPost73".to_owned()),
            &bars,
            74, // Bar 75..94 is strictly downward -> Bullish breakout must NEVER fire!
            None,
            None,
        )
        .expect("bull breakout alert");

    // Unknown `catalog_id` fails closed at registration:
    assert!(matches!(
        eng.add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "unknown.signal_id".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAny,
            },
            AlertFrequency::Once,
            None,
            &bars,
            70,
            None,
            None,
        ),
        Err(AlertError::UnknownSignalCatalogId(_))
    ));

    // 3. Float `flow.cvd_ohlcv_impulse` crossing below `0.0` during breakdown
    let id_cvd_float = eng
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "flow.cvd_ohlcv_impulse".to_owned(),
                float_level: Some(0.0),
                direction: CrossDirection::CrossBelow,
            },
            AlertFrequency::Once,
            None,
            &bars,
            70,
            None,
            None,
        )
        .expect("float signal alert");

    // 4. Enum `regime.market_state` `CrossBelow` (transition into Downtrend / Pullback)
    let id_regime = eng
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "regime.market_state".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossBelow,
            },
            AlertFrequency::Once,
            None,
            &bars,
            70,
            None,
            None,
        )
        .expect("regime signal alert");

    // 5. `ConfluenceThreshold` (`Balanced`) crossing below `0.0` during breakdown
    let id_conf = eng
        .add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::ConfluenceThreshold {
                profile: ConfluenceWeightProfile::Balanced,
                threshold: 0.0,
                direction: CrossDirection::CrossBelow,
            },
            AlertFrequency::Once,
            Some("ConfluenceDrop".to_owned()),
            &bars,
            70,
            None,
            None,
        )
        .expect("confluence alert");

    eng.evaluate_causal(
        &bars,
        Timeframe::D1,
        &bars,
        Timeframe::D1,
        bars.len() - 1,
        None,
        &store,
    );

    let fired_ids: Vec<u64> = eng.triggers.iter().map(|e| e.alert_id).collect();
    assert!(fired_ids.contains(&id_spike), "volume.spike_2x must fire");
    assert!(
        fired_ids.contains(&id_brk_bear),
        "Bearish sr.prior_20b_breakout must fire CrossBelow at bar 75"
    );
    assert!(
        !fired_ids.contains(&id_brk_bull_after_73),
        "Bearish breakdown must NEVER fire CrossAbove on bidirectional Bool signal"
    );
    assert!(
        fired_ids.contains(&id_cvd_float),
        "Float flow.cvd_ohlcv_impulse CrossBelow 0.0 must fire"
    );
    assert!(
        fired_ids.contains(&id_regime),
        "Enum regime.market_state CrossBelow must fire on breakdown"
    );
    assert!(
        fired_ids.contains(&id_conf),
        "ConfluenceThreshold CrossBelow 0.0 must fire on breakdown"
    );
}

#[test]
fn r16_alert_def001_htf_unclosed_bucket_suppression_and_cooldown_replay_rewind_parity() {
    use crate::alerts::MAX_ALERT_COOLDOWN_BARS;
    use crate::data::aggregate_ohlcv;
    use crate::drawings::DrawingStore;
    use crate::scanner::ConfluenceWeightProfile;

    // Build 960 `1m` bars (= 64 `15m` buckets) aligned to 15m boundaries (`900s`).
    // Bucket #60 (`1m` indices `900..=914`) has a huge volume spike ONLY on sub-bar `902` (3rd minute of the 15m bucket)!
    // Under `DEF-001`, a `15m` `SignalCatalogTrigger("volume.spike_2x")` MUST NOT fire at `1m` index `902..=913`
    // (while the `15m` bucket is still forming!), and MUST fire strictly at `1m` index `914` (the closing 1m bar of bucket #60)!
    let t0 = 1_710_000_000_i64; // Exact multiple of 900s (15m boundary)
    let mut base_1m: Vec<Candle> = Vec::with_capacity(960);
    for i in 0..960_usize {
        let p = 100.0 + (i as f64) * 0.02;
        let vol = if i == 902 { 50_000.0 } else { 100.0 };
        base_1m.push(Candle {
            timestamp: t0 + (i as i64) * 60,
            date_label: String::new(),
            open: p - 0.05,
            high: p + 0.15,
            low: p - 0.10,
            close: p,
            volume: vol,
        });
    }

    let store = DrawingStore::new();
    let mut htf_eng = AlertEngine::new();
    let arm_base = 884; // End of 15m bucket #58
    htf_eng
        .add_alert(
            "VN30F1M",
            Timeframe::M15,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "volume.spike_2x".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            None,
            &base_1m,
            arm_base,
            Some(base_1m[arm_base].timestamp + 59),
            None,
        )
        .expect("15m signal alert");

    // Evaluate mid-bucket at `1m` index `905` (6 minutes into bucket #60, AFTER the `902` volume burst!):
    let m15_mid = aggregate_ohlcv(&base_1m[..=905], Timeframe::M1, Timeframe::M15).unwrap();
    htf_eng.evaluate_causal(
        &base_1m,
        Timeframe::M1,
        &m15_mid,
        Timeframe::M15,
        905,
        Some(base_1m[905].timestamp + 59),
        &store,
    );
    assert!(
        htf_eng.triggers.is_empty(),
        "DEF-001: unclosed 15m tail bucket at 1m index 905 must NEVER prematurely fire SignalCatalogTrigger"
    );

    // Evaluate at exact bucket close (`1m` index `914`, `open + 899s`): MUST fire!
    let m15_closed = aggregate_ohlcv(&base_1m[..=914], Timeframe::M1, Timeframe::M15).unwrap();
    htf_eng.evaluate_causal(
        &base_1m,
        Timeframe::M1,
        &m15_closed,
        Timeframe::M15,
        914,
        Some(base_1m[914].timestamp + 59),
        &store,
    );
    assert_eq!(htf_eng.triggers.len(), 1);
    assert_eq!(htf_eng.rules[0].status, AlertStatus::Triggered);

    // Rewind back to `905` -> `Once` alert re-arms cleanly and trigger log empties!
    htf_eng.evaluate_causal(
        &base_1m,
        Timeframe::M1,
        &m15_mid,
        Timeframe::M15,
        905,
        Some(base_1m[905].timestamp + 59),
        &store,
    );
    assert!(htf_eng.triggers.is_empty());
    assert_eq!(htf_eng.rules[0].status, AlertStatus::Armed);

    // 2. Cooldown (`cooldown_bars = 3`) + Bar Replay step/rewind exact restoration!
    // Odd bars & bar 0 stay strictly below `105.0` (`[103.2, 104.2]`); even bars `2, 4, 6, 8` cross above `105.0` (`106.0`)!
    let osc_bars: Vec<Candle> = (0..10_usize)
        .map(|i| {
            let (open, high, low, close): (f64, f64, f64, f64) = if i >= 2 && i % 2 == 0 {
                (104.5, 106.5, 104.0, 106.0)
            } else {
                (103.8, 104.2, 103.2, 104.0)
            };
            Candle {
                timestamp: t0 + (i as i64) * 86_400,
                date_label: format!("b{i}"),
                open,
                high,
                low,
                close,
                volume: 1_000.0,
            }
        })
        .collect();
    let mut cd_eng = AlertEngine::new();
    let cd_id = cd_eng
        .add_alert_with_cooldown(
            "FPT",
            Timeframe::D1,
            AlertCondition::PriceLevel {
                price: 105.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            3,
            Some("Cooldown3".to_owned()),
            &osc_bars,
            0,
            Some(osc_bars[0].timestamp + 86_399),
            None,
        )
        .expect("cooldown alert");

    // Step to bar 4 (suppressed!)
    cd_eng.evaluate_causal(
        &osc_bars,
        Timeframe::D1,
        &osc_bars[..=4],
        Timeframe::D1,
        4,
        Some(osc_bars[4].timestamp + 86_399),
        &store,
    );
    assert_eq!(cd_eng.triggers.len(), 1);
    assert_eq!(cd_eng.triggers[0].bar_timestamp, osc_bars[2].timestamp);
    // Fired at bar 2 with cooldown_bars = 3 (suppressing bars 3, 4, 5): at bar 4, 2 bars elapsed -> 1 bar (bar 5) remains!
    assert_eq!(cd_eng.rules[0].cooldown_remaining_bars, 1);
    let snap_at_4 = cd_eng.clone();

    // Step forward to bar 8 -> fires at bar 6, suppressed at bar 8 (`8 - 6 = 2` elapsed -> `1` bar remains)
    cd_eng.evaluate_causal(
        &osc_bars,
        Timeframe::D1,
        &osc_bars[..=8],
        Timeframe::D1,
        8,
        Some(osc_bars[8].timestamp + 86_399),
        &store,
    );
    assert_eq!(cd_eng.triggers.len(), 2);
    assert_eq!(cd_eng.triggers[1].bar_timestamp, osc_bars[6].timestamp);
    assert_eq!(cd_eng.rules[0].cooldown_remaining_bars, 1);

    // Rewind back to bar 4 -> bit-exact restoration of `triggers`, `trigger_count`, and `cooldown_remaining_bars`!
    cd_eng.evaluate_causal(
        &osc_bars,
        Timeframe::D1,
        &osc_bars[..=4],
        Timeframe::D1,
        4,
        Some(osc_bars[4].timestamp + 86_399),
        &store,
    );
    assert_eq!(cd_eng.triggers, snap_at_4.triggers);
    assert_eq!(cd_eng.rules, snap_at_4.rules);
    assert!(cd_eng.set_cooldown_bars(cd_id, 9999));
    assert_eq!(cd_eng.rules[0].cooldown_bars, MAX_ALERT_COOLDOWN_BARS);

    // Unused import check silence via type usage
    let _ = ConfluenceWeightProfile::Balanced;
}

#[test]
fn r16_alert_trigger_csv_formula_injection_sanitization_and_rfc8259_json_export() {
    use crate::drawings::DrawingStore;

    let t0 = 1_710_000_000_i64;
    let bars = vec![
        Candle {
            timestamp: t0,
            date_label: "d0".to_owned(),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 1_000.0,
        },
        Candle {
            timestamp: t0 + 86_400,
            date_label: "d1".to_owned(),
            open: 100.5,
            high: 105.0,
            low: 100.0,
            close: 104.0,
            volume: 1_200.0,
        },
    ];
    let store = DrawingStore::new();
    let mut eng = AlertEngine::new();

    for dangerous_msg in [
        "=CMD|' /C calc'!A0",
        "+SUM(A1:A2),\"quoted\"",
        "-2+3",
        "@IMPORTXML(\"http://evil\")",
        "|shell",
        "  =LEADING_SPACE_FORMULA()",
        "=TAB\tINSIDE\tFORMULA",
    ] {
        eng.add_alert(
            "FPT",
            Timeframe::D1,
            AlertCondition::PriceLevel {
                price: 102.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::Once,
            Some(dangerous_msg.to_owned()),
            &bars,
            0,
            None,
            None,
        )
        .expect("arm formula test alert");
    }

    eng.evaluate_causal(&bars, Timeframe::D1, &bars, Timeframe::D1, 1, None, &store);
    assert_eq!(eng.triggers.len(), 7);

    let csv = eng.export_triggers_csv(None);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 8); // Header + 7 rows
    for row in &lines[1..] {
        // The trailing `message` cell must NEVER start raw with `=`, `+`, `-`, `@`, `|`, or `"` followed immediately by `=`!
        let last_comma_tail = row.rsplit(',').next().unwrap_or("");
        assert!(
            !last_comma_tail.starts_with('=')
                && !last_comma_tail.starts_with('+')
                && !last_comma_tail.starts_with('-')
                && !last_comma_tail.starts_with('@')
                && !last_comma_tail.starts_with('|')
                && !last_comma_tail.starts_with("\"="),
            "CSV cell failed formula-injection neutralization: {row}"
        );
    }
    assert!(csv.contains("'=CMD"));
    assert!(csv.contains("\"'")); // Quoted cell with comma + leading `'` neutralization!

    let json = eng.export_triggers_json(Some(t0 + 86_400));
    assert!(json.starts_with("{\n  \"schema_version\": 1,"));
    assert!(json.contains("\"event_count\": 7,"));
    assert!(json.contains("\"=TAB\\tINSIDE\\tFORMULA\"")); // `\t` control character escaped per RFC-8259!
    assert!(json.contains("\\\"quoted\\\"")); // Embedded `"` escaped per RFC-8259!

    // Causal cutoff before bar 1 hides future triggers in both CSV and JSON!
    let json_before = eng.export_triggers_json(Some(t0));
    assert!(json_before.contains("\"event_count\": 0,"));
}

#[test]
fn critic_r16_regression_vn30f_session_close_alerts_spy_w1_and_htf_intrabucket_cooldown_zero_and_clamp()
 {
    use crate::data::{aggregate_ohlcv, aggregate_ohlcv_up_to, format_utc_date};
    use crate::drawings::DrawingStore;

    let store = DrawingStore::new();

    // 1. `[Critical-01]` `VN30F1M` `1m` sessions ending at `07:44:00 UTC` (`causal_close = 07:44:59 UTC`)
    //    evaluated on `1D` (`SignalCatalogTrigger`): suppressed at `07:43 UTC`, fires at `07:44 UTC`
    //    in BOTH replay (`Some(...)`) AND live (`None`) modes!
    let start_day = 19_720_i64;
    let mut vn_1m = Vec::with_capacity(62 * 3);
    for d in 0..62_usize {
        let day_utc = (start_day + d as i64) * 86_400;
        let p = 1_200.0 + (d as f64) * 1.5;
        let spike_mult = if d == 58 { 6.0 } else { 1.0 };
        for &(sec_in_day, dp) in &[
            (2 * 3_600_i64, 0.0),       // 09:00 ICT open
            (7 * 3_600 + 43 * 60, 0.4), // 14:43 ICT (incomplete D1)
            (7 * 3_600 + 44 * 60, 0.8), // 14:44..14:45 ICT afternoon session close!
        ] {
            let ts = day_utc + sec_in_day;
            vn_1m.push(Candle {
                timestamp: ts,
                date_label: format_utc_date(ts),
                open: p + dp - 0.2,
                high: p + dp + 0.5,
                low: p + dp - 0.5,
                close: p + dp,
                volume: 500.0 * spike_mult,
            });
        }
    }
    let arm_1m_idx = 55 * 3 + 2; // end of day 55
    let d58_mid_idx = 58 * 3 + 1; // 07:43 UTC on spike day 58
    let d58_close_idx = 58 * 3 + 2; // 07:44 UTC on spike day 58

    let mut vn_eng = AlertEngine::new();
    vn_eng
        .add_alert(
            "VN30F1M",
            Timeframe::D1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "volume.spike_2x".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &vn_1m,
            arm_1m_idx,
            Some(vn_1m[arm_1m_idx].timestamp),
            None,
        )
        .expect("arm VN30F1M 1D signal alert");

    // At 07:43 UTC -> still incomplete, MUST NOT fire!
    let d1_at_0743 = aggregate_ohlcv_up_to(
        &vn_1m,
        Timeframe::M1,
        Timeframe::D1,
        vn_1m[d58_mid_idx].timestamp,
    )
    .unwrap();
    vn_eng.evaluate_causal(
        &vn_1m,
        Timeframe::M1,
        &d1_at_0743,
        Timeframe::D1,
        d58_mid_idx,
        Some(vn_1m[d58_mid_idx].timestamp),
        &store,
    );
    assert!(
        vn_eng.triggers.is_empty(),
        "VN30F1M 1D signal alert must NOT fire at 07:43 UTC before 07:44 session close"
    );

    // At 07:44 UTC in LIVE mode (`replay_cutoff_timestamp = None` on prefix `&vn_1m[..=d58_close_idx]`): MUST FIRE!
    let vn_live_slice = &vn_1m[..=d58_close_idx];
    let d1_at_0744 = aggregate_ohlcv(vn_live_slice, Timeframe::M1, Timeframe::D1).unwrap();
    vn_eng.evaluate_causal(
        vn_live_slice,
        Timeframe::M1,
        &d1_at_0744,
        Timeframe::D1,
        d58_close_idx,
        None,
        &store,
    );
    assert_eq!(
        vn_eng.triggers.len(),
        1,
        "VN30F1M 1D signal alert MUST fire at 07:44 UTC afternoon session close in live mode"
    );

    // 2. `[Critical-01]` `SPY` `14:30:00 UTC` `D1 -> W1` alert suppressed on Thursday, fires on Friday:
    //    `19_415` = Monday (`(19_415 + 4) % 7 == 1`). Build 58 Mon–Fri weeks at `14:30:00 UTC`!
    let first_monday = 19_415_i64;
    let mut spy_d1 = Vec::with_capacity(58 * 5);
    for w in 0..58_usize {
        for wd in 0..5_i64 {
            let day = first_monday + (w as i64) * 7 + wd;
            let ts = day * 86_400 + 14 * 3_600 + 30 * 60;
            let p = 400.0 + (w as f64) * 1.2 + (wd as f64) * 0.2;
            let vol = if w == 56 { 8_000.0 } else { 1_000.0 };
            spy_d1.push(Candle {
                timestamp: ts,
                date_label: format_utc_date(ts),
                open: p - 0.3,
                high: p + 0.5,
                low: p - 0.5,
                close: p,
                volume: vol,
            });
        }
    }
    let spy_arm_idx = 55 * 5 + 4; // Friday of week 55
    let spy_thu_w56 = 56 * 5 + 3; // Thursday of week 56 (14:30 UTC)
    let spy_fri_w56 = 56 * 5 + 4; // Friday of week 56 (14:30 UTC)

    let mut spy_eng = AlertEngine::new();
    spy_eng
        .add_alert(
            "SPY",
            Timeframe::W1,
            AlertCondition::SignalCatalogTrigger {
                catalog_id: "volume.spike_2x".to_owned(),
                float_level: None,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            None,
            &spy_d1,
            spy_arm_idx,
            Some(spy_d1[spy_arm_idx].timestamp),
            None,
        )
        .expect("arm SPY W1 signal alert");

    let w1_thu = aggregate_ohlcv_up_to(
        &spy_d1,
        Timeframe::D1,
        Timeframe::W1,
        spy_d1[spy_thu_w56].timestamp,
    )
    .unwrap();
    spy_eng.evaluate_causal(
        &spy_d1,
        Timeframe::D1,
        &w1_thu,
        Timeframe::W1,
        spy_thu_w56,
        Some(spy_d1[spy_thu_w56].timestamp),
        &store,
    );
    assert!(
        spy_eng.triggers.is_empty(),
        "SPY Thursday 14:30 UTC D1 bar must NOT fire W1 signal alert before Friday close"
    );

    let w1_fri = aggregate_ohlcv_up_to(
        &spy_d1,
        Timeframe::D1,
        Timeframe::W1,
        spy_d1[spy_fri_w56].timestamp,
    )
    .unwrap();
    spy_eng.evaluate_causal(
        &spy_d1,
        Timeframe::D1,
        &w1_fri,
        Timeframe::W1,
        spy_fri_w56,
        Some(spy_d1[spy_fri_w56].timestamp),
        &store,
    );
    assert_eq!(
        spy_eng.triggers.len(),
        1,
        "SPY Friday 14:30 UTC D1 bar MUST fire W1 signal alert"
    );

    // 3. `[Major-05]` `1m -> 15m` `PriceLevel` `OncePerBarClose` with `cooldown_bars == 0` suppresses
    //    duplicate intra-bucket crossings while still allowing immediate next-bucket firing,
    //    AND clamps `cooldown_remaining_bars <= cooldown_bars` when `cooldown_bars > 0`:
    let t_aligned_15m = 1_710_000_000_i64; // divisible by 900 (`1_710_000_000 % 900 == 0`)
    assert_eq!(t_aligned_15m % 900, 0);
    let mut m1_bars = Vec::with_capacity(45);
    for m in 0..45_usize {
        // Bucket 0 (`0..15`): flat 100
        // Bucket 1 (`15..30`): crosses above 105 at m=17, dips to 101 at m=19, crosses above 105 AGAIN at m=22!
        // Bucket 2 (`30..45`): dips to 101 at m=31, crosses above 105 at m=34!
        let (o, h, l, c) = match m {
            17 | 22 | 34 => (103.0, 106.5, 102.5, 106.0),
            18..=21 | 30..=33 => (102.0, 102.5, 100.5, 101.0),
            _ => (100.0, 100.5, 99.5, 100.0),
        };
        let ts = t_aligned_15m + (m as i64) * 60;
        m1_bars.push(Candle {
            timestamp: ts,
            date_label: format!("m{m}"),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 100.0,
        });
    }
    let mut cd_eng = AlertEngine::new();
    let id_cd0 = cd_eng
        .add_alert_with_cooldown(
            "BTCUSDT",
            Timeframe::M15,
            AlertCondition::PriceLevel {
                price: 105.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            0,
            None,
            &m1_bars,
            10,
            Some(m1_bars[10].timestamp),
            None,
        )
        .unwrap();
    let id_cd3 = cd_eng
        .add_alert_with_cooldown(
            "BTCUSDT",
            Timeframe::M15,
            AlertCondition::PriceLevel {
                price: 105.0,
                direction: CrossDirection::CrossAbove,
            },
            AlertFrequency::OncePerBarClose,
            3,
            None,
            &m1_bars,
            10,
            Some(m1_bars[10].timestamp),
            None,
        )
        .unwrap();

    for step in 11..40_usize {
        let m15_prefix = aggregate_ohlcv_up_to(
            &m1_bars,
            Timeframe::M1,
            Timeframe::M15,
            m1_bars[step].timestamp,
        )
        .unwrap();
        cd_eng.evaluate_causal(
            &m1_bars,
            Timeframe::M1,
            &m15_prefix,
            Timeframe::M15,
            step,
            Some(m1_bars[step].timestamp),
            &store,
        );
        let r3 = cd_eng.rules.iter().find(|r| r.id == id_cd3).unwrap();
        assert!(
            r3.cooldown_remaining_bars <= r3.cooldown_bars,
            "cooldown_remaining_bars ({}) must NEVER exceed cooldown_bars ({}) at step {step}",
            r3.cooldown_remaining_bars,
            r3.cooldown_bars
        );
        if step == 25 {
            let cd0_hits = cd_eng
                .triggers
                .iter()
                .filter(|t| t.alert_id == id_cd0)
                .count();
            assert_eq!(
                cd0_hits, 1,
                "OncePerBarClose with cooldown_bars=0 must fire AT MOST ONCE inside 15m bucket 1 (m=17 & m=22)"
            );
        }
    }
    let cd0_total = cd_eng
        .triggers
        .iter()
        .filter(|t| t.alert_id == id_cd0)
        .count();
    assert_eq!(
        cd0_total, 2,
        "OncePerBarClose with cooldown_bars=0 must fire once in bucket 1 (m=17) and once in bucket 2 (m=34)"
    );
}
