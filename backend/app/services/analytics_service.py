from sqlalchemy.orm import Session
from fastapi import HTTPException
import numpy as np
import math
from app.models.trade import Trade
from app.models.replay_session import ReplaySession
from app.models.execution import Execution
from app.models.order import Order
from app.models.candle import Candle
from app.utils.date_range import end_before, start_at
from app.schemas.analytics_schema import (
    AnalyticsResponse, SetupPerformance, DrawdownPeriod,
    BenchmarkPoint, TradeDistribution, GroupPerformance, OutlierImpact
)
from app.schemas.analytics_trust_schema import MetricResult
import json

class AnalyticsService:
    MIN_STATISTICAL_SAMPLE = 30

    @staticmethod
    def _build_equity_curve(db: Session, session: ReplaySession, candles: list) -> list:
        initial_cash = float(session.initial_cash)

        executions = db.query(Execution).filter_by(
            session_id=session.id,
            symbol=session.symbol,
        ).order_by(Execution.execution_date, Execution.id).all()
        order_ids = [execution.order_id for execution in executions]
        orders = {
            order.id: order
            for order in db.query(Order).filter(Order.id.in_(order_ids)).all()
        } if order_ids else {}

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
                if ex.execution_date > candle_date:
                    break

                order = orders.get(ex.order_id)
                if order is None:
                    raise ValueError(f"Execution {ex.id} references missing order {ex.order_id}")
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
    def _periodic_returns(equity_curve: list) -> list[float]:
        returns = []
        for previous, current in zip(equity_curve, equity_curve[1:]):
            previous_equity = float(previous["equity"])
            if previous_equity > 0:
                returns.append(float(current["equity"]) / previous_equity - 1)
        return returns

    @staticmethod
    def _build_metric_results(trades: list, equity_curve: list) -> dict[str, MetricResult]:
        trade_count = len(trades)
        pnl_values = [float(trade.net_pnl or 0.0) for trade in trades]
        winners = [value for value in pnl_values if value > 0]
        losses = [value for value in pnl_values if value <= 0]
        returns = AnalyticsService._periodic_returns(equity_curve)
        min_sample = AnalyticsService.MIN_STATISTICAL_SAMPLE

        def insufficient(sample_size: int, reason: str, period: str) -> MetricResult:
            return MetricResult(
                value=None,
                status="insufficient_data",
                sample_size=sample_size,
                period=period,
                reason=reason,
            )

        if trade_count < min_sample:
            win_rate = insufficient(
                trade_count,
                f"At least {min_sample} closed trades are required before win rate is evidence.",
                "closed_trades",
            )
            profit_factor = insufficient(
                trade_count,
                f"At least {min_sample} closed trades are required before profit factor is evidence.",
                "closed_trades",
            )
            sqn = insufficient(
                trade_count,
                f"SQN requires at least {min_sample} closed trades.",
                "closed_trades",
            )
        else:
            win_rate = MetricResult(
                value=round(len(winners) / trade_count, 6),
                status="valid",
                sample_size=trade_count,
                period="closed_trades",
            )
            gross_loss = abs(sum(losses))
            if gross_loss == 0:
                profit_factor = MetricResult(
                    value=None,
                    status="not_applicable",
                    sample_size=trade_count,
                    period="closed_trades",
                    reason="Profit factor is undefined because there are no losing trades.",
                )
            else:
                profit_factor = MetricResult(
                    value=round(sum(winners) / gross_loss, 4),
                    status="valid",
                    sample_size=trade_count,
                    period="closed_trades",
                )

            sample_std = float(np.std(pnl_values, ddof=1)) if trade_count > 1 else 0.0
            if sample_std == 0:
                sqn = MetricResult(
                    value=None,
                    status="not_applicable",
                    sample_size=trade_count,
                    period="closed_trades",
                    reason="SQN is undefined because closed-trade PnL has zero variance.",
                )
            else:
                sqn = MetricResult(
                    value=round(math.sqrt(trade_count) * float(np.mean(pnl_values)) / sample_std, 4),
                    status="valid",
                    sample_size=trade_count,
                    period="closed_trades",
                )

        return_count = len(returns)
        if return_count < min_sample:
            sharpe = insufficient(
                return_count,
                f"Sharpe requires at least {min_sample} daily equity returns.",
                "daily_equity_returns",
            )
            sortino = insufficient(
                return_count,
                f"Sortino requires at least {min_sample} daily equity returns.",
                "daily_equity_returns",
            )
        else:
            returns_arr = np.array(returns)
            rf_daily = (1 + 0.045) ** (1 / 252) - 1
            excess = returns_arr - rf_daily
            sample_std = float(np.std(excess, ddof=1))
            if sample_std == 0:
                sharpe = MetricResult(
                    value=None,
                    status="not_applicable",
                    sample_size=return_count,
                    period="daily_equity_returns",
                    reason="Sharpe is undefined because periodic excess returns have zero variance.",
                )
            else:
                sharpe = MetricResult(
                    value=round(float(np.mean(excess) / sample_std * np.sqrt(252)), 4),
                    status="valid",
                    sample_size=return_count,
                    period="daily_equity_returns",
                )

            downside = [value for value in returns if value < 0]
            if len(downside) < 2:
                sortino = MetricResult(
                    value=None,
                    status="insufficient_data",
                    sample_size=return_count,
                    period="daily_equity_returns",
                    reason="Sortino requires at least two downside observations.",
                )
            else:
                downside_deviation = math.sqrt(sum(value ** 2 for value in downside) / len(downside))
                sortino = MetricResult(
                    value=round(float(np.mean(returns)) / downside_deviation * np.sqrt(252), 4),
                    status="valid",
                    sample_size=return_count,
                    period="daily_equity_returns",
                )

        total_net_pnl = MetricResult(
            value=round(sum(pnl_values), 2) if trade_count else None,
            status="valid" if trade_count else "not_applicable",
            sample_size=trade_count,
            period="closed_trades",
            reason=None if trade_count else "Net PnL is not applicable because there are no closed trades.",
        )
        expectancy = (
            insufficient(
                trade_count,
                f"Expectancy requires at least {min_sample} closed trades.",
                "closed_trades",
            )
            if trade_count < min_sample else MetricResult(
                value=round(float(np.mean(pnl_values)), 2),
                status="valid",
                sample_size=trade_count,
                period="closed_trades",
            )
        )

        return {
            "total_net_pnl": total_net_pnl,
            "expectancy": expectancy,
            "win_rate": win_rate,
            "profit_factor": profit_factor,
            "sqn": sqn,
            "sharpe_ratio": sharpe,
            "sortino_ratio": sortino,
        }

    @staticmethod
    def _get_benchmark_curve(db: Session, session: ReplaySession, benchmark_symbol: str | None) -> list:
        initial_cash = float(session.initial_cash)

        if not benchmark_symbol:
            return []

        vnindex_candles = db.query(Candle).filter(
            Candle.symbol == benchmark_symbol,
            Candle.timestamp >= start_at(session.start_date),
            Candle.timestamp < end_before(session.end_date)
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
    def get_analytics(
        db: Session, session_id: int, benchmark_symbol: str | None = None,
    ) -> AnalyticsResponse:
        session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
        if session is None:
            raise HTTPException(status_code=404, detail="Session not found")
        trades = db.query(Trade).filter(Trade.session_id == session_id, Trade.exit_date.isnot(None)).order_by(Trade.exit_date).all()

        from app.services.replay_service import ReplayService
        candles = ReplayService.get_candles(db, session_id)
        equity_curve = AnalyticsService._build_equity_curve(db, session, candles)
        dd_stats = AnalyticsService._calculate_max_drawdown(equity_curve)
        drawdown_periods = AnalyticsService._calculate_drawdown_periods(equity_curve)
        if benchmark_symbol is None and session.source_payload:
            try:
                benchmark_symbol = json.loads(session.source_payload).get("benchmark_symbol")
            except (json.JSONDecodeError, TypeError, AttributeError):
                benchmark_symbol = None
        if benchmark_symbol is None:
            benchmark_symbol = "VNINDEX"
        benchmark_curve = AnalyticsService._get_benchmark_curve(db, session, benchmark_symbol)

        total_trades = len(trades)
        metric_results = AnalyticsService._build_metric_results(trades, equity_curve)
        metric_results["benchmark"] = MetricResult(
            value=(benchmark_curve[-1].value / benchmark_curve[0].value - 1) if len(benchmark_curve) >= 2 and benchmark_curve[0].value else None,
            status="valid" if len(benchmark_curve) >= 2 and benchmark_curve[0].value else "not_applicable",
            sample_size=len(benchmark_curve),
            period="benchmark_candles",
            reason=None if len(benchmark_curve) >= 2 and benchmark_curve[0].value else f"Benchmark {benchmark_symbol} has insufficient coverage for this session.",
        )
        if total_trades == 0:
            return AnalyticsResponse(
                benchmark_symbol=benchmark_symbol,
                total_trades=0, win_rate=0.0, total_net_pnl=0.0,
                average_win=0.0, average_loss=0.0, profit_factor=None,
                expectancy=0.0,
                largest_win=0.0,
                largest_loss=0.0,
                max_drawdown=dd_stats["max_drawdown_amount"],
                max_drawdown_pct=dd_stats["max_drawdown_pct"],
                sharpe_ratio=metric_results["sharpe_ratio"].value,
                sortino_ratio=metric_results["sortino_ratio"].value,
                sqn=metric_results["sqn"].value,
                setup_performance=[],
                equity_curve=equity_curve,
                drawdown_periods=drawdown_periods,
                benchmark_curve=benchmark_curve,
                trade_distribution=[],
                symbol_performance=[],
                mistake_performance=[],
                outlier_impact=AnalyticsService._calculate_outlier_impact([]),
                metrics=metric_results,
            )

        winning_trades = [t for t in trades if t.net_pnl and t.net_pnl > 0]
        losing_trades = [t for t in trades if t.net_pnl and t.net_pnl <= 0]

        win_rate = len(winning_trades) / total_trades
        total_net_pnl = sum(t.net_pnl or 0 for t in trades)

        gross_profit = sum(t.net_pnl or 0 for t in winning_trades)
        gross_loss = abs(sum(t.net_pnl or 0 for t in losing_trades))

        average_win = gross_profit / len(winning_trades) if winning_trades else 0
        average_loss = gross_loss / len(losing_trades) if losing_trades else 0

        # Retain the legacy scalar for stored-history/API compatibility. The typed
        # metric result is authoritative for evidence and presentation.
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

        sqn = metric_results["sqn"].value
        sortino_ratio = metric_results["sortino_ratio"].value

        # Equity Curve and related
        max_drawdown = dd_stats["max_drawdown_amount"]
        max_drawdown_pct = dd_stats["max_drawdown_pct"]
        sharpe_ratio = metric_results["sharpe_ratio"].value
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
            benchmark_symbol=benchmark_symbol,
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
            ,metrics=metric_results
        )
