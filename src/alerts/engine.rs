use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::data::{MAX_ABS_PRICE, Timeframe, aggregate_ohlcv};
use crate::drawings::{
    Drawing, DrawingGeometry, DrawingPaneTarget, DrawingStore, MAX_ABS_ANCHOR_TIMESTAMP,
};
use crate::indicators::{compute_macd_slices, compute_rsi_slices};
use crate::model::Candle;

use super::{
    AlertCondition, AlertError, AlertFrequency, AlertId, AlertRule, AlertStatus, AlertTriggerEvent,
    CrossDirection, truncate_utf8_bytes,
};

/// Upper bound on persisted alert rules per instrument.
pub const MAX_ALERT_RULES: usize = 128;

/// Upper bound on retained causal trigger events per instrument.
pub const MAX_TRIGGER_LOG_EVENTS: usize = 256;

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
            let u = (x - start.bar_index) / dx;
            if !(-1e-9..=1.0 + 1e-9).contains(&u) {
                return None;
            }
            let val = start.value + u.clamp(0.0, 1.0) * (end.value - start.value);
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
        DrawingGeometry::Rectangle { .. }
        | DrawingGeometry::FibonacciRetracement { .. }
        | DrawingGeometry::RiskRewardBox { .. } => None,
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
                    DrawingGeometry::Rectangle { .. }
                    | DrawingGeometry::FibonacciRetracement { .. }
                    | DrawingGeometry::RiskRewardBox { .. } => {
                        Err(AlertError::UnsupportedDrawingGeometry)
                    }
                }
            } else {
                Ok(())
            }
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
            if ev.eval_base_timestamp == 0 {
                ev.eval_base_timestamp = ev.bar_timestamp;
            }
            truncate_utf8_bytes(&mut ev.symbol, MAX_SYMBOL_BYTES);
            truncate_utf8_bytes(&mut ev.condition_summary, MAX_SUMMARY_BYTES);
            truncate_utf8_bytes(&mut ev.message, MAX_MESSAGE_BYTES);
        }
        self.triggers.sort_by(|a, b| {
            a.eval_base_timestamp
                .cmp(&b.eval_base_timestamp)
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
        if base_candles.is_empty() {
            return Err(AlertError::EmptySeries);
        }
        validate_condition(&condition, drawings)?;

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
            created_at_timestamp: replay_cutoff_timestamp,
            trigger_count: 0,
            last_triggered_timestamp: None,
            message: msg,
        });
        Ok(id)
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
        let is_untruncated_live =
            replay_cutoff_timestamp.is_none() && end_idx + 1 == base_candles.len();

        let mut tf_candles_cache: BTreeMap<u32, Vec<Candle>> = BTreeMap::new();
        let mut raw_events: Vec<AlertTriggerEvent> = Vec::new();

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
                for i in start_i..base_prefix.len() {
                    let bar = &base_prefix[i];
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
                        rule.trigger_count += 1;
                        rule.last_triggered_timestamp = Some(bar.timestamp);
                        raw_events.push(AlertTriggerEvent {
                            event_id: 0,
                            alert_id: rule.id,
                            symbol: rule.symbol.clone(),
                            timeframe: rule.timeframe,
                            bar_timestamp: bar.timestamp,
                            eval_base_timestamp: bar.timestamp,
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
            };

        for rule in &mut self.rules {
            rule.trigger_count = 0;
            rule.last_triggered_timestamp = None;
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
                    let mut closes: Vec<f64> = series_candles.iter().map(|c| c.close).collect();
                    let rsi = compute_rsi_slices(&closes, period);

                    let arm_base_len =
                        base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                    let t_ge = series_candles
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    let mut start_t = series_candles
                        .partition_point(|c| c.timestamp <= rule.armed_bucket_timestamp)
                        .max(1);

                    // Intra-bucket causal check (`[CRITICAL-03]`): if `rule` was armed mid-bucket on a higher
                    // timeframe and subsequent base ticks arrived inside `armed_bucket_timestamp`, compare the
                    // current bucket's RSI against the exact RSI at the moment of arming!
                    if rule.timeframe > base_tf
                        && arm_base_len > 0
                        && t_ge < series_candles.len()
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
                                    eval_base_timestamp: eval_ts,
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

                    for t in start_t..series_candles.len() {
                        let (Some(prev_v), Some(curr_v)) = (rsi[t - 1], rsi[t]) else {
                            continue;
                        };
                        if let Some(fired_dir) =
                            match_scalar_cross(prev_v, curr_v, level, direction)
                        {
                            let bar_ts = series_candles[t].timestamp;
                            let eval_ts = if rule.timeframe == base_tf {
                                bar_ts
                            } else {
                                base_prefix
                                    [bucket_end_base_index(base_prefix, rule.timeframe, bar_ts)]
                                .timestamp
                            };
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar_ts);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar_ts,
                                eval_base_timestamp: eval_ts,
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
                    let mut closes: Vec<f64> = series_candles.iter().map(|c| c.close).collect();
                    let macd = compute_macd_slices(&closes, fast, slow, signal);

                    let arm_base_len =
                        base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                    let t_ge = series_candles
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    let mut start_t = series_candles
                        .partition_point(|c| c.timestamp <= rule.armed_bucket_timestamp)
                        .max(1);

                    if rule.timeframe > base_tf
                        && arm_base_len > 0
                        && t_ge < series_candles.len()
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
                                    eval_base_timestamp: eval_ts,
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

                    for (t, bar) in series_candles.iter().enumerate().skip(start_t) {
                        let (Some(prev_h), Some(curr_h)) =
                            (macd.histogram[t - 1], macd.histogram[t])
                        else {
                            continue;
                        };
                        if let Some(fired_dir) = match_scalar_cross(prev_h, curr_h, 0.0, direction)
                        {
                            let eval_ts = if rule.timeframe == base_tf {
                                bar.timestamp
                            } else {
                                base_prefix[bucket_end_base_index(
                                    base_prefix,
                                    rule.timeframe,
                                    bar.timestamp,
                                )]
                                .timestamp
                            };
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar.timestamp);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar.timestamp,
                                eval_base_timestamp: eval_ts,
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

                    let arm_base_len =
                        base_prefix.partition_point(|c| c.timestamp <= rule.armed_at_timestamp);
                    let t_ge = series_candles
                        .partition_point(|c| c.timestamp < rule.armed_bucket_timestamp);
                    let mut start_t = series_candles
                        .partition_point(|c| c.timestamp <= rule.armed_bucket_timestamp);
                    let mut allow_depart_up = true;
                    let mut allow_depart_dn = true;

                    // Intra-bucket sloped line check on post-arm base ticks (`[CRITICAL-03]`)
                    if rule.timeframe > base_tf
                        && arm_base_len > 0
                        && t_ge < series_candles.len()
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
                                    eval_base_timestamp: eval_ts,
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

                    for t in start_t..series_candles.len() {
                        let bar_ts = series_candles[t].timestamp;
                        let eval_ts = if rule.timeframe == base_tf {
                            bar_ts
                        } else {
                            base_prefix[bucket_end_base_index(base_prefix, rule.timeframe, bar_ts)]
                                .timestamp
                        };
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
                            rule.trigger_count += 1;
                            rule.last_triggered_timestamp = Some(bar_ts);
                            raw_events.push(AlertTriggerEvent {
                                event_id: 0,
                                alert_id: rule.id,
                                symbol: rule.symbol.clone(),
                                timeframe: rule.timeframe,
                                bar_timestamp: bar_ts,
                                eval_base_timestamp: eval_ts,
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
                }
            }
        }

        // Order events chronologically (`eval_base_timestamp` asc, `bar_timestamp` asc, `alert_id` asc)
        // and assign deterministic `event_id`.
        raw_events.sort_by(|a, b| {
            a.eval_base_timestamp
                .cmp(&b.eval_base_timestamp)
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
}
