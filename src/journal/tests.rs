use crate::backtest::{BacktestExitReason, BacktestTrade};
use crate::data::{DataSourcePreset, days_from_civil, decompose_utc};
use crate::trading::{ClosedTrade, PositionSide};

use super::{
    ChecklistState, ExchangeVenue, MistakeTag, PlanWarning, ResolvedVnPriceUnit, SetupTag,
    TradeJournalStore, TradePlanError, TradePlanInput, VnPriceUnit, compute_daily_price_band,
    compute_settlement_timestamp, compute_settlement_timestamp_for_venue, compute_trade_plan,
    floor_to_board_lot, round_down_to_tick, sanitize_csv_cell, step_one_tick_above,
    step_one_tick_below, tick_size_at_price,
};

#[test]
fn hose_three_tier_tick_ladder_and_dual_unit_scale() {
    // ThousandVnd (CafeF / VN30 quotes: 9.85, 28.50, 239.86)
    assert_eq!(
        VnPriceUnit::Auto.resolve(239.86),
        ResolvedVnPriceUnit::ThousandVnd
    );
    assert!(
        (tick_size_at_price(ExchangeVenue::Hose, 9.85, ResolvedVnPriceUnit::ThousandVnd) - 0.01)
            .abs()
            < 1e-12
    );
    assert!(
        (tick_size_at_price(ExchangeVenue::Hose, 28.50, ResolvedVnPriceUnit::ThousandVnd) - 0.05)
            .abs()
            < 1e-12
    );
    assert!(
        (tick_size_at_price(
            ExchangeVenue::Hose,
            239.86,
            ResolvedVnPriceUnit::ThousandVnd
        ) - 0.10)
            .abs()
            < 1e-12
    );

    // RawVnd (Order-book raw VND quotes: 9_850, 28_500, 239_860)
    assert_eq!(
        VnPriceUnit::Auto.resolve(48_500.0),
        ResolvedVnPriceUnit::RawVnd
    );
    assert!(
        (tick_size_at_price(ExchangeVenue::Hose, 9_850.0, ResolvedVnPriceUnit::RawVnd) - 10.0)
            .abs()
            < 1e-9
    );
    assert!(
        (tick_size_at_price(ExchangeVenue::Hose, 28_500.0, ResolvedVnPriceUnit::RawVnd) - 50.0)
            .abs()
            < 1e-9
    );
    assert!(
        (tick_size_at_price(ExchangeVenue::Hose, 50_000.0, ResolvedVnPriceUnit::RawVnd) - 100.0)
            .abs()
            < 1e-9
    );

    // HNX & UPCOM uniform 100 VND (0.10 in ThousandVnd) across all tiers
    assert!(
        (tick_size_at_price(ExchangeVenue::Hnx, 8.50, ResolvedVnPriceUnit::ThousandVnd) - 0.10)
            .abs()
            < 1e-12
    );
    assert!(
        (tick_size_at_price(
            ExchangeVenue::Upcom,
            14.20,
            ResolvedVnPriceUnit::ThousandVnd
        ) - 0.10)
            .abs()
            < 1e-12
    );

    // Venue inference
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("FPT", DataSourcePreset::Vn30Basket),
        ExchangeVenue::Hose
    );
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("VN30F1M", DataSourcePreset::IntradayVn30F1m),
        ExchangeVenue::Vn30Futures
    );
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("BTCUSDT", DataSourcePreset::BtcUsdt15mJson),
        ExchangeVenue::CryptoSpotPerp
    );
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("SPY", DataSourcePreset::SpyDailyCsv),
        ExchangeVenue::UsEquities
    );
}

#[test]
fn statutory_daily_price_band_rounding_and_hose_50k_tier_crossing() {
    // 1. Exact IEEE-754 boundary (`100.0 * 1.07 = 107.0`, `100.0 * 0.93 = 93.0`)
    let band_100 = compute_daily_price_band(ExchangeVenue::Hose, 100.0, VnPriceUnit::Auto)
        .expect("HOSE has ±7% band");
    assert!((band_100.ceiling_price - 107.0).abs() < 1e-9);
    assert!((band_100.floor_price - 93.0).abs() < 1e-9);

    // 2. Tier-crossing upward across 50.00 (`ref = 49.00`: raw ceiling = 52.43 -> must round DOWN to 0.10 tick = 52.40, NOT 52.45!)
    let band_49 = compute_daily_price_band(ExchangeVenue::Hose, 49.00, VnPriceUnit::Auto)
        .expect("HOSE 49.00 band");
    assert!((band_49.ceiling_price - 52.40).abs() < 1e-9);
    // raw floor = 49.00 * 0.93 = 45.57 (< 50.00, tick 0.05) -> rounded UP to 0.05 tick = 45.60
    assert!((band_49.floor_price - 45.60).abs() < 1e-9);

    // 3. Tier-crossing downward across 50.00 (`ref = 51.00`: raw floor = 47.43 -> rounded UP to 0.05 tick = 47.45)
    let band_51 = compute_daily_price_band(ExchangeVenue::Hose, 51.00, VnPriceUnit::Auto)
        .expect("HOSE 51.00 band");
    assert!((band_51.floor_price - 47.45).abs() < 1e-9);
    // raw ceiling = 51.00 * 1.07 = 54.57 -> rounded DOWN to 0.10 tick = 54.50
    assert!((band_51.ceiling_price - 54.50).abs() < 1e-9);
    assert!(band_51.is_locked_ceiling(54.50, 54.50));
    assert!(band_51.is_locked_floor(47.45, 47.45));
    assert!(!band_51.is_locked_ceiling(54.40, 54.50));

    // 4. HNX ±10% and UPCOM ±15%
    let hnx = compute_daily_price_band(ExchangeVenue::Hnx, 23.40, VnPriceUnit::Auto).unwrap();
    // raw ceiling = 25.74 -> 25.70; raw floor = 21.06 -> 21.10
    assert!((hnx.ceiling_price - 25.70).abs() < 1e-9);
    assert!((hnx.floor_price - 21.10).abs() < 1e-9);

    let upcom = compute_daily_price_band(ExchangeVenue::Upcom, 18.50, VnPriceUnit::Auto).unwrap();
    // raw ceiling = 21.275 -> 21.20; raw floor = 15.725 -> 15.80
    assert!((upcom.ceiling_price - 21.20).abs() < 1e-9);
    assert!((upcom.floor_price - 15.80).abs() < 1e-9);

    // 5. US Equities & Crypto have no statutory band
    assert!(
        compute_daily_price_band(ExchangeVenue::UsEquities, 500.0, VnPriceUnit::Auto).is_none()
    );
    assert!(
        compute_daily_price_band(ExchangeVenue::CryptoSpotPerp, 65_000.0, VnPriceUnit::Auto)
            .is_none()
    );
}

#[test]
fn t_plus_2_business_day_settlement_skips_weekends() {
    // 2026-10-08 is a Thursday -> T+2 business days lands on Monday 2026-10-12!
    let thu_ts = days_from_civil(2026, 10, 8) * 86_400 + 9 * 3_600;
    assert_eq!(decompose_utc(thu_ts).weekday, 3); // Thu = 3
    let settled_mon = compute_settlement_timestamp(thu_ts, 2);
    let dt_mon = decompose_utc(settled_mon);
    assert_eq!((dt_mon.year, dt_mon.month, dt_mon.day), (2026, 10, 12));
    assert_eq!(dt_mon.weekday, 0); // Monday

    // 2026-10-09 is a Friday -> T+2 business days lands on Tuesday 2026-10-13; T+1 lands on Monday 2026-10-12!
    let fri_ts = days_from_civil(2026, 10, 9) * 86_400 + 14 * 3_600;
    let settled_tue = compute_settlement_timestamp(fri_ts, 2);
    let dt_tue = decompose_utc(settled_tue);
    assert_eq!((dt_tue.year, dt_tue.month, dt_tue.day), (2026, 10, 13));
    let us_t1 = compute_settlement_timestamp(fri_ts, 1);
    let dt_us = decompose_utc(us_t1);
    assert_eq!((dt_us.year, dt_us.month, dt_us.day), (2026, 10, 12));

    // T+0 futures returns exact entry timestamp
    assert_eq!(compute_settlement_timestamp(fri_ts, 0), fri_ts);
    assert!((floor_to_board_lot(ExchangeVenue::Hose, 487.9) - 400.0).abs() < 1e-9);
}

#[test]
fn position_planner_all_in_stop_risk_never_exceeds_budget_and_breakeven_exact() {
    let input = TradePlanInput {
        venue: ExchangeVenue::Hose,
        price_unit: VnPriceUnit::ThousandVnd,
        side: PositionSide::Long,
        account_equity: 200_000.0,
        available_cash: 200_000.0,
        leverage: 1.0,
        risk_pct: 1.0, // max_risk_amount = 2,000.0
        entry_price: 235.04,
        stop_loss: 228.08,
        target_price: 252.06,
        reference_price: Some(234.00),
        commission_bps: 15.0, // 0.15%
        tax_sell_bps: 10.0,   // 0.10% VN sell tax
        slippage_bps: 5.0,    // 0.05%
        snap_to_ticks: true,
        entry_timestamp: Some(days_from_civil(2026, 10, 8) * 86_400),
        ..TradePlanInput::default()
    };
    let plan = compute_trade_plan(&input).expect("valid HOSE long plan");

    // Tick-snapped onto 0.10 grid: entry = 235.0, stop = 228.0, target = 252.0
    assert!((plan.snapped_entry - 235.0).abs() < 1e-9);
    assert!((plan.snapped_stop - 228.0).abs() < 1e-9);
    assert!((plan.snapped_target - 252.0).abs() < 1e-9);
    assert_eq!(plan.planned_quantity % 100.0, 0.0);

    // Crucial fix over legacy `trade_planning.py:136`: realized stop loss INCLUDING stop-exit commission
    // + 0.10% VN sell tax + slippage MUST NOT exceed max_risk_amount (2,000.0)!
    let c = 0.0015;
    let s = 0.0005;
    let t_sell = 0.0010;
    let cost_in = plan.planned_quantity * (plan.snapped_entry * (1.0 + s)) * (1.0 + c);
    let stop_out_proceeds =
        plan.planned_quantity * (plan.snapped_stop * (1.0 - s)) * (1.0 - c - t_sell);
    let realized_stop_loss = cost_in - stop_out_proceeds;
    assert!((realized_stop_loss - plan.planned_risk_amount).abs() < 1e-9);
    assert!(realized_stop_loss <= plan.max_risk_amount + 1e-9);
    assert!(plan.net_r_multiple < plan.gross_r_multiple);

    // Verify exact breakeven price yields 0.0 net PnL after round-trip friction
    let be_proceeds =
        plan.planned_quantity * (plan.breakeven_price * (1.0 - s)) * (1.0 - c - t_sell);
    assert!((be_proceeds - cost_in).abs() < 1e-8);

    // Verify Short plan on VN30F1M futures (100 kVND/pt contract multiplier & 17% initial margin)
    let short_input = TradePlanInput {
        venue: ExchangeVenue::Vn30Futures,
        price_unit: VnPriceUnit::Auto,
        side: PositionSide::Short,
        account_equity: 250_000.0,
        available_cash: 250_000.0,
        leverage: 5.0,
        risk_pct: 1.5,
        entry_price: 1_285.0,
        stop_loss: 1_295.0,
        target_price: 1_260.0,
        reference_price: Some(1_280.0),
        commission_bps: 4.0,
        tax_sell_bps: 0.0,
        slippage_bps: 2.0,
        snap_to_ticks: true,
        entry_timestamp: None,
        ..TradePlanInput::default()
    };
    let short_plan = compute_trade_plan(&short_input).expect("valid VN30F short plan");
    assert!(short_plan.affordable);
    assert!((short_plan.contract_multiplier - 100.0).abs() < 1e-9);
    assert!(short_plan.margin_required > 0.0 && short_plan.margin_required <= 250_000.0);
    assert!(short_plan.planned_risk_amount <= short_plan.max_risk_amount + 1e-9);
    assert!(short_plan.breakeven_price < short_plan.snapped_entry);
}

#[test]
fn position_planner_warnings_and_validation_errors() {
    // Inverted brackets rejected
    let mut bad = TradePlanInput::default();
    bad.stop_loss = bad.entry_price + 2.0;
    assert_eq!(
        compute_trade_plan(&bad).unwrap_err(),
        TradePlanError::InvalidBracketsForSide
    );

    // Short on HOSE + Stop below daily floor + Insufficient risk for 100-share lot
    let warn_input = TradePlanInput {
        venue: ExchangeVenue::Hose,
        price_unit: VnPriceUnit::ThousandVnd,
        side: PositionSide::Long,
        account_equity: 1_000.0,
        available_cash: 500.0,
        leverage: 1.0,
        risk_pct: 0.5, // max risk = 5.0, whereas 100 shares * ~20 risk = 2,000!
        entry_price: 100.0,
        stop_loss: 80.0,     // below HOSE floor 93.0!
        target_price: 112.0, // above HOSE ceiling 107.0!
        reference_price: Some(100.0),
        commission_bps: 15.0,
        tax_sell_bps: 10.0,
        slippage_bps: 5.0,
        snap_to_ticks: true,
        entry_timestamp: None,
        ..TradePlanInput::default()
    };
    let plan = compute_trade_plan(&warn_input).expect("computes with warnings");
    assert_eq!(plan.planned_quantity, 0.0);
    assert!(!plan.affordable);
    assert!(
        plan.warnings
            .iter()
            .any(|w| matches!(w, PlanWarning::StopBelowDailyFloorGapRisk { .. }))
    );
    assert!(
        plan.warnings
            .iter()
            .any(|w| matches!(w, PlanWarning::TargetAboveDailyCeilingMultiSession { .. }))
    );
    assert!(
        plan.warnings
            .iter()
            .any(|w| matches!(w, PlanWarning::InsufficientRiskForBoardLot { .. }))
    );
}

#[test]
fn journal_causal_visibility_idempotent_sync_sqn_and_csv_formula_guard() {
    let mut store = TradeJournalStore::new();
    let plan = compute_trade_plan(&TradePlanInput::default()).unwrap();
    let plan_id = store.record_trade_plan(
        "FPT",
        &plan,
        1_700_000_000,
        SetupTag::SqueezeBreakout,
        4,
        ChecklistState::all_checked(),
        "=SUM(A1:A10) breakout note, with \"quotes\"",
    );
    assert_eq!(plan_id, 1);

    let paper_trades = vec![
        ClosedTrade {
            trade_id: 1,
            side: PositionSide::Long,
            quantity: 200.0,
            entry_price: 100.0,
            exit_price: 106.0,
            entry_timestamp: 1_700_000_100,
            exit_timestamp: 1_700_001_000,
            gross_pnl: 1_200.0,
            total_fees: 50.0,
            net_pnl: 1_150.0,
            return_pct: 5.75,
            r_multiple: Some(2.3),
        },
        ClosedTrade {
            trade_id: 2,
            side: PositionSide::Long,
            quantity: 200.0,
            entry_price: 108.0,
            exit_price: 104.0,
            entry_timestamp: 1_700_002_000,
            exit_timestamp: 1_700_003_000,
            gross_pnl: -800.0,
            total_fees: 52.0,
            net_pnl: -852.0,
            return_pct: -3.94,
            r_multiple: Some(-1.1),
        },
    ];
    assert_eq!(
        store.sync_paper_closed_trades("FPT", ExchangeVenue::Hose, &paper_trades),
        2
    );
    // Idempotent second sync adds 0 duplicates and preserves user edits
    if let Some(loss_entry) = store.entry_mut(3) {
        loss_entry.mistake = MistakeTag::FomoChase;
        loss_entry.setup = SetupTag::MomentumContinuation;
        loss_entry.checklist = ChecklistState::empty();
    }
    assert_eq!(
        store.sync_paper_closed_trades("FPT", ExchangeVenue::Hose, &paper_trades),
        0
    );
    assert_eq!(store.entries[2].mistake, MistakeTag::FomoChase);

    // Import 3 backtest trades so total closed R-samples >= 5 (enables SQN!)
    let bt_trades: Vec<BacktestTrade> = (1..=3)
        .map(|k| BacktestTrade {
            trade_id: k,
            side: PositionSide::Long,
            entry_bar_index: (k as usize) * 2,
            exit_bar_index: (k as usize) * 2 + 1,
            entry_timestamp: 1_700_010_000 + (k as i64) * 100,
            exit_timestamp: 1_700_010_050 + (k as i64) * 100,
            entry_price: 100.0,
            exit_price: 105.0,
            quantity: 100.0,
            bars_held: 1,
            gross_pnl: 500.0,
            entry_fee: 10.0,
            exit_fee: 10.0,
            exit_tax: 5.0,
            net_pnl: 475.0,
            return_pct: 4.75,
            exit_reason: BacktestExitReason::TakeProfit,
            mae_pct: -0.5,
            mfe_pct: 5.2,
            r_multiple: Some(1.8 + 0.1 * (k as f64)),
        })
        .collect();
    assert_eq!(
        store.import_backtest_trades(
            "FPT",
            ExchangeVenue::Hose,
            "SMA/EMA",
            SetupTag::TrendPullback,
            &bt_trades,
            10
        ),
        3
    );
    assert_eq!(
        store.import_backtest_trades(
            "FPT",
            ExchangeVenue::Hose,
            "SMA/EMA",
            SetupTag::TrendPullback,
            &bt_trades,
            10
        ),
        0
    );

    // Causal DEF-001 cutoff at 1_700_001_500 hides trade #2 (exit 1_700_003_000) and all 3 backtest trades!
    let stats_causal = store.compute_analytics(Some(1_700_001_500));
    assert_eq!(stats_causal.visible_entry_count, 2); // 1 plan + 1 closed win
    assert_eq!(stats_causal.closed_count, 1);
    assert_eq!(stats_causal.mistake_trade_count, 0);

    // Full timeline analytics
    let stats_all = store.compute_analytics(None);
    assert_eq!(stats_all.visible_entry_count, 6);
    assert_eq!(stats_all.closed_count, 5);
    assert_eq!(stats_all.planned_count, 1);
    assert_eq!(stats_all.mistake_trade_count, 1);
    assert!((stats_all.cost_of_mistakes - 852.0).abs() < 1e-9);
    assert!(stats_all.sqn.is_some_and(|s| s > 1.0));
    assert_eq!(stats_all.current_streak, 3);
    assert_eq!(stats_all.entries_status_check(), 6);

    // CSV formula-injection neutralization
    assert_eq!(
        sanitize_csv_cell("=CMD|' /C calc'!A0"),
        "'=CMD|' /C calc'!A0"
    );
    assert_eq!(sanitize_csv_cell("  +1+2"), "'  +1+2");
    assert_eq!(sanitize_csv_cell("-R1C1"), "'-R1C1");
    assert_eq!(sanitize_csv_cell("@SUM(1,2)"), "\"'@SUM(1,2)\"");
    let csv = store.export_csv(None);
    assert!(csv.contains("\"'=SUM(A1:A10) breakout note, with \"\"quotes\"\"\""));
    let md = store.export_markdown(None);
    assert!(md.contains("Van Tharp SQN"));
}

trait AnalyticsTestExt {
    fn entries_status_check(&self) -> usize;
}

impl AnalyticsTestExt for super::JournalAnalytics {
    fn entries_status_check(&self) -> usize {
        self.planned_count + self.closed_count
    }
}

#[test]
fn critic_r10_regression_vn30f_multiplier_hose_tier_step_local_tz_and_csv_newline() {
    // 1. C-03: VNINDEX / HNX-INDEX must resolve to `VnIndexBenchmark` (not HOSE 100-share penny stock!)
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("VNINDEX", DataSourcePreset::SyntheticVnindex),
        ExchangeVenue::VnIndexBenchmark
    );
    assert_eq!(
        ExchangeVenue::infer_from_symbol_and_preset("VNINDEX", DataSourcePreset::CafeFIndexCsv),
        ExchangeVenue::VnIndexBenchmark
    );
    assert!(
        compute_daily_price_band(ExchangeVenue::VnIndexBenchmark, 1_250.0, VnPriceUnit::Auto)
            .is_none()
    );

    // 2. C-03: VN30F1M RawVnd (`100,000 VND/pt`) & ThousandVnd (`100 kVND/pt`) + 17% margin
    let vn30f_raw = compute_trade_plan(&TradePlanInput {
        venue: ExchangeVenue::Vn30Futures,
        price_unit: VnPriceUnit::RawVnd,
        side: PositionSide::Long,
        account_equity: 300_000_000.0,
        available_cash: 300_000_000.0,
        leverage: 1.0,
        risk_pct: 1.0, // max risk = 3,000,000 VND
        entry_price: 1_280.0,
        stop_loss: 1_270.0, // 10 pts * 100,000 = 1,000,000 VND/contract + VN30F friction -> 2 contracts
        target_price: 1_305.0,
        commission_bps: ExchangeVenue::Vn30Futures.default_commission_bps(),
        tax_sell_bps: ExchangeVenue::Vn30Futures.default_tax_sell_bps(),
        slippage_bps: 1.5,
        ..TradePlanInput::default()
    })
    .expect("VN30F raw plan");
    assert!((vn30f_raw.contract_multiplier - 100_000.0).abs() < 1e-6);
    assert!((vn30f_raw.planned_quantity - 2.0).abs() < 1e-9);
    assert!(vn30f_raw.margin_required > 40_000_000.0);

    // 3. M-01: HOSE tier transitions (`50.00 -> 49.95`, `10.00 -> 9.99`) + `round_down_to_tick` sub-tick floor
    assert_eq!(
        round_down_to_tick(0.05, ExchangeVenue::Hnx, ResolvedVnPriceUnit::ThousandVnd),
        0.0
    );
    assert!(
        (step_one_tick_below(50.00, ExchangeVenue::Hose, ResolvedVnPriceUnit::ThousandVnd) - 49.95)
            .abs()
            < 1e-9
    );
    assert!(
        (step_one_tick_below(10.00, ExchangeVenue::Hose, ResolvedVnPriceUnit::ThousandVnd) - 9.99)
            .abs()
            < 1e-9
    );
    assert!(
        (step_one_tick_above(49.95, ExchangeVenue::Hose, ResolvedVnPriceUnit::ThousandVnd) - 50.00)
            .abs()
            < 1e-9
    );

    // 4. M-02: Short disallowed on HOSE forces `planned_quantity == 0.0` & Short entry charges `tax_sell_bps`
    let hose_short = compute_trade_plan(&TradePlanInput {
        venue: ExchangeVenue::Hose,
        side: PositionSide::Short,
        entry_price: 100.0,
        stop_loss: 105.0,
        target_price: 90.0,
        ..TradePlanInput::default()
    })
    .expect("HOSE short warns and sizes 0");
    assert_eq!(hose_short.planned_quantity, 0.0);
    assert!(!hose_short.affordable);
    assert!(
        hose_short
            .warnings
            .contains(&PlanWarning::ShortDisallowedOnVenue)
    );

    let short_with_sell_tax = compute_trade_plan(&TradePlanInput {
        venue: ExchangeVenue::CryptoSpotPerp,
        side: PositionSide::Short,
        account_equity: 10_000.0,
        available_cash: 10_000.0,
        entry_price: 100.0,
        stop_loss: 105.0,
        target_price: 90.0,
        commission_bps: 10.0,
        tax_sell_bps: 20.0,
        slippage_bps: 0.0,
        snap_to_ticks: false,
        ..TradePlanInput::default()
    })
    .expect("short with sell tax");
    assert!(short_with_sell_tax.breakeven_price < 99.75);

    // 5. M-09: Exchange-local timezone settlement (`2026-10-08 19:00 UTC` = `2026-10-09 02:00 ICT` Friday -> T+2 is Tuesday `2026-10-13 06:00 UTC`!)
    let thu_19_utc = days_from_civil(2026, 10, 8) * 86_400 + 19 * 3_600;
    let hose_settle = compute_settlement_timestamp_for_venue(thu_19_utc, ExchangeVenue::Hose);
    let dt_ict = decompose_utc(hose_settle + 7 * 3_600);
    assert_eq!((dt_ict.year, dt_ict.month, dt_ict.day), (2026, 10, 13));
    assert_eq!(dt_ict.hour, 13);

    // 6. M-05, M-06 & N-01: Rewind prune, multiline/leading-LF CSV formula guard, NaN JSON sanitize, PF & breakeven streak
    assert_eq!(sanitize_csv_cell("\n=CMD()"), "' =CMD()");
    assert!(!sanitize_csv_cell("line1\r\n=line2").contains('\n'));
    assert!(!sanitize_csv_cell("line1\r\n=line2").contains('\r'));

    let mut store = TradeJournalStore::new();
    let t1 = ClosedTrade {
        trade_id: 1,
        side: PositionSide::Long,
        quantity: 100.0,
        entry_price: 50.0,
        exit_price: 55.0,
        entry_timestamp: 1_000,
        exit_timestamp: 2_000,
        gross_pnl: 500.0,
        total_fees: 10.0,
        net_pnl: 490.0,
        return_pct: 9.8,
        r_multiple: Some(2.0),
    };
    let t2_future = ClosedTrade {
        trade_id: 2,
        entry_timestamp: 3_000,
        exit_timestamp: 4_000,
        net_pnl: 0.0, // breakeven
        ..t1.clone()
    };
    assert_eq!(
        store.sync_paper_closed_trades("FPT", ExchangeVenue::Hose, &[t1.clone(), t2_future]),
        2
    );
    let stats_be = store.compute_analytics(None);
    assert_eq!(
        stats_be.current_streak, 0,
        "N-01: breakeven trade must reset current_streak to 0"
    );
    assert_eq!(stats_be.profit_factor, None);

    // Rewind paper session so only `t1` remains -> `t2_future` must be pruned!
    assert_eq!(
        store.sync_paper_closed_trades("FPT", ExchangeVenue::Hose, std::slice::from_ref(&t1)),
        0
    );
    assert_eq!(store.entries.len(), 1);
    assert_eq!(store.compute_analytics(None).current_streak, 1);

    let t3_loss = ClosedTrade {
        trade_id: 3,
        entry_timestamp: 5_000,
        exit_timestamp: 6_000,
        net_pnl: -245.0,
        ..t1.clone()
    };
    assert_eq!(
        store.sync_paper_closed_trades("FPT", ExchangeVenue::Hose, &[t1, t3_loss]),
        1
    );
    assert_eq!(store.entries.len(), 2);
    assert_eq!(store.compute_analytics(None).profit_factor, Some(2.0));
}
