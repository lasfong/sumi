"""Chart adapter for Money Flow Blackbox (BB) series.

Transforms domain BBSymbolSeriesResult objects into lightweight, normalized
data structures suitable for UI charting, indicator overlays, and Replay rendering.
"""

from typing import Any, Dict, List

from app.domain.bb.contracts import BBSymbolSeriesResult


class BBChartAdapter:
    """Adapts BBSymbolSeriesResult for visualization in chart layers."""

    @staticmethod
    def to_chart_series(series: BBSymbolSeriesResult) -> Dict[str, Any]:
        """Convert BBSymbolSeriesResult into time-series records for charting."""
        records: List[Dict[str, Any]] = []

        for pt in series.points:
            rec: Dict[str, Any] = {
                "time": pt.date,
                "pressure": round(pt.daily_pressure, 4),
                "trading_value": round(pt.daily_trading_value, 2),
                "value_source": pt.daily_value_source.value,
                "quality": pt.daily_quality.value,
            }

            for horizon_code, h_pt in pt.horizons.items():
                key = horizon_code.lower()  # e.g. "t03", "t05", "t20"
                rec[key] = round(h_pt.bb_value, 2) if h_pt.bb_value is not None else None
                rec[f"{key}_dir"] = h_pt.direction.value
                rec[f"{key}_regime"] = h_pt.regime.value
                rec[f"{key}_run"] = h_pt.regime_run_length

            records.append(rec)

        return {
            "symbol": series.symbol,
            "flow_method": series.flow_method.value,
            "methodology_version": series.methodology_version,
            "label": "Technical Flow BB",
            "as_of": series.as_of,
            "total_bars": series.total_bars,
            "coverage_ratio": round(series.coverage_ratio, 4),
            "data": records,
        }
