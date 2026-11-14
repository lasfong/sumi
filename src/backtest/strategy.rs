use serde::{Deserialize, Serialize};

use crate::trading::PositionSide;

/// Maximum parameter variants evaluated in a single deterministic sweep (`<= 64`).
pub const MAX_SWEEP_VARIANTS: usize = 64;

/// Canonical rule-based strategy presets available in Sumi Strategy Lab (`Milestone R09 / Wave 8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StrategyPresetKind {
    #[default]
    SmaEmaTrendCross,
    RsiMeanReversion,
    MacdMomentum,
    BollingerSqueezeBreakout,
    DonchianAtrBreakout,
    RsiBollingerConfluence,
}

impl StrategyPresetKind {
    pub const ALL: [Self; 6] = [
        Self::SmaEmaTrendCross,
        Self::RsiMeanReversion,
        Self::MacdMomentum,
        Self::BollingerSqueezeBreakout,
        Self::DonchianAtrBreakout,
        Self::RsiBollingerConfluence,
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

/// Complete deterministic configuration for a backtest or sweep execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BacktestConfig {
    pub preset: StrategyPresetKind,
    pub params: StrategyParams,
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
    }

    out.truncate(MAX_SWEEP_VARIANTS);
    out
}
