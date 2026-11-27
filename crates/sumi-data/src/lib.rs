#![warn(clippy::all, rust_2018_idioms)]

//! `sumi-data` — Cache-First Market Data Engine for Sumi v2.0.
//!
//! Combines:
//! - Pure-Rust `tradingview-rs` (`rustls-tls` only, no `user` / `wreq` / BoringSSL / CMake / Perl)
//!   with `ProData` -> `Data` automatic fallback and rate-limited batch fetching.
//! - Local SQLite `WITHOUT ROWID` clustered OHLCV store supporting KLineChart v10 `DataLoader`
//!   pagination (`Init`, `Forward { before_ms }`, `Backward { after_ms }`) with sub-millisecond reads,
//!   online gap-fill, monotonic >11,000-bar intraday retention, and 100% offline degradation.
//! - High-speed single-pass [`CafeFImporter`] seeding 471+ Vietnamese equities & indices.

pub mod cache_service;
pub mod cafef_importer;
pub mod error;
pub mod logging;
pub mod paths;
pub mod provider;
pub mod ram_cache;
pub mod rate_limit;
pub mod storage;
pub mod tradingview_feed;
pub mod types;

pub use cache_service::{BarsSwrPage, CachePolicy, MarketDataService, collect_bar_events_for};
pub use cafef_importer::{CafeFImportConfig, CafeFImportSummary, CafeFImporter};
pub use error::{DataError, Result};
pub use logging::{RECOMMENDED_LOG_DIRECTIVES, build_safe_filter_spec, init_tracing};
pub use paths::{
    SqliteMigrationReceipt, count_missing_vn_sessions, format_ymd_utc,
    latest_expected_vn_trading_session_ms, legacy_sumi_app_data_dirs, open_url_in_system_browser,
    sha256_bytes_hex, sha256_file_hex, sqlite_backup_atomic, sumi_config_dir, sumi_data_dir,
    user_home_dir, write_private_file_0600, yumeko_config_dir, yumeko_data_dir,
};
pub use provider::{BarSubscription, MarketDataProvider, QuoteSubscription};
pub use ram_cache::{RamCacheStats, SeriesRamCache};
pub use rate_limit::{RateLimitConfig, RateLimitPermit, RateLimitStats, TokenBucketRateLimiter};
pub use storage::{
    CacheBarsResult, CorporateEventMarker, SeriesMeta, SqliteMarketStore, StoreStats,
    VnFreshnessSummary,
};
pub use tradingview_feed::{TradingViewConfig, TradingViewFeed};
pub use types::{
    Adjustment, BarQueryMode, BarsPage, BarsRequest, BatchBarsResult, DataSourceOrigin,
    HistoryFetchOutput, Interval, KLinePeriod, LiveBarEvent, LiveQuote, OhlcvBar, ResolvedSymbol,
    SeriesKey, SymbolKey, SymbolSearchResult, VariableTickTable, VariableTickTier,
    VietnamPriceBand, canonicalize_bar_timestamp_ms, compute_price_precision,
    infer_vn_exchange_for_ticker, normalize_bars, normalize_exchange_alias,
};
