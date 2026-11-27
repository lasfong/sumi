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

#[test]
fn r15_signal_catalog_schema_quality_gate_and_bb_absent_neutral() {
    use super::catalog::{
        CONFLUENCE_WARMUP_BARS, ConfluenceWeightProfile, SIGNAL_CATALOG, SignalCategory,
        SignalQuality, evaluate_causal_confluence, filter_signal_catalog,
        find_catalog_entry_for_scan_kind,
    };

    assert_eq!(SIGNAL_CATALOG.len(), 123);
    for cat in SignalCategory::ALL_TABS {
        if cat != SignalCategory::All {
            let matches = filter_signal_catalog(cat, "");
            assert!(
                !matches.is_empty(),
                "every SignalCategory ({cat:?}) must have >= 1 catalog entry"
            );
        }
    }
    for kind in ScanSignalKind::ALL {
        let e = find_catalog_entry_for_scan_kind(kind);
        assert_ne!(e.params_digest(), 0);
    }
    let vi_hits = filter_signal_catalog(SignalCategory::All, "Khối Lượng");
    assert!(
        vi_hits.len() >= 2,
        "Vietnamese label search must match volume/VSA catalog entries"
    );

    // 1. Warmup < 50 bars -> InsufficientHistory & value == None
    let short_bars: Vec<Candle> = (0..18)
        .map(|i| {
            make_bar(
                i,
                100.0 + i as f64,
                102.0 + i as f64,
                99.0 + i as f64,
                101.0 + i as f64,
                1000.0,
            )
        })
        .collect();
    let short_rep =
        evaluate_causal_confluence(&short_bars, None, false, ConfluenceWeightProfile::Balanced);
    assert_eq!(short_rep.quality, SignalQuality::InsufficientHistory);
    assert!(short_rep.health.is_none());
    let vol_ev = short_rep
        .evaluations
        .iter()
        .find(|e| e.catalog_id == "volume.spike_2x")
        .unwrap();
    assert_eq!(vol_ev.quality, SignalQuality::InsufficientHistory);
    assert!(vol_ev.bool_value.is_none() && vol_ev.float_value.is_none());

    // 2. Zero-volume baseline -> VolumeSpike returns ZeroBaseline (`value = None`), while Health emits `bb_absent_neutral` (no negative penalty)
    let zero_vol_bars: Vec<Candle> = (0..=CONFLUENCE_WARMUP_BARS + 5)
        .map(|i| {
            let p = 100.0 + (i as f64) * 0.8;
            make_bar(i, p - 0.3, p + 0.6, p - 0.6, p, 0.0)
        })
        .collect();
    let zv_rep = evaluate_causal_confluence(
        &zero_vol_bars,
        None,
        false,
        ConfluenceWeightProfile::Balanced,
    );
    assert_eq!(zv_rep.quality, SignalQuality::Valid);
    let bd = zv_rep.health.expect("valid health on zero-volume uptrend");
    assert!(bd.bb_flow_absent_neutral);
    assert_eq!(bd.participation_score, 0.0);
    assert!(bd.composite_score > 0.0);
    let health_ev = zv_rep
        .evaluations
        .iter()
        .find(|e| e.catalog_id == "health.technical_composite")
        .unwrap();
    assert!(health_ev.reasons.iter().any(|r| r == "bb_absent_neutral"));
    assert!(health_ev.reasons.iter().any(|r| r == "OHLCV_PROXY"));
    let zv_spike = zv_rep
        .evaluations
        .iter()
        .find(|e| e.catalog_id == "volume.spike_2x")
        .unwrap();
    assert_eq!(zv_spike.quality, SignalQuality::ZeroBaseline);
    assert!(zv_spike.bool_value.is_none() && zv_spike.float_value.is_none());

    // 3. Negative / invalid volume -> InvalidVolume (`value = None`)
    let mut bad_vol_bars = zero_vol_bars;
    bad_vol_bars.last_mut().unwrap().volume = -50.0;
    let bad_rep = evaluate_causal_confluence(
        &bad_vol_bars,
        None,
        false,
        ConfluenceWeightProfile::TrendFocused,
    );
    assert_eq!(bad_rep.quality, SignalQuality::InvalidVolume);
    assert!(bad_rep.health.is_none());
}

#[test]
fn r15_confluence_def001_future_poisoning_tail_exclusion_and_blind_practice_lock() {
    use super::catalog::{ConfluenceWeightProfile, SignalQuality, evaluate_causal_confluence};

    let mut bars: Vec<Candle> = (0..120)
        .map(|i| {
            let base = 50.0 + (i as f64) * 0.45 + ((i % 7) as f64 - 3.0) * 0.3;
            let vol = if i == 95 { 4_500.0 } else { 1_000.0 };
            make_bar(i, base - 0.4, base + 1.1, base - 0.8, base + 0.5, vol)
        })
        .collect();
    let cutoff_ts = bars[95].timestamp;
    let clean_rep = evaluate_causal_confluence(
        &bars,
        Some(cutoff_ts),
        false,
        ConfluenceWeightProfile::Balanced,
    );
    assert_eq!(clean_rep.evaluated_bar_index, 95);
    assert_eq!(clean_rep.quality, SignalQuality::Valid);

    // Poison future bars > cutoff_ts: report at cutoff_ts must remain bit-exact!
    for b in &mut bars[96..] {
        b.close *= 25.0;
        b.high *= 30.0;
        b.low *= 0.05;
        b.volume *= 500.0;
    }
    let poisoned_rep = evaluate_causal_confluence(
        &bars,
        Some(cutoff_ts),
        false,
        ConfluenceWeightProfile::Balanced,
    );
    assert_eq!(clean_rep, poisoned_rep);

    // Incomplete higher-TF tail bar (`exclude_incomplete_tail_signal == true`) steps back to 94!
    let tail_excl_rep = evaluate_causal_confluence(
        &bars,
        Some(cutoff_ts),
        true,
        ConfluenceWeightProfile::Balanced,
    );
    assert!(tail_excl_rep.tail_bar_excluded);
    assert_eq!(tail_excl_rep.evaluated_bar_index, 94);
    let single_unclosed =
        evaluate_causal_confluence(&bars[..1], None, true, ConfluenceWeightProfile::Balanced);
    assert_eq!(single_unclosed.quality, SignalQuality::UnclosedTailBar);

    // BlindPractice explanation lock: hidden before reveal_ts, revealed at reveal_ts, re-masked on rewind!
    let (_, hits) = evaluate_symbol_scan_and_signals(
        "Preset:FPT",
        "FPT",
        "CSV",
        Timeframe::D1,
        &bars[..=95],
        None,
        80,
        false,
    );
    let target_hit = hits
        .iter()
        .find(|h| h.bar_index == 95 && h.signal == ScanSignalKind::VolumeSpike2x)
        .expect("volume spike hit at bar 95")
        .clone();
    let reveal_close_ts = target_hit.timestamp + 86_399;
    let ctx = ScannerSignalContext::with_reveal_timestamp(
        ScannerReplayIntent::BlindPractice,
        target_hit,
        reveal_close_ts,
    );
    assert!(ctx.revealed_explanation(&bars, Some(cutoff_ts)).is_none());
    let revealed = ctx
        .revealed_explanation(&bars, Some(reveal_close_ts))
        .expect("revealed at bucket close");
    assert_eq!(revealed.bool_value, Some(true));
    assert!(
        ctx.revealed_explanation(&bars, Some(reveal_close_ts - 1))
            .is_none()
    );
}

#[test]
fn critic_r15_regression_explain_unclosed_bar_cutoff_and_nan_inf_ohlc_guards() {
    use super::{
        ConfluenceWeightProfile, HistoricalSignalHit, MarketRegime, SIGNAL_CATALOG, ScanSignalKind,
        SignalDirection, SignalQuality, evaluate_causal_confluence, evaluate_signal_catalog_at_bar,
        explain_historical_signal,
    };

    // 1. R15-M02: `explain_historical_signal` must reject unclosed bar at mid-bucket cutoff
    let mut bars_15m = Vec::with_capacity(64);
    for i in 0..64_usize {
        let ts = 1_710_000_000_i64 + (i as i64) * 900;
        let close = 100.0 + (i as f64) * 0.4;
        let vol = if i == 60 { 6_000.0 } else { 1_000.0 };
        bars_15m.push(Candle {
            timestamp: ts,
            date_label: String::new(),
            open: close - 0.2,
            high: close + 0.5,
            low: close - 0.5,
            close,
            volume: vol,
        });
    }
    let hit_15m = HistoricalSignalHit {
        instrument_id: "Test:15m".to_owned(),
        symbol: "TEST".to_owned(),
        timeframe: Timeframe::M15,
        bar_index: 60,
        timestamp: bars_15m[60].timestamp,
        signal: ScanSignalKind::VolumeSpike2x,
        direction: SignalDirection::Bullish,
        regime: MarketRegime::Uptrend,
        price: bars_15m[60].close,
        metric_value: 6.0,
    };
    let open_ts = hit_15m.timestamp;
    assert!(
        explain_historical_signal(&bars_15m, &hit_15m, Some(open_ts + 179)).is_none(),
        "3m into a 15m bucket must NOT leak unclosed bar OHLCV"
    );
    assert!(
        explain_historical_signal(&bars_15m, &hit_15m, Some(open_ts + 898)).is_none(),
        "1s before 15m bar close must still return None"
    );
    let closed_ev = explain_historical_signal(&bars_15m, &hit_15m, Some(open_ts + 899))
        .expect("exact 15m bar close (open + 899s) must unlock evaluation");
    assert_eq!(closed_ev.quality, SignalQuality::Valid);
    assert_eq!(closed_ev.bool_value, Some(true));

    // 2. R15-M04a: Non-finite prior highs/lows must NEVER trigger false `structure.swing_bos`
    let mut nan_window_bars = bars_15m.clone();
    for b in &mut nan_window_bars[49..59] {
        b.high = f64::NAN;
        b.low = f64::NAN;
    }
    let rep_nan_win = evaluate_signal_catalog_at_bar(
        &nan_window_bars,
        59,
        None,
        ConfluenceWeightProfile::Balanced,
    )
    .expect("bar 59 in range");
    for id in [
        "structure.swing_bos",
        "vsa.stopping_volume",
        "divergence.rsi_hidden_or_classic",
    ] {
        let ev = rep_nan_win
            .evaluations
            .iter()
            .find(|e| e.catalog_id == id)
            .unwrap();
        assert!(
            !ev.quality.is_valid() && ev.bool_value.is_none() && ev.float_value.is_none(),
            "{id} must reject non-finite prior window: {ev:?}"
        );
    }

    // 3. R15-M04b: Non-finite current-bar OHLC with valid positive volume must return non-valid across ALL 17 entries
    let mut nan_curr = bars_15m.clone();
    nan_curr[63].open = f64::NAN;
    nan_curr[63].high = f64::INFINITY;
    nan_curr[63].low = f64::NEG_INFINITY;
    nan_curr[63].close = f64::NAN;
    nan_curr[63].volume = 5_000.0;
    let rep_nan_curr =
        evaluate_causal_confluence(&nan_curr, None, false, ConfluenceWeightProfile::Balanced);
    assert_eq!(rep_nan_curr.evaluations.len(), SIGNAL_CATALOG.len());
    for ev in &rep_nan_curr.evaluations {
        assert!(
            !ev.quality.is_valid() && ev.bool_value.is_none() && ev.float_value.is_none(),
            "all 17 catalog entries must reject NaN/Inf current OHLC: {ev:?}"
        );
    }
}

#[test]
fn r16_precomputed_catalog_matrix_matches_bar_by_bar_evaluation_across_all_17_entries() {
    use super::catalog::{
        ConfluenceWeightProfile, PrecomputedCatalogMatrix, SIGNAL_CATALOG, catalog_entry_index,
        evaluate_signal_catalog_at_bar,
    };

    let mut bars: Vec<Candle> = Vec::with_capacity(140);
    for i in 0..140_usize {
        let wave = ((i as f64) * 0.23).sin() * 3.5;
        let close = 100.0 + (i as f64) * 0.35 + wave;
        let open = close - ((i % 3) as f64 - 1.0) * 0.7;
        let high = open.max(close) + 1.1;
        let low = open.min(close) - 1.0;
        let vol = if i == 68 {
            0.0 // Zero-volume edge case
        } else if i == 112 {
            -10.0 // Negative-volume invalid edge case
        } else if i.is_multiple_of(13) {
            4_800.0 // Volume spike
        } else {
            1_000.0 + ((i % 5) as f64) * 120.0
        };
        bars.push(make_bar(i, open, high, low, close, vol));
    }

    let matrix = PrecomputedCatalogMatrix::compute(&bars);
    assert_eq!(matrix.bar_count, bars.len());

    for bar_idx in 0..bars.len() {
        let rep_bal =
            evaluate_signal_catalog_at_bar(&bars, bar_idx, None, ConfluenceWeightProfile::Balanced)
                .expect("bar in range");
        let rep_tf = evaluate_signal_catalog_at_bar(
            &bars,
            bar_idx,
            None,
            ConfluenceWeightProfile::TrendFocused,
        )
        .expect("bar in range");

        assert_eq!(matrix.regime_at(bar_idx), rep_bal.regime);
        assert_eq!(
            matrix.counts_at(bar_idx, ConfluenceWeightProfile::Balanced),
            (
                rep_bal.bullish_signal_count as u8,
                rep_bal.bearish_signal_count as u8,
            ),
            "Balanced counts mismatch at bar {bar_idx}"
        );
        assert_eq!(
            matrix.counts_at(bar_idx, ConfluenceWeightProfile::TrendFocused),
            (
                rep_tf.bullish_signal_count as u8,
                rep_tf.bearish_signal_count as u8,
            ),
            "TrendFocused counts mismatch at bar {bar_idx}"
        );

        match (
            matrix.health_score_at(bar_idx, ConfluenceWeightProfile::Balanced),
            &rep_bal.health,
        ) {
            (Ok(ms), Some(h)) => {
                assert!(
                    (ms - h.composite_score).abs() < 1e-11,
                    "Balanced health mismatch at bar {bar_idx}: {ms} vs {}",
                    h.composite_score
                );
            }
            (Err(mq), None) => assert_eq!(mq, rep_bal.quality),
            other => panic!("Balanced health state mismatch at bar {bar_idx}: {other:?}"),
        }

        match (
            matrix.health_score_at(bar_idx, ConfluenceWeightProfile::TrendFocused),
            &rep_tf.health,
        ) {
            (Ok(ms), Some(h)) => {
                assert!(
                    (ms - h.composite_score).abs() < 1e-11,
                    "TrendFocused health mismatch at bar {bar_idx}: {ms} vs {}",
                    h.composite_score
                );
            }
            (Err(mq), None) => assert_eq!(mq, rep_tf.quality),
            other => panic!("TrendFocused health state mismatch at bar {bar_idx}: {other:?}"),
        }

        for ev in &rep_bal.evaluations {
            let cat_idx = catalog_entry_index(ev.catalog_id).expect("catalog index");
            let pt = matrix.point_at(cat_idx, bar_idx).expect("matrix point");
            assert_eq!(
                pt.quality, ev.quality,
                "quality mismatch for {} at bar {bar_idx}",
                ev.catalog_id
            );
            assert_eq!(
                pt.bool_value, ev.bool_value,
                "bool mismatch for {} at bar {bar_idx}",
                ev.catalog_id
            );
            assert_eq!(
                pt.direction, ev.direction,
                "direction mismatch for {} at bar {bar_idx}",
                ev.catalog_id
            );
            match (pt.float_value, ev.float_value) {
                (Some(a), Some(b)) => assert!(
                    (a - b).abs() < 1e-11,
                    "float mismatch for {} at bar {bar_idx}: {a} vs {b}",
                    ev.catalog_id
                ),
                (None, None) => {}
                other => panic!(
                    "float presence mismatch for {} at bar {bar_idx}: {other:?}",
                    ev.catalog_id
                ),
            }
        }
        for ev in &rep_tf.evaluations {
            let cat_idx = catalog_entry_index(ev.catalog_id).expect("catalog index");
            let pt = matrix
                .point_at_with_profile(cat_idx, bar_idx, ConfluenceWeightProfile::TrendFocused)
                .expect("TrendFocused matrix point");
            assert_eq!(
                pt.quality, ev.quality,
                "TrendFocused quality mismatch for {} at bar {bar_idx}",
                ev.catalog_id
            );
            assert_eq!(
                pt.bool_value, ev.bool_value,
                "TrendFocused bool mismatch for {} at bar {bar_idx}",
                ev.catalog_id
            );
            assert_eq!(
                pt.direction, ev.direction,
                "TrendFocused direction mismatch for {} at bar {bar_idx}",
                ev.catalog_id
            );
            match (pt.float_value, ev.float_value) {
                (Some(a), Some(b)) => assert!(
                    (a - b).abs() < 1e-11,
                    "TrendFocused float mismatch for {} at bar {bar_idx}: {a} vs {b}",
                    ev.catalog_id
                ),
                (None, None) => {}
                other => panic!(
                    "TrendFocused float presence mismatch for {} at bar {bar_idx}: {other:?}",
                    ev.catalog_id
                ),
            }
        }
        assert_eq!(rep_bal.evaluations.len(), SIGNAL_CATALOG.len());
    }
}

#[test]
fn r16_market_regime_bitmask_catalog_entry_index_and_bidirectional_bool_flags() {
    use super::MarketRegime;
    use super::catalog::{SIGNAL_CATALOG, catalog_entry_index, is_bidirectional_bool_signal};

    let mut union_mask = 0_u8;
    for (idx, reg) in MarketRegime::ALL_FILTERABLE.iter().copied().enumerate() {
        let b = reg.bit();
        assert_eq!(b, 1_u8 << idx);
        assert_eq!(union_mask & b, 0, "regime bits must be disjoint");
        union_mask |= b;
    }
    assert_eq!(union_mask, MarketRegime::ALL_VALID_MASK);
    assert_eq!(MarketRegime::ALL_VALID_MASK, 0x3F);
    assert_eq!(MarketRegime::InsufficientHistory.bit(), 0);

    for (idx, entry) in SIGNAL_CATALOG.iter().enumerate() {
        assert_eq!(catalog_entry_index(entry.id), Some(idx));
    }
    assert_eq!(catalog_entry_index("nonexistent.signal"), None);

    assert!(is_bidirectional_bool_signal("sr.prior_20b_breakout"));
    assert!(is_bidirectional_bool_signal("structure.swing_bos"));
    assert!(is_bidirectional_bool_signal(
        "divergence.rsi_hidden_or_classic"
    ));
    assert!(!is_bidirectional_bool_signal("technical.macd_bull_cross"));
    assert!(!is_bidirectional_bool_signal("volume.spike_2x"));
}

#[test]
fn r18_b3_signal_catalog_72_legacy_superset_causal_pivots_and_12_divergences() {
    use super::SignalDirection;
    use super::catalog::{
        ConfluenceWeightProfile, PrecomputedCatalogMatrix, SIGNAL_CATALOG, catalog_entry_index,
        evaluate_causal_confluence, find_catalog_entry,
    };

    // 1. Exact 100% superset proof against all 72 canonical signal IDs!
    const LEGACY_72_IDS: [&str; 72] = [
        "volume.relative_volume",
        "volume.spike",
        "volume.climax_up",
        "volume.climax_down",
        "pattern.bullish_engulfing",
        "pattern.bearish_engulfing",
        "pattern.hammer",
        "pattern.shooting_star",
        "pattern.morning_star",
        "pattern.evening_star",
        "pattern.piercing_line",
        "pattern.dark_cloud_cover",
        "pattern.tweezer_bottom",
        "pattern.tweezer_top",
        "pattern.inside_bar_breakout_up",
        "pattern.inside_bar_breakout_down",
        "pattern.any_bullish",
        "pattern.any_bearish",
        "tech.macd_signal_cross",
        "tech.macd_zero_cross",
        "tech.rsi_level_cross",
        "tech.ema_cross",
        "tech.swing_break",
        "tech.composite",
        "regime.uptrend",
        "regime.downtrend",
        "regime.sideways",
        "regime.pullback",
        "regime.recovery",
        "regime.new_high",
        "regime.new_low",
        "sr.near_support",
        "sr.near_resistance",
        "structure.candle_score",
        "vsa.strong_demand",
        "vsa.weak_demand",
        "vsa.upthrust",
        "vsa.spring",
        "vsa.strong_demand_at_support",
        "vsa.strong_supply_at_resistance",
        "ichimoku.score",
        "ichimoku.bullish",
        "ichimoku.bearish",
        "ichimoku.tk_cross_bullish",
        "ichimoku.tk_cross_bearish",
        "ichimoku.kumo_breakout_bullish",
        "ichimoku.kumo_breakout_bearish",
        "divergence.confirmed_pivot_high",
        "divergence.confirmed_pivot_low",
        "divergence.rsi_regular_bullish",
        "divergence.rsi_regular_bearish",
        "divergence.rsi_hidden_bullish",
        "divergence.rsi_hidden_bearish",
        "divergence.macd_regular_bullish",
        "divergence.macd_regular_bearish",
        "divergence.macd_hidden_bullish",
        "divergence.macd_hidden_bearish",
        "divergence.stoch_regular_bullish",
        "divergence.stoch_regular_bearish",
        "divergence.stoch_hidden_bullish",
        "divergence.stoch_hidden_bearish",
        "health.score",
        "health.favorable",
        "health.unfavorable",
        "bb.direction_rising",
        "bb.direction_falling",
        "bb.regime_positive",
        "bb.regime_negative",
        "bb.confluence_bullish",
        "bb.confluence_bearish",
        "bb.turn_up",
        "bb.turn_down",
    ];
    assert_eq!(SIGNAL_CATALOG.len(), 123);
    for id in LEGACY_72_IDS {
        let entry = find_catalog_entry(id);
        assert!(entry.is_some(), "missing legacy SignalRegistry ID: {id}");
        if id.starts_with("divergence.") {
            assert_eq!(entry.unwrap().causal_delay_bars, 3);
        }
    }

    // 2. `TEST-CAUSAL-001` (future-append invariance) & `TEST-CAUSAL-002` (`available_at_index == t`, zero double-delay!)
    //    Plus `PrecomputedCatalogMatrix` exact parity across ALL 101 entries!
    let mut bars: Vec<Candle> = (0..110)
        .map(|i| {
            let phase = i as f64;
            let wave = (phase * 0.42).sin() * 4.5 + (phase * 0.11).cos() * 2.0;
            let base = 100.0 + phase * 0.15 + wave;
            make_bar(
                i,
                base - 0.5,
                base + 1.4,
                base - 1.3,
                base + 0.4,
                1_200.0 + (i % 9) as f64 * 250.0,
            )
        })
        .collect();

    let mat_prefix = PrecomputedCatalogMatrix::compute(&bars[..90]);
    let rep_89_clean =
        evaluate_causal_confluence(&bars[..90], None, false, ConfluenceWeightProfile::Balanced);
    assert_eq!(rep_89_clean.evaluations.len(), 123);
    for (e_idx, ev) in rep_89_clean.evaluations.iter().enumerate() {
        assert_eq!(
            ev.available_at_index, 89,
            "TEST-CAUSAL-002 violated for {}: available_at_index={} != 89",
            ev.catalog_id, ev.available_at_index
        );
        let pt = mat_prefix.point_at(e_idx, 89).expect("matrix point");
        assert_eq!(
            pt.bool_value, ev.bool_value,
            "bool mismatch on {}",
            ev.catalog_id
        );
        assert_eq!(
            pt.direction, ev.direction,
            "dir mismatch on {}",
            ev.catalog_id
        );
    }

    // Poison future bars `[90..110)` with extreme prices & volumes -> bar `89` MUST remain 100% bit-identical!
    for b in &mut bars[90..] {
        b.high = 999_999.0;
        b.close = 999_999.0;
        b.low = 0.01;
        b.volume = 999_999_999.0;
    }
    let mat_poisoned = PrecomputedCatalogMatrix::compute(&bars);
    for (e_idx, entry) in SIGNAL_CATALOG.iter().enumerate() {
        assert_eq!(
            mat_prefix.point_at(e_idx, 89),
            mat_poisoned.point_at(e_idx, 89),
            "TEST-CAUSAL-001 future lookahead detected on {}",
            entry.id
        );
    }

    // 3. Directional `sr.near_support` vs `sr.near_resistance` EMA non-contradiction
    //    (rectifying legacy `support_resistance.py:98-100` where `|close - EMA| <= 0.5*ATR` fired BOTH!):
    //    With `EMA20 ~ 100.0`, `ATR ~ 2.0`, `prior_low = 90.0`, `prior_high = 110.0`:
    //    - Bar 25 (`close = 100.3 > EMA20`, within `0.5*ATR` from ABOVE): `sr.near_support == true` (`Bullish`) AND `sr.near_resistance == false`!
    //    - Bar 26 (`close = 99.7 < EMA20`, within `0.5*ATR` from BELOW): `sr.near_resistance == true` (`Bearish`) AND `sr.near_support == false`!
    let mut sr_bars: Vec<Candle> = (0..25)
        .map(|i| {
            let (hi, lo) = if i == 10 {
                (110.0, 90.0)
            } else {
                (101.0, 99.0)
            };
            make_bar(i, 100.0, hi, lo, 100.0, 1_000.0)
        })
        .collect();
    sr_bars.push(make_bar(25, 100.4, 100.8, 100.1, 100.3, 1_200.0)); // testing EMA20 from ABOVE -> support ONLY!
    sr_bars.push(make_bar(26, 99.6, 99.9, 99.2, 99.7, 1_200.0)); // testing EMA20 from BELOW -> resistance ONLY!
    let sr_mat = PrecomputedCatalogMatrix::compute(&sr_bars);
    let sup_idx = catalog_entry_index("sr.near_support").unwrap();
    let res_idx = catalog_entry_index("sr.near_resistance").unwrap();
    assert_eq!(sr_mat.point_at(sup_idx, 25).unwrap().bool_value, Some(true));
    assert_eq!(
        sr_mat.point_at(sup_idx, 25).unwrap().direction,
        Some(SignalDirection::Bullish)
    );
    assert_eq!(
        sr_mat.point_at(res_idx, 25).unwrap().bool_value,
        Some(false)
    );
    assert_eq!(sr_mat.point_at(res_idx, 26).unwrap().bool_value, Some(true));
    assert_eq!(
        sr_mat.point_at(res_idx, 26).unwrap().direction,
        Some(SignalDirection::Bearish)
    );
    assert_eq!(
        sr_mat.point_at(sup_idx, 26).unwrap().bool_value,
        Some(false)
    );

    // 4. Synthetic confirmed `right_bars = 3` Regular Bullish Divergence:
    //    First low `p1 = 30` (`L = 85.0`, deeply oversold crash), recovery, second lower low `p2 = 42` (`L = 83.5`, shallower momentum -> higher Stoch/RSI),
    //    confirmed strictly at `t = p2 + 3 = 45` (NOT at `t = 42`!).
    let mut div_bars = Vec::with_capacity(55);
    for i in 0..55 {
        let px = match i {
            0..=24 => 110.0,
            25 => 104.0,
            26 => 98.0,
            27 => 93.0,
            28 => 89.0,
            29 => 86.5,
            30 => 85.0, // p1 sharp capitulation low
            31 => 89.0,
            32 => 93.0,
            33..=38 => 96.0,
            39 => 93.0,
            40 => 90.0,
            41 => 88.0,
            42 => 84.0, // p2 lower price low, gentler 3-bar drift -> higher RSI(14) & MACD hist!
            43 => 88.5,
            44 => 91.0,
            45 => 93.5, // confirmation bar t = p2 + 3 = 45!
            _ => 95.0,
        };
        div_bars.push(make_bar(i, px + 0.5, px + 1.2, px - 1.0, px, 1_500.0));
    }
    div_bars[30].low = 83.0;
    div_bars[42].low = 81.5; // strictly lower pivot low at p2=42, while close=84 vs p1's deeper 6-bar crash!
    let div_mat = PrecomputedCatalogMatrix::compute(&div_bars);
    let rsi_bull_idx = catalog_entry_index("divergence.rsi_regular_bullish").unwrap();
    let stoch_bull_idx = catalog_entry_index("divergence.stoch_regular_bullish").unwrap();
    assert_eq!(
        div_mat.point_at(rsi_bull_idx, 42).unwrap().bool_value,
        Some(false),
        "must NOT fire unconfirmed pivot at p2=42 without right_bars=3!"
    );
    assert!(
        div_mat.point_at(rsi_bull_idx, 45).unwrap().bool_value == Some(true)
            || div_mat.point_at(stoch_bull_idx, 45).unwrap().bool_value == Some(true),
        "confirmed bullish divergence must fire at t = p2 + 3 = 45!"
    );
}

#[test]
fn r18_b4_proxy_bb_multihorizon_market_bb_at09_at17_and_pit_universe_resolver() {
    use super::catalog::{ConfluenceWeightProfile, evaluate_causal_confluence};
    use super::proxy_bb::{MarketBBAggregator, MarketBBFeedInput, MarketPublicationStatus};
    use super::universe::{
        CoverageStatus, SessionDate, UniverseDefinition, UniverseError, UniverseMode,
        UniverseResolver,
    };
    use crate::data::Timeframe;
    use crate::indicators::{BBDirection, BBHorizon, BBRegime, compute_proxy_bb_series};

    // 1. `ProxyBBCalculator` (`AT01` [0,100], `AT05` zero-liquidity `None`, `AT08` warmup, `AT18` prefix causality)
    let mut bars: Vec<Candle> = (0..65)
        .map(|i| {
            let p = 100.0 + (i as f64) * 0.5;
            make_bar(i, p - 0.3, p + 0.5, p - 0.5, p + 0.4, 2_000.0)
        })
        .collect();
    let pbb = compute_proxy_bb_series(&bars);
    // Warmup (`AT08`): `T03` is `None` on bars `0..2` and `Some` at bar `2`; `T200` is `None` on all 65 bars!
    assert!(pbb.points[1].horizon(BBHorizon::T03).is_warmup);
    assert!(pbb.points[1].horizon(BBHorizon::T03).bb_value.is_none());
    assert!(pbb.points[2].horizon(BBHorizon::T03).bb_value.is_some());
    assert!(pbb.points[64].horizon(BBHorizon::T200).is_warmup);
    let t20_64 = pbb.points[64].horizon(BBHorizon::T20);
    let bb64 = t20_64.bb_value.expect("valid T20");
    assert!((0.0..=100.0).contains(&bb64) && bb64 > 55.0);
    assert_eq!(t20_64.regime, BBRegime::Positive);
    assert_ne!(t20_64.direction, BBDirection::Unknown);

    // Upgraded Family 4 Participation: `bb_flow_absent_neutral == false` when volume > 0!
    let conf_pos =
        evaluate_causal_confluence(&bars, None, false, ConfluenceWeightProfile::Balanced);
    let h_pos = conf_pos.health.expect("valid health");
    assert!(!h_pos.bb_flow_absent_neutral);
    assert!(h_pos.participation_score > 0.0);

    // `AT05`: Zero-liquidity window (`20` consecutive zero-volume bars) MUST emit `bb_value = None` (NEVER `50.0`!)
    for b in &mut bars[30..50] {
        b.volume = 0.0;
    }
    let pbb_zero = compute_proxy_bb_series(&bars);
    let z_pt = pbb_zero.points[49].horizon(BBHorizon::T20);
    assert!(!z_pt.is_warmup);
    assert!(z_pt.is_zero_activity);
    assert_eq!(
        z_pt.bb_value, None,
        "AT05 violated: zero liquidity must return None, not 50.0"
    );

    // 2. `MarketBBAggregator` (`AT09` aggregate-before-ratio != naive mean, `AT17` count vs value breadth)
    let d0 = SessionDate::new(2025, 1, 2).unwrap();
    let d_ts = |off: i64| (1_735_776_000_i64) + off * 86_400; // 2025-01-02 UTC + off days
    // Heavyweight `MEGA` (val ~ 100M, bearish close at low -> BB ~ 0%) vs 2 tiny `SMALL1`, `SMALL2` (val ~ 100k, bullish close at high -> BB ~ 100%)
    let mega: Vec<Candle> = (0..6)
        .map(|i| Candle {
            timestamp: d_ts(i),
            date_label: format!("d{i}"),
            open: 100.0,
            high: 102.0,
            low: 98.0,
            close: 98.0, // P = -1.0 -> 0% buy!
            volume: 1_000_000.0,
        })
        .collect();
    let small: Vec<Candle> = (0..6)
        .map(|i| Candle {
            timestamp: d_ts(i),
            date_label: format!("d{i}"),
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 11.0, // P = +1.0 -> 100% buy!
            volume: 100.0,
        })
        .collect();
    let custom_u = UniverseDefinition::from_auto_str(
        r#"{
          "universe_id": "AT09-TEST",
          "version": "v1",
          "name": "AT09 Weighting Proof",
          "status": "APPROVED",
          "default_mode": "POINT_IN_TIME",
          "effective_from": "2025-01-01",
          "members": [
            {"symbol": "MEGA", "exchange": "HOSE", "status": "ACTIVE"},
            {"symbol": "SMALL1", "exchange": "HOSE", "status": "ACTIVE"},
            {"symbol": "SMALL2", "exchange": "HOSE", "status": "ACTIVE"}
          ]
        }"#,
    )
    .expect("parse custom universe");
    assert_eq!(d0.year, 2025);
    let mbb = MarketBBAggregator::calculate_market_bb(
        &custom_u,
        &[
            MarketBBFeedInput {
                symbol: "MEGA",
                timeframe: Timeframe::D1,
                candles: &mega,
            },
            MarketBBFeedInput {
                symbol: "SMALL1",
                timeframe: Timeframe::D1,
                candles: &small,
            },
            MarketBBFeedInput {
                symbol: "SMALL2",
                timeframe: Timeframe::D1,
                candles: &small,
            },
        ],
        Some(UniverseMode::PointInTime),
        None,
        0.0,
        0.80,
    );
    assert_eq!(
        mbb.publication_status,
        MarketPublicationStatus::CanonicalPublished
    );
    let last_m = mbb.points.last().unwrap();
    let t05_m = last_m.horizon(BBHorizon::T05).market_bb_value.unwrap();
    // Naive average of [0%, 100%, 100%] would be 66.7% (Bullish!), whereas AT09 value-aggregated MarketBB is < 1.0% (Bearish!)
    assert!(
        t05_m < 1.0,
        "AT09 violated: expected value-weighted MarketBB < 1.0%, got {t05_m}"
    );
    let fb05 = last_m.breadth_at(BBHorizon::T05);
    assert_eq!(fb05.positive_count, 2);
    assert_eq!(fb05.negative_count, 1);
    assert!((fb05.positive_count_ratio - (2.0 / 3.0)).abs() < 1e-9);
    assert!(
        fb05.positive_value_ratio < 0.01,
        "AT17 value breadth must reflect MEGA's >99.9% value share"
    );

    // 3. `UniverseResolver` on the exact `pit_test_universe.json` fixture!
    let fixture_json = include_str!("../../../../assets/fixtures/pit_test_universe.json");
    let pit_fix =
        UniverseDefinition::from_auto_str(fixture_json).expect("load pit_test_universe.json");
    assert_eq!(pit_fix.universe_id, "TEST-PIT");
    // Before universe effective_from (2023-12-31): 0 active in PIT!
    let r_pre = UniverseResolver::resolve(
        &pit_fix,
        SessionDate::parse_iso("2023-12-31").unwrap(),
        Some(UniverseMode::PointInTime),
    )
    .unwrap();
    assert!(r_pre.active_symbols.is_empty() && r_pre.is_point_in_time);
    // 2024-03-15: AAA & CCC active; BBB not yet listed (`2024-06-01`); DDD suspended; EEE reserve
    let r_2024_q1 = UniverseResolver::resolve(
        &pit_fix,
        SessionDate::parse_iso("2024-03-15").unwrap(),
        Some(UniverseMode::PointInTime),
    )
    .unwrap();
    assert_eq!(r_2024_q1.active_symbols, vec!["AAA", "CCC"]);
    // 2024-07-01: AAA, BBB, CCC all active
    let r_2024_q3 = UniverseResolver::resolve(
        &pit_fix,
        SessionDate::parse_iso("2024-07-01").unwrap(),
        Some(UniverseMode::PointInTime),
    )
    .unwrap();
    assert_eq!(r_2024_q3.active_symbols, vec!["AAA", "BBB", "CCC"]);
    // 2025-07-01: CCC delisted after 2025-06-30 -> only AAA, BBB active!
    let r_2025_q3 = UniverseResolver::resolve(
        &pit_fix,
        SessionDate::parse_iso("2025-07-01").unwrap(),
        Some(UniverseMode::PointInTime),
    )
    .unwrap();
    assert_eq!(r_2025_q3.active_symbols, vec!["AAA", "BBB"]);
    // RetrospectiveFixed on 2024-03-15: includes BBB (future survivor!) and emits survivor_bias_caveat!
    let r_retro = UniverseResolver::resolve(
        &pit_fix,
        SessionDate::parse_iso("2024-03-15").unwrap(),
        Some(UniverseMode::RetrospectiveFixed),
    )
    .unwrap();
    assert!(!r_retro.is_point_in_time);
    assert!(r_retro.survivor_bias_caveat.is_some());
    assert_eq!(r_retro.active_symbols, vec!["AAA", "BBB", "CCC"]);

    // Coverage check: 2/3 = 66.7% -> Degraded vs 0.80 threshold
    let avail = ["AAA", "BBB"];
    let cov = UniverseResolver::check_coverage(&r_2024_q3, &avail, 0.80).unwrap();
    assert_eq!(cov.status, CoverageStatus::Degraded);

    // Overlapping episode rejection
    let overlap_res = UniverseDefinition::from_auto_str(
        r#"{
          "universe_id": "BAD",
          "version": "v1",
          "members": [
            {"symbol": "AAA", "effective_from": "2024-01-01", "effective_to": "2024-06-30", "status": "ACTIVE"},
            {"symbol": "AAA", "effective_from": "2024-06-01", "effective_to": "2024-12-31", "status": "ACTIVE"}
          ]
        }"#,
    );
    assert!(matches!(
        overlap_res,
        Err(UniverseError::OverlappingMemberEpisodes { .. })
    ));
}

#[test]
fn critic_r18_regression_market_bb_intraday_rollup_venue_close_and_universe_validation() {
    use crate::data::timeframe::{Timeframe, aggregate_ohlcv};
    use crate::indicators::proxy_bb::{BBDirection, BBHorizon, compute_proxy_bb_series};
    use crate::scanner::catalog::{
        ConfluenceWeightProfile, PrecomputedCatalogMatrix, find_catalog_entry,
    };
    use crate::scanner::proxy_bb::{MarketBBAggregator, MarketBBFeedInput};
    use crate::scanner::universe::{
        MemberStatus, SessionDate, UniverseDefinition, UniverseError, UniverseMember, UniverseMode,
        UniverseResolver, UniverseStatus,
    };

    // 1. `[Blocker-02]` Intraday (`M15`) constituent feed MUST aggregate to `D1` before `MarketBB`
    //    and `FlowBreadth` (`AT09` + `AT17`: zero same-date overwrite, exact parity with `D1` feed)!
    let base_day_ts = 1_735_776_000_i64; // 2025-01-02 00:00:00 UTC
    let mut m15_bars = Vec::new();
    for day in 0..24_i64 {
        let day_start = base_day_ts + day * 86_400 + 2 * 3_600; // 09:00 ICT
        for slot in 0..16_i64 {
            let p = 50.0 + (day as f64) * 0.6 + (slot as f64) * 0.12;
            m15_bars.push(Candle {
                timestamp: day_start + slot * 900,
                date_label: format!("d{day}_s{slot}"),
                open: p - 0.05,
                high: p + 0.15,
                low: p - 0.10,
                close: p + 0.10,
                volume: 1_000.0 + (slot as f64) * 50.0,
            });
        }
    }
    let d1_bars = aggregate_ohlcv(&m15_bars, Timeframe::M15, Timeframe::D1).expect("D1 rollup");
    assert_eq!(d1_bars.len(), 24);

    let u_fpt = UniverseDefinition {
        universe_id: "FPT_UNIV".to_owned(),
        version: "v1".to_owned(),
        name: "FPT Test".to_owned(),
        status: UniverseStatus::Approved,
        default_mode: UniverseMode::PointInTime,
        effective_from: None,
        effective_to: None,
        selection_purpose: "test".to_owned(),
        source_evidence: "unit".to_owned(),
        members: vec![UniverseMember {
            symbol: "FPT".to_owned(),
            company_name: None,
            exchange: "HOSE".to_owned(),
            effective_from: None,
            effective_to: None,
            status: MemberStatus::Active,
            sector: None,
            inclusion_reason: "core".to_owned(),
            weight: 1.0,
        }],
    };

    let mbb_from_m15 = MarketBBAggregator::calculate_market_bb(
        &u_fpt,
        &[MarketBBFeedInput {
            symbol: "FPT",
            timeframe: Timeframe::M15,
            candles: &m15_bars,
        }],
        Some(UniverseMode::PointInTime),
        None,
        0.0,
        0.80,
    );
    let mbb_from_d1 = MarketBBAggregator::calculate_market_bb(
        &u_fpt,
        &[MarketBBFeedInput {
            symbol: "FPT",
            timeframe: Timeframe::D1,
            candles: &d1_bars,
        }],
        Some(UniverseMode::PointInTime),
        None,
        0.0,
        0.80,
    );
    assert_eq!(mbb_from_m15.points.len(), 24);
    assert_eq!(mbb_from_m15.points.len(), mbb_from_d1.points.len());
    let last_m15 = mbb_from_m15.points.last().unwrap();
    let last_d1 = mbb_from_d1.points.last().unwrap();
    assert!(
        (last_m15.horizon(BBHorizon::T20).market_total_value
            - last_d1.horizon(BBHorizon::T20).market_total_value)
            .abs()
            < 1e-6,
        "intraday MarketBB lost same-date session traded value: {} vs {}",
        last_m15.horizon(BBHorizon::T20).market_total_value,
        last_d1.horizon(BBHorizon::T20).market_total_value
    );
    assert!(
        (last_m15.horizon(BBHorizon::T20).market_bb_value.unwrap()
            - last_d1.horizon(BBHorizon::T20).market_bb_value.unwrap())
        .abs()
            < 1e-9
    );
    assert_eq!(
        last_m15.breadth_at(BBHorizon::T20).positive_count,
        last_d1.breadth_at(BBHorizon::T20).positive_count
    );

    // 2. `[Blocker-02]` Venue session-close cutoff on `VN30` `D1` vs intraday `M15`:
    //    At `14:00 ICT` (`07:00:00 UTC`) on day 23, day 23 is still open -> withheld (`23` sessions)!
    //    At `14:45 ICT` (`07:44:59 UTC`) on day 23, HOSE ATC closed -> included (`24` sessions)!
    let day23_mid_cut = base_day_ts + 23 * 86_400 + 7 * 3_600;
    let day23_atc_cut = base_day_ts + 23 * 86_400 + 7 * 3_600 + 44 * 60 + 59;
    let mbb_d1_mid = MarketBBAggregator::calculate_market_bb(
        &u_fpt,
        &[MarketBBFeedInput {
            symbol: "FPT",
            timeframe: Timeframe::D1,
            candles: &d1_bars,
        }],
        Some(UniverseMode::PointInTime),
        Some(day23_mid_cut),
        0.0,
        0.80,
    );
    let mbb_d1_atc = MarketBBAggregator::calculate_market_bb(
        &u_fpt,
        &[MarketBBFeedInput {
            symbol: "FPT",
            timeframe: Timeframe::D1,
            candles: &d1_bars,
        }],
        Some(UniverseMode::PointInTime),
        Some(day23_atc_cut),
        0.0,
        0.80,
    );
    let mbb_m15_mid = MarketBBAggregator::calculate_market_bb(
        &u_fpt,
        &[MarketBBFeedInput {
            symbol: "FPT",
            timeframe: Timeframe::M15,
            candles: &m15_bars,
        }],
        Some(UniverseMode::PointInTime),
        Some(day23_mid_cut),
        0.0,
        0.80,
    );
    assert_eq!(mbb_d1_mid.points.len(), 23);
    assert_eq!(mbb_m15_mid.points.len(), 23);
    assert_eq!(mbb_d1_atc.points.len(), 24);

    // 3. `[Blocker-01]` `PrecomputedCatalogMatrix` prefix causality when appending a trailing `NaN` bar!
    let mat_clean = PrecomputedCatalogMatrix::compute(&d1_bars);
    let mut d1_with_nan = d1_bars.clone();
    let mut nan_bar = d1_bars.last().unwrap().clone();
    nan_bar.timestamp += 86_400;
    nan_bar.high = f64::NAN;
    d1_with_nan.push(nan_bar);
    let mat_nan = PrecomputedCatalogMatrix::compute(&d1_with_nan);
    for cat_idx in 0..crate::scanner::SIGNAL_CATALOG.len() {
        for t in 0..d1_bars.len() {
            let p_a =
                mat_clean.point_at_with_profile(cat_idx, t, ConfluenceWeightProfile::Balanced);
            let p_b = mat_nan.point_at_with_profile(cat_idx, t, ConfluenceWeightProfile::Balanced);
            assert_eq!(p_a, p_b, "cat {cat_idx} bar {t} mutated by trailing NaN");
        }
    }

    // 4. `[Minor-02]` `UniverseResolver::validate_definition` catches bad weights & inverted top-level bounds!
    let mut bad_w = u_fpt.clone();
    bad_w.members[0].weight = f64::NAN;
    assert!(matches!(
        UniverseResolver::validate_definition(&bad_w),
        Err(UniverseError::InvalidMemberWeight { .. })
    ));
    bad_w.members[0].weight = 0.0;
    assert!(matches!(
        UniverseResolver::validate_definition(&bad_w),
        Err(UniverseError::InvalidMemberWeight { .. })
    ));
    let mut bad_top = u_fpt.clone();
    bad_top.effective_from = SessionDate::new(2026, 6, 1);
    bad_top.effective_to = SessionDate::new(2026, 1, 1);
    assert!(matches!(
        UniverseResolver::validate_definition(&bad_top),
        Err(UniverseError::InvertedUniverseInterval { .. })
    ));

    // 5. `[Minor-01]` & `[Minor-04]` `SuperTrend(10, 3.0)` catalog params & post-warmup `BBDirection::Unknown` parity!
    let st_entry = find_catalog_entry("technical.supertrend_flip_bull").unwrap();
    assert_eq!(st_entry.params.len(), 2);
    assert!((st_entry.params[0].default_value - 10.0).abs() < 1e-9);
    assert!((st_entry.params[1].default_value - 3.0).abs() < 1e-9);

    let sym_pbb = compute_proxy_bb_series(&d1_bars);
    assert_eq!(
        sym_pbb.points[2].horizon(BBHorizon::T03).direction,
        BBDirection::Unknown
    );
    assert_eq!(
        mbb_from_d1.points[2].horizon(BBHorizon::T03).direction,
        BBDirection::Unknown
    );
}
