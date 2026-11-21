use thiserror::Error;

/// Unified result alias across `sumi-data`.
pub type Result<T> = std::result::Result<T, DataError>;

/// Structured error enum for market data providers, local cache, and importers.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("invalid symbol identifier '{0}' (expected format EXCHANGE:SYMBOL, e.g. HOSE:FPT)")]
    InvalidSymbol(String),

    #[error("unsupported interval '{0}' (supported: 1m, 5m, 15m, 30m, 1h, 4h, 1D, 1W, 1M)")]
    InvalidInterval(String),

    #[error("unsupported adjustment '{0}' (supported: splits, dividends, unadjusted)")]
    InvalidAdjustment(String),

    #[error("symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("market data provider error: {0}")]
    Provider(String),

    #[error("provider unavailable (offline): {0}")]
    Offline(String),

    #[error("request timed out: {0}")]
    Timeout(String),

    #[error("rate limiter closed or saturated: {0}")]
    RateLimited(String),

    #[error("operation cancelled")]
    Cancelled,

    #[error("local storage error: {0}")]
    Storage(String),

    #[error("data import error: {0}")]
    Import(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl DataError {
    /// Returns `true` when this error represents a transport/network/availability failure
    /// where falling back to an alternate server or local offline cache is appropriate.
    #[must_use]
    pub fn is_fallback_eligible(&self) -> bool {
        match self {
            Self::Offline(_) | Self::Timeout(_) => true,
            Self::Provider(msg) => {
                let lower = msg.to_ascii_lowercase();
                lower.contains("websocket")
                    || lower.contains("connection")
                    || lower.contains("timeout")
                    || lower.contains("timed out")
                    || lower.contains("dns")
                    || lower.contains("network")
                    || lower.contains("unreachable")
                    || lower.contains("reset")
                    || lower.contains("tls")
                    || lower.contains("closed")
                    || lower.contains("http")
                    || lower.contains("eof")
            }
            _ => false,
        }
    }
}
