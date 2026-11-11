"""Pure universe resolver implementing point-in-time and retrospective evaluation semantics."""

from __future__ import annotations

from datetime import date
from typing import Optional, Set

from app.domain.universe.models import (
    CoverageStatus,
    MemberStatus,
    UniverseCoverageReport,
    UniverseDefinition,
    UniverseMode,
    UniverseResolution,
)


class UniverseResolver:
    """Pure domain resolver for versioned universes."""

    @staticmethod
    def validate_definition(universe: UniverseDefinition) -> None:
        """Validate universe definition structural integrity and interval sanity."""
        if not universe.universe_id or not universe.universe_id.strip():
            raise ValueError("universe_id cannot be empty.")
        if not universe.version or not universe.version.strip():
            raise ValueError("version cannot be empty.")

        # Validate member dates and intervals
        symbols_seen: dict[str, list[tuple[date, date]]] = {}
        for m in universe.members:
            if not m.symbol or not m.symbol.strip():
                raise ValueError("Member symbol cannot be empty.")
            if m.effective_from and m.effective_to and m.effective_to < m.effective_from:
                raise ValueError(
                    f"Member {m.symbol} has invalid interval: effective_to {m.effective_to} < effective_from {m.effective_from}"
                )

            # Check for overlapping active episodes for the same symbol
            start = m.effective_from or date.min
            end = m.effective_to or date.max
            intervals = symbols_seen.setdefault(m.symbol, [])
            for prev_start, prev_end in intervals:
                if not (end < prev_start or start > prev_end):
                    raise ValueError(
                        f"Overlapping episodes detected for symbol {m.symbol}: [{start}, {end}] overlaps [{prev_start}, {prev_end}]"
                    )
            intervals.append((start, end))

    @classmethod
    def resolve(
        cls,
        universe: UniverseDefinition,
        as_of: date,
        mode: Optional[UniverseMode] = None,
    ) -> UniverseResolution:
        """Resolve universe constituents active as of a specific date.

        In POINT_IN_TIME mode:
            Strictly respects effective date intervals for the universe and each constituent.
            If as_of is prior to the universe's effective_from, no members are active (prevents accidental lookahead backfilling).

        In RETROSPECTIVE_FIXED mode:
            Treats the active constituents as a fixed research universe across history,
            marking is_point_in_time=False and providing an explicit survivor_bias_caveat.
        """
        cls.validate_definition(universe)
        eval_mode = mode if mode is not None else universe.default_mode

        if eval_mode == UniverseMode.POINT_IN_TIME:
            is_pit = True
            caveat = None

            # If the universe as a whole has an effective_from that is in the future relative to as_of,
            # no members are active in point-in-time truth.
            if universe.effective_from is not None and as_of < universe.effective_from:
                active_symbols = []
                excluded_symbols = sorted(list(set(m.symbol for m in universe.members)))
            else:
                active_set = set()
                excluded_set = set()
                for m in universe.members:
                    if m.is_active_on(as_of):
                        active_set.add(m.symbol)
                    else:
                        excluded_set.add(m.symbol)
                # Any active symbol should not be in excluded
                excluded_set = excluded_set - active_set
                active_symbols = sorted(list(active_set))
                excluded_symbols = sorted(list(excluded_set))

            total_target = len(active_symbols)

        elif eval_mode == UniverseMode.RETROSPECTIVE_FIXED:
            is_pit = False
            caveat = (
                f"RETROSPECTIVE_FIXED MODE: Constituents reflect fixed snapshot "
                f"(effective {universe.effective_from.isoformat() if universe.effective_from else 'curated'}). "
                f"Evaluation as of {as_of.isoformat()} carries survivorship bias and must not be used as point-in-time truth."
            )
            # In retrospective mode, all members designated ACTIVE are included
            active_set = set(m.symbol for m in universe.members if m.status == MemberStatus.ACTIVE)
            excluded_set = set(m.symbol for m in universe.members if m.status != MemberStatus.ACTIVE)
            active_symbols = sorted(list(active_set))
            excluded_symbols = sorted(list(excluded_set))
            total_target = len(active_symbols)

        else:
            raise ValueError(f"Unsupported universe resolution mode: {eval_mode}")

        return UniverseResolution(
            universe_id=universe.universe_id,
            version=universe.version,
            as_of=as_of,
            mode=eval_mode,
            is_point_in_time=is_pit,
            active_symbols=active_symbols,
            excluded_symbols=excluded_symbols,
            active_count=len(active_symbols),
            total_target_count=total_target,
            content_hash=universe.content_hash(),
            survivor_bias_caveat=caveat,
        )

    @staticmethod
    def check_coverage(
        resolution: UniverseResolution,
        available_symbols: Set[str],
        threshold: float = 0.80,
    ) -> UniverseCoverageReport:
        """Evaluate coverage of available market data against resolved target universe symbols."""
        if not (0.0 <= threshold <= 1.0):
            raise ValueError("Coverage threshold must be between 0.0 and 1.0.")

        norm_avail = set(s.strip().upper() for s in available_symbols)
        target_set = set(resolution.active_symbols)

        covered = target_set & norm_avail
        missing = target_set - covered

        target_count = len(target_set)
        available_count = len(covered)
        missing_count = len(missing)

        ratio = (available_count / target_count) if target_count > 0 else 1.0

        if ratio >= threshold:
            status = CoverageStatus.SATISFIED
        elif ratio >= 0.50:
            status = CoverageStatus.DEGRADED
        else:
            status = CoverageStatus.FAILED

        return UniverseCoverageReport(
            universe_id=resolution.universe_id,
            version=resolution.version,
            as_of=resolution.as_of,
            target_count=target_count,
            available_count=available_count,
            missing_count=missing_count,
            coverage_ratio=round(ratio, 4),
            missing_symbols=sorted(list(missing)),
            status=status,
            threshold=threshold,
        )
