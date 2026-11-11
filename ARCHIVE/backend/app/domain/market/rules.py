"""Vietnam exchange trading rules: price bands, tick sizes, board lots, and locked limit states.

Implements statutory pricing and tick constraints per HOSE/HNX/UPCoM regulations:
- Circular 120/2020/TT-BTC, HOSE Decision 352/QĐ-SGDHCM, HNX Decision 640/QĐ-SGDHN.
- Ceiling rounds DOWN to nearest tick; Floor rounds UP to nearest tick.
- Locked ceiling / floor detection (TEST-BT-004).
"""

from dataclasses import dataclass, field
import math
from typing import Any, Dict, Optional, Tuple, TYPE_CHECKING

if TYPE_CHECKING:
    from app.domain.market.calendar import TradingCalendar
    from app.domain.market.settlement import SettlementEngine



class MarketRules:
    """Vietnam exchange trading rules and microstructure constraints."""

    EXCHANGE_BANDS: Dict[str, float] = {
        "HOSE": 0.07,
        "HSX": 0.07,
        "HNX": 0.10,
        "UPCOM": 0.15,
        "UPCoM": 0.15,
    }

    def __init__(
        self,
        exchange: str = "HOSE",
        allow_buy_on_ceiling_lock: bool = False,
        allow_sell_on_floor_lock: bool = False,
        lot_size: int = 100,
    ):
        self.exchange = exchange.upper()
        self.daily_limit_pct = self.EXCHANGE_BANDS.get(self.exchange, 0.07)
        self.allow_buy_on_ceiling_lock = allow_buy_on_ceiling_lock
        self.allow_sell_on_floor_lock = allow_sell_on_floor_lock
        self.lot_size = lot_size
        self.board_lot = lot_size

    @classmethod
    def get_band(cls, exchange: str = "HOSE") -> float:
        """Return percentage price band for exchange (e.g. 0.07 for HOSE)."""
        return cls.EXCHANGE_BANDS.get(exchange.upper(), 0.07)

    def get_tick_size(self_or_price, *args, **kwargs) -> float:
        """Determine valid tick step size based on exchange and price tier.

        Works on both instance and class:
        - rules.get_tick_size(price)
        - MarketRules.get_tick_size(price, exchange="HOSE")
        """
        if isinstance(self_or_price, MarketRules):
            price = args[0] if args else kwargs.get("price", 0.0)
            exchange = kwargs.get("exchange", self_or_price.exchange)
        else:
            price = self_or_price
            exchange = args[0] if args else kwargs.get("exchange", "HOSE")

        exch = str(exchange).upper()
        if exch in ("HNX", "UPCOM"):
            return 100.0

        # HOSE tiered schedule
        if price < 10_000.0:
            return 10.0
        elif price < 50_000.0:
            return 50.0
        else:
            return 100.0

    def calculate_price_limits(self_or_ref, *args, **kwargs) -> Tuple[float, float]:
        """Calculate statutory (Floor, Ceiling) prices with regulatory tick rounding.

        - Ceiling rounds DOWN to nearest valid tick so it does not exceed statutory band.
        - Floor rounds UP to nearest valid tick so it does not exceed statutory band.
        - Returns (floor_price, ceiling_price).
        """
        if isinstance(self_or_ref, MarketRules):
            reference_price = args[0] if args else kwargs.get("reference_price", 0.0)
            exchange = kwargs.get("exchange", self_or_ref.exchange)
        else:
            reference_price = self_or_ref
            exchange = args[0] if args else kwargs.get("exchange", "HOSE")

        band = MarketRules.get_band(exchange)
        raw_ceiling = reference_price * (1.0 + band)
        raw_floor = reference_price * (1.0 - band)

        ceil_tick = MarketRules.get_tick_size(raw_ceiling, exchange=exchange)
        floor_tick = MarketRules.get_tick_size(raw_floor, exchange=exchange)

        # Ceiling rounds DOWN
        ceiling = math.floor(round(raw_ceiling / ceil_tick, 8)) * ceil_tick
        # Floor rounds UP
        floor = math.ceil(round(raw_floor / floor_tick, 8)) * floor_tick

        # Boundary adjustments
        if ceiling <= reference_price:
            ceiling = reference_price + ceil_tick
        if floor >= reference_price:
            floor = reference_price - floor_tick
        if floor <= 0:
            floor = floor_tick

        return round(floor, 2), round(ceiling, 2)

    calculate_ceiling_floor = calculate_price_limits

    def calculate_price_bands(self_or_ref, *args, **kwargs) -> Tuple[float, float]:
        """Calculate statutory (Ceiling, Floor) prices.

        Returns (ceiling_price, floor_price).
        """
        floor, ceiling = MarketRules.calculate_price_limits(self_or_ref, *args, **kwargs)
        return ceiling, floor

    def is_locked_ceiling(self_or_high=None, *args, **kwargs) -> bool:
        """Return True if bar is locked at ceiling (trần cứng / trắng bên bán).

        Accepts:
        - (high, low, ceiling_price)
        - kwargs: open_price, high, low, close, ceiling / ceiling_price
        """
        tolerance = kwargs.get("tolerance", 1.0)
        ceiling = kwargs.get("ceiling", kwargs.get("ceiling_price"))
        high = kwargs.get("high")
        low = kwargs.get("low")
        close = kwargs.get("close")

        pos_args = []
        if self_or_high is not None and not isinstance(self_or_high, MarketRules):
            pos_args.append(self_or_high)
        pos_args.extend(args)

        if ceiling is None and len(pos_args) >= 3:
            high, low, ceiling = pos_args[-3], pos_args[-2], pos_args[-1]

        if ceiling is None:
            return False

        if high is not None and low is not None:
            if abs(high - ceiling) <= tolerance and abs(low - ceiling) <= tolerance:
                return True

        if close is not None and high is not None and low is not None:
            if abs(close - ceiling) <= tolerance and abs(low - ceiling) <= tolerance:
                return True

        return False

    def is_locked_floor(self_or_high=None, *args, **kwargs) -> bool:
        """Return True if bar is locked at floor (sàn cứng / trắng bên mua).

        Accepts:
        - (high, low, floor_price)
        - kwargs: open_price, high, low, close, floor / floor_price
        """
        tolerance = kwargs.get("tolerance", 1.0)
        floor = kwargs.get("floor", kwargs.get("floor_price"))
        high = kwargs.get("high")
        low = kwargs.get("low")
        close = kwargs.get("close")

        pos_args = []
        if self_or_high is not None and not isinstance(self_or_high, MarketRules):
            pos_args.append(self_or_high)
        pos_args.extend(args)

        if floor is None and len(pos_args) >= 3:
            high, low, floor = pos_args[-3], pos_args[-2], pos_args[-1]

        if floor is None:
            return False

        if high is not None and low is not None:
            if abs(high - floor) <= tolerance and abs(low - floor) <= tolerance:
                return True

        if close is not None and high is not None and low is not None:
            if abs(close - floor) <= tolerance and abs(high - floor) <= tolerance:
                return True

        return False

    @staticmethod
    def round_board_lot(quantity: float, lot_size: int = 100) -> int:
        """Round order quantity down to the nearest standard board lot multiple (100 shares)."""
        if quantity < lot_size or lot_size <= 0:
            return 0
        return int(math.floor(quantity / lot_size) * lot_size)

    def is_price_within_limits(self_or_price, reference_price: float, exchange: str = "HOSE") -> bool:
        """Validate if a proposed price falls within daily ceiling and floor limits."""
        floor, ceiling = MarketRules.calculate_price_limits(reference_price, exchange=exchange)
        price = self_or_price if not isinstance(self_or_price, MarketRules) else reference_price
        return floor <= price <= ceiling


@dataclass
class MarketProfile:
    """Single authoritative parameterization of Vietnam equity market rules."""
    market: str = "VN"
    lot_size: int = 100
    fee_buy: float = 0.0015
    fee_sell: float = 0.0015
    tax_sell: float = 0.0010
    slippage: float = 0.0
    settlement_days: int = 2
    settlement_mode: str = "T+1.5_CALENDAR"
    settlement_label: str = "T+1 · về sau 1 phiên"
    settlement_release_phase: str = "open"
    sizing_basis: str = "fill_estimate"
    default_exchange: str = "HOSE"
    name: str = "vietnam_default_conservative"
    version: str = "VN_EQUITY_DAILY_V1"
    conservative_locked_limits: bool = True
    conservative_1d_settlement: bool = True
    calendar: Optional[Any] = None
    settlement_engine: Optional[Any] = None
    rules: Optional[MarketRules] = None

    def __post_init__(self):
        from app.domain.market.calendar import TradingCalendar
        from app.domain.market.settlement import SettlementEngine

        if self.calendar is None:
            self.calendar = TradingCalendar()
        if self.settlement_engine is None:
            self.settlement_engine = SettlementEngine(
                calendar=self.calendar,
                settlement_days=self.settlement_days,
                conservative_mode=self.conservative_1d_settlement,
            )
        if self.rules is None:
            self.rules = MarketRules(
                exchange=self.default_exchange,
                allow_buy_on_ceiling_lock=not self.conservative_locked_limits,
                allow_sell_on_floor_lock=not self.conservative_locked_limits,
                lot_size=self.lot_size,
            )

    @property
    def symbol(self) -> str:
        return self.market

    @property
    def fee_rate(self) -> float:
        return self.fee_buy

    @property
    def tax_rate(self) -> float:
        return self.tax_sell

    @property
    def slippage_rate(self) -> float:
        return self.slippage

    def to_dict(self) -> Dict[str, Any]:
        """Convert profile metadata to dictionary for analytics and manifest."""
        return {
            "market": self.market,
            "name": self.name,
            "version": self.version,
            "calendar_version": getattr(self.calendar, "VERSION", "VN_CALENDAR_2018_2026_V1"),
            "settlement_days": self.settlement_days,
            "settlement_mode": self.settlement_mode,
            "settlement_label": self.settlement_label,
            "settlement_release_phase": self.settlement_release_phase,
            "sizing_basis": self.sizing_basis,
            "conservative_mode": self.conservative_1d_settlement,
            "conservative_locked_limits": self.conservative_locked_limits,
            "lot_size": self.lot_size,
            "default_exchange": self.default_exchange,
            "fee_buy": self.fee_buy,
            "fee_sell": self.fee_sell,
            "tax_sell": self.tax_sell,
            "fee_rate": self.fee_rate,
            "tax_rate": self.tax_rate,
            "slippage": self.slippage,
            "slippage_rate": self.slippage_rate,
        }


DEFAULT_VN_MARKET_PROFILE: MarketProfile = MarketProfile()
DEFAULT_MARKET_PROFILE: MarketProfile = DEFAULT_VN_MARKET_PROFILE
