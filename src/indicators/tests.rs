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
        Box::new(AdxIndicator { period: 2 }),
        Box::new(StochasticIndicator {
            k_period: 2,
            k_smooth: 2,
            d_period: 2,
        }),
        Box::new(SuperTrendIndicator {
            period: 2,
            multiplier: 3.0,
        }),
        Box::new(PsarIndicator {
            af_start: 0.02,
            af_step: 0.02,
            af_max: 0.20,
        }),
        Box::new(KeltnerIndicator {
            ema_period: 2,
            atr_period: 2,
            multiplier: 2.0,
        }),
        Box::new(CciIndicator { period: 3 }),
        Box::new(MfiIndicator { period: 3 }),
        Box::new(VolumeSmaIndicator { period: 3 }),
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

#[test]
fn r12_ichimoku_math_displacement_and_def001_causality() {
    use super::ichimoku::{IchimokuConfig, compute_ichimoku, donchian_midpoint_series};

    let empty = compute_ichimoku(&[], IchimokuConfig::default());
    assert!(empty.tenkan.is_empty() && empty.senkou_a.is_empty());

    let short = compute_ichimoku(
        &make_close_candles(&[10.0, 12.0, 11.0]),
        IchimokuConfig::default(),
    );
    assert!(short.tenkan.iter().all(Option::is_none));
    assert_eq!(short.senkou_a.len(), 3 + 26);
    assert!(short.chikou.iter().all(Option::is_none));

    let candles: Vec<Candle> = (0..110)
        .map(|i| {
            let base = 100.0 + (i as f64) * 1.2 + ((i % 7) as f64) * 0.5;
            Candle {
                timestamp: 1_700_000_000 + (i as i64) * 86_400,
                date_label: format!("d{i}"),
                open: base,
                high: base + 4.0,
                low: base - 3.0,
                close: base + 1.5,
                volume: 150_000.0,
            }
        })
        .collect();

    let cfg = IchimokuConfig::default();
    let full = compute_ichimoku(&candles, cfg);
    assert_eq!(full.tenkan.len(), candles.len());
    assert_eq!(full.kijun.len(), candles.len());
    assert_eq!(full.chikou.len(), candles.len());
    assert_eq!(full.senkou_a.len(), candles.len() + 26);
    assert_eq!(full.senkou_b.len(), candles.len() + 26);

    // Exact warmup indices: Tenkan at 8, Kijun at 25, Senkou A at 25 + 26 = 51, Senkou B at 51 + 26 = 77
    assert!(full.tenkan[7].is_none() && full.tenkan[8].is_some());
    assert!(full.kijun[24].is_none() && full.kijun[25].is_some());
    assert!(full.senkou_a[50].is_none() && full.senkou_a[51].is_some());
    assert!(full.senkou_b[76].is_none() && full.senkou_b[77].is_some());

    let manual_tenkan = donchian_midpoint_series(&candles, 9);
    assert_eq!(full.tenkan, manual_tenkan);
    assert!(
        (full.senkou_a[25 + 26].unwrap()
            - 0.5 * (full.tenkan[25].unwrap() + full.kijun[25].unwrap()))
        .abs()
            < 1e-10
    );
    assert_eq!(
        full.senkou_b[51 + 26],
        donchian_midpoint_series(&candles, 52)[51]
    );

    // Forward projection [N .. N + 26) must carry non-None values from tail bars!
    assert!(full.senkou_a[candles.len()].is_some());
    assert!(full.senkou_a[candles.len() + 25].is_some());
    assert!(full.senkou_b[candles.len() + 25].is_some());

    // DEF-001 causal Chikou: strictly masked `None` for all indices `>= N - 26`!
    let chikou_limit = candles.len() - 26;
    for i in 0..chikou_limit {
        assert!((full.chikou[i].unwrap() - candles[i + 26].close).abs() < 1e-12);
    }
    assert!(full.chikou[chikou_limit..].iter().all(Option::is_none));

    // DEF-001 Prefix invariance: past Tenkan/Kijun/SenkouA/SenkouB never shift when future bars append.
    let prefix_k = 80_usize;
    let prefix = compute_ichimoku(&candles[..prefix_k], cfg);
    for i in 0..prefix_k {
        assert_eq!(prefix.tenkan[i], full.tenkan[i]);
        assert_eq!(prefix.kijun[i], full.kijun[i]);
    }
    for i in 0..(prefix_k + 26) {
        assert_eq!(prefix.senkou_a[i], full.senkou_a[i]);
        assert_eq!(prefix.senkou_b[i], full.senkou_b[i]);
    }
}

#[test]
fn r12_cvd_conservation_session_reset_and_sma_isolation() {
    use super::cvd::{CvdConfig, CvdResetMode, compute_bar_volume_delta, compute_cvd};
    use crate::data::Timeframe;

    assert_eq!(compute_cvd(&[], CvdConfig::default()).delta.len(), 0);

    let flat_candle = Candle {
        timestamp: 1_750_000_000,
        date_label: "flat".to_owned(),
        open: 100.0,
        high: 100.0,
        low: 100.0,
        close: 100.0,
        volume: 50_000.0,
    };
    assert_eq!(compute_bar_volume_delta(&flat_candle), Some(0.0));

    let bull_full = Candle {
        timestamp: 1_750_000_000,
        date_label: "bull".to_owned(),
        open: 100.0,
        high: 110.0,
        low: 100.0,
        close: 110.0,
        volume: 25_000.0,
    };
    assert!((compute_bar_volume_delta(&bull_full).unwrap() - 25_000.0).abs() < 1e-9);

    let zero_vol = Candle {
        volume: 0.0,
        ..bull_full.clone()
    };
    assert_eq!(compute_bar_volume_delta(&zero_vol), Some(0.0));

    // Construct 2 intraday UTC+7 trading sessions (`15m` bars across 2 days):
    // Session 0 (`2026-09-01 09:00 UTC+7`): bullish bars
    // Session 1 (`2026-09-02 09:00 UTC+7`): bearish bar first
    let utc7 = 7 * 3_600;
    let day0_0200_utc = 1_788_228_000_i64; // 02:00 UTC = 09:00 UTC+7
    let mut series = Vec::new();
    for b in 0..6_i64 {
        series.push(Candle {
            timestamp: day0_0200_utc + b * 900,
            date_label: format!("s0_{b}"),
            open: 1200.0,
            high: 1210.0,
            low: 1200.0,
            close: 1210.0,
            volume: 1_000.0,
        });
    }
    let day1_0200_utc = day0_0200_utc + 86_400;
    for b in 0..6_i64 {
        series.push(Candle {
            timestamp: day1_0200_utc + b * 900,
            date_label: format!("s1_{b}"),
            open: 1210.0,
            high: 1210.0,
            low: 1200.0,
            close: 1200.0,
            volume: 400.0,
        });
    }

    let ses_cfg = CvdConfig {
        reset_mode: CvdResetMode::Session,
        timeframe: Timeframe::M15,
        utc_offset_seconds: utc7,
        sma_period: 3,
    };
    let ses = compute_cvd(&series, ses_cfg);
    assert_eq!(ses.session_starts, vec![0, 6]);
    assert!((ses.cvd[5].unwrap() - 6_000.0).abs() < 1e-9);
    // Session 1 MUST reset cumulative sum back to `-400.0` (not `6000 - 400`) and isolate SMA!
    assert!((ses.cvd[6].unwrap() - (-400.0)).abs() < 1e-9);
    assert!(ses.cvd_sma[5].is_some());
    assert!(ses.cvd_sma[6].is_none() && ses.cvd_sma[7].is_none() && ses.cvd_sma[8].is_some());
    assert!((ses.cvd_sma[8].unwrap() - ((-400.0 - 800.0 - 1_200.0) / 3.0)).abs() < 1e-9);

    let cum_cfg = CvdConfig {
        reset_mode: CvdResetMode::Cumulative,
        ..ses_cfg
    };
    let cum = compute_cvd(&series, cum_cfg);
    assert_eq!(cum.session_starts, vec![0]);
    assert!((cum.cvd[6].unwrap() - 5_600.0).abs() < 1e-9);

    // DEF-001 Prefix invariance:
    let prefix = compute_cvd(&series[..8], ses_cfg);
    assert_eq!(&ses.cvd[..8], &prefix.cvd[..]);
    assert_eq!(&ses.cvd_sma[..8], &prefix.cvd_sma[..]);
}

#[test]
fn critic_r12_regression_extreme_finite_midpoint_and_cvd_sma() {
    let extreme_ichi: Vec<Candle> = (0..12)
        .map(|i| Candle {
            timestamp: 1_750_000_000 + (i as i64) * 60,
            date_label: format!("e{i}"),
            open: 0.0,
            high: 1.0e308,
            low: -1.0e308,
            close: 0.0,
            volume: 100.0,
        })
        .collect();
    let cfg = IchimokuConfig {
        tenkan_period: 3,
        kijun_period: 1,
        senkou_b_period: 5,
        displacement: 2,
        show_chikou: true,
    };
    let ichi = compute_ichimoku(&extreme_ichi, cfg);
    assert!(ichi.kijun.iter().all(|v| v.is_some_and(f64::is_finite)));
    assert!(
        ichi.tenkan[2..]
            .iter()
            .all(|v| v.is_some_and(f64::is_finite))
    );
    assert!(
        ichi.senkou_b[6..]
            .iter()
            .all(|v| v.is_some_and(f64::is_finite))
    );

    // High-magnitude volume & finite range > f64::MAX / 2 must keep buy fraction & CVD SMA finite
    let mut cvd_bars = vec![Candle {
        timestamp: 1_750_000_000,
        date_label: "c0".to_owned(),
        open: 1.0e308,
        high: 1.6e308,
        low: 1.0e308,
        close: 1.3e308,
        volume: 1.0e308,
    }];
    // Bar 0 has midpoint close -> delta = 0; create 10 up bars pushing running_cvd to 1.0e308
    cvd_bars[0] = Candle {
        timestamp: 1_750_000_000,
        date_label: "c0".to_owned(),
        open: 100.0,
        high: 105.0,
        low: 100.0,
        close: 105.0,
        volume: 1.0e308,
    };
    for i in 1..12 {
        cvd_bars.push(Candle {
            timestamp: 1_750_000_000 + (i as i64) * 60,
            date_label: format!("c{i}"),
            open: 1.0e308,
            high: 1.6e308,
            low: 1.0e308,
            close: 1.3e308,
            volume: 1.0e307,
        });
    }
    let frac = super::volume_profile::intrabar_buy_fraction(&cvd_bars[1]);
    assert!(
        (frac - 0.5).abs() < 1e-12,
        "high-magnitude close must not overflow 2*close: {frac}"
    );
    let cvd_out = compute_cvd(
        &cvd_bars,
        CvdConfig {
            reset_mode: CvdResetMode::Cumulative,
            timeframe: Timeframe::M1,
            utc_offset_seconds: 0,
            sma_period: 9,
        },
    );
    for (idx, val) in cvd_out.cvd_sma.iter().enumerate().skip(8) {
        assert!(
            val.is_some_and(f64::is_finite),
            "cvd_sma[{idx}] overflowed instead of scaled mean"
        );
    }
}

#[test]
fn candle_block_index_and_bucket_sum_match_raw_scans_bit_for_bit() {
    use crate::data::generate_synthetic_ohlcv;

    let candles = generate_synthetic_ohlcv(1_029);
    let mut cache = IndicatorCache::new();
    let cfg = IndicatorConfig {
        show_cvd: true,
        cvd_reset_mode: CvdResetMode::Cumulative,
        ..IndicatorConfig::default()
    };
    cache.sync(&candles, 1, &cfg);
    assert_eq!(cache.candle_idx.source_len(), candles.len());

    for step in [16_usize, 32, 64] {
        for range in [0..candles.len(), 7..991, 512..candles.len()] {
            let mut from_blocks = Vec::new();
            cache.candle_idx.for_each_conflated_bucket(
                &candles,
                range.clone(),
                step,
                |cx, o, h, l, c, v, bull| {
                    from_blocks.push((cx, o, h, l, c, v, bull));
                },
            );
            let mut from_raw = Vec::new();
            let mut r_min = f64::INFINITY;
            let mut r_max = f64::NEG_INFINITY;
            let mut m_vol = 0.0_f64;
            for bar in crate::viewport::iter_conflated_candles(&candles, range.clone(), step) {
                from_raw.push((
                    bar.center_logical_index,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                    bar.is_bullish(),
                ));
                r_min = r_min.min(bar.low);
                r_max = r_max.max(bar.high);
                m_vol = m_vol.max(bar.volume);
            }
            assert_eq!(from_blocks.len(), from_raw.len());
            for (b, r) in from_blocks.iter().zip(from_raw.iter()) {
                assert_eq!(b.0, r.0);
                assert_eq!(b.1, r.1);
                assert_eq!(b.2, r.2);
                assert_eq!(b.3, r.3);
                assert_eq!(b.4, r.4);
                assert!((b.5 - r.5).abs() <= r.5.abs() * 1e-12 + 1e-9);
                assert_eq!(b.6, r.6);
            }
            let (b_min, b_max, b_vol) = cache.candle_idx.autoscale_bounds(&candles, range, step);
            assert_eq!(b_min, r_min);
            assert_eq!(b_max, r_max);
            assert!((b_vol - m_vol).abs() <= m_vol.abs() * 1e-12 + 1e-9);
        }
    }

    let mut b_s = 0_usize;
    while b_s < candles.len() {
        let b_e = (b_s + 32).min(candles.len());
        let got = cache
            .cvd_delta_idx
            .bucket_sum_and_count(&cache.cvd.delta, b_s, b_e);
        let mut raw_sum = 0.0_f64;
        let mut raw_cnt = 0_usize;
        for v in cache.cvd.delta[b_s..b_e].iter().flatten() {
            if v.is_finite() {
                raw_sum += *v;
                raw_cnt += 1;
            }
        }
        let (gs, gc) = got.expect("aligned delta sum");
        assert_eq!(gc, raw_cnt);
        assert!((gs - raw_sum).abs() <= raw_sum.abs() * 1e-12 + 1e-9);
        b_s += 32;
    }

    let mut sparse_series: Vec<Option<f64>> = (0..640)
        .map(|i| Some(100.0 + ((i * 13) % 29) as f64 - ((i % 7) as f64) * 1.5))
        .collect();
    for slot in &mut sparse_series[250..262] {
        *slot = None;
    }
    let sparse_idx = SeriesBlockIndex::build(&sparse_series);
    for (s, e) in [(7, 71), (19, 255), (255, 340), (511, 600), (240, 270)] {
        assert_eq!(
            bucket_line_envelope_indexed(&sparse_series, Some(&sparse_idx), s, e),
            bucket_line_envelope_indexed(&sparse_series, None, s, e),
            "unaligned hybrid envelope mismatch on [{s}..{e})"
        );
        let mut raw_sum = 0.0_f64;
        let mut raw_cnt = 0_usize;
        for v in sparse_series[s..e].iter().flatten() {
            if v.is_finite() {
                raw_sum += *v;
                raw_cnt += 1;
            }
        }
        let (gs, gc) = sparse_idx
            .bucket_sum_and_count(&sparse_series, s, e)
            .expect("unaligned bucket sum");
        assert_eq!(gc, raw_cnt);
        assert!((gs - raw_sum).abs() <= raw_sum.abs().max(1.0) * 1e-9);
    }
}

#[test]
fn r13_vwap_anchor_buckets_and_session_market_profile_invariants() {
    use crate::data::Timeframe;

    // 1. Multi-interval VWAP anchor bucket reset boundaries (Daily / Weekly / Monthly / Yearly)
    // 2026-01-02 (Fri) vs 2026-01-05 (Mon) UTC+0: same Month & Year, different Daily & ISO Week!
    let fri_ts = 1_767_348_000; // 2026-01-02 10:00:00Z
    let mon_ts = 1_767_607_200; // 2026-01-05 10:00:00Z
    let feb_ts = 1_770_026_400; // 2026-02-02 10:00:00Z
    assert_ne!(
        vwap_bucket_key(fri_ts, Timeframe::H1, 0, VwapAnchorMode::Daily),
        vwap_bucket_key(mon_ts, Timeframe::H1, 0, VwapAnchorMode::Daily)
    );
    assert_ne!(
        vwap_bucket_key(fri_ts, Timeframe::H1, 0, VwapAnchorMode::Weekly),
        vwap_bucket_key(mon_ts, Timeframe::H1, 0, VwapAnchorMode::Weekly)
    );
    assert_eq!(
        vwap_bucket_key(fri_ts, Timeframe::H1, 0, VwapAnchorMode::Monthly),
        vwap_bucket_key(mon_ts, Timeframe::H1, 0, VwapAnchorMode::Monthly)
    );
    assert_ne!(
        vwap_bucket_key(mon_ts, Timeframe::H1, 0, VwapAnchorMode::Monthly),
        vwap_bucket_key(feb_ts, Timeframe::H1, 0, VwapAnchorMode::Monthly)
    );
    assert_eq!(
        vwap_bucket_key(mon_ts, Timeframe::H1, 0, VwapAnchorMode::Yearly),
        vwap_bucket_key(feb_ts, Timeframe::H1, 0, VwapAnchorMode::Yearly)
    );

    let hourly: Vec<Candle> = (0..120)
        .map(|i| Candle {
            timestamp: fri_ts + (i as i64) * 3_600,
            date_label: format!("bar-{i}"),
            open: 100.0 + (i as f64) * 0.25,
            high: 101.0 + (i as f64) * 0.25,
            low: 99.0 + (i as f64) * 0.25,
            close: 100.5 + (i as f64) * 0.25,
            volume: 50_000.0,
        })
        .collect();
    let v_daily = compute_session_vwap_with_anchor(
        &hourly,
        Timeframe::H1,
        0,
        VwapAnchorMode::Daily,
        1.0,
        2.0,
    );
    let v_yearly = compute_session_vwap_with_anchor(
        &hourly,
        Timeframe::H1,
        0,
        VwapAnchorMode::Yearly,
        1.0,
        2.0,
    );
    assert_eq!(v_daily.vwap.len(), hourly.len());
    assert_ne!(v_daily.vwap.last(), v_yearly.vwap.last());

    // 2. Session TPO / Market Profile invariants: POC max TPOs, 70% Value Area bounds, IBH >= IBL,
    //    Single Prints interior, flat-candle guard, and DEF-001 causal Naked POC mitigation.
    // Build 3 daily sessions on 15m timeframe: Day 0, Day 1 (away from Day 0 POC -> Naked!), Day 2 (mitigates Day 0 POC!)
    let mut tpo_bars = Vec::new();
    let day0 = 1_767_574_800_i64; // 2026-01-05 01:00Z
    // Day 0 (8 bars, clustered around 100..110 with heavy dwell around 105)
    let d0_ranges = [
        (100.0, 103.0),
        (103.0, 107.0),
        (104.0, 106.0),
        (104.0, 106.5),
        (104.5, 106.0),
        (103.5, 107.0),
        (107.0, 110.0),
        (104.0, 106.0),
    ];
    for (i, &(l, h)) in d0_ranges.iter().enumerate() {
        tpo_bars.push(Candle {
            timestamp: day0 + (i as i64) * 900,
            date_label: format!("d0-{i}"),
            open: l + 0.2,
            high: h,
            low: l,
            close: h - 0.2,
            volume: 10_000.0,
        });
    }
    // Day 1 (6 bars staying strictly >= 120.0, so Day 0 POC ~ 105 stays NAKED at end of Day 1!)
    for i in 0..6 {
        tpo_bars.push(Candle {
            timestamp: day0 + 86_400 + (i as i64) * 900,
            date_label: format!("d1-{i}"),
            open: 120.0,
            high: 124.0,
            low: 120.0,
            close: 122.0,
            volume: 8_000.0,
        });
    }
    let mp_prefix = compute_session_market_profiles(
        &tpo_bars,
        Timeframe::M15,
        MarketProfileConfig {
            session_period: ProfileSessionPeriod::Daily,
            utc_offset_seconds: 0,
            num_bins: 16,
            value_area_pct: 0.70,
            ib_bars: 3,
            max_sessions: 12,
            extend_naked_poc: true,
        },
    );
    assert_eq!(mp_prefix.profiles.len(), 2);
    let p0 = &mp_prefix.profiles[0];
    assert!(p0.val_price <= p0.poc_price && p0.poc_price <= p0.vah_price);
    assert!(p0.ib_high >= p0.ib_low);
    assert_eq!(
        p0.rows[p0.poc_index].tpo_count,
        p0.rows.iter().map(|r| r.tpo_count).max().unwrap()
    );
    assert!(p0.naked_poc_mitigated_index.is_none());
    assert_eq!(p0.naked_poc_end_index, tpo_bars.len() - 1);

    // Day 2 breaks back down through Day 0 POC (~105), causally mitigating ONLY AFTER prefix!
    let mut with_day2 = tpo_bars.clone();
    with_day2.push(Candle {
        timestamp: day0 + 2 * 86_400,
        date_label: "d2-0".to_owned(),
        open: 112.0,
        high: 112.0,
        low: 100.0,
        close: 104.0,
        volume: 25_000.0,
    });
    let mp_full = compute_session_market_profiles(
        &with_day2,
        Timeframe::M15,
        MarketProfileConfig {
            session_period: ProfileSessionPeriod::Daily,
            utc_offset_seconds: 0,
            num_bins: 16,
            value_area_pct: 0.70,
            ib_bars: 3,
            max_sessions: 12,
            extend_naked_poc: true,
        },
    );
    assert_eq!(
        mp_full.profiles[0].naked_poc_mitigated_index,
        Some(tpo_bars.len())
    );
    assert_eq!(mp_full.profiles[0].naked_poc_end_index, tpo_bars.len());
    // Flat zero-range session must never produce NaN/div-by-zero
    let flat = vec![
        Candle {
            timestamp: day0,
            date_label: "flat".to_owned(),
            open: 50.0,
            high: 50.0,
            low: 50.0,
            close: 50.0,
            volume: 100.0,
        };
        5
    ];
    let mp_flat =
        compute_session_market_profiles(&flat, Timeframe::M15, MarketProfileConfig::default());
    assert_eq!(mp_flat.profiles.len(), 1);
    assert!(mp_flat.profiles[0].poc_price.is_finite());
}

#[test]
fn critic_r13_regression_negative_utc_vwap_tpo_valid_ib_and_older_naked_poc() {
    // 1. Negative UTC offset (EST -18_000) MUST NOT shift 00:00 UTC Daily/Weekly bars backward
    //    across leap-year Feb-29/Mar-01, Mondays, month-starts, or Jan-01!
    let start_day_utc = 1_709_164_800_i64; // 2024-02-29 00:00:00 UTC (leap day)
    let anchors = [
        VwapAnchorMode::AutoSession,
        VwapAnchorMode::Daily,
        VwapAnchorMode::Weekly,
        VwapAnchorMode::Monthly,
        VwapAnchorMode::Yearly,
    ];
    for day_off in 0..400_i64 {
        let ts = start_day_utc + day_off * 86_400;
        for tf in [Timeframe::D1, Timeframe::W1] {
            for anchor in anchors {
                let k_utc = vwap_bucket_key(ts, tf, 0, anchor);
                let k_est = vwap_bucket_key(ts, tf, -18_000, anchor);
                let k_ict = vwap_bucket_key(ts, tf, 25_200, anchor);
                assert_eq!(
                    k_est, k_utc,
                    "D1/W1 EST negative offset shifted bucket on day_off={day_off} {tf:?} {anchor:?}"
                );
                assert_eq!(
                    k_ict, k_utc,
                    "D1/W1 ICT positive offset shifted bucket on day_off={day_off} {tf:?} {anchor:?}"
                );
            }
        }
    }

    // Check explicit transitions on EST D1: 2024-02-29 != 2024-03-01 (Monthly),
    // 2024-03-01 (Fri) != 2024-03-04 (Mon) (Weekly), 2024-12-31 != 2025-01-01 (Yearly)
    let feb29 = start_day_utc;
    let mar01 = feb29 + 86_400;
    let mar04 = feb29 + 4 * 86_400;
    assert_ne!(
        vwap_bucket_key(feb29, Timeframe::D1, -18_000, VwapAnchorMode::Monthly),
        vwap_bucket_key(mar01, Timeframe::D1, -18_000, VwapAnchorMode::Monthly)
    );
    assert_eq!(
        vwap_bucket_key(mar01, Timeframe::D1, -18_000, VwapAnchorMode::Weekly),
        vwap_bucket_key(feb29, Timeframe::D1, -18_000, VwapAnchorMode::Weekly)
    );
    assert_ne!(
        vwap_bucket_key(mar01, Timeframe::D1, -18_000, VwapAnchorMode::Weekly),
        vwap_bucket_key(mar04, Timeframe::D1, -18_000, VwapAnchorMode::Weekly)
    );

    // 2. Leading invalid (NaN) candles must NOT consume IB bar slots in TPO Profile.
    let s0 = 1_767_574_800_i64;
    let ib_candles = vec![
        Candle {
            timestamp: s0,
            date_label: "nan0".to_owned(),
            open: f64::NAN,
            high: f64::NAN,
            low: f64::NAN,
            close: f64::NAN,
            volume: 0.0,
        },
        Candle {
            timestamp: s0 + 900,
            date_label: "nan1".to_owned(),
            open: 100.0,
            high: f64::INFINITY,
            low: 99.0,
            close: 100.0,
            volume: 0.0,
        },
        Candle {
            timestamp: s0 + 1800,
            date_label: "v0".to_owned(),
            open: 101.0,
            high: 104.0,
            low: 100.0,
            close: 103.0,
            volume: 1000.0,
        },
        Candle {
            timestamp: s0 + 2700,
            date_label: "v1".to_owned(),
            open: 103.0,
            high: 106.0,
            low: 102.0,
            close: 105.0,
            volume: 1000.0,
        },
        Candle {
            timestamp: s0 + 3600,
            date_label: "v2_post_ib".to_owned(),
            open: 105.0,
            high: 112.0,
            low: 104.0,
            close: 111.0,
            volume: 1000.0,
        },
    ];
    let mp_ib = compute_session_market_profiles(
        &ib_candles,
        Timeframe::M15,
        MarketProfileConfig {
            session_period: ProfileSessionPeriod::Daily,
            utc_offset_seconds: 0,
            num_bins: 12,
            value_area_pct: 0.70,
            ib_bars: 2,
            max_sessions: 8,
            extend_naked_poc: true,
        },
    );
    assert_eq!(mp_ib.profiles.len(), 1);
    assert!(
        (mp_ib.profiles[0].ib_low - 100.0).abs() < 1e-9
            && (mp_ib.profiles[0].ib_high - 106.0).abs() < 1e-9,
        "leading invalid bars must not eat ib_bars slots (got IBL={} IBH={})",
        mp_ib.profiles[0].ib_low,
        mp_ib.profiles[0].ib_high
    );
    assert!((mp_ib.profiles[0].range_high - 112.0).abs() < 1e-9);

    // 3. Unmitigated Naked POC older than `max_sessions` is preserved in `historical_naked_pocs`
    //    and causally stops when touched by a later session bar.
    let mut multi_days = Vec::new();
    for d in 0..4_i64 {
        let base_px = if d == 0 {
            100.0
        } else {
            150.0 + (d as f64) * 5.0
        };
        for b in 0..4_i64 {
            multi_days.push(Candle {
                timestamp: s0 + d * 86_400 + b * 900,
                date_label: format!("d{d}-b{b}"),
                open: base_px,
                high: base_px + 3.0,
                low: base_px - 3.0,
                close: base_px + 1.0,
                volume: 5000.0,
            });
        }
    }
    let mp_old = compute_session_market_profiles(
        &multi_days,
        Timeframe::M15,
        MarketProfileConfig {
            session_period: ProfileSessionPeriod::Daily,
            utc_offset_seconds: 0,
            num_bins: 12,
            value_area_pct: 0.70,
            ib_bars: 2,
            max_sessions: 2,
            extend_naked_poc: true,
        },
    );
    assert_eq!(mp_old.profiles.len(), 2);
    assert_eq!(mp_old.historical_naked_pocs.len(), 2);
    assert!(
        mp_old.historical_naked_pocs[0]
            .naked_poc_mitigated_index
            .is_none()
    );
    assert_eq!(
        mp_old.historical_naked_pocs[0].naked_poc_end_index,
        multi_days.len() - 1
    );
}

#[test]
fn r18_b1_candle_geometry_microprice_invariance_14_patterns_and_candle_score() {
    use super::patterns::{
        CandleGeometry, CandlestickPatternKind, compute_candlestick_patterns_and_scores,
    };

    let mk = |ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64| Candle {
        timestamp: ts,
        date_label: format!("t{ts}"),
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
    };

    // 1. Scale-relative invariance: SHIB-scale (`1e-8`) vs BTC-scale (`1e5`) produce identical ratios & structure_score!
    let macro_bar = mk(1, 100_000.0, 102_000.0, 99_000.0, 101_500.0, 10.0);
    let micro_bar = mk(
        1,
        1.0e-8 * 100.0,
        1.0e-8 * 102.0,
        1.0e-8 * 99.0,
        1.0e-8 * 101.5,
        10.0,
    );
    let g_macro = CandleGeometry::from_candle(&macro_bar);
    let g_micro = CandleGeometry::from_candle(&micro_bar);
    assert!(
        !g_micro.is_flat_range,
        "micro-price candle must NOT collapse as flat range"
    );
    assert!((g_macro.body_ratio - g_micro.body_ratio).abs() < 1e-12);
    assert!((g_macro.close_location - g_micro.close_location).abs() < 1e-12);
    assert!(
        (g_macro.structure_score(0.45, 0.35, 0.20).unwrap()
            - g_micro.structure_score(0.45, 0.35, 0.20).unwrap())
        .abs()
            < 1e-10
    );

    // Flat & invalid OHLC guards
    let flat_bar = mk(2, 50.0, 50.0, 50.0, 50.0, 100.0);
    let g_flat = CandleGeometry::from_candle(&flat_bar);
    assert!(g_flat.is_flat_range);
    assert_eq!(g_flat.structure_score(0.45, 0.35, 0.20), Some(0.0));
    let inv_bar = mk(3, 50.0, 48.0, 52.0, 50.0, 100.0);
    assert_eq!(
        CandleGeometry::from_candle(&inv_bar).structure_score(0.45, 0.35, 0.20),
        None
    );

    // 2. Construct a deterministic sequence triggering all 12 atomic `CandlestickPatternKind`s + `any_bullish` / `any_bearish`:
    let mut seq = Vec::with_capacity(40);
    // Warmup 15 bars (`0..15`) with ATR ~ 2.0
    for i in 0..15 {
        let b = 100.0 + (i % 2) as f64 * 0.5;
        seq.push(mk(i as i64, b, b + 1.0, b - 1.0, b + 0.2, 1000.0));
    }
    // Hammer (`15`): open=99.6, close=99.9, high=100.0, low=98.0 (range=2.0, body=0.3, lower_wick=1.6)
    seq.push(mk(15, 99.6, 100.0, 98.0, 99.9, 1000.0));
    // ShootingStar (`16`): open=100.4, close=100.1, high=102.0, low=100.0
    seq.push(mk(16, 100.4, 102.0, 100.0, 100.1, 1000.0));
    // BullishEngulfing (`17..=18`): bearish 101->100, then bullish 99.8->101.5 (high=101.6, low=99.7)
    seq.push(mk(17, 101.0, 101.2, 99.9, 100.0, 1000.0));
    seq.push(mk(18, 99.8, 101.6, 99.7, 101.5, 1200.0));
    // BearishEngulfing (`19..=20`): bullish 100->101, then bearish 101.2->99.5 (high=101.3, low=99.4)
    seq.push(mk(19, 100.0, 101.1, 99.9, 101.0, 1000.0));
    seq.push(mk(20, 101.2, 101.3, 99.4, 99.5, 1200.0));
    // PiercingLine (`21..=22`): bearish 102->100 (mid=101), then bullish 99.8->101.3 (<102, close_loc>=0.60)
    seq.push(mk(21, 102.0, 102.2, 99.9, 100.0, 1000.0));
    seq.push(mk(22, 99.8, 101.5, 99.7, 101.3, 1100.0));
    // DarkCloudCover (`23..=24`): bullish 100->102 (mid=101), then bearish 102.2->100.7 (>100, close_loc<=0.40)
    seq.push(mk(23, 100.0, 102.1, 99.9, 102.0, 1000.0));
    seq.push(mk(24, 102.2, 102.3, 100.5, 100.7, 1100.0));
    // MorningStar (`25..=27`): strong bear 103->100, small star 100.1->99.9, strong bull 100.0->102.2
    seq.push(mk(25, 103.0, 103.2, 99.8, 100.0, 1000.0));
    seq.push(mk(26, 100.1, 100.3, 99.7, 99.9, 800.0));
    seq.push(mk(27, 100.0, 102.4, 99.9, 102.2, 1300.0));
    // EveningStar (`28..=30`): strong bull 100->103, small star 102.9->103.1, strong bear 103.0->100.8
    seq.push(mk(28, 100.0, 103.2, 99.8, 103.0, 1000.0));
    seq.push(mk(29, 102.9, 103.3, 102.7, 103.1, 800.0));
    seq.push(mk(30, 103.0, 103.1, 100.6, 100.8, 1300.0));
    // TweezerBottom (`31..=32`): bear low=98.00, bull low=98.02 (within 0.15*ATR)
    seq.push(mk(31, 100.0, 100.2, 98.00, 98.4, 1000.0));
    seq.push(mk(32, 98.5, 100.0, 98.02, 99.8, 1000.0));
    // TweezerTop (`33..=34`): bull high=102.00, bear high=101.98
    seq.push(mk(33, 100.0, 102.00, 99.8, 101.6, 1000.0));
    seq.push(mk(34, 101.5, 101.98, 100.0, 100.2, 1000.0));
    // InsideBarBreakoutUp (`35..=37`): mother [99..102], inside [99.6..101.4], breakout close=102.6
    seq.push(mk(35, 99.5, 102.0, 99.0, 101.5, 1000.0));
    seq.push(mk(36, 100.2, 101.4, 99.6, 100.8, 800.0));
    seq.push(mk(37, 100.9, 102.8, 100.7, 102.6, 1500.0));
    // InsideBarBreakoutDown (`38..=40`): mother [99..102], inside [99.6..101.4], breakdown close=98.4
    seq.push(mk(38, 101.5, 102.0, 99.0, 99.5, 1000.0));
    seq.push(mk(39, 100.5, 101.4, 99.6, 100.1, 800.0));
    seq.push(mk(40, 100.0, 100.2, 98.2, 98.4, 1500.0));

    let atr14 = compute_atr(&seq, 14);
    let (_, summaries) = compute_candlestick_patterns_and_scores(&seq, &atr14);

    let expected = [
        (15, CandlestickPatternKind::Hammer, true),
        (16, CandlestickPatternKind::ShootingStar, false),
        (18, CandlestickPatternKind::BullishEngulfing, true),
        (20, CandlestickPatternKind::BearishEngulfing, false),
        (22, CandlestickPatternKind::PiercingLine, true),
        (24, CandlestickPatternKind::DarkCloudCover, false),
        (27, CandlestickPatternKind::MorningStar, true),
        (30, CandlestickPatternKind::EveningStar, false),
        (32, CandlestickPatternKind::TweezerBottom, true),
        (34, CandlestickPatternKind::TweezerTop, false),
        (37, CandlestickPatternKind::InsideBarBreakoutUp, true),
        (40, CandlestickPatternKind::InsideBarBreakoutDown, false),
    ];
    for (idx, kind, is_bull) in expected {
        assert_eq!(
            summaries[idx].pattern_result(kind),
            Some(true),
            "expected {kind:?} at index {idx}"
        );
        if is_bull {
            assert_eq!(summaries[idx].any_bullish(), Some(true));
        } else {
            assert_eq!(summaries[idx].any_bearish(), Some(true));
        }
    }
}

#[test]
fn r18_b2_extended_indicators_adx_stoch_supertrend_psar_keltner_cci_mfi_volsma() {
    let mk = |ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64| Candle {
        timestamp: ts,
        date_label: format!("t{ts}"),
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
    };

    // Build 120 bars: strong uptrend (0..60) then sharp reversal downtrend (60..120)
    let mut bars = Vec::with_capacity(120);
    for i in 0..120 {
        let base = if i < 60 {
            100.0 + (i as f64) * 1.2
        } else {
            172.0 - ((i - 60) as f64) * 1.4
        };
        let wob = ((i % 5) as f64 - 2.0) * 0.25;
        let c = base + wob;
        let o = c - if i < 60 { 0.6 } else { -0.6 };
        bars.push(mk(
            i as i64 * 86_400,
            o,
            o.max(c) + 0.9,
            o.min(c) - 0.8,
            c,
            1_000.0 + (i as f64) * 25.0,
        ));
    }

    // 1. ADX(14): warmup None before 14 (+DI/-DI) and before 27 (ADX); uptrend +DI > -DI, downtrend -DI > +DI
    let adx = compute_adx(&bars, 14);
    assert!(adx.plus_di[..14].iter().all(Option::is_none));
    assert!(adx.adx[..27].iter().all(Option::is_none));
    assert!(adx.plus_di[55].unwrap() > adx.minus_di[55].unwrap());
    assert!(adx.adx[55].unwrap() > 25.0);
    assert!(adx.minus_di[115].unwrap() > adx.plus_di[115].unwrap());
    // Flat zero-TR series must emit 0.0 (never NaN/Inf)
    let flat_bars: Vec<Candle> = (0..40)
        .map(|i| mk(i, 50.0, 50.0, 50.0, 50.0, 0.0))
        .collect();
    let adx_flat = compute_adx(&flat_bars, 14);
    assert_eq!(adx_flat.plus_di[20], Some(0.0));
    assert_eq!(adx_flat.adx[35], Some(0.0));

    // 2. Stochastic(14, 3, 3): O(N) monotonic deque exact parity vs naive O(N*K) scan
    let st = compute_stochastic(&bars, 14, 3, 3);
    for i in 13..bars.len() {
        let hh = bars[i + 1 - 14..=i]
            .iter()
            .map(|b| b.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = bars[i + 1 - 14..=i]
            .iter()
            .map(|b| b.low)
            .fold(f64::INFINITY, f64::min);
        let expected_fk = if hh > ll {
            100.0 * (bars[i].close - ll) / (hh - ll)
        } else {
            50.0
        };
        assert!((st.fast_k[i].unwrap() - expected_fk).abs() < 1e-11);
    }
    assert!(st.slow_k[55].unwrap() > 70.0);
    assert!(st.slow_d[115].unwrap() < 30.0);

    // 3. SuperTrend(10, 3.0) & Parabolic SAR(0.02, 0.02, 0.20) flip bull -> bear across bar 60
    let sup = compute_supertrend(&bars, 10, 3.0);
    assert_eq!(sup.direction[55], Some(1));
    assert!(sup.trend[55].unwrap() < bars[55].close);
    assert_eq!(sup.direction[115], Some(-1));
    assert!(sup.trend[115].unwrap() > bars[115].close);

    let psar = compute_psar(&bars, 0.02, 0.02, 0.20);
    assert_eq!(psar.is_long[55], Some(true));
    assert!(psar.sar[55].unwrap() <= bars[55].low);
    assert_eq!(psar.is_long[115], Some(false));
    assert!(psar.sar[115].unwrap() >= bars[115].high);

    // 4. Keltner Channels(20, 10, 2.0): strict `upper > middle > lower`
    let kc = compute_keltner_channels(&bars, 20, 10, 2.0);
    assert!(kc.middle[..19].iter().all(Option::is_none));
    for i in 20..bars.len() {
        let (u, m, l) = (
            kc.upper[i].unwrap(),
            kc.middle[i].unwrap(),
            kc.lower[i].unwrap(),
        );
        assert!(u > m && m > l);
    }

    // 5. CCI(20): flat-series guard emits exact `0.0` (never 1e15 spike), micro-price scale invariant
    let cci = compute_cci(&bars, 20);
    assert!(cci[55].unwrap() > 50.0);
    assert!(cci[115].unwrap() < -50.0);
    let cci_flat = compute_cci(&flat_bars, 20);
    assert_eq!(cci_flat[25], Some(0.0));

    // 6. MFI(14) & Volume SMA(20): zero-flow emits `50.0`; negative volume rejected in Volume SMA
    let mfi = compute_mfi(&bars, 14);
    assert!(mfi[..14].iter().all(Option::is_none));
    assert!(mfi[55].unwrap() > 60.0);
    assert!(mfi[115].unwrap() < 40.0);
    let mfi_flat = compute_mfi(&flat_bars, 14);
    assert_eq!(mfi_flat[20], Some(50.0));

    let vsma = compute_volume_sma(&bars, 20);
    assert!(vsma[..19].iter().all(Option::is_none));
    let manual_vsma: f64 = bars[30..50].iter().map(|b| b.volume).sum::<f64>() / 20.0;
    assert!((vsma[49].unwrap() - manual_vsma).abs() < 1e-9);
    let mut bad_vol = bars.clone();
    bad_vol[25].volume = -1.0;
    let vsma_bad = compute_volume_sma(&bad_vol, 20);
    assert_eq!(vsma_bad[25], None);
}

#[test]
fn critic_r18_regression_adx_psar_prefix_causality_nan_recovery_and_warmup_bars() {
    use super::proxy_bb::{BBDirection, BBHorizon, compute_proxy_bb_series};

    let base = generate_synthetic_ohlcv(220);
    let clean_200 = &base[..200];

    // 1. `[Blocker-01]` DEF-001 Prefix Causality: appending a future `NaN` or `high < low` candle
    //    at `t = 200` MUST NEVER retroactively erase or mutate `ADX` or `PSAR` on `0..200`!
    let adx_clean = compute_adx(clean_200, 14);
    let psar_clean = compute_psar(clean_200, 0.02, 0.02, 0.20);
    assert!(adx_clean.adx[199].is_some());
    assert!(psar_clean.sar[199].is_some());

    let mut with_nan_tail = clean_200.to_vec();
    let mut bad_nan = base[200].clone();
    bad_nan.high = f64::NAN;
    with_nan_tail.push(bad_nan);

    let adx_nan_tail = compute_adx(&with_nan_tail, 14);
    let psar_nan_tail = compute_psar(&with_nan_tail, 0.02, 0.02, 0.20);
    assert_eq!(&adx_nan_tail.adx[..200], &adx_clean.adx[..200]);
    assert_eq!(&adx_nan_tail.plus_di[..200], &adx_clean.plus_di[..200]);
    assert_eq!(&adx_nan_tail.minus_di[..200], &adx_clean.minus_di[..200]);
    assert_eq!(adx_nan_tail.adx[200], None);
    assert_eq!(&psar_nan_tail.sar[..200], &psar_clean.sar[..200]);
    assert_eq!(&psar_nan_tail.is_long[..200], &psar_clean.is_long[..200]);
    assert_eq!(psar_nan_tail.sar[200], None);

    let mut with_inv_tail = clean_200.to_vec();
    let mut bad_inv = base[200].clone();
    bad_inv.high = 90.0;
    bad_inv.low = 110.0;
    with_inv_tail.push(bad_inv);
    let adx_inv_tail = compute_adx(&with_inv_tail, 14);
    let psar_inv_tail = compute_psar(&with_inv_tail, 0.02, 0.02, 0.20);
    assert_eq!(&adx_inv_tail.adx[..200], &adx_clean.adx[..200]);
    assert_eq!(adx_inv_tail.adx[200], None);
    assert_eq!(&psar_inv_tail.sar[..200], &psar_clean.sar[..200]);
    assert_eq!(psar_inv_tail.sar[200], None);

    // 2. `[Blocker-01]` Early `NaN` recovery on default `ADX(14)` and `PSAR(0.02, 0.02, 0.20)`!
    let mut early_nan = base.clone();
    early_nan[3].high = f64::NAN;
    let adx_rec = compute_adx(&early_nan, 14);
    let psar_rec = compute_psar(&early_nan, 0.02, 0.02, 0.20);
    assert_eq!(adx_rec.adx[3], None);
    assert!(adx_rec.adx[60].is_some_and(f64::is_finite));
    assert_eq!(psar_rec.sar[3], None);
    assert!(psar_rec.sar[6].is_some_and(f64::is_finite));

    // 3. `[Minor-01]` Exact `warmup_bars` contract on `SuperTrendIndicator` and `KeltnerIndicator`!
    let st_ind = SuperTrendIndicator {
        period: 10,
        multiplier: 3.0,
    };
    let st_out = st_ind.evaluate(&base[..30]);
    let st_first = st_out.series[0]
        .values
        .iter()
        .position(Option::is_some)
        .unwrap();
    assert_eq!(st_ind.warmup_bars(), st_first);

    let kc_10_10 = KeltnerIndicator {
        ema_period: 10,
        atr_period: 10,
        multiplier: 2.0,
    };
    let kc_out = kc_10_10.evaluate(&base[..30]);
    let kc_upper_first = kc_out.series[0]
        .values
        .iter()
        .position(Option::is_some)
        .unwrap();
    assert_eq!(kc_10_10.warmup_bars(), kc_upper_first);

    // 4. `[Minor-04]` `ProxyBBCalculator` emits `BBDirection::Unknown` on zero-activity `AT05`
    //    AND on the initial valid bar after warmup / after an `AT05` zero-liquidity gap!
    let mut bb_bars = base[..25].to_vec();
    for b in &mut bb_bars[6..10] {
        b.volume = 0.0;
    }
    let pbb = compute_proxy_bb_series(&bb_bars);
    assert_eq!(
        pbb.points[2].horizon(BBHorizon::T03).direction,
        BBDirection::Unknown
    );
    assert_eq!(
        pbb.points[9].horizon(BBHorizon::T03).direction,
        BBDirection::Unknown
    );
    assert_eq!(
        pbb.points[10].horizon(BBHorizon::T03).direction,
        BBDirection::Unknown
    );
}
