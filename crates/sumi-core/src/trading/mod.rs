//! Pure `f64`/`i64`, `wasm32-unknown-unknown`-compatible Paper Trading Execution & Portfolio Engine (`Milestone R07 / Wave 6`).
//!
//! Provides deterministic, causal OHLC bar matching (`Market`, `Limit`, `Stop`, `StopLoss`, `TakeProfit`),
//! symmetric `Long` and `Short` net-position accounting with exact fee conservation across scale-ins and
//! partial closes, and command-log prefix replay for bit-exact state reconstruction on Bar Replay rewind.

pub mod coaching;
pub mod ladder;
pub mod orders;
pub mod portfolio;
#[cfg(test)]
mod tests;

pub use coaching::{
    CoachingCallout, CoachingSeverity, MAX_DECISION_NOTE_BYTES, MAX_DECISION_NOTES,
    MAX_SAVED_PRACTICE_SESSIONS, MAX_SAVED_SCAN_SNAPSHOTS, MAX_SNAPSHOT_HITS, MistakeCostBreakdown,
    PRACTICE_STORE_SCHEMA_VERSION, PracticeDecisionAction, PracticeDecisionLog,
    PracticeDecisionNote, PracticeScoreboardSummary, PracticeSessionStore,
    SCAN_STORE_SCHEMA_VERSION, SavedPracticeSession, SavedScanSnapshot, ScanSnapshotStore,
    SessionDebriefReport, SetupEdgeBreakdown,
};

pub use ladder::{
    DOM_LADDER_DATA_FIDELITY, DomLadderConfig, DomLadderError, DomLadderRow, DomLadderSnapshot,
    DomPriceBandState, LadderStageInput, compute_causal_dom_ladder, compute_quick_market_brackets,
    exact_tick_distance, infer_causal_prior_session_close, size_ladder_qty_from_risk_budget,
    stage_ladder_order, step_ladder_price_ticks,
};
pub use orders::{
    AccountConfig, FillReason, MAX_ORDER_NOTIONAL, MAX_ORDER_PRICE, MAX_ORDER_QUANTITY,
    MAX_PENDING_ORDERS, MAX_RECORDED_COMMANDS, MIN_ORDER_QUANTITY, Order, OrderKind, OrderRequest,
    OrderSide, OrderStatus, PositionSide, TradingError, apply_slippage, compute_commission,
    validate_brackets_for_side,
};
pub use portfolio::{
    ClosedTrade, DrawingTicketSync, EquityPoint, ExecutionFill, PortfolioMetrics, Position,
    RecordedCommand, RecordedCommandKind, TradingSession, extract_ticket_from_drawing,
};
