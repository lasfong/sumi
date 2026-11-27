use chrono::{Datelike, FixedOffset, TimeZone};
use serde::{Deserialize, Serialize};
use sumi_core::backtest::{
    BacktestConfig, BacktestEquityPoint, BacktestRunOptions, BacktestTearsheet, BacktestTrade,
    ConfluenceThresholdGate, CustomSignalRuleSpec, DEFAULT_MONTE_CARLO_SEED, DirectionalRuleLeg,
    FloatCmpOp, LogicalCombinator, MonteCarloConfig, MonteCarloRiskMetrics, MonteCarloValidity,
    OpenBacktestPosition, PortfolioConstituentAttribution, PortfolioUniverseFilter,
    PortfolioWeightingMode, PositionSizingMode, PresetBattleRow, QuantileEnvelope5,
    RobustnessVerdict, SignalClause, SignalClauseGroup, SignalPredicate, StopTakeConfig,
    StrategyDirectionMode, StrategyParams, StrategyPresetKind, SweepRankingMetric, SweepVariantRow,
    WalkForwardConfig, WalkForwardFoldResult, WalkForwardVerdict,
    compute_advanced_tearsheet_analytics, render_backtest_report_html, run_all_presets_battle,
    run_backtest_with_magnifier, run_backtest_with_options, run_monte_carlo_bootstrap,
    run_parameter_sweep, run_portfolio_backtest, run_walk_forward_validation,
};
use sumi_core::data::{DataSourcePreset, Timeframe, WatchlistFeedEntry};
use sumi_core::model::Candle;
use sumi_core::scanner::{SignalOutputType, find_catalog_entry};
use sumi_data::{Adjustment, BarQueryMode, BarsRequest, Interval, OhlcvBar, SeriesKey, SymbolKey};
use tauri::State;

use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomClauseInput {
    pub catalog_id: String,
    /// For Float signals: `">" | ">=" | "<" | "<=" | "cross_above" | "cross_below"`
    pub op: Option<String>,
    pub threshold: Option<f64>,
    /// For Bool signals: `true` (default) or `false`
    pub bool_expected: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleGroupInput {
    /// `"and"` | `"or"`
    pub combinator: Option<String>,
    pub clauses: Vec<CustomClauseInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomRulesInput {
    pub name: Option<String>,
    /// Legacy flat entry combinator (`"and"` | `"or"`)
    pub entry_combinator: Option<String>,
    #[serde(default)]
    pub entry_clauses: Vec<CustomClauseInput>,
    pub exit_combinator: Option<String>,
    pub exit_clauses: Option<Vec<CustomClauseInput>>,
    pub min_health_score: Option<f64>,
    // Wave 5: Full 2-tier AND/OR 4-leg groups (`sumi-core::backtest::CustomSignalRuleSpec`)
    #[serde(default)]
    pub long_entry_inter_op: Option<String>,
    #[serde(default)]
    pub long_entry_groups: Option<Vec<RuleGroupInput>>,
    #[serde(default)]
    pub long_exit_inter_op: Option<String>,
    #[serde(default)]
    pub long_exit_groups: Option<Vec<RuleGroupInput>>,
    #[serde(default)]
    pub short_entry_inter_op: Option<String>,
    #[serde(default)]
    pub short_entry_groups: Option<Vec<RuleGroupInput>>,
    #[serde(default)]
    pub short_exit_inter_op: Option<String>,
    #[serde(default)]
    pub short_exit_groups: Option<Vec<RuleGroupInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyBacktestInput {
    pub ticker: String,
    pub interval: String,
    /// `"sma_ema"` | `"rsi_mean_rev"` | `"macd"` | `"bb_squeeze"` | `"donchian_atr"` | `"rsi_bb"` | `"custom"`
    pub preset: String,
    /// `"long"` | `"short"` | `"both"`
    pub direction: String,
    pub initial_cash: f64,
    pub commission_bps: f64,
    pub tax_sell_bps: f64,
    pub slippage_bps: f64,
    pub lot_step: f64,
    pub position_size_pct: f64,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub min_holding_bars: Option<usize>,
    pub fast_sma: Option<usize>,
    pub slow_ema: Option<usize>,
    pub rsi_period: Option<usize>,
    pub rsi_oversold: Option<f64>,
    pub rsi_overbought: Option<f64>,
    pub rsi_confluence_buy: Option<f64>,
    pub rsi_confluence_sell: Option<f64>,
    pub macd_fast: Option<usize>,
    pub macd_slow: Option<usize>,
    pub macd_signal: Option<usize>,
    pub macd_trend_ema: Option<usize>,
    pub bb_period: Option<usize>,
    pub bb_std_mult: Option<f64>,
    pub max_squeeze_bw_pct: Option<f64>,
    pub donchian_entry: Option<usize>,
    pub donchian_exit: Option<usize>,
    pub in_sample_split_pct: Option<f64>,
    pub max_bars: Option<usize>,
    pub from_timestamp_ms: Option<i64>,
    pub to_timestamp_ms: Option<i64>,
    #[serde(default)]
    pub max_leverage: Option<f64>,
    #[serde(default)]
    pub custom_rules: Option<CustomRulesInput>,
    // QMV Strategy Parameters (`S1..S6` & QMV Risk Management)
    #[serde(default)]
    pub qmv_support_window: Option<usize>,
    #[serde(default)]
    pub qmv_rvol_confirm: Option<f64>,
    #[serde(default)]
    pub qmv_trail_mode: Option<u8>,
    #[serde(default)]
    pub qmv_div_osc: Option<u8>,
    #[serde(default)]
    pub qmv_confirm_mode: Option<u8>,
    #[serde(default)]
    pub qmv_ichi_variant: Option<u8>,
    #[serde(default)]
    pub qmv_flow_horizon_pair: Option<u8>,
    #[serde(default)]
    pub qmv_flow_low_zone: Option<f64>,
    #[serde(default)]
    pub qmv_flow_high_zone: Option<f64>,
    #[serde(default)]
    pub qmv_use_vnindex_gate: Option<bool>,
    #[serde(default)]
    pub qmv_min_pillars: Option<u8>,
    #[serde(default)]
    pub support_stop_lookback: Option<usize>,
    #[serde(default)]
    pub qmv_wave_trailing_2_3: Option<bool>,
    // Wave 5 — Strategy Tester 2.0 Risk & Magnifier Controls
    #[serde(default)]
    pub max_pyramiding: Option<usize>,
    #[serde(default)]
    pub scale_out_pct: Option<f64>,
    #[serde(default)]
    pub tp1_pct: Option<f64>,
    #[serde(default)]
    pub trailing_stop_atr_mult: Option<f64>,
    #[serde(default)]
    pub trailing_ma_period: Option<usize>,
    #[serde(default)]
    pub trailing_kijun_period: Option<usize>,
    #[serde(default)]
    pub use_bar_magnifier: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyReturnCellDto {
    pub year: i32,
    pub month: u32,
    pub return_pct: f64,
    pub trades_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyEquityPointDto {
    pub bar_index: usize,
    pub timestamp_ms: i64,
    pub equity: f64,
    pub underwater_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyTradeDto {
    pub trade_id: u64,
    pub side: String,
    pub quantity: f64,
    pub entry_bar_index: usize,
    pub exit_bar_index: usize,
    pub entry_timestamp_ms: i64,
    pub exit_timestamp_ms: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub bars_held: usize,
    pub gross_pnl: f64,
    pub net_pnl: f64,
    pub return_pct: f64,
    pub r_multiple: Option<f64>,
    pub mae_pct: f64,
    pub mfe_pct: f64,
    pub exit_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyBacktestResponse {
    pub ticker: String,
    pub interval: String,
    pub preset_label: String,
    pub bar_count: usize,
    pub tearsheet: BacktestTearsheet,
    pub in_sample_tearsheet: Option<BacktestTearsheet>,
    pub out_of_sample_tearsheet: Option<BacktestTearsheet>,
    pub robustness: Option<RobustnessVerdict>,
    pub open_position: Option<OpenBacktestPosition>,
    pub equity_curve: Vec<StrategyEquityPointDto>,
    pub trades: Vec<StrategyTradeDto>,
    pub monthly_returns: Vec<MonthlyReturnCellDto>,
    pub bar_magnifier_bars: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardInput {
    pub backtest: StrategyBacktestInput,
    pub fold_count: usize,
    pub in_sample_ratio: f64,
    pub anchored: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardResponse {
    pub ticker: String,
    pub interval: String,
    pub total_bars: usize,
    pub profitable_folds: usize,
    pub fold_count: usize,
    pub wfe_pct: Option<f64>,
    pub param_stability_pct: f64,
    pub verdict: WalkForwardVerdict,
    pub verdict_label: String,
    pub stitched_oos_tearsheet: BacktestTearsheet,
    pub stitched_oos_curve: Vec<StrategyEquityPointDto>,
    pub folds: Vec<WalkForwardFoldResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarloInput {
    pub backtest: StrategyBacktestInput,
    pub simulations: usize,
    pub seed: Option<u64>,
    pub ruin_drawdown_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonteCarloResponse {
    pub simulations: usize,
    pub seed: u64,
    pub sample_count: usize,
    pub horizon_steps: usize,
    pub validity: MonteCarloValidity,
    pub low_sample_warning: bool,
    pub status_reason: String,
    pub metrics: Option<MonteCarloRiskMetrics>,
    pub checkpoint_envelopes: Vec<QuantileEnvelope5>,
}

fn parse_combinator(raw: Option<&str>) -> LogicalCombinator {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("or" | "any") => LogicalCombinator::AnyOr,
        _ => LogicalCombinator::AllAnd,
    }
}

fn parse_float_op(raw: Option<&str>) -> FloatCmpOp {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("gt" | ">") => FloatCmpOp::Gt,
        Some("lt" | "<") => FloatCmpOp::Lt,
        Some("lte" | "<=") => FloatCmpOp::Lte,
        Some("cross_above") => FloatCmpOp::CrossAbove,
        Some("cross_below") => FloatCmpOp::CrossBelow,
        _ => FloatCmpOp::Gte,
    }
}

fn build_clause_from_input(c: &CustomClauseInput) -> SignalClause {
    let mut cl = SignalClause::default_for_catalog(&c.catalog_id, None);
    if let Some(entry) = find_catalog_entry(&c.catalog_id) {
        match entry.output_type {
            SignalOutputType::Bool => {
                cl.predicate = SignalPredicate::BoolIs(c.bool_expected.unwrap_or(true));
            }
            SignalOutputType::Float => {
                cl.predicate = SignalPredicate::FloatCmp {
                    op: parse_float_op(c.op.as_deref()),
                    threshold: c.threshold.unwrap_or(0.0),
                };
            }
            SignalOutputType::Enum => {}
        }
    }
    cl.sanitize();
    cl
}

fn build_groups_from_input(groups_opt: Option<&[RuleGroupInput]>) -> Vec<SignalClauseGroup> {
    groups_opt
        .unwrap_or(&[])
        .iter()
        .filter_map(|g| {
            let clauses: Vec<SignalClause> = g
                .clauses
                .iter()
                .filter(|c| !c.catalog_id.trim().is_empty())
                .map(build_clause_from_input)
                .collect();
            if clauses.is_empty() {
                None
            } else {
                Some(SignalClauseGroup {
                    intra_op: parse_combinator(g.combinator.as_deref()),
                    clauses,
                })
            }
        })
        .collect()
}

fn build_custom_rule_spec(custom: &CustomRulesInput) -> CustomSignalRuleSpec {
    let mut gate = ConfluenceThresholdGate::default();
    if let Some(min_h) = custom.min_health_score.filter(|v| v.is_finite()) {
        gate.enabled = true;
        gate.min_score = Some(min_h);
    }

    let long_entry_groups = if custom.long_entry_groups.is_some() {
        build_groups_from_input(custom.long_entry_groups.as_deref())
    } else {
        let legacy: Vec<SignalClause> = custom
            .entry_clauses
            .iter()
            .filter(|c| !c.catalog_id.trim().is_empty())
            .map(build_clause_from_input)
            .collect();
        if legacy.is_empty() {
            Vec::new()
        } else {
            vec![SignalClauseGroup {
                intra_op: parse_combinator(custom.entry_combinator.as_deref()),
                clauses: legacy,
            }]
        }
    };

    let long_exit_groups = if custom.long_exit_groups.is_some() {
        build_groups_from_input(custom.long_exit_groups.as_deref())
    } else {
        let legacy: Vec<SignalClause> = custom
            .exit_clauses
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter(|c| !c.catalog_id.trim().is_empty())
            .map(build_clause_from_input)
            .collect();
        if legacy.is_empty() {
            Vec::new()
        } else {
            vec![SignalClauseGroup {
                intra_op: parse_combinator(custom.exit_combinator.as_deref()),
                clauses: legacy,
            }]
        }
    };

    let short_entry_groups = build_groups_from_input(custom.short_entry_groups.as_deref());
    let short_exit_groups = build_groups_from_input(custom.short_exit_groups.as_deref());

    let long_entry = DirectionalRuleLeg {
        enabled: !long_entry_groups.is_empty() || gate.enabled,
        inter_group_op: parse_combinator(custom.long_entry_inter_op.as_deref()),
        groups: long_entry_groups,
        confluence_gate: gate,
    };
    let long_exit = DirectionalRuleLeg {
        enabled: !long_exit_groups.is_empty(),
        inter_group_op: parse_combinator(custom.long_exit_inter_op.as_deref()),
        groups: long_exit_groups,
        confluence_gate: ConfluenceThresholdGate::default(),
    };
    let short_entry = DirectionalRuleLeg {
        enabled: !short_entry_groups.is_empty(),
        inter_group_op: parse_combinator(custom.short_entry_inter_op.as_deref()),
        groups: short_entry_groups,
        confluence_gate: ConfluenceThresholdGate::default(),
    };
    let short_exit = DirectionalRuleLeg {
        enabled: !short_exit_groups.is_empty(),
        inter_group_op: parse_combinator(custom.short_exit_inter_op.as_deref()),
        groups: short_exit_groups,
        confluence_gate: ConfluenceThresholdGate::default(),
    };

    let mut spec = CustomSignalRuleSpec {
        name: custom
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Chiến lược tự thiết kế (123 Tín hiệu)")
            .to_owned(),
        long_entry,
        long_exit,
        short_entry,
        short_exit,
    };
    spec.sanitize();
    spec
}

fn build_backtest_config(input: &StrategyBacktestInput) -> BacktestConfig {
    let preset = match input.preset.trim().to_ascii_lowercase().as_str() {
        "qmv_s1" | "qmv_s1_support_vpa" | "qmv_support_vpa" => StrategyPresetKind::QmvSupportVpa,
        "qmv_s2" | "qmv_s2_safe_bottom" | "qmv_safe_bottom_div" => {
            StrategyPresetKind::QmvSafeBottomDiv
        }
        "qmv_s3" | "qmv_s3_ichimoku_v7" | "qmv_ichimoku_v7" => StrategyPresetKind::QmvIchimokuV7,
        "qmv_s4" | "qmv_s4_dual_cycle_flow" | "qmv_dual_cycle_flow" => {
            StrategyPresetKind::QmvDualCycleFlow
        }
        "qmv_s5" | "qmv_s5_congestion_breakout" | "qmv_congestion_breakout" => {
            StrategyPresetKind::QmvCongestionBreakout
        }
        "qmv_s6" | "qmv_s6_multigate" | "qmv_multigate_confluence" => {
            StrategyPresetKind::QmvMultiGateConfluence
        }
        "rsi_mean_rev" | "rsi" => StrategyPresetKind::RsiMeanReversion,
        "macd" => StrategyPresetKind::MacdMomentum,
        "bb_squeeze" | "bb" => StrategyPresetKind::BollingerSqueezeBreakout,
        "donchian_atr" | "donchian" => StrategyPresetKind::DonchianAtrBreakout,
        "rsi_bb" | "confluence" => StrategyPresetKind::RsiBollingerConfluence,
        "custom" | "custom_rules" => StrategyPresetKind::CustomSignalRules,
        _ => StrategyPresetKind::SmaEmaTrendCross,
    };
    let direction_mode = match input.direction.trim().to_ascii_lowercase().as_str() {
        "short" => StrategyDirectionMode::ShortOnly,
        "both" | "long_short" => StrategyDirectionMode::LongShort,
        _ => StrategyDirectionMode::LongOnly,
    };

    let mut params = StrategyParams::default();
    if let Some(v) = input.fast_sma {
        params.fast_sma = v;
    }
    if let Some(v) = input.slow_ema {
        params.slow_ema = v;
    }
    if let Some(v) = input.rsi_period {
        params.rsi_period = v;
    }
    if let Some(v) = input.rsi_oversold {
        params.rsi_oversold = v;
    }
    if let Some(v) = input.rsi_overbought {
        params.rsi_overbought = v;
    }
    if let Some(v) = input.rsi_confluence_buy {
        params.rsi_confluence_buy = v;
    }
    if let Some(v) = input.rsi_confluence_sell {
        params.rsi_confluence_sell = v;
    }
    if let Some(v) = input.macd_fast {
        params.macd_fast = v;
    }
    if let Some(v) = input.macd_slow {
        params.macd_slow = v;
    }
    if let Some(v) = input.macd_signal {
        params.macd_signal = v;
    }
    if let Some(v) = input.macd_trend_ema {
        params.macd_trend_ema = v;
    }
    if let Some(v) = input.bb_period {
        params.bb_period = v;
    }
    if let Some(v) = input.bb_std_mult {
        params.bb_std_mult = v;
    }
    if let Some(v) = input.max_squeeze_bw_pct {
        params.max_squeeze_bw_pct = v;
    }
    if let Some(v) = input.donchian_entry {
        params.donchian_entry_period = v;
    }
    if let Some(v) = input.donchian_exit {
        params.donchian_exit_period = v;
    }
    if let Some(v) = input.qmv_support_window {
        params.qmv_support_window = v;
    }
    if let Some(v) = input.qmv_rvol_confirm {
        params.qmv_rvol_confirm = v;
    }
    if let Some(v) = input.qmv_trail_mode {
        params.qmv_trail_mode = v;
    }
    if let Some(v) = input.qmv_div_osc {
        params.qmv_div_osc = v;
    }
    if let Some(v) = input.qmv_confirm_mode {
        params.qmv_confirm_mode = v;
    }
    if let Some(v) = input.qmv_ichi_variant {
        params.qmv_ichi_variant = v;
    }
    if let Some(v) = input.qmv_flow_horizon_pair {
        params.qmv_flow_horizon_pair = v;
    }
    if let Some(v) = input.qmv_flow_low_zone {
        params.qmv_flow_low_zone = v;
    }
    if let Some(v) = input.qmv_flow_high_zone {
        params.qmv_flow_high_zone = v;
    }
    if let Some(v) = input.qmv_use_vnindex_gate {
        params.qmv_use_vnindex_gate = v;
    }
    if let Some(v) = input.qmv_min_pillars {
        params.qmv_min_pillars = v;
    }

    let is_qmv_preset = StrategyPresetKind::QMV_PLAYBOOKS.contains(&preset);
    let default_sup_lb = is_qmv_preset.then_some(18);
    let default_wave_trail = is_qmv_preset && params.qmv_trail_mode == 0;

    let risk = StopTakeConfig {
        stop_loss_pct: input.stop_loss_pct.filter(|v| v.is_finite() && *v > 0.0),
        take_profit_pct: input.take_profit_pct.filter(|v| v.is_finite() && *v > 0.0),
        trailing_stop_atr_mult: input
            .trailing_stop_atr_mult
            .filter(|v| v.is_finite() && *v > 0.0),
        min_holding_bars: input.min_holding_bars.unwrap_or(0),
        support_stop_lookback: input.support_stop_lookback.or(default_sup_lb),
        qmv_wave_trailing_2_3: input.qmv_wave_trailing_2_3.unwrap_or(default_wave_trail),
        max_holding_bars: is_qmv_preset.then_some(25),
        max_pyramiding: input.max_pyramiding.unwrap_or(1),
        scale_out_pct: input.scale_out_pct.filter(|v| v.is_finite() && *v > 0.0),
        tp1_pct: input.tp1_pct.filter(|v| v.is_finite() && *v > 0.0),
        trailing_ma_period: input.trailing_ma_period.filter(|&p| p >= 5),
        trailing_kijun_period: input.trailing_kijun_period.filter(|&p| p >= 5),
        use_bar_magnifier: input.use_bar_magnifier.unwrap_or(false),
        ..StopTakeConfig::default()
    };

    let upper_ticker = input.ticker.to_ascii_uppercase();
    let is_vn30f = upper_ticker.contains("VN30F") || upper_ticker.contains("VN301!");
    let venue = sumi_core::journal::ExchangeVenue::infer_from_symbol_and_preset(
        &input.ticker,
        DataSourcePreset::CustomFile,
    );
    let max_leverage = input
        .max_leverage
        .filter(|v| v.is_finite() && *v >= 1.0)
        .unwrap_or(if is_vn30f { 5.88 } else { 1.0 })
        .clamp(1.0, 20.0);

    let custom_rules = input
        .custom_rules
        .as_ref()
        .map(build_custom_rule_spec)
        .unwrap_or_default();

    let mut cfg = BacktestConfig {
        preset,
        params,
        custom_rules,
        direction_mode,
        sizing: PositionSizingMode::PercentEquity(input.position_size_pct.clamp(1.0, 100.0)),
        risk,
        initial_cash: input.initial_cash,
        commission_bps: input.commission_bps,
        tax_sell_bps: input.tax_sell_bps,
        slippage_bps: input.slippage_bps,
        max_leverage,
        lot_step: input.lot_step.max(0.0),
        in_sample_split_pct: input.in_sample_split_pct,
        enforce_vn_microstructure: venue.is_vn_cash_equity(),
        min_entry_avg_vol_20: if venue.is_vn_cash_equity() && is_qmv_preset {
            10_000.0
        } else {
            0.0
        },
        benchmark_non_downtrend_days: None,
    };
    cfg.sanitize();
    cfg
}

fn load_vnindex_non_downtrend_days(state: &AppState) -> Option<std::sync::Arc<std::collections::BTreeSet<i64>>> {
    let key = SymbolKey::parse("HOSE:VNINDEX").ok()?;
    let series = SeriesKey::new(&key, Interval::D1, Adjustment::Splits);
    let res = state
        .market_store
        .query_bars(&series, sumi_data::BarQueryMode::Init, 8_000)
        .ok()?;
    if res.bars.len() < 60 {
        return None;
    }
    let candles: Vec<Candle> = res
        .bars
        .iter()
        .map(|b| b.to_core_candle(Interval::D1))
        .collect();
    let regimes = sumi_core::scanner::compute_regime_series(&candles);
    let mut set = std::collections::BTreeSet::new();
    for (c, reg) in candles.iter().zip(regimes) {
        if reg != sumi_core::scanner::MarketRegime::Downtrend
            && reg != sumi_core::scanner::MarketRegime::InsufficientHistory
        {
            set.insert(c.timestamp.div_euclid(86_400));
        }
    }
    Some(std::sync::Arc::new(set))
}

pub(crate) fn filter_bars_by_range(
    bars: Vec<OhlcvBar>,
    from_timestamp_ms: Option<i64>,
    to_timestamp_ms: Option<i64>,
    max_bars: usize,
) -> DesktopResult<Vec<OhlcvBar>> {
    if let (Some(from_ms), Some(to_ms)) = (from_timestamp_ms, to_timestamp_ms)
        && from_ms > to_ms
    {
        return Err(DesktopError::Validation(
            "Khoảng thời gian Backtest không hợp lệ: 'Từ ngày' phải nhỏ hơn hoặc bằng 'Đến ngày'"
                .to_owned(),
        ));
    }

    let mut filtered: Vec<OhlcvBar> = bars
        .into_iter()
        .filter(|b| {
            let after_from = from_timestamp_ms.is_none_or(|f| b.timestamp >= f);
            let before_to = to_timestamp_ms.is_none_or(|t| b.timestamp <= t);
            after_from && before_to
        })
        .collect();

    if filtered.len() > max_bars {
        let start = filtered.len() - max_bars;
        filtered.drain(..start);
    }
    Ok(filtered)
}

async fn load_backtest_candles(
    state: &AppState,
    ticker: &str,
    interval_str: &str,
    max_bars: usize,
    from_timestamp_ms: Option<i64>,
    to_timestamp_ms: Option<i64>,
) -> DesktopResult<(String, Interval, Timeframe, Vec<Candle>)> {
    let sym = SymbolKey::parse(ticker)?;
    let iv: Interval = interval_str.parse()?;
    let tf = iv.to_core_timeframe().unwrap_or(Timeframe::D1);
    let series = SeriesKey::new(&sym, iv, Adjustment::Splits);
    let svc = state.active_market_service();

    let fetch_limit = if from_timestamp_ms.is_some() || to_timestamp_ms.is_some() {
        max_bars.max(6_000)
    } else {
        max_bars
    };

    let mut bars = svc
        .get_bars_instant(&BarsRequest::init(series.clone(), fetch_limit))
        .await?
        .bars;
    if bars.is_empty() {
        bars = svc
            .get_bars(&BarsRequest::init(series, max_bars.min(1_500)))
            .await?
            .bars;
    }

    let bars = filter_bars_by_range(bars, from_timestamp_ms, to_timestamp_ms, max_bars)?;

    if bars.len() < 30 {
        return Err(DesktopError::Validation(format!(
            "Không đủ lịch sử nến trong khoảng thời gian đã chọn cho {} (có {} nến, cần >= 30)",
            sym.ticker(),
            bars.len()
        )));
    }
    let candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
    Ok((sym.ticker(), iv, tf, candles))
}

fn map_equity_points(pts: &[BacktestEquityPoint]) -> Vec<StrategyEquityPointDto> {
    // Downsample gently if > 800 points so SVG/canvas equity charts stay ultra-snappy.
    let step = (pts.len() / 800).max(1);
    pts.iter()
        .enumerate()
        .filter(|(i, _)| i % step == 0 || *i + 1 == pts.len())
        .map(|(_, p)| StrategyEquityPointDto {
            bar_index: p.bar_index,
            timestamp_ms: p.timestamp.saturating_mul(1_000),
            equity: p.equity,
            underwater_pct: p.underwater_pct,
        })
        .collect()
}

fn map_trades(trades: &[BacktestTrade]) -> Vec<StrategyTradeDto> {
    trades
        .iter()
        .map(|t| StrategyTradeDto {
            trade_id: t.trade_id,
            side: t.side.short_label().to_owned(),
            quantity: t.quantity,
            entry_bar_index: t.entry_bar_index,
            exit_bar_index: t.exit_bar_index,
            entry_timestamp_ms: t.entry_timestamp.saturating_mul(1_000),
            exit_timestamp_ms: t.exit_timestamp.saturating_mul(1_000),
            entry_price: t.entry_price,
            exit_price: t.exit_price,
            bars_held: t.bars_held,
            gross_pnl: t.gross_pnl,
            net_pnl: t.net_pnl,
            return_pct: t.return_pct,
            r_multiple: t.r_multiple,
            mae_pct: t.mae_pct,
            mfe_pct: t.mfe_pct,
            exit_reason: t.exit_reason.short_label().to_owned(),
        })
        .collect()
}

fn compute_monthly_returns(
    equity_curve: &[BacktestEquityPoint],
    trades: &[BacktestTrade],
) -> Vec<MonthlyReturnCellDto> {
    use std::collections::BTreeMap;
    let Some(ict) = FixedOffset::east_opt(7 * 3600) else {
        return Vec::new();
    };
    let mut trade_counts: BTreeMap<(i32, u32), usize> = BTreeMap::new();
    for tr in trades {
        if let Some(dt) = ict.timestamp_opt(tr.exit_timestamp, 0).single() {
            *trade_counts.entry((dt.year(), dt.month())).or_insert(0) += 1;
        }
    }
    let mut month_bounds: BTreeMap<(i32, u32), (f64, f64)> = BTreeMap::new();
    let mut prev_eq: Option<f64> = None;
    for pt in equity_curve {
        let Some(dt) = ict.timestamp_opt(pt.timestamp, 0).single() else {
            continue;
        };
        let ym = (dt.year(), dt.month());
        let start_eq = prev_eq.unwrap_or(pt.equity);
        month_bounds
            .entry(ym)
            .and_modify(|e| e.1 = pt.equity)
            .or_insert((start_eq, pt.equity));
        prev_eq = Some(pt.equity);
    }
    month_bounds
        .into_iter()
        .map(|((year, month), (start_eq, end_eq))| {
            let ret = if start_eq > 1.0e-6 {
                ((end_eq - start_eq) / start_eq) * 100.0
            } else {
                0.0
            };
            MonthlyReturnCellDto {
                year,
                month,
                return_pct: (ret * 100.0).round() / 100.0,
                trades_count: trade_counts.get(&(year, month)).copied().unwrap_or(0),
            }
        })
        .collect()
}

fn load_intrabar_magnifier_candles(
    state: &AppState,
    ticker: &str,
    iv: Interval,
    outer_candles: &[Candle],
) -> Vec<Candle> {
    if iv != Interval::D1 || outer_candles.is_empty() {
        return Vec::new();
    }
    let Ok(sym) = SymbolKey::parse(ticker) else {
        return Vec::new();
    };
    let first_ms = outer_candles
        .first()
        .map_or(0, |c| c.timestamp.saturating_mul(1_000));
    let last_ms = outer_candles
        .last()
        .map_or(i64::MAX, |c| c.timestamp.saturating_add(86_400).saturating_mul(1_000));

    for sub_iv in [Interval::H1, Interval::M15] {
        let series = SeriesKey::new(&sym, sub_iv, Adjustment::Splits);
        if let Ok(page) = state
            .market_store
            .query_bars(&series, BarQueryMode::Init, 20_000)
            && !page.bars.is_empty()
        {
            let filtered: Vec<Candle> = page
                .bars
                .into_iter()
                .filter(|b| b.timestamp >= first_ms && b.timestamp <= last_ms)
                .map(|b| b.to_core_candle(sub_iv))
                .collect();
            if !filtered.is_empty() {
                return filtered;
            }
        }
    }
    Vec::new()
}

#[tauri::command]
pub async fn run_strategy_backtest(
    state: State<'_, AppState>,
    input: StrategyBacktestInput,
) -> DesktopResult<StrategyBacktestResponse> {
    let max_bars = input.max_bars.unwrap_or(2_500).clamp(60, 10_000);
    let (ticker, iv, tf, candles) = load_backtest_candles(
        &state,
        &input.ticker,
        &input.interval,
        max_bars,
        input.from_timestamp_ms,
        input.to_timestamp_ms,
    )
    .await?;
    let mut cfg = build_backtest_config(&input);
    if cfg.enforce_vn_microstructure {
        cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    }
    let intrabar_candles = if cfg.risk.use_bar_magnifier {
        load_intrabar_magnifier_candles(&state, &ticker, iv, &candles)
    } else {
        Vec::new()
    };
    let magnifier_bars = intrabar_candles.len();

    let report = tokio::task::spawn_blocking(move || {
        let sub_slice = (!intrabar_candles.is_empty()).then_some(intrabar_candles.as_slice());
        run_backtest_with_magnifier(
            ticker.clone(),
            tf,
            &candles,
            &cfg,
            BacktestRunOptions::default(),
            sub_slice,
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    let preset_label = if report.config.preset == StrategyPresetKind::CustomSignalRules
        && !report.config.custom_rules.name.trim().is_empty()
    {
        report.config.custom_rules.name.clone()
    } else {
        report.config.preset.full_label().to_owned()
    };
    let monthly_returns = compute_monthly_returns(&report.equity_curve, &report.trades);

    Ok(StrategyBacktestResponse {
        ticker: report.symbol,
        interval: iv.as_str().to_owned(),
        preset_label,
        bar_count: report.bar_count,
        tearsheet: report.tearsheet,
        in_sample_tearsheet: report.in_sample_tearsheet,
        out_of_sample_tearsheet: report.out_of_sample_tearsheet,
        robustness: report.robustness,
        open_position: report.open_position,
        equity_curve: map_equity_points(&report.equity_curve),
        trades: map_trades(&report.trades),
        monthly_returns,
        bar_magnifier_bars: magnifier_bars,
    })
}

#[tauri::command]
pub async fn run_strategy_wfo(
    state: State<'_, AppState>,
    input: WalkForwardInput,
) -> DesktopResult<WalkForwardResponse> {
    let max_bars = input.backtest.max_bars.unwrap_or(2_500).clamp(120, 10_000);
    let (ticker, iv, tf, candles) = load_backtest_candles(
        &state,
        &input.backtest.ticker,
        &input.backtest.interval,
        max_bars,
        input.backtest.from_timestamp_ms,
        input.backtest.to_timestamp_ms,
    )
    .await?;
    let mut cfg = build_backtest_config(&input.backtest);
    if cfg.enforce_vn_microstructure {
        cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    }
    let wf_cfg = WalkForwardConfig {
        fold_count: input.fold_count.clamp(2, 8),
        in_sample_ratio: input.in_sample_ratio.clamp(0.50, 0.85),
        anchored: input.anchored,
        ranking_metric: SweepRankingMetric::SharpeRatio,
    };

    let report = tokio::task::spawn_blocking(move || {
        run_walk_forward_validation(
            ticker,
            tf,
            &candles,
            &cfg,
            wf_cfg,
            BacktestRunOptions::default(),
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    Ok(WalkForwardResponse {
        ticker: report.symbol,
        interval: iv.as_str().to_owned(),
        total_bars: report.total_bars,
        profitable_folds: report.profitable_folds,
        fold_count: report.folds.len(),
        wfe_pct: report.wfe_pct,
        param_stability_pct: report.param_stability_pct,
        verdict: report.verdict,
        verdict_label: report.verdict.label().to_owned(),
        stitched_oos_tearsheet: report.stitched_oos_tearsheet,
        stitched_oos_curve: map_equity_points(&report.stitched_oos_curve),
        folds: report.folds,
    })
}

#[tauri::command]
pub async fn run_strategy_monte_carlo(
    state: State<'_, AppState>,
    input: MonteCarloInput,
) -> DesktopResult<MonteCarloResponse> {
    let max_bars = input.backtest.max_bars.unwrap_or(2_500).clamp(60, 10_000);
    let (ticker, _iv, tf, candles) = load_backtest_candles(
        &state,
        &input.backtest.ticker,
        &input.backtest.interval,
        max_bars,
        input.backtest.from_timestamp_ms,
        input.backtest.to_timestamp_ms,
    )
    .await?;
    let mut cfg = build_backtest_config(&input.backtest);
    if cfg.enforce_vn_microstructure {
        cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    }
    let mc_cfg = MonteCarloConfig {
        simulations: input.simulations,
        seed: input.seed.unwrap_or(DEFAULT_MONTE_CARLO_SEED),
        ruin_drawdown_pct: input.ruin_drawdown_pct,
        horizon_steps: None,
    }
    .sanitized();

    let report = tokio::task::spawn_blocking(move || {
        let bt =
            run_backtest_with_options(ticker, tf, &candles, &cfg, BacktestRunOptions::default());
        run_monte_carlo_bootstrap(
            cfg.initial_cash,
            &bt.trades,
            &bt.equity_curve,
            candles.len(),
            None,
            mc_cfg,
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    Ok(MonteCarloResponse {
        simulations: report.config.simulations,
        seed: report.config.seed,
        sample_count: report.sample_count,
        horizon_steps: report.horizon_steps,
        validity: report.validity,
        low_sample_warning: report.low_sample_warning,
        status_reason: report.status_reason,
        metrics: report.metrics,
        checkpoint_envelopes: report.checkpoint_envelopes,
    })
}

// ─── Portfolio Multi-Symbol Backtest (v2.2) ──────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioBacktestInput {
    pub backtest: StrategyBacktestInput,
    pub tickers: Vec<String>,
    /// Optional QMV basket shortcut (`"QMV30"`, `"QMV60"`, `"CORE10"`, `"DT1"`, `"DT2"`, `"SECTOR:BANKING"`, ...).
    #[serde(default)]
    pub qmv_group: Option<String>,
    /// `"equal"` (default 1/N) | `"inverse_vol"`
    pub weighting: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioBacktestResponse {
    pub preset_label: String,
    pub interval: String,
    pub weighting_label: String,
    pub constituent_count: usize,
    pub initial_capital: f64,
    pub tearsheet: BacktestTearsheet,
    pub avg_constituent_max_dd_pct: f64,
    pub diversification_dd_reduction_pct: f64,
    pub equity_curve: Vec<StrategyEquityPointDto>,
    pub constituents: Vec<PortfolioConstituentAttribution>,
    pub survivor_bias_caveat: Option<String>,
}

#[tauri::command]
pub async fn run_strategy_portfolio(
    state: State<'_, AppState>,
    input: PortfolioBacktestInput,
) -> DesktopResult<PortfolioBacktestResponse> {
    let mut resolved_tickers = input.tickers.clone();
    if let Some(ref grp_raw) = input.qmv_group {
        let grp = grp_raw.trim().to_ascii_uppercase();
        if !grp.is_empty() && grp != "CUSTOM" {
            let hits = state
                .market_store
                .search_local_symbols("", None, 2_000)
                .unwrap_or_default();
            let mut matched = Vec::new();
            for h in hits {
                if !matches!(h.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
                    || h.market_type == "index"
                    || h.symbol.ends_with("F1!")
                {
                    continue;
                }
                let Some(prof) = sumi_core::scanner::lookup_qmv_profile(&h.symbol) else {
                    continue;
                };
                let ok = match grp.as_str() {
                    "QMV30" => prof.in_qmv30,
                    "QMV60" => prof.in_qmv60,
                    "CORE10" => prof.in_yumeko_core10,
                    "DT1" => prof.money_tier == sumi_core::scanner::QmvMoneyTier::Dt1,
                    "DT2" => prof.money_tier == sumi_core::scanner::QmvMoneyTier::Dt2,
                    "DT3" => prof.money_tier == sumi_core::scanner::QmvMoneyTier::Dt3,
                    s if s.starts_with("SECTOR:") => {
                        sumi_core::scanner::QmvSector::parse_str(&s[7..])
                            .is_some_and(|sec| prof.sector == sec)
                    }
                    s if s.starts_with("GROUP:") => {
                        sumi_core::scanner::QmvSectorGroup::parse_str(&s[6..])
                            .is_some_and(|sg| prof.sector_group == sg)
                    }
                    _ => false,
                };
                if ok {
                    matched.push(h.ticker);
                }
            }
            if !matched.is_empty() {
                resolved_tickers = matched;
            }
        }
    }

    if resolved_tickers.is_empty() {
        return Err(DesktopError::Validation(
            "Cần tối thiểu 1 mã trong danh mục để chạy Portfolio Backtest".to_owned(),
        ));
    }

    let max_bars = input.backtest.max_bars.unwrap_or(1_200).clamp(60, 5_000);
    let iv: Interval = input.backtest.interval.parse()?;
    let tf = iv.to_core_timeframe().unwrap_or(Timeframe::D1);
    let mut cfg = build_backtest_config(&input.backtest);
    cfg.enforce_vn_microstructure = true;
    cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    let weighting = match input
        .weighting
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("inverse_vol" | "inv_vol") => PortfolioWeightingMode::CausalInverseVolatility {
            lookback_bars: 20,
        },
        _ => PortfolioWeightingMode::EqualWeight,
    };

    let mut feeds: Vec<WatchlistFeedEntry> = Vec::new();
    for raw_ticker in resolved_tickers.iter().take(80) {
        if let Ok((norm_ticker, _, _, candles)) = load_backtest_candles(
            &state,
            raw_ticker,
            &input.backtest.interval,
            max_bars,
            input.backtest.from_timestamp_ms,
            input.backtest.to_timestamp_ms,
        )
        .await
        {
            let short_sym = norm_ticker
                .split_once(':')
                .map_or(norm_ticker.as_str(), |(_, s)| s)
                .to_owned();
            feeds.push(WatchlistFeedEntry {
                instrument_id: norm_ticker,
                preset: DataSourcePreset::CustomFile,
                symbol: short_sym,
                base_timeframe: tf,
                candles,
            });
        }
    }

    if feeds.is_empty() {
        return Err(DesktopError::Validation(
            "Không có mã nào trong danh mục có đủ tối thiểu 30 cây nến để kiểm định".to_owned(),
        ));
    }

    let report = tokio::task::spawn_blocking(move || {
        run_portfolio_backtest(
            &feeds,
            tf,
            &cfg,
            PortfolioUniverseFilter::AllUniverseFeeds,
            weighting,
            None,
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    let equity_curve = map_equity_points(&report.portfolio_equity_curve);

    Ok(PortfolioBacktestResponse {
        preset_label: report.preset.full_label().to_owned(),
        interval: iv.as_str().to_owned(),
        weighting_label: report.weighting.short_label().to_owned(),
        constituent_count: report.constituents.len(),
        initial_capital: report.initial_capital,
        tearsheet: report.portfolio_tearsheet,
        avg_constituent_max_dd_pct: report.avg_constituent_max_dd_pct,
        diversification_dd_reduction_pct: report.diversification_dd_reduction_pct,
        equity_curve,
        constituents: report.constituents,
        survivor_bias_caveat: report.survivor_bias_caveat,
    })
}

// ─── Wave 3: HTML/PDF Report Export, Parameter Sweep & All-Presets Battle ────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBacktestHtmlResponse {
    pub file_path: String,
    pub html: String,
}

fn parse_ranking_metric(raw: Option<&str>) -> SweepRankingMetric {
    match raw
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("return" | "total_return") => SweepRankingMetric::TotalReturnPct,
        Some("calmar") => SweepRankingMetric::CalmarRatio,
        Some("profit_factor" | "pf") => SweepRankingMetric::ProfitFactor,
        _ => SweepRankingMetric::SharpeRatio,
    }
}

/// Renders an institutional offline-first HTML/Print-to-PDF Backtest Report (`render_backtest_report_html`),
/// writes it to `<SUMI_DATA_DIR>/reports/`, and returns both `filePath` and `html`.
#[tauri::command]
pub async fn export_backtest_report_html(
    state: State<'_, AppState>,
    input: StrategyBacktestInput,
) -> DesktopResult<ExportBacktestHtmlResponse> {
    let max_bars = input.max_bars.unwrap_or(2_500).clamp(60, 5_000);
    let (ticker, iv, tf, candles) = load_backtest_candles(
        &state,
        &input.ticker,
        &input.interval,
        max_bars,
        input.from_timestamp_ms,
        input.to_timestamp_ms,
    )
    .await?;
    let mut cfg = build_backtest_config(&input);
    if cfg.enforce_vn_microstructure {
        cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    }
    let venue = sumi_core::journal::ExchangeVenue::infer_from_symbol_and_preset(
        &ticker,
        DataSourcePreset::CustomFile,
    );
    let data_dir = state.data_dir.clone();

    tokio::task::spawn_blocking(move || {
        let res = run_backtest_with_options(
            ticker.clone(),
            tf,
            &candles,
            &cfg,
            BacktestRunOptions::default(),
        );
        let analytics = compute_advanced_tearsheet_analytics(
            &candles,
            &res,
            sumi_core::backtest::MultiPhaseSplitMode::default(),
            BacktestRunOptions::default(),
        );
        let html = render_backtest_report_html(&res, &candles, venue, Some(&analytics));
        let reports_dir = data_dir.join("reports");
        std::fs::create_dir_all(&reports_dir)?;
        let safe_sym = ticker.replace(':', "_").replace('!', "");
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let file_path = reports_dir.join(format!(
            "sumi_backtest_{}_{}_{}.html",
            safe_sym,
            iv.as_str(),
            ts
        ));
        std::fs::write(&file_path, &html)?;
        Ok(ExportBacktestHtmlResponse {
            file_path: file_path.display().to_string(),
            html,
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterSweepResponseDto {
    pub ticker: String,
    pub interval: String,
    pub preset_label: String,
    pub ranking_label: String,
    pub variants: Vec<SweepVariantRow>,
}

/// Runs a deterministic parameter grid sweep across variants of the active strategy preset.
#[tauri::command]
pub async fn run_strategy_parameter_sweep(
    state: State<'_, AppState>,
    input: StrategyBacktestInput,
    ranking_metric: Option<String>,
) -> DesktopResult<ParameterSweepResponseDto> {
    let max_bars = input.max_bars.unwrap_or(2_500).clamp(60, 5_000);
    let (ticker, iv, tf, candles) = load_backtest_candles(
        &state,
        &input.ticker,
        &input.interval,
        max_bars,
        input.from_timestamp_ms,
        input.to_timestamp_ms,
    )
    .await?;
    let mut cfg = build_backtest_config(&input);
    if cfg.enforce_vn_microstructure {
        cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    }
    let metric = parse_ranking_metric(ranking_metric.as_deref());

    let report = tokio::task::spawn_blocking(move || {
        run_parameter_sweep(
            ticker,
            tf,
            &candles,
            &cfg,
            metric,
            BacktestRunOptions::default(),
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    Ok(ParameterSweepResponseDto {
        ticker: report.symbol,
        interval: iv.as_str().to_owned(),
        preset_label: report.preset.full_label().to_owned(),
        ranking_label: format!("{:?}", report.ranking_metric),
        variants: report.variants,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetBattleItemDto {
    pub rank: usize,
    pub preset_slug: String,
    pub preset_label: String,
    pub param_summary: String,
    pub tearsheet: BacktestTearsheet,
    pub robustness: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetsBattleResponseDto {
    pub ticker: String,
    pub interval: String,
    pub ranking_label: String,
    pub rows: Vec<PresetBattleItemDto>,
}

fn preset_kind_to_slug(kind: StrategyPresetKind) -> &'static str {
    match kind {
        StrategyPresetKind::QmvSupportVpa => "qmv_s1",
        StrategyPresetKind::QmvSafeBottomDiv => "qmv_s2",
        StrategyPresetKind::QmvIchimokuV7 => "qmv_s3",
        StrategyPresetKind::QmvDualCycleFlow => "qmv_s4",
        StrategyPresetKind::QmvCongestionBreakout => "qmv_s5",
        StrategyPresetKind::QmvMultiGateConfluence => "qmv_s6",
        StrategyPresetKind::SmaEmaTrendCross => "sma_ema",
        StrategyPresetKind::RsiMeanReversion => "rsi_mean_rev",
        StrategyPresetKind::MacdMomentum => "macd",
        StrategyPresetKind::BollingerSqueezeBreakout => "bb_squeeze",
        StrategyPresetKind::DonchianAtrBreakout => "donchian_atr",
        StrategyPresetKind::RsiBollingerConfluence => "rsi_bb",
        StrategyPresetKind::CustomSignalRules => "custom",
    }
}

/// Runs head-to-head All-Presets Battle across all built-in QMV S1–S6 and Baseline strategies.
#[tauri::command]
pub async fn run_strategy_presets_battle(
    state: State<'_, AppState>,
    input: StrategyBacktestInput,
    ranking_metric: Option<String>,
) -> DesktopResult<PresetsBattleResponseDto> {
    let max_bars = input.max_bars.unwrap_or(2_500).clamp(60, 5_000);
    let (ticker, iv, tf, candles) = load_backtest_candles(
        &state,
        &input.ticker,
        &input.interval,
        max_bars,
        input.from_timestamp_ms,
        input.to_timestamp_ms,
    )
    .await?;
    let mut cfg = build_backtest_config(&input);
    if cfg.enforce_vn_microstructure {
        cfg.benchmark_non_downtrend_days = load_vnindex_non_downtrend_days(&state);
    }
    let metric = parse_ranking_metric(ranking_metric.as_deref());
    let ticker_out = ticker.clone();

    let rows: Vec<PresetBattleRow> = tokio::task::spawn_blocking(move || {
        run_all_presets_battle(
            ticker,
            tf,
            &candles,
            &cfg,
            metric,
            BacktestRunOptions::default(),
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    Ok(PresetsBattleResponseDto {
        ticker: ticker_out,
        interval: iv.as_str().to_owned(),
        ranking_label: format!("{metric:?}"),
        rows: rows
            .into_iter()
            .map(|r| PresetBattleItemDto {
                rank: r.rank,
                preset_slug: preset_kind_to_slug(r.preset).to_owned(),
                preset_label: r.preset.full_label().to_owned(),
                param_summary: r.param_summary,
                tearsheet: r.tearsheet,
                robustness: r.robustness.map(|v| v.label().to_owned()),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input(preset: &str) -> StrategyBacktestInput {
        StrategyBacktestInput {
            ticker: "HOSE:FPT".to_owned(),
            interval: "1D".to_owned(),
            preset: preset.to_owned(),
            direction: "long".to_owned(),
            initial_cash: 500_000_000.0,
            commission_bps: 15.0,
            tax_sell_bps: 10.0,
            slippage_bps: 5.0,
            lot_step: 100.0,
            position_size_pct: 95.0,
            stop_loss_pct: Some(7.0),
            take_profit_pct: Some(15.0),
            min_holding_bars: Some(2),
            fast_sma: Some(12),
            slow_ema: Some(34),
            rsi_period: Some(10),
            rsi_oversold: Some(25.0),
            rsi_overbought: Some(75.0),
            rsi_confluence_buy: Some(38.0),
            rsi_confluence_sell: Some(66.0),
            macd_fast: Some(8),
            macd_slow: Some(21),
            macd_signal: Some(5),
            macd_trend_ema: Some(100),
            bb_period: Some(18),
            bb_std_mult: Some(2.2),
            max_squeeze_bw_pct: Some(8.5),
            donchian_entry: Some(25),
            donchian_exit: Some(12),
            in_sample_split_pct: Some(70.0),
            max_bars: Some(500),
            from_timestamp_ms: None,
            to_timestamp_ms: None,
            max_leverage: None,
            custom_rules: None,
            qmv_support_window: None,
            qmv_rvol_confirm: None,
            qmv_trail_mode: None,
            qmv_div_osc: None,
            qmv_confirm_mode: None,
            qmv_ichi_variant: None,
            qmv_flow_horizon_pair: None,
            qmv_flow_low_zone: None,
            qmv_flow_high_zone: None,
            qmv_use_vnindex_gate: None,
            qmv_min_pillars: None,
            support_stop_lookback: None,
            qmv_wave_trailing_2_3: None,
            max_pyramiding: None,
            scale_out_pct: None,
            tp1_pct: None,
            trailing_stop_atr_mult: None,
            trailing_ma_period: None,
            trailing_kijun_period: None,
            use_bar_magnifier: None,
        }
    }

    #[test]
    fn build_backtest_config_wires_all_dynamic_strategy_params() {
        let cfg = build_backtest_config(&sample_input("macd"));
        assert_eq!(cfg.preset, StrategyPresetKind::MacdMomentum);
        assert_eq!(cfg.params.macd_fast, 8);
        assert_eq!(cfg.params.macd_slow, 21);
        assert_eq!(cfg.params.macd_signal, 5);
        assert_eq!(cfg.params.macd_trend_ema, 100);
        assert_eq!(cfg.params.bb_period, 18);
        assert!((cfg.params.max_squeeze_bw_pct - 8.5).abs() < 1e-6);
        assert_eq!(cfg.params.donchian_entry_period, 25);
        assert_eq!(cfg.params.donchian_exit_period, 12);
    }

    #[test]
    fn filter_bars_by_range_slices_dates_and_rejects_inverted_window() {
        let step = 86_400_000_i64;
        let base = 1_700_000_000_000_i64;
        let bars: Vec<OhlcvBar> = (0..100)
            .map(|i| OhlcvBar::new(base + i * step, 100.0, 105.0, 95.0, 102.0, 1_000.0))
            .collect();

        let sliced = filter_bars_by_range(
            bars.clone(),
            Some(base + 10 * step),
            Some(base + 59 * step),
            500,
        )
        .unwrap();
        assert_eq!(sliced.len(), 50);
        assert_eq!(sliced.first().unwrap().timestamp, base + 10 * step);
        assert_eq!(sliced.last().unwrap().timestamp, base + 59 * step);

        let err = filter_bars_by_range(bars, Some(base + 60 * step), Some(base + 10 * step), 500);
        assert!(err.is_err());
    }

    #[test]
    fn custom_signal_rules_and_vn30f_leverage_wire_cleanly() {
        let mut inp = sample_input("custom");
        inp.ticker = "HNX:VN30F1!".to_owned();
        inp.custom_rules = Some(CustomRulesInput {
            name: Some("MACD + RVOL".to_owned()),
            entry_combinator: Some("and".to_owned()),
            entry_clauses: vec![
                CustomClauseInput {
                    catalog_id: "technical.macd_bull_cross".to_owned(),
                    op: None,
                    threshold: None,
                    bool_expected: Some(true),
                },
                CustomClauseInput {
                    catalog_id: "volume.relative_volume".to_owned(),
                    op: Some(">=".to_owned()),
                    threshold: Some(1.5),
                    bool_expected: None,
                },
            ],
            exit_combinator: Some("or".to_owned()),
            exit_clauses: Some(vec![CustomClauseInput {
                catalog_id: "technical.macd_bear_cross".to_owned(),
                op: None,
                threshold: None,
                bool_expected: Some(true),
            }]),
            min_health_score: Some(10.0),
            long_entry_inter_op: None,
            long_entry_groups: None,
            long_exit_inter_op: None,
            long_exit_groups: None,
            short_entry_inter_op: None,
            short_entry_groups: None,
            short_exit_inter_op: None,
            short_exit_groups: None,
        });
        let cfg = build_backtest_config(&inp);
        assert_eq!(cfg.preset, StrategyPresetKind::CustomSignalRules);
        assert!((cfg.max_leverage - 5.88).abs() < 0.01);
        assert!(cfg.custom_rules.long_entry.enabled);
        assert_eq!(cfg.custom_rules.long_entry.clause_count(), 2);
        assert!(cfg.custom_rules.long_entry.confluence_gate.enabled);
        assert!(cfg.custom_rules.long_exit.enabled);
    }
}
