"""Multi-Symbol, Multi-Phase Batch Backtest Runner.

Provides compute-once feature caching (NFR-PERF-001, TEST-PERF-001),
strictly independent capital per phase (FR-CORE-009),
and phase-end position holding integrity (TEST-BT-005).

Pure domain orchestration decoupled from FastAPI and database commits.
"""

from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, Dict, List, Optional, Set, Tuple
import time
import pandas as pd

from app.domain.accounting import BUY_FEE_RATE, SELL_TAX_RATE
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.models import BacktestKernelResult
from app.domain.backtest.metrics import (
    BenchmarkMetricRow,
    BenchmarkTableRenderer,
    CrossPhaseComparison,
    PhaseMetricMatrix,
    build_phase_metric_matrix,
    calculate_benchmark_metrics,
)
from app.domain.engine.cache import (
    DomainCache,
    default_domain_cache,
    compute_candle_signature,
    compute_strategy_indicators_signature,
)
from app.domain.engine.strategy_indicator_adapter import StrategyIndicatorAdapter
from app.domain.market.provider import MarketProfile, MarketRuleProvider
from app.domain.strategy.strategy_schema import StrategyConfig
from app.domain.strategy.strategy_rule_evaluator import StrategyRuleEvaluator
from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.utils.date_range import end_before, start_at


@dataclass(frozen=True)
class PhaseDefinition:
    """Definition of an evaluation phase / date range."""
    name: str
    start_date: str
    end_date: str
    description: Optional[str] = None

    def validate(self) -> None:
        """Validate date bounds."""
        if not self.name or not self.name.strip():
            raise ValueError("Phase name cannot be empty")
        if not self.start_date or not self.end_date:
            raise ValueError(f"Phase '{self.name}': start_date and end_date are required")

        start_val = str(self.start_date).strip()
        end_val = str(self.end_date).strip()
        if start_val >= end_val:
            raise ValueError(
                f"Phase '{self.name}': start_date '{self.start_date}' must be strictly before end_date '{self.end_date}'"
            )


@dataclass
class SymbolFeatureCache:
    """Precomputed feature and indicator cache for a single symbol over a spanning range."""
    symbol: str
    full_df: pd.DataFrame
    indicator_values: Dict[str, pd.Series]
    warmup_bars: int
    total_candles: int

    def slice_phase(self, phase: PhaseDefinition) -> Tuple[pd.DataFrame, Dict[str, pd.Series]]:
        """Slice precomputed dataframe and indicator series for a specific phase window.
        
        Zero recomputation of indicators.
        Returns a reindexed DataFrame and aligned indicator series for the phase slice.
        """
        start_dt = start_at(phase.start_date)
        end_dt = end_before(phase.end_date)

        # Boolean mask for timestamp window
        mask = (self.full_df["timestamp"] >= start_dt) & (self.full_df["timestamp"] < end_dt)
        df_sliced = self.full_df.loc[mask].copy().reset_index(drop=True)

        if df_sliced.empty:
            return df_sliced, {k: pd.Series(dtype=float) for k in self.indicator_values.keys()}

        # Slice indicator values by the exact same indices
        indices = self.full_df.index[mask].tolist()
        sliced_indicators: Dict[str, pd.Series] = {}
        for name, series in self.indicator_values.items():
            if isinstance(series, pd.Series):
                sliced_val = series.iloc[indices].copy().reset_index(drop=True)
                sliced_indicators[name] = sliced_val
            elif isinstance(series, (list, tuple)):
                sliced_indicators[name] = pd.Series([series[i] for i in indices], dtype=float)
            else:
                sliced_indicators[name] = series

        return df_sliced, sliced_indicators


@dataclass
class BatchPhaseResult:
    """Simulation outcome for a single (symbol, phase) execution."""
    symbol: str
    phase_name: str
    start_date: str
    end_date: str
    status: str  # "succeeded" | "failed"
    total_candles: int
    initial_cash: float
    final_cash: float
    final_equity: float
    net_pnl: float
    net_return_pct: float
    total_trades: int
    open_position_quantity: float
    open_position_value: float
    warnings: List[str] = field(default_factory=list)
    error_message: Optional[str] = None
    kernel_result: Optional[BacktestKernelResult] = None
    benchmark_metrics: Optional[BenchmarkMetricRow] = None

    def to_dict(self) -> Dict[str, Any]:
        """Serialize summary fields into dict."""
        return {
            "symbol": self.symbol,
            "phase_name": self.phase_name,
            "start_date": self.start_date,
            "end_date": self.end_date,
            "status": self.status,
            "total_candles": self.total_candles,
            "initial_cash": round(self.initial_cash, 2),
            "final_cash": round(self.final_cash, 2),
            "final_equity": round(self.final_equity, 2),
            "net_pnl": round(self.net_pnl, 2),
            "net_return_pct": round(self.net_return_pct, 4),
            "total_trades": self.total_trades,
            "open_position_quantity": self.open_position_quantity,
            "open_position_value": round(self.open_position_value, 2),
            "warnings": list(self.warnings),
            "error_message": self.error_message,
            "benchmark_metrics": self.benchmark_metrics.to_dict() if self.benchmark_metrics else None,
        }


@dataclass
class BatchTimingMetrics:
    """Execution timing and cache performance metadata for a batch run."""
    total_duration_ms: float = 0.0
    feature_compute_ms: float = 0.0
    simulation_ms: float = 0.0
    cache_hits: int = 0
    cache_misses: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "total_duration_ms": round(self.total_duration_ms, 2),
            "feature_compute_ms": round(self.feature_compute_ms, 2),
            "simulation_ms": round(self.simulation_ms, 2),
            "cache_hits": self.cache_hits,
            "cache_misses": self.cache_misses,
        }


@dataclass
class BatchBacktestResult:
    """Aggregated outcome of a multi-symbol, multi-phase batch run."""
    status: str  # "succeeded" | "partial" | "failed"
    total_symbols: int
    total_phases: int
    total_runs: int
    feature_compute_count: int  # Instrumentation counter for TEST-PERF-001
    simulation_run_count: int   # Total simulations performed
    phase_results: List[BatchPhaseResult] = field(default_factory=list)
    summary: Dict[str, Any] = field(default_factory=dict)
    metric_matrix: Optional[PhaseMetricMatrix] = None
    cross_phase_degradations: List[CrossPhaseComparison] = field(default_factory=list)
    markdown_table: Optional[str] = None
    csv_export: Optional[str] = None
    timing_metrics: Optional[BatchTimingMetrics] = None

    def to_dict(self) -> Dict[str, Any]:
        """Serialize aggregated outcome into dict."""
        return {
            "status": self.status,
            "total_symbols": self.total_symbols,
            "total_phases": self.total_phases,
            "total_runs": self.total_runs,
            "feature_compute_count": self.feature_compute_count,
            "simulation_run_count": self.simulation_run_count,
            "phase_results": [r.to_dict() for r in self.phase_results],
            "summary": self.summary,
            "metric_matrix": self.metric_matrix.to_dict() if self.metric_matrix else None,
            "cross_phase_degradations": [d.to_dict() for d in self.cross_phase_degradations],
            "markdown_table": self.markdown_table,
            "csv_export": self.csv_export,
            "timing_metrics": self.timing_metrics.to_dict() if self.timing_metrics else None,
        }



class BatchBacktestRunner:
    """Orchestrator for multi-symbol, multi-phase backtests."""

    @classmethod
    def validate_phases(cls, phases: List[PhaseDefinition]) -> None:
        """Validate list of phases for uniqueness and date bounds."""
        if not phases:
            raise ValueError("Phases list cannot be empty. At least one phase must be defined.")

        seen_names: Set[str] = set()
        for p in phases:
            p.validate()
            if p.name in seen_names:
                raise ValueError(f"Duplicate phase name detected: '{p.name}'")
            seen_names.add(p.name)

    @classmethod
    def determine_spanning_window(
        cls,
        phases: List[PhaseDefinition],
        warmup_days: int = 60,
    ) -> Tuple[str, str]:
        """Determine earliest start date (with warmup) and latest end date across all phases."""
        starts = [start_at(p.start_date) for p in phases]
        ends = [end_before(p.end_date) for p in phases]

        earliest = min(starts)
        latest = max(ends)

        # Apply warmup offset to earliest date
        warmup_earliest = earliest - pd.Timedelta(days=warmup_days)
        warmup_start_str = warmup_earliest.strftime("%Y-%m-%d")
        latest_end_str = latest.strftime("%Y-%m-%d")

        return warmup_start_str, latest_end_str

    @classmethod
    def build_symbol_feature_cache(
        cls,
        symbol: str,
        candles_df: pd.DataFrame,
        strategy: StrategyConfig,
        cache: Optional[DomainCache] = None,
        use_cache: bool = True,
    ) -> SymbolFeatureCache:
        """Compute indicator features ONCE over the spanning DataFrame for a symbol.
        
        Leverages DomainCache if enabled, invalidating automatically when candle
        data or strategy indicator parameters differ.
        """
        if candles_df.empty:
            return SymbolFeatureCache(
                symbol=symbol,
                full_df=candles_df,
                indicator_values={},
                warmup_bars=0,
                total_candles=0,
            )

        # Standardize columns
        required_cols = {"timestamp", "open", "high", "low", "close", "volume"}
        missing = required_cols - set(candles_df.columns)
        if missing:
            raise ValueError(f"Candles DataFrame for {symbol} missing columns: {missing}")

        sorted_df = candles_df.sort_values("timestamp").copy().reset_index(drop=True)

        active_cache = cache if cache is not None else default_domain_cache
        indicator_values = None

        if use_cache and active_cache.enabled:
            candle_sig = compute_candle_signature(sorted_df)
            strategy_sig = compute_strategy_indicators_signature(strategy.indicators)
            cache_key = f"features:{symbol.strip().upper()}:{candle_sig}:{strategy_sig}"
            cached_values = active_cache.get(cache_key)
            if cached_values is not None:
                indicator_values = cached_values

        if indicator_values is None:
            # Compute all indicators defined in strategy once
            indicator_values = StrategyIndicatorAdapter.compute(sorted_df, strategy.indicators)
            if use_cache and active_cache.enabled:
                candle_sig = compute_candle_signature(sorted_df)
                strategy_sig = compute_strategy_indicators_signature(strategy.indicators)
                cache_key = f"features:{symbol.strip().upper()}:{candle_sig}:{strategy_sig}"
                active_cache.set(cache_key, indicator_values)

        # Validate strategy rules against available indicator keys
        StrategyRuleEvaluator.validate_strategy_rules(strategy, set(indicator_values.keys()))

        return SymbolFeatureCache(
            symbol=symbol,
            full_df=sorted_df,
            indicator_values=indicator_values,
            warmup_bars=0,
            total_candles=len(sorted_df),
        )

    @classmethod
    def run_batch(
        cls,
        strategy: StrategyConfig,
        symbols: List[str],
        phases: List[PhaseDefinition],
        candle_provider: Callable[[str, str, str], pd.DataFrame],
        initial_cash_per_run: float = 100_000_000.0,
        execution_profile_name: str = "vietnam_default_conservative",
        exchange: str = "HOSE",
        slippage_rate: float = 0.0,
        cache: Optional[DomainCache] = None,
        use_cache: bool = True,
    ) -> BatchBacktestResult:
        """Execute a complete multi-symbol, multi-phase batch run with independent capital.
        
        Args:
            strategy: Parsed Strategy definition.
            symbols: List of ticker symbols to evaluate.
            phases: List of PhaseDefinition evaluation windows.
            candle_provider: Callable(symbol, spanning_start, spanning_end) -> pd.DataFrame.
            initial_cash_per_run: Independent capital starting balance for each (symbol, phase) run.
            execution_profile_name: Named market rule execution profile.
            exchange: Target exchange (HOSE, HNX, UPCoM).
            slippage_rate: Slippage percentage.
            cache: Optional explicit DomainCache instance (defaults to shared default_domain_cache).
            use_cache: Toggle feature caching for performance/deterministic verification.
        """
        # 1. Validate inputs
        if not symbols:
            raise ValueError("Symbols list cannot be empty")
        clean_symbols = [s.strip().upper() for s in symbols if s and s.strip()]
        if not clean_symbols:
            raise ValueError("At least one valid symbol is required")

        cls.validate_phases(phases)

        profile = MarketRuleProvider.get_profile(execution_profile_name, exchange=exchange)
        spanning_start, spanning_end = cls.determine_spanning_window(phases, warmup_days=90)

        start_time = time.perf_counter()
        feature_compute_ms = 0.0
        simulation_ms = 0.0

        active_cache = cache if cache is not None else default_domain_cache
        initial_hits = active_cache.stats()["hits"] if (use_cache and active_cache.enabled) else 0
        initial_misses = active_cache.stats()["misses"] if (use_cache and active_cache.enabled) else 0

        feature_compute_count = 0
        simulation_run_count = 0
        phase_results: List[BatchPhaseResult] = []

        # 2. Iterate each symbol: compute features once
        for symbol in clean_symbols:
            # Load spanning candles
            try:
                raw_candles = candle_provider(symbol, spanning_start, spanning_end)
            except Exception as exc:
                # Symbol loading failed: mark all phases for this symbol as failed
                for phase in phases:
                    phase_results.append(
                        BatchPhaseResult(
                            symbol=symbol,
                            phase_name=phase.name,
                            start_date=phase.start_date,
                            end_date=phase.end_date,
                            status="failed",
                            total_candles=0,
                            initial_cash=initial_cash_per_run,
                            final_cash=initial_cash_per_run,
                            final_equity=initial_cash_per_run,
                            net_pnl=0.0,
                            net_return_pct=0.0,
                            total_trades=0,
                            open_position_quantity=0.0,
                            open_position_value=0.0,
                            error_message=f"Failed to load candles: {exc}",
                            benchmark_metrics=calculate_benchmark_metrics(
                                trades=[],
                                initial_cash=initial_cash_per_run,
                                ticker=symbol,
                                phase_name=phase.name,
                                start_date=phase.start_date,
                                end_date=phase.end_date,
                                final_cash=initial_cash_per_run,
                                final_equity=initial_cash_per_run,
                            ),
                        )
                    )
                continue

            if raw_candles is None or raw_candles.empty:
                for phase in phases:
                    phase_results.append(
                        BatchPhaseResult(
                            symbol=symbol,
                            phase_name=phase.name,
                            start_date=phase.start_date,
                            end_date=phase.end_date,
                            status="failed",
                            total_candles=0,
                            initial_cash=initial_cash_per_run,
                            final_cash=initial_cash_per_run,
                            final_equity=initial_cash_per_run,
                            net_pnl=0.0,
                            net_return_pct=0.0,
                            total_trades=0,
                            open_position_quantity=0.0,
                            open_position_value=0.0,
                            error_message=f"No candles found for symbol {symbol}",
                            benchmark_metrics=calculate_benchmark_metrics(
                                trades=[],
                                initial_cash=initial_cash_per_run,
                                ticker=symbol,
                                phase_name=phase.name,
                                start_date=phase.start_date,
                                end_date=phase.end_date,
                                final_cash=initial_cash_per_run,
                                final_equity=initial_cash_per_run,
                            ),
                        )
                    )
                continue

            # Compute features ONCE per symbol across spanning window (NFR-PERF-001, TEST-PERF-001)
            try:
                t_feat_start = time.perf_counter()
                cache = cls.build_symbol_feature_cache(
                    symbol, raw_candles, strategy, cache=active_cache, use_cache=use_cache
                )
                feature_compute_ms += (time.perf_counter() - t_feat_start) * 1000.0
                feature_compute_count += 1
            except Exception as exc:
                for phase in phases:
                    phase_results.append(
                        BatchPhaseResult(
                            symbol=symbol,
                            phase_name=phase.name,
                            start_date=phase.start_date,
                            end_date=phase.end_date,
                            status="failed",
                            total_candles=len(raw_candles),
                            initial_cash=initial_cash_per_run,
                            final_cash=initial_cash_per_run,
                            final_equity=initial_cash_per_run,
                            net_pnl=0.0,
                            net_return_pct=0.0,
                            total_trades=0,
                            open_position_quantity=0.0,
                            open_position_value=0.0,
                            error_message=f"Feature calculation failed: {exc}",
                            benchmark_metrics=calculate_benchmark_metrics(
                                trades=[],
                                initial_cash=initial_cash_per_run,
                                ticker=symbol,
                                phase_name=phase.name,
                                start_date=phase.start_date,
                                end_date=phase.end_date,
                                final_cash=initial_cash_per_run,
                                final_equity=initial_cash_per_run,
                            ),
                        )
                    )
                continue

            # 3. Simulate each phase from the precomputed cache with INDEPENDENT capital (FR-CORE-009)
            for phase in phases:
                df_phase, ind_phase = cache.slice_phase(phase)

                if df_phase.empty or len(df_phase) < 2:
                    phase_results.append(
                        BatchPhaseResult(
                            symbol=symbol,
                            phase_name=phase.name,
                            start_date=phase.start_date,
                            end_date=phase.end_date,
                            status="failed",
                            total_candles=len(df_phase),
                            initial_cash=initial_cash_per_run,
                            final_cash=initial_cash_per_run,
                            final_equity=initial_cash_per_run,
                            net_pnl=0.0,
                            net_return_pct=0.0,
                            total_trades=0,
                            open_position_quantity=0.0,
                            open_position_value=0.0,
                            error_message=f"Insufficient candles in phase slice ({len(df_phase)} candles)",
                            benchmark_metrics=calculate_benchmark_metrics(
                                trades=[],
                                initial_cash=initial_cash_per_run,
                                ticker=symbol,
                                phase_name=phase.name,
                                start_date=phase.start_date,
                                end_date=phase.end_date,
                                final_cash=initial_cash_per_run,
                                final_equity=initial_cash_per_run,
                            ),
                        )
                    )
                    continue

                kernel = BacktestExecutionKernel(
                    fee_rate=BUY_FEE_RATE,
                    tax_rate=SELL_TAX_RATE,
                    slippage_rate=slippage_rate,
                    min_holding_bars=2,
                    profile=profile,
                    exchange=exchange,
                )

                try:
                    t_sim_start = time.perf_counter()
                    kernel_res = kernel.run(
                        df=df_phase,
                        indicator_values=ind_phase,
                        strategy=strategy,
                        symbol=symbol,
                        initial_cash=float(initial_cash_per_run),  # Fresh independent capital
                        timeframe="1D",
                        exchange=exchange,
                    )
                    simulation_ms += (time.perf_counter() - t_sim_start) * 1000.0
                    simulation_run_count += 1

                    final_cash = kernel_res.final_cash
                    open_qty = kernel_res.final_position.quantity
                    last_close = float(df_phase.iloc[-1]["close"])
                    open_val = open_qty * last_close
                    final_eq = final_cash + open_val

                    net_pnl = final_eq - initial_cash_per_run
                    net_ret = net_pnl / initial_cash_per_run if initial_cash_per_run > 0 else 0.0
                    closed_trades = [t for t in kernel_res.trades if getattr(t, "status", "") == "closed"]

                    bench_row = calculate_benchmark_metrics(
                        trades=kernel_res.trades,
                        initial_cash=initial_cash_per_run,
                        ticker=symbol,
                        phase_name=phase.name,
                        start_date=phase.start_date,
                        end_date=phase.end_date,
                        final_cash=final_cash,
                        final_equity=final_eq,
                        open_position_quantity=open_qty,
                        open_position_value=open_val,
                        max_drawdown=kernel_res.ledger.max_drawdown,
                    )

                    phase_results.append(
                        BatchPhaseResult(
                            symbol=symbol,
                            phase_name=phase.name,
                            start_date=phase.start_date,
                            end_date=phase.end_date,
                            status="succeeded",
                            total_candles=len(df_phase),
                            initial_cash=initial_cash_per_run,
                            final_cash=final_cash,
                            final_equity=final_eq,
                            net_pnl=net_pnl,
                            net_return_pct=net_ret,
                            total_trades=len(closed_trades),  # Closed trades only (TEST-BT-005)
                            open_position_quantity=open_qty,
                            open_position_value=open_val,
                            warnings=list(kernel_res.warnings),
                            kernel_result=kernel_res,
                            benchmark_metrics=bench_row,
                        )
                    )
                except Exception as exc:
                    phase_results.append(
                        BatchPhaseResult(
                            symbol=symbol,
                            phase_name=phase.name,
                            start_date=phase.start_date,
                            end_date=phase.end_date,
                            status="failed",
                            total_candles=len(df_phase),
                            initial_cash=initial_cash_per_run,
                            final_cash=initial_cash_per_run,
                            final_equity=initial_cash_per_run,
                            net_pnl=0.0,
                            net_return_pct=0.0,
                            total_trades=0,
                            open_position_quantity=0.0,
                            open_position_value=0.0,
                            error_message=f"Simulation failed: {exc}",
                            benchmark_metrics=calculate_benchmark_metrics(
                                trades=[],
                                initial_cash=initial_cash_per_run,
                                ticker=symbol,
                                phase_name=phase.name,
                                start_date=phase.start_date,
                                end_date=phase.end_date,
                                final_cash=initial_cash_per_run,
                                final_equity=initial_cash_per_run,
                            ),
                        )
                    )

        # 4. Synthesize overall batch status and metrics summary
        succeeded = [r for r in phase_results if r.status == "succeeded"]
        failed = [r for r in phase_results if r.status == "failed"]

        if succeeded and not failed:
            overall_status = "succeeded"
        elif succeeded and failed:
            overall_status = "partial"
        else:
            overall_status = "failed"

        total_trades = sum(r.total_trades for r in succeeded)
        total_pnl = sum(r.net_pnl for r in succeeded)
        total_initial_capital = sum(r.initial_cash for r in succeeded)
        overall_return = total_pnl / total_initial_capital if total_initial_capital > 0 else 0.0

        summary = {
            "total_symbols": len(clean_symbols),
            "total_phases": len(phases),
            "total_runs": len(phase_results),
            "succeeded_runs": len(succeeded),
            "failed_runs": len(failed),
            "total_trades": total_trades,
            "total_net_pnl": round(total_pnl, 2),
            "overall_net_return_pct": round(overall_return, 4),
            "feature_compute_count": feature_compute_count,
            "simulation_run_count": simulation_run_count,
        }

        # Build PhaseMetricMatrix and CrossPhaseDegradation
        all_metrics = [r.benchmark_metrics for r in phase_results if r.benchmark_metrics is not None]
        phase_names = [p.name for p in phases]
        metric_matrix = build_phase_metric_matrix(phase_names, all_metrics) if all_metrics else None
        cross_phase_degradations = metric_matrix.cross_phase_degradations if metric_matrix else []
        markdown_table = BenchmarkTableRenderer.render_markdown_table(all_metrics) if all_metrics else None
        csv_export = BenchmarkTableRenderer.render_csv(all_metrics, formatted=True) if all_metrics else None

        total_duration_ms = (time.perf_counter() - start_time) * 1000.0
        cache_hits = (active_cache.stats()["hits"] - initial_hits) if (use_cache and active_cache.enabled) else 0
        cache_misses = (active_cache.stats()["misses"] - initial_misses) if (use_cache and active_cache.enabled) else 0

        timing_metrics = BatchTimingMetrics(
            total_duration_ms=total_duration_ms,
            feature_compute_ms=feature_compute_ms,
            simulation_ms=simulation_ms,
            cache_hits=cache_hits,
            cache_misses=cache_misses,
        )

        return BatchBacktestResult(
            status=overall_status,
            total_symbols=len(clean_symbols),
            total_phases=len(phases),
            total_runs=len(phase_results),
            feature_compute_count=feature_compute_count,
            simulation_run_count=simulation_run_count,
            phase_results=phase_results,
            summary=summary,
            metric_matrix=metric_matrix,
            cross_phase_degradations=cross_phase_degradations,
            markdown_table=markdown_table,
            csv_export=csv_export,
            timing_metrics=timing_metrics,
        )
