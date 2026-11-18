//! Pure `f64`, `wasm32-unknown-unknown`-compatible Multi-Symbol Watchlist & Technical Scanner Engine
//! (`Milestone R08 / Wave 7`).
//!
//! Evaluates multi-symbol Watchlist snapshots (`Last`, `%Chg`, `RVol(20)`, `SMA(20)`, `EMA(50)`,
//! `RSI(14)`, `MACD(12, 26, 9)`, `BB(20, 2.0)` bandwidth, `ATR(14)%`, [`MarketRegime`], and causal
//! [`ScanSignalKind`] triggers), historical signal scans gated by `DEF-001` `cutoff_timestamp`, and
//! type-enforced [`ScannerSignalContext`] (`BlindPractice` vs `SignalReview` signal reveal lock).

pub mod breadth;
pub mod catalog;
pub mod engine;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::model::Candle;

pub use breadth::{
    BreadthFeedInput, MarketBreadthSnapshot, RrgQuadrant, SymbolBreadthRotation,
    compute_market_breadth_and_rrg,
};
pub use catalog::{
    CONFLUENCE_DATA_FIDELITY_PROXY, CONFLUENCE_WARMUP_BARS, CausalConfluenceReport,
    CompactSignalPoint, ConfluenceBias, ConfluenceWeightProfile, PrecomputedCatalogMatrix,
    SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE, SIGNAL_CATALOG, SignalAttributionFactor,
    SignalCatalogEntry, SignalCategory, SignalEvaluation, SignalOutputType, SignalParamSpec,
    SignalQuality, SignalStatus, TechnicalHealthBreakdown, catalog_entry_index,
    evaluate_causal_confluence, evaluate_signal_catalog_at_bar, explain_historical_signal,
    filter_signal_catalog, find_catalog_entry, find_catalog_entry_for_scan_kind,
    is_bidirectional_bool_signal,
};
pub use engine::{
    DEFAULT_BLIND_PRACTICE_LOOKBACK_BARS, compute_causal_rvol, evaluate_symbol_scan,
    evaluate_symbol_scan_and_signals, filter_and_sort_watchlist, filter_historical_signals,
    scan_historical_signals,
};

/// Causal market regime classification aligned with `SIG-REG-001..005` (`ARCHIVE/backend/app/domain/signals/regimes.py`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MarketRegime {
    Uptrend,
    Downtrend,
    Pullback,
    Recovery,
    SidewaysSqueeze,
    #[default]
    Neutral,
    InsufficientHistory,
}

impl MarketRegime {
    pub const ALL_FILTERABLE: [Self; 6] = [
        Self::Uptrend,
        Self::Downtrend,
        Self::Pullback,
        Self::Recovery,
        Self::SidewaysSqueeze,
        Self::Neutral,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Uptrend => "UP",
            Self::Downtrend => "DN",
            Self::Pullback => "PB",
            Self::Recovery => "REC",
            Self::SidewaysSqueeze => "SQZ",
            Self::Neutral => "NEU",
            Self::InsufficientHistory => "WARM",
        }
    }

    #[inline]
    #[must_use]
    pub const fn full_label(self) -> &'static str {
        match self {
            Self::Uptrend => "Uptrend",
            Self::Downtrend => "Downtrend",
            Self::Pullback => "Pullback",
            Self::Recovery => "Recovery",
            Self::SidewaysSqueeze => "Squeeze",
            Self::Neutral => "Neutral",
            Self::InsufficientHistory => "Warmup",
        }
    }

    /// Canonical 6-bit mask covering all valid causal [`MarketRegime::ALL_FILTERABLE`] states (`0x3F`).
    pub const ALL_VALID_MASK: u8 = 0x3F;

    /// Returns the single-bit mask (`1 << 0` .. `1 << 5`) for this regime, or `0` during warmup
    /// (`InsufficientHistory` fails closed against every bitmask).
    #[inline]
    #[must_use]
    pub const fn bit(self) -> u8 {
        match self {
            Self::Uptrend => 1 << 0,
            Self::Downtrend => 1 << 1,
            Self::Pullback => 1 << 2,
            Self::Recovery => 1 << 3,
            Self::SidewaysSqueeze => 1 << 4,
            Self::Neutral => 1 << 5,
            Self::InsufficientHistory => 0,
        }
    }
}

/// Directional bias of a technical or volume scan signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalDirection {
    Bullish,
    Bearish,
}

impl SignalDirection {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Bullish => "BULL",
            Self::Bearish => "BEAR",
        }
    }
}

/// Canonical causal technical & volume triggers evaluated across loaded instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScanSignalKind {
    RsiOversoldCrossUp,
    RsiOverboughtCrossDown,
    MacdBullCross,
    MacdBearCross,
    SmaEmaGoldenCross,
    SmaEmaDeathCross,
    BollingerSqueezeBreakout,
    VolumeSpike2x,
}

impl ScanSignalKind {
    pub const ALL: [Self; 8] = [
        Self::RsiOversoldCrossUp,
        Self::RsiOverboughtCrossDown,
        Self::MacdBullCross,
        Self::MacdBearCross,
        Self::SmaEmaGoldenCross,
        Self::SmaEmaDeathCross,
        Self::BollingerSqueezeBreakout,
        Self::VolumeSpike2x,
    ];

    #[inline]
    #[must_use]
    pub const fn direction(self) -> SignalDirection {
        match self {
            Self::RsiOversoldCrossUp
            | Self::MacdBullCross
            | Self::SmaEmaGoldenCross
            | Self::BollingerSqueezeBreakout
            | Self::VolumeSpike2x => SignalDirection::Bullish,
            Self::RsiOverboughtCrossDown | Self::MacdBearCross | Self::SmaEmaDeathCross => {
                SignalDirection::Bearish
            }
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::RsiOversoldCrossUp => "RSI>30",
            Self::RsiOverboughtCrossDown => "RSI<70",
            Self::MacdBullCross => "MACD+",
            Self::MacdBearCross => "MACD-",
            Self::SmaEmaGoldenCross => "GOLD-X",
            Self::SmaEmaDeathCross => "DEATH-X",
            Self::BollingerSqueezeBreakout => "BB-BRK",
            Self::VolumeSpike2x => "VOL-2X",
        }
    }

    #[inline]
    #[must_use]
    pub const fn full_label(self) -> &'static str {
        match self {
            Self::RsiOversoldCrossUp => "RSI(14) Cross Above 30",
            Self::RsiOverboughtCrossDown => "RSI(14) Cross Below 70",
            Self::MacdBullCross => "MACD Bullish Cross",
            Self::MacdBearCross => "MACD Bearish Cross",
            Self::SmaEmaGoldenCross => "SMA(20)/EMA(50) Golden Cross",
            Self::SmaEmaDeathCross => "SMA(20)/EMA(50) Death Cross",
            Self::BollingerSqueezeBreakout => "Bollinger Squeeze Breakout",
            Self::VolumeSpike2x => "Volume Spike >= 2.0x",
        }
    }
}

/// One causal historical signal occurrence detected on `symbol` at `bar_index` (`timestamp`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalSignalHit {
    pub instrument_id: String,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub bar_index: usize,
    pub timestamp: i64,
    pub price: f64,
    pub signal: ScanSignalKind,
    pub direction: SignalDirection,
    pub regime: MarketRegime,
    pub metric_value: f64,
}

/// Snapshot row for one instrument in the Multi-Symbol Watchlist & Technical Scanner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolScanRow {
    pub instrument_id: String,
    pub symbol: String,
    pub source_label: String,
    pub timeframe: Timeframe,
    pub bar_count: usize,
    pub last_timestamp: i64,
    pub last_close: f64,
    pub change_abs: f64,
    pub change_pct: f64,
    pub last_volume: f64,
    pub rvol_20: Option<f64>,
    pub sma_20: Option<f64>,
    pub ema_50: Option<f64>,
    pub rsi_14: Option<f64>,
    pub macd_hist: Option<f64>,
    pub bb_bandwidth_pct: Option<f64>,
    pub atr_pct: Option<f64>,
    pub regime: MarketRegime,
    pub active_signals: Vec<ScanSignalKind>,
    pub latest_signal: Option<HistoricalSignalHit>,
}

/// Replay launch mode from a scanned historical signal (`ScannerSourceContext.tsx` / `test_scanner_replay_integrity.py`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ScannerReplayIntent {
    /// Starts Bar Replay `lookback_bars` prior to the signal candle and masks signal details until
    /// the causal replay frontier reaches `reveal_at_timestamp` (re-masking on rewind!).
    #[default]
    BlindPractice,
    /// Starts Bar Replay directly on the signal candle with signal context immediately revealed.
    SignalReview,
}

impl ScannerReplayIntent {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::BlindPractice => "BLIND PRACTICE",
            Self::SignalReview => "SIGNAL REVIEW",
        }
    }
}

/// Type-safe causal signal envelope attached to an active Bar Replay session.
///
/// The inner [`HistoricalSignalHit`] is kept private so workstation UI code can access signal
/// metadata **only** through [`Self::revealed_signal`], enforcing zero future-signal leakage prior
/// to `reveal_at_timestamp` during `BlindPractice`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannerSignalContext {
    intent: ScannerReplayIntent,
    reveal_at_timestamp: i64,
    signal: HistoricalSignalHit,
}

impl ScannerSignalContext {
    #[must_use]
    pub fn new(intent: ScannerReplayIntent, signal: HistoricalSignalHit) -> Self {
        let reveal_at_timestamp = signal.timestamp;
        Self {
            intent,
            reveal_at_timestamp,
            signal,
        }
    }

    /// Constructs a [`ScannerSignalContext`] with an explicit causal closing base-bar timestamp
    /// (`reveal_at_timestamp >= signal.timestamp`), ensuring multi-timeframe `BlindPractice` never
    /// reveals a higher-timeframe signal on the opening sub-bar of its bucket (`[CRITICAL-02]`).
    #[must_use]
    pub fn with_reveal_timestamp(
        intent: ScannerReplayIntent,
        signal: HistoricalSignalHit,
        reveal_at_timestamp: i64,
    ) -> Self {
        let reveal_ts = reveal_at_timestamp.max(signal.timestamp);
        Self {
            intent,
            reveal_at_timestamp: reveal_ts,
            signal,
        }
    }

    pub fn sanitize_deserialized(&mut self) {
        crate::alerts::truncate_utf8_bytes(&mut self.signal.instrument_id, 128);
        crate::alerts::truncate_utf8_bytes(&mut self.signal.symbol, 64);
        if !self.signal.price.is_finite() {
            self.signal.price = 0.0;
        }
        if !self.signal.metric_value.is_finite() {
            self.signal.metric_value = 0.0;
        }
        self.reveal_at_timestamp = self.reveal_at_timestamp.max(self.signal.timestamp);
    }

    #[inline]
    #[must_use]
    pub const fn intent(&self) -> ScannerReplayIntent {
        self.intent
    }

    #[inline]
    #[must_use]
    pub const fn reveal_at_timestamp(&self) -> i64 {
        self.reveal_at_timestamp
    }

    #[inline]
    #[must_use]
    pub fn instrument_id(&self) -> &str {
        &self.signal.instrument_id
    }

    #[inline]
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.signal.symbol
    }

    #[inline]
    #[must_use]
    pub const fn timeframe(&self) -> Timeframe {
        self.signal.timeframe
    }

    /// Returns `true` iff the signal is allowed to be revealed at `cutoff_timestamp`.
    #[inline]
    #[must_use]
    pub const fn is_revealed(&self, cutoff_timestamp: Option<i64>) -> bool {
        match self.intent {
            ScannerReplayIntent::SignalReview => true,
            ScannerReplayIntent::BlindPractice => match cutoff_timestamp {
                Some(ts) => ts >= self.reveal_at_timestamp,
                None => true,
            },
        }
    }

    /// Returns the underlying [`HistoricalSignalHit`] iff causally revealed at `cutoff_timestamp`.
    #[inline]
    #[must_use]
    pub fn revealed_signal(&self, cutoff_timestamp: Option<i64>) -> Option<&HistoricalSignalHit> {
        self.is_revealed(cutoff_timestamp).then_some(&self.signal)
    }

    /// Returns the full [`SignalEvaluation`] explanation for the attached signal **only** when
    /// causally revealed at `cutoff_timestamp` (`BlindPractice` non-leakage guarantee across
    /// forward play and backward rewind — `Milestone R15 / DEF-001`).
    #[must_use]
    pub fn revealed_explanation(
        &self,
        candles: &[Candle],
        cutoff_timestamp: Option<i64>,
    ) -> Option<SignalEvaluation> {
        if let Some(ts) = cutoff_timestamp
            && ts < self.reveal_at_timestamp
        {
            return None;
        }
        let hit = self.revealed_signal(cutoff_timestamp)?;
        let slice = match cutoff_timestamp {
            Some(ts) => {
                let bound = ts.max(hit.timestamp);
                let end = candles.partition_point(|c| c.timestamp <= bound);
                &candles[..end]
            }
            None => candles,
        };
        explain_historical_signal(slice, hit, None)
    }
}

/// Sort column selector for the Multi-Symbol Watchlist table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WatchlistSortColumn {
    #[default]
    Symbol,
    LastClose,
    ChangePct,
    RelativeVolume,
    Rsi14,
    SignalCount,
}

impl WatchlistSortColumn {
    pub const ALL: [Self; 6] = [
        Self::Symbol,
        Self::LastClose,
        Self::ChangePct,
        Self::RelativeVolume,
        Self::Rsi14,
        Self::SignalCount,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Symbol => "Sym",
            Self::LastClose => "Last",
            Self::ChangePct => "%Chg",
            Self::RelativeVolume => "RVol",
            Self::Rsi14 => "RSI",
            Self::SignalCount => "Sig",
        }
    }
}

/// Interactive filter criteria shared across Watchlist and Historical Signal views.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScannerFilter {
    pub search_query: String,
    pub signal_filter: Option<ScanSignalKind>,
    pub regime_filter: Option<MarketRegime>,
    pub direction_filter: Option<SignalDirection>,
    pub only_active_signals: bool,
}
