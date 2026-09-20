"""Unit and integration tests for Master 9 benchmark metrics and cross-phase degradation.

Covers:
- FR-CORE-010: Authoritative Master 9 benchmark metrics calculation.
- TEST-MET-001: Golden trade ledger verifying exact outputs without intermediate rounding.
- TEST-MET-002: Edge case handling (0 trades, 0 winners, 0 losers) with None raw representation and 'N/A' presentation semantics.
- Master Spec Section 22: Break-even winner convention (return_pct >= 0), negative loss sign preservation, raw precision.
- Master Spec Section 23: 2D PhaseMetricMatrix assembly and portfolio aggregation.
- Master Spec Section 24: Cross-phase degradation metrics (deltas, ratios, degradation %, stability score).
- Master Spec Section 25: Deterministic Markdown table rendering and CSV export.
- Integration: BatchBacktestRunner and POST /api/backtest/batch/run integration.
"""

from datetime import date, datetime, timedelta
from typing import List, Optional
import pytest
import pandas as pd
from fastapi.testclient import TestClient

from app.domain.backtest.batch_runner import BatchBacktestRunner, PhaseDefinition
from app.domain.backtest.metrics import (
    BenchmarkMetricRow,
    BenchmarkTableRenderer,
    CrossPhaseComparison,
    PhaseMetricMatrix,
    build_phase_metric_matrix,
    calculate_benchmark_metrics,
    compare_phases,
)
from app.domain.backtest.models import BacktestTrade
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.models.candle import Candle


# ============================================================================
# Helpers & Synthetic Data
# ============================================================================

def _make_closed_trade(
    trade_id: str,
    symbol: str,
    entry_bar: int,
    exit_bar: int,
    net_pnl: float,
    pnl_percent: float,
    gross_pnl: Optional[float] = None,
    entry_price: float = 20.0,
    exit_price: float = 22.0,
    quantity: float = 1000.0,
) -> BacktestTrade:
    """Create a closed BacktestTrade with specified accounting metrics."""
    return BacktestTrade(
        id=trade_id,
        symbol=symbol,
        entry_order_id=f"entry_{trade_id}",
        exit_order_id=f"exit_{trade_id}",
        entry_time=datetime(2024, 1, 1) + timedelta(days=entry_bar),
        exit_time=datetime(2024, 1, 1) + timedelta(days=exit_bar),
        entry_bar_index=entry_bar,
        exit_bar_index=exit_bar,
        holding_bars=exit_bar - entry_bar,
        quantity=quantity,
        entry_price=entry_price,
        exit_price=exit_price,
        gross_pnl=gross_pnl if gross_pnl is not None else net_pnl,
        net_pnl=net_pnl,
        fees=10.0,
        taxes=10.0,
        pnl_percent=pnl_percent,
        status="closed",
        result="win" if pnl_percent >= 0 else "loss",
    )


def _make_open_trade(
    trade_id: str,
    symbol: str,
    entry_bar: int,
    quantity: float = 500.0,
    entry_price: float = 30.0,
) -> BacktestTrade:
    """Create an open BacktestTrade (not closed at cutoff)."""
    return BacktestTrade(
        id=trade_id,
        symbol=symbol,
        entry_order_id=f"entry_{trade_id}",
        exit_order_id=None,
        entry_time=datetime(2024, 1, 1) + timedelta(days=entry_bar),
        exit_time=None,
        entry_bar_index=entry_bar,
        exit_bar_index=None,
        holding_bars=10,
        quantity=quantity,
        entry_price=entry_price,
        exit_price=None,
        gross_pnl=1500.0,
        net_pnl=1500.0,
        pnl_percent=10.0,
        status="open",
        result="open",
    )


def _get_test_strategy_dict() -> dict:
    """Standard strategy dictionary for testing."""
    return {
        "name": "Phase Crossover Strategy",
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": 5},
            {"name": "sma_slow", "type": "sma", "length": 15},
        ],
        "entry_rules": [
            {"condition": "sma_fast > sma_slow"},
        ],
        "exit_rules": [
            {"condition": "sma_fast < sma_slow"},
        ],
        "position_sizing": {
            "method": "fixed_quantity",
            "quantity": 1000,
        },
    }


# ============================================================================
# 1. TEST-MET-001: Golden Trade Ledger & Exact Master 9 Metrics
# ============================================================================

def test_golden_trade_ledger_exact_nine_metrics():
    """Verify exact calculation of Master 9 benchmark metrics without intermediate rounding.
    
    Validates:
    - Net Profit: sum of net_pnl across closed trades.
    - % Net Profit: 100 * net_profit / initial_cash.
    - # Trades: count of closed trades (open trades excluded).
    - Avg % Profit/Loss: arithmetic mean of closed trade pnl_percent.
    - Avg Bars Held: arithmetic mean of exit_bar - entry_bar.
    - % of Winners: count(return_pct >= 0) / count(closed_trades) * 100.
    - W. Avg % Profit: arithmetic mean of winning trade returns.
    - L. Avg % Loss: arithmetic mean of losing trade returns (PRESERVING NEGATIVE SIGN).
    """
    initial_cash = 100_000_000.0

    # 4 closed trades + 1 open trade:
    # Trade 1: +10,000,000 (+10.0%), bars = 5
    # Trade 2:  -5,000,000  (-5.0%), bars = 2
    # Trade 3:  +5,000,000 (+10.0%), bars = 3
    # Trade 4:           0   (0.0%), bars = 4 (Break-even: counts as winner!)
    # Trade 5: open trade, must be excluded from closed trade metrics
    trades = [
        _make_closed_trade("T1", "HPG", entry_bar=0, exit_bar=5, net_pnl=10_000_000.0, pnl_percent=10.0, gross_pnl=10_050_000.0),
        _make_closed_trade("T2", "HPG", entry_bar=10, exit_bar=12, net_pnl=-5_000_000.0, pnl_percent=-5.0, gross_pnl=-4_950_000.0),
        _make_closed_trade("T3", "HPG", entry_bar=15, exit_bar=18, net_pnl=5_000_000.0, pnl_percent=10.0, gross_pnl=5_050_000.0),
        _make_closed_trade("T4", "HPG", entry_bar=20, exit_bar=24, net_pnl=0.0, pnl_percent=0.0, gross_pnl=50_000.0),
        _make_open_trade("T5", "HPG", entry_bar=25),
    ]

    metrics = calculate_benchmark_metrics(
        trades=trades,
        initial_cash=initial_cash,
        ticker="HPG",
        phase_name="In-Sample",
        start_date="2024-01-01",
        end_date="2024-03-31",
        final_cash=110_000_000.0,
        final_equity=115_000_000.0,
        open_position_quantity=500.0,
        open_position_value=5_000_000.0,
    )

    # 1. Ticker & Phase
    assert metrics.ticker == "HPG"
    assert metrics.phase_name == "In-Sample"

    # 2. Net Profit: 10M - 5M + 5M + 0 = 10,000,000.0
    assert metrics.net_profit == 10_000_000.0

    # 3. % Net Profit: 10M / 100M = 10.0%
    assert metrics.net_profit_pct == 10.0

    # 4. # Trades: 4 (T5 is open, excluded)
    assert metrics.num_trades == 4

    # 5. Avg % Profit/Loss: (10.0 - 5.0 + 10.0 + 0.0) / 4 = 15.0 / 4 = 3.75%
    assert metrics.avg_profit_loss_pct == 3.75

    # 6. Avg Bars Held: (5 + 2 + 3 + 4) / 4 = 14 / 4 = 3.5
    assert metrics.avg_bars_held == 3.5

    # 7. % of Winners: 3 winners (T1, T3, and break-even T4) out of 4 = 75.0%
    assert metrics.num_winners == 3
    assert metrics.num_breakeven == 1
    assert metrics.num_losers == 1
    assert metrics.win_rate_pct == 75.0

    # 8. W. Avg % Profit: (10.0 + 10.0 + 0.0) / 3 = 20.0 / 3 = 6.666666666666667
    assert pytest.approx(metrics.win_avg_profit_pct, rel=1e-6) == 20.0 / 3.0

    # 9. L. Avg % Loss: -5.0 (PRESERVING NEGATIVE SIGN!)
    assert metrics.loss_avg_loss_pct == -5.0

    # Verify open position tracking
    assert metrics.open_position_quantity == 500.0
    assert metrics.open_position_value == 5_000_000.0
    assert metrics.final_equity == 115_000_000.0


# ============================================================================
# 2. TEST-MET-002: Edge Cases & N/A Presentation Semantics
# ============================================================================

def test_edge_cases_zero_trades():
    """Verify zero closed trades produces None raw metrics and 'N/A' presentation."""
    metrics = calculate_benchmark_metrics(
        trades=[],
        initial_cash=100_000_000.0,
        ticker="SSI",
        phase_name="Test Phase",
    )

    assert metrics.num_trades == 0
    assert metrics.net_profit == 0.0
    assert metrics.net_profit_pct == 0.0
    assert metrics.avg_profit_loss_pct is None
    assert metrics.avg_bars_held is None
    assert metrics.win_rate_pct is None
    assert metrics.win_avg_profit_pct is None
    assert metrics.loss_avg_loss_pct is None
    assert metrics.profit_factor is None

    # Check renderer formats None as 'N/A'
    formatted = BenchmarkTableRenderer.format_row_dict(metrics)
    assert formatted["# Trades"] == "0"
    assert formatted["Avg % Profit/Loss"] == "N/A"
    assert formatted["Avg Bars Held"] == "N/A"
    assert formatted["% of Winners"] == "N/A"
    assert formatted["W. Avg % Profit"] == "N/A"
    assert formatted["L. Avg % Loss"] == "N/A"


def test_edge_cases_all_winners_zero_losers():
    """Verify 100% win rate results in None for loss metrics and 'N/A' in table."""
    trades = [
        _make_closed_trade("T1", "VNM", 0, 3, 2_000_000.0, 5.0),
        _make_closed_trade("T2", "VNM", 5, 8, 4_000_000.0, 10.0),
    ]
    metrics = calculate_benchmark_metrics(
        trades=trades,
        initial_cash=100_000_000.0,
        ticker="VNM",
    )

    assert metrics.num_trades == 2
    assert metrics.num_winners == 2
    assert metrics.num_losers == 0
    assert metrics.win_rate_pct == 100.0
    assert metrics.win_avg_profit_pct == 7.5
    assert metrics.loss_avg_loss_pct is None

    formatted = BenchmarkTableRenderer.format_row_dict(metrics)
    assert formatted["% of Winners"] == "100.0%"
    assert formatted["W. Avg % Profit"] == "7.5%"
    assert formatted["L. Avg % Loss"] == "N/A"


def test_edge_cases_all_losers_zero_winners():
    """Verify 0% win rate results in None for winner metrics and 'N/A' in table."""
    trades = [
        _make_closed_trade("T1", "VNM", 0, 3, -2_000_000.0, -4.0),
        _make_closed_trade("T2", "VNM", 5, 8, -3_000_000.0, -6.0),
    ]
    metrics = calculate_benchmark_metrics(
        trades=trades,
        initial_cash=100_000_000.0,
        ticker="VNM",
    )

    assert metrics.num_trades == 2
    assert metrics.num_winners == 0
    assert metrics.num_losers == 2
    assert metrics.win_rate_pct == 0.0
    assert metrics.win_avg_profit_pct is None
    assert metrics.loss_avg_loss_pct == -5.0

    formatted = BenchmarkTableRenderer.format_row_dict(metrics)
    assert formatted["% of Winners"] == "0.0%"
    assert formatted["W. Avg % Profit"] == "N/A"
    assert formatted["L. Avg % Loss"] == "-5.0%"


def test_break_even_convention():
    """Verify break-even trades (return == 0.0) strictly count as winners."""
    trades = [
        _make_closed_trade("T1", "FPT", 0, 2, 0.0, 0.0),
    ]
    metrics = calculate_benchmark_metrics(
        trades=trades,
        initial_cash=100_000_000.0,
        ticker="FPT",
    )

    assert metrics.num_trades == 1
    assert metrics.num_winners == 1
    assert metrics.num_losers == 0
    assert metrics.win_rate_pct == 100.0
    assert metrics.win_avg_profit_pct == 0.0
    assert metrics.loss_avg_loss_pct is None


# ============================================================================
# 3. Cross-Phase Degradation & PhaseMetricMatrix
# ============================================================================

def test_compare_phases_degradation_calculation():
    """Verify cross-phase comparison metrics: deltas, ratios, degradation %."""
    # In-Sample row
    is_row = BenchmarkMetricRow(
        ticker="HPG",
        phase_name="In-Sample",
        initial_cash=100_000_000.0,
        final_cash=120_000_000.0,
        final_equity=120_000_000.0,
        net_profit=20_000_000.0,
        net_profit_pct=20.0,
        num_trades=10,
        avg_profit_loss_pct=2.0,
        avg_bars_held=5.0,
        win_rate_pct=60.0,
        win_avg_profit_pct=5.0,
        loss_avg_loss_pct=-2.5,
        profit_factor=2.0,
    )

    # Out-of-Sample row with reduced performance
    oos_row = BenchmarkMetricRow(
        ticker="HPG",
        phase_name="Out-of-Sample",
        initial_cash=100_000_000.0,
        final_cash=110_000_000.0,
        final_equity=110_000_000.0,
        net_profit=10_000_000.0,
        net_profit_pct=10.0,
        num_trades=8,
        avg_profit_loss_pct=1.25,
        avg_bars_held=4.5,
        win_rate_pct=50.0,
        win_avg_profit_pct=4.0,
        loss_avg_loss_pct=-3.0,
        profit_factor=1.5,
    )

    comp = compare_phases(is_row, oos_row)

    assert comp.base_phase_name == "In-Sample"
    assert comp.target_phase_name == "Out-of-Sample"
    assert comp.ticker == "HPG"

    # Return: IS=20%, OOS=10% -> delta = -10.0, ratio = 0.5, degradation = -50.0%
    assert comp.return_delta_pct == -10.0
    assert comp.profit_ratio == 0.5
    assert comp.degradation_pct == -50.0
    assert comp.is_degraded is True

    # Win rate: IS=60%, OOS=50% -> delta = -10.0
    assert comp.win_rate_delta_pct == -10.0


def test_build_phase_metric_matrix_portfolio_aggregation():
    """Verify 2D PhaseMetricMatrix aggregates portfolio rows correctly."""
    row_hpg_is = BenchmarkMetricRow(
        ticker="HPG", phase_name="IS", initial_cash=100_000_000.0,
        final_cash=110_000_000.0, final_equity=110_000_000.0,
        net_profit=10_000_000.0, net_profit_pct=10.0, num_trades=5,
        avg_profit_loss_pct=2.0, avg_bars_held=4.0, win_rate_pct=60.0,
        win_avg_profit_pct=5.0, loss_avg_loss_pct=-2.5, num_winners=3, num_losers=2
    )
    row_ssi_is = BenchmarkMetricRow(
        ticker="SSI", phase_name="IS", initial_cash=100_000_000.0,
        final_cash=105_000_000.0, final_equity=105_000_000.0,
        net_profit=5_000_000.0, net_profit_pct=5.0, num_trades=5,
        avg_profit_loss_pct=1.0, avg_bars_held=6.0, win_rate_pct=40.0,
        win_avg_profit_pct=4.0, loss_avg_loss_pct=-1.0, num_winners=2, num_losers=3
    )

    row_hpg_oos = BenchmarkMetricRow(
        ticker="HPG", phase_name="OOS", initial_cash=100_000_000.0,
        final_cash=108_000_000.0, final_equity=108_000_000.0,
        net_profit=8_000_000.0, net_profit_pct=8.0, num_trades=4,
        avg_profit_loss_pct=2.0, avg_bars_held=4.0, win_rate_pct=50.0,
        win_avg_profit_pct=4.0, loss_avg_loss_pct=-2.0, num_winners=2, num_losers=2
    )
    row_ssi_oos = BenchmarkMetricRow(
        ticker="SSI", phase_name="OOS", initial_cash=100_000_000.0,
        final_cash=102_000_000.0, final_equity=102_000_000.0,
        net_profit=2_000_000.0, net_profit_pct=2.0, num_trades=4,
        avg_profit_loss_pct=0.5, avg_bars_held=5.0, win_rate_pct=50.0,
        win_avg_profit_pct=2.0, loss_avg_loss_pct=-1.0, num_winners=2, num_losers=2
    )

    all_metrics = [row_hpg_is, row_ssi_is, row_hpg_oos, row_ssi_oos]
    matrix = build_phase_metric_matrix(["IS", "OOS"], all_metrics)

    assert matrix.phase_names == ["IS", "OOS"]
    assert sorted(matrix.symbols) == ["HPG", "SSI"]
    assert "IS" in matrix.portfolio_by_phase
    assert "OOS" in matrix.portfolio_by_phase

    # Check Portfolio row for IS phase:
    # Total initial capital = 200,000,000.0
    # Total net profit = 15,000,000.0
    # Net profit % = 7.5%
    # Total trades = 10
    port_is = matrix.portfolio_by_phase["IS"]
    assert port_is.ticker == "PORTFOLIO"
    assert port_is.initial_cash == 200_000_000.0
    assert port_is.net_profit == 15_000_000.0
    assert port_is.net_profit_pct == 7.5
    assert port_is.num_trades == 10

    # Cross-phase degradations should have comparisons for HPG, SSI, and PORTFOLIO
    assert len(matrix.cross_phase_degradations) == 3
    port_deg = next(d for d in matrix.cross_phase_degradations if d.ticker == "PORTFOLIO")
    # Portfolio IS net_profit_pct = 7.5%, OOS net_profit = 10M / 200M = 5.0%
    assert port_deg.return_delta_pct == 5.0 - 7.5  # -2.5%


# ============================================================================
# 4. Master Spec Section 25 Table & CSV Rendering
# ============================================================================

def test_benchmark_table_renderer_markdown():
    """Verify Master Section 25.1 Markdown table syntax, headers, and rows."""
    row = BenchmarkMetricRow(
        ticker="HPG",
        phase_name="Bull Phase",
        initial_cash=100_000_000.0,
        final_cash=110_000_000.0,
        final_equity=110_000_000.0,
        net_profit=10_000_000.0,
        net_profit_pct=10.0,
        num_trades=5,
        avg_profit_loss_pct=2.0,
        avg_bars_held=4.2,
        win_rate_pct=60.0,
        win_avg_profit_pct=5.0,
        loss_avg_loss_pct=-2.5,
    )

    table_md = BenchmarkTableRenderer.render_markdown_table([row])

    # Check header exists
    assert "| Ticker | Net Profit | % Net Profit | # Trades | Avg % Profit/Loss | Avg Bars Held | % of Winners | W. Avg % Profit | L. Avg % Loss |" in table_md
    # Check row content
    assert "| HPG | 10,000,000 | 10.0% | 5 | 2.0% | 4.2 | 60.0% | 5.0% | -2.5% |" in table_md


def test_benchmark_table_renderer_csv():
    """Verify deterministic CSV export generation."""
    row = BenchmarkMetricRow(
        ticker="HPG",
        phase_name="Bull Phase",
        initial_cash=100_000_000.0,
        final_cash=110_000_000.0,
        final_equity=110_000_000.0,
        net_profit=10_000_000.0,
        net_profit_pct=10.0,
        num_trades=5,
        avg_profit_loss_pct=2.0,
        avg_bars_held=4.0,
        win_rate_pct=60.0,
        win_avg_profit_pct=5.0,
        loss_avg_loss_pct=-2.5,
    )

    csv_text = BenchmarkTableRenderer.render_csv([row], formatted=True)
    lines = [line.strip() for line in csv_text.strip().splitlines() if line.strip()]
    assert len(lines) == 2
    assert lines[0] == "Ticker,Net Profit,% Net Profit,# Trades,Avg % Profit/Loss,Avg Bars Held,% of Winners,W. Avg % Profit,L. Avg % Loss"
    assert "HPG" in lines[1]
    assert "10.0%" in lines[1]


# ============================================================================
# 5. Integration: BatchBacktestRunner with Metrics & API Validation
# ============================================================================

def test_batch_runner_metrics_integration(db_session):
    """Verify BatchBacktestRunner populates metric_matrix, degradations, markdown_table, and csv_export."""
    base_date = date(2024, 1, 1)
    candles = []
    # 60 candles with price swings to generate trades
    for i in range(60):
        candles.append(Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=20.0 + (i % 8) * 1.0,
            high=22.0 + (i % 8) * 1.0,
            low=19.0 + (i % 8) * 1.0,
            close=21.0 + (i % 8) * 1.0,
            volume=1_000_000,
        ))
    db_session.add_all(candles)
    db_session.commit()

    def provider(sym: str, start: str, end: str) -> pd.DataFrame:
        c_list = db_session.query(Candle).filter(Candle.symbol == sym).order_by(Candle.timestamp).all()
        return pd.DataFrame([{
            "timestamp": c.timestamp, "open": c.open, "high": c.high,
            "low": c.low, "close": c.close, "volume": c.volume
        } for c in c_list])

    strategy = load_strategy_from_dict(_get_test_strategy_dict())
    phases = [
        PhaseDefinition(name="Phase 1", start_date="2024-01-10", end_date="2024-01-30"),
        PhaseDefinition(name="Phase 2", start_date="2024-01-30", end_date="2024-02-20"),
    ]

    result = BatchBacktestRunner.run_batch(
        strategy=strategy,
        symbols=["HPG"],
        phases=phases,
        candle_provider=provider,
        initial_cash_per_run=100_000_000.0,
    )

    assert result.status in ("succeeded", "partial")
    assert result.metric_matrix is not None
    assert result.metric_matrix.phase_names == ["Phase 1", "Phase 2"]
    assert "HPG" in result.metric_matrix.rows_by_symbol
    assert "Phase 1" in result.metric_matrix.rows_by_symbol["HPG"]
    assert "Phase 2" in result.metric_matrix.rows_by_symbol["HPG"]

    # Each phase result must carry benchmark_metrics
    for r in result.phase_results:
        assert r.benchmark_metrics is not None
        assert isinstance(r.benchmark_metrics, BenchmarkMetricRow)
        assert r.benchmark_metrics.ticker == "HPG"

    # Cross phase comparisons must exist
    assert len(result.cross_phase_degradations) > 0
    assert result.cross_phase_degradations[0].base_phase_name == "Phase 1"
    assert result.cross_phase_degradations[0].target_phase_name == "Phase 2"

    # Markdown table and CSV exports must be rendered
    assert result.markdown_table is not None
    assert "| Ticker | Net Profit |" in result.markdown_table
    assert result.csv_export is not None
    assert "Ticker,Net Profit" in result.csv_export


def test_api_batch_run_with_metrics(client, db_session):
    """Verify POST /api/backtest/batch/run returns full metrics matrix and exports."""
    base_date = date(2024, 1, 1)
    candles = []
    for i in range(50):
        candles.append(Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=base_date + timedelta(days=i),
            open=25.0 + (i % 6),
            high=26.0 + (i % 6),
            low=24.0 + (i % 6),
            close=25.5 + (i % 6),
            volume=500_000,
        ))
    db_session.add_all(candles)
    db_session.commit()

    payload = {
        "symbols": ["HPG"],
        "phases": [
            {"name": "In-Sample", "start_date": "2024-01-10", "end_date": "2024-01-25"},
            {"name": "Out-of-Sample", "start_date": "2024-01-25", "end_date": "2024-02-10"},
        ],
        "strategy": _get_test_strategy_dict(),
        "initial_cash": 100_000_000.0,
    }

    response = client.post("/api/backtest/batch/run", json=payload)
    assert response.status_code == 200
    data = response.json()

    assert data["status"] == "succeeded"
    assert "metric_matrix" in data and data["metric_matrix"] is not None
    assert data["metric_matrix"]["phase_names"] == ["In-Sample", "Out-of-Sample"]
    assert "HPG" in data["metric_matrix"]["rows_by_symbol"]
    assert "In-Sample" in data["metric_matrix"]["portfolio_by_phase"]
    assert data["metric_matrix"]["portfolio_by_phase"]["In-Sample"]["ticker"] == "PORTFOLIO"

    assert "cross_phase_degradations" in data and data["cross_phase_degradations"] is not None
    assert len(data["cross_phase_degradations"]) > 0

    assert "markdown_table" in data and data["markdown_table"] is not None
    assert "| Ticker | Net Profit |" in data["markdown_table"]
    assert "csv_export" in data and data["csv_export"] is not None
    assert "Ticker,Net Profit" in data["csv_export"]

    # Verify benchmark_metrics inside phase_results
    for pr in data["phase_results"]:
        assert "benchmark_metrics" in pr and pr["benchmark_metrics"] is not None
        assert pr["benchmark_metrics"]["ticker"] == "HPG"
        assert "net_profit" in pr["benchmark_metrics"]
        assert "net_profit_pct" in pr["benchmark_metrics"]
        assert "num_trades" in pr["benchmark_metrics"]
        assert "avg_profit_loss_pct" in pr["benchmark_metrics"]
        assert "avg_bars_held" in pr["benchmark_metrics"]
        assert "win_rate_pct" in pr["benchmark_metrics"]
        assert "win_avg_profit_pct" in pr["benchmark_metrics"]
        assert "loss_avg_loss_pct" in pr["benchmark_metrics"]
