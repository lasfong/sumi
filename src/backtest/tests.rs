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
        ..BacktestConfig::default()
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
        ..BacktestConfig::default()
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
        custom_rules: CustomSignalRuleSpec::default(),
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

    // Direct `run_backtest_with_options` with intraday cutoff during active VN morning session (`02:03 UTC = 09:03 ICT`)
    // must exclude the unclosed `2026-10-15` D1 bar (`39`), whereas after HOSE afternoon close (`07:45 UTC = 14:45 ICT`,
    // including `09:03 UTC = 16:03 ICT`) the session is closed and monotonically yields `40` bars (`BUG-010`)!
    let slot_0200 = day_2026_10_15 + 2 * 3_600;
    let direct_fpt_morning = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        &fpt_d1,
        &cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: Some(slot_0200 + 3 * 60),
            incomplete_tail_bar: false,
        },
    );
    assert_eq!(direct_fpt_morning.bar_count, 39);
    let direct_fpt_post_close = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        &fpt_d1,
        &cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: Some(slot_0900 + 3 * 60),
            incomplete_tail_bar: false,
        },
    );
    assert_eq!(direct_fpt_post_close.bar_count, 40);

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

#[test]
fn r15_monte_carlo_splitmix64_determinism_monotonic_quantiles_and_edge_cases() {
    let feeds = load_bundled_watchlist_universe();
    let fpt = feeds.iter().find(|f| f.symbol == "FPT").expect("FPT");
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::MacdMomentum,
        initial_cash: 100_000.0,
        ..BacktestConfig::default()
    };
    let bt = run_backtest("FPT", Timeframe::D1, &fpt.candles, &cfg, None);
    assert!(!bt.trades.is_empty());

    let mc_cfg = MonteCarloConfig {
        simulations: 1_000,
        seed: DEFAULT_MONTE_CARLO_SEED,
        ruin_drawdown_pct: 35.0,
        horizon_steps: None,
    };
    let rep_a = run_monte_carlo_bootstrap(
        cfg.initial_cash,
        &bt.trades,
        &bt.equity_curve,
        bt.bar_count,
        None,
        mc_cfg,
    );
    let rep_b = run_monte_carlo_bootstrap(
        cfg.initial_cash,
        &bt.trades,
        &bt.equity_curve,
        bt.bar_count,
        None,
        mc_cfg,
    );
    assert_eq!(
        rep_a.metrics, rep_b.metrics,
        "SplitMix64 must be 100% deterministic"
    );
    assert_eq!(rep_a.checkpoint_envelopes, rep_b.checkpoint_envelopes);

    let m = rep_a.metrics.expect("metrics present");
    assert!(m.terminal_equity.p5 <= m.terminal_equity.p25);
    assert!(m.terminal_equity.p25 <= m.terminal_equity.p50);
    assert!(m.terminal_equity.p50 <= m.terminal_equity.p75);
    assert!(m.terminal_equity.p75 <= m.terminal_equity.p95);
    assert!(0.0 <= m.max_drawdown_p50_pct);
    assert!(m.max_drawdown_p50_pct <= m.max_drawdown_var_95_pct);
    assert!(m.max_drawdown_var_95_pct <= m.max_drawdown_var_99_pct);
    assert!(m.max_drawdown_var_99_pct <= 100.0);
    assert!(m.max_drawdown_var_95_pct <= m.max_drawdown_cvar_95_pct);
    assert!(m.max_drawdown_var_99_pct <= m.max_drawdown_cvar_99_pct);

    let fan = rep_a.bar_fan.as_ref().expect("bar_fan projected");
    assert_eq!(fan.bar_count, bt.bar_count);
    for i in 0..fan.causal_bar_count {
        let (v5, v25, v50, v75, v95) = (
            fan.p5[i].unwrap(),
            fan.p25[i].unwrap(),
            fan.p50[i].unwrap(),
            fan.p75[i].unwrap(),
            fan.p95[i].unwrap(),
        );
        assert!(v5 <= v25 && v25 <= v50 && v50 <= v75 && v75 <= v95);
    }

    // Edge case 1: 0 trades -> InsufficientSamples (`metrics == None`, no fake precision)
    let empty_rep = run_monte_carlo_bootstrap(100_000.0, &[], &[], 120, None, mc_cfg);
    assert_eq!(empty_rep.validity, MonteCarloValidity::InsufficientSamples);
    assert!(empty_rep.metrics.is_none() && empty_rep.bar_fan.is_none());

    // Edge case 2: 1 trade -> SingleSampleDegenerate (`p5 == p95`)
    let one_rep = run_monte_carlo_bootstrap(
        cfg.initial_cash,
        &bt.trades[..1],
        &bt.equity_curve,
        bt.bar_count,
        None,
        mc_cfg,
    );
    assert_eq!(one_rep.validity, MonteCarloValidity::SingleSampleDegenerate);
    assert!(one_rep.low_sample_warning);
    let m1 = one_rep.metrics.unwrap();
    assert!((m1.terminal_equity.p5 - m1.terminal_equity.p95).abs() < 1e-9);

    // Edge case 3: Total liquidation (`net_pnl = -initial_cash`) -> post-bankruptcy trades ignored & permanent bankruptcy at 0.0 (`prob_ruin = 100%`)
    let mut liq_trade = bt.trades[0].clone();
    liq_trade.net_pnl = -100_000.0;
    liq_trade.exit_reason = BacktestExitReason::MarginLiquidation;
    let mut rebound_trade = bt.trades[0].clone();
    rebound_trade.net_pnl = 50_000.0;
    let (ext_rets, _) =
        extract_causal_trade_equity_returns(100_000.0, &[liq_trade, rebound_trade], None);
    assert_eq!(
        ext_rets,
        vec![-1.0],
        "trades after account equity hits 0.0 must never resurrect capital"
    );
    let liq_rep = run_monte_carlo_bootstrap(
        100_000.0,
        &[BacktestTrade {
            net_pnl: -100_000.0,
            exit_reason: BacktestExitReason::MarginLiquidation,
            ..bt.trades[0].clone()
        }],
        &bt.equity_curve,
        bt.bar_count,
        None,
        mc_cfg,
    );
    let ml = liq_rep.metrics.unwrap();
    assert_eq!(ml.terminal_equity.p95, 0.0);
    assert_eq!(ml.max_drawdown_var_95_pct, 100.0);
    assert_eq!(ml.prob_of_ruin_pct, 100.0);

    // Edge case 4: DEF-001 cutoff timestamp masks future trades & future fan bars!
    if bt.trades.len() >= 3 {
        let mid_cut = bt.trades[bt.trades.len() / 2].exit_timestamp;
        let cut_rep = run_monte_carlo_bootstrap(
            cfg.initial_cash,
            &bt.trades,
            &bt.equity_curve,
            bt.bar_count,
            Some(mid_cut),
            mc_cfg,
        );
        let prefix_trades: Vec<BacktestTrade> = bt
            .trades
            .iter()
            .filter(|t| t.exit_timestamp <= mid_cut)
            .cloned()
            .collect();
        let prefix_curve: Vec<BacktestEquityPoint> = bt
            .equity_curve
            .iter()
            .filter(|p| p.timestamp <= mid_cut)
            .copied()
            .collect();
        let exact_prefix_rep = run_monte_carlo_bootstrap(
            cfg.initial_cash,
            &prefix_trades,
            &prefix_curve,
            bt.bar_count,
            Some(mid_cut),
            mc_cfg,
        );
        assert_eq!(cut_rep.metrics, exact_prefix_rep.metrics);
        let cfan = cut_rep.bar_fan.unwrap();
        assert!(cfan.causal_bar_count < cfan.bar_count);
        assert!(cfan.p50[cfan.causal_bar_count].is_none());
    }
}

#[test]
fn critic_r15_regression_wfo_and_portfolio_mc_fan_bar_index_alignment_and_seed_identity() {
    use super::{
        DEFAULT_MONTE_CARLO_SEED, MonteCarloConfig, PortfolioUniverseFilter,
        PortfolioWeightingMode, WalkForwardConfig, map_equity_curve_to_candles,
        run_monte_carlo_bootstrap, run_monte_carlo_portfolio_mapped, run_portfolio_backtest,
        run_walk_forward_validation,
    };
    use crate::data::load_bundled_watchlist_universe;

    assert_eq!(DEFAULT_MONTE_CARLO_SEED, 202_615);

    let feeds = load_bundled_watchlist_universe();
    let btc = feeds
        .iter()
        .find(|f| f.symbol == "BTCUSDT")
        .expect("BTCUSDT");

    // 1. WalkForwardOos with `fold_count = 2, in_sample_ratio = 0.75` (`first_oos_start > stitched_oos_curve.len()`)
    let bt_cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 8,
            slow_ema: 21,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongShort,
        initial_cash: 100_000.0,
        ..BacktestConfig::default()
    };
    let wf_cfg = WalkForwardConfig {
        fold_count: 2,
        in_sample_ratio: 0.75,
        anchored: false,
        ranking_metric: SweepRankingMetric::SharpeRatio,
    };
    let wf = run_walk_forward_validation(
        "BTCUSDT",
        Timeframe::M15,
        &btc.candles,
        &bt_cfg,
        wf_cfg,
        BacktestRunOptions::default(),
    );
    let first_oos = wf.first_oos_bar_index.expect("first_oos_bar_index");
    assert!(
        first_oos > wf.stitched_oos_curve.len(),
        "reproduce critic R15-M01: first_oos ({first_oos}) > oos_len ({})",
        wf.stitched_oos_curve.len()
    );

    let mc_wfo = run_monte_carlo_bootstrap(
        bt_cfg.initial_cash,
        &wf.stitched_oos_trades,
        &wf.stitched_oos_curve,
        btc.candles.len(),
        None,
        MonteCarloConfig {
            simulations: 300,
            ..MonteCarloConfig::default()
        },
    );
    let fan_wfo = mc_wfo.bar_fan.as_ref().expect("WFO bar_fan must exist");
    assert_eq!(fan_wfo.bar_count, btc.candles.len());
    assert_eq!(fan_wfo.causal_bar_count, btc.candles.len());
    for i in 0..first_oos {
        assert!(
            fan_wfo.p50[i].is_none(),
            "In-Sample bar {i} < {first_oos} must stay None in WFO Monte Carlo fan"
        );
    }
    assert!(
        fan_wfo.p50[btc.candles.len() - 1].is_some(),
        "WFO Monte Carlo fan must reach terminal OOS bar"
    );

    // 2. MultiAssetPortfolio (`Vn30EquitiesOnly` 1D) mapped onto intraday `BTCUSDT 15m` candles + Replay cutoff
    let port = run_portfolio_backtest(
        &feeds,
        Timeframe::D1,
        &bt_cfg,
        PortfolioUniverseFilter::AllUniverseFeeds,
        PortfolioWeightingMode::EqualWeight,
        None,
    );
    let fpt = feeds.iter().find(|f| f.symbol == "FPT").expect("FPT");
    let cut_ts = fpt.candles[120].timestamp + 86_399;
    let (mapped_eq, _, _, _) = map_equity_curve_to_candles(
        &port.portfolio_equity_curve,
        &fpt.candles,
        Timeframe::D1,
        Some(cut_ts),
    );
    let mc_port = run_monte_carlo_portfolio_mapped(
        port.initial_capital,
        &port.portfolio_equity_curve,
        &mapped_eq,
        &fpt.candles,
        Some(cut_ts),
        MonteCarloConfig {
            simulations: 300,
            ..MonteCarloConfig::default()
        },
    );
    let fan_port = mc_port
        .bar_fan
        .as_ref()
        .expect("Portfolio mapped bar_fan must exist");
    assert_eq!(fan_port.bar_count, fpt.candles.len());
    assert_eq!(fan_port.causal_bar_count, 121);
    assert!(fan_port.p50[120].is_some());
    assert!(
        fan_port.p50[121].is_none(),
        "bars past cutoff_timestamp must remain None in portfolio mapped fan"
    );
}

#[test]
fn r16_custom_signal_rules_fail_closed_bool_false_and_empty_group_vacuous_guard() {
    use crate::scanner::ConfluenceWeightProfile;

    let base_ts = 1_710_000_000_i64;
    // 1. Series with warmup (< 50), zero-volume baseline (`volume == 0.0`), and negative-volume bar:
    //    `BoolIs(false)` on `volume.spike_2x` MUST fail closed (`false`), NEVER invert `None` -> `true`!
    let mut zero_and_bad_vol: Vec<Candle> = (0..75_usize)
        .map(|i| {
            let p = 100.0 + (i as f64) * 0.5;
            let vol = if i == 60 { -25.0 } else { 0.0 };
            mk_candle(
                base_ts + (i as i64) * 86_400,
                p - 0.3,
                p + 0.7,
                p - 0.6,
                p,
                vol,
            )
        })
        .collect();
    // Ensure warmup subset (< 50) also tested
    let negated_spike_leg = DirectionalRuleLeg {
        enabled: true,
        inter_group_op: LogicalCombinator::AllAnd,
        groups: vec![SignalClauseGroup {
            intra_op: LogicalCombinator::AllAnd,
            clauses: vec![SignalClause {
                catalog_id: "volume.spike_2x".to_owned(),
                predicate: SignalPredicate::BoolIs(false),
                direction_filter: None,
            }],
        }],
        confluence_gate: ConfluenceThresholdGate::default(),
    };
    let mut cfg = BacktestConfig {
        preset: StrategyPresetKind::CustomSignalRules,
        direction_mode: StrategyDirectionMode::LongOnly,
        custom_rules: CustomSignalRuleSpec {
            name: "FailClosedCheck".to_owned(),
            long_entry: negated_spike_leg,
            long_exit: DirectionalRuleLeg::default(),
            short_entry: DirectionalRuleLeg::default(),
            short_exit: DirectionalRuleLeg::default(),
        },
        ..BacktestConfig::default()
    };
    let mat_bad = crate::scanner::PrecomputedCatalogMatrix::compute(&zero_and_bad_vol);
    let counts = count_custom_rule_matches(&cfg.custom_rules, &mat_bad);
    assert_eq!(
        counts,
        (0, 0, 0, 0),
        "BoolIs(false) must NEVER match Warmup, ZeroBaseline, or InvalidVolume bars"
    );
    let res = run_backtest("ZERO", Timeframe::D1, &zero_and_bad_vol, &cfg, None);
    assert!(res.trades.is_empty() && res.open_position.is_none());

    // Once positive baseline volume is restored, `BoolIs(false)` legitimately matches non-spike Valid bars >= 50!
    for c in &mut zero_and_bad_vol {
        c.volume = 1_000.0;
    }
    let mat_ok = crate::scanner::PrecomputedCatalogMatrix::compute(&zero_and_bad_vol);
    let valid_counts = count_custom_rule_matches(&cfg.custom_rules, &mat_ok);
    assert!(
        valid_counts.0 > 0,
        "BoolIs(false) matches Valid non-spike bars once baseline volume > 0"
    );

    // 2. Empty groups / empty legs must NEVER fire vacuous truth (`[].iter().all(...) == true`) even when `confluence_gate` is enabled!
    cfg.custom_rules.long_entry = DirectionalRuleLeg {
        enabled: true,
        inter_group_op: LogicalCombinator::AllAnd,
        groups: vec![SignalClauseGroup {
            intra_op: LogicalCombinator::AllAnd,
            clauses: Vec::new(),
        }],
        confluence_gate: ConfluenceThresholdGate {
            enabled: true,
            profile: ConfluenceWeightProfile::Balanced,
            min_score: Some(-100.0),
            max_score: Some(100.0),
            min_bullish_signals: 0,
            min_bearish_signals: 0,
        },
    };
    assert_eq!(
        count_custom_rule_matches(&cfg.custom_rules, &mat_ok),
        (0, 0, 0, 0),
        "empty clause groups must never vacuous-match"
    );
    cfg.sanitize();
    assert!(cfg.custom_rules.long_entry.groups.is_empty());
}

#[test]
fn r16_custom_signal_rules_two_level_and_or_cross_regime_and_bidirectional_direction_filter() {
    use crate::scanner::{MarketRegime, SignalDirection};

    // Construct a 90-bar series that trends steadily up for 0..75, then suffers a sharp Bearish breakdown
    // (`sr.prior_20b_breakout` + `structure.swing_bos` with `bool_value = Some(true)` and `direction = Bearish`)!
    let base_ts = 1_715_000_000_i64;
    let mut bars: Vec<Candle> = Vec::with_capacity(90);
    for i in 0..75_usize {
        let p = 100.0 + (i as f64) * 0.4;
        bars.push(mk_candle(
            base_ts + (i as i64) * 86_400,
            p - 0.2,
            p + 0.5,
            p - 0.4,
            p,
            1_000.0,
        ));
    }
    // Bar 75 plunges below prior 20-bar low (~122.0) down to 112.0 -> Bearish `sr.prior_20b_breakout`!
    for i in 75..90_usize {
        let p = if i == 75 {
            112.0
        } else {
            112.0 - ((i - 75) as f64) * 0.5
        };
        bars.push(mk_candle(
            base_ts + (i as i64) * 86_400,
            p + 1.5,
            p + 1.8,
            p - 0.6,
            p,
            2_500.0,
        ));
    }

    // Long leg filtering `sr.prior_20b_breakout` with `Bullish` MUST NOT match bar 75's Bearish breakdown,
    // whereas Short leg filtering `Bearish` MUST match bar 75!
    let mut spec = CustomSignalRuleSpec {
        name: "BidirectionalDisambiguation".to_owned(),
        long_entry: DirectionalRuleLeg {
            enabled: true,
            inter_group_op: LogicalCombinator::AllAnd,
            groups: vec![SignalClauseGroup {
                intra_op: LogicalCombinator::AllAnd,
                clauses: vec![SignalClause {
                    catalog_id: "sr.prior_20b_breakout".to_owned(),
                    predicate: SignalPredicate::BoolIs(true),
                    direction_filter: Some(SignalDirection::Bullish),
                }],
            }],
            confluence_gate: ConfluenceThresholdGate::default(),
        },
        long_exit: DirectionalRuleLeg::default(),
        short_entry: DirectionalRuleLeg {
            enabled: true,
            inter_group_op: LogicalCombinator::AllAnd,
            groups: vec![
                SignalClauseGroup {
                    intra_op: LogicalCombinator::AllAnd,
                    clauses: vec![SignalClause {
                        catalog_id: "sr.prior_20b_breakout".to_owned(),
                        predicate: SignalPredicate::BoolIs(true),
                        direction_filter: Some(SignalDirection::Bearish),
                    }],
                },
                SignalClauseGroup {
                    intra_op: LogicalCombinator::AnyOr,
                    clauses: vec![
                        SignalClause {
                            catalog_id: "volume.spike_2x".to_owned(),
                            predicate: SignalPredicate::BoolIs(true),
                            direction_filter: None,
                        },
                        SignalClause {
                            catalog_id: "regime.market_state".to_owned(),
                            predicate: SignalPredicate::RegimeIn {
                                mask: MarketRegime::ALL_VALID_MASK,
                            },
                            direction_filter: None,
                        },
                    ],
                },
            ],
            confluence_gate: ConfluenceThresholdGate::default(),
        },
        short_exit: DirectionalRuleLeg {
            enabled: true,
            inter_group_op: LogicalCombinator::AllAnd,
            groups: vec![SignalClauseGroup {
                intra_op: LogicalCombinator::AllAnd,
                clauses: vec![SignalClause {
                    catalog_id: "flow.cvd_ohlcv_impulse".to_owned(),
                    predicate: SignalPredicate::FloatCmp {
                        op: FloatCmpOp::CrossAbove,
                        threshold: 0.25,
                    },
                    direction_filter: None,
                }],
            }],
            confluence_gate: ConfluenceThresholdGate::default(),
        },
    };
    spec.sanitize();

    let mat_75 = crate::scanner::PrecomputedCatalogMatrix::compute(&bars[..=75]);
    let mat_74 = crate::scanner::PrecomputedCatalogMatrix::compute(&bars[..=74]);
    let counts_at_75 = count_custom_rule_matches(&spec, &mat_75);
    let counts_at_74 = count_custom_rule_matches(&spec, &mat_74);
    assert_eq!(
        counts_at_75.0 - counts_at_74.0,
        0,
        "Bearish breakdown at bar 75 must NOT increment Bullish-filtered long_entry"
    );
    assert_eq!(
        counts_at_75.1 - counts_at_74.1,
        1,
        "Bearish breakdown at bar 75 MUST increment Bearish-filtered 2-group short_entry"
    );
}

#[test]
fn r16_custom_signal_rules_next_bar_open_fill_cutoff_and_future_poisoning_invariance() {
    let feeds = load_bundled_watchlist_universe();
    let fpt = feeds.iter().find(|f| f.symbol == "FPT").expect("FPT");
    assert!(fpt.candles.len() >= 150);

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::CustomSignalRules,
        direction_mode: StrategyDirectionMode::LongOnly,
        sizing: PositionSizingMode::PercentEquity(95.0),
        lot_step: 100.0,
        in_sample_split_pct: Some(0.70),
        custom_rules: CustomSignalRuleSpec::default(),
        ..BacktestConfig::default()
    };

    let cut_idx = 120_usize;
    let cut_ts = fpt.candles[cut_idx].timestamp;
    let clean_bt = run_backtest("FPT", Timeframe::D1, &fpt.candles, &cfg, Some(cut_ts));
    let clean_mat = crate::scanner::PrecomputedCatalogMatrix::compute(&fpt.candles[..=cut_idx]);
    let clean_matches = count_custom_rule_matches(&cfg.custom_rules, &clean_mat);

    // Poison future bars > cut_idx by 50x: both `run_backtest` and causal matrix matches must stay bit-exact!
    let mut poisoned = fpt.candles.clone();
    for c in &mut poisoned[(cut_idx + 1)..] {
        c.open *= 50.0;
        c.high *= 60.0;
        c.low *= 0.02;
        c.close *= 55.0;
        c.volume *= 500.0;
    }
    let poisoned_bt = run_backtest("FPT", Timeframe::D1, &poisoned, &cfg, Some(cut_ts));
    let poisoned_mat = crate::scanner::PrecomputedCatalogMatrix::compute(&poisoned[..=cut_idx]);
    let poisoned_matches = count_custom_rule_matches(&cfg.custom_rules, &poisoned_mat);
    assert_eq!(clean_bt.trades, poisoned_bt.trades);
    assert_eq!(clean_bt.equity_curve, poisoned_bt.equity_curve);
    assert_eq!(clean_bt.tearsheet, poisoned_bt.tearsheet);
    assert_eq!(clean_matches, poisoned_matches);

    // Verify `Open[T+1]` fill invariant on every executed trade: entry_bar_index >= 51 (strictly after warmup bar 50)
    // and entry_price matches `Open[entry_bar_index]` adjusted by slippage!
    let mut zero_slip_cfg = cfg.clone();
    zero_slip_cfg.slippage_bps = 0.0;
    let full_bt = run_backtest("FPT", Timeframe::D1, &fpt.candles, &zero_slip_cfg, None);
    for tr in &full_bt.trades {
        assert!(tr.entry_bar_index >= 51);
        assert!(
            (tr.entry_price - fpt.candles[tr.entry_bar_index].open).abs() < 1e-9,
            "CustomSignalRules trade must fill at exact Open[T+1]"
        );
    }

    // Verify terminal-bar signal produces `EndOfSeriesCutoff` when `incomplete_tail_bar: false` and is suppressed when `incomplete_tail_bar: true`!
    let mut always_valid_cfg = cfg.clone();
    always_valid_cfg.custom_rules.long_entry = DirectionalRuleLeg {
        enabled: true,
        inter_group_op: LogicalCombinator::AllAnd,
        groups: vec![SignalClauseGroup {
            intra_op: LogicalCombinator::AllAnd,
            clauses: vec![SignalClause {
                catalog_id: "regime.market_state".to_owned(),
                predicate: SignalPredicate::RegimeIn {
                    mask: crate::scanner::MarketRegime::ALL_VALID_MASK,
                },
                direction_filter: None,
            }],
        }],
        confluence_gate: ConfluenceThresholdGate::default(),
    };
    // Slice to 52 bars: bar 50 is first valid signal -> opens at bar 51; or slice to 51 bars (`0..=50`): bar 50 is terminal bar -> `EndOfSeriesCutoff`!
    let slice_51 = &fpt.candles[..51];
    let term_closed = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        slice_51,
        &always_valid_cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: None,
            incomplete_tail_bar: false,
        },
    );
    assert_eq!(term_closed.trades.len(), 0);
    assert!(term_closed.open_position.is_none());
    assert_eq!(term_closed.unexecuted_signals.len(), 1);
    assert_eq!(term_closed.unexecuted_signals[0].signal_bar_index, 50);
    assert_eq!(
        term_closed.unexecuted_signals[0].reason,
        UnfilledReason::EndOfSeriesCutoff
    );

    let term_incomplete = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        slice_51,
        &always_valid_cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: None,
            incomplete_tail_bar: true,
        },
    );
    assert!(
        term_incomplete.unexecuted_signals.is_empty(),
        "forming higher-TF tail bar must NOT emit premature CustomSignalRules signal"
    );
}

#[test]
fn r16_custom_signal_rules_12_sweep_wfo_portfolio_battle_and_monte_carlo_integration() {
    let feeds = load_bundled_watchlist_universe();
    let btc = feeds.iter().find(|f| f.symbol == "BTCUSDT").expect("BTC");

    let cfg = BacktestConfig {
        preset: StrategyPresetKind::CustomSignalRules,
        direction_mode: StrategyDirectionMode::LongShort,
        sizing: PositionSizingMode::PercentEquity(90.0),
        in_sample_split_pct: Some(0.70),
        custom_rules: CustomSignalRuleSpec::default(),
        ..BacktestConfig::default()
    };

    // 1. 12-variant Sweep (`Arc<PrecomputedCatalogMatrix>` shared across all 12 variants)
    let sweep = run_parameter_sweep(
        "BTCUSDT",
        btc.base_timeframe,
        &btc.candles,
        &cfg,
        SweepRankingMetric::SharpeRatio,
        BacktestRunOptions::default(),
    );
    assert_eq!(sweep.variants.len(), 12);
    for (idx, v) in sweep.variants.iter().enumerate() {
        assert_eq!(v.rank, idx + 1);
        assert!(v.tearsheet.total_return_pct.is_finite());
        assert!(!v.label.is_empty());
    }

    // 2. All 7 presets battle includes `CustomSignalRules`
    let battle = run_all_presets_battle(
        "BTCUSDT",
        btc.base_timeframe,
        &btc.candles,
        &cfg,
        SweepRankingMetric::SharpeRatio,
        BacktestRunOptions::default(),
    );
    assert_eq!(battle.len(), StrategyPresetKind::ALL.len());
    assert!(
        battle
            .iter()
            .any(|v| v.preset == StrategyPresetKind::CustomSignalRules)
    );

    // 3. 4-Fold Walk-Forward Optimization + Monte Carlo bootstrap determinism
    let wf = run_walk_forward_validation(
        "BTCUSDT",
        btc.base_timeframe,
        &btc.candles,
        &cfg,
        WalkForwardConfig {
            fold_count: 4,
            in_sample_ratio: 0.70,
            anchored: false,
            ranking_metric: SweepRankingMetric::SharpeRatio,
        },
        BacktestRunOptions::default(),
    );
    assert_eq!(wf.folds.len(), 4);
    assert!(!wf.stitched_oos_curve.is_empty() && wf.stitched_oos_curve.len() < btc.candles.len());

    let bt = run_backtest("BTCUSDT", btc.base_timeframe, &btc.candles, &cfg, None);
    let mc_a = run_monte_carlo_bootstrap(
        cfg.initial_cash,
        &bt.trades,
        &bt.equity_curve,
        btc.candles.len(),
        None,
        MonteCarloConfig {
            simulations: 250,
            seed: 0x516E_A116,
            ..MonteCarloConfig::default()
        },
    );
    let mc_b = run_monte_carlo_bootstrap(
        cfg.initial_cash,
        &bt.trades,
        &bt.equity_curve,
        btc.candles.len(),
        None,
        MonteCarloConfig {
            simulations: 250,
            seed: 0x516E_A116,
            ..MonteCarloConfig::default()
        },
    );
    assert_eq!(mc_a.metrics, mc_b.metrics);
    assert_eq!(mc_a.checkpoint_envelopes, mc_b.checkpoint_envelopes);
    assert_eq!(
        mc_a.bar_fan.as_ref().map(|f| (&f.p5, &f.p50, &f.p95)),
        mc_b.bar_fan.as_ref().map(|f| (&f.p5, &f.p50, &f.p95))
    );

    // 4. Multi-Asset Portfolio with `CustomSignalRules`
    let port = run_portfolio_backtest(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::AllUniverseFeeds,
        PortfolioWeightingMode::EqualWeight,
        None,
    );
    assert!(!port.portfolio_equity_curve.is_empty());
    assert!(port.portfolio_tearsheet.final_equity.is_finite());
}

#[test]
fn critic_r16_regression_spy_thursday_w1_vn30f_session_close_longonly_exit_isolation_and_trendfocused_clause()
 {
    use crate::backtest::strategy::evaluate_signal_clause_at_with_profile;
    use crate::scanner::{
        ConfluenceWeightProfile, PrecomputedCatalogMatrix, evaluate_causal_confluence,
    };

    // 1. `[Critical-01]` Non-midnight `SPY` (`14:30:00 UTC`) `D1 -> W1` weekday uses `last_base_open_ts`:
    //    `19789` days since Unix epoch = Thursday `2024-03-07` (`(19789 + 4) % 7 == 4`), `19790` = Friday.
    let spy_thu_open = 19_789_i64 * 86_400 + 14 * 3_600 + 30 * 60;
    let spy_fri_open = 19_790_i64 * 86_400 + 14 * 3_600 + 30 * 60;
    let w1_bucket_open = Timeframe::W1.bucket_start_timestamp(spy_thu_open);
    assert!(
        is_bucket_incomplete_at_close_for_symbol(
            "SPY",
            w1_bucket_open,
            spy_thu_open + 86_399,
            Timeframe::D1,
            Timeframe::W1,
        ),
        "Thursday 14:30 UTC D1 bar (close Fri 14:29:59 UTC) must still be incomplete on W1"
    );
    assert!(
        !is_bucket_incomplete_at_close_for_symbol(
            "SPY",
            w1_bucket_open,
            spy_fri_open + 86_399,
            Timeframe::D1,
            Timeframe::W1,
        ),
        "Friday 14:30 UTC D1 bar must close the W1 bucket"
    );

    // 2. `[Critical-01]` Vietnam `VN30F1M` `1m` morning (`04:29:59 UTC`) & afternoon (`07:44:59 UTC`) closes
    //    vs 24/7 `BTCUSDT` UTC boundary behavior:
    let thu_mid_utc = 19_789_i64 * 86_400;
    let fri_mid_utc = 19_790_i64 * 86_400;
    let vn_morning_close = thu_mid_utc + 4 * 3_600 + 29 * 60 + 59; // 11:30 ICT morning close
    let vn_thu_aft_close = thu_mid_utc + 7 * 3_600 + 44 * 60 + 59; // 14:45 ICT afternoon close (Thu)
    let vn_fri_aft_close = fri_mid_utc + 7 * 3_600 + 44 * 60 + 59; // 14:45 ICT afternoon close (Fri)

    assert!(!is_bucket_incomplete_at_close_for_symbol(
        "VN30F1M",
        Timeframe::H1.bucket_start_timestamp(vn_morning_close),
        vn_morning_close,
        Timeframe::M1,
        Timeframe::H1,
    ));
    assert!(is_bucket_incomplete_at_close_for_symbol(
        "VN30F1M",
        Timeframe::D1.bucket_start_timestamp(vn_morning_close),
        vn_morning_close,
        Timeframe::M1,
        Timeframe::D1,
    ));
    for tf in [Timeframe::H1, Timeframe::H4, Timeframe::D1] {
        assert!(
            !is_bucket_incomplete_at_close_for_symbol(
                "VN30F1M",
                tf.bucket_start_timestamp(vn_thu_aft_close),
                vn_thu_aft_close,
                Timeframe::M1,
                tf,
            ),
            "VN30F1M afternoon close 07:44:59 UTC must close {tf:?}"
        );
    }
    assert!(
        is_bucket_incomplete_at_close_for_symbol(
            "VN30F1M",
            Timeframe::W1.bucket_start_timestamp(vn_thu_aft_close),
            vn_thu_aft_close,
            Timeframe::M1,
            Timeframe::W1,
        ),
        "Thursday afternoon close must NOT prematurely close W1"
    );
    assert!(
        !is_bucket_incomplete_at_close_for_symbol(
            "VN30F1M",
            Timeframe::W1.bucket_start_timestamp(vn_fri_aft_close),
            vn_fri_aft_close,
            Timeframe::M1,
            Timeframe::W1,
        ),
        "Friday afternoon close 07:44:59 UTC MUST close W1 on VN30F1M"
    );
    // 24/7 crypto at 07:44:59 UTC remains unclosed on both H1 and D1:
    assert!(is_bucket_incomplete_at_close_for_symbol(
        "BTCUSDT",
        Timeframe::H1.bucket_start_timestamp(vn_thu_aft_close),
        vn_thu_aft_close,
        Timeframe::M1,
        Timeframe::H1,
    ));
    assert!(is_bucket_incomplete_at_close_for_symbol(
        "BTCUSDT",
        Timeframe::D1.bucket_start_timestamp(vn_thu_aft_close),
        vn_thu_aft_close,
        Timeframe::M1,
        Timeframe::D1,
    ));

    // Round-2 Critic regression: bounded close windows prevent mid-bucket `DEF-001` leaks after 08:00 / 21:00 UTC!
    // (a) Custom non-VN ticker `"AMZN"` at Friday `14:32:59 UTC` (`2` mins into `14:00` `1h` bar) -> `incomplete == true` across `H1`/`H4`/`D1`/`W1`!
    let amzn_fri_1432_close = fri_mid_utc + 14 * 3_600 + 32 * 60 + 59;
    for tf in [Timeframe::H1, Timeframe::H4, Timeframe::D1, Timeframe::W1] {
        assert!(
            is_bucket_incomplete_at_close_for_symbol(
                "AMZN",
                tf.bucket_start_timestamp(amzn_fri_1432_close),
                amzn_fri_1432_close,
                Timeframe::M1,
                tf,
            ),
            "Custom ticker AMZN at 14:32:59 UTC must remain incomplete on {tf:?} (no unbounded >= 07:44:59 leak!)"
        );
    }
    // (b) Anonymous `is_bucket_incomplete_at_close("", ...)` at `07:44:59 UTC` and `22:02:59 UTC` -> pure UTC boundary (`true`)!
    let anon_fri_2202_close = fri_mid_utc + 22 * 3_600 + 2 * 60 + 59;
    for ts_close in [vn_fri_aft_close, anon_fri_2202_close] {
        for tf in [Timeframe::H1, Timeframe::D1, Timeframe::W1] {
            assert!(
                is_bucket_incomplete_at_close(
                    tf.bucket_start_timestamp(ts_close),
                    ts_close,
                    Timeframe::M1,
                    tf,
                ),
                "Symbol-less is_bucket_incomplete_at_close must enforce pure UTC boundaries on {tf:?} at {ts_close}"
            );
        }
    }
    // (c) US equities (`"SPY"`): regular close `20:59:59 UTC` on Friday (EST `2024-03-08`) closes `H1`/`D1`/`W1` (`false`);
    //     at extended-hours `21:05:59 UTC` (`5` mins into `21:00` `1h` bar), the NEW `21:00` `H1` bucket stays
    //     `incomplete == true`, while the already-closed `D1`/`W1` regular session monotonically stays closed (`false`, `BUG-010`)!
    let spy_fri_reg_close = fri_mid_utc + 20 * 3_600 + 59 * 60 + 59;
    let spy_fri_ah_2105_close = fri_mid_utc + 21 * 3_600 + 5 * 60 + 59;
    for tf in [Timeframe::H1, Timeframe::D1, Timeframe::W1] {
        assert!(
            !is_bucket_incomplete_at_close_for_symbol(
                "SPY",
                tf.bucket_start_timestamp(spy_fri_reg_close),
                spy_fri_reg_close,
                Timeframe::M1,
                tf,
            ),
            "SPY regular close 20:59:59 UTC on Friday must close {tf:?}"
        );
    }
    assert!(
        is_bucket_incomplete_at_close_for_symbol(
            "SPY",
            Timeframe::H1.bucket_start_timestamp(spy_fri_ah_2105_close),
            spy_fri_ah_2105_close,
            Timeframe::M1,
            Timeframe::H1,
        ),
        "SPY extended-hours 21:05:59 UTC must remain incomplete on new 21:00 H1 bucket"
    );
    for tf in [Timeframe::D1, Timeframe::W1] {
        assert!(
            !is_bucket_incomplete_at_close_for_symbol(
                "SPY",
                tf.bucket_start_timestamp(spy_fri_ah_2105_close),
                spy_fri_ah_2105_close,
                Timeframe::M1,
                tf,
            ),
            "SPY {tf:?} must stay monotonically closed after 20:59:59 UTC (BUG-010)"
        );
    }

    // 3. `[Major-03]` `CustomSignalRules` in `LongOnly` with `long_exit.enabled = false` & `short_entry.enabled = true`
    //    must NEVER close Long positions via `short_entry` (`SignalExit`), whereas `LongShort` reverses via `SignalReversal`:
    let mut bars = Vec::with_capacity(95);
    let t0 = 1_710_000_000_i64;
    for i in 0..95_usize {
        let wave = ((i as f64) * 0.45).sin() * 2.8;
        let p = 100.0 + (i as f64) * 0.35 + wave;
        let vol = if i == 56 || i == 68 || i == 80 {
            6_000.0
        } else {
            1_000.0
        };
        bars.push(mk_candle(
            t0 + (i as i64) * 86_400,
            p - 0.4,
            p + 1.2,
            p - 1.1,
            p,
            vol,
        ));
    }
    let spike_clause = SignalClause {
        catalog_id: "volume.spike_2x".to_owned(),
        predicate: SignalPredicate::BoolIs(true),
        direction_filter: None,
    };
    let isolated_spec = CustomSignalRuleSpec {
        long_entry: DirectionalRuleLeg {
            enabled: true,
            inter_group_op: LogicalCombinator::AnyOr,
            groups: vec![SignalClauseGroup {
                intra_op: LogicalCombinator::AllAnd,
                clauses: vec![spike_clause.clone()],
            }],
            confluence_gate: ConfluenceThresholdGate::default(),
        },
        long_exit: DirectionalRuleLeg {
            enabled: false,
            ..DirectionalRuleLeg::default()
        },
        short_entry: DirectionalRuleLeg {
            enabled: true,
            inter_group_op: LogicalCombinator::AnyOr,
            groups: vec![SignalClauseGroup {
                intra_op: LogicalCombinator::AllAnd,
                clauses: vec![spike_clause],
            }],
            confluence_gate: ConfluenceThresholdGate::default(),
        },
        short_exit: DirectionalRuleLeg {
            enabled: false,
            ..DirectionalRuleLeg::default()
        },
        ..CustomSignalRuleSpec::default()
    };

    let cfg_long_only = BacktestConfig {
        preset: StrategyPresetKind::CustomSignalRules,
        direction_mode: StrategyDirectionMode::LongOnly,
        risk: StopTakeConfig {
            stop_loss_pct: None,
            take_profit_pct: None,
            trailing_stop_atr_mult: None,
            max_holding_bars: None,
            min_holding_bars: 0,
            ..StopTakeConfig::default()
        },
        custom_rules: isolated_spec.clone(),
        ..BacktestConfig::default()
    };
    let bt_long_only = run_backtest("BTCUSDT", Timeframe::D1, &bars, &cfg_long_only, None);
    assert!(
        bt_long_only
            .trades
            .iter()
            .all(|t| t.exit_reason != BacktestExitReason::SignalExit),
        "LongOnly with long_exit.enabled=false must NEVER close Long via short_entry!"
    );
    assert!(bt_long_only.open_position.is_some());

    let mut sep_short_spec = isolated_spec;
    bars[70].close = bars[69].close - 4.5;
    bars[70].open = bars[69].close;
    bars[70].low = bars[70].close - 0.5;
    sep_short_spec.short_entry.groups[0].clauses[0] = SignalClause {
        catalog_id: "flow.cvd_ohlcv_impulse".to_owned(),
        predicate: SignalPredicate::FloatCmp {
            op: FloatCmpOp::Lte,
            threshold: -0.20,
        },
        direction_filter: None,
    };
    let cfg_long_short = BacktestConfig {
        direction_mode: StrategyDirectionMode::LongShort,
        custom_rules: sep_short_spec,
        ..cfg_long_only
    };
    let bt_long_short = run_backtest("BTCUSDT", Timeframe::D1, &bars, &cfg_long_short, None);
    assert!(
        bt_long_short
            .trades
            .iter()
            .any(|t| t.exit_reason == BacktestExitReason::SignalReversal),
        "LongShort mode must still allow opposite-side entry reversal"
    );

    // 4. `[Major-04]` `evaluate_signal_clause_at_with_profile` & `evaluate_rule_leg_at` honor `TrendFocused`
    //    on `"health.technical_composite"` clauses even when `confluence_gate.enabled == false`:
    let trend_bars: Vec<Candle> = (0..80_usize)
        .map(|i| {
            let p = 100.0 + (i as f64) * 1.25;
            mk_candle(
                t0 + (i as i64) * 86_400,
                p - 0.2,
                p + 0.3,
                p - 0.9,
                p,
                1_000.0,
            )
        })
        .collect();
    let eval_idx = 75_usize;
    let rep_bal = evaluate_causal_confluence(
        &trend_bars[..=eval_idx],
        None,
        false,
        ConfluenceWeightProfile::Balanced,
    );
    let rep_tf = evaluate_causal_confluence(
        &trend_bars[..=eval_idx],
        None,
        false,
        ConfluenceWeightProfile::TrendFocused,
    );
    let s_bal = rep_bal.health.unwrap().composite_score;
    let s_tf = rep_tf.health.unwrap().composite_score;
    assert!(
        (s_tf - s_bal).abs() > 1.0,
        "TrendFocused ({s_tf}) and Balanced ({s_bal}) composite scores must diverge on pure trend"
    );
    let mid_thr = 0.5 * (s_bal + s_tf);
    let health_clause = SignalClause {
        catalog_id: "health.technical_composite".to_owned(),
        predicate: SignalPredicate::FloatCmp {
            op: FloatCmpOp::Gte,
            threshold: mid_thr,
        },
        direction_filter: None,
    };
    let mat = PrecomputedCatalogMatrix::compute(&trend_bars);
    let match_bal = evaluate_signal_clause_at_with_profile(
        &health_clause,
        &mat,
        eval_idx,
        ConfluenceWeightProfile::Balanced,
    );
    let match_tf = evaluate_signal_clause_at_with_profile(
        &health_clause,
        &mat,
        eval_idx,
        ConfluenceWeightProfile::TrendFocused,
    );
    assert_ne!(
        match_bal, match_tf,
        "health.technical_composite clause must distinguish TrendFocused vs Balanced around midpoint {mid_thr}"
    );
}

#[test]
fn r17_multistrategy_pct_utc_alignment_and_intraday_vs_daily_benchmark_causality() {
    use crate::backtest::{ComparisonCurveKind, run_multi_strategy_comparison};
    use crate::data::{DataSourcePreset, WatchlistFeedEntry, load_bundled_watchlist_universe};

    let feeds = load_bundled_watchlist_universe();
    let btc = feeds
        .iter()
        .find(|f| f.symbol == "BTCUSDT")
        .expect("BTCUSDT feed");
    let cfg = BacktestConfig::default();

    let rep = run_multi_strategy_comparison(
        "BTCUSDT",
        DataSourcePreset::BtcUsdt15mJson,
        Timeframe::M15,
        &btc.candles,
        &cfg,
        &feeds,
        None,
        false,
    );
    assert_eq!(rep.curves.len(), 9, "7 presets + B&H + SPY benchmark");
    let bh = rep
        .curves
        .iter()
        .find(|c| c.kind == ComparisonCurveKind::AssetBuyAndHold)
        .expect("B&H curve");
    assert!((bh.return_pct_values[0].unwrap() - 0.0).abs() < 1e-12);

    // Strict DEF-001 cross-timeframe test: intraday 15m VN30F1M bars on Day 2 morning (`02:00 UTC`)
    // MUST NOT leak Day 2's completed D1 VNINDEX close (`23:59:59 UTC`), using Day 1's close (`0.00%`) instead!
    let day1 = 1_760_000_000_i64.div_euclid(86_400) * 86_400;
    let day2 = day1 + 86_400;
    let vnindex_d1 = vec![
        Candle {
            timestamp: day1,
            date_label: "D1".to_owned(),
            open: 1200.0,
            high: 1210.0,
            low: 1195.0,
            close: 1200.0,
            volume: 1_000_000.0,
        },
        Candle {
            timestamp: day2,
            date_label: "D2".to_owned(),
            open: 1200.0,
            high: 1330.0,
            low: 1200.0,
            close: 1320.0, // +10.0% at end of Day 2!
            volume: 1_000_000.0,
        },
    ];
    let custom_universe = vec![WatchlistFeedEntry {
        instrument_id: "BM:VNINDEX".to_owned(),
        preset: DataSourcePreset::CafeFIndexCsv,
        symbol: "VNINDEX".to_owned(),
        base_timeframe: Timeframe::D1,
        candles: vnindex_d1,
    }];
    let intraday_15m = vec![
        Candle {
            timestamp: day2 + 2 * 3_600, // Day 2 02:00 UTC (morning)
            date_label: "02:00".to_owned(),
            open: 1250.0,
            high: 1255.0,
            low: 1248.0,
            close: 1252.0,
            volume: 5_000.0,
        },
        Candle {
            timestamp: day2 + 86_400 + 2 * 3_600, // Day 3 02:00 UTC (after Day 2 D1 closed!)
            date_label: "D3 02:00".to_owned(),
            open: 1252.0,
            high: 1260.0,
            low: 1250.0,
            close: 1258.0,
            volume: 5_000.0,
        },
    ];
    let ms_intra = run_multi_strategy_comparison(
        "VN30F1M",
        DataSourcePreset::IntradayVn30F1m,
        Timeframe::M15,
        &intraday_15m,
        &cfg,
        &custom_universe,
        None,
        false,
    );
    let bm_curve = ms_intra
        .curves
        .iter()
        .find(|c| c.kind == ComparisonCurveKind::MarketBenchmark)
        .expect("VNINDEX benchmark curve");
    assert!(
        (bm_curve.return_pct_values[0].unwrap() - 0.0).abs() < 1e-9,
        "Day 2 morning 15m bar must use Day 1 completed D1 close (0.0%), NOT Day 2 EOD close (+10.0%)"
    );
    assert!(
        (bm_curve.return_pct_values[1].unwrap() - 10.0).abs() < 1e-9,
        "Day 3 morning 15m bar sees Day 2 completed D1 close (+10.0%)"
    );
}

#[test]
fn r17_outlier_impact_drawdown_periods_signal_bar_regime_multiphase_and_manifest() {
    use crate::backtest::{
        BacktestExitReason, MultiPhaseSplitMode, RunManifest,
        compute_entry_regime_breakdown_with_precomputed, compute_outlier_impact,
        compute_yearly_breakdown, extract_drawdown_periods, run_multi_phase_batch,
    };
    use crate::data::load_bundled_watchlist_universe;
    use crate::scanner::MarketRegime;
    use crate::trading::PositionSide;

    let mk_trade =
        |id: u64, entry_bar: usize, exit_bar: usize, exit_ts: i64, pnl: f64| BacktestTrade {
            trade_id: id,
            side: PositionSide::Long,
            quantity: 10.0,
            entry_bar_index: entry_bar,
            exit_bar_index: exit_bar,
            entry_timestamp: exit_ts - 86_400,
            exit_timestamp: exit_ts,
            entry_price: 100.0,
            exit_price: 100.0 + pnl / 10.0,
            bars_held: exit_bar.saturating_sub(entry_bar).max(1),
            gross_pnl: pnl,
            entry_fee: 0.0,
            exit_fee: 0.0,
            exit_tax: 0.0,
            net_pnl: pnl,
            return_pct: pnl / 10.0,
            r_multiple: Some(pnl / 100.0),
            mae_pct: -1.0,
            mfe_pct: 2.0,
            exit_reason: BacktestExitReason::SignalExit,
        };

    // 1. `N < 3` guard on `trimmed_expectancy_10pct` + exact `gross_profit` denominator (`analytics_service.py:421` fix)
    assert_eq!(compute_outlier_impact(&[]).trimmed_expectancy_10pct, None);
    let two_trades = vec![
        mk_trade(1, 1, 2, 1_700_000_000, 200.0),
        mk_trade(2, 3, 4, 1_700_100_000, -100.0),
    ];
    let oi_two = compute_outlier_impact(&two_trades);
    assert_eq!(oi_two.trimmed_expectancy_10pct, None);
    assert!(
        (oi_two.top_3_winners_share_of_gross_profit_pct.unwrap() - 100.0).abs() < 1e-9,
        "200 / 200 gross_profit = 100.0% (NOT 200 / 300 gross_movement = 66.7%!)"
    );

    let five_trades = vec![
        mk_trade(1, 1, 2, 1_704_153_600, 500.0),   // 2024
        mk_trade(2, 3, 4, 1_704_240_000, 100.0),   // 2024
        mk_trade(3, 5, 6, 1_735_776_000, 50.0),    // 2025
        mk_trade(4, 7, 8, 1_735_862_400, -200.0),  // 2025
        mk_trade(5, 9, 10, 1_735_948_800, -250.0), // 2025
    ];
    let oi5 = compute_outlier_impact(&five_trades);
    // gross_profit = 650, top_3_winners = 650 -> 100%; net = +200, ex_top_3 = -450 -> outlier_dependent_warning = true!
    assert!((oi5.gross_profit - 650.0).abs() < 1e-9);
    assert!((oi5.top_3_winners_share_of_gross_profit_pct.unwrap() - 100.0).abs() < 1e-9);
    assert!(oi5.outlier_dependent_warning);
    // sorted = [-250, -200, 50, 100, 500], k = 1 -> mid = [-200, 50, 100] -> mean = -50/3
    assert!((oi5.trimmed_expectancy_10pct.unwrap() - (-50.0 / 3.0)).abs() < 1e-9);

    // 2. Discrete `DrawdownPeriod`s: 1 recovered + 1 unrecovered tail episode (`recovery_timestamp == None`)
    let eq_curve = vec![
        BacktestEquityPoint {
            bar_index: 0,
            timestamp: 1_704_067_200,
            equity: 100_000.0,
            cash: 100_000.0,
            drawdown_abs: 0.0,
            underwater_pct: 0.0,
            position_qty: 0.0,
        },
        BacktestEquityPoint {
            bar_index: 1,
            timestamp: 1_704_153_600,
            equity: 95_000.0,
            cash: 95_000.0,
            drawdown_abs: 5_000.0,
            underwater_pct: -5.0,
            position_qty: 0.0,
        },
        BacktestEquityPoint {
            bar_index: 2,
            timestamp: 1_704_240_000,
            equity: 102_000.0,
            cash: 102_000.0,
            drawdown_abs: 0.0,
            underwater_pct: 0.0,
            position_qty: 0.0,
        },
        BacktestEquityPoint {
            bar_index: 3,
            timestamp: 1_735_689_600,
            equity: 91_800.0,
            cash: 91_800.0,
            drawdown_abs: 10_200.0,
            underwater_pct: -10.0,
            position_qty: 0.0,
        },
    ];
    let dds = extract_drawdown_periods(100_000.0, &eq_curve);
    assert_eq!(dds.len(), 2);
    assert!((dds[0].max_drawdown_pct - 10.0).abs() < 1e-9);
    assert_eq!(
        dds[0].recovery_timestamp, None,
        "tail drawdown must remain unrecovered"
    );
    assert_eq!(dds[1].recovery_timestamp, Some(1_704_240_000));

    // 3. UTC Calendar Year slices (`2024` & `2025`)
    let yrs = compute_yearly_breakdown(100_000.0, &eq_curve, &five_trades);
    assert_eq!(yrs.len(), 2);
    assert_eq!(yrs[0].year, 2024);
    assert_eq!(yrs[0].trade_count, 2);
    assert_eq!(yrs[1].year, 2025);
    assert_eq!(yrs[1].trade_count, 3);

    // 4. Causal Signal-Bar `T = entry_bar_index - 1` regime attribution (`backtest_service.py:583` fix!)
    //    Trade enters at bar 2 (`Open[2]`) and exits at bar 3 (`exit_bar_index = 3`):
    //    - bar 1 (`signal_bar`): `Pullback`
    //    - bar 2 (`entry_bar_index`): `Uptrend`
    //    - bar 3 (`exit_bar_index`): `Downtrend`
    //    Trade MUST be attributed strictly to `Pullback` (`regimes[1]`)!
    let regimes = vec![
        MarketRegime::Neutral,
        MarketRegime::Pullback,
        MarketRegime::Uptrend,
        MarketRegime::Downtrend,
    ];
    let single_t = vec![mk_trade(99, 2, 3, 1_704_240_000, 150.0)];
    let reg_slices = compute_entry_regime_breakdown_with_precomputed(&regimes, &single_t);
    let pb_row = reg_slices
        .iter()
        .find(|r| r.regime == MarketRegime::Pullback)
        .expect("Pullback row");
    let up_row = reg_slices
        .iter()
        .find(|r| r.regime == MarketRegime::Uptrend)
        .expect("Uptrend row");
    let dn_row = reg_slices
        .iter()
        .find(|r| r.regime == MarketRegime::Downtrend)
        .expect("Downtrend row");
    assert_eq!(
        pb_row.trade_count, 1,
        "must attribute to signal bar T = entry_bar - 1"
    );
    assert_eq!(up_row.trade_count, 0, "must NOT peek at Close[entry_bar]");
    assert_eq!(dn_row.trade_count, 0, "must NOT peek at exit_bar_index");

    // 5. `MultiPhaseBatch` & deterministic `RunManifest`
    let feeds = load_bundled_watchlist_universe();
    let btc = feeds.iter().find(|f| f.symbol == "BTCUSDT").unwrap();
    let cfg = BacktestConfig::default();
    let mp = run_multi_phase_batch(
        Timeframe::M15,
        &btc.candles,
        &cfg,
        MultiPhaseSplitMode::Tertiles,
        BacktestRunOptions::default(),
    );
    assert_eq!(mp.phases.len(), 3);

    let bt1 = run_backtest_with_options(
        "BTCUSDT",
        Timeframe::M15,
        &btc.candles,
        &cfg,
        BacktestRunOptions::default(),
    );
    let bt2 = run_backtest_with_options(
        "BTCUSDT",
        Timeframe::M15,
        &btc.candles,
        &cfg,
        BacktestRunOptions::default(),
    );
    let m1 = RunManifest::build(&bt1, BacktestRunOptions::default());
    let m2 = RunManifest::build(&bt2, BacktestRunOptions::default());
    assert_eq!(m1.manifest_digest, m2.manifest_digest);
    let mut cfg_mod = cfg.clone();
    cfg_mod.commission_bps += 1.0;
    let bt3 = run_backtest_with_options(
        "BTCUSDT",
        Timeframe::M15,
        &btc.candles,
        &cfg_mod,
        BacktestRunOptions::default(),
    );
    let m3 = RunManifest::build(&bt3, BacktestRunOptions::default());
    assert_ne!(m1.manifest_digest, m3.manifest_digest);
}

#[test]
fn critic_r17_regression_multistrategy_midbar_cutoff_and_session_close_parity() {
    use crate::data::load_bundled_watchlist_universe;

    let feeds = load_bundled_watchlist_universe();
    let fpt = feeds.iter().find(|f| f.symbol == "FPT").unwrap();
    let cfg = BacktestConfig {
        preset: StrategyPresetKind::DonchianAtrBreakout,
        ..BacktestConfig::default()
    };

    // 1. `[Critical-01a]`: Mid-bar intraday cutoff on coarse `D1` series (`incomplete_tail_bar: false`)
    // MUST withhold the unclosed last `D1` candle in `run_multi_strategy_comparison` (zero lookahead on
    // future EOD High/Low/Close across all 7 presets AND `Asset Buy & Hold`!).
    let mut poisoned = fpt.candles[..80].to_vec();
    let last_open = poisoned[79].timestamp;
    let mid_bar_cutoff = last_open + 2 * 3_600; // 02:00:00 UTC (09:00 ICT — strictly mid-session!)
    poisoned[79].high = 9_999_999.0;
    poisoned[79].close = 9_999_999.0;

    let bt_mid = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        &poisoned,
        &cfg,
        BacktestRunOptions {
            replay_cutoff_timestamp: Some(mid_bar_cutoff),
            incomplete_tail_bar: false,
        },
    );
    assert_eq!(bt_mid.bar_count, 79, "unclosed bar #79 must be excluded");

    let ms_mid = run_multi_strategy_comparison(
        "FPT",
        DataSourcePreset::Vn30Basket,
        Timeframe::D1,
        &poisoned,
        &cfg,
        &feeds,
        Some(mid_bar_cutoff),
        false,
    );
    for curve in &ms_mid.curves {
        assert_eq!(
            curve.return_pct_values[79], None,
            "curve {} leaked unclosed coarse tail bar #79!",
            curve.label
        );
    }
    let active_ms = ms_mid
        .curves
        .iter()
        .find(|c| c.is_active_preset)
        .expect("active preset curve");
    assert_eq!(active_ms.num_trades, bt_mid.tearsheet.num_trades);
    assert!(
        (active_ms.final_return_pct.unwrap_or(0.0) - bt_mid.tearsheet.total_return_pct).abs()
            < 1e-9
    );

    // 2. `[Critical-01b]`: At official Vietnam ATC session close (`07:44:59 UTC` = `14:45 ICT`),
    // `is_bucket_incomplete_at_close_for_symbol` is `false`, AND `run_backtest_with_options`,
    // `run_multi_strategy_comparison`, and `compute_advanced_tearsheet_analytics` ALL keep bar #79
    // with 100% identical trade count, return %, and phase bar coverage!
    let clean_80 = &fpt.candles[..80];
    let atc_close_cutoff = clean_80[79].timestamp + 7 * 3_600 + 44 * 60 + 59; // 07:44:59 UTC
    assert!(!is_bucket_incomplete_at_close_for_symbol(
        "FPT",
        clean_80[79].timestamp,
        atc_close_cutoff,
        Timeframe::M1,
        Timeframe::D1,
    ));
    let opts_atc = BacktestRunOptions {
        replay_cutoff_timestamp: Some(atc_close_cutoff),
        incomplete_tail_bar: false,
    };
    let bt_atc = run_backtest_with_options("FPT", Timeframe::D1, clean_80, &cfg, opts_atc);
    assert_eq!(
        bt_atc.bar_count, 80,
        "completed ATC session bar must NOT be dropped"
    );
    let ms_atc = run_multi_strategy_comparison(
        "FPT",
        DataSourcePreset::Vn30Basket,
        Timeframe::D1,
        clean_80,
        &cfg,
        &feeds,
        Some(atc_close_cutoff),
        false,
    );
    let active_atc = ms_atc.curves.iter().find(|c| c.is_active_preset).unwrap();
    assert!(active_atc.return_pct_values[79].is_some());
    assert_eq!(active_atc.num_trades, bt_atc.tearsheet.num_trades);
    assert!(
        (active_atc.final_return_pct.unwrap_or(0.0) - bt_atc.tearsheet.total_return_pct).abs()
            < 1e-9
    );
    let adv_atc = compute_advanced_tearsheet_analytics(
        clean_80,
        &bt_atc,
        MultiPhaseSplitMode::Halves,
        opts_atc,
    );
    assert_eq!(adv_atc.manifest.bar_count, 80);
    assert_eq!(adv_atc.multi_phase.phases.last().unwrap().end_bar, 80);
}

#[test]
fn r18_b4_pit_vs_retrospective_survivorship_bias_proof_battle_and_portfolio() {
    use crate::backtest::{
        BacktestExitReason, PortfolioUniverseFilter, PortfolioWeightingMode, SweepRankingMetric,
        run_multi_symbol_battle_with_universe, run_pit_episodes_backtest,
        run_portfolio_backtest_with_universe,
    };
    use crate::data::WatchlistFeedEntry;
    use crate::scanner::{SessionDate, UniverseDefinition, UniverseMode};

    // Build 120 daily bars starting at 2025-01-02 (`1_735_776_000` UTC):
    // - Bars `0..70` (`2025-01-02..2025-03-12`): `LATE_WIN` surges +180% BEFORE its PIT inclusion on bar 75 (`2025-03-18`),
    //   then drifts sideways-down after inclusion!
    // - `EARLY_DELIST` rallies then is removed from the PIT universe at bar 65 (`2025-03-07`) while holding a Long position,
    //   then crashes -70% in bars `66..120` AFTER removal!
    let t0 = 1_735_776_000_i64; // 2025-01-02 00:00:00 UTC
    let mut late_candles = Vec::with_capacity(120);
    let mut delist_candles = Vec::with_capacity(120);
    for i in 0..120_usize {
        let ts = t0 + (i as i64) * 86_400;
        let late_px = if i < 20 {
            100.0 - (i as f64) * 0.2 // dip so SMA(10) crosses above EMA(20) around bar 24!
        } else if i < 72 {
            96.0 + ((i - 20) as f64) * 3.4 // massive pre-inclusion rally!
        } else {
            272.0 - ((i - 72) as f64) * 0.15 // flat/slight drift after PIT inclusion!
        };
        late_candles.push(Candle {
            timestamp: ts,
            date_label: format!("d{i}"),
            open: late_px - 0.4,
            high: late_px + 1.0,
            low: late_px - 0.9,
            close: late_px,
            volume: 50_000.0,
        });

        let del_px = if i < 18 {
            100.0 - (i as f64) * 0.2
        } else if i <= 65 {
            96.4 + ((i - 18) as f64) * 1.1 // uptrend while active -> holds open Long at bar 65!
        } else {
            (148.0 - ((i - 65) as f64) * 2.1).max(15.0) // post-removal collapse!
        };
        delist_candles.push(Candle {
            timestamp: ts,
            date_label: format!("d{i}"),
            open: del_px - 0.3,
            high: del_px + 0.8,
            low: del_px - 0.8,
            close: del_px,
            volume: 40_000.0,
        });
    }

    let d_inc = SessionDate::from_utc_timestamp(late_candles[75].timestamp).to_iso_string();
    let d_rem = SessionDate::from_utc_timestamp(delist_candles[65].timestamp).to_iso_string();
    let d_u_from = SessionDate::from_utc_timestamp(late_candles[10].timestamp).to_iso_string();

    let u_json = format!(
        r#"{{
          "universe_id": "SURVIVOR-PROOF",
          "version": "v1",
          "name": "Survivorship Bias Proof Universe",
          "status": "APPROVED",
          "default_mode": "POINT_IN_TIME",
          "effective_from": "{d_u_from}",
          "members": [
            {{
              "symbol": "LATE_WIN",
              "exchange": "HOSE",
              "effective_from": "{d_inc}",
              "status": "ACTIVE"
            }},
            {{
              "symbol": "EARLY_DELIST",
              "exchange": "HOSE",
              "effective_from": "{d_u_from}",
              "effective_to": "{d_rem}",
              "status": "ACTIVE"
            }}
          ]
        }}"#
    );
    let u_def = UniverseDefinition::from_auto_str(&u_json).expect("valid proof universe");

    let mut cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        commission_bps: 5.0,
        slippage_bps: 2.0,
        ..BacktestConfig::default()
    };
    cfg.params.fast_sma = 8;
    cfg.params.slow_ema = 21;
    cfg.risk.take_profit_pct = None;
    cfg.risk.stop_loss_pct = None;

    // 1. Single-symbol `EARLY_DELIST` in `PointInTime` MUST settle its open Long on bar 65 via `UniverseRemoval`!
    let delist_pit = run_pit_episodes_backtest(
        "EARLY_DELIST",
        Timeframe::D1,
        &delist_candles,
        &cfg,
        BacktestRunOptions::default(),
        &u_def,
        Some(UniverseMode::PointInTime),
    );
    assert!(
        delist_pit
            .trades
            .iter()
            .any(|t| t.exit_reason == BacktestExitReason::UniverseRemoval && t.exit_bar_index == 65),
        "open position at PIT removal boundary must close with UniverseRemoval at bar 65: {:?}",
        delist_pit.trades
    );
    assert!(delist_pit.open_position.is_none());

    // 2. `LATE_WIN`: `RetrospectiveFixed` captures the pre-inclusion +180% surge (`total_return_pct > 80%`),
    //    whereas `PointInTime` strictly excludes bars `0..75` from trading (`total_return_pct <= 5%`)!
    let late_retro = run_pit_episodes_backtest(
        "LATE_WIN",
        Timeframe::D1,
        &late_candles,
        &cfg,
        BacktestRunOptions::default(),
        &u_def,
        Some(UniverseMode::RetrospectiveFixed),
    );
    let late_pit = run_pit_episodes_backtest(
        "LATE_WIN",
        Timeframe::D1,
        &late_candles,
        &cfg,
        BacktestRunOptions::default(),
        &u_def,
        Some(UniverseMode::PointInTime),
    );
    assert!(
        late_retro.tearsheet.total_return_pct > late_pit.tearsheet.total_return_pct + 50.0,
        "RetrospectiveFixed ({:.2}%) must show inflated survivorship-bias return vs PIT ({:.2}%)",
        late_retro.tearsheet.total_return_pct,
        late_pit.tearsheet.total_return_pct
    );

    // 3. Portfolio & Battle:
    //    - Pre-effective bars `0..10` (`< d_u_from`) hold 100% cash (`equity == initial_capital`, zero NaN)!
    //    - `PointInTime` marks `is_point_in_time == true` & `survivor_bias_caveat == None`;
    //      `RetrospectiveFixed` marks `is_point_in_time == false` & `survivor_bias_caveat.is_some()`.
    let feeds = vec![
        WatchlistFeedEntry {
            instrument_id: "Test:LATE_WIN".to_owned(),
            preset: DataSourcePreset::Vn30Basket,
            symbol: "LATE_WIN".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: late_candles,
        },
        WatchlistFeedEntry {
            instrument_id: "Test:EARLY_DELIST".to_owned(),
            preset: DataSourcePreset::Vn30Basket,
            symbol: "EARLY_DELIST".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: delist_candles,
        },
    ];
    let port_pit = run_portfolio_backtest_with_universe(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::EqualWeight,
        None,
        Some(&u_def),
        Some(UniverseMode::PointInTime),
    );
    let port_retro = run_portfolio_backtest_with_universe(
        &feeds,
        Timeframe::D1,
        &cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::EqualWeight,
        None,
        Some(&u_def),
        Some(UniverseMode::RetrospectiveFixed),
    );
    assert!(port_pit.is_point_in_time && port_pit.survivor_bias_caveat.is_none());
    assert!(!port_retro.is_point_in_time && port_retro.survivor_bias_caveat.is_some());
    for pt in &port_pit.portfolio_equity_curve[..10] {
        assert!(
            (pt.equity - cfg.initial_cash).abs() < 1e-9,
            "pre-effective PIT portfolio bar must hold 100% cash without NaN, got {}",
            pt.equity
        );
    }
    assert_ne!(
        port_pit.portfolio_tearsheet.total_return_pct,
        port_retro.portfolio_tearsheet.total_return_pct
    );

    let battle_pit = run_multi_symbol_battle_with_universe(
        &feeds,
        Timeframe::D1,
        &cfg,
        SweepRankingMetric::TotalReturnPct,
        None,
        Some(&u_def),
        Some(UniverseMode::PointInTime),
    );
    let battle_retro = run_multi_symbol_battle_with_universe(
        &feeds,
        Timeframe::D1,
        &cfg,
        SweepRankingMetric::TotalReturnPct,
        None,
        Some(&u_def),
        Some(UniverseMode::RetrospectiveFixed),
    );
    assert!(battle_pit.is_point_in_time && battle_pit.survivor_bias_caveat.is_none());
    assert!(!battle_retro.is_point_in_time && battle_retro.survivor_bias_caveat.is_some());
    assert_ne!(
        battle_pit.average_return_pct,
        battle_retro.average_return_pct
    );
}

#[test]
fn critic_r18_regression_pit_episodes_alignment_inverse_vol_newcomer_and_rvol_clause() {
    use crate::backtest::strategy::{FloatCmpOp, SignalClause, SignalClauseGroup, SignalPredicate};
    use crate::data::WatchlistFeedEntry;
    use crate::scanner::SignalDirection;
    use crate::scanner::universe::{
        MemberStatus, SessionDate, UniverseDefinition, UniverseMember, UniverseMode, UniverseStatus,
    };

    let base_ts = 1_704_153_600_i64; // 2024-01-02 UTC
    let d_of = |idx: usize| SessionDate::from_utc_timestamp(base_ts + (idx as i64) * 86_400);

    // 1. `[Major-02]` Disjoint 2-episode PIT constituent (`[10..40)` and `[85..120)`):
    //    `equity_curve.len() == equity_values.len() == candles.len() == 120`, exact `pt.bar_index == idx`,
    //    and `unexecuted_signals[..].signal_bar_index` uses absolute candle indices!
    let mut multi_ep_candles: Vec<Candle> = (0..120)
        .map(|i| {
            let px = 100.0 + (i as f64) * 0.4 + ((i as f64) * 0.35).sin() * 2.0;
            Candle {
                timestamp: base_ts + (i as i64) * 86_400,
                date_label: format!("d{i}"),
                open: px - 0.2,
                high: px + 0.8,
                low: px - 0.8,
                close: px + 0.3,
                volume: 50_000.0,
            }
        })
        .collect();
    // Force an invalid open price on bar 96 inside the second episode so any entry signal at bar 95
    // records an absolute `signal_bar_index == 95` (NOT episode-local `95 - 85 = 10`!).
    multi_ep_candles[96].open = -1.0;

    let u_multi = UniverseDefinition {
        universe_id: "MULTI_EP".to_owned(),
        version: "v1".to_owned(),
        name: "Multi Episode PIT".to_owned(),
        status: UniverseStatus::Approved,
        default_mode: UniverseMode::PointInTime,
        effective_from: None,
        effective_to: None,
        selection_purpose: "test".to_owned(),
        source_evidence: "unit".to_owned(),
        members: vec![
            UniverseMember {
                symbol: "REENTRY".to_owned(),
                company_name: None,
                exchange: "HOSE".to_owned(),
                effective_from: Some(d_of(10)),
                effective_to: Some(d_of(39)),
                status: MemberStatus::Active,
                sector: None,
                inclusion_reason: "ep1".to_owned(),
                weight: 1.0,
            },
            UniverseMember {
                symbol: "REENTRY".to_owned(),
                company_name: None,
                exchange: "HOSE".to_owned(),
                effective_from: Some(d_of(85)),
                effective_to: None,
                status: MemberStatus::Active,
                sector: None,
                inclusion_reason: "ep2".to_owned(),
                weight: 1.0,
            },
        ],
    };

    let mut rule_cfg = BacktestConfig {
        preset: StrategyPresetKind::CustomSignalRules,
        ..BacktestConfig::default()
    };
    rule_cfg.custom_rules.long_entry.groups = vec![SignalClauseGroup {
        clauses: vec![SignalClause {
            catalog_id: "structure.candle_score".to_owned(),
            predicate: SignalPredicate::FloatCmp {
                op: FloatCmpOp::Gte,
                threshold: -100.0,
            },
            direction_filter: None,
        }],
        ..SignalClauseGroup::default()
    }];
    rule_cfg.risk.take_profit_pct = None;
    rule_cfg.risk.stop_loss_pct = None;

    let ep_res = run_pit_episodes_backtest(
        "REENTRY",
        Timeframe::D1,
        &multi_ep_candles,
        &rule_cfg,
        BacktestRunOptions::default(),
        &u_multi,
        Some(UniverseMode::PointInTime),
    );
    assert_eq!(ep_res.equity_curve.len(), multi_ep_candles.len());
    assert_eq!(ep_res.equity_values.len(), multi_ep_candles.len());
    assert_eq!(ep_res.underwater_values.len(), multi_ep_candles.len());
    for (idx, pt) in ep_res.equity_curve.iter().enumerate() {
        assert_eq!(pt.bar_index, idx);
    }
    // Inactive gap bars `40..85` carry constant settled cash with `0.0` exposure!
    let settled_at_39 = ep_res.equity_curve[39].equity;
    for pt in &ep_res.equity_curve[40..85] {
        assert!((pt.equity - settled_at_39).abs() < 1e-9);
        assert_eq!(pt.position_qty, 0.0);
    }

    // Also verify `unexecuted_signals` in episode 2 (`[85..120)`) uses absolute `signal_bar_index >= 85`!
    let mut unexec_candles = multi_ep_candles.clone();
    for b in &mut unexec_candles[86..120] {
        b.open = -1.0;
    }
    let ep_unexec = run_pit_episodes_backtest(
        "REENTRY",
        Timeframe::D1,
        &unexec_candles,
        &rule_cfg,
        BacktestRunOptions::default(),
        &u_multi,
        Some(UniverseMode::PointInTime),
    );
    assert!(
        ep_unexec
            .unexecuted_signals
            .iter()
            .any(|u| u.signal_bar_index >= 85),
        "episode-2 unexecuted signals must carry absolute bar indices >= 85: {:?}",
        ep_unexec.unexecuted_signals
    );
    assert!(
        ep_unexec
            .unexecuted_signals
            .iter()
            .all(|u| !(40..85).contains(&u.signal_bar_index))
    );

    // 2. `[Major-01]` `CausalInverseVolatility` with a late PIT entrant sitting in cash (`0` strategy trades):
    //    Seasoned `LOW_VOL` (~0.4% daily) MUST receive higher weight than `HIGH_VOL` (~4.5% daily),
    //    and late newcomer `NEW_CASH` (~2.0% daily price vol, 0 trades) MUST NOT reset seasoned weights
    //    to `33.33%` NOR receive a `50x` (`> 50%`) flat-equity spike!
    let low_vol_bars: Vec<Candle> = (0..90)
        .map(|i| {
            let p: f64 = 100.0 * (1.0 + 0.004 * if i % 2 == 0 { 1.0 } else { -0.95 });
            Candle {
                timestamp: base_ts + (i as i64) * 86_400,
                date_label: format!("d{i}"),
                open: 100.0,
                high: p.max(100.0) + 0.1,
                low: p.min(100.0) - 0.1,
                close: p,
                volume: 20_000.0,
            }
        })
        .collect();
    let high_vol_bars: Vec<Candle> = (0..90)
        .map(|i| {
            let p: f64 = 100.0 * (1.0 + 0.045 * if i % 2 == 0 { 1.0 } else { -0.95 });
            Candle {
                timestamp: base_ts + (i as i64) * 86_400,
                date_label: format!("d{i}"),
                open: 100.0,
                high: p.max(100.0) + 0.5,
                low: p.min(100.0) - 0.5,
                close: p,
                volume: 20_000.0,
            }
        })
        .collect();
    let newcomer_bars: Vec<Candle> = (0..90)
        .map(|i| {
            let p: f64 = 100.0 * (1.0 + 0.020 * if i % 2 == 0 { 1.0 } else { -0.95 });
            Candle {
                timestamp: base_ts + (i as i64) * 86_400,
                date_label: format!("d{i}"),
                open: 100.0,
                high: p.max(100.0) + 0.2,
                low: p.min(100.0) - 0.2,
                close: p,
                volume: 20_000.0,
            }
        })
        .collect();

    let u_iv = UniverseDefinition {
        universe_id: "IV_PIT".to_owned(),
        version: "v1".to_owned(),
        name: "InvVol PIT".to_owned(),
        status: UniverseStatus::Approved,
        default_mode: UniverseMode::PointInTime,
        effective_from: None,
        effective_to: None,
        selection_purpose: "test".to_owned(),
        source_evidence: "unit".to_owned(),
        members: vec![
            UniverseMember {
                symbol: "LOW_VOL".to_owned(),
                company_name: None,
                exchange: "HOSE".to_owned(),
                effective_from: None,
                effective_to: None,
                status: MemberStatus::Active,
                sector: None,
                inclusion_reason: "core".to_owned(),
                weight: 1.0,
            },
            UniverseMember {
                symbol: "HIGH_VOL".to_owned(),
                company_name: None,
                exchange: "HOSE".to_owned(),
                effective_from: None,
                effective_to: None,
                status: MemberStatus::Active,
                sector: None,
                inclusion_reason: "core".to_owned(),
                weight: 1.0,
            },
            UniverseMember {
                symbol: "NEW_CASH".to_owned(),
                company_name: None,
                exchange: "HOSE".to_owned(),
                effective_from: Some(d_of(87)), // Joins 3 bars before end!
                effective_to: None,
                status: MemberStatus::Active,
                sector: None,
                inclusion_reason: "late_add".to_owned(),
                weight: 1.0,
            },
        ],
    };

    // Strategy that never trades so all 3 constituents sit in flat cash (`equity == 100_000`)!
    let mut no_trade_cfg = BacktestConfig {
        preset: StrategyPresetKind::CustomSignalRules,
        ..BacktestConfig::default()
    };
    no_trade_cfg.custom_rules.long_entry.groups = vec![SignalClauseGroup {
        clauses: vec![SignalClause {
            catalog_id: "structure.candle_score".to_owned(),
            predicate: SignalPredicate::FloatCmp {
                op: FloatCmpOp::Gte,
                threshold: 999.0,
            },
            direction_filter: None,
        }],
        ..SignalClauseGroup::default()
    }];

    let iv_feeds = vec![
        WatchlistFeedEntry {
            instrument_id: "T:LOW_VOL".to_owned(),
            preset: DataSourcePreset::Vn30Basket,
            symbol: "LOW_VOL".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: low_vol_bars,
        },
        WatchlistFeedEntry {
            instrument_id: "T:HIGH_VOL".to_owned(),
            preset: DataSourcePreset::Vn30Basket,
            symbol: "HIGH_VOL".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: high_vol_bars,
        },
        WatchlistFeedEntry {
            instrument_id: "T:NEW_CASH".to_owned(),
            preset: DataSourcePreset::Vn30Basket,
            symbol: "NEW_CASH".to_owned(),
            base_timeframe: Timeframe::D1,
            candles: newcomer_bars,
        },
    ];
    let iv_rep = run_portfolio_backtest_with_universe(
        &iv_feeds,
        Timeframe::D1,
        &no_trade_cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::CausalInverseVolatility { lookback_bars: 20 },
        None,
        Some(&u_iv),
        Some(UniverseMode::PointInTime),
    );
    let w_low = iv_rep
        .constituents
        .iter()
        .find(|c| c.symbol == "LOW_VOL")
        .unwrap()
        .final_weight_pct;
    let w_new = iv_rep
        .constituents
        .iter()
        .find(|c| c.symbol == "NEW_CASH")
        .unwrap()
        .final_weight_pct;
    let w_high = iv_rep
        .constituents
        .iter()
        .find(|c| c.symbol == "HIGH_VOL")
        .unwrap()
        .final_weight_pct;
    assert!(
        w_low > w_new && w_new > w_high,
        "CausalInverseVolatility must rank LOW_VOL ({w_low:.2}%) > NEW_CASH ({w_new:.2}%) > HIGH_VOL ({w_high:.2}%)"
    );
    assert!(
        w_new < 45.0,
        "late newcomer in cash must not spike > 45%: got {w_new:.2}%"
    );

    // 3. `[Minor-03]` `SignalClause::default_for_catalog("volume.relative_volume", Bearish)` -> `Gte 2.0`!
    let rvol_bear =
        SignalClause::default_for_catalog("volume.relative_volume", Some(SignalDirection::Bearish));
    assert_eq!(
        rvol_bear.predicate,
        SignalPredicate::FloatCmp {
            op: FloatCmpOp::Gte,
            threshold: 2.0,
        }
    );
}

#[test]
fn wave3_regression_monotonic_cutoff_no_vn_short_outlier_yearly_breadth_pit_and_catalog() {
    use crate::data::{WatchlistFeedEntry, days_from_civil};
    use crate::journal::{
        ExchangeVenue, JournalEntry, JournalEntrySource, JournalEntryStatus, SetupTag,
        TradeJournalStore,
    };
    use crate::scanner::{
        BreadthFeedInput, MemberStatus, SIGNAL_CATALOG, SessionDate, UniverseDefinition,
        UniverseMember, UniverseMode, UniverseStatus, compute_market_breadth_and_rrg,
    };
    use crate::trading::PositionSide;

    // 1. `BUG-010`: Monotonic VN/US session close cutoff + EDT/EST + non-whitelist US tickers.
    let fri_nov06_day = days_from_civil(2026, 11, 6);
    let fri_nov06_ts = fri_nov06_day * 86_400;
    let fpt_bars: Vec<Candle> = (0..10_i64)
        .map(|k| {
            let ts = (fri_nov06_day - 9 + k) * 86_400;
            mk_candle(
                ts,
                100.0 + k as f64,
                102.0 + k as f64,
                99.0 + k as f64,
                101.0 + k as f64,
                1_000.0,
            )
        })
        .collect();
    let cfg_def = BacktestConfig::default();
    let cut_075959 = fri_nov06_ts + 7 * 3_600 + 59 * 60 + 59;
    let cut_080000 = fri_nov06_ts + 8 * 3_600;
    let b_before = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        &fpt_bars,
        &cfg_def,
        BacktestRunOptions {
            replay_cutoff_timestamp: Some(cut_075959),
            incomplete_tail_bar: false,
        },
    )
    .bar_count;
    let b_after = run_backtest_with_options(
        "FPT",
        Timeframe::D1,
        &fpt_bars,
        &cfg_def,
        BacktestRunOptions {
            replay_cutoff_timestamp: Some(cut_080000),
            incomplete_tail_bar: false,
        },
    )
    .bar_count;
    assert_eq!(b_before, 10);
    assert_eq!(
        b_after, 10,
        "BUG-010: advancing 07:59:59 -> 08:00:00 UTC must NOT drop FPT D1 bar"
    );

    // US EDT (summer 2024-06-14) closes at 19:59:59 UTC and stays closed at 20:00:00 UTC:
    let jun14_day = days_from_civil(2024, 6, 14);
    let jun14_mid = jun14_day * 86_400;
    assert!(!is_bucket_incomplete_at_close_for_symbol(
        "GOOG",
        jun14_mid,
        jun14_mid + 19 * 3_600 + 59 * 60 + 59,
        Timeframe::M1,
        Timeframe::D1,
    ));
    assert!(!is_bucket_incomplete_at_close_for_symbol(
        "GOOG",
        jun14_mid,
        jun14_mid + 20 * 3_600,
        Timeframe::M1,
        Timeframe::D1,
    ));
    // Non-whitelist US tickers map to UsEquities (never HOSE) and stay incomplete at 14:30 UTC:
    for us_sym in ["GOOG", "AMZN", "META", "CRWD", "BRK.B"] {
        assert_eq!(
            ExchangeVenue::infer_from_symbol_and_preset(us_sym, DataSourcePreset::CustomFile),
            ExchangeVenue::UsEquities
        );
        assert!(is_bucket_incomplete_at_close_for_symbol(
            us_sym,
            jun14_mid,
            jun14_mid + 14 * 3_600 + 30 * 60,
            Timeframe::M1,
            Timeframe::D1,
        ));
    }
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("VNXALL", DataSourcePreset::CustomFile),
        ExchangeVenue::VnIndexBenchmark
    );

    // 2. `BUG-008`: VN spot equities (`FPT`, `C69`, `KSV`) MUST NEVER open Short trades in Backtest,
    //    even if `LongShort` or `ShortOnly` is requested; `VN30F1M` & `BTCUSDT` still allow Short.
    let mut swing_bars = Vec::with_capacity(120);
    let t0 = days_from_civil(2025, 1, 2) * 86_400;
    for i in 0..120_usize {
        let p = 50.0 + 15.0 * ((i as f64) * 0.18).sin();
        swing_bars.push(mk_candle(
            t0 + (i as i64) * 86_400,
            p,
            p + 1.0,
            p - 1.0,
            p + 0.2,
            5_000.0,
        ));
    }
    let cfg_ls = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 3,
            slow_ema: 8,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongShort,
        ..BacktestConfig::default()
    };
    let fpt_ls = run_backtest("FPT", Timeframe::D1, &swing_bars, &cfg_ls, None);
    assert!(
        fpt_ls.trades.iter().all(|t| t.side == PositionSide::Long),
        "BUG-008: HOSE stock FPT must NEVER execute Short trades in LongShort backtest"
    );
    let cfg_so = BacktestConfig {
        direction_mode: StrategyDirectionMode::ShortOnly,
        ..cfg_ls.clone()
    };
    let fpt_so = run_backtest("FPT", Timeframe::D1, &swing_bars, &cfg_so, None);
    assert!(
        fpt_so.trades.is_empty() && fpt_so.open_position.is_none(),
        "BUG-008: HOSE stock FPT ShortOnly backtest must execute 0 trades"
    );
    let fut_ls = run_backtest("VN30F1M", Timeframe::D1, &swing_bars, &cfg_ls, None);
    assert!(
        fut_ls.trades.iter().any(|t| t.side == PositionSide::Short),
        "VN30F1M derivatives must still allow Short trades in LongShort backtest"
    );

    // 3. `BUG-016`: `OutlierImpactReport::expectancy_ex_top_3_winners` uses `valid_n - removed_winners_count`.
    let mk_tr = |id: u64, pnl: f64| BacktestTrade {
        trade_id: id,
        side: PositionSide::Long,
        quantity: 100.0,
        entry_bar_index: id as usize,
        exit_bar_index: (id as usize) + 1,
        entry_timestamp: t0 + (id as i64) * 86_400,
        exit_timestamp: t0 + ((id + 1) as i64) * 86_400,
        entry_price: 50.0,
        exit_price: 50.0 + pnl / 100.0,
        bars_held: 1,
        gross_pnl: pnl,
        entry_fee: 0.0,
        exit_fee: 0.0,
        exit_tax: 0.0,
        net_pnl: pnl,
        return_pct: pnl / 50.0,
        r_multiple: None,
        mae_pct: 0.0,
        mfe_pct: 0.0,
        exit_reason: BacktestExitReason::SignalExit,
    };
    let outlier = compute_outlier_impact(&[
        mk_tr(1, 50.0),
        mk_tr(2, -100.0),
        mk_tr(3, -100.0),
        mk_tr(4, -100.0),
    ]);
    assert!((outlier.net_pnl_ex_top_3_winners - (-300.0)).abs() < 1e-9);
    assert!(
        (outlier.expectancy_ex_top_3_winners.unwrap() - (-100.0)).abs() < 1e-9,
        "BUG-016: expected -100.0, got {:?}",
        outlier.expectancy_ex_top_3_winners
    );

    // 4. `BUG-029`: Yearly breakdown after bankruptcy reports 0.0% (NOT fake -100.0%).
    let y2024_ts = days_from_civil(2024, 6, 1) * 86_400;
    let y2025_ts = days_from_civil(2025, 6, 1) * 86_400;
    let bankrupt_curve = vec![
        BacktestEquityPoint {
            bar_index: 0,
            timestamp: y2024_ts,
            equity: 100_000.0,
            cash: 100_000.0,
            drawdown_abs: 0.0,
            underwater_pct: 0.0,
            position_qty: 0.0,
        },
        BacktestEquityPoint {
            bar_index: 1,
            timestamp: y2024_ts + 86_400,
            equity: 0.0,
            cash: 0.0,
            drawdown_abs: 100_000.0,
            underwater_pct: -100.0,
            position_qty: 0.0,
        },
        BacktestEquityPoint {
            bar_index: 2,
            timestamp: y2025_ts,
            equity: 0.0,
            cash: 0.0,
            drawdown_abs: 100_000.0,
            underwater_pct: -100.0,
            position_qty: 0.0,
        },
    ];
    let yearly = compute_yearly_breakdown(100_000.0, &bankrupt_curve, &[]);
    let y2025 = yearly.iter().find(|s| s.year == 2025).unwrap();
    assert!(
        y2025.equity_return_pct.abs() < 1e-9,
        "BUG-029: post-bankruptcy year must report 0.0%, got {:.2}%",
        y2025.equity_return_pct
    );

    // 5. `BUG-029`: `compute_market_breadth_and_rrg` does NOT double-drop closed bar with replay cutoff.
    let b_bars = vec![
        mk_candle(t0, 100.0, 101.0, 99.0, 100.0, 1_000.0),
        mk_candle(t0 + 86_400, 100.0, 106.0, 100.0, 105.0, 1_000.0), // +5% advancer
        mk_candle(t0 + 2 * 86_400, 105.0, 106.0, 95.0, 96.0, 1_000.0), // future bar after cutoff
    ];
    let breadth = compute_market_breadth_and_rrg(
        &[BreadthFeedInput {
            instrument_id: "Custom:FPT",
            symbol: "FPT",
            preset_label: "Custom",
            candles: &b_bars,
            timeframe: Timeframe::D1,
            exclude_incomplete_tail: true,
        }],
        &b_bars,
        Timeframe::D1,
        "VNINDEX",
        Some(t0 + 86_400 + 86_399), // closes bar 1 (`+5%`), excludes bar 2
    );
    assert_eq!(
        breadth.advancers, 1,
        "BUG-029: must evaluate bar 1 (+5%) without double-dropping"
    );
    assert_eq!(breadth.evaluated_timestamp, Some(t0 + 86_400));

    // 6. `BUG-029`: `PortfolioUniverseFilter::Vn30EquitiesOnly` filters CustomFile feeds properly,
    //    and `W1` PIT preserves `UniverseRemoval` closing week return + `BUG-034` no phantom peak.
    let w1_mon0 = days_from_civil(2025, 1, 6) * 86_400; // Monday
    let mut w1_bars = Vec::new();
    for wk in 0..16_i64 {
        let p = 50.0 + (wk as f64) * 2.0;
        w1_bars.push(mk_candle(
            w1_mon0 + wk * 7 * 86_400,
            p,
            p + 1.0,
            p - 0.5,
            p + 0.8,
            10_000.0,
        ));
    }
    let pit_u = UniverseDefinition {
        universe_id: "TEST_W1_PIT".to_owned(),
        version: "1".to_owned(),
        name: "Test W1 PIT".to_owned(),
        status: UniverseStatus::Approved,
        default_mode: UniverseMode::PointInTime,
        effective_from: Some(SessionDate::new(2025, 1, 1).unwrap()),
        effective_to: Some(SessionDate::new(2025, 12, 31).unwrap()),
        selection_purpose: "regression".to_owned(),
        source_evidence: "unit".to_owned(),
        members: vec![UniverseMember {
            symbol: "FPT".to_owned(),
            exchange: "HOSE".to_owned(),
            company_name: None,
            sector: None,
            effective_from: Some(SessionDate::new(2025, 1, 1).unwrap()),
            // Mid-week Wednesday of week 10 (`2025-03-19`): week 10 opens Mon `2025-03-17` (active!)
            // and closes Sun `2025-03-23` (after `effective_to`)!
            effective_to: Some(SessionDate::new(2025, 3, 19).unwrap()),
            status: MemberStatus::Active,
            inclusion_reason: "test".to_owned(),
            weight: 1.0,
        }],
    };
    let custom_feeds = vec![
        WatchlistFeedEntry {
            instrument_id: "Custom:FPT".to_owned(),
            preset: DataSourcePreset::CustomFile,
            symbol: "FPT".to_owned(),
            base_timeframe: Timeframe::W1,
            candles: w1_bars.clone(),
        },
        WatchlistFeedEntry {
            instrument_id: "Custom:C69".to_owned(),
            preset: DataSourcePreset::CustomFile,
            symbol: "C69".to_owned(),
            base_timeframe: Timeframe::W1,
            candles: w1_bars.clone(),
        },
    ];
    let pit_cfg = BacktestConfig {
        preset: StrategyPresetKind::SmaEmaTrendCross,
        params: StrategyParams {
            fast_sma: 2,
            slow_ema: 3,
            ..StrategyParams::default()
        },
        direction_mode: StrategyDirectionMode::LongOnly,
        ..BacktestConfig::default()
    };
    let port_rep = run_portfolio_backtest_with_universe(
        &custom_feeds,
        Timeframe::W1,
        &pit_cfg,
        PortfolioUniverseFilter::Vn30EquitiesOnly,
        PortfolioWeightingMode::EqualWeight,
        None,
        Some(&pit_u),
        Some(UniverseMode::PointInTime),
    );
    assert_eq!(
        port_rep.constituents.len(),
        1,
        "BUG-029: Vn30EquitiesOnly on CustomFile must keep FPT and exclude non-VN30 C69"
    );
    assert_eq!(port_rep.constituents[0].symbol, "FPT");
    assert!(
        (port_rep.portfolio_tearsheet.total_return_pct
            - port_rep.constituents[0].standalone_return_pct)
            .abs()
            < 1e-6,
        "BUG-029: W1 PIT portfolio return ({:.4}%) must match FPT standalone ({:.4}%) including UniverseRemoval week!",
        port_rep.portfolio_tearsheet.total_return_pct,
        port_rep.constituents[0].standalone_return_pct
    );

    // 7. `BUG-034`: `technical.rsi_overbought_cross` default threshold == 70.0 & `export_markdown` strips `\n` in symbol.
    let ob_entry = SIGNAL_CATALOG
        .iter()
        .find(|e| e.id == "technical.rsi_overbought_cross")
        .expect("rsi_overbought_cross");
    let thr_param = ob_entry
        .params
        .iter()
        .find(|p| p.name == "threshold")
        .expect("threshold param");
    assert!((thr_param.default_value - 70.0).abs() < 1e-9);

    let mut journal = TradeJournalStore::default();
    journal.entries.push(JournalEntry {
        id: 1,
        source: JournalEntrySource::PlannedSetup {
            symbol: "FPT".to_owned(),
        },
        status: JournalEntryStatus::Planned,
        symbol: "FPT\nINJECTED".to_owned(),
        venue: ExchangeVenue::Hose,
        side: PositionSide::Long,
        created_at_timestamp: t0,
        entry_timestamp: t0,
        exit_timestamp: None,
        planned_entry: Some(100.0),
        planned_stop: Some(95.0),
        planned_target: Some(110.0),
        planned_quantity: Some(100.0),
        planned_net_r: Some(2.0),
        executed_entry: None,
        executed_exit: None,
        executed_quantity: None,
        gross_pnl: 0.0,
        fees_and_taxes: 0.0,
        net_pnl: 0.0,
        realized_r: None,
        setup: SetupTag::default(),
        mistake: crate::journal::MistakeTag::None,
        conviction: 3,
        checklist: crate::journal::ChecklistState::default(),
        notes: "line1\r\nline2".to_owned(),
    });
    let md = journal.export_markdown(None);
    assert!(
        !md.contains("FPT\nINJECTED"),
        "BUG-034: symbol newlines must be stripped in Markdown table"
    );
    assert!(md.contains("FPT INJECTED"));
}
