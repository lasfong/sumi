"""Candle Structure scoring calculation algorithms (SIG-STR-001).

Strictly pure functions operating on CandleBar primitives.
Provides transparent, deterministic candle scoring in [-100.0, +100.0]
without black-box model fitting or unverified order-flow assumptions.
"""

from typing import Any, Dict, List, Optional

from app.domain.signals.candle_features import calculate_candle_geometry
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)


def calculate_candle_structure_score(
    candles: List[CandleBar],
    w_direction: float = 0.45,
    w_close: float = 0.35,
    w_wick: float = 0.20,
) -> List[SignalOutputPoint]:
    """SIG-STR-001: Transparent Candle Structure Score in [-100.0, +100.0].
    
    Formula:
    - direction = +1 if close > open, -1 if close < open, else 0
    - close_score = 2 * close_location - 1  # [-1, +1]
    - wick_balance = (lower_wick - upper_wick) / range  # [-1, +1]
    - raw = w_direction * direction * body_ratio + w_close * close_score + w_wick * wick_balance
    - score = 100 * clip(raw, -1.0, +1.0)
    
    Protects against flat candles (range <= 1e-8) by producing neutral 0.0.
    """
    points: List[SignalOutputPoint] = []
    total_w = w_direction + w_close + w_wick
    if total_w <= 1e-8:
        norm_w_dir = 0.45
        norm_w_close = 0.35
        norm_w_wick = 0.20
    else:
        norm_w_dir = w_direction / total_w
        norm_w_close = w_close / total_w
        norm_w_wick = w_wick / total_w

    for bar in candles:
        geom = calculate_candle_geometry(bar)

        if geom.range <= 1e-8:
            # Flat candle / zero-range bar protection
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=0.0,
                    quality=SignalQuality.VALID,
                    reasons=["ZERO_RANGE_CANDLE", "NEUTRAL_STRUCTURE"],
                    threshold=0.0,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        direction = 1.0 if bar.close > bar.open else (-1.0 if bar.close < bar.open else 0.0)
        close_score = 2.0 * geom.close_location - 1.0
        wick_balance = max(-1.0, min(1.0, (geom.lower_wick - geom.upper_wick) / geom.range))

        raw = (
            norm_w_dir * direction * geom.body_ratio
            + norm_w_close * close_score
            + norm_w_wick * wick_balance
        )
        score = round(100.0 * max(-1.0, min(1.0, raw)), 2)

        reasons = [
            f"SCORE_{score:.1f}",
            f"DIR_{int(direction)}",
            f"BODY_{geom.body_ratio:.2f}",
            f"CLOSE_LOC_{geom.close_location:.2f}",
            f"WICK_BAL_{wick_balance:.2f}",
        ]
        if score >= 40.0:
            reasons.append("GOOD_CANDLE_STRUCTURE")
        elif score <= -40.0:
            reasons.append("POOR_CANDLE_STRUCTURE")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="float",
                value=score,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=0.0,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points
