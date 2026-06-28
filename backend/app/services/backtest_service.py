import pandas as pd
import numpy as np
from datetime import datetime
from sqlalchemy.orm import Session
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.services.analytics_service import AnalyticsService
from app.models.replay_session import ReplaySession
from app.models.candle import Candle
from app.models.trade import Trade
from app.domain.regime.regime_classifier import RegimeClassifier
import uuid
from app.domain.strategy.rule_evaluator import (
    RuleEvaluationError,
    evaluate_condition,
    evaluate_rule_dsl,
    validate_condition,
    validate_rule_dsl,
)

class BacktestService:
    def __init__(self):
        self.trade_service = TradeLifecycleService()
        self.analytics_service = AnalyticsService()

    async def run_backtest(self, db: Session, config: dict) -> dict:
        symbols = config.get("symbols")
        if symbols:
            clean_symbols = [symbol for symbol in symbols if symbol]
            if not clean_symbols:
                return {
                    "status": "failed",
                    "error_code": "INVALID_SYMBOLS",
                    "message": "At least one symbol is required.",
                    "analytics": None,
                }
            if len(clean_symbols) > 1:
                return await self._run_multi_symbol_backtest(db, config, clean_symbols)
            config = {**config, "symbol": clean_symbols[0]}

        if not config.get("symbol"):
            return {
                "status": "failed",
                "error_code": "INVALID_SYMBOL",
                "message": "symbol or symbols is required.",
                "analytics": None,
            }

        return await self._run_single_symbol_backtest(db, config)

    async def _run_multi_symbol_backtest(self, db: Session, config: dict, symbols: list[str]) -> dict:
        runs = []
        for symbol in symbols:
            run_config = {**config, "symbol": symbol}
            run_config.pop("symbols", None)
            runs.append(await self._run_single_symbol_backtest(db, run_config))

        succeeded = [run for run in runs if run.get("status") == "succeeded"]
        failed = [run for run in runs if run.get("status") == "failed"]
        if succeeded and failed:
            status = "partial"
        elif failed and not succeeded:
            status = "failed"
        else:
            status = "succeeded"

        return {
            "status": status,
            "symbols": symbols,
            "runs": runs,
            "summary": self._summarize_runs(runs),
        }

    def _summarize_runs(self, runs: list[dict]) -> dict:
        analytics_rows = [run["analytics"] for run in runs if run.get("analytics")]
        total_trades = sum(row.get("total_trades", 0) for row in analytics_rows)
        total_net_pnl = sum(row.get("total_net_pnl", 0.0) for row in analytics_rows)
        weighted_wins = sum(row.get("win_rate", 0.0) * row.get("total_trades", 0) for row in analytics_rows)
        symbol_pnls = [
            {"symbol": run.get("symbol"), "net_pnl": run["analytics"].get("total_net_pnl", 0.0)}
            for run in runs
            if run.get("analytics")
        ]

        return {
            "total_symbols": len(runs),
            "succeeded_symbols": len([run for run in runs if run.get("status") == "succeeded"]),
            "failed_symbols": len([run for run in runs if run.get("status") == "failed"]),
            "total_candles": sum(run.get("total_candles", 0) for run in runs),
            "total_trades": total_trades,
            "win_rate": weighted_wins / total_trades if total_trades else 0.0,
            "total_net_pnl": round(total_net_pnl, 2),
            "best_symbol": max(symbol_pnls, key=lambda row: row["net_pnl"]) if symbol_pnls else None,
            "worst_symbol": min(symbol_pnls, key=lambda row: row["net_pnl"]) if symbol_pnls else None,
        }
    
    async def _run_single_symbol_backtest(self, db: Session, config: dict) -> dict:
        """
        Chạy backtest cho 1 strategy trên 1 symbol.
        """
        strategy = load_strategy_from_dict(config["strategy"])
        symbol = config["symbol"]
        start_date = config["start_date"]
        end_date = config["end_date"]
        initial_cash = config.get("initial_cash", 100000000)
        benchmark_symbol = config.get("benchmark_symbol", "VNINDEX")
        
        # 1. Load ALL candles for symbol
        candles = db.query(Candle).filter(
            Candle.symbol == symbol,
            Candle.timestamp >= start_date,
            Candle.timestamp <= end_date
        ).order_by(Candle.timestamp).all()
        
        if len(candles) == 0:
            return {
                "status": "failed",
                "error_code": "NO_CANDLES",
                "message": f"No candles found for {symbol}",
                "symbol": symbol,
                "total_candles": 0,
                "analytics": None,
            }
        
        # 2. Create virtual session
        session = ReplaySession(
            symbol=symbol,
            timeframe="1D",
            start_date=candles[0].timestamp,
            end_date=candles[-1].timestamp,
            current_index=0,
            initial_cash=initial_cash,
            current_cash=initial_cash,
            mode="backtest",
            status="active"
        )
        db.add(session)
        db.flush()
        
        # 3. Precompute indicators
        df = pd.DataFrame([{
            "timestamp": c.timestamp,
            "open": c.open,
            "high": c.high,
            "low": c.low,
            "close": c.close,
            "volume": c.volume
        } for c in candles])
        
        indicator_values = self._compute_indicators(df, strategy.indicators)
        try:
            self._validate_strategy_rules(strategy, set(indicator_values.keys()))
        except RuleEvaluationError as exc:
            return {
                "status": "failed",
                "error_code": "INVALID_RULE",
                "message": str(exc),
                "analytics": None,
            }
        
        # 4. Iterate candles
        has_position = False
        buy_candle_index = None
        
        for i in range(1, len(df)):  # Start from 1 to have previous values
            session.current_index = i
            
            current = self._get_indicator_snapshot(indicator_values, i)
            previous = self._get_indicator_snapshot(indicator_values, i - 1)
            
            close_price = float(df.iloc[i]["close"])
            
            if not has_position:
                # Check entry signals
                if self._evaluate_rules(strategy.entry_rules, current, previous):
                    # BUY
                    try:
                        quantity = self._calculate_quantity(
                            strategy.position_sizing, session, close_price
                        )
                        from app.schemas.decision_schema import DecisionCreate
                        from app.domain.enums import DecisionAction
                        
                        decision = DecisionCreate(
                            action=DecisionAction.BUY,
                            price=close_price,
                            quantity=quantity,
                            order_type="MARKET_AT_CLOSE"
                        )
                        TradeLifecycleService.process_decision(db, session.id, decision)
                        has_position = True
                        buy_candle_index = i
                    except Exception as e:
                        import traceback
                        return {
                            "status": "failed",
                            "error_code": "TRADE_EXECUTION_FAILED",
                            "message": str(e),
                            "bar_index": i,
                            "analytics": None,
                        }
            
            else:
                # Check T+2 first
                if i - buy_candle_index < 2:
                    continue  # Can't sell yet
                
                # Check exit signals
                if self._evaluate_rules(strategy.exit_rules, current, previous):
                    # SELL
                    try:
                        from app.schemas.decision_schema import DecisionCreate
                        from app.domain.enums import DecisionAction
                        
                        decision = DecisionCreate(
                            action=DecisionAction.CLOSE,
                            order_type="MARKET_AT_CLOSE"
                        )
                        TradeLifecycleService.process_decision(db, session.id, decision)
                        has_position = False
                        buy_candle_index = None
                    except Exception as e:
                        import traceback
                        return {
                            "status": "failed",
                            "error_code": "TRADE_EXECUTION_FAILED",
                            "message": str(e),
                            "bar_index": i,
                            "analytics": None,
                        }
        
        # 5. Return analytics
        analytics = AnalyticsService.get_analytics(db, session.id)
        slices = self._build_result_slices(
            db=db,
            session_id=session.id,
            benchmark_symbol=benchmark_symbol,
            start_date=start_date,
            end_date=end_date,
        )
        
        return {
            "session_id": session.id,
            "status": "succeeded",
            "strategy": strategy.name,
            "symbol": symbol,
            "total_candles": len(candles),
            "analytics": analytics.model_dump() if analytics else None,
            "slices": slices,
        }
    
    def _compute_indicators(self, df: pd.DataFrame, indicators_config) -> dict:
        results = {}
        for ind in indicators_config:
            name = ind.name
            itype = ind.type.lower()
            length = ind.length
            
            if itype == "sma":
                results[name] = df["close"].rolling(window=length).mean().values
            elif itype == "ema":
                results[name] = df["close"].ewm(span=length, adjust=False).mean().values
            # Add more if needed
        return results

    def _get_indicator_snapshot(self, indicator_values: dict, index: int) -> dict:
        snapshot = {}
        for k, arr in indicator_values.items():
            val = arr[index]
            snapshot[k] = float(val) if not np.isnan(val) else None
        return snapshot

    def _validate_strategy_rules(self, strategy, indicator_names: set[str]) -> None:
        allowed_names = set(indicator_names)
        allowed_names.update(f"previous_{name}" for name in indicator_names)
        for rule in [*strategy.entry_rules, *strategy.exit_rules]:
            condition = rule.get("condition")
            if condition:
                validate_condition(condition, allowed_names)
            dsl_rule = rule.get("dsl") or self._extract_inline_dsl(rule)
            if dsl_rule:
                validate_rule_dsl(dsl_rule, allowed_names)
        
    def _evaluate_rules(self, rules, current, previous) -> bool:
        """
        Evaluate rule conditions.
        Tất cả rules phải TRUE → signal TRUE.
        """
        if not rules:
            return False
            
        for rule in rules:
            condition = rule.get("condition")
            dsl_rule = rule.get("dsl") or self._extract_inline_dsl(rule)
            
            values = dict(current)
            values.update({f"previous_{key}": val for key, val in previous.items()})
            if condition and not evaluate_condition(condition, values):
                return False
            if dsl_rule and not evaluate_rule_dsl(dsl_rule, values):
                return False
        
        return True

    def _extract_inline_dsl(self, rule: dict) -> dict | None:
        operators = {
            "all", "any", "not", "gt", "gte", "lt", "lte", "eq",
            "cross_up", "cross_down", "between", "rising", "falling",
        }
        inline_keys = [key for key in rule.keys() if key in operators]
        if not inline_keys:
            return None
        if len(inline_keys) > 1:
            raise RuleEvaluationError("Rule must contain only one inline DSL operator.")
        key = inline_keys[0]
        return {key: rule[key]}

    def _build_result_slices(
        self,
        db: Session,
        session_id: int,
        benchmark_symbol: str | None = "VNINDEX",
        start_date=None,
        end_date=None,
    ) -> list[dict]:
        trades = db.query(Trade).filter(
            Trade.session_id == session_id,
            Trade.exit_date.isnot(None)
        ).all()
        regime_map = self._build_regime_map(db, benchmark_symbol, start_date, end_date)
        return [
            *self._slice_trades(trades, "symbol", lambda trade: trade.symbol),
            *self._slice_trades(
                trades,
                "period",
                lambda trade: str(trade.exit_date.year) if trade.exit_date else "Unknown",
            ),
            *self._slice_trades(
                trades,
                "regime",
                lambda trade: self._lookup_trade_regime(trade, regime_map),
            ),
        ]

    def _build_regime_map(self, db: Session, benchmark_symbol: str | None, start_date, end_date) -> dict:
        if not benchmark_symbol:
            return {}

        query = db.query(Candle).filter(Candle.symbol == benchmark_symbol)
        if start_date:
            query = query.filter(Candle.timestamp >= start_date)
        if end_date:
            query = query.filter(Candle.timestamp <= end_date)

        benchmark_candles = query.order_by(Candle.timestamp).all()
        return RegimeClassifier.build_regime_map(benchmark_candles) if benchmark_candles else {}

    def _lookup_trade_regime(self, trade: Trade, regime_map: dict) -> str:
        if not regime_map or not trade.exit_date:
            return "Unknown"
        trade_date = trade.exit_date.date() if hasattr(trade.exit_date, "date") else trade.exit_date
        return regime_map.get(trade_date, "Unknown")

    def _slice_trades(self, trades: list[Trade], group_type: str, key_fn) -> list[dict]:
        grouped = {}
        for trade in trades:
            key = key_fn(trade) or "Unknown"
            if key not in grouped:
                grouped[key] = []
            grouped[key].append(trade.net_pnl or 0.0)

        rows = []
        for key, pnl_values in grouped.items():
            trades_count = len(pnl_values)
            wins = len([pnl for pnl in pnl_values if pnl > 0])
            net_pnl = sum(pnl_values)
            rows.append({
                "group_type": group_type,
                "key": key,
                "trades": trades_count,
                "win_rate": wins / trades_count if trades_count else 0.0,
                "net_pnl": round(net_pnl, 2),
                "average_pnl": round(net_pnl / trades_count, 2) if trades_count else 0.0,
                "best_trade": round(max(pnl_values), 2) if pnl_values else None,
                "worst_trade": round(min(pnl_values), 2) if pnl_values else None,
            })

        return sorted(rows, key=lambda row: (row["group_type"], row["key"]))

    def _calculate_quantity(self, sizing, session, price):
        if sizing.method == "fixed_quantity":
            return sizing.quantity
        elif sizing.method == "percent_equity":
            if sizing.percent:
                amount = session.current_cash * (sizing.percent / 100.0)
                # Need to account for fees approximately
                fee_rate = 0.0015
                max_buyable = int((amount / (1 + fee_rate)) / price)
                # Round down to nearest 100 for Vietnam stocks
                return (max_buyable // 100) * 100
        return 0
