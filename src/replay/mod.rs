//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Causal Bar Replay Engine (`Milestone R07 / Wave 6`).
//!
//! Enforces strict `DEF-001` zero-lookahead causality across `1m`, `5m`, `15m`, `1h`, `4h`, `1D`, and `1W`
//! timeframes by anchoring replay progression to an exact `base_cursor_index` on `base_candles` and
//! materializing active chart series exclusively from `&base_candles[..=base_cursor_index]`.

pub mod controller;
#[cfg(test)]
mod tests;

pub use controller::{
    MAX_PLAYBACK_DT_SECS, MAX_STEPS_PER_FRAME, ReplayController, ReplaySpeed, ReplayStatus,
    ReplayStepUnit, ReplayTransition, next_active_bar_cursor, prev_active_bar_cursor,
};
