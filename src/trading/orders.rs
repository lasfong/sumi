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

/// Paper-trading account rules and execution friction parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountConfig {
    /// Starting account cash (`default = 100_000.0`).
    pub initial_cash: f64,
    /// Per-fill commission in basis points (`1 bp = 0.01%`, `default = 5.0` bps = `0.05%`).
    pub commission_bps: f64,
    /// Adverse execution slippage in basis points (`default = 2.0` bps = `0.02%`).
    pub slippage_bps: f64,
    /// Maximum notional-to-equity leverage (`default = 1.0` cash-backed).
    pub max_leverage: f64,
    /// Whether `Short` positions are permitted (`default = true`).
    pub allow_short: bool,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            initial_cash: 100_000.0,
            commission_bps: 5.0,
            slippage_bps: 2.0,
            max_leverage: 1.0,
            allow_short: true,
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

        if !self.slippage_bps.is_finite() {
            self.slippage_bps = 2.0;
        }
        self.slippage_bps = self.slippage_bps.clamp(0.0, 500.0);

        if !self.max_leverage.is_finite() {
            self.max_leverage = 1.0;
        }
        self.max_leverage = self.max_leverage.clamp(1.0, 100.0);
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
    let s = (slippage_bps.clamp(0.0, 500.0)) * 1.0e-4;
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
