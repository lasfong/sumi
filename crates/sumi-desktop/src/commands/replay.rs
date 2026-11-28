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
    BrokerStateSnapshot, build_broker_snapshot, rescale_replay_broker_on_blind_reveal,
    reset_replay_broker_at, reset_replay_broker_for_blind_drill, restore_replay_broker_from_json,
    snapshot_replay_broker_json,
};
use crate::commands::market::{
    VnBandLevels, compute_vn_band_from_bars, synthesize_fallback_symbol,
};
use crate::commands::workspace::evaluate_replay_bar_alerts;
use crate::error::{DesktopError, DesktopResult};
use crate::state::{AppState, BlindDrillMeta, ReplayRuntime};
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
    /// Wave 5 True Blind Drill metadata (`real_ticker` & `cut_date` masked while `!revealed`).
    #[serde(default)]
    pub blind_drill: Option<BlindDrillMeta>,
}

fn display_ticker_for_runtime(rt: &ReplayRuntime) -> String {
    if let Some(ref bd) = rt.blind_drill
        && bd.enabled
        && !bd.revealed
    {
        return bd.alias_code.clone();
    }
    rt.ticker.clone()
}

fn masked_blind_drill(rt: &ReplayRuntime) -> Option<BlindDrillMeta> {
    rt.blind_drill.as_ref().map(BlindDrillMeta::masked_for_ui)
}

fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Synthetic fixed epoch (`2000-01-03T00:00:00Z`, Monday) used to strip historical calendar dates
/// from Blind Drill candles while preserving weekday & `1W` bucket alignment (`W6-03`).
const BLIND_EPOCH_START_MS: i64 = 946_857_600_000;
const WEEK_DURATION_MS: i64 = 7 * 86_400_000;

fn compute_blind_time_shift_ms(first_bar_timestamp_ms: i64) -> i64 {
    let raw_delta = first_bar_timestamp_ms.saturating_sub(BLIND_EPOCH_START_MS);
    (raw_delta / WEEK_DURATION_MS) * WEEK_DURATION_MS
}

fn scale_and_shift_ohlcv_bars(bars: &[OhlcvBar], scale: f64, time_shift_ms: i64) -> Vec<OhlcvBar> {
    let round2 = |v: f64| (v * scale * 100.0).round() / 100.0;
    bars.iter()
        .map(|b| OhlcvBar {
            timestamp: b.timestamp.saturating_sub(time_shift_ms),
            open: round2(b.open),
            high: round2(b.high),
            low: round2(b.low),
            close: round2(b.close),
            volume: b.volume,
            turnover: b.turnover * scale,
        })
        .collect()
}

#[cfg(test)]
fn scale_ohlcv_bars(bars: &[OhlcvBar], scale: f64) -> Vec<OhlcvBar> {
    scale_and_shift_ohlcv_bars(bars, scale, 0)
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

    let active_blind = rt
        .blind_drill
        .as_ref()
        .filter(|b| b.enabled && !b.revealed);

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
            // When Blind Drill is active, convert back to real timestamps for SQLite lookup and
            // apply `bd.price_scale` + `bd.time_shift_ms` identically (`W6-03`)!
            let first_agg_ts = aggregated[0].timestamp;
            let real_before_ms =
                first_agg_ts.saturating_add(active_blind.map_or(0, |b| b.time_shift_ms));
            if let Ok(sym) = SymbolKey::parse(&rt.ticker) {
                let series = SeriesKey::new(&sym, target_iv, Adjustment::Splits);
                if let Ok(older) = store.query_bars(
                    &series,
                    BarQueryMode::Forward {
                        before_ms: real_before_ms,
                    },
                    2_500,
                ) && !older.bars.is_empty()
                {
                    let mut combined = match active_blind {
                        Some(bd) => {
                            scale_and_shift_ohlcv_bars(&older.bars, bd.price_scale, bd.time_shift_ms)
                        }
                        None => older.bars,
                    };
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
        let real_cutoff_ms = cutoff_ms.saturating_add(active_blind.map_or(0, |b| b.time_shift_ms));
        if let Ok(res) = store.query_bars(
            &series,
            BarQueryMode::Forward {
                before_ms: real_cutoff_ms.saturating_add(1),
            },
            4_000,
        ) {
            return match active_blind {
                Some(bd) => scale_and_shift_ohlcv_bars(&res.bars, bd.price_scale, bd.time_shift_ms),
                None => res.bars,
            };
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
            raw_base_bars: None,
            controller,
            speed_multiplier: 1.0,
            blind_drill: None,
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
        blind_drill: None,
    })
}

/// Wave 5: True Blind Replay Drill — picks a random historical symbol from `QMV60` (or local DB),
/// randomizes a historical cut date with >= 100 bars behind and >= 45 bars ahead, normalizes all
/// OHLC prices so the cut bar closes at `100.00`, and masks symbol identity until revealed!
#[tauri::command]
pub async fn replay_start_random_blind_drill(
    state: State<'_, AppState>,
    interval: Option<String>,
    seed: Option<u64>,
) -> DesktopResult<ReplaySnapshotResponse> {
    let chosen_iv: Interval = interval
        .as_deref()
        .unwrap_or("1D")
        .parse()
        .unwrap_or(Interval::D1);

    let store = state.market_store.clone();
    let mut rng_state = seed.unwrap_or_else(|| {
        Utc::now()
            .timestamp_nanos_opt()
            .map(|n| n as u64)
            .unwrap_or(0x554D_495F_5633_0005)
    });
    if rng_state == 0 {
        rng_state = 0x554D_495F_5633_0005;
    }

    let (real_ticker, raw_bars) = tokio::task::spawn_blocking(move || -> DesktopResult<(String, Vec<OhlcvBar>)> {
        let mut candidates: Vec<(String, Vec<OhlcvBar>)> = Vec::new();
        for sym_code in sumi_core::scanner::qmv_universe::QMV60_SYMBOLS {
            let full = match sym_code {
                "IDC" | "MBS" | "PVS" | "SHS" => format!("HNX:{sym_code}"),
                "BSR" | "DDV" | "STH" | "VGI" => format!("UPCOM:{sym_code}"),
                _ => format!("HOSE:{sym_code}"),
            };
            if let Ok(key) = SymbolKey::parse(&full) {
                let series = SeriesKey::new(&key, chosen_iv, Adjustment::Splits);
                if let Ok(page) = store.query_bars(&series, BarQueryMode::Init, 2_500)
                    && page.bars.len() >= 180
                {
                    candidates.push((key.ticker(), page.bars));
                }
            }
        }
        // Fallback: any cached symbols with >= 150 bars (e.g. Clean-Slate DB with custom stocks)
        if candidates.is_empty()
            && let Ok(syms) = store.search_local_symbols("", None, 120)
        {
            for sym in syms {
                if let Ok(key) = SymbolKey::parse(&sym.ticker) {
                    let series = SeriesKey::new(&key, chosen_iv, Adjustment::Splits);
                    if let Ok(page) = store.query_bars(&series, BarQueryMode::Init, 2_500)
                        && page.bars.len() >= 150
                    {
                        candidates.push((key.ticker(), page.bars));
                    }
                }
            }
        }
        if candidates.is_empty() {
            return Err(DesktopError::Validation(
                "Cần tối thiểu 1 mã có >= 150 cây nến trong kho dữ liệu để luyện Blind Drill ngẫu nhiên".to_owned(),
            ));
        }
        let pick_idx = (splitmix64_next(&mut rng_state) as usize) % candidates.len();
        Ok(candidates.swap_remove(pick_idx))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))??;

    let min_cut = 100.min(raw_bars.len().saturating_sub(50));
    let max_cut = raw_bars.len().saturating_sub(45).max(min_cut + 1);
    let cut_span = max_cut.saturating_sub(min_cut).max(1);
    let cut_idx = min_cut + ((splitmix64_next(&mut rng_state) as usize) % cut_span);

    let raw_cut_bar = raw_bars[cut_idx];
    let price_scale = if raw_cut_bar.close.is_finite() && raw_cut_bar.close > 0.0 {
        100.0 / raw_cut_bar.close
    } else {
        1.0
    };
    let time_shift_ms = compute_blind_time_shift_ms(raw_bars[0].timestamp);
    let scaled_bars = scale_and_shift_ohlcv_bars(&raw_bars, price_scale, time_shift_ms);
    let base_candles: Vec<Candle> = scaled_bars
        .iter()
        .map(|b| b.to_core_candle(chosen_iv))
        .collect();
    let alias_num = 1000 + (splitmix64_next(&mut rng_state) % 9000);
    let alias_code = format!("MÃ ẨN #{alias_num:04}");
    let cut_date = Utc
        .timestamp_millis_opt(raw_cut_bar.timestamp)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();

    let drill_meta = BlindDrillMeta {
        enabled: true,
        alias_code: alias_code.clone(),
        price_scale,
        revealed: false,
        real_ticker: real_ticker.clone(),
        cut_date,
        time_shift_ms,
    };

    let mut controller = ReplayController::new();
    controller.start_at_base_index(&base_candles, cut_idx);

    let visible_bars = scaled_bars[..=cut_idx].to_vec();
    let cutoff_bar = scaled_bars.get(cut_idx).copied();
    let prev_cutoff_close = cut_idx
        .checked_sub(1)
        .and_then(|i| scaled_bars.get(i))
        .map(|b| b.close);
    let cutoff_ms = cutoff_bar.map(|b| b.timestamp);
    let total_bars = scaled_bars.len();
    let status = controller.status;

    {
        let mut rg = state.replay.lock().await;
        *rg = Some(ReplayRuntime {
            ticker: real_ticker,
            interval: chosen_iv,
            tracked_intervals: vec![chosen_iv],
            base_bars: scaled_bars.clone(),
            base_candles: base_candles.clone(),
            raw_base_bars: Some(raw_bars),
            controller,
            speed_multiplier: 1.0,
            blind_drill: Some(drill_meta.clone()),
        });
    }

    let broker_snapshot = reset_replay_broker_for_blind_drill(
        &state,
        &alias_code,
        chosen_iv,
        scaled_bars,
        base_candles,
        cut_idx,
    )
    .await?;

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker: alias_code,
        interval: chosen_iv.as_str().to_owned(),
        tracked_intervals: vec![chosen_iv.as_str().to_owned()],
        cursor_index: cut_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_ms,
        speed_multiplier: 1.0,
        visible_bars,
        stepped_bar: None,
        stepped_bars_by_interval: HashMap::new(),
        cutoff_bar,
        prev_cutoff_close,
        vn_band: None,
        triggered_alerts: Vec::new(),
        broker: Some(broker_snapshot),
        blind_drill: Some(drill_meta.masked_for_ui()),
    })
}

/// Reveals the hidden real ticker, restores unscaled real VND bars & timestamps, and rescales
/// the isolated Replay Broker cleanly so Debrief & subsequent orders match real market prices (`W6-03`).
#[tauri::command]
pub async fn replay_reveal_blind_drill(
    state: State<'_, AppState>,
) -> DesktopResult<ReplaySnapshotResponse> {
    type BlindRevealRestore = (String, Interval, Vec<OhlcvBar>, Vec<Candle>, usize, f64);
    let reveal_restore: Option<BlindRevealRestore> = {
        let mut rg = state.replay.lock().await;
        let rt = rg.as_mut().ok_or_else(|| {
            DesktopError::NotFound("Phiên Bar Replay chưa được khởi động".to_owned())
        })?;
        let mut restore = None;
        if let Some(bd) = rt.blind_drill.as_mut() {
            let was_unrevealed = bd.enabled && !bd.revealed;
            bd.revealed = true;
            if was_unrevealed
                && let Some(raw_bars) = rt.raw_base_bars.take()
                && !raw_bars.is_empty()
            {
                let raw_candles: Vec<Candle> = raw_bars
                    .iter()
                    .map(|b| b.to_core_candle(rt.interval))
                    .collect();
                let cursor_idx = rt
                    .controller
                    .base_cursor_index
                    .min(raw_bars.len().saturating_sub(1));
                rt.base_bars = raw_bars.clone();
                rt.base_candles = raw_candles.clone();
                rt.controller.jump_to_base_index(&rt.base_candles, cursor_idx);
                restore = Some((
                    rt.ticker.clone(),
                    rt.interval,
                    raw_bars,
                    raw_candles,
                    cursor_idx,
                    bd.price_scale,
                ));
            }
        }
        restore
    };

    let broker_snapshot = if let Some((real_ticker, iv, raw_bars, raw_candles, cursor_idx, scale)) =
        reveal_restore
    {
        Some(
            rescale_replay_broker_on_blind_reveal(
                &state,
                &real_ticker,
                iv,
                raw_bars,
                raw_candles,
                cursor_idx,
                scale,
            )
            .await?,
        )
    } else {
        None
    };

    let mut snap = replay_get_state(state).await?;
    snap.broker = broker_snapshot;
    Ok(snap)
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
        display_ticker,
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
        blind_drill_meta,
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
        let is_blind_masked = rt
            .blind_drill
            .as_ref()
            .is_some_and(|b| b.enabled && !b.revealed);
        let band = if is_blind_masked {
            None
        } else {
            compute_replay_vn_band(&state, &rt.ticker, &rt.base_bars[..=new_idx], rt.interval)
        };
        let tracked_strs: Vec<String> = rt
            .tracked_intervals
            .iter()
            .map(|i| i.as_str().to_owned())
            .collect();
        (
            rt.ticker.clone(),
            display_ticker_for_runtime(rt),
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
            masked_blind_drill(rt),
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

    // Evaluate armed Price + Drawing-Line + Indicator (`RSI`/`MACD`/`EMA`) alerts causally during Replay.
    // During an unrevealed Blind Drill, NEVER evaluate alerts belonging to `rt.ticker` — only `display_ticker` (`alias_code`, `W6-03`)!
    let _ = ticker;
    let triggered_alerts = if new_idx > prev_idx
        && let (Some(bar), Some(prev_c)) = (cutoff_bar, prev_cutoff_close)
    {
        let ws = state.workspace.clone();
        let t = display_ticker.clone();
        let iv_str = iv.as_str().to_owned();
        tokio::task::spawn_blocking(move || {
            evaluate_replay_bar_alerts(&ws, &t, prev_c, &bar, &causal_alert_bars, Some(&iv_str))
        })
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker: display_ticker,
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
        blind_drill: blind_drill_meta,
    })
}

#[tauri::command]
pub async fn replay_jump(
    state: State<'_, AppState>,
    target_timestamp_ms: i64,
) -> DesktopResult<ReplaySnapshotResponse> {
    let (
        display_ticker,
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
        blind_drill_meta,
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
        let is_blind_masked = rt
            .blind_drill
            .as_ref()
            .is_some_and(|b| b.enabled && !b.revealed);
        let band = if is_blind_masked {
            None
        } else {
            compute_replay_vn_band(&state, &rt.ticker, &vis, rt.interval)
        };
        let tracked_strs: Vec<String> = rt
            .tracked_intervals
            .iter()
            .map(|i| i.as_str().to_owned())
            .collect();
        (
            display_ticker_for_runtime(rt),
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
            masked_blind_drill(rt),
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
        ticker: display_ticker,
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
        blind_drill: blind_drill_meta,
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
            blind_drill: None,
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
    let is_blind_masked = rt
        .blind_drill
        .as_ref()
        .is_some_and(|b| b.enabled && !b.revealed);
    let band = if is_blind_masked {
        None
    } else {
        compute_replay_vn_band(&state, &rt.ticker, &visible, rt.interval)
    };
    let tracked_strings = rt
        .tracked_intervals
        .iter()
        .map(|i| i.as_str().to_owned())
        .collect();
    Ok(ReplaySnapshotResponse {
        active: rt.controller.is_active(),
        status: rt.controller.status,
        ticker: display_ticker_for_runtime(rt),
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
        blind_drill: masked_blind_drill(rt),
    })
}

// ─── Save / Resume Bar Replay Practice Sessions (v2.2) ───────────────────────

pub(crate) fn guard_save_replay_session_allowed(rt: &ReplayRuntime) -> DesktopResult<()> {
    if rt
        .blind_drill
        .as_ref()
        .is_some_and(|b| b.enabled && !b.revealed)
    {
        return Err(DesktopError::Validation(
            "Không thể lưu phiên khi đang ở chế độ Luyện Mù chưa lật mở mã — hãy bấm 'Lật Mở Mã Thật' trước khi lưu."
                .to_owned(),
        ));
    }
    Ok(())
}

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
        guard_save_replay_session_allowed(rt)?;
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
            raw_base_bars: None,
            controller,
            speed_multiplier: clamped_speed,
            blind_drill: None,
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
        blind_drill: None,
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
            raw_base_bars: None,
            controller: ctrl,
            speed_multiplier: 1.0,
            blind_drill: None,
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

    #[test]
    fn blind_drill_normalization_scales_cut_bar_to_100_and_masks_metadata() {
        let base = 1_700_000_000_000_i64;
        let bars = vec![
            OhlcvBar::new(base, 120_000.0, 125_000.0, 118_000.0, 122_000.0, 500_000.0),
            OhlcvBar::new(base + 86_400_000, 122_000.0, 130_000.0, 121_000.0, 125_000.0, 600_000.0),
        ];
        let scale = 100.0 / bars[1].close;
        let scaled = scale_ohlcv_bars(&bars, scale);
        assert!((scaled[1].close - 100.0).abs() < 1e-6);
        assert!((scaled[0].close - 97.6).abs() < 1e-6);

        let meta = BlindDrillMeta {
            enabled: true,
            alias_code: "MÃ ẨN #4242".to_owned(),
            price_scale: scale,
            revealed: false,
            real_ticker: "HOSE:FPT".to_owned(),
            cut_date: "2024-03-15".to_owned(),
            time_shift_ms: 604_800_000 * 100,
        };
        let masked = meta.masked_for_ui();
        assert_eq!(masked.alias_code, "MÃ ẨN #4242");
        assert!(masked.real_ticker.is_empty());
        assert!(masked.cut_date.is_empty());
        assert_eq!(masked.time_shift_ms, 0);
    }

    #[test]
    fn wave6_blind_drill_scales_1w_and_1h_shifts_timestamps_suppresses_real_alerts_and_blocks_save() {
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let sym = SymbolKey::new("HOSE", "FPT").unwrap();
        let d1_key = SeriesKey::new(&sym, Interval::D1, Adjustment::Splits);
        let w1_key = SeriesKey::new(&sym, Interval::W1, Adjustment::Splits);
        let h1_key = SeriesKey::new(&sym, Interval::H1, Adjustment::Splits);

        // Historical 2024 bars in real VND (~125,000 VND)
        let start_2024_ms = Utc
            .with_ymd_and_hms(2024, 1, 8, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        // Seed older 1W bar in 2023 (prior to base_bars window)
        store
            .upsert_bars(
                &w1_key,
                &[OhlcvBar::new(
                    start_2024_ms - 14 * 86_400_000,
                    120_000.0,
                    126_000.0,
                    119_000.0,
                    125_000.0,
                    1_000_000.0,
                )],
                "test",
                Some(true),
            )
            .unwrap();
        // Seed 1D bars (20 sessions)
        let raw_d1: Vec<OhlcvBar> = (0..20_i64)
            .map(|i| {
                let ts = start_2024_ms + i * 86_400_000;
                OhlcvBar::new(ts, 124_000.0, 126_000.0, 123_000.0, 125_000.0, 500_000.0)
            })
            .collect();
        store.upsert_bars(&d1_key, &raw_d1, "test", Some(true)).unwrap();
        // Seed 1H bar inside the replay window AND 1 future 1H bar beyond cutoff
        let cut_idx = 10_usize;
        let cut_real_ts = raw_d1[cut_idx].timestamp;
        store
            .upsert_bars(
                &h1_key,
                &[
                    OhlcvBar::new(cut_real_ts - 3_600_000, 124_500.0, 125_500.0, 124_000.0, 125_000.0, 80_000.0),
                    OhlcvBar::new(cut_real_ts + 2 * 86_400_000, 150_000.0, 155_000.0, 149_000.0, 152_000.0, 90_000.0),
                ],
                "test",
                Some(true),
            )
            .unwrap();

        let price_scale = 100.0 / raw_d1[cut_idx].close; // 0.0008
        let time_shift_ms = compute_blind_time_shift_ms(raw_d1[0].timestamp);
        let scaled_d1 = scale_and_shift_ohlcv_bars(&raw_d1, price_scale, time_shift_ms);
        assert!(
            scaled_d1[cut_idx].timestamp < Utc.with_ymd_and_hms(2005, 1, 1, 0, 0, 0).single().unwrap().timestamp_millis(),
            "Blind Drill timestamps must be shifted away from 2024 history"
        );
        assert!((scaled_d1[cut_idx].close - 100.0).abs() < 1e-6);

        let base_candles: Vec<Candle> = scaled_d1.iter().map(|b| b.to_core_candle(Interval::D1)).collect();
        let mut ctrl = ReplayController::new();
        ctrl.start_at_base_index(&base_candles, cut_idx);

        let rt = ReplayRuntime {
            ticker: "HOSE:FPT".to_owned(),
            interval: Interval::D1,
            tracked_intervals: vec![Interval::D1],
            base_bars: scaled_d1.clone(),
            base_candles,
            raw_base_bars: Some(raw_d1),
            controller: ctrl,
            speed_multiplier: 1.0,
            blind_drill: Some(BlindDrillMeta {
                enabled: true,
                alias_code: "MÃ ẨN #7777".to_owned(),
                price_scale,
                revealed: false,
                real_ticker: "HOSE:FPT".to_owned(),
                cut_date: "2024-01-18".to_owned(),
                time_shift_ms,
            }),
        };

        // 1. Save session MUST be blocked while unrevealed
        assert!(guard_save_replay_session_allowed(&rt).is_err());

        // 2. Coarser 1W materialization MUST scale prepended older bars to ~100.0 (never raw 125,000!)
        let w1_bars = materialize_replay_bars(&rt, Interval::W1, &store);
        assert!(w1_bars.len() >= 2);
        assert!(
            w1_bars.iter().all(|b| b.close > 80.0 && b.close < 120.0),
            "All 1W bars including prepended older history must be scaled to ~100.0, got: {:?}",
            w1_bars.iter().map(|b| b.close).collect::<Vec<_>>()
        );

        // 3. Finer 1H materialization MUST scale bars to ~100.0 AND cut future 1H bar strictly!
        let h1_bars = materialize_replay_bars(&rt, Interval::H1, &store);
        assert_eq!(h1_bars.len(), 1, "Future 1H bar must be cut strictly");
        assert!((h1_bars[0].close - 100.0).abs() < 1e-6);

        // 4. Armed alert on real ticker HOSE:FPT MUST NOT fire when evaluating with display_ticker ("MÃ ẨN #7777")
        let ws = crate::workspace_db::WorkspaceStore::open_in_memory().unwrap();
        ws.upsert_alert(&PriceAlertRecord {
            id: "al_real_fpt".to_owned(),
            ticker: "HOSE:FPT".to_owned(),
            alert_kind: "price".to_owned(),
            condition: "cross_below".to_owned(),
            target_price: 50_000.0,
            interval: Some("1D".to_owned()),
            params_json: None,
            sound_enabled: true,
            message: "SPOILER FPT".to_owned(),
            status: "armed".to_owned(),
            frequency: "once".to_owned(),
            cooldown_bars: 0,
            last_fired_bar_ts: None,
            created_at_ms: 0,
            triggered_at_ms: None,
        })
        .unwrap();
        let fired = evaluate_replay_bar_alerts(
            &ws,
            &display_ticker_for_runtime(&rt),
            99.0,
            &scaled_d1[cut_idx],
            &scaled_d1[..=cut_idx],
            Some("1D"),
        );
        assert!(fired.is_empty(), "Real ticker alert must NEVER fire during unrevealed Blind Drill!");
    }
}
