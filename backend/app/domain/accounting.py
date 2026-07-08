from dataclasses import dataclass

from app.domain.market_rules import BUY_FEE_RATE, SELL_FEE_RATE, SELL_TAX_RATE


@dataclass(frozen=True)
class ExecutionAmounts:
    gross_amount: float
    net_amount: float
    fee: float
    tax: float


def _validate_execution_inputs(price: float, quantity: float) -> None:
    if price <= 0:
        raise ValueError("Execution price must be greater than zero")
    if quantity <= 0:
        raise ValueError("Execution quantity must be greater than zero")


def calculate_buy_amounts(price: float, quantity: float) -> ExecutionAmounts:
    _validate_execution_inputs(price, quantity)
    gross_amount = price * quantity
    fee = gross_amount * BUY_FEE_RATE
    return ExecutionAmounts(
        gross_amount=gross_amount,
        net_amount=gross_amount + fee,
        fee=fee,
        tax=0.0,
    )


def calculate_sell_amounts(price: float, quantity: float) -> ExecutionAmounts:
    _validate_execution_inputs(price, quantity)
    gross_amount = price * quantity
    fee = gross_amount * SELL_FEE_RATE
    tax = gross_amount * SELL_TAX_RATE
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
