use crate::journal::is_vn_exchange_trading_day_days;
use crate::model::Candle;

use super::calendar::{days_from_civil, format_utc_date};

/// Generates a deterministic, realistic daily OHLCV dataset for the `"VNINDEX"` benchmark series.
///
/// Every bar's `timestamp` (`00:00 UTC` of the trading session) and `date_label` (`YYYY-MM-DD`) are
/// synchronized from the same `VIETNAM_HOLIDAYS_V1`-aware Vietnam exchange trading calendar starting
/// on Monday `2026-05-04` (eliminating weekend/holiday bars and multi-year timestamp drift, `BUG-025`).
#[must_use]
pub fn generate_synthetic_ohlcv(count: usize) -> Vec<Candle> {
    let total = count.max(10);
    let mut candles = Vec::with_capacity(total);

    // Start on Monday 2026-05-04 (first valid VN exchange trading session after 30/04–01/05 holidays).
    let mut day_cursor = days_from_civil(2026, 5, 4);
    let mut prev_close = 1_215.0_f64;

    for i in 0..total {
        while !is_vn_exchange_trading_day_days(day_cursor) {
            day_cursor = day_cursor.saturating_add(1);
        }
        let t = i as f64;

        // Multi-regime deterministic wave:
        // - Bars 0..120 preserve realistic multi-swing baseline.
        // - Bars 120.. cycle realistic multi-month bull/bear regimes for 10K/50K/100K benchmarks.
        let regime_drift = if i < 35 {
            1.65
        } else if i < 65 {
            -0.95
        } else if i < 100 {
            2.15
        } else if i < 120 {
            0.35
        } else {
            let phase = (i - 120) % 400;
            if phase < 140 {
                0.55
            } else if phase < 230 {
                -0.65
            } else if phase < 340 {
                0.72
            } else {
                -0.38
            }
        };

        let swing = (t * 0.37).sin() * 6.8 + (t * 0.83).cos() * 4.4 + (t * 1.71).sin() * 3.1;
        let gap = ((t * 1.13).cos() * 1.8) - ((t * 0.53).sin() * 1.1);

        let open = round_price((prev_close + gap).max(900.0));
        let delta = regime_drift + swing * 0.55;
        let close = round_price((open + delta).max(900.0));

        let body_top = open.max(close);
        let body_bottom = open.min(close);

        // Deterministic upper and lower wicks
        let upper_wick = 2.2 + ((t * 1.31).sin().abs() * 7.4) + ((i % 5) as f64) * 0.9;
        let lower_wick = 2.0 + ((t * 0.97).cos().abs() * 6.9) + ((i % 7) as f64) * 0.8;

        let high = round_price(body_top + upper_wick);
        let low = round_price((body_bottom - lower_wick).max(850.0));

        let vol_base = 420_000.0;
        let vol_impulse = (close - open).abs() * 24_000.0 + (high - low) * 11_500.0;
        let vol_cycle = ((t * 0.49).sin().abs() * 165_000.0).round();
        let volume = (vol_base + vol_impulse + vol_cycle).round();

        let timestamp = day_cursor.saturating_mul(86_400);
        let date_label = format_utc_date(timestamp);
        day_cursor = day_cursor.saturating_add(1);

        candles.push(Candle {
            timestamp,
            date_label,
            open,
            high,
            low,
            close,
            volume,
        });

        prev_close = close;
    }

    candles
}

fn round_price(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
