use crate::drawings::{DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStore};
use crate::model::Candle;

use super::*;

#[test]
fn synthetic_dataset_is_deterministic_and_valid() {
    let first_run = generate_synthetic_ohlcv(120);
    let second_run = generate_synthetic_ohlcv(120);

    assert_eq!(first_run.len(), 120);
    assert_eq!(first_run, second_run);

    let mut bullish_count = 0;
    let mut bearish_count = 0;

    for window in first_run.windows(2) {
        assert!(window[0].timestamp < window[1].timestamp);
        assert_ne!(window[0].date_label, window[1].date_label);
    }

    for candle in &first_run {
        assert!(candle.is_valid(), "Invalid candle: {candle:?}");
        if candle.is_bullish() {
            bullish_count += 1;
        } else {
            bearish_count += 1;
        }
    }

    assert!(
        bullish_count >= 30,
        "Too few bullish candles: {bullish_count}"
    );
    assert!(
        bearish_count >= 30,
        "Too few bearish candles: {bearish_count}"
    );
}

#[test]
fn synthetic_dataset_100k_bars_is_linear_time_and_preserves_120_prefix() {
    let prefix_120 = generate_synthetic_ohlcv(120);
    let large_100k = generate_synthetic_ohlcv(100_000);

    assert_eq!(large_100k.len(), 100_000);
    assert_eq!(&large_100k[..120], prefix_120.as_slice());
    assert_eq!(large_100k[0].date_label, "2026-05-04");
    assert_eq!(
        format_utc_date(large_100k[0].timestamp),
        large_100k[0].date_label
    );
    assert_eq!(
        format_utc_date(large_100k[119].timestamp),
        large_100k[119].date_label
    );
    assert!(large_100k.iter().all(Candle::is_valid));
}

#[test]
fn proleptic_gregorian_calendar_bijection_and_leap_year_invariants() {
    assert_eq!(days_from_civil(1970, 1, 1), 0);
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(weekday_from_days(0), 3); // Thursday

    // Known leap and century boundaries
    assert!(!is_leap_year(1900));
    assert!(is_leap_year(2000));
    assert!(is_leap_year(2024));
    assert!(!is_leap_year(2026));
    assert!(!is_valid_ymd(1900, 2, 29));
    assert!(is_valid_ymd(2000, 2, 29));
    assert!(!is_valid_ymd(2026, 2, 29));
    assert!(!is_valid_ymd(2026, 4, 31));
    assert!(!is_valid_ymd(0, 1, 1));
    assert!(!is_valid_ymd(10_000, 1, 1));

    // Exhaustive round-trip across 0001-01-01..=9999-12-31 sampled + full 1960..=2050 daily check
    let min_day = days_from_civil(1, 1, 1);
    let max_day = days_from_civil(9999, 12, 31);
    assert_eq!(min_day * 86_400, MIN_SUPPORTED_EPOCH_SECS);
    assert_eq!(max_day * 86_400 + 86_399, MAX_SUPPORTED_EPOCH_SECS);
    assert_eq!(civil_from_days(min_day), (1, 1, 1));
    assert_eq!(civil_from_days(max_day), (9999, 12, 31));

    let d1960 = days_from_civil(1960, 1, 1);
    let d2050 = days_from_civil(2050, 12, 31);
    let mut prev_ymd = (1959, 12, 31);
    for d in d1960..=d2050 {
        let (y, m, day) = civil_from_days(d);
        assert!(is_valid_ymd(y, m, day));
        assert_eq!(days_from_civil(y, m, day), d);
        assert!((y, m, day) > prev_ymd);
        prev_ymd = (y, m, day);
    }
}

#[test]
fn multi_format_timestamp_parser_handles_cafef_iso_tz_and_nanosecond_epochs() {
    // 1. CafeF 8-digit YYYYMMDD
    let p_ymd = parse_timestamp("20261005", TimestampUnit::Auto).unwrap();
    assert!(!p_ymd.has_time_component);
    assert_eq!(format_utc_date(p_ymd.utc_seconds), "2026-10-05");

    // 2. ISO-8601 with positive & negative timezone offsets
    let p_ict = parse_timestamp("2026-10-05T09:15:00+07:00", TimestampUnit::Auto).unwrap();
    assert!(p_ict.has_time_component);
    assert_eq!(
        format_utc_datetime_hm(p_ict.utc_seconds),
        "2026-10-05 02:15"
    );

    let p_ny = parse_timestamp("2026-10-05 09:30:00.500-0400", TimestampUnit::Auto).unwrap();
    assert_eq!(format_utc_datetime_hm(p_ny.utc_seconds), "2026-10-05 13:30");

    // Midnight crossover via TZ offset (`2026-10-06T01:30:00+07:00` -> `2026-10-05 18:30 UTC`)
    let p_cross = parse_timestamp("2026-10-06T01:30:00+07:00", TimestampUnit::Auto).unwrap();
    assert_eq!(
        format_utc_datetime_hm(p_cross.utc_seconds),
        "2026-10-05 18:30"
    );

    // 3. Unix s / ms / µs / ns (including exact `i128` > 2^53 nanosecond precision!)
    let base_sec: i64 = 1_777_852_860; // 2026-05-04 00:01:00 UTC
    assert_eq!(
        parse_timestamp(&base_sec.to_string(), TimestampUnit::Auto)
            .unwrap()
            .utc_seconds,
        base_sec
    );
    assert_eq!(
        parse_timestamp(
            &(i128::from(base_sec) * 1_000 + 999).to_string(),
            TimestampUnit::Auto
        )
        .unwrap()
        .utc_seconds,
        base_sec
    );
    assert_eq!(
        parse_timestamp(
            &(i128::from(base_sec) * 1_000_000 + 999_999).to_string(),
            TimestampUnit::Auto
        )
        .unwrap()
        .utc_seconds,
        base_sec
    );
    assert_eq!(
        parse_timestamp(
            &(i128::from(base_sec) * 1_000_000_000 + 999_999_999).to_string(),
            TimestampUnit::Auto
        )
        .unwrap()
        .utc_seconds,
        base_sec
    );

    // Reject invalid dates and malformed offsets
    assert!(parse_timestamp("2026-02-29", TimestampUnit::Auto).is_err());
    assert!(parse_timestamp("2026-10-05T24:00:00Z", TimestampUnit::Auto).is_err());
    assert!(parse_timestamp("2026-10-05T10:00:00+25:00", TimestampUnit::Auto).is_err());
    assert!(parse_timestamp("9999-12-31T23:30:00-02:00", TimestampUnit::Auto).is_err());
}

#[test]
fn csv_loader_parses_rfc4180_delimiters_bom_quotes_dedup_and_envelope_policies() {
    let semicolon_csv = "\u{FEFF}\"Date\"; \"Open\" ;\"High\";\"Low\";\"Close\";\"Volume\"\r\n\
        \"2026-10-06\";1210.0;1220.0;1205.0;1218.0;150000\r\n\
        \"2026-10-05\";1200.0;1212.0;1195.0;1208.0;120000\r\n\
        \"2026-10-05\";1200.0;1215.0;1194.0;1210.0;135000\r\n";

    // Default KeepLast sorts ascending and keeps the amended second `2026-10-05` row
    let rep = load_ohlcv_csv(semicolon_csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep.base_timeframe, Timeframe::D1);
    assert_eq!(rep.candles.len(), 2);
    assert_eq!(rep.deduplicated_count, 1);
    assert_eq!(rep.candles[0].date_label, "2026-10-05");
    assert!((rep.candles[0].close - 1210.0).abs() < 1e-9);
    assert_eq!(rep.candles[1].date_label, "2026-10-06");

    // RejectConflict catches conflicting duplicate timestamps
    let conflict_opts = DataLoadOptions {
        duplicate_policy: DuplicateTimestampPolicy::RejectConflict,
        ..DataLoadOptions::default()
    };
    assert!(matches!(
        load_ohlcv_csv(semicolon_csv, &conflict_opts),
        Err(DataLoadError::DuplicateTimestampConflict { .. })
    ));

    // Envelope violation: RejectRow vs ClampToBody
    let bad_env_csv = "time,open,high,low,close\n2026-10-05,105.0,102.0,108.0,100.0\n";
    assert!(matches!(
        load_ohlcv_csv(bad_env_csv, &DataLoadOptions::default()),
        Err(DataLoadError::InvalidOhlcEnvelope { .. })
    ));
    let clamp_rep = load_ohlcv_csv(
        bad_env_csv,
        &DataLoadOptions {
            envelope_policy: OhlcEnvelopePolicy::ClampToBody,
            ..DataLoadOptions::default()
        },
    )
    .unwrap();
    assert_eq!(clamp_rep.repaired_envelope_count, 1);
    assert!(clamp_rep.candles[0].is_valid());
    assert!((clamp_rep.candles[0].high - 105.0).abs() < 1e-9);
    assert!((clamp_rep.candles[0].low - 100.0).abs() < 1e-9);
}

#[test]
fn real_archive_cafef_multi_symbol_and_spy_dst_weekly_aggregation_invariants() {
    // 1. Real CafeF multi-symbol CSV (`HNX-INDEX` + `VNINDEX`)
    let cafef_vn = load_preset_dataset(DataSourcePreset::CafeFIndexCsv, 120, Some("VNINDEX"));
    let cafef_hnx = load_preset_dataset(DataSourcePreset::CafeFIndexCsv, 120, Some("HNX-INDEX"));
    assert_eq!(cafef_vn.symbol, "VNINDEX");
    assert_eq!(cafef_hnx.symbol, "HNX-INDEX");
    assert!(cafef_vn.available_symbols.len() >= 2);
    assert!(cafef_vn.filtered_other_symbol_rows > 0);
    assert_eq!(cafef_vn.base_timeframe, Timeframe::D1);
    assert!(cafef_vn.candles.iter().all(Candle::is_valid));
    assert!(
        cafef_vn
            .candles
            .windows(2)
            .all(|w| w[0].timestamp < w[1].timestamp)
    );
    // Verify zero cross-contamination between HNX-INDEX (< 600) and VNINDEX (opened at 100 in 2000; > 500 across 2016..2026)
    assert!(cafef_vn.candles.last().unwrap().close > 1_000.0);
    assert!(
        cafef_vn
            .candles
            .iter()
            .filter(|c| c.timestamp >= days_from_civil(2016, 1, 1) * 86_400)
            .all(|c| c.close > 500.0)
    );
    assert!(cafef_hnx.candles.iter().all(|c| c.close < 600.0));

    // 2. Real SPY 5-column CSV (`14:30` EST / `13:30` EDT Unix seconds, no volume column)
    let spy = load_preset_dataset(DataSourcePreset::SpyDailyCsv, 120, None);
    assert_eq!(spy.symbol, "SPY");
    assert_eq!(spy.base_timeframe, Timeframe::D1);
    assert!(spy.candles.len() >= 180);
    assert!(spy.candles.iter().all(|c| c.is_valid() && c.volume == 0.0));

    // Roll SPY `1D -> 1W` and verify every weekly bucket is stamped on ISO Monday `00:00:00 UTC`
    let spy_w1 = aggregate_ohlcv(&spy.candles, Timeframe::D1, Timeframe::W1).unwrap();
    assert!(!spy_w1.is_empty() && spy_w1.len() < spy.candles.len());
    for w in &spy_w1 {
        assert!(w.is_valid());
        assert_eq!(w.timestamp.rem_euclid(86_400), 0);
        let dt = decompose_utc(w.timestamp);
        assert_eq!(
            dt.weekday, 0,
            "1W bar {} must anchor on ISO Monday",
            w.date_label
        );
    }
}

#[test]
fn json_loader_parses_objects_tuples_envelopes_escapes_and_rejects_deep_or_malformed_json() {
    // 1. Object array with Unicode surrogate escapes and out-of-order keys
    let obj_json = r#"[
        {"close": 108.5, "low": 99.0, "high": 110.0, "open": 100.0, "time": "2026-10-05T02:00:00Z", "note": "bull \uD83D\uDE80"},
        {"time": "2026-10-05T02:01:00Z", "open": 108.5, "high": 112.0, "low": 107.0, "close": 111.0, "volume": 450.25}
    ]"#;
    let rep_obj = load_ohlcv_json(obj_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_obj.base_timeframe, Timeframe::M1);
    assert_eq!(rep_obj.candles.len(), 2);
    assert_eq!(rep_obj.candles[0].date_label, "2026-10-05 02:00");
    assert_eq!(rep_obj.candles[1].volume, 450.25);

    // 2. Envelope where `"candles"` precedes `"symbol"` and `"timeframe"`, plus extra Binance kline columns
    let env_json = r#"{
        "meta": {"exchange": "BINANCE", "nested": [1, true, false, null]},
        "candles": [
            [1777852800000, "64100.0", "64500.0", "63950.0", "64320.0", "128.4", 1777853699999, "8250000.0", 942],
            [1777853700000, "64320.0", "64800.0", "64200.0", "64710.0", "195.6", 1777854599999, "12600000.0", 1205]
        ],
        "timeframe": "15m",
        "symbol": "BTCUSDT"
    }"#;
    let rep_env = load_ohlcv_json(env_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_env.symbol, "BTCUSDT");
    assert_eq!(rep_env.base_timeframe, Timeframe::M15);
    assert_eq!(rep_env.candles.len(), 2);

    // 3. Reject malformed RFC-8259 inputs (leading zeros, trailing comma, unpaired surrogate, deep nesting)
    assert!(
        load_ohlcv_json(
            r#"[{"time":01,"open":1,"high":2,"low":1,"close":2}]"#,
            &DataLoadOptions::default()
        )
        .is_err()
    );
    assert!(
        load_ohlcv_json(
            r#"[{"time":1700000000,"open":1,"high":2,"low":1,"close":2,}]"#,
            &DataLoadOptions::default()
        )
        .is_err()
    );
    assert!(
        load_ohlcv_json(
            r#"{"symbol":"\uD800","candles":[]}"#,
            &DataLoadOptions::default()
        )
        .is_err()
    );

    let deep_nested = format!(
        "{{\"extra\":{}{}{},\"candles\":[]}}",
        "[".repeat(20),
        "1",
        "]".repeat(20)
    );
    assert!(load_ohlcv_json(&deep_nested, &DataLoadOptions::default()).is_err());
}

#[test]
fn timeframe_lattice_associativity_downsample_guard_and_def001_causality() {
    let rep_1m = generate_intraday_vn30f_1m_report(3_060); // 12 trading days of 1m bars
    let c_1m = &rep_1m.candles;

    let c_5m = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::M5).unwrap();
    let c_15m = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::M15).unwrap();
    let c_1h = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::H1).unwrap();
    let c_4h = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::H4).unwrap();
    let c_1d = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::D1).unwrap();
    let c_1w = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::W1).unwrap();

    // Multi-hop associativity across the entire lattice `1m -> 5m -> 15m -> 1h -> 4h -> 1D -> 1W`
    assert_eq!(
        aggregate_ohlcv(&c_5m, Timeframe::M5, Timeframe::M15).unwrap(),
        c_15m
    );
    assert_eq!(
        aggregate_ohlcv(&c_15m, Timeframe::M15, Timeframe::H1).unwrap(),
        c_1h
    );
    assert_eq!(
        aggregate_ohlcv(&c_1h, Timeframe::H1, Timeframe::D1).unwrap(),
        c_1d
    );
    assert_eq!(
        aggregate_ohlcv(&c_4h, Timeframe::H4, Timeframe::D1).unwrap(),
        c_1d
    );
    assert_eq!(
        aggregate_ohlcv(&c_1d, Timeframe::D1, Timeframe::W1).unwrap(),
        c_1w
    );
    assert_eq!(
        aggregate_ohlcv(&c_15m, Timeframe::M15, Timeframe::W1).unwrap(),
        c_1w
    );

    // Total volume conservation across all timeframes
    let vol_1m: f64 = c_1m.iter().map(|c| c.volume).sum();
    let vol_1w: f64 = c_1w.iter().map(|c| c.volume).sum();
    assert!((vol_1m - vol_1w).abs() < 1e-6);

    // Reject downward aggregation (`CannotDownsample`)
    assert_eq!(
        aggregate_ohlcv(&c_15m, Timeframe::M15, Timeframe::M5),
        Err(AggregationError::CannotDownsample {
            source: Timeframe::M15,
            target: Timeframe::M5,
        })
    );

    // DEF-001 causality guard: mid-week cutoff on Thursday `2026-09-03` (first legal trading day after
    // official `2026-09-01..02` National Day holidays) must NEVER include Friday `2026-09-04`'s high/low/close!
    let thu_cutoff = days_from_civil(2026, 9, 3) * 86_400 + 8 * 3_600;
    let causal_w1 = aggregate_ohlcv_up_to(c_1m, Timeframe::M1, Timeframe::W1, thu_cutoff).unwrap();
    assert_eq!(causal_w1.len(), 1);
    let last_thu_1m = c_1m
        .iter()
        .rfind(|c| c.timestamp <= thu_cutoff)
        .expect("thursday bar");
    assert!((causal_w1[0].close - last_thu_1m.close).abs() < 1e-9);
    assert!(causal_w1[0].volume < c_1w[0].volume);
}

#[test]
fn drawing_store_remap_timeframe_preserves_exact_anchors_across_round_trips_and_undo_redo() {
    let rep_1m = generate_intraday_vn30f_1m_report(2_040); // 8 sessions
    let c_5m = aggregate_ohlcv(&rep_1m.candles, Timeframe::M1, Timeframe::M5).unwrap();
    let c_1h = aggregate_ohlcv(&rep_1m.candles, Timeframe::M1, Timeframe::H1).unwrap();
    let c_1d = aggregate_ohlcv(&rep_1m.candles, Timeframe::M1, Timeframe::D1).unwrap();
    let c_1w = aggregate_ohlcv(&rep_1m.candles, Timeframe::M1, Timeframe::W1).unwrap();

    let mut store = DrawingStore::new();
    // Place a TrendLine and a same-day horizontal Ray on `5m` (indices 37 and 68 fall inside the SAME trading day!)
    let id_tl = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(37.25, 1280.5),
                end: DrawingAnchor::new(195.75, 1294.0),
            },
            None,
        )
        .unwrap();
    let id_ray = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Ray {
                origin: DrawingAnchor::new(12.0, 1285.0),
                through: DrawingAnchor::new(44.0, 1285.0),
            },
            None,
        )
        .unwrap();

    // Switch `5m -> 1h -> 1D -> 1W`
    store.remap_timeframe(&c_5m, Timeframe::M5, &c_1h, Timeframe::H1);
    store.remap_timeframe(&c_1h, Timeframe::H1, &c_1d, Timeframe::D1);
    store.remap_timeframe(&c_1d, Timeframe::D1, &c_1w, Timeframe::W1);

    for d in &store.drawings {
        assert!(
            d.is_valid(),
            "remapped drawing on 1W must stay valid: {d:?}"
        );
    }

    // Undo and Redo while viewing `1W`, then switch all the way back `1W -> 5m`!
    assert!(store.undo());
    assert!(store.redo());
    store.remap_timeframe(&c_1w, Timeframe::W1, &c_5m, Timeframe::M5);

    let tl = store.drawings.iter().find(|d| d.id == id_tl).unwrap();
    let [a0, a1] = tl.geometry.anchors();
    assert!(
        (a0.bar_index - 37.25).abs() < 1e-9 && (a1.bar_index - 195.75).abs() < 1e-9,
        "sub-bar 5m anchors must survive 5m -> 1h -> 1D -> 1W -> 5m round-trip with zero drift: {a0:?} {a1:?}"
    );

    let ray = store.drawings.iter().find(|d| d.id == id_ray).unwrap();
    let [r0, r1] = ray.geometry.anchors();
    assert!(
        (r0.bar_index - 12.0).abs() < 1e-9 && (r1.bar_index - 44.0).abs() < 1e-9,
        "same-day horizontal 5m Ray must recover exact 5m anchors: {r0:?} {r1:?}"
    );
}

#[test]
fn critic_r06_regression_cafef_8col_intraday_dtyyyymmdd_plus_time_columns() {
    let csv = concat!(
        "<Ticker>,<DTYYYYMMDD>,<Time>,<Open>,<High>,<Low>,<Close>,<Volume>\n",
        "VN30F1M,20261005,09:15:00,1280.0,1282.5,1279.5,1281.0,1500\n",
        "VN30F1M,20261005,09:16:00,1281.0,1283.0,1280.5,1282.0,1800\n",
        "VN30F1M,20261005,103000,1282.0,1284.0,1281.5,1283.5,2100\n",
        "VN30F1M,20261005,1415,1283.5,1285.0,1282.0,1284.0,1900\n",
        "VN30F1M,20261006,90530,1284.0,1286.0,1283.0,1285.5,2200\n",
    );
    let rep = load_ohlcv_csv(csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep.symbol, "VN30F1M");
    assert_eq!(rep.base_timeframe, Timeframe::M1);
    assert_eq!(rep.candles.len(), 5);

    let day_2026_10_05 = days_from_civil(2026, 10, 5) * 86_400;
    assert_eq!(
        rep.candles[0].timestamp,
        day_2026_10_05 + 9 * 3_600 + 15 * 60
    );
    assert_eq!(
        rep.candles[1].timestamp,
        day_2026_10_05 + 9 * 3_600 + 16 * 60
    );
    assert_eq!(
        rep.candles[2].timestamp,
        day_2026_10_05 + 10 * 3_600 + 30 * 60
    );
    assert_eq!(
        rep.candles[3].timestamp,
        day_2026_10_05 + 14 * 3_600 + 15 * 60
    );
    assert_eq!(rep.candles[0].date_label, "2026-10-05 09:15");

    let day_2026_10_06 = days_from_civil(2026, 10, 6) * 86_400;
    assert_eq!(
        rep.candles[4].timestamp,
        day_2026_10_06 + 9 * 3_600 + 5 * 60 + 30
    );
}

#[test]
fn critic_r06_regression_series_timestamp_inference_1969_1975_ms_and_1970_epoch_vs_yyyymmdd() {
    // 1. Pre-1970 negative milliseconds (`-86_400_000`, `-86_340_000`) -> `TimestampUnit::Milliseconds`
    let pre_1970_json = r#"[
        [-86400000, 100.0, 102.0, 99.0, 101.0, 50.0],
        [-86340000, 101.0, 103.0, 100.0, 102.0, 60.0]
    ]"#;
    let rep_neg_ms = load_ohlcv_json(pre_1970_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_neg_ms.candles[0].timestamp, -86_400);
    assert_eq!(rep_neg_ms.candles[1].timestamp, -86_340);
    assert_eq!(rep_neg_ms.base_timeframe, Timeframe::M1);

    // 2. 1975 positive milliseconds (`157_766_400_000` < old `1e12` threshold, `> 1e10` new cap)
    let y1975_json = r#"[
        [157766400000, 50.0, 52.0, 49.0, 51.0, 100.0],
        [157766460000, 51.0, 53.0, 50.0, 52.0, 120.0]
    ]"#;
    let rep_1975 = load_ohlcv_json(y1975_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_1975.candles[0].timestamp, 157_766_400);
    assert_eq!(rep_1975.candles[1].timestamp, 157_766_460);
    assert_eq!(rep_1975.base_timeframe, Timeframe::M1);

    // 3. Aug 1970 numeric Unix seconds (`19_800_000`, `19_800_060`) must NEVER be hijacked as `YYYYMMDD`
    let aug_1970_sec_json = r#"[
        [19800000, 10.0, 11.0, 9.5, 10.5, 1000.0],
        [19800060, 10.5, 11.5, 10.0, 11.0, 1100.0],
        [19800120, 11.0, 12.0, 10.5, 11.5, 1200.0]
    ]"#;
    let rep_1970_sec = load_ohlcv_json(aug_1970_sec_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_1970_sec.candles[0].timestamp, 19_800_000);
    assert_eq!(rep_1970_sec.candles[2].timestamp, 19_800_120);
    assert_eq!(rep_1970_sec.base_timeframe, Timeframe::M1);

    // 4. Jan 1970 daily milliseconds (`86_400_000`, `172_800_000`, `259_200_000` < `1e10`) inferred via delta!
    let jan_1970_ms_json = r#"[
        [86400000, 20.0, 21.0, 19.0, 20.5, 500.0],
        [172800000, 20.5, 22.0, 20.0, 21.5, 600.0],
        [259200000, 21.5, 23.0, 21.0, 22.5, 700.0]
    ]"#;
    let rep_1970_ms = load_ohlcv_json(jan_1970_ms_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_1970_ms.candles[0].timestamp, 86_400);
    assert_eq!(rep_1970_ms.candles[2].timestamp, 259_200);
    assert_eq!(rep_1970_ms.base_timeframe, Timeframe::D1);

    // 5. Mixed string column (`"20260101"` + `"1700000000"`) resolves uniformly as epoch seconds (not mixed YYYYMMDD)
    let mixed_csv = "time,open,high,low,close\n20260101,10,12,9,11\n1700000000,11,13,10,12\n";
    let rep_mixed = load_ohlcv_csv(mixed_csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_mixed.candles[0].timestamp, 20_260_101);
    assert_eq!(rep_mixed.candles[1].timestamp, 1_700_000_000);
}

#[test]
fn critic_r06_regression_lossless_decimal_microseconds_comma_fraction_and_leap_second_guard() {
    // 1. Decimal microseconds near `1.7e15 + 999_999.9` where IEEE-754 `f64` loses unit precision
    let us_csv = concat!(
        "time,open,high,low,close\n",
        "1700000000000000.25,100,101,99,100.5\n",
        "1700000060999999.9,100.5,102,100,101.5\n",
    );
    let rep_us = load_ohlcv_csv(us_csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_us.candles[0].timestamp, 1_700_000_000);
    assert_eq!(
        rep_us.candles[1].timestamp, 1_700_000_060,
        "exact integer floor division must never round .9999999s upward via f64"
    );

    // 2. ISO-8601 comma fractional seconds (`02:00:00,500Z`) + valid UTC `23:59:60Z` leap second
    assert!(parse_timestamp("2026-10-05T02:00:00,500Z", TimestampUnit::Auto).is_ok());
    assert!(parse_timestamp("2016-12-31T23:59:60Z", TimestampUnit::Auto).is_ok());

    // 3. Invalid `:60` outside `23:59:60 UTC` must be rejected
    assert!(parse_timestamp("2026-10-05T12:00:60Z", TimestampUnit::Auto).is_err());
    assert!(parse_time_of_day_hms("12:30:60").is_err());
}

#[test]
fn critic_r06_regression_json_klines_mismatched_envelope_symbol_multiline_csv_and_bounded_io() {
    // 1. `"klines"` envelope key + mismatched outer `"BINANCE:BTCUSDT"` vs row `"BTCUSDT"` (`HIGH-05`)
    let klines_json = r#"{
        "symbol": "BINANCE:BTCUSDT",
        "timeframe": "15m",
        "klines": [
            {"symbol": "BTCUSDT", "time": 1777852800000, "open": 64000, "high": 64200, "low": 63900, "close": 64150, "volume": 12.5},
            {"symbol": "BTCUSDT", "time": 1777853700000, "open": 64150, "high": 64400, "low": 64100, "close": 64300, "volume": 18.0}
        ]
    }"#;
    let rep_klines = load_ohlcv_json(klines_json, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_klines.symbol, "BTCUSDT");
    assert_eq!(rep_klines.base_timeframe, Timeframe::M15);
    assert_eq!(rep_klines.candles.len(), 2);

    // 2. Multiline quoted CSV header with `;` delimiter (`MEDIUM-02`)
    let multiline_csv = concat!(
        "\"Ticker\nSymbol\";\"Trade\nDate\";Open;High;Low;Close;Volume\n",
        "FPT;2026-10-05;132.0;135.0;131.5;134.2;2500000\n",
        "FPT;2026-10-06;134.2;136.0;133.8;135.5;2700000\n",
    );
    let rep_ml = load_ohlcv_csv(multiline_csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_ml.symbol, "FPT");
    assert_eq!(rep_ml.candles.len(), 2);

    // 3. Relative tolerance in `DuplicateTimestampPolicy::RejectConflict` (`MEDIUM-03`)
    let micro_conflict_csv = concat!(
        "time,open,high,low,close\n",
        "1700000000,1e-13,2e-13,1e-13,1.5e-13\n",
        "1700000000,2e-13,4e-13,2e-13,3.0e-13\n",
    );
    let strict_opts = DataLoadOptions {
        duplicate_policy: DuplicateTimestampPolicy::RejectConflict,
        ..DataLoadOptions::default()
    };
    assert!(
        load_ohlcv_csv(micro_conflict_csv, &strict_opts).is_err(),
        "2x discrepancy at 1e-13 micro-price scale must be caught by relative tolerance"
    );

    // 4. Coarse `explicit_timeframe = Some(Timeframe::W1)` on daily rows canonicalizes into weekly buckets (`MEDIUM-04`)
    let daily_for_weekly = concat!(
        "date,open,high,low,close,volume\n",
        "2026-10-05,100,105,98,103,1000\n",
        "2026-10-06,103,110,102,108,1500\n",
        "2026-10-12,108,112,106,111,1200\n",
    );
    let w1_opts = DataLoadOptions {
        explicit_timeframe: Some(Timeframe::W1),
        ..DataLoadOptions::default()
    };
    let rep_w1 = load_ohlcv_csv(daily_for_weekly, &w1_opts).unwrap();
    assert_eq!(rep_w1.base_timeframe, Timeframe::W1);
    assert_eq!(rep_w1.candles.len(), 2);
    assert!((rep_w1.candles[0].high - 110.0).abs() < 1e-9);
    assert!((rep_w1.candles[0].volume - 2_500.0).abs() < 1e-9);

    // 5. Bounded `SymbolTracker` (`MAX_DISCOVERED_SYMBOLS = 2_048`) + `load_ohlcv_file` (`LOW-01`, `HIGH-06`)
    let mut many_sym_csv = String::from("symbol,date,open,high,low,close\n");
    for i in 0..(MAX_DISCOVERED_SYMBOLS + 44) {
        many_sym_csv.push_str(&format!("SYM{i:04},2026-10-05,10,11,9,10.5\n"));
    }
    let rep_many = load_ohlcv_csv(&many_sym_csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(rep_many.available_symbols.len(), MAX_DISCOVERED_SYMBOLS);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let tmp_path =
            std::env::temp_dir().join(format!("sumi_r06_critic_test_{}.csv", std::process::id()));
        std::fs::write(&tmp_path, daily_for_weekly).unwrap();
        let file_rep = load_ohlcv_file(&tmp_path, &DataLoadOptions::default()).unwrap();
        let _ = std::fs::remove_file(&tmp_path);
        assert_eq!(file_rep.candles.len(), 3);
    }
}

#[test]
fn critic_r06_regression_coarse_to_fine_bucket_open_snap_and_ray_boundary_and_vertical_drag() {
    use crate::drawings::{DrawingTool, LocalPoint, PaneDrawingProjection};
    use crate::viewport::{HorzScaleOptions, PriceScale, PriceScaleOptions, TimeScale};

    // 1. Drawing created directly on `1W` (`bar_index = 1.0` -> Monday `00:00 UTC`) remapped down to `1m`
    //    must snap to Monday's first actual `1m` session bar (`02:00 UTC`), NOT extrapolate `-120` bars left (`HIGH-01`)!
    let rep_1m = generate_intraday_vn30f_1m_report(2_040);
    let c_1m = &rep_1m.candles;
    let c_1w = aggregate_ohlcv(c_1m, Timeframe::M1, Timeframe::W1).unwrap();

    let mut store = DrawingStore::new();
    let w1_line_id = store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::TrendLine {
                start: DrawingAnchor::new(0.0, 1280.0),
                end: DrawingAnchor::new(1.0, 1290.0),
            },
            None,
        )
        .unwrap();
    store.remap_timeframe(&c_1w, Timeframe::W1, c_1m, Timeframe::M1);

    let remapped_tl = store.drawings.iter().find(|d| d.id == w1_line_id).unwrap();
    let [a0, a1] = remapped_tl.geometry.anchors();
    let expected_w1_open_idx = c_1m
        .iter()
        .position(|c| c.timestamp >= c_1w[1].timestamp)
        .unwrap() as f64;
    assert!(
        (a0.bar_index - 0.0).abs() < 1e-9,
        "week 0 open (Tuesday 2026-09-01 02:00 UTC) must snap to bar 0, got {}",
        a0.bar_index
    );
    assert!(
        (a1.bar_index - expected_w1_open_idx).abs() < 1e-9,
        "week 1 open must snap to Monday 02:00 UTC bar {expected_w1_open_idx}, got {}",
        a1.bar_index
    );

    // 2. Ray clamping at `+MAX_ABS_BAR_INDEX` preserves rightward direction (`MEDIUM-01`)
    let mut far_store = DrawingStore::new();
    let ray_id = far_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::Ray {
                origin: DrawingAnchor::with_timestamp(
                    10.0,
                    1280.0,
                    (c_1m[0].timestamp + 500_000_000_000) as f64,
                ),
                through: DrawingAnchor::with_timestamp(
                    20.0,
                    1285.0,
                    (c_1m[0].timestamp + 600_000_000_000) as f64,
                ),
            },
            None,
        )
        .unwrap();
    far_store.remap_timeframe(c_1m, Timeframe::M1, c_1m, Timeframe::M1);
    let far_ray = far_store.drawings.iter().find(|d| d.id == ray_id).unwrap();
    assert!(far_ray.is_valid());
    let [ro, rt] = far_ray.geometry.anchors();
    assert!(
        rt.bar_index > ro.bar_index,
        "clamped rightward Ray must keep through > origin, got {ro:?} -> {rt:?}"
    );

    // 3. Vertical-only `DraggingBody` (`dx == 0.0`) preserves exact `bar_index` and `timestamp` (`MEDIUM-01`)
    let mut ts = TimeScale::new(HorzScaleOptions::default());
    ts.set_width(1_000.0);
    ts.set_total_bars(c_1m.len());
    let mut ps = PriceScale::new("right", PriceScaleOptions::default());
    ps.set_height(500.0);
    ps.autoscale_raw_bounds(1250.0, 1320.0, 1280.0);
    let proj = PaneDrawingProjection::new(&ts, &ps, 1_000.0, 500.0, 1280.0);

    let mut drag_store = DrawingStore::new();
    let orig_ts = c_1m[c_1m.len() - 20].timestamp;
    let orig_idx = (c_1m.len() - 20) as f64;
    let h_id = drag_store
        .add_drawing(
            DrawingPaneTarget::Main,
            DrawingGeometry::HorizontalLine {
                anchor: DrawingAnchor::with_timestamp(orig_idx, 1285.0, orig_ts as f64),
            },
            None,
        )
        .unwrap();
    drag_store.set_tool(DrawingTool::Select);
    let start_lp = proj
        .anchor_to_local(DrawingAnchor::new(orig_idx, 1285.0))
        .unwrap();
    let end_lp = LocalPoint::new(start_lp.x, start_lp.y - 25.0);
    let _ = drag_store.pointer_down(DrawingPaneTarget::Main, start_lp, false, &proj, c_1m);
    drag_store.pointer_move(DrawingPaneTarget::Main, end_lp, false, &proj, c_1m);
    let _ = drag_store.pointer_up(DrawingPaneTarget::Main, Some(end_lp), false, &proj, c_1m);

    let dragged = drag_store.drawings.iter().find(|d| d.id == h_id).unwrap();
    let [da, _] = dragged.geometry.anchors();
    assert!((da.bar_index - orig_idx).abs() < 1e-9);
    assert_eq!(da.timestamp, Some(orig_ts as f64));

    // Also verify pure vertical `DraggingBody` away from the handle (`start_lp.x - 80.0`)
    let body_start = LocalPoint::new(start_lp.x - 80.0, end_lp.y);
    let body_end = LocalPoint::new(body_start.x, body_start.y + 30.0);
    let _ = drag_store.pointer_down(DrawingPaneTarget::Main, body_start, false, &proj, c_1m);
    drag_store.pointer_move(DrawingPaneTarget::Main, body_end, false, &proj, c_1m);
    let _ = drag_store.pointer_up(DrawingPaneTarget::Main, Some(body_end), false, &proj, c_1m);
    let body_dragged = drag_store.drawings.iter().find(|d| d.id == h_id).unwrap();
    let [dba, _] = body_dragged.geometry.anchors();
    assert!((dba.bar_index - orig_idx).abs() < 1e-9);
    assert_eq!(dba.timestamp, Some(orig_ts as f64));
}

#[test]
fn r19_cafef_solieugd_5_rule_post_split_normalization_and_rejections() {
    // 1. Valid 1-ULP (`0.0001 kVND == 0.1 VND`) post-split truncation on `Open` (`Open=24.6154 > High=24.6153`):
    let out_open_ulp = evaluate_and_normalize_cafef_split_ohlc(
        "FPT",
        24.6154,
        24.6153,
        24.2000,
        24.4500,
        1_250_000.0,
    );
    assert!(matches!(
        out_open_ulp,
        CafeFSplitNormOutcome::NormalizedSplitPrecision { new_high, new_low, .. }
            if (new_high - 24.6154).abs() < 1e-9 && (new_low - 24.2000).abs() < 1e-9
    ));

    // 2. Valid `Close` post-split discrepancy within `<= 6 bps` & `<= 25 VND` & `< 0.5 * min_step`:
    //    `O = 50.1234`, `H = 50.8234`, `L = 49.5234`, `C = 50.8354` (`delta_close = 12.0 VND = 2.36 bps`, `min_step = 700 VND`):
    let out_close_bps = evaluate_and_normalize_cafef_split_ohlc(
        "MWG", 50.1234, 50.8234, 49.5234, 50.8354, 850_000.0,
    );
    assert!(matches!(
        out_close_bps,
        CafeFSplitNormOutcome::NormalizedSplitPrecision { new_high, .. }
            if (new_high - 50.8354).abs() < 1e-9
    ));

    // 3. Rule 1 rejection: Index / futures / US (`AAPL`, `MSFT`, `SPY`) / Crypto symbols NEVER qualify:
    for non_vn_eq in [
        "VNINDEX",
        "HNX-INDEX",
        "VNALL",
        "VNMID",
        "HNXFIN",
        "VNFINSELECT",
        "VN30F1M",
        "SPY",
        "AAPL",
        "MSFT",
        "NVDA",
        "BTCUSDT",
    ] {
        assert_eq!(
            evaluate_and_normalize_cafef_split_ohlc(
                non_vn_eq, 185.235, 185.50, 184.10, 185.51, 100_000.0,
            ),
            CafeFSplitNormOutcome::RejectedGenuineViolation,
            "{non_vn_eq} must be rejected by Rule 1"
        );
    }
    // `[M4]`: `RawVnd` distressed penny stock at `950 VND` with unadjusted `10 VND` ticks is rejected,
    // while `kVND` luxury `1,000.0004` and domestic ETF `E1VFVN30` / CW `CFPT2501` normalize cleanly:
    assert_eq!(
        evaluate_and_normalize_cafef_split_ohlc("HAG", 950.0, 940.0, 910.0, 930.0, 50_000.0),
        CafeFSplitNormOutcome::RejectedGenuineViolation
    );
    assert!(matches!(
        evaluate_and_normalize_cafef_split_ohlc(
            "VNZ", 1000.0004, 1000.0003, 980.0000, 990.0000, 10_000.0
        ),
        CafeFSplitNormOutcome::NormalizedSplitPrecision { .. }
    ));
    assert!(matches!(
        evaluate_and_normalize_cafef_split_ohlc(
            "E1VFVN30", 24.6154, 24.6153, 24.2000, 24.4500, 100_000.0
        ),
        CafeFSplitNormOutcome::NormalizedSplitPrecision { .. }
    ));

    // 4. Rule 2 rejection: Unadjusted round 10-VND ticks (`0.01 kVND` multiples) without post-split precision:
    assert_eq!(
        evaluate_and_normalize_cafef_split_ohlc("FPT", 24.62, 24.61, 24.20, 24.45, 500_000.0),
        CafeFSplitNormOutcome::RejectedGenuineViolation
    );

    // 5. Rule 3 & Rule 4 rejections: `Open` > 1 ULP (`0.5 VND`) OR `Close` > `6 bps` / `25 VND`:
    assert_eq!(
        evaluate_and_normalize_cafef_split_ohlc(
            "FPT", 24.6159, 24.6153, 24.2000, 24.4500, 500_000.0
        ),
        CafeFSplitNormOutcome::RejectedGenuineViolation
    );
    assert_eq!(
        evaluate_and_normalize_cafef_split_ohlc(
            "FPT", 50.1234, 50.8234, 49.5234, 50.8600, // +36.6 VND > 25 VND cap!
            500_000.0
        ),
        CafeFSplitNormOutcome::RejectedGenuineViolation
    );

    // 6. End-to-end CSV & JSON loader integration with `OhlcEnvelopePolicy::CafeFSplitAware`:
    let split_csv = concat!(
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n",
        "FPT,20260428,24.6154,24.6153,24.2154,24.4500,1200000\n",
        "FPT,20260429,24.4500,24.8500,24.4232,24.4231,1350000\n",
    );
    assert!(matches!(
        load_ohlcv_csv(split_csv, &DataLoadOptions::default()),
        Err(DataLoadError::InvalidOhlcEnvelope { .. })
    ));
    let split_rep = load_ohlcv_csv(
        split_csv,
        &DataLoadOptions {
            envelope_policy: OhlcEnvelopePolicy::CafeFSplitAware,
            ..DataLoadOptions::default()
        },
    )
    .expect("CafeFSplitAware must normalize valid post-split 4-decimal rows");
    assert_eq!(split_rep.candles.len(), 2);
    assert_eq!(split_rep.repaired_envelope_count, 2);
    assert!(split_rep.candles.iter().all(Candle::is_valid));
    assert!((split_rep.candles[0].open - 24.6154).abs() < 1e-9);
    assert!((split_rep.candles[0].high - 24.6154).abs() < 1e-9);
    assert!((split_rep.candles[1].close - 24.4231).abs() < 1e-9);
    assert!((split_rep.candles[1].low - 24.4231).abs() < 1e-9);

    // Genuine corruption (`Low > High`) is STILL rejected even under `CafeFSplitAware`!
    let corrupt_csv = "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20260428,24.6154,24.1000,24.5000,24.4500,1000\n";
    assert!(matches!(
        load_ohlcv_csv(
            corrupt_csv,
            &DataLoadOptions {
                envelope_policy: OhlcEnvelopePolicy::CafeFSplitAware,
                ..DataLoadOptions::default()
            }
        ),
        Err(DataLoadError::InvalidOhlcEnvelope { .. })
    ));
}

#[test]
fn r19_data_quality_audit_vn_holidays_w1_no_false_gaps_outliers_and_causal_readiness() {
    use crate::journal::ExchangeVenue;

    let make_bar = |y: i32, m: u32, d: u32, close: f64, vol: f64| -> Candle {
        Candle {
            timestamp: days_from_civil(y, m, d) * 86_400,
            date_label: format!("{y:04}-{m:02}-{d:02}"),
            open: close * 0.995,
            high: close * 1.01,
            low: close * 0.99,
            close,
            volume: vol,
        }
    };

    // 1. Continuous FPT `D1` series spanning Tết Bính Ngọ (`2026-02-13` Fri -> `2026-02-23` Mon)
    //    and `30/04–01/05` (`2026-04-29` Wed -> `2026-05-04` Mon): ZERO false holiday/weekend gaps!
    let mut fpt_clean = Vec::new();
    let start_d = days_from_civil(2026, 2, 9);
    let end_d = days_from_civil(2026, 5, 8);
    for d in start_d..=end_d {
        if crate::journal::is_vn_exchange_trading_day_days(d) {
            let (y, m, day) = civil_from_days(d);
            fpt_clean.push(make_bar(y, m, day, 130.0, 1_000_000.0));
        }
    }
    let clean_audit = audit_ohlcv_series_with_venue(
        "FPT",
        ExchangeVenue::Hose,
        Timeframe::D1,
        &fpt_clean,
        Some((0, 0)),
    );
    assert_eq!(clean_audit.status, DataReadinessStatus::Ready);
    assert_eq!(clean_audit.missing_session_count, 0);
    assert_eq!(clean_audit.holiday_or_weekend_bar_count, 0);
    assert!((clean_audit.coverage_pct - 100.0).abs() < 1e-9);

    // 2. Aggregated `W1` across the exact same span (including the full-week Tết break!) reports 0 false daily gaps!
    let fpt_w1 = aggregate_ohlcv(&fpt_clean, Timeframe::D1, Timeframe::W1).unwrap();
    let w1_audit =
        audit_ohlcv_series_with_venue("FPT", ExchangeVenue::Hose, Timeframe::W1, &fpt_w1, None);
    assert_eq!(w1_audit.status, DataReadinessStatus::Ready);
    assert_eq!(w1_audit.missing_session_count, 0);
    assert!((w1_audit.coverage_pct - 100.0).abs() < 1e-9);

    // 3. Remove Wednesday `2026-03-11`, inject a holiday bar on `2026-05-01`, a `12x` volume spike,
    //    and an unadjusted 2:1 split jump (`130.0 -> 65.0`): verify every detector fires accurately!
    let mut fpt_dirty: Vec<Candle> = fpt_clean
        .into_iter()
        .filter(|c| c.date_label != "2026-03-11")
        .collect();
    if let Some(bar25) = fpt_dirty.get_mut(25) {
        bar25.volume = 12_500_000.0; // > 10x RVOL(20)
    }
    if let Some(bar30) = fpt_dirty.get_mut(30) {
        bar30.open = 65.0;
        bar30.high = 66.0;
        bar30.low = 64.0;
        bar30.close = 65.0; // -50% unadjusted split jump exceeds HOSE -7% floor!
    }
    fpt_dirty.push(make_bar(2026, 5, 1, 65.0, 500_000.0)); // International Labor Day holiday bar (also out-of-order!)
    fpt_dirty.sort_by_key(|c| c.timestamp);

    let dirty_audit = audit_ohlcv_series_with_venue(
        "FPT",
        ExchangeVenue::Hose,
        Timeframe::D1,
        &fpt_dirty,
        Some((1, 2)),
    );
    assert_eq!(dirty_audit.status, DataReadinessStatus::PartialWarning);
    assert_eq!(dirty_audit.missing_session_count, 1);
    assert_eq!(dirty_audit.missing_session_samples, vec!["2026-03-11"]);
    assert_eq!(dirty_audit.holiday_or_weekend_bar_count, 1);
    assert!(dirty_audit.volume_spike_10x_count >= 1);
    assert!(dirty_audit.band_exceedance_split_hint_count >= 1);
    assert_eq!(dirty_audit.loader_deduplicated_rows, 1);
    assert_eq!(dirty_audit.loader_repaired_envelope_rows, 2);

    // 4. Causal `summarize_universe_readiness` withholds post-cutoff candles (`DEF-001`) AND
    //    `[M5]` audits every feed honestly (`11 / 13` gap-free feeds `Ready`, while the 2 `p0b` archive
    //    `CafeF.INDEX.clean.csv` series genuinely contain historical `2018-01-23..24` HOSE halt & `HNX` test gaps!):
    let feeds = load_bundled_watchlist_universe();
    let full_uni = summarize_universe_readiness(&feeds, None);
    assert_eq!(full_uni.symbols_total, 13);
    assert_eq!(full_uni.symbols_ready, 11);
    assert_eq!(full_uni.status, DataReadinessStatus::PartialWarning);
    let non_cafef: Vec<_> = feeds
        .iter()
        .filter(|f| f.preset != DataSourcePreset::CafeFIndexCsv)
        .cloned()
        .collect();
    let clean_uni = summarize_universe_readiness(&non_cafef, None);
    assert_eq!(clean_uni.status, DataReadinessStatus::Ready);
    assert_eq!(clean_uni.symbols_ready, 11);
    let early_cutoff = feeds[0].candles[10].timestamp + 86_400;
    let cut_uni = summarize_universe_readiness(&feeds, Some(early_cutoff));
    assert!(cut_uni.total_candles < full_uni.total_candles);

    // `[m2]`: `W1` spurious closed-Tết-week bar CANNOT mask a missing valid open week, and out-of-order
    //         candles still report true `min_ts` / `max_ts` for `earliest_date` / `latest_date`!
    let mut w1_masked: Vec<Candle> = fpt_w1
        .into_iter()
        .enumerate()
        .filter_map(|(idx, c)| (idx != 4).then_some(c)) // drop 1 valid open week!
        .collect();
    // Inject a spurious bar during the closed Tết week (`2026-02-16` Monday):
    w1_masked.push(make_bar(2026, 2, 16, 130.0, 100_000.0));
    let w1_masked_rep =
        audit_ohlcv_series_with_venue("FPT", ExchangeVenue::Hose, Timeframe::W1, &w1_masked, None);
    assert_eq!(w1_masked_rep.missing_session_count, 1);
    assert_eq!(w1_masked_rep.status, DataReadinessStatus::PartialWarning);
}

#[test]
fn r19_multi_symbol_cafef_quarantine_and_471_symbol_single_pass_loader() {
    use super::{InvalidRowPolicy, load_ohlcv_csv_multi_symbol};

    // 1. Deterministic synthetic multi-symbol CafeF SolieuGD batch containing:
    //    - `C69` valid 4-decimal post-split rounding (`16.9506` High vs `16.9523` Close -> repaired!)
    //    - `BVS` `2025-05-05` `High = 0` vendor KRX glitch -> quarantined without aborting `BVS`!
    //    - `HNX-INDEX` `2025-07-16` `High = 3` vendor glitch -> quarantined without aborting `HNX-INDEX`!
    let batch_csv = "\
<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>
C69,20261030,16.8543,16.9506,16.7579,16.9523,123300
C69,20261031,16.9000,17.2000,16.8000,17.1000,150000
BVS,20250505,29.6856,0,29.1974,29.4903,297800
BVS,20250506,29.5000,30.1000,29.4000,29.9000,210000
HNX-INDEX,20250716,240.46,3,239.78,241.31,150038511
HNX-INDEX,20250717,241.31,244.10,240.80,243.55,142000000
";

    let opts = DataLoadOptions {
        envelope_policy: OhlcEnvelopePolicy::CafeFSplitAware,
        invalid_row_policy: InvalidRowPolicy::QuarantineRow,
        ..DataLoadOptions::default()
    };
    let reports = load_ohlcv_csv_multi_symbol(batch_csv, &opts).expect("multi-symbol batch loads");
    assert_eq!(reports.len(), 3);
    assert_eq!(reports[0].symbol, "C69");
    assert_eq!(reports[0].candles.len(), 2);
    assert_eq!(reports[0].repaired_envelope_count, 1);
    assert_eq!(reports[0].quarantined_row_count, 0);

    assert_eq!(reports[1].symbol, "BVS");
    assert_eq!(reports[1].candles.len(), 1);
    assert_eq!(reports[1].quarantined_row_count, 1);

    assert_eq!(reports[2].symbol, "HNX-INDEX");
    assert_eq!(reports[2].candles.len(), 1);
    assert_eq!(reports[2].quarantined_row_count, 1);

    // 2. When unpacked local `data/raw/cafef/` files are present, verify full 471-symbol Adjusted
    //    dataset (`941,648` rows) + 2-index `CafeF.INDEX` load cleanly via pure core multi-symbol loader!
    #[cfg(not(target_arch = "wasm32"))]
    {
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("."));
        let adj_path =
            workspace_root.join("data/raw/cafef/CafeF.SolieuGD.Adjusted.Upto06112026.csv");
        let idx_path = workspace_root.join("data/raw/cafef/CafeF.INDEX.Upto06112026.csv");
        if adj_path.exists() && idx_path.exists() {
            let cafef_opts = DataLoadOptions {
                envelope_policy: OhlcEnvelopePolicy::CafeFSplitAware,
                invalid_row_policy: InvalidRowPolicy::QuarantineRow,
                ..DataLoadOptions::default()
            };
            let adj_csv = std::fs::read_to_string(&adj_path).expect("read CafeF Adjusted CSV");
            let adj_reports = load_ohlcv_csv_multi_symbol(&adj_csv, &cafef_opts)
                .expect("471-symbol CafeF Adjusted CSV must load cleanly");
            assert_eq!(adj_reports.len(), 471);

            let fpt = adj_reports
                .iter()
                .find(|r| r.symbol == "FPT")
                .expect("FPT present");
            assert!(fpt.candles.len() > 2_000);

            for ticker in ["VCB", "HPG", "SSI", "BVS", "C69", "MWG"] {
                let rep = adj_reports
                    .iter()
                    .find(|r| r.symbol == ticker)
                    .unwrap_or_else(|| panic!("ticker {ticker} present"));
                assert!(!rep.candles.is_empty());
            }

            let idx_csv = std::fs::read_to_string(&idx_path).expect("read CafeF INDEX CSV");
            let idx_reports = load_ohlcv_csv_multi_symbol(&idx_csv, &cafef_opts)
                .expect("2-symbol CafeF INDEX CSV must load cleanly with quarantined bad rows");
            assert_eq!(idx_reports.len(), 2);
            let vnindex = idx_reports
                .iter()
                .find(|r| r.symbol == "VNINDEX")
                .expect("VNINDEX present");
            assert!(vnindex.candles.len() > 5_000);
        }
    }
}

#[test]
fn wave2_regression_cafef_rule5_real_vnindex_restart_cmdk_timestamp_venues_and_quarantine() {
    use crate::journal::{ExchangeVenue, HNX_LISTED_SYMBOLS, UPCOM_LISTED_SYMBOLS};
    use crate::replay::ReplayController;

    // 1. BUG-005: CafeFSplitAware Rule 5 accepts valid flat post-split session (`O == H == L`, `Close` within 6 bps & 25 VND):
    let flat_split = evaluate_and_normalize_cafef_split_ohlc(
        "RCL", 12.3456, 12.3456, 12.3456, 12.3470, 50_000.0,
    );
    assert!(matches!(
        flat_split,
        CafeFSplitNormOutcome::NormalizedSplitPrecision { new_high, .. }
            if (new_high - 12.3470).abs() < 1e-9
    ));

    // 2. BUG-013: Modern Unix-seconds ending in `000` (`2024-10-04..09`) stay `Seconds`, never `1970`:
    let sec_csv = "time,open,high,low,close,volume\n1728000000,100,102,99,101,1000\n1728432000,101,103,100,102,1200\n";
    let sec_rep = load_ohlcv_csv(sec_csv, &DataLoadOptions::default()).unwrap();
    assert_eq!(sec_rep.candles[0].date_label, "2024-10-04");
    assert_eq!(sec_rep.candles[1].date_label, "2024-10-09");
    assert!(
        matches!(sec_rep.base_timeframe, Timeframe::D1 | Timeframe::W1),
        "5-day Unix-seconds step must stay daily/weekly (never 1970 M5), got {:?}",
        sec_rep.base_timeframe
    );

    // 3. BUG-021: Strictly sorted HNX & UPCOM registries + UPCOM 35.29% legal range not flagged EXTREME_RANGE:
    for w in HNX_LISTED_SYMBOLS.windows(2) {
        assert!(
            w[0] < w[1],
            "HNX_LISTED_SYMBOLS unsorted: {} >= {}",
            w[0],
            w[1]
        );
    }
    for w in UPCOM_LISTED_SYMBOLS.windows(2) {
        assert!(
            w[0] < w[1],
            "UPCOM_LISTED_SYMBOLS unsorted: {} >= {}",
            w[0],
            w[1]
        );
    }
    for hnx_sym in ["C69", "NDN", "BVS", "API", "APS", "RCL", "SHS", "PVS"] {
        assert_eq!(
            ExchangeVenue::infer_from_symbol_and_preset(hnx_sym, DataSourcePreset::CustomFile),
            ExchangeVenue::Hnx,
            "{hnx_sym} must resolve to HNX"
        );
    }
    for upcom_sym in ["KSV", "ABB", "ABI", "ACV", "VGI", "QNS"] {
        assert_eq!(
            ExchangeVenue::infer_from_symbol_and_preset(upcom_sym, DataSourcePreset::CustomFile),
            ExchangeVenue::Upcom,
            "{upcom_sym} must resolve to UPCOM"
        );
    }
    let upcom_bars = vec![Candle {
        timestamp: days_from_civil(2026, 5, 4) * 86_400,
        date_label: "2026-05-04".to_owned(),
        open: 10.0,
        high: 11.5,
        low: 8.5,
        close: 11.5,
        volume: 10_000.0,
    }];
    let upcom_audit = audit_ohlcv_series_with_venue(
        "KSV",
        ExchangeVenue::Upcom,
        Timeframe::D1,
        &upcom_bars,
        None,
    );
    assert_eq!(upcom_audit.extreme_range_bar_count, 0);

    // 4. BUG-025: VNINDEX Synth has 0 weekend/holiday bars; VN30F1M 1m skips 2026-09-01..02 holidays:
    let synth = generate_synthetic_ohlcv(120);
    let synth_audit = audit_ohlcv_series_with_venue(
        "VNINDEX",
        ExchangeVenue::VnIndexBenchmark,
        Timeframe::D1,
        &synth,
        None,
    );
    assert_eq!(synth_audit.holiday_or_weekend_bar_count, 0);
    assert_eq!(synth_audit.missing_session_count, 0);
    let vn30f = generate_intraday_vn30f_1m_report(600);
    assert!(
        vn30f.candles[0].date_label.starts_with("2026-09-03"),
        "VN30F1M 1m must skip National Day 2026-09-01..02, got {}",
        vn30f.candles[0].date_label
    );
    let vn30f_audit = audit_ohlcv_series_with_venue(
        "VN30F1M",
        ExchangeVenue::Vn30Futures,
        Timeframe::M1,
        &vn30f.candles,
        None,
    );
    assert_eq!(vn30f_audit.holiday_or_weekend_bar_count, 0);

    // 5. BUG-031: Leading blank lines delimiter detection, single/multi CSV QuarantineRow, JSON QuarantineRow,
    //    duplicate status downgrade, ReplayController usize::MAX safety:
    let blank_semi_csv = "\n   \n\t\ndate;open;high;low;close;volume\n2026-05-04;10;12;9;11;500\nSHORT_BAD_ROW\n2026-05-05;11;13;10;12;600\n";
    let q_opts = DataLoadOptions {
        invalid_row_policy: InvalidRowPolicy::QuarantineRow,
        ..DataLoadOptions::default()
    };
    let q_rep = load_ohlcv_csv(blank_semi_csv, &q_opts).unwrap();
    assert_eq!(q_rep.candles.len(), 2);
    assert_eq!(q_rep.quarantined_row_count, 1);
    assert!(q_rep.raw_row_count >= q_rep.quarantined_row_count);

    let multi_bad_ts = "symbol,time,open,high,low,close,volume\nAAA,2026-05-04,10,12,9,11,500\nAAA,999999999999999999999,10,12,9,11,500\nAAA,2026-05-05,11,13,10,12,600\n";
    let m_reps = load_ohlcv_csv_multi_symbol(multi_bad_ts, &q_opts).unwrap();
    assert_eq!(m_reps.len(), 1);
    assert_eq!(m_reps[0].candles.len(), 2);
    assert_eq!(m_reps[0].quarantined_row_count, 1);

    let bad_json = r#"[{"time":1777852800,"open":10,"high":12,"low":9,"close":11,"volume":100},{"time":"not-a-date","open":10,"high":12,"low":9,"close":11,"volume":100},{"time":1777939200,"open":11,"high":13,"low":10,"close":12,"volume":200}]"#;
    let j_rep = load_ohlcv_json(bad_json, &q_opts).unwrap();
    assert_eq!(j_rep.candles.len(), 2);
    assert_eq!(j_rep.quarantined_row_count, 1);

    let dup_bars = vec![synth[0].clone(), synth[0].clone()];
    let dup_audit = audit_ohlcv_series_with_venue(
        "VNINDEX",
        ExchangeVenue::VnIndexBenchmark,
        Timeframe::D1,
        &dup_bars,
        None,
    );
    assert_eq!(dup_audit.duplicate_timestamp_count, 1);
    assert_eq!(dup_audit.status, DataReadinessStatus::PartialWarning);

    let mut rc = ReplayController::new();
    rc.status = crate::replay::ReplayStatus::Paused;
    rc.base_cursor_index = usize::MAX;
    assert!(
        rc.materialize_candles(&synth[..5], Timeframe::D1, Timeframe::D1)
            .is_ok()
    );

    // 6. BUG-003 + BUG-006 + BUG-007 + BUG-012 on real CafeF data (when unpacked):
    #[cfg(not(target_arch = "wasm32"))]
    {
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("."));
        let adj_path =
            workspace_root.join("data/raw/cafef/CafeF.SolieuGD.Adjusted.Upto06112026.csv");
        let idx_path = workspace_root.join("data/raw/cafef/CafeF.INDEX.Upto06112026.csv");
        if adj_path.exists() && idx_path.exists() {
            let cafef_opts = DataLoadOptions {
                envelope_policy: OhlcEnvelopePolicy::CafeFSplitAware,
                invalid_row_policy: InvalidRowPolicy::QuarantineRow,
                ..DataLoadOptions::default()
            };
            let adj_csv = std::fs::read_to_string(&adj_path).unwrap();
            let adj_reports = load_ohlcv_csv_multi_symbol(&adj_csv, &cafef_opts).unwrap();
            assert_eq!(adj_reports.len(), 471);

            // BUG-005 real check: RCL retains ~1993 bars (no longer drops 798 flat post-split sessions!)
            let rcl = adj_reports
                .iter()
                .find(|r| r.symbol == "RCL")
                .expect("RCL present");
            assert!(
                rcl.candles.len() > 1_950,
                "RCL must retain valid post-split flat sessions (>1950 bars), got {}",
                rcl.candles.len()
            );

            let fpt = adj_reports
                .iter()
                .find(|r| r.symbol == "FPT")
                .expect("FPT present");
            assert!(fpt.candles.len() > 2_000);

            // BUG-006: Real VNINDEX benchmark loaded (not synthetic sine wave ~1294->2508):
            let uni = load_bundled_watchlist_universe();
            let (bsym, _, bcandles) = resolve_benchmark_series_for_window(
                "VNINDEX",
                fpt.candles.first().unwrap().timestamp,
                fpt.candles.last().unwrap().timestamp,
                &uni,
            )
            .expect("real VNINDEX benchmark");
            assert_eq!(bsym, "VNINDEX");
            assert!(
                bcandles.last().unwrap().close < 2_200.0 && bcandles.len() > 5_000,
                "must resolve real CafeF VNINDEX (>5000 bars), got len={} last={}",
                bcandles.len(),
                bcandles.last().unwrap().close
            );

            // BUG-012: RejectRow OHLC envelope policy rejects bad OHLC CSV cleanly:
            let bad_csv = "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nC69,20261030,10.0,10.0,10.0,10.5,1000\n";
            let err = load_ohlcv_csv(
                bad_csv,
                &DataLoadOptions {
                    envelope_policy: OhlcEnvelopePolicy::RejectRow,
                    invalid_row_policy: InvalidRowPolicy::RejectDataset,
                    ..DataLoadOptions::default()
                },
            );
            assert!(
                err.is_err(),
                "RejectRow policy must reject OHLC envelope violation"
            );

            // BUG-003: ReplayController round-trip serialization preserves FPT cursor:
            let mut replay = ReplayController::new();
            assert!(replay.start_at_base_index(&fpt.candles, 2_000).is_some());
            let encoded = serde_json::to_string(&replay).unwrap();
            let mut restored: ReplayController = serde_json::from_str(&encoded).unwrap();
            restored.sanitize_deserialized(&fpt.candles);
            assert!(restored.is_active());
            assert_eq!(restored.base_cursor_index, 2_000);
        }
    }
}
