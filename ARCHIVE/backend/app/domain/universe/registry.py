"""Registry for discovering, caching, and serving versioned universe definitions."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from app.domain.universe.models import UniverseDefinition
from app.domain.universe.resolver import UniverseResolver


class UniverseRegistry:
    """Registry maintaining available immutable universe definitions."""

    def __init__(self, definitions_dir: Optional[Path] = None) -> None:
        self._definitions: Dict[Tuple[str, str], UniverseDefinition] = {}
        self._definitions_dir = definitions_dir or (Path(__file__).parent / "definitions")
        self._loaded = False

    def load_definitions(self, force_reload: bool = False) -> None:
        """Scan definitions directory and register all valid universe definition files."""
        if self._loaded and not force_reload:
            return

        if not self._definitions_dir.exists():
            self._loaded = True
            return

        # Search for .json definition files in definitions_dir and immediate subdirectories
        for path in self._definitions_dir.glob("**/*.json"):
            if path.is_file():
                try:
                    with open(path, "r", encoding="utf-8") as f:
                        data = json.load(f)
                    # Only parse files that conform to UniverseDefinition schema
                    if "universe_id" in data and "version" in data and "members" in data:
                        defn = UniverseDefinition.from_dict(data)
                        self.register(defn)
                except Exception as exc:
                    # Log or ignore non-universe files
                    continue

        self._loaded = True

    def register(self, definition: UniverseDefinition) -> None:
        """Register a universe definition after validating its structural integrity."""
        UniverseResolver.validate_definition(definition)
        key = (definition.universe_id.upper(), definition.version.strip())
        self._definitions[key] = definition

    def get(self, universe_id: str, version: Optional[str] = None) -> UniverseDefinition:
        """Retrieve a registered universe definition by ID and optional version.

        If version is omitted, returns the latest approved version, or candidate version.
        """
        self.load_definitions()
        uid = universe_id.strip().upper()
        norm_uid = uid.replace("-", "").replace("_", "")

        if version is not None:
            v_str = version.strip()
            key = (uid, v_str)
            if key in self._definitions:
                return self._definitions[key]
            for (u, v), d in self._definitions.items():
                if (u == uid or u.replace("-", "").replace("_", "") == norm_uid) and v == v_str:
                    return d
            raise KeyError(f"Universe '{universe_id}' version '{version}' not found.")

        # Look up candidates matching uid or normalized uid
        matches = [
            d for (u, _), d in self._definitions.items()
            if u == uid or u.replace("-", "").replace("_", "") == norm_uid
        ]
        if not matches:
            raise KeyError(f"Universe '{universe_id}' not found.")

        # Prioritize APPROVED over CANDIDATE
        matches.sort(key=lambda d: (1 if d.status.value == "APPROVED" else 0, d.version), reverse=True)
        return matches[0]

    def list_definitions(self) -> List[UniverseDefinition]:
        """List all registered universe definitions."""
        self.load_definitions()
        return sorted(list(self._definitions.values()), key=lambda d: (d.universe_id, d.version))


# Global singleton instance
_registry_instance: Optional[UniverseRegistry] = None


def get_universe_registry() -> UniverseRegistry:
    """Get or initialize the singleton UniverseRegistry instance."""
    global _registry_instance
    if _registry_instance is None:
        _registry_instance = UniverseRegistry()
        _registry_instance.load_definitions()
    return _registry_instance
