use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sumi_core::journal::{
    ExchangeVenue, FifoSettlementSummary, VnPriceUnit, causal_settlement_eval_timestamp,
    compute_daily_price_band,
};
use sumi_core::model::Candle;
use sumi_core::trading::{
    AccountConfig, ClosedTrade, EquityPoint, ExecutionFill, Order, OrderKind, OrderRequest,
    OrderSide, PortfolioMetrics, Position, PositionSide, TradingSession,
};
use sumi_data::{Adjustment, BarQueryMode, BarsRequest, Interval, OhlcvBar, SeriesKey, SymbolKey};
use tauri::State;

use crate::commands::market::{
    VnBandLevels, compute_vn_band_from_bars, synthesize_fallback_symbol,
};
use crate::error::{DesktopError, DesktopResult};
use crate::state::{AppState, PaperBrokerRuntime, PersistedBrokerSnapshot};
use crate::workspace_db::JournalAnnotationRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedOrder {
    pub order: Order,
    pub vn_order_type: String,
    pub submitted_timestamp_ms: i64,
    pub filled_timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedClosedTrade {
    pub trade: ClosedTrade,
    pub entry_timestamp_ms: i64,
    pub exit_timestamp_ms: i64,
    pub mae_pct: f64,
    pub mfe_pct: f64,
    pub setup_tag: String,
    pub mistake_tag: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokerStateSnapshot {
    pub ticker: String,
    pub interval: String,
    pub venue: ExchangeVenue,
    pub board_lot: f64,
    pub contract_multiplier: f64,
    pub allows_short: bool,
    pub enforce_vn_t2_lot_lock: bool,
    pub mark_price: f64,
    pub mark_timestamp_ms: i64,
    pub vn_band: Option<VnBandLevels>,
    pub config: AccountConfig,
    pub metrics: PortfolioMetrics,
    pub open_position: Option<Position>,
    pub orders: Vec<EnrichedOrder>,
    pub fills: Vec<ExecutionFill>,
    pub fifo_summary: FifoSettlementSummary,
    pub closed_trades: Vec<EnrichedClosedTrade>,
    pub equity_curve: Vec<EquityPoint>,
    pub replay_linked: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPaperOrderInput {
    pub ticker: String,
    pub interval: String,
    /// `"BUY"` | `"SELL"`
    pub side: String,
    /// `"LO"` | `"MP"` | `"ATO"` | `"ATC"` | `"STOP"`
    pub vn_order_type: String,
    pub quantity: f64,
    pub price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

fn infer_venue_for_ticker(ticker: &str) -> ExchangeVenue {
    let upper = ticker.trim().to_ascii_uppercase();
    if let Some((ex, sym)) = upper.split_once(':') {
        if sym.starts_with("VN30F") || sym.starts_with("VN301") {
            return ExchangeVenue::Vn30Futures;
        }
        return match ex {
            "HOSE" | "HSX" => ExchangeVenue::Hose,
            "HNX" => ExchangeVenue::Hnx,
            "UPCOM" => ExchangeVenue::Upcom,
            "BINANCE" | "OKX" | "BYBIT" | "COINBASE" | "KRAKEN" => ExchangeVenue::CryptoSpotPerp,
            _ => ExchangeVenue::UsEquities,
        };
    }
    ExchangeVenue::Hose
}

fn is_market_index_ticker(ticker: &str) -> bool {
    let sym = ticker
        .split_once(':')
        .map_or(ticker, |(_, s)| s)
        .trim()
        .to_ascii_uppercase();
    matches!(
        sym.as_str(),
        "VNINDEX" | "HNXINDEX" | "UPCOMINDEX" | "VN30" | "HNX30" | "SPX" | "NDX" | "DJI" | "DXY"
    )
}

fn compute_trade_mae_mfe(candles: &[Candle], trade: &ClosedTrade) -> (f64, f64) {
    if candles.is_empty() || trade.entry_price <= 0.0 || !trade.entry_price.is_finite() {
        return (0.0, 0.0);
    }
    let start = trade.entry_base_index.min(candles.len() - 1);
    let end = trade.exit_base_index.min(candles.len() - 1).max(start);
    let mut min_low = f64::INFINITY;
    let mut max_high = f64::NEG_INFINITY;
    for c in &candles[start..=end] {
        min_low = min_low.min(c.low);
        max_high = max_high.max(c.high);
    }
    if !min_low.is_finite() || !max_high.is_finite() {
        return (0.0, 0.0);
    }
    match trade.side {
        PositionSide::Long => {
            let mae = (((min_low - trade.entry_price) / trade.entry_price) * 100.0).min(0.0);
            let mfe = (((max_high - trade.entry_price) / trade.entry_price) * 100.0).max(0.0);
            (mae, mfe)
        }
        PositionSide::Short => {
            let mae = (((trade.entry_price - max_high) / trade.entry_price) * 100.0).min(0.0);
            let mfe = (((trade.entry_price - min_low) / trade.entry_price) * 100.0).max(0.0);
            (mae, mfe)
        }
    }
}

/// Initializes the isolated sandbox `state.replay_broker` at the Replay cut bar
/// WITHOUT touching or overwriting the live `state.broker` or SQLite `paper_broker:*` (`BUG-006`).
pub(crate) async fn reset_replay_broker_at(
    state: &AppState,
    ticker: &str,
    iv: Interval,
    base_bars: Vec<OhlcvBar>,
    base_candles: Vec<Candle>,
    cut_idx: usize,
) -> DesktopResult<BrokerStateSnapshot> {
    let venue = infer_venue_for_ticker(ticker);
    let mut cfg = default_account_config_for_venue(venue, iv);
    cfg.sanitize();

    let mut broker = state.replay_broker.lock().await;
    broker.ticker = ticker.to_owned();
    broker.interval = iv;
    broker.base_bars = base_bars;
    broker.base_candles = base_candles.clone();
    broker.cursor_index = cut_idx;
    broker.session = TradingSession::new(cfg);
    broker.order_types_meta.clear();
    broker.session.reset_at(&base_candles, cut_idx);
    build_broker_snapshot(state, &broker, true)
}

pub(crate) async fn ensure_broker_synced(
    state: &AppState,
    ticker: &str,
    interval_str: &str,
) -> DesktopResult<bool> {
    let sym = SymbolKey::parse(ticker)?;
    let iv: Interval = interval_str.parse()?;
    let norm_ticker = sym.ticker();

    // If causal Bar Replay is active on `norm_ticker` (on ANY timeframe pane!), bind to the
    // isolated `state.replay_broker` synchronized with Replay's base series & cursor (`BUG-006` + `BUG-040`).
    let replay_sync: Option<(Interval, Vec<OhlcvBar>, Vec<Candle>, usize)> = {
        let rg = state.replay.lock().await;
        rg.as_ref().and_then(|rt| {
            if rt.controller.is_active() && rt.ticker.eq_ignore_ascii_case(&norm_ticker) {
                Some((
                    rt.interval,
                    rt.base_bars.clone(),
                    rt.base_candles.clone(),
                    rt.controller.base_cursor_index,
                ))
            } else {
                None
            }
        })
    };

    if let Some((base_iv, bars, candles, cursor_idx)) = replay_sync {
        let venue = infer_venue_for_ticker(&norm_ticker);
        let mut broker = state.replay_broker.lock().await;
        if !broker.ticker.eq_ignore_ascii_case(&norm_ticker) || broker.interval != base_iv {
            let mut cfg = default_account_config_for_venue(venue, base_iv);
            cfg.sanitize();
            broker.ticker = norm_ticker;
            broker.interval = base_iv;
            broker.base_bars = bars;
            broker.base_candles = candles.clone();
            broker.cursor_index = cursor_idx;
            broker.session = TradingSession::new(cfg);
            broker.order_types_meta.clear();
            broker.session.reset_at(&candles, cursor_idx);
        } else {
            let prev_cursor = broker.cursor_index;
            broker.base_bars = bars;
            broker.base_candles = candles.clone();
            broker.cursor_index = cursor_idx;
            if cursor_idx > prev_cursor && !candles.is_empty() {
                broker
                    .session
                    .advance_through_bars(&candles, prev_cursor + 1, cursor_idx);
            } else if cursor_idx < prev_cursor && !candles.is_empty() {
                let start = broker.session.session_start_base_index.min(cursor_idx);
                broker.session.rewind_to(&candles, start, cursor_idx);
            }
        }
        return Ok(true);
    }

    // Live Paper Broker path — strictly isolated per `(ticker, interval)` so switching `1D -> 1h`
    // NEVER corrupts or wipes persisted `1D` positions/closed trades/journal (`BUG-007`)!
    let series = SeriesKey::new(&sym, iv, Adjustment::Splits);
    let svc = state.active_market_service();
    let page = svc
        .get_bars_instant(&BarsRequest::init(series.clone(), 1_500))
        .await?;
    let bars = if page.bars.is_empty() {
        svc.get_bars(&BarsRequest::init(series, 500))
            .await
            .map(|p| p.bars)
            .unwrap_or_default()
    } else {
        page.bars
    };
    let candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
    let cursor_idx = candles.len().saturating_sub(1);

    let venue = infer_venue_for_ticker(&norm_ticker);
    let mut broker = state.broker.lock().await;
    let symbol_changed = !broker.ticker.eq_ignore_ascii_case(&norm_ticker) || broker.interval != iv;

    if symbol_changed {
        broker.ticker = norm_ticker.clone();
        broker.interval = iv;
        broker.base_bars = bars;
        broker.base_candles = candles;
        broker.cursor_index = cursor_idx;

        let kv_key_scoped = format!("paper_broker:{}:{}", norm_ticker, iv.as_str());
        let kv_key_legacy = format!("paper_broker:{}", norm_ticker);
        let restored = state
            .workspace
            .get_kv(&kv_key_scoped)
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_value::<PersistedBrokerSnapshot>(v).ok())
            .or_else(|| {
                state
                    .workspace
                    .get_kv(&kv_key_legacy)
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_value::<PersistedBrokerSnapshot>(v).ok())
                    .filter(|snap| snap.interval.eq_ignore_ascii_case(iv.as_str()))
            });

        let candles_snapshot = broker.base_candles.clone();
        if let Some(snap) = restored {
            broker.order_types_meta = snap.order_types_meta;
            let mut cfg = snap.config;
            cfg.venue = venue;
            cfg.base_timeframe_secs = (iv.duration_ms() / 1_000).max(1);
            // Repair legacy VN30F snapshots that had `max_leverage = 5.0` (`BUG-002`)!
            if venue == ExchangeVenue::Vn30Futures {
                cfg.contract_multiplier = 100_000.0;
                cfg.initial_margin_pct = 17.0;
                cfg.max_leverage = 1.0;
            }
            broker.session.restore_session(
                cfg,
                snap.session_start_base_index,
                snap.commands,
                &candles_snapshot,
                cursor_idx,
            );
        } else {
            let mut cfg = default_account_config_for_venue(venue, iv);
            cfg.sanitize();
            broker.session = TradingSession::new(cfg);
            broker.order_types_meta.clear();
            broker.session.reset_at(&candles_snapshot, cursor_idx);
        }
    } else {
        let prev_cursor = broker.cursor_index;
        broker.base_bars = bars;
        broker.base_candles = candles;
        broker.cursor_index = cursor_idx;
        if cursor_idx > prev_cursor && !broker.base_candles.is_empty() {
            let candles_clone = broker.base_candles.clone();
            broker
                .session
                .advance_through_bars(&candles_clone, prev_cursor + 1, cursor_idx);
        } else if cursor_idx < prev_cursor && !broker.base_candles.is_empty() {
            let start = broker.session.session_start_base_index.min(cursor_idx);
            let candles_clone = broker.base_candles.clone();
            broker.session.rewind_to(&candles_clone, start, cursor_idx);
        }
    }

    Ok(false)
}

fn default_account_config_for_venue(venue: ExchangeVenue, iv: Interval) -> AccountConfig {
    let base_timeframe_secs = (iv.duration_ms() / 1_000).max(1);
    match venue {
        ExchangeVenue::Hose | ExchangeVenue::Hnx | ExchangeVenue::Upcom => AccountConfig {
            initial_cash: 500_000_000.0,
            commission_bps: 15.0,
            tax_sell_bps: 10.0,
            slippage_bps: 2.0,
            allow_short: false,
            enforce_vn_t2_lot_lock: true,
            venue,
            base_timeframe_secs,
            ..AccountConfig::default()
        },
        ExchangeVenue::Vn30Futures => AccountConfig {
            initial_cash: 500_000_000.0,
            commission_bps: 2.0,
            tax_sell_bps: 0.0,
            slippage_bps: 1.0,
            contract_multiplier: 100_000.0,
            // Statutory VSD 17.0% initial margin (`max_leverage = 1.0` so margin_frac = 17% / 1.0 = 17%, ~5.88x — `BUG-002`!).
            initial_margin_pct: 17.0,
            max_leverage: 1.0,
            allow_short: true,
            enforce_vn_t2_lot_lock: false,
            venue,
            base_timeframe_secs,
        },
        _ => AccountConfig {
            initial_cash: 100_000.0,
            commission_bps: 5.0,
            tax_sell_bps: 0.0,
            slippage_bps: 2.0,
            allow_short: true,
            enforce_vn_t2_lot_lock: false,
            venue,
            base_timeframe_secs,
            ..AccountConfig::default()
        },
    }
}

pub(crate) fn persist_broker_snapshot(state: &AppState, broker: &PaperBrokerRuntime) {
    let snap = PersistedBrokerSnapshot {
        ticker: broker.ticker.clone(),
        interval: broker.interval.as_str().to_owned(),
        config: broker.session.config,
        session_start_base_index: broker.session.session_start_base_index,
        commands: broker.session.commands.clone(),
        order_types_meta: broker.order_types_meta.clone(),
    };
    if let Ok(val) = serde_json::to_value(&snap) {
        let key = format!("paper_broker:{}:{}", broker.ticker, broker.interval.as_str());
        let _ = state.workspace.set_kv(&key, &val);
    }
}

/// Captures the isolated `replay_broker` state as JSON for `replay_sessions` persistence (v2.2).
pub(crate) async fn snapshot_replay_broker_json(state: &AppState) -> Option<serde_json::Value> {
    let broker = state.replay_broker.lock().await;
    if broker.ticker.is_empty() {
        return None;
    }
    let snap = PersistedBrokerSnapshot {
        ticker: broker.ticker.clone(),
        interval: broker.interval.as_str().to_owned(),
        config: broker.session.config,
        session_start_base_index: broker.session.session_start_base_index,
        commands: broker.session.commands.clone(),
        order_types_meta: broker.order_types_meta.clone(),
    };
    serde_json::to_value(&snap).ok()
}

/// Restores the isolated `replay_broker` from a saved `replay_sessions` JSON snapshot (v2.2).
pub(crate) async fn restore_replay_broker_from_json(
    state: &AppState,
    ticker: &str,
    interval: Interval,
    base_bars: Vec<OhlcvBar>,
    base_candles: Vec<Candle>,
    cursor_idx: usize,
    snap_json: Option<&serde_json::Value>,
) -> Option<BrokerStateSnapshot> {
    let venue = infer_venue_for_ticker(ticker);
    let mut broker = state.replay_broker.lock().await;
    broker.ticker = ticker.to_owned();
    broker.interval = interval;
    broker.base_bars = base_bars;
    broker.base_candles = base_candles.clone();
    broker.cursor_index = cursor_idx;

    if let Some(snap) = snap_json
        .and_then(|v| serde_json::from_value::<PersistedBrokerSnapshot>(v.clone()).ok())
    {
        let mut cfg = snap.config;
        cfg.venue = venue;
        cfg.base_timeframe_secs = (interval.duration_ms() / 1_000).max(1);
        if venue == ExchangeVenue::Vn30Futures {
            cfg.contract_multiplier = 100_000.0;
            cfg.initial_margin_pct = 17.0;
            cfg.max_leverage = 1.0;
        }
        broker.order_types_meta = snap.order_types_meta;
        let start_idx = snap.session_start_base_index.min(cursor_idx);
        broker.session.restore_session(
            cfg,
            start_idx,
            snap.commands,
            &base_candles,
            cursor_idx,
        );
    } else {
        let mut cfg = default_account_config_for_venue(venue, interval);
        cfg.sanitize();
        broker.session = TradingSession::new(cfg);
        broker.order_types_meta.clear();
        broker.session.reset_at(&base_candles, cursor_idx);
    }

    build_broker_snapshot(state, &broker, true).ok()
}

/// Resolves the authoritative daily reference price (previous trading day's `1D` close)
/// even when trading on intraday intervals (`15m`, `1h`, `4h`) (`BUG-009`).
fn resolve_daily_reference_price(
    state: &AppState,
    broker: &PaperBrokerRuntime,
    idx: usize,
) -> f64 {
    if broker.interval.is_intraday()
        && let (Ok(sym), Some(cur_bar)) = (SymbolKey::parse(&broker.ticker), broker.base_bars.get(idx))
    {
        // Compute start of current ICT trading day (UTC+7) in milliseconds.
        let ict_day = (cur_bar.timestamp.div_euclid(1_000) + 7 * 3_600).div_euclid(86_400);
        let ict_day_start_ms = (ict_day * 86_400 - 7 * 3_600) * 1_000;
        let d1_series = SeriesKey::new(&sym, Interval::D1, Adjustment::Splits);
        if let Ok(prev_d1) = state.market_store.query_bars(
            &d1_series,
            BarQueryMode::Forward {
                before_ms: ict_day_start_ms,
            },
            1,
        ) && let Some(prev) = prev_d1.bars.last()
            && prev.close.is_finite()
            && prev.close > 0.0
        {
            return prev.close;
        }
    }
    if idx > 0 {
        broker.base_candles[idx - 1].close
    } else {
        broker.base_candles[idx].open
    }
}

pub(crate) fn build_broker_snapshot(
    state: &AppState,
    broker: &PaperBrokerRuntime,
    replay_linked: bool,
) -> DesktopResult<BrokerStateSnapshot> {
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let (mark_price, mark_ts_secs) = broker
        .base_candles
        .get(idx)
        .map_or((0.0, 0_i64), |c| (c.close, c.timestamp));

    let venue = broker.session.config.venue;
    let eval_ts = causal_settlement_eval_timestamp(
        mark_ts_secs,
        venue,
        broker.session.config.base_timeframe_secs,
    );
    let fifo_summary = broker.session.fifo_holding_summary(venue, eval_ts);
    let metrics = broker.session.metrics(mark_price);

    let vn_band = SymbolKey::parse(&broker.ticker).ok().and_then(|sym_key| {
        let sym_info = state
            .market_store
            .get_symbol(&sym_key)
            .ok()
            .flatten()
            .map(|(s, _)| s)
            .unwrap_or_else(|| synthesize_fallback_symbol(&sym_key));
        if broker.interval.is_intraday() && !broker.base_bars.is_empty() {
            let ref_px = resolve_daily_reference_price(state, broker, idx);
            let synth = [
                OhlcvBar::new(0, ref_px, ref_px, ref_px, ref_px, 0.0),
                broker.base_bars[idx],
            ];
            return compute_vn_band_from_bars(&sym_info, &synth);
        }
        let slice_end = (idx + 1).min(broker.base_bars.len());
        compute_vn_band_from_bars(&sym_info, &broker.base_bars[..slice_end])
    });

    let annotations_map: HashMap<u64, JournalAnnotationRecord> = state
        .workspace
        .list_journal_annotations(&broker.ticker)
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.trade_id, a))
        .collect();

    let orders: Vec<EnrichedOrder> = broker
        .session
        .orders()
        .iter()
        .rev()
        .map(|o| {
            let vn_type = broker
                .order_types_meta
                .get(&o.id)
                .cloned()
                .unwrap_or_else(|| match o.kind {
                    OrderKind::Market => "MP".to_owned(),
                    OrderKind::Limit { .. } => "LO".to_owned(),
                    OrderKind::Stop { .. } => "STOP".to_owned(),
                });
            EnrichedOrder {
                order: o.clone(),
                vn_order_type: vn_type,
                submitted_timestamp_ms: o.submitted_timestamp.saturating_mul(1_000),
                filled_timestamp_ms: o.filled_timestamp.map(|t| t.saturating_mul(1_000)),
            }
        })
        .collect();

    let closed_trades: Vec<EnrichedClosedTrade> = broker
        .session
        .closed_trades()
        .iter()
        .rev()
        .map(|t| {
            let (mae_pct, mfe_pct) = compute_trade_mae_mfe(&broker.base_candles, t);
            let ann = annotations_map.get(&t.trade_id);
            EnrichedClosedTrade {
                trade: t.clone(),
                entry_timestamp_ms: t.entry_timestamp.saturating_mul(1_000),
                exit_timestamp_ms: t.exit_timestamp.saturating_mul(1_000),
                mae_pct,
                mfe_pct,
                setup_tag: ann.map_or_else(String::new, |a| a.setup_tag.clone()),
                mistake_tag: ann.map_or_else(String::new, |a| a.mistake_tag.clone()),
                note: ann.map_or_else(String::new, |a| a.note.clone()),
            }
        })
        .collect();

    Ok(BrokerStateSnapshot {
        ticker: broker.ticker.clone(),
        interval: broker.interval.as_str().to_owned(),
        venue,
        board_lot: venue.board_lot(),
        contract_multiplier: broker.session.config.effective_contract_multiplier(),
        allows_short: broker.session.config.effective_allow_short(),
        enforce_vn_t2_lot_lock: broker.session.config.enforce_vn_t2_lot_lock,
        mark_price,
        mark_timestamp_ms: mark_ts_secs.saturating_mul(1_000),
        vn_band,
        config: broker.session.config,
        metrics,
        open_position: broker.session.open_position().cloned(),
        orders,
        fills: broker.session.fills().to_vec(),
        fifo_summary,
        closed_trades,
        equity_curve: broker.session.equity_curve().to_vec(),
        replay_linked,
    })
}

#[tauri::command]
pub async fn broker_get_state(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
) -> DesktopResult<BrokerStateSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    build_broker_snapshot(&state, &broker, replay_linked)
}

#[tauri::command]
pub async fn broker_submit_order(
    state: State<'_, AppState>,
    input: SubmitPaperOrderInput,
) -> DesktopResult<BrokerStateSnapshot> {
    if is_market_index_ticker(&input.ticker) {
        return Err(DesktopError::Validation(format!(
            "{} là chỉ số thị trường (Index) — không thể đặt lệnh giao dịch trực tiếp. Hãy chọn hợp đồng phái sinh HNX:VN30F1! hoặc cổ phiếu cơ sở.",
            input.ticker
        )));
    }

    let replay_linked = ensure_broker_synced(&state, &input.ticker, &input.interval).await?;
    let mut broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };

    if broker.base_candles.is_empty() {
        return Err(DesktopError::Validation(
            "Chưa có dữ liệu nến cho mã này để đặt lệnh".to_owned(),
        ));
    }

    let venue = broker.session.config.venue;
    let side = match input.side.trim().to_ascii_uppercase().as_str() {
        "BUY" | "LONG" | "MUA" => OrderSide::Buy,
        "SELL" | "SHORT" | "BAN" | "BÁN" => OrderSide::Sell,
        other => {
            return Err(DesktopError::Validation(format!(
                "Chiều lệnh không hợp lệ: {other}"
            )));
        }
    };

    if !input.quantity.is_finite() || input.quantity <= 0.0 {
        return Err(DesktopError::Validation(
            "Khối lượng đặt lệnh phải lớn hơn 0".to_owned(),
        ));
    }

    // 1. Statutory Vietnam 100-share board lot check (HOSE / HNX / UPCOM cash equities).
    if venue.is_vn_cash_equity() {
        let rounded = input.quantity.round();
        if (input.quantity - rounded).abs() > 1e-6 || (rounded as i64) % 100 != 0 {
            return Err(DesktopError::Validation(format!(
                "Sàn {}: khối lượng phải là bội số lô chẵn 100 cổ phiếu (nhập {:.0})",
                venue.short_label(),
                input.quantity
            )));
        }
    }

    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let ref_price = resolve_daily_reference_price(&state, &broker, idx);

    // 2. Statutory Vietnam ceiling/floor band + tick-size validation & distinct ATO/ATC/MP handling (`BUG-009`).
    let vn_order_type = input.vn_order_type.trim().to_ascii_uppercase();
    let mut is_atc_zero_slippage = false;
    let kind = match vn_order_type.as_str() {
        "MP" | "MARKET" => OrderKind::Market,
        "ATC" => {
            // Closing auction order: fills at exact session closing price (`candle.close`, 0 adverse slippage).
            is_atc_zero_slippage = true;
            OrderKind::Market
        }
        "ATO" => {
            // Opening auction order: queues as a ceiling (Buy) / floor (Sell) pending order that
            // does NOT fill immediately on the current bar and executes precisely at next bar's Open!
            let unit = if venue == ExchangeVenue::Vn30Futures {
                VnPriceUnit::ThousandVnd
            } else {
                VnPriceUnit::RawVnd
            };
            let auction_limit = compute_daily_price_band(venue, ref_price, unit)
                .map(|b| match side {
                    OrderSide::Buy => b.ceiling_price,
                    OrderSide::Sell => b.floor_price,
                })
                .unwrap_or_else(|| {
                    let close = broker.base_candles[idx].close;
                    match side {
                        OrderSide::Buy => close * 1.15,
                        OrderSide::Sell => (close * 0.85).max(0.01),
                    }
                });
            OrderKind::Limit {
                limit_price: auction_limit,
            }
        }
        "LO" | "LIMIT" => {
            let lp = input.price.ok_or_else(|| {
                DesktopError::Validation("Lệnh LO yêu cầu nhập giá đặt".to_owned())
            })?;
            validate_vn_price_band_and_tick(venue, ref_price, lp)?;
            OrderKind::Limit { limit_price: lp }
        }
        "STOP" | "STP" => {
            let sp = input.price.ok_or_else(|| {
                DesktopError::Validation("Lệnh STOP yêu cầu nhập giá kích hoạt".to_owned())
            })?;
            validate_vn_price_band_and_tick(venue, ref_price, sp)?;
            OrderKind::Stop { stop_price: sp }
        }
        other => {
            return Err(DesktopError::Validation(format!(
                "Loại lệnh không được hỗ trợ: {other}"
            )));
        }
    };

    let req = OrderRequest {
        side,
        kind,
        quantity: input.quantity,
        stop_loss: input.stop_loss.filter(|v| v.is_finite() && *v > 0.0),
        take_profit: input.take_profit.filter(|v| v.is_finite() && *v > 0.0),
    };

    let candles_clone = broker.base_candles.clone();
    let prev_slippage = broker.session.config.slippage_bps;
    if is_atc_zero_slippage {
        broker.session.config.slippage_bps = 0.0;
    }
    let submit_res = broker.session.submit_order(req, &candles_clone, idx);
    if is_atc_zero_slippage {
        broker.session.config.slippage_bps = prev_slippage;
    }
    let order_id = submit_res.map_err(|e| DesktopError::Validation(e.to_string()))?;

    broker.order_types_meta.insert(order_id, vn_order_type);
    if !replay_linked {
        persist_broker_snapshot(&state, &broker);
    }
    build_broker_snapshot(&state, &broker, replay_linked)
}

pub(crate) fn vn_tick_size_at_price(venue: ExchangeVenue, price: f64) -> Option<f64> {
    match venue {
        ExchangeVenue::Hose => {
            if price < 10_000.0 {
                Some(10.0)
            } else if price < 50_000.0 {
                Some(50.0)
            } else {
                Some(100.0)
            }
        }
        ExchangeVenue::Hnx | ExchangeVenue::Upcom => Some(100.0),
        ExchangeVenue::Vn30Futures => Some(0.1),
        _ => None,
    }
}

fn validate_vn_price_band_and_tick(
    venue: ExchangeVenue,
    reference_price: f64,
    order_price: f64,
) -> DesktopResult<()> {
    if !order_price.is_finite() || order_price <= 0.0 {
        return Err(DesktopError::Validation("Giá đặt không hợp lệ".to_owned()));
    }
    if !venue.is_vn_cash_equity() && venue != ExchangeVenue::Vn30Futures {
        return Ok(());
    }
    let unit = if venue == ExchangeVenue::Vn30Futures {
        VnPriceUnit::ThousandVnd
    } else {
        VnPriceUnit::RawVnd
    };
    if let Some(band) = compute_daily_price_band(venue, reference_price, unit) {
        let eps = 1e-6;
        if order_price > band.ceiling_price + eps || order_price < band.floor_price - eps {
            return Err(DesktopError::Validation(format!(
                "Giá đặt {:.1} nằm ngoài biên độ sàn {} [Sàn {:.1} — Trần {:.1}] (Tham chiếu {:.1})",
                order_price,
                venue.short_label(),
                band.floor_price,
                band.ceiling_price,
                reference_price
            )));
        }
    }
    // Enforce statutory Vietnam tick ladder (`HOSE 10/50/100 VND`, `HNX/UPCOM 100 VND`, `VN30F 0.1 điểm` — `BUG-009`)!
    if let Some(tick) = vn_tick_size_at_price(venue, order_price) {
        let units = order_price / tick;
        if (units - units.round()).abs() > 1e-4 {
            let unit_label = if venue == ExchangeVenue::Vn30Futures {
                "điểm"
            } else {
                "VND"
            };
            return Err(DesktopError::Validation(format!(
                "Giá đặt {order_price} không đúng bước giá quy định sàn {} (bước giá {tick} {unit_label})",
                venue.short_label()
            )));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn broker_cancel_order(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    order_id: u64,
) -> DesktopResult<BrokerStateSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let mut broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let candles_clone = broker.base_candles.clone();
    broker
        .session
        .cancel_order(order_id, &candles_clone, idx)
        .map_err(|e| DesktopError::Validation(e.to_string()))?;
    if !replay_linked {
        persist_broker_snapshot(&state, &broker);
    }
    build_broker_snapshot(&state, &broker, replay_linked)
}

#[tauri::command]
pub async fn broker_modify_order(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    order_id: u64,
    new_price: f64,
) -> DesktopResult<BrokerStateSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let mut broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let ref_price = resolve_daily_reference_price(&state, &broker, idx);
    validate_vn_price_band_and_tick(broker.session.config.venue, ref_price, new_price)?;
    let candles_clone = broker.base_candles.clone();
    broker
        .session
        .modify_order_price(order_id, new_price, &candles_clone, idx)
        .map_err(|e| DesktopError::Validation(e.to_string()))?;
    if !replay_linked {
        persist_broker_snapshot(&state, &broker);
    }
    build_broker_snapshot(&state, &broker, replay_linked)
}

#[tauri::command]
pub async fn broker_update_brackets(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
) -> DesktopResult<BrokerStateSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let mut broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let candles_clone = broker.base_candles.clone();
    broker
        .session
        .update_position_brackets(stop_loss, take_profit, &candles_clone, idx)
        .map_err(|e| DesktopError::Validation(e.to_string()))?;
    if !replay_linked {
        persist_broker_snapshot(&state, &broker);
    }
    build_broker_snapshot(&state, &broker, replay_linked)
}

#[tauri::command]
pub async fn broker_flatten_position(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
) -> DesktopResult<BrokerStateSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let mut broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let candles_clone = broker.base_candles.clone();
    broker
        .session
        .flatten_position(&candles_clone, idx)
        .map_err(|e| DesktopError::Validation(e.to_string()))?;
    if !replay_linked {
        persist_broker_snapshot(&state, &broker);
    }
    build_broker_snapshot(&state, &broker, replay_linked)
}

#[tauri::command]
pub async fn broker_reset_account(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    initial_cash: Option<f64>,
    enforce_t2: Option<bool>,
) -> DesktopResult<BrokerStateSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let mut broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let venue = broker.session.config.venue;
    let mut cfg = default_account_config_for_venue(venue, broker.interval);
    if let Some(cash) = initial_cash
        && cash.is_finite()
        && cash >= 1_000.0
    {
        cfg.initial_cash = cash;
    }
    if let Some(lock) = enforce_t2 {
        cfg.enforce_vn_t2_lot_lock = lock;
    }
    cfg.sanitize();
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    broker.session = TradingSession::new(cfg);
    broker.order_types_meta.clear();
    let candles_clone = broker.base_candles.clone();
    broker.session.reset_at(&candles_clone, idx);
    if !replay_linked {
        persist_broker_snapshot(&state, &broker);
    }
    build_broker_snapshot(&state, &broker, replay_linked)
}

#[tauri::command]
pub async fn broker_update_journal_note(
    state: State<'_, AppState>,
    ticker: String,
    trade_id: u64,
    setup_tag: String,
    mistake_tag: String,
    note: String,
) -> DesktopResult<JournalAnnotationRecord> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || {
        ws.upsert_journal_annotation(&ticker, trade_id, &setup_tag, &mistake_tag, &note)
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vn_hose_price_band_and_tick_size_rejects_off_tick_and_out_of_band() {
        // Reference 100,000 VND on HOSE (±7% -> [93,000, 107,000], tick = 100 VND)
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hose, 100_000.0, 107_000.0).is_ok());
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hose, 100_000.0, 93_000.0).is_ok());
        assert!(
            validate_vn_price_band_and_tick(ExchangeVenue::Hose, 100_000.0, 107_100.0).is_err()
        );
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hose, 100_000.0, 92_900.0).is_err());

        // Off-tick rejections (`BUG-009`): HOSE mid-tier tick = 50 VND, HNX flat tick = 100 VND
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hose, 45_000.0, 42_583.0).is_err());
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hose, 45_000.0, 42_600.0).is_ok());
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hnx, 12_000.0, 11_950.0).is_err());
        assert!(validate_vn_price_band_and_tick(ExchangeVenue::Hnx, 12_000.0, 11_900.0).is_ok());
        assert!(
            validate_vn_price_band_and_tick(ExchangeVenue::Vn30Futures, 1_800.0, 1_805.1).is_ok()
        );
        assert!(
            validate_vn_price_band_and_tick(ExchangeVenue::Vn30Futures, 1_800.0, 1_805.15).is_err()
        );
    }

    #[test]
    fn vn30f_statutory_17_pct_margin_blocks_overleveraged_contracts() {
        let cfg = default_account_config_for_venue(ExchangeVenue::Vn30Futures, Interval::D1);
        assert!((cfg.initial_margin_pct - 17.0).abs() < 1e-9);
        assert!((cfg.max_leverage - 1.0).abs() < 1e-9);
        // At 1,780.2 pts, 1 contract notional = 178,020,000 VND -> 17% margin = 30,263,400 VND.
        // On 500M VND capital, 16 contracts (~484M) fits, 26 contracts (~787M) MUST be rejected!
        let m16 = cfg.required_margin_for(1_780.2, 16.0);
        let m26 = cfg.required_margin_for(1_780.2, 26.0);
        assert!(m16 < 500_000_000.0);
        assert!(m26 > 500_000_000.0);
    }
}
