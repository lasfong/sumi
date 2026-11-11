"""Pure signal models, quality definitions, and canonical hash utilities.

This module is completely decoupled from FastAPI, SQLAlchemy, ORM models,
database connections, and trade execution services.
"""

from dataclasses import dataclass, field
from enum import Enum
import hashlib
import json
import math
from typing import Any, Dict, List, Literal, Optional, Union


class SignalQuality(str, Enum):
    """Quality classification for signal outputs."""
    VALID = "VALID"
    INSUFFICIENT_HISTORY = "INSUFFICIENT_HISTORY"
    INVALID_VOLUME = "INVALID_VOLUME"
    ZERO_BASELINE = "ZERO_BASELINE"


class SignalStatus(str, Enum):
    """Lifecycle status of a registered signal."""
    ACTIVE = "ACTIVE"
    EXPERIMENTAL = "EXPERIMENTAL"
    DEPRECATED = "DEPRECATED"


class SignalOutputType(str, Enum):
    """Type of signal output value."""
    BOOL = "bool"
    FLOAT = "float"
    ENUM = "enum"


@dataclass(frozen=True)
class CandleBar:
    """Canonical daily candle bar input for feature and signal calculations.
    
    Strictly daily session bar with monotonically increasing timestamps.
    """
    index: int
    timestamp: str
    open: float
    high: float
    low: float
    close: float
    volume: Optional[float]


@dataclass
class SignalParameterDefinition:
    """Metadata definition for a configurable signal parameter."""
    name: str
    type: Literal["int", "float"]
    default: Union[int, float]
    min_value: Union[int, float]
    max_value: Union[int, float]
    description: str


@dataclass
class SignalDefinition:
    """Metadata schema defining a registered signal."""
    name: str
    version: str
    category: str
    label_vi: str
    description: str
    output_type: SignalOutputType
    parameters_schema: Dict[str, Any]
    default_parameters: Dict[str, Any]
    dependencies: List[str]
    warmup_bars: int
    causal_delay_bars: int
    status: SignalStatus
    ast_alias: str


def to_finite_float(val: Any) -> Optional[float]:
    """Convert an int or float (other than bool) to a finite Python float.

    Returns float if valid and finite; returns None if non-numeric, bool,
    overflowing (e.g. arbitrarily large int), NaN, or Infinity.
    """
    if val is None or isinstance(val, bool) or not isinstance(val, (int, float)):
        return None
    try:
        f = float(val)
    except (OverflowError, ValueError):
        return None
    if not math.isfinite(f):
        return None
    return f


@dataclass
class SignalOutputPoint:
    """Calculated signal state for a single bar index."""
    bar_index: int
    timestamp: str
    output_type: str
    value: Optional[Union[bool, float, str]]
    quality: SignalQuality
    reasons: List[str] = field(default_factory=list)
    baseline: Optional[float] = None
    current_volume: Optional[float] = None
    relative_volume: Optional[float] = None
    threshold: Optional[float] = None
    availability_event: str = "BAR_CLOSE"
    available_at_index: int = 0
    available_at_timestamp: str = ""

    def to_dict(self) -> Dict[str, Any]:
        """Serialize point into JSON-compliant dictionary with no NaN/Infinity.

        Raises ValueError if a point has an unknown output_type, or if a point with
        quality VALID has an invalid value, non-finite number, or mismatched type.
        """
        out_type = self.output_type.value if hasattr(self.output_type, "value") else str(self.output_type)
        if out_type not in {"bool", "float", "enum"}:
            raise ValueError(
                f"SignalOutputPoint has unknown output_type '{self.output_type}'"
            )

        serialized_val: Optional[Union[bool, float, str]] = None

        if self.quality == SignalQuality.VALID:
            if self.value is None:
                raise ValueError(
                    f"SignalOutputPoint with quality VALID cannot have null value (bar_index={self.bar_index})"
                )

            if out_type == "bool":
                if not isinstance(self.value, bool):
                    raise ValueError(
                        f"SignalOutputPoint with output_type 'bool' and quality VALID must have bool value, "
                        f"got {type(self.value).__name__}"
                    )
                serialized_val = self.value
            elif out_type == "float":
                f_val = to_finite_float(self.value)
                if f_val is None:
                    raise ValueError(
                        f"SignalOutputPoint with output_type 'float' and quality VALID must have finite numeric value, "
                        f"got {type(self.value).__name__}"
                    )
                serialized_val = f_val
            elif out_type == "enum":
                if isinstance(self.value, bool) or not isinstance(self.value, str):
                    raise ValueError(
                        f"SignalOutputPoint with output_type 'enum' and quality VALID must have string value, "
                        f"got {type(self.value).__name__}"
                    )
                serialized_val = self.value
        else:
            serialized_val = None

        return {
            "bar_index": self.bar_index,
            "timestamp": self.timestamp,
            "output_type": out_type,
            "value": serialized_val,
            "quality": self.quality.value,
            "reasons": list(self.reasons),
            "baseline": to_finite_float(self.baseline),
            "current_volume": to_finite_float(self.current_volume),
            "relative_volume": to_finite_float(self.relative_volume),
            "threshold": to_finite_float(self.threshold),
            "availability_event": self.availability_event,
            "available_at_index": self.available_at_index,
            "available_at_timestamp": self.available_at_timestamp,
        }



@dataclass
class SignalSeriesResult:
    """Complete calculation output for a single signal over a replay prefix."""
    session_id: int
    observed_index: int
    signal_name: str
    signal_version: str
    resolved_params: Dict[str, Any]
    params_hash: str
    points: List[SignalOutputPoint]


def compute_canonical_params_hash(
    signal_name: str,
    version: str,
    resolved_params: Dict[str, Any],
    allow_str: bool = False,
) -> str:
    """Compute deterministic SHA-256 hash of canonical UTF-8 JSON.
    
    Requirements:
    - Explicit signal name, version, resolved parameters.
    - Sorted keys.
    - Compact separators (',', ':').
    - Normalized parameter types (accepts only finite non-bool integers/floats by default, or strings if allow_str=True).
    - Reject bool, null, containers, NaN, and Infinity.
    """
    if not isinstance(resolved_params, dict):
        raise TypeError(f"resolved_params must be a dictionary, got {type(resolved_params).__name__}")

    allowed_types = (int, float, str) if allow_str else (int, float)
    for k, v in resolved_params.items():
        if isinstance(v, bool):
            raise TypeError(f"Parameter '{k}' cannot be a boolean")
        if v is None:
            raise TypeError(f"Parameter '{k}' cannot be null")
        if not isinstance(v, allowed_types):
            raise TypeError(f"Parameter '{k}' must be a finite integer or float, got {type(v).__name__}")
        if isinstance(v, float) and (math.isnan(v) or math.isinf(v)):
            raise ValueError(f"Parameter '{k}' cannot be NaN or Infinity")

    canonical_payload = {
        "name": signal_name,
        "params": resolved_params,
        "version": version,
    }
    canonical_json = json.dumps(canonical_payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(canonical_json.encode("utf-8")).hexdigest()
