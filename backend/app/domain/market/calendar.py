"""Vietnam trading calendar and exchange holiday fixtures.

Provides session-based calendar arithmetic strictly excluding weekends
and official Vietnamese public holidays (HOSE, HNX, UPCoM).
Eliminates calendar-day timedelta(days=N) approximations (TEST-BT-003).
"""

from datetime import date, datetime, timedelta
from typing import FrozenSet, Optional, Set, Union


# Immutable official exchange holidays and observed makeup days (2020–2026+)
VIETNAM_HOLIDAYS_V1: FrozenSet[date] = frozenset({
    # -------------------------------------------------------------------------
    # 2020 (10 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2020, 1, 1),    # Tet Duong lich
    date(2020, 1, 23),   # Tet Canh Ty (29 Tet)
    date(2020, 1, 24),   # Tet Canh Ty (30 Tet)
    date(2020, 1, 27),   # Tet Canh Ty (Mung 3 Tet)
    date(2020, 1, 28),   # Tet Canh Ty (Mung 4 Tet)
    date(2020, 1, 29),   # Tet Canh Ty (Mung 5 Tet)
    date(2020, 4, 2),    # Gio To Hung Vuong (10/3 AL)
    date(2020, 4, 30),   # Giai phong mien Nam
    date(2020, 5, 1),    # Quoc te Lao dong
    date(2020, 9, 2),    # Quoc khanh

    # -------------------------------------------------------------------------
    # 2021 (11 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2021, 1, 1),    # Tet Duong lich
    date(2021, 2, 10),   # Tet Tan Suu (29 Tet)
    date(2021, 2, 11),   # Tet Tan Suu (30 Tet)
    date(2021, 2, 12),   # Tet Tan Suu (Mung 1 Tet)
    date(2021, 2, 15),   # Tet Tan Suu (Mung 4 Tet)
    date(2021, 2, 16),   # Tet Tan Suu (Mung 5 Tet)
    date(2021, 4, 21),   # Gio To Hung Vuong (10/3 AL)
    date(2021, 4, 30),   # Giai phong mien Nam
    date(2021, 5, 3),    # Nghi bu Quoc te Lao dong (01/05 on Sat)
    date(2021, 9, 2),    # Quoc khanh
    date(2021, 9, 3),    # Quoc khanh (lien ke)

    # -------------------------------------------------------------------------
    # 2022 (11 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2022, 1, 3),    # Nghi bu Tet Duong lich (01/01 on Sat)
    date(2022, 1, 31),   # Tet Nham Dan (29 Tet)
    date(2022, 2, 1),    # Tet Nham Dan (Mung 1 Tet)
    date(2022, 2, 2),    # Tet Nham Dan (Mung 2 Tet)
    date(2022, 2, 3),    # Tet Nham Dan (Mung 3 Tet)
    date(2022, 2, 4),    # Tet Nham Dan (Mung 4 Tet)
    date(2022, 4, 11),   # Nghi bu Gio To Hung Vuong (10/3 AL on Sun 10/04)
    date(2022, 5, 2),    # Nghi bu 30/04 (on Sat)
    date(2022, 5, 3),    # Nghi bu 01/05 (on Sun)
    date(2022, 9, 1),    # Quoc khanh (lien ke)
    date(2022, 9, 2),    # Quoc khanh

    # -------------------------------------------------------------------------
    # 2023 (11 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2023, 1, 2),    # Nghi bu Tet Duong lich (01/01 on Sun)
    date(2023, 1, 20),   # Tet Quy Mao (29 Tet)
    date(2023, 1, 23),   # Tet Quy Mao (Mung 2 Tet)
    date(2023, 1, 24),   # Tet Quy Mao (Mung 3 Tet)
    date(2023, 1, 25),   # Tet Quy Mao (Mung 4 Tet)
    date(2023, 1, 26),   # Tet Quy Mao (Mung 5 Tet)
    date(2023, 5, 1),    # Quoc te Lao dong
    date(2023, 5, 2),    # Nghi bu Gio To Hung Vuong (29/04 on Sat)
    date(2023, 5, 3),    # Nghi bu 30/04 (on Sun)
    date(2023, 9, 1),    # Quoc khanh (lien ke)
    date(2023, 9, 4),    # Nghi bu Quoc khanh (02/09 on Sat)

    # -------------------------------------------------------------------------
    # 2024 (11 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2024, 1, 1),    # Tet Duong lich
    date(2024, 2, 8),    # Tet Giap Thin (29 Tet)
    date(2024, 2, 9),    # Tet Giap Thin (30 Tet)
    date(2024, 2, 12),   # Tet Giap Thin (Mung 3 Tet)
    date(2024, 2, 13),   # Tet Giap Thin (Mung 4 Tet)
    date(2024, 2, 14),   # Tet Giap Thin (Mung 5 Tet)
    date(2024, 4, 18),   # Gio To Hung Vuong (10/3 AL)
    date(2024, 4, 29),   # Hoan doi ngay lam viec (Sat 04/05 did not trade)
    date(2024, 4, 30),   # Giai phong mien Nam
    date(2024, 5, 1),    # Quoc te Lao dong
    date(2024, 9, 2),    # Quoc khanh
    date(2024, 9, 3),    # Quoc khanh (lien ke)

    # -------------------------------------------------------------------------
    # 2025 (11 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2025, 1, 1),    # Tet Duong lich
    date(2025, 1, 27),   # Tet At Ty (28 Tet)
    date(2025, 1, 28),   # Tet At Ty (29 Tet)
    date(2025, 1, 29),   # Tet At Ty (Mung 1 Tet)
    date(2025, 1, 30),   # Tet At Ty (Mung 2 Tet)
    date(2025, 1, 31),   # Tet At Ty (Mung 3 Tet)
    date(2025, 4, 7),    # Gio To Hung Vuong (10/3 AL)
    date(2025, 4, 30),   # Giai phong mien Nam
    date(2025, 5, 1),    # Quoc te Lao dong
    date(2025, 5, 2),    # Hoan doi ngay lam viec (Sat 26/04 did not trade)
    date(2025, 9, 1),    # Quoc khanh (lien ke)
    date(2025, 9, 2),    # Quoc khanh

    # -------------------------------------------------------------------------
    # 2026 (11 holiday weekdays)
    # -------------------------------------------------------------------------
    date(2026, 1, 1),    # Tet Duong lich
    date(2026, 2, 16),   # Tet Binh Ngo (29 Tet)
    date(2026, 2, 17),   # Tet Binh Ngo (30 Tet)
    date(2026, 2, 18),   # Tet Binh Ngo (Mung 1 Tet)
    date(2026, 2, 19),   # Tet Binh Ngo (Mung 2 Tet)
    date(2026, 2, 20),   # Tet Binh Ngo (Mung 3 Tet)
    date(2026, 4, 27),   # Nghi bu Gio To Hung Vuong (10/3 AL on Sun 26/04)
    date(2026, 4, 30),   # Giai phong mien Nam
    date(2026, 5, 1),    # Quoc te Lao dong
    date(2026, 8, 31),   # Hoan doi ngay lam viec (Sat 22/08 did not trade)
    date(2026, 9, 1),    # Quoc khanh (lien ke)
    date(2026, 9, 2),    # Quoc khanh
})


class TradingCalendar:
    """Authoritative Vietnam exchange trading calendar engine."""

    VERSION: str = "VN_CALENDAR_2020_2026_V1"

    def __init__(self, holidays: Optional[Union[Set[date], FrozenSet[date]]] = None):
        self.holidays: FrozenSet[date] = frozenset(holidays) if holidays is not None else VIETNAM_HOLIDAYS_V1

    def is_trading_day(self, dt: Union[date, datetime]) -> bool:
        """Return True if date is Monday-Friday and not a registered exchange holiday."""
        d = dt.date() if isinstance(dt, datetime) else dt
        if d.weekday() >= 5:  # 5 is Saturday, 6 is Sunday
            return False
        return d not in self.holidays

    def next_trading_day(self, dt: Union[date, datetime]) -> date:
        """Return the immediate next valid trading day."""
        d = dt.date() if isinstance(dt, datetime) else dt
        cur = d + timedelta(days=1)
        while not self.is_trading_day(cur):
            cur += timedelta(days=1)
        return cur

    def previous_trading_day(self, dt: Union[date, datetime]) -> date:
        """Return the immediate preceding valid trading day."""
        d = dt.date() if isinstance(dt, datetime) else dt
        cur = d - timedelta(days=1)
        while not self.is_trading_day(cur):
            cur -= timedelta(days=1)
        return cur

    def add_trading_days(self, start_dt: Union[date, datetime], n: int) -> date:
        """Advance exactly n trading sessions from start_dt without naive timedelta.

        If n == 0, returns start_dt if it is a trading day, else the next trading day.
        If n < 0, moves backward by abs(n) trading sessions.
        """
        cur = start_dt.date() if isinstance(start_dt, datetime) else start_dt
        if n > 0:
            step = 0
            while step < n:
                cur = self.next_trading_day(cur)
                step += 1
        elif n < 0:
            step = 0
            while step > n:
                cur = self.previous_trading_day(cur)
                step -= 1
        else:
            if not self.is_trading_day(cur):
                cur = self.next_trading_day(cur)
        return cur

    def trading_days_between(self, start_dt: Union[date, datetime], end_dt: Union[date, datetime]) -> int:
        """Count the number of trading sessions between start_dt and end_dt.

        Follows standard settlement convention:
        - start_dt is trading day 0 (trade date).
        - Each subsequent trading day up to end_dt (inclusive) increments the count.
        - Returns 0 if start_dt == end_dt.
        - Returns negative integer if end_dt < start_dt.
        """
        d_start = start_dt.date() if isinstance(start_dt, datetime) else start_dt
        d_end = end_dt.date() if isinstance(end_dt, datetime) else end_dt

        if d_start == d_end:
            return 0

        if d_start < d_end:
            count = 0
            cur = d_start + timedelta(days=1)
            while cur <= d_end:
                if self.is_trading_day(cur):
                    count += 1
                cur += timedelta(days=1)
            return count
        else:
            # Reversed interval
            return -self.trading_days_between(d_end, d_start)
