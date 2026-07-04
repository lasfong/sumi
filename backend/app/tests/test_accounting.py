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

