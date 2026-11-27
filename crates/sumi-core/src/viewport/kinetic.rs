use serde::{Deserialize, Serialize};

/// Constants matching Lightweight Charts v5.2.0 `KineticScrollConstants` (`src/gui/pane-widget.ts:37-42`)
/// and `KineticAnimation` (`src/model/kinetic-animation.ts:10-13`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KineticScrollConfig {
    /// Minimum drag velocity in logical pixels per millisecond (`MinScrollSpeed = 0.2`).
    pub min_scroll_speed_px_per_ms: f64,
    /// Maximum capped drag velocity in logical pixels per millisecond (`MaxScrollSpeed = 7.0`).
    pub max_scroll_speed_px_per_ms: f64,
    /// Exponential damping coefficient per millisecond (`DumpingCoeff = 0.997`).
    pub damping_coeff: f64,
    /// Minimum displacement in pixels before pushing a new ring-buffer sample (`ScrollMinMove = 15.0`).
    pub scroll_min_move_px: f64,
    /// Maximum pause between last move sample and pointer release (`MaxStartDelay = 50.0` ms).
    pub max_start_delay_ms: f64,
    /// Stopping threshold in position units (`EpsilonDistance = 1.0`).
    pub epsilon_distance: f64,
}

impl Default for KineticScrollConfig {
    fn default() -> Self {
        Self {
            min_scroll_speed_px_per_ms: 0.2,
            max_scroll_speed_px_per_ms: 7.0,
            damping_coeff: 0.997,
            scroll_min_move_px: 15.0,
            max_start_delay_ms: 50.0,
            epsilon_distance: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct TimeAndPosition {
    time_ms: f64,
    position: f64,
}

/// Inertial horizontal scroll state machine operating on `TimeScale::right_offset` (bar units).
///
/// Mirrors `src/model/kinetic-animation.ts` (L1-143) wired as in `src/gui/pane-widget.ts` (L780-808).
/// Pure `f64` timestamps (`time_ms`) make this struct 100% deterministic in tests and `wasm32`-compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KineticAnimation {
    pos1: Option<TimeAndPosition>,
    pos2: Option<TimeAndPosition>,
    pos3: Option<TimeAndPosition>,
    pos4: Option<TimeAndPosition>,
    animation_start: Option<TimeAndPosition>,
    duration_ms: f64,
    speed_units_per_ms: f64,
    min_move: f64,
    min_speed: f64,
    max_speed: f64,
    damping_coeff: f64,
    max_start_delay_ms: f64,
    epsilon_distance: f64,
}

impl KineticAnimation {
    /// Creates a `KineticAnimation` configured in raw position units (`min_speed`, `max_speed`, `min_move`).
    #[must_use]
    pub fn new(
        min_speed: f64,
        max_speed: f64,
        damping_coeff: f64,
        min_move: f64,
        max_start_delay_ms: f64,
        epsilon_distance: f64,
    ) -> Self {
        let clamped_damping =
            if damping_coeff.is_finite() && (0.01..0.999_999).contains(&damping_coeff) {
                damping_coeff
            } else {
                0.997
            };
        Self {
            pos1: None,
            pos2: None,
            pos3: None,
            pos4: None,
            animation_start: None,
            duration_ms: 0.0,
            speed_units_per_ms: 0.0,
            min_move: min_move.max(0.0),
            min_speed: min_speed.max(1e-9),
            max_speed: max_speed.max(min_speed.max(1e-9)),
            damping_coeff: clamped_damping,
            max_start_delay_ms: max_start_delay_ms.max(0.0),
            epsilon_distance: epsilon_distance.max(1e-6),
        }
    }

    /// Creates a `KineticAnimation` scaled for `TimeScale::right_offset` (bar units) given `bar_spacing`
    /// (`src/gui/pane-widget.ts:780-785`).
    #[must_use]
    pub fn for_bar_spacing(bar_spacing: f64, config: KineticScrollConfig) -> Self {
        let bs = if bar_spacing.is_finite() && bar_spacing > 1e-4 {
            bar_spacing
        } else {
            6.0
        };
        Self::new(
            config.min_scroll_speed_px_per_ms / bs,
            config.max_scroll_speed_px_per_ms / bs,
            config.damping_coeff,
            config.scroll_min_move_px / bs,
            config.max_start_delay_ms,
            config.epsilon_distance / bs,
        )
    }

    /// Records a pointer-drag sample `(right_offset_bars, time_ms)` (`kinetic-animation.ts:51-67`).
    pub fn add_position(&mut self, position: f64, time_ms: f64) {
        if !position.is_finite() || !time_ms.is_finite() {
            return;
        }
        if let Some(p1) = &mut self.pos1 {
            if (time_ms - p1.time_ms).abs() < 1e-9 {
                p1.position = position;
                return;
            }
            if time_ms < p1.time_ms {
                return;
            }
            if (p1.position - position).abs() < self.min_move {
                return;
            }
        }
        self.pos4 = self.pos3;
        self.pos3 = self.pos2;
        self.pos2 = self.pos1;
        self.pos1 = Some(TimeAndPosition { time_ms, position });
    }

    /// Computes initial weighted velocity and launches inertial decay if thresholds are met (`kinetic-animation.ts:69-126`).
    ///
    /// Returns `true` if inertial animation became active.
    pub fn start(&mut self, position: f64, time_ms: f64) -> bool {
        self.animation_start = None;
        self.duration_ms = 0.0;
        self.speed_units_per_ms = 0.0;

        if !position.is_finite() || !time_ms.is_finite() {
            return false;
        }
        let (Some(p1), Some(p2)) = (self.pos1, self.pos2) else {
            return false;
        };
        if time_ms - p1.time_ms > self.max_start_delay_ms || time_ms < p1.time_ms {
            return false;
        }

        let Some((speed1, dist1)) = segment_speed_and_distance(p1, p2, self.max_speed) else {
            return false;
        };
        if speed1 == 0.0 || dist1 == 0.0 {
            return false;
        }

        let mut speeds = [0.0_f64; 3];
        let mut dists = [0.0_f64; 3];
        let mut count = 1_usize;
        speeds[0] = speed1;
        dists[0] = dist1;
        let mut total_distance = dist1;

        if let Some(p3) = self.pos3
            && let Some((speed2, dist2)) = segment_speed_and_distance(p2, p3, self.max_speed)
            && speed2 != 0.0
            && speed2.signum() == speed1.signum()
        {
            speeds[count] = speed2;
            dists[count] = dist2;
            total_distance += dist2;
            count += 1;

            if let Some(p4) = self.pos4
                && let Some((speed3, dist3)) = segment_speed_and_distance(p3, p4, self.max_speed)
                && speed3 != 0.0
                && speed3.signum() == speed1.signum()
            {
                speeds[count] = speed3;
                dists[count] = dist3;
                total_distance += dist3;
                count += 1;
            }
        }

        if total_distance.abs() < 1e-12 {
            return false;
        }

        let mut result_speed = 0.0;
        for i in 0..count {
            result_speed += (dists[i] / total_distance) * speeds[i];
        }

        if result_speed.abs() < self.min_speed || !result_speed.is_finite() {
            return false;
        }

        if !self.damping_coeff.is_finite()
            || self.damping_coeff <= 0.0
            || self.damping_coeff >= 1.0
        {
            return false;
        }
        let ln_k = self.damping_coeff.ln();
        if !ln_k.is_finite() || ln_k.abs() <= 1e-12 {
            return false;
        }
        let arg = (self.epsilon_distance * ln_k) / -result_speed.abs();
        if arg <= 0.0 || !arg.is_finite() {
            return false;
        }
        let duration = (arg.ln() / ln_k).max(0.0);
        if duration <= 0.0 || !duration.is_finite() {
            return false;
        }

        self.animation_start = Some(TimeAndPosition { time_ms, position });
        self.speed_units_per_ms = result_speed;
        self.duration_ms = duration;
        true
    }

    /// Evaluates decayed position at `time_ms` (`kinetic-animation.ts:128-132`).
    #[must_use]
    pub fn position_at(&self, time_ms: f64) -> Option<f64> {
        let start = self.animation_start?;
        if !time_ms.is_finite() {
            return Some(start.position);
        }
        if !self.duration_ms.is_finite()
            || self.duration_ms < 0.0
            || !self.damping_coeff.is_finite()
            || self.damping_coeff <= 0.0
            || self.damping_coeff >= 1.0
        {
            return Some(start.position);
        }
        let elapsed = (time_ms - start.time_ms).clamp(0.0, self.duration_ms);
        let ln_k = self.damping_coeff.ln();
        if !ln_k.is_finite() || ln_k.abs() <= 1e-12 {
            return Some(start.position);
        }
        let pos = start.position
            + self.speed_units_per_ms * (self.damping_coeff.powf(elapsed) - 1.0) / ln_k;
        pos.is_finite().then_some(pos)
    }

    /// Returns `true` when no animation is running or elapsed time reached `duration_ms` (`kinetic-animation.ts:134-136`).
    #[inline]
    #[must_use]
    pub fn is_finished(&self, time_ms: f64) -> bool {
        let Some(start) = self.animation_start else {
            return true;
        };
        if !time_ms.is_finite() {
            return true;
        }
        (time_ms - start.time_ms) >= self.duration_ms
    }

    #[inline]
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.animation_start.is_some()
    }

    #[inline]
    #[must_use]
    pub fn duration_ms(&self) -> f64 {
        self.duration_ms
    }

    #[inline]
    #[must_use]
    pub fn initial_speed(&self) -> f64 {
        self.speed_units_per_ms
    }

    /// Immediately cancels any running animation and clears sample history.
    pub fn stop(&mut self) {
        self.animation_start = None;
        self.duration_ms = 0.0;
        self.speed_units_per_ms = 0.0;
        self.pos1 = None;
        self.pos2 = None;
        self.pos3 = None;
        self.pos4 = None;
    }
}

fn segment_speed_and_distance(
    newer: TimeAndPosition,
    older: TimeAndPosition,
    max_speed: f64,
) -> Option<(f64, f64)> {
    let dt = newer.time_ms - older.time_ms;
    if dt <= 1e-9 || !dt.is_finite() {
        return None;
    }
    let dist = newer.position - older.position;
    let raw_speed = dist / dt;
    if !raw_speed.is_finite() {
        return None;
    }
    let clamped_speed = raw_speed.signum() * raw_speed.abs().min(max_speed);
    Some((clamped_speed, dist))
}
