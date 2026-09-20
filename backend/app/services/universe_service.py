"""Service layer coordinating universe registry, resolution, and coverage checks."""

from __future__ import annotations

from datetime import date
from typing import List, Optional

from app.domain.universe.models import MemberStatus, UniverseMode
from app.domain.universe.registry import get_universe_registry
from app.domain.universe.resolver import UniverseResolver
from app.schemas.universe_schema import (
    UniverseCoverageReportSchema,
    UniverseDetailSchema,
    UniverseMemberSchema,
    UniverseResolutionSchema,
    UniverseSummarySchema,
)


class UniverseService:
    """Service providing read-only universe management operations."""

    @classmethod
    def list_universes(cls) -> List[UniverseSummarySchema]:
        """List all available registered universe specifications."""
        registry = get_universe_registry()
        definitions = registry.list_definitions()
        results = []
        for d in definitions:
            total_count = len(d.members)
            active_count = sum(1 for m in d.members if m.status == MemberStatus.ACTIVE)
            results.append(
                UniverseSummarySchema(
                    universe_id=d.universe_id,
                    version=d.version,
                    name=d.name,
                    status=d.status.value,
                    default_mode=d.default_mode.value,
                    effective_from=d.effective_from.isoformat() if d.effective_from else None,
                    effective_to=d.effective_to.isoformat() if d.effective_to else None,
                    selection_purpose=d.selection_purpose,
                    source_evidence=d.source_evidence,
                    total_member_count=total_count,
                    active_member_count=active_count,
                    content_hash=d.content_hash(),
                )
            )
        return results

    @classmethod
    def get_definition(cls, universe_id: str, version: Optional[str] = None) -> UniverseDetailSchema:
        """Get full definition detail including all member items."""
        registry = get_universe_registry()
        d = registry.get(universe_id, version)
        total_count = len(d.members)
        active_count = sum(1 for m in d.members if m.status == MemberStatus.ACTIVE)
        member_schemas = [
            UniverseMemberSchema(
                symbol=m.symbol,
                exchange=m.exchange,
                company_name=m.company_name,
                sector=m.sector,
                effective_from=m.effective_from.isoformat() if m.effective_from else None,
                effective_to=m.effective_to.isoformat() if m.effective_to else None,
                status=m.status.value,
                inclusion_reason=m.inclusion_reason,
                weight=m.weight,
                metadata=m.metadata,
            )
            for m in d.members
        ]
        return UniverseDetailSchema(
            universe_id=d.universe_id,
            version=d.version,
            name=d.name,
            status=d.status.value,
            default_mode=d.default_mode.value,
            effective_from=d.effective_from.isoformat() if d.effective_from else None,
            effective_to=d.effective_to.isoformat() if d.effective_to else None,
            selection_purpose=d.selection_purpose,
            source_evidence=d.source_evidence,
            total_member_count=total_count,
            active_member_count=active_count,
            content_hash=d.content_hash(),
            selection_rules=d.selection_rules,
            members=member_schemas,
            metadata=d.metadata,
        )

    @classmethod
    def resolve_universe(
        cls,
        universe_id: str,
        as_of: date,
        version: Optional[str] = None,
        mode: Optional[str] = None,
    ) -> UniverseResolutionSchema:
        """Resolve universe active symbols for a specific evaluation date."""
        registry = get_universe_registry()
        d = registry.get(universe_id, version)
        u_mode = UniverseMode(mode) if mode else None
        res = UniverseResolver.resolve(d, as_of=as_of, mode=u_mode)
        return UniverseResolutionSchema(
            universe_id=res.universe_id,
            version=res.version,
            as_of=res.as_of.isoformat(),
            mode=res.mode.value,
            is_point_in_time=res.is_point_in_time,
            active_symbols=res.active_symbols,
            excluded_symbols=res.excluded_symbols,
            active_count=res.active_count,
            total_target_count=res.total_target_count,
            content_hash=res.content_hash,
            survivor_bias_caveat=res.survivor_bias_caveat,
        )

    @classmethod
    def check_coverage(
        cls,
        universe_id: str,
        as_of: date,
        available_symbols: List[str],
        version: Optional[str] = None,
        mode: Optional[str] = None,
        threshold: float = 0.80,
    ) -> UniverseCoverageReportSchema:
        """Evaluate data coverage for a resolved universe against observed symbols."""
        registry = get_universe_registry()
        d = registry.get(universe_id, version)
        u_mode = UniverseMode(mode) if mode else None
        res = UniverseResolver.resolve(d, as_of=as_of, mode=u_mode)
        cov = UniverseResolver.check_coverage(res, set(available_symbols), threshold=threshold)
        return UniverseCoverageReportSchema(
            universe_id=cov.universe_id,
            version=cov.version,
            as_of=cov.as_of.isoformat(),
            target_count=cov.target_count,
            available_count=cov.available_count,
            missing_count=cov.missing_count,
            coverage_ratio=cov.coverage_ratio,
            missing_symbols=cov.missing_symbols,
            status=cov.status.value,
            threshold=cov.threshold,
        )
