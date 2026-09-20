"""Market rule profiles and provider registry for Vietnam equities.

Provides factory functions to resolve named execution profiles:
- 'vietnam_default_conservative' (primary default profile)
- 'vietnam_standard_t2'
- 'vietnam_legacy_t3'
- 'VN_EQUITY_DAILY_V1' (alias for conservative profile)
"""

from dataclasses import dataclass
from typing import Any, Dict, List, Optional

from app.domain.market.calendar import TradingCalendar
from app.domain.market.rules import MarketRules
from app.domain.market.settlement import SettlementEngine


@dataclass
class MarketProfile:
    """Full parameterization of Vietnam equity market rules for backtesting."""
    name: str
    calendar: TradingCalendar
    settlement_engine: SettlementEngine
    rules: MarketRules
    conservative_locked_limits: bool = True
    conservative_1d_settlement: bool = True
    fee_rate: float = 0.0015
    tax_rate: float = 0.0010
    slippage_rate: float = 0.0
    default_exchange: str = "HOSE"
    lot_size: int = 100
    version: str = "VN_EQUITY_DAILY_V1"

    def to_dict(self) -> Dict[str, Any]:
        """Convert profile metadata to dictionary for analytics and manifest."""
        return {
            "name": self.name,
            "version": self.version,
            "calendar_version": self.calendar.VERSION,
            "settlement_days": self.settlement_engine.settlement_days,
            "conservative_mode": self.conservative_1d_settlement,
            "conservative_locked_limits": self.conservative_locked_limits,
            "lot_size": self.lot_size,
            "default_exchange": self.default_exchange,
            "fee_rate": self.fee_rate,
            "tax_rate": self.tax_rate,
            "slippage_rate": self.slippage_rate,
        }


class MarketRuleProvider:
    """Registry and resolver for named market execution profiles."""

    _DEFAULT_PROFILE = "vietnam_default_conservative"

    @classmethod
    def get_profile(
        cls,
        profile_name: Optional[str] = None,
        exchange: str = "HOSE",
    ) -> MarketProfile:
        """Resolve and instantiate a named MarketProfile.

        Defaults to 'vietnam_default_conservative'.
        """
        normalized_name = (profile_name or cls._DEFAULT_PROFILE).strip().lower()

        # Alias resolution
        if normalized_name in ("vn_equity_daily_v1", "default", "conservative", "vietnam_default_conservative"):
            calendar = TradingCalendar()
            settlement_engine = SettlementEngine(calendar=calendar, settlement_days=2, conservative_mode=True)
            return MarketProfile(
                name="vietnam_default_conservative",
                calendar=calendar,
                settlement_engine=settlement_engine,
                rules=MarketRules(exchange=exchange),
                conservative_locked_limits=True,
                conservative_1d_settlement=True,
                fee_rate=0.0015,
                tax_rate=0.0010,
                slippage_rate=0.0,
                default_exchange=exchange.upper(),
                lot_size=100,
                version="VN_EQUITY_DAILY_V1",
            )

        elif normalized_name in ("vietnam_standard_t2", "standard", "t2"):
            calendar = TradingCalendar()
            settlement_engine = SettlementEngine(calendar=calendar, settlement_days=2, conservative_mode=False)
            return MarketProfile(
                name="vietnam_standard_t2",
                calendar=calendar,
                settlement_engine=settlement_engine,
                rules=MarketRules(exchange=exchange),
                conservative_locked_limits=False,
                conservative_1d_settlement=False,
                fee_rate=0.0015,
                tax_rate=0.0010,
                slippage_rate=0.0,
                default_exchange=exchange.upper(),
                lot_size=100,
                version="VN_EQUITY_DAILY_V1",
            )

        elif normalized_name in ("vietnam_legacy_t3", "legacy_t3", "t3", "pre_2022_t3"):
            calendar = TradingCalendar()
            settlement_engine = SettlementEngine(calendar=calendar, settlement_days=3, conservative_mode=True)
            return MarketProfile(
                name="vietnam_legacy_t3",
                calendar=calendar,
                settlement_engine=settlement_engine,
                rules=MarketRules(exchange=exchange),
                conservative_locked_limits=True,
                conservative_1d_settlement=True,
                fee_rate=0.0015,
                tax_rate=0.0010,
                slippage_rate=0.0,
                default_exchange=exchange.upper(),
                lot_size=100,
                version="VN_EQUITY_LEGACY_T3_V1",
            )

        else:
            raise ValueError(f"Unknown market execution profile: {profile_name}")

    @classmethod
    def resolve(
        cls,
        profile_name: Optional[str] = None,
        exchange: str = "HOSE",
    ) -> MarketProfile:
        """Alias for get_profile."""
        return cls.get_profile(profile_name, exchange=exchange)

    @classmethod
    def list_available_profiles(cls) -> List[Dict[str, Any]]:
        """Return descriptors of all registered profiles."""
        return [
            {
                "id": "vietnam_default_conservative",
                "label": "Vietnam Conservative T+1.5 (Recommended)",
                "description": "Standard T+1.5 afternoon settlement, conservative 1D daily ambiguity, locked limit rejection.",
                "settlement": "T+1.5 (Sellable T+2 Close or T+3 Open)",
                "locked_limits": "Reject impossible counter-liquidity fills",
            },
            {
                "id": "vietnam_standard_t2",
                "label": "Vietnam Standard T+2",
                "description": "Permissive settlement allowing T+2 close and intraday exits; permissive locked limit fills.",
                "settlement": "T+1.5 (Permissive)",
                "locked_limits": "Permissive fill with warning",
            },
            {
                "id": "vietnam_legacy_t3",
                "label": "Vietnam Historical Pre-2022 T+3",
                "description": "Historical 3-session clearing cycle in place before August 29, 2022.",
                "settlement": "T+3 Morning",
                "locked_limits": "Reject impossible counter-liquidity fills",
            },
        ]
