import pytest

from app.domain.accounting import (
    calculate_buy_amounts,
    calculate_net_pnl,
    calculate_pnl_percent,
    calculate_sell_amounts,
)


def test_buy_and_sell_amounts_include_vietnam_fee_and_tax():
    buy = calculate_buy_amounts(100.0, 1000)
    sell = calculate_sell_amounts(110.0, 1000)

    assert buy.gross_amount == 100_000.0
    assert buy.fee == 150.0
    assert buy.tax == 0.0
    assert buy.net_amount == 100_150.0

    assert sell.gross_amount == 110_000.0
    assert sell.fee == 165.0
    assert sell.tax == 110.0
    assert sell.net_amount == 109_725.0

    net_pnl = calculate_net_pnl(buy.net_amount, sell.net_amount)
    assert net_pnl == 9_575.0
    assert calculate_pnl_percent(net_pnl, buy.net_amount) == pytest.approx(9.560659011482775)


@pytest.mark.parametrize(
    ("price", "quantity", "message"),
    [
        (0, 100, "price"),
        (-1, 100, "price"),
        (100, 0, "quantity"),
        (100, -1, "quantity"),
    ],
)
@pytest.mark.parametrize("calculator", [calculate_buy_amounts, calculate_sell_amounts])
def test_execution_amounts_reject_non_positive_inputs(calculator, price, quantity, message):
    with pytest.raises(ValueError, match=message):
        calculator(price, quantity)


def test_known_ledger_cash_reconciles_across_partial_and_full_close():
    initial_cash = 100_000.0
    first_buy = calculate_buy_amounts(100.0, 100.0)
    second_buy = calculate_buy_amounts(110.0, 100.0)
    partial_sell = calculate_sell_amounts(120.0, 50.0)
    final_sell = calculate_sell_amounts(90.0, 150.0)

    cash = initial_cash - first_buy.net_amount - second_buy.net_amount
    assert cash == pytest.approx(78_968.5)
    cash += partial_sell.net_amount
    assert cash == pytest.approx(84_953.5)
    cash += final_sell.net_amount
    assert cash == pytest.approx(98_419.75)

    buy_cash_out = first_buy.net_amount + second_buy.net_amount
    sell_cash_in = partial_sell.net_amount + final_sell.net_amount
    net_pnl = calculate_net_pnl(buy_cash_out, sell_cash_in)

    assert net_pnl == pytest.approx(-1_580.25)
    assert cash == pytest.approx(initial_cash + net_pnl)
    assert calculate_pnl_percent(net_pnl, buy_cash_out) == pytest.approx(-7.51372940589)
