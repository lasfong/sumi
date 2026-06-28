from sqlalchemy.orm import Session
from sqlalchemy import func
import numpy as np
import math
from app.models.trade import Trade
from app.models.replay_session import ReplaySession
from app.models.execution import Execution
from app.models.order import Order
from app.models.candle import Candle
from app.schemas.analytics_schema import (
    AnalyticsResponse, SetupPerformance, DrawdownPeriod,
    BenchmarkPoint, TradeDistribution, GroupPerformance, OutlierImpact
)

class AnalyticsService:
    @staticmethod
    def _build_equity_curve(db: Session, session: ReplaySession, candles: list) -> list:
        initial_cash = float(session.initial_cash)

        executions = db.query(Execution).filter_by(session_id=session.id)\
            .order_by(Execution.execution_date).all()

        cash = initial_cash
        holdings = {}
        equity_curve = []
        peak_equity = initial_cash

        exec_index = 0

        for candle in candles:
            # Replay service ensures candle has timestamp
            candle_date = candle.timestamp

            # Apply executions up to this candle
            while exec_index < len(executions):
                ex = executions[exec_index]
                if str(ex.execution_date) > str(candle_date):
                    break

                order = db.query(Order).filter_by(id=ex.order_id).first()
                if order.side == "BUY":
                    cash -= float(ex.net_amount)
                    sym = ex.symbol
                    if sym not in holdings:
                        holdings[sym] = {"qty": 0}
                    holdings[sym]["qty"] += ex.quantity
                elif order.side == "SELL":
                    cash += float(ex.net_amount)
                    sym = ex.symbol
                    if sym in holdings:
                        holdings[sym]["qty"] -= ex.quantity
                        if holdings[sym]["qty"] <= 0:
                            del holdings[sym]

                exec_index += 1

            close_price = float(candle.close)
            holdings_value = sum(h["qty"] * close_price for h in holdings.values())
            equity = cash + holdings_value
            if equity > peak_equity:
                peak_equity = equity
            drawdown = peak_equity - equity
            drawdown_pct = (drawdown / peak_equity * 100) if peak_equity > 0 else 0.0

            equity_curve.append({
                "timestamp": str(candle_date),
                "equity": round(equity, 2),
                "cash": round(cash, 2),
                "holdings_value": round(holdings_value, 2),
                "drawdown": round(drawdown, 2),
                "drawdown_pct": round(drawdown_pct, 2),
            })

        return equity_curve

    @staticmethod
    def _calculate_max_drawdown(equity_curve: list) -> dict:
        if not equity_curve:
            return {"max_drawdown_amount": 0.0, "max_drawdown_pct": 0.0}

        peak = equity_curve[0]["equity"]
        max_dd_amount = 0.0
        max_dd_pct = 0.0

        for point in equity_curve:
            equity = point["equity"]
            if equity > peak:
                peak = equity

            dd_amount = peak - equity
            dd_pct = (dd_amount / peak * 100) if peak > 0 else 0

            if dd_amount > max_dd_amount:
                max_dd_amount = dd_amount
            if dd_pct > max_dd_pct:
                max_dd_pct = dd_pct

        return {
            "max_drawdown_amount": round(max_dd_amount, 2),
            "max_drawdown_pct": round(max_dd_pct, 2)
        }

    @staticmethod
    def _calculate_drawdown_periods(equity_curve: list) -> list:
        if not equity_curve:
            return []

        periods = []
        peak = equity_curve[0]["equity"]
        in_drawdown = False
        dd_start = None
        dd_max_pct = 0.0

        for point in equity_curve:
            equity = point["equity"]

            if equity >= peak:
                if in_drawdown:
                    periods.append(DrawdownPeriod(
                        start=dd_start,
                        end=point["timestamp"],
                        max_drawdown_pct=round(dd_max_pct, 2)
                    ))
                    in_drawdown = False
                    dd_max_pct = 0.0
                peak = equity
            else:
                if not in_drawdown:
                    dd_start = point["timestamp"]
                    in_drawdown = True
                dd_pct = (peak - equity) / peak * 100
                dd_max_pct = max(dd_max_pct, dd_pct)

        if in_drawdown:
            periods.append(DrawdownPeriod(
                start=dd_start,
                end=equity_curve[-1]["timestamp"],
                max_drawdown_pct=round(dd_max_pct, 2)
            ))

        return periods

    @staticmethod
    def _calculate_sharpe_ratio(equity_curve: list, rf_annual: float = 0.045) -> float:
        if len(equity_curve) < 2:
            return 0.0

        equities = [p["equity"] for p in equity_curve]
        returns = []
        for i in range(1, len(equities)):
            if equities[i-1] > 0:
                returns.append(equities[i] / equities[i-1] - 1)

        if not returns:
            return 0.0

        returns_arr = np.array(returns)
        rf_daily = (1 + rf_annual) ** (1/252) - 1
        excess = returns_arr - rf_daily

        std = np.std(excess, ddof=1) if len(excess) > 1 else 0
        if std == 0:
            return 0.0

        sharpe = (np.mean(excess) / std) * np.sqrt(252)
        return round(float(sharpe), 4)

    @staticmethod
    def _get_benchmark_curve(db: Session, session: ReplaySession) -> list:
        initial_cash = float(session.initial_cash)

        vnindex_candles = db.query(Candle).filter(
            Candle.symbol == "VNINDEX",
            Candle.timestamp >= session.start_date,
            Candle.timestamp <= session.end_date
        ).order_by(Candle.timestamp).all()

        if not vnindex_candles:
            return []

        first_close = float(vnindex_candles[0].close)

        return [BenchmarkPoint(
            time=str(c.timestamp),
            value=round(float(c.close) / first_close * initial_cash, 2) if first_close > 0 else initial_cash
        ) for c in vnindex_candles]

    @staticmethod
    def _build_group_performance(trades: list, attr: str, fallback: str) -> list[GroupPerformance]:
        grouped = {}
        for trade in trades:
            key = getattr(trade, attr, None) or fallback
            if key not in grouped:
                grouped[key] = {"trades": 0, "wins": 0, "pnl": []}
            pnl = trade.net_pnl or 0.0
            grouped[key]["trades"] += 1
            grouped[key]["wins"] += 1 if pnl > 0 else 0
            grouped[key]["pnl"].append(pnl)

        rows = []
        for key, stats in grouped.items():
            pnl_values = stats["pnl"]
            trade_count = stats["trades"]
            net_pnl = sum(pnl_values)
            rows.append(GroupPerformance(
                key=key,
                trades=trade_count,
                win_rate=stats["wins"] / trade_count if trade_count else 0.0,
                net_pnl=round(net_pnl, 2),
                average_pnl=round(net_pnl / trade_count, 2) if trade_count else 0.0,
                best_trade=round(max(pnl_values), 2) if pnl_values else None,
                worst_trade=round(min(pnl_values), 2) if pnl_values else None,
            ))

        return sorted(rows, key=lambda row: row.net_pnl, reverse=True)

    @staticmethod
    def _calculate_outlier_impact(trades: list, top_n: int = 3) -> OutlierImpact:
        pnl_values = [trade.net_pnl or 0.0 for trade in trades]
        if not pnl_values:
            return OutlierImpact(top_winners_pnl=0.0, top_losers_pnl=0.0)

        winners = sorted([pnl for pnl in pnl_values if pnl > 0], reverse=True)[:top_n]
        losers = sorted([pnl for pnl in pnl_values if pnl < 0])[:top_n]
        top_winners_pnl = round(sum(winners), 2)
        top_losers_pnl = round(sum(losers), 2)
        gross_movement = sum(abs(pnl) for pnl in pnl_values)

        trimmed = sorted(pnl_values)
        if len(trimmed) >= 5:
            trimmed = trimmed[1:-1]

        return OutlierImpact(
            top_winners_pnl=top_winners_pnl,
            top_losers_pnl=top_losers_pnl,
            top_winners_share=round(top_winners_pnl / gross_movement, 4) if gross_movement else None,
            top_losers_share=round(abs(top_losers_pnl) / gross_movement, 4) if gross_movement else None,
            median_trade_pnl=round(float(np.median(pnl_values)), 2),
            trimmed_expectancy=round(sum(trimmed) / len(trimmed), 2) if trimmed else None,
        )

    @staticmethod
    def get_analytics(db: Session, session_id: int) -> AnalyticsResponse:
        session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
        trades = db.query(Trade).filter(Trade.session_id == session_id, Trade.exit_date.isnot(None)).order_by(Trade.exit_date).all()

        total_trades = len(trades)
        if total_trades == 0:
            return AnalyticsResponse(
                total_trades=0, win_rate=0.0, total_net_pnl=0.0,
                average_win=0.0, average_loss=0.0, profit_factor=0.0
            )

        winning_trades = [t for t in trades if t.net_pnl and t.net_pnl > 0]
        losing_trades = [t for t in trades if t.net_pnl and t.net_pnl <= 0]

        win_rate = len(winning_trades) / total_trades
        total_net_pnl = sum(t.net_pnl or 0 for t in trades)

        gross_profit = sum(t.net_pnl or 0 for t in winning_trades)
        gross_loss = abs(sum(t.net_pnl or 0 for t in losing_trades))

        average_win = gross_profit / len(winning_trades) if winning_trades else 0
        average_loss = gross_loss / len(losing_trades) if losing_trades else 0

        profit_factor = round(gross_profit / gross_loss, 4) if gross_loss > 0 else None

        largest_win = max([t.net_pnl or 0 for t in trades]) if trades else 0
        largest_loss = min([t.net_pnl or 0 for t in trades]) if trades else 0

        # Calculate expectancy
        expectancy = (win_rate * average_win) - ((1 - win_rate) * average_loss)
        expectancy = round(expectancy, 2)

        # Calculate setup performance
        setup_stats = {}
        for t in trades:
            setup = t.setup_type or "Uncategorized"
            if setup not in setup_stats:
                setup_stats[setup] = {"trades": 0, "wins": 0, "net_pnl": 0.0}
            setup_stats[setup]["trades"] += 1
            if t.net_pnl and t.net_pnl > 0:
                setup_stats[setup]["wins"] += 1
            setup_stats[setup]["net_pnl"] += (t.net_pnl or 0)

        setup_performance = []
        for setup, stats in setup_stats.items():
            setup_performance.append(SetupPerformance(
                setup_type=setup,
                trades=stats["trades"],
                win_rate=stats["wins"] / stats["trades"] if stats["trades"] > 0 else 0,
                net_pnl=stats["net_pnl"]
            ))

        # Calculate R-multiple
        r_multiples = []
        for t in trades:
            if t.initial_stop_loss and t.entry_price and t.entry_price != t.initial_stop_loss:
                risk = abs(t.entry_price - t.initial_stop_loss)
                pnl = t.net_pnl or 0
                r_mult = pnl / (risk * t.quantity) if t.quantity > 0 else 0
                r_multiples.append(r_mult)

        average_r = sum(r_multiples) / len(r_multiples) if r_multiples else None

        # Sortino and SQN (simplified)
        avg_pnl = total_net_pnl / total_trades
        variance = sum(((t.net_pnl or 0) - avg_pnl) ** 2 for t in trades) / total_trades
        std_dev = math.sqrt(variance) if variance > 0 else 1.0
        sqn = (math.sqrt(total_trades) * avg_pnl) / std_dev if std_dev > 0 else 0

        downside_variance = sum(min(0, t.net_pnl or 0) ** 2 for t in trades) / total_trades
        downside_std_dev = math.sqrt(downside_variance) if downside_variance > 0 else 1.0
        sortino_ratio = avg_pnl / downside_std_dev if downside_std_dev > 0 else 0

        # Get session candles to build curve
        from app.services.replay_service import ReplayService
        candles = ReplayService.get_candles(db, session_id)

        # Equity Curve and related
        equity_curve = AnalyticsService._build_equity_curve(db, session, candles)
        dd_stats = AnalyticsService._calculate_max_drawdown(equity_curve)
        max_drawdown = dd_stats["max_drawdown_amount"]
        max_drawdown_pct = dd_stats["max_drawdown_pct"]
        drawdown_periods = AnalyticsService._calculate_drawdown_periods(equity_curve)
        sharpe_ratio = AnalyticsService._calculate_sharpe_ratio(equity_curve)
        benchmark_curve = AnalyticsService._get_benchmark_curve(db, session)
        symbol_performance = AnalyticsService._build_group_performance(trades, "symbol", "Unknown")
        mistake_performance = AnalyticsService._build_group_performance(trades, "mistake_tag", "No mistake tag")
        outlier_impact = AnalyticsService._calculate_outlier_impact(trades)

        trade_distribution = [
            TradeDistribution(
                trade_id=t.id,
                symbol=t.symbol,
                net_pnl=t.net_pnl or 0.0,
                pnl_percent=t.pnl_percent or 0.0,
                r_multiple=(t.net_pnl or 0) / (abs(t.entry_price - t.initial_stop_loss) * t.quantity) if t.initial_stop_loss and t.entry_price and t.entry_price != t.initial_stop_loss and t.quantity else None,
                result=t.result
            ) for t in trades
        ]

        return AnalyticsResponse(
            total_trades=total_trades,
            win_rate=win_rate,
            total_net_pnl=total_net_pnl,
            average_win=average_win,
            average_loss=average_loss,
            profit_factor=profit_factor,
            average_r=average_r,
            expectancy=expectancy,
            largest_win=largest_win,
            largest_loss=largest_loss,
            max_drawdown=max_drawdown,
            max_drawdown_pct=max_drawdown_pct,
            sharpe_ratio=sharpe_ratio,
            sortino_ratio=sortino_ratio,
            sqn=sqn,
            setup_performance=setup_performance,
            equity_curve=equity_curve,
            drawdown_periods=drawdown_periods,
            benchmark_curve=benchmark_curve,
            trade_distribution=trade_distribution,
            symbol_performance=symbol_performance,
            mistake_performance=mistake_performance,
            outlier_impact=outlier_impact
        )
