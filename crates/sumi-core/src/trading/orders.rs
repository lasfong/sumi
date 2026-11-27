use core::fmt;

use serde::{Deserialize, Serialize};

/// Minimum positive order quantity accepted (`0.00000001`, supporting satoshi-scale crypto).
pub const MIN_ORDER_QUANTITY: f64 = 1.0e-8;

/// Maximum single-order quantity accepted.
pub const MAX_ORDER_QUANTITY: f64 = 1.0e9;

/// Maximum order/bracket price accepted.
pub const MAX_ORDER_PRICE: f64 = 1.0e12;

/// Maximum order notional (`price * quantity`) accepted to prevent `f64` precision overflow.
pub const MAX_ORDER_NOTIONAL: f64 = 1.0e14;

/// Maximum simultaneous `Pending` working orders allowed in a session.
pub const MAX_PENDING_ORDERS: usize = 64;

/// Maximum recorded user commands retained in the deterministic replay log.
pub const MAX_RECORDED_COMMANDS: usize = 16_384;

const fn default_base_timeframe_secs() -> i64 {
    86_400
}

const fn default_contract_multiplier_one() -> f64 {
    1.0
}

const fn default_initial_margin_pct_100() -> f64 {
    100.0
}

/// Paper-trading account rules and execution friction parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountConfig {
    /// Starting account cash (`default = 100_000.0`).
    pub initial_cash: f64,
    /// Per-fill commission in basis points (`1 bp = 0.01%`, `default = 5.0` bps = `0.05%`).
    pub commission_bps: f64,
    /// Statutory securities transfer tax on gross sell proceeds in basis points (`10.0 bps = 0.10%` on VN cash equities; `default = 0.0`).
    #[serde(default)]
    pub tax_sell_bps: f64,
    /// Adverse execution slippage in basis points (`default = 2.0` bps = `0.02%`).
    pub slippage_bps: f64,
    /// When `false` (default), `Limit` orders fill strictly at their limit price without
    /// adverse slippage exceeding the limit. Set `true` only for conservative stress testing.
    #[serde(default)]
    pub apply_slippage_to_limits: bool,
    /// Point-to-currency contract multiplier (`100.0 kVND/pt` or `100_000.0 VND/pt` for `VN30F1M`, `1.0` for cash/spot).
    #[serde(default = "default_contract_multiplier_one")]
    pub contract_multiplier: f64,
    /// Initial margin requirement percentage (`17.0%` for VSD/HNX `VN30F1M`, `100.0%` cash-funded).
    #[serde(default = "default_initial_margin_pct_100")]
    pub initial_margin_pct: f64,
    /// Maximum notional-to-equity leverage (`default = 1.0` cash-backed).
    pub max_leverage: f64,
    /// Whether `Short` positions are permitted (`default = true`; always gated by `venue.allows_short()`).
    pub allow_short: bool,
    /// Whether VSD `T+2` (`13:00 ICT`) / historical `T+3` (`09:00 ICT`) lot lock is enforced on Vietnam cash equities.
    #[serde(default)]
    pub enforce_vn_t2_lot_lock: bool,
    /// Active exchange venue governing settlement rules (`default = UsEquities`).
    #[serde(default)]
    pub venue: crate::journal::ExchangeVenue,
    /// Base candle duration in seconds used to evaluate coarse-bar (`D1`/`H4`) causal close settlement (`default = 86_400`).
    #[serde(default = "default_base_timeframe_secs")]
    pub base_timeframe_secs: i64,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            initial_cash: 100_000.0,
            commission_bps: 5.0,
            tax_sell_bps: 0.0,
            slippage_bps: 2.0,
            apply_slippage_to_limits: false,
            contract_multiplier: 1.0,
            initial_margin_pct: 100.0,
            max_leverage: 1.0,
            allow_short: true,
            enforce_vn_t2_lot_lock: false,
            venue: crate::journal::ExchangeVenue::UsEquities,
            base_timeframe_secs: 86_400,
        }
    }
}

impl AccountConfig {
    pub fn sanitize(&mut self) {
        if !self.initial_cash.is_finite() {
            self.initial_cash = 100_000.0;
        }
        self.initial_cash = self.initial_cash.clamp(100.0, 1.0e12);

        if !self.commission_bps.is_finite() {
            self.commission_bps = 5.0;
        }
        self.commission_bps = self.commission_bps.clamp(0.0, 500.0);

        if !self.tax_sell_bps.is_finite() {
            self.tax_sell_bps = 0.0;
        }
        self.tax_sell_bps = self.tax_sell_bps.clamp(0.0, 500.0);

        if !self.slippage_bps.is_finite() {
            self.slippage_bps = 2.0;
        }
        self.slippage_bps = self.slippage_bps.clamp(0.0, 500.0);

        if !self.contract_multiplier.is_finite() || self.contract_multiplier <= 0.0 {
            self.contract_multiplier = 1.0;
        }
        self.contract_multiplier = self.contract_multiplier.clamp(1.0e-6, 1.0e9);

        if !self.initial_margin_pct.is_finite() || self.initial_margin_pct <= 0.0 {
            self.initial_margin_pct = 100.0;
        }
        self.initial_margin_pct = self.initial_margin_pct.clamp(1.0, 100.0);

        if !self.max_leverage.is_finite() {
            self.max_leverage = 1.0;
        }
        self.max_leverage = self.max_leverage.clamp(1.0, 100.0);
        self.base_timeframe_secs = self.base_timeframe_secs.clamp(1, 31_536_000);
    }

    /// Effective contract multiplier (`100.0 kVND/pt` or `100_000.0 VND/pt` for `VN30F1M`, `1.0` for spot/equities).
    #[inline]
    #[must_use]
    pub fn effective_contract_multiplier(&self) -> f64 {
        if self.venue == crate::journal::ExchangeVenue::Vn30Futures
            && (self.contract_multiplier - 1.0).abs() < 1.0e-9
        {
            self.venue
                .default_contract_multiplier(crate::journal::VnPriceUnit::Auto, self.initial_cash)
        } else if self.contract_multiplier.is_finite() && self.contract_multiplier > 0.0 {
            self.contract_multiplier
        } else {
            1.0
        }
    }

    /// Effective initial margin percentage (`17.0%` on `VN30F1M` unless explicitly overridden).
    #[inline]
    #[must_use]
    pub fn effective_initial_margin_pct(&self) -> f64 {
        if self.venue == crate::journal::ExchangeVenue::Vn30Futures
            && (self.initial_margin_pct - 100.0).abs() < 1.0e-9
        {
            self.venue.default_initial_margin_pct()
        } else if self.initial_margin_pct.is_finite() && self.initial_margin_pct > 0.0 {
            self.initial_margin_pct.clamp(1.0, 100.0)
        } else {
            100.0
        }
    }

    /// Short selling is permitted strictly when both account policy and exchange venue allow shorting (`BUG-008`).
    #[inline]
    #[must_use]
    pub const fn effective_allow_short(&self) -> bool {
        self.allow_short && self.venue.allows_short()
    }

    /// Computes total execution fee (`commission + sell transfer tax`) respecting `contract_multiplier` (`BUG-001`).
    #[inline]
    #[must_use]
    pub fn compute_fill_fee(&self, price: f64, quantity: f64, side: OrderSide) -> f64 {
        let mult = self.effective_contract_multiplier();
        let gross_notional = price * quantity * mult;
        let comm = gross_notional * (self.commission_bps.clamp(0.0, 500.0) * 1.0e-4);
        let sell_tax = match side {
            OrderSide::Sell => gross_notional * (self.tax_sell_bps.clamp(0.0, 500.0) * 1.0e-4),
            OrderSide::Buy => 0.0,
        };
        (comm + sell_tax).max(0.0)
    }

    /// Computes required margin (`quantity * price * contract_multiplier * (initial_margin_pct / 100) / max_leverage`).
    #[inline]
    #[must_use]
    pub fn required_margin_for(&self, price: f64, quantity: f64) -> f64 {
        let mult = self.effective_contract_multiplier();
        let margin_frac =
            (self.effective_initial_margin_pct() / 100.0) / self.max_leverage.max(1.0);
        (quantity * price * mult * margin_frac).max(0.0)
    }
}

/// Directional side of an order or fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    #[inline]
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }

    #[inline]
    #[must_use]
    pub const fn to_position_side(self) -> PositionSide {
        match self {
            Self::Buy => PositionSide::Long,
            Self::Sell => PositionSide::Short,
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

/// Net directional exposure of an open or closed position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

impl PositionSide {
    #[inline]
    #[must_use]
    pub const fn entry_order_side(self) -> OrderSide {
        match self {
            Self::Long => OrderSide::Buy,
            Self::Short => OrderSide::Sell,
        }
    }

    #[inline]
    #[must_use]
    pub const fn exit_order_side(self) -> OrderSide {
        match self {
            Self::Long => OrderSide::Sell,
            Self::Short => OrderSide::Buy,
        }
    }

    #[inline]
    #[must_use]
    pub const fn sign(self) -> f64 {
        match self {
            Self::Long => 1.0,
            Self::Short => -1.0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
        }
    }
}

/// Order execution trigger semantics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderKind {
    Market,
    Limit { limit_price: f64 },
    Stop { stop_price: f64 },
}

impl OrderKind {
    #[inline]
    #[must_use]
    pub const fn trigger_price(self) -> Option<f64> {
        match self {
            Self::Market => None,
            Self::Limit { limit_price } => Some(limit_price),
            Self::Stop { stop_price } => Some(stop_price),
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Market => "MKT",
            Self::Limit { .. } => "LMT",
            Self::Stop { .. } => "STP",
        }
    }
}

/// Lifecycle state of an [`Order`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Filled,
    Cancelled,
    Rejected { reason: String },
}

/// Cause of an [`crate::trading::ExecutionFill`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FillReason {
    MarketImmediate,
    LimitTriggered,
    StopTriggered,
    StopLossExit,
    TakeProfitExit,
    ManualFlatten,
}

impl FillReason {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::MarketImmediate => "MKT",
            Self::LimitTriggered => "LMT",
            Self::StopTriggered => "STP",
            Self::StopLossExit => "SL",
            Self::TakeProfitExit => "TP",
            Self::ManualFlatten => "EXIT",
        }
    }
}

/// Validated or pending order record in the session order book.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    pub side: OrderSide,
    pub kind: OrderKind,
    pub quantity: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub submitted_base_index: usize,
    pub submitted_timestamp: i64,
    pub status: OrderStatus,
    pub filled_price: Option<f64>,
    pub filled_timestamp: Option<i64>,
}

/// Input parameters for submitting a new paper-trading order.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrderRequest {
    pub side: OrderSide,
    pub kind: OrderKind,
    pub quantity: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

/// Structured validation and execution errors returned by [`crate::trading::TradingSession`].
#[derive(Debug, Clone, PartialEq)]
pub enum TradingError {
    EmptyMarketData,
    InvalidQuantity(f64),
    InvalidPrice(f64),
    NotionalOverflow,
    ShortSellingDisabled,
    InsufficientBuyingPower {
        required: f64,
        available: f64,
    },
    InsufficientSettledShares {
        requested: f64,
        sellable: f64,
        locked_t2: f64,
        earliest_unlock_timestamp: Option<i64>,
    },
    InvalidStopLoss {
        stop_loss: f64,
        reference: f64,
        side: PositionSide,
    },
    InvalidTakeProfit {
        take_profit: f64,
        reference: f64,
        side: PositionSide,
    },
    NoOpenPosition,
    OrderNotFound(u64),
    OrderNotPending(u64),
    TooManyPendingOrders(usize),
    CommandLogFull(usize),
}

impl fmt::Display for TradingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMarketData => write!(f, "no market candles available for paper trading"),
            Self::InvalidQuantity(q) => write!(
                f,
                "invalid order quantity {q} (expected [{MIN_ORDER_QUANTITY}, {MAX_ORDER_QUANTITY}])"
            ),
            Self::InvalidPrice(p) => {
                write!(f, "invalid price {p} (expected (0, {MAX_ORDER_PRICE}])")
            }
            Self::NotionalOverflow => {
                write!(f, "order notional exceeds maximum {MAX_ORDER_NOTIONAL:.0}")
            }
            Self::ShortSellingDisabled => write!(f, "short selling is disabled in account rules"),
            Self::InsufficientBuyingPower {
                required,
                available,
            } => write!(
                f,
                "insufficient buying power: required {required:.2}, available {available:.2}"
            ),
            Self::InsufficientSettledShares {
                requested,
                sellable,
                locked_t2,
                ..
            } => write!(
                f,
                "VSD T+2 lock: requested sell {requested:.0} exceeds settled {sellable:.0} ({locked_t2:.0} awaiting T+2 13:00 ICT)"
            ),
            Self::InvalidStopLoss {
                stop_loss,
                reference,
                side,
            } => match side {
                PositionSide::Long => write!(
                    f,
                    "LONG stop loss ({stop_loss:.4}) must be strictly below reference ({reference:.4})"
                ),
                PositionSide::Short => write!(
                    f,
                    "SHORT stop loss ({stop_loss:.4}) must be strictly above reference ({reference:.4})"
                ),
            },
            Self::InvalidTakeProfit {
                take_profit,
                reference,
                side,
            } => match side {
                PositionSide::Long => write!(
                    f,
                    "LONG take profit ({take_profit:.4}) must be strictly above reference ({reference:.4})"
                ),
                PositionSide::Short => write!(
                    f,
                    "SHORT take profit ({take_profit:.4}) must be strictly below reference ({reference:.4})"
                ),
            },
            Self::NoOpenPosition => write!(f, "no open position"),
            Self::OrderNotFound(id) => write!(f, "order #{id} not found"),
            Self::OrderNotPending(id) => write!(f, "order #{id} is no longer pending"),
            Self::TooManyPendingOrders(max) => {
                write!(f, "pending order book limit ({max}) reached")
            }
            Self::CommandLogFull(max) => {
                write!(
                    f,
                    "session command log limit ({max}) reached; reset session to continue"
                )
            }
        }
    }
}

impl std::error::Error for TradingError {}

/// Applies adverse slippage (`slippage_bps` clamped to `[0, 500]`) to a raw trigger/mark price.
#[inline]
#[must_use]
pub fn apply_slippage(raw_price: f64, side: OrderSide, slippage_bps: f64) -> f64 {
    if !raw_price.is_finite() || raw_price <= 0.0 {
        return f64::NAN;
    }
    let s = if slippage_bps.is_finite() {
        slippage_bps.clamp(0.0, 500.0) * 1.0e-4
    } else {
        0.0
    };
    let factor = match side {
        OrderSide::Buy => 1.0 + s,
        OrderSide::Sell => (1.0 - s).max(1.0e-6),
    };
    (raw_price * factor).max(1.0e-9)
}

/// Computes per-fill execution commission from `price * quantity * (commission_bps * 1e-4)`.
#[inline]
#[must_use]
pub fn compute_commission(price: f64, quantity: f64, commission_bps: f64) -> f64 {
    let c = (commission_bps.clamp(0.0, 500.0)) * 1.0e-4;
    (price * quantity * c).max(0.0)
}

/// Validates directional consistency of optional `stop_loss` and `take_profit` levels against `reference_price`.
pub fn validate_brackets_for_side(
    side: PositionSide,
    reference_price: f64,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
) -> Result<(), TradingError> {
    if !reference_price.is_finite() || reference_price <= 0.0 || reference_price > MAX_ORDER_PRICE {
        return Err(TradingError::InvalidPrice(reference_price));
    }
    if let Some(sl) = stop_loss {
        if !sl.is_finite() || sl <= 0.0 || sl > MAX_ORDER_PRICE {
            return Err(TradingError::InvalidPrice(sl));
        }
        let valid = match side {
            PositionSide::Long => sl < reference_price,
            PositionSide::Short => sl > reference_price,
        };
        if !valid {
            return Err(TradingError::InvalidStopLoss {
                stop_loss: sl,
                reference: reference_price,
                side,
            });
        }
    }
    if let Some(tp) = take_profit {
        if !tp.is_finite() || tp <= 0.0 || tp > MAX_ORDER_PRICE {
            return Err(TradingError::InvalidPrice(tp));
        }
        let valid = match side {
            PositionSide::Long => tp > reference_price,
            PositionSide::Short => tp < reference_price,
        };
        if !valid {
            return Err(TradingError::InvalidTakeProfit {
                take_profit: tp,
                reference: reference_price,
                side,
            });
        }
    }
    Ok(())
}
