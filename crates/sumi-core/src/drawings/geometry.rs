use std::sync::Arc;

use crate::indicators::{VwapBandsOutput, compute_anchored_vwap};
use crate::model::Candle;
use crate::viewport::{PriceScale, TimeScale};

use super::{Drawing, DrawingAnchor, DrawingGeometry, FibDirection, wrap_text_note_lines};

/// Pane-local 2D coordinate (`x ∈ [0, width]`, `y ∈ [0, height]` for points inside the plot canvas).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPoint {
    pub x: f64,
    pub y: f64,
}

impl LocalPoint {
    #[inline]
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline]
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    #[inline]
    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

/// Clipped line segment in pane-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSegment {
    pub start: LocalPoint,
    pub end: LocalPoint,
}

impl LocalSegment {
    #[inline]
    #[must_use]
    pub const fn new(start: LocalPoint, end: LocalPoint) -> Self {
        Self { start, end }
    }
}

/// Axis-aligned box in pane-local coordinates (`left <= right`, `top <= bottom`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl LocalRect {
    #[inline]
    #[must_use]
    pub fn from_corners(a: LocalPoint, b: LocalPoint) -> Self {
        Self {
            left: a.x.min(b.x),
            top: a.y.min(b.y),
            right: a.x.max(b.x),
            bottom: a.y.max(b.y),
        }
    }

    #[inline]
    #[must_use]
    pub fn from_pane_size(width: f64, height: f64) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: width.max(0.0),
            bottom: height.max(0.0),
        }
    }

    #[inline]
    #[must_use]
    pub fn width(self) -> f64 {
        (self.right - self.left).max(0.0)
    }

    #[inline]
    #[must_use]
    pub fn height(self) -> f64 {
        (self.bottom - self.top).max(0.0)
    }

    #[inline]
    #[must_use]
    pub fn contains(self, pt: LocalPoint) -> bool {
        pt.x >= self.left && pt.x <= self.right && pt.y >= self.top && pt.y <= self.bottom
    }

    #[inline]
    #[must_use]
    pub fn contains_with_margin(self, pt: LocalPoint, margin: f64) -> bool {
        pt.x >= self.left - margin
            && pt.x <= self.right + margin
            && pt.y >= self.top - margin
            && pt.y <= self.bottom + margin
    }

    #[inline]
    #[must_use]
    pub fn intersect(self, other: Self) -> Option<Self> {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right.min(other.right);
        let bottom = self.bottom.min(other.bottom);
        (left <= right && top <= bottom).then_some(Self {
            left,
            top,
            right,
            bottom,
        })
    }
}

/// Interactive handle identifier on a projected drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlePart {
    Anchor(usize),
    /// Deterministic rectangle corner index (`0`: `(a0.bar, a0.val)`, `1`: `(a1.bar, a0.val)`,
    /// `2`: `(a1.bar, a1.val)`, `3`: `(a0.bar, a1.val)`), matching `drawingGeometry.ts:21-23`.
    RectCorner(u8),
    /// Left horizontal/entry handle of a `RiskRewardBox`.
    RrEntryLeft,
    /// Right horizontal/entry handle of a `RiskRewardBox`.
    RrEntryRight,
    /// Mid-span target price handle of a `RiskRewardBox`.
    RrTarget,
    /// Mid-span stop-loss price handle of a `RiskRewardBox`.
    RrStop,
    /// Mid-span parallel border offset handle of a `ParallelChannel` (`v1.1 / Wave 2`).
    ChannelOffset,
}

/// Projected handle coordinate in pane-local space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedHandle {
    pub part: HandlePart,
    pub pos: LocalPoint,
}

/// Single projected Fibonacci retracement level clipped horizontally to the pane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedFibLevel {
    pub ratio: f64,
    pub value: f64,
    pub y: f64,
    pub x_left: f64,
    pub x_right: f64,
    pub visible_in_pane: bool,
}

/// Clipped vertical fill band between two consecutive Fibonacci levels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedFibBand {
    pub band_index: usize,
    pub rect: LocalRect,
}

/// Causal measurement summary computed for a [`DrawingGeometry::PriceDateRange`] (`v1.1 / Wave 2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceDateRangeMetrics {
    pub price_delta: f64,
    pub price_pct: Option<f64>,
    pub bar_count: usize,
    pub elapsed_secs: Option<i64>,
    pub total_volume: Option<f64>,
    pub is_bullish: bool,
}

/// Clipped, render-ready and hit-test-ready pane-local geometry for a [`Drawing`].
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectedDrawingShape {
    HorizontalLine {
        y: f64,
        value: f64,
        visible_segment: Option<LocalSegment>,
    },
    Segment {
        raw_start: LocalPoint,
        raw_end: LocalPoint,
        clipped: Option<LocalSegment>,
    },
    ParallelChannel {
        base_line: Option<LocalSegment>,
        parallel_line: Option<LocalSegment>,
        mid_line: Option<LocalSegment>,
        fill_polygon: Vec<LocalPoint>,
    },
    Rectangle {
        raw_rect: LocalRect,
        clipped_fill: Option<LocalRect>,
        clipped_edges: [Option<LocalSegment>; 4],
    },
    PriceDateRange {
        raw_rect: LocalRect,
        clipped_fill: Option<LocalRect>,
        clipped_edges: [Option<LocalSegment>; 4],
        start_local: LocalPoint,
        end_local: LocalPoint,
        metrics: PriceDateRangeMetrics,
    },
    Fibonacci {
        control_segment: Option<LocalSegment>,
        levels: Vec<ProjectedFibLevel>,
        bands: Vec<ProjectedFibBand>,
    },
    RiskReward {
        is_long: bool,
        entry_price: f64,
        stop_price: f64,
        target_price: f64,
        gross_r: f64,
        reward_fill: Option<LocalRect>,
        risk_fill: Option<LocalRect>,
        entry_line: Option<LocalSegment>,
        target_line: Option<LocalSegment>,
        stop_line: Option<LocalSegment>,
        side_borders: [Option<LocalSegment>; 2],
    },
    FixedRangeProfile {
        raw_x_left: f64,
        raw_x_right: f64,
        clipped_box: Option<LocalRect>,
        left_border: Option<LocalSegment>,
        right_border: Option<LocalSegment>,
        min_bar_index: f64,
        max_bar_index: f64,
        row_count: u16,
    },
    AnchoredVwap {
        anchor_local: LocalPoint,
        anchor_marker: Option<LocalSegment>,
        anchor_bar_index: f64,
        anchor_value: f64,
        show_bands: bool,
        tail_segments: Vec<LocalSegment>,
    },
    TextNote {
        anchor_local: LocalPoint,
        box_rect: Option<LocalRect>,
        lines: Vec<String>,
    },
}

/// Complete pane-local projection of a [`Drawing`].
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedDrawing {
    pub id: super::DrawingId,
    pub locked: bool,
    pub anchors_local: Vec<LocalPoint>,
    pub handles: Vec<ProjectedHandle>,
    pub shape: ProjectedDrawingShape,
}

/// Pure `f64` pane-local projection context (`wasm32`-safe, independent of `egui`).
#[derive(Debug, Clone, Copy)]
pub struct PaneDrawingProjection<'a> {
    pub time_scale: &'a TimeScale,
    pub price_scale: &'a PriceScale,
    pub width: f64,
    pub height: f64,
    pub base_value: f64,
    pub candles: Option<&'a [Candle]>,
    /// Optional memoized `(anchor_bar_index, Arc<VwapBandsOutput>)` lookup table populated from
    /// `R11RenderMemo` so `AnchoredVwap` hit-testing & projection never recompute 100K-bar Welford
    /// series per frame (`BUG-015`).
    pub precomputed_avwaps: Option<&'a [(usize, Arc<VwapBandsOutput>)]>,
    /// When `true`, skips building `tail_segments` inside `project_drawing` because the paint pass
    /// renders the `AnchoredVwap` polyline directly from `R11RenderMemo` (`BUG-015`).
    pub skip_avwap_tail_segments: bool,
}

impl<'a> PaneDrawingProjection<'a> {
    #[inline]
    #[must_use]
    pub const fn new(
        time_scale: &'a TimeScale,
        price_scale: &'a PriceScale,
        width: f64,
        height: f64,
        base_value: f64,
    ) -> Self {
        Self {
            time_scale,
            price_scale,
            width,
            height,
            base_value,
            candles: None,
            precomputed_avwaps: None,
            skip_avwap_tail_segments: false,
        }
    }

    #[inline]
    #[must_use]
    pub const fn with_candles(mut self, candles: &'a [Candle]) -> Self {
        self.candles = Some(candles);
        self
    }

    #[inline]
    #[must_use]
    pub const fn with_precomputed_avwaps(
        mut self,
        precomputed: &'a [(usize, Arc<VwapBandsOutput>)],
    ) -> Self {
        self.precomputed_avwaps = Some(precomputed);
        self
    }

    #[inline]
    #[must_use]
    pub const fn with_skip_avwap_tail_segments(mut self, skip: bool) -> Self {
        self.skip_avwap_tail_segments = skip;
        self
    }

    #[inline]
    #[must_use]
    pub fn pane_bounds(&self) -> LocalRect {
        LocalRect::from_pane_size(self.width, self.height)
    }

    #[inline]
    #[must_use]
    pub fn anchor_to_local(&self, anchor: DrawingAnchor) -> Option<LocalPoint> {
        if self.width <= 0.0 || self.height <= 0.0 || !anchor.is_valid() {
            return None;
        }
        let x = self.time_scale.float_index_to_local_x(anchor.bar_index)?;
        let y = self
            .price_scale
            .price_to_local_y(anchor.value, self.base_value)?;
        (x.is_finite() && y.is_finite()).then_some(LocalPoint::new(x, y))
    }

    #[inline]
    #[must_use]
    pub fn local_to_anchor(&self, pt: LocalPoint) -> Option<DrawingAnchor> {
        if self.width <= 0.0 || self.height <= 0.0 || !pt.is_finite() {
            return None;
        }
        let bar_index = self.time_scale.local_x_to_center_float_index(pt.x)?;
        let value = self.price_scale.local_y_to_price(pt.y, self.base_value)?;
        let anchor = DrawingAnchor::new(bar_index, value);
        anchor.is_valid().then_some(anchor)
    }

    /// Projects a [`Drawing`] into clipped pane-local primitives and interactive handles.
    #[must_use]
    pub fn project_drawing(&self, drawing: &Drawing) -> Option<ProjectedDrawing> {
        if !drawing.visible || self.width <= 1.0 || self.height <= 1.0 {
            return None;
        }
        let bounds = self.pane_bounds();

        match drawing.geometry {
            DrawingGeometry::HorizontalLine { anchor } => {
                let p0 = self.anchor_to_local(anchor)?;
                let visible_segment = (p0.y >= bounds.top && p0.y <= bounds.bottom).then(|| {
                    LocalSegment::new(
                        LocalPoint::new(bounds.left, p0.y),
                        LocalPoint::new(bounds.right, p0.y),
                    )
                });
                let handle_x = if self.width >= 28.0 {
                    p0.x.clamp(14.0, self.width - 14.0)
                } else {
                    self.width * 0.5
                };
                let handles = if visible_segment.is_some() {
                    vec![ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: LocalPoint::new(handle_x, p0.y),
                    }]
                } else {
                    Vec::new()
                };
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0],
                    handles,
                    shape: ProjectedDrawingShape::HorizontalLine {
                        y: p0.y,
                        value: anchor.value,
                        visible_segment,
                    },
                })
            }
            DrawingGeometry::TrendLine { start, end } => {
                let p0 = self.anchor_to_local(start)?;
                let p1 = self.anchor_to_local(end)?;
                // Time-directional extension (`trend_extend_left` -> `-∞` X, `trend_extend_right` -> `+∞` X),
                // invariant to whether `start` was placed left-of or right-of `end`!
                let has_horiz_dir = (p1.x - p0.x).abs() > 1.0e-9;
                let (t_min, t_max) = if !has_horiz_dir {
                    (0.0, 1.0)
                } else if p0.x <= p1.x {
                    (
                        if drawing.trend_extend_left {
                            f64::NEG_INFINITY
                        } else {
                            0.0
                        },
                        if drawing.trend_extend_right {
                            f64::INFINITY
                        } else {
                            1.0
                        },
                    )
                } else {
                    (
                        if drawing.trend_extend_right {
                            f64::NEG_INFINITY
                        } else {
                            0.0
                        },
                        if drawing.trend_extend_left {
                            f64::INFINITY
                        } else {
                            1.0
                        },
                    )
                };
                let clipped = clip_parametric_line(p0, p1, t_min, t_max, bounds);
                let mut handles = Vec::with_capacity(2);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }
                if bounds.contains_with_margin(p1, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(1),
                        pos: p1,
                    });
                }
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1],
                    handles,
                    shape: ProjectedDrawingShape::Segment {
                        raw_start: p0,
                        raw_end: p1,
                        clipped,
                    },
                })
            }
            DrawingGeometry::Ray { origin, through } => {
                let p0 = self.anchor_to_local(origin)?;
                let p1 = self.anchor_to_local(through)?;
                let clipped = clip_parametric_line(p0, p1, 0.0, f64::INFINITY, bounds);
                let mut handles = Vec::with_capacity(2);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }
                if bounds.contains_with_margin(p1, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(1),
                        pos: p1,
                    });
                }
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1],
                    handles,
                    shape: ProjectedDrawingShape::Segment {
                        raw_start: p0,
                        raw_end: p1,
                        clipped,
                    },
                })
            }
            DrawingGeometry::ParallelChannel {
                start,
                end,
                channel_offset,
            } => {
                let p0 = self.anchor_to_local(start)?;
                let p1 = self.anchor_to_local(end)?;
                let par_start = DrawingAnchor::new(start.bar_index, start.value + channel_offset);
                let par_end = DrawingAnchor::new(end.bar_index, end.value + channel_offset);
                let q0 = self.anchor_to_local(par_start)?;
                let q1 = self.anchor_to_local(par_end)?;
                let mid_start =
                    DrawingAnchor::new(start.bar_index, start.value + 0.5 * channel_offset);
                let mid_end = DrawingAnchor::new(end.bar_index, end.value + 0.5 * channel_offset);
                let m0 = self.anchor_to_local(mid_start)?;
                let m1 = self.anchor_to_local(mid_end)?;

                let has_horiz_dir = (p1.x - p0.x).abs() > 1.0e-9;
                let (t_min, t_max) = if !has_horiz_dir {
                    (0.0, 1.0)
                } else if p0.x <= p1.x {
                    (
                        if drawing.trend_extend_left {
                            f64::NEG_INFINITY
                        } else {
                            0.0
                        },
                        if drawing.trend_extend_right {
                            f64::INFINITY
                        } else {
                            1.0
                        },
                    )
                } else {
                    (
                        if drawing.trend_extend_right {
                            f64::NEG_INFINITY
                        } else {
                            0.0
                        },
                        if drawing.trend_extend_left {
                            f64::INFINITY
                        } else {
                            1.0
                        },
                    )
                };

                let base_line = clip_parametric_line(p0, p1, t_min, t_max, bounds);
                let parallel_line = clip_parametric_line(q0, q1, t_min, t_max, bounds);
                let mid_line = clip_parametric_line(m0, m1, t_min, t_max, bounds);

                let fill_polygon = if has_horiz_dir {
                    let dx = p1.x - p0.x;
                    let t_bound_a = ((bounds.left - 4.0) - p0.x) / dx;
                    let t_bound_b = ((bounds.right + 4.0) - p0.x) / dx;
                    let (t_lo_bound, t_hi_bound) = if t_bound_a <= t_bound_b {
                        (t_bound_a, t_bound_b)
                    } else {
                        (t_bound_b, t_bound_a)
                    };
                    let t_lo = if t_min.is_infinite() {
                        t_lo_bound.min(0.0)
                    } else {
                        t_min
                    };
                    let t_hi = if t_max.is_infinite() {
                        t_hi_bound.max(1.0)
                    } else {
                        t_max
                    };
                    if t_lo.is_finite() && t_hi.is_finite() && (t_hi - t_lo).abs() > 1.0e-9 {
                        let lerp = |a: LocalPoint, b: LocalPoint, t: f64| {
                            LocalPoint::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
                        };
                        let quad = vec![
                            lerp(p0, p1, t_lo),
                            lerp(p0, p1, t_hi),
                            lerp(q0, q1, t_hi),
                            lerp(q0, q1, t_lo),
                        ];
                        clip_polygon_to_rect(quad, bounds)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                let par_mid = LocalPoint::new(0.5 * (q0.x + q1.x), 0.5 * (q0.y + q1.y));
                let mut handles = Vec::with_capacity(3);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }
                if bounds.contains_with_margin(p1, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(1),
                        pos: p1,
                    });
                }
                if bounds.contains_with_margin(par_mid, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::ChannelOffset,
                        pos: par_mid,
                    });
                }

                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1, par_mid],
                    handles,
                    shape: ProjectedDrawingShape::ParallelChannel {
                        base_line,
                        parallel_line,
                        mid_line,
                        fill_polygon,
                    },
                })
            }
            DrawingGeometry::PriceDateRange { start, end } => {
                let p0 = self.anchor_to_local(start)?;
                let p1 = self.anchor_to_local(end)?;
                let raw_rect = LocalRect::from_corners(p0, p1);
                let clipped_fill = raw_rect
                    .intersect(bounds)
                    .filter(|r| r.width() > 0.0 && r.height() > 0.0);
                let corners = [
                    p0,
                    LocalPoint::new(p1.x, p0.y),
                    p1,
                    LocalPoint::new(p0.x, p1.y),
                ];
                let clipped_edges = [
                    clip_parametric_line(corners[0], corners[1], 0.0, 1.0, bounds),
                    clip_parametric_line(corners[1], corners[2], 0.0, 1.0, bounds),
                    clip_parametric_line(corners[2], corners[3], 0.0, 1.0, bounds),
                    clip_parametric_line(corners[3], corners[0], 0.0, 1.0, bounds),
                ];
                let mut handles = Vec::with_capacity(2);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }
                if bounds.contains_with_margin(p1, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(1),
                        pos: p1,
                    });
                }
                let metrics = compute_price_date_range_metrics(start, end, self.candles);
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1],
                    handles,
                    shape: ProjectedDrawingShape::PriceDateRange {
                        raw_rect,
                        clipped_fill,
                        clipped_edges,
                        start_local: p0,
                        end_local: p1,
                        metrics,
                    },
                })
            }
            DrawingGeometry::Rectangle { corner_a, corner_b } => {
                let p0 = self.anchor_to_local(corner_a)?;
                let p1 = self.anchor_to_local(corner_b)?;
                let raw_rect = LocalRect::from_corners(p0, p1);
                let clipped_fill = raw_rect.intersect(bounds);

                let corners = [
                    p0,
                    LocalPoint::new(p1.x, p0.y),
                    p1,
                    LocalPoint::new(p0.x, p1.y),
                ];
                let clipped_edges = [
                    clip_parametric_line(corners[0], corners[1], 0.0, 1.0, bounds),
                    clip_parametric_line(corners[1], corners[2], 0.0, 1.0, bounds),
                    clip_parametric_line(corners[2], corners[3], 0.0, 1.0, bounds),
                    clip_parametric_line(corners[3], corners[0], 0.0, 1.0, bounds),
                ];
                let mut handles = Vec::with_capacity(4);
                for (idx, &corner_pos) in corners.iter().enumerate() {
                    if bounds.contains_with_margin(corner_pos, 8.0) {
                        handles.push(ProjectedHandle {
                            part: HandlePart::RectCorner(idx as u8),
                            pos: corner_pos,
                        });
                    }
                }
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1],
                    handles,
                    shape: ProjectedDrawingShape::Rectangle {
                        raw_rect,
                        clipped_fill,
                        clipped_edges,
                    },
                })
            }
            DrawingGeometry::FibonacciRetracement {
                start,
                end,
                direction,
                extend_right,
            } => {
                let p0 = self.anchor_to_local(start)?;
                let p1 = self.anchor_to_local(end)?;
                let control_segment = clip_parametric_line(p0, p1, 0.0, 1.0, bounds);

                let raw_x_left = p0.x.min(p1.x);
                let raw_x_right = if extend_right {
                    bounds.right.max(raw_x_left)
                } else {
                    p0.x.max(p1.x).max(raw_x_left + 24.0)
                };
                let vis_x_left = raw_x_left.max(bounds.left);
                let vis_x_right = raw_x_right.min(bounds.right);
                let has_horz_overlap = vis_x_left <= vis_x_right;

                let active_ratios = drawing.effective_fib_ratios();
                let mut levels = Vec::with_capacity(active_ratios.len());
                for &ratio in active_ratios {
                    let val = fibonacci_level_value(start, end, ratio, direction);
                    if let Some(y) = self.price_scale.price_to_local_y(val, self.base_value) {
                        let visible_in_pane =
                            has_horz_overlap && y >= bounds.top && y <= bounds.bottom;
                        levels.push(ProjectedFibLevel {
                            ratio,
                            value: val,
                            y,
                            x_left: vis_x_left,
                            x_right: vis_x_right,
                            visible_in_pane,
                        });
                    }
                }

                let mut bands = Vec::with_capacity(levels.len().saturating_sub(1));
                if has_horz_overlap {
                    for (idx, pair) in levels.windows(2).enumerate() {
                        let band_rect = LocalRect {
                            left: vis_x_left,
                            top: pair[0].y.min(pair[1].y),
                            right: vis_x_right,
                            bottom: pair[0].y.max(pair[1].y),
                        };
                        if let Some(clipped) = band_rect.intersect(bounds)
                            && clipped.width() > 0.0
                            && clipped.height() > 0.0
                        {
                            bands.push(ProjectedFibBand {
                                band_index: idx,
                                rect: clipped,
                            });
                        }
                    }
                }

                let mut handles = Vec::with_capacity(2);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }
                if bounds.contains_with_margin(p1, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(1),
                        pos: p1,
                    });
                }

                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1],
                    handles,
                    shape: ProjectedDrawingShape::Fibonacci {
                        control_segment,
                        levels,
                        bands,
                    },
                })
            }
            DrawingGeometry::RiskRewardBox {
                start,
                end,
                stop_price: _,
            } => {
                let (side, entry_price, resolved_stop, target_price, gross_r) =
                    drawing.geometry.risk_reward_metrics()?;
                let p0 = self.anchor_to_local(start)?;
                let p1 = self.anchor_to_local(end)?;
                let y_stop = self
                    .price_scale
                    .price_to_local_y(resolved_stop, self.base_value)?;
                if !y_stop.is_finite() {
                    return None;
                }

                let raw_x_left = p0.x.min(p1.x);
                let raw_x_right = p0.x.max(p1.x).max(raw_x_left + 36.0);
                let x_mid = 0.5 * (raw_x_left + raw_x_right);

                let reward_raw = LocalRect {
                    left: raw_x_left,
                    top: p0.y.min(p1.y),
                    right: raw_x_right,
                    bottom: p0.y.max(p1.y),
                };
                let risk_raw = LocalRect {
                    left: raw_x_left,
                    top: p0.y.min(y_stop),
                    right: raw_x_right,
                    bottom: p0.y.max(y_stop),
                };
                let reward_fill = reward_raw
                    .intersect(bounds)
                    .filter(|r| r.width() > 0.0 && r.height() > 0.0);
                let risk_fill = risk_raw
                    .intersect(bounds)
                    .filter(|r| r.width() > 0.0 && r.height() > 0.0);

                let entry_line = clip_parametric_line(
                    LocalPoint::new(raw_x_left, p0.y),
                    LocalPoint::new(raw_x_right, p0.y),
                    0.0,
                    1.0,
                    bounds,
                );
                let target_line = clip_parametric_line(
                    LocalPoint::new(raw_x_left, p1.y),
                    LocalPoint::new(raw_x_right, p1.y),
                    0.0,
                    1.0,
                    bounds,
                );
                let stop_line = clip_parametric_line(
                    LocalPoint::new(raw_x_left, y_stop),
                    LocalPoint::new(raw_x_right, y_stop),
                    0.0,
                    1.0,
                    bounds,
                );

                let box_top = p1.y.min(y_stop);
                let box_bot = p1.y.max(y_stop);
                let side_borders = [
                    clip_parametric_line(
                        LocalPoint::new(raw_x_left, box_top),
                        LocalPoint::new(raw_x_left, box_bot),
                        0.0,
                        1.0,
                        bounds,
                    ),
                    clip_parametric_line(
                        LocalPoint::new(raw_x_right, box_top),
                        LocalPoint::new(raw_x_right, box_bot),
                        0.0,
                        1.0,
                        bounds,
                    ),
                ];

                let mut handles = Vec::with_capacity(4);
                let handle_defs = [
                    (HandlePart::RrEntryLeft, LocalPoint::new(raw_x_left, p0.y)),
                    (HandlePart::RrEntryRight, LocalPoint::new(raw_x_right, p0.y)),
                    (HandlePart::RrTarget, LocalPoint::new(x_mid, p1.y)),
                    (HandlePart::RrStop, LocalPoint::new(x_mid, y_stop)),
                ];
                for (part, pos) in handle_defs {
                    if bounds.contains_with_margin(pos, 8.0) {
                        handles.push(ProjectedHandle { part, pos });
                    }
                }

                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1, LocalPoint::new(p1.x, y_stop)],
                    handles,
                    shape: ProjectedDrawingShape::RiskReward {
                        is_long: side == crate::trading::PositionSide::Long,
                        entry_price,
                        stop_price: resolved_stop,
                        target_price,
                        gross_r,
                        reward_fill,
                        risk_fill,
                        entry_line,
                        target_line,
                        stop_line,
                        side_borders,
                    },
                })
            }
            DrawingGeometry::FixedRangeVolumeProfile {
                start,
                end,
                row_count,
            } => {
                let p0 = self.anchor_to_local(start)?;
                let p1 = self.anchor_to_local(end)?;
                let raw_x_left = p0.x.min(p1.x);
                let raw_x_right = p0.x.max(p1.x).max(raw_x_left + 16.0);
                let full_strip = LocalRect {
                    left: raw_x_left,
                    top: bounds.top,
                    right: raw_x_right,
                    bottom: bounds.bottom,
                };
                let clipped_box = full_strip
                    .intersect(bounds)
                    .filter(|r| r.width() > 0.0 && r.height() > 0.0);
                let left_border = clip_parametric_line(
                    LocalPoint::new(raw_x_left, bounds.top),
                    LocalPoint::new(raw_x_left, bounds.bottom),
                    0.0,
                    1.0,
                    bounds,
                );
                let right_border = clip_parametric_line(
                    LocalPoint::new(raw_x_right, bounds.top),
                    LocalPoint::new(raw_x_right, bounds.bottom),
                    0.0,
                    1.0,
                    bounds,
                );
                let clamp_frvp_y = |y: f64| -> f64 {
                    if bounds.height() >= 20.0 {
                        y.clamp(bounds.top + 10.0, bounds.bottom - 10.0)
                    } else {
                        0.5 * (bounds.top + bounds.bottom)
                    }
                };
                let mut handles = Vec::with_capacity(2);
                if p0.x.is_finite() && p0.x >= bounds.left - 8.0 && p0.x <= bounds.right + 8.0 {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: LocalPoint::new(p0.x, clamp_frvp_y(p0.y)),
                    });
                }
                if p1.x.is_finite() && p1.x >= bounds.left - 8.0 && p1.x <= bounds.right + 8.0 {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(1),
                        pos: LocalPoint::new(p1.x, clamp_frvp_y(p1.y)),
                    });
                }
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0, p1],
                    handles,
                    shape: ProjectedDrawingShape::FixedRangeProfile {
                        raw_x_left,
                        raw_x_right,
                        clipped_box,
                        left_border,
                        right_border,
                        min_bar_index: start.bar_index.min(end.bar_index),
                        max_bar_index: start.bar_index.max(end.bar_index),
                        row_count,
                    },
                })
            }
            DrawingGeometry::AnchoredVwap { anchor, show_bands } => {
                let p0 = self.anchor_to_local(anchor)?;
                let marker_half = 18.0_f64;
                let anchor_marker = clip_parametric_line(
                    LocalPoint::new(p0.x, p0.y - marker_half),
                    LocalPoint::new(p0.x, p0.y + marker_half),
                    0.0,
                    1.0,
                    bounds,
                );
                let mut handles = Vec::with_capacity(1);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }
                let mut tail_segments = Vec::new();
                if !self.skip_avwap_tail_segments
                    && let Some(candles) = self.candles
                    && !candles.is_empty()
                    && anchor.bar_index >= -0.5
                {
                    let a_idx = anchor.bar_index.round().max(0.0) as usize;
                    if a_idx < candles.len() {
                        let owned_vwap;
                        let vwap_ref: &VwapBandsOutput = if let Some(cached) = self
                            .precomputed_avwaps
                            .and_then(|list| list.iter().find(|(idx, _)| *idx == a_idx))
                        {
                            cached.1.as_ref()
                        } else {
                            owned_vwap = compute_anchored_vwap(&candles[a_idx..], 0, 1.0, 2.0);
                            &owned_vwap
                        };
                        let strict = self.time_scale.strict_visible_data_range();
                        let vis_start =
                            strict.start.saturating_sub(2).max(a_idx).min(candles.len());
                        let vis_end = strict
                            .end
                            .saturating_add(2)
                            .min(candles.len())
                            .max(vis_start);
                        let step = ((vis_end - vis_start) / 256).max(1);
                        let map_pt = |idx: usize| -> Option<LocalPoint> {
                            let val = vwap_ref.vwap.get(idx - a_idx).copied().flatten()?;
                            if !val.is_finite() {
                                return None;
                            }
                            let x = self.time_scale.float_index_to_local_x(idx as f64)?;
                            let y = self.price_scale.price_to_local_y(val, self.base_value)?;
                            (x.is_finite() && y.is_finite()).then_some(LocalPoint::new(x, y))
                        };
                        let mut prev_pt: Option<LocalPoint> = None;
                        let mut k = vis_start;
                        while k < vis_end {
                            if let Some(curr) = map_pt(k) {
                                if let Some(prev) = prev_pt
                                    && let Some(seg) =
                                        clip_parametric_line(prev, curr, 0.0, 1.0, bounds)
                                {
                                    tail_segments.push(seg);
                                }
                                prev_pt = Some(curr);
                            } else {
                                prev_pt = None;
                            }
                            k += step;
                        }
                        if step > 1
                            && vis_end > vis_start + 1
                            && !(vis_end - 1 - vis_start).is_multiple_of(step)
                            && let Some(curr) = map_pt(vis_end - 1)
                            && let Some(prev) = prev_pt
                            && let Some(seg) = clip_parametric_line(prev, curr, 0.0, 1.0, bounds)
                        {
                            tail_segments.push(seg);
                        }
                    }
                }
                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0],
                    handles,
                    shape: ProjectedDrawingShape::AnchoredVwap {
                        anchor_local: p0,
                        anchor_marker,
                        anchor_bar_index: anchor.bar_index,
                        anchor_value: anchor.value,
                        show_bands,
                        tail_segments,
                    },
                })
            }
            DrawingGeometry::TextNote { anchor, ref text } => {
                let p0 = self.anchor_to_local(anchor)?;
                // Allow up to 14 wrapped lines of 36 chars (`504 >= MAX_TEXT_NOTE_CHARS = 280`) so valid
                // 280-char Vietnamese/CJK/emoji notes are NEVER truncated at 272 chars (`BUG-026`).
                let lines = wrap_text_note_lines(text, 36, 14);
                let max_col_width: f64 = lines
                    .iter()
                    .map(|l| {
                        l.chars()
                            .map(|c| {
                                let cp = c as u32;
                                if (0x1100..=0x115F).contains(&cp)
                                    || (0x2E80..=0xA4CF).contains(&cp)
                                    || (0xAC00..=0xD7A3).contains(&cp)
                                    || (0xF900..=0xFAFF).contains(&cp)
                                    || (0x1F300..=0x1FAFF).contains(&cp)
                                {
                                    1.85
                                } else {
                                    1.0
                                }
                            })
                            .sum::<f64>()
                    })
                    .fold(4.0_f64, f64::max);
                let box_w = (max_col_width * 6.5 + 18.0).clamp(48.0, (self.width - 8.0).max(48.0));
                let box_h =
                    ((lines.len() as f64) * 14.5 + 12.0).clamp(24.0, (self.height - 8.0).max(24.0));

                let visible_near_pane = bounds.contains_with_margin(p0, box_w.max(box_h) + 12.0);
                let box_rect = visible_near_pane.then(|| {
                    let prefer_x = if p0.x + 8.0 + box_w <= bounds.right - 2.0 {
                        p0.x + 8.0
                    } else {
                        (p0.x - 8.0 - box_w).max(bounds.left + 2.0)
                    };
                    let left = prefer_x.clamp(
                        bounds.left + 2.0,
                        (bounds.right - box_w - 2.0).max(bounds.left + 2.0),
                    );
                    let top = (p0.y - 0.5 * box_h).clamp(
                        bounds.top + 2.0,
                        (bounds.bottom - box_h - 2.0).max(bounds.top + 2.0),
                    );
                    LocalRect {
                        left,
                        top,
                        right: (left + box_w).min(bounds.right),
                        bottom: (top + box_h).min(bounds.bottom),
                    }
                });

                let mut handles = Vec::with_capacity(1);
                if bounds.contains_with_margin(p0, 8.0) {
                    handles.push(ProjectedHandle {
                        part: HandlePart::Anchor(0),
                        pos: p0,
                    });
                }

                Some(ProjectedDrawing {
                    id: drawing.id,
                    locked: drawing.locked,
                    anchors_local: vec![p0],
                    handles,
                    shape: ProjectedDrawingShape::TextNote {
                        anchor_local: p0,
                        box_rect,
                        lines,
                    },
                })
            }
        }
    }
}

/// Canonical Fibonacci retracement price interpolation (`drawingGeometry.ts:37-40`).
#[inline]
#[must_use]
pub fn fibonacci_level_value(
    start: DrawingAnchor,
    end: DrawingAnchor,
    ratio: f64,
    direction: FibDirection,
) -> f64 {
    let (v0, v1) = match direction {
        FibDirection::StartToEnd => (start.value, end.value),
        FibDirection::EndToStart => (end.value, start.value),
    };
    // Exact endpoint preservation (`ratio == 0.0` -> `v0`, `ratio == 1.0` -> `v1`) without `(v1 - v0)` overflow!
    (1.0 - ratio) * v0 + ratio * v1
}

/// Exact `f64` Liang-Barsky parametric line/ray/segment clipper over $t \in [t_{\min}, t_{\max}]$.
///
/// Supports finite segments ($[0, 1]$), semi-infinite $360^\circ$ rays ($[0, +\infty)$), vertical/horizontal
/// slopes (`dx == 0` or `dy == 0`), sub-pixel non-zero directions, extreme `±f64::MAX` endpoint spans,
/// and degenerate zero-length segments (`p0 == p1`) without phantom edge rays, `NaN`, or `±Inf`.
#[must_use]
pub fn clip_parametric_line(
    p0: LocalPoint,
    p1: LocalPoint,
    mut t_min: f64,
    mut t_max: f64,
    bounds: LocalRect,
) -> Option<LocalSegment> {
    if !p0.is_finite()
        || !p1.is_finite()
        || t_min.is_nan()
        || t_max.is_nan()
        || t_min == f64::INFINITY
        || t_max == f64::NEG_INFINITY
        || bounds.left > bounds.right
        || bounds.top > bounds.bottom
        || !bounds.left.is_finite()
        || !bounds.right.is_finite()
        || !bounds.top.is_finite()
        || !bounds.bottom.is_finite()
        || t_min > t_max
    {
        return None;
    }

    if p0.x == p1.x && p0.y == p1.y {
        return bounds.contains(p0).then_some(LocalSegment::new(p0, p0));
    }

    let raw_dx = p1.x - p0.x;
    let raw_dy = p1.y - p0.y;

    #[derive(Clone, Copy)]
    enum ClipEdge {
        Left,
        Right,
        Top,
        Bottom,
    }

    let snap_edge = |mut pt: LocalPoint, edge: Option<ClipEdge>| -> LocalPoint {
        match edge {
            Some(ClipEdge::Left) => pt.x = bounds.left,
            Some(ClipEdge::Right) => pt.x = bounds.right,
            Some(ClipEdge::Top) => pt.y = bounds.top,
            Some(ClipEdge::Bottom) => pt.y = bounds.bottom,
            None => {}
        }
        pt
    };

    // Branch 1: Unbounded ray or infinite line (`t_min == -Inf` and/or `t_max == +Inf`).
    // Normalize direction to a unit vector `(ux, uy)` so tiny non-zero slopes never collapse to `0 px`
    // and near-axis slopes (`0 < |dx| <= 1e-12`) clip accurately against all 4 box edges.
    if t_min.is_infinite() || t_max.is_infinite() {
        let (ux, uy, step_len) = if raw_dx.is_finite() && raw_dy.is_finite() {
            let len = raw_dx.hypot(raw_dy);
            if len == 0.0 || !len.is_finite() {
                return bounds.contains(p0).then_some(LocalSegment::new(p0, p0));
            }
            (raw_dx / len, raw_dy / len, len)
        } else {
            let hdx = 0.5 * p1.x - 0.5 * p0.x;
            let hdy = 0.5 * p1.y - 0.5 * p0.y;
            let hlen = hdx.hypot(hdy);
            if hlen == 0.0 || !hlen.is_finite() {
                return None;
            }
            (hdx / hlen, hdy / hlen, 2.0 * hlen)
        };
        let mut s_min = if t_min == f64::NEG_INFINITY {
            f64::NEG_INFINITY
        } else if t_min == 0.0 {
            0.0
        } else {
            let v = t_min * step_len;
            if !v.is_finite() {
                return None;
            }
            v
        };
        let mut s_max = if t_max == f64::INFINITY {
            f64::INFINITY
        } else if t_max == 0.0 {
            0.0
        } else {
            let v = t_max * step_len;
            if !v.is_finite() {
                return None;
            }
            v
        };
        let mut min_edge = None;
        let mut max_edge = None;
        let checks = [
            (ClipEdge::Left, -ux, p0.x - bounds.left),
            (ClipEdge::Right, ux, bounds.right - p0.x),
            (ClipEdge::Top, -uy, p0.y - bounds.top),
            (ClipEdge::Bottom, uy, bounds.bottom - p0.y),
        ];
        for (edge, p, q) in checks {
            if p == 0.0 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    if r > s_max {
                        return None;
                    }
                    if r > s_min {
                        s_min = r;
                        min_edge = Some(edge);
                    }
                } else {
                    if r < s_min {
                        return None;
                    }
                    if r < s_max {
                        s_max = r;
                        max_edge = Some(edge);
                    }
                }
            }
        }
        if !s_min.is_finite() || !s_max.is_finite() || s_min > s_max {
            return None;
        }
        let raw_s_start = if min_edge.is_none() && t_min == 0.0 {
            p0
        } else if min_edge.is_none() && t_min == 1.0 {
            p1
        } else {
            LocalPoint::new(p0.x + s_min * ux, p0.y + s_min * uy)
        };
        let raw_s_end = if max_edge.is_none() && t_max == 0.0 {
            p0
        } else if max_edge.is_none() && t_max == 1.0 {
            p1
        } else {
            LocalPoint::new(p0.x + s_max * ux, p0.y + s_max * uy)
        };
        let start = snap_edge(
            LocalPoint::new(
                raw_s_start.x.clamp(bounds.left, bounds.right),
                raw_s_start.y.clamp(bounds.top, bounds.bottom),
            ),
            min_edge,
        );
        let end = snap_edge(
            LocalPoint::new(
                raw_s_end.x.clamp(bounds.left, bounds.right),
                raw_s_end.y.clamp(bounds.top, bounds.bottom),
            ),
            max_edge,
        );
        return (start.is_finite() && end.is_finite()).then_some(LocalSegment::new(start, end));
    }

    // Branch 2: Finite parameter range `[t_min, t_max]`. Use half-scaled differences if `p1 - p0` overflows `f64::MAX`.
    let use_half_scale = !raw_dx.is_finite() || !raw_dy.is_finite();
    let (dx, dy, q_left, q_right, q_top, q_bottom) = if use_half_scale {
        (
            0.5 * p1.x - 0.5 * p0.x,
            0.5 * p1.y - 0.5 * p0.y,
            0.5 * p0.x - 0.5 * bounds.left,
            0.5 * bounds.right - 0.5 * p0.x,
            0.5 * p0.y - 0.5 * bounds.top,
            0.5 * bounds.bottom - 0.5 * p0.y,
        )
    } else {
        (
            raw_dx,
            raw_dy,
            p0.x - bounds.left,
            bounds.right - p0.x,
            p0.y - bounds.top,
            bounds.bottom - p0.y,
        )
    };

    let mut min_edge = None;
    let mut max_edge = None;
    let checks = [
        (ClipEdge::Left, -dx, q_left),
        (ClipEdge::Right, dx, q_right),
        (ClipEdge::Top, -dy, q_top),
        (ClipEdge::Bottom, dy, q_bottom),
    ];
    for (edge, p, q) in checks {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t_max {
                    return None;
                }
                if r > t_min {
                    t_min = r;
                    min_edge = Some(edge);
                }
            } else {
                if r < t_min {
                    return None;
                }
                if r < t_max {
                    t_max = r;
                    max_edge = Some(edge);
                }
            }
        }
    }

    if !t_min.is_finite() || !t_max.is_finite() || t_min > t_max {
        return None;
    }

    let eval_pt = |t: f64| -> LocalPoint {
        if use_half_scale {
            LocalPoint::new(
                (2.0 * (0.5 * p0.x + t * dx)).clamp(bounds.left, bounds.right),
                (2.0 * (0.5 * p0.y + t * dy)).clamp(bounds.top, bounds.bottom),
            )
        } else {
            LocalPoint::new(
                (p0.x + t * dx).clamp(bounds.left, bounds.right),
                (p0.y + t * dy).clamp(bounds.top, bounds.bottom),
            )
        }
    };
    let start = snap_edge(eval_pt(t_min), min_edge);
    let end = snap_edge(eval_pt(t_max), max_edge);
    (start.is_finite() && end.is_finite()).then_some(LocalSegment::new(start, end))
}

/// Computes causal `ΔPrice`, `Δ%`, bar count, elapsed duration, and total volume for a
/// [`DrawingGeometry::PriceDateRange`] across the currently visible candle slice (`v1.1 / Wave 2`).
#[must_use]
pub fn compute_price_date_range_metrics(
    start: DrawingAnchor,
    end: DrawingAnchor,
    candles: Option<&[Candle]>,
) -> PriceDateRangeMetrics {
    let price_delta = end.value - start.value;
    let price_pct = if start.value.is_finite() && start.value.abs() > 1.0e-12 {
        let pct = (price_delta / start.value.abs()) * 100.0;
        pct.is_finite().then_some(pct)
    } else {
        None
    };
    let b0 = start.bar_index.round();
    let b1 = end.bar_index.round();
    let bar_count = if b0.is_finite() && b1.is_finite() {
        (b1 - b0).abs().round().min(1.0e9) as usize
    } else {
        0
    };

    let resolve_ts = |a: DrawingAnchor| -> Option<i64> {
        if let Some(ts) = a.timestamp
            && ts.is_finite()
        {
            return Some(ts.round() as i64);
        }
        let cs = candles?;
        if cs.is_empty() || !a.bar_index.is_finite() {
            return None;
        }
        let idx = a.bar_index.round().clamp(0.0, (cs.len() - 1) as f64) as usize;
        Some(cs[idx].timestamp)
    };
    let elapsed_secs = resolve_ts(start)
        .zip(resolve_ts(end))
        .map(|(t0, t1)| (t1.saturating_sub(t0)).abs());

    let total_volume = candles.and_then(|cs| {
        if cs.is_empty() || !b0.is_finite() || !b1.is_finite() {
            return None;
        }
        let lo_f = b0.min(b1);
        let hi_f = b0.max(b1);
        let max_i = (cs.len() - 1) as f64;
        if hi_f < 0.0 || lo_f > max_i {
            return None;
        }
        let i0 = lo_f.clamp(0.0, max_i) as usize;
        let i1 = hi_f.clamp(0.0, max_i) as usize;
        let mut sum = 0.0_f64;
        let mut any_finite = false;
        for c in &cs[i0..=i1] {
            if c.volume.is_finite() && c.volume >= 0.0 {
                sum += c.volume;
                any_finite = true;
            }
        }
        (any_finite && sum.is_finite()).then_some(sum)
    });

    PriceDateRangeMetrics {
        price_delta: if price_delta.is_finite() {
            price_delta
        } else {
            0.0
        },
        price_pct,
        bar_count,
        elapsed_secs,
        total_volume,
        is_bullish: price_delta >= 0.0,
    }
}

/// Formats an elapsed duration in seconds into compact TradingView notation (`61d`, `3d 4h`, `6h 30m`, `45m`).
#[must_use]
pub fn format_measure_duration(secs: i64) -> String {
    let abs = secs.unsigned_abs();
    if abs < 60 {
        return format!("{abs}s");
    }
    let total_mins = abs / 60;
    if total_mins < 60 {
        return format!("{total_mins}m");
    }
    let total_hours = total_mins / 60;
    let rem_mins = total_mins % 60;
    if total_hours < 24 {
        if rem_mins == 0 {
            return format!("{total_hours}h");
        }
        return format!("{total_hours}h {rem_mins}m");
    }
    let days = total_hours / 24;
    let rem_hours = total_hours % 24;
    if rem_hours == 0 {
        format!("{days}d")
    } else {
        format!("{days}d {rem_hours}h")
    }
}

/// Exact Sutherland-Hodgman convex polygon clip against an axis-aligned [`LocalRect`].
#[must_use]
pub(crate) fn clip_polygon_to_rect(
    mut poly: Vec<LocalPoint>,
    bounds: LocalRect,
) -> Vec<LocalPoint> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let clip_edge = |pts: Vec<LocalPoint>,
                     inside: fn(LocalPoint, LocalRect) -> bool,
                     intersect: fn(LocalPoint, LocalPoint, LocalRect) -> LocalPoint|
     -> Vec<LocalPoint> {
        let mut out = Vec::with_capacity(pts.len() + 1);
        if pts.is_empty() {
            return out;
        }
        let mut prev = *pts.last().unwrap();
        let mut prev_in = inside(prev, bounds);
        for &curr in &pts {
            let curr_in = inside(curr, bounds);
            if curr_in {
                if !prev_in {
                    out.push(intersect(prev, curr, bounds));
                }
                out.push(curr);
            } else if prev_in {
                out.push(intersect(prev, curr, bounds));
            }
            prev = curr;
            prev_in = curr_in;
        }
        out
    };

    poly = clip_edge(
        poly,
        |p, b| p.x >= b.left,
        |a, c, b| {
            let t = if (c.x - a.x).abs() > 1e-12 {
                (b.left - a.x) / (c.x - a.x)
            } else {
                0.0
            };
            LocalPoint::new(b.left, a.y + t * (c.y - a.y))
        },
    );
    poly = clip_edge(
        poly,
        |p, b| p.x <= b.right,
        |a, c, b| {
            let t = if (c.x - a.x).abs() > 1e-12 {
                (b.right - a.x) / (c.x - a.x)
            } else {
                0.0
            };
            LocalPoint::new(b.right, a.y + t * (c.y - a.y))
        },
    );
    poly = clip_edge(
        poly,
        |p, b| p.y >= b.top,
        |a, c, b| {
            let t = if (c.y - a.y).abs() > 1e-12 {
                (b.top - a.y) / (c.y - a.y)
            } else {
                0.0
            };
            LocalPoint::new(a.x + t * (c.x - a.x), b.top)
        },
    );
    poly = clip_edge(
        poly,
        |p, b| p.y <= b.bottom,
        |a, c, b| {
            let t = if (c.y - a.y).abs() > 1e-12 {
                (b.bottom - a.y) / (c.y - a.y)
            } else {
                0.0
            };
            LocalPoint::new(a.x + t * (c.x - a.x), b.bottom)
        },
    );
    if poly.len() < 3 || !poly.iter().all(|p| p.is_finite()) {
        Vec::new()
    } else {
        poly
    }
}

/// Euclidean distance from `point` to finite `segment` (`drawingGeometry.ts:8-13`).
#[inline]
#[must_use]
pub fn distance_to_segment(point: LocalPoint, segment: LocalSegment) -> f64 {
    let dx = segment.end.x - segment.start.x;
    let dy = segment.end.y - segment.start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= 1e-18 || !len_sq.is_finite() {
        return point.distance(segment.start);
    }
    let t = (((point.x - segment.start.x) * dx + (point.y - segment.start.y) * dy) / len_sq)
        .clamp(0.0, 1.0);
    let proj = LocalPoint::new(segment.start.x + t * dx, segment.start.y + t * dy);
    point.distance(proj)
}
