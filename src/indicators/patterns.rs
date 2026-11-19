//! Pure `f64`, `wasm32-unknown-unknown`-compatible Candlestick Geometry, `14` Causal Candlestick
//! Patterns, and Transparent Candle Structure Scoring (`Milestone R18 / Wave 17` — `B1`).
//!
//! Aligned with `ARCHIVE/backend/app/domain/signals/{candle_features,patterns,structure}.py` while
//! replacing fixed `1e-8` / `1e-6` thresholds with scale-relative floating-point guards so micro-priced
//! tokens (`< 1e-6`) behave identically to `VNINDEX`, `FPT`, `SPY`, and `BTCUSDT`.

use serde::{Deserialize, Serialize};

use crate::model::Candle;

/// Calculated geometric properties of a single OHLC candle bar (`SIG-PAT-001..010`, `SIG-STR-001`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandleGeometry {
    pub range: f64,
    pub body: f64,
    pub upper_wick: f64,
    pub lower_wick: f64,
    pub body_ratio: f64,
    pub upper_wick_ratio: f64,
    pub lower_wick_ratio: f64,
    pub close_location: f64,
    pub midpoint: f64,
    pub is_bullish: bool,
    pub is_bearish: bool,
    pub is_flat: bool,
    pub is_flat_range: bool,
    pub is_valid_ohlc: bool,
}

impl CandleGeometry {
    /// Extracts single-candle geometric features with strict NaN/Inf/inverted-OHLC and scale-relative
    /// zero-range protection.
    #[inline]
    #[must_use]
    pub fn from_candle(bar: &Candle) -> Self {
        let is_valid_ohlc = bar.open.is_finite()
            && bar.high.is_finite()
            && bar.low.is_finite()
            && bar.close.is_finite()
            && bar.high >= bar.low;
        if !is_valid_ohlc {
            return Self {
                range: 0.0,
                body: 0.0,
                upper_wick: 0.0,
                lower_wick: 0.0,
                body_ratio: 0.0,
                upper_wick_ratio: 0.0,
                lower_wick_ratio: 0.0,
                close_location: 0.5,
                midpoint: 0.0,
                is_bullish: false,
                is_bearish: false,
                is_flat: true,
                is_flat_range: true,
                is_valid_ohlc: false,
            };
        }

        let range = (bar.high - bar.low).max(0.0);
        let body = (bar.close - bar.open).abs();
        let body_hi = bar.open.max(bar.close);
        let body_lo = bar.open.min(bar.close);
        let upper_wick = (bar.high - body_hi).max(0.0);
        let lower_wick = (body_lo - bar.low).max(0.0);
        let midpoint = 0.5 * bar.open + 0.5 * bar.close;

        let scale_eps = 1.0e-12 * (bar.high.abs() + bar.low.abs()).max(1.0e-30);
        let is_flat_range = range <= scale_eps;
        let is_bullish = bar.close > bar.open;
        let is_bearish = bar.close < bar.open;
        let is_flat = bar.close == bar.open;

        let (body_ratio, upper_wick_ratio, lower_wick_ratio, close_location) = if is_flat_range {
            (0.0, 0.0, 0.0, 0.5)
        } else {
            (
                (body / range).clamp(0.0, 1.0),
                (upper_wick / range).clamp(0.0, 1.0),
                (lower_wick / range).clamp(0.0, 1.0),
                ((bar.close - bar.low) / range).clamp(0.0, 1.0),
            )
        };

        Self {
            range,
            body,
            upper_wick,
            lower_wick,
            body_ratio,
            upper_wick_ratio,
            lower_wick_ratio,
            close_location,
            midpoint,
            is_bullish,
            is_bearish,
            is_flat,
            is_flat_range,
            is_valid_ohlc: true,
        }
    }

    /// Computes transparent deterministic Candle Structure Score in `[-100.0, +100.0]` (`SIG-STR-001`).
    ///
    /// Returns `None` iff `!self.is_valid_ohlc`; returns `Some(0.0)` on flat zero-range candles.
    #[inline]
    #[must_use]
    pub fn structure_score(&self, w_direction: f64, w_close: f64, w_wick: f64) -> Option<f64> {
        if !self.is_valid_ohlc {
            return None;
        }
        if self.is_flat_range {
            return Some(0.0);
        }
        let total_w = w_direction + w_close + w_wick;
        let (nw_dir, nw_close, nw_wick) = if !total_w.is_finite() || total_w <= 1.0e-12 {
            (0.45, 0.35, 0.20)
        } else {
            (w_direction / total_w, w_close / total_w, w_wick / total_w)
        };
        let direction = if self.is_bullish {
            1.0
        } else if self.is_bearish {
            -1.0
        } else {
            0.0
        };
        let close_score = 2.0 * self.close_location - 1.0;
        let wick_balance = ((self.lower_wick - self.upper_wick) / self.range).clamp(-1.0, 1.0);
        let raw =
            nw_dir * direction * self.body_ratio + nw_close * close_score + nw_wick * wick_balance;
        Some((100.0 * raw.clamp(-1.0, 1.0)).clamp(-100.0, 100.0))
    }
}

/// Canonical `12` atomic causal candlestick patterns (`SIG-PAT-001..009`), aggregated into
/// `pattern.any_bullish` and `pattern.any_bearish` (`SIG-PAT-010` -> `14` total pattern entries).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandlestickPatternKind {
    BullishEngulfing,
    BearishEngulfing,
    Hammer,
    ShootingStar,
    MorningStar,
    EveningStar,
    PiercingLine,
    DarkCloudCover,
    TweezerBottom,
    TweezerTop,
    InsideBarBreakoutUp,
    InsideBarBreakoutDown,
}

impl CandlestickPatternKind {
    pub const ALL: [Self; 12] = [
        Self::BullishEngulfing,
        Self::BearishEngulfing,
        Self::Hammer,
        Self::ShootingStar,
        Self::MorningStar,
        Self::EveningStar,
        Self::PiercingLine,
        Self::DarkCloudCover,
        Self::TweezerBottom,
        Self::TweezerTop,
        Self::InsideBarBreakoutUp,
        Self::InsideBarBreakoutDown,
    ];

    pub const BULLISH: [Self; 6] = [
        Self::BullishEngulfing,
        Self::Hammer,
        Self::MorningStar,
        Self::PiercingLine,
        Self::TweezerBottom,
        Self::InsideBarBreakoutUp,
    ];

    pub const BEARISH: [Self; 6] = [
        Self::BearishEngulfing,
        Self::ShootingStar,
        Self::EveningStar,
        Self::DarkCloudCover,
        Self::TweezerTop,
        Self::InsideBarBreakoutDown,
    ];

    #[inline]
    #[must_use]
    pub const fn bit(self) -> u16 {
        match self {
            Self::BullishEngulfing => 1 << 0,
            Self::BearishEngulfing => 1 << 1,
            Self::Hammer => 1 << 2,
            Self::ShootingStar => 1 << 3,
            Self::MorningStar => 1 << 4,
            Self::EveningStar => 1 << 5,
            Self::PiercingLine => 1 << 6,
            Self::DarkCloudCover => 1 << 7,
            Self::TweezerBottom => 1 << 8,
            Self::TweezerTop => 1 << 9,
            Self::InsideBarBreakoutUp => 1 << 10,
            Self::InsideBarBreakoutDown => 1 << 11,
        }
    }

    #[inline]
    #[must_use]
    pub const fn catalog_id(self) -> &'static str {
        match self {
            Self::BullishEngulfing => "pattern.bullish_engulfing",
            Self::BearishEngulfing => "pattern.bearish_engulfing",
            Self::Hammer => "pattern.hammer",
            Self::ShootingStar => "pattern.shooting_star",
            Self::MorningStar => "pattern.morning_star",
            Self::EveningStar => "pattern.evening_star",
            Self::PiercingLine => "pattern.piercing_line",
            Self::DarkCloudCover => "pattern.dark_cloud_cover",
            Self::TweezerBottom => "pattern.tweezer_bottom",
            Self::TweezerTop => "pattern.tweezer_top",
            Self::InsideBarBreakoutUp => "pattern.inside_bar_breakout_up",
            Self::InsideBarBreakoutDown => "pattern.inside_bar_breakout_down",
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_snake_name(self) -> &'static str {
        match self {
            Self::BullishEngulfing => "bullish_engulfing",
            Self::BearishEngulfing => "bearish_engulfing",
            Self::Hammer => "hammer",
            Self::ShootingStar => "shooting_star",
            Self::MorningStar => "morning_star",
            Self::EveningStar => "evening_star",
            Self::PiercingLine => "piercing_line",
            Self::DarkCloudCover => "dark_cloud_cover",
            Self::TweezerBottom => "tweezer_bottom",
            Self::TweezerTop => "tweezer_top",
            Self::InsideBarBreakoutUp => "inside_bar_breakout_up",
            Self::InsideBarBreakoutDown => "inside_bar_breakout_down",
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_badge(self) -> &'static str {
        match self {
            Self::BullishEngulfing => "BE+",
            Self::BearishEngulfing => "BE-",
            Self::Hammer => "HMR",
            Self::ShootingStar => "SSR",
            Self::MorningStar => "MST",
            Self::EveningStar => "EST",
            Self::PiercingLine => "PRC",
            Self::DarkCloudCover => "DCC",
            Self::TweezerBottom => "TWB",
            Self::TweezerTop => "TWT",
            Self::InsideBarBreakoutUp => "IBU",
            Self::InsideBarBreakoutDown => "IBD",
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_bullish(self) -> bool {
        matches!(
            self,
            Self::BullishEngulfing
                | Self::Hammer
                | Self::MorningStar
                | Self::PiercingLine
                | Self::TweezerBottom
                | Self::InsideBarBreakoutUp
        )
    }

    #[inline]
    #[must_use]
    pub const fn is_bearish(self) -> bool {
        !self.is_bullish()
    }

    #[inline]
    #[must_use]
    pub const fn warmup_bars(self) -> usize {
        match self {
            Self::Hammer | Self::ShootingStar => 0,
            Self::BullishEngulfing
            | Self::BearishEngulfing
            | Self::PiercingLine
            | Self::DarkCloudCover => 1,
            Self::MorningStar | Self::EveningStar => 2,
            Self::TweezerBottom
            | Self::TweezerTop
            | Self::InsideBarBreakoutUp
            | Self::InsideBarBreakoutDown => 14,
        }
    }
}

/// Compact per-bar evaluation of all `12` atomic candlestick patterns, `valid_mask`, and
/// `structure.candle_score` (`0` heap allocations per bar).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CandlePatternBarSummary {
    /// Bitmask of [`CandlestickPatternKind::bit`] whose causal history & ATR inputs were valid at bar `t`.
    pub valid_mask: u16,
    /// Bitmask of [`CandlestickPatternKind::bit`] that fired `true` at bar `t`.
    pub active_mask: u16,
    /// Highest-priority pattern badge to render on the main candlestick chart, if any.
    pub primary_badge: Option<CandlestickPatternKind>,
    /// `structure.candle_score` in `[-100.0, +100.0]`, or `None` if current bar OHLC is invalid.
    pub candle_score: Option<f64>,
}

impl CandlePatternBarSummary {
    /// Evaluates a single [`CandlestickPatternKind`]: returns `Some(true/false)` when valid, or `None`
    /// during warmup / invalid OHLC / invalid ATR.
    #[inline]
    #[must_use]
    pub const fn pattern_result(self, kind: CandlestickPatternKind) -> Option<bool> {
        let b = kind.bit();
        if (self.valid_mask & b) == 0 {
            None
        } else {
            Some((self.active_mask & b) != 0)
        }
    }

    #[inline]
    #[must_use]
    pub fn any_bullish(self) -> Option<bool> {
        self.candle_score?;
        let bull_bits = CandlestickPatternKind::BullishEngulfing.bit()
            | CandlestickPatternKind::Hammer.bit()
            | CandlestickPatternKind::MorningStar.bit()
            | CandlestickPatternKind::PiercingLine.bit()
            | CandlestickPatternKind::TweezerBottom.bit()
            | CandlestickPatternKind::InsideBarBreakoutUp.bit();
        Some((self.active_mask & bull_bits) != 0)
    }

    #[inline]
    #[must_use]
    pub fn any_bearish(self) -> Option<bool> {
        self.candle_score?;
        let bear_bits = CandlestickPatternKind::BearishEngulfing.bit()
            | CandlestickPatternKind::ShootingStar.bit()
            | CandlestickPatternKind::EveningStar.bit()
            | CandlestickPatternKind::DarkCloudCover.bit()
            | CandlestickPatternKind::TweezerTop.bit()
            | CandlestickPatternKind::InsideBarBreakoutDown.bit();
        Some((self.active_mask & bear_bits) != 0)
    }
}

/// Evaluates [`CandleGeometry`], all `14` candlestick patterns, and `structure.candle_score` across
/// `candles` in a single $O(N)$ pass using `atr_14` (`len == candles.len()`).
#[must_use]
pub fn compute_candlestick_patterns_and_scores(
    candles: &[Candle],
    atr_14: &[Option<f64>],
) -> (Vec<CandleGeometry>, Vec<CandlePatternBarSummary>) {
    let n = candles.len();
    let mut geoms = Vec::with_capacity(n);
    for c in candles {
        geoms.push(CandleGeometry::from_candle(c));
    }
    let mut summaries = Vec::with_capacity(n);

    // Priority order for main-pane badge rendering when multiple patterns co-occur on bar `t`
    // (multi-bar structural reversals take visual priority over single-bar pinbars).
    const BADGE_PRIORITY: [CandlestickPatternKind; 12] = [
        CandlestickPatternKind::MorningStar,
        CandlestickPatternKind::EveningStar,
        CandlestickPatternKind::InsideBarBreakoutUp,
        CandlestickPatternKind::InsideBarBreakoutDown,
        CandlestickPatternKind::BullishEngulfing,
        CandlestickPatternKind::BearishEngulfing,
        CandlestickPatternKind::PiercingLine,
        CandlestickPatternKind::DarkCloudCover,
        CandlestickPatternKind::TweezerBottom,
        CandlestickPatternKind::TweezerTop,
        CandlestickPatternKind::Hammer,
        CandlestickPatternKind::ShootingStar,
    ];

    for t in 0..n {
        let g = geoms[t];
        if !g.is_valid_ohlc {
            summaries.push(CandlePatternBarSummary::default());
            continue;
        }
        let c = &candles[t];
        let candle_score = g.structure_score(0.45, 0.35, 0.20);
        let mut valid_mask = 0_u16;
        let mut active_mask = 0_u16;

        // 1. Single-bar patterns (`warmup = 0`): Hammer & Shooting Star
        valid_mask |=
            CandlestickPatternKind::Hammer.bit() | CandlestickPatternKind::ShootingStar.bit();
        if !g.is_flat_range {
            let body_floor = (1.0e-6 * g.range).max(1.0e-30);
            let eff_body = g.body.max(body_floor);
            if g.lower_wick >= 2.0 * eff_body
                && g.upper_wick_ratio <= 0.25
                && g.close_location >= 0.65
                && g.body_ratio <= 0.50
            {
                active_mask |= CandlestickPatternKind::Hammer.bit();
            }
            if g.upper_wick >= 2.0 * eff_body
                && g.lower_wick_ratio <= 0.25
                && g.close_location <= 0.35
                && g.body_ratio <= 0.50
            {
                active_mask |= CandlestickPatternKind::ShootingStar.bit();
            }
        }

        // 2. Two-bar patterns (`warmup = 1`): Engulfing & Piercing / Dark Cloud
        if t >= 1 && geoms[t - 1].is_valid_ohlc {
            let gp = geoms[t - 1];
            let cp = &candles[t - 1];
            valid_mask |= CandlestickPatternKind::BullishEngulfing.bit()
                | CandlestickPatternKind::BearishEngulfing.bit()
                | CandlestickPatternKind::PiercingLine.bit()
                | CandlestickPatternKind::DarkCloudCover.bit();

            if gp.is_bearish
                && g.is_bullish
                && c.open <= cp.close
                && c.close >= cp.open
                && g.body_ratio >= 0.55
                && g.close_location >= 0.70
            {
                active_mask |= CandlestickPatternKind::BullishEngulfing.bit();
            }
            if gp.is_bullish
                && g.is_bearish
                && c.open >= cp.close
                && c.close <= cp.open
                && g.body_ratio >= 0.55
                && g.close_location <= 0.30
            {
                active_mask |= CandlestickPatternKind::BearishEngulfing.bit();
            }
            if gp.is_bearish
                && g.is_bullish
                && c.open <= cp.close
                && c.close >= gp.midpoint
                && c.close < cp.open
                && g.close_location >= 0.60
            {
                active_mask |= CandlestickPatternKind::PiercingLine.bit();
            }
            if gp.is_bullish
                && g.is_bearish
                && c.open >= cp.close
                && c.close <= gp.midpoint
                && c.close > cp.open
                && g.close_location <= 0.40
            {
                active_mask |= CandlestickPatternKind::DarkCloudCover.bit();
            }
        }

        // 3. Three-bar patterns (`warmup = 2`): Morning Star & Evening Star
        if t >= 2 && geoms[t - 2].is_valid_ohlc && geoms[t - 1].is_valid_ohlc {
            let g0 = geoms[t - 2];
            let g1 = geoms[t - 1];
            valid_mask |= CandlestickPatternKind::MorningStar.bit()
                | CandlestickPatternKind::EveningStar.bit();
            let g0_body_ref = g0.body.max((1.0e-6 * g0.range).max(1.0e-30));

            if g0.is_bearish
                && g0.body_ratio >= 0.55
                && g1.body <= 0.40 * g0_body_ref
                && g.is_bullish
                && c.close >= g0.midpoint
                && g.close_location >= 0.65
            {
                active_mask |= CandlestickPatternKind::MorningStar.bit();
            }
            if g0.is_bullish
                && g0.body_ratio >= 0.55
                && g1.body <= 0.40 * g0_body_ref
                && g.is_bearish
                && c.close <= g0.midpoint
                && g.close_location <= 0.35
            {
                active_mask |= CandlestickPatternKind::EveningStar.bit();
            }
        }

        // 4. ATR(14)-normalized patterns (`warmup = 14`): Tweezers & Inside-Bar Breakouts
        if t >= 14
            && let Some(atr) = atr_14.get(t).copied().flatten()
        {
            let atr_floor = 1.0e-12 * (c.high.abs() + c.low.abs()).max(1.0e-30);
            if atr.is_finite() && atr > atr_floor {
                if geoms[t - 1].is_valid_ohlc {
                    let gp = geoms[t - 1];
                    let cp = &candles[t - 1];
                    valid_mask |= CandlestickPatternKind::TweezerBottom.bit()
                        | CandlestickPatternKind::TweezerTop.bit();
                    if (c.low - cp.low).abs() <= 0.15 * atr
                        && (gp.is_bearish || gp.is_flat)
                        && g.is_bullish
                    {
                        active_mask |= CandlestickPatternKind::TweezerBottom.bit();
                    }
                    if (c.high - cp.high).abs() <= 0.15 * atr
                        && (gp.is_bullish || gp.is_flat)
                        && g.is_bearish
                    {
                        active_mask |= CandlestickPatternKind::TweezerTop.bit();
                    }
                }
                if geoms[t - 2].is_valid_ohlc && geoms[t - 1].is_valid_ohlc {
                    let mother = &candles[t - 2];
                    let inside = &candles[t - 1];
                    valid_mask |= CandlestickPatternKind::InsideBarBreakoutUp.bit()
                        | CandlestickPatternKind::InsideBarBreakoutDown.bit();
                    let is_inside = inside.high < mother.high && inside.low > mother.low;
                    if is_inside && c.close > mother.high + 0.05 * atr {
                        active_mask |= CandlestickPatternKind::InsideBarBreakoutUp.bit();
                    }
                    if is_inside && c.close < mother.low - 0.05 * atr {
                        active_mask |= CandlestickPatternKind::InsideBarBreakoutDown.bit();
                    }
                }
            }
        }

        let primary_badge = BADGE_PRIORITY
            .iter()
            .copied()
            .find(|k| (active_mask & k.bit()) != 0);

        summaries.push(CandlePatternBarSummary {
            valid_mask,
            active_mask,
            primary_badge,
            candle_score,
        });
    }

    (geoms, summaries)
}
