use serde::{Deserialize, Serialize};

/// A single OHLCV candlestick bar.
///
/// Pure domain type designed to be 100% compatible with both native desktop
/// rendering and future `wasm32-unknown-unknown` targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    /// Unix epoch timestamp in seconds (UTC).
    pub timestamp: i64,
    /// Human-readable date/time label for axis display (e.g. `"2026-08-14"`).
    pub date_label: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    /// Returns `true` when the candle closed at or above its open (`close >= open`).
    #[must_use]
    pub fn is_bullish(&self) -> bool {
        self.close >= self.open
    }

    /// Signed price change (`close - open`).
    #[must_use]
    pub fn change(&self) -> f64 {
        self.close - self.open
    }

    /// Percentage change from open (`((close - open) / open) * 100.0`).
    #[must_use]
    pub fn change_pct(&self) -> f64 {
        if self.open.abs() < f64::EPSILON {
            0.0
        } else {
            ((self.close - self.open) / self.open) * 100.0
        }
    }

    /// Top of the candle body (`max(open, close)`).
    #[must_use]
    pub fn body_top(&self) -> f64 {
        self.open.max(self.close)
    }

    /// Bottom of the candle body (`min(open, close)`).
    #[must_use]
    pub fn body_bottom(&self) -> f64 {
        self.open.min(self.close)
    }

    /// Verifies OHLCV structural invariants (`low <= body_bottom <= body_top <= high` and `volume >= 0`).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.open.is_finite()
            && self.high.is_finite()
            && self.low.is_finite()
            && self.close.is_finite()
            && self.volume.is_finite()
            && self.volume >= 0.0
            && self.low <= self.body_bottom()
            && self.body_top() <= self.high
    }
}

/// Price bounds for a visible slice of candles, including vertical padding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PriceBounds {
    pub raw_min: f64,
    pub raw_max: f64,
    pub padded_min: f64,
    pub padded_max: f64,
    pub max_volume: f64,
}

impl PriceBounds {
    /// Constructs `PriceBounds` directly from precomputed finite raw `[raw_min, raw_max]` and `max_volume` extents.
    #[must_use]
    pub fn from_raw_extents(
        raw_min: f64,
        raw_max: f64,
        max_volume: f64,
        padding_ratio: f64,
    ) -> Option<Self> {
        if !raw_min.is_finite() || !raw_max.is_finite() || raw_min > raw_max {
            return None;
        }
        let safe_vol = if max_volume.is_finite() {
            max_volume.max(1.0)
        } else {
            1.0
        };
        let span = (raw_max - raw_min).max(1.0);
        let pad = span * padding_ratio.clamp(0.0, 0.5);

        Some(Self {
            raw_min,
            raw_max,
            padded_min: raw_min - pad,
            padded_max: raw_max + pad,
            max_volume: safe_vol,
        })
    }

    /// Computes price and volume bounds across a slice of candles with a configurable vertical padding ratio,
    /// skipping any non-finite values safely.
    #[must_use]
    pub fn from_candles(candles: &[Candle], padding_ratio: f64) -> Option<Self> {
        if candles.is_empty() {
            return None;
        }
        let mut raw_min = f64::INFINITY;
        let mut raw_max = f64::NEG_INFINITY;
        let mut max_volume = 0.0_f64;

        for candle in candles {
            if candle.low.is_finite() && candle.low < raw_min {
                raw_min = candle.low;
            }
            if candle.high.is_finite() && candle.high > raw_max {
                raw_max = candle.high;
            }
            if candle.volume.is_finite() && candle.volume > max_volume {
                max_volume = candle.volume;
            }
        }

        Self::from_raw_extents(raw_min, raw_max, max_volume, padding_ratio)
    }

    #[must_use]
    pub fn span(&self) -> f64 {
        (self.padded_max - self.padded_min).max(1e-6)
    }
}

/// Pure, WebAssembly-compatible coordinate mapper between domain space (candle index, price)
/// and screen space (`x`, `y` pixels inside the chart plot area).
#[deprecated(
    note = "Superseded in Milestone R03 by crate::viewport::{TimeScale, PriceScale, PaneViewport}"
)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportMapping {
    pub plot_left: f32,
    pub plot_top: f32,
    pub plot_width: f32,
    pub plot_height: f32,
    pub visible_count: usize,
    pub min_price: f64,
    pub max_price: f64,
}

#[allow(deprecated)]
impl ViewportMapping {
    /// Creates a new viewport coordinate mapper.
    #[must_use]
    pub fn new(
        plot_left: f32,
        plot_top: f32,
        plot_width: f32,
        plot_height: f32,
        visible_count: usize,
        min_price: f64,
        max_price: f64,
    ) -> Self {
        Self {
            plot_left,
            plot_top,
            plot_width: plot_width.max(1.0),
            plot_height: plot_height.max(1.0),
            visible_count: visible_count.max(1),
            min_price,
            max_price: if (max_price - min_price).abs() < 1e-6 {
                min_price + 1.0
            } else {
                max_price
            },
        }
    }

    /// Horizontal step in pixels allocated to each visible candle slot.
    #[must_use]
    pub fn slot_width(&self) -> f32 {
        self.plot_width / (self.visible_count as f32)
    }

    /// Recommended candle body width in pixels (`68%` of slot width, clamped to `[2.0, 28.0]`).
    #[must_use]
    pub fn candle_body_width(&self) -> f32 {
        (self.slot_width() * 0.68).clamp(2.0, 28.0)
    }

    /// Maps a `0..visible_count` local candle index to the horizontal center `x` coordinate on screen.
    #[must_use]
    pub fn index_to_screen_x(&self, local_index: usize) -> f32 {
        self.plot_left + ((local_index as f32) + 0.5) * self.slot_width()
    }

    /// Maps a screen `x` coordinate back to the nearest visible local candle index (`0..visible_count`).
    #[must_use]
    pub fn screen_x_to_index(&self, screen_x: f32) -> Option<usize> {
        if self.visible_count == 0
            || screen_x < self.plot_left
            || screen_x > self.plot_left + self.plot_width
        {
            return None;
        }
        let rel_x = (screen_x - self.plot_left).clamp(0.0, self.plot_width - 0.001);
        let idx = (rel_x / self.slot_width()).floor() as usize;
        Some(idx.min(self.visible_count.saturating_sub(1)))
    }

    /// Maps a domain price to a vertical `y` screen coordinate (higher price -> smaller `y`).
    #[must_use]
    pub fn price_to_screen_y(&self, price: f64) -> f32 {
        let span = (self.max_price - self.min_price).max(1e-6);
        let normalized = ((self.max_price - price) / span) as f32;
        self.plot_top + normalized * self.plot_height
    }

    /// Maps a vertical `y` screen coordinate back to a domain price.
    #[must_use]
    pub fn screen_y_to_price(&self, screen_y: f32) -> f64 {
        let normalized = f64::from((screen_y - self.plot_top) / self.plot_height);
        self.max_price - normalized * (self.max_price - self.min_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: 1_750_000_000,
            date_label: "2026-06-15".to_owned(),
            open,
            high,
            low,
            close,
            volume: 12_500.0,
        }
    }

    #[test]
    fn candle_bullish_and_bearish_classification() {
        let bull = sample_candle(1250.0, 1268.0, 1245.0, 1262.0);
        assert!(bull.is_bullish());
        assert!(bull.is_valid());
        assert!((bull.change() - 12.0).abs() < 1e-9);

        let doji = sample_candle(1250.0, 1255.0, 1245.0, 1250.0);
        assert!(doji.is_bullish());
        assert!(doji.is_valid());

        let bear = sample_candle(1262.0, 1265.0, 1238.0, 1241.0);
        assert!(!bear.is_bullish());
        assert!(bear.is_valid());
        assert!((bear.change() + 21.0).abs() < 1e-9);
    }

    #[test]
    fn price_bounds_computes_expected_extents_and_padding() {
        let candles = vec![
            sample_candle(100.0, 110.0, 95.0, 105.0),
            sample_candle(105.0, 125.0, 102.0, 120.0),
            sample_candle(120.0, 122.0, 85.0, 90.0),
        ];
        let bounds = PriceBounds::from_candles(&candles, 0.10).expect("non-empty slice");
        assert!((bounds.raw_min - 85.0).abs() < 1e-9);
        assert!((bounds.raw_max - 125.0).abs() < 1e-9);
        // Span is 40.0, 10% padding is 4.0 on each side.
        assert!((bounds.padded_min - 81.0).abs() < 1e-9);
        assert!((bounds.padded_max - 129.0).abs() < 1e-9);
    }

    #[test]
    #[allow(deprecated)]
    fn viewport_mapping_round_trips_time_and_price() {
        let mapper = ViewportMapping::new(20.0, 40.0, 1000.0, 500.0, 100, 1100.0, 1300.0);

        // Top of plot corresponds to max_price (1300.0), bottom to min_price (1100.0).
        assert!((mapper.price_to_screen_y(1300.0) - 40.0).abs() < 1e-4);
        assert!((mapper.price_to_screen_y(1200.0) - 290.0).abs() < 1e-4);
        assert!((mapper.price_to_screen_y(1100.0) - 540.0).abs() < 1e-4);

        for price in [1100.0, 1142.5, 1200.0, 1278.25, 1300.0] {
            let y = mapper.price_to_screen_y(price);
            let recovered = mapper.screen_y_to_price(y);
            assert!((recovered - price).abs() < 1e-3);
        }

        for idx in [0, 1, 24, 50, 77, 99] {
            let x = mapper.index_to_screen_x(idx);
            let recovered_idx = mapper.screen_x_to_index(x).expect("inside plot");
            assert_eq!(recovered_idx, idx);
        }

        assert!(mapper.screen_x_to_index(10.0).is_none());
        assert!(mapper.screen_x_to_index(1030.0).is_none());
    }
}
