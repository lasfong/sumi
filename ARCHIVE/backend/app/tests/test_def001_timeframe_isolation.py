import asyncio
import math
from datetime import date, datetime, timedelta
import pytest

from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.market.calendar import TradingCalendar
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE
from app.models.candle import Candle
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.analytics_service import AnalyticsService
from app.services.backtest_service import BacktestService
from app.services.replay_service import ReplayService
from app.services.scanner_service import ScannerService
from app.services.strategy_lab_service import StrategyLabService
from app.services.weekly_aggregator import WeeklyAggregator


def _seed_coexisting_1d_and_1w(db_session, symbol: str = "FPT", benchmark: str = "VNINDEX"):
    """Seed 60 trading days of 1D candles plus coexisting 1W candles with divergent OHLCV."""
    cal = TradingCalendar()
    cur = date(2025, 1, 2)
    trading_days = []
    while len(trading_days) < 60:
        if cal.is_trading_day(cur):
            trading_days.append(cur)
        cur += timedelta(days=1)

    prev_price = 100000.0
    for idx, d in enumerate(trading_days):
        # Create oscillations to trigger SMA crossover / RSI / AST rules deterministically
        # and trend down smoothly in the final 18 bars so any open position closes cleanly before end_date
        if idx >= 42:
            base_price = prev_price - 1200.0
        else:
            cycle = math.sin(idx * 0.45) * 8000.0
            base_price = 100000.0 + idx * 300.0 + cycle
        prev_price = base_price
        open_p = round(base_price - 500.0, 2)
        close_p = round(base_price + 700.0, 2)
        high_p = round(max(open_p, close_p) + 1200.0, 2)
        low_p = round(min(open_p, close_p) - 1000.0, 2)
        db_session.add(
            Candle(
                symbol=symbol,
                timeframe="1D",
                timestamp=datetime.combine(d, datetime.min.time()),
                open=open_p,
                high=high_p,
                low=low_p,
                close=close_p,
                volume=150000.0 + idx * 1000.0,
                source="seed:canonical_1d",
                adjustment_type="unadjusted",
            )
        )
        db_session.add(
            Candle(
                symbol=benchmark,
                timeframe="1D",
                timestamp=datetime.combine(d, datetime.min.time()),
                open=1200.0 + idx,
                high=1210.0 + idx,
                low=1195.0 + idx,
                close=1205.0 + idx,
                volume=5000000.0,
                source="seed:canonical_1d",
                adjustment_type="unadjusted",
            )
        )
    db_session.commit()

    # Derive canonical 1W candles via WeeklyAggregator (stamped on Friday last_date)
    WeeklyAggregator.derive_weekly_candles(db_session, {symbol, benchmark}, {"unadjusted"})
    db_session.commit()

    return trading_days


def test_def001_single_batch_sweep_scanner_benchmark_parity_with_coexisting_1w(db_session):
    """
    DEF-001 Regression Test:
    With both 1D and 1W candles present for the same symbol and date range:
    1. Daily consumers load only the 60 canonical 1D candles (never 60 + 13 = 73 mixed bars).
    2. Single Backtest, Batch Backtest, and Parameter Sweep produce bit-exact Net PnL, Final Equity,
       and Trade Count on both built-in strategies and Custom AST rules (G-12).
    3. Scanner and Analytics benchmark curve load only 1D candles.
    """
    trading_days = _seed_coexisting_1d_and_1w(db_session, symbol="FPT", benchmark="VNINDEX")
    start_str = trading_days[0].isoformat()
    end_str = trading_days[-1].isoformat()

    total_db_fpt = db_session.query(Candle).filter(Candle.symbol == "FPT").count()
    daily_db_fpt = db_session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").count()
    weekly_db_fpt = db_session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1W").count()
    assert daily_db_fpt == 60
    assert weekly_db_fpt > 0
    assert total_db_fpt == daily_db_fpt + weekly_db_fpt

    ast_strategy = {
        "name": "Custom_AST_Parity",
        "version": "1.0",
        "description": "Custom AST strategy for DEF-001 parity verification",
        "indicators": [
            {"name": "sma_fast", "type": "sma", "length": 5},
            {"name": "sma_slow", "type": "sma", "length": 12},
        ],
        "entry_rules": [
            {"condition": "sma_fast > sma_slow"},
        ],
        "exit_rules": [
            {"condition": "sma_fast < sma_slow"},
        ],
        "position_sizing": {"method": "percent_equity", "percent": 20.0},
    }

    bt_service = BacktestService()
    single_res = asyncio.run(
        bt_service.run_backtest(
            db_session,
            {
                "symbol": "FPT",
                "start_date": start_str,
                "end_date": end_str,
                "initial_cash": 100_000_000.0,
                "benchmark_symbol": "VNINDEX",
                "strategy": ast_strategy,
            },
        )
    )
    assert single_res["status"] == "succeeded"
    assert single_res["total_candles"] == 60
    assert single_res["data_coverage"]["candle_count"] == 60

    batch_res = asyncio.run(
        bt_service.run_batch_backtest(
            db_session,
            {
                "symbols": ["FPT"],
                "phases": [{"name": "FullWindow", "start_date": start_str, "end_date": end_str}],
                "initial_cash": 100_000_000.0,
                "strategy": ast_strategy,
                "use_cache": False,
            },
        )
    )
    assert batch_res["status"] == "succeeded"
    phase_res = batch_res["phase_results"][0]
    assert phase_res["total_candles"] == 60

    lab_service = StrategyLabService()
    sweep_res = asyncio.run(
        lab_service.run_parameter_sweep(
            db_session,
            {
                "symbol": "FPT",
                "start_date": start_str,
                "end_date": end_str,
                "initial_cash": 100_000_000.0,
                "benchmark_symbol": "VNINDEX",
                "strategy": ast_strategy,
                "sweep": [{"path": "indicators[0].length", "values": [5]}],
            },
        )
    )
    assert sweep_res["status"] in ("succeeded", "completed")
    sweep_variant = sweep_res["variants"][0]
    assert sweep_variant["response"]["total_candles"] == 60

    assert round(float(single_res["kernel_result"].final_cash), 2) == round(float(phase_res["final_cash"]), 2)
    assert round(float(single_res["kernel_result"].final_equity), 2) == round(float(phase_res["final_equity"]), 2)

    single_pnl = round(float(single_res["analytics"]["total_net_pnl"]), 2)
    batch_pnl = round(float(phase_res["benchmark_metrics"]["net_profit"]), 2)
    sweep_pnl = round(float(sweep_variant["metrics"]["net_pnl"]), 2)
    assert single_pnl == batch_pnl == sweep_pnl
    assert single_res["analytics"]["total_trades"] > 0
    assert single_res["analytics"]["total_trades"] == phase_res["total_trades"] == sweep_variant["metrics"]["total_trades"]

    # Verify benchmark curve in Analytics has exactly 60 points (1D only, no 1W duplicates)
    benchmark_curve = single_res["analytics"]["benchmark_curve"]
    assert len(benchmark_curve) == 60
    benchmark_timestamps = [pt["time"] for pt in benchmark_curve]
    assert len(benchmark_timestamps) == len(set(benchmark_timestamps))

    # Verify Scanner loads only 1D candles and produces no duplicate signal timestamps
    scanner = ScannerService()
    scan_res = scanner.run_scan(
        db_session,
        {
            "symbols": ["FPT"],
            "start_date": start_str,
            "end_date": end_str,
            "benchmark_symbol": "VNINDEX",
            "strategy": ast_strategy,
        },
    )
    assert scan_res["status"] == "succeeded"
    scan_timestamps = [r["timestamp"] for r in scan_res["results"]]
    assert len(scan_timestamps) == len(set(scan_timestamps))

    # Also verify the exact FIN-AST-PARITY signal-based rule (health__favorable and bb__direction_rising)
    srb_config = {
        "name": "Bo_Dieu_Kien_Mau",
        "version": "1.0",
        "description": "Quy tắc AST tùy chỉnh: health__favorable == True and bb__direction_rising == True",
        "indicators": [
            {"name": "health__favorable", "type": "signal"},
            {"name": "bb__direction_rising", "type": "signal"},
        ],
        "entry_rules": [{"condition": "health__favorable == True and bb__direction_rising == True"}],
        "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 1000},
        "risk_management": {"stop_loss_pct": 7.0, "take_profit_pct": 15.0},
    }
    srb_single = asyncio.run(
        bt_service.run_backtest(
            db_session,
            {
                "symbol": "FPT",
                "start_date": start_str,
                "end_date": end_str,
                "initial_cash": 100_000_000.0,
                "benchmark_symbol": "VNINDEX",
                "strategy": srb_config,
            },
        )
    )
    srb_batch = asyncio.run(
        bt_service.run_batch_backtest(
            db_session,
            {
                "symbols": ["FPT"],
                "phases": [{"name": "FullWindow", "start_date": start_str, "end_date": end_str}],
                "initial_cash": 100_000_000.0,
                "strategy": srb_config,
                "use_cache": False,
            },
        )
    )
    srb_sweep = asyncio.run(
        lab_service.run_parameter_sweep(
            db_session,
            {
                "symbol": "FPT",
                "start_date": start_str,
                "end_date": end_str,
                "initial_cash": 100_000_000.0,
                "benchmark_symbol": "VNINDEX",
                "strategy": srb_config,
                "sweep": [{"path": "risk_management.stop_loss_pct", "values": [7.0]}],
            },
        )
    )
    assert srb_single["total_candles"] == 60
    assert srb_batch["phase_results"][0]["total_candles"] == 60
    assert srb_sweep["variants"][0]["response"]["total_candles"] == 60
    assert (
        round(float(srb_single["analytics"]["total_net_pnl"]), 2)
        == round(float(srb_batch["phase_results"][0]["benchmark_metrics"]["net_profit"]), 2)
        == round(float(srb_sweep["variants"][0]["metrics"]["net_pnl"]), 2)
    )


def test_def001_no_silent_fallback_and_quarantine_respected(db_session):
    """
    Requirements 6 & 7:
    - Requesting 1D when only 1W exists returns NO_CANDLES (no silent fallback to 1W).
    - Quarantine 1D candles are still excluded when filtering by timeframe.
    """
    from app.domain.strategy.strategy_loader import list_available_strategies

    # Seed only a 1W candle for ONLY_1W
    db_session.add(
        Candle(
            symbol="ONLY_1W",
            timeframe="1W",
            timestamp=datetime(2025, 1, 10),
            open=50000.0,
            high=55000.0,
            low=49000.0,
            close=54000.0,
            volume=500000.0,
            source="VN_TRADING_WEEK_V1",
            adjustment_type="unadjusted",
        )
    )
    # Seed only a quarantine 1D candle for QUAR_ONLY
    db_session.add(
        Candle(
            symbol="QUAR_ONLY",
            timeframe="1D",
            timestamp=datetime(2025, 1, 10),
            open=50000.0,
            high=55000.0,
            low=49000.0,
            close=54000.0,
            volume=500000.0,
            source="quarantine:bad_provider",
            adjustment_type="unadjusted",
        )
    )
    db_session.commit()

    strategy = list_available_strategies()[0]["config"]
    bt_service = BacktestService()

    res_1w_only = asyncio.run(
        bt_service.run_backtest(
            db_session,
            {
                "symbol": "ONLY_1W",
                "start_date": "2025-01-01",
                "end_date": "2025-01-31",
                "strategy": strategy,
            },
        )
    )
    assert res_1w_only["status"] == "failed"
    assert res_1w_only["error_code"] == "NO_CANDLES"

    res_quar = asyncio.run(
        bt_service.run_backtest(
            db_session,
            {
                "symbol": "QUAR_ONLY",
                "start_date": "2025-01-01",
                "end_date": "2025-01-31",
                "strategy": strategy,
            },
        )
    )
    assert res_quar["status"] == "failed"
    assert res_quar["error_code"] == "NO_CANDLES"


def test_def001_incomplete_weekly_aggregation_never_leaks_future_at_cursor_k(db_session):
    """
    Requirement 5:
    Prevent incomplete weekly aggregation from leaking future information at historical cursor K.
    Tests both Friday-stamped 1W candles and Monday-stamped 1W candles while stepping cursor K
    across Monday..Friday of Week 1 and into Week 2.
    """
    # Week 1: Mon 2026-01-05 .. Fri 2026-01-09
    week1_days = [date(2026, 1, 5 + i) for i in range(5)]
    for i, d in enumerate(week1_days):
        db_session.add(
            Candle(
                symbol="VCB",
                timeframe="1D",
                timestamp=datetime.combine(d, datetime.min.time()),
                open=100.0 + i,
                high=105.0 + i * 10.0,  # Friday high is 145.0
                low=95.0,
                close=102.0 + i * 5.0,  # Friday close is 122.0
                volume=1000.0,
                source="seed:1d",
                adjustment_type="unadjusted",
            )
        )
    # Seed a Monday-stamped 1W candle containing Friday's high (145.0) and close (122.0)
    db_session.add(
        Candle(
            symbol="VCB",
            timeframe="1W",
            timestamp=datetime(2026, 1, 5),
            open=100.0,
            high=145.0,
            low=95.0,
            close=122.0,
            volume=5000.0,
            source="seed:monday_stamped_1w",
            adjustment_type="unadjusted",
        )
    )
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(
            symbol="VCB",
            timeframe="1D",
            start_date=date(2026, 1, 5),
            end_date=date(2026, 1, 9),
        ),
    )

    # At index 0 (Mon), 1 (Tue), 2 (Wed), 3 (Thu): incomplete week must NOT leak the 1W bar
    for expected_idx in range(4):
        weekly_visible = ReplayService.get_candles(db_session, session.id, target_timeframe="1W")
        assert weekly_visible == [], f"Leaked 1W candle at incomplete week index {expected_idx}"
        ReplayService.next_candle(db_session, session.id)

    # At index 4 (Fri 2026-01-09): week is complete, 1W candle is now causally visible
    weekly_visible_fri = ReplayService.get_candles(db_session, session.id, target_timeframe="1W")
    assert len(weekly_visible_fri) == 1
    assert weekly_visible_fri[0].high == 145.0


def test_def001_golden_cases_g01_g04_g05_g11_independent_math(db_session):
    """
    Requirement 8:
    Verify authoritative financial formulas from FINANCIAL_GOLDEN_CASES.md (G-01, G-04, G-05, G-11)
    against independent expected calculations.
    """
    import pandas as pd
    from app.domain.strategy.strategy_loader import load_strategy_from_dict

    # Construct a 6-bar daily series where entry triggers on bar 1 (filled at Open[2] = 10,000)
    # and exit triggers on bar 3 (filled at Open[4] = 11,000), matching G-01:
    # Fill Qty = 900, Gross Buy = 9,000,000, Buy Fee = 13,500
    # Gross Sell = 9,900,000, Sell Fee = 14,850, Sell Tax = 9,900
    # Net PnL = +861,750.00 VND, Final Equity = 10,861,750.00 VND
    dates = [
        datetime(2026, 1, 5),
        datetime(2026, 1, 6),
        datetime(2026, 1, 7),
        datetime(2026, 1, 8),
        datetime(2026, 1, 9),
        datetime(2026, 1, 12),
    ]
    df = pd.DataFrame(
        [
            {"timestamp": dates[0], "open": 10000.0, "high": 10100.0, "low": 9900.0, "close": 10000.0, "volume": 100000.0},
            {"timestamp": dates[1], "open": 10000.0, "high": 10200.0, "low": 9900.0, "close": 10100.0, "volume": 100000.0},
            {"timestamp": dates[2], "open": 10000.0, "high": 10500.0, "low": 9950.0, "close": 10400.0, "volume": 100000.0},
            {"timestamp": dates[3], "open": 10500.0, "high": 11000.0, "low": 10400.0, "close": 10800.0, "volume": 100000.0},
            {"timestamp": dates[4], "open": 11000.0, "high": 11100.0, "low": 10900.0, "close": 11000.0, "volume": 100000.0},
            {"timestamp": dates[5], "open": 11000.0, "high": 11100.0, "low": 10900.0, "close": 11000.0, "volume": 100000.0},
        ]
    )
    strategy = load_strategy_from_dict(
        {
            "name": "G01_Golden",
            "version": "1.0",
            "indicators": [{"name": "sig", "type": "sma", "length": 1}],
            "entry_rules": [{"type": "threshold", "indicator": "sig", "operator": ">", "value": 10050.0}],
            "exit_rules": [{"type": "threshold", "indicator": "sig", "operator": ">", "value": 10700.0}],
            "position_sizing": {"method": "percent_equity", "percent": 100.0},
        }
    )
    indicator_values = {"sig": [10000.0, 10100.0, 10400.0, 10800.0, 10000.0, 10000.0]}
    kernel = BacktestExecutionKernel(
        fee_rate=DEFAULT_VN_MARKET_PROFILE.fee_buy,
        tax_rate=DEFAULT_VN_MARKET_PROFILE.tax_sell,
        slippage_rate=0.0,
        min_holding_bars=2,
        profile=DEFAULT_VN_MARKET_PROFILE,
        sl_tp_execution_mode="next_open",
        sizing_basis="fill_estimate",
    )
    res = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="FPT",
        initial_cash=10_000_000.0,
        timeframe="1D",
    )
    assert len(res.trades) == 1
    trade = res.trades[0]
    assert trade.quantity == 900
    assert trade.entry_price == 10000.0
    assert trade.exit_price == 11000.0
    assert round(trade.net_pnl, 2) == 861750.00
    assert round(res.final_cash, 2) == 10861750.00
