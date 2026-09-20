"""Adapter translating Doraemon market data API responses or payloads into CanonicalBar sequences."""

from datetime import date, datetime
import json
import os
import ssl
from typing import Any, Dict, List, Optional
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

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


class DoraemonMarketDataAdapter:
    """Adapts Doraemon market price history responses to MarketDataPort."""

    def __init__(
        self,
        base_url: Optional[str] = None,
        api_key: Optional[str] = None,
        timeout_seconds: float = 15.0,
    ):
        self._base_url = (base_url or os.environ.get("DORAEMON_API_URL", "https://api.hieupb.io.vn/api/v1")).rstrip("/")
        self._api_key = api_key or os.environ.get("DORAEMON_API_KEY", "")
        self._timeout = timeout_seconds

    def get_capability_manifest(self) -> MarketDataCapabilityManifest:
        return MarketDataCapabilityManifest(
            provider_name="doraemon_market_api",
            supported_flow_methods=[FlowMethod.OHLCV_PROXY],
            supported_value_sources=[ValueSource.ACTUAL_MATCHED_VALUE, ValueSource.ESTIMATED_TP_X_VOLUME],
            is_point_in_time=False,
            notes="Doraemon production market history (KBS source). Uses actual total_trade_value where present, explicit TP estimated value fallback.",
        )

    def parse_row(self, row: Dict[str, Any], symbol: str) -> Optional[CanonicalBar]:
        """Convert a single Doraemon price history row dictionary to CanonicalBar."""
        date_raw = row.get("trading_date")
        if not date_raw:
            return None

        if isinstance(date_raw, str):
            s_date = date.fromisoformat(date_raw)
        elif isinstance(date_raw, date):
            s_date = date_raw
        else:
            s_date = date.fromisoformat(str(date_raw))

        ts = datetime(s_date.year, s_date.month, s_date.day)

        sym = (row.get("symbol") or symbol).strip().upper()
        op = float(row.get("open_price", 0.0) or 0.0)
        hp = float(row.get("high_price", 0.0) or 0.0)
        lp = float(row.get("low_price", 0.0) or 0.0)
        cp = float(row.get("close_price", 0.0) or 0.0)
        vol = float(row.get("volume", 0.0) or 0.0)

        reported_val = row.get("total_trade_value")
        if reported_val is not None:
            try:
                reported_val = float(reported_val)
            except (ValueError, TypeError):
                reported_val = None

        is_valid, _ = validate_candle_bounds(op, hp, lp, cp, vol)
        quality = DataQuality.HIGH if is_valid else DataQuality.INVALID

        calc_val, val_src = calculate_canonical_trading_value(op, hp, lp, cp, vol, reported_val)

        adj_close = row.get("adjusted_close_price")
        adj_type = AdjustmentType.UNADJUSTED
        if adj_close is not None and float(adj_close) != cp:
            adj_type = AdjustmentType.FULLY_ADJUSTED

        src = row.get("source", "kbs")
        metadata = {
            "source": src,
            "foreign_buy_vol": row.get("foreign_buy_vol"),
            "foreign_sell_vol": row.get("foreign_sell_vol"),
            "foreign_buy_val": row.get("foreign_buy_val"),
            "foreign_sell_val": row.get("foreign_sell_val"),
        }

        return CanonicalBar(
            symbol=sym,
            timestamp=ts,
            session_date=s_date,
            open=op,
            high=hp,
            low=lp,
            close=cp,
            volume=vol,
            trading_value=calc_val,
            value_source=val_src,
            flow_method=FlowMethod.OHLCV_PROXY,
            adjustment_type=adj_type,
            quality=quality,
            source_provider=f"doraemon_{src}",
            raw_metadata=metadata,
        )

    def parse_payload(self, payload: Dict[str, Any], symbol: str) -> List[CanonicalBar]:
        """Parse raw JSON response payload from Doraemon /market/prices/{symbol}/history/full."""
        data = payload.get("data", {})
        if isinstance(data, dict):
            rows = data.get("prices", [])
        elif isinstance(data, list):
            rows = data
        else:
            rows = []

        bars: List[CanonicalBar] = []
        for r in rows:
            if isinstance(r, dict):
                bar = self.parse_row(r, symbol)
                if bar and bar.quality != DataQuality.INVALID:
                    bars.append(bar)

        bars.sort(key=lambda x: x.timestamp)
        return bars

    def get_canonical_bars(
        self,
        symbol: str,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        as_of: Optional[str] = None,
        timeframe: str = "1D",
        limit: Optional[int] = None,
    ) -> List[CanonicalBar]:
        """Fetch and convert remote Doraemon bars, strictly enforcing date and as_of bounds."""
        if not self._api_key:
            raise RuntimeError("DORAEMON_API_KEY is not configured for remote requests.")

        clean_sym = symbol.strip().upper()
        params: Dict[str, Any] = {
            "order": "asc",
        }
        if start_date:
            params["from"] = start_date
        if end_date:
            params["to"] = end_date

        url = f"{self._base_url}/market/prices/{clean_sym}/history/full?{urlencode(params)}"
        req = Request(
            url,
            headers={
                "Accept": "application/json",
                "User-Agent": "Sumi-MarketDataPort/1.0",
                "X-API-Key": self._api_key,
            },
            method="GET",
        )

        ctx = ssl.create_default_context()
        try:
            with urlopen(req, timeout=self._timeout, context=ctx) as resp:
                payload = json.loads(resp.read().decode("utf-8"))
        except HTTPError as exc:
            raise RuntimeError(f"Doraemon API HTTP error: {exc.code}") from exc
        except (URLError, OSError) as exc:
            raise RuntimeError(f"Doraemon API connection failure: {exc}") from exc

        bars = self.parse_payload(payload, clean_sym)

        # Apply strict date filtering and as_of cutoff
        start_dt = start_at(start_date) if start_date else None
        end_dt = end_before(end_date) if end_date else None
        if as_of:
            as_of_dt = end_before(as_of)
            if end_dt is None or as_of_dt < end_dt:
                end_dt = as_of_dt

        filtered = []
        for b in bars:
            if start_dt is not None and b.timestamp < start_dt:
                continue
            if end_dt is not None and b.timestamp >= end_dt:
                continue
            filtered.append(b)

        if limit is not None and limit > 0:
            filtered = filtered[-limit:]

        return filtered
