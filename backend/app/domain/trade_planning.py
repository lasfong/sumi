import math
from typing import Optional
from pydantic import BaseModel, Field


class TradePlanInput(BaseModel):
    risk_percent: float = Field(default=1.0, ge=0.01, le=100.0, description="Risk as a percentage of account equity")
    entry_price: float = Field(..., gt=0, description="Planned entry price per share")
    stop_loss: float = Field(..., gt=0, description="Planned stop loss price per share")
    target_price: float = Field(..., gt=0, description="Planned target price per share")
    lot_size: int = Field(default=100, ge=1, description="Standard lot size increment (Vietnam market default is 100)")
    fee_rate: float = Field(default=0.0015, ge=0.0, description="Transaction fee rate per trade side (0.15% default)")
    tax_rate: float = Field(default=0.001, ge=0.0, description="Tax rate on selling (0.1% default)")


class TradePlanResult(BaseModel):
    entry_price: float
    stop_loss: float
    target_price: float
    direction: str = "LONG"
    risk_per_share: float
    reward_per_share: float
    expected_r_multiple: float
    account_equity: float
    available_cash: float
    risk_percent: float
    max_risk_amount: float
    raw_quantity: float
    lot_rounded_quantity: float
    max_cash_quantity: float
    planned_quantity: float
    planned_risk_amount: float
    estimated_buy_gross: float
    estimated_buy_fee: float
    estimated_buy_net: float
    estimated_sell_gross: float
    estimated_sell_fee: float
    estimated_sell_tax: float
    estimated_sell_net: float
    estimated_total_fees_and_taxes: float
    estimated_net_profit: float
    estimated_net_r: float
    affordable: bool
    status_message: str


def calculate_position_size(
    equity: float,
    available_cash: float,
    plan: TradePlanInput,
) -> TradePlanResult:
    """
    Deterministic position sizing according to Vietnam stock market rules:
    - 100-share standard lot size rounding (floor to multiple of 100).
    - Long-only risk constraint: stop_loss < entry_price and target_price > entry_price.
    - Risk per trade based on account equity and risk_percent.
    - Cash ceiling constraint based on available cash and buy commissions.
    - Estimated fees (0.15% buy + 0.15% sell) and taxes (0.1% sell).
    - Expected gross and net R-multiples.
    """
    if plan.entry_price <= 0:
        raise ValueError("Entry price must be greater than zero")
    if plan.stop_loss <= 0:
        raise ValueError("Stop loss must be greater than zero")
    if plan.target_price <= 0:
        raise ValueError("Target price must be greater than zero")
    if plan.stop_loss >= plan.entry_price:
        raise ValueError("For long trades, stop loss must be strictly below entry price")
    if plan.target_price <= plan.entry_price:
        raise ValueError("For long trades, target price must be strictly above entry price")

    risk_per_share = plan.entry_price - plan.stop_loss
    reward_per_share = plan.target_price - plan.entry_price
    expected_r_multiple = reward_per_share / risk_per_share

    max_risk_amount = max(0.0, equity * (plan.risk_percent / 100.0))
    raw_quantity = max_risk_amount / risk_per_share if risk_per_share > 0 else 0.0

    lot_size = max(1, plan.lot_size)
    lot_rounded_quantity = math.floor(raw_quantity / lot_size) * lot_size

    cost_per_share = plan.entry_price * (1.0 + plan.fee_rate)
    max_cash_shares = max(0.0, available_cash / cost_per_share) if cost_per_share > 0 else 0.0
    max_cash_quantity = math.floor(max_cash_shares / lot_size) * lot_size

    planned_quantity = max(0.0, min(lot_rounded_quantity, max_cash_quantity))
    planned_risk_amount = planned_quantity * risk_per_share

    estimated_buy_gross = plan.entry_price * planned_quantity
    estimated_buy_fee = estimated_buy_gross * plan.fee_rate
    estimated_buy_net = estimated_buy_gross + estimated_buy_fee

    estimated_sell_gross = plan.target_price * planned_quantity
    estimated_sell_fee = estimated_sell_gross * plan.fee_rate
    estimated_sell_tax = estimated_sell_gross * plan.tax_rate
    estimated_sell_net = estimated_sell_gross - estimated_sell_fee - estimated_sell_tax

    estimated_total_fees_and_taxes = estimated_buy_fee + estimated_sell_fee + estimated_sell_tax
    estimated_net_profit = estimated_sell_net - estimated_buy_net if planned_quantity > 0 else 0.0

    total_risk_outlay = planned_risk_amount + estimated_buy_fee
    estimated_net_r = (estimated_net_profit / total_risk_outlay) if total_risk_outlay > 0 and planned_quantity > 0 else 0.0

    affordable = planned_quantity >= lot_size
    if planned_quantity <= 0:
        if lot_rounded_quantity < lot_size:
            status_message = f"Risk allocation ({max_risk_amount:,.0f} VND) is insufficient for minimum lot ({lot_size} shares)."
        else:
            status_message = f"Available cash ({available_cash:,.0f} VND) is insufficient for minimum lot ({lot_size} shares)."
    elif max_cash_quantity < lot_rounded_quantity:
        status_message = f"Position capped at {planned_quantity:,.0f} shares due to available cash constraint."
    else:
        status_message = f"Optimal risk position size: {planned_quantity:,.0f} shares."

    return TradePlanResult(
        entry_price=plan.entry_price,
        stop_loss=plan.stop_loss,
        target_price=plan.target_price,
        direction="LONG",
        risk_per_share=round(risk_per_share, 4),
        reward_per_share=round(reward_per_share, 4),
        expected_r_multiple=round(expected_r_multiple, 4),
        account_equity=round(equity, 2),
        available_cash=round(available_cash, 2),
        risk_percent=plan.risk_percent,
        max_risk_amount=round(max_risk_amount, 2),
        raw_quantity=round(raw_quantity, 2),
        lot_rounded_quantity=lot_rounded_quantity,
        max_cash_quantity=max_cash_quantity,
        planned_quantity=planned_quantity,
        planned_risk_amount=round(planned_risk_amount, 2),
        estimated_buy_gross=round(estimated_buy_gross, 2),
        estimated_buy_fee=round(estimated_buy_fee, 2),
        estimated_buy_net=round(estimated_buy_net, 2),
        estimated_sell_gross=round(estimated_sell_gross, 2),
        estimated_sell_fee=round(estimated_sell_fee, 2),
        estimated_sell_tax=round(estimated_sell_tax, 2),
        estimated_sell_net=round(estimated_sell_net, 2),
        estimated_total_fees_and_taxes=round(estimated_total_fees_and_taxes, 2),
        estimated_net_profit=round(estimated_net_profit, 2),
        estimated_net_r=round(estimated_net_r, 4),
        affordable=affordable,
        status_message=status_message,
    )
