use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::storage::{CacheBarsResult, SeriesMeta};
use crate::types::{BarQueryMode, OhlcvBar, ResolvedSymbol, SeriesKey, SymbolKey, normalize_bars};

/// Default maximum number of hot `(exchange, symbol, interval, adjustment)` series kept in RAM.
pub const DEFAULT_RAM_SERIES_CAPACITY: usize = 128;

/// Default maximum bars retained per series in RAM (sufficient for instant init + scrollback).
pub const DEFAULT_RAM_MAX_BARS_PER_SERIES: usize = 4_000;

/// Default maximum number of resolved symbol metadata entries kept in RAM.
pub const DEFAULT_RAM_SYMBOL_CAPACITY: usize = 512;

#[derive(Debug, Clone)]
struct CachedSeriesEntry {
    bars: Arc<Vec<OhlcvBar>>,
    has_older_on_disk: bool,
    meta: SeriesMeta,
    last_touch: u64,
}

#[derive(Debug, Default)]
struct SeriesCacheInner {
    entries: HashMap<SeriesKey, CachedSeriesEntry>,
    clock: u64,
    hits: u64,
    misses: u64,
}

#[derive(Debug, Default)]
struct SymbolCacheInner {
    entries: HashMap<SymbolKey, (ResolvedSymbol, i64, u64)>,
    clock: u64,
}

/// Runtime observability snapshot for [`SeriesRamCache`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamCacheStats {
    pub cached_series: usize,
    pub cached_symbols: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
}

/// Thread-safe, bounded in-memory LRU cache for hot OHLCV bar series and resolved symbols.
///
/// Sits directly in front of [`crate::SqliteMarketStore`] so repeated symbol & interval switches
/// (`15m` / `1H` / `4H` / `1D`) return in `< 0.5ms` without touching disk or acquiring the SQLite mutex.
pub struct SeriesRamCache {
    series_capacity: usize,
    max_bars_per_series: usize,
    symbol_capacity: usize,
    series: RwLock<SeriesCacheInner>,
    symbols: RwLock<SymbolCacheInner>,
}

impl Default for SeriesRamCache {
    fn default() -> Self {
        Self::new(DEFAULT_RAM_SERIES_CAPACITY, DEFAULT_RAM_MAX_BARS_PER_SERIES)
    }
}

impl SeriesRamCache {
    #[must_use]
    pub fn new(series_capacity: usize, max_bars_per_series: usize) -> Self {
        Self {
            series_capacity: series_capacity.max(1),
            max_bars_per_series: max_bars_per_series.max(100),
            symbol_capacity: DEFAULT_RAM_SYMBOL_CAPACITY,
            series: RwLock::new(SeriesCacheInner::default()),
            symbols: RwLock::new(SymbolCacheInner::default()),
        }
    }

    /// Queries bars from the RAM LRU cache.
    ///
    /// Returns `Some((CacheBarsResult, SeriesMeta))` only when RAM can satisfy the exact
    /// requested page without omitting older/newer bars that exist on disk.
    pub fn query_bars(
        &self,
        key: &SeriesKey,
        mode: BarQueryMode,
        limit: usize,
    ) -> Option<(CacheBarsResult, SeriesMeta)> {
        let limit = limit.max(1);
        let mut guard = self.series.write().ok()?;
        guard.clock = guard.clock.saturating_add(1);
        let tick = guard.clock;

        let Some(entry) = guard.entries.get_mut(key) else {
            guard.misses = guard.misses.saturating_add(1);
            return None;
        };

        let bars = &entry.bars;
        let result = match mode {
            BarQueryMode::Init => {
                if bars.len() < limit && entry.has_older_on_disk {
                    None
                } else {
                    let start = bars.len().saturating_sub(limit);
                    let has_older = start > 0 || entry.has_older_on_disk;
                    Some(CacheBarsResult {
                        bars: bars[start..].to_vec(),
                        has_older,
                        has_newer: false,
                    })
                }
            }
            BarQueryMode::Forward { before_ms } => {
                let end = bars.partition_point(|b| b.timestamp < before_ms);
                if end < limit && entry.has_older_on_disk {
                    None
                } else {
                    let start = end.saturating_sub(limit);
                    let has_older = start > 0 || entry.has_older_on_disk;
                    let has_newer = end < bars.len();
                    Some(CacheBarsResult {
                        bars: bars[start..end].to_vec(),
                        has_older,
                        has_newer,
                    })
                }
            }
            BarQueryMode::Backward { after_ms } => {
                let start = bars.partition_point(|b| b.timestamp <= after_ms);
                let end = (start + limit).min(bars.len());
                let has_newer = end < bars.len();
                let has_older = start > 0 || entry.has_older_on_disk;
                Some(CacheBarsResult {
                    bars: bars[start..end].to_vec(),
                    has_older,
                    has_newer,
                })
            }
        };

        match result {
            Some(page) => {
                entry.last_touch = tick;
                let meta = entry.meta.clone();
                guard.hits = guard.hits.saturating_add(1);
                Some((page, meta))
            }
            None => {
                guard.misses = guard.misses.saturating_add(1);
                None
            }
        }
    }

    /// Inserts or replaces a cached series window (retaining at most `max_bars_per_series` newest bars).
    pub fn put_series(
        &self,
        key: SeriesKey,
        mut bars: Vec<OhlcvBar>,
        mut has_older_on_disk: bool,
        meta: SeriesMeta,
    ) {
        if bars.is_empty() {
            return;
        }
        if bars.len() > self.max_bars_per_series {
            let drain_count = bars.len() - self.max_bars_per_series;
            bars.drain(..drain_count);
            has_older_on_disk = true;
        }
        let Ok(mut guard) = self.series.write() else {
            return;
        };
        guard.clock = guard.clock.saturating_add(1);
        let tick = guard.clock;

        if !guard.entries.contains_key(&key)
            && guard.entries.len() >= self.series_capacity
            && let Some(lru_key) = guard
                .entries
                .iter()
                .min_by_key(|(_, v)| v.last_touch)
                .map(|(k, _)| k.clone())
        {
            guard.entries.remove(&lru_key);
        }

        guard.entries.insert(
            key,
            CachedSeriesEntry {
                bars: Arc::new(bars),
                has_older_on_disk,
                meta,
                last_touch: tick,
            },
        );
    }

    /// Merges newly fetched or live-streamed bars into an existing RAM entry in-place.
    ///
    /// If the series is not currently resident in RAM, this is a no-op (SQLite remains canonical).
    pub fn merge_bars(
        &self,
        key: &SeriesKey,
        incoming: &[OhlcvBar],
        updated_meta: Option<SeriesMeta>,
    ) {
        if incoming.is_empty() {
            if let Some(meta) = updated_meta
                && let Ok(mut guard) = self.series.write()
                && let Some(entry) = guard.entries.get_mut(key)
            {
                entry.meta = meta;
            }
            return;
        }

        let Ok(mut guard) = self.series.write() else {
            return;
        };
        guard.clock = guard.clock.saturating_add(1);
        let tick = guard.clock;

        let Some(entry) = guard.entries.get_mut(key) else {
            return;
        };

        // Fast path: single live bar update or append at the tail.
        if incoming.len() == 1
            && let Some(last) = entry.bars.last()
            && incoming[0].timestamp >= last.timestamp
        {
            let mut next = (*entry.bars).clone();
            let bar = incoming[0];
            if bar.timestamp == last.timestamp {
                if let Some(tail) = next.last_mut() {
                    *tail = bar;
                }
            } else {
                next.push(bar);
                if next.len() > self.max_bars_per_series {
                    next.remove(0);
                    entry.has_older_on_disk = true;
                }
                entry.meta.bar_count = entry.meta.bar_count.saturating_add(1);
            }
            entry.meta.latest_ms = entry.meta.latest_ms.max(bar.timestamp);
            if let Some(meta) = updated_meta {
                entry.meta = meta;
            }
            entry.bars = Arc::new(next);
            entry.last_touch = tick;
            return;
        }

        let mut merged = Vec::with_capacity(entry.bars.len() + incoming.len());
        merged.extend_from_slice(&entry.bars);
        merged.extend_from_slice(incoming);
        let mut normalized = normalize_bars(merged);
        if normalized.len() > self.max_bars_per_series {
            let drain_count = normalized.len() - self.max_bars_per_series;
            normalized.drain(..drain_count);
            entry.has_older_on_disk = true;
        }
        if let Some(meta) = updated_meta {
            entry.meta = meta;
        } else if let (Some(first), Some(last)) = (normalized.first(), normalized.last()) {
            entry.meta.earliest_ms = entry.meta.earliest_ms.min(first.timestamp);
            entry.meta.latest_ms = entry.meta.latest_ms.max(last.timestamp);
            entry.meta.bar_count = entry.meta.bar_count.max(normalized.len());
        }
        entry.bars = Arc::new(normalized);
        entry.last_touch = tick;
    }

    /// Removes a specific series from RAM (e.g. when invalidated explicitly).
    pub fn invalidate_series(&self, key: &SeriesKey) {
        if let Ok(mut guard) = self.series.write() {
            guard.entries.remove(key);
        }
    }

    /// Clears all cached series and symbols (e.g. after bulk CafeF import).
    pub fn clear(&self) {
        if let Ok(mut guard) = self.series.write() {
            guard.entries.clear();
        }
        if let Ok(mut guard) = self.symbols.write() {
            guard.entries.clear();
        }
    }

    /// Looks up a cached [`ResolvedSymbol`] and its `updated_at_ms` in RAM.
    pub fn get_symbol(&self, key: &SymbolKey) -> Option<(ResolvedSymbol, i64)> {
        let mut guard = self.symbols.write().ok()?;
        guard.clock = guard.clock.saturating_add(1);
        let tick = guard.clock;
        let entry = guard.entries.get_mut(key)?;
        entry.2 = tick;
        Some((entry.0.clone(), entry.1))
    }

    /// Stores a [`ResolvedSymbol`] in RAM LRU cache.
    pub fn put_symbol(&self, key: SymbolKey, symbol: ResolvedSymbol, updated_at_ms: i64) {
        let Ok(mut guard) = self.symbols.write() else {
            return;
        };
        guard.clock = guard.clock.saturating_add(1);
        let tick = guard.clock;
        if !guard.entries.contains_key(&key)
            && guard.entries.len() >= self.symbol_capacity
            && let Some(lru_key) = guard
                .entries
                .iter()
                .min_by_key(|(_, (_, _, touch))| *touch)
                .map(|(k, _)| k.clone())
        {
            guard.entries.remove(&lru_key);
        }
        guard.entries.insert(key, (symbol, updated_at_ms, tick));
    }

    /// Returns diagnostic hit/miss and occupancy statistics.
    #[must_use]
    pub fn stats(&self) -> RamCacheStats {
        let (cached_series, hits, misses) = self
            .series
            .read()
            .map(|g| (g.entries.len(), g.hits, g.misses))
            .unwrap_or((0, 0, 0));
        let cached_symbols = self.symbols.read().map(|g| g.entries.len()).unwrap_or(0);
        RamCacheStats {
            cached_series,
            cached_symbols,
            capacity: self.series_capacity,
            hits,
            misses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Adjustment, Interval};

    fn make_bars(count: usize, start_ms: i64, step_ms: i64) -> Vec<OhlcvBar> {
        (0..count)
            .map(|i| {
                let ts = start_ms + (i as i64) * step_ms;
                let base = 100_000.0 + (i as f64) * 100.0;
                OhlcvBar::new(ts, base, base + 500.0, base - 500.0, base + 200.0, 10_000.0)
            })
            .collect()
    }

    fn make_meta(key: &SeriesKey, bars: &[OhlcvBar]) -> SeriesMeta {
        SeriesMeta {
            exchange: key.exchange.clone(),
            symbol: key.symbol.clone(),
            interval: key.interval.as_str().to_owned(),
            adjustment: key.adjustment.as_str().to_owned(),
            earliest_ms: bars.first().map_or(0, |b| b.timestamp),
            latest_ms: bars.last().map_or(0, |b| b.timestamp),
            bar_count: bars.len(),
            reached_start: true,
            source: "test".to_owned(),
            updated_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn ram_lru_cache_evicts_least_recently_used_series() {
        let cache = SeriesRamCache::new(2, 500);
        let k1 = SeriesKey::new(
            &SymbolKey::new("HOSE", "FPT").unwrap(),
            Interval::D1,
            Adjustment::Splits,
        );
        let k2 = SeriesKey::new(
            &SymbolKey::new("HOSE", "VCB").unwrap(),
            Interval::D1,
            Adjustment::Splits,
        );
        let k3 = SeriesKey::new(
            &SymbolKey::new("HOSE", "HPG").unwrap(),
            Interval::D1,
            Adjustment::Splits,
        );

        let bars = make_bars(20, 1_700_000_000_000, 86_400_000);
        cache.put_series(k1.clone(), bars.clone(), false, make_meta(&k1, &bars));
        cache.put_series(k2.clone(), bars.clone(), false, make_meta(&k2, &bars));

        // Touch k1 so k2 becomes the least recently used entry.
        let hit = cache.query_bars(&k1, BarQueryMode::Init, 10);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().0.bars.len(), 10);

        // Insert k3 -> should evict k2 while preserving k1.
        cache.put_series(k3.clone(), bars.clone(), false, make_meta(&k3, &bars));
        assert!(cache.query_bars(&k1, BarQueryMode::Init, 10).is_some());
        assert!(cache.query_bars(&k2, BarQueryMode::Init, 10).is_none());
        assert!(cache.query_bars(&k3, BarQueryMode::Init, 10).is_some());
        assert_eq!(cache.stats().cached_series, 2);
    }

    #[test]
    fn ram_cache_merges_live_updates_and_appends_monotonically() {
        let cache = SeriesRamCache::new(4, 200);
        let k = SeriesKey::new(
            &SymbolKey::new("HOSE", "FPT").unwrap(),
            Interval::H1,
            Adjustment::Splits,
        );
        let bars = make_bars(10, 1_700_000_000_000, 3_600_000);
        cache.put_series(k.clone(), bars.clone(), false, make_meta(&k, &bars));

        // Update latest bar close
        let mut updated_last = *bars.last().unwrap();
        updated_last.close += 1_000.0;
        updated_last.high = updated_last.high.max(updated_last.close);
        cache.merge_bars(&k, &[updated_last], None);

        let (page, _) = cache.query_bars(&k, BarQueryMode::Init, 10).unwrap();
        assert_eq!(page.bars.len(), 10);
        assert_eq!(page.bars.last().unwrap().close, updated_last.close);

        // Append brand-new bar
        let next_bar = OhlcvBar::new(
            updated_last.timestamp + 3_600_000,
            105_000.0,
            106_000.0,
            104_000.0,
            105_500.0,
            25_000.0,
        );
        cache.merge_bars(&k, &[next_bar], None);
        let (page2, _) = cache.query_bars(&k, BarQueryMode::Init, 20).unwrap();
        assert_eq!(page2.bars.len(), 11);
        assert_eq!(page2.bars.last().unwrap().timestamp, next_bar.timestamp);
    }
}
