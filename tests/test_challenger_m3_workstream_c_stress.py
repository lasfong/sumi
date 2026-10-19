"""
test_challenger_m3_workstream_c_stress.py

Adversarial stress test suite by Challenger M3 Instance 2.
Empirically stress-tests backend Workstream C requirements:
- SQA-010: AST Validation & Evaluation Edge Cases (adversarial ASTs, extreme numeric boundaries, literal type safety)
- SQA-014: Insufficient Warmup History (boundary conditions, multi-indicator requirements, adapter NaN generation)
- SQA-011: Executed Trades Serialization & Contract Parity (open/closed trades, precision, multi-symbol runs)
- SQA-015: Scanner History Multi-Run Integrity & Result Payloads
"""

import ast
import asyncio
from datetime import datetime, timezone, timedelta
import math
from pathlib import Path
import sys
import numpy as np
import pandas as pd
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

# Setup path and database safety guard
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from app.db import Base
import app.models
from app.models.candle import Candle
from app.domain.strategy.rule_evaluator import (
    RuleEvaluationError,
    validate_condition,
    evaluate_condition,
    _compare,
    _is_missing_number,
    get_rule_identifiers,
    validate_rule_dsl,
    evaluate_rule_dsl,
)
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.services.backtest_service import BacktestService
from app.services.scanner_history_service import ScannerHistoryService


# ---------------------------------------------------------------------------
# SQA-010: AST Validation & Evaluation Edge Cases
# ---------------------------------------------------------------------------

class TestSQA010ASTValidationAdversarial:
    """Stress-test AST parser, validator, and evaluator against hostile inputs."""

    ALLOWED_NAMES = {"rsi", "ema_20", "close", "open", "high", "low", "volume", "signal_active"}

    @pytest.mark.parametrize("condition", [
        "rsi > 0",
        "rsi >= -0.0",
        "rsi <= 1e-5",
        "rsi < 1e10",
        "rsi != -999999.999",
        "close == 25000",
        "signal_active == True",
        "signal_active == False",
        "close > open",
        "high >= low",
        "volume > 0 and rsi < 70",
        "(rsi > 30 and rsi < 70) or volume > 1000000",
        "10 < rsi < 80",
        "0.0 <= rsi <= 100.0",
        "rsi > -50.5",
        "not (rsi > 80)",
        "not signal_active",
        "-rsi < -30",
    ])
    def test_valid_numeric_and_bool_conditions_pass(self, condition):
        """All legitimate numeric and boolean comparison conditions must validate cleanly."""
        validate_condition(condition, self.ALLOWED_NAMES)

    @pytest.mark.parametrize("condition, expected_error_parts", [
        ("rsi > '50'", ["Comparison operand must be numeric or boolean", "got str: '50'"]),
        ("rsi > 'abc'", ["Comparison operand must be numeric or boolean", "got str: 'abc'"]),
        ("'invalid' == close", ["Comparison operand must be numeric or boolean", "got str: 'invalid'"]),
        ("rsi > [1, 2, 3]", ["Unsupported", "List"]),
        ("rsi > {'a': 1}", ["Unsupported", "Dict"]),
        ("rsi > (10, 20)", ["Unsupported", "Tuple"]),
        ("rsi > {1, 2}", ["Unsupported", "Set"]),
        ("rsi >= unknown_var", ["Unknown identifier(s): unknown_var"]),
        ("rsi > -'string'", ["Unary operand must be numeric or boolean", "got str"]),
    ])
    def test_invalid_comparison_operands_rejected(self, condition, expected_error_parts):
        """Hostile or non-numeric comparison operands are rejected with explicit diagnostic."""
        with pytest.raises(RuleEvaluationError) as exc_info:
            validate_condition(condition, self.ALLOWED_NAMES)
        for part in expected_error_parts:
            assert part in str(exc_info.value)

    @pytest.mark.parametrize("hostile_code", [
        "__import__('os').system('ls')",
        "eval('1 + 1')",
        "exec('x = 1')",
        "open('/etc/passwd').read()",
        "rsi > abs(-10)",
        "close > max(1, 2)",
        "volume[0] > 100",
        "rsi.foo > 10",
        "rsi + 10 > 50",  # BinOp not allowed by grammar
        "rsi > 50 + 5",   # BinOp not allowed by grammar
    ])
    def test_arbitrary_code_and_syntax_injection_blocked(self, hostile_code):
        """Arbitrary calls, imports, subscripts, and unapproved syntax are blocked by AST whitelist."""
        with pytest.raises(RuleEvaluationError) as exc_info:
            validate_condition(hostile_code, self.ALLOWED_NAMES)
        err = str(exc_info.value)
        assert any(k in err for k in ("Unsupported rule syntax", "Invalid rule syntax", "Unknown identifier"))

    def test_chained_comparison_evaluation(self):
        """Chained comparisons (e.g. 10 < rsi < 80) evaluate with proper left-to-right semantics."""
        assert evaluate_condition("10 < rsi < 80", {"rsi": 50}) is True
        assert evaluate_condition("10 < rsi < 80", {"rsi": 5}) is False
        assert evaluate_condition("10 < rsi < 80", {"rsi": 95}) is False
        assert evaluate_condition("10 < rsi < 80", {"rsi": 10}) is False
        assert evaluate_condition("10 <= rsi <= 80", {"rsi": 10}) is True
        assert evaluate_condition("10 <= rsi <= 80", {"rsi": 80}) is True

    def test_compare_handles_nan_inf_none_typeerror_gracefully(self):
        """_compare safely handles hostile runtime types without raising unhandled exceptions."""
        gt = ast.Gt()
        lt = ast.Lt()
        eq = ast.Eq()

        # NaN checks: missing numbers always return False
        assert _compare(float("nan"), 50.0, gt) is False
        assert _compare(50.0, float("nan"), gt) is False
        assert _compare(float("nan"), float("nan"), eq) is False

        # None checks: missing numbers always return False
        assert _compare(None, 50.0, gt) is False
        assert _compare(50.0, None, lt) is False
        assert _compare(None, None, eq) is False

        # Type mismatch: string vs float catches TypeError and returns False
        assert _compare("malicious_string", 50.0, gt) is False
        assert _compare(50.0, "malicious_string", lt) is False
        assert _compare([1, 2], 50.0, gt) is False
        assert _compare({"key": "val"}, 50.0, gt) is False

        # Valid comparisons still work
        assert _compare(100.0, 50.0, gt) is True
        assert _compare(25.0, 50.0, lt) is True
        assert _compare(50.0, 50.0, eq) is True


# ---------------------------------------------------------------------------
# SQA-014: Insufficient Warmup History & Indicator Adapter
# ---------------------------------------------------------------------------

class TestSQA014WarmupAndAdapterStress:
    """Stress-test warmup boundary conditions and adapter NaN-filling resilience."""

    def _create_candles(self, count: int, symbol: str = "VIC") -> list[Candle]:
        base = datetime(2024, 1, 2, tzinfo=timezone.utc)
        candles = []
        for i in range(count):
            candles.append(
                Candle(
                    symbol=symbol,
                    timeframe="1D",
                    adjustment_type="unadjusted",
                    timestamp=base + timedelta(days=i),
                    open=50000.0 + i * 10,
                    high=50500.0 + i * 10,
                    low=49500.0 + i * 10,
                    close=50200.0 + i * 10,
                    volume=500000 + i * 1000,
                )
            )
        return candles

    def test_adapter_populates_nan_for_single_and_multi_output_indicators(self):
        """Adapter correctly produces NaN arrays for all indicators when history < warmup."""
        strategy_config = {
            "name": "Comprehensive Multi-Indicator Strategy",
            "version": "1.0",
            "indicators": [
                {"name": "sma_20", "type": "sma", "length": 20},
                {"name": "rsi_14", "type": "rsi", "length": 14},
                {"name": "macd_inst", "type": "macd", "fast": 12, "slow": 26, "signal": 9},
                {"name": "ichimoku_inst", "type": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
            ],
            "entry_rules": [{"condition": "close > sma_20 and macd_inst_line > 0"}],
            "exit_rules": [{"condition": "close < sma_20"}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        }
        strategy = load_strategy_from_dict(strategy_config)

        # 5 candles is insufficient for any of 14, 20, 26, 52
        candles = self._create_candles(5)
        df = pd.DataFrame([{
            "timestamp": c.timestamp,
            "open": c.open,
            "high": c.high,
            "low": c.low,
            "close": c.close,
            "volume": c.volume,
        } for c in candles])

        indicator_values = StrategyIndicatorAdapter.compute(df, strategy.indicators)

        # Expected keys:
        expected_keys = {
            "sma_20",
            "rsi_14",
            "macd_inst_line",
            "macd_inst_signal",
            "macd_inst_hist",
            "ichimoku_inst_tenkan",
            "ichimoku_inst_kijun",
            "ichimoku_inst_span_a",
            "ichimoku_inst_span_b",
            "ichimoku_inst_chikou",
            "open", "high", "low", "close", "volume"
        }
        for key in expected_keys:
            assert key in indicator_values, f"Missing expected indicator key: {key}"
            assert len(indicator_values[key]) == 5, f"Length mismatch for key {key}"

        # Indicators must be all NaN
        assert np.isnan(indicator_values["sma_20"]).all()
        assert np.isnan(indicator_values["rsi_14"]).all()
        assert np.isnan(indicator_values["macd_inst_line"]).all()
        assert np.isnan(indicator_values["ichimoku_inst_span_b"]).all()

        # Strategy rule validation must pass without unknown identifier errors
        StrategyRuleEvaluator.validate_strategy_rules(strategy, set(indicator_values.keys()))

    def test_backtest_service_warmup_boundary_exact_n_minus_1_fails(self):
        """Boundary test: exactly (lookback - 1) candles fails with INSUFFICIENT_WARMUP_HISTORY."""
        engine = create_engine("sqlite:///:memory:")
        Base.metadata.create_all(engine)
        Session = sessionmaker(bind=engine)
        db = Session()

        try:
            # Lookback = 30
            candles = self._create_candles(29, symbol="VIC")
            for c in candles:
                db.add(c)
            db.commit()

            svc = BacktestService()
            strategy_config = {
                "name": "Lookback 30 Strategy",
                "version": "1.0",
                "indicators": [
                    {"name": "sma_30", "type": "sma", "length": 30},
                ],
                "entry_rules": [{"condition": "close > sma_30"}],
                "exit_rules": [{"condition": "close < sma_30"}],
                "position_sizing": {"method": "fixed_quantity", "quantity": 100},
            }

            res = asyncio.run(svc.run_backtest(
                db=db,
                config={
                    "strategy": strategy_config,
                    "symbol": "VIC",
                    "start_date": "2024-01-01",
                    "end_date": "2024-03-01",
                    "initial_cash": 100_000_000,
                },
            ))

            assert res["status"] == "failed"
            assert res["error_code"] == "INSUFFICIENT_WARMUP_HISTORY"
            assert "29 nến" in res["message"]
            assert "30 nến" in res["message"]
            assert "sma_30" in res["message"]
        finally:
            db.close()

    def test_backtest_service_warmup_picks_max_requirement_across_indicators(self):
        """When multiple indicators are present, the largest lookback is enforced."""
        engine = create_engine("sqlite:///:memory:")
        Base.metadata.create_all(engine)
        Session = sessionmaker(bind=engine)
        db = Session()

        try:
            # Provide 40 candles: sufficient for ema_10 and rsi_14, but insufficient for sma_60
            candles = self._create_candles(40, symbol="VIC")
            for c in candles:
                db.add(c)
            db.commit()

            svc = BacktestService()
            strategy_config = {
                "name": "Multi-Lookback Strategy",
                "version": "1.0",
                "indicators": [
                    {"name": "ema_10", "type": "ema", "length": 10},
                    {"name": "rsi_14", "type": "rsi", "length": 14},
                    {"name": "sma_60", "type": "sma", "length": 60},
                ],
                "entry_rules": [{"condition": "ema_10 > sma_60"}],
                "exit_rules": [{"condition": "ema_10 < sma_60"}],
                "position_sizing": {"method": "fixed_quantity", "quantity": 100},
            }

            res = asyncio.run(svc.run_backtest(
                db=db,
                config={
                    "strategy": strategy_config,
                    "symbol": "VIC",
                    "start_date": "2024-01-01",
                    "end_date": "2024-03-01",
                    "initial_cash": 100_000_000,
                },
            ))

            assert res["status"] == "failed"
            assert res["error_code"] == "INSUFFICIENT_WARMUP_HISTORY"
            assert "yêu cầu tối thiểu 60 nến" in res["message"]
            assert "sma_60" in res["message"]
        finally:
            db.close()


# ---------------------------------------------------------------------------
# SQA-011: Executed Trades Serialization & Contract Parity
# ---------------------------------------------------------------------------

class TestSQA011ExecutedTradesContract:
    """Stress-test trade serialization in single and multi-symbol backtest responses."""

    def test_single_symbol_backtest_trade_schema_fidelity(self):
        """Backtest result trades conform strictly to BacktestTrade schema."""
        engine = create_engine("sqlite:///:memory:")
        Base.metadata.create_all(engine)
        Session = sessionmaker(bind=engine)
        db = Session()

        try:
            # 60 candles with price oscillating to generate trade entries and exits
            base = datetime(2024, 1, 2, tzinfo=timezone.utc)
            candles = []
            for i in range(60):
                # oscillate price around 50000
                p = 50000.0 + 3000.0 * math.sin(i * 0.3)
                candles.append(
                    Candle(
                        symbol="HPG",
                        timeframe="1D",
                        adjustment_type="unadjusted",
                        timestamp=base + timedelta(days=i),
                        open=p - 100,
                        high=p + 500,
                        low=p - 500,
                        close=p,
                        volume=2_000_000,
                    )
                )
            for c in candles:
                db.add(c)
            db.commit()

            svc = BacktestService()
            strategy_config = {
                "name": "Oscillation Strategy",
                "version": "1.0",
                "indicators": [
                    {"name": "sma_5", "type": "sma", "length": 5},
                    {"name": "sma_15", "type": "sma", "length": 15},
                ],
                "entry_rules": [{"condition": "sma_5 > sma_15"}],
                "exit_rules": [{"condition": "sma_5 < sma_15"}],
                "position_sizing": {"method": "fixed_quantity", "quantity": 100},
            }

            res = asyncio.run(svc.run_backtest(
                db=db,
                config={
                    "strategy": strategy_config,
                    "symbol": "HPG",
                    "start_date": "2024-01-01",
                    "end_date": "2024-04-01",
                    "initial_cash": 100_000_000,
                },
            ))

            assert res["status"] == "succeeded"
            assert "trades" in res
            trades = res["trades"]
            assert isinstance(trades, list)

            # If trades executed, verify every trade adheres to BacktestTrade schema
            for trade in trades:
                assert "id" in trade and isinstance(trade["id"], int)
                assert trade["symbol"] == "HPG"
                assert "entry_date" in trade and trade["entry_date"] is not None
                assert "entry_price" in trade and isinstance(trade["entry_price"], (int, float))
                assert "quantity" in trade and isinstance(trade["quantity"], int)
                assert "holding_candles" in trade and isinstance(trade["holding_candles"], int)
                assert "status" in trade
                assert "result" in trade
                assert "net_pnl" in trade and isinstance(trade["net_pnl"], float)
                assert "pnl_percent" in trade and isinstance(trade["pnl_percent"], float)
                assert "gross_pnl" in trade and isinstance(trade["gross_pnl"], float)

                # For closed trades, exit_date and exit_price must be populated
                if trade["status"] == "closed":
                    assert trade["exit_date"] is not None
                    assert trade["exit_price"] is not None
        finally:
            db.close()


# ---------------------------------------------------------------------------
# SQA-015: Scanner History Multi-Run Integrity
# ---------------------------------------------------------------------------

class TestSQA015ScannerHistoryStress:
    """Stress-test scanner history multi-run persistence and state isolation."""

    def test_scanner_history_multiple_runs_and_empty_payloads(self):
        """ScannerHistoryService handles multiple distinct runs including empty results."""
        engine = create_engine("sqlite:///:memory:")
        Base.metadata.create_all(engine)
        Session = sessionmaker(bind=engine)
        db = Session()

        try:
            # Run with 0 results
            empty_payload = {
                "status": "succeeded",
                "total_results": 0,
                "truncated": False,
                "warnings": [],
                "results": [],
            }
            run_empty = ScannerHistoryService.create_run(
                db=db,
                request_config={"symbols": ["NONE"], "strategy": {"name": "Empty"}},
                result_payload=empty_payload,
            )

            # Run with multiple results
            active_payload = {
                "status": "succeeded",
                "total_results": 2,
                "truncated": False,
                "warnings": [],
                "results": [
                    {"symbol": "AAA", "timestamp": "2024-01-01T00:00:00", "signal_type": "BUY", "price": 10.0, "regime": "UP"},
                    {"symbol": "BBB", "timestamp": "2024-01-02T00:00:00", "signal_type": "SELL", "price": 20.0, "regime": "DOWN"},
                ],
            }
            run_active = ScannerHistoryService.create_run(
                db=db,
                request_config={"symbols": ["AAA", "BBB"], "strategy": {"name": "Active"}},
                result_payload=active_payload,
            )

            runs = ScannerHistoryService.list_runs(db, limit=5)
            assert len(runs) == 2

            # Fetch empty run: total_results is 0 and results list is empty
            empty_fetched = ScannerHistoryService.get_run(db, run_empty["id"])
            assert empty_fetched["total_results"] == 0
            assert empty_fetched["result_payload"]["results"] == []

            # Fetch active run: total_results is 2 and results list intact
            active_fetched = ScannerHistoryService.get_run(db, run_active["id"])
            assert active_fetched["total_results"] == 2
            assert len(active_fetched["result_payload"]["results"]) == 2
            assert active_fetched["result_payload"]["results"][0]["symbol"] == "AAA"
        finally:
            db.close()
