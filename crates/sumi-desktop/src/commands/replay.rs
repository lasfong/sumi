use serde::{Deserialize, Serialize};
use sumi_core::data::Timeframe;
use sumi_core::model::Candle;
use sumi_core::replay::{ReplayController, ReplaySpeed, ReplayStatus};
use sumi_data::{Adjustment, BarsRequest, Interval, OhlcvBar, SeriesKey, SymbolKey};
use tauri::State;

use crate::commands::broker::{
    BrokerStateSnapshot, build_broker_snapshot, ensure_broker_synced, persist_broker_snapshot,
};
use crate::error::{DesktopError, DesktopResult};
use crate::state::{AppState, ReplayRuntime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaySnapshotResponse {
    pub active: bool,
    pub status: ReplayStatus,
    pub ticker: String,
    pub interval: String,
    pub cursor_index: usize,
    pub total_bars: usize,
    pub cutoff_timestamp_ms: Option<i64>,
    pub speed_multiplier: f64,
    /// Strict DEF-001 causal slice (`base_bars[..=cursor_index]`, zero future leakage!).
    pub visible_bars: Vec<OhlcvBar>,
    /// Newly revealed single bar when stepping forward by +1 (for smooth KLineChart v10 `subscribeBar` push).
    pub stepped_bar: Option<OhlcvBar>,
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

#[tauri::command]
pub async fn replay_start(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    cut_timestamp_ms: Option<i64>,
) -> DesktopResult<ReplaySnapshotResponse> {
    let sym = SymbolKey::parse(&ticker)?;
    let iv: Interval = interval.parse()?;
    let series = SeriesKey::new(&sym, iv, Adjustment::Splits);
    let svc = state.active_market_service();

    // Load full local history (up to 4,000 bars) so replay has ample lookback and forward runway.
    let mut bars = svc
        .get_bars_instant(&BarsRequest::init(series.clone(), 4_000))
        .await?
        .bars;
    if bars.is_empty() {
        bars = svc.get_bars(&BarsRequest::init(series, 1_000)).await?.bars;
    }
    if bars.len() < 10 {
        return Err(DesktopError::Validation(
            "Cần tối thiểu 10 cây nến để khởi động Bar Replay nhân quả".to_owned(),
        ));
    }

    let base_candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
    let cut_idx = resolve_replay_cut_index(&bars, cut_timestamp_ms);

    let mut controller = ReplayController::new();
    controller.start_at_base_index(&base_candles, cut_idx);

    let visible_bars = bars[..=cut_idx].to_vec();
    let cutoff_ms = bars.get(cut_idx).map(|b| b.timestamp);
    let total_bars = bars.len();
    let status = controller.status;

    {
        let mut rg = state.replay.lock().await;
        *rg = Some(ReplayRuntime {
            ticker: sym.ticker(),
            interval: iv,
            base_bars: bars,
            base_candles: base_candles.clone(),
            controller,
            speed_multiplier: 1.0,
        });
    }

    // Reset Paper Broker cleanly at the Replay cut bar so orders placed during Replay match causally!
    let _ = ensure_broker_synced(&state, &sym.ticker(), iv.as_str()).await?;
    let broker_snapshot = {
        let mut broker = state.broker.lock().await;
        broker.base_candles = base_candles;
        broker.cursor_index = cut_idx;
        let candles_clone = broker.base_candles.clone();
        broker.session.reset_at(&candles_clone, cut_idx);
        broker.order_types_meta.clear();
        persist_broker_snapshot(&state, &broker);
        build_broker_snapshot(&state, &broker, true)?
    };

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker: sym.ticker(),
        interval: iv.as_str().to_owned(),
        cursor_index: cut_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_ms,
        speed_multiplier: 1.0,
        visible_bars,
        stepped_bar: None,
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
        status,
        prev_idx,
        new_idx,
        total_bars,
        speed_mult,
        visible_bars,
        stepped_bar,
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
        let stepped = if delta == 1 && new_idx == prev_idx + 1 {
            rt.base_bars.get(new_idx).copied()
        } else {
            None
        };
        let visible = if stepped.is_some() {
            // Avoid resending full history on single +1 step when client pushes steppedBar
            Vec::new()
        } else {
            rt.base_bars[..=new_idx].to_vec()
        };
        (
            rt.ticker.clone(),
            rt.interval,
            rt.controller.status,
            prev_idx,
            new_idx,
            rt.base_bars.len(),
            rt.speed_multiplier,
            visible,
            stepped,
            rt.base_candles.clone(),
        )
    };

    // Deterministically advance or rewind Paper Broker in lockstep with Replay cursor!
    let broker_snapshot = {
        let mut broker = state.broker.lock().await;
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
        persist_broker_snapshot(&state, &broker);
        build_broker_snapshot(&state, &broker, true)?
    };

    let cutoff_ms = {
        let rg = state.replay.lock().await;
        rg.as_ref()
            .and_then(|rt| rt.base_bars.get(new_idx).map(|b| b.timestamp))
    };

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker,
        interval: iv.as_str().to_owned(),
        cursor_index: new_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_ms,
        speed_multiplier: speed_mult,
        visible_bars,
        stepped_bar,
        broker: Some(broker_snapshot),
    })
}

#[tauri::command]
pub async fn replay_jump(
    state: State<'_, AppState>,
    target_timestamp_ms: i64,
) -> DesktopResult<ReplaySnapshotResponse> {
    let (ticker, iv, status, new_idx, total_bars, speed_mult, visible_bars, base_candles) = {
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
        (
            rt.ticker.clone(),
            rt.interval,
            rt.controller.status,
            new_idx,
            rt.base_bars.len(),
            rt.speed_multiplier,
            rt.base_bars[..=new_idx].to_vec(),
            rt.base_candles.clone(),
        )
    };

    let broker_snapshot = {
        let mut broker = state.broker.lock().await;
        broker.base_candles = base_candles.clone();
        broker.cursor_index = new_idx;
        let start = broker.session.session_start_base_index.min(new_idx);
        broker.session.rewind_to(&base_candles, start, new_idx);
        persist_broker_snapshot(&state, &broker);
        build_broker_snapshot(&state, &broker, true)?
    };

    let cutoff_ms = visible_bars.last().map(|b| b.timestamp);

    Ok(ReplaySnapshotResponse {
        active: true,
        status,
        ticker,
        interval: iv.as_str().to_owned(),
        cursor_index: new_idx,
        total_bars,
        cutoff_timestamp_ms: cutoff_ms,
        speed_multiplier: speed_mult,
        visible_bars,
        stepped_bar: None,
        broker: Some(broker_snapshot),
    })
}

#[tauri::command]
pub async fn replay_set_speed(
    state: State<'_, AppState>,
    speed_multiplier: f64,
) -> DesktopResult<f64> {
    let clamped = if speed_multiplier.is_finite() {
        speed_multiplier.clamp(0.5, 10.0)
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
            cursor_index: 0,
            total_bars: 0,
            cutoff_timestamp_ms: None,
            speed_multiplier: 1.0,
            visible_bars: Vec::new(),
            stepped_bar: None,
            broker: None,
        });
    };
    let idx = rt
        .controller
        .base_cursor_index
        .min(rt.base_bars.len().saturating_sub(1));
    let cutoff_ms = rt.base_bars.get(idx).map(|b| b.timestamp);
    Ok(ReplaySnapshotResponse {
        active: rt.controller.is_active(),
        status: rt.controller.status,
        ticker: rt.ticker.clone(),
        interval: rt.interval.as_str().to_owned(),
        cursor_index: idx,
        total_bars: rt.base_bars.len(),
        cutoff_timestamp_ms: cutoff_ms,
        speed_multiplier: rt.speed_multiplier,
        visible_bars: rt.base_bars[..=idx].to_vec(),
        stepped_bar: None,
        broker: None,
    })
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

        // Exact timestamp hit at index 42
        let exact = resolve_replay_cut_index(&bars, Some(base + 42 * step));
        assert_eq!(exact, 42);
        assert_eq!(bars[..=exact].len(), 43);

        // Timestamp between bar 42 and 43 still lands cleanly on bar 42 (zero future bar leakage!)
        let mid = resolve_replay_cut_index(&bars, Some(base + 42 * step + 12_000));
        assert_eq!(mid, 42);

        // Timestamp before bar 0 clamps safely to index 1
        let early = resolve_replay_cut_index(&bars, Some(base - 10_000));
        assert_eq!(early, 1);

        // Default cut when None leaves 60 bars of forward runway
        let default_idx = resolve_replay_cut_index(&bars, None);
        assert_eq!(default_idx, 140);
    }
}
