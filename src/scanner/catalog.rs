//! Explainable Signal Catalog & Orthogonal 4-Family Causal Confluence Engine (`Milestone R15 / Wave 14`).
//!
//! Pure `f64`/`i64`/`u64`, `wasm32-unknown-unknown`-compatible signal definition registry,
//! quality-gated per-signal explanation builder, and composite Technical Health / Confluence scorer
//! aligned with `ARCHIVE/frontend/src/components/signals/{SignalCatalog,SignalExplanationInspector,SignalInspector}.tsx`
//! and `ARCHIVE/backend/app/domain/signals/{models,volume,health,ichimoku,flow_events}.py`.

use serde::{Deserialize, Serialize};

use crate::indicators::{
    BollingerOutput, CvdConfig, CvdResetMode, IchimokuConfig, IchimokuOutput, MacdOutput,
    compute_atr, compute_bar_volume_delta, compute_bollinger_slices, compute_cvd,
    compute_ema_slices, compute_ichimoku, compute_macd_slices, compute_rsi_slices,
    compute_sma_slices,
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
            Self::Pattern => "Volatility Patterns",
            Self::SupportResistance => "Support & Resistance",
            Self::Structure => "Market Structure",
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

/// Immutable Signal Catalog metadata entry (`17` entries spanning all 11 domain categories).
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

const PARAMS_STRUCT_BOS: &[SignalParamSpec] = &[SignalParamSpec {
    name: "swing_window",
    default_value: 10.0,
    min_value: 3.0,
    max_value: 60.0,
    description: "Prior swing high/low structure window [t-N..t)",
}];

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

/// Canonical 17-entry Explainable Signal Catalog covering all 11 domain categories.
pub static SIGNAL_CATALOG: [SignalCatalogEntry; 17] = [
    SignalCatalogEntry {
        id: "health.technical_composite",
        ast_alias: "HEALTH_SCORE",
        label_en: "Technical Health Composite",
        label_vi: "Điểm Sức Khỏe Kỹ Thuật Tổng Hợp",
        description: "Orthogonal 4-family composite [-100, +100] blending Trend, Momentum, Transition, and OHLCV-Proxy Participation.",
        category: SignalCategory::Health,
        output_type: SignalOutputType::Float,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: CONFLUENCE_WARMUP_BARS,
        causal_delay_bars: 0,
        is_ohlcv_proxy: true,
        scan_kind: None,
        params: PARAMS_HEALTH,
    },
    SignalCatalogEntry {
        id: "regime.market_state",
        ast_alias: "REGIME_STATE",
        label_en: "Causal Market Regime (SIG-REG)",
        label_vi: "Phân Loại Pha Thị Trường Nhân Quả",
        description: "Classifies Uptrend, Downtrend, Pullback, Recovery, or Sideways Squeeze using strict prior-window extrema.",
        category: SignalCategory::Regime,
        output_type: SignalOutputType::Enum,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 50,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: None,
        params: PARAMS_REGIME,
    },
    SignalCatalogEntry {
        id: "technical.rsi_oversold_cross",
        ast_alias: "RSI_CROSS_UP_30",
        label_en: "RSI(14) Cross Above 30",
        label_vi: "RSI(14) Cắt Lên Khỏi Vùng Quá Bán 30",
        description: "Wilder RSI(14) crosses strictly above 30.0 at bar close from <= 30.0 on the prior bar.",
        category: SignalCategory::Technical,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 15,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::RsiOversoldCrossUp),
        params: PARAMS_RSI_CROSS,
    },
    SignalCatalogEntry {
        id: "technical.rsi_overbought_cross",
        ast_alias: "RSI_CROSS_DN_70",
        label_en: "RSI(14) Cross Below 70",
        label_vi: "RSI(14) Cắt Xuống Khỏi Vùng Quá Mua 70",
        description: "Wilder RSI(14) crosses strictly below 70.0 at bar close from >= 70.0 on the prior bar.",
        category: SignalCategory::Technical,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 15,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::RsiOverboughtCrossDown),
        params: PARAMS_RSI_CROSS,
    },
    SignalCatalogEntry {
        id: "technical.macd_bull_cross",
        ast_alias: "MACD_BULL_X",
        label_en: "MACD(12,26,9) Bullish Cross",
        label_vi: "MACD Cắt Lên Đường Tín Hiệu",
        description: "MACD histogram flips positive (prev <= 0, curr > 0) at bar close.",
        category: SignalCategory::Technical,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 34,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::MacdBullCross),
        params: PARAMS_MACD,
    },
    SignalCatalogEntry {
        id: "technical.macd_bear_cross",
        ast_alias: "MACD_BEAR_X",
        label_en: "MACD(12,26,9) Bearish Cross",
        label_vi: "MACD Cắt Xuống Đường Tín Hiệu",
        description: "MACD histogram flips negative (prev >= 0, curr < 0) at bar close.",
        category: SignalCategory::Technical,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 34,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::MacdBearCross),
        params: PARAMS_MACD,
    },
    SignalCatalogEntry {
        id: "technical.sma_ema_golden_cross",
        ast_alias: "GOLDEN_CROSS_20_50",
        label_en: "SMA(20)/EMA(50) Golden Cross",
        label_vi: "Giao Cắt Vàng SMA(20) Lên Trên EMA(50)",
        description: "Fast SMA(20) crosses strictly above slow EMA(50) at bar close.",
        category: SignalCategory::Technical,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 50,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::SmaEmaGoldenCross),
        params: PARAMS_MA_CROSS,
    },
    SignalCatalogEntry {
        id: "technical.sma_ema_death_cross",
        ast_alias: "DEATH_CROSS_20_50",
        label_en: "SMA(20)/EMA(50) Death Cross",
        label_vi: "Giao Cắt Tử Thần SMA(20) Xuống Dưới EMA(50)",
        description: "Fast SMA(20) crosses strictly below slow EMA(50) at bar close.",
        category: SignalCategory::Technical,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 50,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::SmaEmaDeathCross),
        params: PARAMS_MA_CROSS,
    },
    SignalCatalogEntry {
        id: "pattern.bollinger_squeeze_breakout",
        ast_alias: "BB_SQUEEZE_BRK",
        label_en: "Bollinger Squeeze Breakout",
        label_vi: "Phá Vỡ Nén Dải Bollinger(20, 2.0)",
        description: "Prior-bar Bollinger bandwidth <= 10% followed by a close above the upper band.",
        category: SignalCategory::Pattern,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::BollingerSqueezeBreakout),
        params: PARAMS_BB_SQUEEZE,
    },
    SignalCatalogEntry {
        id: "volume.spike_2x",
        ast_alias: "VOL_SPIKE_2X",
        label_en: "Causal Volume Spike >= 2.0x",
        label_vi: "Đột Biến Khối Lượng >= 2.0x (Cửa Sổ Trước)",
        description: "Volume >= 2.0x prior 20-bar mean [t-20..t) (strictly excluding bar t) on a non-bearish close.",
        category: SignalCategory::Volume,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: Some(ScanSignalKind::VolumeSpike2x),
        params: PARAMS_VOLUME_SPIKE,
    },
    SignalCatalogEntry {
        id: "vsa.climax_up",
        ast_alias: "VSA_CLIMAX_UP",
        label_en: "VSA Buying Climax / Wide-Spread Thrust",
        label_vi: "VSA Cao Trào Mua / Nến Biên Độ Rộng",
        description: "RVOL(20) >= 2.0x with spread >= 1.35x ATR(14) and upper-tier close location >= 0.65.",
        category: SignalCategory::Vsa,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: true,
        scan_kind: None,
        params: PARAMS_VSA_CLIMAX,
    },
    SignalCatalogEntry {
        id: "vsa.stopping_volume",
        ast_alias: "VSA_STOPPING_VOL",
        label_en: "VSA Stopping Volume Absorption",
        label_vi: "VSA Khối Lượng Hấp Thụ Đáy",
        description: "New 10-bar low absorbed on RVOL(20) >= 1.8x with close recovering into top 40% of bar range.",
        category: SignalCategory::Vsa,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Experimental,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: true,
        scan_kind: None,
        params: PARAMS_VSA_STOPPING,
    },
    SignalCatalogEntry {
        id: "divergence.rsi_hidden_or_classic",
        ast_alias: "RSI_DIVERGENCE",
        label_en: "Causal RSI(14) Price Divergence",
        label_vi: "Phân Kỳ Giá - RSI(14) Nhân Quả",
        description: "Detects lower-low price with higher RSI (bullish) or higher-high price with lower RSI (bearish) vs prior 10-bar extrema.",
        category: SignalCategory::Divergence,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 25,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: None,
        params: PARAMS_RSI_DIV,
    },
    SignalCatalogEntry {
        id: "ichimoku.tk_kumo_alignment",
        ast_alias: "ICHI_KUMO_ALIGN",
        label_en: "Ichimoku Tenkan/Kijun & Kumo Alignment",
        label_vi: "Đồng Thuận Tenkan/Kijun & Mây Kumo",
        description: "Evaluates Close & Tenkan(9)/Kijun(26) alignment against the causally displaced Kumo cloud (t - 26).",
        category: SignalCategory::Ichimoku,
        output_type: SignalOutputType::Float,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 77,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: None,
        params: PARAMS_ICHIMOKU,
    },
    SignalCatalogEntry {
        id: "flow.cvd_ohlcv_impulse",
        ast_alias: "CVD_PROXY_IMPULSE",
        label_en: "CVD Intrabar Proxy 5B Impulse",
        label_vi: "Xung Lực CVD Xấp Xỉ OHLCV (5 Nến)",
        description: "Measures 5-bar causal Cumulative Volume Delta slope normalized by 20-bar volume (explicit OHLCV_PROXY).",
        category: SignalCategory::Flow,
        output_type: SignalOutputType::Float,
        status: SignalStatus::Experimental,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: true,
        scan_kind: None,
        params: PARAMS_FLOW_CVD,
    },
    SignalCatalogEntry {
        id: "sr.prior_20b_breakout",
        ast_alias: "SR_20B_BREAKOUT",
        label_en: "Prior 20-Bar S/R Range Breakout",
        label_vi: "Phá Vỡ Kháng Cự / Hỗ Trợ 20 Phiên Trước",
        description: "Close breaks strictly above max(High[t-20..t)) or below min(Low[t-20..t)) without current-bar self-inclusion.",
        category: SignalCategory::SupportResistance,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: None,
        params: PARAMS_SR_BREAKOUT,
    },
    SignalCatalogEntry {
        id: "structure.swing_bos",
        ast_alias: "STRUCT_BOS_10B",
        label_en: "10-Bar Market Structure BOS",
        label_vi: "Phá Vỡ Cấu Trúc Đỉnh/Đáy 10 Nến (BOS)",
        description: "Bullish/Bearish Break of Structure confirmed on bar close beyond prior 10-bar swing high/low with EMA(20) filter.",
        category: SignalCategory::Structure,
        output_type: SignalOutputType::Bool,
        status: SignalStatus::Active,
        version: "1.0.0",
        warmup_bars: 20,
        causal_delay_bars: 0,
        is_ohlcv_proxy: false,
        scan_kind: None,
        params: PARAMS_STRUCT_BOS,
    },
];

/// Looks up a catalog definition by `id`.
#[must_use]
pub fn find_catalog_entry(id: &str) -> Option<&'static SignalCatalogEntry> {
    SIGNAL_CATALOG.iter().find(|e| e.id == id)
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
            available_at_index: bar_index.saturating_add(entry.causal_delay_bars),
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
    /// Family 4 (`[-100, +100]`): Causal prior-window RVOL(20) signed by CLV (`bb_absent_neutral` when flat).
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

struct ConfluenceSeriesBundle {
    sma_20: Vec<Option<f64>>,
    ema_20: Vec<Option<f64>>,
    ema_50: Vec<Option<f64>>,
    rsi_14: Vec<Option<f64>>,
    macd: MacdOutput,
    bb: BollingerOutput,
    atr_14: Vec<Option<f64>>,
    ichimoku: IchimokuOutput,
    cvd_values: Vec<Option<f64>>,
}

impl ConfluenceSeriesBundle {
    fn compute(candles: &[Candle]) -> Self {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let sma_20 = compute_sma_slices(&closes, 20);
        let ema_20 = compute_ema_slices(&closes, 20);
        let ema_50 = compute_ema_slices(&closes, 50);
        let rsi_14 = compute_rsi_slices(&closes, 14);
        let macd = compute_macd_slices(&closes, 12, 26, 9);
        let bb = compute_bollinger_slices(&closes, 20, 2.0);
        let atr_14 = compute_atr(candles, 14);
        let ichimoku = compute_ichimoku(candles, IchimokuConfig::default());
        let cvd_values = compute_cvd(
            candles,
            CvdConfig {
                reset_mode: CvdResetMode::Cumulative,
                ..CvdConfig::default()
            },
        )
        .cvd;
        Self {
            sma_20,
            ema_20,
            ema_50,
            rsi_14,
            macd,
            bb,
            atr_14,
            ichimoku,
            cvd_values,
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
    let span = c.high - c.low;
    if span.is_finite() && span > 1.0e-12 {
        ((c.close - c.low) / span).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

#[inline]
fn signed_clv_m1_p1(c: &Candle) -> f64 {
    let span = c.high - c.low;
    if span.is_finite() && span > 1.0e-12 {
        (((c.close - c.low) - (c.high - c.close)) / span).clamp(-1.0, 1.0)
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
    if t < CONFLUENCE_WARMUP_BARS || t >= candles.len() {
        return Err(SignalQuality::InsufficientHistory);
    }
    let c = &candles[t];
    if !c.close.is_finite() || !c.high.is_finite() || !c.low.is_finite() {
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
    let atr_safe = atr.max((c.close.abs() * 0.002).max(1.0e-8));

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
        MarketRegime::SidewaysSqueeze => 0.0,
        MarketRegime::Neutral => 0.0,
        MarketRegime::Downtrend => -0.70,
        MarketRegime::InsufficientHistory => 0.0,
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

    // 4. Participation / Flow family [-1, +1] (`OHLCV_PROXY` + `bb_absent_neutral` contract)
    let mut reasons = vec![
        "OHLCV_PROXY".to_owned(),
        "bb_absent_neutral".to_owned(),
        format!("REGIME_{}", regime.short_label()),
    ];
    let flow_unit = match inspect_prior_rvol_at(candles, t, 20) {
        PriorRvolCheck::Valid { rvol, .. } => {
            let amp = ((rvol - 0.8) / 1.4).clamp(0.0, 1.0);
            let dir = signed_clv_m1_p1(c);
            reasons.push(format!("RVOL20_{rvol:.2}X"));
            (amp * dir).clamp(-1.0, 1.0)
        }
        PriorRvolCheck::ZeroBaseline => {
            // Institutional BB flow absent & volume baseline zero -> neutral 0.0 (never negative penalty)
            reasons.push("VOLUME_ZERO_BASELINE_NEUTRAL".to_owned());
            0.0
        }
        PriorRvolCheck::InvalidVolume => return Err(SignalQuality::InvalidVolume),
        PriorRvolCheck::InsufficientHistory => return Err(SignalQuality::InsufficientHistory),
    };

    let (w_trend, w_mom, w_shift, w_flow) = profile.weights();
    let composite_unit =
        (w_trend * trend_unit + w_mom * mom_unit + w_shift * shift_unit + w_flow * flow_unit)
            .clamp(-1.0, 1.0);
    let composite_score = composite_unit * 100.0;
    let bias = ConfluenceBias::from_score(composite_score);
    reasons.push(format!("BIAS_{}", bias.label().replace(' ', "_")));

    let breakdown = TechnicalHealthBreakdown {
        profile,
        trend_score: trend_unit * 100.0,
        momentum_score: mom_unit * 100.0,
        transition_score: shift_unit * 100.0,
        participation_score: flow_unit * 100.0,
        composite_score,
        bias,
        bb_flow_absent_neutral: true,
    };

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
            label: "Participation (RVOL20 Proxy)".to_owned(),
            observed: breakdown.participation_score,
            reference: 0.0,
            weight: w_flow,
            contribution: w_flow * breakdown.participation_score,
        },
    ];

    Ok((breakdown, reasons, factors))
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

    let mut eval = SignalEvaluation {
        catalog_id: entry.id,
        quality: SignalQuality::Valid,
        bool_value: None,
        float_value: None,
        enum_label: None,
        direction: None,
        bar_index: t,
        available_at_index: t.saturating_add(entry.causal_delay_bars),
        available_at_timestamp: ts,
        availability_event: SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE,
        is_ohlcv_proxy: entry.is_ohlcv_proxy,
        params_digest: entry.params_digest(),
        reasons: Vec::new(),
        factors: Vec::new(),
    };

    match entry.id {
        "health.technical_composite" => {
            match compute_health_at(candles, bundle, regime, t, profile) {
                Ok((bd, reasons, factors)) => {
                    eval.float_value = Some(bd.composite_score);
                    eval.direction = if bd.composite_score >= 12.0 {
                        Some(SignalDirection::Bullish)
                    } else if bd.composite_score <= -12.0 {
                        Some(SignalDirection::Bearish)
                    } else {
                        None
                    };
                    eval.reasons = reasons;
                    eval.factors = factors;
                }
                Err(q) => {
                    return SignalEvaluation::non_valid(entry, q, t, ts, q.code());
                }
            }
        }
        "regime.market_state" => {
            if regime == MarketRegime::InsufficientHistory {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "INSUFFICIENT_REGIME_HISTORY",
                );
            }
            eval.enum_label = Some(regime.full_label().to_owned());
            eval.direction = match regime {
                MarketRegime::Uptrend | MarketRegime::Pullback | MarketRegime::Recovery => {
                    Some(SignalDirection::Bullish)
                }
                MarketRegime::Downtrend => Some(SignalDirection::Bearish),
                MarketRegime::SidewaysSqueeze
                | MarketRegime::Neutral
                | MarketRegime::InsufficientHistory => None,
            };
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
        "technical.rsi_oversold_cross" => {
            let (Some(prev), Some(curr)) = (bundle.rsi_14[t - 1], bundle.rsi_14[t]) else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "RSI_WARMUP",
                );
            };
            let fired = prev <= 30.0 && curr > 30.0;
            eval.bool_value = Some(fired);
            eval.float_value = Some(curr);
            if fired {
                eval.direction = Some(SignalDirection::Bullish);
                eval.reasons.push("RSI14_CROSS_ABOVE_30".to_owned());
            }
            eval.reasons.push(format!("PREV_RSI_{prev:.2}"));
            eval.reasons.push(format!("CURR_RSI_{curr:.2}"));
            eval.factors.push(SignalAttributionFactor {
                factor_key: "rsi_14".to_owned(),
                label: "RSI(14) vs 30".to_owned(),
                observed: curr,
                reference: 30.0,
                weight: 1.0,
                contribution: curr - 30.0,
            });
        }
        "technical.rsi_overbought_cross" => {
            let (Some(prev), Some(curr)) = (bundle.rsi_14[t - 1], bundle.rsi_14[t]) else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "RSI_WARMUP",
                );
            };
            let fired = prev >= 70.0 && curr < 70.0;
            eval.bool_value = Some(fired);
            eval.float_value = Some(curr);
            if fired {
                eval.direction = Some(SignalDirection::Bearish);
                eval.reasons.push("RSI14_CROSS_BELOW_70".to_owned());
            }
            eval.reasons.push(format!("PREV_RSI_{prev:.2}"));
            eval.reasons.push(format!("CURR_RSI_{curr:.2}"));
            eval.factors.push(SignalAttributionFactor {
                factor_key: "rsi_14".to_owned(),
                label: "RSI(14) vs 70".to_owned(),
                observed: curr,
                reference: 70.0,
                weight: 1.0,
                contribution: 70.0 - curr,
            });
        }
        "technical.macd_bull_cross" | "technical.macd_bear_cross" => {
            let is_bull = entry.id == "technical.macd_bull_cross";
            let (Some(prev_h), Some(curr_h)) =
                (bundle.macd.histogram[t - 1], bundle.macd.histogram[t])
            else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "MACD_WARMUP",
                );
            };
            let fired = if is_bull {
                prev_h <= 0.0 && curr_h > 0.0
            } else {
                prev_h >= 0.0 && curr_h < 0.0
            };
            eval.bool_value = Some(fired);
            eval.float_value = Some(curr_h);
            if fired {
                eval.direction = Some(if is_bull {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                });
                eval.reasons.push(if is_bull {
                    "MACD_HIST_CROSS_POS".to_owned()
                } else {
                    "MACD_HIST_CROSS_NEG".to_owned()
                });
            }
            eval.reasons.push(format!("HIST_{curr_h:+.4}"));
            eval.factors.push(SignalAttributionFactor {
                factor_key: "macd_hist".to_owned(),
                label: "MACD Histogram".to_owned(),
                observed: curr_h,
                reference: 0.0,
                weight: 1.0,
                contribution: curr_h,
            });
        }
        "technical.sma_ema_golden_cross" | "technical.sma_ema_death_cross" => {
            let is_gold = entry.id == "technical.sma_ema_golden_cross";
            let (Some(sp), Some(ep), Some(sc), Some(ec)) = (
                bundle.sma_20[t - 1],
                bundle.ema_50[t - 1],
                bundle.sma_20[t],
                bundle.ema_50[t],
            ) else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "MA_WARMUP",
                );
            };
            let prev_d = sp - ep;
            let curr_d = sc - ec;
            let fired = if is_gold {
                prev_d <= 0.0 && curr_d > 0.0
            } else {
                prev_d >= 0.0 && curr_d < 0.0
            };
            eval.bool_value = Some(fired);
            eval.float_value = Some(curr_d);
            if fired {
                eval.direction = Some(if is_gold {
                    SignalDirection::Bullish
                } else {
                    SignalDirection::Bearish
                });
                eval.reasons.push(if is_gold {
                    "SMA20_ABOVE_EMA50".to_owned()
                } else {
                    "SMA20_BELOW_EMA50".to_owned()
                });
            }
            eval.reasons.push(format!("SPREAD_{curr_d:+.3}"));
            eval.factors.push(SignalAttributionFactor {
                factor_key: "ma_spread".to_owned(),
                label: "SMA(20) - EMA(50)".to_owned(),
                observed: curr_d,
                reference: 0.0,
                weight: 1.0,
                contribution: curr_d,
            });
        }
        "pattern.bollinger_squeeze_breakout" => {
            let (Some(prev_bw), Some(prev_up), Some(curr_up)) = (
                bb_bandwidth_ratio_at(&bundle.bb, t - 1),
                bundle.bb.upper[t - 1],
                bundle.bb.upper[t],
            ) else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "BB_WARMUP",
                );
            };
            let fired = prev_bw <= 0.10 && candles[t - 1].close <= prev_up && c.close > curr_up;
            eval.bool_value = Some(fired);
            eval.float_value = Some(prev_bw * 100.0);
            if fired {
                eval.direction = Some(SignalDirection::Bullish);
                eval.reasons.push("BB_SQUEEZE_UPPER_BREAKOUT".to_owned());
            }
            eval.reasons
                .push(format!("PRIOR_BW_{:.2}PCT", prev_bw * 100.0));
            eval.factors.push(SignalAttributionFactor {
                factor_key: "prior_bb_bw_pct".to_owned(),
                label: "Prior BB Width (%)".to_owned(),
                observed: prev_bw * 100.0,
                reference: 10.0,
                weight: 1.0,
                contribution: 10.0 - prev_bw * 100.0,
            });
        }
        "volume.spike_2x" => match inspect_prior_rvol_at(candles, t, 20) {
            PriorRvolCheck::Valid { rvol, baseline } => {
                let fired = rvol >= 2.0 && c.close >= c.open;
                eval.bool_value = Some(fired);
                eval.float_value = Some(rvol);
                if fired {
                    eval.direction = Some(SignalDirection::Bullish);
                    eval.reasons.push("VOLUME_SPIKE_GE_2X".to_owned());
                }
                eval.reasons.push(format!("RVOL_{rvol:.2}X"));
                eval.reasons.push(format!("PRIOR20_AVG_{baseline:.1}"));
                eval.factors.push(SignalAttributionFactor {
                    factor_key: "rvol_20".to_owned(),
                    label: "Prior-20B RVOL".to_owned(),
                    observed: rvol,
                    reference: 2.0,
                    weight: 1.0,
                    contribution: rvol - 2.0,
                });
            }
            PriorRvolCheck::InsufficientHistory => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "RVOL_WARMUP",
                );
            }
            PriorRvolCheck::InvalidVolume => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InvalidVolume,
                    t,
                    ts,
                    "INVALID_NEGATIVE_OR_NAN_VOLUME",
                );
            }
            PriorRvolCheck::ZeroBaseline => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::ZeroBaseline,
                    t,
                    ts,
                    "ZERO_PRIOR20_VOLUME_BASELINE",
                );
            }
        },
        "vsa.climax_up" => match (inspect_prior_rvol_at(candles, t, 20), bundle.atr_14[t]) {
            (PriorRvolCheck::Valid { rvol, .. }, Some(atr)) if atr > 1.0e-12 => {
                let range_atr = (c.high - c.low).max(0.0) / atr;
                let cloc = close_location_01(c);
                let fired = rvol >= 2.0 && range_atr >= 1.35 && cloc >= 0.65;
                eval.bool_value = Some(fired);
                eval.float_value = Some(rvol);
                if fired {
                    eval.direction = Some(SignalDirection::Bullish);
                    eval.reasons.push("VOLUME_CLIMAX_UP".to_owned());
                }
                eval.reasons.push("OHLCV_PROXY".to_owned());
                eval.reasons.push(format!("RVOL_{rvol:.2}"));
                eval.reasons.push(format!("RANGE_ATR_{range_atr:.2}"));
                eval.reasons.push(format!("CLOSE_LOC_{cloc:.2}"));
                eval.factors.push(SignalAttributionFactor {
                    factor_key: "range_atr".to_owned(),
                    label: "Spread / ATR(14)".to_owned(),
                    observed: range_atr,
                    reference: 1.35,
                    weight: 1.0,
                    contribution: range_atr - 1.35,
                });
            }
            (PriorRvolCheck::InvalidVolume, _) => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InvalidVolume,
                    t,
                    ts,
                    "INVALID_VOLUME",
                );
            }
            (PriorRvolCheck::ZeroBaseline, _) => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::ZeroBaseline,
                    t,
                    ts,
                    "ZERO_BASELINE",
                );
            }
            _ => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "VSA_WARMUP",
                );
            }
        },
        "vsa.stopping_volume" => match inspect_prior_rvol_at(candles, t, 20) {
            PriorRvolCheck::Valid { rvol, .. } => {
                let mut prior_low = f64::INFINITY;
                for b in &candles[t - 10..t] {
                    if b.low.is_finite() && b.low < prior_low {
                        prior_low = b.low;
                    }
                }
                if !prior_low.is_finite() {
                    return SignalEvaluation::non_valid(
                        entry,
                        SignalQuality::InsufficientHistory,
                        t,
                        ts,
                        "STOPPING_WINDOW_INVALID",
                    );
                }
                let cloc = close_location_01(c);
                let fired = c.low <= prior_low && rvol >= 1.8 && cloc >= 0.60;
                eval.bool_value = Some(fired);
                eval.float_value = Some(cloc);
                if fired {
                    eval.direction = Some(SignalDirection::Bullish);
                    eval.reasons.push("VSA_STOPPING_ABSORPTION".to_owned());
                }
                eval.reasons.push("OHLCV_PROXY".to_owned());
                eval.reasons.push(format!("RVOL_{rvol:.2}"));
                eval.reasons.push(format!("CLOSE_LOC_{cloc:.2}"));
            }
            PriorRvolCheck::InvalidVolume => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InvalidVolume,
                    t,
                    ts,
                    "INVALID_VOLUME",
                );
            }
            PriorRvolCheck::ZeroBaseline => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::ZeroBaseline,
                    t,
                    ts,
                    "ZERO_BASELINE",
                );
            }
            PriorRvolCheck::InsufficientHistory => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "VSA_WARMUP",
                );
            }
        },
        "divergence.rsi_hidden_or_classic" => {
            let Some(curr_rsi) = bundle.rsi_14[t] else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "RSI_DIV_WARMUP",
                );
            };
            let mut min_low_idx: Option<usize> = None;
            let mut max_high_idx: Option<usize> = None;
            for idx in (t - 10)..t {
                let b = &candles[idx];
                if b.low.is_finite() && min_low_idx.is_none_or(|m| b.low < candles[m].low) {
                    min_low_idx = Some(idx);
                }
                if b.high.is_finite() && max_high_idx.is_none_or(|m| b.high > candles[m].high) {
                    max_high_idx = Some(idx);
                }
            }
            let (Some(lo_idx), Some(hi_idx)) = (min_low_idx, max_high_idx) else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "RSI_DIV_WINDOW_INVALID",
                );
            };
            let bull_div = bundle.rsi_14[lo_idx].is_some_and(|prev_rsi| {
                c.low < candles[lo_idx].low && curr_rsi > prev_rsi + 2.0 && curr_rsi < 48.0
            });
            let bear_div = bundle.rsi_14[hi_idx].is_some_and(|prev_rsi| {
                c.high > candles[hi_idx].high && curr_rsi < prev_rsi - 2.0 && curr_rsi > 52.0
            });
            let fired = bull_div || bear_div;
            eval.bool_value = Some(fired);
            eval.float_value = Some(curr_rsi);
            if bull_div {
                eval.direction = Some(SignalDirection::Bullish);
                eval.reasons.push("BULL_PRICE_LOW_RSI_HIGHER".to_owned());
            } else if bear_div {
                eval.direction = Some(SignalDirection::Bearish);
                eval.reasons.push("BEAR_PRICE_HIGH_RSI_LOWER".to_owned());
            } else {
                eval.reasons.push("NO_10B_PIVOT_DIVERGENCE".to_owned());
            }
        }
        "ichimoku.tk_kumo_alignment" => {
            let (Some(tk), Some(kj), Some(sa), Some(sb)) = (
                bundle.ichimoku.tenkan.get(t).copied().flatten(),
                bundle.ichimoku.kijun.get(t).copied().flatten(),
                bundle.ichimoku.senkou_a.get(t).copied().flatten(),
                bundle.ichimoku.senkou_b.get(t).copied().flatten(),
            ) else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "ICHIMOKU_KUMO_WARMUP_77",
                );
            };
            let k_top = sa.max(sb);
            let k_bot = sa.min(sb);
            let mut pts = 0.0_f64;
            if c.close > k_top {
                pts += 50.0;
                eval.reasons.push("CLOSE_ABOVE_KUMO".to_owned());
            } else if c.close < k_bot {
                pts -= 50.0;
                eval.reasons.push("CLOSE_BELOW_KUMO".to_owned());
            } else {
                eval.reasons.push("CLOSE_INSIDE_KUMO".to_owned());
            }
            if tk > kj {
                pts += 30.0;
                eval.reasons.push("TENKAN_ABOVE_KIJUN".to_owned());
            } else if tk < kj {
                pts -= 30.0;
                eval.reasons.push("TENKAN_BELOW_KIJUN".to_owned());
            }
            if sa > sb {
                pts += 20.0;
                eval.reasons.push("BULLISH_GREEN_KUMO".to_owned());
            } else if sa < sb {
                pts -= 20.0;
                eval.reasons.push("BEARISH_RED_KUMO".to_owned());
            }
            eval.float_value = Some(pts);
            eval.direction = if pts >= 40.0 {
                Some(SignalDirection::Bullish)
            } else if pts <= -40.0 {
                Some(SignalDirection::Bearish)
            } else {
                None
            };
        }
        "flow.cvd_ohlcv_impulse" => match inspect_prior_rvol_at(candles, t, 20) {
            PriorRvolCheck::Valid { baseline, .. } => {
                let (Some(cvd_now), Some(cvd_prev5)) = (
                    bundle.cvd_values.get(t).copied().flatten(),
                    bundle
                        .cvd_values
                        .get(t.saturating_sub(5))
                        .copied()
                        .flatten(),
                ) else {
                    return SignalEvaluation::non_valid(
                        entry,
                        SignalQuality::InsufficientHistory,
                        t,
                        ts,
                        "CVD_WARMUP",
                    );
                };
                let bar_delta = compute_bar_volume_delta(c).unwrap_or(0.0);
                let norm_impulse =
                    ((cvd_now - cvd_prev5) / (5.0 * baseline)).clamp(-5.0, 5.0) * 20.0;
                eval.float_value = Some(norm_impulse);
                eval.direction = if norm_impulse >= 20.0 {
                    Some(SignalDirection::Bullish)
                } else if norm_impulse <= -20.0 {
                    Some(SignalDirection::Bearish)
                } else {
                    None
                };
                eval.reasons.push("OHLCV_PROXY".to_owned());
                eval.reasons
                    .push(format!("CVD_5B_SCORE_{norm_impulse:+.1}"));
                eval.reasons.push(format!("BAR_DELTA_{bar_delta:+.0}"));
            }
            PriorRvolCheck::InvalidVolume => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InvalidVolume,
                    t,
                    ts,
                    "INVALID_VOLUME",
                );
            }
            PriorRvolCheck::ZeroBaseline => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::ZeroBaseline,
                    t,
                    ts,
                    "ZERO_BASELINE",
                );
            }
            PriorRvolCheck::InsufficientHistory => {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "FLOW_WARMUP",
                );
            }
        },
        "sr.prior_20b_breakout" => {
            let mut prior_hi = f64::NEG_INFINITY;
            let mut prior_lo = f64::INFINITY;
            for b in &candles[t - 20..t] {
                if b.high.is_finite() && b.high > prior_hi {
                    prior_hi = b.high;
                }
                if b.low.is_finite() && b.low < prior_lo {
                    prior_lo = b.low;
                }
            }
            if !prior_hi.is_finite() || !prior_lo.is_finite() {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "SR_WINDOW_INVALID",
                );
            }
            let up_brk = c.close > prior_hi;
            let dn_brk = c.close < prior_lo;
            eval.bool_value = Some(up_brk || dn_brk);
            eval.float_value = Some(if up_brk { prior_hi } else { prior_lo });
            if up_brk {
                eval.direction = Some(SignalDirection::Bullish);
                eval.reasons
                    .push(format!("BREAK_ABOVE_20B_HIGH_{prior_hi:.2}"));
            } else if dn_brk {
                eval.direction = Some(SignalDirection::Bearish);
                eval.reasons
                    .push(format!("BREAK_BELOW_20B_LOW_{prior_lo:.2}"));
            } else {
                eval.reasons.push("INSIDE_PRIOR_20B_RANGE".to_owned());
            }
        }
        "structure.swing_bos" => {
            let Some(e20) = bundle.ema_20[t] else {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "BOS_EMA_WARMUP",
                );
            };
            let mut prior_hi = f64::NEG_INFINITY;
            let mut prior_lo = f64::INFINITY;
            for b in &candles[t - 10..t] {
                if b.high.is_finite() && b.high > prior_hi {
                    prior_hi = b.high;
                }
                if b.low.is_finite() && b.low < prior_lo {
                    prior_lo = b.low;
                }
            }
            if !prior_hi.is_finite() || !prior_lo.is_finite() {
                return SignalEvaluation::non_valid(
                    entry,
                    SignalQuality::InsufficientHistory,
                    t,
                    ts,
                    "BOS_WINDOW_INVALID",
                );
            }
            let bull_bos = c.close > prior_hi && c.close > e20;
            let bear_bos = c.close < prior_lo && c.close < e20;
            eval.bool_value = Some(bull_bos || bear_bos);
            eval.float_value = Some(c.close - e20);
            if bull_bos {
                eval.direction = Some(SignalDirection::Bullish);
                eval.reasons.push("BULLISH_BOS_10B".to_owned());
            } else if bear_bos {
                eval.direction = Some(SignalDirection::Bearish);
                eval.reasons.push("BEARISH_BOS_10B".to_owned());
            } else {
                eval.reasons.push("NO_CONFIRMED_BOS".to_owned());
            }
        }
        _ => {}
    }

    eval.enforce_quality_contract();
    eval
}

/// Evaluates the full 17-entry [`SIGNAL_CATALOG`] and 4-family [`TechnicalHealthBreakdown`]
/// strictly within the causal prefix `c.timestamp <= cutoff_timestamp` (`DEF-001`).
///
/// When `exclude_incomplete_tail_signal` is `true` (active mid-bucket `BaseBar` replay on a higher
/// timeframe), close-confirmed signals are evaluated on the last **completed** bar `last_idx - 1`
/// (`tail_bar_excluded = true`); if no prior completed bar exists, returns `SignalQuality::UnclosedTailBar`.
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
        let triggered_or_directional = match ev.output_type_hint(entry) {
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

impl SignalEvaluation {
    #[inline]
    const fn output_type_hint(&self, entry: &SignalCatalogEntry) -> SignalOutputType {
        let _ = self;
        entry.output_type
    }
}
