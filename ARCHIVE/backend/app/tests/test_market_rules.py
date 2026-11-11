"""Unit tests for Vietnam Market-Rule Profile (Batch P2-MKT-02).

Tests:
- TEST-BT-002: T+2 settlement rule and discrete lot sellability.
- TEST-BT-003: Trading calendar, weekend skipping, Tet and national holidays.
- TEST-BT-004: Price bands (HOSE 7%, HNX 10%, UPCoM 15%), statutory tick rounding, locked limits.
- TEST-BT-005: Phase-end holding integrity (open positions at cutoff preserved).
- Provider registry and ExecutionAssumptions schema compatibility.
"""

from datetime import date, timedelta
import pandas as pd
import pytest

from app.domain.market.calendar import TradingCalendar, VIETNAM_HOLIDAYS_V1
from app.domain.market.rules import MarketRules
from app.domain.market.settlement import HoldingLot, SettlementEngine
from app.domain.market.provider import MarketProfile, MarketRuleProvider
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.models import BacktestPosition
from app.schemas.analytics_trust_schema import ExecutionAssumptions
from app.domain.strategy.strategy_loader import load_strategy_from_dict


# =====================================================================
# 1. Trading Calendar Tests (TEST-BT-003)
# =====================================================================

class TestTradingCalendar:
    def setup_method(self):
        self.calendar = TradingCalendar()

    def test_weekend_skipping(self):
        """Weekend days are never trading days."""
        friday = date(2024, 1, 5)
        saturday = date(2024, 1, 6)
        sunday = date(2024, 1, 7)
        monday = date(2024, 1, 8)

        assert self.calendar.is_trading_day(friday) is True
        assert self.calendar.is_trading_day(saturday) is False
        assert self.calendar.is_trading_day(sunday) is False
        assert self.calendar.is_trading_day(monday) is True

        assert self.calendar.next_trading_day(friday) == monday
        assert self.calendar.previous_trading_day(monday) == friday

    def test_tet_holidays_2024(self):
        """Tet 2024 runs from Thursday 2024-02-08 through Wednesday 2024-02-14."""
        wed_before_tet = date(2024, 2, 7)
        thu_tet_start = date(2024, 2, 8)
        wed_tet_end = date(2024, 2, 14)
        thu_market_reopen = date(2024, 2, 15)

        assert self.calendar.is_trading_day(wed_before_tet) is True
        assert self.calendar.is_trading_day(thu_tet_start) is False
        assert self.calendar.is_trading_day(wed_tet_end) is False
        assert self.calendar.is_trading_day(thu_market_reopen) is True

        # Next trading day across Tet holiday block
        assert self.calendar.next_trading_day(wed_before_tet) == thu_market_reopen
        assert self.calendar.previous_trading_day(thu_market_reopen) == wed_before_tet

    def test_national_holidays(self):
        """Verify statutory holidays: New Year, Hung Kings, Reunification/Labor, Independence."""
        # New Year 2024
        assert self.calendar.is_trading_day(date(2024, 1, 1)) is False
        assert self.calendar.is_trading_day(date(2024, 1, 2)) is True

        # Hung Kings 2024 (10th of 3rd lunar month -> 2024-04-18)
        assert self.calendar.is_trading_day(date(2024, 4, 18)) is False

        # Reunification and Labor Day 2024
        assert self.calendar.is_trading_day(date(2024, 4, 30)) is False
        assert self.calendar.is_trading_day(date(2024, 5, 1)) is False

        # Independence Day 2024
        assert self.calendar.is_trading_day(date(2024, 9, 2)) is False
        assert self.calendar.is_trading_day(date(2024, 9, 3)) is False

    def test_add_trading_days(self):
        """Adding N trading days properly skips weekends and holidays."""
        friday = date(2024, 1, 5)
        # +1 -> Mon Jan 8, +2 -> Tue Jan 9
        assert self.calendar.add_trading_days(friday, 1) == date(2024, 1, 8)
        assert self.calendar.add_trading_days(friday, 2) == date(2024, 1, 9)

        # Before Tet 2024: Wed Feb 7 + 1 -> Thu Feb 15, + 2 -> Fri Feb 16
        wed_before_tet = date(2024, 2, 7)
        assert self.calendar.add_trading_days(wed_before_tet, 1) == date(2024, 2, 15)
        assert self.calendar.add_trading_days(wed_before_tet, 2) == date(2024, 2, 16)

    def test_trading_days_between(self):
        """Count trading sessions between dates (inclusive of start, exclusive of end)."""
        mon = date(2024, 1, 8)
        fri = date(2024, 1, 12)
        count = self.calendar.trading_days_between(mon, fri)
        assert count == 4


# =====================================================================
# 2. Market Rules Tests (TEST-BT-004)
# =====================================================================

class TestMarketRules:
    def test_hose_tick_sizes(self):
        """HOSE tiered tick sizes: <10k: 10, 10k-50k: 50, >=50k: 100."""
        hose = MarketRules("HOSE")
        assert hose.get_tick_size(8_500) == 10
        assert hose.get_tick_size(9_990) == 10
        assert hose.get_tick_size(10_000) == 50
        assert hose.get_tick_size(25_450) == 50
        assert hose.get_tick_size(49_950) == 50
        assert hose.get_tick_size(50_000) == 100
        assert hose.get_tick_size(85_000) == 100

    def test_hnx_and_upcom_tick_sizes(self):
        """HNX and UPCoM uniform 100 VND tick size."""
        hnx = MarketRules("HNX")
        upcom = MarketRules("UPCOM")

        assert hnx.get_tick_size(5_000) == 100
        assert hnx.get_tick_size(25_000) == 100
        assert hnx.get_tick_size(70_000) == 100

        assert upcom.get_tick_size(5_000) == 100
        assert upcom.get_tick_size(80_000) == 100

    def test_hose_price_bands_and_statutory_rounding(self):
        """HOSE 7% band with ceiling rounded down, floor rounded up."""
        hose = MarketRules("HOSE")

        # Ref = 25,000 (tick = 50)
        # 7% = 1,750 -> ceiling = 26,750, floor = 23,250
        c, f = hose.calculate_price_bands(25_000)
        assert c == 26_750
        assert f == 23_250

        # Ref = 12,350 (tick = 50)
        # raw ceiling: 12,350 * 1.07 = 13,214.5 -> rounded DOWN to nearest 50 -> 13,200
        # raw floor:   12,350 * 0.93 = 11,485.5 -> rounded UP to nearest 50 -> 11,500
        c, f = hose.calculate_price_bands(12_350)
        assert c == 13_200
        assert f == 11_500

    def test_hnx_price_bands(self):
        """HNX 10% band with 100 tick rounding."""
        hnx = MarketRules("HNX")
        # Ref = 20,500
        # 10% = 2,050 -> ceiling = 22,500 (22,550 down to 100 -> 22,500), floor = 18,500 (18,450 up to 100 -> 18,500)
        c, f = hnx.calculate_price_bands(20_500)
        assert c == 22_500
        assert f == 18_500

    def test_upcom_price_bands(self):
        """UPCoM 15% band with 100 tick rounding."""
        upcom = MarketRules("UPCOM")
        # Ref = 10,000
        # 15% = 1,500 -> ceiling = 11,500, floor = 8,500
        c, f = upcom.calculate_price_bands(10_000)
        assert c == 11_500
        assert f == 8_500

    def test_round_board_lot(self):
        """Vietnam board lot is 100 shares."""
        assert MarketRules.round_board_lot(0) == 0
        assert MarketRules.round_board_lot(45) == 0
        assert MarketRules.round_board_lot(100) == 100
        assert MarketRules.round_board_lot(199) == 100
        assert MarketRules.round_board_lot(250) == 200
        assert MarketRules.round_board_lot(1000) == 1000

    def test_is_locked_ceiling_and_floor(self):
        """Detect flat ceiling-locked and floor-locked bars."""
        hose = MarketRules("HOSE")

        # Flat bar at ceiling
        assert hose.is_locked_ceiling(open_price=107, high=107, low=107, close=107, ceiling=107) is True
        # Close at ceiling, but low is lower (traded below ceiling)
        assert hose.is_locked_ceiling(open_price=105, high=107, low=102, close=107, ceiling=107) is False
        # Not at ceiling
        assert hose.is_locked_ceiling(open_price=105, high=106, low=102, close=105, ceiling=107) is False

        # Flat bar at floor
        assert hose.is_locked_floor(open_price=93, high=93, low=93, close=93, floor=93) is True
        # Traded above floor
        assert hose.is_locked_floor(open_price=95, high=98, low=93, close=93, floor=93) is False


# =====================================================================
# 3. Settlement Engine Tests (TEST-BT-002, TEST-BT-003)
# =====================================================================

class TestSettlementEngine:
    def setup_method(self):
        self.calendar = TradingCalendar()
        self.engine = SettlementEngine(self.calendar)

    def test_t2_settlement_availability_regular_week(self):
        """Buy on Mon (Day T): cannot sell Mon, Tue, or Wed open. Sellable Wed close / Thu open."""
        acq_date = date(2024, 1, 8)  # Monday
        sellable_date = self.engine.calculate_sellable_date(acq_date, session_policy="full_bar")
        assert sellable_date == date(2024, 1, 10)  # Wednesday (T+2)

        lot = HoldingLot(quantity=100, acquisition_date=acq_date, sellable_date=sellable_date)

        # Monday (T+0): unsellable
        assert self.engine.is_sellable(lot, date(2024, 1, 8), "open") is False
        assert self.engine.is_sellable(lot, date(2024, 1, 8), "intraday") is False
        assert self.engine.is_sellable(lot, date(2024, 1, 8), "close") is False

        # Tuesday (T+1): unsellable
        assert self.engine.is_sellable(lot, date(2024, 1, 9), "open") is False
        assert self.engine.is_sellable(lot, date(2024, 1, 9), "intraday") is False
        assert self.engine.is_sellable(lot, date(2024, 1, 9), "close") is False

        # Wednesday (T+2): morning unsellable; afternoon/close sellable
        assert self.engine.is_sellable(lot, date(2024, 1, 10), "open") is False
        assert self.engine.is_sellable(lot, date(2024, 1, 10), "intraday") is False
        assert self.engine.is_sellable(lot, date(2024, 1, 10), "close") is True

        # Thursday (T+3): fully sellable in all sessions
        assert self.engine.is_sellable(lot, date(2024, 1, 11), "open") is True
        assert self.engine.is_sellable(lot, date(2024, 1, 11), "intraday") is True
        assert self.engine.is_sellable(lot, date(2024, 1, 11), "close") is True

    def test_t2_settlement_across_tet_holiday(self):
        """Buy on Wednesday 2024-02-07 before Tet holiday."""
        acq_date = date(2024, 2, 7)  # Wednesday
        sellable_date = self.engine.calculate_sellable_date(acq_date, session_policy="full_bar")
        # Tet: Feb 8 - Feb 14 closed. T+1 = Thu Feb 15, T+2 = Fri Feb 16.
        assert sellable_date == date(2024, 2, 16)

        lot = HoldingLot(quantity=100, acquisition_date=acq_date, sellable_date=sellable_date)

        # Thu Feb 15 (T+1): unsellable
        assert self.engine.is_sellable(lot, date(2024, 2, 15), "close") is False

        # Fri Feb 16 (T+2): sellable at close
        assert self.engine.is_sellable(lot, date(2024, 2, 16), "open") is False
        assert self.engine.is_sellable(lot, date(2024, 2, 16), "close") is True

    def test_fifo_lot_allocation(self):
        """FIFO lot allocation consumes settled lots first."""
        lot1 = HoldingLot(quantity=100, acquisition_date=date(2024, 1, 8), sellable_date=date(2024, 1, 10), lot_id="lot1")
        lot2 = HoldingLot(quantity=100, acquisition_date=date(2024, 1, 9), sellable_date=date(2024, 1, 11), lot_id="lot2")
        lots = [lot1, lot2]

        # On Wed Jan 10 close: only lot1 is sellable (100 shares)
        sellable_qty = self.engine.get_sellable_quantity(lots, date(2024, 1, 10), "close")
        assert sellable_qty == 100

        # Selling 100 shares on Wed close consumes lot1 completely
        sold_qty, remaining_lots = self.engine.allocate_sell(lots, 100, date(2024, 1, 10), "close")
        assert sold_qty == 100
        assert len(remaining_lots) == 1
        assert remaining_lots[0].lot_id == "lot2"
        assert remaining_lots[0].quantity == 100

        # On Thu Jan 11 close (T+2 afternoon): lot2 is now sellable
        assert self.engine.get_sellable_quantity(remaining_lots, date(2024, 1, 11), "close") == 100
        # On Fri Jan 12 open (T+3 open): lot2 is sellable at open
        assert self.engine.get_sellable_quantity(remaining_lots, date(2024, 1, 12), "open") == 100


# =====================================================================
# 4. Market Rule Provider & Profile Resolution
# =====================================================================

class TestMarketRuleProvider:
    def test_resolve_default_conservative_profile(self):
        """Default conservative profile enforces T+1.5 afternoon and conservative limits."""
        profile = MarketRuleProvider.get_profile()
        assert profile.name == "vietnam_default_conservative"
        assert profile.version == "VN_EQUITY_DAILY_V1"
        assert profile.conservative_locked_limits is True
        assert profile.conservative_1d_settlement is True
        assert profile.lot_size == 100
        assert profile.rules.daily_limit_pct == 0.07

    def test_resolve_standard_t2_profile(self):
        profile = MarketRuleProvider.get_profile("vietnam_standard_t2")
        assert profile.name == "vietnam_standard_t2"
        assert profile.conservative_locked_limits is False
        assert profile.conservative_1d_settlement is False

    def test_resolve_alias_vn_equity_daily_v1(self):
        profile = MarketRuleProvider.get_profile("VN_EQUITY_DAILY_V1")
        assert profile.name == "vietnam_default_conservative"

    def test_resolve_unknown_profile_raises(self):
        with pytest.raises(ValueError, match="Unknown market execution profile"):
            MarketRuleProvider.get_profile("nonexistent_profile")


# =====================================================================
# 5. Kernel Market Rule Integration (TEST-BT-002, TEST-BT-004, TEST-BT-005)
# =====================================================================

class TestKernelMarketRuleIntegration:
    @staticmethod
    def _create_strategy(sig_names: list[str], entry_cond: str = "sig_buy > 0", exit_cond: str = "sig_sell > 0"):
        return load_strategy_from_dict({
            "name": "Market Rule Test Strategy",
            "indicators": [{"name": s, "type": "sma", "length": 5} for s in sig_names],
            "entry_rules": [{"condition": entry_cond}],
            "exit_rules": [{"condition": exit_cond}],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        })

    def test_t2_sell_rejection_in_kernel(self):
        """TEST-BT-002: A SELL order generated on T+1 is rejected due to unsettled shares."""
        profile = MarketRuleProvider.get_profile("vietnam_default_conservative")
        kernel = BacktestExecutionKernel(profile=profile, min_holding_bars=0)

        # 5 daily bars:
        # Bar 0 (Mon 2024-01-08): Warmup bar
        # Bar 1 (Tue 2024-01-09): Entry signal at close -> BUY queued for Bar 2 Open
        # Bar 2 (Wed 2024-01-10): Filled BUY at Open. Exit signal at Close -> SELL queued for Bar 3 Open
        # Bar 3 (Thu 2024-01-11): Bar 3 Open is T+1 for Wed buy. Shares unsettled -> REJECTED!
        # Bar 4 (Fri 2024-01-12): Holding
        dates = [
            pd.Timestamp("2024-01-08"),
            pd.Timestamp("2024-01-09"),
            pd.Timestamp("2024-01-10"),
            pd.Timestamp("2024-01-11"),
            pd.Timestamp("2024-01-12"),
        ]
        df = pd.DataFrame({
            "timestamp": dates,
            "open": [100.0, 101.0, 102.0, 103.0, 104.0],
            "high": [102.0, 103.0, 104.0, 105.0, 106.0],
            "low": [99.0, 100.0, 101.0, 102.0, 103.0],
            "close": [101.0, 102.0, 103.0, 104.0, 105.0],
            "volume": [1_000_000] * 5,
        })

        strategy = self._create_strategy(["sig_buy", "sig_sell"], entry_cond="sig_buy > 0", exit_cond="sig_sell > 0")
        indicator_values = {
            "sig_buy": [0, 1, 0, 0, 0],
            "sig_sell": [0, 0, 1, 0, 0],
        }

        result = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="TEST")

        # Assert SELL order on Bar 3 Open was rejected
        rejected_orders = [o for o in result.ledger.orders if o.status == "REJECTED"]
        assert len(rejected_orders) >= 1
        assert "UNSETTLED_HOLDING_T2" in rejected_orders[0].rejection_reason

        # Position should still be open because the sell was rejected
        assert result.open_position.quantity == 100

    def test_locked_ceiling_buy_rejected(self):
        """TEST-BT-004: BUY order rejected when next-day bar is ceiling-locked."""
        profile = MarketRuleProvider.get_profile("vietnam_default_conservative")
        kernel = BacktestExecutionKernel(profile=profile)

        # 3 bars:
        # Bar 0 (Mon): Warmup bar (100,000 VND)
        # Bar 1 (Tue): Entry signal triggers BUY for Bar 2 Open
        # Bar 2 (Wed): Stock gaps to ceiling and locks flat (O=H=L=C=107,000, prev close = 100,000)
        dates = [pd.Timestamp("2024-01-08"), pd.Timestamp("2024-01-09"), pd.Timestamp("2024-01-10")]
        df = pd.DataFrame({
            "timestamp": dates,
            "open": [100_000.0, 100_000.0, 107_000.0],
            "high": [100_500.0, 100_500.0, 107_000.0],
            "low": [99_500.0, 99_500.0, 107_000.0],
            "close": [100_000.0, 100_000.0, 107_000.0],
            "volume": [500_000, 500_000, 10_000],
        })

        strategy = self._create_strategy(["sig_buy", "sig_sell"], entry_cond="sig_buy > 0", exit_cond="sig_sell > 0")
        indicator_values = {
            "sig_buy": [0, 1, 0],
            "sig_sell": [0, 0, 0],
        }
        result = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="TEST")

        rejected_orders = [o for o in result.ledger.orders if o.status == "REJECTED"]
        assert len(rejected_orders) == 1
        assert "LOCKED_CEILING_REJECT" in rejected_orders[0].rejection_reason
        assert result.open_position.quantity == 0

    def test_locked_floor_sell_rejected(self):
        """TEST-BT-004: SELL order rejected when stock is floor-locked."""
        profile = MarketRuleProvider.get_profile("vietnam_default_conservative")
        kernel = BacktestExecutionKernel(profile=profile, min_holding_bars=0)

        # 6 days:
        # Bar 0 (Mon Jan 8): Warmup
        # Bar 1 (Tue Jan 9): Buy signal -> fills Bar 2 at 100,000
        # Bar 2 (Wed Jan 10): Fill Buy at Open 100,000.0
        # Bar 3 (Thu Jan 11): Holding (T+1)
        # Bar 4 (Fri Jan 12): T+2 Close -> Exit signal triggers SELL for Bar 5 Open
        # Bar 5 (Mon Jan 15): Floor lock flat (O=H=L=C=93,000, previous close = 100,000) -> SELL rejected!
        dates = [
            pd.Timestamp("2024-01-08"),
            pd.Timestamp("2024-01-09"),
            pd.Timestamp("2024-01-10"),
            pd.Timestamp("2024-01-11"),
            pd.Timestamp("2024-01-12"),
            pd.Timestamp("2024-01-15"),
        ]
        df = pd.DataFrame({
            "timestamp": dates,
            "open": [100_000.0, 100_000.0, 100_000.0, 100_000.0, 100_000.0, 93_000.0],
            "high": [101_000.0, 101_000.0, 101_000.0, 101_000.0, 101_000.0, 93_000.0],
            "low": [99_000.0, 99_000.0, 99_000.0, 99_000.0, 99_000.0, 93_000.0],
            "close": [100_000.0, 100_000.0, 100_000.0, 100_000.0, 100_000.0, 93_000.0],
            "volume": [1_000_000] * 5 + [10_000],
        })

        strategy = self._create_strategy(["sig_buy", "sig_sell"], entry_cond="sig_buy > 0", exit_cond="sig_sell > 0")
        indicator_values = {
            "sig_buy": [0, 1, 0, 0, 0, 0],
            "sig_sell": [0, 0, 0, 0, 1, 0],
        }

        result = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="TEST")
        floor_rejects = [o for o in result.ledger.orders if o.status == "REJECTED" and "LOCKED_FLOOR_REJECT" in o.rejection_reason]
        assert len(floor_rejects) >= 1
        # Position was NOT sold because of floor lock
        assert result.open_position.quantity == 100

    def test_phase_end_holding_integrity(self):
        """TEST-BT-005: Position bought near cutoff remains open without fabricated exits."""
        profile = MarketRuleProvider.get_profile("vietnam_default_conservative")
        kernel = BacktestExecutionKernel(profile=profile)

        # 4 bars: Bar 0 warmup, Bar 1 signal, Bar 2 fill at 102.0, Bar 3 cutoff
        dates = [
            pd.Timestamp("2024-01-08"),
            pd.Timestamp("2024-01-09"),
            pd.Timestamp("2024-01-10"),
            pd.Timestamp("2024-01-11"),
        ]
        df = pd.DataFrame({
            "timestamp": dates,
            "open": [100.0, 100.0, 102.0, 105.0],
            "high": [101.0, 101.0, 104.0, 106.0],
            "low": [99.0, 99.0, 101.0, 104.0],
            "close": [100.0, 100.0, 103.0, 105.0],
            "volume": [1_000_000] * 4,
        })

        strategy = self._create_strategy(["sig_buy", "sig_sell"], entry_cond="sig_buy > 0", exit_cond="sig_sell > 0")
        indicator_values = {
            "sig_buy": [0, 1, 0, 0],
            "sig_sell": [0, 0, 0, 0],
        }
        result = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="TEST")

        # Position must remain open with 100 shares
        assert result.open_position.quantity == 100
        # No closed trades (all trades open)
        assert len([t for t in result.ledger.trades if t.status == "closed"]) == 0
        # Ending equity reflects final close (105.0 * 100 + remaining cash)
        expected_equity = result.ledger.current_cash + (100 * 105.0)
        assert abs(result.ledger.equity_curve[-1]["portfolio_value"] - expected_equity) < 1e-4

    def test_intraday_sl_suppressed_on_floor_locked_bar(self):
        """Intraday Stop Loss cannot execute when stock is floor-locked in conservative profile."""
        profile = MarketRuleProvider.get_profile("vietnam_default_conservative")
        kernel = BacktestExecutionKernel(profile=profile, min_holding_bars=0)

        # 6 bars:
        # Bar 0 (Mon): Warmup
        # Bar 1 (Tue): Buy signal -> fills Bar 2 at 100,000
        # Bar 2 (Wed): Fill Buy at Open 100,000.0 (entry price = 100,000)
        # Bar 3 (Thu): Holding (T+1)
        # Bar 4 (Fri): Holding (T+2)
        # Bar 5 (Mon): Floor locked at 93,000 (O=H=L=C = 93,000, 7% drop)
        # SL is set to 5% (price = 95,000). Low (93,000) penetrates SL, but bar is floor locked!
        dates = [
            pd.Timestamp("2024-01-08"),
            pd.Timestamp("2024-01-09"),
            pd.Timestamp("2024-01-10"),
            pd.Timestamp("2024-01-11"),
            pd.Timestamp("2024-01-12"),
            pd.Timestamp("2024-01-15"),
        ]
        df = pd.DataFrame({
            "timestamp": dates,
            "open": [100_000.0, 100_000.0, 100_000.0, 100_000.0, 100_000.0, 93_000.0],
            "high": [101_000.0, 101_000.0, 101_000.0, 101_000.0, 101_000.0, 93_000.0],
            "low": [99_000.0, 99_000.0, 99_000.0, 99_000.0, 99_000.0, 93_000.0],
            "close": [100_000.0, 100_000.0, 100_000.0, 100_000.0, 100_000.0, 93_000.0],
            "volume": [1_000_000] * 5 + [10_000],
        })

        strategy = load_strategy_from_dict({
            "name": "SL Floor Lock Strategy",
            "indicators": [{"name": "sig_buy", "type": "sma", "length": 5}],
            "entry_rules": [{"condition": "sig_buy > 0"}],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
            "risk_management": {"stop_loss_percent": 0.05},
        })
        indicator_values = {
            "sig_buy": [0, 1, 0, 0, 0, 0],
        }

        result = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="TEST")
        # Position was NOT exited by SL because of floor lock
        assert result.open_position.quantity == 100
        assert len([t for t in result.ledger.trades if t.status == "closed"]) == 0


# =====================================================================
# 6. Execution Assumptions Schema Test
# =====================================================================

class TestExecutionAssumptionsSchema:
    def test_backward_compatibility_and_defaults(self):
        """ExecutionAssumptions instantiates with defaults matching Vietnam rules."""
        assumptions = ExecutionAssumptions()
        assert assumptions.execution_profile == "vietnam_default_conservative"
        assert assumptions.market_rule_version == "VN_EQUITY_DAILY_V1"
        assert assumptions.calendar_version == "VN_CALENDAR_2020_2026_V1"
        assert assumptions.board_lot == 100
        assert assumptions.locked_limit_policy == "conservative_reject"
        assert "HOSE" in assumptions.price_bands

    def test_serialization(self):
        """ExecutionAssumptions serializes cleanly to dict/json."""
        assumptions = ExecutionAssumptions(
            execution_profile="vietnam_standard_t2",
            board_lot=100,
        )
        data = assumptions.model_dump()
        assert data["execution_profile"] == "vietnam_standard_t2"
        assert data["board_lot"] == 100
