use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::types::{
    Adjustment, BatchBarsResult, HistoryFetchOutput, Interval, LiveBarEvent, LiveQuote,
    ResolvedSymbol, SeriesKey, SymbolKey, SymbolSearchResult,
};

/// Active live candlestick stream backed by a broadcast channel and clean cancellation lifecycle.
///
/// Dropping this subscription automatically cancels the background WebSocket supervisor so
/// no socket or background Tokio task leaks. For deterministic cleanup (awaiting session deletion
/// and socket close), call [`Self::unsubscribe`].
pub struct BarSubscription {
    series: SeriesKey,
    tx: broadcast::Sender<LiveBarEvent>,
    rx: broadcast::Receiver<LiveBarEvent>,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl BarSubscription {
    #[must_use]
    pub fn new(
        series: SeriesKey,
        tx: broadcast::Sender<LiveBarEvent>,
        cancel: CancellationToken,
        task: JoinHandle<()>,
    ) -> Self {
        let rx = tx.subscribe();
        Self {
            series,
            tx,
            rx,
            cancel,
            task: Some(task),
        }
    }

    /// The `(exchange, symbol, interval, adjustment)` key associated with this stream.
    #[must_use]
    pub fn series(&self) -> &SeriesKey {
        &self.series
    }

    /// Creates an additional fan-out receiver listening to the exact same stream.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<LiveBarEvent> {
        self.tx.subscribe()
    }

    /// Awaits the next candlestick event, skipping lagged notifications gracefully if a slow reader falls behind.
    pub async fn recv(&mut self) -> Option<LiveBarEvent> {
        loop {
            match self.rx.recv().await {
                Ok(ev) => return Some(ev),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Signals cancellation, tears down the remote chart session, closes the WebSocket, and awaits exit.
    pub async fn unsubscribe(mut self) {
        self.cancel.cancel();
        if let Some(handle) = self.task.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for BarSubscription {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Active live quote stream backed by a broadcast channel and clean cancellation lifecycle.
pub struct QuoteSubscription {
    symbols: Vec<SymbolKey>,
    tx: broadcast::Sender<LiveQuote>,
    rx: broadcast::Receiver<LiveQuote>,
    cancel: CancellationToken,
    task: Option<JoinHandle<()>>,
}

impl QuoteSubscription {
    #[must_use]
    pub fn new(
        symbols: Vec<SymbolKey>,
        tx: broadcast::Sender<LiveQuote>,
        cancel: CancellationToken,
        task: JoinHandle<()>,
    ) -> Self {
        let rx = tx.subscribe();
        Self {
            symbols,
            tx,
            rx,
            cancel,
            task: Some(task),
        }
    }

    /// Instruments currently watched by this quote subscription.
    #[must_use]
    pub fn symbols(&self) -> &[SymbolKey] {
        &self.symbols
    }

    /// Creates an additional fan-out receiver listening to the exact same quote stream.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<LiveQuote> {
        self.tx.subscribe()
    }

    /// Awaits the next merged quote update.
    pub async fn recv(&mut self) -> Option<LiveQuote> {
        loop {
            match self.rx.recv().await {
                Ok(q) => return Some(q),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Signals cancellation, deletes the remote quote session, closes the WebSocket, and awaits exit.
    pub async fn unsubscribe(mut self) {
        self.cancel.cancel();
        if let Some(handle) = self.task.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for QuoteSubscription {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Unified asynchronous abstraction over an upstream market data source
/// (implemented by [`crate::TradingViewFeed`] and mock/offline test doubles).
#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    /// Searches symbols matching `query`, optionally filtered by `exchange` (e.g. `"HOSE"`, `"BINANCE"`).
    async fn search_symbols(
        &self,
        query: &str,
        exchange: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SymbolSearchResult>>;

    /// Resolves authoritative instrument metadata (precision, tick ladder, Vietnam price bands, session).
    async fn resolve_symbol(&self, symbol: &SymbolKey) -> Result<ResolvedSymbol>;

    /// Retrieves historical OHLCV bars in ascending Unix-millisecond order, deduplicated, with `turnover`.
    ///
    /// - `num_bars`: target number of bars (when `from_ms` is `None`) or maximum bars cap.
    /// - `from_ms`: optional earliest Unix millisecond boundary enabling server pagination.
    async fn fetch_history(
        &self,
        series: &SeriesKey,
        num_bars: usize,
        from_ms: Option<i64>,
    ) -> Result<HistoryFetchOutput>;

    /// Rate-limited concurrent batch historical retrieval across many symbols (used by Scanner).
    async fn fetch_history_batch(
        &self,
        symbols: &[SymbolKey],
        interval: Interval,
        adjustment: Adjustment,
        num_bars: usize,
    ) -> BatchBarsResult;

    /// Subscribes to live real-time OHLCV bar updates for a single `(exchange, symbol, interval, adjustment)`.
    async fn subscribe_bars(&self, series: SeriesKey) -> Result<BarSubscription>;

    /// Subscribes to live real-time quote snapshots for one or more symbols.
    async fn subscribe_quotes(&self, symbols: Vec<SymbolKey>) -> Result<QuoteSubscription>;
}
