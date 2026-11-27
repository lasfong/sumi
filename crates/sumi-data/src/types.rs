use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sumi_core::data::{DataSourcePreset, Timeframe, format_utc_date, format_utc_datetime_hm};
use sumi_core::journal::{
    ExchangeVenue, HNX_LISTED_SYMBOLS, UPCOM_LISTED_SYMBOLS, VnPriceUnit, compute_daily_price_band,
};
use sumi_core::model::Candle;

use crate::error::{DataError, Result};

/// Supported OHLCV chart intervals across `sumi-data`, aligned with TradingView & KLineChart v10.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum Interval {
    #[serde(rename = "1m")]
    M1,
    #[serde(rename = "5m")]
    M5,
    #[serde(rename = "15m")]
    M15,
    #[serde(rename = "30m")]
    M30,
    #[serde(rename = "1h")]
    H1,
    #[serde(rename = "4h")]
    H4,
    #[default]
    #[serde(rename = "1D")]
    D1,
    #[serde(rename = "1W")]
    W1,
    #[serde(rename = "1M")]
    MN1,
}

/// KLineChart v10 `Period` object (`{ span: number, type: 'minute' | 'hour' | 'day' | 'week' | 'month' }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KLinePeriod {
    pub span: u32,
    #[serde(rename = "type")]
    pub kind: &'static str,
}

impl Interval {
    /// Ordered list of all 9 intervals required by Sumi v2.0.
    pub const ALL: [Self; 9] = [
        Self::M1,
        Self::M5,
        Self::M15,
        Self::M30,
        Self::H1,
        Self::H4,
        Self::D1,
        Self::W1,
        Self::MN1,
    ];

    /// Canonical string representation used as SQLite cache key and CLI label.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1D",
            Self::W1 => "1W",
            Self::MN1 => "1M",
        }
    }

    /// Nominal bar duration in **milliseconds** (`1M` uses nominal 30 days for gap-fill estimates).
    #[inline]
    #[must_use]
    pub const fn duration_ms(self) -> i64 {
        match self {
            Self::M1 => 60_000,
            Self::M5 => 300_000,
            Self::M15 => 900_000,
            Self::M30 => 1_800_000,
            Self::H1 => 3_600_000,
            Self::H4 => 14_400_000,
            Self::D1 => 86_400_000,
            Self::W1 => 604_800_000,
            Self::MN1 => 2_592_000_000,
        }
    }

    /// Returns `true` for sub-daily intraday timeframes (`1m`, `5m`, `15m`, `30m`, `1h`, `4h`).
    #[inline]
    #[must_use]
    pub const fn is_intraday(self) -> bool {
        matches!(
            self,
            Self::M1 | Self::M5 | Self::M15 | Self::M30 | Self::H1 | Self::H4
        )
    }

    /// Maps directly to `tradingview::Interval`.
    #[inline]
    #[must_use]
    pub const fn to_tv_interval(self) -> tradingview::Interval {
        match self {
            Self::M1 => tradingview::Interval::OneMinute,
            Self::M5 => tradingview::Interval::FiveMinutes,
            Self::M15 => tradingview::Interval::FifteenMinutes,
            Self::M30 => tradingview::Interval::ThirtyMinutes,
            Self::H1 => tradingview::Interval::OneHour,
            Self::H4 => tradingview::Interval::FourHours,
            Self::D1 => tradingview::Interval::OneDay,
            Self::W1 => tradingview::Interval::OneWeek,
            Self::MN1 => tradingview::Interval::OneMonth,
        }
    }

    /// Maps directly to KLineChart v10 `Period` (`chart.setPeriod(...)`).
    #[inline]
    #[must_use]
    pub const fn to_klinechart_period(self) -> KLinePeriod {
        match self {
            Self::M1 => KLinePeriod {
                span: 1,
                kind: "minute",
            },
            Self::M5 => KLinePeriod {
                span: 5,
                kind: "minute",
            },
            Self::M15 => KLinePeriod {
                span: 15,
                kind: "minute",
            },
            Self::M30 => KLinePeriod {
                span: 30,
                kind: "minute",
            },
            Self::H1 => KLinePeriod {
                span: 1,
                kind: "hour",
            },
            Self::H4 => KLinePeriod {
                span: 4,
                kind: "hour",
            },
            Self::D1 => KLinePeriod {
                span: 1,
                kind: "day",
            },
            Self::W1 => KLinePeriod {
                span: 1,
                kind: "week",
            },
            Self::MN1 => KLinePeriod {
                span: 1,
                kind: "month",
            },
        }
    }

    /// Maps to `sumi_core::data::Timeframe` across all 9 supported intervals (`1m..1M`).
    #[inline]
    #[must_use]
    pub const fn to_core_timeframe(self) -> Option<Timeframe> {
        match self {
            Self::M1 => Some(Timeframe::M1),
            Self::M5 => Some(Timeframe::M5),
            Self::M15 => Some(Timeframe::M15),
            Self::M30 => Some(Timeframe::M30),
            Self::H1 => Some(Timeframe::H1),
            Self::H4 => Some(Timeframe::H4),
            Self::D1 => Some(Timeframe::D1),
            Self::W1 => Some(Timeframe::W1),
            Self::MN1 => Some(Timeframe::MN1),
        }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Interval {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        // Distinguish uppercase "1M" (1 month) from lowercase "1m" (1 minute) first.
        match trimmed {
            "1M" | "M" | "1mo" | "1MO" | "1month" | "month" | "monthly" => return Ok(Self::MN1),
            "1m" | "1" | "1min" | "m1" | "M1" => return Ok(Self::M1),
            _ => {}
        }
        match trimmed.to_ascii_lowercase().as_str() {
            "5m" | "5" | "5min" | "m5" => Ok(Self::M5),
            "15m" | "15" | "15min" | "m15" => Ok(Self::M15),
            "30m" | "30" | "30min" | "m30" => Ok(Self::M30),
            "1h" | "60" | "60m" | "h1" | "1hour" => Ok(Self::H1),
            "4h" | "240" | "240m" | "h4" | "4hour" => Ok(Self::H4),
            "1d" | "d" | "d1" | "day" | "daily" => Ok(Self::D1),
            "1w" | "w" | "w1" | "week" | "weekly" => Ok(Self::W1),
            "mn1" => Ok(Self::MN1),
            _ => Err(DataError::InvalidInterval(s.to_owned())),
        }
    }
}

/// Price adjustment mode for corporate actions (splits / dividends / unadjusted raw prices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Adjustment {
    /// Adjust for stock splits and stock dividends (TradingView server default).
    #[default]
    Splits,
    /// Adjust for splits and cash dividends.
    Dividends,
    /// Raw historical prices without corporate action adjustment.
    Unadjusted,
}

impl Adjustment {
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Splits => "splits",
            Self::Dividends => "dividends",
            Self::Unadjusted => "unadjusted",
        }
    }

    #[inline]
    #[must_use]
    pub const fn to_tv_adjustment(self) -> tradingview::MarketAdjustment {
        match self {
            Self::Splits => tradingview::MarketAdjustment::Splits,
            Self::Dividends => tradingview::MarketAdjustment::Dividends,
            Self::Unadjusted => tradingview::MarketAdjustment::Unadjusted,
        }
    }
}

impl fmt::Display for Adjustment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Adjustment {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "splits" | "split" | "adj" | "adjusted" => Ok(Self::Splits),
            "dividends" | "dividend" | "div" | "total_return" => Ok(Self::Dividends),
            "unadjusted" | "none" | "raw" => Ok(Self::Unadjusted),
            _ => Err(DataError::InvalidAdjustment(s.to_owned())),
        }
    }
}

/// Canonical `(exchange, symbol)` instrument identifier (e.g. `HOSE:FPT`, `HNX:VN30F1!`, `FX_IDC:USDVND`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolKey {
    pub exchange: String,
    pub symbol: String,
}

impl SymbolKey {
    /// Creates a validated canonical uppercase [`SymbolKey`].
    pub fn new(exchange: impl AsRef<str>, symbol: impl AsRef<str>) -> Result<Self> {
        let ex = normalize_exchange_alias(exchange.as_ref());
        let sym = symbol.as_ref().trim().to_ascii_uppercase();
        if !is_valid_identifier_token(&ex) || !is_valid_identifier_token(&sym) {
            return Err(DataError::InvalidSymbol(format!(
                "{}:{}",
                exchange.as_ref(),
                symbol.as_ref()
            )));
        }
        Ok(Self {
            exchange: ex,
            symbol: sym,
        })
    }

    /// Parses `"EXCHANGE:SYMBOL"` (e.g. `"HOSE:FPT"`, `"HNX:VN30F1!"`, `"FX_IDC:USDVND"`).
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        let Some((ex, sym)) = trimmed.split_once(':') else {
            return Err(DataError::InvalidSymbol(raw.to_owned()));
        };
        if sym.contains(':') {
            return Err(DataError::InvalidSymbol(raw.to_owned()));
        }
        Self::new(ex, sym)
    }

    /// Returns canonical `"EXCHANGE:SYMBOL"` ticker string.
    #[must_use]
    pub fn ticker(&self) -> String {
        format!("{}:{}", self.exchange, self.symbol)
    }
}

impl fmt::Display for SymbolKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.exchange, self.symbol)
    }
}

impl FromStr for SymbolKey {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

/// Normalizes common exchange aliases (`HSX` -> `HOSE`, `UPCOM` -> `UPCOM`).
#[must_use]
pub fn normalize_exchange_alias(raw: &str) -> String {
    let upper = raw.trim().to_ascii_uppercase();
    match upper.as_str() {
        "HSX" | "HOSE" => "HOSE".to_owned(),
        _ => upper,
    }
}

fn is_valid_identifier_token(token: &str) -> bool {
    !token.is_empty()
        && token.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'.' | b'!' | b'_' | b'-' | b'&' | b'/')
        })
}

/// Composite primary storage key for OHLCV bar series: `(exchange, symbol, interval, adjustment)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SeriesKey {
    pub exchange: String,
    pub symbol: String,
    pub interval: Interval,
    pub adjustment: Adjustment,
}

impl SeriesKey {
    #[must_use]
    pub fn new(symbol: &SymbolKey, interval: Interval, adjustment: Adjustment) -> Self {
        Self {
            exchange: symbol.exchange.clone(),
            symbol: symbol.symbol.clone(),
            interval,
            adjustment,
        }
    }

    #[must_use]
    pub fn symbol_key(&self) -> SymbolKey {
        SymbolKey {
            exchange: self.exchange.clone(),
            symbol: self.symbol.clone(),
        }
    }

    #[must_use]
    pub fn ticker(&self) -> String {
        format!("{}:{}", self.exchange, self.symbol)
    }
}

impl fmt::Display for SeriesKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}@{}({})",
            self.exchange, self.symbol, self.interval, self.adjustment
        )
    }
}

/// KLineChart v10-compatible OHLCV bar with **Unix millisecond (`i64`)** timestamp and `turnover`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OhlcvBar {
    /// Bar open timestamp in **Unix milliseconds (UTC)**.
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    /// Traded value (`close * volume`), populated automatically when not provided upstream.
    pub turnover: f64,
}

impl OhlcvBar {
    /// Creates an [`OhlcvBar`] and automatically calculates `turnover = close * volume`.
    #[inline]
    #[must_use]
    pub fn new(timestamp_ms: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        let safe_vol = if volume.is_finite() && volume >= 0.0 {
            volume
        } else {
            0.0
        };
        let turnover = if close.is_finite() {
            (close * safe_vol).max(0.0)
        } else {
            0.0
        };
        Self {
            timestamp: timestamp_ms,
            open,
            high,
            low,
            close,
            volume: safe_vol,
            turnover,
        }
    }

    /// Verifies finite OHLCV structure and non-negative volume/turnover.
    #[inline]
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.open.is_finite()
            && self.high.is_finite()
            && self.low.is_finite()
            && self.close.is_finite()
            && self.volume.is_finite()
            && self.turnover.is_finite()
            && self.volume >= 0.0
            && self.low <= self.open.min(self.close)
            && self.open.max(self.close) <= self.high
    }

    /// Converts to a `sumi-core` [`Candle`] (Unix seconds UTC + formatted date label).
    #[must_use]
    pub fn to_core_candle(&self, interval: Interval) -> Candle {
        let ts_secs = self.timestamp.div_euclid(1_000);
        let date_label = if interval.is_intraday() {
            format_utc_datetime_hm(ts_secs)
        } else {
            format_utc_date(ts_secs)
        };
        Candle {
            timestamp: ts_secs,
            date_label,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }

    /// Converts from a `sumi-core` [`Candle`] (seconds -> milliseconds + computed `turnover`).
    #[must_use]
    pub fn from_core_candle(candle: &Candle) -> Self {
        Self::new(
            candle.timestamp.saturating_mul(1_000),
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume,
        )
    }
}

/// Canonicalizes a bar timestamp for storage/deduplication consistency across vendors.
///
/// For `Interval::D1`, snaps exchange-open timestamps (e.g. TradingView `02:00 UTC` on HOSE
/// or `14:30 UTC` on NASDAQ) down to UTC midnight (`00:00:00 UTC`) so CafeF daily history and
/// TradingView gap-fill bars share identical primary-key timestamps with zero duplicate candles.
#[inline]
#[must_use]
pub const fn canonicalize_bar_timestamp_ms(timestamp_ms: i64, interval: Interval) -> i64 {
    match interval {
        Interval::D1 => timestamp_ms.div_euclid(86_400_000) * 86_400_000,
        _ => timestamp_ms,
    }
}

/// Normalizes raw bars into KLineChart v10 invariant order:
/// - filters non-finite OHLC rows and clamps minor inverted wicks safely,
/// - ensures `volume >= 0` and computes `turnover = close * volume` when missing,
/// - sorts strictly ascending by `timestamp` (Unix ms),
/// - deduplicates colliding timestamps keeping the **latest** update.
#[must_use]
pub fn normalize_bars(bars: Vec<OhlcvBar>) -> Vec<OhlcvBar> {
    let mut cleaned: Vec<(usize, OhlcvBar)> = bars
        .into_iter()
        .enumerate()
        .filter_map(|(idx, mut bar)| {
            if !bar.open.is_finite()
                || !bar.high.is_finite()
                || !bar.low.is_finite()
                || !bar.close.is_finite()
            {
                return None;
            }
            let body_max = bar.open.max(bar.close);
            let body_min = bar.open.min(bar.close);
            if bar.high < body_max {
                bar.high = body_max;
            }
            if bar.low > body_min {
                bar.low = body_min;
            }
            if !bar.volume.is_finite() || bar.volume < 0.0 {
                bar.volume = 0.0;
            }
            if !bar.turnover.is_finite() || bar.turnover < 0.0 {
                bar.turnover = (bar.close * bar.volume).max(0.0);
            }
            Some((idx, bar))
        })
        .collect();

    // Stable order on input index so deduplication keeps the latest update for the same timestamp.
    cleaned.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp).then(a.0.cmp(&b.0)));

    let mut deduped: Vec<OhlcvBar> = Vec::with_capacity(cleaned.len());
    for (_, bar) in cleaned {
        if let Some(last) = deduped.last_mut()
            && last.timestamp == bar.timestamp
        {
            *last = bar;
            continue;
        }
        deduped.push(bar);
    }
    deduped
}

/// One bracket of a price-dependent tick table (`tick` applies while `price < upper_bound`,
/// or for all remaining prices when `upper_bound` is `None`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VariableTickTier {
    pub tick: f64,
    pub upper_bound: Option<f64>,
}

/// Parsed price-dependent tick ladder (e.g. HOSE `"10 10000 50 50000 100"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableTickTable {
    pub raw: String,
    pub tiers: Vec<VariableTickTier>,
}

impl VariableTickTable {
    /// Parses TradingView's space-separated `variable_tick_size` specification
    /// (`"t0 b0 t1 b1 ... tn"` where bounds are strictly increasing exclusive upper limits).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let values: Vec<f64> = trimmed
            .split_whitespace()
            .map(|token| {
                token
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v > 0.0)
            })
            .collect::<Option<Vec<f64>>>()?;

        if values.len().is_multiple_of(2) {
            return None;
        }
        let bounds_increasing = values
            .iter()
            .skip(1)
            .step_by(2)
            .zip(values.iter().skip(3).step_by(2))
            .all(|(lo, hi)| lo < hi);
        if !bounds_increasing {
            return None;
        }

        let mut tiers = Vec::with_capacity(values.len() / 2 + 1);
        for chunk in values.chunks(2) {
            match *chunk {
                [tick, upper] => tiers.push(VariableTickTier {
                    tick,
                    upper_bound: Some(upper),
                }),
                [tick] => tiers.push(VariableTickTier {
                    tick,
                    upper_bound: None,
                }),
                _ => return None,
            }
        }
        Some(Self {
            raw: trimmed.to_owned(),
            tiers,
        })
    }

    /// Statutory HOSE cash equity tick ladder in raw VND (`10 / 50 / 100 VND`).
    #[must_use]
    pub fn hose_equity_default() -> Self {
        Self::parse("10 10000 50 50000 100").expect("valid HOSE tick ladder")
    }

    /// Resolves the tick step applicable at `price`.
    #[must_use]
    pub fn tick_size_at(&self, price: f64) -> Option<f64> {
        if !price.is_finite() || price < 0.0 {
            return None;
        }
        for tier in &self.tiers {
            match tier.upper_bound {
                Some(upper) if price < upper => return Some(tier.tick),
                Some(_) => continue,
                None => return Some(tier.tick),
            }
        }
        self.tiers.last().map(|t| t.tick)
    }
}

/// Statutory daily ceiling/floor band specification for Vietnam securities exchanges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VietnamPriceBand {
    /// Venue label (`"HOSE"`, `"HNX"`, `"UPCOM"`, `"VN30F"`).
    pub venue_label: String,
    /// Symmetric band percentage fraction (`0.07` = ±7%, `0.10` = ±10%, `0.15` = ±15%).
    pub daily_limit_pct: f64,
    /// Human-readable display badge (`"±7%"`, `"±10%"`, `"±15%"`).
    pub label: String,
    /// Underlying `sumi-core` exchange venue for statutory ceiling/floor tick rounding.
    pub venue: ExchangeVenue,
}

impl VietnamPriceBand {
    /// Infers the Vietnam statutory daily fluctuation band from `(exchange, symbol, market_type)`.
    ///
    /// Returns `None` for indices (`VNINDEX`, `HNXINDEX`), US stocks, FX, or crypto.
    #[must_use]
    pub fn infer(exchange: &str, symbol: &str, market_type: &str) -> Option<Self> {
        let ex = normalize_exchange_alias(exchange);
        let sym = symbol.trim().to_ascii_uppercase();
        let mt = market_type.trim().to_ascii_lowercase();

        if mt == "index" || is_vn_index_symbol(&sym) {
            return None;
        }
        if sym.starts_with("VN30F") || (ex == "HNX" && (mt == "futures" || sym.starts_with("VN30")))
        {
            return Some(Self {
                venue_label: "VN30F".to_owned(),
                daily_limit_pct: 0.07,
                label: "±7%".to_owned(),
                venue: ExchangeVenue::Vn30Futures,
            });
        }
        match ex.as_str() {
            "HOSE" => Some(Self {
                venue_label: "HOSE".to_owned(),
                daily_limit_pct: 0.07,
                label: "±7%".to_owned(),
                venue: ExchangeVenue::Hose,
            }),
            "HNX" => Some(Self {
                venue_label: "HNX".to_owned(),
                daily_limit_pct: 0.10,
                label: "±10%".to_owned(),
                venue: ExchangeVenue::Hnx,
            }),
            "UPCOM" => Some(Self {
                venue_label: "UPCOM".to_owned(),
                daily_limit_pct: 0.15,
                label: "±15%".to_owned(),
                venue: ExchangeVenue::Upcom,
            }),
            _ => None,
        }
    }

    /// Computes statutory `(floor_price, ceiling_price)` for a given `reference_price` in raw VND (or pts for VN30F).
    #[must_use]
    pub fn ceiling_floor_for_reference(&self, reference_price: f64) -> Option<(f64, f64)> {
        let unit = if self.venue == ExchangeVenue::Vn30Futures {
            VnPriceUnit::ThousandVnd
        } else {
            VnPriceUnit::RawVnd
        };
        let band = compute_daily_price_band(self.venue, reference_price, unit)?;
        Some((band.floor_price, band.ceiling_price))
    }
}

/// Lightweight search hit returned by [`crate::MarketDataProvider::search_symbols`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSearchResult {
    /// Canonical `"EXCHANGE:SYMBOL"` identifier ready to pass to `resolve_symbol` / `bars`.
    pub ticker: String,
    pub exchange: String,
    pub prefix: String,
    pub symbol: String,
    pub description: String,
    pub market_type: String,
    pub currency_code: String,
    pub country_code: String,
    pub provider_id: String,
    pub type_specs: Vec<String>,
}

/// Complete resolved instrument metadata combining KLineChart v10 `SymbolInfo` fields,
/// TradingView `symbol_resolved` microstructure fields, variable tick ladder, and Vietnam price bands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSymbol {
    /// Canonical `"EXCHANGE:SYMBOL"` ticker string (KLineChart `SymbolInfo.ticker`).
    pub ticker: String,
    pub exchange: String,
    pub listed_exchange: String,
    pub symbol: String,
    pub description: String,
    pub market_type: String,
    pub currency: String,
    pub timezone: String,
    pub session: String,
    /// Decimal places for price formatting (KLineChart `SymbolInfo.pricePrecision`).
    pub price_precision: u32,
    /// Decimal places for volume formatting (KLineChart `SymbolInfo.volumePrecision`).
    pub volume_precision: u32,
    pub pricescale: u64,
    pub minmov: u64,
    pub minmove2: u64,
    /// Base tick increment (`minmov / pricescale`).
    pub tick_size: Option<f64>,
    /// Raw variable tick ladder string (e.g. `"10 10000 50 50000 100"` on HOSE).
    pub variable_tick_size: String,
    /// Parsed variable tick ladder for price-dependent tick queries.
    pub variable_tick_table: Option<VariableTickTable>,
    pub point_value: f64,
    pub delay_secs: u64,
    pub isin: String,
    /// Statutory Vietnam daily price limit (`±7%` HOSE/VN30F, `±10%` HNX, `±15%` UPCOM).
    pub vn_price_band: Option<VietnamPriceBand>,
    /// Mapped `sumi-core` microstructure venue when recognized.
    pub exchange_venue: Option<ExchangeVenue>,
}

impl ResolvedSymbol {
    /// Builds a [`ResolvedSymbol`] from TradingView's `symbol_resolved` payload plus requested [`SymbolKey`].
    #[must_use]
    pub fn from_tv_symbol_info(requested: &SymbolKey, info: &tradingview::SymbolInfo) -> Self {
        let pro_prefix = info
            .id
            .as_str()
            .split_once(':')
            .map(|(ex, _)| normalize_exchange_alias(ex))
            .filter(|s| !s.is_empty());
        let exchange = pro_prefix.unwrap_or_else(|| {
            if !requested.exchange.is_empty() {
                requested.exchange.clone()
            } else {
                normalize_exchange_alias(info.exchange.as_str())
            }
        });
        let listed_exchange = if !info.listed_exchange.is_empty() {
            normalize_exchange_alias(info.listed_exchange.as_str())
        } else if !info.exchange.is_empty() {
            normalize_exchange_alias(info.exchange.as_str())
        } else {
            exchange.clone()
        };
        let symbol = if !info.name.is_empty() {
            info.name.as_str().trim().to_ascii_uppercase()
        } else {
            requested.symbol.clone()
        };
        let ticker = format!("{exchange}:{symbol}");
        let market_type = info.market_type.as_str().to_owned();
        let currency = if !info.currency_code.is_empty() {
            info.currency_code.as_str().to_owned()
        } else {
            info.currency_id.as_str().to_owned()
        };

        let pricescale = info.pricescale;
        let minmov = info.minmov;
        let minmove2 = info.minmove2;
        let tick_size = info.tick_size();

        let vn_price_band = VietnamPriceBand::infer(&exchange, &symbol, &market_type);

        // Parse TradingView's variable_tick_size; fallback to statutory HOSE ladder for HOSE equities.
        let mut variable_tick_size = info.variable_tick_size.as_str().trim().to_owned();
        let variable_tick_table = if !variable_tick_size.is_empty() {
            VariableTickTable::parse(&variable_tick_size)
        } else if matches!(
            vn_price_band.as_ref().map(|b| b.venue),
            Some(ExchangeVenue::Hose)
        ) {
            let hose = VariableTickTable::hose_equity_default();
            variable_tick_size = hose.raw.clone();
            Some(hose)
        } else {
            None
        };

        let price_precision =
            compute_price_precision(pricescale, minmov, variable_tick_table.as_ref());
        let volume_precision = compute_volume_precision(&exchange, &market_type);
        let exchange_venue = infer_exchange_venue(&exchange, &symbol, &market_type);

        Self {
            ticker,
            exchange,
            listed_exchange,
            symbol,
            description: info.description.as_str().to_owned(),
            market_type,
            currency,
            timezone: info.timezone.as_str().to_owned(),
            session: info.session.as_str().to_owned(),
            price_precision,
            volume_precision,
            pricescale,
            minmov,
            minmove2,
            tick_size,
            variable_tick_size,
            variable_tick_table,
            point_value: info.pointvalue,
            delay_secs: info.delay,
            isin: info.isin.as_str().to_owned(),
            vn_price_band,
            exchange_venue,
        }
    }

    /// Synthesizes authoritative Vietnam exchange metadata for locally imported CafeF tickers
    /// so offline `search` and `resolve` work cleanly without network access.
    #[must_use]
    pub fn from_cafef_vn_ticker(exchange: &str, symbol: &str, is_index: bool) -> Self {
        let ex = normalize_exchange_alias(exchange);
        let sym = symbol.trim().to_ascii_uppercase();
        let ticker = format!("{ex}:{sym}");
        let market_type = if is_index {
            "index".to_owned()
        } else if sym.starts_with("VN30F") {
            "futures".to_owned()
        } else {
            "stock".to_owned()
        };
        let vn_price_band = VietnamPriceBand::infer(&ex, &sym, &market_type);
        let (pricescale, minmov, variable_tick_table, variable_tick_size, point_value) = if is_index
        {
            (100, 1, None, String::new(), 1.0)
        } else if sym.starts_with("VN30F") {
            (10, 1, None, String::new(), 100_000.0)
        } else if ex == "HOSE" {
            let tbl = VariableTickTable::hose_equity_default();
            let raw = tbl.raw.clone();
            (1, 10, Some(tbl), raw, 1.0)
        } else {
            // HNX & UPCOM stocks: 100 VND flat tick in raw VND.
            (1, 100, None, String::new(), 1.0)
        };
        let price_precision =
            compute_price_precision(pricescale, minmov, variable_tick_table.as_ref());
        let exchange_venue = infer_exchange_venue(&ex, &sym, &market_type);

        Self {
            ticker,
            exchange: ex.clone(),
            listed_exchange: ex,
            symbol: sym.clone(),
            description: if is_index {
                format!("{sym} Vietnam Market Index (CafeF)")
            } else {
                format!("{sym} Vietnam Equity (CafeF)")
            },
            market_type,
            currency: "VND".to_owned(),
            timezone: "Asia/Ho_Chi_Minh".to_owned(),
            session: "0900-1500".to_owned(),
            price_precision,
            volume_precision: 0,
            pricescale,
            minmov,
            minmove2: 0,
            tick_size: (pricescale > 0 && minmov > 0).then(|| minmov as f64 / pricescale as f64),
            variable_tick_size,
            variable_tick_table,
            point_value,
            delay_secs: 0,
            isin: String::new(),
            vn_price_band,
            exchange_venue,
        }
    }

    /// Returns the applicable tick size at `price`, respecting `variable_tick_table` when present.
    #[must_use]
    pub fn tick_size_at(&self, price: f64) -> Option<f64> {
        if let Some(table) = &self.variable_tick_table {
            return table.tick_size_at(price);
        }
        self.tick_size
    }
}

/// Computes KLineChart `pricePrecision` from TradingView's `(pricescale, minmov)` pair.
#[must_use]
pub fn compute_price_precision(
    pricescale: u64,
    minmov: u64,
    variable_table: Option<&VariableTickTable>,
) -> u32 {
    if let Some(tbl) = variable_table {
        // Use the primary unbounded tier (e.g., `0.01` for US equities `"0.0001 1 0.01"`,
        // `100` for HOSE `"10 10000 50 50000 100"`) rather than `max()` over sub-$1 penny tiers!
        let primary_tier = tbl
            .tiers
            .iter()
            .find(|t| t.upper_bound.is_none())
            .or_else(|| tbl.tiers.last());
        if let Some(tier) = primary_tier {
            let tier_decimals = decimals_for_step(tier.tick);
            if pricescale <= 1 || tier_decimals > 0 {
                return tier_decimals;
            }
            if tbl.tiers.iter().all(|t| decimals_for_step(t.tick) == 0) {
                return 0;
            }
        }
    }
    if pricescale <= 1 {
        return 0;
    }
    let mut scale = pricescale;
    let mut mov = minmov.max(1);
    while scale.is_multiple_of(10) && mov.is_multiple_of(10) && scale >= 10 {
        scale /= 10;
        mov /= 10;
    }
    if scale <= 1 {
        return 0;
    }
    ((scale as f64).log10().ceil() as u32).min(8)
}

fn decimals_for_step(step: f64) -> u32 {
    if !step.is_finite() || step <= 0.0 {
        return 0;
    }
    for d in 0..=8_u32 {
        let scaled = step * 10_f64.powi(d as i32);
        if (scaled - scaled.round()).abs() < 1e-6 {
            return d;
        }
    }
    4
}

fn compute_volume_precision(exchange: &str, market_type: &str) -> u32 {
    let mt = market_type.trim().to_ascii_lowercase();
    let ex = exchange.trim().to_ascii_uppercase();
    if mt == "crypto"
        || matches!(
            ex.as_str(),
            "BINANCE" | "OKX" | "BYBIT" | "COINBASE" | "KRAKEN" | "KUCOIN" | "BITGET" | "HTX"
        )
    {
        4
    } else {
        0
    }
}

/// Infers Vietnam exchange (`HOSE`, `HNX`, `UPCOM`) for a bare ticker from CafeF CSV files.
#[must_use]
pub fn infer_vn_exchange_for_ticker(ticker: &str, is_index: bool) -> &'static str {
    let upper = ticker.trim().to_ascii_uppercase();
    if is_index {
        if upper.starts_with("HNX") {
            return "HNX";
        }
        if upper.starts_with("UPCOM") {
            return "UPCOM";
        }
        return "HOSE";
    }
    if upper.starts_with("VN30F") || HNX_LISTED_SYMBOLS.binary_search(&upper.as_str()).is_ok() {
        return "HNX";
    }
    if UPCOM_LISTED_SYMBOLS.binary_search(&upper.as_str()).is_ok() {
        return "UPCOM";
    }
    match ExchangeVenue::infer_from_symbol_and_preset(&upper, DataSourcePreset::CustomFile) {
        ExchangeVenue::Hnx | ExchangeVenue::Vn30Futures => "HNX",
        ExchangeVenue::Upcom => "UPCOM",
        _ => "HOSE",
    }
}

fn is_vn_index_symbol(sym: &str) -> bool {
    matches!(
        sym,
        "VNINDEX"
            | "VN-INDEX"
            | "VN30"
            | "VN30-INDEX"
            | "VN30INDEX"
            | "VN100"
            | "VNXALL"
            | "VNX50"
            | "VNMID"
            | "VNSML"
            | "VNFINLEAD"
            | "VNDIAMOND"
            | "HNX-INDEX"
            | "HNXINDEX"
            | "UPCOM-INDEX"
            | "UPCOMINDEX"
    )
}

fn infer_exchange_venue(exchange: &str, symbol: &str, market_type: &str) -> Option<ExchangeVenue> {
    let ex = normalize_exchange_alias(exchange);
    let sym = symbol.trim().to_ascii_uppercase();
    let mt = market_type.trim().to_ascii_lowercase();
    if sym.starts_with("VN30F") || (ex == "HNX" && (mt == "futures" || sym.starts_with("VN30"))) {
        return Some(ExchangeVenue::Vn30Futures);
    }
    if is_vn_index_symbol(&sym) {
        return Some(ExchangeVenue::VnIndexBenchmark);
    }
    if mt == "crypto"
        || matches!(
            ex.as_str(),
            "BINANCE" | "OKX" | "BYBIT" | "COINBASE" | "KRAKEN" | "KUCOIN" | "BITGET"
        )
    {
        return Some(ExchangeVenue::CryptoSpotPerp);
    }
    match ex.as_str() {
        "HOSE" => Some(ExchangeVenue::Hose),
        "HNX" => Some(ExchangeVenue::Hnx),
        "UPCOM" => Some(ExchangeVenue::Upcom),
        "NASDAQ" | "NYSE" | "AMEX" | "ARCA" | "BATS" => Some(ExchangeVenue::UsEquities),
        _ => None,
    }
}

/// 3 pagination query modes matching KLineChart v10 `DataLoader.getBars({ type, timestamp })`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BarQueryMode {
    /// Initial chart load: returns the latest `limit` bars.
    Init,
    /// User scrolled to the left edge: returns up to `limit` bars strictly older than `before_ms`.
    Forward { before_ms: i64 },
    /// User scrolled to the right edge: returns up to `limit` bars strictly newer than `after_ms`.
    Backward { after_ms: i64 },
}

/// Bar query request passed to [`crate::MarketDataService::get_bars`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BarsRequest {
    pub key: SeriesKey,
    pub mode: BarQueryMode,
    pub limit: usize,
}

impl BarsRequest {
    #[must_use]
    pub fn init(key: SeriesKey, limit: usize) -> Self {
        Self {
            key,
            mode: BarQueryMode::Init,
            limit: limit.max(1),
        }
    }

    #[must_use]
    pub fn forward(key: SeriesKey, before_ms: i64, limit: usize) -> Self {
        Self {
            key,
            mode: BarQueryMode::Forward { before_ms },
            limit: limit.max(1),
        }
    }

    #[must_use]
    pub fn backward(key: SeriesKey, after_ms: i64, limit: usize) -> Self {
        Self {
            key,
            mode: BarQueryMode::Backward { after_ms },
            limit: limit.max(1),
        }
    }
}

/// Provenance indicator showing how a [`BarsPage`] was satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceOrigin {
    /// Served entirely from local SQLite cache (< 5ms).
    CacheHit,
    /// Served from local SQLite cache after automatically gap-filling/backfilling missing bars online.
    CacheWithGapFill,
    /// First-time remote fetch stored into local SQLite cache.
    RemoteLive,
    /// Served purely from local SQLite cache in offline mode or after automatic network fallback.
    OfflineCache,
}

/// Paginated bar response designed for direct consumption by KLineChart v10
/// `callback(page.bars, { forward: page.has_older, backward: page.has_newer })`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BarsPage {
    pub bars: Vec<OhlcvBar>,
    /// True when strictly older bars are available (maps to KLineChart `more.forward`).
    pub has_older: bool,
    /// True when strictly newer bars are available (maps to KLineChart `more.backward`).
    pub has_newer: bool,
    pub source: DataSourceOrigin,
    pub symbol_info: Option<ResolvedSymbol>,
}

/// Raw historical retrieval output returned by a [`crate::MarketDataProvider`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryFetchOutput {
    pub bars: Vec<OhlcvBar>,
    pub symbol_info: Option<ResolvedSymbol>,
    /// True when upstream reported end of history or its server-side history limit.
    pub reached_start_of_history: bool,
    pub pages_fetched: u32,
}

/// Aggregate result of a rate-limited multi-symbol batch historical fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchBarsResult {
    pub successful: Vec<(SymbolKey, Vec<OhlcvBar>)>,
    pub failed: Vec<(SymbolKey, String)>,
    pub elapsed_ms: u64,
}

/// Single candlestick event emitted over a live streaming subscription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveBarEvent {
    pub series: SeriesKey,
    pub bar: OhlcvBar,
    /// `true` for live intra-bar updates (`du`), `false` for initial/snapshot bar (`timescale_update`).
    pub is_update: bool,
}

/// Merged real-time quote snapshot emitted over a live quote subscription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveQuote {
    pub ticker: String,
    pub exchange: String,
    pub symbol: String,
    /// Timestamp in Unix milliseconds (UTC).
    pub timestamp_ms: i64,
    pub last_price: Option<f64>,
    pub bid: Option<f64>,
    pub bid_size: Option<f64>,
    pub ask: Option<f64>,
    pub ask_size: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub prev_close: Option<f64>,
    pub change: Option<f64>,
    pub change_percent: Option<f64>,
    pub volume: Option<f64>,
    pub currency: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_parsing_and_klinechart_period_mapping() {
        let cases = [
            ("1m", Interval::M1, 1, "minute", 60_000),
            ("5m", Interval::M5, 5, "minute", 300_000),
            ("15m", Interval::M15, 15, "minute", 900_000),
            ("30m", Interval::M30, 30, "minute", 1_800_000),
            ("1h", Interval::H1, 1, "hour", 3_600_000),
            ("4h", Interval::H4, 4, "hour", 14_400_000),
            ("1D", Interval::D1, 1, "day", 86_400_000),
            ("1W", Interval::W1, 1, "week", 604_800_000),
            ("1M", Interval::MN1, 1, "month", 2_592_000_000),
        ];
        for (raw, expected, span, kind, ms) in cases {
            let parsed: Interval = raw.parse().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), raw);
            assert_eq!(parsed.duration_ms(), ms);
            let period = parsed.to_klinechart_period();
            assert_eq!(period.span, span);
            assert_eq!(period.kind, kind);
        }
        assert!("invalid".parse::<Interval>().is_err());
    }

    #[test]
    fn symbol_key_supports_vietnam_derivatives_fx_and_crypto() {
        let fpt = SymbolKey::parse("hsx:fpt").unwrap();
        assert_eq!(fpt.exchange, "HOSE");
        assert_eq!(fpt.symbol, "FPT");
        assert_eq!(fpt.ticker(), "HOSE:FPT");

        let vn30f = SymbolKey::parse("HNX:VN30F1!").unwrap();
        assert_eq!(vn30f.ticker(), "HNX:VN30F1!");

        let fx = SymbolKey::parse("FX_IDC:USDVND").unwrap();
        assert_eq!(fx.ticker(), "FX_IDC:USDVND");

        let perp = SymbolKey::parse("OKX:BTCUSDT.P").unwrap();
        assert_eq!(perp.ticker(), "OKX:BTCUSDT.P");

        assert!(SymbolKey::parse("NO_COLON").is_err());
        assert!(SymbolKey::parse("A:B:C").is_err());
    }

    #[test]
    fn variable_tick_table_and_vietnam_price_bands() {
        let hose = VariableTickTable::parse("10 10000 50 50000 100").unwrap();
        assert_eq!(hose.tick_size_at(9_950.0), Some(10.0));
        assert_eq!(hose.tick_size_at(10_000.0), Some(50.0));
        assert_eq!(hose.tick_size_at(49_950.0), Some(50.0));
        assert_eq!(hose.tick_size_at(50_000.0), Some(100.0));
        assert_eq!(hose.tick_size_at(125_000.0), Some(100.0));
        assert!(VariableTickTable::parse("10 10000").is_none());
        assert!(VariableTickTable::parse("10 50000 50 10000 100").is_none());

        let band_hose = VietnamPriceBand::infer("HOSE", "FPT", "stock").unwrap();
        assert_eq!(band_hose.label, "±7%");
        let (fl, ceil) = band_hose.ceiling_floor_for_reference(100_000.0).unwrap();
        assert_eq!(fl, 93_000.0);
        assert_eq!(ceil, 107_000.0);

        let band_hnx = VietnamPriceBand::infer("HNX", "SHS", "stock").unwrap();
        assert_eq!(band_hnx.label, "±10%");

        let band_upcom = VietnamPriceBand::infer("UPCOM", "ACV", "stock").unwrap();
        assert_eq!(band_upcom.label, "±15%");

        let band_fut = VietnamPriceBand::infer("HNX", "VN30F1!", "futures").unwrap();
        assert_eq!(band_fut.label, "±7%");
        assert_eq!(band_fut.venue, ExchangeVenue::Vn30Futures);

        assert!(VietnamPriceBand::infer("HOSE", "VNINDEX", "index").is_none());
        assert!(VietnamPriceBand::infer("BINANCE", "BTCUSDT", "crypto").is_none());
    }

    #[test]
    fn price_precision_and_normalize_bars_invariants() {
        assert_eq!(compute_price_precision(1, 1, None), 0);
        assert_eq!(compute_price_precision(10, 1, None), 1);
        assert_eq!(compute_price_precision(100, 1, None), 2);
        assert_eq!(compute_price_precision(100, 25, None), 2);
        assert_eq!(compute_price_precision(100, 100, None), 0);
        assert_eq!(compute_price_precision(100_000, 1, None), 5);

        let raw = vec![
            OhlcvBar::new(2_000, 10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(1_000, 9.0, 10.0, 8.0, 9.5, 50.0),
            // Duplicate timestamp 2_000: newer update must win!
            OhlcvBar::new(2_000, 10.0, 13.0, 9.0, 12.5, 200.0),
            // Non-finite row must be filtered.
            OhlcvBar::new(3_000, f64::NAN, 10.0, 9.0, 10.0, 10.0),
        ];
        let norm = normalize_bars(raw);
        assert_eq!(norm.len(), 2);
        assert_eq!(norm[0].timestamp, 1_000);
        assert_eq!(norm[1].timestamp, 2_000);
        assert_eq!(norm[1].close, 12.5);
        assert_eq!(norm[1].turnover, 12.5 * 200.0);
    }
}
