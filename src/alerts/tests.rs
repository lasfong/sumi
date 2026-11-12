use crate::data::{Timeframe, aggregate_ohlcv, generate_intraday_vn30f_1m_report};
use crate::drawings::{
    Drawing, DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStore, DrawingStyle,
};
use crate::model::Candle;
use crate::replay::{ReplayController, ReplayStepUnit};

use super::{
    AlertCondition, AlertEngine, AlertFrequency, AlertStatus, CrossDirection,
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
