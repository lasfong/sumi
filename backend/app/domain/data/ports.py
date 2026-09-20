"""Provider-neutral MarketDataPort interface definition.

Decouples quantitative domain algorithms and Money Flow Blackbox (BB) calculators
from concrete database persistence, ORM entities, and vendor wire protocols.
"""

from typing import List, Optional, Protocol
from app.domain.data.contracts import CanonicalBar, MarketDataCapabilityManifest


class MarketDataPort(Protocol):
    """Abstract port for accessing canonical market bar observations."""

    def get_canonical_bars(
        self,
        symbol: str,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        as_of: Optional[str] = None,
        timeframe: str = "1D",
        limit: Optional[int] = None,
    ) -> List[CanonicalBar]:
        """Fetch canonical bars strictly ascending by timestamp.
        
        Invariants:
        - If as_of is provided, no bar past as_of (exclusive of future data) may be returned.
        - Date bounds [start_date, end_date) must be strictly respected.
        - Output bars must be valid CanonicalBar instances.
        """
        ...

    def get_capability_manifest(self) -> MarketDataCapabilityManifest:
        """Return provider capability manifest documenting methods, sources, and bounds."""
        ...
