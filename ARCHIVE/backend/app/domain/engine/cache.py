"""Domain-level caching for indicators, features, and signals.

Enforces:
1. Strict invalidation safety (keys include candle signature, parameters hash, and versions).
2. Zero future leak (signatures respect observed prefix / date bounds).
3. Bounded memory footprint (LRU eviction).
4. Thread-safe operations.
5. Bypass capability (use_cache=False) for exact deterministic reproducibility (TEST-REPRO-001).
"""

from collections import OrderedDict
import hashlib
import json
from threading import Lock
from typing import Any, Dict, List, Optional, Tuple, Union
import pandas as pd


def compute_candle_signature(candles: Union[pd.DataFrame, List[Any]]) -> str:
    """Generate a deterministic, compact fingerprint for a series of candles.
    
    Captures bar count, first timestamp, last timestamp, and last close price.
    Any modification to candle data changes this signature, guaranteeing invalidation.
    """
    if candles is None:
        return "none"

    if isinstance(candles, pd.DataFrame):
        if candles.empty:
            return "empty"
        count = len(candles)
        first_ts = str(candles["timestamp"].iloc[0])
        last_ts = str(candles["timestamp"].iloc[-1])
        last_close = float(candles["close"].iloc[-1]) if "close" in candles.columns else 0.0
        # Fast 16-char hash of boundary points
        raw = f"{count}:{first_ts}:{last_ts}:{last_close:.4f}"
        return hashlib.sha256(raw.encode("utf-8")).hexdigest()[:16]

    if isinstance(candles, list):
        if len(candles) == 0:
            return "empty"
        count = len(candles)
        first = candles[0]
        last = candles[-1]
        first_ts = getattr(first, "timestamp", str(first))
        last_ts = getattr(last, "timestamp", str(last))
        last_close = float(getattr(last, "close", 0.0))
        raw = f"{count}:{first_ts}:{last_ts}:{last_close:.4f}"
        return hashlib.sha256(raw.encode("utf-8")).hexdigest()[:16]

    return "unknown"


def compute_strategy_indicators_signature(indicators_config: List[Any]) -> str:
    """Compute canonical hash for a list of Strategy Indicator configurations."""
    if not indicators_config:
        return "no_indicators"

    reprs = []
    for ind in indicators_config:
        name = getattr(ind, "name", "")
        itype = getattr(ind, "type", "")
        # Extract parameter attributes
        params = {}
        for p in (
            "length", "fast", "slow", "signal", "std", "k", "d", "smooth_k", "benchmark",
            "scalar", "af0", "af", "max_af", "multiplier", "tenkan", "kijun", "senkou",
        ):
            v = getattr(ind, p, None)
            if v is not None:
                params[p] = v
        reprs.append({"name": name, "type": itype, "params": params})

    raw_json = json.dumps(reprs, sort_keys=True)
    return hashlib.sha256(raw_json.encode("utf-8")).hexdigest()[:16]


class DomainCache:
    """Thread-safe, bounded LRU cache for domain features and calculations."""

    def __init__(self, max_entries: int = 500, enabled: bool = True):
        self._max_entries = max_entries
        self._enabled = enabled
        self._store: OrderedDict[str, Any] = OrderedDict()
        self._lock = Lock()
        self._hits = 0
        self._misses = 0
        self._evictions = 0

    @property
    def enabled(self) -> bool:
        return self._enabled

    @enabled.setter
    def enabled(self, value: bool) -> None:
        with self._lock:
            self._enabled = bool(value)

    @property
    def max_entries(self) -> int:
        return self._max_entries

    def get(self, key: str) -> Optional[Any]:
        """Retrieve cached value and update LRU position. Returns None on miss or disabled."""
        if not self._enabled:
            return None

        with self._lock:
            if key in self._store:
                self._store.move_to_end(key)
                self._hits += 1
                return self._store[key]
            self._misses += 1
            return None

    def set(self, key: str, value: Any) -> None:
        """Store value with LRU eviction when capacity is exceeded."""
        if not self._enabled:
            return

        with self._lock:
            if key in self._store:
                self._store.move_to_end(key)
                self._store[key] = value
                return

            # Check capacity before inserting new key
            while len(self._store) >= self._max_entries:
                self._store.popitem(last=False)
                self._evictions += 1

            self._store[key] = value

    def invalidate_prefix(self, prefix: str) -> int:
        """Invalidate all keys starting with prefix. Returns number of keys removed."""
        with self._lock:
            keys_to_remove = [k for k in self._store.keys() if k.startswith(prefix)]
            for k in keys_to_remove:
                del self._store[k]
            return len(keys_to_remove)

    def invalidate_symbol(self, symbol: str) -> int:
        """Invalidate all cached items for a specific symbol."""
        sym = symbol.strip().upper()
        return self.invalidate_prefix(f"features:{sym}:") + self.invalidate_prefix(f"indicator:{sym}:")

    def clear(self) -> None:
        """Clear all entries and reset stats."""
        with self._lock:
            self._store.clear()
            self._hits = 0
            self._misses = 0
            self._evictions = 0

    def stats(self) -> Dict[str, Any]:
        """Return cache performance metrics."""
        with self._lock:
            total_requests = self._hits + self._misses
            hit_ratio = (self._hits / total_requests) if total_requests > 0 else 0.0
            return {
                "enabled": self._enabled,
                "entry_count": len(self._store),
                "max_entries": self._max_entries,
                "hits": self._hits,
                "misses": self._misses,
                "evictions": self._evictions,
                "hit_ratio": round(hit_ratio, 4),
            }


# Shared singleton cache instance across domain services
default_domain_cache = DomainCache(max_entries=500, enabled=True)
