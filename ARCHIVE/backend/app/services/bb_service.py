"""Money Flow Blackbox (BB) domain service.

Orchestrates symbol BB calculations across MarketDataPort adapters,
caching results where applicable and enforcing BB boundary guardrails.
"""

from typing import Dict, List, Optional, Sequence
from sqlalchemy.orm import Session

from app.domain.bb.calculator import ProxyBBCalculator
from app.domain.bb.chart_adapter import BBChartAdapter
from app.domain.bb.contracts import (
    BBHorizon,
    BBSymbolRequest,
    BBSymbolSeriesResult,
    MarketBBSeriesResult,
)
from app.domain.bb.market_aggregator import MarketBBAggregator
from app.domain.data.adapters.sumi_candle_adapter import SumiCandleAdapter
from app.domain.data.ports import MarketDataPort
from app.domain.universe.models import UniverseMode
from app.domain.universe.registry import get_universe_registry


class MoneyFlowBlackboxService:
    """Application service for Blackbox metrics computation."""

    def __init__(self, port: Optional[MarketDataPort] = None):
        self._port = port

    def calculate_symbol_bb(
        self,
        request: BBSymbolRequest,
        db: Optional[Session] = None,
        limit: int = 500,
    ) -> BBSymbolSeriesResult:
        """Calculate Blackbox multi-horizon series for a requested symbol.
        
        Args:
            request: Validated BBSymbolRequest.
            db: Optional SQLAlchemy Session (used with SumiCandleAdapter if no custom port set).
            limit: Maximum lookback count of bars.
            
        Returns:
            BBSymbolSeriesResult containing multi-horizon points and audit metadata.
        """
        port = self._port
        if port is None:
            if db is None:
                raise ValueError("Either a MarketDataPort or an active DB Session must be provided.")
            port = SumiCandleAdapter(db)

        # Retrieve canonical bars up to as_of
        canonical_bars = port.get_canonical_bars(
            symbol=request.symbol,
            timeframe="1D",
            as_of=request.as_of,
            limit=limit,
        )

        # Execute pure calculation
        return ProxyBBCalculator.calculate_series(
            bars=canonical_bars,
            symbol=request.symbol,
            horizons=request.horizons,
            as_of=request.as_of,
        )

    def get_symbol_chart_data(
        self,
        symbol: str,
        horizons: Optional[List[BBHorizon]] = None,
        as_of: Optional[str] = None,
        db: Optional[Session] = None,
        limit: int = 500,
    ) -> dict:
        """Helper to get ready-to-render chart series dictionary."""
        req = BBSymbolRequest(
            symbol=symbol,
            as_of=as_of,
            horizons=horizons or [
                BBHorizon.T03, BBHorizon.T05, BBHorizon.T10, BBHorizon.T20, BBHorizon.T50, BBHorizon.T200
            ],
        )
        series_res = self.calculate_symbol_bb(req, db=db, limit=limit)
        return BBChartAdapter.to_chart_series(series_res)

    def calculate_market_bb(
        self,
        universe_id: str,
        universe_mode: Optional[UniverseMode] = None,
        horizons: Optional[List[BBHorizon]] = None,
        as_of: Optional[str] = None,
        breadth_buffer: float = 0.0,
        coverage_threshold: float = 0.80,
        db: Optional[Session] = None,
        limit: int = 500,
    ) -> MarketBBSeriesResult:
        """Calculate whole-market Blackbox metrics and flow breadth for a target universe.

        Args:
            universe_id: Target universe identifier (e.g. SUMI420).
            universe_mode: POINT_IN_TIME or RETROSPECTIVE_FIXED.
            horizons: Target horizons to compute.
            as_of: Cutoff evaluation date.
            breadth_buffer: Neutral zone buffer for flow breadth.
            coverage_threshold: Minimum coverage ratio for SATISFIED status.
            db: SQLAlchemy session.
            limit: Lookback bar limit per constituent.

        Returns:
            MarketBBSeriesResult containing multi-horizon points, breadth, and publication gate state.
        """
        registry = get_universe_registry()
        try:
            universe = registry.get(universe_id)
        except KeyError as exc:
            raise ValueError(str(exc))

        port = self._port
        if port is None:
            if db is None:
                raise ValueError("Either a MarketDataPort or an active DB Session must be provided.")
            port = SumiCandleAdapter(db)

        # Collect bars for all unique symbols defined in the universe
        unique_symbols = sorted(list(set(m.symbol for m in universe.members)))
        symbol_bars_map: Dict[str, Sequence] = {}

        for sym in unique_symbols:
            bars = port.get_canonical_bars(
                symbol=sym,
                timeframe="1D",
                as_of=as_of,
                limit=limit,
            )
            if bars:
                symbol_bars_map[sym] = bars

        return MarketBBAggregator.calculate_market_bb(
            universe=universe,
            symbol_bars_map=symbol_bars_map,
            mode=universe_mode,
            horizons=horizons,
            as_of=as_of,
            breadth_buffer=breadth_buffer,
            coverage_threshold=coverage_threshold,
        )

