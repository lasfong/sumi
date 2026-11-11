use crate::model::Candle;

/// Generates a deterministic, realistic daily OHLCV dataset for the `"VNINDEX"` benchmark series.
///
/// No external network or random state is used; every invocation returns the exact
/// same sequence of candles so tests, benchmarks (10K / 50K / 100K bars), and visual
/// verifications are 100% reproducible in $O(N)$ time.
#[must_use]
pub fn generate_synthetic_ohlcv(count: usize) -> Vec<Candle> {
    let total = count.max(10);
    let mut candles = Vec::with_capacity(total);

    // Base timestamp: 2026-05-04 00:00:00 UTC
    let base_ts: i64 = 1_777_852_800;
    let day_seconds: i64 = 86_400;

    let mut prev_close = 1_215.0_f64;
    let mut calendar = TradingDayCursor::new_2026_05_01();

    for i in 0..total {
        let t = i as f64;

        // Multi-regime deterministic wave:
        // - Bars 0..120 preserve 100% bit-for-bit parity with Session 01 baseline.
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

        let timestamp = base_ts + (i as i64) * day_seconds;
        let date_label = calendar.next_trading_date_label();

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

/// Incremental $O(1)$-per-bar Gregorian weekday calendar starting at `2026-05-01` (Friday).
struct TradingDayCursor {
    year: u32,
    month: u32,
    day: u32,
    /// `0 = Mon .. 4 = Fri, 5 = Sat, 6 = Sun`
    weekday: u32,
}

impl TradingDayCursor {
    const fn new_2026_05_01() -> Self {
        Self {
            year: 2026,
            month: 5,
            day: 1,
            weekday: 4,
        }
    }

    fn next_trading_date_label(&mut self) -> String {
        while self.weekday >= 5 {
            self.advance_calendar_day();
        }
        let label = format!("{:04}-{:02}-{:02}", self.year, self.month, self.day);
        self.advance_calendar_day();
        label
    }

    fn advance_calendar_day(&mut self) {
        self.weekday = (self.weekday + 1) % 7;
        self.day += 1;
        if self.day > days_in_month(self.year, self.month) {
            self.day = 1;
            self.month += 1;
            if self.month > 12 {
                self.month = 1;
                self.year += 1;
            }
        }
    }
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}
