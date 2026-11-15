use serde::{Deserialize, Serialize};

use crate::data::{
    DataSourcePreset, MAX_SUPPORTED_EPOCH_SECS, MIN_SUPPORTED_EPOCH_SECS, weekday_from_days,
};

const TICK_EPS: f64 = 1.0e-9;
const QUANTIZE_SCALE: f64 = 1.0e8;

/// Exchange microstructure venue governing tick ladders, daily price bands, board lots,
/// contract multipliers, initial margin, settlement cycles, sell transfer taxes, and short-selling eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ExchangeVenue {
    #[default]
    Hose,
    Hnx,
    Upcom,
    Vn30Futures,
    VnIndexBenchmark,
    UsEquities,
    CryptoSpotPerp,
}

impl ExchangeVenue {
    pub const ALL: [Self; 7] = [
        Self::Hose,
        Self::Hnx,
        Self::Upcom,
        Self::Vn30Futures,
        Self::VnIndexBenchmark,
        Self::UsEquities,
        Self::CryptoSpotPerp,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Hose => "HOSE",
            Self::Hnx => "HNX",
            Self::Upcom => "UPCOM",
            Self::Vn30Futures => "VN30F",
            Self::VnIndexBenchmark => "VN-IDX",
            Self::UsEquities => "US EQ",
            Self::CryptoSpotPerp => "CRYPTO",
        }
    }

    #[inline]
    #[must_use]
    pub const fn full_label(self) -> &'static str {
        match self {
            Self::Hose => "HOSE (±7% · Lot 100 · T+2)",
            Self::Hnx => "HNX (±10% · Lot 100 · T+2)",
            Self::Upcom => "UPCOM (±15% · Lot 100 · T+2)",
            Self::Vn30Futures => "HNX VN30F (±7% · Mult 100k · T+0)",
            Self::VnIndexBenchmark => "VN Index Benchmark (No Band · Lot 1 · T+0)",
            Self::UsEquities => "US Equities (No Band · Lot 1 · T+1)",
            Self::CryptoSpotPerp => "Crypto 24/7 (No Band · Frac · T+0)",
        }
    }

    /// Infers the default [`ExchangeVenue`] for a loaded instrument symbol and [`DataSourcePreset`].
    #[must_use]
    pub fn infer_from_symbol_and_preset(symbol: &str, preset: DataSourcePreset) -> Self {
        let upper = symbol.trim().to_ascii_uppercase();
        let is_vn_index_ticker = matches!(
            upper.as_str(),
            "VNINDEX"
                | "VN-INDEX"
                | "VN30"
                | "VN30-INDEX"
                | "VN30INDEX"
                | "HNX-INDEX"
                | "HNXINDEX"
                | "UPCOM-INDEX"
                | "UPCOMINDEX"
        );
        match preset {
            DataSourcePreset::IntradayVn30F1m => Self::Vn30Futures,
            DataSourcePreset::BtcUsdt15mJson => Self::CryptoSpotPerp,
            DataSourcePreset::SpyDailyCsv => Self::UsEquities,
            DataSourcePreset::SyntheticVnindex | DataSourcePreset::CafeFIndexCsv => {
                if upper.starts_with("VN30F") {
                    Self::Vn30Futures
                } else {
                    Self::VnIndexBenchmark
                }
            }
            DataSourcePreset::Vn30Basket => {
                if upper.starts_with("VN30F") {
                    Self::Vn30Futures
                } else if is_vn_index_ticker {
                    Self::VnIndexBenchmark
                } else {
                    Self::Hose
                }
            }
            DataSourcePreset::CustomFile => {
                if upper.starts_with("VN30F") {
                    Self::Vn30Futures
                } else if is_vn_index_ticker {
                    Self::VnIndexBenchmark
                } else if upper.ends_with("USDT")
                    || upper.ends_with("USD")
                    || matches!(upper.as_str(), "BTC" | "ETH" | "SOL")
                {
                    Self::CryptoSpotPerp
                } else if matches!(
                    upper.as_str(),
                    "SPY" | "QQQ" | "IWM" | "AAPL" | "MSFT" | "NVDA" | "TSLA"
                ) {
                    Self::UsEquities
                } else if matches!(
                    upper.as_str(),
                    "SHS" | "PVS" | "IDC" | "MBS" | "CEO" | "TNG" | "HUT" | "VCS"
                ) {
                    Self::Hnx
                } else if matches!(
                    upper.as_str(),
                    "BSR" | "ACV" | "VEA" | "OIL" | "VGI" | "QNS" | "MCH"
                ) {
                    Self::Upcom
                } else {
                    Self::Hose
                }
            }
        }
    }

    /// Statutory regular-session daily price fluctuation band fraction (`Some(0.07)` = `±7.0%`).
    #[inline]
    #[must_use]
    pub const fn daily_band_pct(self) -> Option<f64> {
        match self {
            Self::Hose | Self::Vn30Futures => Some(0.07),
            Self::Hnx => Some(0.10),
            Self::Upcom => Some(0.15),
            Self::VnIndexBenchmark | Self::UsEquities | Self::CryptoSpotPerp => None,
        }
    }

    /// Standard round-lot step size (`100.0` shares on VN stock exchanges, `1.0` for futures/index/US, `0.001` crypto).
    #[inline]
    #[must_use]
    pub const fn board_lot(self) -> f64 {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom => 100.0,
            Self::Vn30Futures | Self::VnIndexBenchmark | Self::UsEquities => 1.0,
            Self::CryptoSpotPerp => 0.001,
        }
    }

    /// Statutory business-day settlement cycle (`T+2` VN equities, `T+1` US equities, `T+0` futures/index/crypto).
    #[inline]
    #[must_use]
    pub const fn settlement_business_days(self) -> u8 {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom => 2,
            Self::UsEquities => 1,
            Self::Vn30Futures | Self::VnIndexBenchmark | Self::CryptoSpotPerp => 0,
        }
    }

    /// Exchange local civil timezone offset from UTC (`+25,200s` = `UTC+7` ICT, `-18,000s` = `UTC-5` EST).
    #[inline]
    #[must_use]
    pub const fn utc_offset_seconds(self) -> i64 {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom | Self::Vn30Futures | Self::VnIndexBenchmark => {
                7 * 3_600
            }
            Self::UsEquities => -5 * 3_600,
            Self::CryptoSpotPerp => 0,
        }
    }

    /// Default initial margin percentage (`17.0%` on VSD/HNX `VN30F` index futures, `100.0%` cash-funded).
    #[inline]
    #[must_use]
    pub const fn default_initial_margin_pct(self) -> f64 {
        match self {
            Self::Vn30Futures => 17.0,
            Self::Hose
            | Self::Hnx
            | Self::Upcom
            | Self::VnIndexBenchmark
            | Self::UsEquities
            | Self::CryptoSpotPerp => 100.0,
        }
    }

    /// Statutory point-to-currency contract multiplier (`100,000 VND/pt` for `VN30F` in `RawVnd`,
    /// `100.0 kVND/pt` when account capital is denominated in `1,000 VND`, `1.0` for cash/spot).
    #[inline]
    #[must_use]
    pub fn default_contract_multiplier(self, price_unit: VnPriceUnit, account_equity: f64) -> f64 {
        match self {
            Self::Vn30Futures => match price_unit {
                VnPriceUnit::RawVnd => 100_000.0,
                VnPriceUnit::ThousandVnd => 100.0,
                VnPriceUnit::Auto => {
                    if account_equity.is_finite() && account_equity >= 1_000_000.0 {
                        100_000.0
                    } else {
                        100.0
                    }
                }
            },
            Self::Hose
            | Self::Hnx
            | Self::Upcom
            | Self::VnIndexBenchmark
            | Self::UsEquities
            | Self::CryptoSpotPerp => 1.0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn default_commission_bps(self) -> f64 {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom => 15.0,
            Self::Vn30Futures => 4.0,
            Self::VnIndexBenchmark | Self::UsEquities => 5.0,
            Self::CryptoSpotPerp => 10.0,
        }
    }

    /// Statutory securities transfer tax on gross sell proceeds (`10.0 bps` = `0.10%` on VN cash equities).
    #[inline]
    #[must_use]
    pub const fn default_tax_sell_bps(self) -> f64 {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom => 10.0,
            Self::Vn30Futures
            | Self::VnIndexBenchmark
            | Self::UsEquities
            | Self::CryptoSpotPerp => 0.0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn allows_short(self) -> bool {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom => false,
            Self::Vn30Futures
            | Self::VnIndexBenchmark
            | Self::UsEquities
            | Self::CryptoSpotPerp => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_vn_cash_equity(self) -> bool {
        matches!(self, Self::Hose | Self::Hnx | Self::Upcom)
    }

    /// Concise display unit suffix for DOM headers and ticket badges (`"kVND"`, `"VND"`, `"pts"`, `"USD"`, `"USDT"`).
    #[inline]
    #[must_use]
    pub const fn price_unit_label(self, resolved_vn_unit: ResolvedVnPriceUnit) -> &'static str {
        match self {
            Self::Hose | Self::Hnx | Self::Upcom => match resolved_vn_unit {
                ResolvedVnPriceUnit::ThousandVnd => "kVND",
                ResolvedVnPriceUnit::RawVnd => "VND",
            },
            Self::Vn30Futures | Self::VnIndexBenchmark => "pts",
            Self::UsEquities => "USD",
            Self::CryptoSpotPerp => "USDT",
        }
    }
}

/// Denomination unit policy for Vietnam cash equity prices (`1,000 VND` CafeF quotes vs raw `VND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum VnPriceUnit {
    /// Auto-detects `ThousandVnd` when reference/entry price `< 1,000.0` (`239.86`, `28.50`, `9.85`)
    /// vs `RawVnd` when `>= 1,000.0` (`239,860`, `28,500`, `9,850`).
    #[default]
    Auto,
    ThousandVnd,
    RawVnd,
}

/// Concrete resolved Vietnam cash equity price unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolvedVnPriceUnit {
    ThousandVnd,
    RawVnd,
}

impl VnPriceUnit {
    pub const ALL: [Self; 3] = [Self::Auto, Self::ThousandVnd, Self::RawVnd];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::ThousandVnd => "1k VND",
            Self::RawVnd => "Full VND",
        }
    }

    #[inline]
    #[must_use]
    pub fn resolve(self, anchor_price: f64) -> ResolvedVnPriceUnit {
        match self {
            Self::ThousandVnd => ResolvedVnPriceUnit::ThousandVnd,
            Self::RawVnd => ResolvedVnPriceUnit::RawVnd,
            Self::Auto => {
                if anchor_price.is_finite() && anchor_price >= 1_000.0 {
                    ResolvedVnPriceUnit::RawVnd
                } else {
                    ResolvedVnPriceUnit::ThousandVnd
                }
            }
        }
    }

    /// Resolves price denomination respecting `venue`: only VN cash equities (`HOSE`/`HNX`/`UPCOM`)
    /// trigger `1,000.0` `Auto` switch (`kVND` vs full `VND`); futures/indices/US/crypto keep `ThousandVnd`
    /// so `VN30F1M` (~`1,305 pts`) never flips into `"VND"`.
    #[inline]
    #[must_use]
    pub fn resolve_for_venue(self, venue: ExchangeVenue, anchor_price: f64) -> ResolvedVnPriceUnit {
        if venue.is_vn_cash_equity() {
            self.resolve(anchor_price)
        } else {
            ResolvedVnPriceUnit::ThousandVnd
        }
    }
}

/// Statutory daily reference, floor, and ceiling price band for a session.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DailyPriceBand {
    pub venue: ExchangeVenue,
    pub reference_price: f64,
    pub band_pct: f64,
    pub floor_price: f64,
    pub ceiling_price: f64,
    pub tick_at_ref: f64,
}

impl DailyPriceBand {
    /// Returns `true` when a session closes pinned at the ceiling price (`trắng bên bán` / locked ceiling).
    #[inline]
    #[must_use]
    pub fn is_locked_ceiling(self, bar_close: f64, bar_high: f64) -> bool {
        let tol = (self.tick_at_ref * 0.05).max(1.0e-6);
        (bar_close - self.ceiling_price).abs() <= tol && bar_high <= self.ceiling_price + tol
    }

    /// Returns `true` when a session closes pinned at the floor price (`trắng bên mua` / locked floor).
    #[inline]
    #[must_use]
    pub fn is_locked_floor(self, bar_close: f64, bar_low: f64) -> bool {
        let tol = (self.tick_at_ref * 0.05).max(1.0e-6);
        (bar_close - self.floor_price).abs() <= tol && bar_low >= self.floor_price - tol
    }
}

#[inline]
fn clean_quantized(value: f64) -> f64 {
    if value.abs() < 1.0e8 {
        (value * QUANTIZE_SCALE).round() / QUANTIZE_SCALE
    } else {
        value
    }
}

/// Returns the valid order-book tick step at `price` for `venue` and resolved `vn_unit`.
#[inline]
#[must_use]
pub fn tick_size_at_price(venue: ExchangeVenue, price: f64, vn_unit: ResolvedVnPriceUnit) -> f64 {
    let p = if price.is_finite() && price > 0.0 {
        price
    } else {
        1.0
    };
    match venue {
        ExchangeVenue::Hose => match vn_unit {
            ResolvedVnPriceUnit::ThousandVnd => {
                if p < 10.0 - TICK_EPS {
                    0.01
                } else if p < 50.0 - TICK_EPS {
                    0.05
                } else {
                    0.10
                }
            }
            ResolvedVnPriceUnit::RawVnd => {
                if p < 10_000.0 - 1.0e-6 {
                    10.0
                } else if p < 50_000.0 - 1.0e-6 {
                    50.0
                } else {
                    100.0
                }
            }
        },
        ExchangeVenue::Hnx | ExchangeVenue::Upcom => match vn_unit {
            ResolvedVnPriceUnit::ThousandVnd => 0.10,
            ResolvedVnPriceUnit::RawVnd => 100.0,
        },
        ExchangeVenue::Vn30Futures => 0.10,
        ExchangeVenue::VnIndexBenchmark => 0.01,
        ExchangeVenue::UsEquities => {
            if p >= 1.0 - TICK_EPS {
                0.01
            } else {
                0.0001
            }
        }
        ExchangeVenue::CryptoSpotPerp => {
            if p >= 100.0 - TICK_EPS {
                0.01
            } else if p >= 1.0 - TICK_EPS {
                0.0001
            } else if p >= 1.0e-4 - 1.0e-12 {
                1.0e-6
            } else {
                1.0e-8
            }
        }
    }
}

/// Rounds `price` **down** (`floor`) to a valid exchange tick at `price` (`<= price`), guarding against IEEE-754 drift.
/// Returns `0.0` when `price < tick` rather than ever rounding up above `price`.
#[inline]
#[must_use]
pub fn round_down_to_tick(price: f64, venue: ExchangeVenue, vn_unit: ResolvedVnPriceUnit) -> f64 {
    if !price.is_finite() || price <= 0.0 {
        return 0.0;
    }
    let tick = tick_size_at_price(venue, price, vn_unit);
    let steps = ((price / tick) + TICK_EPS).floor();
    if steps <= 0.0 {
        return 0.0;
    }
    clean_quantized(steps * tick)
}

/// Rounds `price` **up** (`ceil`) to a valid exchange tick at `price`, guarding against IEEE-754 drift.
#[inline]
#[must_use]
pub fn round_up_to_tick(price: f64, venue: ExchangeVenue, vn_unit: ResolvedVnPriceUnit) -> f64 {
    if !price.is_finite() || price <= 0.0 {
        return 0.0;
    }
    let tick = tick_size_at_price(venue, price, vn_unit);
    let steps = ((price / tick) - TICK_EPS).ceil().max(1.0);
    clean_quantized(steps * tick)
}

/// Steps strictly **1 valid order-book tick below** `price`, respecting `HOSE` tier transitions
/// (`50.00 -> 49.95`, `10.00 -> 9.99`, `50,000 -> 49,950`, `10,000 -> 9,990`).
#[inline]
#[must_use]
pub fn step_one_tick_below(price: f64, venue: ExchangeVenue, vn_unit: ResolvedVnPriceUnit) -> f64 {
    if !price.is_finite() || price <= 0.0 {
        return 0.0;
    }
    let probe = (price * (1.0 - 1.0e-7)).max(0.0);
    let tick_below = tick_size_at_price(venue, probe, vn_unit);
    if price <= tick_below + TICK_EPS {
        return 0.0;
    }
    round_down_to_tick(price - tick_below, venue, vn_unit)
}

/// Steps strictly **1 valid order-book tick above** `price`.
#[inline]
#[must_use]
pub fn step_one_tick_above(price: f64, venue: ExchangeVenue, vn_unit: ResolvedVnPriceUnit) -> f64 {
    if !price.is_finite() || price <= 0.0 {
        return 0.0;
    }
    let tick = tick_size_at_price(venue, price, vn_unit);
    round_up_to_tick(price + tick, venue, vn_unit)
}

/// Snaps `price` to the nearest valid exchange tick at `price`.
#[inline]
#[must_use]
pub fn snap_to_nearest_tick(price: f64, venue: ExchangeVenue, vn_unit: ResolvedVnPriceUnit) -> f64 {
    if !price.is_finite() || price <= 0.0 {
        return 0.0;
    }
    let tick = tick_size_at_price(venue, price, vn_unit);
    let steps = (price / tick).round().max(1.0);
    let snapped = clean_quantized(steps * tick);
    // Re-verify if rounding landed on/above a higher tier boundary (e.g. `49.99` -> `50.00` is valid in both!).
    let verify_tick = tick_size_at_price(venue, snapped, vn_unit);
    let verify_steps = (snapped / verify_tick).round().max(1.0);
    clean_quantized(verify_steps * verify_tick)
}

/// Computes statutory daily floor and ceiling prices using an explicitly resolved `resolved_unit`
/// (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`, `VN30F ±7%`), snapping `reference_price` to a valid venue
/// tick first so `reference_price`, `floor_price`, and `ceiling_price` stay tick-consistent.
#[must_use]
pub fn compute_daily_price_band_with_resolved_unit(
    venue: ExchangeVenue,
    reference_price: f64,
    resolved_unit: ResolvedVnPriceUnit,
) -> Option<DailyPriceBand> {
    let band_pct = venue.daily_band_pct()?;
    if !reference_price.is_finite() || reference_price <= 0.0 {
        return None;
    }
    let snapped_ref = snap_to_nearest_tick(reference_price, venue, resolved_unit);
    if !snapped_ref.is_finite() || snapped_ref <= 0.0 {
        return None;
    }
    let tick_at_ref = tick_size_at_price(venue, snapped_ref, resolved_unit);
    let raw_ceiling = snapped_ref * (1.0 + band_pct);
    let raw_floor = snapped_ref * (1.0 - band_pct);

    let mut ceiling_price = round_down_to_tick(raw_ceiling, venue, resolved_unit);
    let mut floor_price = round_up_to_tick(raw_floor, venue, resolved_unit);

    // Low-price guard (matching HSX minimum 1-tick band rule so floor <= ref <= ceiling).
    if ceiling_price < snapped_ref {
        ceiling_price = snapped_ref;
    }
    if floor_price > snapped_ref {
        floor_price = round_down_to_tick(snapped_ref, venue, resolved_unit).max(tick_at_ref);
    }
    if floor_price > ceiling_price {
        floor_price = ceiling_price;
    }

    Some(DailyPriceBand {
        venue,
        reference_price: snapped_ref,
        band_pct,
        floor_price,
        ceiling_price,
        tick_at_ref,
    })
}

/// Computes statutory daily floor and ceiling prices from `reference_price` (`HOSE ±7%`, `HNX ±10%`,
/// `UPCOM ±15%`, `VN30F ±7%`), rounding ceiling **down** to tick and floor **up** to tick per
/// Vietnam Circular `120/2020/TT-BTC` and HSX/HNX trading regulations.
#[must_use]
pub fn compute_daily_price_band(
    venue: ExchangeVenue,
    reference_price: f64,
    unit: VnPriceUnit,
) -> Option<DailyPriceBand> {
    let resolved_unit = unit.resolve_for_venue(venue, reference_price);
    compute_daily_price_band_with_resolved_unit(venue, reference_price, resolved_unit)
}

/// Floors `raw_quantity` down to an integer multiple of `venue.board_lot()`.
#[inline]
#[must_use]
pub fn floor_to_board_lot(venue: ExchangeVenue, raw_quantity: f64) -> f64 {
    if !raw_quantity.is_finite() || raw_quantity <= 0.0 {
        return 0.0;
    }
    let lot = venue.board_lot();
    if lot <= 0.0 {
        return raw_quantity;
    }
    let lots = ((raw_quantity / lot) + TICK_EPS).floor().max(0.0);
    clean_quantized(lots * lot)
}

/// Advances `entry_timestamp` (`UTC` seconds) by `business_days` (`Mon..Fri` UTC), skipping weekends.
#[must_use]
pub fn compute_settlement_timestamp(entry_timestamp: i64, business_days: u8) -> i64 {
    let clamped = entry_timestamp.clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS);
    if business_days == 0 {
        return clamped;
    }
    let mut day = clamped.div_euclid(86_400);
    let secs_of_day = clamped.rem_euclid(86_400);
    let mut remaining = business_days;
    while remaining > 0 {
        day = day.saturating_add(1);
        if weekday_from_days(day) < 5 {
            remaining -= 1;
        }
    }
    day.saturating_mul(86_400)
        .saturating_add(secs_of_day)
        .clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS)
}

/// Exchange-timezone-aware settlement timestamp (`T+2` VSD `13:00 ICT = 06:00 UTC` on Vietnam cash equities,
/// `T+1` `America/New_York` `UTC-5` calendar days on US equities, `T+0` immediate on futures/crypto).
#[must_use]
pub fn compute_settlement_timestamp_for_venue(entry_timestamp: i64, venue: ExchangeVenue) -> i64 {
    let clamped = entry_timestamp.clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS);
    let business_days = venue.settlement_business_days();
    if business_days == 0 {
        return clamped;
    }
    let tz_offset = venue.utc_offset_seconds();
    let local_ts = clamped.saturating_add(tz_offset);
    let mut local_day = local_ts.div_euclid(86_400);
    let local_secs_of_day = local_ts.rem_euclid(86_400);
    let mut remaining = business_days;
    while remaining > 0 {
        local_day = local_day.saturating_add(1);
        if weekday_from_days(local_day) < 5 {
            remaining -= 1;
        }
    }
    let settle_local_secs = if venue.is_vn_cash_equity() {
        // VSD/VSDC shortened `T+2` settlement credits shares/cash by `13:00 ICT` (afternoon session `T+2`).
        13 * 3_600
    } else {
        local_secs_of_day
    };
    local_day
        .saturating_mul(86_400)
        .saturating_add(settle_local_secs)
        .saturating_sub(tz_offset)
        .max(clamped)
        .clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS)
}
