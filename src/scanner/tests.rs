use crate::data::{Timeframe, load_bundled_watchlist_universe};
use crate::model::Candle;

use super::{
    MarketRegime, ScanSignalKind, ScannerFilter, ScannerReplayIntent, ScannerSignalContext,
    SignalDirection, WatchlistSortColumn, compute_causal_rvol, evaluate_symbol_scan,
    evaluate_symbol_scan_and_signals, filter_and_sort_watchlist, filter_historical_signals,
    scan_historical_signals,
};

fn make_bar(idx: usize, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    let ts = 1_760_000_000 + (idx as i64) * 86_400;
    Candle {
        timestamp: ts,
        date_label: Timeframe::D1.format_candle_label(ts),
        open,
        high,
        low,
        close,
        volume,
    }
}

#[test]
fn regime_classifier_and_causal_rvol_exclude_current_bar_lookahead() {
    // 1. Verify RVOL strictly excludes bar `t` (`window = candles[t-20..t]`) and guards zero baseline.
    let mut flat_vol_bars: Vec<Candle> = (0..25)
        .map(|i| make_bar(i, 100.0, 101.0, 99.0, 100.5, 1_000.0))
        .collect();
    // Massive spike on bar 20: if bar 20 leaked into its own 20-bar baseline, RVOL would be < 20.0!
    flat_vol_bars[20].volume = 5_000.0;
    let rvol = compute_causal_rvol(&flat_vol_bars, 20);
    assert!(rvol[..20].iter().all(Option::is_none));
    assert!((rvol[20].expect("valid rvol at bar 20") - 5.0).abs() < 1e-12);

    // Zero-volume baseline must return `None` rather than `Inf`/`NaN`.
    let zero_bars: Vec<Candle> = (0..25)
        .map(|i| make_bar(i, 100.0, 101.0, 99.0, 100.0, 0.0))
        .collect();
    assert!(
        compute_causal_rvol(&zero_bars, 20)
            .iter()
            .all(Option::is_none)
    );

    // 2. Verify Pullback regime (`TEST-CAUSAL-003` — current bar high spike must NOT poison prior 20-bar high).
    let mut pb_bars = Vec::with_capacity(70);
    let mut price = 100.0;
    for i in 0..64 {
        price += 1.0;
        pb_bars.push(make_bar(
            i,
            price - 0.5,
            price + 0.5,
            price - 1.0,
            price,
            1_000.0,
        ));
    }
    // Prior 20-bar high before bar 65 is ~164.5 (EMA20 ~154.5, EMA50 ~139.5).
    // Pull back below EMA20 (~8.2% below 164.5) over bars 64..=65 while staying well above EMA50.
    pb_bars.push(make_bar(64, 162.0, 162.5, 152.5, 153.0, 950.0));
    // Give bar 65 an intrabar high wick to 200.0 (`> 164.5`) but close at 151.0 (~8.21% below 164.5!).
    // If bar 65 leaked into `recent_high`, drawdown `(200 - 151)/200 = 24.5% > 12%` would fail Pullback!
    pb_bars.push(make_bar(65, 153.0, 200.0, 150.5, 151.0, 950.0));

    let row = evaluate_symbol_scan(
        "TEST:PB",
        "PBTEST",
        "Unit",
        Timeframe::D1,
        &pb_bars,
        None,
        30,
    )
    .expect("valid scan row");
    assert_eq!(row.regime, MarketRegime::Pullback);
}

#[test]
fn technical_and_volume_signals_fire_causally_and_sort_none_metrics_last() {
    let universe = load_bundled_watchlist_universe();
    let rows: Vec<_> = universe
        .iter()
        .filter_map(|entry| {
            evaluate_symbol_scan(
                &entry.instrument_id,
                &entry.symbol,
                entry.preset.short_label(),
                entry.base_timeframe,
                &entry.candles,
                None,
                60,
            )
        })
        .collect();
    assert_eq!(rows.len(), 13);

    // Short 8-bar warmup symbol (`rsi_14 = None`, `rvol_20 = None`) must sort AFTER all finite rows
    // in BOTH ascending and descending order!
    let short_bars: Vec<Candle> = (0..8)
        .map(|i| make_bar(i, 50.0, 51.0, 49.0, 50.5, 500.0))
        .collect();
    let warm_row = evaluate_symbol_scan(
        "TEST:WARM",
        "AAA_WARM",
        "Unit",
        Timeframe::D1,
        &short_bars,
        None,
        20,
    )
    .expect("short row");
    assert_eq!(warm_row.regime, MarketRegime::InsufficientHistory);
    assert!(warm_row.rsi_14.is_none());
    assert!(warm_row.rvol_20.is_none());

    let mut mixed = rows;
    mixed.push(warm_row);

    let asc_rsi = filter_and_sort_watchlist(
        &mixed,
        &ScannerFilter::default(),
        WatchlistSortColumn::Rsi14,
        true,
    );
    assert_eq!(asc_rsi.last().expect("last").symbol, "AAA_WARM");
    assert!(
        asc_rsi[0].rsi_14.expect("min finite rsi") <= asc_rsi[1].rsi_14.expect("next finite rsi")
    );

    let desc_rsi = filter_and_sort_watchlist(
        &mixed,
        &ScannerFilter::default(),
        WatchlistSortColumn::Rsi14,
        false,
    );
    assert_eq!(desc_rsi.last().expect("last").symbol, "AAA_WARM");
    assert!(
        desc_rsi[0].rsi_14.expect("max finite rsi") >= desc_rsi[1].rsi_14.expect("next finite rsi")
    );
}

#[test]
fn historical_signal_scan_enforces_def001_cutoff_and_blind_practice_masking() {
    let universe = load_bundled_watchlist_universe();
    let mwg = universe
        .iter()
        .find(|e| e.symbol == "MWG")
        .expect("MWG in VN30 basket");

    let full_hits = scan_historical_signals(
        &mwg.instrument_id,
        &mwg.symbol,
        mwg.base_timeframe,
        &mwg.candles,
        None,
        180,
    );
    let target = full_hits
        .iter()
        .find(|h| h.signal == ScanSignalKind::RsiOversoldCrossUp)
        .cloned()
        .expect("MWG must emit RsiOversoldCrossUp on recovery bar");

    // 1. Cutoff strictly prior to `target.timestamp` must NOT leak `target` (`DEF-001` lock).
    let pre_cutoff = target.timestamp - 86_400;
    let masked_hits = scan_historical_signals(
        &mwg.instrument_id,
        &mwg.symbol,
        mwg.base_timeframe,
        &mwg.candles,
        Some(pre_cutoff),
        180,
    );
    assert!(masked_hits.iter().all(|h| h.timestamp <= pre_cutoff));
    assert!(!masked_hits.iter().any(|h| h.timestamp == target.timestamp));

    // Also verify prefix equality: scanning full series with `Some(pre_cutoff)` matches scanning
    // the truncated prefix slice bit-for-bit!
    let end = mwg.candles.partition_point(|c| c.timestamp <= pre_cutoff);
    let truncated_hits = scan_historical_signals(
        &mwg.instrument_id,
        &mwg.symbol,
        mwg.base_timeframe,
        &mwg.candles[..end],
        None,
        180,
    );
    assert_eq!(masked_hits, truncated_hits);

    // 2. BlindPractice vs SignalReview envelope reveal & rewind re-masking (`test_scanner_replay_integrity.py`).
    let blind_ctx = ScannerSignalContext::new(ScannerReplayIntent::BlindPractice, target.clone());
    assert!(!blind_ctx.is_revealed(Some(pre_cutoff)));
    assert!(blind_ctx.revealed_signal(Some(pre_cutoff)).is_none());
    assert!(blind_ctx.is_revealed(Some(target.timestamp)));
    assert_eq!(
        blind_ctx
            .revealed_signal(Some(target.timestamp))
            .expect("revealed at signal timestamp")
            .signal,
        ScanSignalKind::RsiOversoldCrossUp
    );
    // Rewinding 1 bar back immediately re-masks!
    assert!(blind_ctx.revealed_signal(Some(pre_cutoff)).is_none());

    let review_ctx = ScannerSignalContext::new(ScannerReplayIntent::SignalReview, target);
    assert!(review_ctx.is_revealed(Some(pre_cutoff)));
    assert!(review_ctx.revealed_signal(Some(pre_cutoff)).is_some());
}

#[test]
fn multi_symbol_universe_vn30_cafef_spy_btc_scans_deterministically() {
    let universe = load_bundled_watchlist_universe();
    assert_eq!(universe.len(), 13);

    let mut all_hits = Vec::new();
    let mut regimes = std::collections::HashSet::new();
    for entry in &universe {
        let row = evaluate_symbol_scan(
            &entry.instrument_id,
            &entry.symbol,
            entry.preset.short_label(),
            entry.base_timeframe,
            &entry.candles,
            None,
            120,
        )
        .expect("non-empty bundled instrument");
        regimes.insert(row.regime);
        all_hits.extend(scan_historical_signals(
            &entry.instrument_id,
            &entry.symbol,
            entry.base_timeframe,
            &entry.candles,
            None,
            120,
        ));
    }

    assert!(regimes.contains(&MarketRegime::Uptrend));
    assert!(regimes.contains(&MarketRegime::Downtrend));
    assert!(regimes.contains(&MarketRegime::SidewaysSqueeze));
    assert!(!all_hits.is_empty());

    let bull_only = filter_historical_signals(
        &all_hits,
        &ScannerFilter {
            direction_filter: Some(SignalDirection::Bullish),
            ..ScannerFilter::default()
        },
    );
    assert!(!bull_only.is_empty());
    assert!(
        bull_only
            .iter()
            .all(|h| h.direction == SignalDirection::Bullish)
    );
}

#[test]
fn critic_r08_regression_single_pass_scan_incomplete_tail_and_reveal_timestamp() {
    let universe = load_bundled_watchlist_universe();

    // 1. Single-pass `evaluate_symbol_scan_and_signals(..., false)` MUST match separate
    //    `(evaluate_symbol_scan, scan_historical_signals)` bit-for-bit across all 13 feeds!
    for entry in &universe {
        let expected_row = evaluate_symbol_scan(
            &entry.instrument_id,
            &entry.symbol,
            entry.preset.short_label(),
            entry.base_timeframe,
            &entry.candles,
            None,
            120,
        );
        let expected_hits = scan_historical_signals(
            &entry.instrument_id,
            &entry.symbol,
            entry.base_timeframe,
            &entry.candles,
            None,
            120,
        );
        let (actual_row, actual_hits) = evaluate_symbol_scan_and_signals(
            &entry.instrument_id,
            &entry.symbol,
            entry.preset.short_label(),
            entry.base_timeframe,
            &entry.candles,
            None,
            120,
            false,
        );
        assert_eq!(actual_row, expected_row);
        assert_eq!(actual_hits, expected_hits);
    }

    // 2. When `exclude_incomplete_tail_signal = true`, an unclosed partial higher-TF tail candle
    //    MUST NOT emit `active_signals` or `hits` at `last_idx`, while closed prior bars remain intact!
    let mwg = universe
        .iter()
        .find(|e| e.symbol == "MWG")
        .expect("MWG feed");
    let full_hits = scan_historical_signals(
        &mwg.instrument_id,
        &mwg.symbol,
        mwg.base_timeframe,
        &mwg.candles,
        None,
        180,
    );
    let sig_hit = full_hits.first().expect("MWG signal").clone();
    let slice_to_sig = &mwg.candles[..=sig_hit.bar_index];
    let (closed_row, closed_hits) = evaluate_symbol_scan_and_signals(
        &mwg.instrument_id,
        &mwg.symbol,
        mwg.preset.short_label(),
        mwg.base_timeframe,
        slice_to_sig,
        None,
        180,
        false,
    );
    assert!(!closed_row.unwrap().active_signals.is_empty());
    assert!(closed_hits.iter().any(|h| h.bar_index == sig_hit.bar_index));

    let (partial_row, partial_hits) = evaluate_symbol_scan_and_signals(
        &mwg.instrument_id,
        &mwg.symbol,
        mwg.preset.short_label(),
        mwg.base_timeframe,
        slice_to_sig,
        None,
        180,
        true,
    );
    assert!(
        partial_row.unwrap().active_signals.is_empty(),
        "unclosed partial higher-TF tail candle must withhold active_signals until bucket close"
    );
    assert!(
        partial_hits.iter().all(|h| h.bar_index < sig_hit.bar_index),
        "unclosed partial higher-TF tail candle must withhold historical hit at last_idx"
    );

    // 3. `ScannerSignalContext::with_reveal_timestamp` keeps higher-TF `BlindPractice` masked at
    //    bucket start (`hit.timestamp = 09:00`) and reveals ONLY at bucket close (`09:14`)!
    let bucket_close_ts = sig_hit.timestamp + 14 * 60;
    let mut ctx = ScannerSignalContext::with_reveal_timestamp(
        ScannerReplayIntent::BlindPractice,
        sig_hit.clone(),
        bucket_close_ts,
    );
    assert!(!ctx.is_revealed(Some(sig_hit.timestamp)));
    assert!(!ctx.is_revealed(Some(bucket_close_ts - 60)));
    assert!(ctx.is_revealed(Some(bucket_close_ts)));
    ctx.sanitize_deserialized();
    assert_eq!(ctx.reveal_at_timestamp, bucket_close_ts);
}

#[test]
fn r13_breadth_aggregate_before_ratio_at05_at09_at17_and_def001_causality() {
    use super::{BreadthFeedInput, RrgQuadrant, compute_market_breadth_and_rrg};

    // 1. AT05/AT09 Aggregate-Before-Ratio proof:
    //    Symbol A ("VCB", VN30): huge bullish bar trading 99B value (`buy_frac = 1.0`, `buy_val = 99B`)
    //    Symbol B ("FPT", VN30): tiny bearish bar trading 1B value (`buy_frac = 0.0`, `buy_val = 0B`)
    //    Cross-sectional naive average = 50%, TRUE value-weighted aggregate = 99%!
    let bench: Vec<Candle> = (0..40)
        .map(|i| {
            make_bar(
                i,
                1200.0,
                1205.0,
                1195.0,
                1200.0 + (i as f64) * 0.5,
                1_000.0,
            )
        })
        .collect();
    let mut vcb: Vec<Candle> = (0..39)
        .map(|i| make_bar(i, 90.0, 91.0, 89.0, 90.0 + (i as f64) * 0.2, 1_000.0))
        .collect();
    // Bar 39: close = 100.0, low = 99.0, high = 100.0, open = 99.0 -> intrabar_buy_fraction = 1.0
    vcb.push(make_bar(39, 99.0, 100.0, 99.0, 100.0, 990_000_000.0));

    let mut fpt: Vec<Candle> = (0..39)
        .map(|i| make_bar(i, 120.0, 121.0, 119.0, 120.0 - (i as f64) * 0.1, 1_000.0))
        .collect();
    // Bar 39: close = 100.0, low = 100.0, high = 101.0, open = 101.0 -> intrabar_buy_fraction = 0.0
    fpt.push(make_bar(39, 101.0, 101.0, 100.0, 100.0, 10_000_000.0));

    let feeds = [
        BreadthFeedInput {
            instrument_id: "HOSE:VCB:1D",
            symbol: "VCB",
            preset_label: "VN30",
            timeframe: Timeframe::D1,
            candles: &vcb,
            exclude_incomplete_tail: false,
        },
        BreadthFeedInput {
            instrument_id: "HOSE:FPT:1D",
            symbol: "FPT",
            preset_label: "VN30",
            timeframe: Timeframe::D1,
            candles: &fpt,
            exclude_incomplete_tail: false,
        },
    ];
    let snap = compute_market_breadth_and_rrg(&feeds, &bench, Timeframe::D1, "VNINDEX", None);
    assert_eq!(snap.participating_count, 2);
    assert_eq!(snap.vn30_participating_count, 2);
    assert_eq!(snap.advancers, 1);
    assert_eq!(snap.decliners, 1);
    // 99B / 100B = 99.0% (not 50.0% naive arithmetic mean!)
    assert!(
        (snap.money_flow_buy_pct - 99.0).abs() < 0.01,
        "AT05/AT09 violated: got {}",
        snap.money_flow_buy_pct
    );
    assert!((snap.vn30_money_flow_buy_pct - 99.0).abs() < 0.01);

    // 2. Quadrant classification check
    assert_eq!(RrgQuadrant::classify(102.0, 101.5), RrgQuadrant::Leading);
    assert_eq!(RrgQuadrant::classify(102.0, 99.0), RrgQuadrant::Weakening);
    assert_eq!(RrgQuadrant::classify(98.0, 98.0), RrgQuadrant::Lagging);
    assert_eq!(RrgQuadrant::classify(98.0, 102.0), RrgQuadrant::Improving);

    // 3. DEF-001 Strict Causal Replay Prefix Invariance:
    //    Corrupting bars after cutoff `T` MUST leave the Breadth + RRG snapshot at `T` bit-exact!
    let universe = load_bundled_watchlist_universe();
    let bench_u: Vec<Candle> = universe
        .iter()
        .find(|e| e.symbol == "VNINDEX")
        .expect("VNINDEX")
        .candles
        .clone();
    let cut_bar_open_ts = bench_u[120].timestamp;
    let cut_close_ts = cut_bar_open_ts + 86_399;
    let feeds_orig: Vec<BreadthFeedInput<'_>> = universe
        .iter()
        .map(|e| BreadthFeedInput {
            instrument_id: &e.instrument_id,
            symbol: &e.symbol,
            preset_label: e.preset.short_label(),
            timeframe: e.base_timeframe,
            candles: &e.candles,
            exclude_incomplete_tail: false,
        })
        .collect();
    let snap_orig = compute_market_breadth_and_rrg(
        &feeds_orig,
        &bench_u,
        Timeframe::D1,
        "VNINDEX",
        Some(cut_close_ts),
    );
    assert_eq!(snap_orig.evaluated_timestamp, Some(cut_bar_open_ts));
    let snap_before_close = compute_market_breadth_and_rrg(
        &feeds_orig,
        &bench_u,
        Timeframe::D1,
        "VNINDEX",
        Some(cut_close_ts - 1),
    );
    assert_eq!(
        snap_before_close.evaluated_timestamp,
        Some(bench_u[119].timestamp),
        "D1 bar closing 1s after cutoff must be withheld causally (DEF-001)"
    );
    drop(feeds_orig);
    let mut poisoned = universe;
    for e in &mut poisoned {
        for c in &mut e.candles {
            if c.timestamp > cut_bar_open_ts {
                c.close *= 9.0;
                c.high *= 9.0;
                c.low *= 0.1;
                c.volume *= 99.0;
            }
        }
    }
    let feeds_poison: Vec<BreadthFeedInput<'_>> = poisoned
        .iter()
        .map(|e| BreadthFeedInput {
            instrument_id: &e.instrument_id,
            symbol: &e.symbol,
            preset_label: e.preset.short_label(),
            timeframe: e.base_timeframe,
            candles: &e.candles,
            exclude_incomplete_tail: false,
        })
        .collect();
    let snap_poison = compute_market_breadth_and_rrg(
        &feeds_poison,
        &bench_u,
        Timeframe::D1,
        "VNINDEX",
        Some(cut_close_ts),
    );
    assert_eq!(snap_orig, snap_poison);
}
