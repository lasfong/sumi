use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::{debug, warn};

use crate::error::Result;
use crate::provider::{BarSubscription, MarketDataProvider, QuoteSubscription};
use crate::ram_cache::{
    DEFAULT_RAM_MAX_BARS_PER_SERIES, DEFAULT_RAM_SERIES_CAPACITY, SeriesRamCache,
};
use crate::storage::{CacheBarsResult, SeriesMeta, SqliteMarketStore, StoreStats};
use crate::types::{
    Adjustment, BarQueryMode, BarsPage, BarsRequest, BatchBarsResult, DataSourceOrigin, Interval,
    OhlcvBar, ResolvedSymbol, SeriesKey, SymbolKey, SymbolSearchResult,
};

const DEFAULT_SYMBOL_TTL_MS: i64 = 24 * 60 * 60 * 1_000;
const MIN_GAP_FILL_BARS: usize = 30;
const MAX_GAP_FILL_BARS: usize = 10_000;
const RAM_WARM_MIN_BARS: usize = 1_500;

/// Result returned by [`MarketDataService::get_bars_stale_while_revalidate`].
#[derive(Debug, Clone)]
pub struct BarsSwrPage {
    /// Immediately renderable bar page (from RAM LRU or SQLite WAL, or cold remote fetch).
    pub page: BarsPage,
    /// When `true`, the caller should spawn a non-blocking [`MarketDataService::revalidate_series`]
    /// task to pull newer bars from upstream and stream updates to active charts.
    pub needs_revalidation: bool,
    /// `true` when served directly from the in-memory LRU cache (`< 0.2ms`).
    pub from_ram: bool,
}

/// Behavioral tuning knobs for [`MarketDataService`].
#[derive(Debug, Clone)]
pub struct CachePolicy {
    /// When `true`, all queries run 100% offline against local SQLite storage with zero network I/O.
    pub offline_mode: bool,
    /// Optional explicit freshness threshold (ms). When `None`, defaults to `interval.duration_ms()`.
    pub stale_threshold_ms: Option<i64>,
    /// Minimum cooldown (ms) after a successful upstream sync before re-querying on weekends/after-hours.
    pub resync_cooldown_ms: i64,
    /// Metadata cache TTL for `resolve_symbol` before refreshing online.
    pub symbol_ttl_ms: i64,
    /// Minimum bars requested when seeding a cold series from upstream.
    pub cold_fetch_min_bars: usize,
    /// Extra lookback cushion when paginating older history via `Forward { before_ms }`.
    pub forward_backfill_multiplier: usize,
    /// Maximum number of hot `(exchange, symbol, interval, adjustment)` series cached in RAM.
    pub ram_cache_capacity: usize,
    /// Maximum bars retained per series inside the RAM LRU cache.
    pub ram_max_bars_per_series: usize,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            offline_mode: false,
            stale_threshold_ms: None,
            resync_cooldown_ms: 60_000,
            symbol_ttl_ms: DEFAULT_SYMBOL_TTL_MS,
            cold_fetch_min_bars: 300,
            forward_backfill_multiplier: 2,
            ram_cache_capacity: DEFAULT_RAM_SERIES_CAPACITY,
            ram_max_bars_per_series: DEFAULT_RAM_MAX_BARS_PER_SERIES,
        }
    }
}

struct InFlightGuard {
    set: Arc<Mutex<HashSet<SeriesKey>>>,
    key: SeriesKey,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.set.lock() {
            guard.remove(&self.key);
        }
    }
}

/// Instant Local-First market data orchestrator layering an in-memory [`SeriesRamCache`] +
/// persistent [`SqliteMarketStore`] in front of any [`MarketDataProvider`]
/// (e.g. [`crate::TradingViewFeed`] or a mock provider in tests).
///
/// Guarantees:
/// - **Instant Local-First (< 2ms)**: RAM LRU + SQLite `WITHOUT ROWID` return cached bars
///   immediately without ever blocking symbol/timeframe switches on upstream network I/O.
/// - **Stale-While-Revalidate (SWR)**: stale series render instantly from local cache while a
///   deduplicated background task pulls missing bars, persists them to SQLite + RAM, and emits
///   smooth updates.
/// - **Graceful Offline Resilience**: operates 100% smoothly in explicit `offline_mode` or whenever
///   upstream network requests fail.
pub struct MarketDataService<P: MarketDataProvider + 'static> {
    provider: Arc<P>,
    store: SqliteMarketStore,
    policy: CachePolicy,
    ram_cache: Arc<SeriesRamCache>,
    in_flight_revalidations: Arc<Mutex<HashSet<SeriesKey>>>,
}

impl<P: MarketDataProvider + 'static> Clone for MarketDataService<P> {
    fn clone(&self) -> Self {
        Self {
            provider: Arc::clone(&self.provider),
            store: self.store.clone(),
            policy: self.policy.clone(),
            ram_cache: Arc::clone(&self.ram_cache),
            in_flight_revalidations: Arc::clone(&self.in_flight_revalidations),
        }
    }
}

impl<P: MarketDataProvider + 'static> MarketDataService<P> {
    #[must_use]
    pub fn new(provider: Arc<P>, store: SqliteMarketStore, policy: CachePolicy) -> Self {
        let ram_cache = Arc::new(SeriesRamCache::new(
            policy.ram_cache_capacity,
            policy.ram_max_bars_per_series,
        ));
        Self {
            provider,
            store,
            policy,
            ram_cache,
            in_flight_revalidations: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    #[must_use]
    pub fn store(&self) -> &SqliteMarketStore {
        &self.store
    }

    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }

    #[must_use]
    pub fn policy(&self) -> &CachePolicy {
        &self.policy
    }

    #[must_use]
    pub fn ram_cache(&self) -> &SeriesRamCache {
        &self.ram_cache
    }

    /// Clears the in-memory RAM cache (e.g. after bulk CafeF CSV import).
    pub fn invalidate_ram_cache(&self) {
        self.ram_cache.clear();
    }

    /// Returns a cloned service handle with `offline_mode` toggled (sharing RAM cache & locks).
    #[must_use]
    pub fn with_offline_mode(&self, offline: bool) -> Self {
        let mut policy = self.policy.clone();
        policy.offline_mode = offline;
        Self {
            provider: Arc::clone(&self.provider),
            store: self.store.clone(),
            policy,
            ram_cache: Arc::clone(&self.ram_cache),
            in_flight_revalidations: Arc::clone(&self.in_flight_revalidations),
        }
    }

    /// Reads local storage diagnostics.
    pub async fn store_stats(&self) -> Result<StoreStats> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || store.stats())
            .await
            .map_err(|e| crate::error::DataError::Storage(e.to_string()))?
    }

    /// Searches symbols online when available, falling back seamlessly to local SQLite cache
    /// when offline or when the upstream endpoint fails.
    pub async fn search_symbols(
        &self,
        query: &str,
        exchange: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SymbolSearchResult>> {
        let max_items = limit.max(1);
        let local_store = self.store.clone();
        let q_owned = query.to_owned();
        let ex_owned = exchange.map(str::to_owned);

        let local_hits = tokio::task::spawn_blocking(move || {
            local_store.search_local_symbols(&q_owned, ex_owned.as_deref(), max_items)
        })
        .await
        .map_err(|e| crate::error::DataError::Storage(e.to_string()))??;

        if self.policy.offline_mode {
            return Ok(local_hits);
        }

        match self
            .provider
            .search_symbols(query, exchange, max_items)
            .await
        {
            Ok(mut remote) => {
                let mut seen: HashSet<String> = remote.iter().map(|r| r.ticker.clone()).collect();
                for hit in local_hits {
                    if remote.len() >= max_items {
                        break;
                    }
                    if seen.insert(hit.ticker.clone()) {
                        remote.push(hit);
                    }
                }
                remote.truncate(max_items);
                Ok(remote)
            }
            Err(err) if !local_hits.is_empty() || err.is_fallback_eligible() => {
                warn!(query, ?exchange, error = %err, "online symbol search unavailable; serving local cache");
                Ok(local_hits)
            }
            Err(err) => Err(err),
        }
    }

    /// Resolves symbol metadata cache-first; refreshes online when missing or past TTL,
    /// and degrades cleanly to cached metadata when offline.
    pub async fn resolve_symbol(&self, symbol: &SymbolKey) -> Result<ResolvedSymbol> {
        let cached = self.get_cached_symbol(symbol).await?;
        let now_ms = chrono::Utc::now().timestamp_millis();

        if let Some((ref info, updated_ms)) = cached {
            let fresh = now_ms.saturating_sub(updated_ms) < self.policy.symbol_ttl_ms;
            if self.policy.offline_mode || fresh {
                return Ok(info.clone());
            }
        } else if self.policy.offline_mode {
            return Err(crate::error::DataError::Offline(format!(
                "symbol {symbol} not present in local cache (offline mode)"
            )));
        }

        match self.provider.resolve_symbol(symbol).await {
            Ok(resolved) => {
                let mut canonical = resolved.clone();
                if canonical.exchange != symbol.exchange || canonical.symbol != symbol.symbol {
                    canonical.exchange = symbol.exchange.clone();
                    canonical.symbol = symbol.symbol.clone();
                    canonical.ticker = symbol.ticker();
                }
                let store = self.store.clone();
                let to_save = canonical.clone();
                let now_save = chrono::Utc::now().timestamp_millis();
                self.ram_cache
                    .put_symbol(symbol.clone(), canonical.clone(), now_save);
                let _ = tokio::task::spawn_blocking(move || {
                    let _ = store.upsert_symbol(&to_save);
                })
                .await;
                Ok(canonical)
            }
            Err(err) => {
                if let Some((stale_info, _)) = cached {
                    warn!(
                        ticker = %symbol,
                        error = %err,
                        "online resolve_symbol failed; serving cached metadata"
                    );
                    Ok(stale_info)
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Instant RAM/disk-only bar query (`< 0.2ms` RAM hit, `< 2ms` SQLite warm) returning cached
    /// bars immediately without awaiting upstream network I/O.
    pub async fn get_bars_instant(&self, req: &BarsRequest) -> Result<BarsPage> {
        let (cached, _meta, _from_ram) = self
            .query_cache_with_ram(&req.key, req.mode, req.limit)
            .await?;
        let symbol_info = self
            .get_cached_symbol(&req.key.symbol_key())
            .await?
            .map(|(s, _)| s);
        Ok(BarsPage {
            bars: cached.bars,
            has_older: cached.has_older,
            has_newer: cached.has_newer,
            source: if self.policy.offline_mode {
                DataSourceOrigin::OfflineCache
            } else {
                DataSourceOrigin::CacheHit
            },
            symbol_info,
        })
    }

    /// Instant Local-First **Stale-While-Revalidate (SWR)** bar query.
    ///
    /// - If RAM LRU or SQLite already holds bars for `Init` (even if stale!), returns them
    ///   **immediately (`< 2ms`)** with `needs_revalidation = true` when upstream refresh is due,
    ///   so symbol & timeframe switches never block on network round-trips.
    /// - Only awaits upstream synchronously when local cache is completely empty (cold series)
    ///   or when `Forward` pagination explicitly needs older bars from upstream.
    pub async fn get_bars_stale_while_revalidate(&self, req: &BarsRequest) -> Result<BarsSwrPage> {
        let (cached, meta, from_ram) = self
            .query_cache_with_ram(&req.key, req.mode, req.limit)
            .await?;
        let cached_sym = self
            .get_cached_symbol(&req.key.symbol_key())
            .await?
            .map(|(s, _)| s);

        if self.policy.offline_mode {
            return Ok(BarsSwrPage {
                page: BarsPage {
                    bars: cached.bars,
                    has_older: cached.has_older,
                    has_newer: cached.has_newer,
                    source: DataSourceOrigin::OfflineCache,
                    symbol_info: cached_sym,
                },
                needs_revalidation: false,
                from_ram,
            });
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        let stale_ms = self
            .policy
            .stale_threshold_ms
            .unwrap_or_else(|| req.key.interval.duration_ms());

        match req.mode {
            BarQueryMode::Init => {
                if !cached.bars.is_empty() {
                    let (needs_revalidation, has_older) = match meta {
                        Some(series_meta) => {
                            let recently_synced = series_meta.source.starts_with("tradingview")
                                && now_ms.saturating_sub(series_meta.updated_at_ms)
                                    < self.policy.resync_cooldown_ms;
                            let is_stale = !recently_synced
                                && now_ms.saturating_sub(series_meta.latest_ms) >= stale_ms;
                            let insufficient =
                                cached.bars.len() < req.limit && !series_meta.reached_start;
                            (
                                is_stale || insufficient,
                                cached.has_older || !series_meta.reached_start,
                            )
                        }
                        None => (true, cached.has_older),
                    };
                    return Ok(BarsSwrPage {
                        page: BarsPage {
                            bars: cached.bars,
                            has_older,
                            has_newer: false,
                            source: DataSourceOrigin::CacheHit,
                            symbol_info: cached_sym,
                        },
                        needs_revalidation,
                        from_ram,
                    });
                }

                // Cold cache (zero local bars): perform synchronous initial fetch.
                let page = self.get_bars(req).await?;
                Ok(BarsSwrPage {
                    page,
                    needs_revalidation: false,
                    from_ram: false,
                })
            }
            BarQueryMode::Forward { .. } => {
                let reached = meta.as_ref().is_some_and(|m| m.reached_start);
                if cached.bars.len() >= req.limit || reached {
                    return Ok(BarsSwrPage {
                        page: BarsPage {
                            bars: cached.bars,
                            has_older: cached.has_older || !reached,
                            has_newer: cached.has_newer,
                            source: DataSourceOrigin::CacheHit,
                            symbol_info: cached_sym,
                        },
                        needs_revalidation: false,
                        from_ram,
                    });
                }
                let page = self.get_bars(req).await?;
                Ok(BarsSwrPage {
                    page,
                    needs_revalidation: false,
                    from_ram: false,
                })
            }
            BarQueryMode::Backward { after_ms } => {
                let latest_local = meta.as_ref().map_or(after_ms, |m| m.latest_ms);
                let is_stale = now_ms.saturating_sub(latest_local) >= stale_ms;
                if !cached.bars.is_empty() || !is_stale {
                    return Ok(BarsSwrPage {
                        page: BarsPage {
                            bars: cached.bars,
                            has_older: cached.has_older,
                            has_newer: cached.has_newer,
                            source: DataSourceOrigin::CacheHit,
                            symbol_info: cached_sym,
                        },
                        needs_revalidation: is_stale,
                        from_ram,
                    });
                }
                let page = self.get_bars(req).await?;
                Ok(BarsSwrPage {
                    page,
                    needs_revalidation: false,
                    from_ram: false,
                })
            }
        }
    }

    /// Performs a deduplicated non-blocking background sync for `key`, persists any fetched bars
    /// into SQLite + RAM LRU, and returns only the newest/updated bars (`timestamp >= prev_latest_ms`)
    /// so active charts can apply them incrementally via `subscribeBar` without repaint flicker.
    pub async fn revalidate_series(&self, key: &SeriesKey, limit: usize) -> Result<Vec<OhlcvBar>> {
        if self.policy.offline_mode {
            return Ok(Vec::new());
        }

        // Deduplicate concurrent background refreshes for the exact same series.
        {
            let mut guard = self.in_flight_revalidations.lock().map_err(|_| {
                crate::error::DataError::Storage("in_flight mutex poisoned".to_owned())
            })?;
            if !guard.insert(key.clone()) {
                return Ok(Vec::new());
            }
        }
        let _inflight = InFlightGuard {
            set: Arc::clone(&self.in_flight_revalidations),
            key: key.clone(),
        };

        let meta = self.get_series_meta(key).await?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let prev_latest_ms = meta.as_ref().map(|m| m.latest_ms);

        let (fetch_bars, from_ms) = match meta {
            None => (limit.max(self.policy.cold_fetch_min_bars), None),
            Some(ref m) => {
                if m.bar_count < limit && !m.reached_start {
                    (limit.max(self.policy.cold_fetch_min_bars), None)
                } else {
                    let missing_est = ((now_ms.saturating_sub(m.latest_ms))
                        / key.interval.duration_ms().max(1))
                        as usize;
                    let count = (missing_est + 10).clamp(MIN_GAP_FILL_BARS, MAX_GAP_FILL_BARS);
                    let from_anchor = m.latest_ms.saturating_sub(key.interval.duration_ms());
                    (count, Some(from_anchor))
                }
            }
        };

        let fetched = self
            .provider
            .fetch_history(key, fetch_bars, from_ms)
            .await?;
        let reached_start = if from_ms.is_none() {
            Some(fetched.reached_start_of_history)
        } else {
            None
        };
        self.persist_fetch_with_flag(key, &fetched, "tradingview", reached_start)
            .await?;

        let delta: Vec<OhlcvBar> = match prev_latest_ms {
            Some(cutoff) => fetched
                .bars
                .into_iter()
                .filter(|b| b.timestamp >= cutoff)
                .collect(),
            None => fetched.bars,
        };
        Ok(delta)
    }

    /// Cache-first paginated OHLCV query supporting all 3 KLineChart v10 `DataLoader` modes
    /// (`Init`, `Forward { before_ms }`, `Backward { after_ms }`) with automatic online gap-fill
    /// and graceful offline fallback.
    pub async fn get_bars(&self, req: &BarsRequest) -> Result<BarsPage> {
        let (cached, meta, _from_ram) = self
            .query_cache_with_ram(&req.key, req.mode, req.limit)
            .await?;
        let cached_sym = self
            .get_cached_symbol(&req.key.symbol_key())
            .await?
            .map(|(s, _)| s);

        if self.policy.offline_mode {
            return Ok(BarsPage {
                bars: cached.bars,
                has_older: cached.has_older,
                has_newer: cached.has_newer,
                source: DataSourceOrigin::OfflineCache,
                symbol_info: cached_sym,
            });
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        let stale_ms = self
            .policy
            .stale_threshold_ms
            .unwrap_or_else(|| req.key.interval.duration_ms());

        match req.mode {
            BarQueryMode::Init => {
                match meta {
                    None => {
                        // Cold cache: fetch initial window from upstream and persist.
                        let fetch_count = req.limit.max(self.policy.cold_fetch_min_bars);
                        let fetched = self
                            .provider
                            .fetch_history(&req.key, fetch_count, None)
                            .await?;
                        self.persist_fetch(&req.key, &fetched, "tradingview")
                            .await?;
                        let fresh = self.query_cache(&req.key, req.mode, req.limit).await?;
                        let has_older = fresh.has_older || !fetched.reached_start_of_history;
                        Ok(BarsPage {
                            bars: fresh.bars,
                            has_older,
                            has_newer: false,
                            source: DataSourceOrigin::RemoteLive,
                            symbol_info: fetched.symbol_info.or(cached_sym),
                        })
                    }
                    Some(series_meta) => {
                        let recently_synced = series_meta.source.starts_with("tradingview")
                            && now_ms.saturating_sub(series_meta.updated_at_ms)
                                < self.policy.resync_cooldown_ms;
                        let is_stale = !recently_synced
                            && now_ms.saturating_sub(series_meta.latest_ms) >= stale_ms;
                        let insufficient =
                            cached.bars.len() < req.limit && !series_meta.reached_start;

                        if !is_stale && !insufficient {
                            let has_older = cached.has_older || !series_meta.reached_start;
                            return Ok(BarsPage {
                                bars: cached.bars,
                                has_older,
                                has_newer: false,
                                source: DataSourceOrigin::CacheHit,
                                symbol_info: cached_sym,
                            });
                        }

                        // Gap-fill missing newest bars (or backfill if cache has fewer bars than requested).
                        let (fetch_bars, from_ms) = if insufficient {
                            (req.limit.max(self.policy.cold_fetch_min_bars), None)
                        } else {
                            let missing_est = ((now_ms.saturating_sub(series_meta.latest_ms))
                                / req.key.interval.duration_ms().max(1))
                                as usize;
                            let count =
                                (missing_est + 10).clamp(MIN_GAP_FILL_BARS, MAX_GAP_FILL_BARS);
                            let from_anchor = series_meta
                                .latest_ms
                                .saturating_sub(req.key.interval.duration_ms());
                            (count, Some(from_anchor))
                        };

                        match self
                            .provider
                            .fetch_history(&req.key, fetch_bars, from_ms)
                            .await
                        {
                            Ok(fetched) => {
                                let reached_start = if from_ms.is_none() {
                                    Some(fetched.reached_start_of_history)
                                } else {
                                    None
                                };
                                self.persist_fetch_with_flag(
                                    &req.key,
                                    &fetched,
                                    "tradingview",
                                    reached_start,
                                )
                                .await?;
                                let fresh = self.query_cache(&req.key, req.mode, req.limit).await?;
                                let updated_meta = self.get_series_meta(&req.key).await?;
                                let reached =
                                    updated_meta.as_ref().is_some_and(|m| m.reached_start);
                                Ok(BarsPage {
                                    bars: fresh.bars,
                                    has_older: fresh.has_older || !reached,
                                    has_newer: false,
                                    source: DataSourceOrigin::CacheWithGapFill,
                                    symbol_info: fetched.symbol_info.or(cached_sym),
                                })
                            }
                            Err(err) => {
                                warn!(
                                    series = %req.key,
                                    error = %err,
                                    "online gap-fill failed; serving locally cached bars offline"
                                );
                                Ok(BarsPage {
                                    bars: cached.bars,
                                    has_older: cached.has_older,
                                    has_newer: false,
                                    source: DataSourceOrigin::OfflineCache,
                                    symbol_info: cached_sym,
                                })
                            }
                        }
                    }
                }
            }
            BarQueryMode::Forward { before_ms } => {
                // Fast path: disk already holds a full page strictly older than `before_ms`.
                if cached.bars.len() >= req.limit {
                    let reached = meta.as_ref().is_some_and(|m| m.reached_start);
                    return Ok(BarsPage {
                        bars: cached.bars,
                        has_older: cached.has_older || !reached,
                        has_newer: cached.has_newer,
                        source: DataSourceOrigin::CacheHit,
                        symbol_info: cached_sym,
                    });
                }

                // If we already know upstream has no older history, return whatever remains locally.
                if meta.as_ref().is_some_and(|m| m.reached_start) {
                    return Ok(BarsPage {
                        bars: cached.bars,
                        has_older: cached.has_older,
                        has_newer: cached.has_newer,
                        source: DataSourceOrigin::CacheHit,
                        symbol_info: cached_sym,
                    });
                }

                // Attempt remote backfill walking further into the past.
                let existing_count = meta.as_ref().map_or(0, |m| m.bar_count);
                let step = req
                    .limit
                    .saturating_mul(self.policy.forward_backfill_multiplier.max(1));
                let target_total = existing_count.saturating_add(step).clamp(req.limit, 16_500);
                let target_from_ms = before_ms
                    .saturating_sub(req.key.interval.duration_ms().saturating_mul(step as i64));

                match self
                    .provider
                    .fetch_history(&req.key, target_total, Some(target_from_ms))
                    .await
                {
                    Ok(fetched) => {
                        self.persist_fetch(&req.key, &fetched, "tradingview")
                            .await?;
                        let fresh = self.query_cache(&req.key, req.mode, req.limit).await?;
                        let reached = fetched.reached_start_of_history
                            || self
                                .get_series_meta(&req.key)
                                .await?
                                .is_some_and(|m| m.reached_start);
                        let has_older = fresh.has_older || (!reached && !fresh.bars.is_empty());
                        Ok(BarsPage {
                            bars: fresh.bars,
                            has_older,
                            has_newer: fresh.has_newer,
                            source: DataSourceOrigin::CacheWithGapFill,
                            symbol_info: fetched.symbol_info.or(cached_sym),
                        })
                    }
                    Err(err) => {
                        warn!(
                            series = %req.key,
                            before_ms,
                            error = %err,
                            "forward backfill failed; returning cached bars"
                        );
                        Ok(BarsPage {
                            bars: cached.bars,
                            has_older: cached.has_older,
                            has_newer: cached.has_newer,
                            source: DataSourceOrigin::OfflineCache,
                            symbol_info: cached_sym,
                        })
                    }
                }
            }
            BarQueryMode::Backward { after_ms } => {
                // If disk already has newer bars up to `limit`, return immediately (< 5ms).
                if cached.bars.len() >= req.limit {
                    return Ok(BarsPage {
                        bars: cached.bars,
                        has_older: cached.has_older,
                        has_newer: cached.has_newer,
                        source: DataSourceOrigin::CacheHit,
                        symbol_info: cached_sym,
                    });
                }

                // Check if the right edge of local storage might be stale; if so, gap-fill.
                let latest_local = meta.as_ref().map_or(after_ms, |m| m.latest_ms);
                let is_stale = now_ms.saturating_sub(latest_local) >= stale_ms;
                if !is_stale {
                    return Ok(BarsPage {
                        bars: cached.bars,
                        has_older: cached.has_older,
                        has_newer: cached.has_newer,
                        source: DataSourceOrigin::CacheHit,
                        symbol_info: cached_sym,
                    });
                }

                let missing_est = ((now_ms.saturating_sub(latest_local))
                    / req.key.interval.duration_ms().max(1))
                    as usize;
                let count = (missing_est + 10).clamp(MIN_GAP_FILL_BARS, MAX_GAP_FILL_BARS);
                let from_anchor = latest_local.saturating_sub(req.key.interval.duration_ms());

                match self
                    .provider
                    .fetch_history(&req.key, count, Some(from_anchor))
                    .await
                {
                    Ok(fetched) => {
                        self.persist_fetch_with_flag(&req.key, &fetched, "tradingview", None)
                            .await?;
                        let fresh = self.query_cache(&req.key, req.mode, req.limit).await?;
                        Ok(BarsPage {
                            bars: fresh.bars,
                            has_older: fresh.has_older,
                            has_newer: fresh.has_newer,
                            source: DataSourceOrigin::CacheWithGapFill,
                            symbol_info: fetched.symbol_info.or(cached_sym),
                        })
                    }
                    Err(err) => {
                        warn!(
                            series = %req.key,
                            after_ms,
                            error = %err,
                            "backward gap-fill failed; serving local cache"
                        );
                        Ok(BarsPage {
                            bars: cached.bars,
                            has_older: cached.has_older,
                            has_newer: cached.has_newer,
                            source: DataSourceOrigin::OfflineCache,
                            symbol_info: cached_sym,
                        })
                    }
                }
            }
        }
    }

    /// Spawns a non-blocking background gap-fill refresh (pairs with [`Self::get_bars_instant`]).
    #[must_use]
    pub fn spawn_background_sync(
        &self,
        key: SeriesKey,
        limit: usize,
    ) -> JoinHandle<Result<BarsPage>> {
        let svc = self.clone();
        tokio::spawn(async move { svc.get_bars(&BarsRequest::init(key, limit)).await })
    }

    /// Rate-limited batch sync across many symbols, persisting every retrieved series to SQLite.
    pub async fn sync_batch(
        &self,
        symbols: &[SymbolKey],
        interval: Interval,
        adjustment: Adjustment,
        num_bars: usize,
    ) -> BatchBarsResult {
        if self.policy.offline_mode {
            return BatchBarsResult {
                successful: Vec::new(),
                failed: symbols
                    .iter()
                    .map(|s| (s.clone(), "offline mode enabled".to_owned()))
                    .collect(),
                elapsed_ms: 0,
            };
        }
        let batch = self
            .provider
            .fetch_history_batch(symbols, interval, adjustment, num_bars)
            .await;

        let store = self.store.clone();
        let to_store: Vec<(SeriesKey, Vec<crate::types::OhlcvBar>, String)> = batch
            .successful
            .iter()
            .map(|(sym, bars)| {
                (
                    SeriesKey::new(sym, interval, adjustment),
                    bars.clone(),
                    "tradingview".to_owned(),
                )
            })
            .collect();

        if !to_store.is_empty() {
            let ram = Arc::clone(&self.ram_cache);
            let to_merge: Vec<(SeriesKey, Vec<OhlcvBar>)> = to_store
                .iter()
                .map(|(k, bars, _)| (k.clone(), bars.clone()))
                .collect();
            let _ = tokio::task::spawn_blocking(move || {
                store.bulk_import_series_and_symbols(&to_store, &[], false)
            })
            .await;
            for (k, bars) in to_merge {
                ram.merge_bars(&k, &bars, None);
            }
        }
        batch
    }

    /// Subscribes to live bars and automatically archives incoming bars into SQLite + RAM LRU so
    /// intraday history grows continuously across sessions beyond TradingView's server depth limit.
    pub async fn subscribe_bars_with_archival(
        &self,
        series: SeriesKey,
    ) -> Result<(BarSubscription, JoinHandle<()>)> {
        if self.policy.offline_mode {
            return Err(crate::error::DataError::Offline(
                "cannot open live bar stream while offline_mode is enabled".to_owned(),
            ));
        }
        let sub = self.provider.subscribe_bars(series.clone()).await?;
        let mut rx = sub.subscribe();
        let store = self.store.clone();
        let ram = Arc::clone(&self.ram_cache);

        let archiver = tokio::spawn(async move {
            while let Ok(ev) = rx.recv().await {
                let s = store.clone();
                let key = ev.series.clone();
                let bar = ev.bar;
                ram.merge_bars(&key, &[bar], None);
                let _ = tokio::task::spawn_blocking(move || {
                    if let Err(err) = s.upsert_bar(&key, &bar, "tradingview_live") {
                        debug!(series = %key, %err, "failed to archive live bar");
                    }
                })
                .await;
            }
        });

        Ok((sub, archiver))
    }

    /// Delegates real-time quote subscription to the underlying provider.
    pub async fn subscribe_quotes(&self, symbols: Vec<SymbolKey>) -> Result<QuoteSubscription> {
        if self.policy.offline_mode {
            return Err(crate::error::DataError::Offline(
                "cannot open live quote stream while offline_mode is enabled".to_owned(),
            ));
        }
        self.provider.subscribe_quotes(symbols).await
    }

    async fn query_cache_with_ram(
        &self,
        key: &SeriesKey,
        mode: BarQueryMode,
        limit: usize,
    ) -> Result<(CacheBarsResult, Option<SeriesMeta>, bool)> {
        if let Some((res, meta)) = self.ram_cache.query_bars(key, mode, limit) {
            return Ok((res, Some(meta), true));
        }

        let store = self.store.clone();
        let key_owned = key.clone();
        let warm_limit = limit
            .max(RAM_WARM_MIN_BARS)
            .min(self.policy.ram_max_bars_per_series);
        let ram = Arc::clone(&self.ram_cache);

        let (page, meta) = tokio::task::spawn_blocking(move || -> Result<_> {
            let meta = store.series_meta(&key_owned)?;
            match mode {
                BarQueryMode::Init => {
                    let warm = store.query_bars(&key_owned, BarQueryMode::Init, warm_limit)?;
                    if !warm.bars.is_empty()
                        && let Some(ref m) = meta
                    {
                        ram.put_series(
                            key_owned.clone(),
                            warm.bars.clone(),
                            warm.has_older,
                            m.clone(),
                        );
                    }
                    let start = warm.bars.len().saturating_sub(limit.max(1));
                    let has_older = start > 0 || warm.has_older;
                    Ok((
                        CacheBarsResult {
                            bars: warm.bars[start..].to_vec(),
                            has_older,
                            has_newer: false,
                        },
                        meta,
                    ))
                }
                _ => {
                    let page = store.query_bars(&key_owned, mode, limit)?;
                    Ok((page, meta))
                }
            }
        })
        .await
        .map_err(|e| crate::error::DataError::Storage(e.to_string()))??;

        Ok((page, meta, false))
    }

    async fn query_cache(
        &self,
        key: &SeriesKey,
        mode: BarQueryMode,
        limit: usize,
    ) -> Result<CacheBarsResult> {
        let (res, _, _) = self.query_cache_with_ram(key, mode, limit).await?;
        Ok(res)
    }

    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<SeriesMeta>> {
        let store = self.store.clone();
        let key_owned = key.clone();
        tokio::task::spawn_blocking(move || store.series_meta(&key_owned))
            .await
            .map_err(|e| crate::error::DataError::Storage(e.to_string()))?
    }

    async fn get_cached_symbol(&self, key: &SymbolKey) -> Result<Option<(ResolvedSymbol, i64)>> {
        if let Some(hit) = self.ram_cache.get_symbol(key) {
            return Ok(Some(hit));
        }
        let store = self.store.clone();
        let key_owned = key.clone();
        let res = tokio::task::spawn_blocking(move || store.get_symbol(&key_owned))
            .await
            .map_err(|e| crate::error::DataError::Storage(e.to_string()))??;
        if let Some((ref sym, updated_ms)) = res {
            self.ram_cache
                .put_symbol(key.clone(), sym.clone(), updated_ms);
        }
        Ok(res)
    }

    async fn persist_fetch(
        &self,
        key: &SeriesKey,
        fetched: &crate::types::HistoryFetchOutput,
        source: &str,
    ) -> Result<()> {
        self.persist_fetch_with_flag(key, fetched, source, Some(fetched.reached_start_of_history))
            .await
    }

    async fn persist_fetch_with_flag(
        &self,
        key: &SeriesKey,
        fetched: &crate::types::HistoryFetchOutput,
        source: &str,
        reached_start: Option<bool>,
    ) -> Result<()> {
        let store = self.store.clone();
        let key_owned = key.clone();
        let bars = fetched.bars.clone();
        let sym_info = fetched.symbol_info.clone().map(|mut info| {
            if info.exchange != key.exchange || info.symbol != key.symbol {
                info.exchange = key.exchange.clone();
                info.symbol = key.symbol.clone();
                info.ticker = key.ticker();
            }
            info
        });
        let src = source.to_owned();
        let bars_for_ram = bars.clone();
        let sym_for_ram = sym_info.clone();
        let updated_meta = tokio::task::spawn_blocking(move || -> Result<Option<SeriesMeta>> {
            store.upsert_bars(&key_owned, &bars, &src, reached_start)?;
            if let Some(info) = sym_info {
                store.upsert_symbol(&info)?;
            }
            store.series_meta(&key_owned)
        })
        .await
        .map_err(|e| crate::error::DataError::Storage(e.to_string()))??;

        self.ram_cache.merge_bars(key, &bars_for_ram, updated_meta);
        if let Some(info) = sym_for_ram {
            self.ram_cache.put_symbol(
                key.symbol_key(),
                info,
                chrono::Utc::now().timestamp_millis(),
            );
        }
        Ok(())
    }
}

/// Convenience timeout helper for CLI or test stream collection.
pub async fn collect_bar_events_for(
    sub: &mut BarSubscription,
    max_events: usize,
    duration: Duration,
) -> Vec<crate::types::LiveBarEvent> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + duration;
    while events.len() < max_events {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, sub.recv()).await {
            Ok(Some(ev)) => events.push(ev),
            Ok(None) | Err(_) => break,
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use tokio::sync::broadcast;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::error::DataError;
    use crate::types::{HistoryFetchOutput, LiveBarEvent, OhlcvBar};

    struct MockProvider {
        bars: Mutex<Vec<OhlcvBar>>,
        fail_network: AtomicBool,
        fetch_calls: AtomicUsize,
        reached_start: AtomicBool,
        delay_ms: AtomicUsize,
    }

    impl MockProvider {
        fn new(bars: Vec<OhlcvBar>) -> Self {
            Self {
                bars: Mutex::new(bars),
                fail_network: AtomicBool::new(false),
                fetch_calls: AtomicUsize::new(0),
                reached_start: AtomicBool::new(true),
                delay_ms: AtomicUsize::new(0),
            }
        }

        fn set_delay_ms(&self, delay_ms: usize) {
            self.delay_ms.store(delay_ms, Ordering::SeqCst);
        }

        fn set_offline(&self, offline: bool) {
            self.fail_network.store(offline, Ordering::SeqCst);
        }

        fn calls(&self) -> usize {
            self.fetch_calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl MarketDataProvider for MockProvider {
        async fn search_symbols(
            &self,
            query: &str,
            exchange: Option<&str>,
            _limit: usize,
        ) -> Result<Vec<SymbolSearchResult>> {
            if self.fail_network.load(Ordering::SeqCst) {
                return Err(DataError::Offline("simulated network outage".to_owned()));
            }
            Ok(vec![SymbolSearchResult {
                ticker: format!(
                    "{}:{}",
                    exchange.unwrap_or("HOSE"),
                    query.to_ascii_uppercase()
                ),
                exchange: exchange.unwrap_or("HOSE").to_owned(),
                prefix: exchange.unwrap_or("HOSE").to_owned(),
                symbol: query.to_ascii_uppercase(),
                description: "Mock Instrument".to_owned(),
                market_type: "stock".to_owned(),
                currency_code: "VND".to_owned(),
                country_code: "VN".to_owned(),
                provider_id: "mock".to_owned(),
                type_specs: Vec::new(),
            }])
        }

        async fn resolve_symbol(&self, symbol: &SymbolKey) -> Result<ResolvedSymbol> {
            if self.fail_network.load(Ordering::SeqCst) {
                return Err(DataError::Offline("simulated network outage".to_owned()));
            }
            Ok(ResolvedSymbol::from_cafef_vn_ticker(
                &symbol.exchange,
                &symbol.symbol,
                false,
            ))
        }

        async fn fetch_history(
            &self,
            series: &SeriesKey,
            num_bars: usize,
            from_ms: Option<i64>,
        ) -> Result<HistoryFetchOutput> {
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            let delay = self.delay_ms.load(Ordering::SeqCst);
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
            }
            if self.fail_network.load(Ordering::SeqCst) {
                return Err(DataError::Offline("simulated network outage".to_owned()));
            }
            let all = self.bars.lock().unwrap().clone();
            let filtered: Vec<OhlcvBar> = if let Some(from) = from_ms {
                all.into_iter().filter(|b| b.timestamp >= from).collect()
            } else {
                all
            };
            let start = filtered.len().saturating_sub(num_bars);
            Ok(HistoryFetchOutput {
                bars: filtered[start..].to_vec(),
                symbol_info: Some(ResolvedSymbol::from_cafef_vn_ticker(
                    &series.exchange,
                    &series.symbol,
                    false,
                )),
                reached_start_of_history: self.reached_start.load(Ordering::SeqCst),
                pages_fetched: 0,
            })
        }

        async fn fetch_history_batch(
            &self,
            symbols: &[SymbolKey],
            interval: Interval,
            adjustment: Adjustment,
            num_bars: usize,
        ) -> BatchBarsResult {
            let mut successful = Vec::new();
            let mut failed = Vec::new();
            for sym in symbols {
                let sk = SeriesKey::new(sym, interval, adjustment);
                match self.fetch_history(&sk, num_bars, None).await {
                    Ok(out) => successful.push((sym.clone(), out.bars)),
                    Err(e) => failed.push((sym.clone(), e.to_string())),
                }
            }
            BatchBarsResult {
                successful,
                failed,
                elapsed_ms: 1,
            }
        }

        async fn subscribe_bars(&self, series: SeriesKey) -> Result<BarSubscription> {
            let (tx, _) = broadcast::channel::<LiveBarEvent>(32);
            let cancel = CancellationToken::new();
            let cancel_c = cancel.clone();
            let tx_c = tx.clone();
            let series_c = series.clone();
            let task = tokio::spawn(async move {
                let bar = OhlcvBar::new(
                    chrono::Utc::now().timestamp_millis(),
                    52_000.0,
                    52_500.0,
                    51_900.0,
                    52_300.0,
                    12_000.0,
                );
                let _ = tx_c.send(LiveBarEvent {
                    series: series_c,
                    bar,
                    is_update: true,
                });
                cancel_c.cancelled().await;
            });
            Ok(BarSubscription::new(series, tx, cancel, task))
        }

        async fn subscribe_quotes(&self, symbols: Vec<SymbolKey>) -> Result<QuoteSubscription> {
            let (tx, _) = broadcast::channel(32);
            let cancel = CancellationToken::new();
            let cancel_c = cancel.clone();
            let task = tokio::spawn(async move {
                cancel_c.cancelled().await;
            });
            Ok(QuoteSubscription::new(symbols, tx, cancel, task))
        }
    }

    fn build_recent_bars(count: usize, step_ms: i64, end_ms: i64) -> Vec<OhlcvBar> {
        let start = end_ms - ((count as i64 - 1) * step_ms);
        (0..count)
            .map(|i| {
                let ts = start + (i as i64) * step_ms;
                let px = 50_000.0 + (i as f64) * 50.0;
                OhlcvBar::new(ts, px, px + 100.0, px - 100.0, px + 40.0, 5_000.0)
            })
            .collect()
    }

    #[tokio::test]
    async fn cache_first_hit_gap_fill_pagination_and_offline_degradation() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let step = Interval::M15.duration_ms();
        // Server holds 20 bars ending right now.
        let full_series = build_recent_bars(20, step, now_ms);
        let provider = Arc::new(MockProvider::new(full_series.clone()));
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let sym = SymbolKey::new("HOSE", "FPT").unwrap();
        let key = SeriesKey::new(&sym, Interval::M15, Adjustment::Splits);

        // Pre-seed cache with older 15 bars (missing newest 5 bars -> stale!).
        store
            .upsert_bars(&key, &full_series[..15], "seed", Some(true))
            .unwrap();

        let service = MarketDataService::new(
            Arc::clone(&provider),
            store.clone(),
            CachePolicy {
                cold_fetch_min_bars: 10,
                ..CachePolicy::default()
            },
        );

        // 1. Instant read returns cached bars immediately without network calls.
        let instant = service
            .get_bars_instant(&BarsRequest::init(key.clone(), 5))
            .await
            .unwrap();
        assert_eq!(instant.source, DataSourceOrigin::CacheHit);
        assert_eq!(instant.bars.len(), 5);
        assert_eq!(
            instant.bars.last().unwrap().timestamp,
            full_series[14].timestamp
        );
        assert_eq!(provider.calls(), 0);

        // 2. Online `get_bars(Init)` detects stale cache, gap-fills newest 5 bars, and preserves all 20!
        let filled = service
            .get_bars(&BarsRequest::init(key.clone(), 5))
            .await
            .unwrap();
        assert_eq!(filled.source, DataSourceOrigin::CacheWithGapFill);
        assert_eq!(filled.bars.len(), 5);
        assert_eq!(
            filled.bars.last().unwrap().timestamp,
            full_series[19].timestamp
        );
        assert!(filled.has_older);
        assert!(!filled.has_newer);
        assert_eq!(provider.calls(), 1);
        assert_eq!(store.series_meta(&key).unwrap().unwrap().bar_count, 20);

        // 3. Subsequent `get_bars(Init)` is a pure CacheHit (0 extra network calls!).
        let cached_again = service
            .get_bars(&BarsRequest::init(key.clone(), 5))
            .await
            .unwrap();
        assert_eq!(cached_again.source, DataSourceOrigin::CacheHit);
        assert_eq!(provider.calls(), 1);

        // 4. Forward pagination (scrolling left) reads purely from cache.
        let fwd = service
            .get_bars(&BarsRequest::forward(
                key.clone(),
                cached_again.bars[0].timestamp,
                5,
            ))
            .await
            .unwrap();
        assert_eq!(fwd.source, DataSourceOrigin::CacheHit);
        assert_eq!(fwd.bars.len(), 5);
        assert_eq!(fwd.bars[0].timestamp, full_series[10].timestamp);
        assert!(fwd.has_older);
        assert!(fwd.has_newer);
        assert_eq!(provider.calls(), 1);

        // 5. Backward pagination (scrolling right) returns newer page cleanly.
        let bwd = service
            .get_bars(&BarsRequest::backward(
                key.clone(),
                fwd.bars.last().unwrap().timestamp,
                5,
            ))
            .await
            .unwrap();
        assert_eq!(bwd.bars.len(), 5);
        assert_eq!(
            bwd.bars.last().unwrap().timestamp,
            full_series[19].timestamp
        );
        assert!(bwd.has_older);
        assert!(!bwd.has_newer);

        // 6. Simulated network outage + stale threshold 0 & cooldown 0 -> graceful OfflineCache degradation!
        provider.set_offline(true);
        let service_degraded = MarketDataService::new(
            Arc::clone(&provider),
            store.clone(),
            CachePolicy {
                stale_threshold_ms: Some(0),
                resync_cooldown_ms: 0,
                ..CachePolicy::default()
            },
        );
        let degraded = service_degraded
            .get_bars(&BarsRequest::init(key.clone(), 5))
            .await
            .unwrap();
        assert_eq!(degraded.source, DataSourceOrigin::OfflineCache);
        assert_eq!(degraded.bars.len(), 5);

        // 7. Explicit offline mode never calls provider at all.
        let offline_svc = service.with_offline_mode(true);
        let calls_before = provider.calls();
        let off = offline_svc
            .get_bars(&BarsRequest::init(key.clone(), 5))
            .await
            .unwrap();
        assert_eq!(off.source, DataSourceOrigin::OfflineCache);
        assert_eq!(provider.calls(), calls_before);
    }

    #[tokio::test]
    async fn live_bar_subscription_archives_into_sqlite() {
        let provider = Arc::new(MockProvider::new(Vec::new()));
        let store = SqliteMarketStore::open_in_memory().unwrap();
        let svc = MarketDataService::new(provider, store.clone(), CachePolicy::default());
        let sym = SymbolKey::new("HOSE", "FPT").unwrap();
        let key = SeriesKey::new(&sym, Interval::M1, Adjustment::Splits);

        let (mut sub, archiver) = svc.subscribe_bars_with_archival(key.clone()).await.unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(2), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ev.bar.close, 52_300.0);

        // Give archiver task a moment to persist before clean shutdown.
        tokio::time::sleep(Duration::from_millis(50)).await;
        sub.unsubscribe().await;
        let _ = tokio::time::timeout(Duration::from_secs(2), archiver).await;

        let persisted = store.query_bars(&key, BarQueryMode::Init, 5).unwrap();
        assert_eq!(persisted.bars.len(), 1);
        assert_eq!(persisted.bars[0].close, 52_300.0);
    }

    #[tokio::test]
    async fn stale_while_revalidate_returns_instantly_and_refreshes_in_background() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let step = Interval::H1.duration_ms();
        let full_series = build_recent_bars(25, step, now_ms);
        let provider = Arc::new(MockProvider::new(full_series.clone()));
        // Simulate slow upstream (150ms) so any blocking network call would fail the instant check!
        provider.set_delay_ms(150);

        let store = SqliteMarketStore::open_in_memory().unwrap();
        let sym = SymbolKey::new("HOSE", "FPT").unwrap();
        let key = SeriesKey::new(&sym, Interval::H1, Adjustment::Splits);

        // Seed stale local history (missing newest 5 bars).
        store
            .upsert_bars(&key, &full_series[..20], "seed", Some(true))
            .unwrap();

        let svc =
            MarketDataService::new(Arc::clone(&provider), store.clone(), CachePolicy::default());

        // 1. First SWR call warms RAM from SQLite and returns IMMEDIATELY (< 50ms) without hitting network.
        let t0 = std::time::Instant::now();
        let swr1 = svc
            .get_bars_stale_while_revalidate(&BarsRequest::init(key.clone(), 10))
            .await
            .unwrap();
        assert!(
            t0.elapsed().as_millis() < 100,
            "SWR must return immediately without awaiting 150ms network"
        );
        assert!(!swr1.from_ram);
        assert!(swr1.needs_revalidation);
        assert_eq!(swr1.page.bars.len(), 10);
        assert_eq!(
            swr1.page.bars.last().unwrap().timestamp,
            full_series[19].timestamp
        );
        assert_eq!(provider.calls(), 0);

        // 2. Second SWR call hits RAM LRU directly!
        let swr2 = svc
            .get_bars_stale_while_revalidate(&BarsRequest::init(key.clone(), 10))
            .await
            .unwrap();
        assert!(swr2.from_ram);
        assert_eq!(provider.calls(), 0);
        assert!(svc.ram_cache().stats().hits >= 1);

        // 3. Concurrent background revalidations are deduplicated to 1 upstream call.
        let svc_a = svc.clone();
        let svc_b = svc.clone();
        let key_a = key.clone();
        let key_b = key.clone();
        let (res_a, res_b) = tokio::join!(
            svc_a.revalidate_series(&key_a, 10),
            svc_b.revalidate_series(&key_b, 10)
        );
        let delta_a = res_a.unwrap();
        let delta_b = res_b.unwrap();
        assert_eq!(provider.calls(), 1);
        let non_empty_delta = if delta_a.is_empty() { delta_b } else { delta_a };
        assert!(non_empty_delta.len() >= 5);
        assert_eq!(
            non_empty_delta.last().unwrap().timestamp,
            full_series[24].timestamp
        );

        // 4. Subsequent SWR call hits updated RAM LRU with fresh newest bar & no revalidation needed!
        let swr3 = svc
            .get_bars_stale_while_revalidate(&BarsRequest::init(key.clone(), 10))
            .await
            .unwrap();
        assert!(swr3.from_ram);
        assert!(!swr3.needs_revalidation);
        assert_eq!(
            swr3.page.bars.last().unwrap().timestamp,
            full_series[24].timestamp
        );
    }
}
