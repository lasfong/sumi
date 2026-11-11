use serde::{Deserialize, Serialize};

use super::price_scale::PriceScale;
use super::time_scale::TimeScale;

/// Precision `f64` rectangle representing a pane's plot region in screen space.
///
/// Decouples core `Logical <-> PaneLocal <-> Screen` math from `egui::Rect` (`f32`) while providing
/// zero-cost conversions at the UI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlotRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl PlotRect {
    #[inline]
    #[must_use]
    pub const fn new(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    #[inline]
    #[must_use]
    pub fn from_egui(rect: egui::Rect) -> Self {
        Self {
            left: f64::from(rect.min.x),
            top: f64::from(rect.min.y),
            width: f64::from(rect.width().max(0.0)),
            height: f64::from(rect.height().max(0.0)),
        }
    }

    #[inline]
    #[must_use]
    pub fn to_egui(self) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(self.left as f32, self.top as f32),
            egui::vec2(self.width.max(0.0) as f32, self.height.max(0.0) as f32),
        )
    }

    #[inline]
    #[must_use]
    pub fn right(&self) -> f64 {
        self.left + self.width
    }

    #[inline]
    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.top + self.height
    }

    #[inline]
    #[must_use]
    pub fn contains_screen(&self, screen_x: f64, screen_y: f64) -> bool {
        screen_x >= self.left
            && screen_x <= self.right()
            && screen_y >= self.top
            && screen_y <= self.bottom()
    }
}

/// Non-monolithic coordinate bridge binding the chart-wide shared `TimeScale`, a pane/axis-specific
/// `PriceScale`, and the pane's screen-space `PlotRect`.
///
/// Enforces explicit separation between:
/// 1. **Logical / Data coordinates:** `(bar_index: i64 / f64, price: f64)`
/// 2. **Pane-local coordinates:** `(local_x ∈ [0, width), local_y ∈ [0, height))`
/// 3. **Screen coordinates:** `(screen_x = plot_rect.left + local_x, screen_y = plot_rect.top + local_y)`
#[derive(Debug, Clone, Copy)]
pub struct PaneViewport<'a> {
    pub time_scale: &'a TimeScale,
    pub price_scale: &'a PriceScale,
    pub plot_rect: PlotRect,
    /// First visible series value used when `price_scale` is in `Percentage` or `IndexedTo100` mode.
    pub base_value: f64,
}

impl<'a> PaneViewport<'a> {
    #[inline]
    #[must_use]
    pub const fn new(
        time_scale: &'a TimeScale,
        price_scale: &'a PriceScale,
        plot_rect: PlotRect,
        base_value: f64,
    ) -> Self {
        Self {
            time_scale,
            price_scale,
            plot_rect,
            base_value,
        }
    }

    #[inline]
    #[must_use]
    pub fn screen_x_to_local_x(&self, screen_x: f64) -> f64 {
        screen_x - self.plot_rect.left
    }

    #[inline]
    #[must_use]
    pub fn local_x_to_screen_x(&self, local_x: f64) -> f64 {
        self.plot_rect.left + local_x
    }

    #[inline]
    #[must_use]
    pub fn screen_y_to_local_y(&self, screen_y: f64) -> f64 {
        screen_y - self.plot_rect.top
    }

    #[inline]
    #[must_use]
    pub fn local_y_to_screen_y(&self, local_y: f64) -> f64 {
        self.plot_rect.top + local_y
    }

    #[inline]
    #[must_use]
    pub fn index_to_screen_x(&self, index: i64) -> Option<f64> {
        self.time_scale
            .index_to_local_x(index)
            .map(|lx| self.local_x_to_screen_x(lx))
    }

    #[inline]
    #[must_use]
    pub fn float_index_to_screen_x(&self, center_index: f64) -> Option<f64> {
        self.time_scale
            .float_index_to_local_x(center_index)
            .map(|lx| self.local_x_to_screen_x(lx))
    }

    #[inline]
    #[must_use]
    pub fn screen_x_to_float_index(&self, screen_x: f64) -> Option<f64> {
        self.time_scale
            .local_x_to_float_index(self.screen_x_to_local_x(screen_x))
    }

    #[inline]
    #[must_use]
    pub fn screen_x_to_index(&self, screen_x: f64) -> Option<i64> {
        self.time_scale
            .local_x_to_index(self.screen_x_to_local_x(screen_x))
    }

    #[inline]
    #[must_use]
    pub fn screen_x_to_clamped_data_index(&self, screen_x: f64) -> Option<usize> {
        self.time_scale
            .local_x_to_clamped_data_index(self.screen_x_to_local_x(screen_x))
    }

    #[inline]
    #[must_use]
    pub fn price_to_screen_y(&self, price: f64) -> Option<f64> {
        self.price_scale
            .price_to_local_y(price, self.base_value)
            .map(|ly| self.local_y_to_screen_y(ly))
    }

    #[inline]
    #[must_use]
    pub fn screen_y_to_price(&self, screen_y: f64) -> Option<f64> {
        self.price_scale
            .local_y_to_price(self.screen_y_to_local_y(screen_y), self.base_value)
    }

    #[inline]
    #[must_use]
    pub fn data_to_screen_pos(&self, center_index: f64, price: f64) -> Option<egui::Pos2> {
        let sx = self.float_index_to_screen_x(center_index)?;
        let sy = self.price_to_screen_y(price)?;
        Some(egui::pos2(sx as f32, sy as f32))
    }
}
