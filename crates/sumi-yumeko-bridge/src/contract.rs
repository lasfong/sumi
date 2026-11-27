//! Wire-identical `yumeko_sumi_contract_v1` DTOs, mandatory `kVND <-> VND` Scale Guard,
//! and `KT01GateVerdictV1` technical confirmation bundle (always `advisory_only = true`).
//!
//! Kept standalone (zero relative Cargo path dependency on `../yumeko`) so Clean-Slate
//! Windows/macOS/Linux builds always succeed without a sibling Yumeko checkout.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sumi_data::{OhlcvBar, canonicalize_bar_timestamp_ms, Interval};
use thiserror::Error;

pub const SUMI_CONTRACT_VERSION_V1: &str = "yumeko_sumi_contract_v1";
pub const SUMI_KT01_VERDICT_VERSION_V1: &str = "sumi_kt01_verdict_v1";

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("bridge_sqlite_error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("bridge_io_error: {0}")]
    Io(#[from] std::io::Error),
    #[error("bridge_json_error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bridge_contract_invalid: {0}")]
    Contract(String),
    #[error("bridge_integrity_failed: {0}")]
    Integrity(String),
    #[error("bridge_scale_guard_violation: {0}")]
    ScaleGuard(String),
}

pub type BridgeResult<T> = Result<T, BridgeError>;

// ============================================================================
// Wire-identical `yumeko_types` & `yumeko_sumi_contract` V1 primitives
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeframeV1 {
    Intraday1m,
    Intraday5m,
    Intraday15m,
    Hourly1h,
    Daily1d,
    Weekly1w,
    Monthly1mo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityUnit {
    Vnd,
    ThousandVnd,
    MillionVnd,
    BillionVnd,
    Percent,
    Ratio,
    Multiple,
    Shares,
    Points,
    QmvDspiIndex,
    RawUnprovedCode,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Unit,
    Thousand,
    Million,
    Billion,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    TrailingFourQuarters,
    LatestQuarterAnnualized,
    YearToDate,
    AdjustedClose,
    UnadjustedClose,
    ProviderRaw,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoricalMode {
    PointInTime,
    Retrospective,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandleBarV1 {
    pub bar_date: NaiveDate,
    pub timestamp_utc: Option<DateTime<Utc>>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub value_traded: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandleSeriesV1 {
    pub schema_version: String,
    pub symbol: String,
    pub timeframe: TimeframeV1,
    pub price_unit: QuantityUnit,
    pub price_scale: Scale,
    pub basis: Basis,
    pub historical_mode: HistoricalMode,
    pub source_provider: String,
    pub source_sha256: String,
    pub as_of: NaiveDate,
    pub bars: Vec<CandleBarV1>,
}

impl CandleSeriesV1 {
    pub fn validate(&self) -> BridgeResult<()> {
        if self.bars.is_empty() {
            return Err(BridgeError::Contract(format!(
                "sumi_series_empty: {}",
                self.symbol
            )));
        }
        if self.price_unit == QuantityUnit::Unknown {
            return Err(BridgeError::Contract(format!(
                "sumi_unknown_price_unit_rejected: {}",
                self.symbol
            )));
        }
        for pair in self.bars.windows(2) {
            let prev_key = (pair[0].bar_date, pair[0].timestamp_utc);
            let next_key = (pair[1].bar_date, pair[1].timestamp_utc);
            if prev_key >= next_key {
                return Err(BridgeError::Contract(format!(
                    "sumi_bars_unsorted_or_duplicate_timestamp: {}",
                    self.symbol
                )));
            }
        }
        for bar in &self.bars {
            if !bar.open.is_finite()
                || !bar.high.is_finite()
                || !bar.low.is_finite()
                || !bar.close.is_finite()
                || !bar.volume.is_finite()
                || bar.high < bar.low
                || bar.open < bar.low
                || bar.open > bar.high
                || bar.close < bar.low
                || bar.close > bar.high
                || bar.volume < 0.0
            {
                return Err(BridgeError::Contract(format!(
                    "sumi_ohlc_envelope_invalid: {} at {}",
                    self.symbol, bar.bar_date
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorColumnV1 {
    pub name: String,
    pub formula_version: String,
    pub unit: QuantityUnit,
    pub values: Vec<Option<f64>>,
    /// True ONLY for proven QMV proprietary fields; Sumi-computed proxies MUST set `false`.
    pub is_provider_proprietary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorFrameV1 {
    pub schema_version: String,
    pub symbol: String,
    pub timeframe: TimeframeV1,
    pub source_series_sha256: String,
    pub dates: Vec<NaiveDate>,
    pub indicators: BTreeMap<String, IndicatorColumnV1>,
}

impl IndicatorFrameV1 {
    pub fn validate(&self) -> BridgeResult<()> {
        let expected = self.dates.len();
        for (name, col) in &self.indicators {
            if col.values.len() != expected {
                return Err(BridgeError::Contract(format!(
                    "sumi_indicator_length_mismatch: {name} has {}, expected {expected}",
                    col.values.len()
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestRequestV1 {
    pub schema_version: String,
    pub request_id: String,
    pub strategy_id: String,
    pub strategy_version: String,
    pub symbols: Vec<String>,
    pub timeframe: TimeframeV1,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub initial_capital_vnd: f64,
    pub fee_bps: f64,
    pub sell_tax_bps: f64,
    pub slippage_bps: f64,
    pub parameters: BTreeMap<String, Value>,
    pub require_point_in_time: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestSummaryV1 {
    pub schema_version: String,
    pub request_id: String,
    pub strategy_id: String,
    pub strategy_version: String,
    pub input_series_hashes: Vec<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_return_pct: f64,
    pub cagr_pct: Option<f64>,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: Option<f64>,
    pub win_rate_pct: Option<f64>,
    pub profit_factor: Option<f64>,
    pub trade_count: u32,
    pub exposure_pct: f64,
    pub advisory_only: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalEvidenceBundleV1 {
    pub schema_version: String,
    pub symbol: String,
    pub as_of: NaiveDate,
    pub candle_series: CandleSeriesV1,
    pub indicator_frame: Option<IndicatorFrameV1>,
    pub backtest_summaries: Vec<BacktestSummaryV1>,
    pub raw_qmv_signals: BTreeMap<String, Value>,
    pub gaps: Vec<String>,
}

impl TechnicalEvidenceBundleV1 {
    pub fn validate(&self) -> BridgeResult<()> {
        self.candle_series.validate()?;
        if let Some(frame) = &self.indicator_frame {
            frame.validate()?;
        }
        Ok(())
    }
}

// ============================================================================
// Mandatory `kVND <-> VND` Scale Guard (Decision #3)
// ============================================================================

/// Extracts the bare uppercase ticker symbol (strips `HOSE:`, `HNX:`, `UPCOM:` exchange prefix).
#[must_use]
pub fn strip_exchange_prefix(ticker_or_symbol: &str) -> String {
    let trimmed = ticker_or_symbol.trim().to_ascii_uppercase();
    trimmed
        .rsplit_once(':')
        .map_or(trimmed.clone(), |(_, sym)| sym.to_owned())
}

/// Returns `true` when `symbol` is a Vietnam market index or VN30 index futures contract whose
/// canonical price unit is **index points (`1.0x`)** rather than `VND` (`1000x` from `kVND`).
#[must_use]
pub fn is_index_or_futures_symbol(ticker_or_symbol: &str) -> bool {
    let sym = strip_exchange_prefix(ticker_or_symbol);
    matches!(
        sym.as_str(),
        "VNINDEX"
            | "VN30"
            | "VN100"
            | "VNALLSHARE"
            | "VNXALL"
            | "VNFINLEAD"
            | "VNDIAMOND"
            | "HNXINDEX"
            | "HNX30"
            | "UPCOMINDEX"
            | "UPINDEX"
    ) || sym.starts_with("VN30F")
        || sym == "VN301!"
        || sym == "VN302!"
        || sym.starts_with("VN100F")
}

/// Returns the multiplicative factor needed to convert `unit` into Sumi's canonical chart price
/// space (`Raw VND` for VN equities, `Points (1.0x)` for indices & VN30F futures).
#[must_use]
pub fn scale_multiplier_to_sumi_chart(unit: QuantityUnit, symbol: &str) -> f64 {
    if is_index_or_futures_symbol(symbol) {
        return 1.0;
    }
    match unit {
        QuantityUnit::ThousandVnd => 1_000.0,
        QuantityUnit::MillionVnd => 1_000_000.0,
        QuantityUnit::BillionVnd => 1_000_000_000.0,
        QuantityUnit::Vnd | QuantityUnit::Points => 1.0,
        _ => 1.0,
    }
}

/// Converts a QMV / DNSE price (with explicit `QuantityUnit`) into Sumi's canonical chart scale
/// (`Raw VND` for VN equities, `Points` for indices & VN30F futures).
///
/// Never applies blind magnitude guessing (`abs(v) < 2`); relies strictly on explicit unit +
/// instrument class (`F03` guard).
#[must_use]
pub fn normalize_price_to_sumi_vnd(value: f64, unit: QuantityUnit, symbol: &str) -> f64 {
    if !value.is_finite() {
        return value;
    }
    value * scale_multiplier_to_sumi_chart(unit, symbol)
}

/// Converts a Sumi chart price (`Raw VND` for equities, `Points` for indices/futures) into
/// `kVND` (`ThousandVnd`) when emitting QMV/Yumeko-compatible equity levels (`132,500 -> 132.5`,
/// while `VNINDEX 1,278.45` and `VN30F1! 1,348.5` remain `1.0x`).
#[must_use]
pub fn normalize_sumi_vnd_to_kvnd(value: f64, symbol: &str) -> f64 {
    if !value.is_finite() || is_index_or_futures_symbol(symbol) {
        return value;
    }
    (value / 1_000.0 * 10_000.0).round() / 10_000.0
}

/// Infers `QuantityUnit` from a DNSE/QMV provider `unit` string without magnitude heuristics.
#[must_use]
pub fn parse_provider_price_unit(raw_unit: Option<&str>, symbol: &str) -> (QuantityUnit, Scale) {
    if is_index_or_futures_symbol(symbol) {
        return (QuantityUnit::Points, Scale::Unit);
    }
    match raw_unit
        .map(str::trim)
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "vnd" | "dong" | "raw_vnd" => (QuantityUnit::Vnd, Scale::Unit),
        "thousand_vnd" | "kvnd" | "1000_vnd" | "thousand" | "" => {
            // QMV & DNSE VN equity price feeds emit prices in thousand VND (e.g. `132.5` = `132,500 VND`).
            (QuantityUnit::ThousandVnd, Scale::Thousand)
        }
        "million_vnd" => (QuantityUnit::MillionVnd, Scale::Million),
        "points" | "point" | "index_points" => (QuantityUnit::Points, Scale::Unit),
        _ => (QuantityUnit::ThousandVnd, Scale::Thousand),
    }
}

/// Converts a validated [`CandleSeriesV1`] into Sumi [`OhlcvBar`] rows normalized to Sumi's chart
/// price space (`Raw VND` for equities, `Points` for indices/futures).
pub fn candle_series_v1_to_sumi_bars(series: &CandleSeriesV1) -> BridgeResult<Vec<OhlcvBar>> {
    series.validate()?;
    let mult = scale_multiplier_to_sumi_chart(series.price_unit, &series.symbol);
    let interval = match series.timeframe {
        TimeframeV1::Intraday1m => Interval::M1,
        TimeframeV1::Intraday5m => Interval::M5,
        TimeframeV1::Intraday15m => Interval::M15,
        TimeframeV1::Hourly1h => Interval::H1,
        TimeframeV1::Daily1d => Interval::D1,
        TimeframeV1::Weekly1w => Interval::W1,
        TimeframeV1::Monthly1mo => Interval::MN1,
    };

    let mut out = Vec::with_capacity(series.bars.len());
    for b in &series.bars {
        let ts_ms = match b.timestamp_utc {
            Some(dt) => canonicalize_bar_timestamp_ms(dt.timestamp_millis(), interval),
            None => {
                let dt = b
                    .bar_date
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| BridgeError::Contract(format!("invalid date {}", b.bar_date)))?
                    .and_utc();
                canonicalize_bar_timestamp_ms(dt.timestamp_millis(), interval)
            }
        };
        let o = b.open * mult;
        let h = b.high * mult;
        let l = b.low * mult;
        let c = b.close * mult;
        // Sanity Scale Guard: reject obvious 1000x corruption where a VN equity ends up < 100 VND or > 50,000,000 VND.
        if !is_index_or_futures_symbol(&series.symbol) && c > 0.0 && (c < 100.0 || c > 50_000_000.0)
        {
            return Err(BridgeError::ScaleGuard(format!(
                "Normalized equity close {} for {} ({:?}, mult={}) outside sane VND bounds [100, 50_000_000]",
                c, series.symbol, series.price_unit, mult
            )));
        }
        let mut bar = OhlcvBar::new(ts_ms, o, h, l, c, b.volume);
        if let Some(vt) = b.value_traded.filter(|v| v.is_finite() && *v >= 0.0) {
            bar.turnover = vt;
        }
        out.push(bar);
    }
    Ok(out)
}

/// Canonical sorted-key JSON SHA-256 hex (matches Yumeko's `yumeko_contracts::canonical_sha256`).
pub fn canonical_sha256_value(value: &Value) -> String {
    let canonical = canonicalize_json(value);
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

pub fn sha256_str_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

fn canonicalize_json(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonicalize_json(v));
            }
            let mut out = serde_json::Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_json).collect()),
        other => other.clone(),
    }
}

// ============================================================================
// KT01 Gate Verdict V1 (Sumi -> Yumeko Step 6 `stock_confirmation_gate`)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Kt01GateVerdict {
    Pass,
    Watch,
    WaitPullback,
    Reject,
}

impl Kt01GateVerdict {
    #[must_use]
    pub const fn label_vi(self) -> &'static str {
        match self {
            Self::Pass => "ĐẠT CỔNG KT01 (Đủ điều kiện giải ngân)",
            Self::Watch => "THEO DÕI (Chờ xác nhận thêm dòng tiền/nến)",
            Self::WaitPullback => "CHỜ ĐIỀU CHỈNH (Quá mua / cách xa hỗ trợ)",
            Self::Reject => "LOẠI / KHÔNG ĐẠT KT01 (Gãy cấu trúc hoặc rủi ro cao)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtfAlignmentSummary {
    pub weekly_trend: String,
    pub daily_regime: String,
    pub daily_health_score: Option<f64>,
    pub hourly_timing: Option<String>,
    pub ma20_vnd: Option<f64>,
    pub ma50_vnd: Option<f64>,
    pub ma200_vnd: Option<f64>,
    pub rsi_14: Option<f64>,
    pub rvol_20: Option<f64>,
    pub active_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kt01ExecutionLevels {
    pub last_close_vnd: f64,
    pub last_close_kvnd: f64,
    pub entry_zone_low_vnd: f64,
    pub entry_zone_high_vnd: f64,
    pub stop_loss_vnd: f64,
    pub stop_loss_kvnd: f64,
    pub take_profit_1_vnd: f64,
    pub take_profit_1_kvnd: f64,
    pub take_profit_2_vnd: f64,
    pub take_profit_2_kvnd: f64,
    pub risk_pct: f64,
    pub reward_tp1_pct: f64,
    pub rr_ratio_tp1: f64,
    pub levels_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kt01AfcSummary {
    pub horizon_bars: usize,
    pub win_rate_pct: f64,
    pub expected_p50_pct: f64,
    pub p10_pct: f64,
    pub p90_pct: f64,
    pub prob_tail_drop_6_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KT01GateVerdictV1 {
    pub schema_version: String,
    pub verdict_id: String,
    pub symbol: String,
    pub ticker: String,
    pub as_of: String,
    pub generated_at_utc: String,
    /// Hard Guard: Sumi emits strictly advisory research evidence, NEVER an autonomous execution order.
    pub advisory_only: bool,
    /// `"qmv_real"` when Yumeko Blackbox rows are present, `"local_proxy"` otherwise.
    pub flow_provenance: String,
    pub verdict: Kt01GateVerdict,
    pub verdict_label_vi: String,
    pub confidence_score: f64,
    pub mtf_alignment: MtfAlignmentSummary,
    pub execution_levels: Kt01ExecutionLevels,
    pub afc_summary: Option<Kt01AfcSummary>,
    pub qmv_blackbox_snapshot: Option<Value>,
    pub dm01_is_real_holding: bool,
    pub dm01_is_yumeko_watchlist: bool,
    pub invalidation_triggers: Vec<String>,
    pub caveats: Vec<String>,
    pub evidence_bundle: Option<TechnicalEvidenceBundleV1>,
    pub verdict_sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_guard_converts_vn_equities_kvnd_to_vnd_and_keeps_indices_and_futures_at_1x() {
        // Equities: 132.5 kVND -> 132,500 VND and inverse 132,500 -> 132.5
        assert!(
            (normalize_price_to_sumi_vnd(132.5, QuantityUnit::ThousandVnd, "FPT") - 132_500.0)
                .abs()
                < 1e-9
        );
        assert!(
            (normalize_price_to_sumi_vnd(132_500.0, QuantityUnit::Vnd, "HOSE:FPT") - 132_500.0)
                .abs()
                < 1e-9
        );
        assert!((normalize_sumi_vnd_to_kvnd(132_500.0, "HOSE:FPT") - 132.5).abs() < 1e-9);

        // Market Indices: strictly 1.0x (never 1,278,450!)
        assert!(
            (normalize_price_to_sumi_vnd(1_278.45, QuantityUnit::ThousandVnd, "HOSE:VNINDEX")
                - 1_278.45)
                .abs()
                < 1e-9
        );
        assert!((normalize_sumi_vnd_to_kvnd(1_278.45, "VNINDEX") - 1_278.45).abs() < 1e-9);

        // Futures: strictly 1.0x (never 1,348,500!)
        assert!(
            (normalize_price_to_sumi_vnd(1_348.5, QuantityUnit::ThousandVnd, "HNX:VN30F1!")
                - 1_348.5)
                .abs()
                < 1e-9
        );
        assert!((normalize_sumi_vnd_to_kvnd(1_348.5, "HNX:VN30F1!") - 1_348.5).abs() < 1e-9);
    }

    #[test]
    fn scale_guard_rejects_corrupted_1000x_equity_drift() {
        let d1 = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
        let bad_unit_series = CandleSeriesV1 {
            schema_version: SUMI_CONTRACT_VERSION_V1.to_owned(),
            symbol: "FPT".to_owned(),
            timeframe: TimeframeV1::Daily1d,
            // Deliberately mis-tagged as `Vnd` while carrying `kVND` value `92.5`:
            price_unit: QuantityUnit::Vnd,
            price_scale: Scale::Unit,
            basis: Basis::AdjustedClose,
            historical_mode: HistoricalMode::PointInTime,
            source_provider: "DNSE".to_owned(),
            source_sha256: "a".repeat(64),
            as_of: d1,
            bars: vec![CandleBarV1 {
                bar_date: d1,
                timestamp_utc: None,
                open: 92.0,
                high: 93.0,
                low: 91.5,
                close: 92.5,
                volume: 1_500_000.0,
                value_traded: None,
            }],
        };
        assert!(
            matches!(
                candle_series_v1_to_sumi_bars(&bad_unit_series),
                Err(BridgeError::ScaleGuard(_))
            ),
            "Scale Guard must catch mis-tagged kVND equity prices (< 100 VND)"
        );
    }

    #[test]
    fn golden_json_matches_yumeko_sumi_contract_v1_wire_format() {
        // Exact JSON shape emitted by `yumeko-sumi-contract::TechnicalEvidenceBundleV1`
        let yumeko_json = r#"{
            "schema_version": "yumeko_sumi_contract_v1",
            "symbol": "FPT",
            "as_of": "2026-10-12",
            "candle_series": {
                "schema_version": "yumeko_sumi_contract_v1",
                "symbol": "FPT",
                "timeframe": "daily1d",
                "price_unit": "thousand_vnd",
                "price_scale": "thousand",
                "basis": "adjusted_close",
                "historical_mode": "point_in_time",
                "source_provider": "QMV Fintech",
                "source_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "as_of": "2026-10-12",
                "bars": [
                    {
                        "bar_date": "2026-10-09",
                        "timestamp_utc": null,
                        "open": 130.0,
                        "high": 133.5,
                        "low": 129.5,
                        "close": 132.0,
                        "volume": 2500000.0,
                        "value_traded": null
                    }
                ]
            },
            "indicator_frame": {
                "schema_version": "yumeko_sumi_contract_v1",
                "symbol": "FPT",
                "timeframe": "daily1d",
                "source_series_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "dates": ["2026-10-09"],
                "indicators": {
                    "sumi_sma_20": {
                        "name": "SMA(20) [Sumi Deterministic]",
                        "formula_version": "sumi.sma.v1(window=20)",
                        "unit": "thousand_vnd",
                        "values": [131.5],
                        "is_provider_proprietary": false
                    }
                }
            },
            "backtest_summaries": [],
            "raw_qmv_signals": {},
            "gaps": []
        }"#;

        let parsed: TechnicalEvidenceBundleV1 =
            serde_json::from_str(yumeko_json).expect("wire-identical deserialization");
        parsed.validate().expect("bundle validates");
        let bars = candle_series_v1_to_sumi_bars(&parsed.candle_series).expect("scale to VND");
        assert_eq!(bars.len(), 1);
        assert!((bars[0].close - 132_000.0).abs() < 1e-6);
    }
}
