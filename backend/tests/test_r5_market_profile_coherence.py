"""Regression & Parity Test Suite for Requirement R5: MarketProfile Fee, Tax, Slippage Coherence.

Covers:
- Dynamic inheritance of fee_buy, fee_sell, tax_sell, slippage, min_holding_bars, and sizing_basis from MarketProfile.
- Slippage applied symmetrically to BUY fill price (Open * (1 + slippage)) and SELL fill price (Open * (1 - slippage)).
- Asymmetric fee rates: fee_buy applied to BUY cash flows/records, fee_sell applied to SELL cash flows/records.
- Tax rate: tax_sell applied to SELL cash flows/records.
- Strict penny-for-penny equality between BacktestExecutionKernel ledger cash delta and sum of trade net_pnl:
    ledger.current_cash - ledger.initial_cash == sum(t.net_pnl for t in ledger.trades)
- Strict equality between trade net_pnl and compute_round_trip_pnl(profile=active_profile):
    trade.net_pnl == compute_round_trip_pnl(...).net_pnl == trade.gross_pnl - trade.fees - trade.taxes
- Constructor explicit overrides take precedence over profile attributes.
- Default Vietnam market profile behavior remains 100% backward compatible (Golden Case G-01 exact match).

All tests operate in-memory or on isolated temporary SQLite databases.
Production backend/sumi.db is NEVER accessed or mutated.
"""

from __future__ import annotations

import math
from datetime import date, datetime, timedelta
from types import SimpleNamespace
from typing import Any, Dict, Generator, List

import numpy as np
import pandas as pd
import pytest

from app.domain.accounting import compute_round_trip_pnl
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.market.calendar import TradingCalendar
from app.domain.market.provider import MarketProfile as ProviderMarketProfile, MarketRuleProvider
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE, MarketProfile as RulesMarketProfile, MarketRules
from app.domain.market.settlement import SettlementEngine


def make_strategy_ns(
    name: str = "test_strategy",
    stop_loss: float | None = None,
    take_profit: float | None = None,
    sizing_method: str = "fixed_quantity",
    sizing_qty: int = 100,
    sizing_pct: float = 100.0,
) -> SimpleNamespace:
    return SimpleNamespace(
        name=name,
        indicators=[
            {"id": "fast_ma", "kind": "sma", "params": {"length": 5}},
            {"id": "slow_ma", "kind": "sma", "params": {"length": 10}},
        ],
        entry_rules=[{"dsl": {"gt": ["fast_ma", "slow_ma"]}}],
        exit_rules=[{"dsl": {"lt": ["fast_ma", "slow_ma"]}}],
        position_sizing=SimpleNamespace(
            method=sizing_method,
            quantity=sizing_qty,
            percent=sizing_pct,
            model_dump=lambda: {"method": sizing_method, "quantity": sizing_qty, "percent": sizing_pct},
        ),
        risk_management=SimpleNamespace(
            stop_loss_percent=stop_loss,
            take_profit_percent=take_profit,
        ) if (stop_loss or take_profit) else None,
    )


def create_15_bar_sample_df(start_date: date = date(2024, 1, 8)) -> pd.DataFrame:
    """Generate 15 sequential daily trading bars (Mon-Fri) for deterministic execution tests."""
    calendar = TradingCalendar()
    dates: List[pd.Timestamp] = []
    cur = start_date
    while len(dates) < 15:
        if calendar.is_trading_day(cur):
            dates.append(pd.Timestamp(cur))
        cur += timedelta(days=1)

    # 15 sequential daily bars:
    # Bar 5 close: fast_ma crosses above slow_ma -> Buy signal queued for Bar 6 Open
    # Bar 6 open: 100.0 -> Buy order fills at 100.0 * (1 + slippage)
    # Bar 10 close: fast_ma crosses below slow_ma -> Exit signal queued for Bar 11 Open
    # Bar 11 open: 110.0 -> Exit order fills at 110.0 * (1 - slippage)
    opens = [
        100.0, 100.0, 100.0, 100.0, 100.0, 100.0,
        100.0, 102.0, 104.0, 106.0, 108.0,
        110.0, 110.0, 110.0, 110.0,
    ]
    closes = [
        100.0, 100.0, 100.0, 100.0, 100.0, 100.0,
        102.0, 104.0, 106.0, 108.0, 110.0,
        110.0, 110.0, 110.0, 110.0,
    ]
    highs = [max(o, c) + 2.0 for o, c in zip(opens, closes)]
    lows = [min(o, c) - 2.0 for o, c in zip(opens, closes)]

    return pd.DataFrame({
        "timestamp": dates,
        "open": opens,
        "high": highs,
        "low": lows,
        "close": closes,
        "volume": [500_000] * 15,
    })


# ============================================================================
# 1. Profile Initialization & Dynamic Rate Binding
# ============================================================================

def test_r5_custom_market_profile_initialization_rates():
    """Verify BacktestExecutionKernel initializes rates from custom MarketProfile."""
    calendar = TradingCalendar()
    settlement = SettlementEngine(calendar=calendar, settlement_days=2, conservative_mode=True)
    custom_profile = RulesMarketProfile(
        fee_buy=0.0025,
        fee_sell=0.0020,
        tax_sell=0.0015,
        slippage=0.0010,
        settlement_days=2,
        sizing_basis="fill_estimate",
        calendar=calendar,
        settlement_engine=settlement,
    )

    kernel = BacktestExecutionKernel(profile=custom_profile)

    assert kernel.fee_rate == 0.0025
    assert kernel.tax_rate == 0.0015
    assert kernel.slippage_rate == 0.0010
    assert kernel.min_holding_bars == 2
    assert kernel.sizing_basis == "fill_estimate"
    assert kernel.has_explicit_profile is True


def test_r5_constructor_override_precedence_over_profile():
    """Verify explicitly passed constructor args override the custom profile."""
    custom_profile = RulesMarketProfile(
        fee_buy=0.0025,
        fee_sell=0.0020,
        tax_sell=0.0015,
        slippage=0.0010,
    )

    kernel = BacktestExecutionKernel(
        profile=custom_profile,
        fee_rate=0.0040,
        slippage_rate=0.0030,
    )

    # Explicit constructor arguments must win
    assert kernel.fee_rate == 0.0040
    assert kernel.slippage_rate == 0.0030
    # Omitted arguments must default from custom profile
    assert kernel.tax_rate == 0.0015


# ============================================================================
# 2. Exact Single Round-Trip Hand-Calculated Parity with Custom Rates
# ============================================================================

def test_r5_custom_profile_single_round_trip_exact_parity():
    """Verify bit-exact hand-calculation, cash flow accounting, and round-trip PnL agreement.

    Custom Profile:
    - fee_buy = 0.0025 (0.25%)
    - fee_sell = 0.0020 (0.20%)
    - tax_sell = 0.0015 (0.15%)
    - slippage = 0.0010 (0.10%)
    - settlement_days = 2

    Hand Calculation:
    - Signal at Bar 5 Close (100.0) -> Buy queued for Bar 6 Open (100.0)
    - Buy fill price = 100.0 * (1.0 + 0.0010) = 100.10
    - Quantity = 100 shares
    - Buy gross = 100 * 100.10 = 10,010.00
    - Buy fee (0.25%) = 10,010.00 * 0.0025 = 25.025
    - Buy net cash outflow = 10,010.00 + 25.025 = 10,035.025

    - Exit signal at Bar 10 Close (110.0) -> Sell queued for Bar 11 Open (110.0)
    - Sell fill price = 110.0 * (1.0 - 0.0010) = 109.89
    - Quantity = 100 shares
    - Sell gross = 100 * 109.89 = 10,989.00
    - Sell fee (0.20%) = 10,989.00 * 0.0020 = 21.978
    - Sell tax (0.15%) = 10,989.00 * 0.0015 = 16.4835
    - Sell net cash inflow = 10,989.00 - 21.978 - 16.4835 = 10,950.5385

    - Cash delta = 10,950.5385 - 10,035.025 = 915.5135
    - Gross PnL = (109.89 - 100.10) * 100 = 979.00
    - Total Fees = 25.025 + 21.978 = 47.003
    - Total Taxes = 16.4835
    - Net PnL = 979.00 - 47.003 - 16.4835 = 915.5135
    """
    df = create_15_bar_sample_df()
    n = len(df)
    fast_ma = np.array([10.0] * 5 + [25.0] * 5 + [5.0] * 5)
    slow_ma = np.array([15.0] * n)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    custom_profile = RulesMarketProfile(
        fee_buy=0.0025,
        fee_sell=0.0020,
        tax_sell=0.0015,
        slippage=0.0010,
        settlement_days=2,
    )

    strategy = make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100)
    kernel = BacktestExecutionKernel(profile=custom_profile)

    initial_cash = 100_000.0
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="CUSTOM_SYM",
        initial_cash=initial_cash,
    )

    # 1. Order & Execution verification
    assert len(result.ledger.executions) == 2
    buy_exec = result.ledger.executions[0]
    sell_exec = result.ledger.executions[1]

    # BUY Execution
    assert buy_exec.side == "BUY"
    assert buy_exec.price == pytest.approx(100.10, rel=1e-6)
    assert buy_exec.gross_amount == pytest.approx(10010.0, rel=1e-6)
    assert buy_exec.fee == pytest.approx(25.025, rel=1e-6)
    assert buy_exec.tax == 0.0
    assert buy_exec.net_amount == pytest.approx(10035.025, rel=1e-6)

    # SELL Execution
    assert sell_exec.side == "SELL"
    assert sell_exec.price == pytest.approx(109.89, rel=1e-6)
    assert sell_exec.gross_amount == pytest.approx(10989.0, rel=1e-6)
    assert sell_exec.fee == pytest.approx(21.978, rel=1e-6)
    assert sell_exec.tax == pytest.approx(16.4835, rel=1e-6)
    assert sell_exec.net_amount == pytest.approx(10950.5385, rel=1e-6)

    # 2. Trade verification
    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 1
    trade = closed_trades[0]

    assert trade.entry_price == pytest.approx(100.10, rel=1e-6)
    assert trade.exit_price == pytest.approx(109.89, rel=1e-6)
    assert trade.gross_pnl == pytest.approx(979.0, rel=1e-6)
    assert trade.fees == pytest.approx(47.003, rel=1e-6)
    assert trade.taxes == pytest.approx(16.4835, rel=1e-6)
    assert trade.net_pnl == pytest.approx(915.5135, rel=1e-6)
    assert trade.result == "win"

    # Trade net_pnl invariant
    assert trade.net_pnl == pytest.approx(trade.gross_pnl - trade.fees - trade.taxes, abs=1e-9)

    # 3. compute_round_trip_pnl bit-exact parity
    rt_pnl = compute_round_trip_pnl(
        entry_price=trade.entry_price,
        exit_price=trade.exit_price,
        quantity=trade.quantity,
        side="LONG",
        profile=custom_profile,
    )
    assert trade.gross_pnl == pytest.approx(rt_pnl.gross_pnl, abs=1e-9)
    assert trade.net_pnl == pytest.approx(rt_pnl.net_pnl, abs=1e-9)
    assert trade.fees == pytest.approx(rt_pnl.total_fees, abs=1e-9)
    assert trade.taxes == pytest.approx(rt_pnl.total_tax, abs=1e-9)
    assert trade.pnl_percent == pytest.approx(rt_pnl.pnl_percent, abs=1e-9)

    # 4. Invariant: Ledger cash delta == sum of closed trade net_pnl
    cash_delta = result.ledger.current_cash - result.ledger.initial_cash
    assert cash_delta == pytest.approx(915.5135, abs=1e-6)
    assert cash_delta == pytest.approx(sum(t.net_pnl for t in result.ledger.trades), abs=1e-9)


# ============================================================================
# 3. Custom Profile Passed to run(profile=...) Dynamically
# ============================================================================

def test_r5_custom_profile_passed_to_run_parity():
    """Verify kernel initialized with defaults receives custom rates when passed via run(profile=...)."""
    df = create_15_bar_sample_df()
    n = len(df)
    fast_ma = np.array([10.0] * 5 + [25.0] * 5 + [5.0] * 5)
    slow_ma = np.array([15.0] * n)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    custom_profile = RulesMarketProfile(
        fee_buy=0.0030,
        fee_sell=0.0025,
        tax_sell=0.0020,
        slippage=0.0015,
        settlement_days=2,
    )

    # Default constructor
    kernel = BacktestExecutionKernel()

    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100),
        symbol="RUN_PROF_SYM",
        initial_cash=100_000.0,
        profile=custom_profile,
    )

    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 1
    trade = closed_trades[0]

    # Verify BUY fill: 100 * (1 + 0.0015) = 100.15
    buy_exec = result.ledger.executions[0]
    assert buy_exec.price == pytest.approx(100.15, rel=1e-6)
    assert buy_exec.fee == pytest.approx(100 * 100.15 * 0.0030, rel=1e-6)

    # Verify SELL fill: 110 * (1 - 0.0015) = 109.835
    sell_exec = result.ledger.executions[1]
    assert sell_exec.price == pytest.approx(109.835, rel=1e-6)
    assert sell_exec.fee == pytest.approx(100 * 109.835 * 0.0025, rel=1e-6)
    assert sell_exec.tax == pytest.approx(100 * 109.835 * 0.0020, rel=1e-6)

    # Invariants
    assert trade.net_pnl == pytest.approx(trade.gross_pnl - trade.fees - trade.taxes, abs=1e-9)
    assert result.ledger.current_cash - result.ledger.initial_cash == pytest.approx(trade.net_pnl, abs=1e-9)


# ============================================================================
# 4. Provider MarketProfile Registry Compatibility
# ============================================================================

def test_r5_provider_market_profile_compatibility():
    """Verify provider.MarketProfile (which defines fee_rate/tax_rate/slippage_rate) functions coherently."""
    calendar = TradingCalendar()
    settlement = SettlementEngine(calendar=calendar, settlement_days=2, conservative_mode=True)
    rules = MarketRules(exchange="HOSE")

    custom_provider_prof = ProviderMarketProfile(
        name="custom_provider_instance",
        calendar=calendar,
        settlement_engine=settlement,
        rules=rules,
        fee_rate=0.0022,
        tax_rate=0.0012,
        slippage_rate=0.0008,
    )

    kernel = BacktestExecutionKernel(profile=custom_provider_prof)
    assert kernel.fee_rate == 0.0022
    assert kernel.tax_rate == 0.0012
    assert kernel.slippage_rate == 0.0008

    df = create_15_bar_sample_df()
    n = len(df)
    fast_ma = np.array([10.0] * 5 + [25.0] * 5 + [5.0] * 5)
    slow_ma = np.array([15.0] * n)

    result = kernel.run(
        df=df,
        indicator_values={"fast_ma": fast_ma, "slow_ma": slow_ma},
        strategy=make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100),
        symbol="PROV_SYM",
        initial_cash=100_000.0,
    )

    trade = result.ledger.trades[0]
    expected_entry_price = 100.0 * (1.0 + 0.0008)
    expected_exit_price = 110.0 * (1.0 - 0.0008)
    assert trade.entry_price == pytest.approx(expected_entry_price, rel=1e-6)
    assert trade.exit_price == pytest.approx(expected_exit_price, rel=1e-6)

    # Invariants
    assert trade.net_pnl == pytest.approx(trade.gross_pnl - trade.fees - trade.taxes, abs=1e-9)
    assert result.ledger.current_cash - result.ledger.initial_cash == pytest.approx(trade.net_pnl, abs=1e-9)


# ============================================================================
# 5. Intraday Exit SL/TP with Custom MarketProfile
# ============================================================================

def test_r5_intraday_touch_custom_profile_coherence():
    """Verify intraday exit utilizes custom fee_sell and tax_sell and maintains cash ledger invariant."""
    calendar = TradingCalendar()
    # Use settlement_days=0 on the custom profile so intraday exit is sellable immediately
    settlement = SettlementEngine(calendar=calendar, settlement_days=0, conservative_mode=False)
    custom_profile = RulesMarketProfile(
        fee_buy=0.0025,
        fee_sell=0.0020,
        tax_sell=0.0015,
        slippage=0.0010,
        settlement_days=0,
        calendar=calendar,
        settlement_engine=settlement,
    )

    base_date = date(2024, 1, 8)
    candles = [
        {"timestamp": pd.Timestamp(base_date), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        {"timestamp": pd.Timestamp(base_date + timedelta(days=1)), "open": 10000.0, "high": 10500.0, "low": 9500.0, "close": 10000.0, "volume": 100_000},
        # Bar 2: Buy fills at Open 10000.0 * (1 + 0.0010) = 10010.0
        {"timestamp": pd.Timestamp(base_date + timedelta(days=2)), "open": 10000.0, "high": 10500.0, "low": 9800.0, "close": 10000.0, "volume": 100_000},
        # Bar 3: Low drops to 9300.0, breaching 5% SL (9500.0). Intraday exit at 9500.0
        {"timestamp": pd.Timestamp(base_date + timedelta(days=3)), "open": 10000.0, "high": 10500.0, "low": 9300.0, "close": 9400.0, "volume": 100_000},
        {"timestamp": pd.Timestamp(base_date + timedelta(days=4)), "open": 9200.0, "high": 9500.0, "low": 9000.0, "close": 9300.0, "volume": 100_000},
    ]
    df = pd.DataFrame(candles)

    fast_ma = np.array([10.0, 30.0, 10.0, 10.0, 10.0])
    slow_ma = np.array([20.0] * 5)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = make_strategy_ns(stop_loss=5.0, sizing_method="fixed_quantity", sizing_qty=100)
    strategy.risk_management.stop_loss_price = 9500.0
    strategy.exit_rules = []

    kernel = BacktestExecutionKernel(profile=custom_profile, sl_tp_execution_mode="intraday_touch")
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="INTRADAY_SL",
        initial_cash=10_000_000.0,
    )

    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 1
    trade = closed_trades[0]

    assert trade.exit_price == pytest.approx(9500.0, rel=1e-6)
    assert trade.result == "loss"
    assert trade.fees == pytest.approx(
        (100 * 10010.0 * 0.0025) + (100 * 9500.0 * 0.0020), rel=1e-6
    )
    assert trade.taxes == pytest.approx(100 * 9500.0 * 0.0015, rel=1e-6)
    assert trade.net_pnl == pytest.approx(trade.gross_pnl - trade.fees - trade.taxes, abs=1e-9)
    assert result.ledger.current_cash - result.ledger.initial_cash == pytest.approx(trade.net_pnl, abs=1e-9)


# ============================================================================
# 6. Multi-Trade Sequence Balance Invariant Under Custom Rates
# ============================================================================

def test_r5_multi_trade_sequence_cash_balance_invariant():
    """Verify that multiple successive trades preserve penny-exact cash balance invariant."""
    calendar = TradingCalendar()
    settlement = SettlementEngine(calendar=calendar, settlement_days=2, conservative_mode=True)
    custom_profile = RulesMarketProfile(
        fee_buy=0.0020,
        fee_sell=0.0015,
        tax_sell=0.0012,
        slippage=0.0005,
        settlement_days=2,
        calendar=calendar,
        settlement_engine=settlement,
    )

    # 30 bars, 2 distinct round trips
    dates = [pd.Timestamp(date(2024, 1, 8)) + pd.Timedelta(days=i) for i in range(30)]
    opens = [100.0] * 30
    highs = [102.0] * 30
    lows = [98.0] * 30
    closes = [100.0] * 30

    # Trade 1: Buy signal at bar 2 -> Sell at bar 8
    # Trade 2: Buy signal at bar 15 -> Sell at bar 22
    fast_ma = np.array(
        [10.0] * 2 + [30.0] * 5 + [10.0] * 7 + [30.0] * 5 + [10.0] * 11
    )
    slow_ma = np.array([20.0] * 30)

    df = pd.DataFrame({"timestamp": dates, "open": opens, "high": highs, "low": lows, "close": closes, "volume": [100_000] * 30})
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    kernel = BacktestExecutionKernel(profile=custom_profile)
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100),
        symbol="MULTI_SYM",
        initial_cash=500_000.0,
    )

    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 2

    # Invariant across all trades
    for trade in closed_trades:
        assert trade.net_pnl == pytest.approx(trade.gross_pnl - trade.fees - trade.taxes, abs=1e-9)

    total_net_pnl = sum(t.net_pnl for t in closed_trades)
    cash_delta = result.ledger.current_cash - result.ledger.initial_cash
    assert cash_delta == pytest.approx(total_net_pnl, abs=1e-9)


# ============================================================================
# 7. Default Profile Backward Compatibility (Golden Case G-01)
# ============================================================================

def test_r5_default_profile_exact_g01_compatibility():
    """Verify default profile preserves Golden Case G-01 exact penny-level hand calculation."""
    df = create_15_bar_sample_df()
    n = len(df)
    # Align prices with G-01:
    # Buy at Bar 6 Open = 10.1 (signal at Bar 5 Close = 10.0)
    # Sell at Bar 11 Open = 11.05 (signal at Bar 10 Close = 11.0)
    df.loc[0:5, "open"] = 10.0
    df.loc[0:5, "close"] = 10.0
    df.loc[6, "open"] = 10.1
    df.loc[6:9, "close"] = 10.5
    df.loc[7:10, "open"] = 10.5
    df.loc[10, "close"] = 11.0
    df.loc[11, "open"] = 11.05
    df.loc[11:14, "close"] = 11.05
    df.loc[12:14, "open"] = 11.05

    fast_ma = np.array([10.0] * 5 + [25.0] * 5 + [5.0] * 5)
    slow_ma = np.array([15.0] * n)
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    kernel = BacktestExecutionKernel()  # Default profile
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=make_strategy_ns(sizing_method="fixed_quantity", sizing_qty=100),
        symbol="G01_PARITY",
        initial_cash=100_000.0,
    )

    closed_trades = [t for t in result.ledger.trades if t.status == "closed"]
    assert len(closed_trades) == 1
    trade = closed_trades[0]

    # G-01 Exact Hand-Calc Parity Values:
    # Buy gross: 1010.0, fee: 1.515, net: 1011.515
    # Sell gross: 1105.0, fee: 1.6575, tax: 1.105, net: 1102.2375
    # Gross PnL: 95.0
    # Fees: 3.1725, Taxes: 1.105
    # Net PnL: 90.7225
    assert trade.gross_pnl == pytest.approx(95.0, abs=1e-4)
    assert trade.fees == pytest.approx(3.1725, abs=1e-4)
    assert trade.taxes == pytest.approx(1.105, abs=1e-4)
    assert trade.net_pnl == pytest.approx(90.7225, abs=1e-4)
    assert result.ledger.current_cash - result.ledger.initial_cash == pytest.approx(90.7225, abs=1e-4)
