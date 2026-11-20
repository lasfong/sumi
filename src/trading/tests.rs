use std::collections::BTreeMap;

use crate::data::Timeframe;
use crate::drawings::{
    Drawing, DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStyle, DrawingTool,
    FibDirection,
};
use crate::indicators::OscillatorPaneId;
use crate::model::Candle;
use crate::replay::{ReplayController, ReplayStepUnit};

use super::{
    AccountConfig, FillReason, MAX_PENDING_ORDERS, OrderKind, OrderRequest, OrderSide, OrderStatus,
    PositionSide, TradingError, TradingSession, apply_slippage, compute_commission,
    extract_ticket_from_drawing,
};

#[derive(Default)]
struct MemStorage(BTreeMap<String, String>);

impl eframe::Storage for MemStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

fn candle(ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        timestamp: ts,
        date_label: String::new(),
        open,
        high,
        low,
        close,
        volume: 1_000.0,
    }
}

fn zero_friction_config() -> AccountConfig {
    AccountConfig {
        initial_cash: 100_000.0,
        commission_bps: 0.0,
        slippage_bps: 0.0,
        max_leverage: 1.0,
        allow_short: true,
        ..AccountConfig::default()
    }
}

#[test]
fn market_long_and_short_round_trips_apply_slippage_and_conserve_fees() {
    let candles = vec![
        candle(1_000, 100.0, 102.0, 99.0, 100.0),
        candle(1_060, 100.5, 112.0, 100.0, 110.0),
        candle(1_120, 110.0, 111.0, 94.0, 95.0),
    ];
    let cfg = AccountConfig {
        initial_cash: 100_000.0,
        commission_bps: 10.0, // 0.10%
        slippage_bps: 5.0,    // 0.05%
        max_leverage: 1.0,
        allow_short: true,
        ..AccountConfig::default()
    };
    let mut session = TradingSession::new(cfg);
    session.reset_at(&candles, 0);

    // 1. Market Buy 100 units at bar 0 (`close = 100.0`, exec = `100.05`, fee = `10.005`).
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 100.0,
                stop_loss: Some(92.0),
                take_profit: Some(125.0),
            },
            &candles,
            0,
        )
        .expect("market buy");
    let expected_entry = apply_slippage(100.0, OrderSide::Buy, 5.0);
    assert!((session.open_position().unwrap().avg_entry_price - expected_entry).abs() < 1e-12);

    // Advance to bar 1 (`close = 110.0`) and flatten Long (`exec = 110.0 * 0.9995 = 109.945`).
    session.advance_through_bars(&candles, 1, 1);
    let exit_fill = session.flatten_position(&candles, 1).expect("flatten long");
    assert_eq!(exit_fill.reason, FillReason::ManualFlatten);
    assert!(session.open_position().is_none());
    assert_eq!(session.closed_trades().len(), 1);

    // 2. Open Market Short 80 units at bar 1 (`close = 110.0`) and flatten at bar 2 (`close = 95.0`).
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 80.0,
                stop_loss: Some(120.0),
                take_profit: Some(85.0),
            },
            &candles,
            1,
        )
        .expect("market short");
    assert_eq!(session.open_position().unwrap().side, PositionSide::Short);

    session.advance_through_bars(&candles, 2, 2);
    session
        .flatten_position(&candles, 2)
        .expect("flatten short");
    assert_eq!(session.closed_trades().len(), 2);
    assert!(session.closed_trades()[1].net_pnl > 0.0);
    assert!(session.closed_trades()[1].r_multiple.unwrap() > 1.0);

    // Exact cash conservation identity: `cash_balance == initial_cash + sum(closed_trades.net_pnl)`.
    let sum_net_pnl: f64 = session.closed_trades().iter().map(|t| t.net_pnl).sum();
    assert!(
        (session.cash_balance() - (cfg.initial_cash + sum_net_pnl)).abs() < 1e-9,
        "cash conservation drift: cash={} vs initial+pnl={}",
        session.cash_balance(),
        cfg.initial_cash + sum_net_pnl
    );
}

#[test]
fn pending_limit_and_stop_orders_handle_gap_opens_and_never_fill_on_submission_bar() {
    let candles = vec![
        candle(1_000, 100.0, 105.0, 95.0, 100.0), // Bar 0: wide range, pending order submitted here must NOT fill on bar 0!
        candle(1_060, 94.0, 99.0, 93.0, 98.0), // Bar 1: gap-down open (`94 <= 96`) -> Buy Limit @ 96 gets price improvement at 94!
        candle(1_120, 106.0, 110.0, 104.0, 108.0), // Bar 2: gap-up open (`106 >= 103`) -> Buy Stop @ 103 fills adversely at 106!
    ];
    let mut session = TradingSession::new(zero_friction_config());
    session.reset_at(&candles, 0);

    // Submit Buy Limit @ 96 on bar 0 (even though bar 0 had `low = 95`, pending orders only evaluate on `b > 0`).
    let lim_id = session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Limit { limit_price: 96.0 },
                quantity: 10.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            0,
        )
        .expect("submit buy limit");
    assert!(session.open_position().is_none());
    assert_eq!(session.orders()[0].id, lim_id);
    assert_eq!(session.orders()[0].status, OrderStatus::Pending);

    // Bar 1 opens at `94.0 <= 96.0` -> price-improved fill at `94.0`!
    session.advance_through_bars(&candles, 1, 1);
    assert_eq!(session.fills().len(), 1);
    assert_eq!(session.fills()[0].reason, FillReason::LimitTriggered);
    assert!((session.fills()[0].price - 94.0).abs() < 1e-12);
    session.flatten_position(&candles, 1).unwrap();

    // Submit Buy Stop @ 103 on bar 1 (`close = 98.0`); bar 2 gaps open at `106.0 >= 103.0` -> fills at `106.0`!
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Stop { stop_price: 103.0 },
                quantity: 10.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            1,
        )
        .expect("submit buy stop");
    session.advance_through_bars(&candles, 2, 2);
    let last_fill = session.fills().last().unwrap();
    assert_eq!(last_fill.reason, FillReason::StopTriggered);
    assert!((last_fill.price - 106.0).abs() < 1e-12);
}

#[test]
fn dual_touch_bar_resolves_pessimistically_sl_before_tp_and_post_entry_follows_4_waypoint_path() {
    // Part A: Pre-existing Long position (`entry=100, SL=95, TP=110`) encounters an outside bar (`O=100, H=112, L=94, C=108`).
    // Both SL (`94 <= 95`) and TP (`112 >= 110`) are touched intrabar -> MUST exit via `StopLossExit` (`DEF-005` guard).
    let candles = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(1_060, 100.0, 112.0, 94.0, 108.0),
    ];
    let mut session = TradingSession::new(zero_friction_config());
    session.reset_at(&candles, 0);
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: Some(95.0),
                take_profit: Some(110.0),
            },
            &candles,
            0,
        )
        .unwrap();
    session.advance_through_bars(&candles, 1, 1);
    assert!(session.open_position().is_none());
    assert_eq!(
        session.fills().last().unwrap().reason,
        FillReason::StopLossExit
    );
    assert!((session.fills().last().unwrap().price - 95.0).abs() < 1e-12);

    // Part B: Pending `Buy Stop @ 105` (`SL = 98, TP = 112`) on a bullish bar `O=100, L=96, H=107, C=106`.
    // Because bullish bar path is `[O=100 -> L=96 -> H=107 -> C=106]`, `L=96` happened *before* `Buy Stop @ 105`
    // triggered on `L -> H`, so `SL = 98` must NOT falsely trigger after entry!
    let breakout_candles = vec![
        candle(2_000, 100.0, 101.0, 99.0, 100.0),
        candle(2_060, 100.0, 107.0, 96.0, 106.0),
    ];
    let mut s2 = TradingSession::new(zero_friction_config());
    s2.reset_at(&breakout_candles, 0);
    s2.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Stop { stop_price: 105.0 },
            quantity: 10.0,
            stop_loss: Some(98.0),
            take_profit: Some(112.0),
        },
        &breakout_candles,
        0,
    )
    .unwrap();
    s2.advance_through_bars(&breakout_candles, 1, 1);
    assert!(
        s2.open_position().is_some(),
        "position should remain open since L=96 occurred prior to Buy Stop @ 105 along [O, L, H, C]"
    );
    assert_eq!(s2.fills().len(), 1);
}

#[test]
fn scale_ins_partial_closes_flips_and_insufficient_flip_rollback_conserve_fees() {
    let candles = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(1_060, 100.0, 121.0, 99.0, 120.0),
        candle(1_120, 120.0, 132.0, 118.0, 130.0),
        candle(1_180, 130.0, 131.0, 109.0, 110.0),
    ];
    let cfg = AccountConfig {
        initial_cash: 10_000.0,
        commission_bps: 8.0,
        slippage_bps: 3.0,
        max_leverage: 1.0,
        allow_short: true,
        ..AccountConfig::default()
    };
    let mut session = TradingSession::new(cfg);
    session.reset_at(&candles, 0);

    // Buy 20 @ ~100, then scale-in Buy 10 @ ~120 (`total = 30`).
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 20.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            0,
        )
        .unwrap();
    session.advance_through_bars(&candles, 1, 1);
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            1,
        )
        .unwrap();
    assert!((session.open_position().unwrap().quantity - 30.0).abs() < 1e-12);

    // Partial close 12 units at bar 2 (`remaining = 18`).
    session.advance_through_bars(&candles, 2, 2);
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 12.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            2,
        )
        .unwrap();
    assert!((session.open_position().unwrap().quantity - 18.0).abs() < 1e-12);
    assert_eq!(session.closed_trades().len(), 0); // Lifecycle still open!

    // Attempt an unaffordable flip (`Sell 500` @ 130 => flip leg `482 * 130 = 62,660 > ~10,500` cash):
    // MUST reject without closing or mutating the open `18.0` Long position!
    let before_pos = session.open_position().cloned().unwrap();
    let before_cash = session.cash_balance();
    let err = session
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 500.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            2,
        )
        .expect_err("unaffordable flip must fail atomically");
    assert!(matches!(err, TradingError::InsufficientBuyingPower { .. }));
    assert_eq!(session.open_position(), Some(&before_pos));
    assert!((session.cash_balance() - before_cash).abs() < 1e-12);

    // Valid atomic flip (`Sell 28` @ bar 2 => closes `18` Long into `ClosedTrade #1` + opens `10` Short!).
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 28.0,
                stop_loss: Some(145.0),
                take_profit: Some(100.0),
            },
            &candles,
            2,
        )
        .expect("valid atomic flip");
    assert_eq!(session.closed_trades().len(), 1);
    assert_eq!(session.open_position().unwrap().side, PositionSide::Short);
    assert!((session.open_position().unwrap().quantity - 10.0).abs() < 1e-12);

    // Flatten remaining Short at bar 3 and verify machine-precision fee/cash conservation!
    session.advance_through_bars(&candles, 3, 3);
    session.flatten_position(&candles, 3).unwrap();
    assert_eq!(session.closed_trades().len(), 2);

    let sum_net_pnl: f64 = session.closed_trades().iter().map(|t| t.net_pnl).sum();
    let sum_fill_fees: f64 = session.fills().iter().map(|f| f.fee).sum();
    let sum_trade_fees: f64 = session.closed_trades().iter().map(|t| t.total_fees).sum();
    assert!((sum_fill_fees - sum_trade_fees).abs() < 1e-9);
    assert!((session.cash_balance() - (cfg.initial_cash + sum_net_pnl)).abs() < 1e-9);
}

#[test]
fn deterministic_command_log_rewind_restores_bit_exact_state() {
    let candles: Vec<Candle> = (0..60)
        .map(|i| {
            let base = 500.0 + ((i as f64) * 0.4).sin() * 18.0 + (i as f64) * 0.6;
            candle(
                1_700_000_000 + (i as i64) * 60,
                base,
                base + 6.0,
                base - 5.5,
                base + 1.2,
            )
        })
        .collect();

    let mut session = TradingSession::default();
    session.reset_at(&candles, 10);
    session.advance_through_bars(&candles, 11, 15);
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 15.0,
                stop_loss: Some(460.0),
                take_profit: Some(590.0),
            },
            &candles,
            15,
        )
        .unwrap();

    session.advance_through_bars(&candles, 16, 22);
    let pending_id = session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Limit { limit_price: 495.0 },
                quantity: 10.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            22,
        )
        .unwrap();

    session.advance_through_bars(&candles, 23, 28);
    session
        .update_position_brackets(Some(475.0), Some(580.0), &candles, 28)
        .unwrap();

    session.advance_through_bars(&candles, 29, 35);

    // Snapshot exact state at bar 35 BEFORE continuing into future bars `36..55`.
    let snap_cash = session.cash_balance();
    let snap_pos = session.open_position().cloned();
    let snap_orders = session.orders().to_vec();
    let snap_fills = session.fills().to_vec();
    let snap_trades = session.closed_trades().to_vec();
    let snap_curve = session.equity_curve().to_vec();
    let snap_dd = session.max_drawdown_pct();

    // Mutate heavily in future bars `36..55` (cancel, flatten, open short, etc.).
    session.advance_through_bars(&candles, 36, 40);
    if session
        .orders()
        .iter()
        .any(|o| o.id == pending_id && o.status == OrderStatus::Pending)
    {
        let _ = session.cancel_order(pending_id, &candles, 40);
    }
    if session.open_position().is_some() {
        let _ = session.flatten_position(&candles, 40);
    }
    session.advance_through_bars(&candles, 41, 50);
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 20.0,
                stop_loss: Some(650.0),
                take_profit: Some(450.0),
            },
            &candles,
            50,
        )
        .unwrap();
    session.advance_through_bars(&candles, 51, 55);

    // Rewind back to bar 35 -> must match the bar-35 snapshot BIT-FOR-BIT!
    session.rewind_to(&candles, 10, 35);
    assert_eq!(session.cash_balance().to_bits(), snap_cash.to_bits());
    assert_eq!(session.open_position(), snap_pos.as_ref());
    assert_eq!(session.orders(), snap_orders.as_slice());
    assert_eq!(session.fills(), snap_fills.as_slice());
    assert_eq!(session.closed_trades(), snap_trades.as_slice());
    assert_eq!(session.equity_curve(), snap_curve.as_slice());
    assert_eq!(session.max_drawdown_pct().to_bits(), snap_dd.to_bits());

    // Persistence round-trip + `sanitize_and_rebuild` also reproduces the exact same ledger!
    let mut storage = MemStorage::default();
    eframe::set_value(&mut storage, "trading_test", &session);
    let mut restored: TradingSession =
        eframe::get_value(&storage, "trading_test").expect("deserialize session");
    restored.sanitize_and_rebuild(&candles, 35);
    assert_eq!(restored.cash_balance().to_bits(), snap_cash.to_bits());
    assert_eq!(restored.fills(), snap_fills.as_slice());
}

#[test]
fn multi_timeframe_15m_stepping_matches_1m_tick_execution_identically() {
    let candles: Vec<Candle> = (0..90)
        .map(|i| {
            let base = 200.0 + ((i as f64) * 0.25).cos() * 12.0;
            candle(
                1_772_442_000 + (i as i64) * 60,
                base,
                base + 4.5,
                base - 4.5,
                base + 0.5,
            )
        })
        .collect();

    let order_req = OrderRequest {
        side: OrderSide::Buy,
        kind: OrderKind::Limit { limit_price: 192.0 },
        quantity: 25.0,
        stop_loss: Some(184.0),
        take_profit: Some(208.0),
    };

    // Session A: step `1m` tick-by-tick (`BaseBar`) from bar `14` to `89`.
    let mut rep_1m = ReplayController::new();
    rep_1m.start_at_base_index(&candles, 14);
    rep_1m.step_unit = ReplayStepUnit::BaseBar;
    let mut sess_1m = TradingSession::default();
    sess_1m.reset_at(&candles, 14);
    sess_1m.submit_order(order_req, &candles, 14).unwrap();
    while let Some(tr) = rep_1m.step_forward(&candles, Timeframe::M1, 1) {
        sess_1m.advance_through_bars(&candles, tr.prev_base_index + 1, tr.new_base_index);
    }

    // Session B: step `15m` bar-by-bar (`ActiveBar`) from bar `14` to `89`.
    let mut rep_15m = ReplayController::new();
    rep_15m.start_at_base_index(&candles, 14);
    rep_15m.step_unit = ReplayStepUnit::ActiveBar;
    let mut sess_15m = TradingSession::default();
    sess_15m.reset_at(&candles, 14);
    sess_15m.submit_order(order_req, &candles, 14).unwrap();
    while let Some(tr) = rep_15m.step_forward(&candles, Timeframe::M15, 1) {
        sess_15m.advance_through_bars(&candles, tr.prev_base_index + 1, tr.new_base_index);
    }

    assert_eq!(sess_1m.fills(), sess_15m.fills());
    assert_eq!(sess_1m.closed_trades(), sess_15m.closed_trades());
    assert_eq!(
        sess_1m.cash_balance().to_bits(),
        sess_15m.cash_balance().to_bits()
    );
}

#[test]
fn drawing_ticket_extraction_and_input_hardening_guards() {
    let fib_long = Drawing::new(
        1,
        DrawingPaneTarget::Main,
        DrawingGeometry::FibonacciRetracement {
            start: DrawingAnchor::new(10.0, 100.0),
            end: DrawingAnchor::new(25.0, 150.0),
            direction: FibDirection::StartToEnd,
            extend_right: false,
        },
        DrawingStyle::default_for_tool(DrawingTool::FibonacciRetracement),
    );
    let sync = extract_ticket_from_drawing(&fib_long).expect("fib long ticket");
    assert_eq!(sync.side, OrderSide::Buy);
    assert_eq!(sync.stop_loss, Some(100.0));
    assert_eq!(sync.take_profit, Some(150.0));
    assert!(sync.entry_price > 100.0 && sync.entry_price < 150.0);

    // Oscillator-pane drawings must never synthesize main-instrument price orders.
    let rsi_line = Drawing::new(
        2,
        DrawingPaneTarget::Oscillator(OscillatorPaneId::Rsi),
        DrawingGeometry::HorizontalLine {
            anchor: DrawingAnchor::new(5.0, 70.0),
        },
        DrawingStyle::default_for_tool(DrawingTool::HorizontalLine),
    );
    assert!(extract_ticket_from_drawing(&rsi_line).is_none());

    // Input hardening: NaN, zero, short-selling disabled, pending order cap.
    let candles = vec![candle(1_000, 100.0, 105.0, 95.0, 100.0)];
    let mut cfg = zero_friction_config();
    cfg.allow_short = false;
    let mut session = TradingSession::new(cfg);
    session.reset_at(&candles, 0);

    assert!(matches!(
        session.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: f64::NAN,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            0
        ),
        Err(TradingError::InvalidQuantity(_))
    ));
    assert_eq!(
        session.submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 1.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            0
        ),
        Err(TradingError::ShortSellingDisabled)
    );

    for _ in 0..MAX_PENDING_ORDERS {
        session
            .submit_order(
                OrderRequest {
                    side: OrderSide::Buy,
                    kind: OrderKind::Limit { limit_price: 50.0 },
                    quantity: 1.0,
                    stop_loss: None,
                    take_profit: None,
                },
                &candles,
                0,
            )
            .unwrap();
    }
    assert_eq!(
        session.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Limit { limit_price: 50.0 },
                quantity: 1.0,
                stop_loss: None,
                take_profit: None,
            },
            &candles,
            0,
        ),
        Err(TradingError::TooManyPendingOrders(MAX_PENDING_ORDERS))
    );
    assert!(compute_commission(100.0, 10.0, 5.0) > 0.0);
}

#[test]
fn critic_r07_regression_pure_reduce_stale_brackets_and_gap_open_bracket_sanitization() {
    // `[CRITICAL-02]` Counterexample 1:
    // Bar 0 (`close = 100`): Open `Long 10 @ 100` (`SL = 95, TP = 115`), then submit `Sell Limit 10 @ 110`
    // carrying stale Ticket UI `SL = 95, TP = 115` (`is_pure_reduce == true`).
    // Bar 1 (`low = 94`): `Long 10` exits via `StopLossExit @ 95`.
    // Bar 2 (`high = 111`): `Sell Limit 10 @ 110` triggers and opens `Short 10 @ 110`.
    // Stale `SL = 95, TP = 115` MUST have been stripped so `Short 10 @ 110` does NOT instantly self-stop-out!
    let candles = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(1_060, 99.0, 100.0, 94.0, 96.0),
        candle(1_120, 102.0, 111.0, 101.0, 108.0),
    ];
    let mut cfg = zero_friction_config();
    cfg.commission_bps = 10.0;
    cfg.slippage_bps = 5.0;
    let mut session = TradingSession::new(cfg);
    session.reset_at(&candles, 0);

    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: Some(95.0),
                take_profit: Some(115.0),
            },
            &candles,
            0,
        )
        .unwrap();
    let reduce_ord_id = session
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Limit { limit_price: 110.0 },
                quantity: 10.0,
                stop_loss: Some(95.0),
                take_profit: Some(115.0),
            },
            &candles,
            0,
        )
        .unwrap();

    // Verify stale opposite-side brackets were stripped on the working order itself:
    let working = session
        .orders()
        .iter()
        .find(|o| o.id == reduce_ord_id)
        .unwrap();
    assert_eq!(working.stop_loss, None);
    assert_eq!(working.take_profit, None);

    session.advance_through_bars(&candles, 1, 2);
    let pos = session
        .open_position()
        .expect("Short 10 @ ~110 must remain open without phantom StopLossExit");
    assert_eq!(pos.side, PositionSide::Short);
    assert!((pos.quantity - 10.0).abs() < 1e-12);
    assert_eq!(pos.stop_loss, None);
    assert_eq!(pos.take_profit, None);
    assert_eq!(
        session.fills().len(),
        3,
        "expected exactly 3 fills (Buy Market, SL Exit, Sell Limit), no phantom 4th exit"
    );

    // `[CRITICAL-02]` Counterexample 2 (Defense-in-depth on Gap-Open):
    // Flat account submits valid `Sell Limit 5 @ 110 (SL = 115, TP = 95)` on bar 0.
    // Bar 1 gaps open at `120.0 > SL (115.0)`! Price-improved `Short @ 120.0` must drop `SL = 115 < 120`
    // rather than immediately triggering a bogus stop-loss exit at `120.0`!
    let gap_candles = vec![
        candle(2_000, 100.0, 101.0, 99.0, 100.0),
        candle(2_060, 120.0, 121.0, 118.0, 119.0),
    ];
    let mut s_gap = TradingSession::new(zero_friction_config());
    s_gap.reset_at(&gap_candles, 0);
    s_gap
        .submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Limit { limit_price: 110.0 },
                quantity: 5.0,
                stop_loss: Some(115.0),
                take_profit: Some(95.0),
            },
            &gap_candles,
            0,
        )
        .unwrap();
    s_gap.advance_through_bars(&gap_candles, 1, 1);
    let gap_pos = s_gap
        .open_position()
        .expect("gap-improved Short @ 120 must stay open");
    assert_eq!(gap_pos.side, PositionSide::Short);
    assert!((gap_pos.avg_entry_price - 120.0).abs() < 1e-12);
    assert_eq!(gap_pos.stop_loss, None);
    assert_eq!(gap_pos.take_profit, Some(95.0));
}

#[test]
fn critic_r07_regression_multi_pending_causal_segment_order_scale_in_r_multiple_and_max_price() {
    use crate::drawings::{
        Drawing, DrawingAnchor, DrawingGeometry, DrawingPaneTarget, DrawingStyle, DrawingTool,
        FibDirection,
    };
    use crate::trading::orders::MAX_ORDER_PRICE;

    // 1. `[MEDIUM-01]` Case A: `Long 10 @ 100` + `Order #1: Sell Limit 10 @ 105` + `Order #2: Sell Stop 10 @ 95`
    //    on bullish bar `[O = 100, L = 94, H = 106, C = 102]`.
    //    Along `[O -> L -> H -> C]`, `L = 94` happens BEFORE `H = 106`, so `Order #2` (`Sell Stop @ 95`)
    //    MUST close the Long first (`-50`), and `Order #1` (`Sell Limit @ 105`) opens `Short 10 @ 105` second!
    let c_bull = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(1_060, 100.0, 106.0, 94.0, 102.0),
    ];
    let mut s1 = TradingSession::new(zero_friction_config());
    s1.reset_at(&c_bull, 0);
    s1.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 10.0,
            stop_loss: None,
            take_profit: None,
        },
        &c_bull,
        0,
    )
    .unwrap();
    s1.submit_order(
        OrderRequest {
            side: OrderSide::Sell,
            kind: OrderKind::Limit { limit_price: 105.0 },
            quantity: 10.0,
            stop_loss: None,
            take_profit: None,
        },
        &c_bull,
        0,
    )
    .unwrap();
    s1.submit_order(
        OrderRequest {
            side: OrderSide::Sell,
            kind: OrderKind::Stop { stop_price: 95.0 },
            quantity: 10.0,
            stop_loss: None,
            take_profit: None,
        },
        &c_bull,
        0,
    )
    .unwrap();
    s1.advance_through_bars(&c_bull, 1, 1);
    assert_eq!(s1.fills().len(), 3);
    assert_eq!(s1.fills()[1].reason, FillReason::StopTriggered);
    assert!((s1.fills()[1].price - 95.0).abs() < 1e-12);
    assert_eq!(s1.fills()[2].reason, FillReason::LimitTriggered);
    assert!((s1.fills()[2].price - 105.0).abs() < 1e-12);
    assert!((s1.closed_trades()[0].net_pnl - (-50.0)).abs() < 1e-12);
    assert_eq!(s1.open_position().unwrap().side, PositionSide::Short);

    // 2. `[MEDIUM-01]` Case B: Flat account places `Buy Limit 5 @ 98 (TP = 99)` and `Buy Limit 5 @ 95 (SL = 92)`.
    //    Bullish bar `[O = 99.5, L = 94.0, H = 100.0, C = 99.6]` fills BOTH scale-in limits on `O -> L`
    //    before exiting ALL `10` units at `TP = 99.0` on `L -> H`!
    let c_scale = vec![
        candle(2_000, 99.5, 100.0, 99.0, 99.5),
        candle(2_060, 99.5, 100.0, 94.0, 99.6),
    ];
    let mut s2 = TradingSession::new(zero_friction_config());
    s2.reset_at(&c_scale, 0);
    s2.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Limit { limit_price: 98.0 },
            quantity: 5.0,
            stop_loss: None,
            take_profit: Some(99.0),
        },
        &c_scale,
        0,
    )
    .unwrap();
    s2.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Limit { limit_price: 95.0 },
            quantity: 5.0,
            stop_loss: Some(92.0),
            take_profit: None,
        },
        &c_scale,
        0,
    )
    .unwrap();
    s2.advance_through_bars(&c_scale, 1, 1);
    assert!(s2.open_position().is_none());
    assert_eq!(s2.closed_trades().len(), 1);
    assert!((s2.closed_trades()[0].quantity - 10.0).abs() < 1e-12);
    assert!((s2.closed_trades()[0].net_pnl - 25.0).abs() < 1e-12); // 5*(99-98) + 5*(99-95) = 5 + 20 = +25

    // 3. `[LOW-01]`: Scale-in updates `initial_risk_amount` so `Buy 1 @ 100 (SL=99)` + `Buy 99 @ 100 (SL=99)`
    //    exiting at `102` (`+$200`) reports `+2.0R`, NOT `+200.0R`!
    let c_rmult = vec![
        candle(3_000, 100.0, 101.0, 99.5, 100.0),
        candle(3_060, 100.0, 103.0, 99.5, 102.0),
    ];
    let mut s3 = TradingSession::new(zero_friction_config());
    s3.reset_at(&c_rmult, 0);
    s3.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 1.0,
            stop_loss: Some(99.0),
            take_profit: Some(102.0),
        },
        &c_rmult,
        0,
    )
    .unwrap();
    s3.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 99.0,
            stop_loss: Some(99.0),
            take_profit: Some(102.0),
        },
        &c_rmult,
        0,
    )
    .unwrap();
    s3.advance_through_bars(&c_rmult, 1, 1);
    let r_mult = s3.closed_trades()[0].r_multiple.expect("r_multiple");
    assert!(
        (r_mult - 2.0).abs() < 1e-9,
        "expected +2.0R after 1+99 scale-in, got {r_mult}"
    );

    // 4. `[LOW-03]`: `extract_ticket_from_drawing` rejects drawings above `MAX_ORDER_PRICE`.
    let huge_fib = Drawing::new(
        99,
        DrawingPaneTarget::Main,
        DrawingGeometry::FibonacciRetracement {
            start: DrawingAnchor::new(0.0, 100.0),
            end: DrawingAnchor::new(10.0, MAX_ORDER_PRICE * 10.0),
            direction: FibDirection::StartToEnd,
            extend_right: false,
        },
        DrawingStyle::default_for_tool(DrawingTool::FibonacciRetracement),
    );
    assert!(extract_ticket_from_drawing(&huge_fib).is_none());

    // 5. `RiskRewardBox` exact Long & Short order ticket extraction (`Milestone R10`):
    let rr_long = Drawing::new(
        100,
        DrawingPaneTarget::Main,
        DrawingGeometry::RiskRewardBox {
            start: DrawingAnchor::new(10.0, 128.5),
            end: DrawingAnchor::new(25.0, 137.8),
            stop_price: 124.2,
        },
        DrawingStyle::default_for_tool(DrawingTool::RiskRewardBox),
    );
    let sync_l = extract_ticket_from_drawing(&rr_long).expect("long RR ticket");
    assert_eq!(sync_l.side, OrderSide::Buy);
    assert!((sync_l.entry_price - 128.5).abs() < 1e-9);
    assert_eq!(sync_l.stop_loss, Some(124.2));
    assert_eq!(sync_l.take_profit, Some(137.8));

    let rr_short = Drawing::new(
        101,
        DrawingPaneTarget::Main,
        DrawingGeometry::RiskRewardBox {
            start: DrawingAnchor::new(10.0, 128.5),
            end: DrawingAnchor::new(25.0, 118.0),
            stop_price: 133.5,
        },
        DrawingStyle::default_for_tool(DrawingTool::RiskRewardBox),
    );
    let sync_s = extract_ticket_from_drawing(&rr_short).expect("short RR ticket");
    assert_eq!(sync_s.side, OrderSide::Sell);
    assert!((sync_s.entry_price - 128.5).abs() < 1e-9);
    assert_eq!(sync_s.stop_loss, Some(133.5));
    assert_eq!(sync_s.take_profit, Some(118.0));
}

#[test]
fn r17_tamper_proof_initial_sl_adherence_and_zero_open_guard() {
    use crate::data::Timeframe;
    use crate::journal::{ChecklistState, MistakeTag, SetupTag};
    use crate::trading::{
        CoachingSeverity, PracticeDecisionAction, PracticeDecisionLog, PracticeScoreboardSummary,
        SessionDebriefReport,
    };

    let bars = vec![
        candle(10_000, 100.0, 101.0, 99.0, 100.0),
        candle(10_060, 100.0, 102.0, 98.0, 99.0),
        candle(10_120, 99.0, 106.0, 98.5, 105.0),
        candle(10_180, 105.0, 107.0, 104.0, 105.0),
    ];
    let mut session = TradingSession::new(zero_friction_config());
    session.reset_at(&bars, 0);
    let mut dec_log = PracticeDecisionLog::new();

    // 1. Zero opened positions: `sl_adherence_pct == None` and ZERO contradictory `< 80%` callout!
    let m0 = session.metrics(100.0);
    assert_eq!(m0.total_opened_positions, 0);
    assert_eq!(m0.sl_adherence_pct, None);
    let deb0 = SessionDebriefReport::build(
        "FPT",
        Timeframe::D1,
        &session,
        100.0,
        &[],
        &dec_log,
        true,
        0,
        Some(10_000),
    );
    assert_eq!(deb0.sl_adherence_pct, None);
    assert!(
        deb0.callouts
            .iter()
            .all(|c| c.severity != CoachingSeverity::Critical),
        "empty session must NEVER emit a false-alarm Critical <80% SL callout"
    );

    // 2. Open Position #1 NAKED (`stop_loss: None`), then attach SL via `update_position_brackets`
    //    AND scale in with SL -> `had_initial_sl_at_open` MUST remain `false`!
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: None,
                take_profit: None,
            },
            &bars,
            0,
        )
        .unwrap();
    session
        .update_position_brackets(Some(95.0), Some(112.0), &bars, 0)
        .unwrap();
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: Some(96.0),
                take_profit: Some(112.0),
            },
            &bars,
            0,
        )
        .unwrap();
    let open_p = session.open_position().expect("open position");
    assert!(!open_p.had_initial_sl_at_open);
    assert_eq!(open_p.entry_stop_loss, None);
    assert_eq!(session.metrics(100.0).sl_adherence_pct, Some(0.0));

    session.advance_through_bars(&bars, 1, 1);
    session.flatten_position(&bars, 1).unwrap(); // closed at 99.0 (loss without initial SL)
    assert!(!session.closed_trades()[0].had_initial_sl_at_open);

    // 3. Open Position #2 WITH valid initial SL (`97.0`) and close breakeven, then Position #3 WITH initial SL (`103.0`)
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: Some(97.0),
                take_profit: Some(110.0),
            },
            &bars,
            1,
        )
        .unwrap();
    session.flatten_position(&bars, 1).unwrap(); // breakeven `0.0` PnL!

    session.advance_through_bars(&bars, 2, 2);
    session
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: Some(103.0),
                take_profit: Some(112.0),
            },
            &bars,
            2,
        )
        .unwrap();

    let _ = dec_log.record_note(
        "FPT",
        Timeframe::D1,
        1,
        10_060,
        99.0,
        PracticeDecisionAction::PlanLong,
        SetupTag::TrendPullback,
        MistakeTag::None,
        4,
        ChecklistState::all_checked(),
        "Bar 1 note",
    );
    let _ = dec_log.record_note(
        "FPT",
        Timeframe::D1,
        3,
        10_180,
        105.0,
        PracticeDecisionAction::ObserveHold,
        SetupTag::SqueezeBreakout,
        MistakeTag::FomoChase,
        2,
        ChecklistState::empty(),
        "Future bar 3 note",
    );

    // At bar 2 (`ts = 10_120`), bar-3 note MUST be causally hidden (`DEF-001`)!
    let sb =
        PracticeScoreboardSummary::compute(&session, 105.0, &dec_log, "FPT", true, 2, Some(10_120));
    assert_eq!(sb.closed_trades, 2);
    assert_eq!(sb.breakeven_count, 1);
    assert_eq!(sb.loss_count, 1);
    assert_eq!(sb.visible_decision_notes, 1);
    assert!((sb.sl_adherence_pct.unwrap() - (200.0 / 3.0)).abs() < 1e-6);

    let deb = SessionDebriefReport::build(
        "FPT",
        Timeframe::D1,
        &session,
        105.0,
        &[],
        &dec_log,
        true,
        2,
        Some(10_120),
    );
    assert_eq!(deb.total_opened_positions, 3);
    assert_eq!(deb.sl_adherent_positions, 2);
    assert_eq!(deb.visible_decision_notes.len(), 1);
    assert!(
        deb.callouts
            .iter()
            .any(|c| c.severity == CoachingSeverity::Critical && c.title.contains("< 80%"))
    );
}

#[test]
fn r17_practice_and_scan_stores_ron_json_roundtrip_pin_eviction_and_mismatch() {
    use crate::data::{DataSourcePreset, Timeframe};
    use crate::scanner::{
        HistoricalSignalHit, MarketRegime, ScanSignalKind, ScannerFilter, ScannerReplayIntent,
        ScannerSignalContext, SignalDirection, WatchlistSortColumn,
    };
    use crate::trading::{
        MAX_SAVED_PRACTICE_SESSIONS, PracticeSessionStore, SavedPracticeSession, SavedScanSnapshot,
        ScanSnapshotStore,
    };
    use crate::viewport::WorkspaceTemplateError;

    let bars = vec![
        candle(1_000, 50.0, 52.0, 49.0, 51.0),
        candle(2_000, 51.0, 54.0, 50.5, 53.0),
    ];
    let hit = HistoricalSignalHit {
        instrument_id: "Preset:VN30 Basket:FPT".to_owned(),
        symbol: "FPT".to_owned(),
        timeframe: Timeframe::D1,
        bar_index: 1,
        timestamp: 2_000,
        price: 53.0,
        signal: ScanSignalKind::MacdBullCross,
        direction: SignalDirection::Bullish,
        regime: MarketRegime::Uptrend,
        metric_value: 1.25,
    };

    let mut p_store = PracticeSessionStore::new();
    for idx in 0..MAX_SAVED_PRACTICE_SESSIONS {
        p_store
            .save_session(SavedPracticeSession {
                slot_id: 0,
                label: format!("Pinned #{idx}"),
                pinned: true,
                sequence_no: 0,
                data_source: DataSourcePreset::Vn30Basket,
                symbol: "FPT".to_owned(),
                base_timeframe: Timeframe::D1,
                active_timeframe: Timeframe::D1,
                expected_bar_count: bars.len(),
                expected_first_timestamp: Some(1_000),
                expected_last_timestamp: Some(2_000),
                replay_active: true,
                replay_start_base_index: 0,
                replay_cursor_base_index: 1,
                blind_lookback_bars: 120,
                scanner_context: Some(ScannerSignalContext::with_reveal_and_preroll(
                    ScannerReplayIntent::BlindPractice,
                    hit.clone(),
                    2_000,
                    Some(120),
                )),
                account_config: zero_friction_config(),
                session_start_base_index: 0,
                commands: Vec::new(),
                decision_notes: Vec::new(),
                session_digest: 0,
            })
            .unwrap();
    }
    assert_eq!(
        p_store.save_session(p_store.slots[0].clone().tap_label("Overflow")),
        Err(WorkspaceTemplateError::SnapshotSlotsFull)
    );

    let json_doc = p_store.to_json_pretty();
    let ron_doc = p_store.to_ron_pretty();
    let from_j = PracticeSessionStore::from_json_str(&json_doc).expect("JSON round-trip");
    let from_r = PracticeSessionStore::from_ron_str(&ron_doc).expect("RON round-trip");
    assert_eq!(from_j.slots.len(), MAX_SAVED_PRACTICE_SESSIONS);
    assert_eq!(from_j, p_store);
    assert_eq!(from_r, p_store);
    assert_eq!(
        from_j.slots[0]
            .scanner_context
            .as_ref()
            .and_then(|c| c.pre_roll_bars()),
        Some(120)
    );

    // Tampered digest MUST fail closed
    let tampered = json_doc.replacen(
        "\"session_digest_hex\": \"",
        "\"session_digest_hex\": \"ff",
        1,
    );
    assert!(PracticeSessionStore::from_json_str(&tampered).is_err());

    // Dataset mismatch check
    assert!(
        from_j.slots[0]
            .verify_dataset_match(DataSourcePreset::Vn30Basket, "FPT", &bars)
            .is_none()
    );
    assert!(
        from_j.slots[0]
            .verify_dataset_match(DataSourcePreset::CustomFile, "FPT", &bars[..1])
            .is_some()
    );

    let mut s_store = ScanSnapshotStore::new();
    s_store
        .save_snapshot(SavedScanSnapshot {
            slot_id: 0,
            label: "VN30 Bull Scan".to_owned(),
            pinned: true,
            sequence_no: 0,
            captured_cutoff_timestamp: Some(2_000),
            timeframe: Timeframe::D1,
            signal_lookback_bars: 80,
            blind_practice_lookback_bars: 120,
            filter: ScannerFilter {
                search_query: "FPT".to_owned(),
                signal_filter: Some(ScanSignalKind::MacdBullCross),
                regime_filter: Some(MarketRegime::Uptrend),
                direction_filter: Some(SignalDirection::Bullish),
                only_active_signals: true,
            },
            sort_col: WatchlistSortColumn::RelativeVolume,
            sort_asc: false,
            hits: vec![hit],
            snapshot_digest: 0,
        })
        .unwrap();
    let s_json = s_store.to_json_pretty();
    let s_ron = s_store.to_ron_pretty();
    assert_eq!(ScanSnapshotStore::from_json_str(&s_json).unwrap(), s_store);
    assert_eq!(ScanSnapshotStore::from_ron_str(&s_ron).unwrap(), s_store);
}

#[test]
fn critic_r17_regression_same_bar_exit_entry_note_isolation_journal_none_override_and_preroll_digest()
 {
    use crate::data::{DataSourcePreset, Timeframe};
    use crate::journal::{ChecklistState, ExchangeVenue, MistakeTag, SetupTag, TradeJournalStore};
    use crate::scanner::{
        HistoricalSignalHit, MarketRegime, ScanSignalKind, ScannerReplayIntent,
        ScannerSignalContext, SignalDirection,
    };
    use crate::trading::{
        PracticeDecisionAction, PracticeDecisionLog, PracticeSessionStore, SavedPracticeSession,
        SessionDebriefReport,
    };

    // 1. `[Major-01]`: Trade #1 (`entry = 0, exit = 1`, `-241.11`, disciplined pullback with SL) exits on bar 1,
    // and Trade #2 (`entry = 1, exit = 2`, `+541.11`, naked FOMO chase) opens on the SAME bar 1 with a note at bar 1!
    let bars = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(2_000, 98.0, 99.0, 97.0, 97.5889), // Trade #1 flattens for -241.11 (qty 100), Trade #2 opens @ 97.5889
        candle(3_000, 100.0, 104.0, 99.0, 103.0), // Trade #2 flattens @ 103.0 (+541.11)
    ];
    let mut sess = TradingSession::new(zero_friction_config());
    sess.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 100.0,
            stop_loss: Some(94.0),
            take_profit: Some(112.0),
        },
        &bars,
        0,
    )
    .unwrap();
    let mut log = PracticeDecisionLog::new();
    let _ = log.record_note(
        "FPT",
        Timeframe::D1,
        0,
        bars[0].timestamp,
        bars[0].close,
        PracticeDecisionAction::PlanLong,
        SetupTag::TrendPullback,
        MistakeTag::EarlyExit, // Tentative pre-trade mistake that trader later clears in Journal!
        4,
        ChecklistState::default(),
        "Initial tentative note",
    );
    sess.flatten_position(&bars, 1).unwrap();

    // Trade #2 opens naked on bar 1 (`same bar` as Trade #1 exit!) + stamps FOMO Chase note at bar 1
    sess.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 100.0,
            stop_loss: None,
            take_profit: None,
        },
        &bars,
        1,
    )
    .unwrap();
    let _ = log.record_note(
        "FPT",
        Timeframe::D1,
        1,
        bars[1].timestamp,
        bars[1].close,
        PracticeDecisionAction::ManagePosition,
        SetupTag::MomentumContinuation,
        MistakeTag::FomoChase,
        2,
        ChecklistState::default(),
        "Chased bar 1 without initial SL",
    );
    sess.flatten_position(&bars, 2).unwrap();

    let mut journal = TradeJournalStore::new();
    journal.sync_paper_closed_trades("FPT", ExchangeVenue::Hose, sess.closed_trades());
    // Trader curates Trade #1 in Journal: explicitly clears mistake to `MistakeTag::None`!
    journal.entries[0].setup = SetupTag::TrendPullback;
    journal.entries[0].mistake = MistakeTag::None;
    journal.entries[0].notes = "Valid pullback stop — no execution mistake".to_owned();
    journal.entries[1].setup = SetupTag::MomentumContinuation;
    journal.entries[1].mistake = MistakeTag::FomoChase;

    let debrief = SessionDebriefReport::build(
        "FPT",
        Timeframe::D1,
        &sess,
        bars[2].close,
        &journal.entries,
        &log,
        false,
        usize::MAX,
        None,
    );
    // `FomoChase` must have `trade_count == 1` and `adverse_cost_abs == 0.0` (Trade #1's -241.11 loss is NOT FomoChase!),
    // and `EarlyExit` must have `trade_count == 0` because curated `JournalEntry[0]` cleared `mistake` to `MistakeTag::None`!
    let fomo = debrief
        .mistake_breakdown
        .iter()
        .find(|m| m.mistake == MistakeTag::FomoChase)
        .expect("FomoChase row");
    assert_eq!(fomo.trade_count, 1);
    assert!(
        fomo.adverse_cost_abs.abs() < 1e-9,
        "Trade #1 loss must never leak into Trade #2 FomoChase"
    );
    let early = debrief
        .mistake_breakdown
        .iter()
        .find(|m| m.mistake == MistakeTag::EarlyExit)
        .expect("EarlyExit note count row");
    assert_eq!(
        early.trade_count, 0,
        "curated JournalEntry MistakeTag::None must override tentative pre-trade note"
    );
    assert_eq!(early.decision_note_count, 1);

    // 2. `[Major-02]`: `pre_roll_bars` is covered by `SplitMix64` digest and rejects tampering (`120 -> 15`)!
    let hit = HistoricalSignalHit {
        instrument_id: "Preset:VN30 Basket:FPT".to_owned(),
        symbol: "FPT".to_owned(),
        timeframe: Timeframe::D1,
        bar_index: 2,
        timestamp: 3_000,
        price: 103.0,
        signal: ScanSignalKind::MacdBullCross,
        direction: SignalDirection::Bullish,
        regime: MarketRegime::Uptrend,
        metric_value: 2.5,
    };
    let mut store = PracticeSessionStore::new();
    store
        .save_session(SavedPracticeSession {
            slot_id: 0,
            label: "Blind +120b".to_owned(),
            pinned: false,
            sequence_no: 0,
            data_source: DataSourcePreset::Vn30Basket,
            symbol: "FPT".to_owned(),
            base_timeframe: Timeframe::D1,
            active_timeframe: Timeframe::D1,
            expected_bar_count: bars.len(),
            expected_first_timestamp: Some(1_000),
            expected_last_timestamp: Some(3_000),
            replay_active: true,
            replay_start_base_index: 0,
            replay_cursor_base_index: 1,
            blind_lookback_bars: 120,
            scanner_context: Some(ScannerSignalContext::with_reveal_and_preroll(
                ScannerReplayIntent::BlindPractice,
                hit,
                3_000,
                Some(120),
            )),
            account_config: zero_friction_config(),
            session_start_base_index: 0,
            commands: Vec::new(),
            decision_notes: Vec::new(),
            session_digest: 0,
        })
        .unwrap();
    let j_doc = store.to_json_pretty();
    let tampered_preroll = j_doc.replacen("\"pre_roll_bars\": 120", "\"pre_roll_bars\": 15", 1);
    assert_ne!(tampered_preroll, j_doc);
    assert!(
        PracticeSessionStore::from_json_str(&tampered_preroll).is_err(),
        "tampering pre_roll_bars must fail SplitMix64 digest verification"
    );
}

#[test]
fn wave1_regression_vn30f_multiplier_margin_sell_tax_and_dom_sizing() {
    use super::Position;
    use crate::journal::{ExchangeVenue, VnPriceUnit};
    use crate::trading::{
        DomLadderConfig, LadderStageInput, compute_causal_dom_ladder,
        size_ladder_qty_from_risk_budget, stage_ladder_order,
    };

    // 1. BUG-001: VN30F1M 100x kVND/pt contract multiplier + 17% initial margin:
    let bars = vec![
        candle(1_750_000_000, 1297.0, 1298.0, 1296.0, 1297.18),
        candle(1_750_000_900, 1297.0, 1297.5, 1292.0, 1292.99),
    ];
    let mut vn30f = TradingSession::new(AccountConfig {
        initial_cash: 100_000.0,
        commission_bps: 4.0,
        slippage_bps: 0.0,
        tax_sell_bps: 0.0,
        contract_multiplier: 100.0,
        initial_margin_pct: 17.0,
        venue: ExchangeVenue::Vn30Futures,
        allow_short: true,
        ..AccountConfig::default()
    });
    // 5 contracts (~110.3k kVND margin) exceeds 100k kVND -> rejected!
    assert!(matches!(
        vn30f.submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 5.0,
                stop_loss: None,
                take_profit: None,
            },
            &bars,
            0,
        ),
        Err(TradingError::InsufficientBuyingPower { .. })
    ));
    vn30f
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 1.0,
                stop_loss: None,
                take_profit: None,
            },
            &bars,
            0,
        )
        .unwrap();
    let unreal = vn30f.open_position().unwrap().unrealized_pnl(1292.99);
    assert!(
        (unreal - (-419.0)).abs() < 1e-6,
        "VN30F1M 1 contract -4.19 pts must have -419.0 kVND unrealized PnL, got {unreal}"
    );
    vn30f.flatten_position(&bars, 1).unwrap();
    let tr = &vn30f.closed_trades()[0];
    // Gross = -419.0 kVND; fees = (1297.18 + 1292.99) * 100 * 0.0004 = 103.6068 kVND; Net = -522.6068 kVND
    assert!((tr.gross_pnl - (-419.0)).abs() < 1e-6);
    assert!((tr.total_fees - ((1297.18 + 1292.99) * 100.0 * 4e-4)).abs() < 1e-6);
    assert!((tr.net_pnl - (tr.gross_pnl - tr.total_fees)).abs() < 1e-6);

    // 2. BUG-001: HOSE cash equity deducts 0.10% (10 bps) sell transfer tax on exit:
    let eq_bars = vec![
        candle(1_750_000_000, 50.0, 51.0, 49.0, 50.0),
        candle(1_750_086_400, 50.0, 52.0, 49.5, 50.0),
    ];
    let mut hose = TradingSession::new(AccountConfig {
        initial_cash: 100_000.0,
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        slippage_bps: 0.0,
        venue: ExchangeVenue::Hose,
        enforce_vn_t2_lot_lock: false,
        ..AccountConfig::default()
    });
    hose.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 100.0,
            stop_loss: None,
            take_profit: None,
        },
        &eq_bars,
        0,
    )
    .unwrap();
    hose.flatten_position(&eq_bars, 1).unwrap();
    let eq_tr = &hose.closed_trades()[0];
    // Buy comm = 5000 * 0.0015 = 7.5; Sell comm + tax = 5000 * (0.0015 + 0.0010) = 12.5 -> total = 20.0
    assert!(
        (eq_tr.total_fees - 20.0).abs() < 1e-9,
        "HOSE round-trip must charge 15bps buy + 15bps sell + 10bps sell tax = 20.0, got {}",
        eq_tr.total_fees
    );

    // 3. BUG-002: DOM risk sizing on VN30F1M never double-leverages (max 4 contracts on 100k kVND @ 1300):
    let sized = size_ladder_qty_from_risk_budget(
        100_000.0,
        5.0,
        1_300.0,
        1,
        OrderSide::Buy,
        ExchangeVenue::Vn30Futures,
        VnPriceUnit::ThousandVnd,
        Some(1_300.0),
    )
    .unwrap();
    assert!(
        (1.0..=4.0).contains(&sized),
        "VN30F1M statutory VSD 17% margin caps 100k kVND @ 1300 at 4 contracts, got {sized}"
    );

    // 4. BUG-024 + BUG-034: HOSE 10.00 tier boundary uniqueness & pure-reduce sell bracket suppression:
    let t0 = 1_750_000_000_i64;
    let tier_bars = vec![
        candle(t0, 9.30, 9.40, 9.25, 9.34),
        candle(t0 + 86_400, 9.40, 9.99, 9.35, 9.99),
    ];
    let snap = compute_causal_dom_ladder(
        &tier_bars,
        &DomLadderConfig {
            venue: ExchangeVenue::Hose,
            vn_unit: VnPriceUnit::ThousandVnd,
            half_rows: 5,
            lookback_bars: 5,
            center_price_override: None,
            reference_price_override: None,
        },
        &[],
        None,
        None,
    )
    .unwrap();
    let ceil_rows: Vec<f64> = snap
        .rows
        .iter()
        .filter(|r| r.band_state == crate::trading::DomPriceBandState::Ceiling)
        .map(|r| r.price)
        .collect();
    let mark_rows: Vec<f64> = snap
        .rows
        .iter()
        .filter(|r| r.is_at_mark)
        .map(|r| r.price)
        .collect();
    assert_eq!(
        ceil_rows.len(),
        1,
        "expected unique CEIL row, got {ceil_rows:?}"
    );
    assert_eq!(
        mark_rows.len(),
        1,
        "expected unique Mark row, got {mark_rows:?}"
    );
    let buy_above = stage_ladder_order(&LadderStageInput {
        side: OrderSide::Buy,
        clicked_price: 10.00,
        mark_price: 9.99,
        quantity: 100.0,
        venue: ExchangeVenue::Hose,
        resolved_unit: crate::journal::ResolvedVnPriceUnit::ThousandVnd,
        daily_band: None,
        stop_loss_ticks: Some(2),
        take_profit_ticks: Some(3),
        open_position: None,
        allow_account_short: false,
    })
    .unwrap();
    assert_eq!(
        buy_above.kind,
        OrderKind::Stop { stop_price: 10.00 },
        "Buy @ 10.00 above Mark 9.99 across HOSE 10.00 tier boundary must be Stop"
    );
    // Pure-reduce sell on HOSE must strip short-side SL/TP brackets (`BUG-034`):
    let reduce_sell = stage_ladder_order(&LadderStageInput {
        side: OrderSide::Sell,
        clicked_price: 51.0,
        mark_price: 50.0,
        quantity: 100.0,
        venue: ExchangeVenue::Hose,
        resolved_unit: crate::journal::ResolvedVnPriceUnit::ThousandVnd,
        daily_band: None,
        stop_loss_ticks: Some(2),
        take_profit_ticks: Some(3),
        open_position: Some(Position {
            side: PositionSide::Long,
            quantity: 200.0,
            avg_entry_price: 49.0,
            contract_multiplier: 1.0,
            opened_timestamp: t0,
            opened_base_index: 0,
            stop_loss: None,
            take_profit: None,
            initial_stop_loss: None,
            initial_risk_amount: None,
            entry_stop_loss: None,
            had_initial_sl_at_open: false,
            total_entry_qty: 200.0,
            total_entry_notional: 9_800.0,
            total_entry_fees: 0.0,
            open_entry_fees: 0.0,
            closed_qty_acc: 0.0,
            exit_notional_acc: 0.0,
            realized_gross_pnl_acc: 0.0,
            realized_exit_fees_acc: 0.0,
        }),
        allow_account_short: false,
    })
    .unwrap();
    assert_eq!(reduce_sell.stop_loss, None);
    assert_eq!(reduce_sell.take_profit, None);
}

#[test]
fn wave1_regression_core_t2_lock_no_hose_short_def005_gap_scalein_and_coaching() {
    use crate::data::days_from_civil;
    use crate::journal::{
        ExchangeVenue, TradeJournalStore, compute_settlement_timestamp_for_venue,
    };
    use crate::trading::{
        PracticeDecisionLog, PracticeSessionStore, SavedPracticeSession, SessionDebriefReport,
    };

    // 1. BUG-004 + BUG-008 + BUG-017: Direct TradingSession T+2 lock, partial Flatten, HOSE no-short, non-midnight 1D, W1, US T+1:
    let mon = days_from_civil(2026, 10, 5) * 86_400; // Mon 00:00 UTC
    let tue = days_from_civil(2026, 10, 6) * 86_400; // Tue 00:00 UTC
    let wed = days_from_civil(2026, 10, 7) * 86_400; // Wed 00:00 UTC
    let thu = days_from_civil(2026, 10, 8) * 86_400; // Thu 00:00 UTC
    // Non-midnight VN 1D close timestamps (08:00 UTC = 15:00 ICT):
    let vn_bars = vec![
        candle(mon + 8 * 3_600, 50.0, 51.0, 49.0, 50.0),
        candle(tue + 8 * 3_600, 50.0, 51.0, 49.0, 50.5),
        candle(wed + 8 * 3_600, 50.5, 52.0, 50.0, 51.0),
        candle(thu + 8 * 3_600, 51.0, 52.0, 50.0, 51.5),
    ];
    let mut sess = TradingSession::new(AccountConfig {
        initial_cash: 100_000.0,
        commission_bps: 0.0,
        slippage_bps: 0.0,
        venue: ExchangeVenue::Hose,
        enforce_vn_t2_lot_lock: true,
        allow_short: true, // venue Hose overrides to disallow short (`BUG-008`)!
        base_timeframe_secs: 86_400,
        ..AccountConfig::default()
    });
    // Flat HOSE Short MUST fail even with allow_short=true (`BUG-008`):
    assert!(matches!(
        sess.submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 100.0,
                stop_loss: None,
                take_profit: None,
            },
            &vn_bars,
            0,
        ),
        Err(TradingError::ShortSellingDisabled)
    ));
    // Buy 200 on Mon (08:00 UTC):
    sess.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 200.0,
            stop_loss: None,
            take_profit: None,
        },
        &vn_bars,
        0,
    )
    .unwrap();
    // On Tue (T+1, 08:00 UTC): MUST STILL BE LOCKED (`BUG-004` + `BUG-017` non-midnight 1D)!
    assert!(matches!(
        sess.submit_order(
            OrderRequest {
                side: OrderSide::Sell,
                kind: OrderKind::Market,
                quantity: 100.0,
                stop_loss: None,
                take_profit: None,
            },
            &vn_bars,
            1,
        ),
        Err(TradingError::InsufficientSettledShares { .. })
    ));
    assert!(matches!(
        sess.flatten_position(&vn_bars, 1),
        Err(TradingError::InsufficientSettledShares { .. })
    ));
    // Buy 300 more on Wed (now Mon 200 settled, Wed 300 locked):
    sess.submit_order(
        OrderRequest {
            side: OrderSide::Buy,
            kind: OrderKind::Market,
            quantity: 300.0,
            stop_loss: None,
            take_profit: None,
        },
        &vn_bars,
        2,
    )
    .unwrap();
    // Partial Flatten on Wed sells ONLY 200 settled and keeps 300 locked (`BUG-004`):
    let flat_fill = sess.flatten_position(&vn_bars, 2).unwrap();
    assert!((flat_fill.quantity - 200.0).abs() < 1e-9);
    assert!((sess.open_position().unwrap().quantity - 300.0).abs() < 1e-9);

    // W1 base bar on T+0 stays locked (`BUG-017`):
    let w1_bars = vec![candle(mon, 50.0, 52.0, 49.0, 51.0)];
    let mut w1_sess = TradingSession::new(AccountConfig {
        venue: ExchangeVenue::Hose,
        enforce_vn_t2_lot_lock: true,
        base_timeframe_secs: 604_800,
        ..AccountConfig::default()
    });
    w1_sess
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 100.0,
                stop_loss: None,
                take_profit: None,
            },
            &w1_bars,
            0,
        )
        .unwrap();
    assert!(matches!(
        w1_sess.flatten_position(&w1_bars, 0),
        Err(TradingError::InsufficientSettledShares { .. })
    ));

    // US Equities `SPY` Mon 00:00 UTC settles T+1 Tue 00:00 UTC (NOT Wed `BUG-017`):
    assert_eq!(
        compute_settlement_timestamp_for_venue(mon, ExchangeVenue::UsEquities),
        tue
    );

    // 2. BUG-009: DEF-005 pessimistic SL-before-TP on Phase A gap-open Buy Stop fill:
    let gap_bars = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(2_000, 102.5, 106.0, 97.0, 101.0),
    ];
    let mut gap_sess = TradingSession::new(zero_friction_config());
    gap_sess
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Stop { stop_price: 102.0 },
                quantity: 10.0,
                stop_loss: Some(98.0),
                take_profit: Some(105.0),
            },
            &gap_bars,
            0,
        )
        .unwrap();
    gap_sess.advance_through_bars(&gap_bars, 1, 1);
    assert_eq!(gap_sess.closed_trades().len(), 1);
    assert_eq!(
        gap_sess.closed_trades()[0].exit_reason,
        FillReason::StopLossExit,
        "Phase A gap-open dual-touch must enforce pessimistic DEF-005 SL before TP"
    );
    assert!((gap_sess.closed_trades()[0].exit_price - 98.0).abs() < 1e-9);

    // 3. BUG-017: Scale-in margin uses live mark-to-market notional (blocks 1.5x leverage on 1.0x cash account):
    let rally_bars = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(2_000, 160.0, 162.0, 159.0, 160.0),
    ];
    let mut cash_sess = TradingSession::new(AccountConfig {
        initial_cash: 10_000.0,
        commission_bps: 0.0,
        slippage_bps: 0.0,
        max_leverage: 1.0,
        ..AccountConfig::default()
    });
    // Buy 90 @ 100 = 9,000 (equity at bar 1 @ 160 = 10,000 + 5,400 = 15,400 -> max total qty @ 160 = 96.25 -> max add = 6.25):
    cash_sess
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 90.0,
                stop_loss: None,
                take_profit: None,
            },
            &rally_bars,
            0,
        )
        .unwrap();
    assert!(
        matches!(
            cash_sess.submit_order(
                OrderRequest {
                    side: OrderSide::Buy,
                    kind: OrderKind::Market,
                    quantity: 20.0,
                    stop_loss: None,
                    take_profit: None,
                },
                &rally_bars,
                1,
            ),
            Err(TradingError::InsufficientBuyingPower { .. })
        ),
        "1.0x cash account must reject mark-to-market scale-in exceeding live equity"
    );

    // 4. BUG-030: Profit-locking SL > entry on naked Long does NOT fabricate R or checklist.stop_and_lot_sized,
    //    and SessionDebriefReport::build filters headline KPIs consistently by cutoff_timestamp:
    let mut naked_sess = TradingSession::new(zero_friction_config());
    let n_bars = vec![
        candle(1_000, 100.0, 101.0, 99.0, 100.0),
        candle(2_000, 110.0, 112.0, 109.0, 110.0),
        candle(3_000, 112.0, 113.0, 111.0, 112.0),
    ];
    naked_sess
        .submit_order(
            OrderRequest {
                side: OrderSide::Buy,
                kind: OrderKind::Market,
                quantity: 10.0,
                stop_loss: None,
                take_profit: None,
            },
            &n_bars,
            0,
        )
        .unwrap();
    naked_sess
        .update_position_brackets(Some(105.0), None, &n_bars, 1)
        .unwrap();
    naked_sess.flatten_position(&n_bars, 2).unwrap();
    assert_eq!(naked_sess.closed_trades()[0].r_multiple, None);
    let mut jrn = TradeJournalStore::new();
    jrn.sync_paper_closed_trades("SPY", ExchangeVenue::UsEquities, naked_sess.closed_trades());
    assert!(!jrn.entries[0].checklist.stop_and_lot_sized);

    // Cutoff before exit_timestamp=3_000 excludes future closed trade from headline KPIs (`BUG-030`):
    let debrief_past = SessionDebriefReport::build(
        "SPY",
        Timeframe::D1,
        &naked_sess,
        110.0,
        &jrn.entries,
        &PracticeDecisionLog::new(),
        true,
        1,
        Some(2_000),
    );
    assert_eq!(debrief_past.closed_trades, 0);

    // 5. BUG-034: >80B label deduplication in PracticeSessionStore:
    let mut pstore = PracticeSessionStore::new();
    let long_label = "A".repeat(100);
    let make_slot = || SavedPracticeSession {
        slot_id: 0,
        label: long_label.clone(),
        pinned: false,
        sequence_no: 0,
        data_source: crate::data::DataSourcePreset::Vn30Basket,
        symbol: "FPT".to_owned(),
        base_timeframe: Timeframe::D1,
        active_timeframe: Timeframe::D1,
        expected_bar_count: 3,
        expected_first_timestamp: Some(1_000),
        expected_last_timestamp: Some(3_000),
        replay_active: false,
        replay_start_base_index: 0,
        replay_cursor_base_index: 2,
        blind_lookback_bars: 120,
        scanner_context: None,
        account_config: zero_friction_config(),
        session_start_base_index: 0,
        commands: Vec::new(),
        decision_notes: Vec::new(),
        session_digest: 0,
    };
    let id1 = pstore.save_session(make_slot()).unwrap();
    let id2 = pstore.save_session(make_slot()).unwrap();
    assert_eq!(id1, id2);
    assert_eq!(pstore.slots.len(), 1);
}

trait TapLabelExt {
    fn tap_label(self, lbl: &str) -> Self;
}
impl TapLabelExt for crate::trading::SavedPracticeSession {
    fn tap_label(mut self, lbl: &str) -> Self {
        self.label = lbl.to_owned();
        self
    }
}
