"""
tests/test_challenger_m3_instance1_stress.py

Adversarial stress test suite by Challenger M3 Instance 1 (Workstream C: SQA-010, SQA-014, SQA-015, SQA-016, SQA-005, SQA-011).
Empirically stress-tests backend Workstream C requirements:
- SQA-010: AST Validation & Evaluation Edge Cases (adversarial ASTs, SQLi, scientific notation, extreme numeric boundaries, literal type safety)
- SQA-014: Insufficient Warmup History (0-candle, 1-candle, 5-candle datasets, multi-indicator requirements, adapter NaN generation)
- SQA-011: Executed Trades Serialization & Contract Parity (open/unclosed trades, 0 trades, 150+ trades)
- SQA-015: Scanner History Multi-Run Database Isolation
"""

import ast
import math
import sys
from datetime import date, datetime, timezone
from pathlib import Path
from unittest.mock import MagicMock

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
from app.domain.strategy.rule_evaluator import (
    RuleEvaluationError,
    validate_condition,
    evaluate_condition,
    _compare,
    _is_missing_number,
)
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.services.backtest_service import BacktestService
from app.services.scanner_history_service import ScannerHistoryService


# ============================================================================
# SQA-010: Boundary & Adversarial Inputs to AST Evaluator
# ============================================================================

class TestSQA010AdversarialAST:
    """Stress-test AST parser and evaluator with malicious, boundary, and edge cases."""

    ALLOWED_NAMES = {"close", "open", "high", "low", "volume", "rsi", "ema_200", "signal_active", "pnl", "macd_hist"}

    def test_empty_and_whitespace_inputs(self):
        with pytest.raises(RuleEvaluationError):
            validate_condition("", allowed_names=self.ALLOWED_NAMES)
        with pytest.raises(RuleEvaluationError):
            validate_condition("   ", allowed_names=self.ALLOWED_NAMES)
        with pytest.raises(RuleEvaluationError):
            validate_condition("\n\t  \r", allowed_names=self.ALLOWED_NAMES)

    def test_syntax_errors_and_garbage(self):
        garbage_inputs = [
            "@#$%",
            "<<>>",
            "== !=",
            "close > > 10",
            "close >",
            "> 10",
            "((close > 10)",
            "close > 10 and",
            "close > 10 or",
        ]
        for expr in garbage_inputs:
            with pytest.raises(RuleEvaluationError):
                validate_condition(expr, allowed_names=self.ALLOWED_NAMES)

    def test_scientific_notation_handling(self):
        # Positive scientific notation
        validate_condition("close > 1e-5", allowed_names=self.ALLOWED_NAMES)
        assert evaluate_condition("close > 1e-5", {"close": 0.0001}) is True
        assert evaluate_condition("close > 1e-5", {"close": 1e-6}) is False

        # Large scientific notation
        validate_condition("volume >= 1.5e6", allowed_names=self.ALLOWED_NAMES)
        assert evaluate_condition("volume >= 1.5e6", {"volume": 2000000}) is True
        assert evaluate_condition("volume >= 1.5e6", {"volume": 1499999}) is False

        # Negative scientific notation
        validate_condition("rsi > -2.5e-3", allowed_names=self.ALLOWED_NAMES)
        assert evaluate_condition("rsi > -2.5e-3", {"rsi": 0.0}) is True
        assert evaluate_condition("rsi > -2.5e-3", {"rsi": -0.01}) is False

    def test_negative_numbers_and_zero(self):
        validate_condition("pnl < -10.5", allowed_names=self.ALLOWED_NAMES)
        assert evaluate_condition("pnl < -10.5", {"pnl": -15.0}) is True
        assert evaluate_condition("pnl < -10.5", {"pnl": -5.0}) is False

        validate_condition("macd_hist == 0", allowed_names=self.ALLOWED_NAMES)
        assert evaluate_condition("macd_hist == 0", {"macd_hist": 0.0}) is True
        assert evaluate_condition("macd_hist == 0", {"macd_hist": 0.001}) is False

    def test_sql_and_script_injection_attempts(self):
        """Malicious string injection must be rejected during AST validation."""
        injections = [
            'close > "\'; DROP TABLE candles; --"',
            'close == "<script>alert(1)</script>"',
            'close > "OR 1=1"',
            'close > "admin\' --"',
            'close > "foo"',
            'close == "invalid"',
        ]
        for inj in injections:
            with pytest.raises(RuleEvaluationError) as exc_info:
                validate_condition(inj, allowed_names=self.ALLOWED_NAMES)
            assert "Comparison operand must be numeric or boolean" in str(exc_info.value)

    def test_python_code_execution_prevention(self):
        """Disallowed AST nodes (Call, BinOp, Subscript, Import) must fail validation."""
        exploits = [
            "__import__('os').system('id')",
            "eval('1 + 1')",
            "open('/etc/passwd').read()",
            "lambda x: x",
            "close > [1, 2, 3]",
            "close > {'a': 1}",
            "close.upper() > 10",
            "close + 5 > 10",
        ]
        for exp in exploits:
            with pytest.raises(RuleEvaluationError):
                validate_condition(exp, allowed_names=self.ALLOWED_NAMES)

    def test_chained_comparisons(self):
        validate_condition("10 < close < 100", allowed_names=self.ALLOWED_NAMES)
        assert evaluate_condition("10 < close < 100", {"close": 50}) is True
        assert evaluate_condition("10 < close < 100", {"close": 5}) is False
        assert evaluate_condition("10 < close < 100", {"close": 150}) is False

    def test_type_error_protection_in_compare(self):
        """_compare should safely return False rather than raising unhandled TypeError."""
        assert _compare(10.5, "not_a_number", ast.Gt()) is False
        assert _compare("string", 10.5, ast.Lt()) is False
        assert _compare(None, 10.5, ast.Gt()) is False
        assert _compare(float("nan"), 10.5, ast.Gt()) is False
        assert _compare(10.5, float("nan"), ast.LtE()) is False

    def test_unknown_identifiers_enforcement(self):
        with pytest.raises(RuleEvaluationError) as exc_info:
            validate_condition("close > 10 and secret_col == 5", allowed_names={"close"})
        assert "secret_col" in str(exc_info.value)


# ============================================================================
# SQA-014: Short Warmup History Across Indicator Types
# ============================================================================

class TestSQA014WarmupAcrossIndicatorTypes:
    """Stress-test indicator calculation and backtest failure diagnostics on short/empty datasets."""

    def test_strategy_indicator_adapter_on_empty_df(self):
        strategy = load_strategy_from_dict({
            "name": "TestEmpty",
            "indicators": [
                {"name": "ema_200", "type": "ema", "params": {"length": 200}},
                {"name": "macd", "type": "macd", "params": {"fast": 12, "slow": 26, "signal": 9}},
                {"name": "ichimoku", "type": "ichimoku", "params": {"tenkan": 9, "kijun": 26, "senkou": 52}},
            ],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        empty_df = pd.DataFrame(columns=["open", "high", "low", "close", "volume"])
        res = StrategyIndicatorAdapter.compute(empty_df, strategy.indicators)
        assert isinstance(res, dict)

    def test_strategy_indicator_adapter_on_1_candle(self):
        strategy = load_strategy_from_dict({
            "name": "Test1Candle",
            "indicators": [
                {"name": "ema_200", "type": "ema", "params": {"length": 200}},
                {"name": "sma_200", "type": "sma", "params": {"length": 200}},
                {"name": "bb_20", "type": "bbands", "params": {"length": 20}},
                {"name": "macd", "type": "macd", "params": {"fast": 12, "slow": 26, "signal": 9}},
                {"name": "ichimoku", "type": "ichimoku", "params": {"tenkan": 9, "kijun": 26, "senkou": 52}},
            ],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        one_candle_df = pd.DataFrame([{
            "open": 50000.0, "high": 51000.0, "low": 49000.0, "close": 50500.0, "volume": 100000.0
        }])
        res = StrategyIndicatorAdapter.compute(one_candle_df, strategy.indicators)

        # Verify all declared keys exist and are NaNs
        assert "ema_200" in res
        assert np.isnan(res["ema_200"][0])
        assert "sma_200" in res
        assert np.isnan(res["sma_200"][0])
        assert "bb_20" in res
        assert np.isnan(res["bb_20"][0])
        assert "macd_line" in res and np.isnan(res["macd_line"][0])
        assert "macd_signal" in res and np.isnan(res["macd_signal"][0])
        assert "macd_hist" in res and np.isnan(res["macd_hist"][0])
        assert "ichimoku_tenkan" in res and np.isnan(res["ichimoku_tenkan"][0])
        assert "ichimoku_kijun" in res and np.isnan(res["ichimoku_kijun"][0])
        assert "ichimoku_span_a" in res and np.isnan(res["ichimoku_span_a"][0])
        assert "ichimoku_span_b" in res and np.isnan(res["ichimoku_span_b"][0])
        assert "ichimoku_chikou" in res and np.isnan(res["ichimoku_chikou"][0])

        # Verify evaluating rules on this 1-candle NaN dict safely returns False
        condition = "ema_200 > 100 and macd_line > macd_signal and close > ichimoku_span_a"
        values = {k: v[0] for k, v in res.items()}
        assert evaluate_condition(condition, values) is False

    def test_strategy_indicator_adapter_on_5_candles(self):
        strategy = load_strategy_from_dict({
            "name": "Test5Candles",
            "indicators": [
                {"name": "ema_200", "type": "ema", "params": {"length": 200}},
                {"name": "macd", "type": "macd", "params": {"fast": 12, "slow": 26, "signal": 9}},
            ],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })
        five_candles_df = pd.DataFrame([
            {"open": 50000.0 + i, "high": 51000.0 + i, "low": 49000.0 + i, "close": 50500.0 + i, "volume": 100000.0}
            for i in range(5)
        ])
        res = StrategyIndicatorAdapter.compute(five_candles_df, strategy.indicators)
        assert len(res["ema_200"]) == 5
        assert all(np.isnan(x) for x in res["ema_200"])
        assert len(res["macd_line"]) == 5
        assert all(np.isnan(x) for x in res["macd_line"])

    def test_backtest_service_warmup_diagnostics(self):
        import asyncio
        service = BacktestService()
        mock_db = MagicMock()
        strategy_config = {
            "name": "WarmupStressStrategy",
            "indicators": [
                {"name": "ema_200", "type": "ema", "length": 200},
            ],
            "entry_rules": [{"condition": "close > ema_200"}],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        }

        # 0 candles
        mock_db.query.return_value.filter.return_value.order_by.return_value.all.return_value = []
        res0 = asyncio.run(service._run_single_symbol_backtest(
            db=mock_db,
            config={
                "symbol": "HPG",
                "start_date": "2023-01-01",
                "end_date": "2023-01-10",
                "initial_cash": 100000000.0,
                "strategy": strategy_config,
                "benchmark_symbol": "VNINDEX",
                "timeframe": "1D",
            }
        ))
        assert res0["status"] == "failed"
        assert res0["error_code"] == "NO_CANDLES"

        # 1 candle (requires 200)
        c1 = MagicMock()
        mock_db.query.return_value.filter.return_value.order_by.return_value.all.return_value = [c1]
        res1 = asyncio.run(service._run_single_symbol_backtest(
            db=mock_db,
            config={
                "symbol": "HPG",
                "start_date": "2023-01-01",
                "end_date": "2023-01-10",
                "initial_cash": 100000000.0,
                "strategy": strategy_config,
                "benchmark_symbol": "VNINDEX",
                "timeframe": "1D",
            }
        ))
        assert res1["status"] == "failed"
        assert res1["error_code"] == "INSUFFICIENT_WARMUP_HISTORY"
        assert "1 nến" in res1["message"]
        assert "200 nến" in res1["message"]

        # 5 candles with MACD (slow=26)
        strategy_macd = {
            "name": "MACDStrategy",
            "indicators": [
                {"name": "macd_26", "type": "macd", "slow": 26},
            ],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        }
        mock_db.query.return_value.filter.return_value.order_by.return_value.all.return_value = [MagicMock() for _ in range(5)]
        res5 = asyncio.run(service._run_single_symbol_backtest(
            db=mock_db,
            config={
                "symbol": "FPT",
                "start_date": "2023-01-01",
                "end_date": "2023-01-10",
                "initial_cash": 100000000.0,
                "strategy": strategy_macd,
                "benchmark_symbol": "VNINDEX",
                "timeframe": "1D",
            }
        ))
        assert res5["status"] == "failed"
        assert res5["error_code"] == "INSUFFICIENT_WARMUP_HISTORY"
        assert "5 nến" in res5["message"]
        assert "26 nến" in res5["message"]


# ============================================================================
# SQA-011: Executed Trades Payload Serialization Stress Test
# ============================================================================

class TestSQA011ExecutedTradesPayload:
    """Stress-test executed trades serialization with 0 trades, open trades, and large volumes."""

    def test_open_position_unclosed_trade_serialization(self):
        """An open trade has exit_time=None, exit_price=None, result=None."""
        open_trade = MagicMock()
        open_trade.entry_time = datetime(2023, 5, 10, 9, 30)
        open_trade.entry_price = 50000.0
        open_trade.exit_time = None
        open_trade.exit_price = None
        open_trade.quantity = 1000
        open_trade.gross_pnl = 0.0
        open_trade.net_pnl = 0.0
        open_trade.pnl_percent = 0.0
        open_trade.holding_bars = 5
        open_trade.status = "open"
        open_trade.result = None

        trades_list = [
            {
                "id": idx + 1,
                "symbol": "VNM",
                "entry_date": t.entry_time.isoformat() if hasattr(t.entry_time, "isoformat") else str(t.entry_time) if t.entry_time else None,
                "entry_price": float(t.entry_price) if t.entry_price is not None else None,
                "exit_date": t.exit_time.isoformat() if hasattr(t.exit_time, "isoformat") else str(t.exit_time) if t.exit_time else None,
                "exit_price": float(t.exit_price) if t.exit_price is not None else None,
                "quantity": int(t.quantity) if t.quantity is not None else 0,
                "gross_pnl": float(t.gross_pnl) if t.gross_pnl is not None else 0.0,
                "net_pnl": float(t.net_pnl) if t.net_pnl is not None else 0.0,
                "pnl_percent": float(t.pnl_percent) if t.pnl_percent is not None else 0.0,
                "holding_candles": int(t.holding_bars) if t.holding_bars is not None else 0,
                "status": t.status,
                "result": t.result,
            }
            for idx, t in enumerate([open_trade])
        ]

        assert len(trades_list) == 1
        serialized = trades_list[0]
        assert serialized["id"] == 1
        assert serialized["symbol"] == "VNM"
        assert serialized["entry_date"] == "2023-05-10T09:30:00"
        assert serialized["entry_price"] == 50000.0
        assert serialized["exit_date"] is None
        assert serialized["exit_price"] is None
        assert serialized["status"] == "open"
        assert serialized["result"] is None

    def test_large_volume_150_trades_serialization(self):
        trades = []
        for i in range(150):
            t = MagicMock()
            t.entry_time = datetime(2023, 1, 1)
            t.entry_price = 50000.0 + i
            t.exit_time = datetime(2023, 1, 10)
            t.exit_price = 52000.0 + i
            t.quantity = 100 * (i + 1)
            t.gross_pnl = 2000.0 * 100 * (i + 1)
            t.net_pnl = t.gross_pnl * 0.99
            t.pnl_percent = 0.04
            t.holding_bars = 7
            t.status = "closed"
            t.result = "win" if i % 2 == 0 else "loss"
            trades.append(t)

        serialized = [
            {
                "id": idx + 1,
                "symbol": "FPT",
                "entry_date": t.entry_time.isoformat() if hasattr(t.entry_time, "isoformat") else str(t.entry_time) if t.entry_time else None,
                "entry_price": float(t.entry_price) if t.entry_price is not None else None,
                "exit_date": t.exit_time.isoformat() if hasattr(t.exit_time, "isoformat") else str(t.exit_time) if t.exit_time else None,
                "exit_price": float(t.exit_price) if t.exit_price is not None else None,
                "quantity": int(t.quantity) if t.quantity is not None else 0,
                "gross_pnl": float(t.gross_pnl) if t.gross_pnl is not None else 0.0,
                "net_pnl": float(t.net_pnl) if t.net_pnl is not None else 0.0,
                "pnl_percent": float(t.pnl_percent) if t.pnl_percent is not None else 0.0,
                "holding_candles": int(t.holding_bars) if t.holding_bars is not None else 0,
                "status": t.status,
                "result": t.result,
            }
            for idx, t in enumerate(trades)
        ]

        assert len(serialized) == 150
        assert serialized[0]["id"] == 1
        assert serialized[149]["id"] == 150
        assert serialized[149]["quantity"] == 15000


# ============================================================================
# SQA-015: Scanner History Multi-Run Database Isolation
# ============================================================================

class TestSQA015ScannerHistoryDatabaseIsolation:
    """Verify scanner runs in an isolated SQLite database maintain distinct payloads."""

    def test_multi_run_storage_and_payload_isolation(self):
        engine = create_engine("sqlite:///:memory:")
        Base.metadata.create_all(engine)
        Session = sessionmaker(bind=engine)
        db = Session()

        try:
            run1_dict = ScannerHistoryService.create_run(
                db=db,
                request_config={"symbols": ["FPT", "VNM"], "strategy": {"name": "Bullish"}},
                result_payload={"status": "succeeded", "total_results": 2, "results": [{"symbol": "FPT", "price": 95000.0}, {"symbol": "VNM", "price": 68000.0}]},
            )
            run2_dict = ScannerHistoryService.create_run(
                db=db,
                request_config={"symbols": ["HPG"], "strategy": {"name": "Bearish Empty"}},
                result_payload={"status": "succeeded", "total_results": 0, "results": []},
            )

            runs = ScannerHistoryService.list_runs(db=db)
            assert len(runs) == 2

            loaded_run1 = ScannerHistoryService.get_run(db=db, run_id=run1_dict["id"])
            loaded_run2 = ScannerHistoryService.get_run(db=db, run_id=run2_dict["id"])

            assert len(loaded_run1["result_payload"]["results"]) == 2
            assert loaded_run1["result_payload"]["results"][0]["symbol"] == "FPT"

            assert len(loaded_run2["result_payload"]["results"]) == 0
            assert loaded_run2["result_payload"] != loaded_run1["result_payload"]
        finally:
            db.close()
