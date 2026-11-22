use serde::{Deserialize, Serialize};
use sumi_core::backtest::{
    BacktestConfig, BacktestEquityPoint, BacktestRunOptions, BacktestTearsheet, BacktestTrade,
    DEFAULT_MONTE_CARLO_SEED, MonteCarloConfig, MonteCarloRiskMetrics, MonteCarloValidity,
    OpenBacktestPosition, PositionSizingMode, QuantileEnvelope5, RobustnessVerdict, StopTakeConfig,
    StrategyDirectionMode, StrategyParams, StrategyPresetKind, SweepRankingMetric,
    WalkForwardConfig, WalkForwardFoldResult, WalkForwardVerdict, run_backtest_with_options,
    run_monte_carlo_bootstrap, run_walk_forward_validation,
};
use sumi_core::data::Timeframe;
use sumi_core::model::Candle;
use sumi_data::{Adjustment, BarsRequest, Interval, OhlcvBar, SeriesKey, SymbolKey};
use tauri::State;

use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyBacktestInput {
    pub ticker: String,
    pub interval: String,
    /// `"sma_ema"` | `"rsi_mean_rev"` | `"macd"` | `"bb_squeeze"` | `"donchian_atr"` | `"rsi_bb"`
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

fn build_backtest_config(input: &StrategyBacktestInput) -> BacktestConfig {
    let preset = match input.preset.trim().to_ascii_lowercase().as_str() {
        "rsi_mean_rev" | "rsi" => StrategyPresetKind::RsiMeanReversion,
        "macd" => StrategyPresetKind::MacdMomentum,
        "bb_squeeze" | "bb" => StrategyPresetKind::BollingerSqueezeBreakout,
        "donchian_atr" | "donchian" => StrategyPresetKind::DonchianAtrBreakout,
        "rsi_bb" | "confluence" => StrategyPresetKind::RsiBollingerConfluence,
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

    let risk = StopTakeConfig {
        stop_loss_pct: input.stop_loss_pct.filter(|v| v.is_finite() && *v > 0.0),
        take_profit_pct: input.take_profit_pct.filter(|v| v.is_finite() && *v > 0.0),
        min_holding_bars: input.min_holding_bars.unwrap_or(0),
        ..StopTakeConfig::default()
    };

    let mut cfg = BacktestConfig {
        preset,
        params,
        direction_mode,
        sizing: PositionSizingMode::PercentEquity(input.position_size_pct.clamp(1.0, 100.0)),
        risk,
        initial_cash: input.initial_cash,
        commission_bps: input.commission_bps,
        tax_sell_bps: input.tax_sell_bps,
        slippage_bps: input.slippage_bps,
        max_leverage: 1.0,
        lot_step: input.lot_step.max(0.0),
        in_sample_split_pct: input.in_sample_split_pct,
        ..BacktestConfig::default()
    };
    cfg.sanitize();
    cfg
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
    let cfg = build_backtest_config(&input);

    let report = tokio::task::spawn_blocking(move || {
        run_backtest_with_options(
            ticker.clone(),
            tf,
            &candles,
            &cfg,
            BacktestRunOptions::default(),
        )
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?;

    Ok(StrategyBacktestResponse {
        ticker: report.symbol,
        interval: iv.as_str().to_owned(),
        preset_label: report.config.preset.full_label().to_owned(),
        bar_count: report.bar_count,
        tearsheet: report.tearsheet,
        in_sample_tearsheet: report.in_sample_tearsheet,
        out_of_sample_tearsheet: report.out_of_sample_tearsheet,
        robustness: report.robustness,
        open_position: report.open_position,
        equity_curve: map_equity_points(&report.equity_curve),
        trades: map_trades(&report.trades),
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
    let cfg = build_backtest_config(&input.backtest);
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
    let cfg = build_backtest_config(&input.backtest);
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
}
