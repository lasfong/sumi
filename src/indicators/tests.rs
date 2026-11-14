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

#[test]
fn r11_volume_profile_conserves_exact_total_buy_sell_volume_and_identifies_poc_vah_val() {
    let mut candles = generate_synthetic_ohlcv(80);
    // Inject an unmistakable high-volume cluster around price 125.0 so POC sits right inside it.
    let mid_bar = Candle {
        timestamp: candles[40].timestamp,
        date_label: candles[40].date_label.clone(),
        open: 124.0,
        high: 126.0,
        low: 124.0,
        close: 125.5,
        volume: 5_000_000.0,
    };
    candles[40] = mid_bar;

    let res = compute_volume_profile(
        &candles,
        VolumeProfileConfig {
            row_count: 48,
            value_area_pct: 0.70,
        },
    )
    .expect("valid volume profile");

    let expected_total_vol: f64 = candles.iter().map(|c| c.volume).sum();
    let sum_bin_tot: f64 = res.bins.iter().map(|b| b.total_volume).sum();
    let sum_bin_buy: f64 = res.bins.iter().map(|b| b.buy_volume).sum();
    let sum_bin_sell: f64 = res.bins.iter().map(|b| b.sell_volume).sum();

    assert!(
        (res.total_volume - expected_total_vol).abs() < 1e-6,
        "total volume conservation violated: {} vs {expected_total_vol}",
        res.total_volume
    );
    assert!(
        (sum_bin_tot - expected_total_vol).abs() < 1e-6,
        "bin total sum violated: {sum_bin_tot} vs {expected_total_vol}"
    );
    assert!(
        (sum_bin_buy + sum_bin_sell - expected_total_vol).abs() < 1e-6,
        "buy+sell sum violated: {} vs {expected_total_vol}",
        sum_bin_buy + sum_bin_sell
    );
    assert!((sum_bin_buy - res.total_buy_volume).abs() < 1e-6);
    assert!((sum_bin_sell - res.total_sell_volume).abs() < 1e-6);

    assert!(res.val_price <= res.poc_price && res.poc_price <= res.vah_price);
    let va_vol: f64 = res
        .bins
        .iter()
        .filter(|b| b.in_value_area)
        .map(|b| b.total_volume)
        .sum();
    assert!(
        va_vol >= 0.70 * res.total_volume - 1e-6,
        "Value Area must cover >=70% of volume ({va_vol} / {})",
        res.total_volume
    );

    // Zero-volume & flat-candle TPO fallback edge case must remain finite and valid.
    let flat_zero = vec![
        Candle {
            timestamp: 1_700_000_000,
            date_label: "2026-01-01".to_owned(),
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 0.0,
        };
        5
    ];
    let tpo = compute_volume_profile(&flat_zero, VolumeProfileConfig::default())
        .expect("flat zero-volume TPO fallback");
    assert!(tpo.poc_price.is_finite() && tpo.val_price <= tpo.vah_price);
}

#[test]
fn r11_session_vwap_resets_at_vietnam_utc7_day_boundary_and_monthly_on_d1() {
    // 2026-09-01 00:00:00 UTC:
    // At 16:45 UTC on 2026-09-01, UTC+7 is 23:45 (day 1 in VN).
    // At 17:15 UTC on 2026-09-01, UTC+7 is 00:15 on 2026-09-02 (new day in VN, SAME day in UTC!).
    let base_day_utc = 1_788_220_800_i64; // 2026-09-01 00:00:00 UTC
    let ts_2345_vn = base_day_utc + 16 * 3600 + 45 * 60;
    let ts_2359_vn = base_day_utc + 16 * 3600 + 59 * 60;
    let ts_0015_vn = base_day_utc + 17 * 3600 + 15 * 60;

    let bars = vec![
        Candle {
            timestamp: ts_2345_vn,
            date_label: "2026-09-01 23:45".to_owned(),
            open: 100.0,
            high: 102.0,
            low: 98.0,
            close: 100.0,
            volume: 10_000.0,
        },
        Candle {
            timestamp: ts_2359_vn,
            date_label: "2026-09-01 23:59".to_owned(),
            open: 110.0,
            high: 112.0,
            low: 108.0,
            close: 110.0,
            volume: 10_000.0,
        },
        Candle {
            timestamp: ts_0015_vn,
            date_label: "2026-09-02 00:15".to_owned(),
            open: 200.0,
            high: 201.0,
            low: 199.0,
            close: 200.0,
            volume: 5_000.0,
        },
    ];

    let vn_out = compute_session_vwap(&bars, Timeframe::M15, 25_200, 1.0, 2.0);
    assert_eq!(vn_out.session_starts, vec![0, 2]);
    // Bar 2 starts a fresh UTC+7 session: zero variance => upper_1 == lower_1 == vwap == 200.0!
    assert!((vn_out.vwap[2].unwrap() - 200.0).abs() < 1e-9);
    assert!((vn_out.upper_1[2].unwrap() - 200.0).abs() < 1e-9);
    assert!((vn_out.lower_1[2].unwrap() - 200.0).abs() < 1e-9);

    let utc_out = compute_session_vwap(&bars, Timeframe::M15, 0, 1.0, 2.0);
    assert_eq!(
        utc_out.session_starts,
        vec![0],
        "UTC+0 stays in same UTC day while UTC+7 resets!"
    );

    // D1 bars across Jan 30 -> Jan 31 -> Feb 01 reset on calendar month boundary.
    let jan30 = 1_769_731_200_i64; // 2026-01-30 00:00:00 UTC
    let d1_bars = vec![
        Candle {
            timestamp: jan30,
            date_label: "2026-01-30".to_owned(),
            open: 50.0,
            high: 52.0,
            low: 48.0,
            close: 50.0,
            volume: 1_000.0,
        },
        Candle {
            timestamp: jan30 + 86_400,
            date_label: "2026-01-31".to_owned(),
            open: 54.0,
            high: 55.0,
            low: 53.0,
            close: 54.0,
            volume: 1_000.0,
        },
        Candle {
            timestamp: jan30 + 2 * 86_400,
            date_label: "2026-02-01".to_owned(),
            open: 80.0,
            high: 81.0,
            low: 79.0,
            close: 80.0,
            volume: 1_000.0,
        },
    ];
    let d1_out = compute_session_vwap(&d1_bars, Timeframe::D1, 25_200, 1.0, 2.0);
    assert_eq!(d1_out.session_starts, vec![0, 2]);
    assert!((d1_out.vwap[2].unwrap() - 80.0).abs() < 1e-9);
}

#[test]
fn r11_anchored_vwap_masks_pre_anchor_bars_and_accumulates_welford_bands() {
    let bars = make_close_candles(&[100.0, 102.0, 105.0, 110.0, 106.0, 115.0]);
    let out = compute_anchored_vwap(&bars, 2, 1.0, 2.0);
    assert_eq!(out.session_starts, vec![2]);
    assert_eq!(out.vwap[0], None);
    assert_eq!(out.vwap[1], None);
    assert!((out.vwap[2].unwrap() - 105.0).abs() < 1e-9);
    assert!((out.upper_1[2].unwrap() - 105.0).abs() < 1e-9);
    assert!((out.lower_2[2].unwrap() - 105.0).abs() < 1e-9);

    for i in 3..bars.len() {
        let v = out.vwap[i].unwrap();
        let u1 = out.upper_1[i].unwrap();
        let l1 = out.lower_1[i].unwrap();
        let u2 = out.upper_2[i].unwrap();
        let l2 = out.lower_2[i].unwrap();
        assert!(l2 <= l1 && l1 <= v && v <= u1 && u1 <= u2);
        assert!(u1 > v && l1 < v);
    }
}

#[test]
fn r11_relative_strength_enforces_def001_bar_close_causality_and_holiday_carry_forward() {
    let day_0 = 1_788_220_800_i64; // 2026-09-01 00:00:00 UTC
    // D1 benchmark closes: day 0 = 1000, day 1 = 1500 (huge spike), day 2 omitted (holiday!), day 3 = 1200.
    let bench = vec![
        Candle {
            timestamp: day_0,
            date_label: "2026-09-01".to_owned(),
            open: 1000.0,
            high: 1010.0,
            low: 990.0,
            close: 1000.0,
            volume: 1000.0,
        },
        Candle {
            timestamp: day_0 + 86_400,
            date_label: "2026-09-02".to_owned(),
            open: 1000.0,
            high: 1510.0,
            low: 990.0,
            close: 1500.0,
            volume: 1000.0,
        },
        Candle {
            timestamp: day_0 + 3 * 86_400,
            date_label: "2026-09-04".to_owned(),
            open: 1200.0,
            high: 1210.0,
            low: 1190.0,
            close: 1200.0,
            volume: 1000.0,
        },
    ];

    // Intraday H1 bars:
    // bar 0: day 1 03:15 UTC (10:15 VN) -> day 1 D1 has NOT closed yet -> must align to day 0 close (1000)!
    // bar 1: day 2 10:00 UTC (holiday on benchmark) -> must carry forward day 1 D1 close (1500)!
    // bar 2: day 3 23:00 UTC -> after day 3 close minus 1s -> aligns to day 3 close (1200) unless clipped by replay!
    let sym = vec![
        Candle {
            timestamp: day_0 + 86_400 + 3 * 3600,
            date_label: "2026-09-02 03:00".to_owned(),
            open: 50.0,
            high: 51.0,
            low: 49.0,
            close: 50.0,
            volume: 500.0,
        },
        Candle {
            timestamp: day_0 + 2 * 86_400 + 10 * 3600,
            date_label: "2026-09-03 10:00".to_owned(),
            open: 75.0,
            high: 76.0,
            low: 74.0,
            close: 75.0,
            volume: 500.0,
        },
        Candle {
            timestamp: day_0 + 3 * 86_400 + 23 * 3600,
            date_label: "2026-09-04 23:00".to_owned(),
            open: 60.0,
            high: 61.0,
            low: 59.0,
            close: 60.0,
            volume: 500.0,
        },
    ];

    let full = compute_relative_strength(&sym, Timeframe::H1, &bench, Timeframe::D1, 2, None);
    assert_eq!(full.aligned_benchmark_close[0], Some(1000.0));
    assert_eq!(full.aligned_benchmark_close[1], Some(1500.0));
    assert_eq!(full.aligned_benchmark_close[2], Some(1200.0));
    // All 3 bars have exact ratio (50/1000) == (75/1500) == (60/1200) -> RS == 100.0 throughout!
    for v in &full.rs_line {
        assert!((v.unwrap() - 100.0).abs() < 1e-9);
    }
    assert!((full.rs_sma[1].unwrap() - 100.0).abs() < 1e-9);

    // With causal replay cutoff before day 3 D1 completes, bar 2 cannot see benchmark day 3 (1200.0).
    let replay_cut = day_0 + 3 * 86_400 + 12 * 3600;
    let replayed = compute_relative_strength(
        &sym,
        Timeframe::H1,
        &bench,
        Timeframe::D1,
        2,
        Some(replay_cut),
    );
    assert_ne!(replayed.aligned_benchmark_close[2], Some(1200.0));
    assert_eq!(replayed.aligned_benchmark_close[2], Some(1500.0));
}

#[test]
fn critic_r11_regression_session_bucket_d1_w1_negative_utc_offset() {
    // B2: `D1`/`W1` session resets must use calendar UTC date regardless of negative `tz_offset`
    // (previously `2024-02-01T00:00Z + (-5h)` shifted back into `2024-01-31` and delayed the monthly reset!).
    let base_ts = 1_704_067_200_i64; // 2024-01-01T00:00:00Z (leap year Jan 1)
    let d1_candles: Vec<Candle> = [0_i64, 30, 31, 59, 60] // Jan 1, Jan 31, Feb 1, Feb 29, Mar 1
        .into_iter()
        .enumerate()
        .map(|(i, day)| Candle {
            timestamp: base_ts + day * 86_400,
            date_label: format!("d{i}"),
            open: 100.0 + i as f64,
            high: 102.0 + i as f64,
            low: 99.0 + i as f64,
            close: 101.0 + i as f64,
            volume: 50_000.0,
        })
        .collect();

    let vwap_utc_neg5 = compute_session_vwap(&d1_candles, Timeframe::D1, -18_000, 1.0, 2.0);
    let vwap_utc_pos7 = compute_session_vwap(&d1_candles, Timeframe::D1, 25_200, 1.0, 2.0);
    assert_eq!(vwap_utc_neg5.session_starts, vec![0, 2, 4]);
    assert_eq!(vwap_utc_pos7.session_starts, vec![0, 2, 4]);

    // `W1` resets yearly (YTD VWAP): check 2023-12-25 (`base_ts - 7*86400`) -> 2024-01-01 (`base_ts`).
    let w1_candles = vec![
        Candle {
            timestamp: base_ts - 7 * 86_400,
            date_label: "2023-12-25".to_owned(),
            open: 100.0,
            high: 102.0,
            low: 99.0,
            close: 101.0,
            volume: 50_000.0,
        },
        Candle {
            timestamp: base_ts, // 2024-01-01T00:00Z -> must reset at idx 1 even at tz = -5h!
            date_label: "2024-01-01".to_owned(),
            open: 103.0,
            high: 104.0,
            low: 102.0,
            close: 103.5,
            volume: 60_000.0,
        },
    ];
    let w1_utc_neg5 = compute_session_vwap(&w1_candles, Timeframe::W1, -18_000, 1.0, 2.0);
    let w1_utc_pos7 = compute_session_vwap(&w1_candles, Timeframe::W1, 25_200, 1.0, 2.0);
    assert_eq!(w1_utc_neg5.session_starts, vec![0, 1]);
    assert_eq!(w1_utc_pos7.session_starts, vec![0, 1]);

    // Meanwhile intraday `M15` STILL honors negative UTC offset local midnight (`05:00Z` in `UTC-5`).
    let m15_base = base_ts + 4 * 3_600 + 45 * 60; // 04:45Z (= 23:45 UTC-5 Jan 1)
    let m15_candles = vec![
        Candle {
            timestamp: m15_base,
            date_label: "04:45Z".to_owned(),
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.0,
            volume: 100.0,
        },
        Candle {
            timestamp: m15_base + 900, // 05:00Z (= 00:00 UTC-5 Jan 2 -> session reset!)
            date_label: "05:00Z".to_owned(),
            open: 12.0,
            high: 13.0,
            low: 11.0,
            close: 12.0,
            volume: 100.0,
        },
    ];
    let m15_neg5 = compute_session_vwap(&m15_candles, Timeframe::M15, -18_000, 1.0, 2.0);
    assert_eq!(m15_neg5.session_starts, vec![0, 1]);
}

#[test]
fn critic_r11_regression_vwap_extreme_magnitude_no_inf_or_nan_collapse() {
    // B3: Extreme high (`1e160`) and sub-normal/tiny (`1e-160`) price/volume regimes must never
    // overflow `(w * p)` or `delta^2` into Inf/NaN bands in Session VWAP or Anchored VWAP.
    for &(price_scale, vol_scale) in &[(1.0e160_f64, 1.0e160_f64), (1.0e-160_f64, 1.0e-160_f64)] {
        let candles: Vec<Candle> = (0..12)
            .map(|i| {
                let mult = 1.0 + ((i % 4) as f64) * 0.05;
                Candle {
                    timestamp: 1_700_000_000 + (i as i64) * 3_600,
                    date_label: format!("b{i}"),
                    open: price_scale * mult,
                    high: price_scale * (mult + 0.02),
                    low: price_scale * (mult - 0.02),
                    close: price_scale * mult,
                    volume: vol_scale * (1.0 + (i as f64) * 0.5),
                }
            })
            .collect();

        for out in [
            compute_session_vwap(&candles, Timeframe::D1, 25_200, 1.0, 2.0),
            compute_anchored_vwap(&candles, 0, 1.0, 2.0),
        ] {
            for i in 0..candles.len() {
                let v = out.vwap[i].expect("finite vwap");
                let u1 = out.upper_1[i].expect("finite upper_1");
                let l1 = out.lower_1[i].expect("finite lower_1");
                let u2 = out.upper_2[i].expect("finite upper_2");
                let l2 = out.lower_2[i].expect("finite lower_2");
                assert!(
                    v.is_finite()
                        && u1.is_finite()
                        && l1.is_finite()
                        && u2.is_finite()
                        && l2.is_finite()
                );
                assert!(l2 <= l1 + 1e-12 * price_scale.abs());
                assert!(l1 <= v + 1e-12 * price_scale.abs());
                assert!(v <= u1 + 1e-12 * price_scale.abs());
                assert!(u1 <= u2 + 1e-12 * price_scale.abs());
            }
        }
    }
}

#[test]
fn critic_r11_regression_vp_micro_price_and_dust_volume_va() {
    // B3: Micro-prices (`~1e-9`) must keep positive bin bounds (`price_low > 0`), and dust volumes
    // (`1e-18`) must expand Value Area properly instead of collapsing onto POC via absolute `f64::EPSILON`.
    let micro: Vec<Candle> = (0..16)
        .map(|i| {
            let base = 1.0e-9 + (i as f64) * 5.0e-11;
            Candle {
                timestamp: 1_700_000_000 + (i as i64) * 86_400,
                date_label: format!("m{i}"),
                open: base,
                high: base + 1.0e-11,
                low: (base - 5.0e-12).max(1.0e-12),
                close: base + 4.0e-12,
                volume: 10_000.0,
            }
        })
        .collect();
    let micro_res = compute_volume_profile(
        &micro,
        VolumeProfileConfig {
            row_count: 16,
            value_area_pct: 0.70,
        },
    )
    .expect("micro price volume profile");
    assert!(
        micro_res
            .bins
            .iter()
            .all(|b| b.price_low > 0.0 && b.price_high > b.price_low)
    );
    assert!(
        micro_res.poc_price > 0.0
            && micro_res.val_price > 0.0
            && micro_res.vah_price >= micro_res.val_price
    );

    let dust: Vec<Candle> = (0..10)
        .map(|i| {
            let p = 100.0 + (i as f64) * 2.0;
            Candle {
                timestamp: 1_700_000_000 + (i as i64) * 86_400,
                date_label: format!("u{i}"),
                open: p,
                high: p + 0.5,
                low: p - 0.5,
                close: p,
                volume: 1.0e-18,
            }
        })
        .collect();
    let dust_res = compute_volume_profile(
        &dust,
        VolumeProfileConfig {
            row_count: 10,
            value_area_pct: 0.70,
        },
    )
    .expect("dust volume profile");
    assert!(
        dust_res.vah_price > dust_res.val_price + 4.0,
        "dust VA collapsed: VAL={} VAH={}",
        dust_res.val_price,
        dust_res.vah_price
    );
}

#[test]
fn critic_r11_regression_rs_sma_spike_drift_and_benchmark_coverage() {
    // M5 (a): Extreme spike in RS series must not leave permanent floating subtraction drift in `rs_sma`.
    let n = 260_usize;
    let bench: Vec<Candle> = (0..n)
        .map(|i| Candle {
            timestamp: 1_700_000_000 + (i as i64) * 86_400,
            date_label: format!("b{i}"),
            open: 1000.0,
            high: 1005.0,
            low: 995.0,
            close: 1000.0,
            volume: 1_000_000.0,
        })
        .collect();
    let mut sym = bench.clone();
    sym[10].close = 1.0e14; // extreme spike early in series
    let rs_out = compute_relative_strength(&sym, Timeframe::D1, &bench, Timeframe::D1, 10, None);
    let tail_sma = rs_out.rs_sma[n - 1].expect("tail rs sma");
    assert!(
        (tail_sma - 100.0).abs() < 1.0e-6,
        "rolling RS SMA suffered catastrophic spike subtraction drift: {tail_sma}"
    );

    // M5 (b): Watchlist benchmark resolver must reject high-row-density stale archives that stop early
    // before `ref_last_ts` and select the candidate covering the full reference window.
    use crate::data::{DataSourcePreset, WatchlistFeedEntry, resolve_benchmark_series_for_window};
    let t0 = 1_700_000_000_i64;
    let t_last = t0 + 120 * 86_400;
    let stale_dense: Vec<Candle> = (0..500)
        .map(|i| Candle {
            timestamp: t0 + (i as i64) * 3_600, // ends ~20 days in, 100 days before t_last!
            date_label: format!("s{i}"),
            open: 1200.0,
            high: 1205.0,
            low: 1195.0,
            close: 1200.0,
            volume: 1000.0,
        })
        .collect();
    let full_cov: Vec<Candle> = (0..=120)
        .map(|d| Candle {
            timestamp: t0 + d * 86_400,
            date_label: format!("f{d}"),
            open: 1300.0,
            high: 1305.0,
            low: 1295.0,
            close: 1300.0,
            volume: 1000.0,
        })
        .collect();
    let universe = vec![
        WatchlistFeedEntry {
            instrument_id: "vnindex_stale".to_owned(),
            preset: DataSourcePreset::SyntheticVnindex,
            symbol: "VNINDEX".to_owned(),
            base_timeframe: Timeframe::H1,
            candles: stale_dense,
        },
        WatchlistFeedEntry {
            instrument_id: "vnindex_full".to_owned(),
            preset: DataSourcePreset::SyntheticVnindex,
            symbol: "VNINDEX".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: full_cov,
        },
    ];
    let (_, tf, resolved) = resolve_benchmark_series_for_window("VNINDEX", t0, t_last, &universe)
        .expect("resolve benchmark");
    assert_eq!(tf, Timeframe::D1);
    assert!(resolved.last().unwrap().timestamp >= t_last - 86_400);
}
