use crate::data::generate_synthetic_ohlcv;
use crate::model::Candle;

use super::*;

fn make_close_candles(closes: &[f64]) -> Vec<Candle> {
    closes
        .iter()
        .enumerate()
        .map(|(i, &c)| Candle {
            timestamp: 1_700_000_000 + (i as i64) * 86_400,
            date_label: format!("2026-01-{:02}", (i % 28) + 1),
            open: c,
            high: c + 2.0,
            low: c - 2.0,
            close: c,
            volume: 100_000.0,
        })
        .collect()
}

#[test]
fn sma_basic_and_long_series_drift_invariance() {
    // Exact CHART_ENGINE_RESEARCH_MAP.md §Subsystem 3 acceptance fixture:
    // SMA(5) of [1, 2, 3, 4, 5, 6, 7] == [None, None, None, None, 3.0, 4.0, 5.0]
    let out = compute_sma_slices(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0], 5);
    assert_eq!(
        out,
        vec![None, None, None, None, Some(3.0), Some(4.0), Some(5.0)]
    );

    // Period 1 copies finite inputs directly (`ta_SMA.c` / `ta_EMA.c`)
    assert_eq!(
        compute_sma_slices(&[10.5, 20.5], 1),
        vec![Some(10.5), Some(20.5)]
    );
    assert_eq!(compute_sma_slices(&[10.0, 20.0], 0), vec![None, None]);
    assert_eq!(compute_sma_slices(&[10.0, 20.0], 5), vec![None, None]);

    // 10,000 flat bars after a 1e12 spike must match an unspiked window sum/N (zero subtraction drift;
    // a naive sliding sum without 32*N reseed loses 11 digits of mantissa ~0.005 here!)
    let mut series = vec![1234.56; 10_000];
    series[10] = 1.0e12;
    let sma20 = compute_sma_slices(&series, 20);
    let exact_flat_mean = [1234.56_f64; 20].iter().sum::<f64>() / 20.0;
    assert_eq!(sma20[9_999], Some(exact_flat_mean));
    assert!((sma20[9_999].unwrap() - 1234.56).abs() < 1e-12);
}

#[test]
fn ema_warm_up_seed_and_zero_ulp_flat_invariance() {
    // EMA(5) on [2, 4, 6, 8, 10, 12, 14]:
    // k = 2 / 6 = 1/3; seed at idx 4 = (2+4+6+8+10)/5 = 6.0
    // idx 5: 6.0 + (1/3)*(12 - 6) = 8.0
    // idx 6: 8.0 + (1/3)*(14 - 8) = 10.0
    let ema = compute_ema_slices(&[2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0], 5);
    assert_eq!(ema[..4], [None, None, None, None]);
    assert!((ema[4].unwrap() - 6.0).abs() < 1e-12);
    assert!((ema[5].unwrap() - 8.0).abs() < 1e-12);
    assert!((ema[6].unwrap() - 10.0).abs() < 1e-12);

    // Zero-ULP flat invariance across 100,000 bars
    let flat = vec![1285.75_f64; 100_000];
    let ema_flat = compute_ema_slices(&flat, 20);
    assert_eq!(ema_flat[19], Some(1285.75));
    assert_eq!(ema_flat[99_999], Some(1285.75));
}

#[test]
fn rsi_wilder_reference_and_ta_lib_edge_cases() {
    // Classic Wilder 14-period RSI golden dataset (first 16 closes of Wilder 1978 / StockCharts table)
    let closes = [
        44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03, 45.61,
        46.28, 46.28, 46.00, 46.03, 46.41, 46.22, 45.64,
    ];
    let rsi = compute_rsi_slices(&closes, 14);
    assert!(rsi[..14].iter().all(Option::is_none));
    // Seed at index 14: sum_gain = 0.06+0.72+0.50+0.27+0.32+0.42+0.24+0.14+0.67 = 3.34 -> avg_gain = 3.34/14
    // sum_loss = 0.25+0.54+0.19+0.42 = 1.40 -> avg_loss = 1.40/14
    // RSI[14] = 100 * 3.34 / (3.34 + 1.40) = 334 / 4.74 = 70.46413502109705
    assert!((rsi[14].unwrap() - 70.464_135_021).abs() < 1e-6);
    // Index 15: delta = -0.28 -> avg_gain = 43.42 / 196, avg_loss = 22.12 / 196
    // RSI[15] = 100 * 43.42 / 65.54 = 66.24961855355508
    assert!((rsi[15].unwrap() - (4_342.0 / 65.54)).abs() < 1e-12);

    // TA-Lib #480: flat price series (0 gain, 0 loss) MUST answer neutral 50.0 (never NaN or 0.0)
    let flat_rsi = compute_rsi_slices(&[100.0; 20], 14);
    assert_eq!(flat_rsi[14], Some(50.0));
    assert_eq!(flat_rsi[19], Some(50.0));

    // TA-Lib #253: tiny sub-epsilon micro-cap/crypto prices (`1e-12`) must NOT be zeroed out!
    let micro: Vec<f64> = (0..20).map(|i| 1e-12 * (1.0 + (i as f64) * 0.1)).collect();
    let micro_rsi = compute_rsi_slices(&micro, 14);
    assert_eq!(micro_rsi[14], Some(100.0));

    // Monotonically falling series -> 0.0
    let falling: Vec<f64> = (0..20).map(|i| 200.0 - (i as f64)).collect();
    let fall_rsi = compute_rsi_slices(&falling, 14);
    assert_eq!(fall_rsi[14], Some(0.0));
}

#[test]
fn bollinger_population_stddev_and_volatility_expansion() {
    // Exact population stddev check on window [2, 4, 4, 4, 5, 5, 7, 9] (N = 8):
    // mean = 5.0, population variance = (9 + 1 + 1 + 1 + 0 + 0 + 4 + 16) / 8 = 32 / 8 = 4.0 -> sigma = 2.0!
    // With std_mult = 2.0: middle = 5.0, upper = 9.0, lower = 1.0
    let bb = compute_bollinger_slices(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0], 8, 2.0);
    assert!(bb.middle[..7].iter().all(Option::is_none));
    assert!((bb.middle[7].unwrap() - 5.0).abs() < 1e-12);
    assert!((bb.upper[7].unwrap() - 9.0).abs() < 1e-12);
    assert!((bb.lower[7].unwrap() - 1.0).abs() < 1e-12);

    // Flat series followed by volatility expansion
    let mut series = vec![100.0; 25];
    for i in 25..45 {
        series.push(if i % 2 == 0 { 115.0 } else { 85.0 });
    }
    let bb20 = compute_bollinger_slices(&series, 20, 2.0);
    let flat_width = bb20.upper[24].unwrap() - bb20.lower[24].unwrap();
    let vol_width = bb20.upper[44].unwrap() - bb20.lower[44].unwrap();
    assert_eq!(flat_width, 0.0);
    assert!((vol_width - 60.0).abs() < 1e-9); // sigma = 15.0 -> width = 4 * sigma = 60.0
}

#[test]
fn macd_alignment_period_swap_and_signal_one_branch() {
    let closes: Vec<f64> = (0..60)
        .map(|i| 100.0 + (i as f64) * 0.8 + ((i as f64) * 0.3).sin() * 3.0)
        .collect();
    let m1 = compute_macd_slices(&closes, 12, 26, 9);
    let m_swapped = compute_macd_slices(&closes, 26, 12, 9);
    assert_eq!(m1, m_swapped, "slow < fast must swap cleanly per ta_MACD.c");

    // Warmup exactness: macd_line starts at 25 (slow - 1), signal & histogram start at 33 (25 + 8)
    assert!(m1.macd_line[..25].iter().all(Option::is_none));
    assert!(m1.macd_line[25].is_some());
    assert!(m1.signal_line[..33].iter().all(Option::is_none));
    assert!(m1.signal_line[33].is_some());
    assert!(m1.histogram[33].is_some());

    // Exact identity: macd_line == EMA(12) - EMA(26) and histogram == macd - signal
    let ema12 = compute_ema_slices(&closes, 12);
    let ema26 = compute_ema_slices(&closes, 26);
    for i in 25..60 {
        let expected_m = ema12[i].unwrap() - ema26[i].unwrap();
        assert!((m1.macd_line[i].unwrap() - expected_m).abs() < 1e-12);
    }
    for i in 33..60 {
        let expected_h = m1.macd_line[i].unwrap() - m1.signal_line[i].unwrap();
        assert!((m1.histogram[i].unwrap() - expected_h).abs() < 1e-12);
    }

    // Signal period == 1 -> signal == macd and histogram == 0.0 (`ta_MACD.c:212–215`)
    let m_sig1 = compute_macd_slices(&closes, 12, 26, 1);
    assert_eq!(m_sig1.histogram[25], Some(0.0));
    assert_eq!(m_sig1.signal_line[25], m_sig1.macd_line[25]);
}

#[test]
fn atr_true_range_gaps_and_wilder_smoothing() {
    // Construct 5 candles with known gaps so TR[1..=3] = [10.0, 14.0, 6.0] -> ATR(3) at idx 3 = 10.0
    // idx 4: TR[4] = 16.0 -> ATR[4] = (10.0 * 2 + 16.0) / 3 = 12.0
    let candles = vec![
        Candle {
            timestamp: 1,
            date_label: "D0".into(),
            open: 100.0,
            high: 105.0,
            low: 95.0,
            close: 100.0,
            volume: 1000.0,
        },
        // Gap up: high=108, low=102, prev_close=100 -> max(6, |100-108|=8, |100-102|=2) -> wait: make high=110, low=102 -> TR = 10.0
        Candle {
            timestamp: 2,
            date_label: "D1".into(),
            open: 103.0,
            high: 110.0,
            low: 102.0,
            close: 108.0,
            volume: 1000.0,
        },
        // Gap down: high=104, low=94, prev_close=108 -> max(10, |108-104|=4, |108-94|=14) -> TR = 14.0
        Candle {
            timestamp: 3,
            date_label: "D2".into(),
            open: 100.0,
            high: 104.0,
            low: 94.0,
            close: 96.0,
            volume: 1000.0,
        },
        // Inside bar: high=100, low=94, prev_close=96 -> max(6, 4, 2) -> TR = 6.0
        Candle {
            timestamp: 4,
            date_label: "D3".into(),
            open: 96.0,
            high: 100.0,
            low: 94.0,
            close: 98.0,
            volume: 1000.0,
        },
        // Wide bar: high=110, low=94, prev_close=98 -> max(16, 12, 4) -> TR = 16.0
        Candle {
            timestamp: 5,
            date_label: "D4".into(),
            open: 98.0,
            high: 110.0,
            low: 94.0,
            close: 105.0,
            volume: 1000.0,
        },
    ];

    let atr = compute_atr(&candles, 3);
    assert_eq!(atr[..3], [None, None, None]);
    assert!((atr[3].unwrap() - 10.0).abs() < 1e-12);
    assert!((atr[4].unwrap() - 12.0).abs() < 1e-12);
}

#[test]
fn non_finite_bars_recover_cleanly_and_polymorphic_trait_works() {
    let mut closes = vec![100.0, 102.0, 104.0, f64::NAN, 110.0, 112.0, 114.0, 116.0];
    closes.push(118.0);
    let candles = make_close_candles(&closes);

    let indicators: Vec<Box<dyn Indicator>> = vec![
        Box::new(SmaIndicator::new(3)),
        Box::new(EmaIndicator::new(3)),
        Box::new(BollingerBandsIndicator::new(3, 2.0)),
        Box::new(RsiIndicator::new(3)),
        Box::new(MacdIndicator::new(2, 3, 2)),
        Box::new(AtrIndicator::new(3)),
    ];

    for ind in &indicators {
        let out = ind.evaluate(&candles);
        assert!(!out.series.is_empty());
        for s in &out.series {
            assert_eq!(s.values.len(), candles.len());
            assert_eq!(
                s.values[3], None,
                "{}::{} must emit None on NaN bar",
                out.id, s.key
            );
            assert!(
                s.values[8].is_some_and(f64::is_finite),
                "{}::{} must recover cleanly after NaN bar",
                out.id,
                s.key
            );
        }
    }
}

#[test]
fn large_100k_cache_and_right_aligned_conflation_pan_invariance() {
    let candles = generate_synthetic_ohlcv(100_000);
    let mut cfg = IndicatorConfig {
        show_sma: true,
        show_ema: true,
        show_bbands: true,
        show_rsi: true,
        show_macd: true,
        show_atr: true,
        ..IndicatorConfig::default()
    };
    cfg.sanitize_deserialized();

    let mut cache = IndicatorCache::new();
    cache.sync(&candles, 1, &cfg);

    assert!(cache.sma[99_999].is_some_and(f64::is_finite));
    assert!(cache.ema[99_999].is_some_and(f64::is_finite));
    assert!(cache.bbands.upper[99_999].is_some_and(f64::is_finite));
    assert!(cache.rsi[99_999].is_some_and(|v| (0.0..=100.0).contains(&v)));
    assert!(cache.macd.histogram[99_999].is_some_and(f64::is_finite));
    assert!(cache.atr[99_999].is_some_and(|v| v > 0.0));

    // Right-aligned conflated line & histogram buckets must NOT shift when start_idx pans by 1 bar!
    let mut pts_a = Vec::new();
    let mut pts_b = Vec::new();
    visit_conflated_line_segments(&cache.rsi, 1000..99_999, 32, |s| {
        if let Some(p) = s {
            pts_a.push(p);
        }
    });
    visit_conflated_line_segments(&cache.rsi, 1001..99_999, 32, |s| {
        if let Some(p) = s {
            pts_b.push(p);
        }
    });
    assert_eq!(
        pts_a.last(),
        pts_b.last(),
        "rightmost conflated line bucket must be 100% invariant across 1-bar pan"
    );
    assert!(pts_a.len() <= 6_500);

    let mut hist_a = Vec::new();
    let mut hist_b = Vec::new();
    visit_conflated_histogram_bars(&cache.macd.histogram, 1000..99_999, 32, |b| hist_a.push(b));
    visit_conflated_histogram_bars(&cache.macd.histogram, 1001..99_999, 32, |b| hist_b.push(b));
    assert_eq!(hist_a.last(), hist_b.last());
    assert!(hist_a.len() <= 3_200);
}

#[test]
fn series_block_index_matches_raw_slice_scans_bit_for_bit() {
    // Non-multiple-of-16 length (4,123 bars) with intentional None gaps to stress partial tail & gap blocks!
    let candles = generate_synthetic_ohlcv(4_123);
    let mut rsi = compute_rsi(&candles, 14);
    rsi[800..850].fill(None);
    let block_idx = SeriesBlockIndex::build(&rsi);

    for range in [
        0..4_123,
        14..4_123,
        37..3_999,
        805..845,
        790..910,
        4_100..4_123,
    ] {
        assert_eq!(
            block_idx.range_bounds(&rsi, range.clone()),
            visible_series_bounds(&[&rsi], range.clone()),
            "range_bounds mismatch on {range:?}"
        );

        for step in [1, 4, 16, 32, 64, 128] {
            let mut raw_line = Vec::new();
            let mut idx_line = Vec::new();
            visit_conflated_line_segments(&rsi, range.clone(), step, |s| raw_line.push(s));
            visit_conflated_line_segments_indexed(
                &rsi,
                Some(&block_idx),
                range.clone(),
                step,
                |s| {
                    idx_line.push(s);
                },
            );
            assert_eq!(
                idx_line, raw_line,
                "indexed line conflation mismatch on range={range:?}, step={step}"
            );

            let mut raw_hist = Vec::new();
            let mut idx_hist = Vec::new();
            visit_conflated_histogram_bars(&rsi, range.clone(), step, |b| raw_hist.push(b));
            visit_conflated_histogram_bars_indexed(
                &rsi,
                Some(&block_idx),
                range.clone(),
                step,
                |b| {
                    idx_hist.push(b);
                },
            );
            assert_eq!(
                idx_hist, raw_hist,
                "indexed histogram conflation mismatch on range={range:?}, step={step}"
            );
        }
    }
}

#[test]
fn critic_r04_regression_bollinger_extreme_magnitude_and_period_domain() {
    // 1. `period < 2` must return empty `None` bands matching `ta_BBANDS.c:157` & `IndicatorConfig`
    let bb_p1 = compute_bollinger_slices(&[10.0, 20.0, 30.0], 1, 2.0);
    assert!(bb_p1.middle.iter().all(Option::is_none));
    assert_eq!(BollingerBandsIndicator::new(1, 2.0).warmup_bars(), 1);
    assert_eq!(RsiIndicator::new(1).warmup_bars(), 2);

    // 2. `|w - mean| > sqrt(f64::MAX)` (`2.0e154`): `d * d` overflows `f64::MAX`, yet true `mean` and `sigma`
    // are finite (`mean = 0.5e154`, `sigma = 0.8660254037844386e154`). Must NEVER emit zero-width bands,
    // and `peak_sum2` must NEVER stick at `+Inf` after the spike leaves the window!
    let data = [0.0, 2.0e154, 0.0, 0.0, 10.0, 10.0, 10.0, 10.0];
    let bb = compute_bollinger_slices(&data, 4, 2.0);
    let m3 = bb.middle[3].expect("finite mean at idx 3");
    let u3 = bb.upper[3].expect("finite upper at idx 3");
    let l3 = bb.lower[3].expect("finite lower at idx 3");
    let expected_sigma = 1.5e154 * (1.0_f64 / 3.0).sqrt();
    assert!((m3 - 0.5e154).abs() <= 1e140);
    assert!(((u3 - m3) - 2.0 * expected_sigma).abs() <= 1e140 * 8.0);
    assert!(
        u3 > m3 && l3 < m3,
        "bands must never collapse to zero width on overflow"
    );
    assert_eq!(bb.middle[7], Some(10.0));
    assert_eq!(bb.upper[7], Some(10.0));
    assert_eq!(bb.lower[7], Some(10.0));

    // 3. Opposite-sign `±1e308` with `std_mult = 2.0`: `upper = 2e308 > f64::MAX` must yield `None`, never `Some(0.0)`
    let bb_inf = compute_bollinger_slices(&[-1.0e308, 1.0e308], 2, 2.0);
    assert_eq!(bb_inf.middle[1], None);
    assert_eq!(bb_inf.upper[1], None);
    assert_eq!(bb_inf.lower[1], None);
}

#[test]
fn critic_r04_regression_high_magnitude_overflow_sma_ema_rsi_and_sterbenz() {
    // 1. Mixed-sign extreme SMA window followed by normal values recovers finite sliding sum
    let sma = compute_sma_slices(&[1.0e308, 1.0e308, -1.0e308, 10.0, 20.0, 30.0], 3);
    assert!(sma[2].is_some_and(f64::is_finite));
    assert!((sma[5].unwrap() - 20.0).abs() < 1e-12);

    // 2. `compute_ema_optional_slice` high-magnitude seed (`9 * 3e307 = 2.7e308 > f64::MAX`)
    let opt_high = vec![Some(3.0e307); 12];
    let ema_opt = compute_ema_optional_slice(&opt_high, 9);
    assert!(ema_opt[..8].iter().all(Option::is_none));
    for v in &ema_opt[8..] {
        let x = v.expect("scaled seed must not overflow");
        assert!((x - 3.0e307).abs() <= 1e293);
    }

    // 3. Sterbenz `EMA(2)` opposite-sign step (`delta = val - prev` overflows `f64::MAX`)
    let ema2 = compute_ema_slices(&[-1.0e308, -1.0e308, 1.0e308], 2);
    assert_eq!(ema2[1], Some(-1.0e308));
    let expected_step = (-1.0e308 / 3.0) + 2.0 * (1.0e308 / 3.0);
    assert!((ema2[2].unwrap() - expected_step).abs() <= 1e293);

    // 4. High-magnitude Wilder `RSI(14)` (`avg_gain * 13` > `f64::MAX`) stays finite in `[0, 100]`
    let alt: Vec<f64> = (0..24)
        .map(|i| if i % 2 == 0 { 0.0 } else { 5.0e307 })
        .collect();
    let rsi = compute_rsi_slices(&alt, 14);
    for (idx, val) in rsi.iter().enumerate().skip(14) {
        let r = val.unwrap_or_else(|| panic!("RSI[{idx}] overflowed to None"));
        assert!((0.0..=100.0).contains(&r));
    }
}

#[test]
fn critic_r04_regression_sub_bucket_gap_breaks_conflated_segments() {
    let mut s: Vec<Option<f64>> = (0..128).map(|i| Some(100.0 + (i as f64) * 0.5)).collect();
    // Boundary single-bar gap at bucket start `48`, plus internal single-bar gap at `85`
    s[48] = None;
    s[85] = None;
    let bi = SeriesBlockIndex::build(&s);

    for step in [16, 32] {
        let mut raw = Vec::new();
        let mut indexed = Vec::new();
        visit_conflated_line_segments(&s, 0..128, step, |pt| raw.push(pt));
        visit_conflated_line_segments_indexed(&s, Some(&bi), 0..128, step, |pt| indexed.push(pt));
        assert_eq!(indexed, raw, "indexed vs raw mismatch at step={step}");
        let gap_breaks = indexed.iter().filter(|pt| pt.is_none()).count();
        assert!(
            gap_breaks >= 2,
            "expected >= 2 segment breaks for boundary + internal None gaps at step={step}, got {gap_breaks}"
        );
    }
}
