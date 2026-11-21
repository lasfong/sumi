use std::f64::consts::PI;
use std::ops::Range;

use crate::model::Candle;

/// Computes LWC's bar-chart column width in physical bitmap pixels (`src/renderers/optimal-bar-width.ts:1-3`).
#[inline]
#[must_use]
pub fn optimal_bar_width_px(bar_spacing: f64, pixel_ratio: f64) -> f64 {
    let dpr = sanitize_dpr(pixel_ratio);
    let bs = bar_spacing.max(0.0);
    (bs * 0.3 * dpr).floor().max(1.0)
}

/// Computes LWC's candlestick body width in physical bitmap pixels with wick parity alignment
/// (`src/renderers/optimal-bar-width.ts:5-19` + `src/renderers/candlesticks-renderer.ts:48-59`).
#[inline]
#[must_use]
pub fn optimal_candlestick_width_px(bar_spacing: f64, pixel_ratio: f64) -> f64 {
    let dpr = sanitize_dpr(pixel_ratio);
    let bs = if bar_spacing.is_finite() && bar_spacing > 0.0 {
        bar_spacing
    } else {
        1.0
    };

    let raw_px = if (2.5..=4.0).contains(&bs) {
        (3.0 * dpr).floor()
    } else {
        let reducing_coeff = 0.2;
        let coeff = 1.0 - reducing_coeff * (bs.max(4.0) - 4.0).atan() / (PI * 0.5);
        let res = (bs * coeff * dpr).floor();
        let scaled_bs = (bs * dpr).floor();
        res.min(scaled_bs).max(dpr.floor().max(1.0))
    };

    let mut body_px = raw_px.max(1.0) as i64;
    let wick_px = (dpr.floor() as i64).max(1);
    if body_px >= 2 && (wick_px % 2) != (body_px % 2) {
        body_px -= 1;
    }
    body_px.max(1) as f64
}

/// Logical-point candlestick and wick stroke widths ready for `egui::Painter` submission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandlestickBarMetrics {
    pub body_width_logical: f64,
    pub wick_width_logical: f64,
}

impl CandlestickBarMetrics {
    #[inline]
    #[must_use]
    pub fn compute(bar_spacing: f64, pixels_per_point: f64) -> Self {
        let dpr = sanitize_dpr(pixels_per_point);
        let body_px = optimal_candlestick_width_px(bar_spacing, dpr);
        let wick_px = dpr
            .floor()
            .min((bar_spacing.max(0.0) * dpr).floor().max(1.0))
            .max(1.0)
            .min(body_px);
        Self {
            body_width_logical: (body_px / dpr).max(1.0 / dpr),
            wick_width_logical: (wick_px / dpr).max(1.0 / dpr),
        }
    }
}

/// Aggregated OHLCV candle produced on-the-fly when `conflation_factor > 1` (`bar_spacing < 1.0 / dpr`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConflatedCandle<'a> {
    pub start_index: usize,
    pub end_index: usize,
    pub center_logical_index: f64,
    pub timestamp: i64,
    pub date_label: &'a str,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl<'a> ConflatedCandle<'a> {
    #[inline]
    #[must_use]
    pub fn is_bullish(&self) -> bool {
        self.close >= self.open
    }

    #[inline]
    #[must_use]
    pub fn change(&self) -> f64 {
        self.close - self.open
    }

    #[inline]
    #[must_use]
    pub fn change_pct(&self) -> f64 {
        if self.open.abs() < f64::EPSILON {
            0.0
        } else {
            ((self.close - self.open) / self.open) * 100.0
        }
    }
}

/// Zero-allocation iterator yielding conflated (or 1:1) OHLCV candles across `slice_range`.
pub struct ConflatedCandlesIter<'a> {
    candles: &'a [Candle],
    cursor: usize,
    end: usize,
    step: usize,
}

impl<'a> Iterator for ConflatedCandlesIter<'a> {
    type Item = ConflatedCandle<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.end {
            return None;
        }
        let start = self.cursor;
        let chunk_end = (start + self.step).min(self.end);
        self.cursor = chunk_end;

        let first = &self.candles[start];
        if chunk_end - start == 1 {
            return Some(ConflatedCandle {
                start_index: start,
                end_index: start,
                center_logical_index: start as f64,
                timestamp: first.timestamp,
                date_label: &first.date_label,
                open: first.open,
                high: first.high,
                low: first.low,
                close: first.close,
                volume: first.volume,
            });
        }

        let last = &self.candles[chunk_end - 1];
        let mut high = if first.high.is_finite() {
            first.high
        } else {
            f64::NEG_INFINITY
        };
        let mut low = if first.low.is_finite() {
            first.low
        } else {
            f64::INFINITY
        };
        let mut volume = 0.0;
        for c in &self.candles[start..chunk_end] {
            if c.high.is_finite() {
                high = high.max(c.high);
            }
            if c.low.is_finite() {
                low = low.min(c.low);
            }
            if c.volume.is_finite() && c.volume >= 0.0 {
                volume += c.volume;
            }
        }
        if !high.is_finite() {
            high = first.high;
        }
        if !low.is_finite() {
            low = first.low;
        }
        let end_idx = chunk_end - 1;
        Some(ConflatedCandle {
            start_index: start,
            end_index: end_idx,
            center_logical_index: (start + end_idx) as f64 * 0.5,
            timestamp: last.timestamp,
            date_label: &last.date_label,
            open: first.open,
            high,
            low,
            close: last.close,
            volume,
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.end.saturating_sub(self.cursor).div_ceil(self.step);
        (rem, Some(rem))
    }
}

/// Iterates visible `candles[slice_range]` merged in aligned blocks of `conflation_factor >= 1`
/// with **zero heap allocations**.
#[inline]
pub fn iter_conflated_candles(
    candles: &[Candle],
    slice_range: Range<usize>,
    conflation_factor: usize,
) -> ConflatedCandlesIter<'_> {
    let step = conflation_factor.max(1);
    let clamped_end = slice_range.end.min(candles.len());
    let raw_start = slice_range.start.min(clamped_end);
    // Align BOTH start and end chunk boundaries to multiples of `step` (bounded by `candles.len()`)
    // so 1px panning never truncates or shimmers either edge's conflated OHLCV bucket!
    let (aligned_start, aligned_end) = if step > 1 && raw_start < clamped_end {
        let s = (raw_start / step) * step;
        let e = clamped_end
            .div_ceil(step)
            .saturating_mul(step)
            .min(candles.len());
        (s, e)
    } else {
        (raw_start, clamped_end)
    };
    ConflatedCandlesIter {
        candles,
        cursor: aligned_start,
        end: aligned_end,
        step,
    }
}

/// Computes the single conflated (or 1:1) OHLCV bucket containing `data_index` in $O(\text{conflation\_factor})$.
#[inline]
#[must_use]
pub fn conflated_candle_at(
    candles: &[Candle],
    data_index: usize,
    conflation_factor: usize,
) -> Option<ConflatedCandle<'_>> {
    if data_index >= candles.len() {
        return None;
    }
    let step = conflation_factor.max(1);
    let start = (data_index / step) * step;
    let end = (start + step).min(candles.len());
    iter_conflated_candles(candles, start..end, step).next()
}

#[inline]
fn sanitize_dpr(dpr: f64) -> f64 {
    if dpr.is_finite() && dpr > 0.0 {
        dpr
    } else {
        1.0
    }
}
