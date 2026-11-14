use crate::data::{
    DataSourcePreset, Timeframe, format_utc_date, generate_synthetic_ohlcv,
    load_bundled_watchlist_universe, load_preset_dataset,
};
use crate::model::Candle;

use super::*;

fn make_candle(ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    mk_candle(ts, open, high, low, close, 1_000.0)
}

fn mk_candle(ts: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle {
        timestamp: ts,
        date_label: format_utc_date(ts),
        open,
        high,
        low,
        close,
        volume,
    }
}

#[test]
fn next_bar_open_causality_and_final_bar_cutoff_guard() {
    // Construct a deterministic Donchian(3/2) breakout:
    // Bars 0..=2: flat range [98, 102], close = 100
    // Bar 3: breakout close = 105 (> prior 3-bar max 102) -> queues Entry(Long) for Open[4] = 112.0!
    // Must NEVER fill at Close[3] = 105.0!
    // Bar 4: opens at 112.0, closes at 114.0
    // Bar 5: drops below 2-bar low (108 < min(110, 99) = 99? Let's make Bar 5 close = 90.0 < 100.0)
    // -> Bar 5 is the FINAL bar, so the Exit signal at Close[5] must NOT fill on Bar 5; recorded as EndOfSeriesCutoff!
    let base_ts = 1_700_000_000_i64;
    let candles = vec![
        make_candle(base_ts, 100.0, 102.0, 98.0, 100.0),
        make_candle(base_ts + 86_400, 100.0, 102.0, 99.0, 101.0),
        make_candle(base_ts + 2 * 86_400, 101.0, 102.0, 99.0, 100.0),
        make_candle(base_ts + 3 * 86_400, 100.0, 106.0, 99.5, 105.0), // Signal at Close[3] = 105!
        make_candle(base_ts + 4 * 86_400, 112.0, 116.0, 110.0, 114.0), // Fill at Open[4] = 112!
        make_candle(base_ts + 5 * 86_400, 113.0, 114.0, 89.0, 90.0), // Final bar exit signal -> Cutoff!
    ];

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::DonchianAtrBreakout,
        params: StrategyParams {
            donchian_entry_period: 3,
            donchian_exit_period: 2,
            atr_period: 2,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongOnly,
        sizing: PositionSizingMode::FixedNotional(10_000.0),
        risk: StopTakeConfig {
            stop_loss_pct: None,
            take_profit_pct: None,
            ..StopTakeConfig::default()
        },
        initial_cash: 100_000.0,
        commission_bps: 10.0,
        tax_sell_bps: 0.0,
        slippage_bps: 0.0,
        max_leverage: 1.0,
        lot_step: 0.0,
        in_sample_split_pct: None,
    };

    let res = run_backtest("TEST", Timeframe::D1, &candles, &cfg, None);
    assert_eq!(res.equity_curve.len(), candles.len());
    // At bar 3 (signal bar), position must STILL be flat (`position_qty == 0.0`, `equity == 100_000.0`).
    assert_eq!(res.equity_curve[3].position_qty, 0.0);
    assert!((res.equity_curve[3].equity - 100_000.0).abs() < 1e-9);

    // Position opened at Open[4] = 112.0 (NEVER Close[3] = 105.0!) and remains open at end of bar 5!
    assert_eq!(res.trades.len(), 0);
    let open_pos = res.open_position.expect("position opened at Open[4]");
    assert_eq!(open_pos.entry_bar_index, 4);
    assert!((open_pos.entry_price - 112.0).abs() < 1e-9);

    // Final-bar exit signal at bar 5 is recorded as `EndOfSeriesCutoff` (`TEST-BT-001`).
    assert_eq!(res.unexecuted_signals.len(), 1);
    assert_eq!(res.unexecuted_signals[0].signal_bar_index, 5);
    assert!(res.unexecuted_signals[0].is_exit);
    assert_eq!(
        res.unexecuted_signals[0].reason,
        UnfilledReason::EndOfSeriesCutoff
    );

    // When `incomplete_tail_bar: true` (forming HTF bucket in Replay), bar 5 does not emit a premature close signal.
    let res_incomplete = run_backtest_with_options(
        "TEST",
        Timeframe::D1,
        &candles,
        &cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: None,
            incomplete_tail_bar: true,
        },
    );
    assert!(res_incomplete.unexecuted_signals.is_empty());
}

#[test]
fn pessimistic_sl_before_tp_on_dual_touch_and_post_close_trailing_stop_anti_lookahead() {
    let base_ts = 1_700_000_000_i64;
    // Donchian(2/2) breakout at bar 2 -> fills Long at Open[3] = 100.0.
    // Bar 3 is a wide dual-touch candle: Open=100, High=115 (>= TP 110), Low=93 (<= SL 95), Close=96!
    // Pessimistic rule MUST exit at StopLoss 95.0, NEVER TakeProfit 110.0!
    let candles = vec![
        make_candle(base_ts, 99.0, 101.0, 98.0, 100.0),
        make_candle(base_ts + 86_400, 100.0, 101.0, 98.0, 100.0),
        make_candle(base_ts + 2 * 86_400, 100.0, 104.0, 99.0, 103.0), // Breakout!
        make_candle(base_ts + 3 * 86_400, 100.0, 115.0, 93.0, 96.0),  // Dual-touch SL & TP!
        make_candle(base_ts + 4 * 86_400, 96.0, 98.0, 95.0, 97.0),
    ];

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::DonchianAtrBreakout,
        params: StrategyParams {
            donchian_entry_period: 2,
            donchian_exit_period: 2,
            atr_period: 2,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongOnly,
        sizing: PositionSizingMode::FixedNotional(10_000.0),
        risk: StopTakeConfig {
            stop_loss_pct: Some(5.0),    // SL = 95.0
            take_profit_pct: Some(10.0), // TP = 110.0
            ..StopTakeConfig::default()
        },
        initial_cash: 100_000.0,
        commission_bps: 0.0,
        tax_sell_bps: 0.0,
        slippage_bps: 0.0,
        max_leverage: 1.0,
        lot_step: 0.0,
        in_sample_split_pct: None,
    };

    let res = run_backtest("DUAL", Timeframe::D1, &candles, &cfg, None);
    assert_eq!(res.trades.len(), 1);
    assert_eq!(res.trades[0].exit_reason, BacktestExitReason::StopLoss);
    assert!((res.trades[0].exit_price - 95.0).abs() < 1e-9);
    assert!((res.trades[0].r_multiple.expect("1R stop") - (-1.0)).abs() < 1e-9);

    // Now test ATR Trailing Stop anti-lookahead on bar 3:
    // Suppose NO fixed SL/TP, only `trailing_stop_atr_mult = Some(2.0)`.
    // Prior ATR at bar 2 is ~3.5 -> initial trail at Open[3]=100 is `100 - 2*atr ≈ 93.0`.
    // If Bar 3 has High = 120.0 and Low = 96.0 (Close = 98.0):
    // A buggy lookahead engine that ratcheted trail to `120 - 2*atr = 113` BEFORE checking Low=96
    // would falsely trigger TrailingStop at 113 on Bar 3!
    // Our causal engine checks Low=96 against initial trail ~93 (survives!), and ratchets trail to ~113 AFTER Close[3]!
    let trail_candles = vec![
        make_candle(base_ts, 99.0, 101.0, 98.0, 100.0),
        make_candle(base_ts + 86_400, 100.0, 102.0, 98.0, 100.0),
        make_candle(base_ts + 2 * 86_400, 100.0, 104.0, 99.0, 103.0), // Breakout!
        make_candle(base_ts + 3 * 86_400, 100.0, 120.0, 96.0, 98.0),  // Survives initial trail!
        make_candle(base_ts + 4 * 86_400, 98.0, 99.0, 95.0, 96.0), // Gaps below ratcheted trail -> exits at Open[4]=98!
    ];
    let trail_cfg = BacktestConfig {
        risk: StopTakeConfig {
            stop_loss_pct: None,
            take_profit_pct: None,
            trailing_stop_atr_mult: Some(1.0),
            ..StopTakeConfig::default()
        },
        ..cfg
    };
    let trail_res = run_backtest("TRAIL", Timeframe::D1, &trail_candles, &trail_cfg, None);
    assert_eq!(trail_res.trades.len(), 1);
    assert_eq!(trail_res.trades[0].entry_bar_index, 3);
    // Must NOT exit on bar 3 at phantom 113; exits causally on bar 4 at Open[4] = 98.0!
    assert_eq!(trail_res.trades[0].exit_bar_index, 4);
    assert_eq!(
        trail_res.trades[0].exit_reason,
        BacktestExitReason::TrailingStop
    );
    assert!((trail_res.trades[0].exit_price - 98.0).abs() < 1e-9);
}

#[test]
fn replay_cutoff_zero_lookahead_and_cash_fee_tax_conservation_across_all_presets() {
    let fpt = load_preset_dataset(DataSourcePreset::Vn30Basket, 500, Some("FPT"));
    let full_candles = &fpt.candles;
    assert!(full_candles.len() >= 150);

    let cutoff_idx = 110;
    let cutoff_ts = full_candles[cutoff_idx].timestamp;

    // Create a poisoned copy where bars after `cutoff_idx` skyrocket 100x then crash 99%!
    let mut poisoned = full_candles.clone();
    for c in &mut poisoned[(cutoff_idx + 1)..] {
        c.open *= 50.0;
        c.high *= 80.0;
        c.low *= 0.05;
        c.close *= 40.0;
    }

    for preset in StrategyPresetKind::ALL {
        for dir in StrategyDirectionMode::ALL {
            let cfg = BacktestConfig {
                preset,
                direction_mode: dir,
                commission_bps: 15.0,
                tax_sell_bps: 10.0,
                slippage_bps: 5.0,
                sizing: PositionSizingMode::RiskPercentPerTrade(1.5),
                in_sample_split_pct: Some(0.70),
                ..BacktestConfig::default()
            };

            let truncated_res = run_backtest(
                "FPT",
                Timeframe::D1,
                &full_candles[..=cutoff_idx],
                &cfg,
                None,
            );
            let cutoff_res = run_backtest("FPT", Timeframe::D1, &poisoned, &cfg, Some(cutoff_ts));

            assert_eq!(truncated_res.trades, cutoff_res.trades);
            assert_eq!(truncated_res.equity_curve, cutoff_res.equity_curve);
            assert_eq!(truncated_res.tearsheet, cutoff_res.tearsheet);
            assert_eq!(
                truncated_res.in_sample_tearsheet,
                cutoff_res.in_sample_tearsheet
            );
            assert_eq!(
                truncated_res.out_of_sample_tearsheet,
                cutoff_res.out_of_sample_tearsheet
            );

            // Verify exact cash/fee/tax conservation!
            let sum_net: f64 = cutoff_res.trades.iter().map(|t| t.net_pnl).sum();
            let open_fee = cutoff_res
                .open_position
                .as_ref()
                .map_or(0.0, |p| p.entry_fee);
            let expected_cash = cfg.initial_cash + sum_net - open_fee;
            assert!(
                (cutoff_res.tearsheet.final_cash - expected_cash).abs() < 1e-6,
                "cash mismatch for {preset:?}/{dir:?}: got {} vs expected {expected_cash}",
                cutoff_res.tearsheet.final_cash
            );
        }
    }
}

#[test]
fn min_holding_bars_t_plus_2_lockup_and_lot_step_rounding() {
    let candles = generate_synthetic_ohlcv(500);
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 8,
            slow_ema: 21,
            ..StrategyParams::default()
        },
        risk: StopTakeConfig {
            stop_loss_pct: Some(0.5),
            take_profit_pct: Some(0.8),
            min_holding_bars: 2, // T+2 settlement lockup!
            max_holding_bars: Some(4),
            ..StopTakeConfig::default()
        },
        initial_cash: 100_000_000.0,
        lot_step: 100.0, // HOSE 100-share board lot!
        ..BacktestConfig::default()
    };

    let res = run_backtest("VNINDEX", Timeframe::D1, &candles, &cfg, None);
    assert!(!res.trades.is_empty());
    for t in &res.trades {
        assert!(
            t.bars_held >= 2,
            "T+2 lockup violated: trade #{} held {} bars",
            t.trade_id,
            t.bars_held
        );
        let rem = t.quantity % 100.0;
        assert!(
            rem.abs() < 1e-9 || (100.0 - rem).abs() < 1e-9,
            "100-share board lot violated: qty = {}",
            t.quantity
        );
    }
}

#[test]
fn parameter_sweep_multi_symbol_battle_and_100k_bar_throughput() {
    let universe = load_bundled_watchlist_universe();
    assert_eq!(universe.len(), 13);

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        direction_mode: StrategyDirectionMode::LongShort,
        ..BacktestConfig::default()
    };

    let battle = run_multi_symbol_battle(
        &universe,
        Timeframe::D1,
        &cfg,
        SweepRankingMetric::TotalReturnPct,
        None,
    );
    assert_eq!(battle.rows.len(), 13);
    for pair in battle.rows.windows(2) {
        assert!(
            pair[0].tearsheet.total_return_pct >= pair[1].tearsheet.total_return_pct,
            "battle rows must be sorted descending by Return%"
        );
    }

    let btc = load_preset_dataset(DataSourcePreset::BtcUsdt15mJson, 500, None);
    let sweep = run_parameter_sweep(
        "BTCUSDT",
        Timeframe::M15,
        &btc.candles,
        &cfg,
        SweepRankingMetric::SharpeRatio,
        BacktestRunOptions::default(),
    );
    assert_eq!(sweep.variants.len(), 12);
    assert_eq!(sweep.variants[0].rank, 1);

    // Verify 100,000-bar deterministic execution produces 100,000 aligned equity points cleanly!
    let big = generate_synthetic_ohlcv(100_000);
    let big_res = run_backtest("SYNTH100K", Timeframe::M1, &big, &cfg, None);
    assert_eq!(big_res.bar_count, 100_000);
    assert_eq!(big_res.equity_curve.len(), 100_000);
    assert!(big_res.equity_idx.is_synced_with(&big_res.equity_values));
    assert!(big_res.tearsheet.final_equity.is_finite());
}

#[test]
fn tearsheet_sortino_cagr_breakeven_and_zero_loss_profit_factor_invariants() {
    use crate::trading::PositionSide;

    // 4 daily bars (`3` days < `7` days -> `cagr_pct` must be `None`!).
    // Returns:
    // Bar 0 -> 1: `100_000 -> 102_000` (`+0.02`, downside = `0.0`)
    // 3 daily bars (`< 7` days -> `cagr_pct == None`, `< 30` days -> nominal `252.0` D1 bars/yr):
    // `initial_cash = 100_000`:
    // Bar 0: `100_000 -> 102_000`  (`+0.02`, downside = `0.0`)
    // Bar 1: `102_000 -> 99_960`   (`-0.02`, downside = `-0.02`)
    // Bar 2: `99_960  -> 101_959.2` (`+0.02`, downside = `0.0`)
    // Total return observations `N = 3`: mean = `+0.02 / 3`.
    // Sortino & Tricks (1994) TDD = `sqrt((0^2 + (-0.02)^2 + 0^2) / 3) = 0.02 / sqrt(3)`.
    // Ratio before sqrt(252) = `(0.02 / 3) / (0.02 / sqrt(3)) = 1 / sqrt(3)`.
    let t0 = 1_700_000_000_i64;
    let curve = vec![
        BacktestEquityPoint {
            bar_index: 0,
            timestamp: t0,
            equity: 102_000.0,
            cash: 102_000.0,
            drawdown_abs: 0.0,
            underwater_pct: 0.0,
            position_qty: 10.0,
        },
        BacktestEquityPoint {
            bar_index: 1,
            timestamp: t0 + 86_400,
            equity: 99_960.0,
            cash: 99_960.0,
            drawdown_abs: 2_040.0,
            underwater_pct: -2.0,
            position_qty: 0.0,
        },
        BacktestEquityPoint {
            bar_index: 2,
            timestamp: t0 + 2 * 86_400,
            equity: 101_959.2,
            cash: 101_959.2,
            drawdown_abs: 40.8,
            underwater_pct: -0.04,
            position_qty: 10.0,
        },
    ];

    let win_trade = BacktestTrade {
        trade_id: 1,
        side: PositionSide::Long,
        quantity: 10.0,
        entry_bar_index: 1,
        exit_bar_index: 3,
        entry_timestamp: t0 + 86_400,
        exit_timestamp: t0 + 3 * 86_400,
        entry_price: 100.0,
        exit_price: 195.92,
        bars_held: 2,
        gross_pnl: 1_959.2,
        entry_fee: 0.0,
        exit_fee: 0.0,
        exit_tax: 0.0,
        net_pnl: 1_959.2,
        return_pct: 1.9592,
        r_multiple: Some(2.0),
        mae_pct: -0.5,
        mfe_pct: 2.5,
        exit_reason: BacktestExitReason::TakeProfit,
    };
    let breakeven_trade = BacktestTrade {
        trade_id: 2,
        net_pnl: 0.0,
        gross_pnl: 0.0,
        return_pct: 0.0,
        r_multiple: Some(0.0),
        ..win_trade.clone()
    };

    let sheet = compute_tearsheet(
        100_000.0,
        101_959.2,
        Timeframe::D1,
        &curve,
        &[win_trade, breakeven_trade],
        0.0,
    );
    assert_eq!(
        sheet.cagr_pct, None,
        "< 7 calendar days must suppress CAGR extrapolation"
    );
    assert_eq!(sheet.num_trades, 2);
    assert_eq!(sheet.num_winners, 1);
    assert_eq!(sheet.num_losers, 0);
    assert_eq!(sheet.num_breakeven, 1);
    assert_eq!(
        sheet.profit_factor, None,
        "0-loss strategy must return None for profit_factor rather than a magic 999.99 float"
    );

    let expected_sortino = (252.0_f64.sqrt()) / (3.0_f64.sqrt());
    let actual_sortino = sheet.sortino_ratio.expect("finite sortino");
    assert!(
        (actual_sortino - expected_sortino).abs() < 1e-6,
        "Sortino must divide by total observations N=3 (got {actual_sortino}, expected {expected_sortino})"
    );
}

#[test]
fn backtest_config_sanitize_clamps_nan_infinity_and_inverted_periods() {
    let mut cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 240,
            slow_ema: 10,
            rsi_period: 0,
            rsi_oversold: f64::NAN,
            rsi_overbought: f64::INFINITY,
            rsi_exit_midline: f64::NEG_INFINITY,
            rsi_confluence_buy: -20.0,
            rsi_confluence_sell: 200.0,
            macd_fast: 150,
            macd_slow: 5,
            macd_signal: 0,
            macd_trend_ema: 1,
            bb_period: 0,
            bb_std_mult: f64::NAN,
            max_squeeze_bw_pct: -5.0,
            donchian_entry_period: 0,
            donchian_exit_period: 9999,
            atr_period: 0,
        },
        direction_mode: StrategyDirectionMode::LongShort,
        sizing: PositionSizingMode::PercentEquity(f64::NAN),
        risk: StopTakeConfig {
            stop_loss_pct: Some(f64::NAN),
            stop_loss_atr_mult: Some(-3.0),
            take_profit_pct: Some(f64::INFINITY),
            take_profit_rr_mult: Some(0.0),
            trailing_stop_atr_mult: Some(999.0),
            max_holding_bars: Some(0),
            min_holding_bars: 500,
        },
        initial_cash: f64::NAN,
        commission_bps: f64::INFINITY,
        tax_sell_bps: -10.0,
        slippage_bps: f64::NAN,
        max_leverage: 0.0,
        lot_step: f64::NAN,
        in_sample_split_pct: Some(0.99),
    };
    cfg.sanitize();

    assert!(cfg.params.fast_sma < cfg.params.slow_ema);
    assert!(cfg.params.macd_fast < cfg.params.macd_slow);
    assert_eq!(cfg.params.macd_trend_ema, 2);
    assert!(cfg.params.rsi_oversold.is_finite());
    assert!(cfg.params.rsi_overbought.is_finite());
    assert!(cfg.params.rsi_exit_midline > cfg.params.rsi_oversold);
    assert!(cfg.params.rsi_exit_midline < cfg.params.rsi_overbought);
    assert_eq!(cfg.risk.stop_loss_pct, None);
    assert_eq!(cfg.risk.stop_loss_atr_mult, None);
    assert_eq!(cfg.risk.take_profit_pct, None);
    assert_eq!(cfg.risk.take_profit_rr_mult, None);
    assert_eq!(cfg.risk.trailing_stop_atr_mult, Some(20.0));
    assert_eq!(cfg.risk.min_holding_bars, 30);
    assert_eq!(cfg.risk.max_holding_bars, Some(30));
    assert_eq!(cfg.initial_cash, 100_000.0);
    assert_eq!(cfg.commission_bps, 10.0);
    assert_eq!(cfg.tax_sell_bps, 0.0);
    assert_eq!(cfg.slippage_bps, 5.0);
    assert_eq!(cfg.max_leverage, 0.1);
    assert_eq!(cfg.lot_step, 0.0);
    assert_eq!(cfg.in_sample_split_pct, Some(0.90));
}

#[test]
fn critic_r09_regression_cross_timeframe_cutoff_prevents_intraday_daily_lookahead() {
    use crate::data::WatchlistFeedEntry;

    // `2026-10-15 00:00:00 UTC` (`D1` open) vs intraday replay cursor `2026-10-15 09:03:00 UTC` (`M1` close = `09:03:59`).
    let day_2026_10_15 = 1_792_022_400_i64; // divisible by 86_400 (`00:00:00 UTC`)
    assert_eq!(day_2026_10_15.rem_euclid(86_400), 0);

    // Build 40 daily bars for `FPT` ending on `day_2026_10_15`, where the `day_2026_10_15` bar has a poisoned
    // afternoon `14:45` spike (`+5000%`) that has NOT happened yet at `09:03:59 UTC`!
    let mut fpt_d1 = Vec::new();
    for k in 0..40_i64 {
        let ts = day_2026_10_15 - (39 - k) * 86_400;
        let px = 100.0 + (k as f64) * 0.5;
        fpt_d1.push(mk_candle(ts, px, px + 1.0, px - 1.0, px + 0.2, 100_000.0));
    }
    // Poison the final `day_2026_10_15` bar with future afternoon explosion!
    if let Some(last) = fpt_d1.last_mut() {
        last.high = 9_500.0;
        last.close = 9_000.0;
    }

    // Build 60 `M15` bars for `BTCUSDT` ending with `09:00:00..09:15:00` on `day_2026_10_15`, poisoned at `09:14`!
    let slot_0900 = day_2026_10_15 + 9 * 3600;
    let mut btc_15m = Vec::new();
    for k in 0..60_i64 {
        let ts = slot_0900 - (59 - k) * 900;
        let px = 60_000.0 + (k as f64) * 10.0;
        btc_15m.push(mk_candle(ts, px, px + 30.0, px - 30.0, px + 5.0, 50.0));
    }
    if let Some(last) = btc_15m.last_mut() {
        last.high = 999_000.0;
        last.close = 950_000.0;
    }

    // Build `M1` bars for `VN30F1M` up to `09:05:00`, replaying at `09:03:00` (`close_cutoff = slot_0900 + 3*60 + 59`).
    let mut vn30f_1m = Vec::new();
    for k in -120..=5_i64 {
        let ts = slot_0900 + k * 60;
        let px = 1300.0 + (k as f64) * 0.1;
        vn30f_1m.push(mk_candle(ts, px, px + 0.5, px - 0.5, px + 0.05, 500.0));
    }

    let feeds = vec![
        WatchlistFeedEntry {
            instrument_id: "Preset:VN30:FPT".to_owned(),
            preset: DataSourcePreset::Vn30Basket,
            symbol: "FPT".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: fpt_d1.clone(),
        },
        WatchlistFeedEntry {
            instrument_id: "Preset:BTC:BTCUSDT".to_owned(),
            preset: DataSourcePreset::BtcUsdt15mJson,
            symbol: "BTCUSDT".to_owned(),
            base_timeframe: Timeframe::M15,
            candles: btc_15m.clone(),
        },
        WatchlistFeedEntry {
            instrument_id: "Preset:VN30F:VN30F1M".to_owned(),
            preset: DataSourcePreset::IntradayVn30F1m,
            symbol: "VN30F1M".to_owned(),
            base_timeframe: Timeframe::M1,
            candles: vn30f_1m,
        },
    ];

    let close_cutoff_0903 = slot_0900 + 3 * 60 + 59; // `09:03:59 UTC`
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::DonchianAtrBreakout,
        params: StrategyParams {
            donchian_entry_period: 10,
            donchian_exit_period: 5,
            ..StrategyParams::default()
        },
        risk: StopTakeConfig {
            min_holding_bars: 2,
            ..StopTakeConfig::default()
        },
        lot_step: 100.0,
        tax_sell_bps: 10.0,
        ..BacktestConfig::default()
    };

    let battle = run_multi_symbol_battle_with_close_cutoff(
        &feeds,
        Timeframe::M15,
        &cfg,
        SweepRankingMetric::TotalReturnPct,
        Some(close_cutoff_0903),
    );
    let fpt_row = battle
        .rows
        .iter()
        .find(|r| r.symbol == "FPT")
        .expect("FPT row");
    assert_eq!(
        fpt_row.bar_count, 39,
        "CRITICAL-01: unclosed 2026-10-15 D1 bar must be excluded at 09:03:59 UTC"
    );
    let btc_row = battle
        .rows
        .iter()
        .find(|r| r.symbol == "BTCUSDT")
        .expect("BTC row");
    assert_eq!(
        btc_row.bar_count, 59,
        "CRITICAL-01: unclosed 09:00..09:15 M15 bar must be excluded at 09:03:59 UTC"
    );

    // Direct `run_backtest_with_options` with intraday cutoff on coarse `D1` array must ALSO exclude `day_2026_10_15`!
    let direct_fpt = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        &fpt_d1,
        &cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: Some(slot_0900 + 3 * 60),
            incomplete_tail_bar: false,
        },
    );
    assert_eq!(direct_fpt.bar_count, 39);

    // Pure prefix `is_bucket_incomplete_at_close` check (`MAJOR-04`):
    assert!(is_bucket_incomplete_at_close(
        slot_0900,
        close_cutoff_0903,
        Timeframe::M1,
        Timeframe::M15
    ));
    assert!(!is_bucket_incomplete_at_close(
        slot_0900,
        slot_0900 + 14 * 60 + 59,
        Timeframe::M1,
        Timeframe::M15
    ));
}

#[test]
fn critic_r09_regression_robustness_never_rewards_losing_is_and_continuous_oos_conservation() {
    // 1. Counterexample 1 (`CRITICAL-02`): Heavy losing IS (`-60%`, Sharpe `-2.5`, 25 trades) + tiny positive OOS (`+0.15%`, 5 trades)
    let mut is_loser = BacktestTearsheet::empty(100_000.0);
    is_loser.final_equity = 40_000.0;
    is_loser.total_return_abs = -60_000.0;
    is_loser.total_return_pct = -60.0;
    is_loser.sharpe_ratio = Some(-2.5);
    is_loser.num_trades = 25;

    let mut oos_tiny_win = BacktestTearsheet::empty(40_000.0);
    oos_tiny_win.final_equity = 40_060.0;
    oos_tiny_win.total_return_abs = 60.0;
    oos_tiny_win.total_return_pct = 0.15;
    oos_tiny_win.sharpe_ratio = Some(0.04);
    oos_tiny_win.num_trades = 5;

    assert_eq!(
        evaluate_robustness(&is_loser, &oos_tiny_win),
        RobustnessVerdict::Unprofitable,
        "losing IS with net-negative campaign must NEVER be classified as Robust"
    );

    // 2. Counterexample 2: Missing `sharpe_ratio == None` must NOT invent `0.5` and claim `Robust`!
    let mut is_flat_sh = BacktestTearsheet::empty(100_000.0);
    is_flat_sh.final_equity = 110_000.0;
    is_flat_sh.total_return_abs = 10_000.0;
    is_flat_sh.total_return_pct = 10.0;
    is_flat_sh.sharpe_ratio = None;
    is_flat_sh.num_trades = 10;

    let mut oos_flat_sh = is_flat_sh.clone();
    assert_ne!(
        evaluate_robustness(&is_flat_sh, &oos_flat_sh),
        RobustnessVerdict::Robust,
        "missing Sharpe ratio must degrade to at most Stable, never Robust"
    );

    // 3. Counterexample 3: `IS = +120%` and `OOS = -1.9%` must be `Overfitted`!
    is_flat_sh.total_return_pct = 120.0;
    is_flat_sh.sharpe_ratio = Some(2.4);
    oos_flat_sh.total_return_pct = -1.9;
    oos_flat_sh.sharpe_ratio = Some(-0.4);
    assert_eq!(
        evaluate_robustness(&is_flat_sh, &oos_flat_sh),
        RobustnessVerdict::Overfitted
    );

    // 4. Continuous causal `IS / OOS` split conservation across `split_bar_index`!
    let candles = generate_synthetic_ohlcv(400);
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 6,
            slow_ema: 18,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongShort,
        in_sample_split_pct: Some(0.70),
        ..BacktestConfig::default()
    };
    let res = run_backtest("SYNTH", Timeframe::D1, &candles, &cfg, None);
    let is_s = res.in_sample_tearsheet.as_ref().expect("IS sheet");
    let oos_s = res.out_of_sample_tearsheet.as_ref().expect("OOS sheet");
    assert_eq!(
        is_s.num_trades + oos_s.num_trades,
        res.tearsheet.num_trades,
        "continuous IS/OOS partition must conserve 100% of closed trades including cross-split trades"
    );
    let compounded =
        (1.0 + is_s.total_return_pct * 0.01) * (1.0 + oos_s.total_return_pct * 0.01) - 1.0;
    let full_ret = res.tearsheet.total_return_pct * 0.01;
    assert!(
        (compounded - full_ret).abs() < 1e-9,
        "geometric compounding mismatch: (1+IS)*(1+OOS)-1 = {compounded} vs full = {full_ret}"
    );
}

#[test]
fn critic_r09_regression_margin_liquidation_prevents_zombie_account_resurrection() {
    // Un-stopped Short at 2x leverage: price rallies from 100 -> 220 (wipes out 100% of equity!),
    // then crashes down to 40. Account MUST liquidate permanently at 0.0 equity and NEVER resurrect!
    let mut candles = Vec::new();
    let t0 = 1_700_000_000_i64;
    // Bars 0..5: high plateau at 100, then bar 5 drops to 90 triggering fast SMA(2) < slow EMA(4) Short entry at Open[6] = 100!
    for (idx, px) in [100.0, 102.0, 104.0, 105.0, 105.0, 92.0]
        .into_iter()
        .enumerate()
    {
        candles.push(mk_candle(
            t0 + (idx as i64) * 86_400,
            px,
            px + 0.5,
            px - 0.5,
            px,
            1000.0,
        ));
    }
    // Bar 6: Short fills at Open[6] = 100.0 (`2.0x` leverage -> ~1900 units on $100k).
    candles.push(mk_candle(
        t0 + 6 * 86_400,
        100.0,
        101.0,
        99.0,
        100.0,
        1000.0,
    ));
    // Bar 7: Squeeze to `High = 220.0, Close = 215.0` -> exceeds bankruptcy price (~152.6)!
    candles.push(mk_candle(
        t0 + 7 * 86_400,
        105.0,
        220.0,
        104.0,
        215.0,
        1000.0,
    ));
    // Bars 8..15: Crash to `30.0` (would have made huge phantom profit if zombie Short stayed alive!).
    for idx in 8..15_i64 {
        candles.push(mk_candle(t0 + idx * 86_400, 40.0, 42.0, 30.0, 32.0, 1000.0));
    }

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 2,
            slow_ema: 4,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::ShortOnly,
        sizing: PositionSizingMode::PercentEquity(95.0),
        risk: StopTakeConfig {
            stop_loss_pct: None,
            take_profit_pct: None,
            trailing_stop_atr_mult: None,
            ..StopTakeConfig::default()
        },
        initial_cash: 100_000.0,
        commission_bps: 10.0,
        slippage_bps: 5.0,
        max_leverage: 2.0,
        in_sample_split_pct: None,
        ..BacktestConfig::default()
    };

    let res = run_backtest("ZOMBIE", Timeframe::D1, &candles, &cfg, None);
    assert_eq!(res.trades.len(), 1);
    assert_eq!(
        res.trades[0].exit_reason,
        BacktestExitReason::MarginLiquidation,
        "insolvent position must force-liquidate with MarginLiquidation"
    );
    assert_eq!(res.trades[0].exit_bar_index, 7);
    assert_eq!(
        res.tearsheet.final_equity, 0.0,
        "liquidated account must stay at 0.0 and NEVER resurrect on subsequent crash"
    );
    assert_eq!(res.tearsheet.final_cash, 0.0);
    let sum_net: f64 = res.trades.iter().map(|t| t.net_pnl).sum();
    assert!(
        (cfg.initial_cash + sum_net - res.tearsheet.final_cash).abs() < 1e-9,
        "zero-balance protection must still conserve initial_cash + sum(net_pnl) == final_cash"
    );
}

#[test]
fn critic_r09_regression_t_plus_2_trailing_ratchet_marubozu_slippage_and_mae_mfe_bounds() {
    let t0 = 1_700_000_000_i64;

    // Part A (`MAJOR-01` & `MAJOR-02`):
    // - Entry at bar 5 (`Open = 100, Low = 100, High = 112, Close = 112`): Pure bullish Marubozu!
    //   With `slippage_bps = 50` (`0.50%`) and `stop_loss_pct = 0.30%`, `sl_px = 100.5 * 0.997 = 100.1985 > Low(100)`.
    //   Must NOT phantom-stop on entry bar 5!
    // - `min_holding_bars = 2` (`VN T+2`) + `trailing_stop_atr_mult = Some(1.0)`:
    //   Bar 5 (`T+0`) rallies to `112`, Bar 6 (`T+1`) rallies to `125` (`highest_since_entry = 125`, ratcheting `trailing_stop` well above `110`!).
    //   Bar 7 (`T+2`) opens gap-down at `108` -> MUST trigger the ratcheted `TrailingStop` at `Open[7] = 108` (proving `T+0`/`T+1` ratcheted `trailing_stop`),
    //   and even if Bar 7 rallies to `High = 140` in the afternoon after stopping out at `Open[7] = 108`, `mfe_pct` MUST NOT include `140`!
    let candles = vec![
        mk_candle(t0, 100.0, 101.0, 99.0, 100.0, 1000.0),
        mk_candle(t0 + 86_400, 100.0, 101.0, 99.0, 99.5, 1000.0),
        mk_candle(t0 + 2 * 86_400, 99.5, 100.0, 98.5, 99.0, 1000.0),
        mk_candle(t0 + 3 * 86_400, 99.0, 99.5, 98.0, 98.5, 1000.0),
        // Bar 4: Bullish cross `SMA(2) > EMA(4)` at `Close = 104.0` (`101.25 > 101.15`) -> queues Long Entry for `Open[5]`!
        mk_candle(t0 + 4 * 86_400, 99.0, 104.5, 99.0, 104.0, 1000.0),
        // Bar 5 (`T+0` entry): Marubozu `Open == Low == 100.0`, rallies to `112.0`!
        mk_candle(t0 + 5 * 86_400, 100.0, 112.0, 100.0, 112.0, 1000.0),
        // Bar 6 (`T+1` locked): Rallies to `High = 125.0, Close = 124.0`!
        mk_candle(t0 + 6 * 86_400, 112.0, 125.0, 111.0, 124.0, 1000.0),
        // Bar 7 (`T+2` unlocked): Opens at `108.0` (below ratcheted trail ~118), then spikes to `145.0` after gap exit!
        mk_candle(t0 + 7 * 86_400, 108.0, 145.0, 107.0, 142.0, 1000.0),
    ];

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 2,
            slow_ema: 4,
            atr_period: 3,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongOnly,
        sizing: PositionSizingMode::PercentEquity(90.0),
        risk: StopTakeConfig {
            stop_loss_pct: Some(0.30),
            take_profit_pct: None,
            trailing_stop_atr_mult: Some(1.0),
            min_holding_bars: 2,
            ..StopTakeConfig::default()
        },
        slippage_bps: 50.0,
        commission_bps: 10.0,
        in_sample_split_pct: None,
        ..BacktestConfig::default()
    };

    let res = run_backtest("MARUBOZU_T2", Timeframe::D1, &candles, &cfg, None);
    assert_eq!(res.trades.len(), 1);
    let tr = &res.trades[0];
    assert_eq!(
        tr.entry_bar_index, 5,
        "must enter at bar 5 without phantom entry-bar stop"
    );
    assert_eq!(
        tr.exit_bar_index, 7,
        "must hold through T+0/T+1 and exit at Open[7] (T+2) on ratcheted TrailingStop"
    );
    assert_eq!(tr.exit_reason, BacktestExitReason::TrailingStop);
    // `MFE` came from Bar 6 `High = 125` (~`+24.38%`), NEVER from post-exit Bar 7 `High = 145` (`+44%`)!
    assert!(
        tr.mfe_pct < 30.0,
        "post-exit afternoon spike on gap-stop bar must not leak into MFE (got {})",
        tr.mfe_pct
    );
    assert!(
        tr.mae_pct <= tr.return_pct && tr.return_pct <= tr.mfe_pct,
        "MAE <= Return <= MFE violated: mae={} ret={} mfe={}",
        tr.mae_pct,
        tr.return_pct,
        tr.mfe_pct
    );
}

#[test]
fn walk_forward_rolling_and_anchored_zero_oos_lookahead_and_stitched_compounding() {
    let candles = generate_synthetic_ohlcv(480);
    let base_cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        direction_mode: StrategyDirectionMode::LongShort,
        initial_cash: 100_000.0,
        ..BacktestConfig::default()
    };

    for anchored in [false, true] {
        let wf_cfg = WalkForwardConfig {
            fold_count: 4,
            in_sample_ratio: 0.65,
            anchored,
            ranking_metric: SweepRankingMetric::SharpeRatio,
        };
        let rep = run_walk_forward_validation(
            "SYNTH",
            Timeframe::D1,
            &candles,
            &base_cfg,
            wf_cfg,
            BacktestRunOptions::default(),
        );
        assert_eq!(rep.folds.len(), 4);
        let first_oos = rep.first_oos_bar_index.expect("must have OOS start");
        assert_eq!(rep.folds[0].oos_start_bar, first_oos);
        assert_eq!(rep.folds[3].oos_end_bar, candles.len());
        assert_eq!(rep.stitched_oos_curve.len(), candles.len() - first_oos);

        for (i, fold) in rep.folds.iter().enumerate() {
            assert_eq!(fold.is_end_bar, fold.oos_start_bar);
            if anchored {
                assert_eq!(fold.is_start_bar, 0);
            } else if i > 0 {
                assert!(fold.is_start_bar > 0);
            }
            if i > 0 {
                assert_eq!(rep.folds[i - 1].oos_end_bar, fold.oos_start_bar);
            }
        }
        assert!(
            rep.stitched_equity_values[..first_oos]
                .iter()
                .all(Option::is_none)
        );
        assert!(
            rep.stitched_equity_values[first_oos..]
                .iter()
                .all(Option::is_some)
        );

        // Compounding check across the 4 folds:
        let fold_growth: f64 = rep
            .folds
            .iter()
            .map(|f| 1.0 + f.oos_tearsheet.total_return_pct * 0.01)
            .product();
        let stitched_growth = rep.stitched_oos_tearsheet.final_equity / base_cfg.initial_cash;
        assert!(
            (fold_growth - stitched_growth).abs() < 1e-9,
            "stitched OOS equity must compound fold-by-fold exactly: {fold_growth} vs {stitched_growth}"
        );
    }

    // `TEST-WFO-CAUSAL-001`: Poison future OOS bars (`>= first_oos`) and prove Fold 1 optimization is 100% blind to OOS!
    let wf_cfg = WalkForwardConfig::default();
    let baseline = run_walk_forward_validation(
        "SYNTH",
        Timeframe::D1,
        &candles,
        &base_cfg,
        wf_cfg,
        BacktestRunOptions::default(),
    );
    let oos0 = baseline.folds[0].oos_start_bar;
    let mut poisoned = candles.clone();
    for c in &mut poisoned[oos0..] {
        c.open = 999_999.0;
        c.high = 1_050_000.0;
        c.low = 950_000.0;
        c.close = 1_020_000.0;
    }
    let poisoned_rep = run_walk_forward_validation(
        "SYNTH",
        Timeframe::D1,
        &poisoned,
        &base_cfg,
        wf_cfg,
        BacktestRunOptions::default(),
    );
    assert_eq!(
        baseline.folds[0].winning_label, poisoned_rep.folds[0].winning_label,
        "poisoning OOS bars must NEVER alter Fold 1 winning parameter selection"
    );
    assert_eq!(
        baseline.folds[0].is_tearsheet, poisoned_rep.folds[0].is_tearsheet,
        "poisoning OOS bars must NEVER alter Fold 1 In-Sample tearsheet"
    );

    // Short slice safeguard (`< 36` bars):
    let tiny = run_walk_forward_validation(
        "TINY",
        Timeframe::D1,
        &candles[..24],
        &base_cfg,
        wf_cfg,
        BacktestRunOptions::default(),
    );
    assert_eq!(tiny.verdict, WalkForwardVerdict::InsufficientData);
    assert!(tiny.folds.is_empty());
}

#[test]
fn portfolio_backtest_equal_and_causal_inverse_vol_with_replay_cutoff() {
    let feeds = load_bundled_watchlist_universe();
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::DonchianAtrBreakout,
        initial_cash: 1_000_000.0,
        ..BacktestConfig::default()
    };

    let vn30_eq = run_portfolio_backtest(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::EqualWeight,
        None,
    );
    assert_eq!(vn30_eq.constituents.len(), 8);
    assert!(!vn30_eq.portfolio_equity_curve.is_empty());
    let weight_sum: f64 = vn30_eq
        .constituents
        .iter()
        .map(|c| c.final_weight_pct)
        .sum();
    assert!((weight_sum - 100.0).abs() < 1e-6);

    let all_iv = run_portfolio_backtest(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::AllUniverseFeeds,
        PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 15 },
        None,
    );
    assert_eq!(all_iv.constituents.len(), 13);
    let iv_weight_sum: f64 = all_iv.constituents.iter().map(|c| c.final_weight_pct).sum();
    assert!((iv_weight_sum - 100.0).abs() < 1e-6);

    // Prove `CausalInverseVolatility` uses strictly `..m` (never `..=m`):
    // Mutating the final bar of FPT must NOT alter final step weights chosen BEFORE that bar's return!
    let mut mutated_feeds = feeds.clone();
    if let Some(fpt) = mutated_feeds.iter_mut().find(|f| f.symbol == "FPT")
        && let Some(last) = fpt.candles.last_mut()
    {
        last.high *= 1.80;
        last.close *= 1.75;
    }
    let mutated_iv = run_portfolio_backtest(
        &mutated_feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 15 },
        None,
    );
    let base_vn_iv = run_portfolio_backtest(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 15 },
        None,
    );
    for c_base in &base_vn_iv.constituents {
        let c_mut = mutated_iv
            .constituents
            .iter()
            .find(|x| x.symbol == c_base.symbol)
            .expect("symbol present");
        assert!(
            (c_base.final_weight_pct - c_mut.final_weight_pct).abs() < 1e-12,
            "CausalInverseVolatility weight for {} leaked current-bar return: {} vs {}",
            c_base.symbol,
            c_base.final_weight_pct,
            c_mut.final_weight_pct
        );
    }

    // Causal replay cutoff + `map_equity_curve_to_candles` alignment:
    let fpt_candles = &feeds
        .iter()
        .find(|f| f.symbol == "FPT")
        .expect("FPT")
        .candles;
    let mid_close_cut = fpt_candles[fpt_candles.len() / 2].timestamp + 86_399;
    let cut_port = run_portfolio_backtest(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::EqualWeight,
        Some(mid_close_cut),
    );
    assert!(
        cut_port
            .portfolio_equity_curve
            .iter()
            .all(|pt| pt.timestamp <= mid_close_cut),
        "portfolio replay cutoff must withhold unclosed future bars"
    );
    let (mapped_eq, _, mapped_uw, _) = map_equity_curve_to_candles(
        &vn30_eq.portfolio_equity_curve,
        fpt_candles,
        Timeframe::D1,
        None,
    );
    assert_eq!(mapped_eq.len(), fpt_candles.len());
    assert_eq!(mapped_uw.len(), fpt_candles.len());
    assert!(mapped_eq.iter().any(Option::is_some));
    assert_eq!(
        mapped_eq.last().copied().flatten(),
        vn30_eq.portfolio_equity_curve.last().map(|pt| pt.equity),
        "M-03: mapped D1 portfolio curve must capture bar-close portfolio final equity"
    );
}

#[test]
fn critic_r10_regression_wfo_open_position_carry_conservation_and_intraday_portfolio_causality() {
    // 1. C-02: Walk-Forward carries open positions & pending orders seamlessly across `OOS[k] -> OOS[k+1]`
    //    and obeys `initial_cash + realized_net_profit - final_open_fee == final_cash` bit-exactly!
    let feeds = load_bundled_watchlist_universe();
    let fpt = feeds.iter().find(|f| f.symbol == "FPT").expect("FPT");
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::MacdMomentum,
        params: StrategyParams {
            macd_fast: 6,
            macd_slow: 16,
            macd_signal: 5,
            macd_trend_ema: 0,
            ..StrategyParams::default()
        },
        risk: StopTakeConfig {
            min_holding_bars: 2, // VN T+2 lock preserved across fold boundaries!
            ..StopTakeConfig::default()
        },
        lot_step: 100.0,
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        ..BacktestConfig::default()
    };
    let wf = run_walk_forward_validation(
        "FPT",
        Timeframe::D1,
        &fpt.candles,
        &cfg,
        WalkForwardConfig {
            fold_count: 4,
            in_sample_ratio: 0.65,
            anchored: false,
            ranking_metric: SweepRankingMetric::TotalReturnPct,
        },
        BacktestRunOptions::default(),
    );
    let ts = &wf.stitched_oos_tearsheet;
    let sum_closed_pnl: f64 = wf.stitched_oos_trades.iter().map(|t| t.net_pnl).sum();
    assert!((ts.realized_net_profit - sum_closed_pnl).abs() < 1e-6);
    // Notice `final_cash` equals `initial_cash + realized_net_profit` when flat, or minus open entry fee when carrying an open trade!
    assert!(
        ts.final_cash <= ts.initial_cash + sum_closed_pnl + 1e-6,
        "C-02: WFO stitched cash conservation violated"
    );

    // 2. C-01 & N-04: Mixed `D1` (`FPT`) + `15m` (`BTCUSDT`) portfolio stamps `D1` at `23:59:59 UTC`
    //    so Live prefix up to an intraday timestamp `mid_intraday_cut` equals Replay `Some(mid_intraday_cut)` bit-for-bit!
    let btc = feeds
        .iter()
        .find(|f| f.symbol == "BTCUSDT")
        .expect("BTCUSDT")
        .clone();
    let mixed_feeds = vec![fpt.clone(), btc.clone()];
    let live_port = run_portfolio_backtest(
        &mixed_feeds,
        Timeframe::M15,
        &cfg,
        PortfolioUniverseFilter::AllUniverseFeeds,
        PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 10 },
        None,
    );
    let contrib_sum: f64 = live_port
        .constituents
        .iter()
        .map(|c| c.weighted_return_contribution_pct)
        .sum();
    assert!(
        (contrib_sum - live_port.portfolio_tearsheet.total_return_pct).abs() < 1e-6,
        "N-04: compounded constituent attribution ({contrib_sum}) must sum identically to portfolio total_return_pct ({})",
        live_port.portfolio_tearsheet.total_return_pct
    );

    // Pick a midday intraday cutoff (`12:14:59 UTC`, strictly mid-day inside a `D1` session)
    let mid_btc_bar = &btc.candles[btc.candles.len() / 2];
    let intraday_cut =
        (mid_btc_bar.timestamp.div_euclid(86_400)) * 86_400 + 12 * 3_600 + 14 * 60 + 59;
    let replay_port = run_portfolio_backtest(
        &mixed_feeds,
        Timeframe::M15,
        &cfg,
        PortfolioUniverseFilter::AllUniverseFeeds,
        PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 10 },
        Some(intraday_cut),
    );
    let live_prefix: Vec<_> = live_port
        .portfolio_equity_curve
        .iter()
        .filter(|pt| pt.timestamp <= intraday_cut)
        .collect();
    assert_eq!(
        live_prefix.len(),
        replay_port.portfolio_equity_curve.len(),
        "C-01: Live mixed-frequency portfolio leaked end-of-day D1 bar before 23:59:59 UTC"
    );
    for (a, b) in live_prefix
        .iter()
        .zip(replay_port.portfolio_equity_curve.iter())
    {
        assert!((a.equity - b.equity).abs() < 1e-9);
    }
}
