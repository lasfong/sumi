use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::data::{Timeframe, remap_timestamp_to_bar_index, resolve_anchor_timestamp};
use crate::model::Candle;

use super::geometry::{HandlePart, LocalPoint, PaneDrawingProjection};
use super::hit_test::{
    HitMatch, HitTarget, MAGNET_THRESHOLD_PX, hit_test_pane_drawings_causal, snap_to_visible_ohlc,
};
use super::{
    Drawing, DrawingAnchor, DrawingGeometry, DrawingId, DrawingPaneTarget, DrawingStyle,
    DrawingTool, FibDirection, MagnetMode, has_non_zero_direction,
};

/// Maximum undo/redo history depth retained in memory.
pub const MAX_UNDO_STEPS: usize = 64;

/// Minimum pointer drag distance (`px`) before a single press-drag-release gesture commits a 2-anchor tool.
pub const DRAG_COMMIT_DISTANCE_PX: f64 = 6.0;

/// Minimum pointer movement (`px`) before a handle or body drag activates, preventing pure selection clicks
/// or micro-jitter from mutating geometry or wiping `redo_stack`.
pub const EDIT_DRAG_THRESHOLD_PX: f64 = 2.0;

/// Reserved sentinel identifier used exclusively for uncommitted rubber-band preview shapes.
pub const PREVIEW_DRAWING_ID: DrawingId = u64::MAX;

/// Safety cap on persisted drawing count loaded from storage.
pub const MAX_PERSISTED_DRAWINGS: usize = 4_096;

const COMPACT_ID_THRESHOLD: DrawingId = u64::MAX - 1_000_000;

/// Transactional multi-property edit payload committed by the Interactive Drawing Property Inspector (`Milestone R15`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingPropertyPatch {
    pub visible: bool,
    pub locked: bool,
    pub style: DrawingStyle,
    pub geometry: DrawingGeometry,
    #[serde(default)]
    pub trend_extend_left: bool,
    #[serde(default)]
    pub trend_extend_right: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_fib_levels: Option<Vec<f64>>,
}

impl DrawingPropertyPatch {
    #[inline]
    #[must_use]
    pub fn from_drawing(drawing: &Drawing) -> Self {
        Self {
            visible: drawing.visible,
            locked: drawing.locked,
            style: drawing.style,
            geometry: drawing.geometry.clone(),
            trend_extend_left: drawing.trend_extend_left,
            trend_extend_right: drawing.trend_extend_right,
            custom_fib_levels: drawing.custom_fib_levels.clone(),
        }
    }
}

/// Validation failure returned by [`DrawingStore::apply_drawing_property_patch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingPatchError {
    DrawingNotFound,
    CausallyMasked,
    DrawingLocked,
    ToolKindMismatch,
    InvalidStyle,
    InvalidGeometry,
}

impl DrawingPatchError {
    #[inline]
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::DrawingNotFound => "Drawing no longer exists in store",
            Self::CausallyMasked => "Drawing is causally masked at active Bar Replay cutoff",
            Self::DrawingLocked => "Drawing is locked — unlock first to edit properties",
            Self::ToolKindMismatch => "Patch geometry kind does not match target drawing",
            Self::InvalidStyle => "Line width must be 1..=8 px and fill opacity 0..=0.65",
            Self::InvalidGeometry => "Invalid anchor coordinates or bracket ordering for tool",
        }
    }
}

/// Reversible command recorded on the undo/redo stack (`DrawingCommandHistory.ts`).
#[derive(Debug, Clone, PartialEq)]
pub enum DrawingCommand {
    Create(Drawing),
    Modify {
        before: Drawing,
        after: Drawing,
    },
    Delete {
        index: usize,
        drawing: Drawing,
    },
    ReplaceAll {
        before: Vec<Drawing>,
        after: Vec<Drawing>,
        before_selected: Option<DrawingId>,
        after_selected: Option<DrawingId>,
    },
}

/// Live interaction phase for rubber-band creation and transactional handle/body drags.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DrawingInteractionPhase {
    #[default]
    Idle,
    PlacingSingle {
        tool: DrawingTool,
        pane: DrawingPaneTarget,
        preview_anchor: DrawingAnchor,
    },
    PlacingTwoAnchor {
        tool: DrawingTool,
        pane: DrawingPaneTarget,
        first_anchor: DrawingAnchor,
        first_local: LocalPoint,
        preview_anchor: DrawingAnchor,
        wait_for_second_press: bool,
        second_press_active: bool,
    },
    DraggingHandle {
        drawing_id: DrawingId,
        pane: DrawingPaneTarget,
        handle: HandlePart,
        start_local: LocalPoint,
        grab_offset: LocalPoint,
        drag_activated: bool,
        original: Drawing,
    },
    DraggingBody {
        drawing_id: DrawingId,
        pane: DrawingPaneTarget,
        start_local: LocalPoint,
        original_anchors_local: Vec<LocalPoint>,
        drag_activated: bool,
        original: Drawing,
    },
}

/// Signals whether a pane pointer gesture was claimed by the drawing engine or passes through to `ChartViewportState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerCaptureOutcome {
    PassThroughToViewport,
    CapturedByDrawing,
}

const fn default_next_id() -> DrawingId {
    1
}

/// Persisted drawing collection + transient creation/drag/undo controller (`Milestone R05`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawingStore {
    #[serde(default)]
    pub drawings: Vec<Drawing>,
    #[serde(default = "default_next_id")]
    next_id: DrawingId,
    #[serde(default)]
    pub active_tool: DrawingTool,
    #[serde(default)]
    pub magnet_mode: MagnetMode,
    #[serde(default)]
    pub selected_id: Option<DrawingId>,

    #[serde(skip)]
    phase: DrawingInteractionPhase,
    #[serde(skip)]
    hovered_hit: Option<(DrawingPaneTarget, HitMatch)>,
    #[serde(skip)]
    undo_stack: Vec<DrawingCommand>,
    #[serde(skip)]
    redo_stack: Vec<DrawingCommand>,
    #[serde(skip)]
    replay_cutoff_timestamp: Option<i64>,
    #[serde(skip)]
    causal_visible_candles: usize,
}

impl Default for DrawingStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawingStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            drawings: Vec::new(),
            next_id: 1,
            active_tool: DrawingTool::Select,
            magnet_mode: MagnetMode::Off,
            selected_id: None,
            phase: DrawingInteractionPhase::Idle,
            hovered_hit: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            replay_cutoff_timestamp: None,
            causal_visible_candles: usize::MAX,
        }
    }

    /// Sanitizes state loaded from `serde` storage (`eframe` persistence), dropping invalid/duplicate entries
    /// and compacting near-`u64::MAX` identifiers so `alloc_id` never collides with `PREVIEW_DRAWING_ID`.
    pub fn sanitize_deserialized(&mut self) {
        self.phase = DrawingInteractionPhase::Idle;
        self.hovered_hit = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.replay_cutoff_timestamp = None;
        self.causal_visible_candles = usize::MAX;

        if self.drawings.len() > MAX_PERSISTED_DRAWINGS {
            self.drawings.truncate(MAX_PERSISTED_DRAWINGS);
        }

        let mut seen = HashSet::with_capacity(self.drawings.len());
        let mut max_id: DrawingId = 0;
        self.drawings.retain_mut(|d| {
            d.style.sanitize();
            d.sanitize_modifiers();
            if let DrawingGeometry::FixedRangeVolumeProfile { row_count, .. } = &mut d.geometry {
                *row_count = (*row_count).clamp(8, 200);
            }
            if !d.is_valid() || !seen.insert(d.id) {
                return false;
            }
            max_id = max_id.max(d.id);
            true
        });

        if max_id >= COMPACT_ID_THRESHOLD || self.next_id >= COMPACT_ID_THRESHOLD {
            self.compact_ids();
        } else {
            self.next_id = self.next_id.max(max_id.saturating_add(1)).max(1);
        }

        if let Some(sel) = self.selected_id
            && !self.drawings.iter().any(|d| d.id == sel)
        {
            self.selected_id = None;
        }
        // Always start fresh sessions in `Select` cursor mode so restored workspaces pan normally.
        self.active_tool = DrawingTool::Select;
    }

    #[inline]
    #[must_use]
    pub const fn phase(&self) -> &DrawingInteractionPhase {
        &self.phase
    }

    #[inline]
    #[must_use]
    pub const fn hovered_hit(&self) -> Option<(DrawingPaneTarget, HitMatch)> {
        self.hovered_hit
    }

    #[inline]
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    #[inline]
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    #[inline]
    #[must_use]
    pub fn has_unlocked_drawings(&self) -> bool {
        self.drawings.iter().any(|d| {
            !d.locked
                && d.is_causally_visible(self.replay_cutoff_timestamp, self.causal_visible_candles)
        })
    }

    #[must_use]
    pub fn selected_drawing(&self) -> Option<&Drawing> {
        let id = self.selected_id?;
        self.drawings.iter().find(|d| {
            d.id == id
                && d.is_causally_eligible(self.replay_cutoff_timestamp, self.causal_visible_candles)
        })
    }

    /// Iterates over all drawings causally eligible at the active replay cutoff (including hidden
    /// `visible == false` or `locked == true` annotations so the Inspector layer list can manage them).
    pub fn causally_eligible_drawings(&self) -> impl Iterator<Item = &Drawing> {
        let cutoff = self.replay_cutoff_timestamp;
        let vis_cnt = self.causal_visible_candles;
        self.drawings
            .iter()
            .filter(move |d| d.is_causally_eligible(cutoff, vis_cnt))
    }

    /// Selects a causally eligible drawing by `id` (or clears selection when `None`).
    pub fn select_drawing_by_id(&mut self, id: Option<DrawingId>) -> bool {
        self.rollback_active_phase();
        let Some(target_id) = id else {
            self.selected_id = None;
            return true;
        };
        let cutoff = self.replay_cutoff_timestamp;
        let vis_cnt = self.causal_visible_candles;
        if self
            .drawings
            .iter()
            .any(|d| d.id == target_id && d.is_causally_eligible(cutoff, vis_cnt))
        {
            self.selected_id = Some(target_id);
            true
        } else {
            false
        }
    }

    #[inline]
    #[must_use]
    pub const fn replay_cutoff_timestamp(&self) -> Option<i64> {
        self.replay_cutoff_timestamp
    }

    /// Synchronizes the active causal Bar Replay cutoff (`Some(cutoff_ts)`) or clears it (`None`)
    /// when exiting replay (`Milestone R07 / DEF-001`).
    pub fn set_replay_cutoff_timestamp(
        &mut self,
        cutoff: Option<i64>,
        visible_candle_count: usize,
    ) {
        self.replay_cutoff_timestamp = cutoff;
        self.causal_visible_candles = if cutoff.is_some() {
            visible_candle_count
        } else {
            usize::MAX
        };
        self.sanitize_causal_selection_and_hover();
    }

    fn sanitize_causal_selection_and_hover(&mut self) {
        let cutoff = self.replay_cutoff_timestamp;
        let vis_cnt = self.causal_visible_candles;
        if let Some(sel_id) = self.selected_id {
            let masked = self
                .drawings
                .iter()
                .find(|d| d.id == sel_id)
                .is_none_or(|d| !d.is_causally_eligible(cutoff, vis_cnt));
            if masked {
                self.rollback_active_phase();
                self.selected_id = None;
            }
        }
        if let Some((_, hit)) = self.hovered_hit {
            let masked = self
                .drawings
                .iter()
                .find(|d| d.id == hit.drawing_id)
                .is_none_or(|d| !d.is_causally_visible(cutoff, vis_cnt));
            if masked {
                self.hovered_hit = None;
            }
        }
    }

    /// Refreshes cached `DrawingAnchor::timestamp` values for anchors that were drawn into
    /// right-offset future whitespace (`bar_index > old_last_idx`) and have now been reached by
    /// newly materialized replay bars on the **same** timeframe (`bar_index <= new_last_idx`).
    pub fn refresh_materialized_anchor_timestamps(
        &mut self,
        old_candle_count: usize,
        new_candles: &[Candle],
        tf: Timeframe,
    ) {
        if self.replay_cutoff_timestamp.is_some() {
            self.causal_visible_candles = new_candles.len();
        }
        if new_candles.len() <= old_candle_count || new_candles.is_empty() {
            return;
        }
        let old_last = if old_candle_count == 0 {
            -1.0
        } else {
            (old_candle_count - 1) as f64
        };
        let new_last = (new_candles.len() - 1) as f64;
        let refresh_anchor = |a: &mut DrawingAnchor| {
            if a.bar_index > old_last
                && a.bar_index <= new_last
                && let Some(ts) = resolve_anchor_timestamp(a.bar_index, new_candles, tf)
                && ts.is_finite()
                && ts.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP
            {
                a.timestamp = Some(ts);
            }
        };
        let refresh_geom = |geom: &mut DrawingGeometry| match geom {
            DrawingGeometry::HorizontalLine { anchor }
            | DrawingGeometry::AnchoredVwap { anchor, .. }
            | DrawingGeometry::TextNote { anchor, .. } => refresh_anchor(anchor),
            DrawingGeometry::TrendLine { start, end }
            | DrawingGeometry::Ray {
                origin: start,
                through: end,
            }
            | DrawingGeometry::Rectangle {
                corner_a: start,
                corner_b: end,
            }
            | DrawingGeometry::FibonacciRetracement { start, end, .. }
            | DrawingGeometry::RiskRewardBox { start, end, .. }
            | DrawingGeometry::FixedRangeVolumeProfile { start, end, .. } => {
                refresh_anchor(start);
                refresh_anchor(end);
            }
        };
        let refresh_cmd = |cmd: &mut DrawingCommand| match cmd {
            DrawingCommand::Create(d) | DrawingCommand::Delete { drawing: d, .. } => {
                refresh_geom(&mut d.geometry);
            }
            DrawingCommand::Modify { before, after } => {
                refresh_geom(&mut before.geometry);
                refresh_geom(&mut after.geometry);
            }
            DrawingCommand::ReplaceAll { before, after, .. } => {
                for d in before {
                    refresh_geom(&mut d.geometry);
                }
                for d in after {
                    refresh_geom(&mut d.geometry);
                }
            }
        };
        for d in &mut self.drawings {
            refresh_geom(&mut d.geometry);
        }
        for cmd in &mut self.undo_stack {
            refresh_cmd(cmd);
        }
        for cmd in &mut self.redo_stack {
            refresh_cmd(cmd);
        }
    }

    #[inline]
    #[must_use]
    pub fn is_capturing_pane_gesture(&self, pane: DrawingPaneTarget) -> bool {
        if self.active_tool != DrawingTool::Select {
            if matches!(
                self.active_tool,
                DrawingTool::RiskRewardBox
                    | DrawingTool::FixedRangeVolumeProfile
                    | DrawingTool::AnchoredVwap
            ) && pane != DrawingPaneTarget::Main
            {
                return false;
            }
            return true;
        }
        match self.phase {
            DrawingInteractionPhase::Idle => false,
            DrawingInteractionPhase::PlacingSingle { pane: p, .. }
            | DrawingInteractionPhase::PlacingTwoAnchor { pane: p, .. }
            | DrawingInteractionPhase::DraggingHandle { pane: p, .. }
            | DrawingInteractionPhase::DraggingBody { pane: p, .. } => p == pane,
        }
    }

    /// Allocates an ID and inserts a validated drawing programmatically (e.g. showcase presets or tests).
    pub fn add_drawing(
        &mut self,
        pane: DrawingPaneTarget,
        geometry: DrawingGeometry,
        style: Option<DrawingStyle>,
    ) -> Option<DrawingId> {
        if matches!(
            geometry,
            DrawingGeometry::RiskRewardBox { .. }
                | DrawingGeometry::FixedRangeVolumeProfile { .. }
                | DrawingGeometry::AnchoredVwap { .. }
        ) && pane != DrawingPaneTarget::Main
        {
            return None;
        }
        if !geometry.is_valid() {
            return None;
        }
        let id = self.alloc_id();
        let resolved_style =
            style.unwrap_or_else(|| DrawingStyle::default_for_tool(geometry.tool_kind()));
        let mut drawing = Drawing::new(id, pane, geometry, resolved_style);
        drawing.created_at_timestamp = self.replay_cutoff_timestamp;
        self.push_undo(DrawingCommand::Create(drawing.clone()));
        self.drawings.push(drawing);
        Some(id)
    }

    /// Atomically replaces the drawing collection with `entries` (allocating fresh monotonic IDs for any
    /// entry with `id == 0` while preserving `locked`, `visible`, `style`, and `magnet_mode`) as a single
    /// undoable `DrawingCommand::ReplaceAll` step.
    pub fn replace_all(
        &mut self,
        entries: Vec<Drawing>,
        select_index: Option<usize>,
    ) -> Option<DrawingId> {
        self.rollback_active_phase();
        self.hovered_hit = None;
        let before = self.drawings.clone();
        let before_selected = self.selected_id;

        let mut prepared = Vec::with_capacity(entries.len());
        let mut chosen_selected: Option<DrawingId> = None;
        for (idx, mut d) in entries.into_iter().enumerate() {
            d.style.sanitize();
            d.sanitize_modifiers();
            if !d.geometry.is_valid()
                || (matches!(
                    d.geometry,
                    DrawingGeometry::RiskRewardBox { .. }
                        | DrawingGeometry::FixedRangeVolumeProfile { .. }
                        | DrawingGeometry::AnchoredVwap { .. }
                ) && d.pane != DrawingPaneTarget::Main)
            {
                continue;
            }
            if d.id == 0 || d.id == PREVIEW_DRAWING_ID {
                d.id = self.alloc_id();
            } else {
                self.next_id = self.next_id.max(d.id.saturating_add(1));
            }
            if select_index == Some(idx) {
                chosen_selected = Some(d.id);
            }
            prepared.push(d);
        }

        if before != prepared {
            self.drawings.clone_from(&prepared);
            self.selected_id = chosen_selected;
            self.active_tool = DrawingTool::Select;
            self.push_undo(DrawingCommand::ReplaceAll {
                before,
                after: prepared,
                before_selected,
                after_selected: chosen_selected,
            });
        } else {
            self.selected_id = chosen_selected;
            self.active_tool = DrawingTool::Select;
        }
        chosen_selected
    }

    /// Switches active annotation tool, cleanly rolling back any uncommitted drag or partial placement.
    pub fn set_tool(&mut self, tool: DrawingTool) {
        if tool != self.active_tool {
            self.rollback_active_phase();
            self.active_tool = tool;
            if tool != DrawingTool::Select {
                self.selected_id = None;
            }
        }
    }

    /// Handles `Escape` or explicit cancel: rolls back active placement/drag first, otherwise clears selection.
    pub fn cancel_or_deselect(&mut self) -> bool {
        if self.phase != DrawingInteractionPhase::Idle {
            self.rollback_active_phase();
            self.active_tool = DrawingTool::Select;
            return true;
        }
        if self.active_tool != DrawingTool::Select {
            self.active_tool = DrawingTool::Select;
            return true;
        }
        if self.selected_id.take().is_some() {
            return true;
        }
        false
    }

    /// Cancels only transient placement/drag state (e.g. on dataset reload) while keeping committed drawings.
    pub fn cancel_active_gesture(&mut self) {
        self.rollback_active_phase();
        self.hovered_hit = None;
    }

    /// Remaps all committed drawings and undo/redo history entries from `(old_candles, old_tf)`
    /// onto `(new_candles, new_tf)` using exact market UTC timestamps (`Milestone R06`).
    ///
    /// Sub-bar timestamps recorded on finer timeframes (`1m`/`5m`) are preserved across coarser
    /// excursions (`5m -> 1D -> 1W -> 5m`) with zero drift, and re-evaluated automatically whenever
    /// a user edits a handle or body on the new timeframe.
    pub fn remap_timeframe(
        &mut self,
        old_candles: &[Candle],
        old_tf: Timeframe,
        new_candles: &[Candle],
        new_tf: Timeframe,
    ) {
        if old_candles.is_empty() || new_candles.is_empty() {
            return;
        }
        self.rollback_active_phase();
        self.hovered_hit = None;

        for d in &mut self.drawings {
            remap_drawing_geometry(&mut d.geometry, old_candles, old_tf, new_candles, new_tf);
        }
        for cmd in &mut self.undo_stack {
            remap_command_anchors(cmd, old_candles, old_tf, new_candles, new_tf);
        }
        for cmd in &mut self.redo_stack {
            remap_command_anchors(cmd, old_candles, old_tf, new_candles, new_tf);
        }
        if self.replay_cutoff_timestamp.is_some() {
            self.causal_visible_candles = new_candles.len();
        }
        self.sanitize_causal_selection_and_hover();
    }

    /// Deletes the currently selected drawing if it exists, is causally visible, and is unlocked.
    pub fn delete_selected(&mut self) -> bool {
        let Some(sel_id) = self.selected_id else {
            return false;
        };
        self.rollback_active_phase();
        let Some(idx) = self.drawings.iter().position(|d| d.id == sel_id) else {
            self.selected_id = None;
            return false;
        };
        if !self.drawings[idx]
            .is_causally_eligible(self.replay_cutoff_timestamp, self.causal_visible_candles)
        {
            self.selected_id = None;
            return false;
        }
        if self.drawings[idx].locked {
            return false;
        }
        let removed = self.drawings.remove(idx);
        self.selected_id = None;
        self.push_undo(DrawingCommand::Delete {
            index: idx,
            drawing: removed,
        });
        true
    }

    /// Removes all causally visible unlocked drawings across all panes (preserving locked and
    /// causally masked future annotations).
    pub fn clear_unlocked(&mut self) -> usize {
        self.rollback_active_phase();
        let cutoff = self.replay_cutoff_timestamp;
        let vis_cnt = self.causal_visible_candles;
        let before = self.drawings.clone();
        let before_selected = self.selected_id;
        self.drawings
            .retain(|d| d.locked || !d.is_causally_visible(cutoff, vis_cnt));
        let removed_count = before.len().saturating_sub(self.drawings.len());
        if removed_count > 0 {
            if let Some(sel) = self.selected_id
                && !self.drawings.iter().any(|d| d.id == sel)
            {
                self.selected_id = None;
            }
            self.push_undo(DrawingCommand::ReplaceAll {
                before,
                after: self.drawings.clone(),
                before_selected,
                after_selected: self.selected_id,
            });
        }
        removed_count
    }

    /// Toggles the `locked` state of the currently selected drawing.
    pub fn toggle_selected_lock(&mut self) -> bool {
        self.mutate_selected(|d| {
            d.locked = !d.locked;
            true
        })
    }

    /// Cycles the color preset of the currently selected unlocked drawing.
    pub fn cycle_selected_color(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            d.style.color = d.style.color.next();
            true
        })
    }

    /// Cycles the stroke dash pattern (`Solid -> Dashed -> Dotted`) of the selected unlocked drawing.
    pub fn cycle_selected_dash(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            d.style.dash = d.style.dash.next();
            true
        })
    }

    /// Reverses `0% <-> 100%` direction on a selected unlocked `FibonacciRetracement`.
    pub fn reverse_selected_fibonacci(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            if let DrawingGeometry::FibonacciRetracement { direction, .. } = &mut d.geometry {
                *direction = direction.reversed();
                true
            } else {
                false
            }
        })
    }

    /// Toggles `extend_right` on a selected unlocked `FibonacciRetracement`.
    pub fn toggle_selected_fib_extend(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            if let DrawingGeometry::FibonacciRetracement { extend_right, .. } = &mut d.geometry {
                *extend_right = !*extend_right;
                true
            } else {
                false
            }
        })
    }

    /// Flips Long <-> Short around `start.value` (`entry_price`) on a selected unlocked `RiskRewardBox`.
    pub fn reverse_selected_risk_reward(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            if let DrawingGeometry::RiskRewardBox {
                start,
                end,
                stop_price,
            } = &mut d.geometry
            {
                let Some(side) =
                    DrawingGeometry::risk_reward_side_of(start.value, end.value, *stop_price)
                else {
                    return false;
                };
                let entry = start.value;
                let reward_dist = (end.value - entry).abs();
                let risk_dist = (entry - *stop_price).abs();
                let (new_target, new_stop) = match side {
                    crate::trading::PositionSide::Long => (
                        (entry - reward_dist).max(entry * 0.05),
                        (entry + risk_dist).min(super::MAX_ABS_DRAWING_VALUE),
                    ),
                    crate::trading::PositionSide::Short => (
                        (entry + reward_dist).min(super::MAX_ABS_DRAWING_VALUE),
                        (entry - risk_dist).max(entry * 0.05),
                    ),
                };
                if DrawingGeometry::risk_reward_side_of(entry, new_target, new_stop).is_none() {
                    return false;
                }
                end.value = new_target;
                *stop_price = new_stop;
                true
            } else {
                false
            }
        })
    }

    /// Updates `(entry, stop, target)` levels on a selected unlocked `RiskRewardBox` (e.g. from Risk Planner tick snapping).
    pub fn update_selected_risk_reward_levels(
        &mut self,
        entry: f64,
        stop: f64,
        target: f64,
    ) -> bool {
        if DrawingGeometry::risk_reward_side_of(entry, target, stop).is_none() {
            return false;
        }
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            if let DrawingGeometry::RiskRewardBox {
                start,
                end,
                stop_price,
            } = &mut d.geometry
            {
                start.value = entry;
                end.value = target;
                *stop_price = stop;
                true
            } else {
                false
            }
        })
    }

    /// Toggles `show_bands` on a selected unlocked `AnchoredVwap` drawing (`Milestone R11`).
    pub fn toggle_selected_avwap_bands(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            if let DrawingGeometry::AnchoredVwap { show_bands, .. } = &mut d.geometry {
                *show_bands = !*show_bands;
                true
            } else {
                false
            }
        })
    }

    /// Cycles histogram bin count (`16 -> 24 -> 32 -> 48 -> 16`) on a selected unlocked `FixedRangeVolumeProfile` (`Milestone R11`).
    pub fn cycle_selected_frvp_rows(&mut self) -> bool {
        self.mutate_selected(|d| {
            if d.locked {
                return false;
            }
            if let DrawingGeometry::FixedRangeVolumeProfile { row_count, .. } = &mut d.geometry {
                *row_count = match *row_count {
                    0..=19 => 24,
                    20..=28 => 32,
                    29..=40 => 48,
                    _ => 16,
                };
                true
            } else {
                false
            }
        })
    }

    /// Commits a validated transactional [`DrawingPropertyPatch`] from the Interactive Drawing Property
    /// Inspector (`Milestone R15`) as **at most one** [`DrawingCommand::Modify`] on `undo_stack`.
    ///
    /// - Returns `Ok(false)` (leaving `redo_stack` untouched) when the sanitized target state equals `before`.
    /// - Rejects style/geometry/visibility edits when `before.locked && patch.locked`.
    /// - Preserves sub-bar `DrawingAnchor::timestamp` precision when `bar_index` is unchanged, and
    ///   re-stamps `timestamp` against `candles` (`None` in right-offset whitespace) when `bar_index` moves.
    pub fn apply_drawing_property_patch(
        &mut self,
        id: DrawingId,
        patch: DrawingPropertyPatch,
        candles: &[Candle],
        tf: Timeframe,
    ) -> Result<bool, DrawingPatchError> {
        self.rollback_active_phase();
        let cutoff = self.replay_cutoff_timestamp;
        let vis_cnt = self.causal_visible_candles;
        let Some(slot_idx) = self.drawings.iter().position(|d| d.id == id) else {
            return Err(DrawingPatchError::DrawingNotFound);
        };
        if !self.drawings[slot_idx].is_causally_eligible(cutoff, vis_cnt) {
            return Err(DrawingPatchError::CausallyMasked);
        }

        let before = self.drawings[slot_idx].clone();
        if patch.geometry.tool_kind() != before.geometry.tool_kind() {
            return Err(DrawingPatchError::ToolKindMismatch);
        }
        if !patch.style.line_width.is_finite()
            || !(1.0..=8.0).contains(&patch.style.line_width)
            || !patch.style.fill_opacity.is_finite()
            || !(0.0..=0.65).contains(&patch.style.fill_opacity)
        {
            return Err(DrawingPatchError::InvalidStyle);
        }

        let mut next_geom = patch.geometry;
        if let DrawingGeometry::TextNote { text, .. } = &mut next_geom {
            let cleaned = super::sanitize_text_note_content(text);
            if cleaned.is_empty() {
                return Err(DrawingPatchError::InvalidGeometry);
            }
            *text = cleaned;
        }
        sync_patched_geometry_timestamps(&before.geometry, &mut next_geom, candles, tf);
        if !next_geom.is_valid() {
            return Err(DrawingPatchError::InvalidGeometry);
        }

        let next_ext_l =
            matches!(next_geom, DrawingGeometry::TrendLine { .. }) && patch.trend_extend_left;
        let next_ext_r =
            matches!(next_geom, DrawingGeometry::TrendLine { .. }) && patch.trend_extend_right;
        let next_custom_fib = if matches!(next_geom, DrawingGeometry::FibonacciRetracement { .. }) {
            match patch.custom_fib_levels {
                None => None,
                Some(ref raw) => {
                    let Some(sanitized) = super::sanitize_custom_fib_levels(raw) else {
                        return Err(DrawingPatchError::InvalidGeometry);
                    };
                    Some(sanitized)
                }
            }
        } else {
            None
        };

        let mut next_style = patch.style;
        next_style.sanitize();

        if before.locked
            && patch.locked
            && (patch.visible != before.visible
                || next_style != before.style
                || next_geom != before.geometry
                || next_ext_l != before.trend_extend_left
                || next_ext_r != before.trend_extend_right
                || next_custom_fib != before.custom_fib_levels)
        {
            return Err(DrawingPatchError::DrawingLocked);
        }

        let mut after = before.clone();
        after.visible = patch.visible;
        after.locked = patch.locked;
        after.style = next_style;
        after.geometry = next_geom;
        after.trend_extend_left = next_ext_l;
        after.trend_extend_right = next_ext_r;
        after.custom_fib_levels = next_custom_fib;
        if let Some(cut_ts) = cutoff
            && after.created_at_timestamp.is_none()
            && !after.is_causally_eligible(Some(cut_ts), vis_cnt)
        {
            after.created_at_timestamp = Some(cut_ts);
        }
        if !after.is_valid() {
            return Err(DrawingPatchError::InvalidGeometry);
        }

        if after == before {
            return Ok(false);
        }
        self.drawings[slot_idx] = after.clone();
        self.push_undo(DrawingCommand::Modify { before, after });
        Ok(true)
    }

    pub fn undo(&mut self) -> bool {
        self.rollback_active_phase();
        let Some(cmd) = self.undo_stack.pop() else {
            return false;
        };
        match &cmd {
            DrawingCommand::Create(created) => {
                self.drawings.retain(|d| d.id != created.id);
                if self.selected_id == Some(created.id) {
                    self.selected_id = None;
                }
            }
            DrawingCommand::Modify { before, .. } => {
                if let Some(slot) = self.drawings.iter_mut().find(|d| d.id == before.id) {
                    *slot = before.clone();
                }
            }
            DrawingCommand::Delete { index, drawing } => {
                let insert_at = (*index).min(self.drawings.len());
                self.drawings.insert(insert_at, drawing.clone());
                self.selected_id = Some(drawing.id);
            }
            DrawingCommand::ReplaceAll {
                before,
                before_selected,
                ..
            } => {
                self.drawings.clone_from(before);
                self.selected_id =
                    (*before_selected).filter(|&sel| self.drawings.iter().any(|d| d.id == sel));
            }
        }
        self.redo_stack.push(cmd);
        self.sanitize_causal_selection_and_hover();
        true
    }

    pub fn redo(&mut self) -> bool {
        self.rollback_active_phase();
        let Some(cmd) = self.redo_stack.pop() else {
            return false;
        };
        match &cmd {
            DrawingCommand::Create(created) => {
                if !self.drawings.iter().any(|d| d.id == created.id) {
                    self.drawings.push(created.clone());
                }
                self.selected_id = Some(created.id);
            }
            DrawingCommand::Modify { after, .. } => {
                if let Some(slot) = self.drawings.iter_mut().find(|d| d.id == after.id) {
                    *slot = after.clone();
                }
            }
            DrawingCommand::Delete { drawing, .. } => {
                self.drawings.retain(|d| d.id != drawing.id);
                if self.selected_id == Some(drawing.id) {
                    self.selected_id = None;
                }
            }
            DrawingCommand::ReplaceAll {
                after,
                after_selected,
                ..
            } => {
                self.drawings.clone_from(after);
                self.selected_id =
                    (*after_selected).filter(|&sel| self.drawings.iter().any(|d| d.id == sel));
            }
        }
        self.undo_stack.push(cmd);
        self.sanitize_causal_selection_and_hover();
        true
    }

    /// Constructs the transient rubber-band preview [`Drawing`] while a 1-click or 2-click placement is active.
    #[must_use]
    pub fn preview_drawing(&self) -> Option<Drawing> {
        match self.phase {
            DrawingInteractionPhase::PlacingSingle {
                tool,
                pane,
                preview_anchor,
            } => {
                let geom = match tool {
                    DrawingTool::AnchoredVwap => {
                        let mut clamped = preview_anchor;
                        if clamped.bar_index < 0.0 {
                            clamped.bar_index = 0.0;
                            clamped.timestamp = None;
                        }
                        DrawingGeometry::AnchoredVwap {
                            anchor: clamped,
                            show_bands: true,
                        }
                    }
                    DrawingTool::TextNote => DrawingGeometry::TextNote {
                        anchor: preview_anchor,
                        text: "Note".to_owned(),
                    },
                    _ => DrawingGeometry::HorizontalLine {
                        anchor: preview_anchor,
                    },
                };
                geom.is_valid().then(|| {
                    Drawing::new(
                        PREVIEW_DRAWING_ID,
                        pane,
                        geom,
                        DrawingStyle::default_for_tool(tool),
                    )
                })
            }
            DrawingInteractionPhase::PlacingTwoAnchor {
                tool,
                pane,
                first_anchor,
                preview_anchor,
                ..
            } => {
                let geom = build_two_anchor_geometry(tool, first_anchor, preview_anchor)?;
                geom.is_valid().then(|| {
                    Drawing::new(
                        PREVIEW_DRAWING_ID,
                        pane,
                        geom,
                        DrawingStyle::default_for_tool(tool),
                    )
                })
            }
            _ => None,
        }
    }

    /// Updates hover hit-test and rubber-band/drag state when the pointer moves inside `pane`.
    pub fn pointer_move(
        &mut self,
        pane: DrawingPaneTarget,
        local_pos: LocalPoint,
        snap_modifier: bool,
        proj: &PaneDrawingProjection<'_>,
        candles: &[Candle],
    ) {
        // Strict cross-pane isolation: while a placement or drag is active on `owner_pane`,
        // pointer motion over any OTHER pane must never run hover hit-testing or leak `hovered_hit`!
        let active_owner_pane = match self.phase {
            DrawingInteractionPhase::Idle => None,
            DrawingInteractionPhase::PlacingSingle { pane: p, .. }
            | DrawingInteractionPhase::PlacingTwoAnchor { pane: p, .. }
            | DrawingInteractionPhase::DraggingHandle { pane: p, .. }
            | DrawingInteractionPhase::DraggingBody { pane: p, .. } => Some(p),
        };
        if let Some(owner) = active_owner_pane
            && owner != pane
        {
            self.hovered_hit = None;
            return;
        }

        let effective_magnet = self.effective_magnet(pane, snap_modifier);

        match &mut self.phase {
            DrawingInteractionPhase::PlacingSingle { preview_anchor, .. } => {
                if let Some(anchor) =
                    resolve_anchor(pane, local_pos, effective_magnet, proj, candles)
                {
                    *preview_anchor = anchor;
                }
                self.hovered_hit = None;
            }
            DrawingInteractionPhase::PlacingTwoAnchor { preview_anchor, .. } => {
                if let Some(anchor) =
                    resolve_anchor(pane, local_pos, effective_magnet, proj, candles)
                {
                    *preview_anchor = anchor;
                }
                self.hovered_hit = None;
            }
            DrawingInteractionPhase::DraggingHandle {
                drawing_id,
                handle,
                start_local,
                grab_offset,
                drag_activated,
                original,
                ..
            } => {
                self.hovered_hit = None;
                if !*drag_activated {
                    if local_pos.distance(*start_local) < EDIT_DRAG_THRESHOLD_PX {
                        return;
                    }
                    *drag_activated = true;
                }
                let target_id = *drawing_id;
                let target_handle = *handle;
                let pure_vertical = (local_pos.x - start_local.x).abs() <= 1e-9
                    && effective_magnet == MagnetMode::Off;
                let target_local = if effective_magnet == MagnetMode::Ohlc {
                    local_pos
                } else {
                    LocalPoint::new(local_pos.x + grab_offset.x, local_pos.y + grab_offset.y)
                };
                if let Some(mut anchor) =
                    resolve_anchor(pane, target_local, effective_magnet, proj, candles)
                    && let Some(d) = self.drawings.iter_mut().find(|d| d.id == target_id)
                    && !d.locked
                {
                    if pure_vertical {
                        let [orig_a0, orig_a1] = original.geometry.anchors();
                        let src = match target_handle {
                            HandlePart::Anchor(0)
                            | HandlePart::RectCorner(0)
                            | HandlePart::RectCorner(3) => orig_a0,
                            HandlePart::RrEntryLeft => {
                                if orig_a0.bar_index <= orig_a1.bar_index {
                                    orig_a0
                                } else {
                                    orig_a1
                                }
                            }
                            HandlePart::RrEntryRight => {
                                if orig_a0.bar_index <= orig_a1.bar_index {
                                    orig_a1
                                } else {
                                    orig_a0
                                }
                            }
                            _ => orig_a1,
                        };
                        anchor.bar_index = src.bar_index;
                        anchor.timestamp = src.timestamp;
                    }
                    if matches!(original.geometry, DrawingGeometry::AnchoredVwap { .. })
                        && anchor.bar_index < 0.0
                    {
                        anchor.bar_index = 0.0;
                        anchor.timestamp = candles.first().map(|c| c.timestamp as f64);
                    }
                    let mut next_geom = original.geometry.clone();
                    apply_handle_drag(&mut next_geom, &original.geometry, target_handle, anchor);
                    clamp_avwap_geom_left(&mut next_geom, candles);
                    if next_geom.is_valid() {
                        d.geometry = next_geom;
                    }
                    if let Some(cutoff_ts) = self.replay_cutoff_timestamp {
                        d.created_at_timestamp = Some(
                            d.created_at_timestamp
                                .map_or(cutoff_ts, |t| t.max(cutoff_ts)),
                        );
                    }
                }
            }
            DrawingInteractionPhase::DraggingBody {
                drawing_id,
                start_local,
                original_anchors_local,
                drag_activated,
                original,
                ..
            } => {
                self.hovered_hit = None;
                if !*drag_activated {
                    if local_pos.distance(*start_local) < EDIT_DRAG_THRESHOLD_PX {
                        return;
                    }
                    *drag_activated = true;
                }
                let target_id = *drawing_id;
                let dx = local_pos.x - start_local.x;
                let dy = local_pos.y - start_local.y;
                if let Some(mut translated_geom) = translate_geometry_in_local_space(
                    &original.geometry,
                    original_anchors_local,
                    dx,
                    dy,
                    proj,
                ) && let Some(d) = self.drawings.iter_mut().find(|d| d.id == target_id)
                    && !d.locked
                {
                    clamp_avwap_geom_left(&mut translated_geom, candles);
                    d.geometry = translated_geom;
                    if let Some(cutoff_ts) = self.replay_cutoff_timestamp {
                        d.created_at_timestamp = Some(
                            d.created_at_timestamp
                                .map_or(cutoff_ts, |t| t.max(cutoff_ts)),
                        );
                    }
                }
            }
            DrawingInteractionPhase::Idle => {
                if self.active_tool == DrawingTool::Select {
                    let hit_proj = (*proj).with_candles(candles);
                    self.hovered_hit = hit_test_pane_drawings_causal(
                        &self.drawings,
                        pane,
                        &hit_proj,
                        local_pos,
                        self.selected_id,
                        self.replay_cutoff_timestamp,
                        candles.len(),
                    )
                    .map(|h| (pane, h));
                } else {
                    self.hovered_hit = None;
                }
            }
        }
    }

    pub fn clear_hover_for_pane(&mut self, pane: DrawingPaneTarget) {
        if self.hovered_hit.is_some_and(|(p, _)| p == pane) {
            self.hovered_hit = None;
        }
    }

    /// Processes primary pointer button press inside `pane`.
    pub fn pointer_down(
        &mut self,
        pane: DrawingPaneTarget,
        local_pos: LocalPoint,
        snap_modifier: bool,
        proj: &PaneDrawingProjection<'_>,
        candles: &[Candle],
    ) -> PointerCaptureOutcome {
        let effective_magnet = self.effective_magnet(pane, snap_modifier);

        // 1. Continue an active 2-click placement if waiting for the second click!
        if let DrawingInteractionPhase::PlacingTwoAnchor {
            pane: placing_pane,
            preview_anchor,
            wait_for_second_press,
            second_press_active,
            ..
        } = &mut self.phase
        {
            if *placing_pane == pane && *wait_for_second_press {
                if let Some(anchor) =
                    resolve_anchor(pane, local_pos, effective_magnet, proj, candles)
                {
                    *preview_anchor = anchor;
                    *second_press_active = true;
                }
                return PointerCaptureOutcome::CapturedByDrawing;
            }
            // Clicking a different pane while waiting for click #2 restarts placement cleanly on the new pane.
            self.phase = DrawingInteractionPhase::Idle;
        }

        // 2. Start a new drawing placement when a creation tool is active.
        match self.active_tool {
            tool @ (DrawingTool::HorizontalLine
            | DrawingTool::AnchoredVwap
            | DrawingTool::TextNote) => {
                if tool == DrawingTool::AnchoredVwap && pane != DrawingPaneTarget::Main {
                    return PointerCaptureOutcome::PassThroughToViewport;
                }
                if let Some(mut anchor) =
                    resolve_anchor(pane, local_pos, effective_magnet, proj, candles)
                {
                    if tool == DrawingTool::AnchoredVwap && anchor.bar_index < 0.0 {
                        anchor.bar_index = 0.0;
                        anchor.timestamp = candles.first().map(|c| c.timestamp as f64);
                    }
                    self.phase = DrawingInteractionPhase::PlacingSingle {
                        tool,
                        pane,
                        preview_anchor: anchor,
                    };
                }
                return PointerCaptureOutcome::CapturedByDrawing;
            }
            tool @ (DrawingTool::TrendLine
            | DrawingTool::Ray
            | DrawingTool::Rectangle
            | DrawingTool::FibonacciRetracement
            | DrawingTool::RiskRewardBox
            | DrawingTool::FixedRangeVolumeProfile) => {
                if matches!(
                    tool,
                    DrawingTool::RiskRewardBox | DrawingTool::FixedRangeVolumeProfile
                ) && pane != DrawingPaneTarget::Main
                {
                    return PointerCaptureOutcome::PassThroughToViewport;
                }
                if let Some(anchor) =
                    resolve_anchor(pane, local_pos, effective_magnet, proj, candles)
                {
                    self.phase = DrawingInteractionPhase::PlacingTwoAnchor {
                        tool,
                        pane,
                        first_anchor: anchor,
                        first_local: local_pos,
                        preview_anchor: anchor,
                        wait_for_second_press: false,
                        second_press_active: false,
                    };
                }
                return PointerCaptureOutcome::CapturedByDrawing;
            }
            DrawingTool::Select => {}
        }

        // 3. In `Select` cursor mode, hit-test drawings on `pane`.
        let hit_proj = (*proj).with_candles(candles);
        if let Some(hit) = hit_test_pane_drawings_causal(
            &self.drawings,
            pane,
            &hit_proj,
            local_pos,
            self.selected_id,
            self.replay_cutoff_timestamp,
            candles.len(),
        ) {
            self.selected_id = Some(hit.drawing_id);
            self.hovered_hit = Some((pane, hit));

            let Some(drawing) = self
                .drawings
                .iter()
                .find(|d| d.id == hit.drawing_id)
                .cloned()
            else {
                return PointerCaptureOutcome::PassThroughToViewport;
            };

            // Locked drawings select cleanly for Inspector unlock, while passing pointer gestures
            // through to the viewport so dragging across locked support lines or zones still pans the chart!
            if drawing.locked {
                return PointerCaptureOutcome::PassThroughToViewport;
            }

            match hit.target {
                HitTarget::Handle(handle) => {
                    let grab_offset = proj
                        .project_drawing(&drawing)
                        .and_then(|p| p.handles.into_iter().find(|h| h.part == handle))
                        .map_or(LocalPoint::new(0.0, 0.0), |h| {
                            LocalPoint::new(h.pos.x - local_pos.x, h.pos.y - local_pos.y)
                        });
                    self.phase = DrawingInteractionPhase::DraggingHandle {
                        drawing_id: drawing.id,
                        pane,
                        handle,
                        start_local: local_pos,
                        grab_offset,
                        drag_activated: false,
                        original: drawing,
                    };
                }
                HitTarget::Body => {
                    if let Some(projected) = proj.project_drawing(&drawing) {
                        self.phase = DrawingInteractionPhase::DraggingBody {
                            drawing_id: drawing.id,
                            pane,
                            start_local: local_pos,
                            original_anchors_local: projected.anchors_local,
                            drag_activated: false,
                            original: drawing,
                        };
                    }
                }
            }
            return PointerCaptureOutcome::CapturedByDrawing;
        }

        PointerCaptureOutcome::PassThroughToViewport
    }

    /// Processes primary pointer button release inside (or at the end of a drag from) `pane`.
    pub fn pointer_up(
        &mut self,
        pane: DrawingPaneTarget,
        local_pos: Option<LocalPoint>,
        snap_modifier: bool,
        proj: &PaneDrawingProjection<'_>,
        candles: &[Candle],
    ) -> bool {
        let effective_magnet = self.effective_magnet(pane, snap_modifier);

        let current_phase = std::mem::take(&mut self.phase);
        match current_phase {
            DrawingInteractionPhase::Idle => false,
            DrawingInteractionPhase::PlacingSingle {
                tool,
                pane: placing_pane,
                preview_anchor,
            } => {
                if placing_pane != pane {
                    self.phase = DrawingInteractionPhase::PlacingSingle {
                        tool,
                        pane: placing_pane,
                        preview_anchor,
                    };
                    return false;
                }
                let mut final_anchor = local_pos
                    .and_then(|pt| resolve_anchor(pane, pt, effective_magnet, proj, candles))
                    .unwrap_or(preview_anchor);
                let geom = match tool {
                    DrawingTool::AnchoredVwap => {
                        if final_anchor.bar_index < 0.0 {
                            final_anchor.bar_index = 0.0;
                            final_anchor.timestamp = candles.first().map(|c| c.timestamp as f64);
                        }
                        DrawingGeometry::AnchoredVwap {
                            anchor: final_anchor,
                            show_bands: true,
                        }
                    }
                    DrawingTool::TextNote => DrawingGeometry::TextNote {
                        anchor: final_anchor,
                        text: "Note".to_owned(),
                    },
                    _ => DrawingGeometry::HorizontalLine {
                        anchor: final_anchor,
                    },
                };
                if let Some(new_id) = self.add_drawing(pane, geom, None) {
                    self.selected_id = Some(new_id);
                    self.active_tool = DrawingTool::Select;
                    return true;
                }
                false
            }
            DrawingInteractionPhase::PlacingTwoAnchor {
                tool,
                pane: placing_pane,
                first_anchor,
                first_local,
                preview_anchor,
                wait_for_second_press,
                second_press_active,
            } => {
                if placing_pane != pane {
                    self.phase = DrawingInteractionPhase::PlacingTwoAnchor {
                        tool,
                        pane: placing_pane,
                        first_anchor,
                        first_local,
                        preview_anchor,
                        wait_for_second_press,
                        second_press_active,
                    };
                    return false;
                }
                let end_anchor = local_pos
                    .and_then(|pt| resolve_anchor(pane, pt, effective_magnet, proj, candles))
                    .unwrap_or(preview_anchor);
                let dragged_far_enough =
                    local_pos.is_some_and(|pt| pt.distance(first_local) >= DRAG_COMMIT_DISTANCE_PX);
                let should_try_commit = if wait_for_second_press {
                    second_press_active
                } else {
                    dragged_far_enough
                };

                if should_try_commit
                    && has_non_zero_direction(first_anchor, end_anchor)
                    && let Some(geom) = build_two_anchor_geometry(tool, first_anchor, end_anchor)
                    && let Some(new_id) = self.add_drawing(pane, geom, None)
                {
                    self.selected_id = Some(new_id);
                    self.active_tool = DrawingTool::Select;
                    return true;
                }

                // Stay in 2-click placement waiting for the user's second click!
                self.phase = DrawingInteractionPhase::PlacingTwoAnchor {
                    tool,
                    pane: placing_pane,
                    first_anchor,
                    first_local,
                    preview_anchor: end_anchor,
                    wait_for_second_press: true,
                    second_press_active: false,
                };
                false
            }
            DrawingInteractionPhase::DraggingHandle {
                drawing_id,
                drag_activated,
                original,
                ..
            }
            | DrawingInteractionPhase::DraggingBody {
                drawing_id,
                drag_activated,
                original,
                ..
            } => {
                if !drag_activated {
                    return false;
                }
                if let Some(after) = self.drawings.iter().find(|d| d.id == drawing_id).cloned()
                    && after != original
                {
                    self.push_undo(DrawingCommand::Modify {
                        before: original,
                        after,
                    });
                    return true;
                }
                false
            }
        }
    }

    /// Deselects when the user clicks empty canvas without dragging in `Select` mode.
    pub fn on_empty_canvas_click(&mut self) {
        if self.active_tool == DrawingTool::Select && self.phase == DrawingInteractionPhase::Idle {
            self.selected_id = None;
        }
    }

    fn effective_magnet(&self, pane: DrawingPaneTarget, snap_modifier: bool) -> MagnetMode {
        if pane != DrawingPaneTarget::Main {
            return MagnetMode::Off;
        }
        if snap_modifier {
            // Holding Ctrl/Cmd temporarily toggles magnet relative to toolbar setting!
            match self.magnet_mode {
                MagnetMode::Off => MagnetMode::Ohlc,
                MagnetMode::Ohlc => MagnetMode::Off,
            }
        } else {
            self.magnet_mode
        }
    }

    fn alloc_id(&mut self) -> DrawingId {
        if self.next_id >= COMPACT_ID_THRESHOLD {
            self.compact_ids();
        }
        let mut id = self.next_id.max(1);
        while self.drawings.iter().any(|d| d.id == id) || id == PREVIEW_DRAWING_ID {
            if id >= COMPACT_ID_THRESHOLD {
                self.compact_ids();
                id = self.next_id.max(1);
                break;
            }
            id += 1;
        }
        self.next_id = id.saturating_add(1);
        id
    }

    fn compact_ids(&mut self) {
        use std::collections::HashMap;

        let mut remap = HashMap::with_capacity(self.drawings.len());
        for (idx, d) in self.drawings.iter_mut().enumerate() {
            let new_id = (idx as DrawingId) + 1;
            remap.insert(d.id, new_id);
            d.id = new_id;
        }
        if let Some(sel) = self.selected_id {
            self.selected_id = remap.get(&sel).copied();
        }
        let remap_drawing = |d: &mut Drawing| {
            if let Some(&nid) = remap.get(&d.id) {
                d.id = nid;
            }
        };
        let remap_cmd = |cmd: &mut DrawingCommand| match cmd {
            DrawingCommand::Create(d) | DrawingCommand::Delete { drawing: d, .. } => {
                remap_drawing(d);
            }
            DrawingCommand::Modify { before, after } => {
                remap_drawing(before);
                remap_drawing(after);
            }
            DrawingCommand::ReplaceAll {
                before,
                after,
                before_selected,
                after_selected,
            } => {
                for d in before {
                    remap_drawing(d);
                }
                for d in after {
                    remap_drawing(d);
                }
                if let Some(s) = before_selected {
                    *before_selected = remap.get(s).copied();
                }
                if let Some(s) = after_selected {
                    *after_selected = remap.get(s).copied();
                }
            }
        };
        for cmd in &mut self.undo_stack {
            remap_cmd(cmd);
        }
        for cmd in &mut self.redo_stack {
            remap_cmd(cmd);
        }
        self.next_id = (self.drawings.len() as DrawingId) + 1;
    }

    fn push_undo(&mut self, cmd: DrawingCommand) {
        if self.undo_stack.len() >= MAX_UNDO_STEPS {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
    }

    fn mutate_selected(&mut self, f: impl FnOnce(&mut Drawing) -> bool) -> bool {
        let Some(sel_id) = self.selected_id else {
            return false;
        };
        self.rollback_active_phase();
        let cutoff = self.replay_cutoff_timestamp;
        let vis_cnt = self.causal_visible_candles;
        let Some(slot) = self.drawings.iter_mut().find(|d| d.id == sel_id) else {
            return false;
        };
        if !slot.is_causally_eligible(cutoff, vis_cnt) {
            self.selected_id = None;
            return false;
        }
        let before = slot.clone();
        if !f(slot) || *slot == before {
            return false;
        }
        let after = slot.clone();
        self.push_undo(DrawingCommand::Modify { before, after });
        true
    }

    fn rollback_active_phase(&mut self) {
        let prev = std::mem::take(&mut self.phase);
        match prev {
            DrawingInteractionPhase::DraggingHandle {
                drawing_id,
                original,
                ..
            }
            | DrawingInteractionPhase::DraggingBody {
                drawing_id,
                original,
                ..
            } => {
                if let Some(slot) = self.drawings.iter_mut().find(|d| d.id == drawing_id) {
                    *slot = original;
                }
            }
            DrawingInteractionPhase::Idle
            | DrawingInteractionPhase::PlacingSingle { .. }
            | DrawingInteractionPhase::PlacingTwoAnchor { .. } => {}
        }
    }
}

fn resolve_anchor(
    pane: DrawingPaneTarget,
    local_pos: LocalPoint,
    magnet: MagnetMode,
    proj: &PaneDrawingProjection<'_>,
    candles: &[Candle],
) -> Option<DrawingAnchor> {
    let raw = proj.local_to_anchor(local_pos)?;
    if pane == DrawingPaneTarget::Main && magnet == MagnetMode::Ohlc {
        Some(snap_to_visible_ohlc(
            raw,
            local_pos,
            candles,
            proj,
            magnet,
            MAGNET_THRESHOLD_PX,
        ))
    } else {
        Some(raw)
    }
}

fn build_two_anchor_geometry(
    tool: DrawingTool,
    a0: DrawingAnchor,
    a1: DrawingAnchor,
) -> Option<DrawingGeometry> {
    if !a0.is_valid() || !a1.is_valid() {
        return None;
    }
    match tool {
        DrawingTool::TrendLine => Some(DrawingGeometry::TrendLine { start: a0, end: a1 }),
        DrawingTool::Ray => has_non_zero_direction(a0, a1).then_some(DrawingGeometry::Ray {
            origin: a0,
            through: a1,
        }),
        DrawingTool::Rectangle => Some(DrawingGeometry::Rectangle {
            corner_a: a0,
            corner_b: a1,
        }),
        DrawingTool::FibonacciRetracement => Some(DrawingGeometry::FibonacciRetracement {
            start: a0,
            end: a1,
            direction: FibDirection::StartToEnd,
            extend_right: false,
        }),
        DrawingTool::RiskRewardBox => {
            if a0.value <= 0.0 || a1.value <= 0.0 {
                return None;
            }
            let entry = a0.value;
            let flat_eps = (entry * 1.0e-4).max(1.0e-6);
            let (end_anchor, stop_price) = if a1.value > entry + flat_eps {
                // Upward drag -> Long setup with default 2.0R stop
                let reward = a1.value - entry;
                let stop = (entry - 0.5 * reward)
                    .max(entry * 0.25)
                    .min(entry - flat_eps);
                (a1, stop)
            } else if a1.value < entry - flat_eps {
                // Downward drag -> Short setup with default 2.0R stop
                let target_val = a1.value.max(entry * 0.05);
                let reward = (entry - target_val).max(flat_eps);
                let stop = (entry + 0.5 * reward).min(super::MAX_ABS_DRAWING_VALUE);
                (
                    DrawingAnchor {
                        value: target_val,
                        ..a1
                    },
                    stop,
                )
            } else {
                // Pure horizontal drag -> clean Long 2.0R setup (+4% target / -2% stop)
                let target_val = (entry * 1.04).min(super::MAX_ABS_DRAWING_VALUE);
                let stop = (entry * 0.98).max(entry * 0.25);
                (
                    DrawingAnchor {
                        value: target_val,
                        ..a1
                    },
                    stop,
                )
            };
            let candidate = DrawingGeometry::RiskRewardBox {
                start: a0,
                end: end_anchor,
                stop_price,
            };
            candidate.is_valid().then_some(candidate)
        }
        DrawingTool::FixedRangeVolumeProfile => ((a1.bar_index - a0.bar_index).abs() >= 0.5)
            .then_some(DrawingGeometry::FixedRangeVolumeProfile {
                start: a0,
                end: a1,
                row_count: 24,
            }),
        DrawingTool::Select
        | DrawingTool::HorizontalLine
        | DrawingTool::AnchoredVwap
        | DrawingTool::TextNote => None,
    }
}

fn apply_handle_drag(
    geom: &mut DrawingGeometry,
    original_geom: &DrawingGeometry,
    handle: HandlePart,
    anchor: DrawingAnchor,
) {
    if !anchor.is_valid() {
        return;
    }
    match (geom, handle) {
        (geom, HandlePart::Anchor(idx)) => {
            let _ = geom.set_anchor(idx, anchor);
        }
        (DrawingGeometry::Rectangle { corner_a, corner_b }, HandlePart::RectCorner(corner_idx)) => {
            match corner_idx {
                0 => *corner_a = anchor,
                1 => {
                    corner_a.value = anchor.value;
                    corner_b.bar_index = anchor.bar_index;
                    corner_b.timestamp = anchor.timestamp;
                }
                2 => *corner_b = anchor,
                3 => {
                    corner_a.bar_index = anchor.bar_index;
                    corner_a.timestamp = anchor.timestamp;
                    corner_b.value = anchor.value;
                }
                _ => {}
            }
        }
        (
            DrawingGeometry::RiskRewardBox {
                start,
                end,
                stop_price,
            },
            HandlePart::RrEntryLeft
            | HandlePart::RrEntryRight
            | HandlePart::RrTarget
            | HandlePart::RrStop,
        ) => {
            let DrawingGeometry::RiskRewardBox {
                start: orig_start,
                end: orig_end,
                stop_price: orig_stop,
            } = original_geom
            else {
                return;
            };
            let Some(orig_side) =
                DrawingGeometry::risk_reward_side_of(orig_start.value, orig_end.value, *orig_stop)
            else {
                return;
            };
            let sep = (orig_start.value.abs() * 1.0e-4).max(1.0e-6);
            let min_pos = (orig_start.value * 1.0e-4).max(1.0e-6);
            let orig_risk_dist = (orig_start.value - *orig_stop).abs().max(sep);

            match handle {
                HandlePart::RrTarget => {
                    if anchor.value >= orig_start.value {
                        end.value = anchor
                            .value
                            .clamp(orig_start.value + sep, super::MAX_ABS_DRAWING_VALUE);
                        *stop_price = if orig_side == crate::trading::PositionSide::Long {
                            *orig_stop
                        } else {
                            let max_long_sl = (orig_start.value - sep).max(min_pos);
                            (orig_start.value - orig_risk_dist).clamp(min_pos, max_long_sl)
                        };
                    } else {
                        let max_short_tp = (orig_start.value - sep).max(min_pos);
                        end.value = anchor.value.clamp(min_pos, max_short_tp);
                        *stop_price = if orig_side == crate::trading::PositionSide::Short {
                            *orig_stop
                        } else {
                            (orig_start.value + orig_risk_dist)
                                .clamp(orig_start.value + sep, super::MAX_ABS_DRAWING_VALUE)
                        };
                    }
                }
                HandlePart::RrStop => match orig_side {
                    crate::trading::PositionSide::Long => {
                        let max_long_sl = (orig_start.value - sep).max(min_pos);
                        *stop_price = anchor.value.clamp(min_pos, max_long_sl);
                    }
                    crate::trading::PositionSide::Short => {
                        *stop_price = anchor
                            .value
                            .clamp(orig_start.value + sep, super::MAX_ABS_DRAWING_VALUE);
                    }
                },
                HandlePart::RrEntryLeft | HandlePart::RrEntryRight => {
                    let is_left = handle == HandlePart::RrEntryLeft;
                    let start_is_left = orig_start.bar_index <= orig_end.bar_index;
                    if is_left == start_is_left {
                        start.bar_index = anchor.bar_index;
                        start.timestamp = anchor.timestamp;
                    } else {
                        end.bar_index = anchor.bar_index;
                        end.timestamp = anchor.timestamp;
                    }
                    let (lo_bound, hi_bound) = match orig_side {
                        crate::trading::PositionSide::Long => {
                            (*orig_stop + sep, orig_end.value - sep)
                        }
                        crate::trading::PositionSide::Short => {
                            (orig_end.value + sep, *orig_stop - sep)
                        }
                    };
                    if lo_bound > 0.0 && lo_bound <= hi_bound {
                        start.value = anchor.value.clamp(lo_bound, hi_bound);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn sync_patched_anchor_timestamp(
    prev: DrawingAnchor,
    next: &mut DrawingAnchor,
    candles: &[Candle],
    tf: Timeframe,
) {
    if !next.bar_index.is_finite() {
        return;
    }
    if (next.bar_index - prev.bar_index).abs() <= 1.0e-9 {
        // Bar index was not edited -> preserve existing sub-bar UTC timestamp precision across TF hops!
        if next.timestamp.is_none() {
            next.timestamp = prev.timestamp;
        }
        return;
    }
    if candles.is_empty() {
        next.timestamp = None;
        return;
    }
    let last_idx = (candles.len() - 1) as f64;
    if next.bar_index <= last_idx {
        next.timestamp = resolve_anchor_timestamp(next.bar_index, candles, tf)
            .filter(|ts| ts.is_finite() && ts.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP);
    } else {
        // Right-offset future whitespace (`bar_index > last_idx`) remains unstamped until bars materialize.
        next.timestamp = None;
    }
}

fn sync_patched_geometry_timestamps(
    before: &DrawingGeometry,
    after: &mut DrawingGeometry,
    candles: &[Candle],
    tf: Timeframe,
) {
    let prev_anchors = before.anchors();
    match after {
        DrawingGeometry::HorizontalLine { anchor }
        | DrawingGeometry::AnchoredVwap { anchor, .. }
        | DrawingGeometry::TextNote { anchor, .. } => {
            sync_patched_anchor_timestamp(prev_anchors[0], anchor, candles, tf);
        }
        DrawingGeometry::TrendLine { start, end }
        | DrawingGeometry::Ray {
            origin: start,
            through: end,
        }
        | DrawingGeometry::Rectangle {
            corner_a: start,
            corner_b: end,
        }
        | DrawingGeometry::FibonacciRetracement { start, end, .. }
        | DrawingGeometry::RiskRewardBox { start, end, .. }
        | DrawingGeometry::FixedRangeVolumeProfile { start, end, .. } => {
            sync_patched_anchor_timestamp(prev_anchors[0], start, candles, tf);
            sync_patched_anchor_timestamp(prev_anchors[1], end, candles, tf);
        }
    }
}

fn remap_single_anchor(
    anchor: &mut DrawingAnchor,
    old_candles: &[Candle],
    old_tf: Timeframe,
    new_candles: &[Candle],
    new_tf: Timeframe,
) {
    let existing_ts = anchor
        .timestamp
        .filter(|t| t.is_finite() && t.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP);
    let old_last = (old_candles.len().saturating_sub(1)) as f64;
    if existing_ts.is_none()
        && anchor.bar_index > old_last
        && old_tf == new_tf
        && old_candles.first().map(|c| c.timestamp) == new_candles.first().map(|c| c.timestamp)
    {
        // Keep unmaterialized right-offset whitespace projections (`bar_index > old_last`) unstamped
        // on the same timeframe so session/overnight gaps are never baked into synthetic timestamps.
        return;
    }

    let ts =
        existing_ts.or_else(|| resolve_anchor_timestamp(anchor.bar_index, old_candles, old_tf));
    if let Some(resolved_ts) = ts
        && resolved_ts.is_finite()
        && resolved_ts.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP
        && let Some((new_idx, snapped_ts)) =
            remap_timestamp_to_bar_index(resolved_ts, old_tf, new_candles, new_tf)
    {
        anchor.bar_index = new_idx;
        let new_last = (new_candles.len().saturating_sub(1)) as f64;
        if existing_ts.is_some() || new_idx <= new_last {
            if snapped_ts.is_finite() && snapped_ts.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP {
                anchor.timestamp = Some(snapped_ts);
            }
        } else {
            anchor.timestamp = None;
        }
    }
}

fn remap_drawing_geometry(
    geom: &mut DrawingGeometry,
    old_candles: &[Candle],
    old_tf: Timeframe,
    new_candles: &[Candle],
    new_tf: Timeframe,
) {
    match geom {
        DrawingGeometry::HorizontalLine { anchor }
        | DrawingGeometry::AnchoredVwap { anchor, .. }
        | DrawingGeometry::TextNote { anchor, .. } => {
            remap_single_anchor(anchor, old_candles, old_tf, new_candles, new_tf);
        }
        DrawingGeometry::TrendLine { start, end }
        | DrawingGeometry::Rectangle {
            corner_a: start,
            corner_b: end,
        }
        | DrawingGeometry::FibonacciRetracement { start, end, .. }
        | DrawingGeometry::RiskRewardBox { start, end, .. }
        | DrawingGeometry::FixedRangeVolumeProfile { start, end, .. } => {
            remap_single_anchor(start, old_candles, old_tf, new_candles, new_tf);
            remap_single_anchor(end, old_candles, old_tf, new_candles, new_tf);
        }
        DrawingGeometry::Ray { origin, through } => {
            let was_non_vertical = (through.bar_index - origin.bar_index).abs() > 1e-9;
            let orig_dir_sign = if through.bar_index >= origin.bar_index {
                1.0
            } else {
                -1.0
            };
            remap_single_anchor(origin, old_candles, old_tf, new_candles, new_tf);
            remap_single_anchor(through, old_candles, old_tf, new_candles, new_tf);
            let collapsed_horiz =
                was_non_vertical && (through.bar_index - origin.bar_index).abs() <= 1e-9;
            if collapsed_horiz || !has_non_zero_direction(*origin, *through) {
                let candidate = through.bar_index + orig_dir_sign * 1e-3;
                if candidate.abs() <= super::MAX_ABS_BAR_INDEX {
                    through.bar_index = candidate;
                    if through.timestamp.is_none() {
                        through.timestamp =
                            resolve_anchor_timestamp(through.bar_index, new_candles, new_tf)
                                .filter(|t| {
                                    t.is_finite() && t.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP
                                });
                    }
                } else {
                    origin.bar_index = (through.bar_index - orig_dir_sign * 1e-3)
                        .clamp(-super::MAX_ABS_BAR_INDEX, super::MAX_ABS_BAR_INDEX);
                    if origin.timestamp.is_none() {
                        origin.timestamp =
                            resolve_anchor_timestamp(origin.bar_index, new_candles, new_tf).filter(
                                |t| t.is_finite() && t.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP,
                            );
                    }
                }
            }
        }
    }
}

fn remap_command_anchors(
    cmd: &mut DrawingCommand,
    old_candles: &[Candle],
    old_tf: Timeframe,
    new_candles: &[Candle],
    new_tf: Timeframe,
) {
    match cmd {
        DrawingCommand::Create(d) | DrawingCommand::Delete { drawing: d, .. } => {
            remap_drawing_geometry(&mut d.geometry, old_candles, old_tf, new_candles, new_tf);
        }
        DrawingCommand::Modify { before, after } => {
            remap_drawing_geometry(
                &mut before.geometry,
                old_candles,
                old_tf,
                new_candles,
                new_tf,
            );
            remap_drawing_geometry(
                &mut after.geometry,
                old_candles,
                old_tf,
                new_candles,
                new_tf,
            );
        }
        DrawingCommand::ReplaceAll { before, after, .. } => {
            for d in before {
                remap_drawing_geometry(&mut d.geometry, old_candles, old_tf, new_candles, new_tf);
            }
            for d in after {
                remap_drawing_geometry(&mut d.geometry, old_candles, old_tf, new_candles, new_tf);
            }
        }
    }
}

fn translate_single_anchor_local(
    orig_anchor: DrawingAnchor,
    orig_pt: LocalPoint,
    dx: f64,
    dy: f64,
    proj: &PaneDrawingProjection<'_>,
) -> Option<DrawingAnchor> {
    let moved = proj.local_to_anchor(LocalPoint::new(orig_pt.x + dx, orig_pt.y + dy))?;
    // Preserve exact sub-bar `bar_index` and cached UTC `timestamp` during pure vertical drags (`dx == 0`).
    let next_anchor = if dx.abs() <= 1e-9 {
        DrawingAnchor {
            bar_index: orig_anchor.bar_index,
            value: moved.value,
            timestamp: orig_anchor.timestamp,
        }
    } else {
        DrawingAnchor::new(moved.bar_index, moved.value)
    };
    next_anchor.is_valid().then_some(next_anchor)
}

#[inline]
fn clamp_avwap_geom_left(geom: &mut DrawingGeometry, candles: &[Candle]) {
    if let DrawingGeometry::AnchoredVwap { anchor, .. } = geom
        && anchor.bar_index < 0.0
    {
        anchor.bar_index = 0.0;
        anchor.timestamp = candles.first().map(|c| c.timestamp as f64);
    }
}

fn translate_geometry_in_local_space(
    original: &DrawingGeometry,
    original_local: &[LocalPoint],
    dx: f64,
    dy: f64,
    proj: &PaneDrawingProjection<'_>,
) -> Option<DrawingGeometry> {
    if !dx.is_finite() || !dy.is_finite() {
        return None;
    }
    match *original {
        DrawingGeometry::HorizontalLine { anchor } => {
            let orig_pt = *original_local.first()?;
            let next_anchor = translate_single_anchor_local(anchor, orig_pt, dx, dy, proj)?;
            Some(DrawingGeometry::HorizontalLine {
                anchor: next_anchor,
            })
        }
        DrawingGeometry::TrendLine { start, end } => {
            if original_local.len() < 2 {
                return None;
            }
            let a0 = translate_single_anchor_local(start, original_local[0], dx, dy, proj)?;
            let a1 = translate_single_anchor_local(end, original_local[1], dx, dy, proj)?;
            Some(DrawingGeometry::TrendLine { start: a0, end: a1 })
        }
        DrawingGeometry::Ray { origin, through } => {
            if original_local.len() < 2 {
                return None;
            }
            let a0 = translate_single_anchor_local(origin, original_local[0], dx, dy, proj)?;
            let a1 = translate_single_anchor_local(through, original_local[1], dx, dy, proj)?;
            has_non_zero_direction(a0, a1).then_some(DrawingGeometry::Ray {
                origin: a0,
                through: a1,
            })
        }
        DrawingGeometry::Rectangle { corner_a, corner_b } => {
            if original_local.len() < 2 {
                return None;
            }
            let a0 = translate_single_anchor_local(corner_a, original_local[0], dx, dy, proj)?;
            let a1 = translate_single_anchor_local(corner_b, original_local[1], dx, dy, proj)?;
            Some(DrawingGeometry::Rectangle {
                corner_a: a0,
                corner_b: a1,
            })
        }
        DrawingGeometry::FibonacciRetracement {
            start,
            end,
            direction,
            extend_right,
        } => {
            if original_local.len() < 2 {
                return None;
            }
            let a0 = translate_single_anchor_local(start, original_local[0], dx, dy, proj)?;
            let a1 = translate_single_anchor_local(end, original_local[1], dx, dy, proj)?;
            Some(DrawingGeometry::FibonacciRetracement {
                start: a0,
                end: a1,
                direction,
                extend_right,
            })
        }
        DrawingGeometry::RiskRewardBox { start, end, .. } => {
            if original_local.len() < 3 {
                return None;
            }
            let a0 = translate_single_anchor_local(start, original_local[0], dx, dy, proj)?;
            let a1 = translate_single_anchor_local(end, original_local[1], dx, dy, proj)?;
            let stop_moved = proj.local_to_anchor(LocalPoint::new(
                original_local[2].x + dx,
                original_local[2].y + dy,
            ))?;
            let candidate = DrawingGeometry::RiskRewardBox {
                start: a0,
                end: a1,
                stop_price: stop_moved.value,
            };
            candidate.is_valid().then_some(candidate)
        }
        DrawingGeometry::FixedRangeVolumeProfile {
            start,
            end,
            row_count,
        } => {
            if original_local.len() < 2 {
                return None;
            }
            let a0 = translate_single_anchor_local(start, original_local[0], dx, dy, proj)?;
            let a1 = translate_single_anchor_local(end, original_local[1], dx, dy, proj)?;
            let candidate = DrawingGeometry::FixedRangeVolumeProfile {
                start: a0,
                end: a1,
                row_count,
            };
            candidate.is_valid().then_some(candidate)
        }
        DrawingGeometry::AnchoredVwap { anchor, show_bands } => {
            let orig_pt = *original_local.first()?;
            let mut next_anchor = translate_single_anchor_local(anchor, orig_pt, dx, dy, proj)?;
            if next_anchor.bar_index < 0.0 {
                next_anchor.bar_index = 0.0;
            }
            let candidate = DrawingGeometry::AnchoredVwap {
                anchor: next_anchor,
                show_bands,
            };
            candidate.is_valid().then_some(candidate)
        }
        DrawingGeometry::TextNote { anchor, ref text } => {
            let orig_pt = *original_local.first()?;
            let next_anchor = translate_single_anchor_local(anchor, orig_pt, dx, dy, proj)?;
            let candidate = DrawingGeometry::TextNote {
                anchor: next_anchor,
                text: text.clone(),
            };
            candidate.is_valid().then_some(candidate)
        }
    }
}
