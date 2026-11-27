use std::collections::HashMap;

use chrono::{Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sumi_core::data::Timeframe;
use sumi_core::model::Candle;
use sumi_core::replay::{ReplayController, ReplaySpeed, ReplayStatus};
use sumi_data::{
    Adjustment, BarQueryMode, BarsRequest, Interval, OhlcvBar, SeriesKey, SqliteMarketStore,
    SymbolKey,
};
use tauri::State;

use crate::commands::broker::{
    BrokerStateSnapshot, build_broker_snapshot, reset_replay_broker_at,
    restore_replay_broker_from_json, snapshot_replay_broker_json,
};
use crate::commands::market::{
    VnBandLevels, compute_vn_band_from_bars, synthesize_fallback_symbol,
};
use crate::commands::workspace::evaluate_replay_bar_alerts;
use crate::error::{DesktopError, DesktopResult};
use crate::state::{AppState, ReplayRuntime};
use crate::workspace_db::{PriceAlertRecord, SavedReplaySessionRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaySnapshotResponse {
    pub active: bool,
    pub status: ReplayStatus,
    pub ticker: String,
    pub interval: String,
    pub tracked_intervals: Vec<String>,
    pub cursor_index: usize,
    pub total_bars: usize,
    pub cutoff_timestamp_ms: Option<i64>,
    pub speed_multiplier: f64,
    /// Strict DEF-001 causal slice (`base_bars[..=cursor_index]`, zero future leakage!).
    pub visible_bars: Vec<OhlcvBar>,
    /// Newly revealed single bar on the base interval when stepping forward by +1.
    pub stepped_bar: Option<OhlcvBar>,
    /// Causally aggregated tail bars for EVERY tracked interval (`"1h"`, `"4h"`, `"1D"`, ...),
    /// enabling synchronized zero-lookahead multi-timeframe Bar Replay across panes (`BUG-040`).
    pub stepped_bars_by_interval: HashMap<String, OhlcvBar>,
    /// Exact bar at `cutoff_timestamp_ms` (drives Blind Mode Watchlist/Detail without future leaks, `BUG-041`).
    pub cutoff_bar: Option<OhlcvBar>,
    /// Previous bar close at cutoff (for computing historical replay change % in Blind Mode).
    pub prev_cutoff_close: Option<f64>,
    /// VN Ceiling / Reference / Floor bands computed strictly at `cutoff_timestamp_ms`.
    pub vn_band: Option<VnBandLevels>,
    /// Armed price alerts triggered during this replay step (`BUG-017`).
    pub triggered_alerts: Vec<PriceAlertRecord>,
    pub broker: Option<BrokerStateSnapshot>,
}

fn map_speed(mult: f64) -> ReplaySpeed {
    if mult >= 15.0 {
        ReplaySpeed::X20
    } else if mult >= 8.0 {
        ReplaySpeed::X10
    } else if mult >= 4.0 {
        ReplaySpeed::X5
    } else if mult >= 1.5 {
        ReplaySpeed::X2
    } else {
        ReplaySpeed::X1
    }
}

pub(crate) fn resolve_replay_cut_index(bars: &[OhlcvBar], cut_timestamp_ms: Option<i64>) -> usize {
    if bars.len() <= 1 {
        return 0;
    }
    let default_cut = bars.len().saturating_sub(60).max(5).min(bars.len() - 1);
    if let Some(ts_ms) = cut_timestamp_ms {
        let pos = bars.partition_point(|b| b.timestamp <= ts_ms);
        pos.saturating_sub(1).clamp(1, bars.len() - 1)
    } else {
        default_cut
    }
}

/// Causally materializes the visible bar sequence for `target_iv` under an active `ReplayRuntime`,
/// enforcing strict `DEF-001` zero-lookahead across ALL timeframes (`1h` -> `4h`, `1D`, `1W`, `1M`, etc.).
pub(crate) fn materialize_replay_bars(
    rt: &ReplayRuntime,
    target_iv: Interval,
    store: &SqliteMarketStore,
) -> Vec<OhlcvBar> {
    if rt.base_bars.is_empty() {
        return Vec::new();
    }
    let cutoff_idx = rt
        .controller
        .base_cursor_index
        .min(rt.base_bars.len().saturating_sub(1));
    let cutoff_ms = rt.base_bars[cutoff_idx].timestamp;

    if target_iv == rt.interval {
        return rt.base_bars[..=cutoff_idx].to_vec();
    }

    // Coarser target timeframe (e.g. base = `1h`, target = `4h` or `1D`):
    // Aggregate strictly from `base_candles[..=cutoff_idx]` via `ReplayController::materialize_candles`
    // so the forming higher-TF candle reflects ONLY revealed lower-TF bars (zero intra-bar lookahead!).
    if target_iv.duration_ms() > rt.interval.duration_ms() {
        let aggregated: Vec<OhlcvBar> = match (
            rt.interval.to_core_timeframe(),
            target_iv.to_core_timeframe(),
        ) {
            (Some(base_tf), Some(target_tf)) if base_tf.can_aggregate_to(target_tf) => rt
                .controller
                .materialize_candles(&rt.base_candles, base_tf, target_tf)
                .map(|cs| cs.iter().map(OhlcvBar::from_core_candle).collect())
                .unwrap_or_default(),
            _ if target_iv == Interval::MN1 => aggregate_monthly_causal(&rt.base_bars[..=cutoff_idx]),
            _ => Vec::new(),
        };

        if !aggregated.is_empty() {
            // Seamlessly prepend older cached target_iv bars strictly before the aggregated window
            // so higher-TF panes retain deep multi-year context with zero future leakage.
            let first_agg_ts = aggregated[0].timestamp;
            if let Ok(sym) = SymbolKey::parse(&rt.ticker) {
                let series = SeriesKey::new(&sym, target_iv, Adjustment::Splits);
                if let Ok(older) = store.query_bars(
                    &series,
                    BarQueryMode::Forward {
                        before_ms: first_agg_ts,
                    },
                    2_500,
                ) && !older.bars.is_empty()
                {
                    let mut combined = older.bars;
                    combined.extend(aggregated);
                    return combined;
                }
            }
            return aggregated;
        }
    }

    // Finer target timeframe (opened after replay started on a coarser base) or fallback:
    // Load cached target_iv bars and strictly slice `timestamp <= cutoff_ms` (DEF-001 zero lookahead).
    if let Ok(sym) = SymbolKey::parse(&rt.ticker) {
        let series = SeriesKey::new(&sym, target_iv, Adjustment::Splits);
        if let Ok(res) = store.query_bars(
            &series,
            BarQueryMode::Forward {
                before_ms: cutoff_ms.saturating_add(1),
            },
            4_000,
        ) {
            return res.bars;
        }
    }

    Vec::new()
}

/// Computes the causal latest bar for every tracked timeframe when stepping `+1` (`< 0.3ms`).
fn compute_stepped_bars_by_interval(
    rt: &ReplayRuntime,
    store: &SqliteMarketStore,
) -> HashMap<String, OhlcvBar> {
    let mut out = HashMap::new();
    let cutoff_idx = rt
        .controller
        .base_cursor_index
        .min(rt.base_bars.len().saturating_sub(1));
    if let Some(base_bar) = rt.base_bars.get(cutoff_idx).copied() {
        out.insert(rt.interval.as_str().to_owned(), base_bar);
    }

    for &iv in &rt.tracked_intervals {
        if iv == rt.interval {
            continue;
        }
        if iv.duration_ms() > rt.interval.duration_ms() {
            // Fast tail window aggregation: take only the recent tail of base_candles needed
            // to cover at least 2 full buckets of `iv`, keeping single-step latency sub-millisecond!
            let bars_per_bucket = (iv.duration_ms() / rt.interval.duration_ms().max(1)).max(1) as usize;
            let tail_len = (bars_per_bucket.saturating_mul(3)).clamp(24, 1_000);
            let start = cutoff_idx.saturating_add(1).saturating_sub(tail_len);
            let tail_candles = &rt.base_candles[start..=cutoff_idx];

            let last_bar = match (rt.interval.to_core_timeframe(), iv.to_core_timeframe()) {
                (Some(base_tf), Some(target_tf)) if base_tf.can_aggregate_to(target_tf) => {
                    sumi_core::data::aggregate_ohlcv(tail_candles, base_tf, target_tf)
                        .ok()
                        .and_then(|cs| cs.last().map(OhlcvBar::from_core_candle))
                }
                _ if iv == Interval::MN1 => {
                    let tail_bars = &rt.base_bars[start..=cutoff_idx];
                    aggregate_monthly_causal(tail_bars).into_iter().last()
                }
                _ => materialize_replay_bars(rt, iv, store).into_iter().last(),
            };
            if let Some(bar) = last_bar {
                out.insert(iv.as_str().to_owned(), bar);
            }
        } else if let Some(bar) = materialize_replay_bars(rt, iv, store).into_iter().last() {
            out.insert(iv.as_str().to_owned(), bar);
        }
    }

    out
}

fn aggregate_monthly_causal(bars: &[OhlcvBar]) -> Vec<OhlcvBar> {
    let mut out: Vec<OhlcvBar> = Vec::new();
    let mut cur_ym: Option<(i32, u32)> = None;
    for b in bars {
        let Some(dt) = Utc.timestamp_millis_opt(b.timestamp).single() else {
            continue;
        };
        let ym = (dt.year(), dt.month());
        if cur_ym != Some(ym) {
            cur_ym = Some(ym);
            let bucket_ts = Utc
                .with_ymd_and_hms(ym.0, ym.1, 1, 0, 0, 0)
                .single()
                .map_or(b.timestamp, |d| d.timestamp_millis());
            out.push(OhlcvBar {
                timestamp: bucket_ts,
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
                turnover: b.turnover,
            });
        } else if let Some(last) = out.last_mut() {
            last.high = last.high.max(b.high);
            last.low = last.low.min(b.low);
            last.close = b.close;
            last.volume += b.volume;
            last.turnover += b.turnover;
        }
    }
    out
}

fn compute_replay_vn_band(
    state: &AppState,
    ticker: &str,
    rt_bars_up_to_cutoff: &[OhlcvBar],
    rt_interval: Interval,
) -> Option<VnBandLevels> {
    let sym = SymbolKey::parse(ticker).ok()?;
    let sym_info = state
        .market_store
        .get_symbol(&sym)
        .ok()
        .flatten()
        .map(|(s, _)| s)
        .unwrap_or_else(|| synthesize_fallback_symbol(&sym));
    if rt_interval == Interval::D1 {
        return compute_vn_band_from_bars(&sym_info, rt_bars_up_to_cutoff);
    }
    // For intraday replay, compute daily band from prior trading day's D1 close <= cutoff!
    if let Some(last) = rt_bars_up_to_cutoff.last() {
        let d1_series = SeriesKey::new(&sym, Interval::D1, Adjustment::Splits);
        if let Ok(d1) = state.market_store.query_bars(
            &d1_series,
            BarQueryMode::Forward {
                before_ms: last.timestamp.saturating_add(1),
            },
            3,
        ) && d1.bars.len() >= 2
        {
            return compute_vn_band_from_bars(&sym_info, &d1.bars);
        }
    }
    compute_vn_band_from_bars(&sym_info, rt_bars_up_to_cutoff)
}

#[tauri::command]
pub async fn replay_start(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    cut_timestamp_ms: Option<i64>,
    active_intervals: Option<Vec<String>>,
) -> DesktopResult<ReplaySnapshotResponse> {
    let sym = SymbolKey::parse(&ticker)?;
    let requested_iv: Interval = interval.parse()?;
    let svc = state.active_market_service();

    // Collect all unique open pane intervals for this ticker so Multi-Timeframe Bar Replay
    // automatically drives from the finest timeframe (e.g. `1h` when observing `1h` + `4h`, `BUG-040`).
    let mut tracked_intervals: Vec<Interval> = vec![requested_iv];
    if let Some(ivs) = active_intervals {
        for raw in ivs {
            if let Ok(parsed) = raw.parse::<Interval>()
                && !tracked_intervals.contains(&parsed)
            {
                tracked_intervals.push(parsed);
            }
        }
    }
    tracked_intervals.sort_by_key(|iv| iv.duration_ms());

    // Pick the finest tracked interval that can provide >= 10 historical bars as causal base.
    let mut chosen_iv = requested_iv;
    let mut bars: Vec<OhlcvBar> = Vec::new();
    for &cand_iv in &tracked_intervals {
        let series = SeriesKey::new(&sym, cand_iv, Adjustment::Splits);
        let mut cand_bars = svc
            .get_bars_instant(&BarsRequest::init(series.clone(), 4_000))
            .await
            .map(|p| p.bars)
            .unwrap_or_default();
        if cand_bars.len() < 10 {
            cand_bars = svc
                .get_bars(&BarsRequest::init(series, 1_000))
                .await
                .map(|p| p.bars)
                .unwrap_or_default();
        }
        if cand_bars.len() >= 10 {
            chosen_iv = cand_iv;
            bars = cand_bars;
            break;
        }
    }

    if bars.len() < 10 {
        return Err(DesktopError::Validation(
            "Cần tối thiểu 10 cây nến để khởi động Bar Replay nhân quả".to_owned(),
        ));
    }

    let base_candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(chosen_iv)).collect();
    let cut_idx = resolve_replay_cut_index(&bars, cut_timestamp_ms);

    let mut controller = ReplayController::new();
    controller.start_at_base_index(&base_candles, cut_idx);

    let visible_bars = bars[..=cut_idx].to_vec();
    let cutoff_bar = bars.get(cut_idx).copied();
    let prev_cutoff_close = cut_idx
        .checked_sub(1)
        .and_then(|i| bars.get(i))
        .map(|b| b.close);
    let cutoff_ms = cutoff_bar.map(|b| b.timestamp);
    let total_bars = bars.len();
    let status = controller.status;
    let vn_band = compute_replay_vn_band(&state, &sym.ticker(), &visible_bars, chosen_iv);
    let tracked_strings: Vec<String> = tracked_intervals
        .iter()
        .map(|iv| iv.as_str().to_owned())
        .collect();

    {
        let mut rg = state.replay.lock().await;
        *rg = Some(ReplayRuntime {
            ticker: sym.ticker(),
            interval: chosen_iv,
            tracked_intervals,
            base_bars: bars.clone(),
            base_candles: base_candles.clone(),
            controller,
            speed_multiplier: 1.0,
        });
    }

    // Initialize isolated sandbox Replay Broker (NEVER touches or wipes live `paper_broker:*` in SQLite — `BUG-006`!).
    let broker_snapshot = reset_replay_broker_at(
        &state,
        &sym.ticker(),
        chosen_iv,
        bars,
        base_candles,
        cut_idx,
    )
    .await?;

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker: sym.ticker(),
        interval: chosen_iv.as_str().to_owned(),
        tracked_intervals: tracked_strings,
        cursor_index: cut_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_ms,
        speed_multiplier: 1.0,
        visible_bars,
        stepped_bar: None,
        stepped_bars_by_interval: HashMap::new(),
        cutoff_bar,
        prev_cutoff_close,
        vn_band,
        triggered_alerts: Vec::new(),
        broker: Some(broker_snapshot),
    })
}

#[tauri::command]
pub async fn replay_step(
    state: State<'_, AppState>,
    delta: i32,
) -> DesktopResult<ReplaySnapshotResponse> {
    if delta == 0 {
        return replay_get_state(state).await;
    }

    let (
        ticker,
        iv,
        tracked_strings,
        status,
        prev_idx,
        new_idx,
        total_bars,
        speed_mult,
        visible_bars,
        stepped_bar,
        stepped_bars_by_interval,
        cutoff_bar,
        prev_cutoff_close,
        vn_band,
        base_bars_clone,
        base_candles,
    ) = {
        let mut rg = state.replay.lock().await;
        let rt = rg.as_mut().ok_or_else(|| {
            DesktopError::NotFound("Phiên Bar Replay chưa được khởi động".to_owned())
        })?;
        let tf = rt.interval.to_core_timeframe().unwrap_or(Timeframe::D1);
        let prev_idx = rt.controller.base_cursor_index;
        if delta > 0 {
            rt.controller
                .step_forward(&rt.base_candles, tf, delta as usize);
        } else {
            rt.controller
                .step_backward(&rt.base_candles, tf, delta.unsigned_abs() as usize);
        }
        let new_idx = rt
            .controller
            .base_cursor_index
            .min(rt.base_bars.len().saturating_sub(1));
        let is_single_forward = delta == 1 && new_idx == prev_idx + 1;
        let stepped = if is_single_forward {
            rt.base_bars.get(new_idx).copied()
        } else {
            None
        };
        let stepped_map = if is_single_forward {
            compute_stepped_bars_by_interval(rt, &state.market_store)
        } else {
            HashMap::new()
        };
        let visible = if is_single_forward {
            Vec::new()
        } else {
            rt.base_bars[..=new_idx].to_vec()
        };
        let cutoff_bar = rt.base_bars.get(new_idx).copied();
        let prev_close = new_idx
            .checked_sub(1)
            .and_then(|i| rt.base_bars.get(i))
            .map(|b| b.close);
        let band = compute_replay_vn_band(&state, &rt.ticker, &rt.base_bars[..=new_idx], rt.interval);
        let tracked_strs: Vec<String> = rt
            .tracked_intervals
            .iter()
            .map(|i| i.as_str().to_owned())
            .collect();
        (
            rt.ticker.clone(),
            rt.interval,
            tracked_strs,
            rt.controller.status,
            prev_idx,
            new_idx,
            rt.base_bars.len(),
            rt.speed_multiplier,
            visible,
            stepped,
            stepped_map,
            cutoff_bar,
            prev_close,
            band,
            rt.base_bars.clone(),
            rt.base_candles.clone(),
        )
    };

    // Deterministically advance or rewind isolated Replay Broker (live broker remains untouched!).
    let causal_alert_bars: Vec<OhlcvBar> = if new_idx > prev_idx {
        let start = new_idx.saturating_sub(260);
        base_bars_clone[start..=new_idx].to_vec()
    } else {
        Vec::new()
    };
    let broker_snapshot = {
        let mut broker = state.replay_broker.lock().await;
        broker.base_bars = base_bars_clone;
        broker.base_candles = base_candles.clone();
        broker.cursor_index = new_idx;
        if new_idx > prev_idx {
            broker
                .session
                .advance_through_bars(&base_candles, prev_idx + 1, new_idx);
        } else if new_idx < prev_idx {
            let start = broker.session.session_start_base_index.min(new_idx);
            broker.session.rewind_to(&base_candles, start, new_idx);
        }
        build_broker_snapshot(&state, &broker, true)?
    };

    // Evaluate armed Price + Drawing-Line + Indicator (`RSI`/`MACD`/`EMA`) alerts causally during Replay (`BUG-017` + v2.2!).
    let triggered_alerts = if new_idx > prev_idx
        && let (Some(bar), Some(prev_c)) = (cutoff_bar, prev_cutoff_close)
    {
        let ws = state.workspace.clone();
        let t = ticker.clone();
        tokio::task::spawn_blocking(move || {
            evaluate_replay_bar_alerts(&ws, &t, prev_c, &bar, &causal_alert_bars)
        })
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker,
        interval: iv.as_str().to_owned(),
        tracked_intervals: tracked_strings,
        cursor_index: new_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_bar.map(|b| b.timestamp),
        speed_multiplier: speed_mult,
        visible_bars,
        stepped_bar,
        stepped_bars_by_interval,
        cutoff_bar,
        prev_cutoff_close,
        vn_band,
        triggered_alerts,
        broker: Some(broker_snapshot),
    })
}

#[tauri::command]
pub async fn replay_jump(
    state: State<'_, AppState>,
    target_timestamp_ms: i64,
) -> DesktopResult<ReplaySnapshotResponse> {
    let (
        ticker,
        iv,
        tracked_strings,
        status,
        new_idx,
        total_bars,
        speed_mult,
        visible_bars,
        cutoff_bar,
        prev_cutoff_close,
        vn_band,
        base_bars_clone,
        base_candles,
    ) = {
        let mut rg = state.replay.lock().await;
        let rt = rg.as_mut().ok_or_else(|| {
            DesktopError::NotFound("Phiên Bar Replay chưa được khởi động".to_owned())
        })?;
        let pos = rt
            .base_bars
            .partition_point(|b| b.timestamp <= target_timestamp_ms);
        let target_idx = pos.saturating_sub(1).clamp(1, rt.base_bars.len() - 1);
        rt.controller
            .jump_to_base_index(&rt.base_candles, target_idx);
        let new_idx = rt
            .controller
            .base_cursor_index
            .min(rt.base_bars.len().saturating_sub(1));
        let vis = rt.base_bars[..=new_idx].to_vec();
        let cutoff_bar = rt.base_bars.get(new_idx).copied();
        let prev_close = new_idx
            .checked_sub(1)
            .and_then(|i| rt.base_bars.get(i))
            .map(|b| b.close);
        let band = compute_replay_vn_band(&state, &rt.ticker, &vis, rt.interval);
        let tracked_strs: Vec<String> = rt
            .tracked_intervals
            .iter()
            .map(|i| i.as_str().to_owned())
            .collect();
        (
            rt.ticker.clone(),
            rt.interval,
            tracked_strs,
            rt.controller.status,
            new_idx,
            rt.base_bars.len(),
            rt.speed_multiplier,
            vis,
            cutoff_bar,
            prev_close,
            band,
            rt.base_bars.clone(),
            rt.base_candles.clone(),
        )
    };

    let broker_snapshot = {
        let mut broker = state.replay_broker.lock().await;
        broker.base_bars = base_bars_clone;
        broker.base_candles = base_candles.clone();
        broker.cursor_index = new_idx;
        let start = broker.session.session_start_base_index.min(new_idx);
        broker.session.rewind_to(&base_candles, start, new_idx);
        build_broker_snapshot(&state, &broker, true)?
    };

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker,
        interval: iv.as_str().to_owned(),
        tracked_intervals: tracked_strings,
        cursor_index: new_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_bar.map(|b| b.timestamp),
        speed_multiplier: speed_mult,
        visible_bars,
        stepped_bar: None,
        stepped_bars_by_interval: HashMap::new(),
        cutoff_bar,
        prev_cutoff_close,
        vn_band,
        triggered_alerts: Vec::new(),
        broker: Some(broker_snapshot),
    })
}

#[tauri::command]
pub async fn replay_set_speed(
    state: State<'_, AppState>,
    speed_multiplier: f64,
) -> DesktopResult<f64> {
    // Allow full `0.5x..20.0x` range so `ReplaySpeed::X20` is reachable (`BUG-028`).
    let clamped = if speed_multiplier.is_finite() {
        speed_multiplier.clamp(0.5, 20.0)
    } else {
        1.0
    };
    let mut rg = state.replay.lock().await;
    if let Some(rt) = rg.as_mut() {
        rt.speed_multiplier = clamped;
        rt.controller.speed = map_speed(clamped);
    }
    Ok(clamped)
}

#[tauri::command]
pub async fn replay_stop(state: State<'_, AppState>) -> DesktopResult<()> {
    let mut rg = state.replay.lock().await;
    *rg = None;
    Ok(())
}

#[tauri::command]
pub async fn replay_get_state(state: State<'_, AppState>) -> DesktopResult<ReplaySnapshotResponse> {
    let rg = state.replay.lock().await;
    let Some(rt) = rg.as_ref() else {
        return Ok(ReplaySnapshotResponse {
            active: false,
            status: ReplayStatus::Inactive,
            ticker: String::new(),
            interval: "1D".to_owned(),
            tracked_intervals: Vec::new(),
            cursor_index: 0,
            total_bars: 0,
            cutoff_timestamp_ms: None,
            speed_multiplier: 1.0,
            visible_bars: Vec::new(),
            stepped_bar: None,
            stepped_bars_by_interval: HashMap::new(),
            cutoff_bar: None,
            prev_cutoff_close: None,
            vn_band: None,
            triggered_alerts: Vec::new(),
            broker: None,
        });
    };
    let idx = rt
        .controller
        .base_cursor_index
        .min(rt.base_bars.len().saturating_sub(1));
    let cutoff_bar = rt.base_bars.get(idx).copied();
    let prev_close = idx
        .checked_sub(1)
        .and_then(|i| rt.base_bars.get(i))
        .map(|b| b.close);
    let visible = rt.base_bars[..=idx].to_vec();
    let band = compute_replay_vn_band(&state, &rt.ticker, &visible, rt.interval);
    let tracked_strings = rt
        .tracked_intervals
        .iter()
        .map(|i| i.as_str().to_owned())
        .collect();
    Ok(ReplaySnapshotResponse {
        active: rt.controller.is_active(),
        status: rt.controller.status,
        ticker: rt.ticker.clone(),
        interval: rt.interval.as_str().to_owned(),
        tracked_intervals: tracked_strings,
        cursor_index: idx,
        total_bars: rt.base_bars.len(),
        cutoff_timestamp_ms: cutoff_bar.map(|b| b.timestamp),
        speed_multiplier: rt.speed_multiplier,
        visible_bars: visible,
        stepped_bar: None,
        stepped_bars_by_interval: HashMap::new(),
        cutoff_bar,
        prev_cutoff_close: prev_close,
        vn_band: band,
        triggered_alerts: Vec::new(),
        broker: None,
    })
}

// ─── Save / Resume Bar Replay Practice Sessions (v2.2) ───────────────────────

#[tauri::command]
pub async fn list_replay_sessions(
    state: State<'_, AppState>,
) -> DesktopResult<Vec<SavedReplaySessionRecord>> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.list_replay_sessions())
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn save_replay_session(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    blind_mode: Option<bool>,
) -> DesktopResult<SavedReplaySessionRecord> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err(DesktopError::Validation(
            "Tên phiên Bar Replay không được để trống".to_owned(),
        ));
    }

    let (ticker, base_interval, tracked_intervals, cursor_timestamp_ms, speed_multiplier) = {
        let rg = state.replay.lock().await;
        let rt = rg.as_ref().ok_or_else(|| {
            DesktopError::Validation(
                "Chưa có phiên Bar Replay đang hoạt động để lưu".to_owned(),
            )
        })?;
        let idx = rt
            .controller
            .base_cursor_index
            .min(rt.base_bars.len().saturating_sub(1));
        let cursor_ts = rt
            .base_bars
            .get(idx)
            .map(|b| b.timestamp)
            .ok_or_else(|| DesktopError::Internal("Chuỗi nến Replay rỗng".to_owned()))?;
        let ivs: Vec<String> = rt
            .tracked_intervals
            .iter()
            .map(|iv| iv.as_str().to_owned())
            .collect();
        (
            rt.ticker.clone(),
            rt.interval.as_str().to_owned(),
            ivs,
            cursor_ts,
            rt.speed_multiplier,
        )
    };

    let broker_snapshot_json = snapshot_replay_broker_json(&state).await;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let rec = SavedReplaySessionRecord {
        id: id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("replay_{now_ms}")),
        name: clean_name.to_owned(),
        ticker,
        base_interval,
        tracked_intervals,
        cursor_timestamp_ms,
        speed_multiplier,
        blind_mode: blind_mode.unwrap_or(true),
        broker_snapshot_json,
        updated_at_ms: now_ms,
    };

    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.save_replay_session(&rec))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn resume_replay_session(
    state: State<'_, AppState>,
    id: String,
) -> DesktopResult<ReplaySnapshotResponse> {
    let ws = state.workspace.clone();
    let saved = tokio::task::spawn_blocking(move || ws.get_replay_session(&id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))??
        .ok_or_else(|| {
            DesktopError::NotFound("Không tìm thấy phiên Bar Replay đã lưu".to_owned())
        })?;

    let sym = SymbolKey::parse(&saved.ticker)?;
    let chosen_iv: Interval = saved.base_interval.parse()?;
    let svc = state.active_market_service();
    let series = SeriesKey::new(&sym, chosen_iv, Adjustment::Splits);
    let mut bars = svc
        .get_bars_instant(&BarsRequest::init(series.clone(), 4_000))
        .await
        .map(|p| p.bars)
        .unwrap_or_default();
    if bars.len() < 10 {
        bars = svc
            .get_bars(&BarsRequest::init(series, 1_000))
            .await
            .map(|p| p.bars)
            .unwrap_or_default();
    }
    if bars.len() < 10 {
        return Err(DesktopError::Validation(
            "Không đủ dữ liệu nến để khôi phục phiên Bar Replay".to_owned(),
        ));
    }

    let mut tracked_intervals: Vec<Interval> = vec![chosen_iv];
    for raw in &saved.tracked_intervals {
        if let Ok(parsed) = raw.parse::<Interval>()
            && !tracked_intervals.contains(&parsed)
        {
            tracked_intervals.push(parsed);
        }
    }
    tracked_intervals.sort_by_key(|iv| iv.duration_ms());

    let base_candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(chosen_iv)).collect();
    let cut_idx = resolve_replay_cut_index(&bars, Some(saved.cursor_timestamp_ms));
    let mut controller = ReplayController::new();
    controller.start_at_base_index(&base_candles, cut_idx);
    let clamped_speed = saved.speed_multiplier.clamp(0.5, 20.0);
    controller.speed = map_speed(clamped_speed);

    let visible_bars = bars[..=cut_idx].to_vec();
    let cutoff_bar = bars.get(cut_idx).copied();
    let prev_cutoff_close = cut_idx
        .checked_sub(1)
        .and_then(|i| bars.get(i))
        .map(|b| b.close);
    let total_bars = bars.len();
    let status = controller.status;
    let vn_band = compute_replay_vn_band(&state, &sym.ticker(), &visible_bars, chosen_iv);
    let tracked_strings: Vec<String> = tracked_intervals
        .iter()
        .map(|iv| iv.as_str().to_owned())
        .collect();

    {
        let mut rg = state.replay.lock().await;
        *rg = Some(ReplayRuntime {
            ticker: sym.ticker(),
            interval: chosen_iv,
            tracked_intervals,
            base_bars: bars.clone(),
            base_candles: base_candles.clone(),
            controller,
            speed_multiplier: clamped_speed,
        });
    }

    let broker_snapshot = restore_replay_broker_from_json(
        &state,
        &sym.ticker(),
        chosen_iv,
        bars,
        base_candles,
        cut_idx,
        saved.broker_snapshot_json.as_ref(),
    )
    .await;

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker: sym.ticker(),
        interval: chosen_iv.as_str().to_owned(),
        tracked_intervals: tracked_strings,
        cursor_index: cut_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_bar.map(|b| b.timestamp),
        speed_multiplier: clamped_speed,
        visible_bars,
        stepped_bar: None,
        stepped_bars_by_interval: HashMap::new(),
        cutoff_bar,
        prev_cutoff_close,
        vn_band,
        triggered_alerts: Vec::new(),
        broker: broker_snapshot,
    })
}

#[tauri::command]
pub async fn delete_replay_session(state: State<'_, AppState>, id: String) -> DesktopResult<()> {
    let ws = state.workspace.clone();
    tokio::task::spawn_blocking(move || ws.delete_replay_session(&id))
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_replay_cut_index_cuts_exact_bar_with_zero_future_leak() {
        let step = 86_400_000_i64;
        let base = 1_700_000_000_000_i64;
        let bars: Vec<OhlcvBar> = (0..200)
            .map(|i| OhlcvBar::new(base + i * step, 100.0, 105.0, 95.0, 102.0, 1_000.0))
            .collect();

        let exact = resolve_replay_cut_index(&bars, Some(base + 42 * step));
        assert_eq!(exact, 42);
        assert_eq!(bars[..=exact].len(), 43);

        let mid = resolve_replay_cut_index(&bars, Some(base + 42 * step + 12_000));
        assert_eq!(mid, 42);

        let early = resolve_replay_cut_index(&bars, Some(base - 10_000));
        assert_eq!(early, 1);

        let default_idx = resolve_replay_cut_index(&bars, None);
        assert_eq!(default_idx, 140);
    }

    #[test]
    fn multi_timeframe_causal_materialization_1h_to_4h_has_zero_future_leak() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let start_ms = 1_700_006_400_000_i64; // aligned 00:00 UTC boundary
        let step_ms = 3_600_000_i64; // 1h
        // Create 12 hourly bars (3 full 4h buckets); bar 6 spikes to 999.0 in future!
        let base_bars: Vec<OhlcvBar> = (0..12)
            .map(|i| {
                let high = if i == 6 { 999.0 } else { 105.0 + i as f64 };
                OhlcvBar::new(
                    start_ms + i * step_ms,
                    100.0 + i as f64,
                    high,
                    95.0,
                    102.0 + i as f64,
                    100.0,
                )
            })
            .collect();
        let base_candles: Vec<Candle> = base_bars
            .iter()
            .map(|b| b.to_core_candle(Interval::H1))
            .collect();
        let mut ctrl = ReplayController::new();
        // Cut at bar index 5 (2nd hour inside the 2nd 4h bucket: bars 4 & 5 visible; bar 6's 999.0 is FUTURE!)
        ctrl.start_at_base_index(&base_candles, 5);

        let rt = ReplayRuntime {
            ticker: "BINANCE:BTCUSDT".to_owned(),
            interval: Interval::H1,
            tracked_intervals: vec![Interval::H1, Interval::H4],
            base_bars,
            base_candles,
            controller: ctrl,
            speed_multiplier: 1.0,
        };

        let h4 = materialize_replay_bars(&rt, Interval::H4, &store);
        assert_eq!(h4.len(), 2, "must emit 1 complete + 1 partial 4h bucket");
        assert!(
            h4[1].high < 200.0,
            "forming 4h candle MUST NOT leak future bar 6 high=999.0 (got {})",
            h4[1].high
        );
        assert!((h4[1].volume - 200.0).abs() < 1e-9, "partial 4h bar must sum only bars 4..=5 volume");
    }
}
