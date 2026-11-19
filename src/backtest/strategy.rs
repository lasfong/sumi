use serde::{Deserialize, Serialize};

use crate::alerts::truncate_utf8_bytes;
use crate::scanner::{
    ConfluenceWeightProfile, MarketRegime, PrecomputedCatalogMatrix, SIGNAL_CATALOG,
    SignalDirection, SignalOutputType, catalog_entry_index, find_catalog_entry,
    is_bidirectional_bool_signal,
};
use crate::trading::PositionSide;

/// Maximum parameter variants evaluated in a single deterministic sweep (`<= 64`).
pub const MAX_SWEEP_VARIANTS: usize = 64;

/// Maximum clause groups in a single [`DirectionalRuleLeg`] (`Milestone R16 / Wave 15`).
pub const MAX_RULE_GROUPS: usize = 4;

/// Maximum atomic signal clauses inside one [`SignalClauseGroup`] (`Milestone R16 / Wave 15`).
pub const MAX_CLAUSES_PER_GROUP: usize = 8;

/// Maximum UTF-8 byte length of [`CustomSignalRuleSpec::name`].
pub const MAX_RULE_NAME_BYTES: usize = 64;

/// Canonical rule-based strategy presets available in Sumi Strategy Lab (`Milestone R09 / R16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StrategyPresetKind {
    #[default]
    SmaEmaTrendCross,
    RsiMeanReversion,
    MacdMomentum,
    BollingerSqueezeBreakout,
    DonchianAtrBreakout,
    RsiBollingerConfluence,
    CustomSignalRules,
}

impl StrategyPresetKind {
    pub const ALL: [Self; 7] = [
        Self::SmaEmaTrendCross,
        Self::RsiMeanReversion,
        Self::MacdMomentum,
        Self::BollingerSqueezeBreakout,
        Self::DonchianAtrBreakout,
        Self::RsiBollingerConfluence,
        Self::CustomSignalRules,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::SmaEmaTrendCross => "SMA/EMA Cross",
            Self::RsiMeanReversion => "RSI Mean-Rev",
            Self::MacdMomentum => "MACD Momentum",
            Self::BollingerSqueezeBreakout => "BB Squeeze",
            Self::DonchianAtrBreakout => "Donchian ATR",
            Self::RsiBollingerConfluence => "RSI + BB Conf",
            Self::CustomSignalRules => "Custom Rules",
        }
    }

    #[inline]
    #[must_use]
    pub const fn full_label(self) -> &'static str {
        match self {
            Self::SmaEmaTrendCross => "SMA/EMA Trend Crossover",
            Self::RsiMeanReversion => "RSI Mean Reversion",
            Self::MacdMomentum => "MACD Histogram Momentum",
            Self::BollingerSqueezeBreakout => "Bollinger Squeeze Breakout",
            Self::DonchianAtrBreakout => "Donchian Channel + ATR Breakout",
            Self::RsiBollingerConfluence => "RSI + Bollinger Band Confluence",
            Self::CustomSignalRules => "Composable Signal-Catalog Rule Builder",
        }
    }

    #[inline]
    #[must_use]
    pub const fn summary_rule(self) -> &'static str {
        match self {
            Self::SmaEmaTrendCross => "Fast SMA crosses Slow EMA -> Fill at Next Open",
            Self::RsiMeanReversion => "RSI crosses out of Oversold/Overbought -> Exit at Midline",
            Self::MacdMomentum => "MACD Hist zero-cross aligned with Trend EMA -> Next Open",
            Self::BollingerSqueezeBreakout => {
                "Prior bandwidth <= Max Squeeze% + Close breaks outer Band"
            }
            Self::DonchianAtrBreakout => {
                "Close breaks prior N-bar High/Low channel -> M-bar channel exit"
            }
            Self::RsiBollingerConfluence => {
                "Close pierces Bollinger Band + RSI extreme -> Mid-Band exit"
            }
            Self::CustomSignalRules => {
                "2-Level AND/OR Signal-Catalog Clauses + Confluence Gate -> Open[T+1]"
            }
        }
    }
}

/// Permitted position directionality for strategy execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StrategyDirectionMode {
    #[default]
    LongOnly,
    ShortOnly,
    LongShort,
}

impl StrategyDirectionMode {
    pub const ALL: [Self; 3] = [Self::LongOnly, Self::ShortOnly, Self::LongShort];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::LongOnly => "Long",
            Self::ShortOnly => "Short",
            Self::LongShort => "L+S",
        }
    }

    #[inline]
    #[must_use]
    pub const fn allows_side(self, side: PositionSide) -> bool {
        match (self, side) {
            (Self::LongOnly, PositionSide::Long)
            | (Self::ShortOnly, PositionSide::Short)
            | (Self::LongShort, _) => true,
            (Self::LongOnly, PositionSide::Short) | (Self::ShortOnly, PositionSide::Long) => false,
        }
    }
}

/// Causal position-sizing rule evaluated at `Open[i]` when an entry or reversal fills.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PositionSizingMode {
    /// Allocate `pct` (`1.0..=100.0%`) of current mark-to-market equity.
    PercentEquity(f64),
    /// Risk `risk_pct` (`0.1..=20.0%`) of equity divided by unit initial stop distance.
    RiskPercentPerTrade(f64),
    /// Allocate a fixed quote-currency notional capped by `equity * max_leverage`.
    FixedNotional(f64),
}

impl Default for PositionSizingMode {
    fn default() -> Self {
        Self::PercentEquity(95.0)
    }
}

impl PositionSizingMode {
    pub fn sanitize(&mut self) {
        match self {
            Self::PercentEquity(pct) => {
                if !pct.is_finite() {
                    *pct = 95.0;
                }
                *pct = pct.clamp(1.0, 100.0);
            }
            Self::RiskPercentPerTrade(pct) => {
                if !pct.is_finite() {
                    *pct = 1.5;
                }
                *pct = pct.clamp(0.1, 20.0);
            }
            Self::FixedNotional(notional) => {
                if !notional.is_finite() {
                    *notional = 25_000.0;
                }
                *notional = notional.clamp(10.0, 1.0e12);
            }
        }
    }

    #[must_use]
    pub fn short_label(self) -> String {
        match self {
            Self::PercentEquity(pct) => format!("{pct:.0}% Eq"),
            Self::RiskPercentPerTrade(pct) => format!("{pct:.1}% Risk"),
            Self::FixedNotional(n) => format!("Fix {n:.0}"),
        }
    }
}

/// Tunable indicator & threshold parameters shared across [`StrategyPresetKind`] variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StrategyParams {
    // Trend Crossover (`SmaEmaTrendCross`)
    pub fast_sma: usize,
    pub slow_ema: usize,

    // RSI (`RsiMeanReversion`, `RsiBollingerConfluence`)
    pub rsi_period: usize,
    pub rsi_oversold: f64,
    pub rsi_overbought: f64,
    pub rsi_exit_midline: f64,
    pub rsi_confluence_buy: f64,
    pub rsi_confluence_sell: f64,

    // MACD (`MacdMomentum`)
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    /// Optional trend filter EMA period (`0` = disabled).
    pub macd_trend_ema: usize,

    // Bollinger Bands (`BollingerSqueezeBreakout`, `RsiBollingerConfluence`)
    pub bb_period: usize,
    pub bb_std_mult: f64,
    pub max_squeeze_bw_pct: f64,

    // Donchian + ATR (`DonchianAtrBreakout` & ATR-based stops)
    pub donchian_entry_period: usize,
    pub donchian_exit_period: usize,
    pub atr_period: usize,
}

impl Default for StrategyParams {
    fn default() -> Self {
        Self {
            fast_sma: 20,
            slow_ema: 50,
            rsi_period: 14,
            rsi_oversold: 30.0,
            rsi_overbought: 70.0,
            rsi_exit_midline: 50.0,
            rsi_confluence_buy: 38.0,
            rsi_confluence_sell: 62.0,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            macd_trend_ema: 50,
            bb_period: 20,
            bb_std_mult: 2.0,
            max_squeeze_bw_pct: 12.0,
            donchian_entry_period: 20,
            donchian_exit_period: 10,
            atr_period: 14,
        }
    }
}

impl StrategyParams {
    /// Clamps every parameter to a finite, domain-valid range (`serde` untrusted input safe).
    pub fn sanitize(&mut self) {
        self.fast_sma = self.fast_sma.clamp(2, 200);
        self.slow_ema = self.slow_ema.clamp(3, 250);
        if self.slow_ema <= self.fast_sma {
            self.slow_ema = (self.fast_sma + 1).min(250);
            if self.fast_sma >= self.slow_ema {
                self.fast_sma = self.slow_ema.saturating_sub(1).max(2);
            }
        }

        self.rsi_period = self.rsi_period.clamp(2, 200);
        if !self.rsi_oversold.is_finite() {
            self.rsi_oversold = 30.0;
        }
        self.rsi_oversold = self.rsi_oversold.clamp(5.0, 48.0);
        if !self.rsi_overbought.is_finite() {
            self.rsi_overbought = 70.0;
        }
        self.rsi_overbought = self.rsi_overbought.clamp(52.0, 95.0);
        if !self.rsi_exit_midline.is_finite() {
            self.rsi_exit_midline = 50.0;
        }
        self.rsi_exit_midline = self
            .rsi_exit_midline
            .clamp(self.rsi_oversold + 1.0, self.rsi_overbought - 1.0);

        if !self.rsi_confluence_buy.is_finite() {
            self.rsi_confluence_buy = 38.0;
        }
        self.rsi_confluence_buy = self.rsi_confluence_buy.clamp(5.0, 49.0);
        if !self.rsi_confluence_sell.is_finite() {
            self.rsi_confluence_sell = 62.0;
        }
        self.rsi_confluence_sell = self.rsi_confluence_sell.clamp(51.0, 95.0);

        self.macd_fast = self.macd_fast.clamp(2, 120);
        self.macd_slow = self.macd_slow.clamp(3, 250);
        if self.macd_slow <= self.macd_fast {
            self.macd_slow = (self.macd_fast + 1).min(250);
            if self.macd_fast >= self.macd_slow {
                self.macd_fast = self.macd_slow.saturating_sub(1).max(2);
            }
        }
        self.macd_signal = self.macd_signal.clamp(2, 120);
        if self.macd_trend_ema == 1 {
            self.macd_trend_ema = 2;
        } else if self.macd_trend_ema > 250 {
            self.macd_trend_ema = 250;
        }

        self.bb_period = self.bb_period.clamp(2, 200);
        if !self.bb_std_mult.is_finite() {
            self.bb_std_mult = 2.0;
        }
        self.bb_std_mult = self.bb_std_mult.clamp(0.5, 5.0);
        if !self.max_squeeze_bw_pct.is_finite() {
            self.max_squeeze_bw_pct = 12.0;
        }
        self.max_squeeze_bw_pct = self.max_squeeze_bw_pct.clamp(0.5, 100.0);

        self.donchian_entry_period = self.donchian_entry_period.clamp(2, 250);
        self.donchian_exit_period = self.donchian_exit_period.clamp(2, 250);
        self.atr_period = self.atr_period.clamp(2, 200);
    }

    #[must_use]
    pub fn summary_for_preset(&self, preset: StrategyPresetKind) -> String {
        match preset {
            StrategyPresetKind::SmaEmaTrendCross => {
                format!("SMA({}) / EMA({})", self.fast_sma, self.slow_ema)
            }
            StrategyPresetKind::RsiMeanReversion => format!(
                "RSI({}) {:.0}/{:.0}->Exit {:.0}",
                self.rsi_period, self.rsi_oversold, self.rsi_overbought, self.rsi_exit_midline
            ),
            StrategyPresetKind::MacdMomentum => {
                if self.macd_trend_ema >= 2 {
                    format!(
                        "MACD({},{},{})+EMA({})",
                        self.macd_fast, self.macd_slow, self.macd_signal, self.macd_trend_ema
                    )
                } else {
                    format!(
                        "MACD({},{},{})",
                        self.macd_fast, self.macd_slow, self.macd_signal
                    )
                }
            }
            StrategyPresetKind::BollingerSqueezeBreakout => format!(
                "BB({},{:.1}) Sqz<={:.0}%",
                self.bb_period, self.bb_std_mult, self.max_squeeze_bw_pct
            ),
            StrategyPresetKind::DonchianAtrBreakout => format!(
                "Donchian({}/{}) ATR({})",
                self.donchian_entry_period, self.donchian_exit_period, self.atr_period
            ),
            StrategyPresetKind::RsiBollingerConfluence => format!(
                "RSI({})<{:.0}/>{:.0} + BB({},{:.1})",
                self.rsi_period,
                self.rsi_confluence_buy,
                self.rsi_confluence_sell,
                self.bb_period,
                self.bb_std_mult
            ),
            StrategyPresetKind::CustomSignalRules => "Custom Signal Rules".to_owned(),
        }
    }
}

/// Protective stop-loss, take-profit, trailing-stop, and holding-period rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StopTakeConfig {
    pub stop_loss_pct: Option<f64>,
    pub stop_loss_atr_mult: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub take_profit_rr_mult: Option<f64>,
    pub trailing_stop_atr_mult: Option<f64>,
    pub max_holding_bars: Option<usize>,
    /// Minimum bars held before exits can trigger (`0` general, `2` for Vietnam `T+2` spot).
    pub min_holding_bars: usize,
}

impl Default for StopTakeConfig {
    fn default() -> Self {
        Self {
            stop_loss_pct: Some(5.0),
            stop_loss_atr_mult: None,
            take_profit_pct: Some(12.0),
            take_profit_rr_mult: None,
            trailing_stop_atr_mult: None,
            max_holding_bars: None,
            min_holding_bars: 0,
        }
    }
}

impl StopTakeConfig {
    pub fn sanitize(&mut self) {
        self.stop_loss_pct = sanitize_opt_range(self.stop_loss_pct, 0.1, 80.0);
        self.stop_loss_atr_mult = sanitize_opt_range(self.stop_loss_atr_mult, 0.2, 20.0);
        self.take_profit_pct = sanitize_opt_range(self.take_profit_pct, 0.1, 500.0);
        self.take_profit_rr_mult = sanitize_opt_range(self.take_profit_rr_mult, 0.2, 20.0);
        self.trailing_stop_atr_mult = sanitize_opt_range(self.trailing_stop_atr_mult, 0.2, 20.0);
        self.min_holding_bars = self.min_holding_bars.min(30);
        if let Some(max_h) = self.max_holding_bars {
            let clean = max_h.clamp(1, 10_000).max(self.min_holding_bars.max(1));
            self.max_holding_bars = Some(clean);
        }
    }
}

#[inline]
fn sanitize_opt_range(val: Option<f64>, min: f64, max: f64) -> Option<f64> {
    val.filter(|v| v.is_finite() && *v > 0.0)
        .map(|v| v.clamp(min, max))
}

/// Boolean combinator joining clauses within a group or groups within a [`DirectionalRuleLeg`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LogicalCombinator {
    #[default]
    AllAnd,
    AnyOr,
}

impl LogicalCombinator {
    pub const ALL: [Self; 2] = [Self::AllAnd, Self::AnyOr];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::AllAnd => "AND",
            Self::AnyOr => "OR",
        }
    }
}

/// Scalar comparison & crossover operator for [`SignalOutputType::Float`] catalog signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum FloatCmpOp {
    #[default]
    Gt,
    Gte,
    Lt,
    Lte,
    CrossAbove,
    CrossBelow,
}

impl FloatCmpOp {
    pub const ALL: [Self; 6] = [
        Self::Gt,
        Self::Gte,
        Self::Lt,
        Self::Lte,
        Self::CrossAbove,
        Self::CrossBelow,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::CrossAbove => "Cross ▲",
            Self::CrossBelow => "Cross ▼",
        }
    }
}

/// 6-bit mask over [`MarketRegime::ALL_FILTERABLE`] (`Uptrend=1<<0` .. `Neutral=1<<5`).
pub type RegimeBitmask = u8;

/// Type-checked predicate matched against a [`SIGNAL_CATALOG`] entry (`Milestone R16 / Wave 15`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignalPredicate {
    BoolIs(bool),
    FloatCmp { op: FloatCmpOp, threshold: f64 },
    RegimeIn { mask: RegimeBitmask },
}

impl Default for SignalPredicate {
    fn default() -> Self {
        Self::BoolIs(true)
    }
}

/// Single quality-gated `SIGNAL_CATALOG` clause inside a [`SignalClauseGroup`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SignalClause {
    pub catalog_id: String,
    pub predicate: SignalPredicate,
    /// Optional `Bullish`/`Bearish` filter for bidirectional `Bool` signals (`sr.prior_20b_breakout`,
    /// `structure.swing_bos`, `divergence.rsi_hidden_or_classic`) when `BoolIs(true)`.
    pub direction_filter: Option<SignalDirection>,
}

impl Default for SignalClause {
    fn default() -> Self {
        Self {
            catalog_id: "technical.macd_bull_cross".to_owned(),
            predicate: SignalPredicate::BoolIs(true),
            direction_filter: None,
        }
    }
}

impl SignalClause {
    /// Creates a well-typed default clause for `catalog_id` biased toward `preferred_dir`.
    #[must_use]
    pub fn default_for_catalog(catalog_id: &str, preferred_dir: Option<SignalDirection>) -> Self {
        let is_bear = preferred_dir == Some(SignalDirection::Bearish);
        let (predicate, direction_filter) = match find_catalog_entry(catalog_id) {
            Some(entry) => match entry.output_type {
                SignalOutputType::Bool => {
                    let df = if is_bidirectional_bool_signal(entry.id) {
                        preferred_dir
                    } else {
                        None
                    };
                    (SignalPredicate::BoolIs(true), df)
                }
                SignalOutputType::Float => {
                    let (op, threshold) = if entry.id == "volume.relative_volume" {
                        (FloatCmpOp::Gte, 2.0)
                    } else if is_bear {
                        (FloatCmpOp::Lte, -15.0)
                    } else {
                        (FloatCmpOp::Gte, 15.0)
                    };
                    (SignalPredicate::FloatCmp { op, threshold }, None)
                }
                SignalOutputType::Enum => {
                    let mask = if is_bear {
                        MarketRegime::Downtrend.bit()
                    } else {
                        MarketRegime::Uptrend.bit() | MarketRegime::Recovery.bit()
                    };
                    (SignalPredicate::RegimeIn { mask }, None)
                }
            },
            None => (SignalPredicate::BoolIs(true), None),
        };
        Self {
            catalog_id: catalog_id.to_owned(),
            predicate,
            direction_filter,
        }
    }

    pub fn sanitize(&mut self) {
        self.catalog_id = self.catalog_id.trim().to_owned();
        truncate_utf8_bytes(&mut self.catalog_id, MAX_RULE_NAME_BYTES);
        if let Some(entry) = find_catalog_entry(&self.catalog_id) {
            match entry.output_type {
                SignalOutputType::Bool => {
                    let expected = match self.predicate {
                        SignalPredicate::BoolIs(b) => b,
                        SignalPredicate::FloatCmp { .. } | SignalPredicate::RegimeIn { .. } => true,
                    };
                    self.predicate = SignalPredicate::BoolIs(expected);
                    if !expected || !is_bidirectional_bool_signal(entry.id) {
                        self.direction_filter = None;
                    }
                }
                SignalOutputType::Float => {
                    let (op, raw_thr) = match self.predicate {
                        SignalPredicate::FloatCmp { op, threshold } => (op, threshold),
                        SignalPredicate::BoolIs(b) => {
                            (FloatCmpOp::Gte, if b { 15.0 } else { -15.0 })
                        }
                        SignalPredicate::RegimeIn { .. } => (FloatCmpOp::Gte, 0.0),
                    };
                    let clean_thr = if raw_thr.is_finite() {
                        raw_thr.clamp(-1.0e12, 1.0e12)
                    } else {
                        0.0
                    };
                    self.predicate = SignalPredicate::FloatCmp {
                        op,
                        threshold: clean_thr,
                    };
                    self.direction_filter = None;
                }
                SignalOutputType::Enum => {
                    let mask = match self.predicate {
                        SignalPredicate::RegimeIn { mask } => mask & MarketRegime::ALL_VALID_MASK,
                        SignalPredicate::BoolIs(_) | SignalPredicate::FloatCmp { .. } => {
                            MarketRegime::Uptrend.bit()
                        }
                    };
                    self.predicate = SignalPredicate::RegimeIn { mask };
                    self.direction_filter = None;
                }
            }
        } else {
            // Unknown `catalog_id` from untrusted input: sanitize numeric payload & fail closed at eval.
            if let SignalPredicate::FloatCmp { threshold, .. } = &mut self.predicate {
                if !threshold.is_finite() {
                    *threshold = 0.0;
                } else {
                    *threshold = threshold.clamp(-1.0e12, 1.0e12);
                }
            }
            if let SignalPredicate::RegimeIn { mask } = &mut self.predicate {
                *mask &= MarketRegime::ALL_VALID_MASK;
            }
            self.direction_filter = None;
        }
    }

    #[must_use]
    pub fn short_summary(&self) -> String {
        let alias =
            find_catalog_entry(&self.catalog_id).map_or(self.catalog_id.as_str(), |e| e.ast_alias);
        match self.predicate {
            SignalPredicate::BoolIs(b) => match (b, self.direction_filter) {
                (true, Some(d)) => format!("{alias}[{}]", d.short_label()),
                (true, None) => alias.to_owned(),
                (false, _) => format!("!{alias}"),
            },
            SignalPredicate::FloatCmp { op, threshold } => {
                format!("{alias} {} {threshold:.1}", op.short_label())
            }
            SignalPredicate::RegimeIn { mask } => {
                let mut tags = Vec::new();
                for reg in MarketRegime::ALL_FILTERABLE {
                    if (mask & reg.bit()) != 0 {
                        tags.push(reg.short_label());
                    }
                }
                if tags.is_empty() {
                    format!("{alias} in ∅")
                } else {
                    format!("{alias} in [{}]", tags.join("|"))
                }
            }
        }
    }
}

/// Bounded group of [`SignalClause`] items (`<= MAX_CLAUSES_PER_GROUP`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SignalClauseGroup {
    pub intra_op: LogicalCombinator,
    pub clauses: Vec<SignalClause>,
}

impl SignalClauseGroup {
    pub fn sanitize(&mut self) {
        self.clauses.truncate(MAX_CLAUSES_PER_GROUP);
        for cl in &mut self.clauses {
            cl.sanitize();
        }
    }
}

/// Optional 4-family composite Technical Health / Confluence filter gate (`[-100, +100]`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfluenceThresholdGate {
    pub enabled: bool,
    pub profile: ConfluenceWeightProfile,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
    pub min_bullish_signals: u8,
    pub min_bearish_signals: u8,
}

impl Default for ConfluenceThresholdGate {
    fn default() -> Self {
        Self {
            enabled: false,
            profile: ConfluenceWeightProfile::Balanced,
            min_score: None,
            max_score: None,
            min_bullish_signals: 0,
            min_bearish_signals: 0,
        }
    }
}

impl ConfluenceThresholdGate {
    pub fn sanitize(&mut self) {
        self.min_score = self
            .min_score
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(-100.0, 100.0));
        self.max_score = self
            .max_score
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(-100.0, 100.0));
        if let (Some(lo), Some(hi)) = (self.min_score, self.max_score)
            && lo > hi
        {
            self.min_score = Some(hi);
            self.max_score = Some(lo);
        }
        let max_cnt = SIGNAL_CATALOG.len() as u8;
        self.min_bullish_signals = self.min_bullish_signals.min(max_cnt);
        self.min_bearish_signals = self.min_bearish_signals.min(max_cnt);
    }
}

/// Bounded 2-level rule leg (`long_entry`, `long_exit`, `short_entry`, or `short_exit`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DirectionalRuleLeg {
    pub enabled: bool,
    pub inter_group_op: LogicalCombinator,
    pub groups: Vec<SignalClauseGroup>,
    pub confluence_gate: ConfluenceThresholdGate,
}

impl DirectionalRuleLeg {
    pub fn sanitize(&mut self) {
        self.groups.truncate(MAX_RULE_GROUPS);
        for g in &mut self.groups {
            g.sanitize();
        }
        // Prevent vacuous truth on empty groups (`[].iter().all(...) == true`).
        self.groups.retain(|g| !g.clauses.is_empty());
        self.confluence_gate.sanitize();
    }

    #[must_use]
    pub fn clause_count(&self) -> usize {
        self.groups.iter().map(|g| g.clauses.len()).sum()
    }
}

/// Complete bounded, non-recursive user-authored Signal-Catalog Strategy Specification (`Milestone R16 / Wave 15`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomSignalRuleSpec {
    pub name: String,
    pub long_entry: DirectionalRuleLeg,
    pub long_exit: DirectionalRuleLeg,
    pub short_entry: DirectionalRuleLeg,
    pub short_exit: DirectionalRuleLeg,
}

impl Default for CustomSignalRuleSpec {
    fn default() -> Self {
        let mut spec = Self {
            name: "Confluence + MACD/BOS Rule".to_owned(),
            long_entry: DirectionalRuleLeg {
                enabled: true,
                inter_group_op: LogicalCombinator::AllAnd,
                groups: vec![SignalClauseGroup {
                    intra_op: LogicalCombinator::AnyOr,
                    clauses: vec![
                        SignalClause {
                            catalog_id: "technical.macd_bull_cross".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                        SignalClause {
                            catalog_id: "structure.swing_bos".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: Some(SignalDirection::Bullish),
                        },
                        SignalClause {
                            catalog_id: "technical.rsi_oversold_cross".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                    ],
                }],
                confluence_gate: ConfluenceThresholdGate {
                    enabled: true,
                    profile: ConfluenceWeightProfile::Balanced,
                    min_score: Some(12.0),
                    max_score: None,
                    min_bullish_signals: 1,
                    min_bearish_signals: 0,
                },
            },
            long_exit: DirectionalRuleLeg {
                enabled: true,
                inter_group_op: LogicalCombinator::AnyOr,
                groups: vec![SignalClauseGroup {
                    intra_op: LogicalCombinator::AnyOr,
                    clauses: vec![
                        SignalClause {
                            catalog_id: "technical.macd_bear_cross".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                        SignalClause {
                            catalog_id: "health.technical_composite".to_owned(),
                            predicate: SignalPredicate::FloatCmp {
                                op: FloatCmpOp::CrossBelow,
                                threshold: -10.0,
                            },
                            direction_filter: None,
                        },
                    ],
                }],
                confluence_gate: ConfluenceThresholdGate::default(),
            },
            short_entry: DirectionalRuleLeg {
                enabled: true,
                inter_group_op: LogicalCombinator::AllAnd,
                groups: vec![SignalClauseGroup {
                    intra_op: LogicalCombinator::AnyOr,
                    clauses: vec![
                        SignalClause {
                            catalog_id: "technical.macd_bear_cross".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                        SignalClause {
                            catalog_id: "structure.swing_bos".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: Some(SignalDirection::Bearish),
                        },
                        SignalClause {
                            catalog_id: "technical.rsi_overbought_cross".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                    ],
                }],
                confluence_gate: ConfluenceThresholdGate {
                    enabled: true,
                    profile: ConfluenceWeightProfile::Balanced,
                    min_score: None,
                    max_score: Some(-12.0),
                    min_bullish_signals: 0,
                    min_bearish_signals: 1,
                },
            },
            short_exit: DirectionalRuleLeg {
                enabled: true,
                inter_group_op: LogicalCombinator::AnyOr,
                groups: vec![SignalClauseGroup {
                    intra_op: LogicalCombinator::AnyOr,
                    clauses: vec![
                        SignalClause {
                            catalog_id: "technical.macd_bull_cross".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                        SignalClause {
                            catalog_id: "health.technical_composite".to_owned(),
                            predicate: SignalPredicate::FloatCmp {
                                op: FloatCmpOp::CrossAbove,
                                threshold: 10.0,
                            },
                            direction_filter: None,
                        },
                    ],
                }],
                confluence_gate: ConfluenceThresholdGate::default(),
            },
        };
        spec.sanitize();
        spec
    }
}

impl CustomSignalRuleSpec {
    pub fn sanitize(&mut self) {
        self.name = self.name.trim().to_owned();
        truncate_utf8_bytes(&mut self.name, MAX_RULE_NAME_BYTES);
        if self.name.is_empty() {
            self.name = "Custom Signal Rule".to_owned();
        }
        self.long_entry.sanitize();
        self.long_exit.sanitize();
        self.short_entry.sanitize();
        self.short_exit.sanitize();
    }

    #[must_use]
    pub fn short_summary(&self) -> String {
        format!(
            "{} ({}LE/{}LX/{}SE/{}SX)",
            self.name,
            self.long_entry.clause_count(),
            self.long_exit.clause_count(),
            self.short_entry.clause_count(),
            self.short_exit.clause_count()
        )
    }
}

/// Evaluates a single [`SignalClause`] at `bar_index` with strict fail-closed `SignalQuality::Valid` gating:
/// any `!is_valid()` quality (`InsufficientHistory`, `InvalidVolume`, `ZeroBaseline`, `UnclosedTailBar`)
/// returns `false` — **including** `BoolIs(false)`!
#[must_use]
pub fn evaluate_signal_clause_at(
    clause: &SignalClause,
    matrix: &PrecomputedCatalogMatrix,
    bar_index: usize,
) -> bool {
    evaluate_signal_clause_at_with_profile(
        clause,
        matrix,
        bar_index,
        ConfluenceWeightProfile::Balanced,
    )
}

/// Evaluates a single [`SignalClause`] at `bar_index` using `profile` for `"health.technical_composite"`.
#[must_use]
pub fn evaluate_signal_clause_at_with_profile(
    clause: &SignalClause,
    matrix: &PrecomputedCatalogMatrix,
    bar_index: usize,
    profile: ConfluenceWeightProfile,
) -> bool {
    let Some(cat_idx) = catalog_entry_index(&clause.catalog_id) else {
        return false;
    };
    let Some(pt) = matrix.point_at_with_profile(cat_idx, bar_index, profile) else {
        return false;
    };
    if !pt.quality.is_valid() {
        return false;
    }

    match clause.predicate {
        SignalPredicate::BoolIs(expected) => {
            if pt.bool_value != Some(expected) {
                return false;
            }
            if expected
                && let Some(req_dir) = clause.direction_filter
                && pt.direction != Some(req_dir)
            {
                return false;
            }
            true
        }
        SignalPredicate::FloatCmp { op, threshold } => {
            if !threshold.is_finite() {
                return false;
            }
            let Some(curr) = pt.float_value.filter(|v| v.is_finite()) else {
                return false;
            };
            match op {
                FloatCmpOp::Gt => curr > threshold,
                FloatCmpOp::Gte => curr >= threshold,
                FloatCmpOp::Lt => curr < threshold,
                FloatCmpOp::Lte => curr <= threshold,
                FloatCmpOp::CrossAbove => {
                    if bar_index == 0 {
                        return false;
                    }
                    let Some(prev_pt) =
                        matrix.point_at_with_profile(cat_idx, bar_index - 1, profile)
                    else {
                        return false;
                    };
                    if !prev_pt.quality.is_valid() {
                        return false;
                    }
                    let Some(prev) = prev_pt.float_value.filter(|v| v.is_finite()) else {
                        return false;
                    };
                    prev <= threshold && curr > threshold
                }
                FloatCmpOp::CrossBelow => {
                    if bar_index == 0 {
                        return false;
                    }
                    let Some(prev_pt) =
                        matrix.point_at_with_profile(cat_idx, bar_index - 1, profile)
                    else {
                        return false;
                    };
                    if !prev_pt.quality.is_valid() {
                        return false;
                    }
                    let Some(prev) = prev_pt.float_value.filter(|v| v.is_finite()) else {
                        return false;
                    };
                    prev >= threshold && curr < threshold
                }
            }
        }
        SignalPredicate::RegimeIn { mask } => {
            let regime = matrix.regime_at(bar_index);
            let bit = regime.bit();
            bit != 0 && ((mask & MarketRegime::ALL_VALID_MASK) & bit) != 0
        }
    }
}

/// Evaluates a [`DirectionalRuleLeg`] at `bar_index` (`false` when disabled or completely empty).
#[must_use]
pub fn evaluate_rule_leg_at(
    leg: &DirectionalRuleLeg,
    matrix: &PrecomputedCatalogMatrix,
    bar_index: usize,
) -> bool {
    if !leg.enabled || leg.groups.is_empty() {
        return false;
    }
    let profile = leg.confluence_gate.profile;
    let eval_group =
        |group: &SignalClauseGroup| -> bool {
            if group.clauses.is_empty() {
                return false;
            }
            match group.intra_op {
                LogicalCombinator::AllAnd => group.clauses.iter().all(|cl| {
                    evaluate_signal_clause_at_with_profile(cl, matrix, bar_index, profile)
                }),
                LogicalCombinator::AnyOr => group.clauses.iter().any(|cl| {
                    evaluate_signal_clause_at_with_profile(cl, matrix, bar_index, profile)
                }),
            }
        };
    let groups_ok = match leg.inter_group_op {
        LogicalCombinator::AllAnd => leg.groups.iter().all(eval_group),
        LogicalCombinator::AnyOr => leg.groups.iter().any(eval_group),
    };
    if !groups_ok {
        return false;
    }

    if leg.confluence_gate.enabled {
        let Ok(score) = matrix.health_score_at(bar_index, leg.confluence_gate.profile) else {
            return false;
        };
        if !score.is_finite() {
            return false;
        }
        if let Some(min_s) = leg.confluence_gate.min_score
            && score < min_s
        {
            return false;
        }
        if let Some(max_s) = leg.confluence_gate.max_score
            && score > max_s
        {
            return false;
        }
        let (bull_cnt, bear_cnt) = matrix.counts_at(bar_index, leg.confluence_gate.profile);
        if bull_cnt < leg.confluence_gate.min_bullish_signals {
            return false;
        }
        if bear_cnt < leg.confluence_gate.min_bearish_signals {
            return false;
        }
    }

    true
}

/// Evaluates `(long_entry, short_entry, long_exit, short_exit)` for `spec` at `bar_index`.
#[must_use]
pub fn evaluate_custom_rules_at_bar(
    spec: &CustomSignalRuleSpec,
    matrix: &PrecomputedCatalogMatrix,
    bar_index: usize,
) -> (bool, bool, bool, bool) {
    if bar_index >= matrix.bar_count {
        return (false, false, false, false);
    }
    (
        evaluate_rule_leg_at(&spec.long_entry, matrix, bar_index),
        evaluate_rule_leg_at(&spec.short_entry, matrix, bar_index),
        evaluate_rule_leg_at(&spec.long_exit, matrix, bar_index),
        evaluate_rule_leg_at(&spec.short_exit, matrix, bar_index),
    )
}

/// Counts causal bar matches `(long_entries, short_entries, long_exits, short_exits)` across `matrix`
/// (starting at `bar_index >= 1`, matching `evaluate_preset_signals_at_bar`).
#[must_use]
pub fn count_custom_rule_matches(
    spec: &CustomSignalRuleSpec,
    matrix: &PrecomputedCatalogMatrix,
) -> (usize, usize, usize, usize) {
    let mut le = 0_usize;
    let mut se = 0_usize;
    let mut lx = 0_usize;
    let mut sx = 0_usize;
    for i in 1..matrix.bar_count {
        let (b_en, s_en, l_ex, s_ex) = evaluate_custom_rules_at_bar(spec, matrix, i);
        if b_en {
            le += 1;
        }
        if s_en {
            se += 1;
        }
        if l_ex {
            lx += 1;
        }
        if s_ex {
            sx += 1;
        }
    }
    (le, se, lx, sx)
}

/// Complete deterministic configuration for a backtest or sweep execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BacktestConfig {
    pub preset: StrategyPresetKind,
    pub params: StrategyParams,
    pub custom_rules: CustomSignalRuleSpec,
    pub direction_mode: StrategyDirectionMode,
    pub sizing: PositionSizingMode,
    pub risk: StopTakeConfig,
    pub initial_cash: f64,
    pub commission_bps: f64,
    /// Optional sell-side securities tax in bps (`10.0` = `0.10%` for Vietnam spot equities).
    pub tax_sell_bps: f64,
    pub slippage_bps: f64,
    pub max_leverage: f64,
    /// Discrete lot-size step (`0.0` = fractional allowed; `1.0` = whole shares; `100.0` = HOSE board lot).
    pub lot_step: f64,
    /// Optional In-Sample split ratio (`0.50..=0.90`, e.g. `Some(0.70)` = `70% IS / 30% OOS`).
    pub in_sample_split_pct: Option<f64>,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            preset: StrategyPresetKind::SmaEmaTrendCross,
            params: StrategyParams::default(),
            custom_rules: CustomSignalRuleSpec::default(),
            direction_mode: StrategyDirectionMode::LongOnly,
            sizing: PositionSizingMode::PercentEquity(95.0),
            risk: StopTakeConfig::default(),
            initial_cash: 100_000.0,
            commission_bps: 10.0,
            tax_sell_bps: 0.0,
            slippage_bps: 5.0,
            max_leverage: 1.0,
            lot_step: 0.0,
            in_sample_split_pct: Some(0.70),
        }
    }
}

impl BacktestConfig {
    pub fn sanitize(&mut self) {
        self.params.sanitize();
        self.custom_rules.sanitize();
        self.sizing.sanitize();
        self.risk.sanitize();

        if !self.initial_cash.is_finite() {
            self.initial_cash = 100_000.0;
        }
        self.initial_cash = self.initial_cash.clamp(100.0, 1.0e12);

        if !self.commission_bps.is_finite() {
            self.commission_bps = 10.0;
        }
        self.commission_bps = self.commission_bps.clamp(0.0, 500.0);

        if !self.tax_sell_bps.is_finite() {
            self.tax_sell_bps = 0.0;
        }
        self.tax_sell_bps = self.tax_sell_bps.clamp(0.0, 500.0);

        if !self.slippage_bps.is_finite() {
            self.slippage_bps = 5.0;
        }
        self.slippage_bps = self.slippage_bps.clamp(0.0, 500.0);

        if !self.max_leverage.is_finite() {
            self.max_leverage = 1.0;
        }
        self.max_leverage = self.max_leverage.clamp(0.1, 20.0);

        if !self.lot_step.is_finite() || self.lot_step < 1.0 {
            self.lot_step = 0.0;
        } else {
            self.lot_step = self.lot_step.floor().min(100_000.0);
        }

        self.in_sample_split_pct = self
            .in_sample_split_pct
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(0.50, 0.90));
    }

    #[must_use]
    pub fn param_summary(&self) -> String {
        if self.preset == StrategyPresetKind::CustomSignalRules {
            self.custom_rules.short_summary()
        } else {
            self.params.summary_for_preset(self.preset)
        }
    }
}

/// Objective metric used to rank deterministic parameter sweep variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SweepRankingMetric {
    #[default]
    TotalReturnPct,
    SharpeRatio,
    ProfitFactor,
    CalmarRatio,
}

impl SweepRankingMetric {
    pub const ALL: [Self; 4] = [
        Self::TotalReturnPct,
        Self::SharpeRatio,
        Self::ProfitFactor,
        Self::CalmarRatio,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::TotalReturnPct => "Return%",
            Self::SharpeRatio => "Sharpe",
            Self::ProfitFactor => "PF",
            Self::CalmarRatio => "Calmar",
        }
    }
}

/// Builds a deterministic, domain-curated 12-variant 2D parameter sweep grid around `base`.
#[must_use]
pub fn build_preset_sweep_configs(base: &BacktestConfig) -> Vec<(String, BacktestConfig)> {
    let mut clean_base = base.clone();
    clean_base.sanitize();
    let mut out = Vec::with_capacity(12);

    match clean_base.preset {
        StrategyPresetKind::SmaEmaTrendCross => {
            for &fast in &[10_usize, 15, 20, 30] {
                for &slow in &[40_usize, 50, 80] {
                    if fast < slow {
                        let mut cfg = clean_base.clone();
                        cfg.params.fast_sma = fast;
                        cfg.params.slow_ema = slow;
                        cfg.sanitize();
                        out.push((format!("SMA({fast})/EMA({slow})"), cfg));
                    }
                }
            }
        }
        StrategyPresetKind::RsiMeanReversion => {
            for &period in &[7_usize, 14, 21] {
                for &(os, ob) in &[
                    (25.0_f64, 75.0_f64),
                    (30.0, 70.0),
                    (35.0, 65.0),
                    (28.0, 72.0),
                ] {
                    let mut cfg = clean_base.clone();
                    cfg.params.rsi_period = period;
                    cfg.params.rsi_oversold = os;
                    cfg.params.rsi_overbought = ob;
                    cfg.sanitize();
                    out.push((format!("RSI({period}) {os:.0}/{ob:.0}"), cfg));
                }
            }
        }
        StrategyPresetKind::MacdMomentum => {
            for &(fast, slow, sig) in &[(8_usize, 21_usize, 5_usize), (12, 26, 9), (16, 34, 9)] {
                for &trend in &[0_usize, 34, 50, 100] {
                    let mut cfg = clean_base.clone();
                    cfg.params.macd_fast = fast;
                    cfg.params.macd_slow = slow;
                    cfg.params.macd_signal = sig;
                    cfg.params.macd_trend_ema = trend;
                    cfg.sanitize();
                    let label = if trend == 0 {
                        format!("MACD({fast},{slow},{sig}) NoEMA")
                    } else {
                        format!("MACD({fast},{slow},{sig})+E{trend}")
                    };
                    out.push((label, cfg));
                }
            }
        }
        StrategyPresetKind::BollingerSqueezeBreakout => {
            for &period in &[15_usize, 20, 30] {
                for &(mult, bw) in &[(1.8_f64, 10.0_f64), (2.0, 12.0), (2.0, 16.0), (2.2, 14.0)] {
                    let mut cfg = clean_base.clone();
                    cfg.params.bb_period = period;
                    cfg.params.bb_std_mult = mult;
                    cfg.params.max_squeeze_bw_pct = bw;
                    cfg.sanitize();
                    out.push((format!("BB({period},{mult:.1})<={bw:.0}%"), cfg));
                }
            }
        }
        StrategyPresetKind::DonchianAtrBreakout => {
            for &entry_p in &[15_usize, 20, 30, 45] {
                for &exit_p in &[8_usize, 10, 15] {
                    let mut cfg = clean_base.clone();
                    cfg.params.donchian_entry_period = entry_p;
                    cfg.params.donchian_exit_period = exit_p;
                    cfg.sanitize();
                    out.push((format!("Donchian({entry_p}/{exit_p})"), cfg));
                }
            }
        }
        StrategyPresetKind::RsiBollingerConfluence => {
            for &rsi_p in &[10_usize, 14, 21] {
                for &(buy_lvl, sell_lvl, std_m) in &[
                    (32.0_f64, 68.0_f64, 2.0_f64),
                    (38.0, 62.0, 2.0),
                    (40.0, 60.0, 1.8),
                    (35.0, 65.0, 2.2),
                ] {
                    let mut cfg = clean_base.clone();
                    cfg.params.rsi_period = rsi_p;
                    cfg.params.rsi_confluence_buy = buy_lvl;
                    cfg.params.rsi_confluence_sell = sell_lvl;
                    cfg.params.bb_std_mult = std_m;
                    cfg.sanitize();
                    out.push((
                        format!("R{rsi_p}({buy_lvl:.0}/{sell_lvl:.0})+BB({std_m:.1})"),
                        cfg,
                    ));
                }
            }
        }
        StrategyPresetKind::CustomSignalRules => {
            let gates: [(&str, Option<(ConfluenceWeightProfile, f64)>); 4] = [
                ("BaseGate", None),
                (
                    "ConfBal>=10",
                    Some((ConfluenceWeightProfile::Balanced, 10.0)),
                ),
                (
                    "ConfBal>=25",
                    Some((ConfluenceWeightProfile::Balanced, 25.0)),
                ),
                (
                    "ConfTrd>=20",
                    Some((ConfluenceWeightProfile::TrendFocused, 20.0)),
                ),
            ];
            let risks: [(&str, Option<(f64, f64)>); 3] = [
                ("BaseSL/TP", None),
                ("SL3.5/TP9", Some((3.5, 9.0))),
                ("SL6.5/TP16", Some((6.5, 16.0))),
            ];
            for (g_lbl, gate_override) in gates {
                for (r_lbl, risk_override) in risks {
                    let mut cfg = clean_base.clone();
                    if let Some((prof, thr)) = gate_override {
                        if cfg.custom_rules.long_entry.enabled {
                            cfg.custom_rules.long_entry.confluence_gate.enabled = true;
                            cfg.custom_rules.long_entry.confluence_gate.profile = prof;
                            cfg.custom_rules.long_entry.confluence_gate.min_score = Some(thr);
                            cfg.custom_rules.long_entry.confluence_gate.max_score = None;
                        }
                        if cfg.custom_rules.short_entry.enabled {
                            cfg.custom_rules.short_entry.confluence_gate.enabled = true;
                            cfg.custom_rules.short_entry.confluence_gate.profile = prof;
                            cfg.custom_rules.short_entry.confluence_gate.min_score = None;
                            cfg.custom_rules.short_entry.confluence_gate.max_score = Some(-thr);
                        }
                    }
                    if let Some((sl_pct, tp_pct)) = risk_override {
                        cfg.risk.stop_loss_pct = Some(sl_pct);
                        cfg.risk.take_profit_pct = Some(tp_pct);
                    }
                    cfg.sanitize();
                    out.push((format!("{g_lbl}·{r_lbl}"), cfg));
                }
            }
        }
    }

    out.truncate(MAX_SWEEP_VARIANTS);
    out
}
