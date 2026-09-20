"""Domain models and contracts for managed point-in-time and retrospective universes."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import date
from enum import Enum
import hashlib
import json
from typing import Any, Optional


class UniverseMode(str, Enum):
    """Universe resolution mode.

    POINT_IN_TIME: strictly resolves membership active on the evaluation date.
    RETROSPECTIVE_FIXED: fixed membership snapshot evaluated across history;
      mandates explicit survivor-bias caveat when evaluated before effective_from.
    """
    POINT_IN_TIME = "POINT_IN_TIME"
    RETROSPECTIVE_FIXED = "RETROSPECTIVE_FIXED"


class UniverseStatus(str, Enum):
    """Lifecycle status of a universe definition."""
    CANDIDATE = "CANDIDATE"
    APPROVED = "APPROVED"
    DEPRECATED = "DEPRECATED"


class MemberStatus(str, Enum):
    """Membership state of a constituent within a universe."""
    ACTIVE = "ACTIVE"
    DELISTED = "DELISTED"
    SUSPENDED = "SUSPENDED"
    RESERVE = "RESERVE"


class CoverageStatus(str, Enum):
    """Evaluation of data coverage relative to target universe constituents."""
    SATISFIED = "SATISFIED"
    DEGRADED = "DEGRADED"
    FAILED = "FAILED"


@dataclass(frozen=True)
class UniverseMember:
    """Constituent security in a universe."""
    symbol: str
    exchange: str
    company_name: Optional[str] = None
    sector: Optional[str] = None
    effective_from: Optional[date] = None
    effective_to: Optional[date] = None
    status: MemberStatus = MemberStatus.ACTIVE
    inclusion_reason: str = ""
    weight: float = 1.0
    metadata: dict[str, Any] = field(default_factory=dict)

    def is_active_on(self, target_date: date) -> bool:
        """Evaluate if member is actively eligible on target date."""
        if self.status != MemberStatus.ACTIVE:
            return False
        if self.effective_from is not None and self.effective_from > target_date:
            return False
        if self.effective_to is not None and self.effective_to < target_date:
            return False
        return True

    def to_dict(self) -> dict[str, Any]:
        return {
            "symbol": self.symbol,
            "exchange": self.exchange,
            "company_name": self.company_name,
            "sector": self.sector,
            "effective_from": self.effective_from.isoformat() if self.effective_from else None,
            "effective_to": self.effective_to.isoformat() if self.effective_to else None,
            "status": self.status.value,
            "inclusion_reason": self.inclusion_reason,
            "weight": self.weight,
            "metadata": self.metadata,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> UniverseMember:
        eff_from = date.fromisoformat(data["effective_from"]) if data.get("effective_from") else None
        eff_to = date.fromisoformat(data["effective_to"]) if data.get("effective_to") else None
        status_raw = data.get("status", "ACTIVE")
        status = MemberStatus(status_raw) if isinstance(status_raw, str) else status_raw
        return cls(
            symbol=data["symbol"].strip().upper(),
            exchange=data.get("exchange", "UNKNOWN").strip().upper(),
            company_name=data.get("company_name"),
            sector=data.get("sector"),
            effective_from=eff_from,
            effective_to=eff_to,
            status=status,
            inclusion_reason=data.get("inclusion_reason", ""),
            weight=float(data.get("weight", 1.0)),
            metadata=dict(data.get("metadata", {})),
        )


@dataclass(frozen=True)
class UniverseDefinition:
    """Portable immutable specification of a managed universe."""
    universe_id: str
    version: str
    name: str
    status: UniverseStatus
    default_mode: UniverseMode
    effective_from: Optional[date] = None
    effective_to: Optional[date] = None
    selection_purpose: str = ""
    selection_rules: list[str] = field(default_factory=list)
    source_evidence: str = ""
    members: list[UniverseMember] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)

    def content_hash(self) -> str:
        """Compute deterministic SHA-256 hash over canonical sorted member fields and parameters."""
        sorted_members = sorted(self.members, key=lambda m: (m.symbol, m.effective_from or date.min))
        canonical_payload = {
            "universe_id": self.universe_id,
            "version": self.version,
            "status": self.status.value,
            "default_mode": self.default_mode.value,
            "effective_from": self.effective_from.isoformat() if self.effective_from else None,
            "effective_to": self.effective_to.isoformat() if self.effective_to else None,
            "selection_rules": sorted(self.selection_rules),
            "members": [
                {
                    "symbol": m.symbol,
                    "exchange": m.exchange,
                    "status": m.status.value,
                    "effective_from": m.effective_from.isoformat() if m.effective_from else None,
                    "effective_to": m.effective_to.isoformat() if m.effective_to else None,
                    "weight": m.weight,
                }
                for m in sorted_members
            ],
        }
        serialized = json.dumps(canonical_payload, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(serialized.encode("utf-8")).hexdigest()

    def to_dict(self) -> dict[str, Any]:
        return {
            "universe_id": self.universe_id,
            "version": self.version,
            "name": self.name,
            "status": self.status.value,
            "default_mode": self.default_mode.value,
            "effective_from": self.effective_from.isoformat() if self.effective_from else None,
            "effective_to": self.effective_to.isoformat() if self.effective_to else None,
            "selection_purpose": self.selection_purpose,
            "selection_rules": self.selection_rules,
            "source_evidence": self.source_evidence,
            "content_hash": self.content_hash(),
            "members": [m.to_dict() for m in self.members],
            "metadata": self.metadata,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> UniverseDefinition:
        eff_from = date.fromisoformat(data["effective_from"]) if data.get("effective_from") else None
        eff_to = date.fromisoformat(data["effective_to"]) if data.get("effective_to") else None
        status = UniverseStatus(data.get("status", "CANDIDATE"))
        default_mode = UniverseMode(data.get("default_mode", "POINT_IN_TIME"))
        members = [UniverseMember.from_dict(m) for m in data.get("members", [])]
        return cls(
            universe_id=data["universe_id"].strip().upper(),
            version=data["version"].strip(),
            name=data.get("name", data["universe_id"]),
            status=status,
            default_mode=default_mode,
            effective_from=eff_from,
            effective_to=eff_to,
            selection_purpose=data.get("selection_purpose", ""),
            selection_rules=list(data.get("selection_rules", [])),
            source_evidence=data.get("source_evidence", ""),
            members=members,
            metadata=dict(data.get("metadata", {})),
        )


@dataclass(frozen=True)
class UniverseResolution:
    """Result of resolving a universe as of a specific date."""
    universe_id: str
    version: str
    as_of: date
    mode: UniverseMode
    is_point_in_time: bool
    active_symbols: list[str]
    excluded_symbols: list[str]
    active_count: int
    total_target_count: int
    content_hash: str
    survivor_bias_caveat: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "universe_id": self.universe_id,
            "version": self.version,
            "as_of": self.as_of.isoformat(),
            "mode": self.mode.value,
            "is_point_in_time": self.is_point_in_time,
            "active_symbols": self.active_symbols,
            "excluded_symbols": self.excluded_symbols,
            "active_count": self.active_count,
            "total_target_count": self.total_target_count,
            "content_hash": self.content_hash,
            "survivor_bias_caveat": self.survivor_bias_caveat,
        }


@dataclass(frozen=True)
class UniverseCoverageReport:
    """Coverage reconciliation report for market data availability against universe target."""
    universe_id: str
    version: str
    as_of: date
    target_count: int
    available_count: int
    missing_count: int
    coverage_ratio: float
    missing_symbols: list[str]
    status: CoverageStatus
    threshold: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "universe_id": self.universe_id,
            "version": self.version,
            "as_of": self.as_of.isoformat(),
            "target_count": self.target_count,
            "available_count": self.available_count,
            "missing_count": self.missing_count,
            "coverage_ratio": self.coverage_ratio,
            "missing_symbols": self.missing_symbols,
            "status": self.status.value,
            "threshold": self.threshold,
        }
