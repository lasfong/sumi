use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::backtest::is_bucket_incomplete_at_close_for_symbol;
use crate::data::{MAX_ABS_PRICE, Timeframe, aggregate_ohlcv};
use crate::drawings::{
    Drawing, DrawingGeometry, DrawingPaneTarget, DrawingStore, MAX_ABS_ANCHOR_TIMESTAMP,
};
use crate::indicators::{compute_macd_slices, compute_rsi_slices};
use crate::journal::ledger::sanitize_csv_cell;
use crate::model::Candle;
use crate::scanner::{
    MarketRegime, PrecomputedCatalogMatrix, SIGNAL_CATALOG, SignalDirection, SignalOutputType,
    catalog_entry_index, find_catalog_entry,
};

use super::{
    AlertCondition, AlertError, AlertFrequency, AlertId, AlertRule, AlertStatus, AlertTriggerEvent,
    CrossDirection, truncate_utf8_bytes,
};

/// Upper bound on persisted alert rules per instrument.
pub const MAX_ALERT_RULES: usize = 128;

/// Upper bound on retained causal trigger events per instrument.
pub const MAX_TRIGGER_LOG_EVENTS: usize = 256;

/// Upper bound on causal cooldown suppression bars (`OncePerBarClose`).
pub const MAX_ALERT_COOLDOWN_BARS: usize = 500;

const MAX_SYMBOL_BYTES: usize = 64;
const MAX_SUMMARY_BYTES: usize = 128;
pub const MAX_MESSAGE_BYTES: usize = 160;

/// Evaluates the exact domain price of a `Main`-pane [`Drawing`] (`HorizontalLine`, finite
/// `TrendLine`, or $360^\circ$ `Ray`) at integer logical `bar_index`.
#[must_use]
pub fn drawing_line_value_at_bar(drawing: &Drawing, bar_index: usize) -> Option<f64> {
    if !drawing.visible || drawing.pane != DrawingPaneTarget::Main {
        return None;
    }
    let x = bar_index as f64;
    match drawing.geometry {
        DrawingGeometry::HorizontalLine { anchor } => {
            (anchor.value.is_finite() && anchor.value > 0.0 && anchor.value <= MAX_ABS_PRICE)
                .then_some(anchor.value)
        }
        DrawingGeometry::TrendLine { start, end } => {
            let dx = end.bar_index - start.bar_index;
            if !dx.is_finite() || dx.abs() < 1e-9 {
                return None;
            }
            let left_bar = start.bar_index.min(end.bar_index);
            let right_bar = start.bar_index.max(end.bar_index);
            if !drawing.trend_extend_left && x < left_bar - 1e-9 {
                return None;
            }
            if !drawing.trend_extend_right && x > right_bar + 1e-9 {
                return None;
            }
            let mut u = (x - start.bar_index) / dx;
            if !drawing.trend_extend_left && !drawing.trend_extend_right {
                u = u.clamp(0.0, 1.0);
            }
            let val = start.value + u * (end.value - start.value);
            (val.is_finite() && val.abs() <= MAX_ABS_PRICE).then_some(val)
        }
        DrawingGeometry::Ray { origin, through } => {
            let dx = through.bar_index - origin.bar_index;
            if !dx.is_finite() || dx.abs() < 1e-9 {
                return None;
            }
            let u = (x - origin.bar_index) / dx;
            if u < -1e-9 {
                return None;
            }
            let val = origin.value + u.max(0.0) * (through.value - origin.value);
            (val.is_finite() && val.abs() <= MAX_ABS_PRICE).then_some(val)
        }
        DrawingGeometry::ParallelChannel { .. }
        | DrawingGeometry::Rectangle { .. }
        | DrawingGeometry::FibonacciRetracement { .. }
        | DrawingGeometry::RiskRewardBox { .. }
        | DrawingGeometry::PriceDateRange { .. }
        | DrawingGeometry::FixedRangeVolumeProfile { .. }
        | DrawingGeometry::AnchoredVwap { .. }
        | DrawingGeometry::TextNote { .. } => None,
    }
}

/// Resolves the reference line price at `prev_bar_index` (`t - 1`), preserving gap-open continuity
/// when stepping onto the very first covered bar of a `TrendLine` or rightward `Ray` (`[HIGH-03]`)
/// while extrapolating along the line's true slope so parallel channels never false-trigger!
fn drawing_line_prev_reference(drawing: &Drawing, prev_bar_index: usize) -> Option<f64> {
    if let Some(v) = drawing_line_value_at_bar(drawing, prev_bar_index) {
        return Some(v);
    }
    let px = prev_bar_index as f64;
    match drawing.geometry {
        DrawingGeometry::TrendLine { start, end } => {
            let dx = end.bar_index - start.bar_index;
            if !dx.is_finite() || dx.abs() < 1e-9 {
                return None;
            }
            let first_x = start.bar_index.min(end.bar_index);
            if px <= first_x && first_x <= px + 1.0 + 1e-9 {
                let u = (px - start.bar_index) / dx;
                let val = start.value + u * (end.value - start.value);
                (val.is_finite() && val.abs() <= MAX_ABS_PRICE).then_some(val)
            } else {
                None
            }
        }
        DrawingGeometry::Ray { origin, through } => {
            let dx = through.bar_index - origin.bar_index;
            if dx.is_finite()
                && dx >= 1e-9
                && px <= origin.bar_index
                && origin.bar_index <= px + 1.0 + 1e-9
            {
                let u = (px - origin.bar_index) / dx;
                let val = origin.value + u * (through.value - origin.value);
                (val.is_finite() && val.abs() <= MAX_ABS_PRICE).then_some(val)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Evaluates whether the 4-waypoint OHLC trajectory (`[prev_close?, open, w1, w2, close]`) crosses
/// `level` in `direction`, returning `Some((crossed_direction, observed_price))` on first match.
#[must_use]
pub fn evaluate_4_waypoint_cross(
    prev_close: Option<f64>,
    bar: &Candle,
    level: f64,
    direction: CrossDirection,
) -> Option<(CrossDirection, f64)> {
    evaluate_4_waypoint_cross_with_depart(prev_close, bar, level, direction, true, true)
}

fn evaluate_4_waypoint_cross_with_depart(
    prev_close: Option<f64>,
    bar: &Candle,
    level: f64,
    direction: CrossDirection,
    allow_depart_up: bool,
    allow_depart_dn: bool,
) -> Option<(CrossDirection, f64)> {
    if !level.is_finite() || !bar.is_valid() {
        return None;
    }
    let (w1, w2) = if bar.close >= bar.open {
        (bar.low, bar.high)
    } else {
        (bar.high, bar.low)
    };
    let mut pts = [0.0_f64; 5];
    let len = if let Some(pc) = prev_close.filter(|v| v.is_finite()) {
        pts[0] = pc;
        pts[1] = bar.open;
        pts[2] = w1;
        pts[3] = w2;
        pts[4] = bar.close;
        5
    } else {
        pts[0] = bar.open;
        pts[1] = w1;
        pts[2] = w2;
        pts[3] = bar.close;
        4
    };

    for seg in pts[..len].windows(2) {
        let a = seg[0];
        let b = seg[1];
        let crossed_up = (a < level && b >= level) || (allow_depart_up && a == level && b > level);
        let crossed_dn = (a > level && b <= level) || (allow_depart_dn && a == level && b < level);
        match direction {
            CrossDirection::CrossAbove if crossed_up => {
                return Some((CrossDirection::CrossAbove, b.max(level)));
            }
            CrossDirection::CrossBelow if crossed_dn => {
                return Some((CrossDirection::CrossBelow, b.min(level)));
            }
            CrossDirection::CrossAny if crossed_up => {
                return Some((CrossDirection::CrossAbove, b.max(level)));
            }
            CrossDirection::CrossAny if crossed_dn => {
                return Some((CrossDirection::CrossBelow, b.min(level)));
            }
            _ => {}
        }
    }
    None
}

/// Evaluates zero-crossings along a relative signed offset trajectory `d = price - line_value`.
fn evaluate_relative_offset_zero_cross_with_depart(
    prev_offset: Option<f64>,
    bar: &Candle,
    line_value: f64,
    direction: CrossDirection,
    allow_depart_up: bool,
    allow_depart_dn: bool,
) -> Option<(CrossDirection, f64)> {
    if !line_value.is_finite() || !bar.is_valid() {
        return None;
    }
    let (w1, w2) = if bar.close >= bar.open {
        (bar.low, bar.high)
    } else {
        (bar.high, bar.low)
    };
    let mut offsets = [0.0_f64; 5];
    let mut prices = [0.0_f64; 5];
    let len = if let Some(d0) = prev_offset.filter(|v| v.is_finite()) {
        offsets[0] = d0;
        prices[0] = line_value + d0;
        offsets[1] = bar.open - line_value;
        prices[1] = bar.open;
        offsets[2] = w1 - line_value;
        prices[2] = w1;
        offsets[3] = w2 - line_value;
        prices[3] = w2;
        offsets[4] = bar.close - line_value;
        prices[4] = bar.close;
        5
    } else {
        offsets[0] = bar.open - line_value;
        prices[0] = bar.open;
        offsets[1] = w1 - line_value;
        prices[1] = w1;
        offsets[2] = w2 - line_value;
        prices[2] = w2;
        offsets[3] = bar.close - line_value;
        prices[3] = bar.close;
        4
    };

    for idx in 0..(len - 1) {
        let da = offsets[idx];
        let db = offsets[idx + 1];
        let pb = prices[idx + 1];
        let crossed_up = (da < 0.0 && db >= 0.0) || (allow_depart_up && da == 0.0 && db > 0.0);
        let crossed_dn = (da > 0.0 && db <= 0.0) || (allow_depart_dn && da == 0.0 && db < 0.0);
        match direction {
            CrossDirection::CrossAbove if crossed_up => {
                return Some((CrossDirection::CrossAbove, pb.max(line_value)));
            }
            CrossDirection::CrossBelow if crossed_dn => {
                return Some((CrossDirection::CrossBelow, pb.min(line_value)));
            }
            CrossDirection::CrossAny if crossed_up => {
                return Some((CrossDirection::CrossAbove, pb.max(line_value)));
            }
            CrossDirection::CrossAny if crossed_dn => {
                return Some((CrossDirection::CrossBelow, pb.min(line_value)));
            }
            _ => {}
        }
    }
    None
}

fn validate_condition(
    condition: &AlertCondition,
    drawings: Option<&DrawingStore>,
) -> Result<(), AlertError> {
    match *condition {
        AlertCondition::PriceLevel { price, .. } => {
            if !price.is_finite() || price <= 0.0 || price > MAX_ABS_PRICE {
                return Err(AlertError::InvalidPriceLevel);
            }
            Ok(())
        }
        AlertCondition::RsiThreshold { period, level, .. } => {
            if !(2..=250).contains(&period) || !level.is_finite() || !(0.0..=100.0).contains(&level)
            {
                return Err(AlertError::InvalidIndicatorParams);
            }
            Ok(())
        }
        AlertCondition::MacdHistogramZeroCross {
            fast, slow, signal, ..
        } => {
            if fast < 2 || slow < 2 || signal < 2 || fast >= slow || slow > 250 || signal > 250 {
                return Err(AlertError::InvalidIndicatorParams);
            }
            Ok(())
        }
        AlertCondition::DrawingLine { drawing_id, .. } => {
            if drawing_id == 0 {
                return Err(AlertError::DrawingNotFound(0));
            }
            if let Some(store) = drawings {
                let Some(d) = store.drawings.iter().find(|d| d.id == drawing_id) else {
                    return Err(AlertError::DrawingNotFound(drawing_id));
                };
                if d.pane != DrawingPaneTarget::Main {
                    return Err(AlertError::UnsupportedDrawingPane);
                }
                if !d.visible || !d.geometry.is_valid() {
                    return Err(AlertError::UnsupportedDrawingGeometry);
                }
                match d.geometry {
                    DrawingGeometry::HorizontalLine { anchor } => {
                        if !anchor.value.is_finite()
                            || anchor.value <= 0.0
                            || anchor.value > MAX_ABS_PRICE
                        {
                            return Err(AlertError::UnsupportedDrawingGeometry);
                        }
                        Ok(())
                    }
                    DrawingGeometry::TrendLine { start, end } => {
                        let dx = end.bar_index - start.bar_index;
                        if !dx.is_finite() || dx.abs() < 1e-9 {
                            return Err(AlertError::UnsupportedDrawingGeometry);
                        }
                        Ok(())
                    }
                    DrawingGeometry::Ray { origin, through } => {
                        let dx = through.bar_index - origin.bar_index;
                        if !dx.is_finite() || dx.abs() < 1e-9 {
                            return Err(AlertError::UnsupportedDrawingGeometry);
                        }
                        Ok(())
                    }
                    DrawingGeometry::ParallelChannel { .. }
                    | DrawingGeometry::Rectangle { .. }
                    | DrawingGeometry::FibonacciRetracement { .. }
                    | DrawingGeometry::RiskRewardBox { .. }
                    | DrawingGeometry::PriceDateRange { .. }
                    | DrawingGeometry::FixedRangeVolumeProfile { .. }
                    | DrawingGeometry::AnchoredVwap { .. }
                    | DrawingGeometry::TextNote { .. } => {
                        Err(AlertError::UnsupportedDrawingGeometry)
                    }
                }
            } else {
                Ok(())
            }
        }
        AlertCondition::SignalCatalogTrigger {
            ref catalog_id,
            float_level,
            ..
        } => {
            let trimmed = catalog_id.trim();
            let Some(entry) = find_catalog_entry(trimmed) else {
                return Err(AlertError::UnknownSignalCatalogId(catalog_id.clone()));
            };
            match entry.output_type {
                SignalOutputType::Float => {
                    let Some(lvl) = float_level else {
                        return Err(AlertError::InvalidIndicatorParams);
                    };
                    if !lvl.is_finite() || lvl.abs() > MAX_ABS_PRICE {
                        return Err(AlertError::InvalidIndicatorParams);
                    }
                }
                SignalOutputType::Bool | SignalOutputType::Enum => {
                    if let Some(lvl) = float_level
                        && (!lvl.is_finite() || lvl.abs() > MAX_ABS_PRICE)
                    {
                        return Err(AlertError::InvalidIndicatorParams);
                    }
                }
            }
            Ok(())
        }
        AlertCondition::ConfluenceThreshold { threshold, .. } => {
            if !threshold.is_finite() || !(-100.0..=100.0).contains(&threshold) {
                return Err(AlertError::InvalidConfluenceThreshold);
            }
            Ok(())
        }
    }
}

#[inline]
fn bucket_end_base_index(base_prefix: &[Candle], tf: Timeframe, bucket_ts: i64) -> usize {
    base_prefix
        .partition_point(|c| tf.bucket_start_timestamp(c.timestamp) <= bucket_ts)
        .saturating_sub(1)
        .min(base_prefix.len() - 1)
}

#[inline]
fn aggregate_candle_slice(bars: &[Candle]) -> Option<Candle> {
    let first = bars.first()?;
    let last = bars.last()?;
    let mut high = first.high;
    let mut low = first.low;
    let mut volume = 0.0;
    for b in bars {
        high = high.max(b.high);
        low = low.min(b.low);
        volume += b.volume;
    }
    Some(Candle {
        timestamp: last.timestamp,
        date_label: String::new(),
        open: first.open,
        high,
        low,
        close: last.close,
        volume,
    })
}

#[inline]
fn match_scalar_cross(
    prev_v: f64,
    curr_v: f64,
    level: f64,
    dir: CrossDirection,
) -> Option<CrossDirection> {
    let up = prev_v <= level && curr_v > level;
    let dn = prev_v >= level && curr_v < level;
    match dir {
        CrossDirection::CrossAbove if up => Some(CrossDirection::CrossAbove),
        CrossDirection::CrossBelow if dn => Some(CrossDirection::CrossBelow),
        CrossDirection::CrossAny if up => Some(CrossDirection::CrossAbove),
        CrossDirection::CrossAny if dn => Some(CrossDirection::CrossBelow),
        _ => None,
    }
}

/// Serializable per-instrument causal Alert Engine (`Milestone R08`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AlertEngine {
    pub rules: Vec<AlertRule>,
    pub triggers: Vec<AlertTriggerEvent>,
    pub(crate) next_id: AlertId,
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertEngine {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rules: Vec::new(),
            triggers: Vec::new(),
            next_id: 1,
        }
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.triggers.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn armed_count(&self, replay_cutoff_timestamp: Option<i64>) -> usize {
        self.rules
            .iter()
            .filter(|r| {
                r.is_causally_visible(replay_cutoff_timestamp) && r.status == AlertStatus::Armed
            })
            .count()
    }

    #[inline]
    #[must_use]
    pub fn triggered_count(&self, replay_cutoff_timestamp: Option<i64>) -> usize {
        self.rules
            .iter()
            .filter(|r| {
                r.is_causally_visible(replay_cutoff_timestamp) && r.status == AlertStatus::Triggered
            })
            .count()
    }

    /// Sanitizes deserialized alert rules and trigger history (`[CRITICAL-01]`, `[MEDIUM-03]`).
    pub fn sanitize_deserialized(&mut self) {
        self.rules.retain(|rule| {
            rule.id > 0
                && !rule.symbol.trim().is_empty()
                && validate_condition(&rule.condition, None).is_ok()
                && (rule.armed_at_timestamp as f64).abs() <= MAX_ABS_ANCHOR_TIMESTAMP
                && rule
                    .created_at_timestamp
                    .is_none_or(|ts| (ts as f64).abs() <= MAX_ABS_ANCHOR_TIMESTAMP)
        });
        if self.rules.len() > MAX_ALERT_RULES {
            self.rules.truncate(MAX_ALERT_RULES);
        }

        let mut seen = HashSet::with_capacity(self.rules.len());
        self.rules.retain(|r| seen.insert(r.id));

        for rule in &mut self.rules {
            truncate_utf8_bytes(&mut rule.symbol, MAX_SYMBOL_BYTES);
            truncate_utf8_bytes(&mut rule.message, MAX_MESSAGE_BYTES);
            rule.cooldown_bars = rule.cooldown_bars.min(MAX_ALERT_COOLDOWN_BARS);
            rule.cooldown_remaining_bars = 0;
            if let AlertCondition::SignalCatalogTrigger {
                ref mut catalog_id,
                ref mut float_level,
                ..
            } = rule.condition
            {
                *catalog_id = catalog_id.trim().to_owned();
                if find_catalog_entry(catalog_id)
                    .is_some_and(|e| e.output_type != SignalOutputType::Float)
                {
                    *float_level = None;
                }
            }
            rule.armed_bucket_timestamp = rule
                .timeframe
                .bucket_start_timestamp(rule.armed_at_timestamp);
        }

        let max_id = self.rules.iter().map(|r| r.id).max().unwrap_or(0);
        if max_id >= u64::MAX - 2 || self.next_id >= u64::MAX - 2 {
            let mut remap = BTreeMap::new();
            for (idx, rule) in self.rules.iter_mut().enumerate() {
                let new_id = (idx as u64) + 1;
                remap.insert(rule.id, new_id);
                rule.id = new_id;
            }
            for ev in &mut self.triggers {
                if let Some(&mapped) = remap.get(&ev.alert_id) {
                    ev.alert_id = mapped;
                }
            }
            self.next_id = (self.rules.len() as u64) + 1;
        } else {
            self.next_id = self.next_id.max(max_id.saturating_add(1)).max(1);
        }

        let valid_rule_ids: HashSet<AlertId> = self.rules.iter().map(|r| r.id).collect();
        self.triggers.retain(|ev| {
            valid_rule_ids.contains(&ev.alert_id)
                && !ev.symbol.trim().is_empty()
                && ev.threshold_value.is_finite()
                && ev.observed_value.is_finite()
                && (ev.bar_timestamp as f64).abs() <= MAX_ABS_ANCHOR_TIMESTAMP
        });
        if self.triggers.len() > MAX_TRIGGER_LOG_EVENTS {
            let drop_n = self.triggers.len() - MAX_TRIGGER_LOG_EVENTS;
            self.triggers.drain(0..drop_n);
        }
        for ev in &mut self.triggers {
            if ev.eval_base_timestamp.is_none() {
                ev.eval_base_timestamp = Some(ev.bar_timestamp);
            }
            truncate_utf8_bytes(&mut ev.symbol, MAX_SYMBOL_BYTES);
            truncate_utf8_bytes(&mut ev.condition_summary, MAX_SUMMARY_BYTES);
            truncate_utf8_bytes(&mut ev.message, MAX_MESSAGE_BYTES);
        }
        self.triggers.sort_by(|a, b| {
            a.effective_eval_base_timestamp()
                .cmp(&b.effective_eval_base_timestamp())
                .then_with(|| a.bar_timestamp.cmp(&b.bar_timestamp))
                .then_with(|| a.alert_id.cmp(&b.alert_id))
        });
        for (idx, ev) in self.triggers.iter_mut().enumerate() {
            ev.event_id = (idx as u64) + 1;
        }
    }

    fn allocate_id(&mut self) -> AlertId {
        if self.next_id >= u64::MAX - 2 {
            let mut remap = BTreeMap::new();
            for (idx, rule) in self.rules.iter_mut().enumerate() {
                let new_id = (idx as u64) + 1;
                remap.insert(rule.id, new_id);
                rule.id = new_id;
            }
            for ev in &mut self.triggers {
                if let Some(&mapped) = remap.get(&ev.alert_id) {
                    ev.alert_id = mapped;
                }
            }
            self.next_id = (self.rules.len() as u64) + 1;
        }
        let id = self.next_id.max(1);
        self.next_id = id.saturating_add(1);
        id
    }

    /// Arms a new [`AlertRule`] at the current causal chart frontier (`base_candles[cursor_base_index]`).
    #[allow(clippy::too_many_arguments)]
    pub fn add_alert(
        &mut self,
        symbol: &str,
        timeframe: Timeframe,
        condition: AlertCondition,
        frequency: AlertFrequency,
        message: Option<String>,
        base_candles: &[Candle],
        cursor_base_index: usize,
        replay_cutoff_timestamp: Option<i64>,
        drawings: Option<&DrawingStore>,
    ) -> Result<AlertId, AlertError> {
        self.add_alert_with_cooldown(
            symbol,
            timeframe,
            condition,
            frequency,
            0,
            message,
            base_candles,
            cursor_base_index,
            replay_cutoff_timestamp,
            drawings,
        )
    }

    /// Arms a new [`AlertRule`] with explicit `cooldown_bars` (`0..=MAX_ALERT_COOLDOWN_BARS`).
    #[allow(clippy::too_many_arguments)]
    pub fn add_alert_with_cooldown(
        &mut self,
        symbol: &str,
        timeframe: Timeframe,
        mut condition: AlertCondition,
        frequency: AlertFrequency,
        cooldown_bars: usize,
        message: Option<String>,
        base_candles: &[Candle],
        cursor_base_index: usize,
        replay_cutoff_timestamp: Option<i64>,
        drawings: Option<&DrawingStore>,
    ) -> Result<AlertId, AlertError> {
        if base_candles.is_empty() {
            return Err(AlertError::EmptySeries);
        }
        validate_condition(&condition, drawings)?;
        if let AlertCondition::SignalCatalogTrigger {
            ref mut catalog_id,
            ref mut float_level,
            ..
        } = condition
        {
            *catalog_id = catalog_id.trim().to_owned();
            if find_catalog_entry(catalog_id)
                .is_some_and(|e| e.output_type != SignalOutputType::Float)
            {
                *float_level = None;
            }
        }

        while self.rules.len() >= MAX_ALERT_RULES {
            self.rules.remove(0);
        }

        let clamped_idx = cursor_base_index.min(base_candles.len() - 1);
        let armed_at_timestamp =
            replay_cutoff_timestamp.unwrap_or(base_candles[clamped_idx].timestamp);
        let armed_bucket_timestamp = timeframe.bucket_start_timestamp(armed_at_timestamp);
        let id = self.allocate_id();
        let mut clean_sym = symbol.trim().to_owned();
        if clean_sym.is_empty() {
            clean_sym = "UNKNOWN".to_owned();
        }
        truncate_utf8_bytes(&mut clean_sym, MAX_SYMBOL_BYTES);

        let default_msg = format!("{clean_sym} · {}", condition.summary());
        let mut msg = message
            .map(|m| m.trim().to_owned())
            .filter(|m| !m.is_empty())
            .unwrap_or(default_msg);
        truncate_utf8_bytes(&mut msg, MAX_MESSAGE_BYTES);

        self.rules.push(AlertRule {
            id,
            symbol: clean_sym,
            timeframe,
            condition,
            frequency,
            status: AlertStatus::Armed,
            armed_at_timestamp,
            armed_bucket_timestamp,
            // Always stamp causal creation timestamp (even when authored in Live mode) so rewinding
            // Bar Replay into the past masks future-created alerts (`DEF-001`, `BUG-028`).
            created_at_timestamp: Some(replay_cutoff_timestamp.unwrap_or(armed_at_timestamp)),
            cooldown_bars: cooldown_bars.min(MAX_ALERT_COOLDOWN_BARS),
            cooldown_remaining_bars: 0,
            trigger_count: 0,
            last_triggered_timestamp: None,
            message: msg,
        });
        Ok(id)
    }

    /// Updates `cooldown_bars` (`0..=MAX_ALERT_COOLDOWN_BARS`) on an existing rule.
    pub fn set_cooldown_bars(&mut self, id: AlertId, cooldown_bars: usize) -> bool {
        let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) else {
            return false;
        };
        rule.cooldown_bars = cooldown_bars.min(MAX_ALERT_COOLDOWN_BARS);
        true
    }

    /// Removes an [`AlertRule`] and purges its historical events from [`Self::triggers`].
    pub fn remove_alert(&mut self, id: AlertId) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        if self.rules.len() != before {
            self.triggers.retain(|ev| ev.alert_id != id);
            true
        } else {
            false
        }
    }

    /// Toggles an [`AlertRule`] between `Paused` and `Armed`.
    pub fn toggle_pause(&mut self, id: AlertId) -> bool {
        let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) else {
            return false;
        };
        rule.status = match rule.status {
            AlertStatus::Paused => AlertStatus::Armed,
            AlertStatus::Armed | AlertStatus::Triggered => AlertStatus::Paused,
        };
        true
    }

    /// Clears all alert rules and trigger history.
    pub fn clear_all(&mut self) {
        self.rules.clear();
        self.triggers.clear();
    }

    /// Deterministically evaluates all non-paused, causally visible [`AlertRule`]s over the causal
    /// prefix `base_candles[..=cursor_base_index]` (`cutoff_timestamp`), rebuilding `triggers`,
    /// `trigger_count`, `last_triggered_timestamp`, and `Armed <-> Triggered` statuses so Bar Replay
    /// stepping AND rewinding remain 100% idempotent and lookahead-free!
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_causal(
        &mut self,
        base_candles: &[Candle],
        base_tf: Timeframe,
        active_candles: &[Candle],
        active_tf: Timeframe,
        cursor_base_index: usize,
        replay_cutoff_timestamp: Option<i64>,
        drawings: &DrawingStore,
    ) {
        self.triggers.clear();
        if base_candles.is_empty() || self.rules.is_empty() {
            for rule in &mut self.rules {
                rule.trigger_count = 0;
                rule.last_triggered_timestamp = None;
                rule.cooldown_remaining_bars = 0;
                if rule.status == AlertStatus::Triggered {
                    rule.status = AlertStatus::Armed;
                }
            }
            return;
        }

        let end_idx = match replay_cutoff_timestamp {
            Some(cutoff) => {
                let p = base_candles.partition_point(|c| c.timestamp <= cutoff);
                if p == 0 {
                    for rule in &mut self.rules {
                        rule.trigger_count = 0;
                        rule.last_triggered_timestamp = None;
                        rule.cooldown_remaining_bars = 0;
                        if rule.status == AlertStatus::Triggered {
                            rule.status = AlertStatus::Armed;
                        }
                    }
                    return;
                }
                (p - 1).min(cursor_base_index).min(base_candles.len() - 1)
            }
            None => cursor_base_index.min(base_candles.len() - 1),
        };
        let base_prefix = &base_candles[..=end_idx];
        let last_base_ts = base_prefix[base_prefix.len() - 1].timestamp;
        let last_base_close_ts =
            last_base_ts.saturating_add(base_tf.seconds().max(1).saturating_sub(1));
        let is_untruncated_live =
            replay_cutoff_timestamp.is_none() && end_idx + 1 == base_candles.len();

        let mut tf_candles_cache: BTreeMap<u32, Vec<Candle>> = BTreeMap::new();
        let mut tf_catalog_cache: BTreeMap<u32, PrecomputedCatalogMatrix> = BTreeMap::new();
        let mut raw_events: Vec<AlertTriggerEvent> = Vec::new();

        let compute_cooldown_rem =
            |rule: &AlertRule, last_fired_bucket: Option<usize>, closed_buckets: usize| -> usize {
                if rule.frequency == AlertFrequency::OncePerBarClose && rule.cooldown_bars > 0 {
                    last_fired_bucket
                        .map_or(0, |prev_t| {
                            (prev_t + rule.cooldown_bars + 1).saturating_sub(closed_buckets)
                        })
                        .min(rule.cooldown_bars)
                } else {
                    0
                }
            };

        let is_suppressed_by_cooldown =
            |rule: &AlertRule, last_fired_bucket: Option<usize>, bucket_idx: usize| -> bool {
                rule.frequency == AlertFrequency::OncePerBarClose
                    && last_fired_bucket.is_some_and(|prev_t| {
                        bucket_idx <= prev_t.saturating_add(rule.cooldown_bars)
                    })
            };

        let evaluate_horizontal_level_on_base =
            |rule: &mut AlertRule,
             level: f64,
             direction: CrossDirection,
             min_visible_ts: Option<i64>,
             summary: &str,
             raw_events: &mut Vec<AlertTriggerEvent>| {
                let start_i =
                    base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                let mut allow_depart_up = true;
                let mut allow_depart_dn = true;
                let mut last_fired_bucket: Option<usize> = None;

                // Track monotonic bucket ordinal on `rule.timeframe` across `base_prefix` for cooldowns.
                let mut cur_bucket_start: Option<i64> = None;
                let mut cur_bucket_idx: usize = 0;
                for bar in &base_prefix[..start_i] {
                    let b_ts = rule.timeframe.bucket_start_timestamp(bar.timestamp);
                    if cur_bucket_start != Some(b_ts) {
                        if cur_bucket_start.is_some() {
                            cur_bucket_idx += 1;
                        }
                        cur_bucket_start = Some(b_ts);
                    }
                }

                for i in start_i..base_prefix.len() {
                    let bar = &base_prefix[i];
                    let b_ts = rule.timeframe.bucket_start_timestamp(bar.timestamp);
                    if cur_bucket_start != Some(b_ts) {
                        if cur_bucket_start.is_some() {
                            cur_bucket_idx += 1;
                        }
                        cur_bucket_start = Some(b_ts);
                    }
                    if min_visible_ts.is_some_and(|cts| bar.timestamp < cts) {
                        continue;
                    }
                    let prev_close = (i > 0).then(|| base_prefix[i - 1].close);
                    if let Some((fired_dir, observed)) = evaluate_4_waypoint_cross_with_depart(
                        prev_close,
                        bar,
                        level,
                        direction,
                        allow_depart_up,
                        allow_depart_dn,
                    ) {
                        match fired_dir {
                            CrossDirection::CrossAbove => {
                                allow_depart_up = bar.close < level;
                                allow_depart_dn = true;
                            }
                            CrossDirection::CrossBelow => {
                                allow_depart_dn = bar.close > level;
                                allow_depart_up = true;
                            }
                            CrossDirection::CrossAny => {}
                        }
                        if is_suppressed_by_cooldown(rule, last_fired_bucket, cur_bucket_idx) {
                            continue;
                        }
                        last_fired_bucket = Some(cur_bucket_idx);
                        rule.trigger_count += 1;
                        rule.last_triggered_timestamp = Some(bar.timestamp);
                        raw_events.push(AlertTriggerEvent {
                            event_id: 0,
                            alert_id: rule.id,
                            symbol: rule.symbol.clone(),
                            timeframe: rule.timeframe,
                            bar_timestamp: bar.timestamp,
                            eval_base_timestamp: Some(bar.timestamp),
                            threshold_value: level,
                            observed_value: observed,
                            direction: fired_dir,
                            condition_summary: summary.to_owned(),
                            message: rule.message.clone(),
                        });
                        if rule.frequency == AlertFrequency::Once {
                            rule.status = AlertStatus::Triggered;
                            break;
                        }
                    } else {
                        if bar.low < level || bar.close < level {
                            allow_depart_up = true;
                        }
                        if bar.high > level || bar.close > level {
                            allow_depart_dn = true;
                        }
                    }
                }

                if let Some(last_b_ts) = cur_bucket_start {
                    let tail_incomplete = rule.timeframe > base_tf
                        && is_bucket_incomplete_at_close_for_symbol(
                            &rule.symbol,
                            last_b_ts,
                            last_base_close_ts,
                            base_tf,
                            rule.timeframe,
                        );
                    let closed_buckets = if tail_incomplete {
                        cur_bucket_idx
                    } else {
                        cur_bucket_idx + 1
                    };
                    rule.cooldown_remaining_bars =
                        compute_cooldown_rem(rule, last_fired_bucket, closed_buckets);
                }
            };

        for rule in &mut self.rules {
            rule.trigger_count = 0;
            rule.last_triggered_timestamp = None;
            rule.cooldown_remaining_bars = 0;
            if rule.status == AlertStatus::Paused {
                continue;
            }
            rule.status = AlertStatus::Armed;
            if !rule.is_causally_visible(replay_cutoff_timestamp)
                || last_base_ts <= rule.armed_at_timestamp
            {
                continue;
            }

            let mut summary = rule.condition.summary();
            truncate_utf8_bytes(&mut summary, MAX_SUMMARY_BYTES);

            match rule.condition {
                AlertCondition::PriceLevel { price, direction } => {
                    evaluate_horizontal_level_on_base(
                        rule,
                        price,
                        direction,
                        None,
                        &summary,
                        &mut raw_events,
                    );
                }
                AlertCondition::RsiThreshold {
                    period,
                    level,
                    direction,
                } => {
                    let series_candles: &[Candle] = if rule.timeframe == base_tf {
                        base_prefix
                    } else if is_untruncated_live
                        && rule.timeframe == active_tf
                        && active_candles.last().is_some_and(|l| {
                            l.timestamp == active_tf.bucket_start_timestamp(last_base_ts)
                        })
                    {
                        active_candles
                    } else if base_tf.can_aggregate_to(rule.timeframe) {
                        let key = rule.timeframe.seconds() as u32;
                        tf_candles_cache.entry(key).or_insert_with(|| {
                            aggregate_ohlcv(base_prefix, base_tf, rule.timeframe)
                                .unwrap_or_else(|_| base_prefix.to_vec())
                        })
                    } else {
                        base_prefix
                    };
                    if series_candles.len() < 2 {
                        continue;
                    }
                    let tail_incomplete = rule.timeframe > base_tf
                        && is_bucket_incomplete_at_close_for_symbol(
                            &rule.symbol,
                            series_candles[series_candles.len() - 1].timestamp,
                            last_base_close_ts,
                            base_tf,
                            rule.timeframe,
                        );
                    let closed_buckets = if tail_incomplete {
                        series_candles.len().saturating_sub(1)
                    } else {
                        series_candles.len()
                    };
                    if closed_buckets < 2 {
                        continue;
                    }
                    let mut closes: Vec<f64> = series_candles[..closed_buckets]
                        .iter()
                        .map(|c| c.close)
                        .collect();
                    let rsi = compute_rsi_slices(&closes, period);

                    let arm_base_len =
                        base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                    let t_ge = series_candles[..closed_buckets]
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    // On `rule.timeframe == base_tf` (e.g. SPY D1 at 14:30 UTC), gate strictly by `armed_at_timestamp`
                    // so the arming bar itself is NEVER retroactively triggered (`BUG-011`).
                    let mut start_t = if rule.timeframe == base_tf {
                        series_candles[..closed_buckets]
                            .partition_point(|c| c.timestamp <= rule.armed_at_timestamp)
                            .max(1)
                    } else {
                        series_candles[..closed_buckets]
                            .partition_point(|c| c.timestamp <= rule.armed_bucket_timestamp)
                            .max(1)
                    };
                    let mut last_fired_bucket: Option<usize> = None;

                    // Intra-bucket causal check (`[CRITICAL-03]`): only on completed HTF buckets (`t_ge < closed_buckets`)
                    // so mid-week unclosed HTF bars never emit ghost triggers (`BUG-011`).
                    if rule.timeframe > base_tf
                        && arm_base_len > 0
                        && t_ge < closed_buckets
                        && series_candles[t_ge].timestamp == rule.armed_bucket_timestamp
                    {
                        let b_end = bucket_end_base_index(
                            base_prefix,
                            rule.timeframe,
                            rule.armed_bucket_timestamp,
                        );
                        if b_end >= arm_base_len
                            && let Some(curr_v) = rsi[t_ge]
                        {
                            let saved_close = closes[t_ge];
                            closes[t_ge] = base_prefix[arm_base_len - 1].close;
                            let rsi_at_arm = compute_rsi_slices(&closes[..=t_ge], period);
                            closes[t_ge] = saved_close;
                            if let Some(arm_v) = rsi_at_arm[t_ge]
                                && let Some(fired_dir) =
                                    match_scalar_cross(arm_v, curr_v, level, direction)
                            {
                                last_fired_bucket = Some(t_ge);
                                let bar_ts = series_candles[t_ge].timestamp;
                                let eval_ts = base_prefix[b_end].timestamp;
                                rule.trigger_count += 1;
                                rule.last_triggered_timestamp = Some(bar_ts);
                                raw_events.push(AlertTriggerEvent {
                                    event_id: 0,
                                    alert_id: rule.id,
                                    symbol: rule.symbol.clone(),
                                    timeframe: rule.timeframe,
                                    bar_timestamp: bar_ts,
                                    eval_base_timestamp: Some(eval_ts),
                                    threshold_value: level,
                                    observed_value: curr_v,
                                    direction: fired_dir,
                                    condition_summary: summary.clone(),
                                    message: rule.message.clone(),
                                });
                                if rule.frequency == AlertFrequency::Once {
                                    rule.status = AlertStatus::Triggered;
                                    continue;
                                }
                            }
                        }
                        start_t = (t_ge + 1).max(1);
                    }

                    for t in start_t..closed_buckets {
                        let bar_ts = series_candles[t].timestamp;
                        let eval_ts = if rule.timeframe == base_tf {
                            bar_ts
                        } else {
                            base_prefix[bucket_end_base_index(base_prefix, rule.timeframe, bar_ts)]
                                .timestamp
                        };
                        if eval_ts <= rule.armed_at_timestamp {
                            continue;
                        }
                        let (Some(prev_v), Some(curr_v)) = (rsi[t - 1], rsi[t]) else {
                            continue;
                        };
                        if let Some(fired_dir) =
                            match_scalar_cross(prev_v, curr_v, level, direction)
                        {
                            if is_suppressed_by_cooldown(rule, last_fired_bucket, t) {
                                continue;
                            }
                            last_fired_bucket = Some(t);
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar_ts);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar_ts,
                                eval_base_timestamp: Some(eval_ts),
                                threshold_value: level,
                                observed_value: curr_v,
                                direction: fired_dir,
                                condition_summary: summary.clone(),
                                message: rule.message.clone(),
                            });
                            if rule.frequency == AlertFrequency::Once {
                                rule.status = AlertStatus::Triggered;
                                break;
                            }
                        }
                    }

                    rule.cooldown_remaining_bars =
                        compute_cooldown_rem(rule, last_fired_bucket, closed_buckets);
                }
                AlertCondition::MacdHistogramZeroCross {
                    fast,
                    slow,
                    signal,
                    direction,
                } => {
                    let series_candles: &[Candle] = if rule.timeframe == base_tf {
                        base_prefix
                    } else if is_untruncated_live
                        && rule.timeframe == active_tf
                        && active_candles.last().is_some_and(|l| {
                            l.timestamp == active_tf.bucket_start_timestamp(last_base_ts)
                        })
                    {
                        active_candles
                    } else if base_tf.can_aggregate_to(rule.timeframe) {
                        let key = rule.timeframe.seconds() as u32;
                        tf_candles_cache.entry(key).or_insert_with(|| {
                            aggregate_ohlcv(base_prefix, base_tf, rule.timeframe)
                                .unwrap_or_else(|_| base_prefix.to_vec())
                        })
                    } else {
                        base_prefix
                    };
                    if series_candles.len() < 2 {
                        continue;
                    }
                    let tail_incomplete = rule.timeframe > base_tf
                        && is_bucket_incomplete_at_close_for_symbol(
                            &rule.symbol,
                            series_candles[series_candles.len() - 1].timestamp,
                            last_base_close_ts,
                            base_tf,
                            rule.timeframe,
                        );
                    let closed_buckets = if tail_incomplete {
                        series_candles.len().saturating_sub(1)
                    } else {
                        series_candles.len()
                    };
                    if closed_buckets < 2 {
                        continue;
                    }
                    let mut closes: Vec<f64> = series_candles[..closed_buckets]
                        .iter()
                        .map(|c| c.close)
                        .collect();
                    let macd = compute_macd_slices(&closes, fast, slow, signal);

                    let arm_base_len =
                        base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                    let t_ge = series_candles[..closed_buckets]
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    let mut start_t = if rule.timeframe == base_tf {
                        series_candles[..closed_buckets]
                            .partition_point(|c| c.timestamp <= rule.armed_at_timestamp)
                            .max(1)
                    } else {
                        series_candles[..closed_buckets]
                            .partition_point(|c| c.timestamp <= rule.armed_bucket_timestamp)
                            .max(1)
                    };
                    let mut last_fired_bucket: Option<usize> = None;

                    if rule.timeframe > base_tf
                        && arm_base_len > 0
                        && t_ge < closed_buckets
                        && series_candles[t_ge].timestamp == rule.armed_bucket_timestamp
                    {
                        let b_end = bucket_end_base_index(
                            base_prefix,
                            rule.timeframe,
                            rule.armed_bucket_timestamp,
                        );
                        if b_end >= arm_base_len
                            && let Some(curr_h) = macd.histogram[t_ge]
                        {
                            let saved_close = closes[t_ge];
                            closes[t_ge] = base_prefix[arm_base_len - 1].close;
                            let macd_at_arm =
                                compute_macd_slices(&closes[..=t_ge], fast, slow, signal);
                            closes[t_ge] = saved_close;
                            if let Some(arm_h) = macd_at_arm.histogram[t_ge]
                                && let Some(fired_dir) =
                                    match_scalar_cross(arm_h, curr_h, 0.0, direction)
                            {
                                last_fired_bucket = Some(t_ge);
                                let bar_ts = series_candles[t_ge].timestamp;
                                let eval_ts = base_prefix[b_end].timestamp;
                                rule.trigger_count += 1;
                                rule.last_triggered_timestamp = Some(bar_ts);
                                raw_events.push(AlertTriggerEvent {
                                    event_id: 0,
                                    alert_id: rule.id,
                                    symbol: rule.symbol.clone(),
                                    timeframe: rule.timeframe,
                                    bar_timestamp: bar_ts,
                                    eval_base_timestamp: Some(eval_ts),
                                    threshold_value: 0.0,
                                    observed_value: curr_h,
                                    direction: fired_dir,
                                    condition_summary: summary.clone(),
                                    message: rule.message.clone(),
                                });
                                if rule.frequency == AlertFrequency::Once {
                                    rule.status = AlertStatus::Triggered;
                                    continue;
                                }
                            }
                        }
                        start_t = (t_ge + 1).max(1);
                    }

                    for (t, bar) in series_candles[..closed_buckets]
                        .iter()
                        .enumerate()
                        .skip(start_t)
                    {
                        let eval_ts = if rule.timeframe == base_tf {
                            bar.timestamp
                        } else {
                            base_prefix
                                [bucket_end_base_index(base_prefix, rule.timeframe, bar.timestamp)]
                            .timestamp
                        };
                        if eval_ts <= rule.armed_at_timestamp {
                            continue;
                        }
                        let (Some(prev_h), Some(curr_h)) =
                            (macd.histogram[t - 1], macd.histogram[t])
                        else {
                            continue;
                        };
                        if let Some(fired_dir) = match_scalar_cross(prev_h, curr_h, 0.0, direction)
                        {
                            if is_suppressed_by_cooldown(rule, last_fired_bucket, t) {
                                continue;
                            }
                            last_fired_bucket = Some(t);
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar.timestamp);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar.timestamp,
                                eval_base_timestamp: Some(eval_ts),
                                threshold_value: 0.0,
                                observed_value: curr_h,
                                direction: fired_dir,
                                condition_summary: summary.clone(),
                                message: rule.message.clone(),
                            });
                            if rule.frequency == AlertFrequency::Once {
                                rule.status = AlertStatus::Triggered;
                                break;
                            }
                        }
                    }

                    rule.cooldown_remaining_bars =
                        compute_cooldown_rem(rule, last_fired_bucket, closed_buckets);
                }
                AlertCondition::DrawingLine {
                    drawing_id,
                    direction,
                } => {
                    let Some(raw_drawing) = drawings.drawings.iter().find(|d| d.id == drawing_id)
                    else {
                        continue;
                    };
                    if !raw_drawing.visible
                        || raw_drawing.pane != DrawingPaneTarget::Main
                        || !raw_drawing.is_causally_visible(
                            replay_cutoff_timestamp,
                            active_candles.len().max(1),
                        )
                    {
                        continue;
                    }

                    // Horizontal lines are invariant to timeframe (`y == anchor.value`), so evaluate
                    // directly on `base_prefix` for bit-exact `1m == 15m` sub-bar parity (`[CRITICAL-03]`)!
                    if let DrawingGeometry::HorizontalLine { anchor } = raw_drawing.geometry {
                        if anchor.value.is_finite()
                            && anchor.value > 0.0
                            && anchor.value <= MAX_ABS_PRICE
                        {
                            evaluate_horizontal_level_on_base(
                                rule,
                                anchor.value,
                                direction,
                                raw_drawing.created_at_timestamp,
                                &summary,
                                &mut raw_events,
                            );
                        }
                        continue;
                    }

                    let series_candles: &[Candle] = if rule.timeframe == base_tf {
                        base_prefix
                    } else if is_untruncated_live
                        && rule.timeframe == active_tf
                        && active_candles.last().is_some_and(|l| {
                            l.timestamp == active_tf.bucket_start_timestamp(last_base_ts)
                        })
                    {
                        active_candles
                    } else if base_tf.can_aggregate_to(rule.timeframe) {
                        let key = rule.timeframe.seconds() as u32;
                        tf_candles_cache.entry(key).or_insert_with(|| {
                            aggregate_ohlcv(base_prefix, base_tf, rule.timeframe)
                                .unwrap_or_else(|_| base_prefix.to_vec())
                        })
                    } else {
                        base_prefix
                    };
                    if series_candles.is_empty() {
                        continue;
                    }

                    let same_slice_as_active = rule.timeframe == active_tf
                        && series_candles.len() == active_candles.len()
                        && series_candles
                            .last()
                            .zip(active_candles.last())
                            .is_some_and(|(a, b)| a.timestamp == b.timestamp);

                    let remapped_store;
                    let eval_store: &DrawingStore =
                        if same_slice_as_active || active_candles.is_empty() {
                            drawings
                        } else {
                            let mut tmp = drawings.clone();
                            tmp.remap_timeframe(
                                active_candles,
                                active_tf,
                                series_candles,
                                rule.timeframe,
                            );
                            remapped_store = tmp;
                            &remapped_store
                        };

                    let Some(target_drawing) =
                        eval_store.drawings.iter().find(|d| d.id == drawing_id)
                    else {
                        continue;
                    };
                    if !target_drawing.visible
                        || target_drawing.pane != DrawingPaneTarget::Main
                        || !target_drawing
                            .is_causally_visible(replay_cutoff_timestamp, series_candles.len())
                    {
                        continue;
                    }

                    let tail_incomplete = rule.timeframe > base_tf
                        && is_bucket_incomplete_at_close_for_symbol(
                            &rule.symbol,
                            series_candles[series_candles.len() - 1].timestamp,
                            last_base_close_ts,
                            base_tf,
                            rule.timeframe,
                        );
                    let closed_buckets = if tail_incomplete {
                        series_candles.len().saturating_sub(1)
                    } else {
                        series_candles.len()
                    };
                    if closed_buckets == 0 {
                        continue;
                    }

                    let arm_base_len =
                        base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                    let t_ge = series_candles[..closed_buckets]
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    let mut start_t = if rule.timeframe == base_tf {
                        series_candles[..closed_buckets]
                            .partition_point(|c| c.timestamp <= rule.armed_at_timestamp)
                    } else {
                        series_candles[..closed_buckets]
                            .partition_point(|c| c.timestamp <= rule.armed_bucket_timestamp)
                    };
                    let mut allow_depart_up = true;
                    let mut allow_depart_dn = true;
                    let mut last_fired_bucket: Option<usize> = None;

                    // Intra-bucket sloped line check on post-arm base ticks of completed buckets (`[CRITICAL-03]`, `BUG-011`)
                    if rule.timeframe > base_tf
                        && arm_base_len > 0
                        && t_ge < closed_buckets
                        && series_candles[t_ge].timestamp == rule.armed_bucket_timestamp
                    {
                        let b_end = bucket_end_base_index(
                            base_prefix,
                            rule.timeframe,
                            rule.armed_bucket_timestamp,
                        );
                        if b_end >= arm_base_len
                            && target_drawing
                                .created_at_timestamp
                                .is_none_or(|cts| cts <= base_prefix[b_end].timestamp)
                            && let Some(line_t) = drawing_line_value_at_bar(target_drawing, t_ge)
                            && let Some(post_arm_bar) =
                                aggregate_candle_slice(&base_prefix[arm_base_len..=b_end])
                        {
                            let prev_off = Some(base_prefix[arm_base_len - 1].close - line_t);
                            if let Some((fired_dir, observed)) =
                                evaluate_relative_offset_zero_cross_with_depart(
                                    prev_off,
                                    &post_arm_bar,
                                    line_t,
                                    direction,
                                    allow_depart_up,
                                    allow_depart_dn,
                                )
                            {
                                match fired_dir {
                                    CrossDirection::CrossAbove => {
                                        allow_depart_up = post_arm_bar.close < line_t;
                                        allow_depart_dn = true;
                                    }
                                    CrossDirection::CrossBelow => {
                                        allow_depart_dn = post_arm_bar.close > line_t;
                                        allow_depart_up = true;
                                    }
                                    CrossDirection::CrossAny => {}
                                }
                                last_fired_bucket = Some(t_ge);
                                let bar_ts = series_candles[t_ge].timestamp;
                                let eval_ts = base_prefix[b_end].timestamp;
                                rule.trigger_count += 1;
                                rule.last_triggered_timestamp = Some(bar_ts);
                                raw_events.push(AlertTriggerEvent {
                                    event_id: 0,
                                    alert_id: rule.id,
                                    symbol: rule.symbol.clone(),
                                    timeframe: rule.timeframe,
                                    bar_timestamp: bar_ts,
                                    eval_base_timestamp: Some(eval_ts),
                                    threshold_value: line_t,
                                    observed_value: observed,
                                    direction: fired_dir,
                                    condition_summary: summary.clone(),
                                    message: rule.message.clone(),
                                });
                                if rule.frequency == AlertFrequency::Once {
                                    rule.status = AlertStatus::Triggered;
                                    continue;
                                }
                            } else {
                                if post_arm_bar.low < line_t || post_arm_bar.close < line_t {
                                    allow_depart_up = true;
                                }
                                if post_arm_bar.high > line_t || post_arm_bar.close > line_t {
                                    allow_depart_dn = true;
                                }
                            }
                        }
                        start_t = t_ge + 1;
                    }

                    for t in start_t..closed_buckets {
                        let bar_ts = series_candles[t].timestamp;
                        let eval_ts = if rule.timeframe == base_tf {
                            bar_ts
                        } else {
                            base_prefix[bucket_end_base_index(base_prefix, rule.timeframe, bar_ts)]
                                .timestamp
                        };
                        if eval_ts <= rule.armed_at_timestamp {
                            continue;
                        }
                        if !target_drawing
                            .created_at_timestamp
                            .is_none_or(|cts| cts <= eval_ts)
                        {
                            continue;
                        }
                        let Some(line_t) = drawing_line_value_at_bar(target_drawing, t) else {
                            continue;
                        };
                        let prev_offset = if t > 0 {
                            drawing_line_prev_reference(target_drawing, t - 1)
                                .map(|line_prev| series_candles[t - 1].close - line_prev)
                        } else {
                            None
                        };
                        let bar = &series_candles[t];
                        if let Some((fired_dir, observed)) =
                            evaluate_relative_offset_zero_cross_with_depart(
                                prev_offset,
                                bar,
                                line_t,
                                direction,
                                allow_depart_up,
                                allow_depart_dn,
                            )
                        {
                            match fired_dir {
                                CrossDirection::CrossAbove => {
                                    allow_depart_up = bar.close < line_t;
                                    allow_depart_dn = true;
                                }
                                CrossDirection::CrossBelow => {
                                    allow_depart_dn = bar.close > line_t;
                                    allow_depart_up = true;
                                }
                                CrossDirection::CrossAny => {}
                            }
                            if is_suppressed_by_cooldown(rule, last_fired_bucket, t) {
                                continue;
                            }
                            last_fired_bucket = Some(t);
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar_ts);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar.timestamp,
                                eval_base_timestamp: Some(eval_ts),
                                threshold_value: line_t,
                                observed_value: observed,
                                direction: fired_dir,
                                condition_summary: summary.clone(),
                                message: rule.message.clone(),
                            });
                            if rule.frequency == AlertFrequency::Once {
                                rule.status = AlertStatus::Triggered;
                                break;
                            }
                        } else {
                            if bar.low < line_t || bar.close < line_t {
                                allow_depart_up = true;
                            }
                            if bar.high > line_t || bar.close > line_t {
                                allow_depart_dn = true;
                            }
                        }
                    }

                    rule.cooldown_remaining_bars =
                        compute_cooldown_rem(rule, last_fired_bucket, closed_buckets);
                }
                AlertCondition::SignalCatalogTrigger {
                    ref catalog_id,
                    float_level,
                    direction,
                } => {
                    let Some(cat_idx) = catalog_entry_index(catalog_id.trim()) else {
                        continue;
                    };
                    let entry = &SIGNAL_CATALOG[cat_idx];
                    let series_candles: &[Candle] = if rule.timeframe == base_tf {
                        base_prefix
                    } else if is_untruncated_live
                        && rule.timeframe == active_tf
                        && active_candles.last().is_some_and(|l| {
                            l.timestamp == active_tf.bucket_start_timestamp(last_base_ts)
                        })
                    {
                        active_candles
                    } else if base_tf.can_aggregate_to(rule.timeframe) {
                        let key = rule.timeframe.seconds() as u32;
                        tf_candles_cache.entry(key).or_insert_with(|| {
                            aggregate_ohlcv(base_prefix, base_tf, rule.timeframe)
                                .unwrap_or_else(|_| base_prefix.to_vec())
                        })
                    } else {
                        base_prefix
                    };
                    if series_candles.is_empty() {
                        continue;
                    }
                    let tail_incomplete = rule.timeframe > base_tf
                        && is_bucket_incomplete_at_close_for_symbol(
                            &rule.symbol,
                            series_candles[series_candles.len() - 1].timestamp,
                            last_base_close_ts,
                            base_tf,
                            rule.timeframe,
                        );
                    let closed_len = if tail_incomplete {
                        series_candles.len().saturating_sub(1)
                    } else {
                        series_candles.len()
                    };
                    if closed_len == 0 {
                        continue;
                    }
                    let tf_key =
                        if rule.timeframe == base_tf || !base_tf.can_aggregate_to(rule.timeframe) {
                            base_tf.seconds() as u32
                        } else {
                            rule.timeframe.seconds() as u32
                        };
                    let matrix = tf_catalog_cache.entry(tf_key).or_insert_with(|| {
                        PrecomputedCatalogMatrix::compute(&series_candles[..closed_len])
                    });
                    let matrix = if matrix.bar_count == closed_len {
                        matrix as &PrecomputedCatalogMatrix
                    } else {
                        *matrix = PrecomputedCatalogMatrix::compute(&series_candles[..closed_len]);
                        matrix as &PrecomputedCatalogMatrix
                    };

                    let start_t = series_candles[..closed_len]
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    let mut last_fired_bucket: Option<usize> = None;

                    for (t, bar_candle) in series_candles
                        .iter()
                        .enumerate()
                        .take(closed_len)
                        .skip(start_t)
                    {
                        let bar_ts = bar_candle.timestamp;
                        let eval_ts = if rule.timeframe == base_tf {
                            bar_ts
                        } else {
                            base_prefix[bucket_end_base_index(base_prefix, rule.timeframe, bar_ts)]
                                .timestamp
                        };
                        if eval_ts <= rule.armed_at_timestamp {
                            continue;
                        }
                        let Some(pt) = matrix.point_at(cat_idx, t) else {
                            continue;
                        };
                        if !pt.quality.is_valid() {
                            continue;
                        }

                        let matched: Option<(CrossDirection, f64, f64)> = match entry.output_type {
                            SignalOutputType::Bool => {
                                if pt.bool_value != Some(true) {
                                    None
                                } else {
                                    let prev_same_active = t > 0
                                        && matrix.point_at(cat_idx, t - 1).is_some_and(|p| {
                                            p.quality.is_valid()
                                                && p.bool_value == Some(true)
                                                && p.direction == pt.direction
                                        });
                                    if prev_same_active {
                                        None
                                    } else {
                                        let nat_dir = match pt.direction {
                                            Some(SignalDirection::Bearish) => {
                                                CrossDirection::CrossBelow
                                            }
                                            _ => CrossDirection::CrossAbove,
                                        };
                                        if direction == CrossDirection::CrossAny
                                            || direction == nat_dir
                                        {
                                            let obs = pt
                                                .float_value
                                                .filter(|v| v.is_finite())
                                                .unwrap_or(1.0);
                                            Some((nat_dir, 1.0, obs))
                                        } else {
                                            None
                                        }
                                    }
                                }
                            }
                            SignalOutputType::Float => {
                                let lvl = float_level.unwrap_or(0.0);
                                if t == 0 || !lvl.is_finite() {
                                    None
                                } else if let (Some(prev_pt), Some(curr_v)) =
                                    (matrix.point_at(cat_idx, t - 1), pt.float_value)
                                    && prev_pt.quality.is_valid()
                                    && let Some(prev_v) = prev_pt.float_value
                                    && prev_v.is_finite()
                                    && curr_v.is_finite()
                                {
                                    match_scalar_cross(prev_v, curr_v, lvl, direction)
                                        .map(|fd| (fd, lvl, curr_v))
                                } else {
                                    None
                                }
                            }
                            SignalOutputType::Enum => {
                                if t == 0 {
                                    None
                                } else if let Some(prev_pt) = matrix.point_at(cat_idx, t - 1)
                                    && prev_pt.quality.is_valid()
                                {
                                    let prev_reg = matrix.regime_at(t - 1);
                                    let curr_reg = matrix.regime_at(t);
                                    if curr_reg == prev_reg
                                        || curr_reg == MarketRegime::InsufficientHistory
                                    {
                                        None
                                    } else {
                                        let obs = f64::from(curr_reg.bit());
                                        match curr_reg {
                                            MarketRegime::Uptrend | MarketRegime::Recovery => {
                                                matches!(
                                                    direction,
                                                    CrossDirection::CrossAbove
                                                        | CrossDirection::CrossAny
                                                )
                                                .then_some((CrossDirection::CrossAbove, 0.0, obs))
                                            }
                                            MarketRegime::Downtrend => matches!(
                                                direction,
                                                CrossDirection::CrossBelow
                                                    | CrossDirection::CrossAny
                                            )
                                            .then_some((CrossDirection::CrossBelow, 0.0, obs)),
                                            MarketRegime::Pullback => (direction
                                                == CrossDirection::CrossAny)
                                                .then_some((CrossDirection::CrossAbove, 0.0, obs)),
                                            MarketRegime::SidewaysSqueeze
                                            | MarketRegime::Neutral
                                            | MarketRegime::InsufficientHistory => (direction
                                                == CrossDirection::CrossAny)
                                                .then_some((CrossDirection::CrossBelow, 0.0, obs)),
                                        }
                                    }
                                } else {
                                    None
                                }
                            }
                        };

                        if let Some((fired_dir, thr_val, obs_val)) = matched {
                            if is_suppressed_by_cooldown(rule, last_fired_bucket, t) {
                                continue;
                            }
                            last_fired_bucket = Some(t);
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar_ts);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar_ts,
                                eval_base_timestamp: Some(eval_ts),
                                threshold_value: thr_val,
                                observed_value: obs_val,
                                direction: fired_dir,
                                condition_summary: summary.clone(),
                                message: rule.message.clone(),
                            });
                            if rule.frequency == AlertFrequency::Once {
                                rule.status = AlertStatus::Triggered;
                                break;
                            }
                        }
                    }

                    rule.cooldown_remaining_bars =
                        compute_cooldown_rem(rule, last_fired_bucket, closed_len);
                }
                AlertCondition::ConfluenceThreshold {
                    profile,
                    threshold,
                    direction,
                } => {
                    let series_candles: &[Candle] = if rule.timeframe == base_tf {
                        base_prefix
                    } else if is_untruncated_live
                        && rule.timeframe == active_tf
                        && active_candles.last().is_some_and(|l| {
                            l.timestamp == active_tf.bucket_start_timestamp(last_base_ts)
                        })
                    {
                        active_candles
                    } else if base_tf.can_aggregate_to(rule.timeframe) {
                        let key = rule.timeframe.seconds() as u32;
                        tf_candles_cache.entry(key).or_insert_with(|| {
                            aggregate_ohlcv(base_prefix, base_tf, rule.timeframe)
                                .unwrap_or_else(|_| base_prefix.to_vec())
                        })
                    } else {
                        base_prefix
                    };
                    if series_candles.len() < 2 {
                        continue;
                    }
                    let tail_incomplete = rule.timeframe > base_tf
                        && is_bucket_incomplete_at_close_for_symbol(
                            &rule.symbol,
                            series_candles[series_candles.len() - 1].timestamp,
                            last_base_close_ts,
                            base_tf,
                            rule.timeframe,
                        );
                    let closed_len = if tail_incomplete {
                        series_candles.len().saturating_sub(1)
                    } else {
                        series_candles.len()
                    };
                    if closed_len < 2 {
                        continue;
                    }
                    let tf_key =
                        if rule.timeframe == base_tf || !base_tf.can_aggregate_to(rule.timeframe) {
                            base_tf.seconds() as u32
                        } else {
                            rule.timeframe.seconds() as u32
                        };
                    let matrix = tf_catalog_cache.entry(tf_key).or_insert_with(|| {
                        PrecomputedCatalogMatrix::compute(&series_candles[..closed_len])
                    });
                    let matrix = if matrix.bar_count == closed_len {
                        matrix as &PrecomputedCatalogMatrix
                    } else {
                        *matrix = PrecomputedCatalogMatrix::compute(&series_candles[..closed_len]);
                        matrix as &PrecomputedCatalogMatrix
                    };

                    let start_t = series_candles[..closed_len]
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp)
                        .max(1);
                    let mut last_fired_bucket: Option<usize> = None;

                    for (t, bar_candle) in series_candles
                        .iter()
                        .enumerate()
                        .take(closed_len)
                        .skip(start_t)
                    {
                        let bar_ts = bar_candle.timestamp;
                        let eval_ts = if rule.timeframe == base_tf {
                            bar_ts
                        } else {
                            base_prefix[bucket_end_base_index(base_prefix, rule.timeframe, bar_ts)]
                                .timestamp
                        };
                        if eval_ts <= rule.armed_at_timestamp {
                            continue;
                        }
                        let (Ok(prev_s), Ok(curr_s)) = (
                            matrix.health_score_at(t - 1, profile),
                            matrix.health_score_at(t, profile),
                        ) else {
                            continue;
                        };
                        if !prev_s.is_finite() || !curr_s.is_finite() {
                            continue;
                        }
                        if let Some(fired_dir) =
                            match_scalar_cross(prev_s, curr_s, threshold, direction)
                        {
                            if is_suppressed_by_cooldown(rule, last_fired_bucket, t) {
                                continue;
                            }
                            last_fired_bucket = Some(t);
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar_ts);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar_ts,
                                eval_base_timestamp: Some(eval_ts),
                                threshold_value: threshold,
                                observed_value: curr_s,
                                direction: fired_dir,
                                condition_summary: summary.clone(),
                                message: rule.message.clone(),
                            });
                            if rule.frequency == AlertFrequency::Once {
                                rule.status = AlertStatus::Triggered;
                                break;
                            }
                        }
                    }

                    rule.cooldown_remaining_bars =
                        compute_cooldown_rem(rule, last_fired_bucket, closed_len);
                }
            }
        }

        // Order events chronologically (`eval_base_timestamp` asc, `bar_timestamp` asc, `alert_id` asc)
        // and assign deterministic `event_id`.
        raw_events.sort_by(|a, b| {
            a.effective_eval_base_timestamp()
                .cmp(&b.effective_eval_base_timestamp())
                .then_with(|| a.bar_timestamp.cmp(&b.bar_timestamp))
                .then_with(|| a.alert_id.cmp(&b.alert_id))
        });
        if raw_events.len() > MAX_TRIGGER_LOG_EVENTS {
            let drop_n = raw_events.len() - MAX_TRIGGER_LOG_EVENTS;
            raw_events.drain(0..drop_n);
        }
        for (idx, ev) in raw_events.iter_mut().enumerate() {
            ev.event_id = (idx as u64) + 1;
        }
        self.triggers = raw_events;
    }

    /// Returns all causally visible [`AlertTriggerEvent`] items up to `replay_cutoff_timestamp` (`DEF-001`).
    #[must_use]
    pub fn visible_triggers(
        &self,
        replay_cutoff_timestamp: Option<i64>,
    ) -> Vec<&AlertTriggerEvent> {
        self.triggers
            .iter()
            .filter(|ev| {
                let ets = ev.effective_eval_base_timestamp();
                if replay_cutoff_timestamp.is_some_and(|cut| ets > cut) {
                    return false;
                }
                self.rules
                    .iter()
                    .find(|r| r.id == ev.alert_id)
                    .is_none_or(|r| r.is_causally_visible(replay_cutoff_timestamp))
            })
            .collect()
    }

    /// Exports causally visible trigger events to RFC-4180 CSV with spreadsheet formula-injection
    /// protection (`sanitize_csv_cell` neutralizing leading `=`, `+`, `-`, `@`, `|`, `\t`, `\r`, `\n`).
    #[must_use]
    pub fn export_triggers_csv(&self, replay_cutoff_timestamp: Option<i64>) -> String {
        let visible = self.visible_triggers(replay_cutoff_timestamp);
        let mut csv = String::with_capacity(140 + visible.len() * 120);
        csv.push_str(
            "event_id,alert_id,symbol,timeframe,bar_timestamp,eval_base_timestamp,direction,threshold_value,observed_value,condition_summary,message\n",
        );
        for ev in visible {
            let ets = ev.effective_eval_base_timestamp();
            let dir_code = match ev.direction {
                CrossDirection::CrossAbove => "CROSS_ABOVE",
                CrossDirection::CrossBelow => "CROSS_BELOW",
                CrossDirection::CrossAny => "CROSS_ANY",
            };
            let thr = if ev.threshold_value.is_finite() {
                format!("{:.6}", ev.threshold_value)
            } else {
                "0.000000".to_owned()
            };
            let obs = if ev.observed_value.is_finite() {
                format!("{:.6}", ev.observed_value)
            } else {
                "0.000000".to_owned()
            };
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{}",
                ev.event_id,
                ev.alert_id,
                sanitize_csv_cell(&ev.symbol),
                ev.timeframe.label(),
                ev.bar_timestamp,
                ets,
                dir_code,
                thr,
                obs,
                sanitize_csv_cell(&ev.condition_summary),
                sanitize_csv_cell(&ev.message),
            );
        }
        csv
    }

    /// Exports causally visible trigger events to deterministic RFC-8259 JSON (`0` wall-clock fields,
    /// `0` external crates, full `\u00XX` control escaping).
    #[must_use]
    pub fn export_triggers_json(&self, replay_cutoff_timestamp: Option<i64>) -> String {
        let visible = self.visible_triggers(replay_cutoff_timestamp);
        let mut out = String::with_capacity(128 + visible.len() * 220);
        out.push_str("{\n  \"schema_version\": 1,\n  \"replay_cutoff_timestamp\": ");
        match replay_cutoff_timestamp {
            Some(cut) => {
                let _ = write!(out, "{cut}");
            }
            None => out.push_str("null"),
        }
        let _ = writeln!(
            out,
            ",\n  \"event_count\": {},\n  \"events\": [",
            visible.len()
        );
        for (idx, ev) in visible.iter().enumerate() {
            let ets = ev.effective_eval_base_timestamp();
            let dir_code = match ev.direction {
                CrossDirection::CrossAbove => "CROSS_ABOVE",
                CrossDirection::CrossBelow => "CROSS_BELOW",
                CrossDirection::CrossAny => "CROSS_ANY",
            };
            let thr = if ev.threshold_value.is_finite() {
                ev.threshold_value
            } else {
                0.0
            };
            let obs = if ev.observed_value.is_finite() {
                ev.observed_value
            } else {
                0.0
            };
            out.push_str("    {\n");
            let _ = writeln!(out, "      \"event_id\": {},", ev.event_id);
            let _ = writeln!(out, "      \"alert_id\": {},", ev.alert_id);
            out.push_str("      \"symbol\": ");
            push_json_escaped_str(&mut out, &ev.symbol);
            out.push_str(",\n      \"timeframe\": ");
            push_json_escaped_str(&mut out, ev.timeframe.label());
            let _ = writeln!(out, ",\n      \"bar_timestamp\": {},", ev.bar_timestamp);
            let _ = writeln!(out, "      \"eval_base_timestamp\": {ets},");
            out.push_str("      \"direction\": ");
            push_json_escaped_str(&mut out, dir_code);
            let _ = writeln!(out, ",\n      \"threshold_value\": {thr:.6},");
            let _ = writeln!(out, "      \"observed_value\": {obs:.6},");
            out.push_str("      \"condition_summary\": ");
            push_json_escaped_str(&mut out, &ev.condition_summary);
            out.push_str(",\n      \"message\": ");
            push_json_escaped_str(&mut out, &ev.message);
            if idx + 1 == visible.len() {
                out.push_str("\n    }\n");
            } else {
                out.push_str("\n    },\n");
            }
        }
        out.push_str("  ]\n}\n");
        out
    }
}

fn push_json_escaped_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
