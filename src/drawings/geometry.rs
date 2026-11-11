use crate::viewport::{PriceScale, TimeScale};

use super::{Drawing, DrawingAnchor, DrawingGeometry, FIBONACCI_RATIOS, FibDirection};

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
    Rectangle {
        raw_rect: LocalRect,
        clipped_fill: Option<LocalRect>,
        clipped_edges: [Option<LocalSegment>; 4],
    },
    Fibonacci {
        control_segment: Option<LocalSegment>,
        levels: Vec<ProjectedFibLevel>,
        bands: Vec<ProjectedFibBand>,
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
        }
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
                let clipped = clip_parametric_line(p0, p1, 0.0, 1.0, bounds);
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

                let mut levels = Vec::with_capacity(FIBONACCI_RATIOS.len());
                for ratio in FIBONACCI_RATIOS {
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
        || !t_min.is_finite()
        || t_max.is_nan()
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

    // Branch 1: Semi-infinite ray (`t_max == +Inf`). Normalize direction to a unit vector `(ux, uy)`
    // so tiny non-zero slopes never collapse to `0 px` and near-axis slopes (`0 < |dx| <= 1e-12`)
    // are clipped accurately against all 4 box edges (preventing phantom edge rays).
    if t_max.is_infinite() {
        if t_max.is_sign_negative() {
            return None;
        }
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
        let mut s_min = if t_min == 0.0 { 0.0 } else { t_min * step_len };
        if !s_min.is_finite() {
            return None;
        }
        let mut s_max = f64::INFINITY;
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
        let start = snap_edge(
            LocalPoint::new(
                (p0.x + s_min * ux).clamp(bounds.left, bounds.right),
                (p0.y + s_min * uy).clamp(bounds.top, bounds.bottom),
            ),
            min_edge,
        );
        let end = snap_edge(
            LocalPoint::new(
                (p0.x + s_max * ux).clamp(bounds.left, bounds.right),
                (p0.y + s_max * uy).clamp(bounds.top, bounds.bottom),
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
