//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Vietnam & Multi-Asset Market Microstructure,
//! Friction-Aware Position Sizing Risk Planner, and Structured Trade Journal (`Milestone R10 / Wave 9`).
//!
//! Enforces statutory Vietnam `HOSE` / `HNX` / `UPCOM` / `VN30F` tick ladders, daily price bands
//! (`±7%` / `±10%` / `±15%` with tier-aware ceiling-down / floor-up tick rounding), `100`-share board lots,
//! `VN30F` `100,000 VND/pt` (`100 kVND/pt`) contract multiplier & `17%` margin, `T+2` VSD local-timezone
//! settlement, all-in stop-out risk sizing (`commission_bps + tax_sell_bps + slippage_bps`),
//! `DEF-001` Bar Replay causal visibility, and formula-injection-safe CSV/Markdown exports.

pub mod ledger;
pub mod market_rules;
pub mod planner;
#[cfg(test)]
mod tests;

pub use ledger::{
    ChecklistState, JournalAnalytics, JournalEntry, JournalEntrySource, JournalEntryStatus,
    JournalSetupBreakdown, MAX_JOURNAL_ENTRIES, MAX_JOURNAL_NOTE_BYTES, MistakeTag, SetupTag,
    TradeJournalStore, sanitize_csv_cell,
};
pub use market_rules::{
    DailyPriceBand, ExchangeVenue, ResolvedVnPriceUnit, VnPriceUnit, compute_daily_price_band,
    compute_daily_price_band_with_resolved_unit, compute_settlement_timestamp,
    compute_settlement_timestamp_for_venue, floor_to_board_lot, round_down_to_tick,
    round_up_to_tick, snap_to_nearest_tick, step_one_tick_above, step_one_tick_below,
    tick_size_at_price,
};
pub use planner::{
    PlanWarning, TradePlanComputed, TradePlanError, TradePlanInput, compute_trade_plan,
};
