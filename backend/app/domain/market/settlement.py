"""Vietnam cash-equity settlement engine and holding lot accounting.

Enforces VSD/VSDC settlement rules:
- T+1.5 afternoon session availability (VSD Decision 109/QĐ-VSD).
- Conservative 1D ambiguity policy:
  - Day T: Unsellable (0 trading sessions).
  - Day T+1: Unsellable (1 trading session).
  - Day T+2:
    - Open: Unsellable (shares in transit until afternoon session).
    - Intraday: Unsellable in conservative mode (morning vs afternoon ambiguity).
    - Close: Sellable in afternoon ATC session.
  - Day T+3: Fully sellable across Open, Intraday, and Close (TEST-BT-002).
"""

from dataclasses import dataclass
from datetime import date, datetime
from typing import List, Optional, Tuple, Union

from app.domain.market.calendar import TradingCalendar


@dataclass
class HoldingLot:
    """Represents a discrete tax/settlement lot of shares."""
    quantity: float
    acquisition_date: date
    price: float = 0.0
    symbol: str = ""
    lot_id: str = ""
    acquisition_bar_index: int = 0
    sellable_date: Optional[date] = None
    is_settled: bool = False

    def __post_init__(self):
        if not self.lot_id:
            import uuid
            self.lot_id = f"lot_{uuid.uuid4().hex[:8]}"
        if self.sellable_date is None:
            self.sellable_date = self.acquisition_date


class SettlementEngine:
    """Manages trade session settlement counting and sell eligibility."""

    def __init__(
        self,
        calendar: Optional[TradingCalendar] = None,
        settlement_days: int = 2,
        conservative_mode: bool = True,
    ):
        self.calendar: TradingCalendar = calendar or TradingCalendar()
        self.settlement_days: int = settlement_days
        self.conservative_mode: bool = conservative_mode

    def calculate_sellable_date(
        self,
        acquisition_date: Union[date, datetime],
        session_policy: str = "afternoon",
    ) -> date:
        """Calculate the trading date when shares become sellable (Day T+2 under T+1.5).

        - If session_policy == 'open_only': returns Day T+3 (first trading day sellable at Open).
        - Otherwise ('afternoon', 'close', 'full_bar'): returns Day T+2 (afternoon/close session).
        """
        days_to_add = (self.settlement_days + 1) if session_policy == "open_only" else self.settlement_days
        return self.calendar.add_trading_days(acquisition_date, days_to_add)

    def is_sellable(
        self,
        lot_or_date: Union[HoldingLot, date, datetime],
        current_date: Union[date, datetime],
        current_session_phase: str = "open",
    ) -> bool:
        """Evaluate whether a lot acquired on lot_acquisition_date can be sold on current_date.

        Args:
            lot_or_date: HoldingLot instance or Trade date when buy order was filled.
            current_date: Evaluation date.
            current_session_phase: Session phase: 'open', 'intraday', 'close', or 'afternoon'.
        """
        lot_acquisition_date = getattr(lot_or_date, "acquisition_date", lot_or_date)
        d_acq = lot_acquisition_date.date() if isinstance(lot_acquisition_date, datetime) else lot_acquisition_date
        d_cur = current_date.date() if isinstance(current_date, datetime) else current_date

        trading_days = self.calendar.trading_days_between(d_acq, d_cur)

        # Day T (0 trading days) and Day T+1 (1 trading day) are always unsellable
        if trading_days < self.settlement_days:
            return False

        # Day T+2 (settlement day under T+1.5)
        if trading_days == self.settlement_days:
            phase = current_session_phase.lower()
            if self.conservative_mode:
                # In conservative mode, Open and Intraday (High/Low) exits are blocked on T+2
                # Only afternoon / Close (ATC) exits are permitted
                return phase in ("close", "pm", "afternoon", "atc")
            else:
                # Permissive mode allows exit at Close or intraday, but never morning Open
                return phase != "open"

        # Day T+3 or later: 100% sellable in all phases
        return True

    def get_sellable_quantity(
        self,
        lots: List[HoldingLot],
        current_date: Union[date, datetime],
        current_session_phase: str = "open",
    ) -> float:
        """Sum quantity of all lots that are currently sellable."""
        total = 0.0
        for lot in lots:
            if self.is_sellable(lot.acquisition_date, current_date, current_session_phase):
                total += lot.quantity
        return total

    def allocate_sell(
        self,
        lots: List[HoldingLot],
        sell_quantity: float,
        current_date: Union[date, datetime],
        current_session_phase: str = "open",
    ) -> Tuple[float, List[HoldingLot]]:
        """Consume sellable lots FIFO up to sell_quantity.

        Returns:
            (actual_sold_quantity, remaining_lots)
        """
        remaining_needed = sell_quantity
        sold_total = 0.0
        new_lots: List[HoldingLot] = []

        for lot in lots:
            if remaining_needed > 0 and self.is_sellable(lot.acquisition_date, current_date, current_session_phase):
                if lot.quantity <= remaining_needed:
                    sold_total += lot.quantity
                    remaining_needed -= lot.quantity
                    # Entire lot consumed
                else:
                    sold_total += remaining_needed
                    lot.quantity -= remaining_needed
                    remaining_needed = 0.0
                    new_lots.append(lot)
            else:
                new_lots.append(lot)

        return sold_total, new_lots
