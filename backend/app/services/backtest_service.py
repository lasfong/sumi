import pandas as pd
import hashlib
import json
import math
from datetime import timezone
from datetime import datetime
from sqlalchemy.orm import Session
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.services.analytics_service import AnalyticsService
from app.models.replay_session import ReplaySession
from app.models.candle import Candle
from app.models.trade import Trade
from app.domain.regime.regime_classifier import RegimeClassifier
import uuid
from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.utils.date_range import end_before, start_at
from app.domain.accounting import BUY_FEE_RATE, SELL_FEE_RATE, SELL_TAX_RATE
from app.schemas.analytics_trust_schema import DataCoverage, ExecutionAssumptions, RunManifest

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
            Candle.timestamp >= start_at(start_date),
            Candle.timestamp < end_before(end_date)
        ).order_by(Candle.timestamp).all()
        
        if len(candles) == 0:
            return {
                "status": "failed",
                "error_code": "NO_CANDLES",
                "message": f"No candles found for {symbol}",
                "symbol": symbol,
                "total_candles": 0,
                "analytics": None,
                "data_coverage": DataCoverage(
                    requested_start=str(start_date), requested_end=str(end_date),
                    symbols_requested=[symbol], excluded_data=[f"No candles found for {symbol}"],
                ).model_dump(),
            }

        coverage = self._build_coverage(candles, symbol, start_date, end_date, strategy)
        assumptions = self._build_assumptions(strategy)
        manifest = self._build_manifest(
            strategy, symbol, candles, assumptions,
            initial_cash=initial_cash,
            benchmark_symbol=benchmark_symbol,
        )
        
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
        session.source_payload = json.dumps({"benchmark_symbol": benchmark_symbol})
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
        
        indicator_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)
        try:
            StrategyRuleEvaluator.validate_strategy_rules(strategy, set(indicator_values.keys()))
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
            
            current = StrategyRuleEvaluator.indicator_snapshot(indicator_values, i)
            previous = StrategyRuleEvaluator.indicator_snapshot(indicator_values, i - 1)
            
            close_price = float(df.iloc[i]["close"])
            
            if not has_position:
                # Check entry signals
                if StrategyRuleEvaluator.evaluate_rules(strategy.entry_rules, current, previous):
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
                if StrategyRuleEvaluator.evaluate_rules(strategy.exit_rules, current, previous):
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
        
        # Persist the final analyzed boundary. Trade lifecycle commits at fills,
        # but a run can continue beyond its last fill; Analytics must reproduce
        # the same periodic-return sample after this request/session is reopened.
        db.add(session)
        db.commit()
        db.refresh(session)

        # 5. Return analytics
        analytics = AnalyticsService.get_analytics(db, session.id, benchmark_symbol=benchmark_symbol)
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
            "data_coverage": coverage.model_dump(),
            "execution_assumptions": assumptions.model_dump(),
            "run_manifest": manifest.model_dump(mode="json"),
        }

    def _build_coverage(self, candles, symbol, start_date, end_date, strategy) -> DataCoverage:
        timestamps = [c.timestamp for c in candles]
        warmup = self._estimate_warmup(strategy, candles)
        gaps = []
        for previous, current in zip(timestamps, timestamps[1:]):
            delta = (current - previous).days if hasattr(current - previous, "days") else 0
            if delta > 4:
                gaps.append(f"{previous}..{current} ({delta - 1} calendar days not represented)")
        return DataCoverage(
            requested_start=str(start_date), requested_end=str(end_date),
            actual_start=str(timestamps[0]), actual_end=str(timestamps[-1]),
            symbols_requested=[symbol], symbols_covered=[symbol], candle_count=len(candles),
            warmup_candles=min(warmup, len(candles)), gaps=gaps,
            excluded_data=[] if warmup == 0 else [f"{warmup} warm-up candle(s) excluded from signal evaluation"],
        )

    def _estimate_warmup(self, strategy, candles) -> int:
        if not strategy.indicators or not candles:
            return 0
        frame = pd.DataFrame([{
            "timestamp": candle.timestamp,
            "open": candle.open,
            "high": candle.high,
            "low": candle.low,
            "close": candle.close,
            "volume": candle.volume,
        } for candle in candles])
        outputs = StrategyIndicatorAdapter.compute(frame, strategy.indicators)
        first_finite_indexes = []
        for values in outputs.values():
            first_finite = next((
                index for index, value in enumerate(values)
                if value is not None and pd.notna(value) and bool(pd.api.types.is_number(value)) and math.isfinite(float(value))
            ), len(candles))
            first_finite_indexes.append(first_finite)
        return max(first_finite_indexes, default=0)

    def _build_assumptions(self, strategy) -> ExecutionAssumptions:
        return ExecutionAssumptions(
            fees={"buy_rate": BUY_FEE_RATE, "sell_rate": SELL_FEE_RATE},
            taxes={"sell_tax_rate": SELL_TAX_RATE},
            position_sizing=strategy.position_sizing.model_dump(),
        )

    def _build_manifest(
        self, strategy, symbol, candles, assumptions,
        *, initial_cash: float, benchmark_symbol: str | None,
    ) -> RunManifest:
        strategy_payload = strategy.model_dump(mode="json")
        data_hasher = hashlib.sha256(json.dumps({
            "symbol": symbol,
            "timeframe": "1D",
            "adjustment_type": str(candles[0].adjustment_type),
            "first": str(candles[0].timestamp),
            "last": str(candles[-1].timestamp),
            "count": len(candles),
        }, sort_keys=True).encode())
        for candle in candles:
            data_hasher.update(
                "|".join(str(value) for value in (
                    candle.timestamp, candle.open, candle.high, candle.low,
                    candle.close, candle.volume, candle.timeframe, candle.adjustment_type,
                )).encode()
            )
        data_identity = data_hasher.hexdigest()
        assumptions_payload = assumptions.model_dump(mode="json")
        assumptions_identity = hashlib.sha256(json.dumps(assumptions_payload, sort_keys=True).encode()).hexdigest()
        input_hash = hashlib.sha256(json.dumps({
            "strategy": strategy_payload,
            "data_identity": data_identity,
            "assumptions_identity": assumptions_identity,
            "initial_cash": float(initial_cash),
            "benchmark_symbol": benchmark_symbol,
        }, sort_keys=True).encode()).hexdigest()
        return RunManifest(
            strategy_name=strategy.name,
            strategy_version=strategy.version,
            strategy_parameters=strategy_payload,
            data_identity=data_identity,
            assumptions_identity=assumptions_identity,
            run_timestamp=datetime.now(timezone.utc),
            input_hash=input_hash,
        )
    
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
            query = query.filter(Candle.timestamp >= start_at(start_date))
        if end_date:
            query = query.filter(Candle.timestamp < end_before(end_date))

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
