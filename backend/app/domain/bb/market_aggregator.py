"""Pure domain aggregator and calculation engine for Market Blackbox and Flow Breadth.

Implements:
- Aggregate-before-ratio: Sums constituent active buy and sell values across eligible universe
  members before computing rolling horizon ratio (AT09).
- Anti-averaging principle: Guarantees market money flow is trading-value weighted, strictly
  forbidding simple averaging of symbol scores (AT10).
- Universe Point-in-Time (PIT) boundary enforcement: Constituents contribute only during
  active eligibility intervals (AT11).
- Missing data handling: Missing vendor bars drop coverage rather than injecting zero values (AT12).
- Independent Flow Breadth: Measures participation across count and trading value independently
  from Market BB score (AT17).
- Publication Gate: Blocks canonical publication for candidate/retrospective universes and low
  constituent coverage (<50%), enforcing explicit status and survivor caveats.
"""

from __future__ import annotations

from datetime import date
from typing import Dict, List, Optional, Sequence, Set, Tuple

from app.domain.bb.calculator import ProxyBBCalculator
from app.domain.bb.contracts import (
    BBDirection,
    BBHorizon,
    BBRegime,
    FlowBreadthHorizonPoint,
    FlowMethod,
    MarketBBHorizonPoint,
    MarketBBSessionPoint,
    MarketBBSeriesResult,
    MarketPublicationStatus,
)
from app.domain.data.contracts import CanonicalBar, DataQuality
from app.domain.universe.models import (
    CoverageStatus,
    UniverseCoverageReport,
    UniverseDefinition,
    UniverseMode,
    UniverseStatus,
)
from app.domain.universe.resolver import UniverseResolver


class MarketBBAggregator:
    """Pure domain calculator for whole-market money flow aggregation and flow breadth."""

    METHODOLOGY_VERSION = "bb_v1_market_aggregate"
    FLOW_METHOD = FlowMethod.OHLCV_PROXY

    @staticmethod
    def decompose_bar_values(
        bar: CanonicalBar,
        prev_close: Optional[float] = None,
    ) -> Tuple[float, float, float]:
        """Decompose a single canonical bar into active Buy and Sell values under OHLCV_PROXY.

        Returns:
            (buy_value, sell_value, pressure)
            guaranteeing buy_value + sell_value == bar.trading_value
            and buy_value - sell_value == pressure * bar.trading_value.
        """
        if prev_close is None:
            true_high = bar.high
            true_low = bar.low
        else:
            true_high = max(bar.high, prev_close)
            true_low = min(bar.low, prev_close)

        tr_range = true_high - true_low
        if tr_range <= 0.0:
            pressure = 0.0
        else:
            raw_p = (2.0 * bar.close - true_high - true_low) / tr_range
            pressure = max(-1.0, min(1.0, raw_p))

        val = bar.trading_value
        if val <= 0.0:
            return 0.0, 0.0, pressure

        buy_value = 0.5 * (1.0 + pressure) * val
        sell_value = 0.5 * (1.0 - pressure) * val
        return buy_value, sell_value, pressure

    @classmethod
    def evaluate_publication_gate(
        cls,
        universe: UniverseDefinition,
        eval_mode: UniverseMode,
        coverage_report: UniverseCoverageReport,
    ) -> Tuple[MarketPublicationStatus, List[str]]:
        """Evaluate publication gate criteria for market-level metrics."""
        reasons: List[str] = []

        # 1. Check coverage thresholds
        if coverage_report.status == CoverageStatus.FAILED or coverage_report.target_count == 0:
            reasons.append(
                f"Coverage failed minimum threshold ({coverage_report.coverage_ratio * 100:.1f}% "
                f"available: {coverage_report.available_count}/{coverage_report.target_count})."
            )
            return MarketPublicationStatus.UNAVAILABLE_DEGRADED, reasons

        # 2. Check retrospective mode
        if eval_mode == UniverseMode.RETROSPECTIVE_FIXED:
            reasons.append(
                "Evaluated under RETROSPECTIVE_FIXED mode; carries survivorship bias and "
                "cannot be published as canonical point-in-time market truth."
            )

        # 3. Check universe candidate status
        if universe.status != UniverseStatus.APPROVED:
            reasons.append(
                f"Universe '{universe.universe_id}' status is '{universe.status.value}', "
                f"not APPROVED canonical universe."
            )

        # 4. Check degraded coverage
        if coverage_report.status == CoverageStatus.DEGRADED:
            reasons.append(
                f"Coverage is DEGRADED ({coverage_report.coverage_ratio * 100:.1f}% < 80.0%)."
            )

        if reasons:
            return MarketPublicationStatus.RESEARCH_RETROSPECTIVE, reasons

        return MarketPublicationStatus.CANONICAL_PUBLISHED, []

    @classmethod
    def calculate_market_bb(
        cls,
        universe: UniverseDefinition,
        symbol_bars_map: Dict[str, Sequence[CanonicalBar]],
        mode: Optional[UniverseMode] = None,
        horizons: Optional[Sequence[BBHorizon]] = None,
        as_of: Optional[str] = None,
        breadth_buffer: float = 0.0,
        coverage_threshold: float = 0.80,
    ) -> MarketBBSeriesResult:
        """Calculate whole-market aggregate Money Flow BB and independent Flow Breadth."""
        eval_mode = mode if mode is not None else universe.default_mode
        active_horizons = list(horizons) if horizons else [
            BBHorizon.T03, BBHorizon.T05, BBHorizon.T10, BBHorizon.T20, BBHorizon.T50, BBHorizon.T200
        ]

        # 1. Filter and index symbol bars by date
        symbol_bars_by_date: Dict[str, Dict[str, CanonicalBar]] = {}
        all_dates_set: Set[str] = set()

        def _to_date_str(val: Any) -> str:
            if hasattr(val, "isoformat"):
                return val.isoformat()
            return str(val)

        for sym, bars in symbol_bars_map.items():
            sorted_bars = sorted(bars, key=lambda b: (_to_date_str(b.session_date), str(b.timestamp)))
            if as_of is not None:
                as_of_str = _to_date_str(as_of)
                sorted_bars = [b for b in sorted_bars if _to_date_str(b.session_date) <= as_of_str]
            date_dict = {_to_date_str(b.session_date): b for b in sorted_bars}
            symbol_bars_by_date[sym.upper().strip()] = date_dict
            all_dates_set.update(date_dict.keys())

        chronological_dates = sorted(list(all_dates_set))
        if not chronological_dates:
            return MarketBBSeriesResult(
                universe_id=universe.universe_id,
                universe_version=universe.version,
                universe_mode=eval_mode.value,
                is_point_in_time=(eval_mode == UniverseMode.POINT_IN_TIME),
                publication_status=MarketPublicationStatus.UNAVAILABLE_DEGRADED,
                flow_method=cls.FLOW_METHOD,
                methodology_version=cls.METHODOLOGY_VERSION,
                as_of=as_of,
                survivor_bias_caveat=None,
                points=[],
                gate_reasons=["No market data available for calculation."],
            )

        # 2. Precompute single-symbol Blackbox series to supply individual scores for Breadth
        symbol_bb_results: Dict[str, Any] = {}
        for sym, bars in symbol_bars_map.items():
            norm_sym = sym.upper().strip()
            res = ProxyBBCalculator.calculate_series(
                bars=bars,
                symbol=norm_sym,
                horizons=active_horizons,
                as_of=as_of,
            )
            # Map by session date
            symbol_bb_results[norm_sym] = {p.date: p for p in res.points}

        # 3. For each calendar date, resolve active universe constituents and compute daily market values
        daily_market_buy: List[float] = []
        daily_market_sell: List[float] = []
        daily_market_total: List[float] = []
        session_resolutions: List[Any] = []
        session_coverages: List[UniverseCoverageReport] = []

        for d_str in chronological_dates:
            d_obj = d_str if isinstance(d_str, date) else date.fromisoformat(d_str)
            resolution = UniverseResolver.resolve(universe, as_of=d_obj, mode=eval_mode)
            session_resolutions.append(resolution)

            # Check which active symbols have data on this date
            reporting_symbols = [
                s for s in resolution.active_symbols
                if s in symbol_bars_by_date and d_str in symbol_bars_by_date[s]
            ]
            cov = UniverseResolver.check_coverage(
                resolution, set(reporting_symbols), threshold=coverage_threshold
            )
            session_coverages.append(cov)

            # Aggregate Buy and Sell value across reporting active constituents (AT09)
            m_buy = 0.0
            m_sell = 0.0
            for s in reporting_symbols:
                bar = symbol_bars_by_date[s][d_str]
                # Find previous close for symbol s if available
                # Get all dates for symbol s prior to d_str
                s_prev_close = None
                s_dates = [sd for sd in symbol_bars_by_date[s].keys() if sd < d_str]
                if s_dates:
                    last_date = max(s_dates)
                    s_prev_close = symbol_bars_by_date[s][last_date].close

                b_val, s_val, _ = cls.decompose_bar_values(bar, prev_close=s_prev_close)
                m_buy += b_val
                m_sell += s_val

            daily_market_buy.append(m_buy)
            daily_market_sell.append(m_sell)
            daily_market_total.append(m_buy + m_sell)

        # 4. Compute rolling Market BB and Flow Breadth across horizons
        horizon_prev_score: Dict[BBHorizon, Optional[float]] = {h: None for h in active_horizons}
        session_points: List[MarketBBSessionPoint] = []
        n_sessions = len(chronological_dates)

        for t in range(n_sessions):
            d_str = chronological_dates[t]
            resolution = session_resolutions[t]
            cov = session_coverages[t]

            horizons_dict: Dict[str, MarketBBHorizonPoint] = {}
            breadth_dict: Dict[str, FlowBreadthHorizonPoint] = {}

            for h in active_horizons:
                lookback = h.lookback_bars
                window_start = t - lookback + 1

                # Check warmup (AT08)
                if window_start < 0:
                    pt = MarketBBHorizonPoint(
                        horizon=h,
                        market_bb_value=None,
                        market_buy_value=sum(daily_market_buy[:t + 1]),
                        market_sell_value=sum(daily_market_sell[:t + 1]),
                        market_total_value=sum(daily_market_total[:t + 1]),
                        market_oib=None,
                        is_warmup=True,
                        direction=BBDirection.UNKNOWN,
                        regime=BBRegime.UNKNOWN,
                        quality=DataQuality.HIGH,
                    )
                else:
                    rolling_buy = sum(daily_market_buy[window_start:t + 1])
                    rolling_sell = sum(daily_market_sell[window_start:t + 1])
                    rolling_total = rolling_buy + rolling_sell

                    if rolling_total <= 0.0:
                        # Zero activity (AT05)
                        score = None
                        oib = None
                        regime = BBRegime.NEUTRAL
                    else:
                        # Market BB Formula (AT09): 100 * Buy / Total
                        score = 100.0 * (rolling_buy / rolling_total)
                        oib = (rolling_buy - rolling_sell) / rolling_total
                        regime = BBRegime.POSITIVE if score > 50.0 else (
                            BBRegime.NEGATIVE if score < 50.0 else BBRegime.NEUTRAL
                        )

                    # Direction causality
                    prev_score = horizon_prev_score[h]
                    if prev_score is None or score is None:
                        direction = BBDirection.UNKNOWN
                    elif score > prev_score:
                        direction = BBDirection.RISING
                    elif score < prev_score:
                        direction = BBDirection.FALLING
                    else:
                        direction = BBDirection.FLAT

                    if score is not None:
                        horizon_prev_score[h] = score

                    pt = MarketBBHorizonPoint(
                        horizon=h,
                        market_bb_value=score,
                        market_buy_value=rolling_buy,
                        market_sell_value=rolling_sell,
                        market_total_value=rolling_total,
                        market_oib=oib,
                        is_warmup=False,
                        direction=direction,
                        regime=regime,
                        quality=DataQuality.HIGH,
                    )

                horizons_dict[h.value] = pt

                # 5. Calculate Independent Flow Breadth for horizon h (AT17)
                breadth_pt = cls._calculate_breadth(
                    horizon=h,
                    d_str=d_str,
                    active_symbols=resolution.active_symbols,
                    symbol_bb_results=symbol_bb_results,
                    buffer=breadth_buffer,
                )
                breadth_dict[h.value] = breadth_pt

            session_pt = MarketBBSessionPoint(
                date=d_str,
                horizons=horizons_dict,
                breadth=breadth_dict,
                symbol_coverage=cov.coverage_ratio,
                value_coverage=1.0,  # Proxy uses full observed member value
                reporting_symbols_count=cov.available_count,
                total_target_count=cov.target_count,
                coverage_status=cov.status.value,
            )
            session_points.append(session_pt)

        # 6. Evaluate series-level publication gate
        latest_coverage = session_coverages[-1]
        pub_status, gate_reasons = cls.evaluate_publication_gate(
            universe=universe,
            eval_mode=eval_mode,
            coverage_report=latest_coverage,
        )

        survivor_caveat = None
        if eval_mode == UniverseMode.RETROSPECTIVE_FIXED:
            survivor_caveat = (
                f"RETROSPECTIVE_FIXED MODE: Evaluated on fixed universe snapshot. "
                f"Carries survivorship bias and must not be used as canonical point-in-time market truth."
            )

        return MarketBBSeriesResult(
            universe_id=universe.universe_id,
            universe_version=universe.version,
            universe_mode=eval_mode.value,
            is_point_in_time=(eval_mode == UniverseMode.POINT_IN_TIME),
            publication_status=pub_status,
            flow_method=cls.FLOW_METHOD,
            methodology_version=cls.METHODOLOGY_VERSION,
            as_of=as_of,
            survivor_bias_caveat=survivor_caveat,
            points=session_points,
            gate_reasons=gate_reasons,
        )

    @classmethod
    def _calculate_breadth(
        cls,
        horizon: BBHorizon,
        d_str: str,
        active_symbols: Sequence[str],
        symbol_bb_results: Dict[str, Dict[str, Any]],
        buffer: float = 0.0,
    ) -> FlowBreadthHorizonPoint:
        """Compute count and trading-value breadth for a given session and horizon (AT17)."""
        pos_count = 0
        neu_count = 0
        neg_count = 0

        pos_val = 0.0
        neu_val = 0.0
        neg_val = 0.0

        for sym in active_symbols:
            if sym in symbol_bb_results and d_str in symbol_bb_results[sym]:
                sym_pt = symbol_bb_results[sym][d_str]
                h_pt = sym_pt.horizons.get(horizon.value)
                if h_pt and h_pt.bb_value is not None and not h_pt.is_warmup:
                    score = h_pt.bb_value
                    # Weight by rolling denominator (rolling trading value over horizon)
                    v = h_pt.raw_denominator or sym_pt.daily_trading_value

                    if score > 50.0 + buffer:
                        pos_count += 1
                        pos_val += v
                    elif score < 50.0 - buffer:
                        neg_count += 1
                        neg_val += v
                    else:
                        neu_count += 1
                        neu_val += v

        total_count = pos_count + neu_count + neg_count
        total_val = pos_val + neu_val + neg_val

        pos_c_ratio = (pos_count / total_count) if total_count > 0 else 0.0
        neu_c_ratio = (neu_count / total_count) if total_count > 0 else 0.0
        neg_c_ratio = (neg_count / total_count) if total_count > 0 else 0.0

        pos_v_ratio = (pos_val / total_val) if total_val > 0.0 else 0.0
        neu_v_ratio = (neu_val / total_val) if total_val > 0.0 else 0.0
        neg_v_ratio = (neg_val / total_val) if total_val > 0.0 else 0.0

        return FlowBreadthHorizonPoint(
            horizon=horizon,
            positive_count=pos_count,
            neutral_count=neu_count,
            negative_count=neg_count,
            total_count=total_count,
            positive_count_ratio=pos_c_ratio,
            neutral_count_ratio=neu_c_ratio,
            negative_count_ratio=neg_c_ratio,
            positive_value=pos_val,
            neutral_value=neu_val,
            negative_value=neg_val,
            total_value=total_val,
            positive_value_ratio=pos_v_ratio,
            neutral_value_ratio=neu_v_ratio,
            negative_value_ratio=neg_v_ratio,
            buffer=buffer,
        )
