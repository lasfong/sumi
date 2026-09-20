"""Pure in-memory next-event backtest execution kernel.

Guarantees:
- Causal execution: signals evaluated on Close[T] fill at Open[T+1] (BT-TIME-001).
- Timestamp separation: signal_timestamp != order_timestamp != fill_timestamp (BT-TIME-002).
- Zero database interactions during simulation (NFR-DET-001).
- Vietnam equity constraints: 100-share minimum lots, T+2 sell holding rule.
- Cutoff boundary: signals on final bar N-1 are marked UNFILLED_AT_CUTOFF (TEST-BT-001).
"""

import math
import uuid
from datetime import date, datetime
from typing import Any, Dict, List, Optional, Union

import numpy as np
import pandas as pd

from app.domain.backtest.models import (
    BacktestExecution,
    BacktestKernelResult,
    BacktestLedger,
    BacktestOrder,
    BacktestPosition,
    BacktestTrade,
    SignalEvent,
)
from app.domain.market import (
    HoldingLot,
    MarketProfile,
    MarketRuleProvider,
    MarketRules,
    SettlementEngine,
    TradingCalendar,
)
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator


class BacktestExecutionKernel:
    """Pure in-memory event-driven backtest kernel."""

    def __init__(
        self,
        fee_rate: float = 0.0015,
        tax_rate: float = 0.001,
        slippage_rate: float = 0.0,
        min_holding_bars: int = 2,
        profile: Optional[Union[str, MarketProfile]] = None,
        exchange: str = "HOSE",
    ):
        self.fee_rate = fee_rate
        self.tax_rate = tax_rate
        self.slippage_rate = slippage_rate
        self.min_holding_bars = min_holding_bars
        self.exchange = exchange.upper()
        if profile is not None:
            self.profile = (
                MarketRuleProvider.get_profile(profile, exchange=self.exchange)
                if isinstance(profile, str)
                else profile
            )
            self.has_explicit_profile = True
        else:
            self.profile = MarketRuleProvider.get_profile("vietnam_default_conservative", exchange=self.exchange)
            self.has_explicit_profile = False

    def run(
        self,
        df: pd.DataFrame,
        indicator_values: Dict[str, np.ndarray],
        strategy: Any,
        symbol: str,
        initial_cash: float = 100_000_000.0,
        timeframe: str = "1D",
        profile: Optional[Union[str, MarketProfile]] = None,
        exchange: Optional[str] = None,
    ) -> BacktestKernelResult:
        """Execute simulation over candle DataFrame in-memory."""
        if df.empty:
            raise ValueError("Cannot run backtest on empty DataFrame")

        active_exchange = (exchange or self.exchange).upper()
        if profile is not None:
            active_profile = (
                MarketRuleProvider.get_profile(profile, exchange=active_exchange)
                if isinstance(profile, str)
                else profile
            )
            has_explicit = True
        else:
            active_profile = self.profile
            has_explicit = getattr(self, "has_explicit_profile", False)

        total_candles = len(df)
        cash = float(initial_cash)
        peak_equity = cash
        max_drawdown = 0.0

        position = BacktestPosition(symbol=symbol)
        position.lots: List[HoldingLot] = []
        pending_order: Optional[BacktestOrder] = None
        active_trade: Optional[BacktestTrade] = None

        orders: List[BacktestOrder] = []
        executions: List[BacktestExecution] = []
        trades: List[BacktestTrade] = []
        equity_curve: List[Dict[str, Any]] = []
        unexecuted_signals: List[Dict[str, Any]] = []
        warnings: List[str] = []

        start_date = df.iloc[0]["timestamp"]
        end_date = df.iloc[-1]["timestamp"]

        for i in range(total_candles):
            row = df.iloc[i]
            open_price = float(row["open"])
            high_price = float(row["high"])
            low_price = float(row["low"])
            close_price = float(row["close"])
            bar_ts = row["timestamp"]
            bar_date = bar_ts.date() if isinstance(bar_ts, (datetime, pd.Timestamp)) else bar_ts

            ref_price = float(df.iloc[i - 1]["close"]) if i > 0 else open_price
            floor_price, ceiling_price = active_profile.rules.calculate_price_limits(ref_price, active_exchange)

            # -----------------------------------------------------------------
            # 1. Fill Pending Order from Bar i-1 on Bar i Open (BT-TIME-001)
            # -----------------------------------------------------------------
            if pending_order is not None and pending_order.status == "QUEUED":
                fill_price = open_price * (1.0 + self.slippage_rate) if pending_order.side == "BUY" else open_price * (1.0 - self.slippage_rate)

                if pending_order.side == "BUY":
                    # Locked ceiling check (TEST-BT-004)
                    is_ceiling_locked = active_profile.rules.is_locked_ceiling(high_price, low_price, ceiling_price)
                    if is_ceiling_locked and active_profile.conservative_locked_limits:
                        pending_order.status = "REJECTED"
                        pending_order.rejection_reason = "LOCKED_CEILING_REJECT: Locked ceiling bar; zero counter-liquidity to fill BUY order"
                        warnings.append(f"Bar {i} ({bar_ts}): Buy order rejected due to ceiling locked limit")
                        pending_order = None
                    else:
                        lot_size = active_profile.lot_size
                        cost_per_share = fill_price * (1.0 + self.fee_rate)
                        max_affordable = int((cash / cost_per_share) // lot_size) * lot_size

                        if max_affordable < lot_size:
                            pending_order.status = "REJECTED"
                            pending_order.rejection_reason = f"Insufficient cash for minimum {lot_size} shares"
                            warnings.append(f"Bar {i} ({bar_ts}): Buy order rejected due to insufficient cash")
                            pending_order = None
                        else:
                            fill_qty = min(pending_order.quantity, float(max_affordable))
                            fill_qty = float(active_profile.rules.round_board_lot(fill_qty, lot_size))
                            if fill_qty < lot_size:
                                pending_order.status = "REJECTED"
                                pending_order.rejection_reason = f"Insufficient cash for minimum {lot_size} shares"
                                warnings.append(f"Bar {i} ({bar_ts}): Buy order rejected due to insufficient cash")
                                pending_order = None
                            else:
                                gross = fill_qty * fill_price
                                fee = gross * self.fee_rate
                                net = gross + fee

                                cash -= net

                                execution = BacktestExecution(
                                    id=f"exec_{uuid.uuid4().hex[:8]}",
                                    order_id=pending_order.id,
                                    symbol=symbol,
                                    side="BUY",
                                    quantity=fill_qty,
                                    price=fill_price,
                                    gross_amount=gross,
                                    fee=fee,
                                    tax=0.0,
                                    net_amount=net,
                                    signal_timestamp=pending_order.signal_timestamp,
                                    signal_bar_index=pending_order.signal_bar_index,
                                    order_timestamp=pending_order.order_timestamp,
                                    order_bar_index=pending_order.order_bar_index,
                                    fill_timestamp=bar_ts,
                                    fill_bar_index=i,
                                )
                                executions.append(execution)
                                pending_order.status = "FILLED"

                                new_total_qty = position.quantity + fill_qty
                                position.cost_basis += gross
                                position.average_price = position.cost_basis / new_total_qty if new_total_qty > 0 else fill_price
                                position.quantity = new_total_qty
                                position.last_buy_bar_index = i
                                position.last_buy_timestamp = bar_ts

                                sellable_date = active_profile.settlement_engine.calculate_sellable_date(bar_date, session_policy="full_bar")
                                lot = HoldingLot(
                                    lot_id=f"lot_{uuid.uuid4().hex[:8]}",
                                    symbol=symbol,
                                    quantity=fill_qty,
                                    price=fill_price,
                                    acquisition_date=bar_date,
                                    acquisition_bar_index=i,
                                    sellable_date=sellable_date,
                                    is_settled=False,
                                )
                                position.lots.append(lot)

                                if active_trade is None:
                                    active_trade = BacktestTrade(
                                        id=f"trade_{uuid.uuid4().hex[:8]}",
                                        symbol=symbol,
                                        entry_order_id=pending_order.id,
                                        entry_time=bar_ts,
                                        entry_bar_index=i,
                                        quantity=fill_qty,
                                        entry_price=fill_price,
                                        fees=fee,
                                        taxes=0.0,
                                        status="open",
                                        result="open",
                                    )
                                    trades.append(active_trade)
                                else:
                                    active_trade.quantity += fill_qty
                                    active_trade.fees += fee

                                pending_order = None

                elif pending_order.side == "SELL":
                    if position.quantity > 0:
                        sellable_qty = active_profile.settlement_engine.get_sellable_quantity(
                            position.lots, bar_date, current_session_phase="open"
                        )
                        if not position.lots:
                            holding_bars = (i - position.last_buy_bar_index) if position.last_buy_bar_index is not None else 0
                            sellable_qty = position.quantity if holding_bars >= self.min_holding_bars else 0.0

                        if sellable_qty <= 0:
                            pending_order.status = "REJECTED"
                            pending_order.rejection_reason = "UNSETTLED_HOLDING_T2: Shares unsettled under Vietnam T+2 rules"
                            warnings.append(f"Bar {i} ({bar_ts}): Sell order rejected due to unsettled shares")
                            pending_order = None
                        else:
                            is_floor_locked = active_profile.rules.is_locked_floor(high_price, low_price, floor_price)
                            if is_floor_locked and active_profile.conservative_locked_limits:
                                pending_order.status = "REJECTED"
                                pending_order.rejection_reason = "LOCKED_FLOOR_REJECT: Locked floor bar; zero counter-liquidity to fill SELL order"
                                warnings.append(f"Bar {i} ({bar_ts}): Sell order rejected due to floor locked limit")
                                pending_order = None
                            else:
                                fill_qty = min(pending_order.quantity, sellable_qty)
                                gross = fill_qty * fill_price
                                fee = gross * self.fee_rate
                                tax = gross * self.tax_rate
                                net = gross - fee - tax

                                cash += net

                                execution = BacktestExecution(
                                    id=f"exec_{uuid.uuid4().hex[:8]}",
                                    order_id=pending_order.id,
                                    symbol=symbol,
                                    side="SELL",
                                    quantity=fill_qty,
                                    price=fill_price,
                                    gross_amount=gross,
                                    fee=fee,
                                    tax=tax,
                                    net_amount=net,
                                    signal_timestamp=pending_order.signal_timestamp,
                                    signal_bar_index=pending_order.signal_bar_index,
                                    order_timestamp=pending_order.order_timestamp,
                                    order_bar_index=pending_order.order_bar_index,
                                    fill_timestamp=bar_ts,
                                    fill_bar_index=i,
                                )
                                executions.append(execution)
                                pending_order.status = "FILLED"

                                if position.lots:
                                    _, remaining_lots = active_profile.settlement_engine.allocate_sell(
                                        position.lots, fill_qty, bar_date, current_session_phase="open"
                                    )
                                    position.lots = remaining_lots

                                position.quantity -= fill_qty
                                if position.quantity <= 0:
                                    position.quantity = 0.0
                                    position.average_price = 0.0
                                    position.cost_basis = 0.0
                                    position.last_buy_bar_index = None

                                if active_trade is not None:
                                    active_trade.exit_order_id = pending_order.id
                                    active_trade.exit_time = bar_ts
                                    active_trade.exit_bar_index = i
                                    active_trade.exit_price = fill_price
                                    active_trade.fees += fee
                                    active_trade.taxes += tax
                                    active_trade.gross_pnl = (fill_price - active_trade.entry_price) * fill_qty
                                    active_trade.net_pnl = active_trade.gross_pnl - active_trade.fees - active_trade.taxes
                                    if active_trade.entry_price * fill_qty > 0:
                                        active_trade.pnl_percent = (active_trade.net_pnl / (active_trade.entry_price * fill_qty)) * 100.0
                                    active_trade.holding_bars = i - active_trade.entry_bar_index
                                    active_trade.status = "closed"
                                    if active_trade.net_pnl > 0.0:
                                        active_trade.result = "win"
                                    elif active_trade.net_pnl < 0.0:
                                        active_trade.result = "loss"
                                    else:
                                        active_trade.result = "breakeven"
                                    active_trade = None

                                pending_order = None
                    else:
                        pending_order.status = "REJECTED"
                        pending_order.rejection_reason = "No position to sell"
                        pending_order = None

            # -----------------------------------------------------------------
            # 2. Intraday Stop Loss / Take Profit (if position held & T+2 satisfied)
            # -----------------------------------------------------------------
            if position.quantity > 0 and pending_order is None and position.last_buy_bar_index is not None:
                holding_bars = i - position.last_buy_bar_index
                if position.lots:
                    sellable_intraday = active_profile.settlement_engine.get_sellable_quantity(
                        position.lots, bar_date, current_session_phase="intraday"
                    ) > 0
                    if not has_explicit:
                        sellable_intraday = sellable_intraday or (holding_bars >= self.min_holding_bars)
                else:
                    sellable_intraday = holding_bars >= self.min_holding_bars

                if sellable_intraday and hasattr(strategy, "risk_management") and strategy.risk_management:
                    sl_pct = getattr(strategy.risk_management, "stop_loss_percent", None) or getattr(strategy.risk_management, "stop_loss", None)
                    tp_pct = getattr(strategy.risk_management, "take_profit_percent", None) or getattr(strategy.risk_management, "take_profit", None)

                    sl_price = position.average_price * (1.0 - sl_pct / 100.0) if sl_pct else None
                    tp_price = position.average_price * (1.0 + tp_pct / 100.0) if tp_pct else None

                    hit_sl = sl_price is not None and low_price <= sl_price
                    hit_tp = tp_price is not None and high_price >= tp_price

                    # Conservative precedence: Stop Loss triggers first if simultaneous
                    if hit_sl:
                        is_floor_locked = active_profile.rules.is_locked_floor(high_price, low_price, floor_price)
                        if is_floor_locked and active_profile.conservative_locked_limits:
                            warnings.append(f"Bar {i} ({bar_ts}): Intraday stop-loss suppressed due to floor locked limit")
                        else:
                            exit_price = min(open_price, sl_price)
                            self._execute_intraday_exit(
                                i=i,
                                bar_ts=bar_ts,
                                symbol=symbol,
                                exit_price=exit_price,
                                reason="STOP_LOSS",
                                position=position,
                                active_trade=active_trade,
                                orders=orders,
                                executions=executions,
                                active_profile=active_profile,
                                bar_date=bar_date,
                            )
                            cash += self._last_exit_net
                            active_trade = None
                    elif hit_tp:
                        is_floor_locked = active_profile.rules.is_locked_floor(high_price, low_price, floor_price)
                        if is_floor_locked and active_profile.conservative_locked_limits:
                            warnings.append(f"Bar {i} ({bar_ts}): Intraday take-profit suppressed due to floor locked limit")
                        else:
                            exit_price = max(open_price, tp_price)
                            self._execute_intraday_exit(
                                i=i,
                                bar_ts=bar_ts,
                                symbol=symbol,
                                exit_price=exit_price,
                                reason="TAKE_PROFIT",
                                position=position,
                                active_trade=active_trade,
                                orders=orders,
                                executions=executions,
                                active_profile=active_profile,
                                bar_date=bar_date,
                            )
                            cash += self._last_exit_net
                            active_trade = None

            # -----------------------------------------------------------------
            # 3. Strategy Rule Evaluation on Bar Close (Signal at Close[T])
            # -----------------------------------------------------------------
            if i >= 1:
                current = StrategyRuleEvaluator.indicator_snapshot(indicator_values, i)
                previous = StrategyRuleEvaluator.indicator_snapshot(indicator_values, i - 1)

                if position.quantity == 0 and pending_order is None:
                    # Check Entry Rules
                    if StrategyRuleEvaluator.evaluate_rules(strategy.entry_rules, current, previous):
                        if i == total_candles - 1:
                            # Final Bar Cutoff (TEST-BT-001)
                            cutoff_order = BacktestOrder(
                                id=f"ord_{uuid.uuid4().hex[:8]}",
                                symbol=symbol,
                                side="BUY",
                                order_type="MARKET_NEXT_OPEN",
                                quantity=0.0,
                                signal_bar_index=i,
                                signal_timestamp=bar_ts,
                                order_bar_index=i,
                                order_timestamp=bar_ts,
                                requested_price=close_price,
                                status="UNFILLED_AT_CUTOFF",
                                rejection_reason="Signal on final bar; no subsequent trading bar exists",
                            )
                            orders.append(cutoff_order)
                            unexecuted_signals.append({
                                "bar_index": i,
                                "timestamp": bar_ts,
                                "action": "BUY",
                                "reason": "UNFILLED_AT_CUTOFF",
                            })
                        else:
                            qty = self._calculate_quantity(strategy.position_sizing, cash, close_price, lot_size=active_profile.lot_size)
                            if qty >= active_profile.lot_size:
                                order = BacktestOrder(
                                    id=f"ord_{uuid.uuid4().hex[:8]}",
                                    symbol=symbol,
                                    side="BUY",
                                    order_type="MARKET_NEXT_OPEN",
                                    quantity=float(qty),
                                    signal_bar_index=i,
                                    signal_timestamp=bar_ts,
                                    order_bar_index=i,
                                    order_timestamp=bar_ts,
                                    requested_price=close_price,
                                    status="QUEUED",
                                )
                                orders.append(order)
                                pending_order = order
                            else:
                                rejected_order = BacktestOrder(
                                    id=f"ord_{uuid.uuid4().hex[:8]}",
                                    symbol=symbol,
                                    side="BUY",
                                    order_type="MARKET_NEXT_OPEN",
                                    quantity=float(qty),
                                    signal_bar_index=i,
                                    signal_timestamp=bar_ts,
                                    order_bar_index=i,
                                    order_timestamp=bar_ts,
                                    requested_price=close_price,
                                    status="REJECTED",
                                    rejection_reason=f"Insufficient cash for minimum {active_profile.lot_size}-share lot",
                                )
                                orders.append(rejected_order)
                                warnings.append(f"Bar {i} ({bar_ts}): Buy signal rejected due to cash sizing below {active_profile.lot_size} shares")

                elif position.quantity > 0 and pending_order is None and position.last_buy_bar_index is not None:
                    # In explicit profile mode, evaluate exit signal so order is queued and checked at execution time.
                    # In legacy mode (without explicit profile), enforce min_holding_bars before evaluating exit rules.
                    if has_explicit:
                        evaluate_exit = True
                    else:
                        holding_bars = i - position.last_buy_bar_index
                        evaluate_exit = (holding_bars >= self.min_holding_bars)

                    if evaluate_exit:
                        if StrategyRuleEvaluator.evaluate_rules(strategy.exit_rules, current, previous):
                            if i == total_candles - 1:
                                # Final Bar Cutoff (TEST-BT-001)
                                cutoff_order = BacktestOrder(
                                    id=f"ord_{uuid.uuid4().hex[:8]}",
                                    symbol=symbol,
                                    side="SELL",
                                    order_type="MARKET_NEXT_OPEN",
                                    quantity=position.quantity,
                                    signal_bar_index=i,
                                    signal_timestamp=bar_ts,
                                    order_bar_index=i,
                                    order_timestamp=bar_ts,
                                    requested_price=close_price,
                                    status="UNFILLED_AT_CUTOFF",
                                    rejection_reason="Exit signal on final bar; position closed at cutoff or held",
                                )
                                orders.append(cutoff_order)
                                unexecuted_signals.append({
                                    "bar_index": i,
                                    "timestamp": bar_ts,
                                    "action": "SELL",
                                    "reason": "UNFILLED_AT_CUTOFF",
                                })
                            else:
                                order = BacktestOrder(
                                    id=f"ord_{uuid.uuid4().hex[:8]}",
                                    symbol=symbol,
                                    side="SELL",
                                    order_type="MARKET_NEXT_OPEN",
                                    quantity=position.quantity,
                                    signal_bar_index=i,
                                    signal_timestamp=bar_ts,
                                    order_bar_index=i,
                                    order_timestamp=bar_ts,
                                    requested_price=close_price,
                                    status="QUEUED",
                                )
                                orders.append(order)
                                pending_order = order

            # -----------------------------------------------------------------
            # 4. Bar Equity & Drawdown Accounting
            # -----------------------------------------------------------------
            position.current_price = close_price
            position.unrealized_pnl = (close_price - position.average_price) * position.quantity if position.quantity > 0 else 0.0
            bar_portfolio_value = cash + (position.quantity * close_price)
            peak_equity = max(peak_equity, bar_portfolio_value)
            drawdown = (peak_equity - bar_portfolio_value) / peak_equity if peak_equity > 0 else 0.0
            max_drawdown = max(max_drawdown, drawdown)

            iso_date = bar_ts.isoformat() if hasattr(bar_ts, "isoformat") else str(bar_ts)
            equity_curve.append({
                "date": iso_date,
                "timestamp": bar_ts,
                "bar_index": i,
                "cash": cash,
                "portfolio_value": bar_portfolio_value,
                "close": close_price,
                "drawdown": drawdown,
                "has_position": position.quantity > 0,
            })

        # Phase-End Boundary Assessment (TEST-BT-005)
        is_unsellable_at_end = False
        if position.quantity > 0 and position.lots:
            final_bar_date = df.iloc[-1]["timestamp"]
            final_date = final_bar_date.date() if isinstance(final_bar_date, (datetime, pd.Timestamp)) else final_bar_date
            sellable_at_end = active_profile.settlement_engine.get_sellable_quantity(
                position.lots, final_date, current_session_phase="close"
            )
            if sellable_at_end < position.quantity:
                is_unsellable_at_end = True

        ledger = BacktestLedger(
            initial_cash=initial_cash,
            current_cash=cash,
            portfolio_value=bar_portfolio_value if total_candles > 0 else initial_cash,
            peak_portfolio_value=peak_equity,
            max_drawdown=max_drawdown,
            orders=orders,
            executions=executions,
            trades=trades,
            equity_curve=equity_curve,
            unexecuted_signals=unexecuted_signals,
            warnings=warnings,
        )

        result = BacktestKernelResult(
            ledger=ledger,
            symbol=symbol,
            timeframe=timeframe,
            start_date=start_date,
            end_date=end_date,
            total_candles=total_candles,
            last_analyzed_index=total_candles - 1,
        )

        # Attach market profile metadata and open position properties (TEST-BT-005)
        result.execution_profile = active_profile.name
        result.market_rule_version = active_profile.version
        result.calendar_version = active_profile.calendar.VERSION
        result.open_position = position
        result.has_unsellable_open_position = is_unsellable_at_end

        return result

    def _execute_intraday_exit(
        self,
        i: int,
        bar_ts: Any,
        symbol: str,
        exit_price: float,
        reason: str,
        position: BacktestPosition,
        active_trade: Optional[BacktestTrade],
        orders: List[BacktestOrder],
        executions: List[BacktestExecution],
        active_profile: Optional[MarketProfile] = None,
        bar_date: Optional[date] = None,
    ) -> None:
        """Helper to process intraday SL/TP executions."""
        qty = position.quantity
        gross = qty * exit_price
        fee = gross * self.fee_rate
        tax = gross * self.tax_rate
        net = gross - fee - tax
        self._last_exit_net = net

        order = BacktestOrder(
            id=f"ord_{uuid.uuid4().hex[:8]}",
            symbol=symbol,
            side="SELL",
            order_type=reason,
            quantity=qty,
            signal_bar_index=i,
            signal_timestamp=bar_ts,
            order_bar_index=i,
            order_timestamp=bar_ts,
            requested_price=exit_price,
            status="FILLED",
        )
        orders.append(order)

        execution = BacktestExecution(
            id=f"exec_{uuid.uuid4().hex[:8]}",
            order_id=order.id,
            symbol=symbol,
            side="SELL",
            quantity=qty,
            price=exit_price,
            gross_amount=gross,
            fee=fee,
            tax=tax,
            net_amount=net,
            signal_timestamp=bar_ts,
            signal_bar_index=i,
            order_timestamp=bar_ts,
            order_bar_index=i,
            fill_timestamp=bar_ts,
            fill_bar_index=i,
        )
        executions.append(execution)

        # Allocate sell across discrete lots FIFO if tracked
        if hasattr(position, "lots") and position.lots and active_profile and bar_date:
            _, remaining_lots = active_profile.settlement_engine.allocate_sell(
                position.lots, qty, bar_date, current_session_phase="intraday"
            )
            position.lots = remaining_lots

        position.quantity = 0.0
        position.average_price = 0.0
        position.cost_basis = 0.0
        position.last_buy_bar_index = None

        if active_trade is not None:
            active_trade.exit_order_id = order.id
            active_trade.exit_time = bar_ts
            active_trade.exit_bar_index = i
            active_trade.exit_price = exit_price
            active_trade.fees += fee
            active_trade.taxes += tax
            active_trade.gross_pnl = (exit_price - active_trade.entry_price) * active_trade.quantity
            active_trade.net_pnl = active_trade.gross_pnl - active_trade.fees - active_trade.taxes
            if active_trade.entry_price * active_trade.quantity > 0:
                active_trade.pnl_percent = (active_trade.net_pnl / (active_trade.entry_price * active_trade.quantity)) * 100.0
            active_trade.holding_bars = i - active_trade.entry_bar_index
            active_trade.status = "closed"
            active_trade.exit_reason = reason
            if active_trade.net_pnl > 0.0:
                active_trade.result = "win"
            elif active_trade.net_pnl < 0.0:
                active_trade.result = "loss"
            else:
                active_trade.result = "breakeven"

    def _calculate_quantity(self, sizing: Any, current_cash: float, price: float, lot_size: int = 100) -> int:
        """Calculate fee-aware lot sizing aligned with Vietnam 100-share lots."""
        if not sizing or lot_size <= 0:
            return 0

        method = getattr(sizing, "method", None) or (sizing.get("method") if isinstance(sizing, dict) else None)
        cost_per_share = price * (1.0 + self.fee_rate)

        if method == "fixed_quantity":
            qty = getattr(sizing, "quantity", None) or (sizing.get("quantity") if isinstance(sizing, dict) else 0)
            max_buyable = int((current_cash / cost_per_share) // lot_size) * lot_size
            return max(0, min(int(qty), max_buyable))

        elif method == "percent_equity":
            percent = getattr(sizing, "percent", None) or (sizing.get("percent") if isinstance(sizing, dict) else 100.0)
            amount = current_cash * (float(percent) / 100.0)
            max_buyable = int((amount / cost_per_share) // lot_size) * lot_size
            return max(0, max_buyable)

        return 0
