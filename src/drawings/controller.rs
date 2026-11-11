use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::data::{Timeframe, remap_timestamp_to_bar_index, resolve_anchor_timestamp};
use crate::model::Candle;

use super::geometry::{HandlePart, LocalPoint, PaneDrawingProjection};
use super::hit_test::{
    HitMatch, HitTarget, MAGNET_THRESHOLD_PX, hit_test_pane_drawings, snap_to_visible_ohlc,
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
        }
    }

    /// Sanitizes state loaded from `serde` storage (`eframe` persistence), dropping invalid/duplicate entries
    /// and compacting near-`u64::MAX` identifiers so `alloc_id` never collides with `PREVIEW_DRAWING_ID`.
    pub fn sanitize_deserialized(&mut self) {
        self.phase = DrawingInteractionPhase::Idle;
        self.hovered_hit = None;
        self.undo_stack.clear();
        self.redo_stack.clear();

        if self.drawings.len() > MAX_PERSISTED_DRAWINGS {
            self.drawings.truncate(MAX_PERSISTED_DRAWINGS);
        }

        let mut seen = HashSet::with_capacity(self.drawings.len());
        let mut max_id: DrawingId = 0;
        self.drawings.retain_mut(|d| {
            d.style.sanitize();
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
    pub fn has_unlocked_drawings(&self) -> bool {
        self.drawings.iter().any(|d| !d.locked)
    }

    #[must_use]
    pub fn selected_drawing(&self) -> Option<&Drawing> {
        let id = self.selected_id?;
        self.drawings.iter().find(|d| d.id == id)
    }

    #[inline]
    #[must_use]
    pub fn is_capturing_pane_gesture(&self, pane: DrawingPaneTarget) -> bool {
        if self.active_tool != DrawingTool::Select {
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
        if !geometry.is_valid() {
            return None;
        }
        let id = self.alloc_id();
        let resolved_style =
            style.unwrap_or_else(|| DrawingStyle::default_for_tool(geometry.tool_kind()));
        let drawing = Drawing::new(id, pane, geometry, resolved_style);
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
            if !d.geometry.is_valid() {
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
    }

    /// Deletes the currently selected drawing if it exists and is unlocked.
    pub fn delete_selected(&mut self) -> bool {
        let Some(sel_id) = self.selected_id else {
            return false;
        };
        self.rollback_active_phase();
        let Some(idx) = self.drawings.iter().position(|d| d.id == sel_id) else {
            self.selected_id = None;
            return false;
        };
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

    /// Removes all unlocked drawings across all panes (preserving locked annotations).
    pub fn clear_unlocked(&mut self) -> usize {
        self.rollback_active_phase();
        let before = self.drawings.clone();
        let before_selected = self.selected_id;
        self.drawings.retain(|d| d.locked);
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
        true
    }

    /// Constructs the transient rubber-band preview [`Drawing`] while a 1-click or 2-click placement is active.
    #[must_use]
    pub fn preview_drawing(&self) -> Option<Drawing> {
        match self.phase {
            DrawingInteractionPhase::PlacingSingle {
                pane,
                preview_anchor,
            } => {
                let geom = DrawingGeometry::HorizontalLine {
                    anchor: preview_anchor,
                };
                geom.is_valid().then(|| {
                    Drawing::new(
                        PREVIEW_DRAWING_ID,
                        pane,
                        geom,
                        DrawingStyle::default_for_tool(DrawingTool::HorizontalLine),
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
                            _ => orig_a1,
                        };
                        anchor.bar_index = src.bar_index;
                        anchor.timestamp = src.timestamp;
                    }
                    apply_handle_drag(&mut d.geometry, target_handle, anchor);
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
                if let Some(translated_geom) = translate_geometry_in_local_space(
                    &original.geometry,
                    original_anchors_local,
                    dx,
                    dy,
                    proj,
                ) && let Some(d) = self.drawings.iter_mut().find(|d| d.id == target_id)
                    && !d.locked
                {
                    d.geometry = translated_geom;
                }
            }
            DrawingInteractionPhase::Idle => {
                if self.active_tool == DrawingTool::Select {
                    self.hovered_hit = hit_test_pane_drawings(
                        &self.drawings,
                        pane,
                        proj,
                        local_pos,
                        self.selected_id,
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
            DrawingTool::HorizontalLine => {
                if let Some(anchor) =
                    resolve_anchor(pane, local_pos, effective_magnet, proj, candles)
                {
                    self.phase = DrawingInteractionPhase::PlacingSingle {
                        pane,
                        preview_anchor: anchor,
                    };
                }
                return PointerCaptureOutcome::CapturedByDrawing;
            }
            tool @ (DrawingTool::TrendLine
            | DrawingTool::Ray
            | DrawingTool::Rectangle
            | DrawingTool::FibonacciRetracement) => {
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
        if let Some(hit) =
            hit_test_pane_drawings(&self.drawings, pane, proj, local_pos, self.selected_id)
        {
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
                pane: placing_pane,
                preview_anchor,
            } => {
                if placing_pane != pane {
                    self.phase = DrawingInteractionPhase::PlacingSingle {
                        pane: placing_pane,
                        preview_anchor,
                    };
                    return false;
                }
                let final_anchor = local_pos
                    .and_then(|pt| resolve_anchor(pane, pt, effective_magnet, proj, candles))
                    .unwrap_or(preview_anchor);
                let geom = DrawingGeometry::HorizontalLine {
                    anchor: final_anchor,
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
        let Some(slot) = self.drawings.iter_mut().find(|d| d.id == sel_id) else {
            return false;
        };
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
        DrawingTool::Select | DrawingTool::HorizontalLine => None,
    }
}

fn apply_handle_drag(geom: &mut DrawingGeometry, handle: HandlePart, anchor: DrawingAnchor) {
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
        _ => {}
    }
}

fn remap_single_anchor(
    anchor: &mut DrawingAnchor,
    old_candles: &[Candle],
    old_tf: Timeframe,
    new_candles: &[Candle],
    new_tf: Timeframe,
) {
    let ts = anchor
        .timestamp
        .filter(|t| t.is_finite() && t.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP)
        .or_else(|| resolve_anchor_timestamp(anchor.bar_index, old_candles, old_tf));
    if let Some(resolved_ts) = ts
        && resolved_ts.is_finite()
        && resolved_ts.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP
        && let Some((new_idx, snapped_ts)) =
            remap_timestamp_to_bar_index(resolved_ts, old_tf, new_candles, new_tf)
    {
        anchor.bar_index = new_idx;
        if snapped_ts.is_finite() && snapped_ts.abs() <= super::MAX_ABS_ANCHOR_TIMESTAMP {
            anchor.timestamp = Some(snapped_ts);
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
        DrawingGeometry::HorizontalLine { anchor } => {
            remap_single_anchor(anchor, old_candles, old_tf, new_candles, new_tf);
        }
        DrawingGeometry::TrendLine { start, end }
        | DrawingGeometry::Rectangle {
            corner_a: start,
            corner_b: end,
        }
        | DrawingGeometry::FibonacciRetracement { start, end, .. } => {
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
    }
}
