use crate::model::Candle;

use super::geometry::{
    HandlePart, LocalPoint, LocalSegment, PaneDrawingProjection, ProjectedDrawing,
    ProjectedDrawingShape, distance_to_segment,
};
use super::{Drawing, DrawingAnchor, DrawingId, MagnetMode};

/// Handle hit-test radius for the currently selected drawing (`px`).
pub const SELECTED_HANDLE_HIT_RADIUS_PX: f64 = 8.0;

/// Handle hit-test radius for unselected unlocked drawings (`px`).
pub const HANDLE_HIT_RADIUS_PX: f64 = 7.5;

/// Stroke/edge hit-test tolerance (`px`).
pub const STROKE_HIT_TOLERANCE_PX: f64 = 6.0;

/// Default OHLC magnet snap radius (`px`).
pub const MAGNET_THRESHOLD_PX: f64 = 12.0;

/// LWC-aligned hit-test precedence (`ipane-primitive.ts:94-103`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HitPriority {
    /// Interior of an already-selected filled shape (`Rectangle`).
    SelectedBody = 0,
    /// Line stroke, ray, horizontal level, border edge, or Fibonacci control/level line.
    Stroke = 1,
    /// Point/corner handle of an unselected unlocked drawing.
    Handle = 2,
    /// Point/corner handle of the currently selected unlocked drawing.
    SelectedHandle = 3,
}

/// Specific part of a drawing intersected by the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitTarget {
    Handle(HandlePart),
    Body,
}

/// Winning hit-test match on a pane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitMatch {
    pub drawing_id: DrawingId,
    pub locked: bool,
    pub target: HitTarget,
    pub priority: HitPriority,
    pub distance: f64,
    pub z_order: usize,
}

/// Hit-tests a single [`ProjectedDrawing`] against pane-local `cursor`.
#[must_use]
pub fn hit_test_projected_drawing(
    projected: &ProjectedDrawing,
    cursor: LocalPoint,
    is_selected: bool,
    z_order: usize,
) -> Option<HitMatch> {
    if !cursor.is_finite() {
        return None;
    }

    // 1. Point/corner handles take top precedence on unlocked drawings.
    if !projected.locked {
        let radius = if is_selected {
            SELECTED_HANDLE_HIT_RADIUS_PX
        } else {
            HANDLE_HIT_RADIUS_PX
        };
        let priority = if is_selected {
            HitPriority::SelectedHandle
        } else {
            HitPriority::Handle
        };
        let mut best_handle: Option<(HandlePart, f64)> = None;
        for handle in &projected.handles {
            let d = cursor.distance(handle.pos);
            if d <= radius && best_handle.is_none_or(|(_, prev_d)| d <= prev_d) {
                // Later handles win exact ties (`drawingGeometry.ts:45, 50`).
                best_handle = Some((handle.part, d));
            }
        }
        if let Some((part, distance)) = best_handle {
            return Some(HitMatch {
                drawing_id: projected.id,
                locked: false,
                target: HitTarget::Handle(part),
                priority,
                distance,
                z_order,
            });
        }
    }

    // 2. Stroke / segment / border / level hit-testing (`HitPriority::Stroke`).
    match &projected.shape {
        ProjectedDrawingShape::HorizontalLine {
            visible_segment, ..
        } => {
            let seg = (*visible_segment)?;
            let dist = (cursor.y - seg.start.y).abs();
            (cursor.x >= seg.start.x - STROKE_HIT_TOLERANCE_PX
                && cursor.x <= seg.end.x + STROKE_HIT_TOLERANCE_PX
                && dist <= STROKE_HIT_TOLERANCE_PX)
                .then_some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::Stroke,
                    distance: dist,
                    z_order,
                })
        }
        ProjectedDrawingShape::Segment { clipped, .. } => {
            let seg = (*clipped)?;
            let dist = distance_to_segment(cursor, seg);
            (dist <= STROKE_HIT_TOLERANCE_PX).then_some(HitMatch {
                drawing_id: projected.id,
                locked: projected.locked,
                target: HitTarget::Body,
                priority: HitPriority::Stroke,
                distance: dist,
                z_order,
            })
        }
        ProjectedDrawingShape::Rectangle {
            clipped_fill,
            clipped_edges,
            ..
        } => {
            let mut min_edge_dist = f64::INFINITY;
            for edge in clipped_edges.iter().flatten() {
                min_edge_dist = min_edge_dist.min(distance_to_segment(cursor, *edge));
            }
            if min_edge_dist <= STROKE_HIT_TOLERANCE_PX {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::Stroke,
                    distance: min_edge_dist,
                    z_order,
                });
            }
            // Interior body hit applies ONLY when already selected AND unlocked so locked or unselected
            // rectangles never hijack chart pan or block click-to-deselect inside the box!
            if is_selected
                && !projected.locked
                && let Some(fill) = clipped_fill
                && fill.contains(cursor)
            {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::SelectedBody,
                    distance: 0.0,
                    z_order,
                });
            }
            None
        }
        ProjectedDrawingShape::Fibonacci {
            control_segment,
            levels,
            ..
        } => {
            let mut min_dist = f64::INFINITY;
            if let Some(ctrl) = control_segment {
                min_dist = min_dist.min(distance_to_segment(cursor, *ctrl));
            }
            for lvl in levels {
                if lvl.visible_in_pane {
                    let seg = LocalSegment::new(
                        LocalPoint::new(lvl.x_left, lvl.y),
                        LocalPoint::new(lvl.x_right, lvl.y),
                    );
                    min_dist = min_dist.min(distance_to_segment(cursor, seg));
                }
            }
            (min_dist <= STROKE_HIT_TOLERANCE_PX).then_some(HitMatch {
                drawing_id: projected.id,
                locked: projected.locked,
                target: HitTarget::Body,
                priority: HitPriority::Stroke,
                distance: min_dist,
                z_order,
            })
        }
        ProjectedDrawingShape::RiskReward {
            reward_fill,
            risk_fill,
            entry_line,
            target_line,
            stop_line,
            side_borders,
            ..
        } => {
            let mut min_dist = f64::INFINITY;
            for seg in [
                *entry_line,
                *target_line,
                *stop_line,
                side_borders[0],
                side_borders[1],
            ]
            .into_iter()
            .flatten()
            {
                min_dist = min_dist.min(distance_to_segment(cursor, seg));
            }
            if min_dist <= STROKE_HIT_TOLERANCE_PX {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::Stroke,
                    distance: min_dist,
                    z_order,
                });
            }
            if is_selected && !projected.locked {
                let inside = reward_fill.is_some_and(|r| r.contains(cursor))
                    || risk_fill.is_some_and(|r| r.contains(cursor));
                if inside {
                    return Some(HitMatch {
                        drawing_id: projected.id,
                        locked: projected.locked,
                        target: HitTarget::Body,
                        priority: HitPriority::SelectedBody,
                        distance: 0.0,
                        z_order,
                    });
                }
            }
            None
        }
        ProjectedDrawingShape::FixedRangeProfile {
            clipped_box,
            left_border,
            right_border,
            ..
        } => {
            let mut min_dist = f64::INFINITY;
            for seg in [*left_border, *right_border].into_iter().flatten() {
                min_dist = min_dist.min(distance_to_segment(cursor, seg));
            }
            if min_dist <= STROKE_HIT_TOLERANCE_PX {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::Stroke,
                    distance: min_dist,
                    z_order,
                });
            }
            if is_selected
                && !projected.locked
                && let Some(rect) = clipped_box
                && rect.contains(cursor)
            {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::SelectedBody,
                    distance: 0.0,
                    z_order,
                });
            }
            None
        }
        ProjectedDrawingShape::AnchoredVwap {
            anchor_local,
            anchor_marker,
            tail_segments,
            ..
        } => {
            let mut min_dist = cursor.distance(*anchor_local);
            if let Some(seg) = anchor_marker {
                min_dist = min_dist.min(distance_to_segment(cursor, *seg));
            }
            for seg in tail_segments {
                min_dist = min_dist.min(distance_to_segment(cursor, *seg));
            }
            if min_dist <= STROKE_HIT_TOLERANCE_PX + 2.0 {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::Stroke,
                    distance: min_dist,
                    z_order,
                });
            }
            if is_selected && !projected.locked && min_dist <= 16.0 {
                return Some(HitMatch {
                    drawing_id: projected.id,
                    locked: projected.locked,
                    target: HitTarget::Body,
                    priority: HitPriority::SelectedBody,
                    distance: min_dist,
                    z_order,
                });
            }
            None
        }
        ProjectedDrawingShape::TextNote {
            anchor_local,
            box_rect,
            ..
        } => {
            let mut min_dist = cursor.distance(*anchor_local);
            if let Some(rect) = box_rect {
                if rect.contains(cursor) {
                    return Some(HitMatch {
                        drawing_id: projected.id,
                        locked: projected.locked,
                        target: HitTarget::Body,
                        priority: HitPriority::Stroke,
                        distance: 0.0,
                        z_order,
                    });
                }
                let edges = [
                    LocalSegment::new(
                        LocalPoint::new(rect.left, rect.top),
                        LocalPoint::new(rect.right, rect.top),
                    ),
                    LocalSegment::new(
                        LocalPoint::new(rect.right, rect.top),
                        LocalPoint::new(rect.right, rect.bottom),
                    ),
                    LocalSegment::new(
                        LocalPoint::new(rect.right, rect.bottom),
                        LocalPoint::new(rect.left, rect.bottom),
                    ),
                    LocalSegment::new(
                        LocalPoint::new(rect.left, rect.bottom),
                        LocalPoint::new(rect.left, rect.top),
                    ),
                ];
                for e in edges {
                    min_dist = min_dist.min(distance_to_segment(cursor, e));
                }
                if min_dist <= STROKE_HIT_TOLERANCE_PX {
                    return Some(HitMatch {
                        drawing_id: projected.id,
                        locked: projected.locked,
                        target: HitTarget::Body,
                        priority: HitPriority::Stroke,
                        distance: min_dist,
                        z_order,
                    });
                }
            }
            None
        }
    }
}

/// Evaluates all visible drawings belonging to `pane` and returns the highest-precedence [`HitMatch`].
///
/// Selection is strictly order-independent and transitive:
/// 1. Maximize `HitPriority`.
/// 2. Compute the true minimum distance `min_dist` among hits at `max_priority`.
/// 3. Return the highest-Z-order hit within `min_dist + 0.5 px` at `max_priority`.
#[must_use]
pub fn hit_test_pane_drawings(
    drawings: &[Drawing],
    pane: super::DrawingPaneTarget,
    proj: &PaneDrawingProjection<'_>,
    cursor: LocalPoint,
    selected_id: Option<DrawingId>,
) -> Option<HitMatch> {
    hit_test_pane_drawings_causal(drawings, pane, proj, cursor, selected_id, None, usize::MAX)
}

/// Causality-aware variant of [`hit_test_pane_drawings`] that skips drawings masked at
/// `replay_cutoff_timestamp` (`DEF-001`).
#[must_use]
pub fn hit_test_pane_drawings_causal(
    drawings: &[Drawing],
    pane: super::DrawingPaneTarget,
    proj: &PaneDrawingProjection<'_>,
    cursor: LocalPoint,
    selected_id: Option<DrawingId>,
    replay_cutoff_timestamp: Option<i64>,
    visible_candle_count: usize,
) -> Option<HitMatch> {
    const INLINE_CAP: usize = 32;
    let mut inline_hits: [Option<HitMatch>; INLINE_CAP] = [None; INLINE_CAP];
    let mut count = 0_usize;
    let mut max_priority: Option<HitPriority> = None;
    let mut min_dist_at_priority = f64::INFINITY;

    for (z_order, drawing) in drawings.iter().enumerate() {
        if !drawing.is_causally_visible(replay_cutoff_timestamp, visible_candle_count)
            || drawing.pane != pane
        {
            continue;
        }
        let Some(projected) = proj.project_drawing(drawing) else {
            continue;
        };
        let is_selected = selected_id == Some(drawing.id);
        let Some(candidate) = hit_test_projected_drawing(&projected, cursor, is_selected, z_order)
        else {
            continue;
        };
        match max_priority {
            None => {
                max_priority = Some(candidate.priority);
                min_dist_at_priority = candidate.distance;
            }
            Some(mp) if candidate.priority > mp => {
                max_priority = Some(candidate.priority);
                min_dist_at_priority = candidate.distance;
            }
            Some(mp) if candidate.priority == mp => {
                min_dist_at_priority = min_dist_at_priority.min(candidate.distance);
            }
            Some(_) => {}
        }
        if count < INLINE_CAP {
            inline_hits[count] = Some(candidate);
        }
        count += 1;
    }

    let target_priority = max_priority?;
    let max_tie_dist = min_dist_at_priority + 0.5;

    if count <= INLINE_CAP {
        let mut winner: Option<HitMatch> = None;
        for hit in inline_hits[..count].iter().flatten().copied() {
            if hit.priority == target_priority
                && hit.distance <= max_tie_dist
                && winner.is_none_or(|w| hit.z_order >= w.z_order)
            {
                winner = Some(hit);
            }
        }
        return winner;
    }

    // Rare fallback when > 32 drawings simultaneously intersect the cursor: scan top-down by Z-order.
    for (z_order, drawing) in drawings.iter().enumerate().rev() {
        if !drawing.is_causally_visible(replay_cutoff_timestamp, visible_candle_count)
            || drawing.pane != pane
        {
            continue;
        }
        let Some(projected) = proj.project_drawing(drawing) else {
            continue;
        };
        let is_selected = selected_id == Some(drawing.id);
        if let Some(candidate) =
            hit_test_projected_drawing(&projected, cursor, is_selected, z_order)
            && candidate.priority == target_priority
            && candidate.distance <= max_tie_dist
        {
            return Some(candidate);
        }
    }
    None
}

/// Zero-allocation `O(threshold / bar_spacing)` OHLC candle magnet (`drawingMagnet.ts:6-16`).
///
/// Deterministically breaks exact ties by `(distance_sq, candle_index, field_rank [Open, High, Low, Close])`.
#[must_use]
pub fn snap_to_visible_ohlc(
    raw_anchor: DrawingAnchor,
    cursor: LocalPoint,
    candles: &[Candle],
    proj: &PaneDrawingProjection<'_>,
    mode: MagnetMode,
    threshold_px: f64,
) -> DrawingAnchor {
    if mode == MagnetMode::Off
        || candles.is_empty()
        || !cursor.is_finite()
        || !threshold_px.is_finite()
        || threshold_px <= 0.0
    {
        return raw_anchor;
    }

    let bar_spacing = proj.time_scale.bar_spacing();
    if !bar_spacing.is_finite() || bar_spacing <= 0.0 {
        return raw_anchor;
    }

    let Some(center_float) = proj.time_scale.local_x_to_center_float_index(cursor.x) else {
        return raw_anchor;
    };
    let bar_radius = (threshold_px / bar_spacing).ceil() + 1.0;
    let last_idx = (candles.len() - 1) as f64;
    let min_f = (center_float - bar_radius).floor();
    let max_f = (center_float + bar_radius).ceil();
    if max_f < 0.0 || min_f > last_idx {
        return raw_anchor;
    }

    let start_idx = min_f.clamp(0.0, last_idx) as usize;
    let end_idx = max_f.clamp(0.0, last_idx) as usize;
    let threshold_sq = threshold_px * threshold_px;

    let mut winner: Option<(f64, usize, usize, f64)> = None;
    for (offset, candle) in candles[start_idx..=end_idx].iter().enumerate() {
        let idx = start_idx + offset;
        let Some(cx) = proj.time_scale.index_to_local_x(idx as i64) else {
            continue;
        };
        let dx = cx - cursor.x;
        if dx.abs() > threshold_px {
            continue;
        }
        let fields = [candle.open, candle.high, candle.low, candle.close];
        for (field_rank, &price) in fields.iter().enumerate() {
            let Some(cy) = proj.price_scale.price_to_local_y(price, proj.base_value) else {
                continue;
            };
            let dy = cy - cursor.y;
            if dy.abs() > threshold_px {
                continue;
            }
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > threshold_sq {
                continue;
            }
            let replace = match winner {
                None => true,
                Some((best_d, best_idx, best_rank, _)) => {
                    if (dist_sq - best_d).abs() > 1e-12 {
                        dist_sq < best_d
                    } else if idx != best_idx {
                        idx < best_idx
                    } else {
                        field_rank < best_rank
                    }
                }
            };
            if replace {
                winner = Some((dist_sq, idx, field_rank, price));
            }
        }
    }

    if let Some((_, idx, _, price)) = winner {
        let snapped = DrawingAnchor::new(idx as f64, price);
        if snapped.is_valid() {
            return snapped;
        }
    }
    raw_anchor
}
