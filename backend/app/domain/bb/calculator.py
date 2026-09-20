"""Pure Money Flow Blackbox (BB) calculation engine using OHLCV_PROXY method.

Implements the authoritative v3 OHLCV_PROXY formula:
1. True Range:
   TH[t] = max(High[t], Close[t-1])
   TL[t] = min(Low[t], Close[t-1])
2. Pressure:
   P[t] = (2 * Close[t] - TH[t] - TL[t]) / (TH[t] - TL[t])  # in [-1, +1]; if denom==0 -> 0
3. Signed Value:
   SP[t] = P[t] * TradingValue[t]
4. Rolling Horizon H:
   oib_raw = sum_H(SP) / sum_H(TradingValue)  # in [-1, +1]; if denom==0 -> None
   ProxyBB_H = 50 * (1 + oib_raw)            # in [0, 100]

Strictly compliant with:
- AT01: Bounds [0, 100] and [-1, +1]
- AT05: Zero activity yields null, never silently 50
- AT06: Rolling window (not block)
- AT07: Future invariance
- AT08: Warm-up null policy
- AT13: Method boundary & version metadata
- AT15: True range formula & zero-range candle handling
- AT16: Value source traceability
- AT18: Direction & regime causality
- AT20: Reproducibility
"""

from typing import List, Optional, Sequence

from app.domain.bb.contracts import (
    BBDirection,
    BBHorizon,
    BBRegime,
    BBSymbolHorizonPoint,
    BBSymbolPoint,
    BBSymbolSeriesResult,
)
from app.domain.data.contracts import CanonicalBar, DataQuality, FlowMethod, ValueSource


class ProxyBBCalculator:
    """Deterministic, pure calculation engine for OHLCV_PROXY Money Flow Blackbox."""

    METHODOLOGY_VERSION = "bb_v1_ohlcv_proxy"
    FLOW_METHOD = FlowMethod.OHLCV_PROXY

    @classmethod
    def calculate_series(
        cls,
        bars: Sequence[CanonicalBar],
        symbol: str,
        horizons: Optional[Sequence[BBHorizon]] = None,
        as_of: Optional[str] = None,
    ) -> BBSymbolSeriesResult:
        """Calculate multi-horizon Blackbox series from canonical bars.
        
        Args:
            bars: Chronologically sorted sequence of CanonicalBar records.
            symbol: Ticker symbol.
            horizons: Target horizons to compute (defaults to T03, T05, T10, T20, T50, T200).
            as_of: Optional ISO date string cutoff (inclusive calendar date).
            
        Returns:
            BBSymbolSeriesResult with complete point history and metadata.
        """
        if not bars:
            return BBSymbolSeriesResult(
                symbol=symbol,
                flow_method=cls.FLOW_METHOD,
                methodology_version=cls.METHODOLOGY_VERSION,
                as_of=as_of,
                requested_horizons=list(horizons) if horizons else [],
                points=[],
                total_bars=0,
                coverage_ratio=0.0,
            )

        # Enforce chronological ordering
        sorted_bars = sorted(bars, key=lambda b: (b.session_date, b.timestamp))

        # Enforce as_of cutoff if requested (AT07 future invariance)
        if as_of is not None:
            sorted_bars = [b for b in sorted_bars if b.session_date <= as_of]

        if not sorted_bars:
            return BBSymbolSeriesResult(
                symbol=symbol,
                flow_method=cls.FLOW_METHOD,
                methodology_version=cls.METHODOLOGY_VERSION,
                as_of=as_of,
                requested_horizons=list(horizons) if horizons else [],
                points=[],
                total_bars=0,
                coverage_ratio=0.0,
            )

        active_horizons = list(horizons) if horizons else [
            BBHorizon.T03, BBHorizon.T05, BBHorizon.T10, BBHorizon.T20, BBHorizon.T50, BBHorizon.T200
        ]

        # 1. Compute single-session primitives
        pressures: List[float] = []
        trading_values: List[float] = []
        signed_pressures: List[float] = []

        n = len(sorted_bars)
        for i, bar in enumerate(sorted_bars):
            if i == 0:
                true_high = bar.high
                true_low = bar.low
            else:
                prev_close = sorted_bars[i - 1].close
                true_high = max(bar.high, prev_close)
                true_low = min(bar.low, prev_close)

            tr_range = true_high - true_low
            if tr_range <= 0.0:
                # Flat bar / zero-range candle (AT15)
                pressure = 0.0
            else:
                raw_p = (2.0 * bar.close - true_high - true_low) / tr_range
                # Strict clamp to [-1.0, 1.0] (AT01, AT15)
                pressure = max(-1.0, min(1.0, raw_p))

            val = bar.trading_value
            sp = pressure * val

            pressures.append(pressure)
            trading_values.append(val)
            signed_pressures.append(sp)

        # 2. Compute rolling metrics per horizon across all sessions
        # Keep track of state for direction and regime run length
        horizon_prev_score: dict[BBHorizon, Optional[float]] = {h: None for h in active_horizons}
        horizon_prev_regime: dict[BBHorizon, BBRegime] = {h: BBRegime.UNKNOWN for h in active_horizons}
        horizon_run_length: dict[BBHorizon, int] = {h: 0 for h in active_horizons}

        result_points: List[BBSymbolPoint] = []

        for t in range(n):
            current_bar = sorted_bars[t]
            horizon_points_dict: dict[str, BBSymbolHorizonPoint] = {}

            for h in active_horizons:
                lookback = h.lookback_bars
                window_start = t - lookback + 1

                # AT08 Warm-up: requires lookback valid sessions
                if window_start < 0:
                    pt = BBSymbolHorizonPoint(
                        horizon=h,
                        bb_value=None,
                        oib_raw=None,
                        raw_numerator=None,
                        raw_denominator=None,
                        is_warmup=True,
                        direction=BBDirection.UNKNOWN,
                        regime=BBRegime.UNKNOWN,
                        regime_run_length=0,
                        value_source=current_bar.value_source,
                        quality=current_bar.quality,
                    )
                    horizon_points_dict[h.value] = pt
                    continue

                # Valid rolling window slice [window_start : t + 1] (AT06)
                num_sum = sum(signed_pressures[window_start : t + 1])
                den_sum = sum(trading_values[window_start : t + 1])

                # Window-level value source and quality
                window_bars = sorted_bars[window_start : t + 1]
                if all(b.value_source == ValueSource.ACTUAL_MATCHED_VALUE for b in window_bars):
                    w_source = ValueSource.ACTUAL_MATCHED_VALUE
                elif any(b.value_source == ValueSource.ESTIMATED_TP_X_VOLUME for b in window_bars):
                    w_source = ValueSource.ESTIMATED_TP_X_VOLUME
                elif any(b.value_source == ValueSource.ESTIMATED_CLOSE_X_VOLUME for b in window_bars):
                    w_source = ValueSource.ESTIMATED_CLOSE_X_VOLUME
                else:
                    w_source = ValueSource.UNAVAILABLE

                # Aggregate quality: lowest quality present in window
                w_quality = cls._aggregate_window_quality(window_bars)

                # AT05 Zero activity: if denominator == 0, BB is null, never silently 50
                if den_sum <= 0.0:
                    pt = BBSymbolHorizonPoint(
                        horizon=h,
                        bb_value=None,
                        oib_raw=None,
                        raw_numerator=num_sum,
                        raw_denominator=0.0,
                        is_warmup=False,
                        direction=BBDirection.FLAT,
                        regime=BBRegime.NEUTRAL,
                        regime_run_length=0,
                        value_source=w_source,
                        quality=w_quality,
                    )
                    horizon_points_dict[h.value] = pt
                    horizon_prev_score[h] = None
                    horizon_prev_regime[h] = BBRegime.NEUTRAL
                    horizon_run_length[h] = 0
                    continue

                # Standard calculation
                oib_raw = max(-1.0, min(1.0, num_sum / den_sum))
                bb_val = max(0.0, min(100.0, 50.0 * (1.0 + oib_raw)))  # AT01 bounds

                # Direction: compared to previous session's valid score (AT18 causality)
                prev_val = horizon_prev_score[h]
                if prev_val is None:
                    direction = BBDirection.FLAT
                elif bb_val > prev_val + 1e-7:
                    direction = BBDirection.RISING
                elif bb_val < prev_val - 1e-7:
                    direction = BBDirection.FALLING
                else:
                    direction = BBDirection.FLAT

                # Regime: baseline around 50.0
                if bb_val > 50.0 + 1e-7:
                    regime = BBRegime.POSITIVE
                elif bb_val < 50.0 - 1e-7:
                    regime = BBRegime.NEGATIVE
                else:
                    regime = BBRegime.NEUTRAL

                # Regime run length
                prev_reg = horizon_prev_regime[h]
                if regime == prev_reg:
                    run_len = horizon_run_length[h] + 1
                else:
                    run_len = 1

                # Update trackers
                horizon_prev_score[h] = bb_val
                horizon_prev_regime[h] = regime
                horizon_run_length[h] = run_len

                pt = BBSymbolHorizonPoint(
                    horizon=h,
                    bb_value=bb_val,
                    oib_raw=oib_raw,
                    raw_numerator=num_sum,
                    raw_denominator=den_sum,
                    is_warmup=False,
                    direction=direction,
                    regime=regime,
                    regime_run_length=run_len,
                    value_source=w_source,
                    quality=w_quality,
                )
                horizon_points_dict[h.value] = pt

            sym_pt = BBSymbolPoint(
                date=current_bar.session_date,
                horizons=horizon_points_dict,
                daily_pressure=pressures[t],
                daily_trading_value=trading_values[t],
                daily_value_source=current_bar.value_source,
                daily_quality=current_bar.quality,
            )
            result_points.append(sym_pt)

        coverage = len(result_points) / max(1, len(sorted_bars))

        return BBSymbolSeriesResult(
            symbol=symbol,
            flow_method=cls.FLOW_METHOD,
            methodology_version=cls.METHODOLOGY_VERSION,
            as_of=as_of,
            requested_horizons=active_horizons,
            points=result_points,
            total_bars=len(result_points),
            coverage_ratio=coverage,
        )

    @staticmethod
    def _aggregate_window_quality(window_bars: Sequence[CanonicalBar]) -> DataQuality:
        """Determines the conservative composite quality for a window."""
        priorities = {
            DataQuality.INVALID: 1,
            DataQuality.SUSPECT: 2,
            DataQuality.DEGRADED: 3,
            DataQuality.MEDIUM: 4,
            DataQuality.HIGH: 5,
        }
        min_p = min(priorities.get(b.quality, 5) for b in window_bars)
        for q, p in priorities.items():
            if p == min_p:
                return q
        return DataQuality.HIGH
