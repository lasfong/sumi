use serde::{Deserialize, Serialize};

use crate::data::{
    DataSourcePreset, MAX_SUPPORTED_EPOCH_SECS, MIN_SUPPORTED_EPOCH_SECS, civil_from_days,
    days_from_civil, weekday_from_days,
};
use crate::trading::OrderSide;

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
                | "VN100"
                | "VNXALL"
                | "VNX50"
                | "VNMID"
                | "VNSML"
                | "VNFINLEAD"
                | "VNDIAMOND"
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
                let is_vn_etf = upper.starts_with("E1VF") || upper.starts_with("FUE");
                let is_us_short_ticker = matches!(
                    upper.as_str(),
                    "SPY"
                        | "QQQ"
                        | "IWM"
                        | "DIA"
                        | "VOO"
                        | "VTI"
                        | "AMD"
                        | "JPM"
                        | "IBM"
                        | "BAC"
                        | "DIS"
                        | "WMT"
                        | "XOM"
                        | "CVX"
                        | "NKE"
                        | "CAT"
                        | "BA"
                        | "GE"
                        | "GM"
                        | "GS"
                        | "HD"
                        | "KO"
                        | "MCD"
                        | "MMM"
                        | "PG"
                        | "UNH"
                        | "UPS"
                        | "V"
                        | "MA"
                        | "F"
                        | "C"
                        | "T"
                );
                let is_non_vn_alpha_ticker = !is_vn_etf
                    && !upper.starts_with("VN")
                    && (upper.contains('.')
                        || (upper.len() >= 4 && upper.bytes().all(|b| b.is_ascii_alphabetic()))
                        || (upper.len() <= 2
                            && !upper.is_empty()
                            && upper.bytes().all(|b| b.is_ascii_alphabetic())));
                if upper.starts_with("VN30F") {
                    Self::Vn30Futures
                } else if is_vn_index_ticker {
                    Self::VnIndexBenchmark
                } else if upper.ends_with("USDT")
                    || upper.ends_with("USD")
                    || upper.ends_with("BUSD")
                    || upper.ends_with("USDC")
                    || matches!(upper.as_str(), "BTC" | "ETH" | "SOL" | "BNB" | "XRP")
                {
                    Self::CryptoSpotPerp
                } else if HNX_LISTED_SYMBOLS.binary_search(&upper.as_str()).is_ok() {
                    Self::Hnx
                } else if UPCOM_LISTED_SYMBOLS.binary_search(&upper.as_str()).is_ok() {
                    Self::Upcom
                } else if is_us_short_ticker || is_non_vn_alpha_ticker {
                    Self::UsEquities
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

/// Statutory VSD/VSDC `T+2` afternoon (`13:00 ICT`) cutover date (`2022-08-29`, Decision `109/QĐ-VSD`).
/// Strictly sorted official `HNX` (`±10%` daily band, `0.10 kVND` flat tick) ticker registry (`BUG-021`).
pub const HNX_LISTED_SYMBOLS: &[&str] = &[
    "AAV", "AMV", "API", "APS", "BCC", "BII", "BNA", "BVS", "C69", "CEO", "CTP", "CVN", "DIH",
    "DL1", "DST", "DTD", "DVG", "DVM", "DXP", "EVS", "FID", "GKM", "HDA", "HLD", "HMR", "HOM",
    "HTP", "HUT", "IDC", "IDJ", "IPA", "ITQ", "IVS", "KSQ", "L14", "L40", "LAS", "LDP", "LIG",
    "MBG", "MBS", "MST", "NAG", "NBC", "NDN", "NRC", "NSH", "NTP", "NVB", "PCH", "PGN", "PHP",
    "PLC", "PPT", "PSD", "PSI", "PV2", "PVB", "PVC", "PVG", "PVL", "PVS", "RCL", "S99", "SCI",
    "SD6", "SDA", "SHS", "SJE", "SRA", "SVN", "TA9", "TD6", "TIG", "TNG", "TTH", "TV4", "TVC",
    "TVD", "VC2", "VC3", "VC7", "VCS", "VFS", "VGS", "VHE", "VIG", "VTV", "VTZ", "WSS",
];

/// Strictly sorted official `UPCOM` (`±15%` daily band, `0.10 kVND` flat tick) ticker registry (`BUG-021`).
pub const UPCOM_LISTED_SYMBOLS: &[&str] = &[
    "AAH", "AAS", "ABB", "ABI", "ABW", "ACM", "ACV", "AFX", "AGM", "AMS", "ATG", "AVG", "BCR",
    "BGE", "BIG", "BMS", "BOT", "BSR", "C4G", "CKA", "CLI", "CTX", "DCS", "DDG", "DDV", "DFF",
    "DGT", "DHB", "DIC", "DRI", "DTI", "DTP", "DVN", "F88", "FOX", "FTM", "G20", "G36", "GCF",
    "GDA", "GPC", "HBC", "HHG", "HIO", "HKB", "HNG", "HNM", "HSV", "HU4", "HVA", "HWS", "KHD",
    "KLB", "KSV", "KVC", "LCM", "LMH", "LSG", "LTG", "MCH", "MEC", "MKP", "MML", "MPC", "MPT",
    "MSR", "MVC", "NED", "OIL", "PAS", "PFL", "PIV", "POM", "POS", "PPH", "PSP", "PTV", "PVO",
    "PVV", "PVX", "PXI", "PXL", "PXS", "PXT", "QBS", "QNS", "QTP", "RIC", "SBB", "SBS", "SCL",
    "SDD", "SGP", "SRB", "STH", "TAR", "TCD", "TIN", "TSA", "TT6", "TTN", "TV1", "TVN", "UDJ",
    "VAB", "VEA", "VGI", "VGT", "VHG", "VIW", "VKC", "VLB", "VLC", "VST", "VTD", "VVS", "XMC",
];

/// Acquisitions prior to `2022-08-29` followed the historical `T+3` morning (`09:00 ICT`) settlement cycle.
#[allow(clippy::inconsistent_digit_grouping)]
pub const VSD_T2_CUTOVER_YMD: i32 = 2022_08_29;

/// Authoritative Vietnam Stock Exchange (`HOSE` / `HNX` / `UPCOM` / `VN30F`) weekday public holidays,
/// compensatory Mondays/Tuesdays (`nghỉ bù`), and government bridge-day swaps (`hoán đổi`) across `2016–2030`
/// (`171` strictly sorted `YYYYMMDD` entries; Saturday civil-service makeup workdays never trade on VN exchanges).
#[allow(clippy::inconsistent_digit_grouping)]
pub const VIETNAM_HOLIDAYS_V1: [i32; 171] = [
    // 2016 (10 official HOSE/HNX weekday holidays: New Year, Tết Bính Thân, Giỗ Tổ, 30/04–01/05 comp, 02/09)
    2016_01_01, 2016_02_08, 2016_02_09, 2016_02_10, 2016_02_11, 2016_02_12, 2016_04_18, 2016_05_02,
    2016_05_03,
    2016_09_02, // 2017 (10 official HOSE/HNX weekday holidays: New Year comp, Tết Đinh Dậu, Giỗ Tổ, 30/04–01/05, 02/09 comp)
    2017_01_02, 2017_01_26, 2017_01_27, 2017_01_30, 2017_01_31, 2017_02_01, 2017_04_06, 2017_05_01,
    2017_05_02, 2017_09_04, // 2018 (11 weekday holidays, incl. 31/12 bridge swap)
    2018_01_01, 2018_02_14, 2018_02_15, 2018_02_16, 2018_02_19, 2018_02_20, 2018_04_25, 2018_04_30,
    2018_05_01, 2018_09_03, 2018_12_31, // 2019 (11 weekday holidays, incl. 29/04 bridge swap)
    2019_01_01, 2019_02_04, 2019_02_05, 2019_02_06, 2019_02_07, 2019_02_08, 2019_04_15, 2019_04_29,
    2019_04_30, 2019_05_01, 2019_09_02, // 2020 (10 weekday holidays)
    2020_01_01, 2020_01_23, 2020_01_24, 2020_01_27, 2020_01_28, 2020_01_29, 2020_04_02, 2020_04_30,
    2020_05_01, 2020_09_02, // 2021 (11 weekday holidays)
    2021_01_01, 2021_02_10, 2021_02_11, 2021_02_12, 2021_02_15, 2021_02_16, 2021_04_21, 2021_04_30,
    2021_05_03, 2021_09_02, 2021_09_03, // 2022 (11 weekday holidays)
    2022_01_03, 2022_01_31, 2022_02_01, 2022_02_02, 2022_02_03, 2022_02_04, 2022_04_11, 2022_05_02,
    2022_05_03, 2022_09_01, 2022_09_02, // 2023 (11 weekday holidays)
    2023_01_02, 2023_01_20, 2023_01_23, 2023_01_24, 2023_01_25, 2023_01_26, 2023_05_01, 2023_05_02,
    2023_05_03, 2023_09_01, 2023_09_04, // 2024 (12 weekday holidays, incl. 29/04 swap)
    2024_01_01, 2024_02_08, 2024_02_09, 2024_02_12, 2024_02_13, 2024_02_14, 2024_04_18, 2024_04_29,
    2024_04_30, 2024_05_01, 2024_09_02,
    2024_09_03, // 2025 (12 weekday holidays, incl. 02/05 swap)
    2025_01_01, 2025_01_27, 2025_01_28, 2025_01_29, 2025_01_30, 2025_01_31, 2025_04_07, 2025_04_30,
    2025_05_01, 2025_05_02, 2025_09_01,
    2025_09_02, // 2026 (14 weekday holidays, incl. 02/01 & 31/08 swaps + 24/11 Culture Day)
    2026_01_01, 2026_01_02, 2026_02_16, 2026_02_17, 2026_02_18, 2026_02_19, 2026_02_20, 2026_04_27,
    2026_04_30, 2026_05_01, 2026_08_31, 2026_09_01, 2026_09_02,
    2026_11_24, // 2027 (12 statutory weekday holidays, incl. 24/11 Culture Day)
    2027_01_01, 2027_02_05, 2027_02_08, 2027_02_09, 2027_02_10, 2027_02_11, 2027_04_16, 2027_04_30,
    2027_05_03, 2027_09_02, 2027_09_03,
    2027_11_24, // 2028 (12 statutory weekday holidays, incl. 24/11 Culture Day)
    2028_01_03, 2028_01_25, 2028_01_26, 2028_01_27, 2028_01_28, 2028_01_31, 2028_04_04, 2028_05_01,
    2028_05_02, 2028_09_01, 2028_09_04,
    2028_11_24, // 2029 (12 statutory weekday holidays, incl. 26/11 compensatory for Sat 24/11)
    2029_01_01, 2029_02_12, 2029_02_13, 2029_02_14, 2029_02_15, 2029_02_16, 2029_04_23, 2029_04_30,
    2029_05_01, 2029_09_03, 2029_09_04,
    2029_11_26, // 2030 (12 statutory weekday holidays, incl. 25/11 compensatory for Sun 24/11)
    2030_01_01, 2030_02_01, 2030_02_04, 2030_02_05, 2030_02_06, 2030_02_07, 2030_04_12, 2030_04_30,
    2030_05_01, 2030_09_02, 2030_09_03, 2030_11_25,
];

/// Returns `true` if civil date `(year, month, day)` is an official Vietnam exchange weekday holiday.
/// Uses `VIETNAM_HOLIDAYS_V1` ($O(\log N)$ binary search) for `2016..=2030`, with a deterministic
/// Labor Code Art. 111(3) & 112 + Resolution `28/2026/QH16` statutory fixed-holiday + compensatory
/// Monday/Tuesday fallback outside `2016..=2030` so out-of-horizon dates never hit an unprotected cliff (`BUG-CAL-01`).
#[must_use]
pub fn is_vn_exchange_holiday_ymd(year: i32, month: u32, day: u32) -> bool {
    let days = days_from_civil(year, month, day);
    let wd = weekday_from_days(days);
    if wd >= 5 {
        return false;
    }
    if (2016..=2030).contains(&year) {
        let packed = year
            .saturating_mul(10_000)
            .saturating_add((month as i32) * 100)
            .saturating_add(day as i32);
        return VIETNAM_HOLIDAYS_V1.binary_search(&packed).is_ok();
    }

    // Statutory fallback outside 2016..=2030 (`01/01`, `30/04`, `01/05`, `02/09`, `24/11` `2026+` + compensatory Mon/Tue):
    match (month, day) {
        (1, 1) | (4, 30) | (5, 1) | (9, 2) => true,
        (11, 24) if year >= 2026 => true,
        (11, 25) if year >= 2026 => weekday_from_days(days_from_civil(year, 11, 24)) == 6,
        (11, 26) if year >= 2026 => weekday_from_days(days_from_civil(year, 11, 24)) == 5,
        // Compensatory Monday/Tuesday when New Year `01/01` fell on Sat/Sun:
        (1, 2) => weekday_from_days(days_from_civil(year, 1, 1)) == 6,
        (1, 3) => weekday_from_days(days_from_civil(year, 1, 1)) == 5,
        // Compensatory Monday/Tuesday when `30/04` or `01/05` fell on Sat/Sun:
        (5, 2) => {
            let wd_apr30 = weekday_from_days(days_from_civil(year, 4, 30));
            wd_apr30 == 5 || wd_apr30 == 6
        }
        (5, 3) => {
            let wd_apr30 = weekday_from_days(days_from_civil(year, 4, 30));
            // Apr 30 = Sat & May 1 = Sun -> May 2 & May 3 both compensated; or May 1 = Sat -> May 3 compensated
            wd_apr30 == 4 || wd_apr30 == 5
        }
        // Two-day National Day (`02/09` + adjacent/compensatory weekday from `2021+`):
        (9, 3) if year >= 2021 => {
            let wd_sep2 = weekday_from_days(days_from_civil(year, 9, 2));
            wd_sep2 != 5 // Mon..Thu -> 03/09, Sun -> Mon 03/09
        }
        (9, 4) if year >= 2021 => {
            let wd_sep2 = weekday_from_days(days_from_civil(year, 9, 2));
            wd_sep2 == 5 || wd_sep2 == 6 // Sat -> Mon 04/09, Sun -> Mon 03/09 + Tue 04/09
        }
        (9, 1) if year >= 2021 => {
            let wd_sep2 = weekday_from_days(days_from_civil(year, 9, 2));
            // Fri -> Thu 01/09 + Fri 02/09; Sat -> Fri 01/09 + Mon 04/09 (`BUG-034`).
            wd_sep2 == 4 || wd_sep2 == 5
        }
        (9, 3) => weekday_from_days(days_from_civil(year, 9, 2)) == 6,
        (9, 4) => weekday_from_days(days_from_civil(year, 9, 2)) == 5,
        _ => false,
    }
}

/// Returns `true` if `ict_epoch_days` (`days since 1970-01-01` in Vietnam civil time `UTC+7`) is an open
/// Vietnam stock exchange session (`Mon..Fri` and not in `VIETNAM_HOLIDAYS_V1`).
#[inline]
#[must_use]
pub fn is_vn_exchange_trading_day_days(ict_epoch_days: i64) -> bool {
    if weekday_from_days(ict_epoch_days) >= 5 {
        return false;
    }
    let (y, m, d) = civil_from_days(ict_epoch_days);
    !is_vn_exchange_holiday_ymd(y, m, d)
}

/// Advances `start_ict_days` by `delta_days` valid Vietnam exchange trading days (`n == 0` snaps forward
/// to the next valid trading day if `start_ict_days` falls on a weekend/holiday).
#[must_use]
pub fn add_vn_trading_days(start_ict_days: i64, delta_days: i32) -> i64 {
    let mut cur = start_ict_days;
    if delta_days == 0 {
        while !is_vn_exchange_trading_day_days(cur) {
            cur = cur.saturating_add(1);
        }
        return cur;
    }
    let step = if delta_days > 0 { 1_i64 } else { -1_i64 };
    let mut rem = delta_days.unsigned_abs();
    while rem > 0 {
        cur = cur.saturating_add(step);
        if is_vn_exchange_trading_day_days(cur) {
            rem -= 1;
        }
    }
    cur
}

/// Counts signed Vietnam exchange trading days in `(start_ict_days, end_ict_days]` (`0` when equal).
#[must_use]
pub fn vn_trading_days_between(start_ict_days: i64, end_ict_days: i64) -> i32 {
    if start_ict_days == end_ict_days {
        return 0;
    }
    if end_ict_days < start_ict_days {
        return -vn_trading_days_between(end_ict_days, start_ict_days);
    }
    let mut count = 0_i32;
    let mut cur = start_ict_days;
    while cur < end_ict_days {
        cur = cur.saturating_add(1);
        if is_vn_exchange_trading_day_days(cur) {
            count = count.saturating_add(1);
        }
    }
    count
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

/// Exchange-timezone-aware & holiday-aware settlement timestamp (`T+2` VSD `13:00 ICT = 06:00 UTC` on Vietnam
/// cash equities from `2022-08-29` onward, `T+3` `09:00 ICT = 02:00 UTC` prior to `2022-08-29`, skipping all
/// `VIETNAM_HOLIDAYS_V1` holidays; `T+1` `America/New_York` `UTC-5` weekdays on US equities; `T+0` immediate on futures/crypto).
#[must_use]
pub fn compute_settlement_timestamp_for_venue(entry_timestamp: i64, venue: ExchangeVenue) -> i64 {
    let clamped = entry_timestamp.clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS);
    let base_business_days = venue.settlement_business_days();
    if base_business_days == 0 {
        return clamped;
    }
    let tz_offset = venue.utc_offset_seconds();
    let local_ts = clamped.saturating_add(tz_offset);
    let mut local_day = local_ts.div_euclid(86_400);
    let local_secs_of_day = local_ts.rem_euclid(86_400);

    if venue.is_vn_cash_equity() {
        // Snap weekend/holiday order timestamps forward to the actual `T+0` execution trading day first (`m3` fix).
        let exec_day = add_vn_trading_days(local_day, 0);
        let (ey, em, ed) = civil_from_days(exec_day);
        let entry_ymd = ey
            .saturating_mul(10_000)
            .saturating_add((em as i32) * 100)
            .saturating_add(ed as i32);
        let (vn_days, settle_local_secs) = if entry_ymd < VSD_T2_CUTOVER_YMD {
            // Historical pre-2022-08-29 VSD `T+3` morning release (`09:00 ICT = 02:00 UTC`).
            (3_i32, 9 * 3_600)
        } else {
            // Current VSD/VSDC `T+2` (`T+1.5`) afternoon session release (`13:00 ICT = 06:00 UTC`).
            (2_i32, 13 * 3_600)
        };
        let settle_day = add_vn_trading_days(exec_day, vn_days);
        return settle_day
            .saturating_mul(86_400)
            .saturating_add(settle_local_secs)
            .saturating_sub(tz_offset)
            .max(clamped)
            .clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS);
    }

    // UTC-midnight daily bars (`SPY CSV`, `D1` feeds) carry civil date `YYYY-MM-DD 00:00 UTC`
    // rather than an intraday clock instant; applying `UTC-5` would shift Monday `00:00 UTC` back
    // into Sunday `19:00 EST` and advance `T+1` to Wednesday (`BUG-017`).
    if clamped.rem_euclid(86_400) == 0 {
        let mut utc_day = clamped.div_euclid(86_400);
        while weekday_from_days(utc_day) >= 5 {
            utc_day = utc_day.saturating_add(1);
        }
        let mut remaining = base_business_days;
        while remaining > 0 {
            utc_day = utc_day.saturating_add(1);
            if weekday_from_days(utc_day) < 5 {
                remaining -= 1;
            }
        }
        return utc_day
            .saturating_mul(86_400)
            .max(clamped)
            .clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS);
    }

    while weekday_from_days(local_day) >= 5 {
        local_day = local_day.saturating_add(1);
    }
    let mut remaining = base_business_days;
    while remaining > 0 {
        local_day = local_day.saturating_add(1);
        if weekday_from_days(local_day) < 5 {
            remaining -= 1;
        }
    }
    local_day
        .saturating_mul(86_400)
        .saturating_add(local_secs_of_day)
        .saturating_sub(tz_offset)
        .max(clamped)
        .clamp(MIN_SUPPORTED_EPOCH_SECS, MAX_SUPPORTED_EPOCH_SECS)
}

/// Maps a bar timestamp (`UTC` seconds) to its causal settlement evaluation timestamp for `venue` and
/// `base_timeframe_secs` (`BUG-017`):
/// - Intraday bars (`< 1D`) evaluate at exact bar close `bar_ts + bar_secs - 1`.
/// - Daily (`1D`) and weekly (`1W`) bars evaluate at the close of `bar_ts`'s **same trading day** in the
///   exchange's civil calendar (`23:59:59 ICT = 16:59:59 UTC` on Vietnam venues) so non-midnight `1D` bars
///   (`> 06:00 UTC`) never bleed into `T+2` on `T+1` and `1W` base bars never unlock `T+0`.
#[inline]
#[must_use]
pub fn causal_settlement_eval_timestamp(
    bar_timestamp: i64,
    venue: ExchangeVenue,
    base_timeframe_secs: i64,
) -> i64 {
    let tf = base_timeframe_secs.max(1);
    if tf < 86_400 {
        return bar_timestamp.saturating_add(tf.saturating_sub(1));
    }
    if venue.is_vn_cash_equity() {
        let tz = venue.utc_offset_seconds();
        let local_day = bar_timestamp.saturating_add(tz).div_euclid(86_400);
        return local_day
            .saturating_mul(86_400)
            .saturating_add(86_399)
            .saturating_sub(tz);
    }
    if bar_timestamp.rem_euclid(86_400) == 0 {
        return bar_timestamp.saturating_add(86_399);
    }
    let tz = venue.utc_offset_seconds();
    let local_day = bar_timestamp.saturating_add(tz).div_euclid(86_400);
    local_day
        .saturating_mul(86_400)
        .saturating_add(86_399)
        .saturating_sub(tz)
}

/// Individual FIFO acquisition lot tracked for statutory `T+2` (`13:00 ICT`) / `T+3` (`09:00 ICT`) settlement (`Milestone R19 / C2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoldingLot {
    pub lot_id: u64,
    pub buy_fill_id: u64,
    pub buy_base_index: usize,
    pub buy_timestamp: i64,
    pub buy_price: f64,
    pub initial_qty: f64,
    pub remaining_qty: f64,
    pub sellable_at_timestamp: i64,
}

impl HoldingLot {
    /// Returns `true` when this lot's shares have cleared VSD settlement at `as_of_timestamp` (`UTC` seconds).
    #[inline]
    #[must_use]
    pub const fn is_sellable_at(&self, as_of_timestamp: i64) -> bool {
        as_of_timestamp >= self.sellable_at_timestamp
    }
}

/// Causal FIFO settlement breakdown of open long holdings as of `as_of_timestamp` (`Milestone R19 / C2`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FifoSettlementSummary {
    pub venue: ExchangeVenue,
    pub as_of_timestamp: i64,
    pub lots: Vec<HoldingLot>,
    pub total_held_qty: f64,
    pub sellable_qty: f64,
    pub locked_t2_qty: f64,
    pub earliest_unlock_timestamp: Option<i64>,
    pub unsettled_sell_violation_qty: f64,
}

/// Reconstructs open FIFO [`HoldingLot`]s and `T+2`/`T+1`/`T+0` sellable vs locked balances strictly
/// from causal execution fills (`timestamp <= as_of_timestamp`, `DEF-001` replay-rewind safe).
#[must_use]
pub fn compute_fifo_holding_lots(
    fills: &[(u64, usize, i64, OrderSide, f64, f64)],
    venue: ExchangeVenue,
    as_of_timestamp: i64,
) -> FifoSettlementSummary {
    compute_fifo_holding_lots_with_bar_secs(fills, venue, as_of_timestamp, 1)
}

/// Reconstructs open FIFO [`HoldingLot`]s with explicit `base_timeframe_secs` so coarse-bar (`D1` / `H4`)
/// sell fills stamped at bar open (`00:00 UTC` / `04:00 UTC`) evaluate VSD `13:00 ICT = 06:00 UTC`
/// (and pre-2022 `09:00 ICT = 02:00 UTC`) clearance at causal bar close `min(ts + bar_secs - 1, as_of_timestamp)`.
#[must_use]
pub fn compute_fifo_holding_lots_with_bar_secs(
    fills: &[(u64, usize, i64, OrderSide, f64, f64)],
    venue: ExchangeVenue,
    as_of_timestamp: i64,
    base_timeframe_secs: i64,
) -> FifoSettlementSummary {
    let tf_secs = base_timeframe_secs.max(1);
    let effective_as_of = if tf_secs >= 86_400 && as_of_timestamp.rem_euclid(86_400) != 86_399 {
        causal_settlement_eval_timestamp(as_of_timestamp, venue, tf_secs).max(as_of_timestamp)
    } else {
        as_of_timestamp
    };
    let mut ordered: Vec<(usize, u64, usize, i64, OrderSide, f64, f64)> = fills
        .iter()
        .enumerate()
        .filter(|(_, (_, _, ts, _, qty, px))| {
            *ts <= as_of_timestamp && qty.is_finite() && *qty > 1.0e-12 && px.is_finite()
        })
        .map(|(ord, &(fid, bidx, ts, side, qty, px))| (ord, fid, bidx, ts, side, qty, px))
        .collect();
    ordered.sort_by_key(|&(ord, fid, bidx, ts, _, _, _)| (ts, bidx, fid, ord));

    let mut lots: Vec<HoldingLot> = Vec::new();
    let mut short_open_qty = 0.0_f64;
    let mut next_lot_id = 1_u64;
    let mut unsettled_sell_violation_qty = 0.0_f64;

    for (_, fill_id, base_idx, ts, side, qty, price) in ordered {
        match side {
            OrderSide::Buy => {
                let mut rem_buy = qty;
                if short_open_qty > 1.0e-12 {
                    let cover = rem_buy.min(short_open_qty);
                    short_open_qty = (short_open_qty - cover).max(0.0);
                    rem_buy = (rem_buy - cover).max(0.0);
                }
                if rem_buy > 1.0e-9 {
                    let clean_q = clean_quantized(rem_buy);
                    let sellable_at = compute_settlement_timestamp_for_venue(ts, venue);
                    lots.push(HoldingLot {
                        lot_id: next_lot_id,
                        buy_fill_id: fill_id,
                        buy_base_index: base_idx,
                        buy_timestamp: ts,
                        buy_price: price,
                        initial_qty: clean_q,
                        remaining_qty: clean_q,
                        sellable_at_timestamp: sellable_at,
                    });
                    next_lot_id = next_lot_id.saturating_add(1);
                }
            }
            OrderSide::Sell => {
                let mut rem_sell = qty;
                let sell_eval_ts =
                    causal_settlement_eval_timestamp(ts, venue, tf_secs).min(effective_as_of);
                // 1. Prefer consuming already-settled FIFO lots first (`sell_eval_ts >= lot.sellable_at_timestamp`)
                //    so a trader holding `100` settled Monday shares + `100` unsettled Wednesday shares
                //    selling `100` shares on Wednesday cleanly sells the settled Monday lot without violation!
                for lot in &mut lots {
                    if rem_sell <= 1.0e-9 {
                        break;
                    }
                    if lot.remaining_qty > 1.0e-9 && lot.is_sellable_at(sell_eval_ts) {
                        let take = rem_sell.min(lot.remaining_qty);
                        lot.remaining_qty = clean_quantized((lot.remaining_qty - take).max(0.0));
                        rem_sell = (rem_sell - take).max(0.0);
                    }
                }
                // 2. If `rem_sell` still exceeds settled shares (e.g. when `enforce_vn_t2_lot_lock == false`),
                //    consume remaining unsettled lots in strict FIFO order and record `unsettled_sell_violation_qty`.
                for lot in &mut lots {
                    if rem_sell <= 1.0e-9 {
                        break;
                    }
                    if lot.remaining_qty > 1.0e-9 {
                        let take = rem_sell.min(lot.remaining_qty);
                        lot.remaining_qty = clean_quantized((lot.remaining_qty - take).max(0.0));
                        rem_sell = (rem_sell - take).max(0.0);
                        unsettled_sell_violation_qty =
                            clean_quantized(unsettled_sell_violation_qty + take);
                    }
                }
                lots.retain(|l| l.remaining_qty > 1.0e-9);
                if rem_sell > 1.0e-9 && venue.allows_short() {
                    short_open_qty = clean_quantized(short_open_qty + rem_sell);
                }
            }
        }
    }

    let mut total_held_qty = 0.0_f64;
    let mut sellable_qty = 0.0_f64;
    let mut locked_t2_qty = 0.0_f64;
    let mut earliest_unlock_timestamp: Option<i64> = None;

    for lot in &lots {
        total_held_qty += lot.remaining_qty;
        if lot.is_sellable_at(effective_as_of) {
            sellable_qty += lot.remaining_qty;
        } else {
            locked_t2_qty += lot.remaining_qty;
            earliest_unlock_timestamp = Some(match earliest_unlock_timestamp {
                Some(prev) => prev.min(lot.sellable_at_timestamp),
                None => lot.sellable_at_timestamp,
            });
        }
    }

    FifoSettlementSummary {
        venue,
        as_of_timestamp,
        lots,
        total_held_qty: clean_quantized(total_held_qty),
        sellable_qty: clean_quantized(sellable_qty),
        locked_t2_qty: clean_quantized(locked_t2_qty),
        earliest_unlock_timestamp,
        unsettled_sell_violation_qty,
    }
}

/// Returns `true` iff `days_since_epoch` falls within US Eastern Daylight Time (`EDT`, `UTC-4`)
/// under the post-2007 statutory rule (second Sunday of March through the Saturday before the
/// first Sunday of November; `BUG-010`).
#[inline]
#[must_use]
pub fn is_us_nyse_dst_day(days_since_epoch: i64) -> bool {
    let (year, month, day) = civil_from_days(days_since_epoch);
    match month {
        4..=10 => true,
        1 | 2 | 12 => false,
        3 => {
            let mar1_days = days_from_civil(year, 3, 1);
            let w_sun0 = (mar1_days + 4).rem_euclid(7); // 0 = Sun, 1 = Mon, .., 6 = Sat
            let first_sun = 1 + (7 - w_sun0) % 7;
            let second_sun = (first_sun + 7) as u32;
            day >= second_sun
        }
        11 => {
            let nov1_days = days_from_civil(year, 11, 1);
            let w_sun0 = (nov1_days + 4).rem_euclid(7);
            let first_sun = (1 + (7 - w_sun0) % 7) as u32;
            day < first_sun
        }
        _ => false,
    }
}

/// Returns the inclusive closing second-of-day in UTC for the regular NYSE/NASDAQ 16:00 ET
/// session on `days_since_epoch`: `19:59:59 UTC` during `EDT` (`UTC-4`) and `20:59:59 UTC`
/// during `EST` (`UTC-5`) (`BUG-010`).
#[inline]
#[must_use]
pub fn us_nyse_regular_close_tod_utc(days_since_epoch: i64) -> i64 {
    if is_us_nyse_dst_day(days_since_epoch) {
        19 * 3_600 + 59 * 60 + 59
    } else {
        20 * 3_600 + 59 * 60 + 59
    }
}
