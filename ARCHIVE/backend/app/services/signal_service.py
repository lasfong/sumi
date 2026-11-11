"""Signal Service orchestrating replay-scoped signal calculations.

Strictly preserves server-side session bounds and validates timeframe and monotonic input.
"""

from typing import List
from fastapi import HTTPException
from sqlalchemy.orm import Session

from app.domain.signals.models import CandleBar, compute_canonical_params_hash
from app.domain.signals.registry import SignalRegistry
from app.schemas.signal_schema import (
    SignalCalculationItemRequest,
    SignalCalculationRequest,
    SignalCalculationResponse,
    SignalCalculationSeriesResponse,
    SignalDefinitionResponse,
    SignalOutputPointResponse,
    SignalRegistryResponse,
)
from app.domain.engine.cache import default_domain_cache, compute_candle_signature
from app.services.replay_service import ReplayService


class SignalService:
    """Application service for signal registry and calculation."""

    @classmethod
    def get_registry(cls) -> SignalRegistryResponse:
        """Return all registered signal definitions."""
        definitions = SignalRegistry.list_definitions()
        items = [
            SignalDefinitionResponse(
                name=d.name,
                version=d.version,
                category=d.category,
                label_vi=d.label_vi,
                description=d.description,
                output_type=d.output_type.value,
                parameters_schema=d.parameters_schema,
                default_parameters=d.default_parameters,
                dependencies=d.dependencies,
                warmup_bars=d.warmup_bars,
                causal_delay_bars=d.causal_delay_bars,
                status=d.status.value,
                ast_alias=d.ast_alias,
            )
            for d in definitions
        ]
        return SignalRegistryResponse(signals=items)

    @classmethod
    def calculate_replay_signals(
        cls,
        db: Session,
        session_id: int,
        request: SignalCalculationRequest,
    ) -> SignalCalculationResponse:
        """Calculate requested signals for a replay session prefix."""
        # 1. Fetch replay session (raises 404 if not found)
        session = ReplayService.get_session(db, session_id)

        # 2. Validate timeframe (First slice only supports 1D)
        if session.timeframe != "1D":
            raise HTTPException(
                status_code=422,
                detail=f"Timeframe '{session.timeframe}' is not supported in this slice. Only '1D' is supported.",
            )

        # 3. Retrieve authoritative session candles up to session.current_index
        candles = ReplayService.get_candles(db, session_id)
        if not candles:
            return SignalCalculationResponse(
                session_id=session.id,
                observed_current_index=session.current_index,
                timeframe=session.timeframe,
                results=[],
            )

        # 4. Check for monotonic, non-duplicate timestamps
        prior_ts = None
        candle_bars: List[CandleBar] = []
        for i, c in enumerate(candles):
            ts_str = c.timestamp.isoformat() if hasattr(c.timestamp, "isoformat") else str(c.timestamp)
            if prior_ts is not None and ts_str <= prior_ts:
                raise HTTPException(
                    status_code=422,
                    detail=f"Non-monotonic or duplicate timestamp detected at bar index {i}: '{ts_str}' <= '{prior_ts}'",
                )
            prior_ts = ts_str
            candle_bars.append(
                CandleBar(
                    index=i,
                    timestamp=ts_str,
                    open=float(c.open),
                    high=float(c.high),
                    low=float(c.low),
                    close=float(c.close),
                    volume=float(c.volume) if c.volume is not None else None,
                )
            )

        # 5. Process each requested signal
        series_results: List[SignalCalculationSeriesResponse] = []
        candle_sig = compute_candle_signature(candle_bars)
        for item in request.signals:
            defn = SignalRegistry.get_definition(item.name)
            if not defn:
                raise HTTPException(
                    status_code=422,
                    detail=f"Unknown signal: '{item.name}'",
                )

            if item.version is not None and item.version != defn.version:
                raise HTTPException(
                    status_code=422,
                    detail=f"Unsupported version '{item.version}' for signal '{item.name}'. Expected '{defn.version}'.",
                )

            try:
                resolved = SignalRegistry.resolve_and_validate_params(item.name, item.params)
                params_hash = compute_canonical_params_hash(
                    item.name, defn.version, resolved, allow_str=any(isinstance(v, str) for v in resolved.values())
                )
            except ValueError as ve:
                raise HTTPException(status_code=422, detail=str(ve))

            cache_key = f"signal:{session.id}:{session.current_index}:{item.name}:{defn.version}:{params_hash}:{candle_sig}"
            calc_result = default_domain_cache.get(cache_key)

            if calc_result is None:
                try:
                    calc_result = SignalRegistry.calculate(
                        name=item.name,
                        candles=candle_bars,
                        params=resolved,
                        session_id=session.id,
                        observed_index=session.current_index,
                    )
                    default_domain_cache.set(cache_key, calc_result)
                except ValueError as ve:
                    raise HTTPException(status_code=422, detail=str(ve))

            point_responses = [
                SignalOutputPointResponse(**pt.to_dict())
                for pt in calc_result.points
            ]

            series_results.append(
                SignalCalculationSeriesResponse(
                    signal_name=calc_result.signal_name,
                    signal_version=calc_result.signal_version,
                    resolved_params=calc_result.resolved_params,
                    params_hash=calc_result.params_hash,
                    points=point_responses,
                )
            )

        return SignalCalculationResponse(
            session_id=session.id,
            observed_current_index=session.current_index,
            timeframe=session.timeframe,
            results=series_results,
        )
