use serde::{Deserialize, Serialize};

use crate::trading::{MAX_ORDER_PRICE, MAX_ORDER_QUANTITY, PositionSide};

use super::market_rules::{
    DailyPriceBand, ExchangeVenue, VnPriceUnit, compute_daily_price_band,
    compute_settlement_timestamp_for_venue, floor_to_board_lot, round_down_to_tick,
    round_up_to_tick, snap_to_nearest_tick, step_one_tick_above, step_one_tick_below,
};

const fn default_contract_multiplier() -> f64 {
    1.0
}

const fn default_initial_margin_pct() -> f64 {
    100.0
}

const fn default_max_position_pct() -> f64 {
    100.0
}

/// Actionable risk-management and exchange-compliance advisories emitted by [`compute_trade_plan`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PlanWarning {
    ShortDisallowedOnVenue,
    EntryAboveDailyCeiling { ceiling: f64 },
    EntryBelowDailyFloor { floor: f64 },
    StopBelowDailyFloorGapRisk { floor: f64 },
    StopAboveDailyCeilingGapRisk { ceiling: f64 },
    TargetAboveDailyCeilingMultiSession { ceiling: f64 },
    TargetBelowDailyFloorMultiSession { floor: f64 },
    CappedByAvailableCash,
    CappedByMaxPositionPct { max_position_pct: f64 },
    InsufficientRiskForBoardLot { min_lot: f64, single_lot_risk: f64 },
    NegativeNetRewardAfterFriction,
}

impl PlanWarning {
    #[must_use]
    pub fn description(self) -> String {
        match self {
            Self::ShortDisallowedOnVenue => {
                "Short selling is disabled on VN cash equity venues (HOSE/HNX/UPCOM).".to_owned()
            }
            Self::EntryAboveDailyCeiling { ceiling } => {
                format!(
                    "Entry exceeds today's statutory ceiling ({ceiling:.2}); order would reject."
                )
            }
            Self::EntryBelowDailyFloor { floor } => {
                format!("Entry is below today's statutory floor ({floor:.2}); order would reject.")
            }
            Self::StopBelowDailyFloorGapRisk { floor } => {
                format!(
                    "Stop lies below today's floor ({floor:.2}): locked-floor liquidity / T+2 overnight gap risk."
                )
            }
            Self::StopAboveDailyCeilingGapRisk { ceiling } => {
                format!(
                    "Short stop lies above today's ceiling ({ceiling:.2}): locked-ceiling squeeze risk."
                )
            }
            Self::TargetAboveDailyCeilingMultiSession { ceiling } => {
                format!(
                    "Target exceeds today's ceiling ({ceiling:.2}): requires multi-session hold (T+2)."
                )
            }
            Self::TargetBelowDailyFloorMultiSession { floor } => {
                format!(
                    "Short target lies below today's floor ({floor:.2}): requires multi-session hold."
                )
            }
            Self::CappedByAvailableCash => {
                "Position size capped by available buying power / margin rather than risk budget."
                    .to_owned()
            }
            Self::CappedByMaxPositionPct { max_position_pct } => {
                format!(
                    "Position size capped by max concentration limit ({max_position_pct:.1}% of equity)."
                )
            }
            Self::InsufficientRiskForBoardLot {
                min_lot,
                single_lot_risk,
            } => {
                format!(
                    "1 board lot ({min_lot:.0}) risks {single_lot_risk:.2}, exceeding max risk budget (qty = 0)."
                )
            }
            Self::NegativeNetRewardAfterFriction => {
                "Net reward at target is <= 0 after round-trip commission, sell tax, and slippage."
                    .to_owned()
            }
        }
    }
}

/// Validation failure returned when a [`TradePlanInput`] is mathematically ill-posed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradePlanError {
    InvalidAccountState,
    InvalidPriceLevels,
    InvalidBracketsForSide,
    InvalidFrictionRates,
    DegenerateRiskDistance,
}

impl TradePlanError {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidAccountState => {
                "Account equity, cash, leverage, margin %, multiplier, and risk % must be finite and positive."
            }
            Self::InvalidPriceLevels => {
                "Entry, stop-loss, and target prices must be finite and > 0."
            }
            Self::InvalidBracketsForSide => {
                "Long requires Stop < Entry < Target; Short requires Target < Entry < Stop."
            }
            Self::InvalidFrictionRates => {
                "Commission, tax, and slippage bps must be finite and below 5,000 bps."
            }
            Self::DegenerateRiskDistance => {
                "Tick-snapped entry and stop-loss collapse to zero risk distance."
            }
        }
    }
}

/// Input configuration for [`compute_trade_plan`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePlanInput {
    pub venue: ExchangeVenue,
    pub price_unit: VnPriceUnit,
    pub side: PositionSide,
    pub account_equity: f64,
    pub available_cash: f64,
    pub leverage: f64,
    #[serde(default = "default_contract_multiplier")]
    pub contract_multiplier: f64,
    #[serde(default = "default_initial_margin_pct")]
    pub initial_margin_pct: f64,
    #[serde(default = "default_max_position_pct")]
    pub max_position_pct: f64,
    pub risk_pct: f64,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub target_price: f64,
    pub reference_price: Option<f64>,
    pub commission_bps: f64,
    pub tax_sell_bps: f64,
    pub slippage_bps: f64,
    pub snap_to_ticks: bool,
    pub entry_timestamp: Option<i64>,
}

impl Default for TradePlanInput {
    fn default() -> Self {
        let venue = ExchangeVenue::Hose;
        Self {
            venue,
            price_unit: VnPriceUnit::Auto,
            side: PositionSide::Long,
            account_equity: 100_000.0,
            available_cash: 100_000.0,
            leverage: 1.0,
            contract_multiplier: 1.0,
            initial_margin_pct: venue.default_initial_margin_pct(),
            max_position_pct: 100.0,
            risk_pct: 1.0,
            entry_price: 235.0,
            stop_loss: 228.0,
            target_price: 252.0,
            reference_price: Some(234.0),
            commission_bps: venue.default_commission_bps(),
            tax_sell_bps: venue.default_tax_sell_bps(),
            slippage_bps: 5.0,
            snap_to_ticks: true,
            entry_timestamp: None,
        }
    }
}

/// Complete friction-adjusted, tick-snapped, board-lot-rounded trade plan (`Milestone R10`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePlanComputed {
    pub venue: ExchangeVenue,
    pub side: PositionSide,
    pub contract_multiplier: f64,
    pub snapped_entry: f64,
    pub snapped_stop: f64,
    pub snapped_target: f64,
    pub daily_band: Option<DailyPriceBand>,
    pub breakeven_price: f64,
    pub max_risk_amount: f64,
    pub risk_per_unit_gross: f64,
    pub risk_per_unit_net: f64,
    pub reward_per_unit_gross: f64,
    pub reward_per_unit_net: f64,
    pub gross_r_multiple: f64,
    pub net_r_multiple: f64,
    pub required_win_rate_pct: f64,
    pub raw_risk_quantity: f64,
    pub max_cash_quantity: f64,
    pub planned_quantity: f64,
    pub position_notional: f64,
    pub margin_required: f64,
    pub planned_risk_amount: f64,
    pub planned_risk_pct_actual: f64,
    pub estimated_net_profit: f64,
    pub estimated_round_trip_friction: f64,
    pub settlement_timestamp: Option<i64>,
    pub affordable: bool,
    pub warnings: Vec<PlanWarning>,
}

/// Computes an exact all-in friction-aware position sizing plan, guaranteeing that realized loss
/// on stop-out (including `contract_multiplier`, entry commission, stop-exit commission,
/// `tax_sell_bps` on sell leg, and adverse entry/exit slippage) never exceeds
/// `account_equity * (risk_pct / 100.0)`.
pub fn compute_trade_plan(input: &TradePlanInput) -> Result<TradePlanComputed, TradePlanError> {
    if !input.account_equity.is_finite()
        || input.account_equity <= 0.0
        || !input.available_cash.is_finite()
        || input.available_cash < 0.0
        || !input.leverage.is_finite()
        || input.leverage <= 0.0
        || !input.contract_multiplier.is_finite()
        || !(1.0e-6..=1.0e9).contains(&input.contract_multiplier)
        || !input.initial_margin_pct.is_finite()
        || !(0.1..=1_000.0).contains(&input.initial_margin_pct)
        || !input.max_position_pct.is_finite()
        || !(0.1..=1_000.0).contains(&input.max_position_pct)
        || !input.risk_pct.is_finite()
        || input.risk_pct <= 0.0
        || input.risk_pct > 100.0
    {
        return Err(TradePlanError::InvalidAccountState);
    }

    if !input.entry_price.is_finite()
        || input.entry_price <= 0.0
        || input.entry_price > MAX_ORDER_PRICE
        || !input.stop_loss.is_finite()
        || input.stop_loss <= 0.0
        || input.stop_loss > MAX_ORDER_PRICE
        || !input.target_price.is_finite()
        || input.target_price <= 0.0
        || input.target_price > MAX_ORDER_PRICE
    {
        return Err(TradePlanError::InvalidPriceLevels);
    }

    if !input.commission_bps.is_finite()
        || !(0.0..5_000.0).contains(&input.commission_bps)
        || !input.tax_sell_bps.is_finite()
        || !(0.0..5_000.0).contains(&input.tax_sell_bps)
        || !input.slippage_bps.is_finite()
        || !(0.0..5_000.0).contains(&input.slippage_bps)
        || (input.commission_bps + input.tax_sell_bps) >= 9_500.0
    {
        return Err(TradePlanError::InvalidFrictionRates);
    }

    match input.side {
        PositionSide::Long => {
            if !(input.stop_loss < input.entry_price - 1.0e-9
                && input.target_price > input.entry_price + 1.0e-9)
            {
                return Err(TradePlanError::InvalidBracketsForSide);
            }
        }
        PositionSide::Short => {
            if !(input.stop_loss > input.entry_price + 1.0e-9
                && input.target_price < input.entry_price - 1.0e-9)
            {
                return Err(TradePlanError::InvalidBracketsForSide);
            }
        }
    }

    let unit_anchor = input
        .reference_price
        .filter(|r| r.is_finite() && *r > 0.0)
        .unwrap_or(input.entry_price);
    let resolved_unit = input.price_unit.resolve(unit_anchor);

    let (snapped_entry, snapped_stop, snapped_target) = if input.snap_to_ticks {
        let e = snap_to_nearest_tick(input.entry_price, input.venue, resolved_unit);
        match input.side {
            PositionSide::Long => {
                let mut s = round_down_to_tick(input.stop_loss, input.venue, resolved_unit);
                if s >= e || s <= 0.0 {
                    s = step_one_tick_below(e, input.venue, resolved_unit);
                }
                let mut t = round_down_to_tick(input.target_price, input.venue, resolved_unit);
                if t <= e {
                    t = step_one_tick_above(e, input.venue, resolved_unit);
                }
                (e, s, t)
            }
            PositionSide::Short => {
                let mut s = round_up_to_tick(input.stop_loss, input.venue, resolved_unit);
                if s <= e {
                    s = step_one_tick_above(e, input.venue, resolved_unit);
                }
                let mut t = round_up_to_tick(input.target_price, input.venue, resolved_unit);
                if t >= e || t <= 0.0 {
                    t = step_one_tick_below(e, input.venue, resolved_unit);
                }
                (e, s, t)
            }
        }
    } else {
        (input.entry_price, input.stop_loss, input.target_price)
    };

    let mult = if input.venue == ExchangeVenue::Vn30Futures
        && (input.contract_multiplier - 1.0).abs() < 1.0e-9
    {
        input
            .venue
            .default_contract_multiplier(input.price_unit, input.account_equity)
    } else {
        input.contract_multiplier
    };
    let eff_margin_pct = if input.venue == ExchangeVenue::Vn30Futures
        && (input.initial_margin_pct - 100.0).abs() < 1.0e-9
    {
        input.venue.default_initial_margin_pct()
    } else {
        input.initial_margin_pct
    };
    let risk_pts = (snapped_entry - snapped_stop).abs();
    let reward_pts = (snapped_target - snapped_entry).abs();
    let risk_per_unit_gross = risk_pts * mult;
    let reward_per_unit_gross = reward_pts * mult;
    if !risk_per_unit_gross.is_finite()
        || risk_pts <= 1.0e-9
        || snapped_stop <= 0.0
        || snapped_target <= 0.0
    {
        return Err(TradePlanError::DegenerateRiskDistance);
    }

    let c = input.commission_bps * 1.0e-4;
    let s = input.slippage_bps * 1.0e-4;
    let t_sell = input.tax_sell_bps * 1.0e-4;
    let lev = input.leverage.max(1.0e-6);
    let margin_frac = ((eff_margin_pct / 100.0) / lev).max(1.0e-6);

    let (
        risk_per_unit_net,
        reward_per_unit_net,
        breakeven_price,
        cash_req_per_unit,
        unit_margin_only,
        unit_friction,
    ) = match input.side {
        PositionSide::Long => {
            let entry_exec = snapped_entry * (1.0 + s);
            let cost_in = entry_exec * mult * (1.0 + c);
            let stop_exec = snapped_stop * (1.0 - s);
            let proceeds_stop = stop_exec * mult * (1.0 - c - t_sell);
            let target_exec = snapped_target * (1.0 - s);
            let proceeds_target = target_exec * mult * (1.0 - c - t_sell);

            let denom = (1.0 - s) * (1.0 - c - t_sell);
            let be = if denom > 1.0e-9 {
                (entry_exec * (1.0 + c)) / denom
            } else {
                snapped_entry
            };
            let margin_only = entry_exec * mult * margin_frac;
            let cash_per_unit = margin_only + (entry_exec * mult * c);
            let rt_friction = mult
                * ((entry_exec - snapped_entry)
                    + (entry_exec * c)
                    + (snapped_target - target_exec)
                    + (target_exec * (c + t_sell)));
            (
                cost_in - proceeds_stop,
                proceeds_target - cost_in,
                be,
                cash_per_unit,
                margin_only,
                rt_friction,
            )
        }
        PositionSide::Short => {
            let entry_exec = snapped_entry * (1.0 - s);
            let credit_in = entry_exec * mult * (1.0 - c - t_sell);
            let stop_exec = snapped_stop * (1.0 + s);
            let cost_stop = stop_exec * mult * (1.0 + c);
            let target_exec = snapped_target * (1.0 + s);
            let cost_target = target_exec * mult * (1.0 + c);

            let denom = (1.0 + s) * (1.0 + c);
            let be = if denom > 1.0e-9 {
                (entry_exec * (1.0 - c - t_sell)) / denom
            } else {
                snapped_entry
            };
            let margin_only = entry_exec * mult * margin_frac;
            let cash_per_unit = margin_only + (entry_exec * mult * (c + t_sell));
            let rt_friction = mult
                * ((snapped_entry - entry_exec)
                    + (entry_exec * (c + t_sell))
                    + (target_exec - snapped_target)
                    + (target_exec * c));
            (
                cost_stop - credit_in,
                credit_in - cost_target,
                be,
                cash_per_unit,
                margin_only,
                rt_friction,
            )
        }
    };

    if !risk_per_unit_net.is_finite() || risk_per_unit_net <= 1.0e-9 {
        return Err(TradePlanError::DegenerateRiskDistance);
    }

    let max_risk_amount = input.account_equity * (input.risk_pct / 100.0);
    let raw_risk_quantity = (max_risk_amount / risk_per_unit_net).min(MAX_ORDER_QUANTITY);
    let max_cash_quantity = if cash_req_per_unit > 1.0e-12 {
        (input.available_cash / cash_req_per_unit).min(MAX_ORDER_QUANTITY)
    } else {
        0.0
    };
    let max_conc_margin = input.account_equity * (input.max_position_pct / 100.0);
    let max_conc_quantity = if unit_margin_only > 1.0e-12 {
        (max_conc_margin / unit_margin_only).min(MAX_ORDER_QUANTITY)
    } else {
        MAX_ORDER_QUANTITY
    };

    let uncapped_qty = raw_risk_quantity
        .min(max_cash_quantity)
        .min(max_conc_quantity);
    let short_disallowed = input.side == PositionSide::Short && !input.venue.allows_short();
    let planned_quantity = if short_disallowed {
        0.0
    } else {
        floor_to_board_lot(input.venue, uncapped_qty)
    };

    let gross_r_multiple = reward_per_unit_gross / risk_per_unit_gross;
    let net_r_multiple = reward_per_unit_net / risk_per_unit_net;
    let required_win_rate_pct = if net_r_multiple > 0.0 {
        100.0 / (1.0 + net_r_multiple)
    } else {
        100.0
    };

    let position_notional = planned_quantity * snapped_entry * mult;
    let margin_required = planned_quantity * cash_req_per_unit;
    let planned_risk_amount = planned_quantity * risk_per_unit_net;
    let planned_risk_pct_actual = (planned_risk_amount / input.account_equity) * 100.0;
    let estimated_net_profit = planned_quantity * reward_per_unit_net;
    let estimated_round_trip_friction = planned_quantity * unit_friction;

    let daily_band = input
        .reference_price
        .and_then(|ref_px| compute_daily_price_band(input.venue, ref_px, input.price_unit));

    let mut warnings = Vec::new();
    if short_disallowed {
        warnings.push(PlanWarning::ShortDisallowedOnVenue);
    }
    if let Some(band) = daily_band {
        let tol = 1.0e-6;
        if snapped_entry > band.ceiling_price + tol {
            warnings.push(PlanWarning::EntryAboveDailyCeiling {
                ceiling: band.ceiling_price,
            });
        }
        if snapped_entry < band.floor_price - tol {
            warnings.push(PlanWarning::EntryBelowDailyFloor {
                floor: band.floor_price,
            });
        }
        match input.side {
            PositionSide::Long => {
                if snapped_stop < band.floor_price - tol {
                    warnings.push(PlanWarning::StopBelowDailyFloorGapRisk {
                        floor: band.floor_price,
                    });
                }
                if snapped_target > band.ceiling_price + tol {
                    warnings.push(PlanWarning::TargetAboveDailyCeilingMultiSession {
                        ceiling: band.ceiling_price,
                    });
                }
            }
            PositionSide::Short => {
                if snapped_stop > band.ceiling_price + tol {
                    warnings.push(PlanWarning::StopAboveDailyCeilingGapRisk {
                        ceiling: band.ceiling_price,
                    });
                }
                if snapped_target < band.floor_price - tol {
                    warnings.push(PlanWarning::TargetBelowDailyFloorMultiSession {
                        floor: band.floor_price,
                    });
                }
            }
        }
    }

    if max_cash_quantity + 1.0e-9 < raw_risk_quantity && max_cash_quantity <= max_conc_quantity {
        warnings.push(PlanWarning::CappedByAvailableCash);
    } else if max_conc_quantity + 1.0e-9 < raw_risk_quantity {
        warnings.push(PlanWarning::CappedByMaxPositionPct {
            max_position_pct: input.max_position_pct,
        });
    }
    let min_lot = input.venue.board_lot();
    let single_lot_risk = min_lot * risk_per_unit_net;
    if !short_disallowed && planned_quantity <= 0.0 && raw_risk_quantity < min_lot {
        warnings.push(PlanWarning::InsufficientRiskForBoardLot {
            min_lot,
            single_lot_risk,
        });
    }
    if reward_per_unit_net <= 0.0 {
        warnings.push(PlanWarning::NegativeNetRewardAfterFriction);
    }

    let settlement_timestamp = input
        .entry_timestamp
        .map(|ts| compute_settlement_timestamp_for_venue(ts, input.venue));

    Ok(TradePlanComputed {
        venue: input.venue,
        side: input.side,
        contract_multiplier: mult,
        snapped_entry,
        snapped_stop,
        snapped_target,
        daily_band,
        breakeven_price,
        max_risk_amount,
        risk_per_unit_gross,
        risk_per_unit_net,
        reward_per_unit_gross,
        reward_per_unit_net,
        gross_r_multiple,
        net_r_multiple,
        required_win_rate_pct,
        raw_risk_quantity,
        max_cash_quantity,
        planned_quantity,
        position_notional,
        margin_required,
        planned_risk_amount,
        planned_risk_pct_actual,
        estimated_net_profit,
        estimated_round_trip_friction,
        settlement_timestamp,
        affordable: planned_quantity > 0.0 && !short_disallowed,
        warnings,
    })
}
