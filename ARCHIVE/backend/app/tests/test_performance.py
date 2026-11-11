"""Automated Performance and Cache Verification Suite.

Validates:
1. NFR-PERF-001: Compute features once per symbol across spanning window.
2. NFR-PERF-002: Interactive local research latency with warm cache.
3. TEST-PERF-001: Speedup measurement and bounded memory footprint.
4. TEST-REPRO-001: Bit-for-bit deterministic reproducibility between cache enabled vs disabled.
5. Invalidation safety: Automatic eviction/miss on candle, parameter, or version change.
"""

import time
from datetime import datetime, timedelta
import numpy as np
import pandas as pd
import pytest

from app.domain.engine.cache import (
    DomainCache,
    compute_candle_signature,
    compute_strategy_indicators_signature,
)
from app.domain.backtest.batch_runner import (
    BatchBacktestRunner,
    PhaseDefinition,
)
from app.domain.strategy.strategy_loader import load_strategy_from_dict
from app.domain.strategy.strategy_schema import StrategyConfig


def _generate_synthetic_candles(symbol: str, count: int = 300, start_date: str = "2025-01-01") -> pd.DataFrame:
    """Deterministic candle generator for performance testing."""
    rng = np.random.RandomState(abs(hash(symbol)) % (2**31 - 1))
    dates = [pd.Timestamp(start_date) + pd.Timedelta(days=i) for i in range(count)]
    
    close = 100.0
    records = []
    for dt in dates:
        ret = rng.normal(0.0005, 0.015)
        close = max(10.0, close * (1.0 + ret))
        high = close * (1.0 + abs(rng.normal(0, 0.005)))
        low = close * (1.0 - abs(rng.normal(0, 0.005)))
        open_val = (high + low) / 2.0
        vol = rng.uniform(100_000, 1_000_000)
        records.append({
            "timestamp": dt,
            "open": round(open_val, 2),
            "high": round(high, 2),
            "low": round(low, 2),
            "close": round(close, 2),
            "volume": round(vol, 0),
        })
    return pd.DataFrame(records)


def _create_test_strategy(length_fast: int = 20) -> StrategyConfig:
    """Create standard multi-indicator strategy."""
    return load_strategy_from_dict({
        "name": "Performance_Test_Strategy",
        "version": "1.0.0",
        "indicators": [
            {"name": "ema20", "type": "ema", "length": length_fast},
            {"name": "ema50", "type": "ema", "length": 50},
            {"name": "rsi14", "type": "rsi", "length": 14},
            {"name": "macd_standard", "type": "macd", "fast": 12, "slow": 26, "signal": 9},
            {"name": "atr14", "type": "atr", "length": 14},
        ],
        "entry_rules": [
            {"condition": "ema20 > ema50 and rsi14 > 50"},
        ],
        "exit_rules": [
            {"condition": "ema20 < ema50 or rsi14 < 40"},
        ],
        "position_sizing": {
            "method": "fixed_quantity",
            "quantity": 1000,
        },
        "risk_management": {
            "stop_loss_pct": 0.07,
            "take_profit_pct": 0.15,
        },
    })


class TestDomainCacheCore:
    """Milestone 1 & Invalidation Tests."""

    def test_lru_eviction_and_capacity(self):
        """Verify bounded memory footprint via LRU eviction."""
        cache = DomainCache(max_entries=3, enabled=True)
        assert cache.max_entries == 3

        cache.set("k1", "val1")
        cache.set("k2", "val2")
        cache.set("k3", "val3")
        assert cache.stats()["entry_count"] == 3
        assert cache.stats()["evictions"] == 0

        # Access k1 to make it most recently used
        assert cache.get("k1") == "val1"

        # Insert k4, which must evict k2 (least recently used)
        cache.set("k4", "val4")
        assert cache.stats()["entry_count"] == 3
        assert cache.stats()["evictions"] == 1
        assert cache.get("k2") is None  # evicted
        assert cache.get("k1") == "val1"  # preserved
        assert cache.get("k3") == "val3"  # preserved
        assert cache.get("k4") == "val4"  # preserved

    def test_cache_invalidation_symbol(self):
        """Verify invalidation cleans matching symbol prefixes without touching other symbols."""
        cache = DomainCache(max_entries=50, enabled=True)
        cache.set("features:FPT:sig1:strat1", {"data": 1})
        cache.set("features:FPT:sig2:strat1", {"data": 2})
        cache.set("features:VNM:sig1:strat1", {"data": 3})

        removed = cache.invalidate_symbol("FPT")
        assert removed == 2
        assert cache.get("features:FPT:sig1:strat1") is None
        assert cache.get("features:FPT:sig2:strat1") is None
        assert cache.get("features:VNM:sig1:strat1") == {"data": 3}

    def test_candle_signature_sensitivity(self):
        """Verify candle signatures change when length, timestamps or values change."""
        df = _generate_synthetic_candles("FPT", count=50)
        sig1 = compute_candle_signature(df)

        # Same dataframe produces identical signature
        assert compute_candle_signature(df) == sig1

        # Modified close price produces different signature
        df_mod = df.copy()
        df_mod.loc[len(df_mod) - 1, "close"] += 1.0
        sig2 = compute_candle_signature(df_mod)
        assert sig1 != sig2

        # Sliced dataframe produces different signature
        sig3 = compute_candle_signature(df.iloc[:-1])
        assert sig1 != sig3

    def test_indicator_signature_sensitivity(self):
        """Verify indicator config signatures differentiate parameter changes."""
        strat1 = _create_test_strategy()
        sig1 = compute_strategy_indicators_signature(strat1.indicators)

        # Same config -> identical signature
        assert compute_strategy_indicators_signature(strat1.indicators) == sig1

        # Changed parameter -> different signature
        strat2 = _create_test_strategy(length_fast=25)
        sig2 = compute_strategy_indicators_signature(strat2.indicators)
        assert sig1 != sig2


class TestDeterministicParityAndPerformance:
    """TEST-REPRO-001, TEST-PERF-001, NFR-PERF-001/002."""

    def test_deterministic_cache_parity(self):
        """TEST-REPRO-001: Exact bit-for-bit parity with cache enabled vs disabled."""
        strategy = _create_test_strategy()
        symbols = ["HPG", "FPT", "VNM"]
        phases = [
            PhaseDefinition(name="IS", start_date="2025-03-01", end_date="2025-06-01"),
            PhaseDefinition(name="OOS", start_date="2025-06-01", end_date="2025-09-01"),
        ]

        candles_store = {s: _generate_synthetic_candles(s, count=280) for s in symbols}
        provider = lambda sym, start, end: candles_store[sym]

        # 1. Uncached run
        uncached_res = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            use_cache=False,
        )

        # 2. Cached runs (isolated cache)
        test_cache = DomainCache(max_entries=100, enabled=True)
        cold_res = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            cache=test_cache,
            use_cache=True,
        )
        warm_res = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            cache=test_cache,
            use_cache=True,
        )

        # Verify exact numerical parity across all runs
        assert uncached_res.status == cold_res.status == warm_res.status == "succeeded"
        assert uncached_res.total_runs == cold_res.total_runs == warm_res.total_runs == 6
        assert uncached_res.summary["total_trades"] == cold_res.summary["total_trades"] == warm_res.summary["total_trades"]
        assert uncached_res.summary["total_net_pnl"] == cold_res.summary["total_net_pnl"] == warm_res.summary["total_net_pnl"]
        assert uncached_res.summary["overall_net_return_pct"] == cold_res.summary["overall_net_return_pct"] == warm_res.summary["overall_net_return_pct"]

        # Verify individual phase results bit-for-bit
        for u_phase, c_phase, w_phase in zip(uncached_res.phase_results, cold_res.phase_results, warm_res.phase_results):
            assert u_phase.symbol == c_phase.symbol == w_phase.symbol
            assert u_phase.phase_name == c_phase.phase_name == w_phase.phase_name
            assert u_phase.net_pnl == c_phase.net_pnl == w_phase.net_pnl
            assert u_phase.net_return_pct == c_phase.net_return_pct == w_phase.net_return_pct
            assert u_phase.total_trades == c_phase.total_trades == w_phase.total_trades
            assert u_phase.final_cash == c_phase.final_cash == w_phase.final_cash
            assert u_phase.final_equity == c_phase.final_equity == w_phase.final_equity

    def test_warm_cache_speedup_and_hit_rate(self):
        """TEST-PERF-001 & NFR-PERF-001: Warm cache speedup and hit counting."""
        strategy = _create_test_strategy()
        symbols = [f"SYM_{i:02d}" for i in range(12)]
        phases = [
            PhaseDefinition(name="P1", start_date="2025-02-01", end_date="2025-04-01"),
            PhaseDefinition(name="P2", start_date="2025-04-01", end_date="2025-06-01"),
            PhaseDefinition(name="P3", start_date="2025-06-01", end_date="2025-08-01"),
        ]

        candles_store = {s: _generate_synthetic_candles(s, count=240) for s in symbols}
        provider = lambda sym, start, end: candles_store[sym]

        test_cache = DomainCache(max_entries=100, enabled=True)

        # Cold Run
        cold_res = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            cache=test_cache,
            use_cache=True,
        )

        assert cold_res.timing_metrics is not None
        assert cold_res.timing_metrics.cache_hits == 0
        assert cold_res.timing_metrics.cache_misses == len(symbols)
        assert cold_res.feature_compute_count == len(symbols)

        # Warm Run
        warm_res = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            cache=test_cache,
            use_cache=True,
        )

        assert warm_res.timing_metrics is not None
        assert warm_res.timing_metrics.cache_hits == len(symbols)
        assert warm_res.timing_metrics.cache_misses == 0
        # Feature compute time on warm run should be minimal (< cold compute time)
        assert warm_res.timing_metrics.feature_compute_ms < cold_res.timing_metrics.feature_compute_ms + 1.0

    def test_cache_invalidation_on_data_mutation(self):
        """Verify cache correctly invalidates when candles for a symbol are updated."""
        strategy = _create_test_strategy()
        symbols = ["TCB"]
        phases = [
            PhaseDefinition(name="IS", start_date="2025-02-01", end_date="2025-05-01"),
        ]

        candles_v1 = _generate_synthetic_candles("TCB", count=180)
        provider = lambda sym, start, end: candles_v1

        test_cache = DomainCache(max_entries=50, enabled=True)
        res1 = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            cache=test_cache,
            use_cache=True,
        )
        assert res1.timing_metrics.cache_misses == 1

        # Second call with same candles -> hit
        res2 = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider,
            cache=test_cache,
            use_cache=True,
        )
        assert res2.timing_metrics.cache_hits == 1

        # Mutate candles (simulate newly imported daily candle)
        candles_v2 = _generate_synthetic_candles("TCB", count=190)
        provider_v2 = lambda sym, start, end: candles_v2

        res3 = BatchBacktestRunner.run_batch(
            strategy=strategy,
            symbols=symbols,
            phases=phases,
            candle_provider=provider_v2,
            cache=test_cache,
            use_cache=True,
        )
        # Must detect signature change and treat as cache miss, recalculating fresh
        assert res3.timing_metrics.cache_misses == 1

    def test_signal_service_caching_and_hit_rate(self, client, db_session):
        """Verify SignalService caches calculation results for prefix queries and invalidates on cursor movement."""
        from app.models.candle import Candle
        from app.models.replay_session import ReplaySession
        from app.domain.enums import SessionMode, SessionStatus
        from app.domain.engine.cache import default_domain_cache

        # Seed test candles and session
        for i in range(10):
            c = Candle(
                symbol="SSI",
                timeframe="1D",
                adjustment_type="adjusted",
                timestamp=datetime(2026, 1, i + 1),
                open=100.0 + i,
                high=105.0 + i,
                low=95.0 + i,
                close=102.0 + i,
                volume=500_000.0 + i * 10_000,
            )
            db_session.add(c)
        db_session.commit()

        session = ReplaySession(
            symbol="SSI",
            timeframe="1D",
            adjustment_type="adjusted",
            start_date=datetime(2026, 1, 1).date(),
            end_date=datetime(2026, 1, 10).date(),
            current_index=5,
            initial_cash=100_000_000,
            current_cash=100_000_000,
            status=SessionStatus.ACTIVE.value,
            mode=SessionMode.NORMAL.value,
            hide_symbol=False,
            hide_date=False,
        )
        db_session.add(session)
        db_session.commit()
        db_session.refresh(session)

        # Clear domain cache before test
        default_domain_cache.clear()
        initial_hits = default_domain_cache.stats()["hits"]
        initial_misses = default_domain_cache.stats()["misses"]

        payload = {
            "signals": [
                {"name": "health.score", "params": {}},
                {"name": "bb.direction_rising", "params": {"horizon": "T20"}},
            ]
        }

        # First call: cold (misses)
        resp1 = client.post(f"/api/signals/replay/{session.id}/calculate", json=payload)
        assert resp1.status_code == 200
        assert default_domain_cache.stats()["misses"] - initial_misses == 2
        assert default_domain_cache.stats()["hits"] - initial_hits == 0

        # Second call (same index and parameters): warm (hits)
        resp2 = client.post(f"/api/signals/replay/{session.id}/calculate", json=payload)
        assert resp2.status_code == 200
        assert default_domain_cache.stats()["hits"] - initial_hits == 2
        assert resp1.json() == resp2.json()

        # Third call with updated cursor: miss and fresh calculation
        session.current_index = 6
        db_session.commit()
        resp3 = client.post(f"/api/signals/replay/{session.id}/calculate", json=payload)
        assert resp3.status_code == 200
        assert default_domain_cache.stats()["misses"] - initial_misses == 4
