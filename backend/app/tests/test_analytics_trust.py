from types import SimpleNamespace

import numpy as np
import pytest

from app.services.analytics_service import AnalyticsService
from app.models.candle import Candle
from app.models.replay_session import ReplaySession
from datetime import date, timedelta


def _trades(values):
    return [SimpleNamespace(net_pnl=value) for value in values]


def _curve_from_returns(returns, initial=100_000.0):
    equity = initial
    points = [{"equity": equity}]
    for value in returns:
        equity *= 1 + value
        points.append({"equity": equity})
    return points


def test_small_samples_are_nullable_with_reasons():
    metrics = AnalyticsService._build_metric_results(_trades(range(1, 30)), _curve_from_returns([0.001] * 29))

    assert metrics["win_rate"].value is None
    assert metrics["profit_factor"].value is None
    assert metrics["sqn"].status == "insufficient_data"
    assert metrics["sqn"].sample_size == 29
    assert metrics["sharpe_ratio"].status == "insufficient_data"
    assert metrics["sharpe_ratio"].sample_size == 29
    assert all(metric.reason for name, metric in metrics.items() if name != "total_net_pnl")


def test_thirty_trade_sqn_matches_hand_calculation():
    pnl = [100.0, -50.0] * 15
    metrics = AnalyticsService._build_metric_results(_trades(pnl), _curve_from_returns([0.001, -0.002] * 15))
    expected = np.sqrt(30) * np.mean(pnl) / np.std(pnl, ddof=1)

    assert metrics["sqn"].status == "valid"
    assert metrics["sqn"].value == pytest.approx(expected, abs=1e-4)
    assert metrics["win_rate"].value == pytest.approx(0.5)
    assert metrics["profit_factor"].value == pytest.approx(2.0)


def test_periodic_sharpe_and_sortino_match_documented_daily_returns():
    returns = [0.01, -0.005, 0.004, -0.002, 0.003] * 6
    metrics = AnalyticsService._build_metric_results(_trades([100.0, -50.0] * 15), _curve_from_returns(returns))
    rf_daily = (1 + 0.045) ** (1 / 252) - 1
    excess = np.array(returns) - rf_daily
    expected_sharpe = np.mean(excess) / np.std(excess, ddof=1) * np.sqrt(252)
    downside = [value for value in returns if value < 0]
    expected_sortino = np.mean(returns) / np.sqrt(np.mean(np.square(downside))) * np.sqrt(252)

    assert metrics["sharpe_ratio"].status == "valid"
    assert metrics["sharpe_ratio"].value == pytest.approx(expected_sharpe, abs=1e-4)
    assert metrics["sortino_ratio"].status == "valid"
    assert metrics["sortino_ratio"].value == pytest.approx(expected_sortino, abs=1e-4)


def test_flat_and_all_win_cases_refuse_false_precision():
    metrics = AnalyticsService._build_metric_results(_trades([100.0] * 30), _curve_from_returns([0.0] * 30))

    assert metrics["profit_factor"].status == "not_applicable"
    assert metrics["profit_factor"].value is None
    assert metrics["sqn"].status == "not_applicable"
    assert metrics["sharpe_ratio"].status == "not_applicable"
    assert metrics["sortino_ratio"].status == "insufficient_data"


def test_all_loss_case_is_honest_and_hand_calculable():
    metrics = AnalyticsService._build_metric_results(_trades([-100.0] * 30), _curve_from_returns([-0.001] * 30))

    assert metrics["win_rate"].status == "valid"
    assert metrics["win_rate"].value == 0
    assert metrics["profit_factor"].status == "valid"
    assert metrics["profit_factor"].value == 0
    assert metrics["sqn"].status == "not_applicable"
    assert "zero variance" in metrics["sqn"].reason


def test_missing_benchmark_is_not_applicable_with_reason(db_session):
    session = ReplaySession(
        symbol="MISSING_BENCH", timeframe="1D", adjustment_type="unadjusted",
        start_date=date(2024, 1, 1), end_date=date(2024, 1, 4), current_index=2,
        initial_cash=100_000, current_cash=100_000, status="active", mode="normal",
    )
    db_session.add(session)
    for index in range(3):
        close = 100 + index
        db_session.add(Candle(
            symbol="MISSING_BENCH", timeframe="1D", adjustment_type="unadjusted",
            timestamp=date(2024, 1, 1) + timedelta(days=index), open=close, high=close,
            low=close, close=close, volume=1000,
        ))
    db_session.commit()

    report = AnalyticsService.get_analytics(db_session, session.id, benchmark_symbol="NO_SUCH_BENCH")

    assert report.benchmark_curve == []
    assert report.metrics["benchmark"].status == "not_applicable"
    assert report.metrics["benchmark"].sample_size == 0
    assert "NO_SUCH_BENCH" in report.metrics["benchmark"].reason


def test_configured_benchmark_is_used_instead_of_vnindex(db_session):
    session = ReplaySession(
        symbol="BENCH_TARGET", timeframe="1D", adjustment_type="unadjusted",
        start_date=date(2024, 1, 1), end_date=date(2024, 1, 3), current_index=1,
        initial_cash=100_000, current_cash=100_000, status="active", mode="backtest",
        source_payload='{"benchmark_symbol":"ALT_BENCH"}',
    )
    db_session.add(session)
    for symbol, closes in (("BENCH_TARGET", [100, 101]), ("ALT_BENCH", [1000, 1200]), ("VNINDEX", [1000, 900])):
        for index, close in enumerate(closes):
            db_session.add(Candle(
                symbol=symbol, timeframe="1D", adjustment_type="unadjusted",
                timestamp=date(2024, 1, 1) + timedelta(days=index), open=close,
                high=close, low=close, close=close, volume=1000,
            ))
    db_session.commit()

    report = AnalyticsService.get_analytics(db_session, session.id)

    assert [point.value for point in report.benchmark_curve] == [100_000, 120_000]
    assert report.benchmark_symbol == "ALT_BENCH"
    assert report.metrics["benchmark"].status == "valid"
    assert report.metrics["benchmark"].value == pytest.approx(0.2)


def test_partial_coverage_reports_gaps_and_excluded_warmup():
    candles = [
        SimpleNamespace(timestamp=date(2024, 1, 1), open=100, high=101, low=99, close=100, volume=1000),
        SimpleNamespace(timestamp=date(2024, 1, 2), open=101, high=102, low=100, close=101, volume=1000),
        SimpleNamespace(timestamp=date(2024, 1, 8), open=102, high=103, low=101, close=102, volume=1000),
    ]
    from app.services.backtest_service import BacktestService
    from app.domain.strategy.strategy_loader import load_strategy_from_dict
    strategy = load_strategy_from_dict({
        "name": "Partial Coverage",
        "indicators": [{"name": "sma", "type": "sma", "length": 2}],
        "entry_rules": [], "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })

    coverage = BacktestService()._build_coverage(candles, "PARTIAL", "2024-01-01", "2024-01-10", strategy)

    assert coverage.actual_start == "2024-01-01"
    assert coverage.actual_end == "2024-01-08"
    assert coverage.gaps == ["2024-01-02..2024-01-08 (5 calendar days not represented)"]
    assert coverage.warmup_candles == 1
    assert coverage.excluded_data == ["1 warm-up candle(s) excluded from signal evaluation"]
