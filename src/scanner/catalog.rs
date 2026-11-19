//! Explainable `101`-Entry Signal Catalog & Orthogonal 4-Family Causal Confluence Engine
//! (`Milestones R15–R18 / Waves 14–17` — `B1`, `B3`, `B4`).
//!
//! Pure `f64`/`i64`/`u64`, `wasm32-unknown-unknown`-compatible signal definition registry (`100%`
//! strict superset of all `72` legacy `SignalRegistry` entries + `17` R15 entries + `12` `B2`
//! extended indicator triggers), quality-gated per-signal explanation builder, confirmed-pivot
//! four-way divergence engine (`left_bars = 3, right_bars = 3`, `available_at_index = t`),
//! `ProxyBB`-upgraded 4-family Technical Health scorer, and $O(N)$ [`PrecomputedCatalogMatrix`].

use serde::{Deserialize, Serialize};

use crate::indicators::{
    AdxOutput, BBDirection, BBHorizon, BBRegime, BollingerOutput, CandleGeometry,
    CandlePatternBarSummary, CandlestickPatternKind, CvdConfig, CvdResetMode, IchimokuConfig,
    IchimokuOutput, KeltnerOutput, MacdOutput, ProxyBBSeriesOutput, PsarOutput, StochasticOutput,
    SuperTrendOutput, compute_adx, compute_atr, compute_bar_volume_delta, compute_bollinger_slices,
    compute_candlestick_patterns_and_scores, compute_cci, compute_cvd, compute_ema_slices,
    compute_ichimoku, compute_keltner_channels, compute_macd_slices, compute_mfi,
    compute_proxy_bb_series, compute_psar, compute_rsi_slices, compute_sma_slices,
    compute_stochastic, compute_supertrend, donchian_midpoint_series,
};
use crate::model::Candle;

use super::{HistoricalSignalHit, MarketRegime, ScanSignalKind, SignalDirection};

/// Minimum causal bar index (`50` prior bars -> `bar_index >= 50`, i.e. 51 candles) required for
/// valid 4-family composite Technical Health scoring (`ARCHIVE/backend/app/domain/signals/health.py`).
pub const CONFLUENCE_WARMUP_BARS: usize = 50;

/// Canonical availability event emitted for all bar-close-confirmed causal signals (`DEF-001`).
pub const SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE: &str = "BAR_CLOSE";

/// Honest data-fidelity disclosure for OHLCV-derived participation & volume-delta proxies.
pub const CONFLUENCE_DATA_FIDELITY_PROXY: &str = "OHLCV_BAR_CLOSE_PROXY";

/// Signal taxonomy matching the 12 category tabs of `SignalCatalog.tsx:12-25`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SignalCategory {
    #[default]
    All,
    Health,
    Flow,
    Divergence,
    Ichimoku,
    Vsa,
    Volume,
    Technical,
    Regime,
    Pattern,
    SupportResistance,
    Structure,
}

impl SignalCategory {
    pub const ALL_TABS: [Self; 12] = [
        Self::All,
        Self::Health,
        Self::Flow,
        Self::Divergence,
        Self::Ichimoku,
        Self::Vsa,
        Self::Volume,
        Self::Technical,
        Self::Regime,
        Self::Pattern,
        Self::SupportResistance,
        Self::Structure,
    ];

    #[inline]
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Health => "health",
            Self::Flow => "flow",
            Self::Divergence => "divergence",
            Self::Ichimoku => "ichimoku",
            Self::Vsa => "vsa",
            Self::Volume => "volume",
            Self::Technical => "technical",
            Self::Regime => "regime",
            Self::Pattern => "pattern",
            Self::SupportResistance => "sr",
            Self::Structure => "structure",
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Health => "Health",
            Self::Flow => "Flow",
            Self::Divergence => "Div",
            Self::Ichimoku => "Ichi",
            Self::Vsa => "VSA",
            Self::Volume => "Vol",
            Self::Technical => "Tech",
            Self::Regime => "Regime",
            Self::Pattern => "Pattern",
            Self::SupportResistance => "S/R",
            Self::Structure => "Struct",
        }
    }

    #[inline]
    #[must_use]
    pub const fn full_label(self) -> &'static str {
        match self {
            Self::All => "All Categories",
            Self::Health => "Technical Health",
            Self::Flow => "Order Flow Proxy",
            Self::Divergence => "Momentum Divergence",
            Self::Ichimoku => "Ichimoku Kinko Hyo",
            Self::Vsa => "Volume Spread Analysis",
            Self::Volume => "Relative Volume",
            Self::Technical => "Technical Crossovers",
            Self::Regime => "Market Regime",
            Self::Pattern => "Candlestick & Volatility Patterns",
            Self::SupportResistance => "Support & Resistance",
            Self::Structure => "Market & Candle Structure",
        }
    }
}

/// Output value domain declared by a [`SignalCatalogEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalOutputType {
    Bool,
    Float,
    Enum,
}

impl SignalOutputType {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Bool => "BOOL",
            Self::Float => "FLOAT",
            Self::Enum => "ENUM",
        }
    }
}

/// Maturity tier of a catalog definition (`SignalCatalog.tsx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalStatus {
    Active,
    Experimental,
}

impl SignalStatus {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Experimental => "EXPERIMENTAL",
        }
    }
}

/// Causal data-validity gate (`ARCHIVE/backend/app/domain/signals/models.py:15-20` + R15 tail-bar guard).
///
/// Whenever `quality != SignalQuality::Valid`, [`SignalEvaluation::bool_value`] and
/// [`SignalEvaluation::float_value`] are guaranteed to be `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalQuality {
    Valid,
    InsufficientHistory,
    InvalidVolume,
    ZeroBaseline,
    UnclosedTailBar,
}

impl SignalQuality {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Valid => "VALID",
            Self::InsufficientHistory => "INSUFFICIENT_HISTORY",
            Self::InvalidVolume => "INVALID_VOLUME",
            Self::ZeroBaseline => "ZERO_BASELINE",
            Self::UnclosedTailBar => "UNCLOSED_TAIL_BAR",
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Parameter schema item attached to a [`SignalCatalogEntry`] (`UI-SIG-003` advanced bounds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalParamSpec {
    pub name: &'static str,
    pub default_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub description: &'static str,
}

/// Immutable Signal Catalog metadata entry (`101` entries spanning all 11 domain categories).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalCatalogEntry {
    pub id: &'static str,
    pub ast_alias: &'static str,
    pub label_en: &'static str,
    pub label_vi: &'static str,
    pub description: &'static str,
    pub category: SignalCategory,
    pub output_type: SignalOutputType,
    pub status: SignalStatus,
    pub version: &'static str,
    pub warmup_bars: usize,
    pub causal_delay_bars: usize,
    pub is_ohlcv_proxy: bool,
    pub scan_kind: Option<ScanSignalKind>,
    pub params: &'static [SignalParamSpec],
}

impl SignalCatalogEntry {
    /// Deterministic `SplitMix64` digest over `(id, version, params)` mirroring `compute_canonical_params_hash`.
    #[must_use]
    pub fn params_digest(&self) -> u64 {
        let mut h = 0xCBF2_9CE4_8422_2325_u64;
        mix_bytes(&mut h, self.id.as_bytes());
        mix_bytes(&mut h, self.version.as_bytes());
        for p in self.params {
            mix_bytes(&mut h, p.name.as_bytes());
            mix_u64(&mut h, canonical_f64_bits(p.default_value));
            mix_u64(&mut h, canonical_f64_bits(p.min_value));
            mix_u64(&mut h, canonical_f64_bits(p.max_value));
        }
        h
    }
}

#[inline]
fn canonical_f64_bits(v: f64) -> u64 {
    if v == 0.0 {
        0.0_f64.to_bits()
    } else if v.is_nan() {
        0x7FF8_0000_0000_0000
    } else {
        v.to_bits()
    }
}

#[inline]
fn mix_u64(state: &mut u64, word: u64) {
    let mut z = state.wrapping_add(word).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    *state = z ^ (z >> 31);
}

#[inline]
fn mix_bytes(state: &mut u64, bytes: &[u8]) {
    mix_u64(state, bytes.len() as u64);
    for &b in bytes {
        mix_u64(state, u64::from(b));
    }
}

const PARAMS_EMPTY: &[SignalParamSpec] = &[];

const PARAMS_RSI_CROSS: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "period",
        default_value: 14.0,
        min_value: 2.0,
        max_value: 100.0,
        description: "Wilder RSI smoothing period",
    },
    SignalParamSpec {
        name: "threshold",
        default_value: 30.0,
        min_value: 5.0,
        max_value: 95.0,
        description: "Oversold/Overbought crossover level",
    },
];

const PARAMS_MACD: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "fast",
        default_value: 12.0,
        min_value: 2.0,
        max_value: 100.0,
        description: "Fast EMA period",
    },
    SignalParamSpec {
        name: "slow",
        default_value: 26.0,
        min_value: 5.0,
        max_value: 200.0,
        description: "Slow EMA period",
    },
    SignalParamSpec {
        name: "signal",
        default_value: 9.0,
        min_value: 2.0,
        max_value: 60.0,
        description: "Signal EMA period",
    },
];

const PARAMS_MA_CROSS: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "sma_fast",
        default_value: 20.0,
        min_value: 5.0,
        max_value: 100.0,
        description: "Fast SMA period",
    },
    SignalParamSpec {
        name: "ema_slow",
        default_value: 50.0,
        min_value: 10.0,
        max_value: 250.0,
        description: "Slow EMA period",
    },
];

const PARAMS_BB_SQUEEZE: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "period",
        default_value: 20.0,
        min_value: 5.0,
        max_value: 100.0,
        description: "Bollinger middle SMA period",
    },
    SignalParamSpec {
        name: "max_prior_bw_pct",
        default_value: 10.0,
        min_value: 1.0,
        max_value: 30.0,
        description: "Prior-bar max Bollinger bandwidth (%)",
    },
];

const PARAMS_VOLUME_SPIKE: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "period",
        default_value: 20.0,
        min_value: 1.0,
        max_value: 252.0,
        description: "Strict prior-window baseline bars [t-period..t)",
    },
    SignalParamSpec {
        name: "multiplier",
        default_value: 2.0,
        min_value: 0.5,
        max_value: 100.0,
        description: "Minimum relative volume ratio",
    },
];

const PARAMS_PERIOD_20: &[SignalParamSpec] = &[SignalParamSpec {
    name: "period",
    default_value: 20.0,
    min_value: 1.0,
    max_value: 252.0,
    description: "Causal prior lookback window [t-period..t)",
}];

const PARAMS_SUPERTREND_10_3: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "period",
        default_value: 10.0,
        min_value: 2.0,
        max_value: 100.0,
        description: "Wilder ATR lookback period for SuperTrend",
    },
    SignalParamSpec {
        name: "multiplier",
        default_value: 3.0,
        min_value: 0.5,
        max_value: 10.0,
        description: "ATR band multiplier around HL2",
    },
];

const PARAMS_VSA_CLIMAX: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "rvol_min",
        default_value: 2.0,
        min_value: 1.2,
        max_value: 10.0,
        description: "Prior-window RVOL(20) threshold",
    },
    SignalParamSpec {
        name: "range_atr_min",
        default_value: 1.35,
        min_value: 0.8,
        max_value: 5.0,
        description: "Bar range / ATR(14) expansion ratio",
    },
];

const PARAMS_VSA_STOPPING: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "rvol_min",
        default_value: 1.8,
        min_value: 1.1,
        max_value: 10.0,
        description: "Prior-window RVOL(20) absorption threshold",
    },
    SignalParamSpec {
        name: "close_loc_min",
        default_value: 0.60,
        min_value: 0.50,
        max_value: 0.95,
        description: "Minimum close location inside [Low, High]",
    },
];

const PARAMS_RSI_DIV: &[SignalParamSpec] = &[SignalParamSpec {
    name: "pivot_lookback",
    default_value: 10.0,
    min_value: 4.0,
    max_value: 40.0,
    description: "Prior bar comparison window [t-lookback..t)",
}];

const PARAMS_CONFIRMED_PIVOT: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "left_bars",
        default_value: 3.0,
        min_value: 1.0,
        max_value: 20.0,
        description: "Left swing neighbor bars",
    },
    SignalParamSpec {
        name: "right_bars",
        default_value: 3.0,
        min_value: 1.0,
        max_value: 20.0,
        description: "Causal confirmation delay bars (p = t - right_bars)",
    },
];

const PARAMS_CONFIRMED_DIV: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "left_bars",
        default_value: 3.0,
        min_value: 1.0,
        max_value: 20.0,
        description: "Left swing neighbor bars",
    },
    SignalParamSpec {
        name: "right_bars",
        default_value: 3.0,
        min_value: 1.0,
        max_value: 20.0,
        description: "Causal confirmation delay bars (p = t - right_bars)",
    },
    SignalParamSpec {
        name: "min_separation",
        default_value: 5.0,
        min_value: 1.0,
        max_value: 50.0,
        description: "Minimum bars between confirmed pivots",
    },
    SignalParamSpec {
        name: "max_separation",
        default_value: 60.0,
        min_value: 5.0,
        max_value: 200.0,
        description: "Maximum bars between confirmed pivots",
    },
];

const PARAMS_ICHIMOKU: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "tenkan",
        default_value: 9.0,
        min_value: 5.0,
        max_value: 50.0,
        description: "Tenkan-sen Donchian period",
    },
    SignalParamSpec {
        name: "kijun",
        default_value: 26.0,
        min_value: 10.0,
        max_value: 100.0,
        description: "Kijun-sen & forward Kumo displacement",
    },
    SignalParamSpec {
        name: "senkou_b",
        default_value: 52.0,
        min_value: 20.0,
        max_value: 200.0,
        description: "Senkou Span B period",
    },
];

const PARAMS_FLOW_CVD: &[SignalParamSpec] = &[SignalParamSpec {
    name: "slope_bars",
    default_value: 5.0,
    min_value: 2.0,
    max_value: 30.0,
    description: "Causal OHLCV-proxy CVD impulse window",
}];

const PARAMS_SR_BREAKOUT: &[SignalParamSpec] = &[SignalParamSpec {
    name: "prior_window",
    default_value: 20.0,
    min_value: 5.0,
    max_value: 120.0,
    description: "Prior resistance/support window [t-N..t)",
}];

const PARAMS_SR_PROXIMITY: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "period",
        default_value: 20.0,
        min_value: 5.0,
        max_value: 252.0,
        description: "Prior rolling S/R lookback [t-period..t)",
    },
    SignalParamSpec {
        name: "tolerance_atr",
        default_value: 0.50,
        min_value: 0.10,
        max_value: 2.0,
        description: "ATR(14) proximity tolerance band",
    },
];

const PARAMS_STRUCT_BOS: &[SignalParamSpec] = &[SignalParamSpec {
    name: "swing_window",
    default_value: 10.0,
    min_value: 3.0,
    max_value: 60.0,
    description: "Prior swing high/low structure window [t-N..t)",
}];

const PARAMS_CANDLE_SCORE: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "w_direction",
        default_value: 0.45,
        min_value: 0.0,
        max_value: 1.0,
        description: "Signed body ratio weight",
    },
    SignalParamSpec {
        name: "w_close",
        default_value: 0.35,
        min_value: 0.0,
        max_value: 1.0,
        description: "Close location weight",
    },
    SignalParamSpec {
        name: "w_wick",
        default_value: 0.20,
        min_value: 0.0,
        max_value: 1.0,
        description: "Lower-vs-upper wick balance weight",
    },
];

const PARAMS_ENGULFING: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "min_body_ratio",
        default_value: 0.55,
        min_value: 0.10,
        max_value: 1.0,
        description: "Minimum engulfing candle body/range ratio",
    },
    SignalParamSpec {
        name: "min_close_location",
        default_value: 0.70,
        min_value: 0.50,
        max_value: 1.0,
        description: "Directional close location quality gate",
    },
];

const PARAMS_PINBAR: &[SignalParamSpec] = &[SignalParamSpec {
    name: "wick_multiplier",
    default_value: 2.0,
    min_value: 1.0,
    max_value: 5.0,
    description: "Minimum rejection wick / body ratio",
}];

const PARAMS_TWEEZER: &[SignalParamSpec] = &[SignalParamSpec {
    name: "tolerance_atr",
    default_value: 0.15,
    min_value: 0.01,
    max_value: 1.0,
    description: "Maximum two-bar high/low difference in ATR(14)",
}];

const PARAMS_INSIDE_BRK: &[SignalParamSpec] = &[SignalParamSpec {
    name: "breakout_buffer_atr",
    default_value: 0.05,
    min_value: 0.0,
    max_value: 0.5,
    description: "Mother-bar breakout buffer in ATR(14)",
}];

const PARAMS_BB_HORIZON: &[SignalParamSpec] = &[SignalParamSpec {
    name: "lookback",
    default_value: 20.0,
    min_value: 3.0,
    max_value: 200.0,
    description: "ProxyBB rolling horizon (T20 = 20 bars)",
}];

const PARAMS_BB_CONFLUENCE: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "short_lookback",
        default_value: 5.0,
        min_value: 3.0,
        max_value: 50.0,
        description: "Short ProxyBB horizon (T05)",
    },
    SignalParamSpec {
        name: "long_lookback",
        default_value: 20.0,
        min_value: 10.0,
        max_value: 200.0,
        description: "Structural ProxyBB horizon (T20)",
    },
];

const PARAMS_BB_TURN_UP: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "threshold",
        default_value: 40.0,
        min_value: 10.0,
        max_value: 90.0,
        description: "Parameterized turn-up score level",
    },
    SignalParamSpec {
        name: "tolerance",
        default_value: 2.0,
        min_value: 0.0,
        max_value: 15.0,
        description: "Prior-bar threshold tolerance",
    },
];

const PARAMS_BB_TURN_DOWN: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "threshold",
        default_value: 60.0,
        min_value: 10.0,
        max_value: 90.0,
        description: "Parameterized turn-down score level",
    },
    SignalParamSpec {
        name: "tolerance",
        default_value: 2.0,
        min_value: 0.0,
        max_value: 15.0,
        description: "Prior-bar threshold tolerance",
    },
];

const PARAMS_HEALTH: &[SignalParamSpec] = &[
    SignalParamSpec {
        name: "warmup_bars",
        default_value: 50.0,
        min_value: 50.0,
        max_value: 200.0,
        description: "Minimum causal history before emitting score",
    },
    SignalParamSpec {
        name: "strong_threshold",
        default_value: 35.0,
        min_value: 15.0,
        max_value: 80.0,
        description: "Strong bullish/bearish composite threshold",
    },
];

const PARAMS_REGIME: &[SignalParamSpec] = &[SignalParamSpec {
    name: "ema_slow",
    default_value: 50.0,
    min_value: 20.0,
    max_value: 200.0,
    description: "Slow trend baseline for SIG-REG-001..005",
}];

const PARAMS_PERIOD_14: &[SignalParamSpec] = &[SignalParamSpec {
    name: "period",
    default_value: 14.0,
    min_value: 2.0,
    max_value: 200.0,
    description: "Indicator smoothing period",
}];

macro_rules! sig_entry {
    (
        $id:expr, $alias:expr, $en:expr, $vi:expr, $desc:expr,
        $cat:ident, $out:ident, $status:ident, $warmup:expr, $delay:expr,
        $proxy:expr, $scan:expr, $params:expr
    ) => {
        SignalCatalogEntry {
            id: $id,
            ast_alias: $alias,
            label_en: $en,
            label_vi: $vi,
            description: $desc,
            category: SignalCategory::$cat,
            output_type: SignalOutputType::$out,
            status: SignalStatus::$status,
            version: "1.0.0",
            warmup_bars: $warmup,
            causal_delay_bars: $delay,
            is_ohlcv_proxy: $proxy,
            scan_kind: $scan,
            params: $params,
        }
    };
}

/// Canonical `101`-entry Explainable Signal Catalog (`17` R15 + `72` exact Legacy `SignalRegistry` + `12` `B2` indicator triggers).
pub static SIGNAL_CATALOG: [SignalCatalogEntry; 101] = [
    // =========================================================================
    // [0..17] Preserved R15/R16 Canonical Entries (`0` Breaking ID Changes)
    // =========================================================================
    sig_entry!(
        "health.technical_composite",
        "HEALTH_SCORE",
        "Technical Health Composite",
        "Điểm Sức Khỏe Kỹ Thuật Tổng Hợp",
        "Orthogonal 4-family composite [-100, +100] blending Trend, Momentum, Transition, and ProxyBB/RVOL Participation.",
        Health,
        Float,
        Active,
        CONFLUENCE_WARMUP_BARS,
        0,
        true,
        None,
        PARAMS_HEALTH
    ),
    sig_entry!(
        "regime.market_state",
        "REGIME_STATE",
        "Causal Market Regime (SIG-REG)",
        "Phân Loại Pha Thị Trường Nhân Quả",
        "Classifies Uptrend, Downtrend, Pullback, Recovery, or Sideways Squeeze using strict prior-window extrema.",
        Regime,
        Enum,
        Active,
        50,
        0,
        false,
        None,
        PARAMS_REGIME
    ),
    sig_entry!(
        "technical.rsi_oversold_cross",
        "RSI_CROSS_UP_30",
        "RSI(14) Cross Above 30",
        "RSI(14) Cắt Lên Khỏi Vùng Quá Bán 30",
        "Wilder RSI(14) crosses strictly above 30.0 at bar close from <= 30.0 on the prior bar.",
        Technical,
        Bool,
        Active,
        15,
        0,
        false,
        Some(ScanSignalKind::RsiOversoldCrossUp),
        PARAMS_RSI_CROSS
    ),
    sig_entry!(
        "technical.rsi_overbought_cross",
        "RSI_CROSS_DN_70",
        "RSI(14) Cross Below 70",
        "RSI(14) Cắt Xuống Khỏi Vùng Quá Mua 70",
        "Wilder RSI(14) crosses strictly below 70.0 at bar close from >= 70.0 on the prior bar.",
        Technical,
        Bool,
        Active,
        15,
        0,
        false,
        Some(ScanSignalKind::RsiOverboughtCrossDown),
        PARAMS_RSI_CROSS
    ),
    sig_entry!(
        "technical.macd_bull_cross",
        "MACD_BULL_X",
        "MACD(12,26,9) Bullish Cross",
        "MACD Cắt Lên Đường Tín Hiệu",
        "MACD histogram flips positive (prev <= 0, curr > 0) at bar close.",
        Technical,
        Bool,
        Active,
        34,
        0,
        false,
        Some(ScanSignalKind::MacdBullCross),
        PARAMS_MACD
    ),
    sig_entry!(
        "technical.macd_bear_cross",
        "MACD_BEAR_X",
        "MACD(12,26,9) Bearish Cross",
        "MACD Cắt Xuống Đường Tín Hiệu",
        "MACD histogram flips negative (prev >= 0, curr < 0) at bar close.",
        Technical,
        Bool,
        Active,
        34,
        0,
        false,
        Some(ScanSignalKind::MacdBearCross),
        PARAMS_MACD
    ),
    sig_entry!(
        "technical.sma_ema_golden_cross",
        "GOLDEN_CROSS_20_50",
        "SMA(20)/EMA(50) Golden Cross",
        "Giao Cắt Vàng SMA(20) Lên Trên EMA(50)",
        "Fast SMA(20) crosses strictly above slow EMA(50) at bar close.",
        Technical,
        Bool,
        Active,
        50,
        0,
        false,
        Some(ScanSignalKind::SmaEmaGoldenCross),
        PARAMS_MA_CROSS
    ),
    sig_entry!(
        "technical.sma_ema_death_cross",
        "DEATH_CROSS_20_50",
        "SMA(20)/EMA(50) Death Cross",
        "Giao Cắt Tử Thần SMA(20) Xuống Dưới EMA(50)",
        "Fast SMA(20) crosses strictly below slow EMA(50) at bar close.",
        Technical,
        Bool,
        Active,
        50,
        0,
        false,
        Some(ScanSignalKind::SmaEmaDeathCross),
        PARAMS_MA_CROSS
    ),
    sig_entry!(
        "pattern.bollinger_squeeze_breakout",
        "BB_SQUEEZE_BRK",
        "Bollinger Squeeze Breakout",
        "Phá Vỡ Nén Dải Bollinger(20, 2.0)",
        "Prior-bar Bollinger bandwidth <= 10% followed by a close above the upper band.",
        Pattern,
        Bool,
        Active,
        20,
        0,
        false,
        Some(ScanSignalKind::BollingerSqueezeBreakout),
        PARAMS_BB_SQUEEZE
    ),
    sig_entry!(
        "volume.spike_2x",
        "VOL_SPIKE_2X",
        "Causal Volume Spike >= 2.0x",
        "Đột Biến Khối Lượng >= 2.0x (Cửa Sổ Trước)",
        "Volume >= 2.0x prior 20-bar mean [t-20..t) (strictly excluding bar t) on a non-bearish close.",
        Volume,
        Bool,
        Active,
        20,
        0,
        false,
        Some(ScanSignalKind::VolumeSpike2x),
        PARAMS_VOLUME_SPIKE
    ),
    sig_entry!(
        "vsa.climax_up",
        "VSA_CLIMAX_UP",
        "VSA Buying Climax / Wide-Spread Thrust",
        "VSA Cao Trào Mua / Nến Biên Độ Rộng",
        "RVOL(20) >= 2.0x with spread >= 1.35x ATR(14) and upper-tier close location >= 0.65.",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_VSA_CLIMAX
    ),
    sig_entry!(
        "vsa.stopping_volume",
        "VSA_STOPPING_VOL",
        "VSA Stopping Volume Absorption",
        "VSA Khối Lượng Hấp Thụ Đáy",
        "New 10-bar low absorbed on RVOL(20) >= 1.8x with close recovering into top 40% of bar range.",
        Vsa,
        Bool,
        Experimental,
        20,
        0,
        true,
        None,
        PARAMS_VSA_STOPPING
    ),
    sig_entry!(
        "divergence.rsi_hidden_or_classic",
        "RSI_DIVERGENCE",
        "Causal RSI(14) Price Divergence",
        "Phân Kỳ Giá - RSI(14) Nhân Quả",
        "Detects lower-low price with higher RSI (bullish) or higher-high price with lower RSI (bearish) vs prior 10-bar extrema.",
        Divergence,
        Bool,
        Active,
        25,
        0,
        false,
        None,
        PARAMS_RSI_DIV
    ),
    sig_entry!(
        "ichimoku.tk_kumo_alignment",
        "ICHI_KUMO_ALIGN",
        "Ichimoku Tenkan/Kijun & Kumo Alignment",
        "Đồng Thuận Tenkan/Kijun & Mây Kumo",
        "Evaluates Close & Tenkan(9)/Kijun(26) alignment against the causally displaced Kumo cloud (t - 26).",
        Ichimoku,
        Float,
        Active,
        77,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "flow.cvd_ohlcv_impulse",
        "CVD_PROXY_IMPULSE",
        "CVD Intrabar Proxy 5B Impulse",
        "Xung Lực CVD Xấp Xỉ OHLCV (5 Nến)",
        "Measures 5-bar causal Cumulative Volume Delta slope normalized by 20-bar volume (explicit OHLCV_PROXY).",
        Flow,
        Float,
        Experimental,
        20,
        0,
        true,
        None,
        PARAMS_FLOW_CVD
    ),
    sig_entry!(
        "sr.prior_20b_breakout",
        "SR_20B_BREAKOUT",
        "Prior 20-Bar S/R Range Breakout",
        "Phá Vỡ Kháng Cự / Hỗ Trợ 20 Phiên Trước",
        "Close breaks strictly above max(High[t-20..t)) or below min(Low[t-20..t)) without current-bar self-inclusion.",
        SupportResistance,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_SR_BREAKOUT
    ),
    sig_entry!(
        "structure.swing_bos",
        "STRUCT_BOS_10B",
        "10-Bar Market Structure BOS",
        "Phá Vỡ Cấu Trúc Đỉnh/Đáy 10 Nến (BOS)",
        "Bullish/Bearish Break of Structure confirmed on bar close beyond prior 10-bar swing high/low with EMA(20) filter.",
        Structure,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_STRUCT_BOS
    ),
    // =========================================================================
    // [17..21] Legacy Volume Registry (`4` entries)
    // =========================================================================
    sig_entry!(
        "volume.relative_volume",
        "volume__relative_volume",
        "Causal Relative Volume RVOL(20)",
        "Khối Lượng Tương Đối RVOL(20)",
        "Ratio of bar t volume to mean(Volume[t-20..t)) strictly excluding bar t.",
        Volume,
        Float,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "volume.spike",
        "volume__spike",
        "Volume Spike >= 2.0x (Bidirectional)",
        "Đột Biến Khối Lượng (Volume Spike)",
        "Fires when RVOL(20) >= 2.0x on either bullish or bearish close.",
        Volume,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_VOLUME_SPIKE
    ),
    sig_entry!(
        "volume.climax_up",
        "volume__climax_up",
        "Volume Climax Up",
        "Cao Trào Khối Lượng Tăng (Volume Climax Up)",
        "Wide-spread bar (range/ATR14 >= 1.5) with RVOL(20) >= 2.0x closing in top 25% of range.",
        Volume,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_VSA_CLIMAX
    ),
    sig_entry!(
        "volume.climax_down",
        "volume__climax_down",
        "Volume Climax Down",
        "Cao Trào Khối Lượng Giảm (Volume Climax Down)",
        "Wide-spread bar (range/ATR14 >= 1.5) with RVOL(20) >= 2.0x closing in bottom 25% of range.",
        Volume,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_VSA_CLIMAX
    ),
    // =========================================================================
    // [21..35] `B1` Causal Candlestick Patterns (`14` entries)
    // =========================================================================
    sig_entry!(
        "pattern.bullish_engulfing",
        "pattern__bullish_engulfing",
        "Bullish Engulfing",
        "Nhấn Chìm Tăng (Bullish Engulfing)",
        "2-bar bullish reversal engulfing prior bearish body with body_ratio >= 0.55 and close_location >= 0.70.",
        Pattern,
        Bool,
        Active,
        1,
        0,
        false,
        None,
        PARAMS_ENGULFING
    ),
    sig_entry!(
        "pattern.bearish_engulfing",
        "pattern__bearish_engulfing",
        "Bearish Engulfing",
        "Nhấn Chìm Giảm (Bearish Engulfing)",
        "2-bar bearish reversal engulfing prior bullish body with body_ratio >= 0.55 and close_location <= 0.30.",
        Pattern,
        Bool,
        Active,
        1,
        0,
        false,
        None,
        PARAMS_ENGULFING
    ),
    sig_entry!(
        "pattern.hammer",
        "pattern__hammer",
        "Hammer / Bullish Pinbar",
        "Nến Búa / Pinbar Tăng (Hammer)",
        "Lower rejection wick >= 2x body, upper_wick_ratio <= 0.25, body_ratio <= 0.50, close_location >= 0.65.",
        Pattern,
        Bool,
        Active,
        0,
        0,
        false,
        None,
        PARAMS_PINBAR
    ),
    sig_entry!(
        "pattern.shooting_star",
        "pattern__shooting_star",
        "Shooting Star / Bearish Pinbar",
        "Nến Sao Băng / Pinbar Giảm (Shooting Star)",
        "Upper rejection wick >= 2x body, lower_wick_ratio <= 0.25, body_ratio <= 0.50, close_location <= 0.35.",
        Pattern,
        Bool,
        Active,
        0,
        0,
        false,
        None,
        PARAMS_PINBAR
    ),
    sig_entry!(
        "pattern.morning_star",
        "pattern__morning_star",
        "Morning Star (3-Bar)",
        "Sao Mai (Morning Star)",
        "3-bar bullish reversal: solid bearish bar, small star body, bullish reclaim >= first bar midpoint.",
        Pattern,
        Bool,
        Active,
        2,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "pattern.evening_star",
        "pattern__evening_star",
        "Evening Star (3-Bar)",
        "Sao Hôm (Evening Star)",
        "3-bar bearish reversal: solid bullish bar, small star body, bearish breakdown <= first bar midpoint.",
        Pattern,
        Bool,
        Active,
        2,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "pattern.piercing_line",
        "pattern__piercing_line",
        "Piercing Line",
        "Đường Xuyên Phá (Piercing Line)",
        "Gap-or-equal open below prior bearish close rebounding above prior body midpoint (close_location >= 0.60).",
        Pattern,
        Bool,
        Active,
        1,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "pattern.dark_cloud_cover",
        "pattern__dark_cloud_cover",
        "Dark Cloud Cover",
        "Mây Đen Bao Phủ (Dark Cloud Cover)",
        "Gap-or-equal open above prior bullish close rejecting below prior body midpoint (close_location <= 0.40).",
        Pattern,
        Bool,
        Active,
        1,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "pattern.tweezer_bottom",
        "pattern__tweezer_bottom",
        "Tweezer Bottom",
        "Đáy Nhíp (Tweezer Bottom)",
        "Two consecutive lows within 0.15 * ATR(14) transitioning from bearish/flat to bullish.",
        Pattern,
        Bool,
        Active,
        14,
        0,
        false,
        None,
        PARAMS_TWEEZER
    ),
    sig_entry!(
        "pattern.tweezer_top",
        "pattern__tweezer_top",
        "Tweezer Top",
        "Đỉnh Nhíp (Tweezer Top)",
        "Two consecutive highs within 0.15 * ATR(14) transitioning from bullish/flat to bearish.",
        Pattern,
        Bool,
        Active,
        14,
        0,
        false,
        None,
        PARAMS_TWEEZER
    ),
    sig_entry!(
        "pattern.inside_bar_breakout_up",
        "pattern__inside_bar_breakout_up",
        "Inside Bar Breakout Up",
        "Phá Vỡ Inside Bar Tăng",
        "Inside bar at t-1 inside mother bar t-2 followed by close > mother_high + 0.05 * ATR(14).",
        Pattern,
        Bool,
        Active,
        14,
        0,
        false,
        None,
        PARAMS_INSIDE_BRK
    ),
    sig_entry!(
        "pattern.inside_bar_breakout_down",
        "pattern__inside_bar_breakout_down",
        "Inside Bar Breakout Down",
        "Phá Vỡ Inside Bar Giảm",
        "Inside bar at t-1 inside mother bar t-2 followed by close < mother_low - 0.05 * ATR(14).",
        Pattern,
        Bool,
        Active,
        14,
        0,
        false,
        None,
        PARAMS_INSIDE_BRK
    ),
    sig_entry!(
        "pattern.any_bullish",
        "pattern__any_bullish",
        "Any Bullish Candlestick Pattern",
        "Bất Kỳ Mô Hình Nến Tăng Nào",
        "Aggregate trigger across all 6 causal bullish candlestick patterns with matched pattern attribution.",
        Pattern,
        Bool,
        Active,
        0,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "pattern.any_bearish",
        "pattern__any_bearish",
        "Any Bearish Candlestick Pattern",
        "Bất Kỳ Mô Hình Nến Giảm Nào",
        "Aggregate trigger across all 6 causal bearish candlestick patterns with matched pattern attribution.",
        Pattern,
        Bool,
        Active,
        0,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    // =========================================================================
    // [35..41] Legacy `tech.*` Triggers (`6` entries)
    // =========================================================================
    sig_entry!(
        "tech.macd_signal_cross",
        "tech__macd_signal_cross",
        "MACD Signal Crossover (Bidirectional)",
        "Giao Cắt MACD Signal",
        "MACD(12,26) crosses above (Bullish) or below (Bearish) Signal(9).",
        Technical,
        Bool,
        Active,
        34,
        0,
        false,
        None,
        PARAMS_MACD
    ),
    sig_entry!(
        "tech.macd_zero_cross",
        "tech__macd_zero_cross",
        "MACD Zero-Line Cross (Bidirectional)",
        "Giao Cắt MACD Qua Mức 0",
        "MACD(12,26) line crosses above or below 0.0 at bar close.",
        Technical,
        Bool,
        Active,
        26,
        0,
        false,
        None,
        PARAMS_MACD
    ),
    sig_entry!(
        "tech.rsi_level_cross",
        "tech__rsi_level_cross",
        "RSI(14) Threshold Cross (30 / 70)",
        "Giao Cắt Mức RSI",
        "RSI(14) crosses above 30 (Bullish) or below 70 (Bearish).",
        Technical,
        Bool,
        Active,
        15,
        0,
        false,
        None,
        PARAMS_RSI_CROSS
    ),
    sig_entry!(
        "tech.ema_cross",
        "tech__ema_cross",
        "EMA(20)/EMA(50) Crossover",
        "Giao Cắt Đường EMA(20/50)",
        "Fast EMA(20) crosses strictly above (Bullish) or below (Bearish) slow EMA(50).",
        Technical,
        Bool,
        Active,
        50,
        0,
        false,
        None,
        PARAMS_MA_CROSS
    ),
    sig_entry!(
        "tech.swing_break",
        "tech__swing_break",
        "Confirmed Swing Reference Break",
        "Phá Vỡ Đỉnh/Đáy Swing Đã Xác Nhận",
        "Close breaks confirmed 3-bar swing high/low (confirmed <= t-1) by 0.05 * ATR(14).",
        Technical,
        Bool,
        Active,
        14,
        0,
        false,
        None,
        PARAMS_INSIDE_BRK
    ),
    sig_entry!(
        "tech.composite",
        "tech__composite",
        "Composite Technical Multi-Trigger (>= 2)",
        "Bộ Kích Hoạt Kỹ Thuật Tổng Hợp",
        "Fires when >= 2 technical triggers confirm in the same direction on bar t.",
        Technical,
        Bool,
        Active,
        50,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    // =========================================================================
    // [41..48] Legacy `regime.*` Signals (`7` entries)
    // =========================================================================
    sig_entry!(
        "regime.uptrend",
        "regime__uptrend",
        "Uptrend Regime (SIG-REG-001)",
        "Chế Độ Xu Hướng Tăng",
        "Close > EMA(20) > EMA(50) with positive 5-bar EMA(20) slope.",
        Regime,
        Bool,
        Active,
        54,
        0,
        false,
        None,
        PARAMS_REGIME
    ),
    sig_entry!(
        "regime.downtrend",
        "regime__downtrend",
        "Downtrend Regime (SIG-REG-002)",
        "Chế Độ Xu Hướng Giảm",
        "Close < EMA(20) < EMA(50) with negative 5-bar EMA(20) slope.",
        Regime,
        Bool,
        Active,
        54,
        0,
        false,
        None,
        PARAMS_REGIME
    ),
    sig_entry!(
        "regime.sideways",
        "regime__sideways",
        "Sideways Consolidation Regime",
        "Chế Độ Đi Ngang (Sideways)",
        "5-bar relative EMA(20) slope < 0.8% consolidation regime.",
        Regime,
        Bool,
        Active,
        25,
        0,
        false,
        None,
        PARAMS_REGIME
    ),
    sig_entry!(
        "regime.pullback",
        "regime__pullback",
        "Uptrend Pullback Regime",
        "Chế Độ Điều Chỉnh (Pullback)",
        "Healthy 2%..12% pullback from prior 20-bar high holding above EMA(50) - 0.5 * ATR(14).",
        Regime,
        Bool,
        Active,
        50,
        0,
        false,
        None,
        PARAMS_REGIME
    ),
    sig_entry!(
        "regime.recovery",
        "regime__recovery",
        "Early Recovery Regime",
        "Chế Độ Phục Hồi Sớm",
        "Downtrend context (EMA20 < EMA50) showing >= 2 of 3 early rebound confirmations.",
        Regime,
        Bool,
        Active,
        50,
        0,
        false,
        None,
        PARAMS_REGIME
    ),
    sig_entry!(
        "regime.new_high",
        "regime__new_high",
        "Causal 20-Bar New High",
        "Đỉnh Cao Mới 20 Phiên (New High)",
        "High[t] > max(High[t-20..t)) strictly excluding current bar t (TEST-CAUSAL-003).",
        Regime,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "regime.new_low",
        "regime__new_low",
        "Causal 20-Bar New Low",
        "Đáy Thấp Mới 20 Phiên (New Low)",
        "Low[t] < min(Low[t-20..t)) strictly excluding current bar t (TEST-CAUSAL-003).",
        Regime,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_PERIOD_20
    ),
    // =========================================================================
    // [48..51] `B3` Support/Resistance & `B1` Candle Structure Score (`3` entries)
    // =========================================================================
    sig_entry!(
        "sr.near_support",
        "sr__near_support",
        "Near Support Level (0.5 ATR)",
        "Gần Ngưỡng Hỗ Trợ",
        "Within 0.5 * ATR(14) of prior 20-bar low or testing EMA(20/50/200) from above (Close >= EMA).",
        SupportResistance,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_SR_PROXIMITY
    ),
    sig_entry!(
        "sr.near_resistance",
        "sr__near_resistance",
        "Near Resistance Level (0.5 ATR)",
        "Gần Ngưỡng Kháng Cự",
        "Within 0.5 * ATR(14) of prior 20-bar high or testing EMA(20/50/200) from below (Close <= EMA).",
        SupportResistance,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_SR_PROXIMITY
    ),
    sig_entry!(
        "structure.candle_score",
        "structure__candle_score",
        "Candle Structure Score [-100, +100]",
        "Điểm Cấu Trúc Nến (Candle Structure Score)",
        "Transparent weighted score of signed body ratio (45%), close location (35%), and wick balance (20%).",
        Structure,
        Float,
        Active,
        0,
        0,
        false,
        None,
        PARAMS_CANDLE_SCORE
    ),
    // =========================================================================
    // [51..57] `B3` Volume Spread Analysis (`6` entries)
    // =========================================================================
    sig_entry!(
        "vsa.strong_demand",
        "vsa__strong_demand",
        "VSA Strong Demand",
        "Cầu Mạnh (VSA Strong Demand)",
        "Positive close expansion bar: RVOL(20) >= 1.5, Spread/ATR >= 1.2, body_ratio >= 0.50, close_loc >= 0.70.",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_VSA_CLIMAX
    ),
    sig_entry!(
        "vsa.weak_demand",
        "vsa__weak_demand",
        "VSA Weak / No Demand",
        "Cầu Yếu / Cạn Cầu (Weak Demand)",
        "Up/flat close on narrow spread (Spread/ATR <= 0.70) and low volume (RVOL(20) <= 0.80).",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "vsa.upthrust",
        "vsa__upthrust",
        "VSA Upthrust Rejection",
        "Upthrust / Kéo Xả Kỹ Thuật",
        "High breaches prior 20-bar resistance then closes back below with upper_wick >= 35% and RVOL >= 1.5x.",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "vsa.spring",
        "vsa__spring",
        "VSA Spring / Shakeout",
        "Spring / Shakeout / Đạp Kéo",
        "Low undercuts prior 20-bar support then reclaims above with lower_wick >= 35%, close_loc >= 0.70, RVOL >= 1.3x.",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "vsa.strong_demand_at_support",
        "vsa__strong_demand_at_support",
        "VSA Strong Demand at Support",
        "Cầu Mạnh Tại Hỗ Trợ",
        "Confluence of sr.near_support AND (vsa.strong_demand(rvol>=1.3) OR vsa.spring).",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_SR_PROXIMITY
    ),
    sig_entry!(
        "vsa.strong_supply_at_resistance",
        "vsa__strong_supply_at_resistance",
        "VSA Strong Supply at Resistance",
        "Cung Mạnh Tại Kháng Cự",
        "Confluence of sr.near_resistance AND (vsa.upthrust(rvol>=1.3) OR high-volume bearish rejection).",
        Vsa,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_SR_PROXIMITY
    ),
    // =========================================================================
    // [57..64] `B3` Full Ichimoku Signal Suite (`7` entries)
    // =========================================================================
    sig_entry!(
        "ichimoku.score",
        "ichimoku__score",
        "Ichimoku 4-Factor Score [-100, +100]",
        "Điểm Số Ichimoku (Ichimoku Score)",
        "4-factor causal score: Close vs visible Kumo(t-26), Tenkan vs Kijun, Chikou vs Close(t-26), and future Kumo.",
        Ichimoku,
        Float,
        Active,
        77,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "ichimoku.bullish",
        "ichimoku__bullish",
        "Ichimoku Bullish State (Score >= 50)",
        "Trạng Thái Tăng Ichimoku",
        "Fires when causal 4-factor Ichimoku score >= +50.0.",
        Ichimoku,
        Bool,
        Active,
        77,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "ichimoku.bearish",
        "ichimoku__bearish",
        "Ichimoku Bearish State (Score <= -50)",
        "Trạng Thái Giảm Ichimoku",
        "Fires when causal 4-factor Ichimoku score <= -50.0.",
        Ichimoku,
        Bool,
        Active,
        77,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "ichimoku.tk_cross_bullish",
        "ichimoku__tk_cross_bullish",
        "Ichimoku Bullish TK Cross",
        "Giao Cắt Vàng Tenkan/Kijun",
        "Tenkan-sen(9) crosses strictly above Kijun-sen(26) at bar close.",
        Ichimoku,
        Bool,
        Active,
        26,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "ichimoku.tk_cross_bearish",
        "ichimoku__tk_cross_bearish",
        "Ichimoku Bearish TK Cross",
        "Giao Cắt Tử Thần Tenkan/Kijun",
        "Tenkan-sen(9) crosses strictly below Kijun-sen(26) at bar close.",
        Ichimoku,
        Bool,
        Active,
        26,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "ichimoku.kumo_breakout_bullish",
        "ichimoku__kumo_breakout_bullish",
        "Ichimoku Bullish Kumo Breakout",
        "Phá Vỡ Mây Kumo Tăng",
        "Close crosses strictly above the visible Kumo cloud top (displaced from t - 26).",
        Ichimoku,
        Bool,
        Active,
        78,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    sig_entry!(
        "ichimoku.kumo_breakout_bearish",
        "ichimoku__kumo_breakout_bearish",
        "Ichimoku Bearish Kumo Breakout",
        "Phá Vỡ Mây Kumo Giảm",
        "Close crosses strictly below the visible Kumo cloud bottom (displaced from t - 26).",
        Ichimoku,
        Bool,
        Active,
        78,
        0,
        false,
        None,
        PARAMS_ICHIMOKU
    ),
    // =========================================================================
    // [64..78] `B3` Confirmed-Pivot Divergences (`right_bars = 3`, `14` entries)
    // =========================================================================
    sig_entry!(
        "divergence.confirmed_pivot_high",
        "divergence__confirmed_pivot_high",
        "Confirmed Swing Pivot High (+3B)",
        "Đỉnh Pivot Xác Nhận (right_bars = 3)",
        "Confirms swing high at candidate p = t - 3 strictly at bar close t (zero lookahead).",
        Divergence,
        Bool,
        Active,
        6,
        3,
        false,
        None,
        PARAMS_CONFIRMED_PIVOT
    ),
    sig_entry!(
        "divergence.confirmed_pivot_low",
        "divergence__confirmed_pivot_low",
        "Confirmed Swing Pivot Low (+3B)",
        "Đáy Pivot Xác Nhận (right_bars = 3)",
        "Confirms swing low at candidate p = t - 3 strictly at bar close t (zero lookahead).",
        Divergence,
        Bool,
        Active,
        6,
        3,
        false,
        None,
        PARAMS_CONFIRMED_PIVOT
    ),
    sig_entry!(
        "divergence.rsi_regular_bullish",
        "divergence__rsi_regular_bullish",
        "RSI(14) Regular Bullish Divergence (+3B)",
        "Phân Kỳ Thường Tăng RSI",
        "Confirmed pivot lows (p = t - 3): Price Lower Low + RSI(14) Higher Low.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.rsi_regular_bearish",
        "divergence__rsi_regular_bearish",
        "RSI(14) Regular Bearish Divergence (+3B)",
        "Phân Kỳ Thường Giảm RSI",
        "Confirmed pivot highs (p = t - 3): Price Higher High + RSI(14) Lower High.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.rsi_hidden_bullish",
        "divergence__rsi_hidden_bullish",
        "RSI(14) Hidden Bullish Divergence (+3B)",
        "Phân Kỳ Ẩn Tăng RSI",
        "Confirmed pivot lows (p = t - 3): Price Higher Low + RSI(14) Lower Low.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.rsi_hidden_bearish",
        "divergence__rsi_hidden_bearish",
        "RSI(14) Hidden Bearish Divergence (+3B)",
        "Phân Kỳ Ẩn Giảm RSI",
        "Confirmed pivot highs (p = t - 3): Price Lower High + RSI(14) Higher High.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.macd_regular_bullish",
        "divergence__macd_regular_bullish",
        "MACD Hist Regular Bullish Divergence (+3B)",
        "Phân Kỳ Thường Tăng MACD Histogram",
        "Confirmed pivot lows (p = t - 3): Price Lower Low + MACD Histogram Higher Low.",
        Divergence,
        Bool,
        Active,
        36,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.macd_regular_bearish",
        "divergence__macd_regular_bearish",
        "MACD Hist Regular Bearish Divergence (+3B)",
        "Phân Kỳ Thường Giảm MACD Histogram",
        "Confirmed pivot highs (p = t - 3): Price Higher High + MACD Histogram Lower High.",
        Divergence,
        Bool,
        Active,
        36,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.macd_hidden_bullish",
        "divergence__macd_hidden_bullish",
        "MACD Hist Hidden Bullish Divergence (+3B)",
        "Phân Kỳ Ẩn Tăng MACD Histogram",
        "Confirmed pivot lows (p = t - 3): Price Higher Low + MACD Histogram Lower Low.",
        Divergence,
        Bool,
        Active,
        36,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.macd_hidden_bearish",
        "divergence__macd_hidden_bearish",
        "MACD Hist Hidden Bearish Divergence (+3B)",
        "Phân Kỳ Ẩn Giảm MACD Histogram",
        "Confirmed pivot highs (p = t - 3): Price Lower High + MACD Histogram Higher High.",
        Divergence,
        Bool,
        Active,
        36,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.stoch_regular_bullish",
        "divergence__stoch_regular_bullish",
        "Stochastic Regular Bullish Divergence (+3B)",
        "Phân Kỳ Thường Tăng Stochastic",
        "Confirmed pivot lows (p = t - 3): Price Lower Low + Stochastic %K(14) Higher Low.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.stoch_regular_bearish",
        "divergence__stoch_regular_bearish",
        "Stochastic Regular Bearish Divergence (+3B)",
        "Phân Kỳ Thường Giảm Stochastic",
        "Confirmed pivot highs (p = t - 3): Price Higher High + Stochastic %K(14) Lower High.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.stoch_hidden_bullish",
        "divergence__stoch_hidden_bullish",
        "Stochastic Hidden Bullish Divergence (+3B)",
        "Phân Kỳ Ẩn Tăng Stochastic",
        "Confirmed pivot lows (p = t - 3): Price Higher Low + Stochastic %K(14) Lower Low.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    sig_entry!(
        "divergence.stoch_hidden_bearish",
        "divergence__stoch_hidden_bearish",
        "Stochastic Hidden Bearish Divergence (+3B)",
        "Phân Kỳ Ẩn Giảm Stochastic",
        "Confirmed pivot highs (p = t - 3): Price Lower High + Stochastic %K(14) Higher High.",
        Divergence,
        Bool,
        Active,
        20,
        3,
        false,
        None,
        PARAMS_CONFIRMED_DIV
    ),
    // =========================================================================
    // [78..81] Legacy `health.*` Aliases (`3` entries)
    // =========================================================================
    sig_entry!(
        "health.score",
        "health__score",
        "Technical Health Score (Alias)",
        "Điểm Sức Khỏe Kỹ Thuật (SIG-HEALTH-001)",
        "4-family Technical Health score [-100, +100] respecting active Balanced/TrendFocused profile.",
        Health,
        Float,
        Active,
        CONFLUENCE_WARMUP_BARS,
        0,
        true,
        None,
        PARAMS_HEALTH
    ),
    sig_entry!(
        "health.favorable",
        "health__favorable",
        "Technical Health Favorable (>= +35)",
        "Trạng Thái Kỹ Thuật Tích Cực",
        "Composite Technical Health score >= +35.0.",
        Health,
        Bool,
        Active,
        CONFLUENCE_WARMUP_BARS,
        0,
        true,
        None,
        PARAMS_HEALTH
    ),
    sig_entry!(
        "health.unfavorable",
        "health__unfavorable",
        "Technical Health Unfavorable (<= -35)",
        "Trạng Thái Kỹ Thuật Tiêu Cực",
        "Composite Technical Health score <= -35.0.",
        Health,
        Bool,
        Active,
        CONFLUENCE_WARMUP_BARS,
        0,
        true,
        None,
        PARAMS_HEALTH
    ),
    // =========================================================================
    // [81..89] `B4` Money Flow `ProxyBB` Signals (`8` entries, `AT05` Gated)
    // =========================================================================
    sig_entry!(
        "bb.direction_rising",
        "bb__direction_rising",
        "ProxyBB(T20) Direction Rising",
        "Dòng Tiền BB Chuyển Động Tăng",
        "OHLCV_PROXY Money Flow BB(T20) increases above prior valid session score.",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_HORIZON
    ),
    sig_entry!(
        "bb.direction_falling",
        "bb__direction_falling",
        "ProxyBB(T20) Direction Falling",
        "Dòng Tiền BB Chuyển Động Giảm",
        "OHLCV_PROXY Money Flow BB(T20) decreases below prior valid session score.",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_HORIZON
    ),
    sig_entry!(
        "bb.regime_positive",
        "bb__regime_positive",
        "ProxyBB(T20) Positive Regime (> 50)",
        "Chế Độ Dòng Tiền BB Tích Cực",
        "OHLCV_PROXY Money Flow BB(T20) stands above 50.0 equilibrium (net buy pressure).",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_HORIZON
    ),
    sig_entry!(
        "bb.regime_negative",
        "bb__regime_negative",
        "ProxyBB(T20) Negative Regime (< 50)",
        "Chế Độ Dòng Tiền BB Tiêu Cực",
        "OHLCV_PROXY Money Flow BB(T20) stands below 50.0 equilibrium (net sell pressure).",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_HORIZON
    ),
    sig_entry!(
        "bb.confluence_bullish",
        "bb__confluence_bullish",
        "ProxyBB Multi-Horizon Bullish Confluence (T05 + T20)",
        "Đồng Thuận Tăng Đa Kỳ Hạn Dòng Tiền BB",
        "Both T05 and T20 ProxyBB > 50.0 with T05 rising or >= T20.",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_CONFLUENCE
    ),
    sig_entry!(
        "bb.confluence_bearish",
        "bb__confluence_bearish",
        "ProxyBB Multi-Horizon Bearish Confluence (T05 + T20)",
        "Đồng Thuận Giảm Đa Kỳ Hạn Dòng Tiền BB",
        "Both T05 and T20 ProxyBB < 50.0 with T05 falling or <= T20.",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_CONFLUENCE
    ),
    sig_entry!(
        "bb.turn_up",
        "bb__turn_up",
        "ProxyBB(T20) Turn Up Across 40",
        "Dòng Tiền BB Ngoặt Lên Qua Ngưỡng 40",
        "Prior BB(T20) <= 42.0 and current BB(T20) turns upward >= 40.0.",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_TURN_UP
    ),
    sig_entry!(
        "bb.turn_down",
        "bb__turn_down",
        "ProxyBB(T20) Turn Down Across 60",
        "Dòng Tiền BB Ngoặt Xuống Qua Ngưỡng 60",
        "Prior BB(T20) >= 58.0 and current BB(T20) turns downward <= 60.0.",
        Flow,
        Bool,
        Active,
        20,
        0,
        true,
        None,
        PARAMS_BB_TURN_DOWN
    ),
    // =========================================================================
    // [89..101] `B2` Extended Indicator Triggers (`12` entries)
    // =========================================================================
    sig_entry!(
        "technical.adx_strong_trend",
        "ADX_STRONG_25",
        "ADX(14) Strong Trend >= 25",
        "Xu Hướng Mạnh ADX(14) >= 25",
        "Wilder ADX(14) >= 25.0 signed Bullish (+DI > -DI) or Bearish (-DI > +DI).",
        Technical,
        Bool,
        Active,
        27,
        0,
        false,
        None,
        PARAMS_PERIOD_14
    ),
    sig_entry!(
        "technical.di_bull_cross",
        "DI_BULL_X",
        "ADX +DI(14) Cross Above -DI(14)",
        "+DI(14) Cắt Lên Trên -DI(14)",
        "Wilder +DI(14) crosses strictly above -DI(14) at bar close.",
        Technical,
        Bool,
        Active,
        15,
        0,
        false,
        None,
        PARAMS_PERIOD_14
    ),
    sig_entry!(
        "technical.di_bear_cross",
        "DI_BEAR_X",
        "ADX -DI(14) Cross Above +DI(14)",
        "-DI(14) Cắt Lên Trên +DI(14)",
        "Wilder -DI(14) crosses strictly above +DI(14) at bar close.",
        Technical,
        Bool,
        Active,
        15,
        0,
        false,
        None,
        PARAMS_PERIOD_14
    ),
    sig_entry!(
        "technical.stoch_bull_cross",
        "STOCH_BULL_X",
        "Stochastic(14,3,3) Oversold Bull Cross",
        "Stochastic Cắt Lên Vùng Quá Bán",
        "Slow %K crosses above Slow %D from <= 25.0.",
        Technical,
        Bool,
        Active,
        18,
        0,
        false,
        None,
        PARAMS_PERIOD_14
    ),
    sig_entry!(
        "technical.stoch_bear_cross",
        "STOCH_BEAR_X",
        "Stochastic(14,3,3) Overbought Bear Cross",
        "Stochastic Cắt Xuống Vùng Quá Mua",
        "Slow %K crosses below Slow %D from >= 75.0.",
        Technical,
        Bool,
        Active,
        18,
        0,
        false,
        None,
        PARAMS_PERIOD_14
    ),
    sig_entry!(
        "technical.supertrend_flip_bull",
        "SUPERTREND_FLIP_UP",
        "SuperTrend(10, 3.0) Flip Bullish",
        "SuperTrend(10, 3.0) Đảo Chiều Tăng",
        "SuperTrend direction flips from -1 (resistance) to +1 (support) on bar close.",
        Technical,
        Bool,
        Active,
        10,
        0,
        false,
        None,
        PARAMS_SUPERTREND_10_3
    ),
    sig_entry!(
        "technical.supertrend_flip_bear",
        "SUPERTREND_FLIP_DN",
        "SuperTrend(10, 3.0) Flip Bearish",
        "SuperTrend(10, 3.0) Đảo Chiều Giảm",
        "SuperTrend direction flips from +1 (support) to -1 (resistance) on bar close.",
        Technical,
        Bool,
        Active,
        10,
        0,
        false,
        None,
        PARAMS_SUPERTREND_10_3
    ),
    sig_entry!(
        "technical.psar_flip_bull",
        "PSAR_FLIP_UP",
        "Parabolic SAR Flip Bullish",
        "Parabolic SAR Đảo Chiều Tăng",
        "Wilder Parabolic SAR(0.02, 0.20) flips from short to long.",
        Technical,
        Bool,
        Active,
        2,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "technical.psar_flip_bear",
        "PSAR_FLIP_DN",
        "Parabolic SAR Flip Bearish",
        "Parabolic SAR Đảo Chiều Giảm",
        "Wilder Parabolic SAR(0.02, 0.20) flips from long to short.",
        Technical,
        Bool,
        Active,
        2,
        0,
        false,
        None,
        PARAMS_EMPTY
    ),
    sig_entry!(
        "technical.keltner_upper_breakout",
        "KELTNER_UP_BRK",
        "Keltner Channel(20, 10, 2.0) Upper Breakout",
        "Phá Vỡ Cạnh Trên Kênh Keltner",
        "Close crosses strictly above EMA(20) + 2.0 * ATR(10).",
        Technical,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "technical.cci_oversold_cross",
        "CCI_CROSS_UP_M100",
        "CCI(20) Cross Above -100",
        "CCI(20) Cắt Lên Khỏi -100",
        "Commodity Channel Index CCI(20) crosses strictly above -100.0 from <= -100.0.",
        Technical,
        Bool,
        Active,
        20,
        0,
        false,
        None,
        PARAMS_PERIOD_20
    ),
    sig_entry!(
        "technical.mfi_oversold_cross",
        "MFI_CROSS_UP_20",
        "MFI(14) Cross Above 20",
        "MFI(14) Cắt Lên Khỏi Vùng Quá Bán 20",
        "Volume-weighted Money Flow Index MFI(14) crosses strictly above 20.0 from <= 20.0.",
        Technical,
        Bool,
        Active,
        15,
        0,
        false,
        None,
        PARAMS_PERIOD_14
    ),
];

/// Looks up a catalog definition by `id`.
#[must_use]
pub fn find_catalog_entry(id: &str) -> Option<&'static SignalCatalogEntry> {
    SIGNAL_CATALOG.iter().find(|e| e.id == id)
}

/// Resolves the canonical `[0..SIGNAL_CATALOG.len())` index of `id` inside [`SIGNAL_CATALOG`].
#[must_use]
pub fn catalog_entry_index(id: &str) -> Option<usize> {
    SIGNAL_CATALOG.iter().position(|e| e.id == id)
}

/// Returns `true` when a [`SignalOutputType::Bool`] catalog entry can fire in **either**
/// `Bullish` or `Bearish` direction on `bool_value == Some(true)` (requiring directional
/// disambiguation in Long vs Short rules and directional alerts).
#[inline]
#[must_use]
pub fn is_bidirectional_bool_signal(id: &str) -> bool {
    matches!(
        id,
        "divergence.rsi_hidden_or_classic"
            | "sr.prior_20b_breakout"
            | "structure.swing_bos"
            | "volume.spike"
            | "tech.macd_signal_cross"
            | "tech.macd_zero_cross"
            | "tech.rsi_level_cross"
            | "tech.ema_cross"
            | "tech.swing_break"
            | "tech.composite"
            | "technical.adx_strong_trend"
    )
}

/// Looks up the catalog definition mapped to a [`ScanSignalKind`].
#[must_use]
pub fn find_catalog_entry_for_scan_kind(kind: ScanSignalKind) -> &'static SignalCatalogEntry {
    SIGNAL_CATALOG
        .iter()
        .find(|e| e.scan_kind == Some(kind))
        .expect("every ScanSignalKind has a canonical SIGNAL_CATALOG entry")
}

/// Filters [`SIGNAL_CATALOG`] by category tab and case-insensitive search query across English/Vietnamese labels, alias, and description.
#[must_use]
pub fn filter_signal_catalog(
    category: SignalCategory,
    search_query: &str,
) -> Vec<&'static SignalCatalogEntry> {
    let q = search_query.trim().to_lowercase();
    SIGNAL_CATALOG
        .iter()
        .filter(|entry| {
            if category != SignalCategory::All && entry.category != category {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            entry.id.to_lowercase().contains(&q)
                || entry.ast_alias.to_lowercase().contains(&q)
                || entry.label_en.to_lowercase().contains(&q)
                || entry.label_vi.to_lowercase().contains(&q)
                || entry.description.to_lowercase().contains(&q)
                || entry.category.short_label().to_lowercase().contains(&q)
        })
        .collect()
}

/// Weighting preset for the 4 orthogonal Technical Health families (`health.py`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ConfluenceWeightProfile {
    /// `health_v1_balanced`: Trend `35%`, Momentum `25%`, Transition `20%`, Participation `20%`.
    #[default]
    Balanced,
    /// `health_v1_trend_focused`: Trend `50%`, Momentum `20%`, Transition `15%`, Participation `15%`.
    TrendFocused,
}

impl ConfluenceWeightProfile {
    #[inline]
    #[must_use]
    pub const fn weights(self) -> (f64, f64, f64, f64) {
        match self {
            Self::Balanced => (0.35, 0.25, 0.20, 0.20),
            Self::TrendFocused => (0.50, 0.20, 0.15, 0.15),
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced (35/25/20/20)",
            Self::TrendFocused => "Trend (50/20/15/15)",
        }
    }
}

/// Directional summary of [`CausalConfluenceReport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ConfluenceBias {
    StrongBullish,
    MildBullish,
    #[default]
    Neutral,
    MildBearish,
    StrongBearish,
}

impl ConfluenceBias {
    #[inline]
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if !score.is_finite() {
            Self::Neutral
        } else if score >= 35.0 {
            Self::StrongBullish
        } else if score >= 12.0 {
            Self::MildBullish
        } else if score <= -35.0 {
            Self::StrongBearish
        } else if score <= -12.0 {
            Self::MildBearish
        } else {
            Self::Neutral
        }
    }

    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StrongBullish => "STRONG BULLISH",
            Self::MildBullish => "MILD BULLISH",
            Self::Neutral => "NEUTRAL",
            Self::MildBearish => "MILD BEARISH",
            Self::StrongBearish => "STRONG BEARISH",
        }
    }
}

/// Single explainable feature factor contributing to a signal or composite family.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalAttributionFactor {
    pub factor_key: String,
    pub label: String,
    pub observed: f64,
    pub reference: f64,
    pub weight: f64,
    pub contribution: f64,
}

/// Quality-gated evaluation of one [`SignalCatalogEntry`] at a specific causal bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalEvaluation {
    pub catalog_id: &'static str,
    pub quality: SignalQuality,
    pub bool_value: Option<bool>,
    pub float_value: Option<f64>,
    pub enum_label: Option<String>,
    pub direction: Option<SignalDirection>,
    pub bar_index: usize,
    pub available_at_index: usize,
    pub available_at_timestamp: i64,
    pub availability_event: &'static str,
    pub is_ohlcv_proxy: bool,
    pub params_digest: u64,
    pub reasons: Vec<String>,
    pub factors: Vec<SignalAttributionFactor>,
}

impl SignalEvaluation {
    fn non_valid(
        entry: &'static SignalCatalogEntry,
        quality: SignalQuality,
        bar_index: usize,
        timestamp: i64,
        reason: &str,
    ) -> Self {
        debug_assert!(!quality.is_valid());
        Self {
            catalog_id: entry.id,
            quality,
            bool_value: None,
            float_value: None,
            enum_label: None,
            direction: None,
            bar_index,
            // Signal is evaluated at confirmation bar `bar_index` (`t = p + right_bars` for delayed pivots),
            // so its causal availability index is strictly `bar_index` (`TEST-CAUSAL-002`).
            available_at_index: bar_index,
            available_at_timestamp: timestamp,
            availability_event: SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE,
            is_ohlcv_proxy: entry.is_ohlcv_proxy,
            params_digest: entry.params_digest(),
            reasons: vec![reason.to_owned()],
            factors: Vec::new(),
        }
    }

    fn enforce_quality_contract(&mut self) {
        if !self.quality.is_valid() {
            self.bool_value = None;
            self.float_value = None;
            self.enum_label = None;
            self.direction = None;
        }
    }
}

/// Orthogonal 4-family Technical Health breakdown (`[-100.0, +100.0]` each).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalHealthBreakdown {
    pub profile: ConfluenceWeightProfile,
    /// Family 1 (`[-100, +100]`): EMA(20)/EMA(50) stack, 5-bar slope, and causal Ichimoku Kumo.
    pub trend_score: f64,
    /// Family 2 (`[-100, +100]`): RSI(14) centered bias + ATR-normalized MACD histogram.
    pub momentum_score: f64,
    /// Family 3 (`[-100, +100]`): Regime transition, Bollinger squeeze/expansion, structure shift.
    pub transition_score: f64,
    /// Family 4 (`[-100, +100]`): Blends RVOL(20) signed by CLV + ProxyBB(T20) (`bb_absent_neutral` when zero-liquidity `AT05`).
    pub participation_score: f64,
    /// Weighted composite score in `[-100.0, +100.0]`.
    pub composite_score: f64,
    pub bias: ConfluenceBias,
    pub bb_flow_absent_neutral: bool,
}

/// Complete causal confluence & per-signal explanation snapshot at `evaluated_bar_index`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalConfluenceReport {
    pub quality: SignalQuality,
    pub evaluated_bar_index: usize,
    pub evaluated_timestamp: i64,
    pub evaluated_close: f64,
    pub tail_bar_excluded: bool,
    pub data_fidelity: &'static str,
    pub regime: MarketRegime,
    pub health: Option<TechnicalHealthBreakdown>,
    pub bullish_signal_count: usize,
    pub bearish_signal_count: usize,
    pub evaluations: Vec<SignalEvaluation>,
}

/// Precomputed $O(N)$ confirmed swing pivots (`left_bars = 3, right_bars = 3`) and causal
/// `tech.swing_break` levels (`confirmed <= t - 1`).
#[derive(Debug, Clone, Default)]
struct PrecomputedPivotState {
    /// At confirmation bar `t >= 6`: `Some(true)` if candidate `p = t - 3` is a strict confirmed pivot high,
    /// `Some(false)` if valid window but not pivot, `None` if warmup or NaN/Inf in `candles[t-6..=t]`.
    confirmed_high: Vec<Option<bool>>,
    confirmed_low: Vec<Option<bool>>,
    /// When `confirmed_high[t] == Some(true)` and a prior confirmed high `p1` exists with `5 <= (t-3) - p1 <= 60`,
    /// stores `Some((p1, t - 3))`.
    high_pair: Vec<Option<(usize, usize)>>,
    low_pair: Vec<Option<(usize, usize)>>,
    /// Latest 3-bar swing high/low confirmed on or before `t - 1` (`SIG-TECH-005`).
    prior_swing_high: Vec<Option<f64>>,
    prior_swing_low: Vec<Option<f64>>,
}

impl PrecomputedPivotState {
    fn compute(candles: &[Candle]) -> Self {
        let n = candles.len();
        let mut confirmed_high = vec![None; n];
        let mut confirmed_low = vec![None; n];
        let mut high_pair = vec![None; n];
        let mut low_pair = vec![None; n];
        let mut prior_swing_high = vec![None; n];
        let mut prior_swing_low = vec![None; n];

        let mut last_confirmed_high_p: Option<usize> = None;
        let mut last_confirmed_low_p: Option<usize> = None;
        let mut running_swing_hi: Option<f64> = None;
        let mut running_swing_lo: Option<f64> = None;

        for t in 0..n {
            // 1. Update `running_swing_hi` / `running_swing_lo` from candidate `idx_pivot = t - 1 - 3 = t - 4`
            //    confirmed at `t - 1` (`technical.py:315-327`).
            if t >= 7 {
                let piv = t - 4;
                let win = &candles[piv - 3..=piv + 3];
                if win
                    .iter()
                    .all(|b| b.high.is_finite() && b.low.is_finite() && b.high >= b.low)
                {
                    let ch = candles[piv].high;
                    if win.iter().all(|b| b.high <= ch) {
                        running_swing_hi = Some(ch);
                    }
                    let cl = candles[piv].low;
                    if win.iter().all(|b| b.low >= cl) {
                        running_swing_lo = Some(cl);
                    }
                }
            }
            prior_swing_high[t] = running_swing_hi;
            prior_swing_low[t] = running_swing_lo;

            // 2. Confirmed Pivot High / Low at `t >= 6` (`p = t - 3`, `divergence.py:80-122`).
            if t >= 6 {
                let p = t - 3;
                let win = &candles[p - 3..=p + 3]; // == candles[t - 6 ..= t] (zero future lookahead!)
                if win
                    .iter()
                    .all(|b| b.high.is_finite() && b.low.is_finite() && b.high >= b.low)
                {
                    let hi_p = candles[p].high;
                    let is_hi = hi_p > candles[p - 1].high
                        && hi_p > candles[p - 2].high
                        && hi_p > candles[p - 3].high
                        && hi_p >= candles[p + 1].high
                        && hi_p >= candles[p + 2].high
                        && hi_p > candles[p + 3].high;
                    confirmed_high[t] = Some(is_hi);
                    if is_hi {
                        if let Some(p1) = last_confirmed_high_p {
                            let sep = p - p1;
                            if (5..=60).contains(&sep) {
                                high_pair[t] = Some((p1, p));
                            }
                        }
                        last_confirmed_high_p = Some(p);
                    }

                    let lo_p = candles[p].low;
                    let is_lo = lo_p < candles[p - 1].low
                        && lo_p < candles[p - 2].low
                        && lo_p < candles[p - 3].low
                        && lo_p <= candles[p + 1].low
                        && lo_p <= candles[p + 2].low
                        && lo_p < candles[p + 3].low;
                    confirmed_low[t] = Some(is_lo);
                    if is_lo {
                        if let Some(p1) = last_confirmed_low_p {
                            let sep = p - p1;
                            if (5..=60).contains(&sep) {
                                low_pair[t] = Some((p1, p));
                            }
                        }
                        last_confirmed_low_p = Some(p);
                    }
                }
            }
        }

        Self {
            confirmed_high,
            confirmed_low,
            high_pair,
            low_pair,
            prior_swing_high,
            prior_swing_low,
        }
    }
}

struct ConfluenceSeriesBundle {
    sma_20: Vec<Option<f64>>,
    ema_20: Vec<Option<f64>>,
    ema_50: Vec<Option<f64>>,
    ema_200: Vec<Option<f64>>,
    rsi_14: Vec<Option<f64>>,
    macd: MacdOutput,
    bb: BollingerOutput,
    atr_14: Vec<Option<f64>>,
    ichimoku: IchimokuOutput,
    ichi_raw_sa: Vec<Option<f64>>,
    ichi_raw_sb: Vec<Option<f64>>,
    cvd_values: Vec<Option<f64>>,
    adx_14: AdxOutput,
    stoch_14: StochasticOutput,
    supertrend_10: SuperTrendOutput,
    psar: PsarOutput,
    keltner_20: KeltnerOutput,
    cci_20: Vec<Option<f64>>,
    mfi_14: Vec<Option<f64>>,
    candle_geoms: Vec<CandleGeometry>,
    candle_patterns: Vec<CandlePatternBarSummary>,
    proxy_bb: ProxyBBSeriesOutput,
    pivots: PrecomputedPivotState,
}

impl ConfluenceSeriesBundle {
    fn compute(candles: &[Candle]) -> Self {
        let n = candles.len();
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let sma_20 = compute_sma_slices(&closes, 20);
        let ema_20 = compute_ema_slices(&closes, 20);
        let ema_50 = compute_ema_slices(&closes, 50);
        let ema_200 = compute_ema_slices(&closes, 200);
        let rsi_14 = compute_rsi_slices(&closes, 14);
        let macd = compute_macd_slices(&closes, 12, 26, 9);
        let bb = compute_bollinger_slices(&closes, 20, 2.0);
        let atr_14 = compute_atr(candles, 14);
        let ichimoku = compute_ichimoku(candles, IchimokuConfig::default());
        let ichi_raw_sb = donchian_midpoint_series(candles, 52);
        let mut ichi_raw_sa = Vec::with_capacity(n);
        for t in 0..n {
            let sa = match (ichimoku.tenkan[t], ichimoku.kijun[t]) {
                (Some(tk), Some(kj)) if tk.is_finite() && kj.is_finite() => {
                    Some(0.5 * tk + 0.5 * kj)
                }
                _ => None,
            };
            ichi_raw_sa.push(sa);
        }
        let cvd_values = compute_cvd(
            candles,
            CvdConfig {
                reset_mode: CvdResetMode::Cumulative,
                ..CvdConfig::default()
            },
        )
        .cvd;
        let adx_14 = compute_adx(candles, 14);
        let stoch_14 = compute_stochastic(candles, 14, 3, 3);
        let supertrend_10 = compute_supertrend(candles, 10, 3.0);
        let psar = compute_psar(candles, 0.02, 0.02, 0.20);
        let keltner_20 = compute_keltner_channels(candles, 20, 10, 2.0);
        let cci_20 = compute_cci(candles, 20);
        let mfi_14 = compute_mfi(candles, 14);
        let (candle_geoms, candle_patterns) =
            compute_candlestick_patterns_and_scores(candles, &atr_14);
        let proxy_bb = compute_proxy_bb_series(candles);
        let pivots = PrecomputedPivotState::compute(candles);

        Self {
            sma_20,
            ema_20,
            ema_50,
            ema_200,
            rsi_14,
            macd,
            bb,
            atr_14,
            ichimoku,
            ichi_raw_sa,
            ichi_raw_sb,
            cvd_values,
            adx_14,
            stoch_14,
            supertrend_10,
            psar,
            keltner_20,
            cci_20,
            mfi_14,
            candle_geoms,
            candle_patterns,
            proxy_bb,
            pivots,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PriorRvolCheck {
    Valid { rvol: f64, baseline: f64 },
    InsufficientHistory,
    InvalidVolume,
    ZeroBaseline,
}

fn inspect_prior_rvol_at(candles: &[Candle], t: usize, period: usize) -> PriorRvolCheck {
    if period == 0 || t < period || t >= candles.len() {
        return PriorRvolCheck::InsufficientHistory;
    }
    let curr_vol = candles[t].volume;
    if !curr_vol.is_finite() || curr_vol < 0.0 {
        return PriorRvolCheck::InvalidVolume;
    }
    let mut sum = 0.0_f64;
    for bar in &candles[t - period..t] {
        if !bar.volume.is_finite() || bar.volume < 0.0 {
            return PriorRvolCheck::InvalidVolume;
        }
        sum += bar.volume;
    }
    let baseline = sum / (period as f64);
    if !baseline.is_finite() {
        return PriorRvolCheck::InvalidVolume;
    }
    if baseline <= 0.0 {
        return PriorRvolCheck::ZeroBaseline;
    }
    let rvol = curr_vol / baseline;
    if rvol.is_finite() {
        PriorRvolCheck::Valid { rvol, baseline }
    } else {
        PriorRvolCheck::InvalidVolume
    }
}

#[inline]
fn close_location_01(c: &Candle) -> f64 {
    CandleGeometry::from_candle(c).close_location
}

#[inline]
fn signed_clv_m1_p1(c: &Candle) -> f64 {
    let g = CandleGeometry::from_candle(c);
    if !g.is_flat_range {
        (2.0 * g.close_location - 1.0).clamp(-1.0, 1.0)
    } else if c.close > c.open {
        0.5
    } else if c.close < c.open {
        -0.5
    } else {
        0.0
    }
}

#[inline]
fn bb_bandwidth_ratio_at(bb: &BollingerOutput, idx: usize) -> Option<f64> {
    let u = bb.upper.get(idx).copied().flatten()?;
    let m = bb.middle.get(idx).copied().flatten()?;
    let l = bb.lower.get(idx).copied().flatten()?;
    if !u.is_finite() || !m.is_finite() || !l.is_finite() || m.abs() <= 1.0e-12 {
        return None;
    }
    let r = (u - l) / m;
    r.is_finite().then_some(r)
}

#[inline]
fn prior_window_high_low(candles: &[Candle], t: usize, period: usize) -> Option<(f64, f64)> {
    if period == 0 || t < period || t > candles.len() {
        return None;
    }
    let mut hi = f64::NEG_INFINITY;
    let mut lo = f64::INFINITY;
    for b in &candles[t - period..t] {
        if !b.high.is_finite() || !b.low.is_finite() || b.high < b.low {
            return None;
        }
        hi = hi.max(b.high);
        lo = lo.min(b.low);
    }
    (hi.is_finite() && lo.is_finite()).then_some((hi, lo))
}

fn compute_health_core_at(
    candles: &[Candle],
    bundle: &ConfluenceSeriesBundle,
    regime: MarketRegime,
    rvol20: PriorRvolCheck,
    t: usize,
    profile: ConfluenceWeightProfile,
) -> Result<(TechnicalHealthBreakdown, Option<f64>, Option<f64>), SignalQuality> {
    if t < CONFLUENCE_WARMUP_BARS || t >= candles.len() {
        return Err(SignalQuality::InsufficientHistory);
    }
    let c = &candles[t];
    if !c.close.is_finite() || !c.high.is_finite() || !c.low.is_finite() || c.high < c.low {
        return Err(SignalQuality::InsufficientHistory);
    }
    let (Some(e20), Some(e50), Some(rsi), Some(atr)) = (
        bundle.ema_20[t],
        bundle.ema_50[t],
        bundle.rsi_14[t],
        bundle.atr_14[t],
    ) else {
        return Err(SignalQuality::InsufficientHistory);
    };
    let atr_safe = atr.max((c.close.abs() * 0.002).max(1.0e-12));

    // 1. Trend family [-1, +1]
    let stack_unit = ((e20 - e50) / (2.0 * atr_safe)).clamp(-1.0, 1.0);
    let price_vs_e20 = ((c.close - e20) / (2.0 * atr_safe)).clamp(-1.0, 1.0);
    let slope_unit = bundle.ema_20[t.saturating_sub(5)].map_or(0.0, |e_prev| {
        ((e20 - e_prev) / (1.5 * atr_safe)).clamp(-1.0, 1.0)
    });
    let kumo_unit = match (
        bundle.ichimoku.senkou_a.get(t).copied().flatten(),
        bundle.ichimoku.senkou_b.get(t).copied().flatten(),
    ) {
        (Some(sa), Some(sb)) => {
            let k_top = sa.max(sb);
            let k_bot = sa.min(sb);
            if c.close > k_top {
                0.85
            } else if c.close < k_bot {
                -0.85
            } else {
                0.0
            }
        }
        _ => stack_unit,
    };
    let trend_unit =
        (0.35 * stack_unit + 0.30 * price_vs_e20 + 0.20 * slope_unit + 0.15 * kumo_unit)
            .clamp(-1.0, 1.0);

    // 2. Momentum family [-1, +1]
    let rsi_unit = ((rsi - 50.0) / 25.0).clamp(-1.0, 1.0);
    let macd_unit =
        bundle.macd.histogram[t].map_or(0.0, |mh| (mh / (0.6 * atr_safe)).clamp(-1.0, 1.0));
    let mom_unit = (0.55 * rsi_unit + 0.45 * macd_unit).clamp(-1.0, 1.0);

    // 3. Transition / Shift family [-1, +1]
    let regime_unit: f64 = match regime {
        MarketRegime::Uptrend => 0.70,
        MarketRegime::Pullback => 0.35,
        MarketRegime::Recovery => 0.45,
        MarketRegime::SidewaysSqueeze
        | MarketRegime::Neutral
        | MarketRegime::InsufficientHistory => 0.0,
        MarketRegime::Downtrend => -0.70,
    };
    let bw_shift_unit: f64 = match (
        bb_bandwidth_ratio_at(&bundle.bb, t),
        bb_bandwidth_ratio_at(&bundle.bb, t.saturating_sub(1)),
    ) {
        (Some(curr_bw), Some(prev_bw)) if curr_bw > prev_bw => {
            if c.close >= e20 {
                0.35
            } else {
                -0.35
            }
        }
        _ => 0.0,
    };
    let shift_unit = (0.70 * regime_unit + 0.30 * bw_shift_unit).clamp(-1.0, 1.0);

    // 4. Participation / Flow family [-1, +1]:
    //    Upgraded in R18 (`B4`) to incorporate live `ProxyBB(T20)` whenever `AT05` liquidity > 0,
    //    while preserving `bb_flow_absent_neutral = true` (`"bb_absent_neutral"`, `0.0` penalty)
    //    on zero-volume / `AT05` `None` bars!
    let bb_t20_opt = bundle
        .proxy_bb
        .points
        .get(t)
        .and_then(|p| p.horizon(BBHorizon::T20).bb_value);
    let (flow_unit, bb_absent, rvol_obs) = match (rvol20, bb_t20_opt) {
        (PriorRvolCheck::InvalidVolume, _) => return Err(SignalQuality::InvalidVolume),
        (PriorRvolCheck::InsufficientHistory, _) => return Err(SignalQuality::InsufficientHistory),
        (PriorRvolCheck::ZeroBaseline, None) => (0.0, true, None),
        (PriorRvolCheck::ZeroBaseline, Some(bb20)) => {
            let bb_unit = ((bb20 - 50.0) / 50.0).clamp(-1.0, 1.0);
            ((0.45 * bb_unit).clamp(-1.0, 1.0), false, None)
        }
        (PriorRvolCheck::Valid { rvol, .. }, Some(bb20)) => {
            let amp = ((rvol - 0.8) / 1.4).clamp(0.0, 1.0);
            let rvol_unit = (amp * signed_clv_m1_p1(c)).clamp(-1.0, 1.0);
            let bb_unit = ((bb20 - 50.0) / 50.0).clamp(-1.0, 1.0);
            (
                (0.55 * rvol_unit + 0.45 * bb_unit).clamp(-1.0, 1.0),
                false,
                Some(rvol),
            )
        }
        (PriorRvolCheck::Valid { rvol, .. }, None) => {
            let amp = ((rvol - 0.8) / 1.4).clamp(0.0, 1.0);
            let rvol_unit = (amp * signed_clv_m1_p1(c)).clamp(-1.0, 1.0);
            (rvol_unit, true, Some(rvol))
        }
    };

    let (w_trend, w_mom, w_shift, w_flow) = profile.weights();
    let composite_unit =
        (w_trend * trend_unit + w_mom * mom_unit + w_shift * shift_unit + w_flow * flow_unit)
            .clamp(-1.0, 1.0);
    let composite_score = composite_unit * 100.0;
    let bias = ConfluenceBias::from_score(composite_score);

    Ok((
        TechnicalHealthBreakdown {
            profile,
            trend_score: trend_unit * 100.0,
            momentum_score: mom_unit * 100.0,
            transition_score: shift_unit * 100.0,
            participation_score: flow_unit * 100.0,
            composite_score,
            bias,
            bb_flow_absent_neutral: bb_absent,
        },
        rvol_obs,
        bb_t20_opt,
    ))
}

fn compute_health_at(
    candles: &[Candle],
    bundle: &ConfluenceSeriesBundle,
    regime: MarketRegime,
    t: usize,
    profile: ConfluenceWeightProfile,
) -> Result<
    (
        TechnicalHealthBreakdown,
        Vec<String>,
        Vec<SignalAttributionFactor>,
    ),
    SignalQuality,
> {
    let rvol20 = inspect_prior_rvol_at(candles, t, 20);
    let (breakdown, rvol_obs, bb_t20_opt) =
        compute_health_core_at(candles, bundle, regime, rvol20, t, profile)?;

    let mut reasons = vec!["OHLCV_PROXY".to_owned()];
    if breakdown.bb_flow_absent_neutral {
        reasons.push("bb_absent_neutral".to_owned());
    }
    if let Some(bb20) = bb_t20_opt {
        reasons.push(format!("PROXY_BB_T20_{bb20:.1}"));
    }
    reasons.push(format!("REGIME_{}", regime.short_label()));
    match rvol_obs {
        Some(rvol) => reasons.push(format!("RVOL20_{rvol:.2}X")),
        None => reasons.push("VOLUME_ZERO_BASELINE_NEUTRAL".to_owned()),
    }
    reasons.push(format!("BIAS_{}", breakdown.bias.label().replace(' ', "_")));

    let (w_trend, w_mom, w_shift, w_flow) = profile.weights();
    let factors = vec![
        SignalAttributionFactor {
            factor_key: "trend".to_owned(),
            label: "Trend (EMA20/50 + Kumo)".to_owned(),
            observed: breakdown.trend_score,
            reference: 0.0,
            weight: w_trend,
            contribution: w_trend * breakdown.trend_score,
        },
        SignalAttributionFactor {
            factor_key: "momentum".to_owned(),
            label: "Momentum (RSI14 + MACD)".to_owned(),
            observed: breakdown.momentum_score,
            reference: 0.0,
            weight: w_mom,
            contribution: w_mom * breakdown.momentum_score,
        },
        SignalAttributionFactor {
            factor_key: "transition".to_owned(),
            label: "Transition (Regime + BB)".to_owned(),
            observed: breakdown.transition_score,
            reference: 0.0,
            weight: w_shift,
            contribution: w_shift * breakdown.transition_score,
        },
        SignalAttributionFactor {
            factor_key: "participation".to_owned(),
            label: if breakdown.bb_flow_absent_neutral {
                "Participation (RVOL20 Proxy)".to_owned()
            } else {
                "Participation (RVOL20 + ProxyBB T20)".to_owned()
            },
            observed: breakdown.participation_score,
            reference: 0.0,
            weight: w_flow,
            contribution: w_flow * breakdown.participation_score,
        },
    ];

    Ok((breakdown, reasons, factors))
}

#[derive(Debug, Clone, Copy)]
enum DivergenceKind {
    RegularBullish,
    RegularBearish,
    HiddenBullish,
    HiddenBearish,
}

fn eval_confirmed_divergence(
    candles: &[Candle],
    pivots: &PrecomputedPivotState,
    osc: &[Option<f64>],
    kind: DivergenceKind,
    t: usize,
) -> (CompactSignalPoint, Option<(usize, usize, f64, f64)>) {
    let is_bull = matches!(
        kind,
        DivergenceKind::RegularBullish | DivergenceKind::HiddenBullish
    );
    let confirmed_flag = if is_bull {
        pivots.confirmed_low.get(t).copied().flatten()
    } else {
        pivots.confirmed_high.get(t).copied().flatten()
    };
    let Some(is_new_pivot) = confirmed_flag else {
        return (
            CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
            None,
        );
    };
    let p = t - 3;
    let Some(osc_p2) = osc.get(p).copied().flatten().filter(|v| v.is_finite()) else {
        return (
            CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
            None,
        );
    };
    if !is_new_pivot {
        return (
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(false),
                float_value: Some(osc_p2),
                direction: None,
            },
            None,
        );
    }

    let pair = if is_bull {
        pivots.low_pair.get(t).copied().flatten()
    } else {
        pivots.high_pair.get(t).copied().flatten()
    };
    let mut fired = false;
    let mut detail = None;
    if let Some((p1, p2)) = pair
        && let Some(osc_p1) = osc.get(p1).copied().flatten().filter(|v| v.is_finite())
    {
        let (price1, price2) = if is_bull {
            (candles[p1].low, candles[p2].low)
        } else {
            (candles[p1].high, candles[p2].high)
        };
        fired = match kind {
            DivergenceKind::RegularBullish => price2 < price1 && osc_p2 > osc_p1,
            DivergenceKind::HiddenBullish => price2 > price1 && osc_p2 < osc_p1,
            DivergenceKind::RegularBearish => price2 > price1 && osc_p2 < osc_p1,
            DivergenceKind::HiddenBearish => price2 < price1 && osc_p2 > osc_p1,
        };
        if fired {
            detail = Some((p1, p2, osc_p1, osc_p2));
        }
    }

    let dir = if fired {
        Some(if is_bull {
            SignalDirection::Bullish
        } else {
            SignalDirection::Bearish
        })
    } else {
        None
    };
    (
        CompactSignalPoint {
            quality: SignalQuality::Valid,
            bool_value: Some(fired),
            float_value: Some(osc_p2),
            direction: dir,
        },
        detail,
    )
}

fn eval_sr_proximity(
    candles: &[Candle],
    bundle: &ConfluenceSeriesBundle,
    t: usize,
    is_support: bool,
) -> Option<(bool, f64, [bool; 4])> {
    let atr = bundle.atr_14.get(t).copied().flatten()?;
    let c = &candles[t];
    let atr_floor = 1.0e-12 * (c.high.abs() + c.low.abs()).max(1.0e-30);
    if !atr.is_finite() || atr <= atr_floor {
        return None;
    }
    let (prior_hi, prior_lo) = prior_window_high_low(candles, t, 20)?;
    let tol = 0.5 * atr;
    let mut matched = [false; 4]; // [rolling_20, ema20, ema50, ema200]

    if is_support {
        if (c.close - prior_lo).abs() <= tol || (c.low - prior_lo).abs() <= tol {
            matched[0] = true;
        }
        if let Some(e20) = bundle.ema_20[t]
            && c.close >= e20
            && (c.close - e20) <= tol
        {
            matched[1] = true;
        }
        if let Some(e50) = bundle.ema_50[t]
            && c.close >= e50
            && (c.close - e50) <= tol
        {
            matched[2] = true;
        }
        if let Some(e200) = bundle.ema_200[t]
            && c.close >= e200
            && (c.close - e200) <= tol
        {
            matched[3] = true;
        }
        let any = matched.iter().any(|&m| m);
        Some((any, prior_lo, matched))
    } else {
        if (c.close - prior_hi).abs() <= tol || (c.high - prior_hi).abs() <= tol {
            matched[0] = true;
        }
        if let Some(e20) = bundle.ema_20[t]
            && c.close <= e20
            && (e20 - c.close) <= tol
        {
            matched[1] = true;
        }
        if let Some(e50) = bundle.ema_50[t]
            && c.close <= e50
            && (e50 - c.close) <= tol
        {
            matched[2] = true;
        }
        if let Some(e200) = bundle.ema_200[t]
            && c.close <= e200
            && (e200 - c.close) <= tol
        {
            matched[3] = true;
        }
        let any = matched.iter().any(|&m| m);
        Some((any, prior_hi, matched))
    }
}

fn eval_ichimoku_4factor_score(
    candles: &[Candle],
    bundle: &ConfluenceSeriesBundle,
    t: usize,
) -> Option<(f64, i32, i32)> {
    if t < 26 {
        return None;
    }
    let tk = bundle.ichimoku.tenkan.get(t).copied().flatten()?;
    let kj = bundle.ichimoku.kijun.get(t).copied().flatten()?;
    let vis_sa = bundle.ichimoku.senkou_a.get(t).copied().flatten()?;
    let vis_sb = bundle.ichimoku.senkou_b.get(t).copied().flatten()?;
    let raw_sa = bundle.ichi_raw_sa.get(t).copied().flatten()?;
    let raw_sb = bundle.ichi_raw_sb.get(t).copied().flatten()?;
    let past_close = candles[t - 26].close;
    if !past_close.is_finite() {
        return None;
    }
    let c = &candles[t];
    let top = vis_sa.max(vis_sb);
    let bot = vis_sa.min(vis_sb);

    let mut bull = 0_i32;
    let mut bear = 0_i32;
    if c.close > top {
        bull += 1;
    } else if c.close < bot {
        bear += 1;
    }
    if tk > kj {
        bull += 1;
    } else if tk < kj {
        bear += 1;
    }
    if c.close > past_close {
        bull += 1;
    } else if c.close < past_close {
        bear += 1;
    }
    if raw_sa > raw_sb {
        bull += 1;
    } else if raw_sa < raw_sb {
        bear += 1;
    }
    let score = 25.0 * f64::from(bull - bear);
    Some((score, bull, bear))
}

#[inline]
fn health_derived_compact_point(
    entry_id: &str,
    health_res: Result<f64, SignalQuality>,
) -> Option<CompactSignalPoint> {
    match entry_id {
        "health.technical_composite" | "health.score" => Some(match health_res {
            Ok(score) => CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: None,
                float_value: Some(score),
                direction: if score >= 12.0 {
                    Some(SignalDirection::Bullish)
                } else if score <= -12.0 {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            },
            Err(q) => CompactSignalPoint::non_valid(q),
        }),
        "health.favorable" => Some(match health_res {
            Ok(score) => {
                let fired = score >= 35.0;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(score),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            Err(q) => CompactSignalPoint::non_valid(q),
        }),
        "health.unfavorable" => Some(match health_res {
            Ok(score) => {
                let fired = score <= -35.0;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(score),
                    direction: fired.then_some(SignalDirection::Bearish),
                }
            }
            Err(q) => CompactSignalPoint::non_valid(q),
        }),
        _ => None,
    }
}

#[inline]
fn is_profile_dependent_health_entry(entry_id: &str) -> bool {
    matches!(
        entry_id,
        "health.technical_composite" | "health.score" | "health.favorable" | "health.unfavorable"
    )
}

/// Single-source-of-truth compact mathematical evaluator for all `101` [`SIGNAL_CATALOG`] entries.
fn evaluate_compact_entry_at(
    entry: &'static SignalCatalogEntry,
    candles: &[Candle],
    bundle: &ConfluenceSeriesBundle,
    regime: MarketRegime,
    rvol20: PriorRvolCheck,
    health_score: Result<f64, SignalQuality>,
    t: usize,
) -> CompactSignalPoint {
    if t < entry.warmup_bars || t >= candles.len() {
        return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
    }
    let c = &candles[t];
    if !c.open.is_finite()
        || !c.high.is_finite()
        || !c.low.is_finite()
        || !c.close.is_finite()
        || c.high < c.low
    {
        return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
    }

    if let Some(hp) = health_derived_compact_point(entry.id, health_score) {
        return hp;
    }

    let atr_floor = 1.0e-12 * (c.high.abs() + c.low.abs()).max(1.0e-30);

    match entry.id {
        "regime.market_state" => {
            if regime == MarketRegime::InsufficientHistory {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let dir = match regime {
                MarketRegime::Uptrend | MarketRegime::Pullback | MarketRegime::Recovery => {
                    Some(SignalDirection::Bullish)
                }
                MarketRegime::Downtrend => Some(SignalDirection::Bearish),
                MarketRegime::SidewaysSqueeze
                | MarketRegime::Neutral
                | MarketRegime::InsufficientHistory => None,
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: None,
                float_value: None,
                direction: dir,
            }
        }
        "technical.rsi_oversold_cross" => {
            let (Some(prev), Some(curr)) = (bundle.rsi_14[t - 1], bundle.rsi_14[t]) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = prev <= 30.0 && curr > 30.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(curr),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "technical.rsi_overbought_cross" => {
            let (Some(prev), Some(curr)) = (bundle.rsi_14[t - 1], bundle.rsi_14[t]) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = prev >= 70.0 && curr < 70.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(curr),
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        "technical.macd_bull_cross" | "technical.macd_bear_cross" => {
            let is_bull = entry.id == "technical.macd_bull_cross";
            let (Some(prev_h), Some(curr_h)) =
                (bundle.macd.histogram[t - 1], bundle.macd.histogram[t])
            else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = if is_bull {
                prev_h <= 0.0 && curr_h > 0.0
            } else {
                prev_h >= 0.0 && curr_h < 0.0
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(curr_h),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "technical.sma_ema_golden_cross" | "technical.sma_ema_death_cross" => {
            let is_gold = entry.id == "technical.sma_ema_golden_cross";
            let (Some(sp), Some(ep), Some(sc), Some(ec)) = (
                bundle.sma_20[t - 1],
                bundle.ema_50[t - 1],
                bundle.sma_20[t],
                bundle.ema_50[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let prev_d = sp - ep;
            let curr_d = sc - ec;
            let fired = if is_gold {
                prev_d <= 0.0 && curr_d > 0.0
            } else {
                prev_d >= 0.0 && curr_d < 0.0
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(curr_d),
                direction: fired.then_some(if is_gold {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "pattern.bollinger_squeeze_breakout" => {
            let (Some(prev_bw), Some(prev_up), Some(curr_up)) = (
                bb_bandwidth_ratio_at(&bundle.bb, t - 1),
                bundle.bb.upper[t - 1],
                bundle.bb.upper[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            if !candles[t - 1].close.is_finite() {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let fired = prev_bw <= 0.10 && candles[t - 1].close <= prev_up && c.close > curr_up;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(prev_bw * 100.0),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "volume.spike_2x" => match rvol20 {
            PriorRvolCheck::Valid { rvol, .. } => {
                let fired = rvol >= 2.0 && c.close >= c.open;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            PriorRvolCheck::InsufficientHistory => {
                CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory)
            }
            PriorRvolCheck::InvalidVolume => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            PriorRvolCheck::ZeroBaseline => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
        },
        "volume.relative_volume" => match rvol20 {
            PriorRvolCheck::Valid { rvol, .. } => CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: None,
                float_value: Some(rvol),
                direction: if rvol >= 1.5 {
                    Some(if c.close >= c.open {
                        SignalDirection::Bullish
                    } else {
                        SignalDirection::Bearish
                    })
                } else {
                    None
                },
            },
            PriorRvolCheck::InsufficientHistory => {
                CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory)
            }
            PriorRvolCheck::InvalidVolume => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            PriorRvolCheck::ZeroBaseline => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
        },
        "volume.spike" => match rvol20 {
            PriorRvolCheck::Valid { rvol, .. } => {
                let fired = rvol >= 2.0;
                let dir = fired.then_some(if c.close >= c.open {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                });
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: dir,
                }
            }
            PriorRvolCheck::InsufficientHistory => {
                CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory)
            }
            PriorRvolCheck::InvalidVolume => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            PriorRvolCheck::ZeroBaseline => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
        },
        "volume.climax_up" | "volume.climax_down" => match (rvol20, bundle.atr_14[t]) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr)) if atr > atr_floor => {
                let g = bundle.candle_geoms[t];
                let range_atr = g.range / atr;
                let is_up = entry.id == "volume.climax_up";
                let fired = if is_up {
                    rvol >= 2.0 && range_atr >= 1.5 && g.close_location >= 0.75
                } else {
                    rvol >= 2.0 && range_atr >= 1.5 && g.close_location <= 0.25
                };
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(if is_up {
                        SignalDirection::Bullish
                    } else {
                        SignalDirection::Bearish
                    }),
                }
            }
            (PriorRvolCheck::InvalidVolume, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.climax_up" => match (rvol20, bundle.atr_14[t]) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr)) if atr > atr_floor => {
                let range_atr = (c.high - c.low).max(0.0) / atr;
                let cloc = close_location_01(c);
                let fired = rvol >= 2.0 && range_atr >= 1.35 && cloc >= 0.65;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.stopping_volume" => match rvol20 {
            PriorRvolCheck::Valid { rvol, .. } => {
                let Some((_, prior_low)) = prior_window_high_low(candles, t, 10) else {
                    return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
                };
                let cloc = close_location_01(c);
                let fired = c.low <= prior_low && rvol >= 1.8 && cloc >= 0.60;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(cloc),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            PriorRvolCheck::InvalidVolume => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            PriorRvolCheck::ZeroBaseline => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            PriorRvolCheck::InsufficientHistory => {
                CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory)
            }
        },
        "vsa.strong_demand" => match (rvol20, bundle.atr_14[t]) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr))
                if atr > atr_floor && candles[t - 1].close.is_finite() =>
            {
                let g = bundle.candle_geoms[t];
                let range_atr = g.range / atr;
                let fired = c.close > candles[t - 1].close
                    && range_atr >= 1.2
                    && g.body_ratio >= 0.50
                    && g.close_location >= 0.70
                    && rvol >= 1.5;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.weak_demand" => match (rvol20, bundle.atr_14[t]) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr))
                if atr > atr_floor && candles[t - 1].close.is_finite() =>
            {
                let g = bundle.candle_geoms[t];
                let range_atr = g.range / atr;
                let fired = c.close >= candles[t - 1].close && range_atr <= 0.70 && rvol <= 0.80;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bearish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.upthrust" => match (
            rvol20,
            bundle.atr_14[t],
            prior_window_high_low(candles, t, 20),
        ) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr), Some((prior_res, _)))
                if atr > atr_floor =>
            {
                let g = bundle.candle_geoms[t];
                let fired = c.high > prior_res
                    && c.close < prior_res
                    && g.upper_wick_ratio >= 0.35
                    && g.close_location <= 0.45
                    && rvol >= 1.5;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bearish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.spring" => match (
            rvol20,
            bundle.atr_14[t],
            prior_window_high_low(candles, t, 20),
        ) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr), Some((_, prior_sup)))
                if atr > atr_floor =>
            {
                let g = bundle.candle_geoms[t];
                let fired = c.low < prior_sup
                    && c.close > prior_sup
                    && g.lower_wick_ratio >= 0.35
                    && g.close_location >= 0.70
                    && rvol >= 1.30;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.strong_demand_at_support" => match (
            rvol20,
            bundle.atr_14[t],
            eval_sr_proximity(candles, bundle, t, true),
            prior_window_high_low(candles, t, 20),
        ) {
            (
                PriorRvolCheck::Valid { rvol, .. },
                Some(atr),
                Some((near_sup, _, _)),
                Some((_, prior_sup)),
            ) if atr > atr_floor && candles[t - 1].close.is_finite() => {
                let g = bundle.candle_geoms[t];
                let is_dem = c.close > candles[t - 1].close
                    && (g.range / atr) >= 1.2
                    && g.body_ratio >= 0.50
                    && g.close_location >= 0.70
                    && rvol >= 1.30;
                let is_spr = c.low < prior_sup
                    && c.close > prior_sup
                    && g.lower_wick_ratio >= 0.35
                    && g.close_location >= 0.70
                    && rvol >= 1.30;
                let fired = near_sup && (is_dem || is_spr);
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bullish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        "vsa.strong_supply_at_resistance" => match (
            rvol20,
            bundle.atr_14[t],
            eval_sr_proximity(candles, bundle, t, false),
            prior_window_high_low(candles, t, 20),
        ) {
            (
                PriorRvolCheck::Valid { rvol, .. },
                Some(atr),
                Some((near_res, _, _)),
                Some((prior_res, _)),
            ) if atr > atr_floor && candles[t - 1].close.is_finite() => {
                let g = bundle.candle_geoms[t];
                let is_up = c.high > prior_res
                    && c.close < prior_res
                    && g.upper_wick_ratio >= 0.35
                    && g.close_location <= 0.45
                    && rvol >= 1.30;
                let is_rej = rvol >= 1.30
                    && c.close < candles[t - 1].close
                    && g.upper_wick_ratio >= 0.30
                    && g.close_location <= 0.40;
                let fired = near_res && (is_up || is_rej);
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: Some(fired),
                    float_value: Some(rvol),
                    direction: fired.then_some(SignalDirection::Bearish),
                }
            }
            (PriorRvolCheck::InvalidVolume, _, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            (PriorRvolCheck::ZeroBaseline, _, _, _) => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
        },
        // =====================================================================
        // B1 Candlestick Patterns (`14`) & Candle Structure Score (`1`)
        // =====================================================================
        "pattern.bullish_engulfing"
        | "pattern.bearish_engulfing"
        | "pattern.hammer"
        | "pattern.shooting_star"
        | "pattern.morning_star"
        | "pattern.evening_star"
        | "pattern.piercing_line"
        | "pattern.dark_cloud_cover"
        | "pattern.tweezer_bottom"
        | "pattern.tweezer_top"
        | "pattern.inside_bar_breakout_up"
        | "pattern.inside_bar_breakout_down" => {
            let kind = CandlestickPatternKind::ALL
                .into_iter()
                .find(|k| k.catalog_id() == entry.id)
                .expect("known candlestick pattern");
            let sum = bundle.candle_patterns[t];
            let Some(fired) = sum.pattern_result(kind) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: sum.candle_score,
                direction: fired.then_some(if kind.is_bullish() {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "pattern.any_bullish" => {
            let sum = bundle.candle_patterns[t];
            let Some(fired) = sum.any_bullish() else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: sum.candle_score,
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "pattern.any_bearish" => {
            let sum = bundle.candle_patterns[t];
            let Some(fired) = sum.any_bearish() else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: sum.candle_score,
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        "structure.candle_score" => {
            let Some(score) = bundle.candle_patterns[t].candle_score else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: None,
                float_value: Some(score),
                direction: if score >= 25.0 {
                    Some(SignalDirection::Bullish)
                } else if score <= -25.0 {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        // =====================================================================
        // Legacy `tech.*` Bidirectional Triggers (`6`)
        // =====================================================================
        "tech.macd_signal_cross" => {
            let (Some(prev_h), Some(curr_h)) =
                (bundle.macd.histogram[t - 1], bundle.macd.histogram[t])
            else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let up = prev_h <= 0.0 && curr_h > 0.0;
            let dn = prev_h >= 0.0 && curr_h < 0.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up || dn),
                float_value: Some(curr_h),
                direction: if up {
                    Some(SignalDirection::Bullish)
                } else if dn {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "tech.macd_zero_cross" => {
            let (Some(prev_m), Some(curr_m)) =
                (bundle.macd.macd_line[t - 1], bundle.macd.macd_line[t])
            else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let up = prev_m <= 0.0 && curr_m > 0.0;
            let dn = prev_m >= 0.0 && curr_m < 0.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up || dn),
                float_value: Some(curr_m),
                direction: if up {
                    Some(SignalDirection::Bullish)
                } else if dn {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "tech.rsi_level_cross" => {
            let (Some(prev_r), Some(curr_r)) = (bundle.rsi_14[t - 1], bundle.rsi_14[t]) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let up = prev_r <= 30.0 && curr_r > 30.0;
            let dn = prev_r >= 70.0 && curr_r < 70.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up || dn),
                float_value: Some(curr_r),
                direction: if up {
                    Some(SignalDirection::Bullish)
                } else if dn {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "tech.ema_cross" => {
            let (Some(f0), Some(s0), Some(f1), Some(s1)) = (
                bundle.ema_20[t - 1],
                bundle.ema_50[t - 1],
                bundle.ema_20[t],
                bundle.ema_50[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let prev_d = f0 - s0;
            let curr_d = f1 - s1;
            let up = prev_d <= 0.0 && curr_d > 0.0;
            let dn = prev_d >= 0.0 && curr_d < 0.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up || dn),
                float_value: Some(curr_d),
                direction: if up {
                    Some(SignalDirection::Bullish)
                } else if dn {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "tech.swing_break" => {
            let Some(atr) = bundle.atr_14[t].filter(|&a| a.is_finite() && a > atr_floor) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let sh = bundle.pivots.prior_swing_high[t];
            let sl = bundle.pivots.prior_swing_low[t];
            if sh.is_none() && sl.is_none() {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let up = sh.is_some_and(|h| c.close > h + 0.05 * atr);
            let dn = !up && sl.is_some_and(|l| c.close < l - 0.05 * atr);
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up || dn),
                float_value: sh.or(sl),
                direction: if up {
                    Some(SignalDirection::Bullish)
                } else if dn {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "tech.composite" => {
            let mut bull_c = 0_u8;
            let mut bear_c = 0_u8;
            if let (Some(p), Some(cur)) = (bundle.macd.histogram[t - 1], bundle.macd.histogram[t]) {
                if p <= 0.0 && cur > 0.0 {
                    bull_c += 1;
                } else if p >= 0.0 && cur < 0.0 {
                    bear_c += 1;
                }
            }
            if let (Some(p), Some(cur)) = (bundle.macd.macd_line[t - 1], bundle.macd.macd_line[t]) {
                if p <= 0.0 && cur > 0.0 {
                    bull_c += 1;
                } else if p >= 0.0 && cur < 0.0 {
                    bear_c += 1;
                }
            }
            if let (Some(p), Some(cur)) = (bundle.rsi_14[t - 1], bundle.rsi_14[t]) {
                if p <= 50.0 && cur > 50.0 {
                    bull_c += 1;
                } else if p >= 50.0 && cur < 50.0 {
                    bear_c += 1;
                }
            }
            if let (Some(f0), Some(s0), Some(f1), Some(s1)) = (
                bundle.ema_20[t - 1],
                bundle.ema_50[t - 1],
                bundle.ema_20[t],
                bundle.ema_50[t],
            ) {
                if f0 <= s0 && f1 > s1 {
                    bull_c += 1;
                } else if f0 >= s0 && f1 < s1 {
                    bear_c += 1;
                }
            }
            if let Some(atr) = bundle.atr_14[t].filter(|&a| a.is_finite() && a > atr_floor) {
                if bundle.pivots.prior_swing_high[t].is_some_and(|h| c.close > h + 0.05 * atr) {
                    bull_c += 1;
                } else if bundle.pivots.prior_swing_low[t].is_some_and(|l| c.close < l - 0.05 * atr)
                {
                    bear_c += 1;
                }
            }
            let up = bull_c >= 2 && bull_c >= bear_c;
            let dn = !up && bear_c >= 2;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up || dn),
                float_value: Some(f64::from(bull_c.max(bear_c))),
                direction: if up {
                    Some(SignalDirection::Bullish)
                } else if dn {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        // =====================================================================
        // Legacy `regime.*` Signals (`7`)
        // =====================================================================
        "regime.uptrend" => {
            let (Some(ef), Some(es), Some(ef_prev)) = (
                bundle.ema_20[t],
                bundle.ema_50[t],
                bundle.ema_20[t.saturating_sub(5)],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = c.close > ef && ef > es && ef > ef_prev;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(ef - es),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "regime.downtrend" => {
            let (Some(ef), Some(es), Some(ef_prev)) = (
                bundle.ema_20[t],
                bundle.ema_50[t],
                bundle.ema_20[t.saturating_sub(5)],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = c.close < ef && ef < es && ef < ef_prev;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(ef - es),
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        "regime.sideways" => {
            let (Some(ef), Some(ef_prev)) = (bundle.ema_20[t], bundle.ema_20[t.saturating_sub(5)])
            else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            if ef_prev.abs() <= 1.0e-12 {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let rel_slope = (ef - ef_prev).abs() / ef_prev.abs();
            let fired = rel_slope < 0.008;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(rel_slope * 100.0),
                direction: None,
            }
        }
        "regime.pullback" => {
            if regime == MarketRegime::InsufficientHistory {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let fired = regime == MarketRegime::Pullback;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: None,
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "regime.recovery" => {
            if regime == MarketRegime::InsufficientHistory {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let fired = regime == MarketRegime::Recovery;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: None,
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "regime.new_high" => {
            let Some((prior_hi, _)) = prior_window_high_low(candles, t, 20) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = c.high > prior_hi;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(prior_hi),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "regime.new_low" => {
            let Some((_, prior_lo)) = prior_window_high_low(candles, t, 20) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = c.low < prior_lo;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(prior_lo),
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        // =====================================================================
        // Support & Resistance (`sr.prior_20b_breakout`, `sr.near_support`, `sr.near_resistance`)
        // =====================================================================
        "sr.near_support" => {
            let Some((fired, ref_lvl, _)) = eval_sr_proximity(candles, bundle, t, true) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(ref_lvl),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "sr.near_resistance" => {
            let Some((fired, ref_lvl, _)) = eval_sr_proximity(candles, bundle, t, false) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(ref_lvl),
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        "sr.prior_20b_breakout" => {
            let Some((prior_hi, prior_lo)) = prior_window_high_low(candles, t, 20) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let up_brk = c.close > prior_hi;
            let dn_brk = c.close < prior_lo;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(up_brk || dn_brk),
                float_value: Some(if up_brk { prior_hi } else { prior_lo }),
                direction: if up_brk {
                    Some(SignalDirection::Bullish)
                } else if dn_brk {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "structure.swing_bos" => {
            let Some(e20) = bundle.ema_20[t] else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let Some((prior_hi, prior_lo)) = prior_window_high_low(candles, t, 10) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let bull_bos = c.close > prior_hi && c.close > e20;
            let bear_bos = c.close < prior_lo && c.close < e20;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(bull_bos || bear_bos),
                float_value: Some(c.close - e20),
                direction: if bull_bos {
                    Some(SignalDirection::Bullish)
                } else if bear_bos {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        // =====================================================================
        // Ichimoku Suite (`1` R15 + `7` B3)
        // =====================================================================
        "ichimoku.tk_kumo_alignment" => {
            let (Some(tk), Some(kj), Some(sa), Some(sb)) = (
                bundle.ichimoku.tenkan.get(t).copied().flatten(),
                bundle.ichimoku.kijun.get(t).copied().flatten(),
                bundle.ichimoku.senkou_a.get(t).copied().flatten(),
                bundle.ichimoku.senkou_b.get(t).copied().flatten(),
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let k_top = sa.max(sb);
            let k_bot = sa.min(sb);
            let mut pts = 0.0_f64;
            if c.close > k_top {
                pts += 50.0;
            } else if c.close < k_bot {
                pts -= 50.0;
            }
            if tk > kj {
                pts += 30.0;
            } else if tk < kj {
                pts -= 30.0;
            }
            if sa > sb {
                pts += 20.0;
            } else if sa < sb {
                pts -= 20.0;
            }
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: None,
                float_value: Some(pts),
                direction: if pts >= 40.0 {
                    Some(SignalDirection::Bullish)
                } else if pts <= -40.0 {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "ichimoku.score" => {
            let Some((score, _, _)) = eval_ichimoku_4factor_score(candles, bundle, t) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: None,
                float_value: Some(score),
                direction: if score >= 50.0 {
                    Some(SignalDirection::Bullish)
                } else if score <= -50.0 {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                },
            }
        }
        "ichimoku.bullish" => {
            let Some((score, _, _)) = eval_ichimoku_4factor_score(candles, bundle, t) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = score >= 50.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(score),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "ichimoku.bearish" => {
            let Some((score, _, _)) = eval_ichimoku_4factor_score(candles, bundle, t) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = score <= -50.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(score),
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        "ichimoku.tk_cross_bullish" | "ichimoku.tk_cross_bearish" => {
            let (Some(tk0), Some(kj0), Some(tk1), Some(kj1)) = (
                bundle.ichimoku.tenkan[t - 1],
                bundle.ichimoku.kijun[t - 1],
                bundle.ichimoku.tenkan[t],
                bundle.ichimoku.kijun[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_bull = entry.id == "ichimoku.tk_cross_bullish";
            let fired = if is_bull {
                tk0 <= kj0 && tk1 > kj1
            } else {
                tk0 >= kj0 && tk1 < kj1
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(tk1 - kj1),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "ichimoku.kumo_breakout_bullish" | "ichimoku.kumo_breakout_bearish" => {
            let (Some(sa0), Some(sb0), Some(sa1), Some(sb1)) = (
                bundle.ichimoku.senkou_a.get(t - 1).copied().flatten(),
                bundle.ichimoku.senkou_b.get(t - 1).copied().flatten(),
                bundle.ichimoku.senkou_a.get(t).copied().flatten(),
                bundle.ichimoku.senkou_b.get(t).copied().flatten(),
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let prev_c = candles[t - 1].close;
            if !prev_c.is_finite() {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let is_bull = entry.id == "ichimoku.kumo_breakout_bullish";
            let (fired, edge) = if is_bull {
                let top0 = sa0.max(sb0);
                let top1 = sa1.max(sb1);
                (prev_c <= top0 && c.close > top1, top1)
            } else {
                let bot0 = sa0.min(sb0);
                let bot1 = sa1.min(sb1);
                (prev_c >= bot0 && c.close < bot1, bot1)
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(edge),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        // =====================================================================
        // Divergences (`1` R15 + `14` B3 Confirmed-Pivot Divergences)
        // =====================================================================
        "divergence.rsi_hidden_or_classic" => {
            let Some(curr_rsi) = bundle.rsi_14[t] else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let mut min_low_idx: Option<usize> = None;
            let mut max_high_idx: Option<usize> = None;
            for idx in (t - 10)..t {
                let b = &candles[idx];
                if !b.high.is_finite() || !b.low.is_finite() || b.high < b.low {
                    return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
                }
                if min_low_idx.is_none_or(|m| b.low < candles[m].low) {
                    min_low_idx = Some(idx);
                }
                if max_high_idx.is_none_or(|m| b.high > candles[m].high) {
                    max_high_idx = Some(idx);
                }
            }
            let (Some(lo_idx), Some(hi_idx)) = (min_low_idx, max_high_idx) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let bull_div = bundle.rsi_14[lo_idx].is_some_and(|prev_rsi| {
                c.low < candles[lo_idx].low && curr_rsi > prev_rsi + 2.0 && curr_rsi < 48.0
            });
            let bear_div = bundle.rsi_14[hi_idx].is_some_and(|prev_rsi| {
                c.high > candles[hi_idx].high && curr_rsi < prev_rsi - 2.0 && curr_rsi > 52.0
            });
            let fired = bull_div || bear_div;
            let dir = if bull_div {
                Some(SignalDirection::Bullish)
            } else if bear_div {
                Some(SignalDirection::Bearish)
            } else {
                None
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(curr_rsi),
                direction: dir,
            }
        }
        "divergence.confirmed_pivot_high" => {
            let Some(fired) = bundle.pivots.confirmed_high[t] else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(candles[t - 3].high),
                direction: fired.then_some(SignalDirection::Bearish),
            }
        }
        "divergence.confirmed_pivot_low" => {
            let Some(fired) = bundle.pivots.confirmed_low[t] else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(candles[t - 3].low),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "divergence.rsi_regular_bullish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.rsi_14,
                DivergenceKind::RegularBullish,
                t,
            )
            .0
        }
        "divergence.rsi_regular_bearish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.rsi_14,
                DivergenceKind::RegularBearish,
                t,
            )
            .0
        }
        "divergence.rsi_hidden_bullish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.rsi_14,
                DivergenceKind::HiddenBullish,
                t,
            )
            .0
        }
        "divergence.rsi_hidden_bearish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.rsi_14,
                DivergenceKind::HiddenBearish,
                t,
            )
            .0
        }
        "divergence.macd_regular_bullish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.macd.histogram,
                DivergenceKind::RegularBullish,
                t,
            )
            .0
        }
        "divergence.macd_regular_bearish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.macd.histogram,
                DivergenceKind::RegularBearish,
                t,
            )
            .0
        }
        "divergence.macd_hidden_bullish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.macd.histogram,
                DivergenceKind::HiddenBullish,
                t,
            )
            .0
        }
        "divergence.macd_hidden_bearish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.macd.histogram,
                DivergenceKind::HiddenBearish,
                t,
            )
            .0
        }
        "divergence.stoch_regular_bullish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.stoch_14.fast_k,
                DivergenceKind::RegularBullish,
                t,
            )
            .0
        }
        "divergence.stoch_regular_bearish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.stoch_14.fast_k,
                DivergenceKind::RegularBearish,
                t,
            )
            .0
        }
        "divergence.stoch_hidden_bullish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.stoch_14.fast_k,
                DivergenceKind::HiddenBullish,
                t,
            )
            .0
        }
        "divergence.stoch_hidden_bearish" => {
            eval_confirmed_divergence(
                candles,
                &bundle.pivots,
                &bundle.stoch_14.fast_k,
                DivergenceKind::HiddenBearish,
                t,
            )
            .0
        }
        // =====================================================================
        // Flow (`flow.cvd_ohlcv_impulse` + `8` B4 `bb.*` Money Flow ProxyBB)
        // =====================================================================
        "flow.cvd_ohlcv_impulse" => match rvol20 {
            PriorRvolCheck::Valid { baseline, .. } => {
                let (Some(cvd_now), Some(cvd_prev5)) = (
                    bundle.cvd_values.get(t).copied().flatten(),
                    bundle
                        .cvd_values
                        .get(t.saturating_sub(5))
                        .copied()
                        .flatten(),
                ) else {
                    return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
                };
                let norm_impulse =
                    ((cvd_now - cvd_prev5) / (5.0 * baseline)).clamp(-5.0, 5.0) * 20.0;
                CompactSignalPoint {
                    quality: SignalQuality::Valid,
                    bool_value: None,
                    float_value: Some(norm_impulse),
                    direction: if norm_impulse >= 20.0 {
                        Some(SignalDirection::Bullish)
                    } else if norm_impulse <= -20.0 {
                        Some(SignalDirection::Bearish)
                    } else {
                        None
                    },
                }
            }
            PriorRvolCheck::InvalidVolume => {
                CompactSignalPoint::non_valid(SignalQuality::InvalidVolume)
            }
            PriorRvolCheck::ZeroBaseline => {
                CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline)
            }
            PriorRvolCheck::InsufficientHistory => {
                CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory)
            }
        },
        "bb.direction_rising"
        | "bb.direction_falling"
        | "bb.regime_positive"
        | "bb.regime_negative" => {
            let h20 = bundle.proxy_bb.points[t].horizon(BBHorizon::T20);
            if h20.is_invalid {
                return CompactSignalPoint::non_valid(SignalQuality::InvalidVolume);
            }
            if h20.is_warmup {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            if h20.is_zero_activity {
                return CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline);
            }
            let Some(val) = h20.bb_value else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let (fired, is_bull) = match entry.id {
                "bb.direction_rising" => (h20.direction == BBDirection::Rising, true),
                "bb.direction_falling" => (h20.direction == BBDirection::Falling, false),
                "bb.regime_positive" => (h20.regime == BBRegime::Positive, true),
                _ => (h20.regime == BBRegime::Negative, false),
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(val),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "bb.confluence_bullish" | "bb.confluence_bearish" => {
            let pt = &bundle.proxy_bb.points[t];
            let h05 = pt.horizon(BBHorizon::T05);
            let h20 = pt.horizon(BBHorizon::T20);
            if h05.is_invalid || h20.is_invalid {
                return CompactSignalPoint::non_valid(SignalQuality::InvalidVolume);
            }
            if h05.is_warmup || h20.is_warmup {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            if h05.is_zero_activity || h20.is_zero_activity {
                return CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline);
            }
            let (Some(v05), Some(v20)) = (h05.bb_value, h20.bb_value) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_bull = entry.id == "bb.confluence_bullish";
            let fired = if is_bull {
                h05.regime == BBRegime::Positive
                    && h20.regime == BBRegime::Positive
                    && (h05.direction == BBDirection::Rising || v05 >= v20)
            } else {
                h05.regime == BBRegime::Negative
                    && h20.regime == BBRegime::Negative
                    && (h05.direction == BBDirection::Falling || v05 <= v20)
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(v20),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "bb.turn_up" | "bb.turn_down" => {
            let h20 = bundle.proxy_bb.points[t].horizon(BBHorizon::T20);
            let h20_prev = bundle.proxy_bb.points[t - 1].horizon(BBHorizon::T20);
            if h20.is_invalid || h20_prev.is_invalid {
                return CompactSignalPoint::non_valid(SignalQuality::InvalidVolume);
            }
            if h20.is_warmup || h20_prev.is_warmup {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            if h20.is_zero_activity || h20_prev.is_zero_activity {
                return CompactSignalPoint::non_valid(SignalQuality::ZeroBaseline);
            }
            let (Some(curr_v), Some(prev_v)) = (h20.bb_value, h20_prev.bb_value) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_up = entry.id == "bb.turn_up";
            let fired = if is_up {
                prev_v <= 42.0 && curr_v > prev_v && curr_v >= 40.0
            } else {
                prev_v >= 58.0 && curr_v < prev_v && curr_v <= 60.0
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(curr_v),
                direction: fired.then_some(if is_up {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        // =====================================================================
        // B2 Extended Indicator Triggers (`12`)
        // =====================================================================
        "technical.adx_strong_trend" => {
            let (Some(adx), Some(pdi), Some(mdi)) = (
                bundle.adx_14.adx[t],
                bundle.adx_14.plus_di[t],
                bundle.adx_14.minus_di[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = adx >= 25.0 && (pdi - mdi).abs() > 1.0e-9;
            let dir = if !fired {
                None
            } else if pdi > mdi {
                Some(SignalDirection::Bullish)
            } else {
                Some(SignalDirection::Bearish)
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(adx),
                direction: dir,
            }
        }
        "technical.di_bull_cross" | "technical.di_bear_cross" => {
            let (Some(p0), Some(m0), Some(p1), Some(m1)) = (
                bundle.adx_14.plus_di[t - 1],
                bundle.adx_14.minus_di[t - 1],
                bundle.adx_14.plus_di[t],
                bundle.adx_14.minus_di[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_bull = entry.id == "technical.di_bull_cross";
            let fired = if is_bull {
                p0 <= m0 && p1 > m1
            } else {
                m0 <= p0 && m1 > p1
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(p1 - m1),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "technical.stoch_bull_cross" | "technical.stoch_bear_cross" => {
            let (Some(k0), Some(d0), Some(k1), Some(d1)) = (
                bundle.stoch_14.slow_k[t - 1],
                bundle.stoch_14.slow_d[t - 1],
                bundle.stoch_14.slow_k[t],
                bundle.stoch_14.slow_d[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_bull = entry.id == "technical.stoch_bull_cross";
            let fired = if is_bull {
                k0 <= 25.0 && k0 <= d0 && k1 > d1
            } else {
                k0 >= 75.0 && k0 >= d0 && k1 < d1
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(k1),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "technical.supertrend_flip_bull" | "technical.supertrend_flip_bear" => {
            let (Some(d0), Some(d1), Some(st1)) = (
                bundle.supertrend_10.direction[t - 1],
                bundle.supertrend_10.direction[t],
                bundle.supertrend_10.trend[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_bull = entry.id == "technical.supertrend_flip_bull";
            let fired = if is_bull {
                d0 == -1 && d1 == 1
            } else {
                d0 == 1 && d1 == -1
            };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(st1),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "technical.psar_flip_bull" | "technical.psar_flip_bear" => {
            let (Some(l0), Some(l1), Some(sar1)) = (
                bundle.psar.is_long[t - 1],
                bundle.psar.is_long[t],
                bundle.psar.sar[t],
            ) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let is_bull = entry.id == "technical.psar_flip_bull";
            let fired = if is_bull { !l0 && l1 } else { l0 && !l1 };
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(sar1),
                direction: fired.then_some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                }),
            }
        }
        "technical.keltner_upper_breakout" => {
            let (Some(u0), Some(u1)) = (bundle.keltner_20.upper[t - 1], bundle.keltner_20.upper[t])
            else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let prev_c = candles[t - 1].close;
            if !prev_c.is_finite() {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            }
            let fired = prev_c <= u0 && c.close > u1;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(u1),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "technical.cci_oversold_cross" => {
            let (Some(c0), Some(c1)) = (bundle.cci_20[t - 1], bundle.cci_20[t]) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = c0 <= -100.0 && c1 > -100.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(c1),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        "technical.mfi_oversold_cross" => {
            let (Some(m0), Some(m1)) = (bundle.mfi_14[t - 1], bundle.mfi_14[t]) else {
                return CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory);
            };
            let fired = m0 <= 20.0 && m1 > 20.0;
            CompactSignalPoint {
                quality: SignalQuality::Valid,
                bool_value: Some(fired),
                float_value: Some(m1),
                direction: fired.then_some(SignalDirection::Bullish),
            }
        }
        _ => CompactSignalPoint::non_valid(SignalQuality::InsufficientHistory),
    }
}

fn evaluate_single_entry_at(
    entry: &'static SignalCatalogEntry,
    candles: &[Candle],
    bundle: &ConfluenceSeriesBundle,
    regime: MarketRegime,
    t: usize,
    profile: ConfluenceWeightProfile,
) -> SignalEvaluation {
    let ts = candles.get(t).map_or(0, |c| c.timestamp);
    if t < entry.warmup_bars || t >= candles.len() {
        return SignalEvaluation::non_valid(
            entry,
            SignalQuality::InsufficientHistory,
            t,
            ts,
            "INSUFFICIENT_WARMUP_BARS",
        );
    }

    let c = &candles[t];
    if !c.open.is_finite()
        || !c.high.is_finite()
        || !c.low.is_finite()
        || !c.close.is_finite()
        || c.high < c.low
    {
        return SignalEvaluation::non_valid(
            entry,
            SignalQuality::InsufficientHistory,
            t,
            ts,
            "NON_FINITE_OR_INVERTED_OHLC",
        );
    }

    // Special rich attribution path for Technical Health entries:
    if is_profile_dependent_health_entry(entry.id) {
        return match compute_health_at(candles, bundle, regime, t, profile) {
            Ok((bd, mut reasons, factors)) => {
                let pt = health_derived_compact_point(entry.id, Ok(bd.composite_score))
                    .expect("health entry");
                if entry.id == "health.favorable" && pt.bool_value == Some(true) {
                    reasons.insert(0, "HEALTH_FAVORABLE_GE_35".to_owned());
                } else if entry.id == "health.unfavorable" && pt.bool_value == Some(true) {
                    reasons.insert(0, "HEALTH_UNFAVORABLE_LE_M35".to_owned());
                }
                SignalEvaluation {
                    catalog_id: entry.id,
                    quality: SignalQuality::Valid,
                    bool_value: pt.bool_value,
                    float_value: pt.float_value,
                    enum_label: None,
                    direction: pt.direction,
                    bar_index: t,
                    available_at_index: t,
                    available_at_timestamp: ts,
                    availability_event: SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE,
                    is_ohlcv_proxy: entry.is_ohlcv_proxy,
                    params_digest: entry.params_digest(),
                    reasons,
                    factors,
                }
            }
            Err(q) => SignalEvaluation::non_valid(entry, q, t, ts, q.code()),
        };
    }

    let rvol20 = inspect_prior_rvol_at(candles, t, 20);
    let pt = evaluate_compact_entry_at(
        entry,
        candles,
        bundle,
        regime,
        rvol20,
        Err(SignalQuality::InsufficientHistory),
        t,
    );
    if !pt.quality.is_valid() {
        return SignalEvaluation::non_valid(entry, pt.quality, t, ts, pt.quality.code());
    }

    let mut eval = SignalEvaluation {
        catalog_id: entry.id,
        quality: SignalQuality::Valid,
        bool_value: pt.bool_value,
        float_value: pt.float_value,
        enum_label: (entry.id == "regime.market_state").then(|| regime.full_label().to_owned()),
        direction: pt.direction,
        bar_index: t,
        available_at_index: t,
        available_at_timestamp: ts,
        availability_event: SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE,
        is_ohlcv_proxy: entry.is_ohlcv_proxy,
        params_digest: entry.params_digest(),
        reasons: Vec::new(),
        factors: Vec::new(),
    };

    if entry.is_ohlcv_proxy {
        eval.reasons.push("OHLCV_PROXY".to_owned());
    }

    match entry.id {
        "regime.market_state" => {
            eval.reasons
                .push(format!("REGIME_{}", regime.short_label()));
            if let (Some(e20), Some(e50)) = (bundle.ema_20[t], bundle.ema_50[t]) {
                eval.factors.push(SignalAttributionFactor {
                    factor_key: "ema20_vs_ema50".to_owned(),
                    label: "EMA(20) - EMA(50)".to_owned(),
                    observed: e20 - e50,
                    reference: 0.0,
                    weight: 1.0,
                    contribution: e20 - e50,
                });
            }
        }
        "pattern.any_bullish" => {
            let sum = bundle.candle_patterns[t];
            for k in CandlestickPatternKind::BULLISH {
                if sum.pattern_result(k) == Some(true) {
                    eval.reasons.push(k.short_snake_name().to_owned());
                }
            }
            if eval.reasons.is_empty() {
                eval.reasons.push("NO_BULLISH_PATTERN".to_owned());
            }
        }
        "pattern.any_bearish" => {
            let sum = bundle.candle_patterns[t];
            for k in CandlestickPatternKind::BEARISH {
                if sum.pattern_result(k) == Some(true) {
                    eval.reasons.push(k.short_snake_name().to_owned());
                }
            }
            if eval.reasons.is_empty() {
                eval.reasons.push("NO_BEARISH_PATTERN".to_owned());
            }
        }
        "sr.near_support" | "sr.near_resistance" => {
            let is_sup = entry.id == "sr.near_support";
            if let Some((fired, _, matched)) = eval_sr_proximity(candles, bundle, t, is_sup) {
                if fired {
                    if matched[0] {
                        eval.reasons.push(if is_sup {
                            "ROLLING_LOW_20".to_owned()
                        } else {
                            "ROLLING_HIGH_20".to_owned()
                        });
                    }
                    if matched[1] {
                        eval.reasons.push("EMA20".to_owned());
                    }
                    if matched[2] {
                        eval.reasons.push("EMA50".to_owned());
                    }
                    if matched[3] {
                        eval.reasons.push("EMA200".to_owned());
                    }
                } else {
                    eval.reasons.push(if is_sup {
                        "NOT_NEAR_SUPPORT".to_owned()
                    } else {
                        "NOT_NEAR_RESISTANCE".to_owned()
                    });
                }
            }
        }
        "divergence.confirmed_pivot_high" | "divergence.confirmed_pivot_low" => {
            let p = t - 3;
            if eval.bool_value == Some(true) {
                eval.reasons.push(format!("{}_at_{p}", entry.ast_alias));
                eval.reasons.push(format!("confirmed_at_{t}"));
            } else {
                eval.reasons.push("NO_CONFIRMED_PIVOT".to_owned());
            }
        }
        id if id.starts_with("divergence.") && id != "divergence.rsi_hidden_or_classic" => {
            let p = t - 3;
            if eval.bool_value == Some(true) {
                let pair = if pt.direction == Some(SignalDirection::Bullish) {
                    bundle.pivots.low_pair[t]
                } else {
                    bundle.pivots.high_pair[t]
                };
                if let Some((p1, p2)) = pair {
                    eval.reasons
                        .push(format!("pivot1_at_{p1}_pivot2_at_{p2}_sep_{}", p2 - p1));
                }
                eval.reasons.push(format!("confirmed_at_{t}_pivot_{p}"));
            } else {
                eval.reasons.push("NO_CONFIRMED_DIVERGENCE".to_owned());
            }
        }
        "flow.cvd_ohlcv_impulse" => {
            let bar_delta = compute_bar_volume_delta(c).unwrap_or(0.0);
            if let Some(fv) = eval.float_value {
                eval.reasons.push(format!("CVD_5B_SCORE_{fv:+.1}"));
            }
            eval.reasons.push(format!("BAR_DELTA_{bar_delta:+.0}"));
        }
        id if id.starts_with("bb.") => {
            let h20 = bundle.proxy_bb.points[t].horizon(BBHorizon::T20);
            eval.reasons.push(format!(
                "BB_T20_{}_{}",
                h20.regime.code(),
                h20.direction.code()
            ));
            if let Some(fv) = eval.float_value {
                eval.reasons.push(format!("BB_T20_SCORE_{fv:.2}"));
            }
        }
        _ => {
            if eval.bool_value == Some(true) {
                eval.reasons.push(entry.ast_alias.to_owned());
            } else if let Some(fv) = eval.float_value {
                eval.reasons.push(format!("VALUE_{fv:.3}"));
            } else {
                eval.reasons.push("INACTIVE".to_owned());
            }
        }
    }

    if eval.factors.is_empty()
        && let Some(fv) = eval.float_value
    {
        eval.factors.push(SignalAttributionFactor {
            factor_key: entry.id.to_owned(),
            label: entry.label_en.to_owned(),
            observed: fv,
            reference: 0.0,
            weight: 1.0,
            contribution: fv,
        });
    }

    eval.enforce_quality_contract();
    eval
}

/// Evaluates the full `101`-entry [`SIGNAL_CATALOG`] and 4-family [`TechnicalHealthBreakdown`]
/// strictly within the causal prefix `c.timestamp <= cutoff_timestamp` (`DEF-001`).
#[must_use]
pub fn evaluate_causal_confluence(
    candles: &[Candle],
    cutoff_timestamp: Option<i64>,
    exclude_incomplete_tail_signal: bool,
    profile: ConfluenceWeightProfile,
) -> CausalConfluenceReport {
    let slice = match cutoff_timestamp {
        Some(cut) => {
            let end = candles.partition_point(|c| c.timestamp <= cut);
            &candles[..end]
        }
        None => candles,
    };

    if slice.is_empty() {
        return CausalConfluenceReport {
            quality: SignalQuality::InsufficientHistory,
            evaluated_bar_index: 0,
            evaluated_timestamp: 0,
            evaluated_close: 0.0,
            tail_bar_excluded: exclude_incomplete_tail_signal,
            data_fidelity: CONFLUENCE_DATA_FIDELITY_PROXY,
            regime: MarketRegime::InsufficientHistory,
            health: None,
            bullish_signal_count: 0,
            bearish_signal_count: 0,
            evaluations: SIGNAL_CATALOG
                .iter()
                .map(|e| {
                    SignalEvaluation::non_valid(
                        e,
                        SignalQuality::InsufficientHistory,
                        0,
                        0,
                        "EMPTY_CAUSAL_SLICE",
                    )
                })
                .collect(),
        };
    }

    if exclude_incomplete_tail_signal && slice.len() < 2 {
        let ts = slice[0].timestamp;
        return CausalConfluenceReport {
            quality: SignalQuality::UnclosedTailBar,
            evaluated_bar_index: 0,
            evaluated_timestamp: ts,
            evaluated_close: slice[0].close,
            tail_bar_excluded: true,
            data_fidelity: CONFLUENCE_DATA_FIDELITY_PROXY,
            regime: MarketRegime::InsufficientHistory,
            health: None,
            bullish_signal_count: 0,
            bearish_signal_count: 0,
            evaluations: SIGNAL_CATALOG
                .iter()
                .map(|e| {
                    SignalEvaluation::non_valid(
                        e,
                        SignalQuality::UnclosedTailBar,
                        0,
                        ts,
                        "UNCLOSED_HIGHER_TF_TAIL_BAR",
                    )
                })
                .collect(),
        };
    }

    let eval_idx = if exclude_incomplete_tail_signal {
        slice.len() - 2
    } else {
        slice.len() - 1
    };

    evaluate_causal_confluence_at_index_inner(
        slice,
        eval_idx,
        exclude_incomplete_tail_signal,
        profile,
    )
}

/// Evaluates causal confluence & catalog explanations at an explicit bar index `bar_index` within
/// the causal slice `c.timestamp <= cutoff_timestamp`.
#[must_use]
pub fn evaluate_signal_catalog_at_bar(
    candles: &[Candle],
    bar_index: usize,
    cutoff_timestamp: Option<i64>,
    profile: ConfluenceWeightProfile,
) -> Option<CausalConfluenceReport> {
    let slice = match cutoff_timestamp {
        Some(cut) => {
            let end = candles.partition_point(|c| c.timestamp <= cut);
            &candles[..end]
        }
        None => candles,
    };
    if bar_index >= slice.len() {
        return None;
    }
    Some(evaluate_causal_confluence_at_index_inner(
        slice, bar_index, false, profile,
    ))
}

/// Builds the exact [`SignalEvaluation`] for a scanned [`HistoricalSignalHit`] at its trigger bar,
/// gated strictly by bar-close causality (`hit.timestamp + timeframe_span <= cutoff_timestamp`, `DEF-001`).
#[must_use]
pub fn explain_historical_signal(
    candles: &[Candle],
    hit: &HistoricalSignalHit,
    cutoff_timestamp: Option<i64>,
) -> Option<SignalEvaluation> {
    let bar_span = hit.timeframe.seconds().max(1).saturating_sub(1);
    if let Some(cut) = cutoff_timestamp
        && hit.timestamp.saturating_add(bar_span) > cut
    {
        return None;
    }
    let slice = match cutoff_timestamp {
        Some(cut) => {
            let end = candles.partition_point(|c| c.timestamp.saturating_add(bar_span) <= cut);
            &candles[..end]
        }
        None => candles,
    };
    let idx = if slice
        .get(hit.bar_index)
        .is_some_and(|c| c.timestamp == hit.timestamp)
    {
        hit.bar_index
    } else {
        slice.iter().position(|c| c.timestamp == hit.timestamp)?
    };
    let entry = find_catalog_entry_for_scan_kind(hit.signal);
    let bundle = ConfluenceSeriesBundle::compute(slice);
    let regime = super::engine::classify_regime_from_slices(
        slice,
        &bundle.ema_20,
        &bundle.ema_50,
        &bundle.atr_14,
        &bundle.bb,
        idx,
    );
    Some(evaluate_single_entry_at(
        entry,
        slice,
        &bundle,
        regime,
        idx,
        ConfluenceWeightProfile::Balanced,
    ))
}

fn evaluate_causal_confluence_at_index_inner(
    slice: &[Candle],
    eval_idx: usize,
    tail_bar_excluded: bool,
    profile: ConfluenceWeightProfile,
) -> CausalConfluenceReport {
    let bundle = ConfluenceSeriesBundle::compute(slice);
    let regime = super::engine::classify_regime_from_slices(
        slice,
        &bundle.ema_20,
        &bundle.ema_50,
        &bundle.atr_14,
        &bundle.bb,
        eval_idx,
    );
    let health_res = compute_health_at(slice, &bundle, regime, eval_idx, profile);
    let (quality, health) = match health_res {
        Ok((bd, _, _)) => (SignalQuality::Valid, Some(bd)),
        Err(q) => (q, None),
    };

    let mut evaluations = Vec::with_capacity(SIGNAL_CATALOG.len());
    let mut bull_cnt = 0_usize;
    let mut bear_cnt = 0_usize;

    for entry in &SIGNAL_CATALOG {
        let ev = evaluate_single_entry_at(entry, slice, &bundle, regime, eval_idx, profile);
        let triggered_or_directional = match entry.output_type {
            SignalOutputType::Bool => ev.bool_value == Some(true),
            SignalOutputType::Float | SignalOutputType::Enum => ev.direction.is_some(),
        };
        if triggered_or_directional {
            match ev.direction {
                Some(SignalDirection::Bullish) => bull_cnt += 1,
                Some(SignalDirection::Bearish) => bear_cnt += 1,
                None => {}
            }
        }
        evaluations.push(ev);
    }

    CausalConfluenceReport {
        quality,
        evaluated_bar_index: eval_idx,
        evaluated_timestamp: slice[eval_idx].timestamp,
        evaluated_close: slice[eval_idx].close,
        tail_bar_excluded,
        data_fidelity: CONFLUENCE_DATA_FIDELITY_PROXY,
        regime,
        health,
        bullish_signal_count: bull_cnt,
        bearish_signal_count: bear_cnt,
        evaluations,
    }
}

/// Zero-heap-allocation compact evaluation of one [`SignalCatalogEntry`] at bar `t` (`Milestone R16 / R18`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompactSignalPoint {
    pub quality: SignalQuality,
    pub bool_value: Option<bool>,
    pub float_value: Option<f64>,
    pub direction: Option<SignalDirection>,
}

impl CompactSignalPoint {
    #[inline]
    #[must_use]
    pub const fn non_valid(quality: SignalQuality) -> Self {
        Self {
            quality,
            bool_value: None,
            float_value: None,
            direction: None,
        }
    }
}

/// Precomputed $O(N)$ matrix over all `101` [`SIGNAL_CATALOG`] entries, [`MarketRegime`], and 4-family
/// Technical Health scores (`Balanced` & `TrendFocused`) across a causal candle slice.
#[derive(Debug, Clone)]
pub struct PrecomputedCatalogMatrix {
    pub bar_count: usize,
    pub points: Box<[Vec<CompactSignalPoint>]>,
    pub regimes: Vec<MarketRegime>,
    pub health_balanced: Vec<Result<f64, SignalQuality>>,
    pub health_trend_focused: Vec<Result<f64, SignalQuality>>,
    pub counts_balanced: Vec<(u8, u8)>,
    pub counts_trend_focused: Vec<(u8, u8)>,
}

impl PrecomputedCatalogMatrix {
    /// Precomputes all `101` [`SIGNAL_CATALOG`] series, [`MarketRegime`], and both [`ConfluenceWeightProfile`]
    /// composite scores across `candles` in $O(N)$ time with zero per-bar string allocations.
    #[must_use]
    pub fn compute(candles: &[Candle]) -> Self {
        let n = candles.len();
        let mut points: Box<[Vec<CompactSignalPoint>]> = (0..SIGNAL_CATALOG.len())
            .map(|_| Vec::with_capacity(n))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        if n == 0 {
            return Self {
                bar_count: 0,
                points,
                regimes: Vec::new(),
                health_balanced: Vec::new(),
                health_trend_focused: Vec::new(),
                counts_balanced: Vec::new(),
                counts_trend_focused: Vec::new(),
            };
        }

        let bundle = ConfluenceSeriesBundle::compute(candles);
        let mut regimes = Vec::with_capacity(n);
        let mut health_balanced = Vec::with_capacity(n);
        let mut health_trend_focused = Vec::with_capacity(n);
        let mut counts_balanced = Vec::with_capacity(n);
        let mut counts_trend_focused = Vec::with_capacity(n);

        let add_pt_dir =
            |entry: &SignalCatalogEntry, pt: CompactSignalPoint, b: &mut u8, s: &mut u8| {
                let active = match entry.output_type {
                    SignalOutputType::Bool => pt.bool_value == Some(true),
                    SignalOutputType::Float | SignalOutputType::Enum => pt.direction.is_some(),
                };
                if active {
                    match pt.direction {
                        Some(SignalDirection::Bullish) => *b = b.saturating_add(1),
                        Some(SignalDirection::Bearish) => *s = s.saturating_add(1),
                        None => {}
                    }
                }
            };

        for t in 0..n {
            let regime = super::engine::classify_regime_from_slices(
                candles,
                &bundle.ema_20,
                &bundle.ema_50,
                &bundle.atr_14,
                &bundle.bb,
                t,
            );
            regimes.push(regime);

            let rvol20 = inspect_prior_rvol_at(candles, t, 20);
            let h_bal = compute_health_core_at(
                candles,
                &bundle,
                regime,
                rvol20,
                t,
                ConfluenceWeightProfile::Balanced,
            )
            .map(|(bd, _, _)| bd.composite_score);
            let h_trd = compute_health_core_at(
                candles,
                &bundle,
                regime,
                rvol20,
                t,
                ConfluenceWeightProfile::TrendFocused,
            )
            .map(|(bd, _, _)| bd.composite_score);
            health_balanced.push(h_bal);
            health_trend_focused.push(h_trd);

            let mut bull_bal = 0_u8;
            let mut bear_bal = 0_u8;
            let mut bull_trd = 0_u8;
            let mut bear_trd = 0_u8;

            for (cat_idx, entry) in SIGNAL_CATALOG.iter().enumerate() {
                let pt_bal =
                    evaluate_compact_entry_at(entry, candles, &bundle, regime, rvol20, h_bal, t);
                add_pt_dir(entry, pt_bal, &mut bull_bal, &mut bear_bal);

                if is_profile_dependent_health_entry(entry.id) {
                    let pt_trd = if !pt_bal.quality.is_valid() && t < entry.warmup_bars {
                        pt_bal
                    } else {
                        evaluate_compact_entry_at(entry, candles, &bundle, regime, rvol20, h_trd, t)
                    };
                    add_pt_dir(entry, pt_trd, &mut bull_trd, &mut bear_trd);
                } else {
                    add_pt_dir(entry, pt_bal, &mut bull_trd, &mut bear_trd);
                }
                points[cat_idx].push(pt_bal);
            }

            counts_balanced.push((bull_bal, bear_bal));
            counts_trend_focused.push((bull_trd, bear_trd));
        }

        Self {
            bar_count: n,
            points,
            regimes,
            health_balanced,
            health_trend_focused,
            counts_balanced,
            counts_trend_focused,
        }
    }

    #[inline]
    #[must_use]
    pub fn point_at(&self, catalog_idx: usize, bar_index: usize) -> Option<CompactSignalPoint> {
        self.points.get(catalog_idx)?.get(bar_index).copied()
    }

    /// Returns the [`CompactSignalPoint`] for `catalog_idx` at `bar_index` under `profile`
    /// (`health.technical_composite`, `health.score`, `health.favorable`, and `health.unfavorable`
    /// adapt seamlessly to `Balanced` vs `TrendFocused`).
    #[inline]
    #[must_use]
    pub fn point_at_with_profile(
        &self,
        catalog_idx: usize,
        bar_index: usize,
        profile: ConfluenceWeightProfile,
    ) -> Option<CompactSignalPoint> {
        let entry = SIGNAL_CATALOG.get(catalog_idx)?;
        if profile == ConfluenceWeightProfile::TrendFocused
            && is_profile_dependent_health_entry(entry.id)
        {
            let base_pt = self.points.get(catalog_idx)?.get(bar_index).copied()?;
            if !base_pt.quality.is_valid() {
                return Some(base_pt);
            }
            let h_trd = self.health_trend_focused.get(bar_index).copied()?;
            return health_derived_compact_point(entry.id, h_trd);
        }
        self.point_at(catalog_idx, bar_index)
    }

    #[inline]
    pub fn health_score_at(
        &self,
        bar_index: usize,
        profile: ConfluenceWeightProfile,
    ) -> Result<f64, SignalQuality> {
        let vec = match profile {
            ConfluenceWeightProfile::Balanced => &self.health_balanced,
            ConfluenceWeightProfile::TrendFocused => &self.health_trend_focused,
        };
        vec.get(bar_index)
            .copied()
            .unwrap_or(Err(SignalQuality::InsufficientHistory))
    }

    #[inline]
    #[must_use]
    pub fn regime_at(&self, bar_index: usize) -> MarketRegime {
        self.regimes
            .get(bar_index)
            .copied()
            .unwrap_or(MarketRegime::InsufficientHistory)
    }

    #[inline]
    #[must_use]
    pub fn counts_at(&self, bar_index: usize, profile: ConfluenceWeightProfile) -> (u8, u8) {
        let vec = match profile {
            ConfluenceWeightProfile::Balanced => &self.counts_balanced,
            ConfluenceWeightProfile::TrendFocused => &self.counts_trend_focused,
        };
        vec.get(bar_index).copied().unwrap_or((0, 0))
    }
}
