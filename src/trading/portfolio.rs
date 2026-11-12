use serde::{Deserialize, Serialize};

use crate::drawings::{Drawing, DrawingGeometry, DrawingPaneTarget, fibonacci_level_value};
use crate::model::Candle;

use super::orders::{
    AccountConfig, FillReason, MAX_ORDER_NOTIONAL, MAX_ORDER_PRICE, MAX_ORDER_QUANTITY,
    MAX_PENDING_ORDERS, MAX_RECORDED_COMMANDS, MIN_ORDER_QUANTITY, Order, OrderKind, OrderRequest,
    OrderSide, OrderStatus, PositionSide, TradingError, apply_slippage, compute_commission,
    validate_brackets_for_side,
};

/// Active net position in the paper-trading portfolio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub side: PositionSide,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub opened_timestamp: i64,
    pub opened_base_index: usize,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub initial_stop_loss: Option<f64>,
    pub initial_risk_amount: Option<f64>,
    pub total_entry_qty: f64,
    pub total_entry_notional: f64,
    pub total_entry_fees: f64,
    pub open_entry_fees: f64,
    pub closed_qty_acc: f64,
    pub exit_notional_acc: f64,
    pub realized_gross_pnl_acc: f64,
    pub realized_exit_fees_acc: f64,
}

impl Position {
    /// Mark-to-market unrealized gross PnL at `mark_price`.
    #[inline]
    #[must_use]
    pub fn unrealized_pnl(&self, mark_price: f64) -> f64 {
        if !mark_price.is_finite() || self.quantity <= 0.0 {
            return 0.0;
        }
        self.side.sign() * (mark_price - self.avg_entry_price) * self.quantity
    }

    /// Percentage unrealized move relative to `avg_entry_price`.
    #[inline]
    #[must_use]
    pub fn unrealized_return_pct(&self, mark_price: f64) -> f64 {
        if self.avg_entry_price <= 0.0 || !mark_price.is_finite() {
            return 0.0;
        }
        self.side.sign() * ((mark_price - self.avg_entry_price) / self.avg_entry_price) * 100.0
    }
}

/// Immutable execution fill record emitted for chart markers and audit blotter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionFill {
    pub fill_id: u64,
    pub order_id: u64,
    pub side: OrderSide,
    pub reason: FillReason,
    pub quantity: f64,
    pub price: f64,
    pub fee: f64,
    pub realized_net_pnl: Option<f64>,
    pub base_index: usize,
    pub timestamp: i64,
}

/// Completed round-trip position lifecycle record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedTrade {
    pub trade_id: u64,
    pub side: PositionSide,
    pub quantity: f64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    pub gross_pnl: f64,
    pub total_fees: f64,
    pub net_pnl: f64,
    pub return_pct: f64,
    pub r_multiple: Option<f64>,
}

/// Mark-to-market equity curve sample at a causal bar close.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EquityPoint {
    pub base_index: usize,
    pub timestamp: i64,
    pub equity: f64,
    pub drawdown_pct: f64,
}

/// Aggregated scoreboard metrics for the workstation HUD and benchmark telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PortfolioMetrics {
    pub cash_balance: f64,
    pub unrealized_pnl: f64,
    pub equity: f64,
    pub net_pnl: f64,
    pub return_pct: f64,
    pub max_drawdown_pct: f64,
    pub pending_order_count: usize,
    pub fill_count: usize,
    pub closed_trade_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub win_rate_pct: f64,
    pub profit_factor: Option<f64>,
    pub avg_r_multiple: Option<f64>,
}

/// Deterministic user action stored in the command log for bit-exact causal rewind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordedCommandKind {
    SubmitOrder(OrderRequest),
    CancelOrder {
        order_id: u64,
    },
    UpdateBrackets {
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
    },
    FlattenPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedCommand {
    pub base_index: usize,
    pub kind: RecordedCommandKind,
}

/// Suggested order-ticket parameters extracted from a selected chart [`Drawing`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawingTicketSync {
    pub side: OrderSide,
    pub entry_price: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

/// Extracts type-safe paper-trading ticket parameters from a selected main-pane [`Drawing`].
#[must_use]
pub fn extract_ticket_from_drawing(drawing: &Drawing) -> Option<DrawingTicketSync> {
    if drawing.pane != DrawingPaneTarget::Main {
        return None;
    }
    match drawing.geometry {
        DrawingGeometry::HorizontalLine { anchor } => {
            (anchor.value.is_finite() && anchor.value > 0.0 && anchor.value <= MAX_ORDER_PRICE)
                .then_some(DrawingTicketSync {
                    side: OrderSide::Buy,
                    entry_price: anchor.value,
                    stop_loss: None,
                    take_profit: None,
                })
        }
        DrawingGeometry::FibonacciRetracement {
            start,
            end,
            direction,
            ..
        } => {
            let low = start.value.min(end.value);
            let high = start.value.max(end.value);
            if !low.is_finite()
                || !high.is_finite()
                || low <= 0.0
                || high > MAX_ORDER_PRICE
                || (high - low) <= 1e-6
            {
                return None;
            }
            let raw_fib_618 = fibonacci_level_value(start, end, 0.618, direction);
            let entry = raw_fib_618.clamp(low + 0.1 * (high - low), high - 0.1 * (high - low));
            if end.value >= start.value {
                Some(DrawingTicketSync {
                    side: OrderSide::Buy,
                    entry_price: entry,
                    stop_loss: Some(low),
                    take_profit: Some(high),
                })
            } else {
                Some(DrawingTicketSync {
                    side: OrderSide::Sell,
                    entry_price: entry,
                    stop_loss: Some(high),
                    take_profit: Some(low),
                })
            }
        }
        DrawingGeometry::Rectangle { corner_a, corner_b }
        | DrawingGeometry::TrendLine {
            start: corner_a,
            end: corner_b,
        }
        | DrawingGeometry::Ray {
            origin: corner_a,
            through: corner_b,
        } => {
            let low = corner_a.value.min(corner_b.value);
            let high = corner_a.value.max(corner_b.value);
            if !low.is_finite()
                || !high.is_finite()
                || low <= 0.0
                || high > MAX_ORDER_PRICE
                || (high - low) <= 1e-6
            {
                return None;
            }
            let mid = 0.5 * (low + high);
            if corner_b.value >= corner_a.value {
                Some(DrawingTicketSync {
                    side: OrderSide::Buy,
                    entry_price: mid,
                    stop_loss: Some(low),
                    take_profit: Some(high),
                })
            } else {
                Some(DrawingTicketSync {
                    side: OrderSide::Sell,
                    entry_price: mid,
                    stop_loss: Some(high),
                    take_profit: Some(low),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SegmentCrossingEvent {
    BracketExit {
        reason: FillReason,
        trigger_price: f64,
    },
    PendingOrder {
        ord_idx: usize,
        trigger_price: f64,
        reason: FillReason,
    },
}

/// Deterministic causal paper-trading ledger and execution engine (`Milestone R07`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TradingSession {
    pub config: AccountConfig,
    pub session_start_base_index: usize,
    pub commands: Vec<RecordedCommand>,

    #[serde(skip)]
    cash_balance: f64,
    #[serde(skip)]
    open_position: Option<Position>,
    #[serde(skip)]
    orders: Vec<Order>,
    #[serde(skip)]
    fills: Vec<ExecutionFill>,
    #[serde(skip)]
    closed_trades: Vec<ClosedTrade>,
    #[serde(skip)]
    equity_curve: Vec<EquityPoint>,
    #[serde(skip)]
    peak_equity: f64,
    #[serde(skip)]
    max_drawdown_pct: f64,
    #[serde(skip)]
    next_order_id: u64,
    #[serde(skip)]
    next_fill_id: u64,
    #[serde(skip)]
    next_trade_id: u64,
    #[serde(skip)]
    last_processed_base_index: Option<usize>,
}

impl Default for TradingSession {
    fn default() -> Self {
        Self::new(AccountConfig::default())
    }
}

impl TradingSession {
    #[must_use]
    pub fn new(mut config: AccountConfig) -> Self {
        config.sanitize();
        let initial_cash = config.initial_cash;
        Self {
            config,
            session_start_base_index: 0,
            commands: Vec::new(),
            cash_balance: initial_cash,
            open_position: None,
            orders: Vec::new(),
            fills: Vec::new(),
            closed_trades: Vec::new(),
            equity_curve: Vec::new(),
            peak_equity: initial_cash,
            max_drawdown_pct: 0.0,
            next_order_id: 1,
            next_fill_id: 1,
            next_trade_id: 1,
            last_processed_base_index: None,
        }
    }

    /// Resets the trading session cleanly at `start_base_index` on `base_candles`.
    pub fn reset_at(&mut self, base_candles: &[Candle], start_base_index: usize) {
        self.config.sanitize();
        self.commands.clear();
        let clamped_start = if base_candles.is_empty() {
            0
        } else {
            start_base_index.min(base_candles.len() - 1)
        };
        self.session_start_base_index = clamped_start;
        self.clear_runtime_ledger();
        if let Some(c) = base_candles.get(clamped_start) {
            self.last_processed_base_index = Some(clamped_start);
            self.record_equity_point(clamped_start, c.timestamp, c.close);
        }
    }

    /// Sanitizes deserialized configuration and deterministically rebuilds the ledger up to
    /// `cursor_base_index` from `self.commands`.
    pub fn sanitize_and_rebuild(&mut self, base_candles: &[Candle], cursor_base_index: usize) {
        self.config.sanitize();
        if self.commands.len() > MAX_RECORDED_COMMANDS {
            self.commands.truncate(MAX_RECORDED_COMMANDS);
        }
        if base_candles.is_empty() {
            self.commands.clear();
            self.session_start_base_index = 0;
            self.clear_runtime_ledger();
            return;
        }
        let clamped_cursor = cursor_base_index.min(base_candles.len() - 1);
        self.session_start_base_index = self.session_start_base_index.min(clamped_cursor);
        self.rewind_to(base_candles, self.session_start_base_index, clamped_cursor);
    }

    #[inline]
    #[must_use]
    pub const fn cash_balance(&self) -> f64 {
        self.cash_balance
    }

    #[inline]
    #[must_use]
    pub const fn open_position(&self) -> Option<&Position> {
        self.open_position.as_ref()
    }

    #[inline]
    #[must_use]
    pub fn orders(&self) -> &[Order] {
        &self.orders
    }

    pub fn pending_orders(&self) -> impl Iterator<Item = &Order> {
        self.orders
            .iter()
            .filter(|o| o.status == OrderStatus::Pending)
    }

    #[inline]
    #[must_use]
    pub fn fills(&self) -> &[ExecutionFill] {
        &self.fills
    }

    #[inline]
    #[must_use]
    pub fn closed_trades(&self) -> &[ClosedTrade] {
        &self.closed_trades
    }

    #[inline]
    #[must_use]
    pub fn equity_curve(&self) -> &[EquityPoint] {
        &self.equity_curve
    }

    #[inline]
    #[must_use]
    pub const fn max_drawdown_pct(&self) -> f64 {
        self.max_drawdown_pct
    }

    /// Computes account equity (`cash_balance + unrealized_pnl`) at `mark_price`.
    #[inline]
    #[must_use]
    pub fn equity_at_price(&self, mark_price: f64) -> f64 {
        let unreal = self
            .open_position
            .as_ref()
            .map_or(0.0, |p| p.unrealized_pnl(mark_price));
        self.cash_balance + unreal
    }

    /// Computes a complete snapshot of portfolio performance metrics at `mark_price`.
    #[must_use]
    pub fn metrics(&self, mark_price: f64) -> PortfolioMetrics {
        let unrealized_pnl = self
            .open_position
            .as_ref()
            .map_or(0.0, |p| p.unrealized_pnl(mark_price));
        let equity = self.cash_balance + unrealized_pnl;
        let net_pnl = equity - self.config.initial_cash;
        let return_pct = if self.config.initial_cash > 0.0 {
            (net_pnl / self.config.initial_cash) * 100.0
        } else {
            0.0
        };

        let mut win_count = 0_usize;
        let mut loss_count = 0_usize;
        let mut gross_wins = 0.0_f64;
        let mut gross_losses = 0.0_f64;
        let mut r_sum = 0.0_f64;
        let mut r_count = 0_usize;

        for t in &self.closed_trades {
            if t.net_pnl > 1e-9 {
                win_count += 1;
                gross_wins += t.net_pnl;
            } else if t.net_pnl < -1e-9 {
                loss_count += 1;
                gross_losses += t.net_pnl.abs();
            }
            if let Some(r) = t.r_multiple
                && r.is_finite()
            {
                r_sum += r;
                r_count += 1;
            }
        }

        let closed_trade_count = self.closed_trades.len();
        let win_rate_pct = if closed_trade_count > 0 {
            ((win_count as f64) / (closed_trade_count as f64)) * 100.0
        } else {
            0.0
        };
        let profit_factor = if gross_losses > 1e-9 {
            Some(gross_wins / gross_losses)
        } else {
            None
        };
        let avg_r_multiple = if r_count > 0 {
            Some(r_sum / (r_count as f64))
        } else {
            None
        };
        let pending_order_count = self
            .orders
            .iter()
            .filter(|o| o.status == OrderStatus::Pending)
            .count();

        let live_dd = if self.peak_equity > 0.0 {
            (((self.peak_equity - equity) / self.peak_equity) * 100.0).max(0.0)
        } else {
            0.0
        };

        PortfolioMetrics {
            cash_balance: self.cash_balance,
            unrealized_pnl,
            equity,
            net_pnl,
            return_pct,
            max_drawdown_pct: self.max_drawdown_pct.max(live_dd),
            pending_order_count,
            fill_count: self.fills.len(),
            closed_trade_count,
            win_count,
            loss_count,
            win_rate_pct,
            profit_factor,
            avg_r_multiple,
        }
    }

    /// Submits a validated `Market`, `Limit`, or `Stop` order at `cursor_base_index`.
    ///
    /// - `OrderKind::Market` executes immediately at `base_candles[cursor_base_index].close` (with
    ///   slippage & commission).
    /// - `OrderKind::Limit` and `OrderKind::Stop` enter the `Pending` order book and evaluate
    ///   causally on subsequent bars `b > cursor_base_index`.
    pub fn submit_order(
        &mut self,
        req: OrderRequest,
        base_candles: &[Candle],
        cursor_base_index: usize,
    ) -> Result<u64, TradingError> {
        if base_candles.is_empty() {
            return Err(TradingError::EmptyMarketData);
        }
        if self.commands.len() >= MAX_RECORDED_COMMANDS {
            return Err(TradingError::CommandLogFull(MAX_RECORDED_COMMANDS));
        }
        let idx = cursor_base_index.min(base_candles.len() - 1);
        self.sync_cursor_before_command(base_candles, idx);
        let order_id = self.apply_submit_order_internal(req, idx, &base_candles[idx])?;
        self.commands.push(RecordedCommand {
            base_index: idx,
            kind: RecordedCommandKind::SubmitOrder(req),
        });
        self.record_equity_point(idx, base_candles[idx].timestamp, base_candles[idx].close);
        Ok(order_id)
    }

    /// Cancels a `Pending` working order at `cursor_base_index`.
    pub fn cancel_order(
        &mut self,
        order_id: u64,
        base_candles: &[Candle],
        cursor_base_index: usize,
    ) -> Result<(), TradingError> {
        if base_candles.is_empty() {
            return Err(TradingError::EmptyMarketData);
        }
        if self.commands.len() >= MAX_RECORDED_COMMANDS {
            return Err(TradingError::CommandLogFull(MAX_RECORDED_COMMANDS));
        }
        let idx = cursor_base_index.min(base_candles.len() - 1);
        self.sync_cursor_before_command(base_candles, idx);
        self.apply_cancel_order_internal(order_id)?;
        self.commands.push(RecordedCommand {
            base_index: idx,
            kind: RecordedCommandKind::CancelOrder { order_id },
        });
        self.record_equity_point(idx, base_candles[idx].timestamp, base_candles[idx].close);
        Ok(())
    }

    /// Updates protective `stop_loss` and/or `take_profit` levels on the currently open position.
    ///
    /// Validated against the current mark price (`base_candles[cursor_base_index].close`) so both
    /// protective stops and profit-locking breakeven/trailing stops are supported cleanly.
    pub fn update_position_brackets(
        &mut self,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
        base_candles: &[Candle],
        cursor_base_index: usize,
    ) -> Result<(), TradingError> {
        if base_candles.is_empty() {
            return Err(TradingError::EmptyMarketData);
        }
        if self.commands.len() >= MAX_RECORDED_COMMANDS {
            return Err(TradingError::CommandLogFull(MAX_RECORDED_COMMANDS));
        }
        let idx = cursor_base_index.min(base_candles.len() - 1);
        self.sync_cursor_before_command(base_candles, idx);
        self.apply_update_brackets_internal(stop_loss, take_profit, base_candles[idx].close)?;
        self.commands.push(RecordedCommand {
            base_index: idx,
            kind: RecordedCommandKind::UpdateBrackets {
                stop_loss,
                take_profit,
            },
        });
        self.record_equity_point(idx, base_candles[idx].timestamp, base_candles[idx].close);
        Ok(())
    }

    /// Immediately closes the entire open position at `base_candles[cursor_base_index].close`
    /// (with slippage & commission).
    pub fn flatten_position(
        &mut self,
        base_candles: &[Candle],
        cursor_base_index: usize,
    ) -> Result<ExecutionFill, TradingError> {
        if base_candles.is_empty() {
            return Err(TradingError::EmptyMarketData);
        }
        if self.commands.len() >= MAX_RECORDED_COMMANDS {
            return Err(TradingError::CommandLogFull(MAX_RECORDED_COMMANDS));
        }
        let idx = cursor_base_index.min(base_candles.len() - 1);
        self.sync_cursor_before_command(base_candles, idx);
        let fill = self.apply_flatten_internal(idx, &base_candles[idx])?;
        self.commands.push(RecordedCommand {
            base_index: idx,
            kind: RecordedCommandKind::FlattenPosition,
        });
        self.record_equity_point(idx, base_candles[idx].timestamp, base_candles[idx].close);
        Ok(fill)
    }

    /// Advances the paper-trading simulation causally across `base_candles[from_base_index..=to_base_index]`.
    pub fn advance_through_bars(
        &mut self,
        base_candles: &[Candle],
        from_base_index: usize,
        to_base_index: usize,
    ) {
        if base_candles.is_empty() || from_base_index > to_base_index {
            return;
        }
        let last = base_candles.len() - 1;
        let end = to_base_index.min(last);
        let start = from_base_index.min(end);
        for (b, bar) in base_candles.iter().enumerate().take(end + 1).skip(start) {
            if self.last_processed_base_index.is_some_and(|prev| b <= prev) {
                continue;
            }
            self.process_bar_arrival(b, bar);
            self.last_processed_base_index = Some(b);
        }
    }

    /// Truncates future commands (`cmd.base_index > target_base_index`) and deterministically
    /// replays the causal command log from `start_base_index..=target_base_index`.
    pub fn rewind_to(
        &mut self,
        base_candles: &[Candle],
        start_base_index: usize,
        target_base_index: usize,
    ) {
        self.config.sanitize();
        if base_candles.is_empty() {
            self.commands.clear();
            self.clear_runtime_ledger();
            return;
        }
        let last = base_candles.len() - 1;
        let target = target_base_index.min(last);
        self.commands.retain(|cmd| cmd.base_index <= target);
        self.commands.sort_by_key(|cmd| cmd.base_index);
        let earliest_cmd = self.commands.first().map(|c| c.base_index);
        let effective_start = start_base_index
            .min(target)
            .min(earliest_cmd.unwrap_or(target));
        self.session_start_base_index = effective_start;

        let retained_commands = self.commands.clone();
        self.clear_runtime_ledger();

        let mut cmd_cursor = 0_usize;
        for (b, bar) in base_candles
            .iter()
            .enumerate()
            .take(target + 1)
            .skip(effective_start)
        {
            if b > effective_start {
                self.process_bar_arrival(b, bar);
            } else {
                self.record_equity_point(b, bar.timestamp, bar.close);
            }
            self.last_processed_base_index = Some(b);

            let mut executed_cmd_at_b = false;
            while cmd_cursor < retained_commands.len()
                && retained_commands[cmd_cursor].base_index == b
            {
                executed_cmd_at_b = true;
                match &retained_commands[cmd_cursor].kind {
                    RecordedCommandKind::SubmitOrder(req) => {
                        let _ = self.apply_submit_order_internal(*req, b, bar);
                    }
                    RecordedCommandKind::CancelOrder { order_id } => {
                        let _ = self.apply_cancel_order_internal(*order_id);
                    }
                    RecordedCommandKind::UpdateBrackets {
                        stop_loss,
                        take_profit,
                    } => {
                        let _ = self.apply_update_brackets_internal(
                            *stop_loss,
                            *take_profit,
                            bar.close,
                        );
                    }
                    RecordedCommandKind::FlattenPosition => {
                        let _ = self.apply_flatten_internal(b, bar);
                    }
                }
                cmd_cursor += 1;
            }
            if executed_cmd_at_b {
                self.record_equity_point(b, bar.timestamp, bar.close);
            }
        }
    }

    fn clear_runtime_ledger(&mut self) {
        let initial_cash = self.config.initial_cash;
        self.cash_balance = initial_cash;
        self.open_position = None;
        self.orders.clear();
        self.fills.clear();
        self.closed_trades.clear();
        self.equity_curve.clear();
        self.peak_equity = initial_cash;
        self.max_drawdown_pct = 0.0;
        self.next_order_id = 1;
        self.next_fill_id = 1;
        self.next_trade_id = 1;
        self.last_processed_base_index = None;
    }

    fn sync_cursor_before_command(&mut self, base_candles: &[Candle], idx: usize) {
        match self.last_processed_base_index {
            None => {
                self.session_start_base_index = idx;
                self.last_processed_base_index = Some(idx);
                self.record_equity_point(idx, base_candles[idx].timestamp, base_candles[idx].close);
            }
            Some(prev) if idx > prev => {
                self.advance_through_bars(base_candles, prev + 1, idx);
            }
            Some(prev) if idx < prev => {
                let start = self.session_start_base_index.min(idx);
                self.rewind_to(base_candles, start, idx);
            }
            Some(_) => {}
        }
    }

    fn alloc_order_id(&mut self) -> u64 {
        let id = self.next_order_id;
        self.next_order_id = self.next_order_id.saturating_add(1);
        id
    }

    fn alloc_fill_id(&mut self) -> u64 {
        let id = self.next_fill_id;
        self.next_fill_id = self.next_fill_id.saturating_add(1);
        id
    }

    fn alloc_trade_id(&mut self) -> u64 {
        let id = self.next_trade_id;
        self.next_trade_id = self.next_trade_id.saturating_add(1);
        id
    }

    fn apply_submit_order_internal(
        &mut self,
        req: OrderRequest,
        base_index: usize,
        candle: &Candle,
    ) -> Result<u64, TradingError> {
        if !req.quantity.is_finite()
            || !(MIN_ORDER_QUANTITY..=MAX_ORDER_QUANTITY).contains(&req.quantity)
        {
            return Err(TradingError::InvalidQuantity(req.quantity));
        }

        let reference_price = match req.kind {
            OrderKind::Market => apply_slippage(candle.close, req.side, self.config.slippage_bps),
            OrderKind::Limit { limit_price } => {
                if !limit_price.is_finite() || limit_price <= 0.0 || limit_price > MAX_ORDER_PRICE {
                    return Err(TradingError::InvalidPrice(limit_price));
                }
                limit_price
            }
            OrderKind::Stop { stop_price } => {
                if !stop_price.is_finite() || stop_price <= 0.0 || stop_price > MAX_ORDER_PRICE {
                    return Err(TradingError::InvalidPrice(stop_price));
                }
                stop_price
            }
        };

        let notional = reference_price * req.quantity;
        if !notional.is_finite() || notional > MAX_ORDER_NOTIONAL {
            return Err(TradingError::NotionalOverflow);
        }

        // Determine whether this order purely reduces/closes an opposite open position.
        let is_pure_reduce = self.open_position.as_ref().is_some_and(|pos| {
            req.side == pos.side.exit_order_side() && req.quantity <= pos.quantity + 1e-12
        });

        let (effective_sl, effective_tp) = if !is_pure_reduce {
            let target_pos_side = req.side.to_position_side();
            if target_pos_side == PositionSide::Short && !self.config.allow_short {
                return Err(TradingError::ShortSellingDisabled);
            }
            validate_brackets_for_side(
                target_pos_side,
                reference_price,
                req.stop_loss,
                req.take_profit,
            )?;
            (req.stop_loss, req.take_profit)
        } else {
            // Pure-reduce orders may carry stale opposite-side Ticket UI SL/TP inputs (`[CRITICAL-02]`).
            // Strip any bracket that is invalid for `req.side.to_position_side()` so if the working
            // order later opens or flips into a new position after the original position exits, it
            // can never attach inverted brackets.
            let target_pos_side = req.side.to_position_side();
            if (target_pos_side != PositionSide::Short || self.config.allow_short)
                && validate_brackets_for_side(
                    target_pos_side,
                    reference_price,
                    req.stop_loss,
                    req.take_profit,
                )
                .is_ok()
            {
                (req.stop_loss, req.take_profit)
            } else {
                (None, None)
            }
        };

        match req.kind {
            OrderKind::Market => {
                // Validate buying power / margin before allocating or mutating state.
                self.validate_execution_capacity(req.side, req.quantity, reference_price)?;
                let order_id = self.alloc_order_id();
                let order = Order {
                    id: order_id,
                    side: req.side,
                    kind: req.kind,
                    quantity: req.quantity,
                    stop_loss: effective_sl,
                    take_profit: effective_tp,
                    submitted_base_index: base_index,
                    submitted_timestamp: candle.timestamp,
                    status: OrderStatus::Filled,
                    filled_price: Some(reference_price),
                    filled_timestamp: Some(candle.timestamp),
                };
                self.orders.push(order);
                self.execute_fill_mutation(
                    order_id,
                    req.side,
                    req.quantity,
                    reference_price,
                    reference_price,
                    FillReason::MarketImmediate,
                    effective_sl,
                    effective_tp,
                    base_index,
                    candle.timestamp,
                );
                Ok(order_id)
            }
            OrderKind::Limit { .. } | OrderKind::Stop { .. } => {
                let pending_count = self
                    .orders
                    .iter()
                    .filter(|o| o.status == OrderStatus::Pending)
                    .count();
                if pending_count >= MAX_PENDING_ORDERS {
                    return Err(TradingError::TooManyPendingOrders(MAX_PENDING_ORDERS));
                }
                // Pre-check estimated buying power at submission so blatantly unaffordable orders
                // fail fast with a clear message instead of silently rejecting later.
                let est_exec = apply_slippage(reference_price, req.side, self.config.slippage_bps);
                self.validate_execution_capacity(req.side, req.quantity, est_exec)?;

                let order_id = self.alloc_order_id();
                self.orders.push(Order {
                    id: order_id,
                    side: req.side,
                    kind: req.kind,
                    quantity: req.quantity,
                    stop_loss: effective_sl,
                    take_profit: effective_tp,
                    submitted_base_index: base_index,
                    submitted_timestamp: candle.timestamp,
                    status: OrderStatus::Pending,
                    filled_price: None,
                    filled_timestamp: None,
                });
                Ok(order_id)
            }
        }
    }

    fn apply_cancel_order_internal(&mut self, order_id: u64) -> Result<(), TradingError> {
        let Some(order) = self.orders.iter_mut().find(|o| o.id == order_id) else {
            return Err(TradingError::OrderNotFound(order_id));
        };
        if order.status != OrderStatus::Pending {
            return Err(TradingError::OrderNotPending(order_id));
        }
        order.status = OrderStatus::Cancelled;
        Ok(())
    }

    fn apply_update_brackets_internal(
        &mut self,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
        mark_price: f64,
    ) -> Result<(), TradingError> {
        let Some(pos) = self.open_position.as_ref() else {
            return Err(TradingError::NoOpenPosition);
        };
        validate_brackets_for_side(pos.side, mark_price, stop_loss, take_profit)?;
        if let Some(pos_mut) = self.open_position.as_mut() {
            pos_mut.stop_loss = stop_loss;
            pos_mut.take_profit = take_profit;
            if pos_mut.initial_stop_loss.is_none()
                && let Some(sl) = stop_loss
            {
                let risk_per_unit = (pos_mut.avg_entry_price - sl).abs();
                if risk_per_unit > 1e-9 {
                    pos_mut.initial_stop_loss = Some(sl);
                    pos_mut.initial_risk_amount = Some(risk_per_unit * pos_mut.quantity);
                }
            }
        }
        Ok(())
    }

    fn apply_flatten_internal(
        &mut self,
        base_index: usize,
        candle: &Candle,
    ) -> Result<ExecutionFill, TradingError> {
        let Some(pos) = self.open_position.as_ref() else {
            return Err(TradingError::NoOpenPosition);
        };
        let exit_side = pos.side.exit_order_side();
        let qty = pos.quantity;
        let exec_price = apply_slippage(candle.close, exit_side, self.config.slippage_bps);
        let order_id = self.alloc_order_id();
        self.orders.push(Order {
            id: order_id,
            side: exit_side,
            kind: OrderKind::Market,
            quantity: qty,
            stop_loss: None,
            take_profit: None,
            submitted_base_index: base_index,
            submitted_timestamp: candle.timestamp,
            status: OrderStatus::Filled,
            filled_price: Some(exec_price),
            filled_timestamp: Some(candle.timestamp),
        });
        let fill = self.execute_fill_mutation(
            order_id,
            exit_side,
            qty,
            exec_price,
            exec_price,
            FillReason::ManualFlatten,
            None,
            None,
            base_index,
            candle.timestamp,
        );
        Ok(fill)
    }

    fn validate_execution_capacity(
        &self,
        side: OrderSide,
        quantity: f64,
        exec_price: f64,
    ) -> Result<(), TradingError> {
        let fee = compute_commission(exec_price, quantity, self.config.commission_bps);
        match &self.open_position {
            None => {
                if side == OrderSide::Sell && !self.config.allow_short {
                    return Err(TradingError::ShortSellingDisabled);
                }
                let required_margin = (quantity * exec_price) / self.config.max_leverage;
                let post_fee_cash = self.cash_balance - fee;
                if post_fee_cash < required_margin || post_fee_cash <= 0.0 {
                    return Err(TradingError::InsufficientBuyingPower {
                        required: required_margin + fee,
                        available: self.cash_balance.max(0.0),
                    });
                }
                Ok(())
            }
            Some(pos) if side == pos.side.entry_order_side() => {
                let new_qty = pos.quantity + quantity;
                let new_avg =
                    (pos.quantity * pos.avg_entry_price + quantity * exec_price) / new_qty;
                let required_margin = (new_qty * new_avg) / self.config.max_leverage;
                let post_fee_equity = (self.cash_balance - fee) + pos.unrealized_pnl(exec_price);
                if post_fee_equity < required_margin || post_fee_equity <= 0.0 {
                    return Err(TradingError::InsufficientBuyingPower {
                        required: required_margin + fee,
                        available: (self.cash_balance + pos.unrealized_pnl(exec_price)).max(0.0),
                    });
                }
                Ok(())
            }
            Some(pos) => {
                // Closing or reducing an existing position is always permitted!
                if quantity <= pos.quantity + 1e-12 {
                    return Ok(());
                }
                // Position flip: check opposite-side permission and post-close margin sufficiency.
                let new_side = side.to_position_side();
                if new_side == PositionSide::Short && !self.config.allow_short {
                    return Err(TradingError::ShortSellingDisabled);
                }
                let flip_qty = quantity - pos.quantity;
                let close_fee =
                    compute_commission(exec_price, pos.quantity, self.config.commission_bps);
                let close_gross = pos.unrealized_pnl(exec_price);
                let cash_after_close = self.cash_balance + close_gross - close_fee;
                let open_fee = compute_commission(exec_price, flip_qty, self.config.commission_bps);
                let required_margin = (flip_qty * exec_price) / self.config.max_leverage;
                let cash_after_both = cash_after_close - open_fee;
                if cash_after_both < required_margin || cash_after_both <= 0.0 {
                    return Err(TradingError::InsufficientBuyingPower {
                        required: required_margin + open_fee,
                        available: cash_after_close.max(0.0),
                    });
                }
                Ok(())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_fill_mutation(
        &mut self,
        order_id: u64,
        side: OrderSide,
        quantity: f64,
        exec_price: f64,
        planned_ref_price: f64,
        reason: FillReason,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
        base_index: usize,
        timestamp: i64,
    ) -> ExecutionFill {
        // Handle opposite-side close / flip first if an open position exists on the opposite side.
        let same_side_existing = self
            .open_position
            .as_ref()
            .is_some_and(|p| side == p.side.entry_order_side());

        if self.open_position.is_none() || same_side_existing {
            let fee = compute_commission(exec_price, quantity, self.config.commission_bps);
            self.cash_balance -= fee;

            if let Some(pos) = self.open_position.as_mut() {
                let prev_notional = pos.avg_entry_price * pos.quantity;
                let add_notional = exec_price * quantity;
                pos.quantity += quantity;
                pos.avg_entry_price = (prev_notional + add_notional) / pos.quantity;
                pos.total_entry_qty += quantity;
                pos.total_entry_notional += add_notional;
                pos.total_entry_fees += fee;
                pos.open_entry_fees += fee;

                let (clean_sl, clean_tp) =
                    sanitize_brackets_for_position(pos.side, exec_price, stop_loss, take_profit);
                if clean_sl.is_some() {
                    pos.stop_loss = clean_sl;
                    pos.initial_stop_loss = clean_sl;
                }
                if clean_tp.is_some() {
                    pos.take_profit = clean_tp;
                }
                if let (Some(s), Some(t)) = (pos.stop_loss, pos.take_profit) {
                    let ordered = match pos.side {
                        PositionSide::Long => s < t - 1e-9,
                        PositionSide::Short => s > t + 1e-9,
                    };
                    if !ordered {
                        if clean_sl.is_some() {
                            pos.take_profit = None;
                        } else {
                            pos.stop_loss = None;
                        }
                    }
                }
                // Recompute `initial_risk_amount` across total scaled-in quantity (`[LOW-01]`).
                let risk_sl = pos
                    .stop_loss
                    .filter(|&sl| match pos.side {
                        PositionSide::Long => sl < pos.avg_entry_price - 1e-9,
                        PositionSide::Short => sl > pos.avg_entry_price + 1e-9,
                    })
                    .or_else(|| {
                        pos.initial_stop_loss.filter(|&sl| match pos.side {
                            PositionSide::Long => sl < pos.avg_entry_price - 1e-9,
                            PositionSide::Short => sl > pos.avg_entry_price + 1e-9,
                        })
                    });
                if let Some(r_sl) = risk_sl {
                    if pos.initial_stop_loss.is_none() {
                        pos.initial_stop_loss = Some(r_sl);
                    }
                    pos.initial_risk_amount =
                        Some((pos.avg_entry_price - r_sl).abs() * pos.quantity);
                }
            } else {
                let pos_side = side.to_position_side();
                let (clean_sl, clean_tp) =
                    sanitize_brackets_for_position(pos_side, exec_price, stop_loss, take_profit);
                let initial_risk_amount = clean_sl.and_then(|sl| {
                    let direct_dist = match pos_side {
                        PositionSide::Long => exec_price - sl,
                        PositionSide::Short => sl - exec_price,
                    };
                    let fallback_dist = (planned_ref_price - sl).abs();
                    let unit_risk = if direct_dist > 1e-9 {
                        direct_dist
                    } else {
                        fallback_dist
                    };
                    (unit_risk > 1e-9).then_some(unit_risk * quantity)
                });
                self.open_position = Some(Position {
                    side: pos_side,
                    quantity,
                    avg_entry_price: exec_price,
                    opened_timestamp: timestamp,
                    opened_base_index: base_index,
                    stop_loss: clean_sl,
                    take_profit: clean_tp,
                    initial_stop_loss: clean_sl,
                    initial_risk_amount,
                    total_entry_qty: quantity,
                    total_entry_notional: exec_price * quantity,
                    total_entry_fees: fee,
                    open_entry_fees: fee,
                    closed_qty_acc: 0.0,
                    exit_notional_acc: 0.0,
                    realized_gross_pnl_acc: 0.0,
                    realized_exit_fees_acc: 0.0,
                });
            }

            let fill = ExecutionFill {
                fill_id: self.alloc_fill_id(),
                order_id,
                side,
                reason,
                quantity,
                price: exec_price,
                fee,
                realized_net_pnl: None,
                base_index,
                timestamp,
            };
            self.fills.push(fill.clone());
            return fill;
        }

        // Opposite-side reduce, full close, or atomic two-leg flip!
        let open_qty = self.open_position.as_ref().map_or(0.0, |p| p.quantity);
        let close_qty = if (quantity - open_qty).abs() <= 1e-12 {
            open_qty
        } else {
            quantity.min(open_qty)
        };
        let flip_qty = (quantity - close_qty).max(0.0);

        let close_fill = self.close_position_slice(
            order_id, side, close_qty, exec_price, reason, base_index, timestamp,
        );

        if flip_qty > MIN_ORDER_QUANTITY {
            self.execute_fill_mutation(
                order_id,
                side,
                flip_qty,
                exec_price,
                planned_ref_price,
                reason,
                stop_loss,
                take_profit,
                base_index,
                timestamp,
            )
        } else {
            close_fill
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn close_position_slice(
        &mut self,
        order_id: u64,
        exit_side: OrderSide,
        close_qty: f64,
        exec_price: f64,
        reason: FillReason,
        base_index: usize,
        timestamp: i64,
    ) -> ExecutionFill {
        let mut pos = self
            .open_position
            .take()
            .expect("close_position_slice requires an open position");
        let actual_close = close_qty.min(pos.quantity);
        let remaining_qty = (pos.quantity - actual_close).max(0.0);
        let is_full_close = remaining_qty <= 1e-12;

        let exit_fee = compute_commission(exec_price, actual_close, self.config.commission_bps);
        let gross_pnl_chunk = pos.side.sign() * (exec_price - pos.avg_entry_price) * actual_close;
        let alloc_entry_fee = if is_full_close || pos.quantity <= 0.0 {
            let rem = pos.open_entry_fees;
            pos.open_entry_fees = 0.0;
            rem
        } else {
            let share = (actual_close / pos.quantity).clamp(0.0, 1.0);
            let alloc = pos.open_entry_fees * share;
            pos.open_entry_fees = (pos.open_entry_fees - alloc).max(0.0);
            alloc
        };

        self.cash_balance += gross_pnl_chunk - exit_fee;
        pos.closed_qty_acc += actual_close;
        pos.exit_notional_acc += actual_close * exec_price;
        pos.realized_gross_pnl_acc += gross_pnl_chunk;
        pos.realized_exit_fees_acc += exit_fee;
        pos.quantity = if is_full_close { 0.0 } else { remaining_qty };

        let chunk_net_pnl = gross_pnl_chunk - alloc_entry_fee - exit_fee;

        if is_full_close {
            let total_fees = pos.total_entry_fees + pos.realized_exit_fees_acc;
            let net_pnl = pos.realized_gross_pnl_acc - total_fees;
            let entry_vwap = if pos.total_entry_qty > 0.0 {
                pos.total_entry_notional / pos.total_entry_qty
            } else {
                pos.avg_entry_price
            };
            let exit_vwap = if pos.closed_qty_acc > 0.0 {
                pos.exit_notional_acc / pos.closed_qty_acc
            } else {
                exec_price
            };
            let return_pct = if pos.total_entry_notional > 0.0 {
                (net_pnl / pos.total_entry_notional) * 100.0
            } else {
                0.0
            };
            let r_multiple = pos
                .initial_risk_amount
                .filter(|&r| r > 1e-9)
                .map(|r| net_pnl / r);
            let trade_id = self.alloc_trade_id();
            self.closed_trades.push(ClosedTrade {
                trade_id,
                side: pos.side,
                quantity: pos.closed_qty_acc,
                entry_price: entry_vwap,
                exit_price: exit_vwap,
                entry_timestamp: pos.opened_timestamp,
                exit_timestamp: timestamp,
                gross_pnl: pos.realized_gross_pnl_acc,
                total_fees,
                net_pnl,
                return_pct,
                r_multiple,
            });
        } else {
            self.open_position = Some(pos);
        }

        let fill = ExecutionFill {
            fill_id: self.alloc_fill_id(),
            order_id,
            side: exit_side,
            reason,
            quantity: actual_close,
            price: exec_price,
            fee: exit_fee,
            realized_net_pnl: Some(chunk_net_pnl),
            base_index,
            timestamp,
        };
        self.fills.push(fill.clone());
        fill
    }

    fn process_bar_arrival(&mut self, base_index: usize, candle: &Candle) {
        // Determine causal 4-waypoint bar path (`[O, L, H, C]` vs `[O, H, L, C]`), preserving
        // pessimistic SL-before-TP resolution (`DEF-005`) whenever a pre-existing position touches
        // both `SL` and `TP` inside `[candle.low, candle.high]`.
        let pre_dual_touch_side = self.open_position.as_ref().and_then(|pos| {
            let (Some(sl), Some(tp)) = (pos.stop_loss, pos.take_profit) else {
                return None;
            };
            match pos.side {
                PositionSide::Long if candle.low <= sl && candle.high >= tp => {
                    Some(PositionSide::Long)
                }
                PositionSide::Short if candle.high >= sl && candle.low <= tp => {
                    Some(PositionSide::Short)
                }
                _ => None,
            }
        });
        let low_first = match pre_dual_touch_side {
            Some(PositionSide::Long) => true,
            Some(PositionSide::Short) => false,
            None => candle.close >= candle.open,
        };
        let waypoints = if low_first {
            [candle.open, candle.low, candle.high, candle.close]
        } else {
            [candle.open, candle.high, candle.low, candle.close]
        };

        // Phase A: Exact Gap-Open evaluation at `price = candle.open` (`t = Open`).
        if let Some((reason, raw_exit_price)) = self.check_position_bracket_at_price(candle.open) {
            self.execute_bracket_exit_at(base_index, candle.timestamp, reason, raw_exit_price);
        }
        let open_pending_indices: Vec<usize> = self
            .orders
            .iter()
            .enumerate()
            .filter_map(|(i, o)| {
                (o.status == OrderStatus::Pending && o.submitted_base_index < base_index)
                    .then_some(i)
            })
            .collect();
        for ord_idx in open_pending_indices {
            let (side, kind) = {
                let o = &self.orders[ord_idx];
                if o.status != OrderStatus::Pending {
                    continue;
                }
                (o.side, o.kind)
            };
            if let Some(reason) = evaluate_pending_trigger_at_open(side, kind, candle.open) {
                self.try_fill_pending_order_at(
                    ord_idx,
                    candle.open,
                    reason,
                    base_index,
                    candle.timestamp,
                );
                if let Some((b_reason, raw_exit)) =
                    self.check_position_bracket_at_price(candle.open)
                {
                    self.execute_bracket_exit_at(base_index, candle.timestamp, b_reason, raw_exit);
                }
            }
        }

        // Phase B: Causal segment-by-segment simulation across `(w_0 -> w_1)`, `(w_1 -> w_2)`, `(w_2 -> w_3)` (`[MEDIUM-01]`).
        for seg_idx in 0..3 {
            let mut cur_px = waypoints[seg_idx];
            let seg_end = waypoints[seg_idx + 1];
            if (seg_end - cur_px).abs() <= 1e-15 {
                continue;
            }
            for _ in 0..1024 {
                let Some(ev) = self.find_earliest_segment_event(base_index, cur_px, seg_end) else {
                    break;
                };
                match ev {
                    SegmentCrossingEvent::BracketExit {
                        reason,
                        trigger_price,
                    } => {
                        self.execute_bracket_exit_at(
                            base_index,
                            candle.timestamp,
                            reason,
                            trigger_price,
                        );
                        cur_px = trigger_price;
                    }
                    SegmentCrossingEvent::PendingOrder {
                        ord_idx,
                        trigger_price,
                        reason,
                    } => {
                        self.try_fill_pending_order_at(
                            ord_idx,
                            trigger_price,
                            reason,
                            base_index,
                            candle.timestamp,
                        );
                        cur_px = trigger_price;
                    }
                }
            }
        }

        // Phase C: End-of-bar mark-to-market equity & drawdown update on `candle.close`.
        self.record_equity_point(base_index, candle.timestamp, candle.close);
    }

    fn check_position_bracket_at_price(&self, price: f64) -> Option<(FillReason, f64)> {
        let pos = self.open_position.as_ref()?;
        match pos.side {
            PositionSide::Long => {
                if let Some(sl) = pos.stop_loss
                    && price <= sl
                {
                    return Some((FillReason::StopLossExit, price));
                }
                if let Some(tp) = pos.take_profit
                    && price >= tp
                {
                    return Some((FillReason::TakeProfitExit, price));
                }
                None
            }
            PositionSide::Short => {
                if let Some(sl) = pos.stop_loss
                    && price >= sl
                {
                    return Some((FillReason::StopLossExit, price));
                }
                if let Some(tp) = pos.take_profit
                    && price <= tp
                {
                    return Some((FillReason::TakeProfitExit, price));
                }
                None
            }
        }
    }

    fn execute_bracket_exit_at(
        &mut self,
        base_index: usize,
        timestamp: i64,
        reason: FillReason,
        raw_exit_price: f64,
    ) {
        let Some(pos) = self.open_position.as_ref() else {
            return;
        };
        let exit_side = pos.side.exit_order_side();
        let qty = pos.quantity;
        let exec_price = apply_slippage(raw_exit_price, exit_side, self.config.slippage_bps);
        let order_id = self.alloc_order_id();
        self.orders.push(Order {
            id: order_id,
            side: exit_side,
            kind: OrderKind::Market,
            quantity: qty,
            stop_loss: None,
            take_profit: None,
            submitted_base_index: base_index,
            submitted_timestamp: timestamp,
            status: OrderStatus::Filled,
            filled_price: Some(exec_price),
            filled_timestamp: Some(timestamp),
        });
        let _ = self.close_position_slice(
            order_id, exit_side, qty, exec_price, reason, base_index, timestamp,
        );
    }

    fn try_fill_pending_order_at(
        &mut self,
        ord_idx: usize,
        raw_trigger_price: f64,
        reason: FillReason,
        base_index: usize,
        timestamp: i64,
    ) {
        let (order_id, side, kind, qty, sl, tp) = {
            let o = &self.orders[ord_idx];
            if o.status != OrderStatus::Pending {
                return;
            }
            (o.id, o.side, o.kind, o.quantity, o.stop_loss, o.take_profit)
        };
        let exec_price = apply_slippage(raw_trigger_price, side, self.config.slippage_bps);
        match self.validate_execution_capacity(side, qty, exec_price) {
            Err(err) => {
                self.orders[ord_idx].status = OrderStatus::Rejected {
                    reason: err.to_string(),
                };
            }
            Ok(()) => {
                let planned_ref = kind.trigger_price().unwrap_or(raw_trigger_price);
                self.orders[ord_idx].status = OrderStatus::Filled;
                self.orders[ord_idx].filled_price = Some(exec_price);
                self.orders[ord_idx].filled_timestamp = Some(timestamp);
                let _ = self.execute_fill_mutation(
                    order_id,
                    side,
                    qty,
                    exec_price,
                    planned_ref,
                    reason,
                    sl,
                    tp,
                    base_index,
                    timestamp,
                );
            }
        }
    }

    fn find_earliest_segment_event(
        &self,
        base_index: usize,
        cur_px: f64,
        seg_end: f64,
    ) -> Option<SegmentCrossingEvent> {
        let mut best: Option<(f64, u8, usize, SegmentCrossingEvent)> = None;
        let mut consider = |dist: f64, rank: u8, tie_idx: usize, ev: SegmentCrossingEvent| {
            let replace = match best {
                None => true,
                Some((b_dist, b_rank, b_idx, _)) => {
                    if (dist - b_dist).abs() > 1e-12 {
                        dist < b_dist
                    } else if rank != b_rank {
                        rank < b_rank
                    } else {
                        tie_idx < b_idx
                    }
                }
            };
            if replace {
                best = Some((dist, rank, tie_idx, ev));
            }
        };

        let seg_min = cur_px.min(seg_end);
        let seg_max = cur_px.max(seg_end);

        if let Some(pos) = self.open_position.as_ref() {
            match pos.side {
                PositionSide::Long => {
                    if let Some(sl) = pos.stop_loss
                        && seg_min <= sl
                    {
                        let px = sl.min(cur_px);
                        consider(
                            (px - cur_px).abs(),
                            0,
                            0,
                            SegmentCrossingEvent::BracketExit {
                                reason: FillReason::StopLossExit,
                                trigger_price: px,
                            },
                        );
                    }
                    if let Some(tp) = pos.take_profit
                        && seg_max >= tp
                    {
                        let px = tp.max(cur_px);
                        consider(
                            (px - cur_px).abs(),
                            1,
                            0,
                            SegmentCrossingEvent::BracketExit {
                                reason: FillReason::TakeProfitExit,
                                trigger_price: px,
                            },
                        );
                    }
                }
                PositionSide::Short => {
                    if let Some(sl) = pos.stop_loss
                        && seg_max >= sl
                    {
                        let px = sl.max(cur_px);
                        consider(
                            (px - cur_px).abs(),
                            0,
                            0,
                            SegmentCrossingEvent::BracketExit {
                                reason: FillReason::StopLossExit,
                                trigger_price: px,
                            },
                        );
                    }
                    if let Some(tp) = pos.take_profit
                        && seg_min <= tp
                    {
                        let px = tp.min(cur_px);
                        consider(
                            (px - cur_px).abs(),
                            1,
                            0,
                            SegmentCrossingEvent::BracketExit {
                                reason: FillReason::TakeProfitExit,
                                trigger_price: px,
                            },
                        );
                    }
                }
            }
        }

        for (ord_idx, o) in self.orders.iter().enumerate() {
            if o.status != OrderStatus::Pending || o.submitted_base_index >= base_index {
                continue;
            }
            if let Some((px, reason)) =
                evaluate_pending_trigger_on_segment(o.side, o.kind, cur_px, seg_end)
            {
                consider(
                    (px - cur_px).abs(),
                    2,
                    ord_idx,
                    SegmentCrossingEvent::PendingOrder {
                        ord_idx,
                        trigger_price: px,
                        reason,
                    },
                );
            }
        }

        best.map(|(_, _, _, ev)| ev)
    }

    fn record_equity_point(&mut self, base_index: usize, timestamp: i64, mark_price: f64) {
        let eq = self.equity_at_price(mark_price);
        if eq > self.peak_equity {
            self.peak_equity = eq;
        }
        let dd_pct = if self.peak_equity > 0.0 {
            (((self.peak_equity - eq) / self.peak_equity) * 100.0).max(0.0)
        } else {
            0.0
        };
        if dd_pct > self.max_drawdown_pct {
            self.max_drawdown_pct = dd_pct;
        }
        if let Some(last) = self.equity_curve.last_mut()
            && last.base_index == base_index
        {
            last.timestamp = timestamp;
            last.equity = eq;
            last.drawdown_pct = dd_pct;
            return;
        }
        self.equity_curve.push(EquityPoint {
            base_index,
            timestamp,
            equity: eq,
            drawdown_pct: dd_pct,
        });
    }
}

fn sanitize_brackets_for_position(
    side: PositionSide,
    ref_price: f64,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    let clean_sl = stop_loss.filter(|&sl| {
        sl.is_finite()
            && sl > 0.0
            && sl <= MAX_ORDER_PRICE
            && match side {
                PositionSide::Long => sl < ref_price - 1e-9,
                PositionSide::Short => sl > ref_price + 1e-9,
            }
    });
    let clean_tp = take_profit.filter(|&tp| {
        tp.is_finite()
            && tp > 0.0
            && tp <= MAX_ORDER_PRICE
            && match side {
                PositionSide::Long => tp > ref_price + 1e-9,
                PositionSide::Short => tp < ref_price - 1e-9,
            }
    });
    if let (Some(s), Some(t)) = (clean_sl, clean_tp) {
        let valid_pair = match side {
            PositionSide::Long => s < t - 1e-9,
            PositionSide::Short => s > t + 1e-9,
        };
        if !valid_pair {
            return (None, None);
        }
    }
    (clean_sl, clean_tp)
}

fn evaluate_pending_trigger_at_open(
    side: OrderSide,
    kind: OrderKind,
    open_price: f64,
) -> Option<FillReason> {
    match (side, kind) {
        (_, OrderKind::Market) => Some(FillReason::MarketImmediate),
        (OrderSide::Buy, OrderKind::Limit { limit_price }) if open_price <= limit_price => {
            Some(FillReason::LimitTriggered)
        }
        (OrderSide::Sell, OrderKind::Limit { limit_price }) if open_price >= limit_price => {
            Some(FillReason::LimitTriggered)
        }
        (OrderSide::Buy, OrderKind::Stop { stop_price }) if open_price >= stop_price => {
            Some(FillReason::StopTriggered)
        }
        (OrderSide::Sell, OrderKind::Stop { stop_price }) if open_price <= stop_price => {
            Some(FillReason::StopTriggered)
        }
        _ => None,
    }
}

fn evaluate_pending_trigger_on_segment(
    side: OrderSide,
    kind: OrderKind,
    cur_px: f64,
    seg_end: f64,
) -> Option<(f64, FillReason)> {
    match (side, kind) {
        (_, OrderKind::Market) => Some((cur_px, FillReason::MarketImmediate)),
        (OrderSide::Buy, OrderKind::Limit { limit_price })
            if cur_px >= limit_price && seg_end <= limit_price =>
        {
            Some((limit_price, FillReason::LimitTriggered))
        }
        (OrderSide::Sell, OrderKind::Limit { limit_price })
            if cur_px <= limit_price && seg_end >= limit_price =>
        {
            Some((limit_price, FillReason::LimitTriggered))
        }
        (OrderSide::Buy, OrderKind::Stop { stop_price })
            if cur_px <= stop_price && seg_end >= stop_price =>
        {
            Some((stop_price, FillReason::StopTriggered))
        }
        (OrderSide::Sell, OrderKind::Stop { stop_price })
            if cur_px >= stop_price && seg_end <= stop_price =>
        {
            Some((stop_price, FillReason::StopTriggered))
        }
        _ => None,
    }
}
