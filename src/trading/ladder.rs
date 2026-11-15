//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Synthetic Depth-of-Market (`DOM`) /
//! Footprint Price Ladder & Click-to-Stage Execution module (`Milestone R14 / Wave 13`).
//!
//! Strictly adheres to `DEF-001` Honest Proxy Labeling (`OHLCV_SYNTH_DOM_PROXY`), strict
//! Bar Replay causality (`timestamp <= causal_cutoff_ts`), and Vietnam / Global exchange
//! microstructure rules (`tick_size_at_price`, `step_one_tick_above`/`below`,
//! `compute_daily_price_band`, `board_lot`, `allows_short`).

use core::fmt;
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::journal::{
    DailyPriceBand, ExchangeVenue, ResolvedVnPriceUnit, TradePlanInput, VnPriceUnit,
    compute_daily_price_band_with_resolved_unit, compute_trade_plan, floor_to_board_lot,
    round_down_to_tick, round_up_to_tick, snap_to_nearest_tick, step_one_tick_above,
    step_one_tick_below, tick_size_at_price,
};
use crate::model::Candle;

use super::orders::{
    Order, OrderKind, OrderRequest, OrderSide, OrderStatus, PositionSide, apply_slippage,
    validate_brackets_for_side,
};
use super::portfolio::Position;

/// Honest provenance badge stamped on every synthetic OHLCV-derived DOM ladder snapshot (`DEF-001`).
pub const DOM_LADDER_DATA_FIDELITY: &str = "OHLCV_SYNTH_DOM_PROXY";

const MAX_HALF_ROWS: usize = 30;
const MAX_LOOKBACK_BARS: usize = 500;
const PRICE_EQ_EPS: f64 = 1.0e-7;

/// Statutory daily price band zone classification for a DOM ladder price level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DomPriceBandState {
    AboveCeiling,
    Ceiling,
    Reference,
    Floor,
    BelowFloor,
    #[default]
    Normal,
}

impl DomPriceBandState {
    #[inline]
    #[must_use]
    pub const fn short_tag(self) -> Option<&'static str> {
        match self {
            Self::AboveCeiling => Some(">CEIL"),
            Self::Ceiling => Some("CEIL"),
            Self::Reference => Some("REF"),
            Self::Floor => Some("FLR"),
            Self::BelowFloor => Some("<FLR"),
            Self::Normal => None,
        }
    }
}

/// Configuration parameters for constructing a causal DOM / Footprint price ladder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DomLadderConfig {
    pub venue: ExchangeVenue,
    pub vn_unit: VnPriceUnit,
    /// Number of price levels above AND below `center_price` (`total rows = 2 * half_rows + 1`, default `10` -> `21` rows).
    pub half_rows: usize,
    /// Trailing causal OHLCV bars aggregated into per-level buy/sell footprint volume (`default = 120`, clamped `1..=500`).
    pub lookback_bars: usize,
    /// Optional explicit scroll-center override (snapped to valid venue ticks); reset on symbol/source change.
    pub center_price_override: Option<f64>,
    /// Optional explicit reference price override for daily `CEIL`/`REF`/`FLOOR` bands.
    pub reference_price_override: Option<f64>,
}

impl Default for DomLadderConfig {
    fn default() -> Self {
        Self {
            venue: ExchangeVenue::Hose,
            vn_unit: VnPriceUnit::Auto,
            half_rows: 10,
            lookback_bars: 120,
            center_price_override: None,
            reference_price_override: None,
        }
    }
}

/// Single price rung on the DOM / Footprint ladder ordered descending (highest price first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomLadderRow {
    pub price: f64,
    pub tick_size: f64,
    pub band_state: DomPriceBandState,
    pub is_at_mark: bool,
    pub is_at_position_entry: bool,
    /// Synthetic OHLCV proxy resting bid depth (`0.0` at/above mark or outside daily price band).
    pub synth_bid_depth: f64,
    /// Synthetic OHLCV proxy resting ask depth (`0.0` at/below mark or outside daily price band).
    pub synth_ask_depth: f64,
    /// Causal `CLV`-attributed aggressive buy volume over the lookback window.
    pub traded_buy_volume: f64,
    /// Causal `CLV`-attributed aggressive sell volume over the lookback window.
    pub traded_sell_volume: f64,
    /// Signed volume delta (`traded_buy_volume - traded_sell_volume`).
    pub net_delta: f64,
    /// Sum of `Pending` buy order quantities staged at this tick level.
    pub working_buy_qty: f64,
    /// Sum of `Pending` sell order quantities staged at this tick level.
    pub working_sell_qty: f64,
    /// Working `Pending` order IDs mapped to this row (enables 1-click level cancellation).
    pub working_order_ids: Vec<u64>,
}

/// Complete causal DOM / Footprint ladder snapshot for rendering and click-to-stage execution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DomLadderSnapshot {
    pub data_fidelity: &'static str,
    pub venue: ExchangeVenue,
    pub resolved_unit: ResolvedVnPriceUnit,
    pub price_decimals: usize,
    pub mark_price: f64,
    pub center_price: f64,
    pub daily_band: Option<DailyPriceBand>,
    pub rows: Vec<DomLadderRow>,
    pub max_row_depth: f64,
    pub max_row_volume: f64,
    pub cumulative_buy_volume: f64,
    pub cumulative_sell_volume: f64,
    /// Aggregate buy share `[0.0, 100.0]`, defaulting to `50.0` when causal lookback volume is `0.0`.
    pub bid_ask_imbalance_pct: f64,
    pub poc_price: Option<f64>,
    pub causal_bar_count: usize,
    pub last_causal_timestamp: Option<i64>,
}

/// Parameters required to validate and convert a DOM cell click into a paper-trading [`OrderRequest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LadderStageInput {
    pub side: OrderSide,
    pub clicked_price: f64,
    pub mark_price: f64,
    pub quantity: f64,
    pub venue: ExchangeVenue,
    pub resolved_unit: ResolvedVnPriceUnit,
    pub daily_band: Option<DailyPriceBand>,
    pub stop_loss_ticks: Option<u32>,
    pub take_profit_ticks: Option<u32>,
    pub open_position: Option<Position>,
    pub allow_account_short: bool,
}

/// Structured validation errors emitted during DOM ladder construction or click-to-stage execution.
#[derive(Debug, Clone, PartialEq)]
pub enum DomLadderError {
    EmptyCausalSeries,
    NonFiniteInput(&'static str),
    InvalidConfig(&'static str),
    InvalidQuantity {
        requested: f64,
        board_lot: f64,
    },
    OutsideDailyPriceBand {
        price: f64,
        floor: f64,
        ceiling: f64,
        decimals: usize,
    },
    ShortNotAllowedByVenue {
        venue: ExchangeVenue,
        requested_sell: f64,
        closable_long: f64,
    },
    InvalidBracketPrice(&'static str),
}

impl fmt::Display for DomLadderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCausalSeries => {
                write!(
                    f,
                    "no causal market candles available at or before replay cutoff"
                )
            }
            Self::NonFiniteInput(field) => {
                write!(f, "non-finite or non-positive ladder input for {field}")
            }
            Self::InvalidConfig(reason) => {
                write!(f, "invalid DOM ladder configuration: {reason}")
            }
            Self::InvalidQuantity {
                requested,
                board_lot,
            } => {
                write!(
                    f,
                    "order quantity {requested} snaps below venue minimum board lot {board_lot}"
                )
            }
            Self::OutsideDailyPriceBand {
                price,
                floor,
                ceiling,
                decimals,
            } => {
                let dec = *decimals;
                write!(
                    f,
                    "price {price:.dec$} lies outside statutory daily band [{floor:.dec$}, {ceiling:.dec$}]"
                )
            }
            Self::ShortNotAllowedByVenue {
                venue,
                requested_sell,
                closable_long,
            } => {
                write!(
                    f,
                    "short selling not allowed on {} (sell {requested_sell:.2} exceeds closable long {closable_long:.2})",
                    venue.short_label()
                )
            }
            Self::InvalidBracketPrice(reason) => {
                write!(f, "invalid DOM bracket configuration: {reason}")
            }
        }
    }
}

#[inline]
fn is_causal_valid_bar(c: &Candle, causal_cutoff_ts: Option<i64>) -> bool {
    c.is_valid()
        && c.close > 0.0
        && c.open > 0.0
        && causal_cutoff_ts.is_none_or(|cut| c.timestamp <= cut)
}

#[inline]
fn local_trading_day(timestamp: i64, venue: ExchangeVenue) -> i64 {
    timestamp
        .saturating_add(venue.utc_offset_seconds())
        .div_euclid(86_400)
}

/// Infers the causal prior local trading day's closing reference price strictly at or before
/// `causal_cutoff_ts` using `venue.utc_offset_seconds()`, falling back to the first causal
/// bar's open when all causal bars belong to a single local trading day.
#[must_use]
pub fn infer_causal_prior_session_close(
    candles: &[Candle],
    venue: ExchangeVenue,
    causal_cutoff_ts: Option<i64>,
) -> Option<f64> {
    let mut first_open: Option<f64> = None;
    let mut prev_day_last_close: Option<f64> = None;
    let mut cur_day: Option<i64> = None;
    let mut cur_day_last_close: Option<f64> = None;

    for c in candles {
        if !is_causal_valid_bar(c, causal_cutoff_ts) {
            continue;
        }
        if first_open.is_none() {
            first_open = Some(c.open);
        }
        let day = local_trading_day(c.timestamp, venue);
        match cur_day {
            None => {
                cur_day = Some(day);
                cur_day_last_close = Some(c.close);
            }
            Some(prev) if day != prev => {
                prev_day_last_close = cur_day_last_close;
                cur_day = Some(day);
                cur_day_last_close = Some(c.close);
            }
            Some(_) => {
                cur_day_last_close = Some(c.close);
            }
        }
    }

    prev_day_last_close.or(first_open)
}

#[inline]
fn decimals_for_venue_ladder_tick(
    venue: ExchangeVenue,
    tick: f64,
    resolved_unit: ResolvedVnPriceUnit,
) -> usize {
    let min_dec = match venue {
        ExchangeVenue::Vn30Futures => 1,
        ExchangeVenue::Hose | ExchangeVenue::Hnx | ExchangeVenue::Upcom => match resolved_unit {
            ResolvedVnPriceUnit::RawVnd => 0,
            ResolvedVnPriceUnit::ThousandVnd => 2,
        },
        ExchangeVenue::VnIndexBenchmark
        | ExchangeVenue::UsEquities
        | ExchangeVenue::CryptoSpotPerp => 2,
    };
    if !tick.is_finite() || tick <= 0.0 {
        return min_dec;
    }
    let mut d = if tick >= 1.0 {
        0
    } else {
        (-tick.log10().floor()).max(0.0) as usize
    };
    while d < 8 {
        let scaled = tick * 10_f64.powi(d as i32);
        if (scaled - scaled.round()).abs() <= 1.0e-5 {
            break;
        }
        d += 1;
    }
    d.clamp(min_dec, 8)
}

#[inline]
fn integrate_piecewise_ticks(lo: f64, hi: f64, lower_tiers: &[(f64, f64)], top_tick: f64) -> f64 {
    if !lo.is_finite() || !hi.is_finite() || hi <= lo {
        return 0.0;
    }
    let mut cursor = lo;
    let mut ticks = 0.0;
    for &(bound, step) in lower_tiers {
        if cursor < bound && hi > cursor {
            let seg_hi = hi.min(bound);
            if seg_hi > cursor && step > 0.0 {
                ticks += (seg_hi - cursor) / step;
            }
            cursor = seg_hi;
        }
    }
    if hi > cursor && top_tick > 0.0 {
        ticks += (hi - cursor) / top_tick;
    }
    ticks
}

/// Computes `O(1)` exchange tick distance between `a` and `b` across piecewise tick tiers,
/// preventing proximity weights from pinning at ladder edges when scrolled far from mark price.
#[inline]
#[must_use]
pub fn exact_tick_distance(
    a: f64,
    b: f64,
    venue: ExchangeVenue,
    resolved_unit: ResolvedVnPriceUnit,
) -> f64 {
    if !a.is_finite() || !b.is_finite() || a <= 0.0 || b <= 0.0 {
        return 0.0;
    }
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    if (hi - lo) <= PRICE_EQ_EPS {
        return 0.0;
    }
    match venue {
        ExchangeVenue::Hose => match resolved_unit {
            ResolvedVnPriceUnit::ThousandVnd => {
                integrate_piecewise_ticks(lo, hi, &[(10.0, 0.01), (50.0, 0.05)], 0.10)
            }
            ResolvedVnPriceUnit::RawVnd => {
                integrate_piecewise_ticks(lo, hi, &[(10_000.0, 10.0), (50_000.0, 50.0)], 100.0)
            }
        },
        ExchangeVenue::Hnx | ExchangeVenue::Upcom => {
            let step = tick_size_at_price(venue, lo, resolved_unit);
            integrate_piecewise_ticks(lo, hi, &[], step)
        }
        ExchangeVenue::Vn30Futures => integrate_piecewise_ticks(lo, hi, &[], 0.10),
        ExchangeVenue::VnIndexBenchmark => integrate_piecewise_ticks(lo, hi, &[], 0.01),
        ExchangeVenue::UsEquities => integrate_piecewise_ticks(lo, hi, &[(1.0, 0.0001)], 0.01),
        ExchangeVenue::CryptoSpotPerp => integrate_piecewise_ticks(
            lo,
            hi,
            &[(1.0e-4, 1.0e-8), (1.0, 1.0e-6), (100.0, 0.0001)],
            0.01,
        ),
    }
}

#[inline]
fn typical_row_weight(row_price: f64, row_tick: f64, bar_typ: f64, span: f64) -> f64 {
    let sigma = (0.35 * span).max(row_tick).max(1.0e-9);
    let z = (row_price - bar_typ) / sigma;
    0.25 + 0.75 * (-0.5 * z * z).exp()
}

#[inline]
fn classify_band_state(
    price: f64,
    tick: f64,
    band: Option<&DailyPriceBand>,
    snapped_ref: Option<f64>,
) -> DomPriceBandState {
    let Some(band) = band else {
        return DomPriceBandState::Normal;
    };
    let tol = (tick * 0.49).max(PRICE_EQ_EPS);
    if price > band.ceiling_price + tol {
        DomPriceBandState::AboveCeiling
    } else if (price - band.ceiling_price).abs() <= tol {
        DomPriceBandState::Ceiling
    } else if price < band.floor_price - tol {
        DomPriceBandState::BelowFloor
    } else if (price - band.floor_price).abs() <= tol {
        DomPriceBandState::Floor
    } else if snapped_ref.is_some_and(|r| (price - r).abs() <= (tick * 0.49).max(PRICE_EQ_EPS)) {
        DomPriceBandState::Reference
    } else {
        DomPriceBandState::Normal
    }
}

#[inline]
fn order_stage_price(order: &Order) -> Option<f64> {
    match order.kind {
        OrderKind::Limit { limit_price } => limit_price.is_finite().then_some(limit_price),
        OrderKind::Stop { stop_price } => stop_price.is_finite().then_some(stop_price),
        OrderKind::Market => None,
    }
}

/// Computes a pure `f64`/`i64` WASM-safe synthetic DOM & Footprint Price Ladder strictly from
/// causal OHLCV bars (`timestamp <= causal_cutoff_ts`), pending working orders, and active position.
pub fn compute_causal_dom_ladder(
    candles: &[Candle],
    config: &DomLadderConfig,
    pending_orders: &[Order],
    open_position: Option<&Position>,
    causal_cutoff_ts: Option<i64>,
) -> Result<DomLadderSnapshot, DomLadderError> {
    if config.half_rows == 0 {
        return Err(DomLadderError::InvalidConfig("half_rows must be >= 1"));
    }
    if config.lookback_bars == 0 {
        return Err(DomLadderError::InvalidConfig("lookback_bars must be >= 1"));
    }
    let half_rows = config.half_rows.min(MAX_HALF_ROWS);
    let lookback = config.lookback_bars.min(MAX_LOOKBACK_BARS);

    // Single causal pass collecting latest causal bar, prior local-day close, and trailing lookback ring.
    let mut causal_bar_count: usize = 0;
    let mut last_causal: Option<&Candle> = None;
    let mut first_open: Option<f64> = None;
    let mut prev_day_last_close: Option<f64> = None;
    let mut cur_day: Option<i64> = None;
    let mut cur_day_last_close: Option<f64> = None;
    let mut lookback_ring: VecDeque<&Candle> = VecDeque::with_capacity(lookback);

    for c in candles {
        if !is_causal_valid_bar(c, causal_cutoff_ts) {
            continue;
        }
        causal_bar_count += 1;
        if first_open.is_none() {
            first_open = Some(c.open);
        }
        let day = local_trading_day(c.timestamp, config.venue);
        match cur_day {
            None => {
                cur_day = Some(day);
                cur_day_last_close = Some(c.close);
            }
            Some(prev) if day != prev => {
                prev_day_last_close = cur_day_last_close;
                cur_day = Some(day);
                cur_day_last_close = Some(c.close);
            }
            Some(_) => {
                cur_day_last_close = Some(c.close);
            }
        }
        if last_causal.is_none_or(|lc| c.timestamp >= lc.timestamp) {
            last_causal = Some(c);
        }
        if lookback_ring.len() == lookback {
            lookback_ring.pop_front();
        }
        lookback_ring.push_back(c);
    }

    let Some(mark_candle) = last_causal else {
        return Err(DomLadderError::EmptyCausalSeries);
    };
    let mark_price = mark_candle.close;
    let last_causal_timestamp = Some(mark_candle.timestamp);

    let anchor_price = config
        .center_price_override
        .filter(|p| p.is_finite() && *p > 0.0)
        .unwrap_or(mark_price);
    let ref_price = config
        .reference_price_override
        .filter(|p| p.is_finite() && *p > 0.0)
        .or(prev_day_last_close.or(first_open));
    let unit_anchor = ref_price.unwrap_or(mark_price);
    let resolved_unit = config.vn_unit.resolve_for_venue(config.venue, unit_anchor);
    let daily_band = ref_price
        .and_then(|r| compute_daily_price_band_with_resolved_unit(config.venue, r, resolved_unit));

    // Quantize center and mark BEFORE stepping so exchange tier boundaries (e.g. HOSE 10.00 / 50.00)
    // are landed on explicitly and never skipped.
    let center_price = snap_to_nearest_tick(anchor_price, config.venue, resolved_unit);
    let snapped_mark = snap_to_nearest_tick(mark_price, config.venue, resolved_unit);
    let snapped_ref =
        daily_band.map(|b| snap_to_nearest_tick(b.reference_price, config.venue, resolved_unit));
    let snapped_pos_entry = open_position
        .filter(|p| p.quantity > 0.0 && p.avg_entry_price.is_finite() && p.avg_entry_price > 0.0)
        .map(|p| snap_to_nearest_tick(p.avg_entry_price, config.venue, resolved_unit));

    let mut upper = Vec::with_capacity(half_rows);
    let mut cur = center_price;
    for _ in 0..half_rows {
        let next = step_one_tick_above(cur, config.venue, resolved_unit);
        if !next.is_finite() || next <= cur {
            break;
        }
        upper.push(next);
        cur = next;
    }

    let mut lower = Vec::with_capacity(half_rows);
    let mut cur_dn = center_price;
    for _ in 0..half_rows {
        let prev = step_one_tick_below(cur_dn, config.venue, resolved_unit);
        if !prev.is_finite() || prev <= 0.0 || prev >= cur_dn {
            break;
        }
        lower.push(prev);
        cur_dn = prev;
    }

    let total_rows = upper.len() + 1 + lower.len();
    let mut prices = Vec::with_capacity(total_rows);
    for &p in upper.iter().rev() {
        prices.push(p);
    }
    prices.push(center_price);
    for &p in &lower {
        prices.push(p);
    }

    let mut min_tick = f64::INFINITY;
    let mut rows: Vec<DomLadderRow> = prices
        .into_iter()
        .map(|price| {
            let tick_size = tick_size_at_price(config.venue, price, resolved_unit);
            min_tick = min_tick.min(tick_size);
            let band_state =
                classify_band_state(price, tick_size, daily_band.as_ref(), snapped_ref);
            let tol = (tick_size * 0.49).max(PRICE_EQ_EPS);
            DomLadderRow {
                price,
                tick_size,
                band_state,
                is_at_mark: (price - snapped_mark).abs() <= tol,
                is_at_position_entry: snapped_pos_entry.is_some_and(|e| (price - e).abs() <= tol),
                synth_bid_depth: 0.0,
                synth_ask_depth: 0.0,
                traded_buy_volume: 0.0,
                traded_sell_volume: 0.0,
                net_delta: 0.0,
                working_buy_qty: 0.0,
                working_sell_qty: 0.0,
                working_order_ids: Vec::new(),
            }
        })
        .collect();

    let price_decimals = decimals_for_venue_ladder_tick(config.venue, min_tick, resolved_unit);

    // Distribute causal OHLCV bar volumes across intersecting ladder rows via CLV buyer ratio
    // and typical-price Gaussian proximity weighting (conserving exact buy/sell volume across hit rows).
    let mut cumulative_buy_volume = 0.0_f64;
    let mut cumulative_sell_volume = 0.0_f64;
    for bar in &lookback_ring {
        if bar.volume <= 0.0 || !bar.volume.is_finite() {
            continue;
        }
        let span = bar.high - bar.low;
        let buy_ratio = if span > 1.0e-12 {
            ((bar.close - bar.low) / span).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let buy_vol = bar.volume * buy_ratio;
        let sell_vol = (bar.volume - buy_vol).max(0.0);
        cumulative_buy_volume += buy_vol;
        cumulative_sell_volume += sell_vol;

        let bar_typ = 0.5 * bar.close + 0.25 * (bar.high + bar.low);
        let eff_span = span.max(0.0);
        // `rows` are strictly descending (`high` to `low`).
        let mut w_sum = 0.0_f64;
        for row in &rows {
            let half = 0.5 * row.tick_size;
            if row.price + half >= bar.low && row.price - half <= bar.high {
                w_sum += typical_row_weight(row.price, row.tick_size, bar_typ, eff_span);
            }
        }
        if w_sum > 0.0 {
            for row in &mut rows {
                let half = 0.5 * row.tick_size;
                if row.price + half >= bar.low && row.price - half <= bar.high {
                    let frac =
                        typical_row_weight(row.price, row.tick_size, bar_typ, eff_span) / w_sum;
                    row.traded_buy_volume += buy_vol * frac;
                    row.traded_sell_volume += sell_vol * frac;
                }
            }
        }
    }

    let mut max_row_volume = 0.0_f64;
    let mut poc_price: Option<f64> = None;
    let mut best_poc_dist = f64::INFINITY;
    for row in &mut rows {
        row.net_delta = row.traded_buy_volume - row.traded_sell_volume;
        let row_vol = row.traded_buy_volume + row.traded_sell_volume;
        if row_vol > max_row_volume + PRICE_EQ_EPS
            || ((row_vol - max_row_volume).abs() <= PRICE_EQ_EPS
                && row_vol > 0.0
                && (row.price - snapped_mark).abs() < best_poc_dist)
        {
            max_row_volume = row_vol;
            poc_price = Some(row.price);
            best_poc_dist = (row.price - snapped_mark).abs();
        }
    }

    // Synthesize lot-snapped resting depth strictly inside today's band around snapped_mark.
    let board_lot = config.venue.board_lot();
    let lookback_mean_vol = if lookback_ring.is_empty() {
        board_lot
    } else {
        ((cumulative_buy_volume + cumulative_sell_volume) / (lookback_ring.len() as f64))
            .max(board_lot)
    };
    let base_depth_pool = (lookback_mean_vol * 0.025).clamp(board_lot, board_lot * 2_000.0);

    let mut max_row_depth = 0.0_f64;
    for row in &mut rows {
        let in_band = !matches!(
            row.band_state,
            DomPriceBandState::AboveCeiling | DomPriceBandState::BelowFloor
        );
        if !in_band || row.is_at_mark {
            continue;
        }
        let dist_ticks = exact_tick_distance(row.price, snapped_mark, config.venue, resolved_unit);
        let proximity_weight = 0.35 + 0.65 * (-dist_ticks / 6.0).exp();
        let row_vol = row.traded_buy_volume + row.traded_sell_volume;
        let vol_affinity = if max_row_volume > 0.0 {
            0.5 + 0.5 * (row_vol / max_row_volume).clamp(0.0, 1.0)
        } else {
            0.75
        };
        let raw_depth = base_depth_pool * proximity_weight * vol_affinity;
        let snapped_depth = floor_to_board_lot(config.venue, raw_depth).max(board_lot);
        if row.price > snapped_mark + (0.49 * row.tick_size) {
            row.synth_ask_depth = snapped_depth;
        } else if row.price < snapped_mark - (0.49 * row.tick_size) {
            row.synth_bid_depth = snapped_depth;
        }
        max_row_depth = max_row_depth
            .max(row.synth_bid_depth)
            .max(row.synth_ask_depth);
    }

    // Overlay Pending working orders onto the closest ladder rung within half-tick tolerance.
    for order in pending_orders {
        if order.status != OrderStatus::Pending
            || !order.quantity.is_finite()
            || order.quantity <= 0.0
        {
            continue;
        }
        let Some(order_price) = order_stage_price(order) else {
            continue;
        };
        if let Some(best_row) = rows
            .iter_mut()
            .filter(|r| (r.price - order_price).abs() <= (0.51 * r.tick_size).max(PRICE_EQ_EPS))
            .min_by(|a, b| {
                (a.price - order_price)
                    .abs()
                    .total_cmp(&(b.price - order_price).abs())
            })
        {
            match order.side {
                OrderSide::Buy => best_row.working_buy_qty += order.quantity,
                OrderSide::Sell => best_row.working_sell_qty += order.quantity,
            }
            best_row.working_order_ids.push(order.id);
        }
    }

    let total_traded = cumulative_buy_volume + cumulative_sell_volume;
    let bid_ask_imbalance_pct = if total_traded > 0.0 && total_traded.is_finite() {
        ((cumulative_buy_volume / total_traded) * 100.0).clamp(0.0, 100.0)
    } else {
        50.0
    };

    Ok(DomLadderSnapshot {
        data_fidelity: DOM_LADDER_DATA_FIDELITY,
        venue: config.venue,
        resolved_unit,
        price_decimals,
        mark_price: snapped_mark,
        center_price,
        daily_band,
        rows,
        max_row_depth,
        max_row_volume,
        cumulative_buy_volume,
        cumulative_sell_volume,
        bid_ask_imbalance_pct,
        poc_price,
        causal_bar_count,
        last_causal_timestamp,
    })
}

/// Steps `start` by `signed_ticks` exchange-aligned price ticks (`+` above, `-` below),
/// respecting dynamic venue tick-size tier boundaries (e.g. `HOSE` `10.00` / `50.00`) and returning
/// `None` if any step reaches a non-positive or non-finite price.
#[must_use]
pub fn step_ladder_price_ticks(
    start: f64,
    signed_ticks: i32,
    venue: ExchangeVenue,
    resolved_unit: ResolvedVnPriceUnit,
) -> Option<f64> {
    if !start.is_finite() || start <= 0.0 {
        return None;
    }
    let mut price = snap_to_nearest_tick(start, venue, resolved_unit);
    if !price.is_finite() || price <= 0.0 {
        return None;
    }
    let steps = signed_ticks.unsigned_abs();
    for _ in 0..steps {
        let next = if signed_ticks >= 0 {
            step_one_tick_above(price, venue, resolved_unit)
        } else {
            step_one_tick_below(price, venue, resolved_unit)
        };
        if !next.is_finite() || next <= 0.0 {
            return None;
        }
        if (signed_ticks >= 0 && next <= price) || (signed_ticks < 0 && next >= price) {
            return None;
        }
        price = next;
    }
    Some(price)
}

fn step_n_ticks_above(
    start: f64,
    ticks: u32,
    venue: ExchangeVenue,
    resolved_unit: ResolvedVnPriceUnit,
) -> f64 {
    let signed = i32::try_from(ticks).unwrap_or(i32::MAX);
    step_ladder_price_ticks(start, signed, venue, resolved_unit).unwrap_or(f64::NAN)
}

fn step_n_ticks_below(
    start: f64,
    ticks: u32,
    venue: ExchangeVenue,
    resolved_unit: ResolvedVnPriceUnit,
) -> f64 {
    let mag = i32::try_from(ticks).unwrap_or(i32::MAX);
    step_ladder_price_ticks(start, -mag, venue, resolved_unit).unwrap_or(f64::NAN)
}

/// Validates exchange microstructure rules (tick snapping, statutory daily price band on entry,
/// board-lot rounding, venue short-selling prohibition) and converts a DOM Bid/Ask cell click into
/// a tick-aligned `Limit` or `Stop` [`OrderRequest`].
///
/// NOTE: Optional `stop_loss_ticks` / `take_profit_ticks` are intentionally NOT clamped to
/// `daily_band` because a multi-session bracket can sit outside today's statutory ceiling/floor
/// (e.g. a `Buy Stop` at `CEIL` keeps `TP` strictly above `CEIL`).
pub fn stage_ladder_order(input: &LadderStageInput) -> Result<OrderRequest, DomLadderError> {
    if !input.clicked_price.is_finite() || input.clicked_price <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("clicked_price"));
    }
    if !input.mark_price.is_finite() || input.mark_price <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("mark_price"));
    }
    if !input.quantity.is_finite() || input.quantity <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("quantity"));
    }

    let entry = snap_to_nearest_tick(input.clicked_price, input.venue, input.resolved_unit);
    if !entry.is_finite() || entry <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("snapped_entry"));
    }

    let board_lot = input.venue.board_lot();
    let qty = floor_to_board_lot(input.venue, input.quantity);
    if !qty.is_finite() || qty < board_lot - PRICE_EQ_EPS || qty <= 0.0 {
        return Err(DomLadderError::InvalidQuantity {
            requested: input.quantity,
            board_lot,
        });
    }

    if let Some(band) = input.daily_band {
        let band_tol = (band.tick_at_ref * 0.05).max(PRICE_EQ_EPS);
        if entry < band.floor_price - band_tol || entry > band.ceiling_price + band_tol {
            return Err(DomLadderError::OutsideDailyPriceBand {
                price: entry,
                floor: band.floor_price,
                ceiling: band.ceiling_price,
                decimals: decimals_for_venue_ladder_tick(
                    input.venue,
                    band.tick_at_ref,
                    input.resolved_unit,
                ),
            });
        }
    }

    if input.side == OrderSide::Sell {
        let closable_long = input
            .open_position
            .as_ref()
            .filter(|p| p.side == PositionSide::Long && p.quantity.is_finite() && p.quantity > 0.0)
            .map_or(0.0, |p| p.quantity);
        if qty > closable_long + PRICE_EQ_EPS
            && (!input.venue.allows_short() || !input.allow_account_short)
        {
            return Err(DomLadderError::ShortNotAllowedByVenue {
                venue: input.venue,
                requested_sell: qty,
                closable_long,
            });
        }
    }

    let tick_at_entry = tick_size_at_price(input.venue, entry, input.resolved_unit);
    let tick_tol = 0.25 * tick_at_entry;
    let kind = match input.side {
        OrderSide::Buy => {
            if entry <= input.mark_price + tick_tol {
                OrderKind::Limit { limit_price: entry }
            } else {
                OrderKind::Stop { stop_price: entry }
            }
        }
        OrderSide::Sell => {
            if entry >= input.mark_price - tick_tol {
                OrderKind::Limit { limit_price: entry }
            } else {
                OrderKind::Stop { stop_price: entry }
            }
        }
    };

    let stop_loss = match input.stop_loss_ticks.filter(|&n| n > 0) {
        None => None,
        Some(n) => {
            let sl = match input.side {
                OrderSide::Buy => step_n_ticks_below(entry, n, input.venue, input.resolved_unit),
                OrderSide::Sell => step_n_ticks_above(entry, n, input.venue, input.resolved_unit),
            };
            if !sl.is_finite() || sl <= 0.0 {
                return Err(DomLadderError::InvalidBracketPrice(
                    "stop-loss ticks stepped to non-positive price",
                ));
            }
            Some(sl)
        }
    };

    let take_profit = match input.take_profit_ticks.filter(|&n| n > 0) {
        None => None,
        Some(n) => {
            let tp = match input.side {
                OrderSide::Buy => step_n_ticks_above(entry, n, input.venue, input.resolved_unit),
                OrderSide::Sell => step_n_ticks_below(entry, n, input.venue, input.resolved_unit),
            };
            if !tp.is_finite() || tp <= 0.0 {
                return Err(DomLadderError::InvalidBracketPrice(
                    "take-profit ticks stepped to non-positive price",
                ));
            }
            Some(tp)
        }
    };

    Ok(OrderRequest {
        side: input.side,
        kind,
        quantity: qty,
        stop_loss,
        take_profit,
    })
}

/// Computes slippage-safe directional SL/TP bracket levels around a `Market` order's expected fill
/// (`apply_slippage(mark_price, side, slippage_bps)`), preventing false `InvalidTakeProfit` or
/// `InvalidStopLoss` rejections when `TP`/`SL` ticks sit inside the market slippage buffer.
pub fn compute_quick_market_brackets(
    mark_price: f64,
    side: OrderSide,
    slippage_bps: f64,
    stop_loss_ticks: u32,
    take_profit_ticks: u32,
    venue: ExchangeVenue,
    resolved_unit: ResolvedVnPriceUnit,
) -> Result<(Option<f64>, Option<f64>), DomLadderError> {
    if !mark_price.is_finite() || mark_price <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("mark_price"));
    }
    let eff_slip = if slippage_bps.is_finite() {
        slippage_bps.max(0.0)
    } else {
        0.0
    };
    let expected_fill = apply_slippage(mark_price, side, eff_slip);
    if !expected_fill.is_finite() || expected_fill <= 0.0 {
        return Err(DomLadderError::InvalidBracketPrice(
            "expected market fill after slippage is non-positive",
        ));
    }

    let step_from_expected_fill =
        |ticks: u32, step_up: bool| -> Result<Option<f64>, DomLadderError> {
            if ticks == 0 {
                return Ok(None);
            }
            let base = if step_up {
                round_up_to_tick(expected_fill, venue, resolved_unit)
            } else {
                round_down_to_tick(expected_fill, venue, resolved_unit)
            };
            if !base.is_finite() || base <= 0.0 {
                return Err(DomLadderError::InvalidBracketPrice(
                    "bracket anchor rounded non-positive around expected market fill",
                ));
            }
            let px = if step_up {
                step_n_ticks_above(base, ticks, venue, resolved_unit)
            } else {
                step_n_ticks_below(base, ticks, venue, resolved_unit)
            };
            let valid = px.is_finite()
                && px > 0.0
                && if step_up {
                    px > expected_fill
                } else {
                    px < expected_fill
                };
            if !valid {
                return Err(DomLadderError::InvalidBracketPrice(
                    "bracket price stepped outside valid range around expected market fill",
                ));
            }
            Ok(Some(px))
        };

    let (sl_up, tp_up) = match side {
        OrderSide::Buy => (false, true),
        OrderSide::Sell => (true, false),
    };
    let stop_loss = step_from_expected_fill(stop_loss_ticks, sl_up)?;
    let take_profit = step_from_expected_fill(take_profit_ticks, tp_up)?;

    if validate_brackets_for_side(
        side.to_position_side(),
        expected_fill,
        stop_loss,
        take_profit,
    )
    .is_err()
    {
        return Err(DomLadderError::InvalidBracketPrice(
            "computed quick brackets failed directional validation against expected fill",
        ));
    }
    Ok((stop_loss, take_profit))
}

/// Computes a friction-aware, board-lot-rounded DOM order quantity using `compute_trade_plan`
/// given `equity`, `risk_pct`, `entry_price`, and `stop_ticks` (`>= 1`).
#[allow(clippy::too_many_arguments)]
pub fn size_ladder_qty_from_risk_budget(
    equity: f64,
    risk_pct: f64,
    entry_price: f64,
    stop_ticks: u32,
    side: OrderSide,
    venue: ExchangeVenue,
    vn_unit: VnPriceUnit,
    reference_price: Option<f64>,
) -> Result<f64, DomLadderError> {
    if !equity.is_finite() || equity <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("equity"));
    }
    if !risk_pct.is_finite() || risk_pct <= 0.0 || risk_pct > 100.0 {
        return Err(DomLadderError::NonFiniteInput("risk_pct"));
    }
    if !entry_price.is_finite() || entry_price <= 0.0 {
        return Err(DomLadderError::NonFiniteInput("entry_price"));
    }
    if stop_ticks == 0 {
        return Err(DomLadderError::InvalidBracketPrice(
            "stop_ticks must be >= 1 for risk sizing",
        ));
    }

    let unit_anchor = reference_price
        .filter(|p| p.is_finite() && *p > 0.0)
        .unwrap_or(entry_price);
    let resolved_unit = vn_unit.resolve_for_venue(venue, unit_anchor);
    let explicit_vn_unit = if venue.is_vn_cash_equity() {
        match resolved_unit {
            ResolvedVnPriceUnit::ThousandVnd => VnPriceUnit::ThousandVnd,
            ResolvedVnPriceUnit::RawVnd => VnPriceUnit::RawVnd,
        }
    } else {
        vn_unit
    };
    let snapped_entry = snap_to_nearest_tick(entry_price, venue, resolved_unit);
    let (sl, tp) = match side {
        OrderSide::Buy => (
            step_n_ticks_below(snapped_entry, stop_ticks, venue, resolved_unit),
            step_n_ticks_above(
                snapped_entry,
                stop_ticks.saturating_mul(2),
                venue,
                resolved_unit,
            ),
        ),
        OrderSide::Sell => (
            step_n_ticks_above(snapped_entry, stop_ticks, venue, resolved_unit),
            step_n_ticks_below(
                snapped_entry,
                stop_ticks.saturating_mul(2),
                venue,
                resolved_unit,
            ),
        ),
    };
    if !sl.is_finite() || !tp.is_finite() || sl <= 0.0 || tp <= 0.0 {
        return Err(DomLadderError::InvalidBracketPrice(
            "risk stop/target stepped to invalid price",
        ));
    }

    let plan = compute_trade_plan(&TradePlanInput {
        venue,
        price_unit: explicit_vn_unit,
        side: side.to_position_side(),
        account_equity: equity,
        available_cash: equity,
        leverage: if venue == ExchangeVenue::Vn30Futures {
            100.0 / venue.default_initial_margin_pct()
        } else {
            1.0
        },
        contract_multiplier: venue.default_contract_multiplier(explicit_vn_unit, equity),
        initial_margin_pct: venue.default_initial_margin_pct(),
        max_position_pct: 100.0,
        risk_pct,
        entry_price: snapped_entry,
        stop_loss: sl,
        target_price: tp,
        reference_price: reference_price.filter(|p| p.is_finite() && *p > 0.0),
        commission_bps: venue.default_commission_bps(),
        tax_sell_bps: venue.default_tax_sell_bps(),
        slippage_bps: 2.0,
        snap_to_ticks: true,
        entry_timestamp: None,
    })
    .map_err(|e| DomLadderError::InvalidBracketPrice(e.message()))?;

    Ok(plan.planned_quantity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::compute_daily_price_band;

    fn mk_bar(ts: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: ts,
            date_label: format!("t{ts}"),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    #[test]
    fn r14_hose_tier_crossing_step_precision_and_no_skipped_ticks() {
        // Test 50.00 tier transition (`49.95` vs `50.00` vs `50.10`) and 10.00 tier transition (`9.99` vs `10.00` vs `10.05`)
        let t0 = 1_750_000_000_i64;
        let candles = vec![
            mk_bar(t0, 49.60, 50.30, 49.50, 49.90, 200_000.0),
            mk_bar(t0 + 86_400, 49.90, 50.40, 49.85, 50.00, 350_000.0),
        ];
        let cfg = DomLadderConfig {
            venue: ExchangeVenue::Hose,
            vn_unit: VnPriceUnit::ThousandVnd,
            half_rows: 3,
            lookback_bars: 10,
            center_price_override: Some(49.97), // Unaligned -> must snap to 49.95 or 50.00 BEFORE stepping!
            reference_price_override: Some(49.90),
        };
        let snap = compute_causal_dom_ladder(&candles, &cfg, &[], None, None).unwrap();
        assert_eq!(snap.data_fidelity, DOM_LADDER_DATA_FIDELITY);
        assert_eq!(snap.rows.len(), 7);
        let prices: Vec<f64> = snap.rows.iter().map(|r| r.price).collect();
        // Snap(49.97) = 49.95 -> 3 up (`50.00`, `50.10`, `50.20`), center `49.95`, 3 down (`49.90`, `49.85`, `49.80`)
        assert_eq!(
            prices,
            vec![50.20, 50.10, 50.00, 49.95, 49.90, 49.85, 49.80]
        );

        // Cross 10.00 boundary on HOSE ThousandVnd
        let low_candles = vec![
            mk_bar(t0, 9.95, 10.15, 9.90, 10.00, 150_000.0),
            mk_bar(t0 + 86_400, 10.00, 10.10, 9.96, 10.00, 180_000.0),
        ];
        let low_cfg = DomLadderConfig {
            venue: ExchangeVenue::Hose,
            vn_unit: VnPriceUnit::ThousandVnd,
            half_rows: 2,
            lookback_bars: 10,
            center_price_override: Some(10.00),
            reference_price_override: Some(10.00),
        };
        let low_snap = compute_causal_dom_ladder(&low_candles, &low_cfg, &[], None, None).unwrap();
        let low_prices: Vec<f64> = low_snap.rows.iter().map(|r| r.price).collect();
        assert_eq!(low_prices, vec![10.10, 10.05, 10.00, 9.99, 9.98]);
    }

    #[test]
    fn r14_stage_order_respects_daily_band_short_guard_and_unclamped_brackets() {
        let band = compute_daily_price_band(ExchangeVenue::Hose, 50.00, VnPriceUnit::ThousandVnd)
            .expect("HOSE daily band");
        assert!((band.ceiling_price - 53.50).abs() < 1e-6);
        assert!((band.floor_price - 46.50).abs() < 1e-6);

        // 1. Entry above ceiling must reject
        let err_above = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Buy,
            clicked_price: 53.60,
            mark_price: 50.00,
            quantity: 100.0,
            venue: ExchangeVenue::Hose,
            resolved_unit: ResolvedVnPriceUnit::ThousandVnd,
            daily_band: Some(band),
            stop_loss_ticks: Some(2),
            take_profit_ticks: Some(4),
            open_position: None,
            allow_account_short: true,
        })
        .unwrap_err();
        assert!(matches!(
            err_above,
            DomLadderError::OutsideDailyPriceBand { .. }
        ));

        // 2. Buy AT ceiling (`53.50`) with `+3` tick TP must keep `TP = 53.80 > 53.50` (NOT clamped to ceiling!)
        let req_at_ceil = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Buy,
            clicked_price: 53.50,
            mark_price: 50.00,
            quantity: 200.0,
            venue: ExchangeVenue::Hose,
            resolved_unit: ResolvedVnPriceUnit::ThousandVnd,
            daily_band: Some(band),
            stop_loss_ticks: Some(2),
            take_profit_ticks: Some(3),
            open_position: None,
            allow_account_short: true,
        })
        .unwrap();
        assert_eq!(req_at_ceil.kind, OrderKind::Stop { stop_price: 53.50 });
        assert_eq!(req_at_ceil.stop_loss, Some(53.30));
        assert_eq!(req_at_ceil.take_profit, Some(53.80));

        // 3. HOSE blocks naked short AND excess sell beyond closable long, even when `allow_account_short = true`
        let open_long = Position {
            side: PositionSide::Long,
            quantity: 200.0,
            avg_entry_price: 49.50,
            opened_timestamp: 1_750_000_000,
            opened_base_index: 0,
            stop_loss: None,
            take_profit: None,
            initial_stop_loss: None,
            initial_risk_amount: None,
            total_entry_qty: 200.0,
            total_entry_notional: 9_900.0,
            total_entry_fees: 0.0,
            open_entry_fees: 0.0,
            closed_qty_acc: 0.0,
            exit_notional_acc: 0.0,
            realized_gross_pnl_acc: 0.0,
            realized_exit_fees_acc: 0.0,
        };
        let err_excess = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Sell,
            clicked_price: 51.00,
            mark_price: 50.00,
            quantity: 300.0,
            venue: ExchangeVenue::Hose,
            resolved_unit: ResolvedVnPriceUnit::ThousandVnd,
            daily_band: Some(band),
            stop_loss_ticks: None,
            take_profit_ticks: None,
            open_position: Some(open_long.clone()),
            allow_account_short: true,
        })
        .unwrap_err();
        assert!(matches!(
            err_excess,
            DomLadderError::ShortNotAllowedByVenue {
                venue: ExchangeVenue::Hose,
                requested_sell: 300.0,
                closable_long: 200.0
            }
        ));

        // Closing existing long `<= 200.0` succeeds cleanly as Sell Limit (`>= mark`) or Sell Stop (`< mark`)
        let ok_close = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Sell,
            clicked_price: 51.00,
            mark_price: 50.00,
            quantity: 200.0,
            venue: ExchangeVenue::Hose,
            resolved_unit: ResolvedVnPriceUnit::ThousandVnd,
            daily_band: Some(band),
            stop_loss_ticks: None,
            take_profit_ticks: None,
            open_position: Some(open_long),
            allow_account_short: true,
        })
        .unwrap();
        assert_eq!(ok_close.kind, OrderKind::Limit { limit_price: 51.00 });

        // 4. VN30Futures allows short when `allow_account_short = true`
        let fut_short = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Sell,
            clicked_price: 1_318.0,
            mark_price: 1_320.0,
            quantity: 2.0,
            venue: ExchangeVenue::Vn30Futures,
            resolved_unit: ResolvedVnPriceUnit::ThousandVnd,
            daily_band: None,
            stop_loss_ticks: Some(5),
            take_profit_ticks: Some(10),
            open_position: None,
            allow_account_short: true,
        })
        .unwrap();
        assert_eq!(
            fut_short.kind,
            OrderKind::Stop {
                stop_price: 1_318.0
            }
        );
        assert_eq!(fut_short.stop_loss, Some(1_318.5));
        assert_eq!(fut_short.take_profit, Some(1_317.0));
    }

    #[test]
    fn r14_causal_cutoff_invariance_and_working_orders_mapping() {
        // ICT UTC+7: 2026-06-15 14:30 ICT (day 1 close = 60.0), 2026-06-16 09:30 ICT (day 2), future bar on 2026-06-17
        let day1_close_ts = 1_781_508_600_i64;
        let day2_open_ts = day1_close_ts + 19 * 3_600;
        let future_ts = day2_open_ts + 86_400;

        let prefix = vec![
            mk_bar(day1_close_ts - 3_600, 59.5, 60.2, 59.4, 59.8, 100_000.0),
            mk_bar(day1_close_ts, 59.8, 60.3, 59.7, 60.0, 120_000.0),
            mk_bar(day2_open_ts, 60.1, 60.6, 59.9, 60.4, 250_000.0),
        ];
        let mut with_future = prefix.clone();
        with_future.push(mk_bar(future_ts, 61.0, 64.0, 60.5, 63.8, 999_999.0));

        let pending = vec![
            Order {
                id: 7,
                side: OrderSide::Buy,
                kind: OrderKind::Limit { limit_price: 60.1 },
                quantity: 200.0,
                stop_loss: None,
                take_profit: None,
                submitted_base_index: 2,
                submitted_timestamp: day2_open_ts,
                status: OrderStatus::Pending,
                filled_price: None,
                filled_timestamp: None,
            },
            Order {
                id: 8,
                side: OrderSide::Sell,
                kind: OrderKind::Limit { limit_price: 60.8 },
                quantity: 100.0,
                stop_loss: None,
                take_profit: None,
                submitted_base_index: 2,
                submitted_timestamp: day2_open_ts,
                status: OrderStatus::Pending,
                filled_price: None,
                filled_timestamp: None,
            },
        ];

        let cfg = DomLadderConfig {
            venue: ExchangeVenue::Hose,
            vn_unit: VnPriceUnit::ThousandVnd,
            half_rows: 6,
            lookback_bars: 50,
            center_price_override: None,
            reference_price_override: None,
        };
        let snap_prefix =
            compute_causal_dom_ladder(&prefix, &cfg, &pending, None, Some(day2_open_ts)).unwrap();
        let snap_future =
            compute_causal_dom_ladder(&with_future, &cfg, &pending, None, Some(day2_open_ts))
                .unwrap();
        assert_eq!(
            snap_prefix, snap_future,
            "DEF-001 replay cutoff must be bit-invariant"
        );
        assert_eq!(
            snap_prefix.daily_band.map(|b| b.reference_price),
            Some(60.0),
            "must resolve prior local trading day close as reference price"
        );
        let row_601 = snap_prefix
            .rows
            .iter()
            .find(|r| (r.price - 60.1).abs() < 1e-6)
            .unwrap();
        assert_eq!(row_601.working_buy_qty, 200.0);
        assert_eq!(row_601.working_order_ids, vec![7]);
        let row_608 = snap_prefix
            .rows
            .iter()
            .find(|r| (r.price - 60.8).abs() < 1e-6)
            .unwrap();
        assert_eq!(row_608.working_sell_qty, 100.0);
        assert_eq!(row_608.working_order_ids, vec![8]);
    }

    #[test]
    fn critic_r14_regression_vn30f_unit_label_decimals_and_ref_tick_snap() {
        let resolved = VnPriceUnit::Auto.resolve_for_venue(ExchangeVenue::Vn30Futures, 1305.4);
        assert_eq!(resolved, ResolvedVnPriceUnit::ThousandVnd);
        assert_eq!(ExchangeVenue::Vn30Futures.price_unit_label(resolved), "pts");
        assert_eq!(
            ExchangeVenue::Hose.price_unit_label(ResolvedVnPriceUnit::ThousandVnd),
            "kVND"
        );
        assert_eq!(
            ExchangeVenue::Hose.price_unit_label(ResolvedVnPriceUnit::RawVnd),
            "VND"
        );

        let t0 = 1_750_000_000_i64;
        let candles = vec![
            mk_bar(t0, 1300.0, 1308.0, 1298.0, 1305.4, 15_000.0),
            mk_bar(t0 + 86_400, 1305.5, 1310.0, 1302.0, 1306.8, 18_000.0),
        ];
        let cfg = DomLadderConfig {
            venue: ExchangeVenue::Vn30Futures,
            vn_unit: VnPriceUnit::Auto,
            half_rows: 5,
            lookback_bars: 10,
            ..DomLadderConfig::default()
        };
        let snap = compute_causal_dom_ladder(&candles, &cfg, &[], None, None).unwrap();
        assert_eq!(snap.resolved_unit, ResolvedVnPriceUnit::ThousandVnd);
        assert_eq!(snap.price_decimals, 1);

        let err = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Buy,
            clicked_price: 2000.0,
            mark_price: snap.mark_price,
            quantity: 1.0,
            venue: ExchangeVenue::Vn30Futures,
            resolved_unit: snap.resolved_unit,
            daily_band: snap.daily_band,
            stop_loss_ticks: None,
            take_profit_ticks: None,
            open_position: None,
            allow_account_short: true,
        })
        .unwrap_err();
        match err {
            DomLadderError::OutsideDailyPriceBand {
                decimals,
                floor,
                ceiling,
                ..
            } => {
                assert_eq!(decimals, 1);
                // `round_up_to_tick(1305.4 * 0.93 = 1214.022, 0.1) = 1214.1`
                // `round_down_to_tick(1305.4 * 1.07 = 1396.778, 0.1) = 1396.7`
                assert!((floor - 1214.1).abs() < 1e-6);
                assert!((ceiling - 1396.7).abs() < 1e-6);
            }
            other => panic!("expected OutsideDailyPriceBand, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(
            msg.contains("1214.1") && msg.contains("1396.7"),
            "msg={msg}"
        );
    }

    #[test]
    fn critic_r14_regression_split_1000_threshold_unit_consistency() {
        let t0 = 1_750_000_000_i64;
        // Prior daily close sits below 1000 (`998.0`) while current mark sits above (`1005.0`).
        // Single anchor resolution from `ref_price` (`998.0`) must keep ThousandVnd consistent
        // across daily band, tick snapping, and order validation.
        let candles = vec![
            mk_bar(t0, 995.0, 1000.0, 992.0, 998.0, 200_000.0),
            mk_bar(t0 + 86_400, 998.0, 1010.0, 997.0, 1005.0, 250_000.0),
        ];
        let cfg = DomLadderConfig {
            venue: ExchangeVenue::Hose,
            vn_unit: VnPriceUnit::Auto,
            half_rows: 5,
            lookback_bars: 10,
            ..DomLadderConfig::default()
        };
        let snap = compute_causal_dom_ladder(&candles, &cfg, &[], None, None).unwrap();
        assert_eq!(snap.resolved_unit, ResolvedVnPriceUnit::ThousandVnd);
        let band = snap.daily_band.expect("HOSE daily band");
        assert!(
            band.floor_price < 998.0 && band.ceiling_price > 1005.0,
            "band must stay in ThousandVnd: {band:?}"
        );

        let stepped =
            step_ladder_price_ticks(snap.mark_price, -2, snap.venue, snap.resolved_unit).unwrap();
        let req = stage_ladder_order(&LadderStageInput {
            side: OrderSide::Buy,
            clicked_price: stepped,
            mark_price: snap.mark_price,
            quantity: 100.0,
            venue: snap.venue,
            resolved_unit: snap.resolved_unit,
            daily_band: snap.daily_band,
            stop_loss_ticks: Some(2),
            take_profit_ticks: Some(3),
            open_position: None,
            allow_account_short: false,
        })
        .expect("consistent resolved unit must validate order cleanly");
        assert!(matches!(req.kind, OrderKind::Limit { .. }));
    }

    #[test]
    fn critic_r14_regression_dom_scrolled_off_mark_weight_decay() {
        let t0 = 1_750_000_000_i64;
        let candles = vec![
            mk_bar(t0, 49.00, 50.00, 48.80, 49.50, 300_000.0),
            mk_bar(t0 + 86_400, 49.50, 50.20, 49.40, 50.00, 400_000.0),
        ];
        let at_mark = compute_causal_dom_ladder(
            &candles,
            &DomLadderConfig {
                venue: ExchangeVenue::Hose,
                vn_unit: VnPriceUnit::ThousandVnd,
                half_rows: 5,
                lookback_bars: 10,
                center_price_override: None,
                reference_price_override: Some(49.50),
            },
            &[],
            None,
            None,
        )
        .unwrap();
        let scrolled_center = step_ladder_price_ticks(
            50.00,
            25,
            ExchangeVenue::Hose,
            ResolvedVnPriceUnit::ThousandVnd,
        )
        .unwrap();
        let off_mark = compute_causal_dom_ladder(
            &candles,
            &DomLadderConfig {
                venue: ExchangeVenue::Hose,
                vn_unit: VnPriceUnit::ThousandVnd,
                half_rows: 5,
                lookback_bars: 10,
                center_price_override: Some(scrolled_center),
                reference_price_override: Some(49.50),
            },
            &[],
            None,
            None,
        )
        .unwrap();

        let near_ask = at_mark
            .rows
            .iter()
            .filter(|r| r.synth_ask_depth > 0.0)
            .map(|r| r.synth_ask_depth)
            .fold(0.0_f64, f64::max);
        let far_ask = off_mark
            .rows
            .iter()
            .filter(|r| r.synth_ask_depth > 0.0)
            .map(|r| r.synth_ask_depth)
            .fold(0.0_f64, f64::max);
        assert!(
            far_ask <= near_ask * 0.50 && far_ask < near_ask,
            "scrolled +25 ticks off mark must decay resting depth strictly (near={near_ask}, far={far_ask})"
        );
        let dist_25 = exact_tick_distance(
            scrolled_center,
            50.00,
            ExchangeVenue::Hose,
            ResolvedVnPriceUnit::ThousandVnd,
        );
        assert!((dist_25 - 25.0).abs() < 1e-6);
    }

    #[test]
    fn critic_r14_regression_quick_market_order_tight_tp_with_slippage() {
        use crate::trading::{AccountConfig, TradingSession};

        let t0 = 1_750_000_000_i64;
        // High slippage (15 bps on 1305.0 = ~1.96 pts) with tight 1-tick TP/SL (`0.1` pt)
        let candles = vec![
            mk_bar(t0, 1300.0, 1306.0, 1298.0, 1304.0, 10_000.0),
            mk_bar(t0 + 60, 1304.0, 1307.0, 1303.0, 1305.0, 12_000.0),
        ];
        for (venue, unit, mark) in [
            (
                ExchangeVenue::Vn30Futures,
                ResolvedVnPriceUnit::ThousandVnd,
                1305.0,
            ),
            (ExchangeVenue::Hose, ResolvedVnPriceUnit::ThousandVnd, 85.0),
        ] {
            for side in [OrderSide::Buy, OrderSide::Sell] {
                let mut session = TradingSession::new(AccountConfig {
                    initial_cash: 1_000_000.0,
                    slippage_bps: 15.0,
                    allow_short: true,
                    ..AccountConfig::default()
                });
                let (sl, tp) = compute_quick_market_brackets(mark, side, 15.0, 2, 1, venue, unit)
                    .expect("tight 1-tick TP brackets with slippage");
                let bar = [mk_bar(t0, mark, mark + 1.0, mark - 1.0, mark, 10_000.0)];
                let qty = venue.board_lot();
                let res = session.submit_order(
                    OrderRequest {
                        side,
                        kind: OrderKind::Market,
                        quantity: qty,
                        stop_loss: sl,
                        take_profit: tp,
                    },
                    &bar,
                    0,
                );
                assert!(
                    res.is_ok(),
                    "{venue:?} {side:?} quick market failed: {res:?}"
                );
            }
        }

        // Reject invalid zero config values explicitly
        let err_rows = compute_causal_dom_ladder(
            &candles,
            &DomLadderConfig {
                half_rows: 0,
                ..DomLadderConfig::default()
            },
            &[],
            None,
            None,
        )
        .unwrap_err();
        assert!(matches!(err_rows, DomLadderError::InvalidConfig(_)));
        let err_lb = compute_causal_dom_ladder(
            &candles,
            &DomLadderConfig {
                lookback_bars: 0,
                ..DomLadderConfig::default()
            },
            &[],
            None,
            None,
        )
        .unwrap_err();
        assert!(matches!(err_lb, DomLadderError::InvalidConfig(_)));
    }

    #[test]
    fn critic_r14_regression_empty_single_nan_inf_zero_vol_doji_dom_ladder() {
        let cfg = DomLadderConfig::default();
        assert_eq!(
            compute_causal_dom_ladder(&[], &cfg, &[], None, None).unwrap_err(),
            DomLadderError::EmptyCausalSeries
        );

        let t0 = 1_750_000_000_i64;
        let single_zero_doji = vec![mk_bar(t0, 50.0, 50.0, 50.0, 50.0, 0.0)];
        let snap = compute_causal_dom_ladder(&single_zero_doji, &cfg, &[], None, None).unwrap();
        assert_eq!(snap.causal_bar_count, 1);
        assert!(snap.rows.iter().all(|r| r.synth_bid_depth.is_finite()
            && r.synth_ask_depth.is_finite()
            && r.traded_buy_volume == 0.0
            && r.traded_sell_volume == 0.0));
        assert!((snap.bid_ask_imbalance_pct - 50.0).abs() < 1e-9);

        let non_finite_only = vec![
            mk_bar(t0, f64::NAN, 51.0, 49.0, 50.0, 100.0),
            mk_bar(t0 + 60, 50.0, f64::INFINITY, 49.0, 50.0, 100.0),
        ];
        assert_eq!(
            compute_causal_dom_ladder(&non_finite_only, &cfg, &[], None, None).unwrap_err(),
            DomLadderError::EmptyCausalSeries
        );

        let mixed = vec![
            mk_bar(t0, 50.0, 50.5, 49.5, 50.2, 10_000.0),
            mk_bar(t0 + 60, 50.2, f64::NAN, 49.8, 50.1, 10_000.0),
        ];
        let mixed_snap = compute_causal_dom_ladder(&mixed, &cfg, &[], None, None).unwrap();
        assert_eq!(mixed_snap.causal_bar_count, 1);
        assert!((mixed_snap.mark_price - 50.2).abs() < 1e-9);

        assert!(matches!(
            stage_ladder_order(&LadderStageInput {
                side: OrderSide::Buy,
                clicked_price: f64::NAN,
                mark_price: 50.0,
                quantity: 100.0,
                venue: ExchangeVenue::Hose,
                resolved_unit: ResolvedVnPriceUnit::ThousandVnd,
                daily_band: None,
                stop_loss_ticks: None,
                take_profit_ticks: None,
                open_position: None,
                allow_account_short: false,
            }),
            Err(DomLadderError::NonFiniteInput("clicked_price"))
        ));
    }

    #[test]
    fn critic_r14_regression_dom_causality_strict_cutoff_and_prior_close() {
        let t0 = 1_750_000_000_i64;
        let base = vec![
            mk_bar(t0, 80.0, 82.0, 79.5, 81.0, 120_000.0),
            mk_bar(t0 + 86_400, 81.0, 83.0, 80.5, 82.5, 150_000.0),
            mk_bar(t0 + 2 * 86_400, 82.5, 84.0, 82.0, 83.2, 180_000.0),
        ];
        let cutoff = base[2].timestamp;
        let mut with_future = base.clone();
        with_future.push(mk_bar(
            t0 + 3 * 86_400,
            150.0,
            200.0,
            140.0,
            195.0,
            99_000_000.0,
        ));
        with_future.push(mk_bar(
            t0 + 4 * 86_400,
            20.0,
            25.0,
            10.0,
            12.0,
            88_000_000.0,
        ));

        let cfg = DomLadderConfig::default();
        let snap_base = compute_causal_dom_ladder(&base, &cfg, &[], None, Some(cutoff)).unwrap();
        let snap_future =
            compute_causal_dom_ladder(&with_future, &cfg, &[], None, Some(cutoff)).unwrap();
        assert_eq!(snap_base, snap_future);
        assert_eq!(
            snap_future.daily_band.unwrap().reference_price,
            base[1].close,
            "daily reference price must use prior causal session close"
        );
    }
}
