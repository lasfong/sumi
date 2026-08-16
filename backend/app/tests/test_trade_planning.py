import pytest
from app.domain.trade_planning import (
    TradePlanInput,
    TradePlanResult,
    calculate_position_size,
)


def test_hand_calculated_position_sizing_basic():
    """
    Hand-calculated test fixture for PRO-TRADE-02, PRO-TRADE-09:
    Equity: 100,000,000 VND
    Available cash: 100,000,000 VND
    Risk %: 1.0% -> 1,000,000 VND
    Entry price: 50,000 VND
    Stop loss: 47,500 VND (risk per share = 2,500 VND)
    Target price: 57,500 VND (reward per share = 7,500 VND)
    Expected R: 7,500 / 2,500 = 3.00

    Raw quantity: 1,000,000 / 2,500 = 400 shares
    Lot rounded quantity: 400 shares (multiple of 100)
    Max cash shares: 100,000,000 / (50,000 * 1.0015) = 100,000,000 / 50,075 = 1997.004 -> 1900 shares
    Planned quantity: min(400, 1900) = 400 shares

    Planned risk: 400 * 2,500 = 1,000,000 VND
    Buy gross: 400 * 50,000 = 20,000,000 VND
    Buy fee (0.15%): 30,000 VND
    Buy net: 20,030,000 VND
    Sell gross (at target): 400 * 57,500 = 23,000,000 VND
    Sell fee (0.15%): 34,500 VND
    Sell tax (0.1%): 23,000 VND
    Sell net: 23,000,000 - 34,500 - 23,000 = 22,942,500 VND
    Net profit: 22,942,500 - 20,030,000 = 2,912,500 VND
    Total risk outlay: 1,000,000 + 30,000 = 1,030,000 VND
    Net R: 2,912,500 / 1,030,000 = 2.8277
    """
    plan = TradePlanInput(
        risk_percent=1.0,
        entry_price=50000.0,
        stop_loss=47500.0,
        target_price=57500.0,
        lot_size=100,
        fee_rate=0.0015,
        tax_rate=0.001,
    )
    result = calculate_position_size(equity=100_000_000.0, available_cash=100_000_000.0, plan=plan)

    assert result.risk_per_share == 2500.0
    assert result.reward_per_share == 7500.0
    assert result.expected_r_multiple == 3.0
    assert result.max_risk_amount == 1_000_000.0
    assert result.raw_quantity == 400.0
    assert result.lot_rounded_quantity == 400.0
    assert result.planned_quantity == 400.0
    assert result.planned_risk_amount == 1_000_000.0
    assert result.estimated_buy_gross == 20_000_000.0
    assert result.estimated_buy_fee == 30_000.0
    assert result.estimated_buy_net == 20_030_000.0
    assert result.estimated_sell_gross == 23_000_000.0
    assert result.estimated_sell_fee == 34_500.0
    assert result.estimated_sell_tax == 23_000.0
    assert result.estimated_sell_net == 22_942_500.0
    assert result.estimated_total_fees_and_taxes == 87_500.0
    assert result.estimated_net_profit == 2_912_500.0
    assert result.estimated_net_r == pytest.approx(2.8277, abs=0.001)
    assert result.affordable is True


def test_hand_calculated_position_sizing_cash_constrained():
    """
    Cash-constrained scenario:
    Account equity: 50,000,000 VND
    Available cash: 10,000,000 VND
    Risk %: 2.0% -> 1,000,000 VND
    Entry price: 100,000 VND
    Stop loss: 95,000 VND (risk per share = 5,000 VND)
    Target price: 120,000 VND (reward per share = 20,000 VND)

    Raw quantity: 1,000,000 / 5,000 = 200 shares
    Max cash shares: 10,000,000 / (100,000 * 1.0015) = 10,000,000 / 100,150 = 99.85 shares
    Max cash lot-rounded: floor(99.85 / 100) * 100 = 0 shares
    Planned quantity: min(200, 0) = 0 shares
    Affordable: False
    """
    plan = TradePlanInput(
        risk_percent=2.0,
        entry_price=100000.0,
        stop_loss=95000.0,
        target_price=120000.0,
        lot_size=100,
    )
    result = calculate_position_size(equity=50_000_000.0, available_cash=10_000_000.0, plan=plan)

    assert result.raw_quantity == 200.0
    assert result.lot_rounded_quantity == 200.0
    assert result.max_cash_quantity == 0.0
    assert result.planned_quantity == 0.0
    assert result.affordable is False
    assert "insufficient" in result.status_message.lower()


def test_hand_calculated_position_sizing_lot_rounding():
    """
    Test lot floor rounding (Vietnam standard 100 shares):
    Equity: 10,000,000 VND
    Available cash: 10,000,000 VND
    Risk %: 1.0% -> 100,000 VND
    Entry: 23,400 VND
    Stop: 22,100 VND (risk per share = 1,300 VND)
    Target: 26,000 VND

    Raw quantity: 100,000 / 1,300 = 76.923 shares
    Lot rounded: floor(76.923 / 100) * 100 = 0 shares -> Not affordable
    """
    plan = TradePlanInput(
        risk_percent=1.0,
        entry_price=23400.0,
        stop_loss=22100.0,
        target_price=26000.0,
    )
    result = calculate_position_size(equity=10_000_000.0, available_cash=10_000_000.0, plan=plan)
    assert result.raw_quantity == pytest.approx(76.92, abs=0.01)
    assert result.lot_rounded_quantity == 0.0
    assert result.planned_quantity == 0.0
    assert result.affordable is False


def test_trade_planning_invalid_inputs():
    # Stop loss >= entry price
    with pytest.raises(ValueError, match="strictly below"):
        calculate_position_size(
            100_000_000, 100_000_000,
            TradePlanInput(risk_percent=1, entry_price=50, stop_loss=50, target_price=60)
        )

    # Target price <= entry price
    with pytest.raises(ValueError, match="strictly above"):
        calculate_position_size(
            100_000_000, 100_000_000,
            TradePlanInput(risk_percent=1, entry_price=50, stop_loss=40, target_price=50)
        )
