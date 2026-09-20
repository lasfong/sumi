"""Adapter parsing in-memory dictionaries, lists, or DataFrame fixtures into CanonicalBar sequences."""

from datetime import date, datetime
from typing import Any, Dict, List, Optional, Union
import pandas as pd

from app.domain.data.contracts import (
    AdjustmentType,
    CanonicalBar,
    DataQuality,
    FlowMethod,
    MarketDataCapabilityManifest,
    ValueSource,
    calculate_canonical_trading_value,
    validate_candle_bounds,
)
from app.utils.date_range import end_before, start_at


class FixtureMarketDataAdapter:
    """Provides deterministic CanonicalBar sequences from in-memory test fixtures or CSV data."""

    def __init__(self, records: Optional[List[Union[CanonicalBar, Dict[str, Any]]]] = None):
        self._bars: List[CanonicalBar] = []
        if records:
            for item in records:
                if isinstance(item, CanonicalBar):
                    self._bars.append(item)
                elif isinstance(item, dict):
                    self._bars.append(self._dict_to_canonical(item))

    def _dict_to_canonical(self, d: Dict[str, Any]) -> CanonicalBar:
        ts = d.get("timestamp")
        if isinstance(ts, str):
            ts_dt = pd.Timestamp(ts).to_pydatetime()
        elif isinstance(ts, datetime):
            ts_dt = ts
        else:
            ts_dt = pd.Timestamp(ts).to_pydatetime()

        s_date = d.get("session_date")
        if isinstance(s_date, str):
            s_date = date.fromisoformat(s_date)
        elif not isinstance(s_date, date):
            s_date = ts_dt.date()

        sym = str(d.get("symbol", "")).upper()
        op = float(d.get("open", 0.0))
        hp = float(d.get("high", 0.0))
        lp = float(d.get("low", 0.0))
        cp = float(d.get("close", 0.0))
        vol = float(d.get("volume", 0.0))

        reported_val = d.get("trading_value")
        if reported_val is not None:
            reported_val = float(reported_val)

        is_valid, _ = validate_candle_bounds(op, hp, lp, cp, vol)
        quality_str = d.get("quality", "HIGH" if is_valid else "INVALID")
        quality = DataQuality(quality_str) if isinstance(quality_str, str) else quality_str

        # Value source
        if "value_source" in d and isinstance(d["value_source"], (str, ValueSource)):
            val_src = ValueSource(d["value_source"])
            calc_val = reported_val if reported_val is not None else op * vol
        else:
            calc_val, val_src = calculate_canonical_trading_value(op, hp, lp, cp, vol, reported_val)

        flow_method_val = d.get("flow_method", FlowMethod.OHLCV_PROXY)
        flow_method = FlowMethod(flow_method_val) if isinstance(flow_method_val, str) else flow_method_val

        adj_type_val = d.get("adjustment_type", AdjustmentType.UNADJUSTED)
        adj_type = AdjustmentType(adj_type_val) if isinstance(adj_type_val, str) else adj_type_val

        return CanonicalBar(
            symbol=sym,
            timestamp=ts_dt,
            session_date=s_date,
            open=op,
            high=hp,
            low=lp,
            close=cp,
            volume=vol,
            trading_value=calc_val,
            value_source=val_src,
            flow_method=flow_method,
            adjustment_type=adj_type,
            quality=quality,
            source_provider=d.get("source_provider", "fixture"),
            raw_metadata=d.get("raw_metadata", {}),
        )

    def add_bar(self, bar: Union[CanonicalBar, Dict[str, Any]]) -> None:
        if isinstance(bar, CanonicalBar):
            self._bars.append(bar)
        elif isinstance(bar, dict):
            self._bars.append(self._dict_to_canonical(bar))

    def get_capability_manifest(self) -> MarketDataCapabilityManifest:
        return MarketDataCapabilityManifest(
            provider_name="fixture_adapter",
            supported_flow_methods=[FlowMethod.OHLCV_PROXY, FlowMethod.TICK_TEST_ESTIMATE_RESEARCH_ONLY],
            supported_value_sources=[ValueSource.ACTUAL_MATCHED_VALUE, ValueSource.ESTIMATED_TP_X_VOLUME],
            is_point_in_time=True,
            notes="In-memory test fixture provider for unit testing.",
        )

    def get_canonical_bars(
        self,
        symbol: str,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        as_of: Optional[str] = None,
        timeframe: str = "1D",
        limit: Optional[int] = None,
    ) -> List[CanonicalBar]:
        clean_sym = symbol.strip().upper()
        start_dt = start_at(start_date) if start_date else None
        end_dt = end_before(end_date) if end_date else None

        if as_of:
            as_of_dt = end_before(as_of)
            if end_dt is None or as_of_dt < end_dt:
                end_dt = as_of_dt

        matching: List[CanonicalBar] = []
        for b in self._bars:
            if b.symbol != clean_sym:
                continue
            if start_dt is not None and b.timestamp < start_dt:
                continue
            if end_dt is not None and b.timestamp >= end_dt:
                continue
            matching.append(b)

        matching.sort(key=lambda x: x.timestamp)
        if limit is not None and limit > 0:
            matching = matching[-limit:]
        return matching
