"""Adapter translating Sumi local Candle entities or DataFrames into CanonicalBar sequences."""

from datetime import date, datetime
from typing import Any, Callable, Dict, List, Optional, Union
import pandas as pd
from sqlalchemy.orm import Session

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
from app.models.candle import Candle
from app.utils.date_range import end_before, start_at


class SumiCandleAdapter:
    """Adapts local Sumi database Candle models or DataFrames to MarketDataPort."""

    def __init__(self, db_session_or_provider: Optional[Union[Session, Callable[[str, str, str], pd.DataFrame]]] = None):
        self._db_or_provider = db_session_or_provider

    def get_capability_manifest(self) -> MarketDataCapabilityManifest:
        return MarketDataCapabilityManifest(
            provider_name="sumi_local_candles",
            supported_flow_methods=[FlowMethod.OHLCV_PROXY],
            supported_value_sources=[ValueSource.ESTIMATED_TP_X_VOLUME, ValueSource.ESTIMATED_CLOSE_X_VOLUME],
            is_point_in_time=False,
            notes="Local database unadjusted OHLCV candles. Trading value estimated from typical price x volume.",
        )

    def adapt_candle_entity(self, c: Any, strict: bool = False) -> Optional[CanonicalBar]:
        """Convert a single Candle ORM entity or dict-like object to CanonicalBar."""
        ts = getattr(c, "timestamp", None)
        if isinstance(ts, str):
            ts = pd.Timestamp(ts).to_pydatetime()
        elif hasattr(ts, "to_pydatetime"):
            ts = ts.to_pydatetime()
        elif not isinstance(ts, datetime):
            ts = pd.Timestamp(ts).to_pydatetime()

        s_date = ts.date()
        sym = str(getattr(c, "symbol", "")).upper()
        op = float(getattr(c, "open", 0.0))
        hp = float(getattr(c, "high", 0.0))
        lp = float(getattr(c, "low", 0.0))
        cp = float(getattr(c, "close", 0.0))
        vol = float(getattr(c, "volume", 0.0))

        reported_val = getattr(c, "trading_value", None)
        if reported_val is not None:
            reported_val = float(reported_val)

        is_valid, errors = validate_candle_bounds(op, hp, lp, cp, vol)
        if not is_valid:
            if strict:
                raise ValueError(f"Invalid candle bounds: {errors}")
            quality = DataQuality.INVALID
        else:
            quality = DataQuality.HIGH

        calc_val, val_src = calculate_canonical_trading_value(op, hp, lp, cp, vol, reported_val)

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
            adjustment_type=AdjustmentType.UNADJUSTED,
            quality=quality,
            source_provider=getattr(c, "source", "sumi_local") or "sumi_local",
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
        """Fetch and convert candles within [start_date, end_date) up to as_of."""
        clean_sym = symbol.strip().upper()
        start_dt = start_at(start_date) if start_date else None
        end_dt = end_before(end_date) if end_date else None

        if as_of:
            as_of_dt = end_before(as_of)
            if end_dt is None or as_of_dt < end_dt:
                end_dt = as_of_dt

        raw_candles: List[Any] = []
        if isinstance(self._db_or_provider, Session):
            query = self._db_or_provider.query(Candle).filter(
                Candle.symbol == clean_sym,
                Candle.timeframe == timeframe,
            )
            if start_dt is not None:
                query = query.filter(Candle.timestamp >= start_dt)
            if end_dt is not None:
                query = query.filter(Candle.timestamp < end_dt)

            if limit is not None and limit > 0 and start_dt is None:
                # Limit recent bars ascending
                raw_candles = query.order_by(Candle.timestamp.desc()).limit(limit).all()
                raw_candles.reverse()
            else:
                raw_candles = query.order_by(Candle.timestamp.asc()).all()
                if limit is not None and limit > 0:
                    raw_candles = raw_candles[-limit:]
        elif callable(self._db_or_provider):
            df = self._db_or_provider(clean_sym, start_date or "1970-01-01", end_date or "2099-12-31")
            if df is not None and not df.empty:
                df_filtered = df.copy()
                if start_dt is not None:
                    df_filtered = df_filtered[df_filtered["timestamp"] >= start_dt]
                if end_dt is not None:
                    df_filtered = df_filtered[df_filtered["timestamp"] < end_dt]
                df_filtered.sort_values("timestamp", ascending=True, inplace=True)
                if limit is not None and limit > 0:
                    df_filtered = df_filtered.tail(limit)
                raw_candles = [row for _, row in df_filtered.iterrows()]

        bars: List[CanonicalBar] = []
        for c in raw_candles:
            bar = self.adapt_candle_entity(c)
            if bar is not None and bar.quality != DataQuality.INVALID:
                bars.append(bar)

        return bars
