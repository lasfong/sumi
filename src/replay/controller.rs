use serde::{Deserialize, Serialize};

use crate::data::{AggregationError, Timeframe, aggregate_ohlcv};
use crate::model::Candle;

/// Upper bound on automatic playback steps processed in a single UI frame so tab/window stalls
/// never jump dozens of bars instantaneously.
pub const MAX_STEPS_PER_FRAME: usize = 8;

/// Upper bound (`seconds`) on per-frame `dt` fed into the playback accumulator.
pub const MAX_PLAYBACK_DT_SECS: f64 = 0.25;

/// Deterministic auto-play speed presets (`steps / second`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReplaySpeed {
    #[default]
    X1,
    X2,
    X5,
    X10,
    X20,
}

impl ReplaySpeed {
    pub const ALL: [Self; 5] = [Self::X1, Self::X2, Self::X5, Self::X10, Self::X20];

    #[inline]
    #[must_use]
    pub const fn steps_per_second(self) -> f64 {
        match self {
            Self::X1 => 1.0,
            Self::X2 => 2.0,
            Self::X5 => 5.0,
            Self::X10 => 10.0,
            Self::X20 => 20.0,
        }
    }

    #[inline]
    #[must_use]
    pub fn interval_secs(self) -> f64 {
        1.0 / self.steps_per_second()
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::X1 => "1x",
            Self::X2 => "2x",
            Self::X5 => "5x",
            Self::X10 => "10x",
            Self::X20 => "20x",
        }
    }

    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::X1 => Self::X2,
            Self::X2 => Self::X5,
            Self::X5 => Self::X10,
            Self::X10 => Self::X20,
            Self::X20 => Self::X1,
        }
    }
}

/// Granularity of a single `+1` / `-1` or auto-play step when `active_timeframe > base_timeframe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReplayStepUnit {
    /// Advances by one full active-timeframe bar (completing the current partial bucket or stepping
    /// to the close of the next active bucket).
    #[default]
    ActiveBar,
    /// Advances by a single `base_timeframe` candle (`1m` / `1D` sub-bar tick) so higher-timeframe
    /// bars (`15m`, `4h`, `1D`, `1W`) form dynamically tick-by-tick without future leakage.
    BaseBar,
}

impl ReplayStepUnit {
    pub const ALL: [Self; 2] = [Self::ActiveBar, Self::BaseBar];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::ActiveBar => "Bar",
            Self::BaseBar => "Tick",
        }
    }

    #[inline]
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::ActiveBar => Self::BaseBar,
            Self::BaseBar => Self::ActiveBar,
        }
    }
}

/// Lifecycle state of the causal Bar Replay session.
///
/// Note that interactive cut-bar selection (`selecting_cut_bar: bool`) is kept strictly orthogonal
/// to [`ReplayStatus`] so activating `Jump to Bar` while already paused in Replay **never** drops
/// [`ReplayStatus::is_active`] or exposes future bars (`DEF-001`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReplayStatus {
    #[default]
    Inactive,
    Paused,
    Playing,
    Completed,
}

impl ReplayStatus {
    #[inline]
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Paused | Self::Playing | Self::Completed)
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Inactive => "LIVE",
            Self::Paused => "PAUSED",
            Self::Playing => "PLAYING",
            Self::Completed => "END",
        }
    }
}

/// Exact base-series cursor delta emitted by a forward, backward, or jump transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayTransition {
    pub prev_base_index: usize,
    pub new_base_index: usize,
    pub cutoff_timestamp: i64,
}

/// Deterministic, zero-lookahead multi-timeframe Bar Replay state machine (`Milestone R07`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReplayController {
    pub status: ReplayStatus,
    pub selecting_cut_bar: bool,
    pub start_base_index: usize,
    pub base_cursor_index: usize,
    pub speed: ReplaySpeed,
    pub step_unit: ReplayStepUnit,

    #[serde(skip)]
    playback_accumulator_secs: f64,
}

impl ReplayController {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            status: ReplayStatus::Inactive,
            selecting_cut_bar: false,
            start_base_index: 0,
            base_cursor_index: 0,
            speed: ReplaySpeed::X1,
            step_unit: ReplayStepUnit::ActiveBar,
            playback_accumulator_secs: 0.0,
        }
    }

    /// Sanitizes deserialized state against `base_candles`, pausing any restored `Playing` state so
    /// reopened workspaces never advance bars unexpectedly on startup.
    pub fn sanitize_deserialized(&mut self, base_candles: &[Candle]) {
        self.selecting_cut_bar = false;
        self.playback_accumulator_secs = 0.0;
        if base_candles.is_empty() || !self.status.is_active() {
            self.status = ReplayStatus::Inactive;
            self.start_base_index = 0;
            self.base_cursor_index = 0;
            return;
        }
        let last = base_candles.len() - 1;
        self.base_cursor_index = self.base_cursor_index.min(last);
        self.start_base_index = self.start_base_index.min(self.base_cursor_index);
        self.status = if self.base_cursor_index >= last {
            ReplayStatus::Completed
        } else {
            ReplayStatus::Paused
        };
    }

    #[inline]
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    #[inline]
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.status == ReplayStatus::Playing
    }

    #[inline]
    #[must_use]
    pub const fn playback_accumulator_secs(&self) -> f64 {
        self.playback_accumulator_secs
    }

    /// Returns the active causal cutoff timestamp (`UTC` seconds) when Replay is active.
    #[must_use]
    pub fn cutoff_timestamp(&self, base_candles: &[Candle]) -> Option<i64> {
        if !self.is_active() || base_candles.is_empty() {
            return None;
        }
        let idx = self.base_cursor_index.min(base_candles.len() - 1);
        Some(base_candles[idx].timestamp)
    }

    /// Enters interactive on-chart `Jump to Bar` scrubber selection, pausing playback if active
    /// while strictly preserving any active causal cutoff (`DEF-001`).
    pub fn enter_cut_selection(&mut self) {
        if self.status == ReplayStatus::Playing {
            self.status = ReplayStatus::Paused;
        }
        self.playback_accumulator_secs = 0.0;
        self.selecting_cut_bar = true;
    }

    /// Cancels `Jump to Bar` scrubber selection without altering the replay cursor.
    pub fn cancel_cut_selection(&mut self) {
        self.selecting_cut_bar = false;
    }

    /// Resolves a clicked `active_bar_index` on `(active_candles, active_tf)` to the causal closing
    /// `base_candles` index of that bucket, clamped to `max_allowed_base_index` when already in Replay.
    #[must_use]
    pub fn resolve_cut_base_index(
        base_candles: &[Candle],
        active_candles: &[Candle],
        active_tf: Timeframe,
        active_bar_index: usize,
        max_allowed_base_index: Option<usize>,
    ) -> Option<usize> {
        if base_candles.is_empty() || active_candles.is_empty() {
            return None;
        }
        let clamped_active = active_bar_index.min(active_candles.len() - 1);
        let target_bucket_ts =
            active_tf.bucket_start_timestamp(active_candles[clamped_active].timestamp);
        let end = base_candles
            .partition_point(|c| active_tf.bucket_start_timestamp(c.timestamp) <= target_bucket_ts);
        let mut resolved = end.saturating_sub(1).min(base_candles.len() - 1);
        if let Some(cap) = max_allowed_base_index {
            resolved = resolved.min(cap.min(base_candles.len() - 1));
        }
        Some(resolved)
    }

    /// Starts (or resets) causal Bar Replay anchored at `base_index` on `base_candles`.
    pub fn start_at_base_index(
        &mut self,
        base_candles: &[Candle],
        base_index: usize,
    ) -> Option<i64> {
        if base_candles.is_empty() {
            self.exit_replay();
            return None;
        }
        let last = base_candles.len() - 1;
        let idx = base_index.min(last);
        self.selecting_cut_bar = false;
        self.playback_accumulator_secs = 0.0;
        self.start_base_index = idx;
        self.base_cursor_index = idx;
        self.status = if idx >= last {
            ReplayStatus::Completed
        } else {
            ReplayStatus::Paused
        };
        Some(base_candles[idx].timestamp)
    }

    /// Exits Bar Replay and restores full unmasked chart visibility.
    pub fn exit_replay(&mut self) {
        self.status = ReplayStatus::Inactive;
        self.selecting_cut_bar = false;
        self.playback_accumulator_secs = 0.0;
        self.start_base_index = 0;
        self.base_cursor_index = 0;
    }

    pub fn pause(&mut self) {
        if self.status == ReplayStatus::Playing {
            self.status = ReplayStatus::Paused;
        }
        self.playback_accumulator_secs = 0.0;
    }

    /// Toggles between `Playing` and `Paused` when Replay is active; returns `true` if now playing.
    pub fn toggle_play_pause(&mut self, base_candles: &[Candle]) -> bool {
        if !self.is_active() || base_candles.is_empty() {
            return false;
        }
        self.selecting_cut_bar = false;
        self.playback_accumulator_secs = 0.0;
        let last = base_candles.len() - 1;
        if self.base_cursor_index >= last {
            self.status = ReplayStatus::Completed;
            return false;
        }
        self.status = match self.status {
            ReplayStatus::Playing => ReplayStatus::Paused,
            ReplayStatus::Paused | ReplayStatus::Completed => ReplayStatus::Playing,
            ReplayStatus::Inactive => ReplayStatus::Inactive,
        };
        self.status == ReplayStatus::Playing
    }

    /// Advances the replay cursor forward by `steps` units (`ActiveBar` or `BaseBar`).
    pub fn step_forward(
        &mut self,
        base_candles: &[Candle],
        active_tf: Timeframe,
        steps: usize,
    ) -> Option<ReplayTransition> {
        if !self.is_active() || base_candles.is_empty() || steps == 0 {
            return None;
        }
        self.selecting_cut_bar = false;
        let last = base_candles.len() - 1;
        let prev = self.base_cursor_index.min(last);
        if prev >= last {
            self.base_cursor_index = last;
            self.status = ReplayStatus::Completed;
            self.playback_accumulator_secs = 0.0;
            return None;
        }

        let next = match self.step_unit {
            ReplayStepUnit::BaseBar => prev.saturating_add(steps).min(last),
            ReplayStepUnit::ActiveBar => {
                next_active_bar_cursor(base_candles, active_tf, prev, steps)
            }
        };
        if next <= prev {
            return None;
        }
        self.base_cursor_index = next;
        if next >= last {
            self.status = ReplayStatus::Completed;
            self.playback_accumulator_secs = 0.0;
        }
        Some(ReplayTransition {
            prev_base_index: prev,
            new_base_index: next,
            cutoff_timestamp: base_candles[next].timestamp,
        })
    }

    /// Steps the replay cursor backward by `steps` units, pausing playback and preserving causal invariants.
    pub fn step_backward(
        &mut self,
        base_candles: &[Candle],
        active_tf: Timeframe,
        steps: usize,
    ) -> Option<ReplayTransition> {
        if !self.is_active() || base_candles.is_empty() || steps == 0 {
            return None;
        }
        self.selecting_cut_bar = false;
        self.playback_accumulator_secs = 0.0;
        let last = base_candles.len() - 1;
        let prev = self.base_cursor_index.min(last);
        if prev == 0 {
            self.status = if last == 0 {
                ReplayStatus::Completed
            } else {
                ReplayStatus::Paused
            };
            return None;
        }

        let next = match self.step_unit {
            ReplayStepUnit::BaseBar => prev.saturating_sub(steps),
            ReplayStepUnit::ActiveBar => {
                prev_active_bar_cursor(base_candles, active_tf, prev, steps)
            }
        };
        if next >= prev {
            return None;
        }
        self.base_cursor_index = next;
        self.start_base_index = self.start_base_index.min(next);
        self.status = ReplayStatus::Paused;
        Some(ReplayTransition {
            prev_base_index: prev,
            new_base_index: next,
            cutoff_timestamp: base_candles[next].timestamp,
        })
    }

    /// Jumps directly to a causal `target_base_index` (`<= current` or `< total`) while active.
    pub fn jump_to_base_index(
        &mut self,
        base_candles: &[Candle],
        target_base_index: usize,
    ) -> Option<ReplayTransition> {
        if !self.is_active() || base_candles.is_empty() {
            return None;
        }
        self.selecting_cut_bar = false;
        self.playback_accumulator_secs = 0.0;
        let last = base_candles.len() - 1;
        let prev = self.base_cursor_index.min(last);
        let next = target_base_index.min(last);
        if next == prev {
            if self.status == ReplayStatus::Playing {
                self.status = ReplayStatus::Paused;
            }
            return None;
        }
        self.base_cursor_index = next;
        self.start_base_index = self.start_base_index.min(next);
        self.status = if next >= last {
            ReplayStatus::Completed
        } else {
            ReplayStatus::Paused
        };
        Some(ReplayTransition {
            prev_base_index: prev,
            new_base_index: next,
            cutoff_timestamp: base_candles[next].timestamp,
        })
    }

    /// Jumps back to `start_base_index` (`Paused`).
    pub fn jump_to_start(&mut self, base_candles: &[Candle]) -> Option<ReplayTransition> {
        let target = self.start_base_index;
        self.jump_to_base_index(base_candles, target)
    }

    /// Consumes host frame `dt_secs` while `Playing`, advancing up to [`MAX_STEPS_PER_FRAME`] steps.
    pub fn advance_playback_dt(
        &mut self,
        dt_secs: f64,
        base_candles: &[Candle],
        active_tf: Timeframe,
    ) -> Option<ReplayTransition> {
        if self.status != ReplayStatus::Playing || base_candles.is_empty() {
            return None;
        }
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            return None;
        }
        let clamped_dt = dt_secs.min(MAX_PLAYBACK_DT_SECS);
        let interval = self.speed.interval_secs();
        self.playback_accumulator_secs = (self.playback_accumulator_secs + clamped_dt)
            .min(interval * (MAX_STEPS_PER_FRAME as f64));
        if self.playback_accumulator_secs + 1e-12 < interval {
            return None;
        }
        let raw_steps = (self.playback_accumulator_secs / interval).floor() as usize;
        let steps = raw_steps.clamp(1, MAX_STEPS_PER_FRAME);
        self.playback_accumulator_secs =
            (self.playback_accumulator_secs - (steps as f64) * interval).max(0.0);
        self.step_forward(base_candles, active_tf, steps)
    }

    /// Materializes the visible chart series on `active_tf` from `base_candles` with strict
    /// `DEF-001` zero-lookahead causality (`&base_candles[..=base_cursor_index]`).
    pub fn materialize_candles(
        &self,
        base_candles: &[Candle],
        base_tf: Timeframe,
        active_tf: Timeframe,
    ) -> Result<Vec<Candle>, AggregationError> {
        if !base_tf.can_aggregate_to(active_tf) {
            return Err(AggregationError::CannotDownsample {
                source: base_tf,
                target: active_tf,
            });
        }
        // Validate full `base_candles` monotonicity upfront even when Replay is sliced so corrupted
        // tail data can never hide past `base_cursor_index` (`aggregate_ohlcv_up_to` parity).
        for (idx, c) in base_candles.iter().enumerate() {
            if !c.is_valid() {
                return Err(AggregationError::InvalidCandle { index: idx });
            }
            if idx > 0 && c.timestamp < base_candles[idx - 1].timestamp {
                return Err(AggregationError::NonMonotonicTimestamps {
                    index: idx,
                    prev: base_candles[idx - 1].timestamp,
                    current: c.timestamp,
                });
            }
        }
        if !self.is_active() || base_candles.is_empty() {
            return aggregate_ohlcv(base_candles, base_tf, active_tf);
        }
        let end = (self.base_cursor_index + 1).min(base_candles.len());
        aggregate_ohlcv(&base_candles[..end], base_tf, active_tf)
    }
}

/// Computes the causal base-candle index reached by advancing `steps` active-timeframe bars.
///
/// - If `current_cursor` sits mid-bucket on `active_tf`, the first step completes that current bucket.
/// - If `current_cursor` already sits on the final base candle of its `active_tf` bucket, the step
///   advances to the final base candle of the next `active_tf` bucket.
#[must_use]
pub fn next_active_bar_cursor(
    base_candles: &[Candle],
    active_tf: Timeframe,
    current_cursor: usize,
    steps: usize,
) -> usize {
    if base_candles.is_empty() {
        return 0;
    }
    let last = base_candles.len() - 1;
    let mut cursor = current_cursor.min(last);
    for _ in 0..steps {
        if cursor >= last {
            break;
        }
        let cur_bucket = active_tf.bucket_start_timestamp(base_candles[cursor].timestamp);
        let next_bucket = active_tf.bucket_start_timestamp(base_candles[cursor + 1].timestamp);
        let target_bucket = if next_bucket == cur_bucket {
            cur_bucket
        } else {
            next_bucket
        };
        let end = base_candles[cursor + 1..]
            .partition_point(|c| active_tf.bucket_start_timestamp(c.timestamp) <= target_bucket);
        cursor = (cursor + end).min(last);
    }
    cursor
}

/// Computes the causal base-candle index reached by stepping backward `steps` active-timeframe bars.
#[must_use]
pub fn prev_active_bar_cursor(
    base_candles: &[Candle],
    active_tf: Timeframe,
    current_cursor: usize,
    steps: usize,
) -> usize {
    if base_candles.is_empty() {
        return 0;
    }
    let last = base_candles.len() - 1;
    let mut cursor = current_cursor.min(last);
    for _ in 0..steps {
        if cursor == 0 {
            break;
        }
        let cur_bucket = active_tf.bucket_start_timestamp(base_candles[cursor].timestamp);
        let bucket_first = base_candles[..=cursor]
            .partition_point(|c| active_tf.bucket_start_timestamp(c.timestamp) < cur_bucket);
        if bucket_first == 0 {
            cursor = 0;
            break;
        }
        cursor = bucket_first - 1;
    }
    cursor
}
