import math
from dataclasses import dataclass
from typing import Any, Optional

from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE, MarketProfile

BUY_FEE_RATE: float = DEFAULT_VN_MARKET_PROFILE.fee_buy
SELL_FEE_RATE: float = DEFAULT_VN_MARKET_PROFILE.fee_sell
SELL_TAX_RATE: float = DEFAULT_VN_MARKET_PROFILE.tax_sell


@dataclass(frozen=True)
class ExecutionAmounts:
    gross_amount: float
    net_amount: float
    fee: float
    tax: float


@dataclass(frozen=True)
class RoundTripPnL:
    gross_pnl: float
    entry_fee: float
    exit_fee: float
    exit_tax: float
    total_fees: float
    total_tax: float
    net_pnl: float
    pnl_percent: float
    is_winner: bool
    is_breakeven: bool
    is_loser: bool

    @property
    def total_friction(self) -> float:
        return self.total_fees + self.total_tax

    @property
    def is_win(self) -> bool:
        return self.is_winner

    @property
    def is_loss(self) -> bool:
        return self.is_loser


def _validate_execution_inputs(price: float, quantity: float) -> None:
    if price <= 0:
        raise ValueError("Execution price must be greater than zero")
    if quantity <= 0:
        raise ValueError("Execution quantity must be greater than zero")


def calculate_buy_amounts(
    price: float,
    quantity: float,
    profile: Optional[Any] = None,
) -> ExecutionAmounts:
    _validate_execution_inputs(price, quantity)
    active_profile = profile or DEFAULT_VN_MARKET_PROFILE
    fee_rate = getattr(active_profile, "fee_buy", getattr(active_profile, "fee_rate", BUY_FEE_RATE))
    gross_amount = price * quantity
    fee = gross_amount * fee_rate
    return ExecutionAmounts(
        gross_amount=gross_amount,
        net_amount=gross_amount + fee,
        fee=fee,
        tax=0.0,
    )


def calculate_sell_amounts(
    price: float,
    quantity: float,
    profile: Optional[Any] = None,
) -> ExecutionAmounts:
    _validate_execution_inputs(price, quantity)
    active_profile = profile or DEFAULT_VN_MARKET_PROFILE
    fee_rate = getattr(active_profile, "fee_sell", getattr(active_profile, "fee_rate", SELL_FEE_RATE))
    tax_rate = getattr(active_profile, "tax_sell", getattr(active_profile, "tax_rate", SELL_TAX_RATE))
    gross_amount = price * quantity
    fee = gross_amount * fee_rate
    tax = gross_amount * tax_rate
    return ExecutionAmounts(
        gross_amount=gross_amount,
        net_amount=gross_amount - fee - tax,
        fee=fee,
        tax=tax,
    )


def calculate_net_pnl(buy_cash_out: float, sell_cash_in: float) -> float:
    return sell_cash_in - buy_cash_out


def calculate_pnl_percent(net_pnl: float, cash_out: float) -> float:
    return (net_pnl / cash_out * 100) if cash_out > 0 else 0.0


def compute_round_trip_pnl(
    entry_price: float,
    exit_price: float,
    quantity: float,
    side: str = "LONG",
    profile: Optional[MarketProfile] = None,
) -> RoundTripPnL:
    _validate_execution_inputs(entry_price, quantity)
    _validate_execution_inputs(exit_price, quantity)
    prof = profile or DEFAULT_VN_MARKET_PROFILE

    is_long = side.upper() in ("LONG", "BUY")
    if is_long:
        gross_pnl = (exit_price - entry_price) * quantity
        entry_amounts = calculate_buy_amounts(entry_price, quantity, prof)
        exit_amounts = calculate_sell_amounts(exit_price, quantity, prof)
        entry_fee = entry_amounts.fee
        exit_fee = exit_amounts.fee
        exit_tax = exit_amounts.tax
        total_fees = entry_fee + exit_fee
        total_tax = exit_tax
        net_pnl = gross_pnl - total_fees - total_tax
        cost_basis = entry_amounts.net_amount
    else:
        gross_pnl = (entry_price - exit_price) * quantity
        entry_amounts = calculate_sell_amounts(entry_price, quantity, prof)
        exit_amounts = calculate_buy_amounts(exit_price, quantity, prof)
        entry_fee = entry_amounts.fee
        entry_tax = entry_amounts.tax
        exit_fee = exit_amounts.fee
        exit_tax = 0.0
        total_fees = entry_fee + exit_fee
        total_tax = entry_tax
        net_pnl = gross_pnl - total_fees - total_tax
        cost_basis = entry_amounts.gross_amount

    pnl_percent = (net_pnl / cost_basis * 100.0) if cost_basis > 0 else 0.0
    is_breakeven = math.isclose(net_pnl, 0.0, abs_tol=1e-9)
    is_winner = (net_pnl > 0.0) and not is_breakeven
    is_loser = (net_pnl < 0.0) and not is_breakeven

    return RoundTripPnL(
        gross_pnl=round(gross_pnl, 6),
        entry_fee=round(entry_fee, 6),
        exit_fee=round(exit_fee, 6),
        exit_tax=round(exit_tax, 6),
        total_fees=round(total_fees, 6),
        total_tax=round(total_tax, 6),
        net_pnl=round(net_pnl, 6),
        pnl_percent=round(pnl_percent, 6),
        is_winner=is_winner,
        is_breakeven=is_breakeven,
        is_loser=is_loser,
    )
