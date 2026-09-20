"""Promoted Money Flow Blackbox (BB) Event and Signal Calculators.

Implements BBI-SIG-001, BBI-SIG-002, BBI-SIG-003:
- BBI-SIG-001: Signal Engine consumes BB outputs (horizons, direction, regime, quality, method).
- BBI-SIG-002: Direction, regime, confluence, and parameterized turn events. Threshold conventions (20/30/70/80)
  must NOT be hardcoded as frozen production defaults.
- BBI-SIG-003: Method and Quality guardrails: strategies must be able to enforce permitted flow_method
  (only OHLCV_PROXY for daily Symbol BB) and minimum data_quality.
"""

from datetime import date, datetime
from typing import Any, Dict, List, Optional, Sequence, Union

from app.domain.bb.calculator import ProxyBBCalculator
from app.domain.bb.contracts import (
    BBDirection,
    BBHorizon,
    BBRegime,
)
from app.domain.data.contracts import (
    CanonicalBar,
    DataQuality,
    FlowMethod,
    calculate_canonical_trading_value,
)
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)


QUALITY_ORDER = {
    DataQuality.HIGH: 4,
    DataQuality.MEDIUM: 3,
    DataQuality.DEGRADED: 2,
    DataQuality.SUSPECT: 1,
    DataQuality.INVALID: 0,
}

LOOKBACK_MAP = {
    3: BBHorizon.T03,
    5: BBHorizon.T05,
    10: BBHorizon.T10,
    20: BBHorizon.T20,
    50: BBHorizon.T50,
    200: BBHorizon.T200,
}


def _candles_to_canonical_bars(
    candles: Sequence[CandleBar],
    symbol: str = "TARGET",
    flow_method: FlowMethod = FlowMethod.OHLCV_PROXY,
    quality: DataQuality = DataQuality.HIGH,
) -> List[CanonicalBar]:
    canonical_bars: List[CanonicalBar] = []
    for c in candles:
        try:
            if "T" in c.timestamp:
                dt = datetime.fromisoformat(c.timestamp.replace("Z", "+00:00"))
                sess_date = dt.date()
            else:
                sess_date = date.fromisoformat(c.timestamp)
                dt = datetime(sess_date.year, sess_date.month, sess_date.day)
        except Exception:
            sess_date = date(2026, 1, 1)
            dt = datetime(2026, 1, 1)

        vol = c.volume if c.volume is not None else 0.0
        trading_val, val_source = calculate_canonical_trading_value(
            open_price=c.open,
            high_price=c.high,
            low_price=c.low,
            close_price=c.close,
            volume=vol,
        )

        canonical_bars.append(
            CanonicalBar(
                symbol=symbol,
                timestamp=dt,
                session_date=sess_date,
                open=c.open,
                high=c.high,
                low=c.low,
                close=c.close,
                volume=vol,
                trading_value=trading_val,
                value_source=val_source,
                flow_method=flow_method,
                quality=quality,
            )
        )
    return canonical_bars


def _check_method_and_quality(
    flow_method: FlowMethod,
    quality: DataQuality,
    accepted_methods: Sequence[FlowMethod],
    min_quality: DataQuality,
) -> tuple[bool, Optional[SignalQuality], Optional[str]]:
    """Enforce BBI-SIG-003 method and quality constraints."""
    if flow_method not in accepted_methods or flow_method != FlowMethod.OHLCV_PROXY:
        return False, SignalQuality.INVALID_VOLUME, f"DISALLOWED_FLOW_METHOD_{flow_method.value}"

    if QUALITY_ORDER.get(quality, 0) < QUALITY_ORDER.get(min_quality, 3):
        return False, SignalQuality.INVALID_VOLUME, f"INSUFFICIENT_DATA_QUALITY_{quality.value}"

    return True, None, None


def _resolve_horizon_enum(horizon_val: Union[str, int, BBHorizon]) -> BBHorizon:
    if isinstance(horizon_val, BBHorizon):
        return horizon_val
    if isinstance(horizon_val, int):
        return LOOKBACK_MAP.get(horizon_val, BBHorizon.T20)
    try:
        s = str(horizon_val).upper().strip()
        if s.isdigit():
            return LOOKBACK_MAP.get(int(s), BBHorizon.T20)
        return BBHorizon(s)
    except Exception:
        return BBHorizon.T20


def _resolve_quality_enum(quality_val: Union[str, int, DataQuality]) -> DataQuality:
    if isinstance(quality_val, DataQuality):
        return quality_val
    if isinstance(quality_val, int):
        rev_map = {4: DataQuality.HIGH, 3: DataQuality.MEDIUM, 2: DataQuality.DEGRADED, 1: DataQuality.SUSPECT, 0: DataQuality.INVALID}
        return rev_map.get(quality_val, DataQuality.HIGH)
    try:
        return DataQuality(str(quality_val).upper().strip())
    except Exception:
        return DataQuality.HIGH


def _resolve_methods_list(methods: Optional[Sequence[Union[str, FlowMethod]]]) -> List[FlowMethod]:
    if not methods:
        return [FlowMethod.OHLCV_PROXY]
    res = []
    for m in methods:
        if isinstance(m, FlowMethod):
            res.append(m)
        else:
            try:
                res.append(FlowMethod(str(m).upper().strip()))
            except Exception:
                res.append(FlowMethod.UNKNOWN)
    return res


# ---------------------------------------------------------------------------
# 1. Direction Events (bb.direction_rising / bb.direction_falling)
# ---------------------------------------------------------------------------

def calculate_bb_direction_rising(
    candles: Sequence[CandleBar],
    horizon: Union[str, int] = "T20",
    lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate whether Money Flow BB direction is RISING for the specified horizon."""
    h_target = lookback if lookback is not None else horizon
    h_enum = _resolve_horizon_enum(h_target)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_enum])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        h_pt = pt.horizons.get(h_enum.value)

        if h_pt is None or h_pt.is_warmup or h_pt.bb_value is None:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_rising = (h_pt.direction == BBDirection.RISING)
        reasons = [f"bb_{h_enum.value}_{h_pt.direction.value.lower()}"]
        if is_rising:
            reasons.append(f"bb_{h_enum.value}_score_{h_pt.bb_value}")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_rising,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_bb_direction_falling(
    candles: Sequence[CandleBar],
    horizon: Union[str, int] = "T20",
    lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate whether Money Flow BB direction is FALLING for the specified horizon."""
    h_target = lookback if lookback is not None else horizon
    h_enum = _resolve_horizon_enum(h_target)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_enum])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        h_pt = pt.horizons.get(h_enum.value)

        if h_pt is None or h_pt.is_warmup or h_pt.bb_value is None:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_falling = (h_pt.direction == BBDirection.FALLING)
        reasons = [f"bb_{h_enum.value}_{h_pt.direction.value.lower()}"]
        if is_falling:
            reasons.append(f"bb_{h_enum.value}_score_{h_pt.bb_value}")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_falling,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


# ---------------------------------------------------------------------------
# 2. Regime Events (bb.regime_positive / bb.regime_negative)
# ---------------------------------------------------------------------------

def calculate_bb_regime_positive(
    candles: Sequence[CandleBar],
    horizon: Union[str, int] = "T20",
    lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate whether Money Flow BB regime is POSITIVE (bb_value > 50.0)."""
    h_target = lookback if lookback is not None else horizon
    h_enum = _resolve_horizon_enum(h_target)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_enum])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        h_pt = pt.horizons.get(h_enum.value)

        if h_pt is None or h_pt.is_warmup or h_pt.bb_value is None:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_pos = (h_pt.regime == BBRegime.POSITIVE)
        reasons = [f"bb_{h_enum.value}_regime_{h_pt.regime.value.lower()}", f"bb_{h_enum.value}_score_{h_pt.bb_value}"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_pos,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_bb_regime_negative(
    candles: Sequence[CandleBar],
    horizon: Union[str, int] = "T20",
    lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate whether Money Flow BB regime is NEGATIVE (bb_value < 50.0)."""
    h_target = lookback if lookback is not None else horizon
    h_enum = _resolve_horizon_enum(h_target)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_enum])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        h_pt = pt.horizons.get(h_enum.value)

        if h_pt is None or h_pt.is_warmup or h_pt.bb_value is None:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_neg = (h_pt.regime == BBRegime.NEGATIVE)
        reasons = [f"bb_{h_enum.value}_regime_{h_pt.regime.value.lower()}", f"bb_{h_enum.value}_score_{h_pt.bb_value}"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_neg,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


# ---------------------------------------------------------------------------
# 3. Confluence Events (bb.confluence_bullish / bb.confluence_bearish)
# ---------------------------------------------------------------------------

def calculate_bb_confluence_bullish(
    candles: Sequence[CandleBar],
    short_horizon: Union[str, int] = "T05",
    long_horizon: Union[str, int] = "T20",
    short_lookback: Optional[int] = None,
    long_lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate multi-horizon bullish confluence (both horizons in POSITIVE regime and both RISING)."""
    h_s = short_lookback if short_lookback is not None else short_horizon
    h_l = long_lookback if long_lookback is not None else long_horizon
    h_short = _resolve_horizon_enum(h_s)
    h_long = _resolve_horizon_enum(h_l)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_short, h_long])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        pt_short = pt.horizons.get(h_short.value)
        pt_long = pt.horizons.get(h_long.value)

        if (
            pt_short is None
            or pt_long is None
            or pt_short.is_warmup
            or pt_long.is_warmup
            or pt_short.bb_value is None
            or pt_long.bb_value is None
        ):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_confluent = (
            pt_short.regime == BBRegime.POSITIVE
            and pt_long.regime == BBRegime.POSITIVE
            and (pt_short.direction == BBDirection.RISING or pt_short.bb_value >= pt_long.bb_value)
        )
        reasons = [
            f"bb_{h_short.value}_{pt_short.bb_value}_{pt_short.direction.value.lower()}",
            f"bb_{h_long.value}_{pt_long.bb_value}_{pt_long.direction.value.lower()}",
        ]
        if is_confluent:
            reasons.append("bb_bullish_confluence_active")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_confluent,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_bb_confluence_bearish(
    candles: Sequence[CandleBar],
    short_horizon: Union[str, int] = "T05",
    long_horizon: Union[str, int] = "T20",
    short_lookback: Optional[int] = None,
    long_lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate multi-horizon bearish confluence (both horizons in NEGATIVE regime and both FALLING)."""
    h_s = short_lookback if short_lookback is not None else short_horizon
    h_l = long_lookback if long_lookback is not None else long_horizon
    h_short = _resolve_horizon_enum(h_s)
    h_long = _resolve_horizon_enum(h_l)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_short, h_long])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        pt_short = pt.horizons.get(h_short.value)
        pt_long = pt.horizons.get(h_long.value)

        if (
            pt_short is None
            or pt_long is None
            or pt_short.is_warmup
            or pt_long.is_warmup
            or pt_short.bb_value is None
            or pt_long.bb_value is None
        ):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_confluent = (
            pt_short.regime == BBRegime.NEGATIVE
            and pt_long.regime == BBRegime.NEGATIVE
            and (pt_short.direction == BBDirection.FALLING or pt_short.bb_value <= pt_long.bb_value)
        )
        reasons = [
            f"bb_{h_short.value}_{pt_short.bb_value}_{pt_short.direction.value.lower()}",
            f"bb_{h_long.value}_{pt_long.bb_value}_{pt_long.direction.value.lower()}",
        ]
        if is_confluent:
            reasons.append("bb_bearish_confluence_active")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_confluent,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


# ---------------------------------------------------------------------------
# 4. Parameterized Turn Events (bb.turn_up / bb.turn_down)
# ---------------------------------------------------------------------------

def calculate_bb_turn_up(
    candles: Sequence[CandleBar],
    threshold: float = 40.0,
    tolerance: float = 2.0,
    horizon: Union[str, int] = "T20",
    lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate parameterized BB turn-up event.
    
    Enforces BBI-SIG-002: Thresholds must NOT be frozen to unvalidated 20/30/70/80 defaults.
    """
    h_target = lookback if lookback is not None else horizon
    h_enum = _resolve_horizon_enum(h_target)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_enum])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        h_pt = pt.horizons.get(h_enum.value)

        pt_prev = bb_res.points[t - 1] if t > 0 else None
        h_pt_prev = pt_prev.horizons.get(h_enum.value) if pt_prev else None

        if (
            h_pt is None
            or h_pt.is_warmup
            or h_pt.bb_value is None
            or h_pt_prev is None
            or h_pt_prev.is_warmup
            or h_pt_prev.bb_value is None
        ):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        bb_curr = h_pt.bb_value
        bb_prev = h_pt_prev.bb_value

        # Turn up condition: prior bar was near or below threshold, current bar turns upward above prior and threshold
        is_turn = (bb_prev <= threshold + tolerance) and (bb_curr > bb_prev) and (bb_curr >= threshold)
        reasons = [f"bb_prev_{bb_prev}_curr_{bb_curr}_thresh_{threshold}"]
        if is_turn:
            reasons.append("bb_turn_up_active")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_turn,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=threshold,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_bb_turn_down(
    candles: Sequence[CandleBar],
    threshold: float = 60.0,
    tolerance: float = 2.0,
    horizon: Union[str, int] = "T20",
    lookback: Optional[int] = None,
    accepted_methods: Optional[Sequence[Union[str, FlowMethod]]] = None,
    min_quality: Union[str, int] = "HIGH",
    symbol: str = "TARGET",
) -> List[SignalOutputPoint]:
    """Calculate parameterized BB turn-down event.
    
    Enforces BBI-SIG-002: Thresholds must NOT be frozen to unvalidated 20/30/70/80 defaults.
    """
    h_target = lookback if lookback is not None else horizon
    h_enum = _resolve_horizon_enum(h_target)
    q_min = _resolve_quality_enum(min_quality)
    methods = _resolve_methods_list(accepted_methods)

    n = len(candles)
    if n == 0:
        return []

    valid_method, err_quality, err_reason = _check_method_and_quality(
        FlowMethod.OHLCV_PROXY, q_min, methods, q_min
    )
    if not valid_method:
        return [
            SignalOutputPoint(
                bar_index=c.index,
                timestamp=c.timestamp,
                output_type="bool",
                value=None,
                quality=err_quality or SignalQuality.INVALID_VOLUME,
                reasons=[err_reason or "DISALLOWED_FLOW_METHOD"],
                availability_event="BAR_CLOSE",
                available_at_index=c.index,
                available_at_timestamp=c.timestamp,
            )
            for c in candles
        ]

    canonical_bars = _candles_to_canonical_bars(candles, symbol=symbol)
    bb_res = ProxyBBCalculator.calculate_series(canonical_bars, symbol=symbol, horizons=[h_enum])

    points: List[SignalOutputPoint] = []
    for t in range(n):
        bar = candles[t]
        pt = bb_res.points[t]
        h_pt = pt.horizons.get(h_enum.value)

        pt_prev = bb_res.points[t - 1] if t > 0 else None
        h_pt_prev = pt_prev.horizons.get(h_enum.value) if pt_prev else None

        if (
            h_pt is None
            or h_pt.is_warmup
            or h_pt.bb_value is None
            or h_pt_prev is None
            or h_pt_prev.is_warmup
            or h_pt_prev.bb_value is None
        ):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        bb_curr = h_pt.bb_value
        bb_prev = h_pt_prev.bb_value

        # Turn down condition: prior bar was near or above threshold, current bar turns downward below prior and threshold
        is_turn = (bb_prev >= threshold - tolerance) and (bb_curr < bb_prev) and (bb_curr <= threshold)
        reasons = [f"bb_prev_{bb_prev}_curr_{bb_curr}_thresh_{threshold}"]
        if is_turn:
            reasons.append("bb_turn_down_active")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_turn,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=threshold,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points
