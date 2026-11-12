use std::cmp::Ordering;

use crate::data::Timeframe;
use crate::indicators::{
    BollingerOutput, MacdOutput, compute_atr, compute_bollinger_slices, compute_ema_slices,
    compute_macd_slices, compute_rsi_slices, compute_sma_slices,
};
use crate::model::Candle;

use super::{
    HistoricalSignalHit, MarketRegime, ScanSignalKind, ScannerFilter, SymbolScanRow,
    WatchlistSortColumn,
};

/// Default pre-signal lookback bars used when launching `ScannerReplayIntent::BlindPractice`.
pub const DEFAULT_BLIND_PRACTICE_LOOKBACK_BARS: usize = 15;

struct PrecomputedScannerSeries {
    sma_20: Vec<Option<f64>>,
    ema_20: Vec<Option<f64>>,
    ema_50: Vec<Option<f64>>,
    rsi_14: Vec<Option<f64>>,
    macd: MacdOutput,
    bb: BollingerOutput,
    atr_14: Vec<Option<f64>>,
    rvol_20: Vec<Option<f64>>,
}

impl PrecomputedScannerSeries {
    fn compute(candles: &[Candle]) -> Self {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let sma_20 = compute_sma_slices(&closes, 20);
        let ema_20 = compute_ema_slices(&closes, 20);
        let ema_50 = compute_ema_slices(&closes, 50);
        let rsi_14 = compute_rsi_slices(&closes, 14);
        let macd = compute_macd_slices(&closes, 12, 26, 9);
        let bb = compute_bollinger_slices(&closes, 20, 2.0);
        let atr_14 = compute_atr(candles, 14);
        let rvol_20 = compute_causal_rvol(candles, 20);
        Self {
            sma_20,
            ema_20,
            ema_50,
            rsi_14,
            macd,
            bb,
            atr_14,
            rvol_20,
        }
    }
}

/// Computes causal Relative Volume (`RVOL`) across chronologically ordered `candles` using a
/// strict prior window `candles[t - period .. t]` that **excludes** current bar `t`
/// (`ARCHIVE/backend/app/domain/signals/volume.py:94–103`).
#[must_use]
pub fn compute_causal_rvol(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    let n = candles.len();
    let mut out = vec![None; n];
    if period == 0 || period >= n {
        return out;
    }

    for (t, dst) in out.iter_mut().enumerate().skip(period) {
        let curr_vol = candles[t].volume;
        if !curr_vol.is_finite() || curr_vol < 0.0 {
            continue;
        }
        let window = &candles[t - period..t];
        let mut sum = 0.0_f64;
        let mut valid = true;
        for bar in window {
            if !bar.volume.is_finite() || bar.volume < 0.0 {
                valid = false;
                break;
            }
            sum += bar.volume;
        }
        if !valid {
            continue;
        }
        let baseline = sum / (period as f64);
        if baseline.is_finite() && baseline > 0.0 {
            let rvol = curr_vol / baseline;
            if rvol.is_finite() {
                *dst = Some(rvol);
            }
        }
    }
    out
}

#[inline]
fn causal_slice(candles: &[Candle], cutoff_timestamp: Option<i64>) -> &[Candle] {
    match cutoff_timestamp {
        Some(cutoff) => {
            let end = candles.partition_point(|c| c.timestamp <= cutoff);
            &candles[..end]
        }
        None => candles,
    }
}

#[inline]
fn bb_bandwidth_ratio(bb: &BollingerOutput, idx: usize) -> Option<f64> {
    let upper = bb.upper.get(idx).copied().flatten()?;
    let middle = bb.middle.get(idx).copied().flatten()?;
    let lower = bb.lower.get(idx).copied().flatten()?;
    if !upper.is_finite() || !middle.is_finite() || !lower.is_finite() || middle.abs() <= 1e-12 {
        return None;
    }
    let ratio = (upper - lower) / middle;
    ratio.is_finite().then_some(ratio)
}

fn classify_regime_at(
    candles: &[Candle],
    series: &PrecomputedScannerSeries,
    t: usize,
) -> MarketRegime {
    if t < 20 || t >= candles.len() {
        return MarketRegime::InsufficientHistory;
    }
    let close = candles[t].close;
    if !close.is_finite() {
        return MarketRegime::InsufficientHistory;
    }

    // 1. SIG-REG-001 / SIG-REG-002: Uptrend & Downtrend (EMA20 vs EMA50 + 5-bar EMA20 slope)
    if t >= 54
        && let (Some(e20), Some(e50), Some(e20_prev5)) = (
            series.ema_20[t],
            series.ema_50[t],
            series.ema_20[t.saturating_sub(5)],
        )
    {
        if close > e20 && e20 > e50 && e20 > e20_prev5 {
            return MarketRegime::Uptrend;
        }
        if close < e20 && e20 < e50 && e20 < e20_prev5 {
            return MarketRegime::Downtrend;
        }
    }

    // 2. SIG-REG-004: Pullback in Uptrend (strictly excludes bar t from prior 20-bar high: TEST-CAUSAL-003)
    if t >= 50
        && let (Some(e20), Some(e50), Some(atr)) =
            (series.ema_20[t], series.ema_50[t], series.atr_14[t])
        && e20 > e50
    {
        let mut recent_high = f64::NEG_INFINITY;
        for bar in &candles[t - 20..t] {
            if bar.high.is_finite() && bar.high > recent_high {
                recent_high = bar.high;
            }
        }
        if recent_high.is_finite() && recent_high > 1e-8 {
            let drawdown = (recent_high - close) / recent_high;
            let support = e50 - 0.5 * atr;
            if (0.02..=0.12).contains(&drawdown) && close >= support {
                return MarketRegime::Pullback;
            }
        }
    }

    // 3. SIG-REG-005: Early Recovery from Downtrend (strictly excludes bar t from prior 20-bar low)
    if t >= 50
        && let (Some(e20), Some(e50), Some(e20_prev3)) =
            (series.ema_20[t], series.ema_50[t], series.ema_20[t - 3])
        && e20 < e50
    {
        let mut recent_low = f64::INFINITY;
        for bar in &candles[t - 20..t] {
            if bar.low.is_finite() && bar.low < recent_low {
                recent_low = bar.low;
            }
        }
        let c1 = u8::from(close > e20);
        let c2 = u8::from(e20 > e20_prev3);
        let c3 = u8::from(recent_low.is_finite() && recent_low > 0.0 && close > recent_low * 1.03);
        if c1 + c2 + c3 >= 2 {
            return MarketRegime::Recovery;
        }
    }

    // 4. SIG-REG-003 / Squeeze: tight Bollinger bandwidth (<= 6%) or flat 5-bar EMA20 slope (< 0.8%)
    if let Some(bw) = bb_bandwidth_ratio(&series.bb, t)
        && bw <= 0.06
    {
        return MarketRegime::SidewaysSqueeze;
    }
    if t >= 25
        && let (Some(e20), Some(e20_prev5)) = (series.ema_20[t], series.ema_20[t - 5])
        && e20_prev5.abs() > 1e-8
        && ((e20 - e20_prev5).abs() / e20_prev5) < 0.008
    {
        return MarketRegime::SidewaysSqueeze;
    }

    MarketRegime::Neutral
}

fn evaluate_signals_at_bar(
    candles: &[Candle],
    series: &PrecomputedScannerSeries,
    t: usize,
) -> Vec<(ScanSignalKind, f64)> {
    let mut hits = Vec::new();
    if t == 0 || t >= candles.len() {
        return hits;
    }

    // 1. RSI(14) 30 / 70 crossovers
    if let (Some(prev_rsi), Some(curr_rsi)) = (series.rsi_14[t - 1], series.rsi_14[t]) {
        if prev_rsi <= 30.0 && curr_rsi > 30.0 {
            hits.push((ScanSignalKind::RsiOversoldCrossUp, curr_rsi));
        }
        if prev_rsi >= 70.0 && curr_rsi < 70.0 {
            hits.push((ScanSignalKind::RsiOverboughtCrossDown, curr_rsi));
        }
    }

    // 2. MACD(12, 26, 9) histogram zero crossovers (MACD line crossing Signal line)
    if let (Some(prev_h), Some(curr_h)) = (series.macd.histogram[t - 1], series.macd.histogram[t]) {
        if prev_h <= 0.0 && curr_h > 0.0 {
            hits.push((ScanSignalKind::MacdBullCross, curr_h));
        }
        if prev_h >= 0.0 && curr_h < 0.0 {
            hits.push((ScanSignalKind::MacdBearCross, curr_h));
        }
    }

    // 3. SMA(20) / EMA(50) Golden & Death crosses
    if let (Some(s_prev), Some(e_prev), Some(s_curr), Some(e_curr)) = (
        series.sma_20[t - 1],
        series.ema_50[t - 1],
        series.sma_20[t],
        series.ema_50[t],
    ) {
        let prev_diff = s_prev - e_prev;
        let curr_diff = s_curr - e_curr;
        if prev_diff <= 0.0 && curr_diff > 0.0 {
            hits.push((ScanSignalKind::SmaEmaGoldenCross, curr_diff));
        }
        if prev_diff >= 0.0 && curr_diff < 0.0 {
            hits.push((ScanSignalKind::SmaEmaDeathCross, curr_diff));
        }
    }

    // 4. Bollinger Squeeze Breakout (prior bandwidth <= 10%, close breaks above upper band)
    if t >= 20
        && let (Some(prev_bw), Some(prev_up), Some(curr_up)) = (
            bb_bandwidth_ratio(&series.bb, t - 1),
            series.bb.upper[t - 1],
            series.bb.upper[t],
        )
        && prev_bw <= 0.10
        && candles[t - 1].close <= prev_up
        && candles[t].close > curr_up
    {
        hits.push((ScanSignalKind::BollingerSqueezeBreakout, prev_bw * 100.0));
    }

    // 5. Causal Relative Volume >= 2.0x bullish expansion
    if t >= 20
        && let Some(rvol) = series.rvol_20[t]
        && rvol >= 2.0
        && candles[t].close >= candles[t].open
    {
        hits.push((ScanSignalKind::VolumeSpike2x, rvol));
    }

    hits
}

/// Evaluates BOTH the [`SymbolScanRow`] Watchlist snapshot AND the newest-first
/// [`HistoricalSignalHit`] list in a **single** indicator pass (`[LOW-01]`, cutting multi-symbol
/// scanner CPU cost by 50%).
///
/// When `exclude_incomplete_tail_signal` is `true` (active mid-bucket `BaseBar` replay on a higher
/// timeframe), close-confirmed signals on the unclosed tail bar `last_idx` are withheld until the
/// higher-timeframe bucket closes (`[CRITICAL-02]`).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_symbol_scan_and_signals(
    instrument_id: &str,
    symbol: &str,
    source_label: &str,
    timeframe: Timeframe,
    candles: &[Candle],
    cutoff_timestamp: Option<i64>,
    signal_lookback_bars: usize,
    exclude_incomplete_tail_signal: bool,
) -> (Option<SymbolScanRow>, Vec<HistoricalSignalHit>) {
    let slice = causal_slice(candles, cutoff_timestamp);
    let Some(last_idx) = slice.len().checked_sub(1) else {
        return (None, Vec::new());
    };
    let last = &slice[last_idx];
    let series = PrecomputedScannerSeries::compute(slice);

    let (change_abs, change_pct) = if last_idx >= 1 {
        let prev_close = slice[last_idx - 1].close;
        let diff = last.close - prev_close;
        let pct = if prev_close.abs() > 1e-12 {
            (diff / prev_close) * 100.0
        } else {
            0.0
        };
        (diff, pct)
    } else {
        (last.change(), last.change_pct())
    };

    let bb_bandwidth_pct = bb_bandwidth_ratio(&series.bb, last_idx).map(|r| r * 100.0);
    let atr_pct = series.atr_14[last_idx].and_then(|atr| {
        if last.close.abs() > 1e-12 {
            let pct = (atr / last.close) * 100.0;
            pct.is_finite().then_some(pct)
        } else {
            None
        }
    });

    let regime = classify_regime_at(slice, &series, last_idx);
    let active_signals: Vec<ScanSignalKind> = if exclude_incomplete_tail_signal {
        Vec::new()
    } else {
        evaluate_signals_at_bar(slice, &series, last_idx)
            .into_iter()
            .map(|(s, _)| s)
            .collect()
    };

    let max_signal_idx_opt = if exclude_incomplete_tail_signal {
        last_idx.checked_sub(1)
    } else {
        Some(last_idx)
    };

    let mut hits = Vec::new();
    if let Some(max_sig_idx) = max_signal_idx_opt
        && max_sig_idx >= 1
    {
        let start_idx = if signal_lookback_bars == 0 {
            1
        } else {
            last_idx.saturating_sub(signal_lookback_bars).max(1)
        };
        if start_idx <= max_sig_idx {
            for idx in (start_idx..=max_sig_idx).rev() {
                let bar_hits = evaluate_signals_at_bar(slice, &series, idx);
                if bar_hits.is_empty() {
                    continue;
                }
                let bar_regime = classify_regime_at(slice, &series, idx);
                for (sig, metric_value) in bar_hits {
                    hits.push(HistoricalSignalHit {
                        instrument_id: instrument_id.to_owned(),
                        symbol: symbol.to_owned(),
                        timeframe,
                        bar_index: idx,
                        timestamp: slice[idx].timestamp,
                        price: slice[idx].close,
                        signal: sig,
                        direction: sig.direction(),
                        regime: bar_regime,
                        metric_value,
                    });
                }
            }
        }
    }

    let latest_signal = hits.first().cloned();
    let row = SymbolScanRow {
        instrument_id: instrument_id.to_owned(),
        symbol: symbol.to_owned(),
        source_label: source_label.to_owned(),
        timeframe,
        bar_count: slice.len(),
        last_timestamp: last.timestamp,
        last_close: last.close,
        change_abs,
        change_pct,
        last_volume: last.volume,
        rvol_20: series.rvol_20[last_idx],
        sma_20: series.sma_20[last_idx],
        ema_50: series.ema_50[last_idx],
        rsi_14: series.rsi_14[last_idx],
        macd_hist: series.macd.histogram[last_idx],
        bb_bandwidth_pct,
        atr_pct,
        regime,
        active_signals,
        latest_signal,
    };

    (Some(row), hits)
}

/// Evaluates the causal [`SymbolScanRow`] Watchlist snapshot for a single instrument up to an
/// optional `cutoff_timestamp` (`DEF-001` zero-lookahead).
#[must_use]
pub fn evaluate_symbol_scan(
    instrument_id: &str,
    symbol: &str,
    source_label: &str,
    timeframe: Timeframe,
    candles: &[Candle],
    cutoff_timestamp: Option<i64>,
    signal_lookback_bars: usize,
) -> Option<SymbolScanRow> {
    evaluate_symbol_scan_and_signals(
        instrument_id,
        symbol,
        source_label,
        timeframe,
        candles,
        cutoff_timestamp,
        signal_lookback_bars.max(1),
        false,
    )
    .0
}

/// Scans an instrument's causal history (`<= cutoff_timestamp`) over the most recent `lookback_bars`
/// and returns [`HistoricalSignalHit`]s ordered newest-first (`timestamp` desc, then `ScanSignalKind`).
#[must_use]
pub fn scan_historical_signals(
    instrument_id: &str,
    symbol: &str,
    timeframe: Timeframe,
    candles: &[Candle],
    cutoff_timestamp: Option<i64>,
    lookback_bars: usize,
) -> Vec<HistoricalSignalHit> {
    evaluate_symbol_scan_and_signals(
        instrument_id,
        symbol,
        "",
        timeframe,
        candles,
        cutoff_timestamp,
        lookback_bars,
        false,
    )
    .1
}

#[inline]
fn compare_opt_f64_none_last(a: Option<f64>, b: Option<f64>, ascending: bool) -> Ordering {
    let af = a.filter(|v| v.is_finite());
    let bf = b.filter(|v| v.is_finite());
    match (af, bf) {
        (Some(va), Some(vb)) => {
            let ord = va.total_cmp(&vb);
            if ascending { ord } else { ord.reverse() }
        }
        // Valid finite metrics ALWAYS precede `None`/`NaN` regardless of sort direction!
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[inline]
fn compare_f64_finite_first(a: f64, b: f64, ascending: bool) -> Ordering {
    compare_opt_f64_none_last(Some(a), Some(b), ascending)
}

/// Filters and deterministically sorts [`SymbolScanRow`] rows, placing `None` / `NaN` metrics last
/// in **both** ascending and descending modes and breaking ties by `(symbol, instrument_id)`.
#[must_use]
pub fn filter_and_sort_watchlist(
    rows: &[SymbolScanRow],
    filter: &ScannerFilter,
    sort_col: WatchlistSortColumn,
    ascending: bool,
) -> Vec<SymbolScanRow> {
    let q = filter.search_query.trim().to_ascii_lowercase();
    let mut filtered: Vec<SymbolScanRow> = rows
        .iter()
        .filter(|row| {
            if !q.is_empty()
                && !row.symbol.to_ascii_lowercase().contains(&q)
                && !row.source_label.to_ascii_lowercase().contains(&q)
                && !row.regime.full_label().to_ascii_lowercase().contains(&q)
            {
                return false;
            }
            if let Some(req_regime) = filter.regime_filter
                && row.regime != req_regime
            {
                return false;
            }
            if filter.only_active_signals && row.active_signals.is_empty() {
                return false;
            }
            if let Some(req_sig) = filter.signal_filter {
                let matches_active = row.active_signals.contains(&req_sig);
                let matches_latest = !filter.only_active_signals
                    && row
                        .latest_signal
                        .as_ref()
                        .is_some_and(|hit| hit.signal == req_sig);
                if !matches_active && !matches_latest {
                    return false;
                }
            }
            if let Some(req_dir) = filter.direction_filter {
                let matches_active = row.active_signals.iter().any(|s| s.direction() == req_dir);
                let matches_latest = !filter.only_active_signals
                    && row
                        .latest_signal
                        .as_ref()
                        .is_some_and(|hit| hit.direction == req_dir);
                if !matches_active && !matches_latest {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    filtered.sort_by(|a, b| {
        let primary = match sort_col {
            WatchlistSortColumn::Symbol => {
                let ord = a
                    .symbol
                    .to_ascii_uppercase()
                    .cmp(&b.symbol.to_ascii_uppercase());
                if ascending { ord } else { ord.reverse() }
            }
            WatchlistSortColumn::LastClose => {
                compare_f64_finite_first(a.last_close, b.last_close, ascending)
            }
            WatchlistSortColumn::ChangePct => {
                compare_f64_finite_first(a.change_pct, b.change_pct, ascending)
            }
            WatchlistSortColumn::RelativeVolume => {
                compare_opt_f64_none_last(a.rvol_20, b.rvol_20, ascending)
            }
            WatchlistSortColumn::Rsi14 => compare_opt_f64_none_last(a.rsi_14, b.rsi_14, ascending),
            WatchlistSortColumn::SignalCount => {
                let ca = a.active_signals.len() * 2 + usize::from(a.latest_signal.is_some());
                let cb = b.active_signals.len() * 2 + usize::from(b.latest_signal.is_some());
                let ord = ca.cmp(&cb);
                if ascending { ord } else { ord.reverse() }
            }
        };
        primary
            .then_with(|| {
                a.symbol
                    .to_ascii_uppercase()
                    .cmp(&b.symbol.to_ascii_uppercase())
            })
            .then_with(|| a.instrument_id.cmp(&b.instrument_id))
    });

    filtered
}

/// Applies [`ScannerFilter`] to a slice of [`HistoricalSignalHit`]s, preserving deterministic
/// newest-first (`timestamp` desc, `symbol` asc) ordering.
#[must_use]
pub fn filter_historical_signals(
    hits: &[HistoricalSignalHit],
    filter: &ScannerFilter,
) -> Vec<HistoricalSignalHit> {
    let q = filter.search_query.trim().to_ascii_lowercase();
    let mut out: Vec<HistoricalSignalHit> = hits
        .iter()
        .filter(|hit| {
            if !q.is_empty()
                && !hit.symbol.to_ascii_lowercase().contains(&q)
                && !hit.signal.full_label().to_ascii_lowercase().contains(&q)
                && !hit.signal.short_label().to_ascii_lowercase().contains(&q)
            {
                return false;
            }
            if let Some(sig) = filter.signal_filter
                && hit.signal != sig
            {
                return false;
            }
            if let Some(reg) = filter.regime_filter
                && hit.regime != reg
            {
                return false;
            }
            if let Some(dir) = filter.direction_filter
                && hit.direction != dir
            {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        b.timestamp
            .cmp(&a.timestamp)
            .then_with(|| {
                a.symbol
                    .to_ascii_uppercase()
                    .cmp(&b.symbol.to_ascii_uppercase())
            })
            .then_with(|| a.bar_index.cmp(&b.bar_index))
    });
    out
}
