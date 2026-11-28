use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{Datelike, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{OnceCell, broadcast, mpsc, oneshot};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use tradingview::chart::{DataPoint, OHLCV};
use tradingview::client::misc::{advanced_search_symbol, list_symbols};
use tradingview::fundamental::{
    FundamentalRegistry, fetch_fundamental_registry, get_fundamental_data,
};
use tradingview::historical::{HistoricalClient, HistoricalRequest, PaginationStop};
use tradingview::live::handler::Handler;
use tradingview::live::models::{DataServer, TradingViewDataEvent};
use tradingview::live::websocket::WebSocketClient;
use tradingview::models::{FinancialPeriod, MarketType, StocksType};
use tradingview::quote::models::QuoteData;
use tradingview::utils::gen_session_id;
use url::Url;

use crate::error::{DataError, Result};
use crate::provider::{BarSubscription, MarketDataProvider, QuoteSubscription};
use crate::rate_limit::{RateLimitConfig, TokenBucketRateLimiter};
use crate::storage::QuarterlyFundamentalRow;
use crate::types::{
    Adjustment, BatchBarsResult, HistoryFetchOutput, Interval, LiveBarEvent, LiveQuote, OhlcvBar,
    ResolvedSymbol, SeriesKey, SymbolKey, SymbolSearchResult, canonicalize_bar_timestamp_ms,
    normalize_bars, normalize_exchange_alias,
};

const BROADCAST_CAPACITY: usize = 1024;
const INTERNAL_MPSC_CAPACITY: usize = 2048;
const SYMBOL_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

/// Configuration for [`TradingViewFeed`].
#[derive(Debug, Clone)]
pub struct TradingViewConfig {
    /// TradingView WebSocket auth token (defaults to anonymous `"unauthorized_user_token"`).
    pub auth_token: String,
    /// Primary TradingView data server (`ProData` provides ~10k–16.5k bar depth).
    pub primary_server: DataServer,
    /// Automatic fallback server when the primary server fails transport/handshake.
    pub fallback_server: Option<DataServer>,
    /// Per-request / per-page network timeout.
    pub request_timeout: Duration,
    /// Delay between paginated `request_more_data` frames.
    pub page_delay: Duration,
    /// Rate-limit & concurrency settings protecting against IP throttling.
    pub rate_limit: RateLimitConfig,
    /// Initial exponential backoff delay when reconnecting a dropped live stream.
    pub reconnect_base_delay: Duration,
    /// Maximum exponential backoff cap for live stream reconnection.
    pub reconnect_max_delay: Duration,
    /// Optional custom WebSocket endpoint (for local mock servers or proxies).
    pub endpoint_override: Option<String>,
}

impl Default for TradingViewConfig {
    fn default() -> Self {
        Self {
            auth_token: "unauthorized_user_token".to_owned(),
            primary_server: DataServer::ProData,
            fallback_server: Some(DataServer::Data),
            request_timeout: Duration::from_secs(25),
            page_delay: Duration::from_millis(500),
            rate_limit: RateLimitConfig::tradingview_default(),
            reconnect_base_delay: Duration::from_secs(1),
            reconnect_max_delay: Duration::from_secs(30),
            endpoint_override: None,
        }
    }
}

/// Pure-Rust (`rustls-tls`, zero `wreq`/BoringSSL/CMake/Perl) TradingView market data provider.
#[derive(Clone)]
pub struct TradingViewFeed {
    config: Arc<TradingViewConfig>,
    rate_limiter: TokenBucketRateLimiter,
    symbol_cache: Arc<RwLock<HashMap<SymbolKey, (ResolvedSymbol, Instant)>>>,
    fundamental_registry: Arc<OnceCell<Arc<FundamentalRegistry>>>,
}

impl TradingViewFeed {
    /// Creates a new [`TradingViewFeed`] with default `ProData` -> `Data` fallback and rate limiting.
    #[must_use]
    pub fn new(config: TradingViewConfig) -> Self {
        let rate_limiter = TokenBucketRateLimiter::new(config.rate_limit);
        Self {
            config: Arc::new(config),
            rate_limiter,
            symbol_cache: Arc::new(RwLock::new(HashMap::new())),
            fundamental_registry: Arc::new(OnceCell::new()),
        }
    }

    async fn get_or_fetch_fundamental_registry(&self) -> Result<Arc<FundamentalRegistry>> {
        let res = self
            .fundamental_registry
            .get_or_try_init(|| async {
                let reg = timeout(Duration::from_secs(15), fetch_fundamental_registry())
                    .await
                    .map_err(|_| DataError::Timeout("tradingview fundamental registry".to_owned()))?
                    .map_err(|e| DataError::Provider(format!("fundamental registry: {e}")))?;
                Ok::<Arc<FundamentalRegistry>, DataError>(Arc::new(reg))
            })
            .await?;
        Ok(Arc::clone(res))
    }

    /// Fetches historical quarterly fundamental series (`EPS TTM`, `BVPS`, `ROE`, `P/E`, `P/B`,
    /// `Net Income YoY`) via `tradingview-rs::fundamental` (`STD;Fund_*` Pine studies, 100% guest-accessible)
    /// as the Point-in-Time fallback for symbols outside Yumeko's 54 `CSTC` stocks (`W6-06`).
    pub async fn fetch_quarterly_fundamentals(
        &self,
        key: &SymbolKey,
    ) -> Result<Vec<QuarterlyFundamentalRow>> {
        let registry = self.get_or_fetch_fundamental_registry().await?;
        let wire_symbol = map_to_tv_wire_symbol(key);
        let tv_exchange = wire_symbol.exchange.as_str();
        let tv_symbol = wire_symbol.symbol.as_str();
        let auth = Some(self.config.auth_token.as_str());
        let server = self.config.primary_server;

        let metric_specs: &[(&str, &[(&str, FinancialPeriod)])] = &[
            (
                "eps_ttm",
                &[
                    ("earnings_per_share_diluted", FinancialPeriod::TrailingTwelveMonths),
                    ("earnings_per_share_basic", FinancialPeriod::TrailingTwelveMonths),
                    ("earnings_per_share_diluted", FinancialPeriod::FiscalQuarter),
                ],
            ),
            (
                "bvps",
                &[
                    ("book_value_per_share", FinancialPeriod::FiscalQuarter),
                    ("book_value_per_share", FinancialPeriod::FiscalYear),
                ],
            ),
            (
                "roe_pct",
                &[
                    ("return_on_equity", FinancialPeriod::FiscalQuarter),
                    ("return_on_equity", FinancialPeriod::TrailingTwelveMonths),
                ],
            ),
            (
                "pe",
                &[
                    ("price_earnings", FinancialPeriod::TrailingTwelveMonths),
                    ("price_earnings", FinancialPeriod::FiscalQuarter),
                ],
            ),
            (
                "pb",
                &[("price_book", FinancialPeriod::FiscalQuarter)],
            ),
            (
                "net_income",
                &[("net_income", FinancialPeriod::FiscalQuarter)],
            ),
        ];

        // Quarter bucket keyed by `(year, quarter)` -> `(period_end_ms, metric map)`
        type QuarterBucket = (i64, HashMap<&'static str, f64>);
        let mut buckets: std::collections::BTreeMap<(i32, u32), QuarterBucket> =
            std::collections::BTreeMap::new();

        for &(slot, candidates) in metric_specs {
            let mut fetched_pts: Option<Vec<DataPoint>> = None;
            for &(fund_id, ref period) in candidates {
                if registry.lookup(fund_id, Some(period)).is_none() {
                    continue;
                }
                let _permit = self.rate_limiter.acquire().await;
                let fut = get_fundamental_data(
                    &registry,
                    fund_id,
                    Some(period),
                    tv_symbol,
                    tv_exchange,
                    tradingview::models::Interval::OneDay,
                    1_200,
                    auth,
                    server,
                );
                if let Ok(Ok(study)) = timeout(Duration::from_secs(12), fut).await
                    && !study.data.is_empty()
                {
                    fetched_pts = Some(study.data);
                    break;
                }
            }
            let Some(pts) = fetched_pts else {
                continue;
            };
            for dp in pts {
                if dp.value.len() < 2 {
                    continue;
                }
                let ts_sec = dp.value[0] as i64;
                let val = dp.value[1];
                if ts_sec <= 0 || !val.is_finite() || val.abs() >= 1e50 {
                    continue;
                }
                let Some(dt) = Utc.timestamp_opt(ts_sec, 0).single() else {
                    continue;
                };
                let q = ((dt.month() - 1) / 3) + 1;
                let ts_ms = ts_sec.saturating_mul(1_000);
                let entry = buckets
                    .entry((dt.year(), q))
                    .or_insert_with(|| (ts_ms, HashMap::new()));
                if ts_ms >= entry.0 {
                    entry.0 = ts_ms;
                }
                entry.1.insert(slot, val);
            }
        }

        let now_ms = Utc::now().timestamp_millis();
        let sym_canonical = key.ticker();
        let mut rows = Vec::with_capacity(buckets.len());
        for (&(year, q), (period_end_ms, map)) in &buckets {
            let net_income_yoy_pct = map.get("net_income").and_then(|&cur_ni| {
                let prev_ni = buckets
                    .get(&(year - 1, q))
                    .and_then(|(_, pm)| pm.get("net_income"))
                    .copied()?;
                if prev_ni.abs() < 1e-6 {
                    return None;
                }
                let pct = ((cur_ni - prev_ni) / prev_ni.abs()) * 100.0;
                pct.is_finite().then_some(pct)
            });
            rows.push(QuarterlyFundamentalRow {
                symbol: sym_canonical.clone(),
                period_end_ms: *period_end_ms,
                period_label: format!("{year}-Q{q}"),
                eps_ttm: map.get("eps_ttm").copied(),
                bvps: map.get("bvps").copied(),
                pe: map.get("pe").copied(),
                pb: map.get("pb").copied(),
                roe_pct: map.get("roe_pct").copied(),
                net_income_yoy_pct,
                source: "tradingview_fundamental".to_owned(),
                updated_at_ms: now_ms,
            });
        }
        rows.sort_by_key(|r| std::cmp::Reverse(r.period_end_ms));
        Ok(rows)
    }

    /// Returns a reference to the shared rate limiter for observability and testing.
    #[must_use]
    pub fn rate_limiter(&self) -> &TokenBucketRateLimiter {
        &self.rate_limiter
    }

    /// Fetches the complete Vietnam listed stock/index/futures catalog (`HOSE`, `HNX`, `UPCOM`,
    /// ~1,600+ tickers) using `tradingview-rs::list_symbols` so Clean-Slate installations and
    /// Stock Screener cover the entire market without requiring manual CafeF CSV imports.
    pub async fn list_vn_universe_symbols(&self) -> Result<Vec<ResolvedSymbol>> {
        let mut seen = std::collections::HashSet::<String>::new();
        let mut out = Vec::new();

        for (tv_exchange, market_type) in [
            ("HOSE", MarketType::Stocks(StocksType::All)),
            ("HNX", MarketType::Stocks(StocksType::All)),
            ("UPCOM", MarketType::Stocks(StocksType::All)),
            ("HOSE", MarketType::Indices),
            ("HNX", MarketType::Indices),
            ("HNX", MarketType::Futures),
        ] {
            let _permit = self.rate_limiter.acquire().await;
            let req = list_symbols()
                .exchange(tv_exchange)
                .market_type(market_type)
                .call();
            let raw_list = match timeout(self.config.request_timeout, req).await {
                Ok(Ok(items)) => items,
                Ok(Err(err)) => {
                    warn!(exchange = tv_exchange, error = %err, "list_symbols failed for exchange");
                    continue;
                }
                Err(_) => {
                    warn!(exchange = tv_exchange, "list_symbols timed out for exchange");
                    continue;
                }
            };

            for item in raw_list {
                let ex_norm = normalize_exchange_alias(if item.exchange.is_empty() {
                    tv_exchange
                } else {
                    item.exchange.as_str()
                });
                let sym_upper = item.symbol.trim().to_ascii_uppercase();
                if sym_upper.is_empty() {
                    continue;
                }
                let canonical_key = map_from_tv_wire_symbol(&SymbolKey {
                    exchange: ex_norm.clone(),
                    symbol: sym_upper,
                });
                let ticker = canonical_key.ticker();
                if !seen.insert(ticker.clone()) {
                    continue;
                }
                let is_index = matches!(market_type, MarketType::Indices)
                    || item.market_type.eq_ignore_ascii_case("index");
                let mut resolved = ResolvedSymbol::from_cafef_vn_ticker(
                    &canonical_key.exchange,
                    &canonical_key.symbol,
                    is_index,
                );
                if !item.description.trim().is_empty() {
                    resolved.description = item.description.trim().to_owned();
                }
                if !item.market_type.trim().is_empty() {
                    resolved.market_type = item.market_type.trim().to_ascii_lowercase();
                }
                if !item.currency_code.trim().is_empty() {
                    resolved.currency = item.currency_code.trim().to_owned();
                }
                out.push(resolved);
            }
        }

        Ok(out)
    }

    fn server_candidates(&self) -> Vec<DataServer> {
        let mut servers = vec![self.config.primary_server];
        if let Some(fb) = self.config.fallback_server
            && fb != self.config.primary_server
        {
            servers.push(fb);
        }
        servers
    }

    fn parsed_endpoint(&self) -> Result<Option<Url>> {
        match &self.config.endpoint_override {
            Some(raw) => {
                let parsed = Url::parse(raw).map_err(|e| {
                    DataError::Provider(format!("invalid endpoint URL '{raw}': {e}"))
                })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    fn build_historical_client(&self, server: DataServer) -> Result<HistoricalClient> {
        let mut client = HistoricalClient::new(&self.config.auth_token, server);
        if let Some(ep) = &self.config.endpoint_override {
            client = client
                .with_endpoint(ep)
                .map_err(|e| DataError::Provider(format!("invalid endpoint '{ep}': {e}")))?;
        }
        Ok(client)
    }

    async fn resolve_symbol_on_server(
        &self,
        symbol: &SymbolKey,
        server: DataServer,
    ) -> Result<ResolvedSymbol> {
        let endpoint = self.parsed_endpoint()?;
        let (tx, rx) = oneshot::channel::<Result<tradingview::SymbolInfo>>();
        let sender = Arc::new(Mutex::new(Some(tx)));
        let wire_symbol = map_to_tv_wire_symbol(symbol);
        let instrument = wire_symbol.ticker();
        let handler = ResolveSymbolHandler {
            sender: Arc::clone(&sender),
            ticker: symbol.ticker(),
        };

        let ws = WebSocketClient::builder()
            .auth_token(self.config.auth_token.as_str())
            .server(server)
            .handler(handler)
            .maybe_endpoint(endpoint)
            .build()
            .await
            .map_err(map_tv_error)?;

        let chart_session = gen_session_id("cs");
        let sym_series_id = format!("sds_sym_{}", tradingview::utils::gen_id());

        let reader_ws = Arc::clone(&ws);
        let reader_sender = Arc::clone(&sender);
        let instrument_for_err = instrument.clone();
        let reader_handle = tokio::spawn(async move {
            let _ = reader_ws.subscribe().await;
            if let Ok(mut guard) = reader_sender.lock()
                && let Some(tx) = guard.take()
            {
                let _ = tx.send(Err(DataError::Provider(format!(
                    "connection closed while resolving {instrument_for_err}"
                ))));
            }
        });

        let setup_res = async {
            ws.create_chart_session(&chart_session)
                .await
                .map_err(map_tv_error)?;
            ws.resolve_symbol()
                .session(&chart_session)
                .symbol_series_id(&sym_series_id)
                .instrument(&instrument)
                .call()
                .await
                .map_err(map_tv_error)?;
            Ok::<(), DataError>(())
        }
        .await;

        let resolved_res = match setup_res {
            Ok(()) => match timeout(self.config.request_timeout, rx).await {
                Ok(Ok(Ok(tv_info))) => Ok(ResolvedSymbol::from_tv_symbol_info(symbol, &tv_info)),
                Ok(Ok(Err(err))) => Err(err),
                Ok(Err(_)) => Err(DataError::Provider(format!(
                    "connection closed while resolving {instrument}"
                ))),
                Err(_) => Err(DataError::Timeout(format!(
                    "timed out after {:?} resolving {instrument}",
                    self.config.request_timeout
                ))),
            },
            Err(err) => Err(err),
        };

        let _ = ws.delete_chart_session(&chart_session).await;
        let _ = ws.close().await;
        reader_handle.abort();

        resolved_res
    }

    async fn fetch_history_on_server(
        &self,
        series: &SeriesKey,
        num_bars: usize,
        from_ms: Option<i64>,
        server: DataServer,
    ) -> Result<HistoryFetchOutput> {
        let client = self.build_historical_client(server)?;
        let requested_bars = num_bars.max(1) as u64;
        let wire_sym = map_to_tv_wire_symbol(&series.symbol_key());

        let request = if let Some(from_ms_val) = from_ms {
            let now_secs = chrono::Utc::now().timestamp();
            let from_secs = from_ms_val
                .div_euclid(1_000)
                .clamp(0, now_secs.saturating_sub(1));
            HistoricalRequest::builder()
                .symbol(wire_sym.symbol.clone())
                .exchange(wire_sym.exchange.clone())
                .interval(series.interval.to_tv_interval())
                .adjustment(series.adjustment.to_tv_adjustment())
                .from_timestamp(from_secs)
                .max_bars(requested_bars)
                .page_delay(self.config.page_delay)
                .timeout(self.config.request_timeout)
                .allow_partial(true)
                .build()
        } else if requested_bars > 5_000 {
            HistoricalRequest::builder()
                .symbol(wire_sym.symbol.clone())
                .exchange(wire_sym.exchange.clone())
                .interval(series.interval.to_tv_interval())
                .adjustment(series.adjustment.to_tv_adjustment())
                .max_bars(requested_bars)
                .page_delay(self.config.page_delay)
                .timeout(self.config.request_timeout)
                .allow_partial(true)
                .build()
        } else {
            HistoricalRequest::builder()
                .symbol(wire_sym.symbol.clone())
                .exchange(wire_sym.exchange.clone())
                .interval(series.interval.to_tv_interval())
                .adjustment(series.adjustment.to_tv_adjustment())
                .num_bars(requested_bars)
                .timeout(self.config.request_timeout)
                .build()
        };

        let hist = client.retrieve(request).await.map_err(map_tv_error)?;
        let sym_key = series.symbol_key();
        let resolved = ResolvedSymbol::from_tv_symbol_info(&sym_key, &hist.symbol_info);
        if let Ok(mut cache) = self.symbol_cache.write() {
            cache.insert(sym_key, (resolved.clone(), Instant::now()));
        }

        let raw_bars: Vec<OhlcvBar> = hist
            .data
            .iter()
            .filter_map(|dp| datapoint_to_ohlcv_bar(dp, series.interval))
            .collect();
        let bars = normalize_bars(raw_bars);
        let reached_start = matches!(
            hist.pagination_stop,
            Some(
                PaginationStop::EndOfHistory
                    | PaginationStop::BarLimit
                    | PaginationStop::NoMoreData
            )
        );

        Ok(HistoryFetchOutput {
            bars,
            symbol_info: Some(resolved),
            reached_start_of_history: reached_start,
            pages_fetched: hist.pages_fetched,
        })
    }
}

impl Default for TradingViewFeed {
    fn default() -> Self {
        Self::new(TradingViewConfig::default())
    }
}

#[async_trait]
impl MarketDataProvider for TradingViewFeed {
    async fn search_symbols(
        &self,
        query: &str,
        exchange: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SymbolSearchResult>> {
        let _permit = self.rate_limiter.acquire().await?;
        let trimmed_query = query.trim();
        let normalized_exchange = exchange
            .map(normalize_exchange_alias)
            .filter(|s| !s.is_empty());

        let resp = advanced_search_symbol()
            .search(trimmed_query)
            .maybe_exchange(normalized_exchange.as_deref())
            .call()
            .await
            .map_err(map_tv_error)?;

        let max_items = limit.max(1);
        let mut results: Vec<SymbolSearchResult> = resp
            .symbols
            .into_iter()
            .map(|sym| {
                let raw_prefix = if !sym.prefix.trim().is_empty() {
                    sym.prefix.trim()
                } else {
                    sym.exchange.trim()
                };
                let canonical_ex = normalize_exchange_alias(raw_prefix);
                let canonical_sym = sym.symbol.trim().to_ascii_uppercase();
                let ticker = if canonical_ex.is_empty() {
                    canonical_sym.clone()
                } else {
                    format!("{canonical_ex}:{canonical_sym}")
                };
                SymbolSearchResult {
                    ticker,
                    exchange: canonical_ex,
                    prefix: sym.prefix,
                    symbol: canonical_sym,
                    description: sym.description,
                    market_type: sym.market_type,
                    currency_code: sym.currency_code,
                    country_code: sym.country_code,
                    provider_id: sym.data_provider,
                    type_specs: sym.type_specs,
                }
            })
            .filter(|hit| match &normalized_exchange {
                Some(ex_filter) => {
                    hit.exchange.eq_ignore_ascii_case(ex_filter)
                        || hit.prefix.eq_ignore_ascii_case(ex_filter)
                }
                None => true,
            })
            .take(max_items)
            .collect();

        // Deduplicate by canonical ticker preserving server relevance ordering.
        let mut seen = std::collections::HashSet::with_capacity(results.len());
        results.retain(|item| seen.insert(item.ticker.clone()));
        Ok(results)
    }

    async fn resolve_symbol(&self, symbol: &SymbolKey) -> Result<ResolvedSymbol> {
        if let Ok(cache) = self.symbol_cache.read()
            && let Some((cached, ts)) = cache.get(symbol)
            && ts.elapsed() < SYMBOL_CACHE_TTL
        {
            return Ok(cached.clone());
        }

        let _permit = self.rate_limiter.acquire().await?;
        let servers = self.server_candidates();
        let mut last_err: Option<DataError> = None;

        for (idx, server) in servers.iter().copied().enumerate() {
            match self.resolve_symbol_on_server(symbol, server).await {
                Ok(resolved) => {
                    if let Ok(mut cache) = self.symbol_cache.write() {
                        cache.insert(symbol.clone(), (resolved.clone(), Instant::now()));
                    }
                    return Ok(resolved);
                }
                Err(err) => {
                    let can_fallback = idx + 1 < servers.len() && err.is_fallback_eligible();
                    if can_fallback {
                        warn!(
                            ticker = %symbol,
                            primary = ?server,
                            error = %err,
                            "resolve_symbol failed on primary server; falling back"
                        );
                        last_err = Some(err);
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        Err(last_err
            .unwrap_or_else(|| DataError::Provider(format!("unable to resolve symbol {symbol}"))))
    }

    async fn fetch_history(
        &self,
        series: &SeriesKey,
        num_bars: usize,
        from_ms: Option<i64>,
    ) -> Result<HistoryFetchOutput> {
        let _permit = self.rate_limiter.acquire().await?;
        let servers = self.server_candidates();
        let mut last_err: Option<DataError> = None;

        for (idx, server) in servers.iter().copied().enumerate() {
            match self
                .fetch_history_on_server(series, num_bars, from_ms, server)
                .await
            {
                Ok(output) => return Ok(output),
                Err(err) => {
                    let can_fallback = idx + 1 < servers.len() && err.is_fallback_eligible();
                    if can_fallback {
                        warn!(
                            series = %series,
                            primary = ?server,
                            error = %err,
                            "fetch_history failed on primary server; falling back"
                        );
                        last_err = Some(err);
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            DataError::Provider(format!("unable to fetch history for {series}"))
        }))
    }

    async fn fetch_history_batch(
        &self,
        symbols: &[SymbolKey],
        interval: Interval,
        adjustment: Adjustment,
        num_bars: usize,
    ) -> BatchBarsResult {
        let started = Instant::now();
        let mut join_set = tokio::task::JoinSet::new();

        for sym in symbols.iter().cloned() {
            let feed = self.clone();
            join_set.spawn(async move {
                let series = SeriesKey::new(&sym, interval, adjustment);
                let res = feed.fetch_history(&series, num_bars, None).await;
                (sym, res)
            });
        }

        let mut successful = Vec::with_capacity(symbols.len());
        let mut failed = Vec::new();

        while let Some(joined) = join_set.join_next().await {
            match joined {
                Ok((sym, Ok(output))) => successful.push((sym, output.bars)),
                Ok((sym, Err(err))) => failed.push((sym, err.to_string())),
                Err(join_err) => {
                    warn!(error = %join_err, "batch history worker panicked or cancelled");
                }
            }
        }

        BatchBarsResult {
            successful,
            failed,
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    }

    async fn subscribe_bars(&self, series: SeriesKey) -> Result<BarSubscription> {
        let endpoint = self.parsed_endpoint()?;
        let (btx, _) = broadcast::channel::<LiveBarEvent>(BROADCAST_CAPACITY);
        let cancel = CancellationToken::new();

        let config = Arc::clone(&self.config);
        let servers = self.server_candidates();
        let series_clone = series.clone();
        let btx_clone = btx.clone();
        let cancel_clone = cancel.clone();

        let task = tokio::spawn(async move {
            run_bar_stream_supervisor(
                config,
                servers,
                endpoint,
                series_clone,
                btx_clone,
                cancel_clone,
            )
            .await;
        });

        Ok(BarSubscription::new(series, btx, cancel, task))
    }

    async fn subscribe_quotes(&self, symbols: Vec<SymbolKey>) -> Result<QuoteSubscription> {
        if symbols.is_empty() {
            return Err(DataError::InvalidSymbol(
                "subscribe_quotes requires at least one symbol".to_owned(),
            ));
        }
        let endpoint = self.parsed_endpoint()?;
        let (btx, _) = broadcast::channel::<LiveQuote>(BROADCAST_CAPACITY);
        let cancel = CancellationToken::new();

        let config = Arc::clone(&self.config);
        let servers = self.server_candidates();
        let symbols_clone = symbols.clone();
        let btx_clone = btx.clone();
        let cancel_clone = cancel.clone();

        let task = tokio::spawn(async move {
            run_quote_stream_supervisor(
                config,
                servers,
                endpoint,
                symbols_clone,
                btx_clone,
                cancel_clone,
            )
            .await;
        });

        Ok(QuoteSubscription::new(symbols, btx, cancel, task))
    }
}

// =============================================================================
// WebSocket Handlers & Supervised Reconnect Loops
// =============================================================================

struct ResolveSymbolHandler {
    sender: Arc<Mutex<Option<oneshot::Sender<Result<tradingview::SymbolInfo>>>>>,
    ticker: String,
}

impl ResolveSymbolHandler {
    fn complete(&self, res: Result<tradingview::SymbolInfo>) {
        if let Ok(mut guard) = self.sender.lock()
            && let Some(tx) = guard.take()
        {
            let _ = tx.send(res);
        }
    }
}

impl Handler for ResolveSymbolHandler {
    fn handle_events(&self, event: TradingViewDataEvent, message: &[Value]) {
        match event {
            TradingViewDataEvent::OnSymbolResolved => {
                if let Some(payload) = message.get(2) {
                    match tradingview::SymbolInfo::deserialize(payload) {
                        Ok(info) => self.complete(Ok(info)),
                        Err(e) => self.complete(Err(DataError::Provider(format!(
                            "failed to parse symbol_resolved for {}: {e}",
                            self.ticker
                        )))),
                    }
                }
            }
            TradingViewDataEvent::OnError(tv_err) => {
                let err = match tv_err {
                    tradingview::error::TradingViewError::SymbolError => {
                        DataError::SymbolNotFound(self.ticker.clone())
                    }
                    other => DataError::Provider(format!(
                        "TradingView error resolving {}: {other:?}",
                        self.ticker
                    )),
                };
                self.complete(Err(err));
            }
            _ => {}
        }
    }

    fn handle_quote_data(&self, _message: &[Value]) {}
    fn handle_series_data(&self, _event: TradingViewDataEvent, _messages: &[Value]) {}

    fn notify_error(&self, error: tradingview::Error, _message: &[Value]) {
        self.complete(Err(map_tv_error(error)));
    }
}

struct BarStreamHandler {
    series: SeriesKey,
    tx: mpsc::Sender<LiveBarEvent>,
}

impl Handler for BarStreamHandler {
    fn handle_events(&self, event: TradingViewDataEvent, message: &[Value]) {
        let is_update = match event {
            TradingViewDataEvent::OnChartData => false,
            TradingViewDataEvent::OnChartDataUpdate => true,
            TradingViewDataEvent::OnError(tv_err) => {
                warn!(
                    series = %self.series,
                    error = ?tv_err,
                    "TradingView protocol error on live bar stream"
                );
                return;
            }
            _ => return,
        };

        let Some(obj) = message.get(1).and_then(Value::as_object) else {
            return;
        };
        for series_val in obj.values() {
            let Some(rows) = series_val.get("s").and_then(Value::as_array) else {
                continue;
            };
            // On initial snapshot (`timescale_update`), push only the latest bar so subscribers
            // receive a clean single-bar current candle (aligned with KLineChart v10 `subscribeBar`).
            let slice: &[Value] = if !is_update && rows.len() > 1 {
                &rows[rows.len() - 1..]
            } else {
                rows.as_slice()
            };
            for row in slice {
                let Ok(dp) = DataPoint::deserialize(row) else {
                    continue;
                };
                if let Some(bar) = datapoint_to_ohlcv_bar(&dp, self.series.interval) {
                    let _ = self.tx.try_send(LiveBarEvent {
                        series: self.series.clone(),
                        bar,
                        is_update,
                    });
                }
            }
        }
    }

    fn handle_quote_data(&self, _message: &[Value]) {}
    fn handle_series_data(&self, _event: TradingViewDataEvent, _messages: &[Value]) {}

    fn notify_error(&self, error: tradingview::Error, _message: &[Value]) {
        debug!(series = %self.series, %error, "WebSocket transport notification on bar stream");
    }
}

async fn run_bar_stream_supervisor(
    config: Arc<TradingViewConfig>,
    servers: Vec<DataServer>,
    endpoint: Option<Url>,
    series: SeriesKey,
    btx: broadcast::Sender<LiveBarEvent>,
    cancel: CancellationToken,
) {
    let mut attempt: u32 = 0;

    while !cancel.is_cancelled() {
        let server = servers[(attempt as usize) % servers.len()];
        let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<LiveBarEvent>(INTERNAL_MPSC_CAPACITY);
        let handler = BarStreamHandler {
            series: series.clone(),
            tx: mpsc_tx,
        };

        let connect_res = WebSocketClient::builder()
            .auth_token(config.auth_token.as_str())
            .server(server)
            .handler(handler)
            .maybe_endpoint(endpoint.clone())
            .build()
            .await;

        let ws = match connect_res {
            Ok(client) => client,
            Err(err) => {
                attempt = attempt.saturating_add(1);
                let delay = backoff_delay(
                    config.reconnect_base_delay,
                    config.reconnect_max_delay,
                    attempt,
                );
                warn!(
                    series = %series,
                    ?server,
                    attempt,
                    ?delay,
                    %err,
                    "live bar stream connect failed; scheduling reconnect"
                );
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => break,
                    () = tokio::time::sleep(delay) => continue,
                }
            }
        };

        let chart_session = gen_session_id("cs");
        let sym_series_id = format!("sds_sym_{}", tradingview::utils::gen_id());
        let instrument = map_to_tv_wire_symbol(&series.symbol_key()).ticker();

        let setup_ok = async {
            ws.create_chart_session(&chart_session).await?;
            ws.resolve_symbol()
                .session(&chart_session)
                .symbol_series_id(&sym_series_id)
                .instrument(&instrument)
                .adjustment(series.adjustment.to_tv_adjustment())
                .call()
                .await?;
            ws.create_series()
                .chart_session(&chart_session)
                .series_identifier("sds_1")
                .series_id("s1")
                .symbol_series_id(&sym_series_id)
                .interval(series.interval.to_tv_interval())
                .bar_count(2)
                .call()
                .await?;
            Ok::<(), tradingview::Error>(())
        }
        .await;

        if let Err(err) = setup_ok {
            let _ = ws.delete_chart_session(&chart_session).await;
            let _ = ws.close().await;
            attempt = attempt.saturating_add(1);
            let delay = backoff_delay(
                config.reconnect_base_delay,
                config.reconnect_max_delay,
                attempt,
            );
            warn!(series = %series, %err, ?delay, "bar session setup failed; reconnecting");
            tokio::select! {
                biased;
                () = cancel.cancelled() => break,
                () = tokio::time::sleep(delay) => continue,
            }
        }

        info!(series = %series, ?server, session = %chart_session, "live bar stream connected");
        attempt = 0;

        // Forward single-bar events asynchronously without blocking the WS reader.
        let forward_btx = btx.clone();
        let forward_task = tokio::spawn(async move {
            while let Some(ev) = mpsc_rx.recv().await {
                let _ = forward_btx.send(ev);
            }
        });

        let reader_ws = Arc::clone(&ws);
        let reader_exit = tokio::select! {
            biased;
            () = cancel.cancelled() => {
                debug!(series = %series, session = %chart_session, "unsubscribing live bar stream");
                None
            }
            res = reader_ws.subscribe() => Some(res),
        };

        let _ = ws.delete_chart_session(&chart_session).await;
        let _ = ws.close().await;
        forward_task.abort();

        if cancel.is_cancelled() {
            break;
        }

        attempt = attempt.saturating_add(1);
        let delay = backoff_delay(
            config.reconnect_base_delay,
            config.reconnect_max_delay,
            attempt,
        );
        warn!(
            series = %series,
            exit = ?reader_exit,
            attempt,
            ?delay,
            "live bar stream disconnected; auto-reconnecting"
        );
        tokio::select! {
            biased;
            () = cancel.cancelled() => break,
            () = tokio::time::sleep(delay) => {}
        }
    }
}

struct QuoteStreamHandler {
    tx: mpsc::Sender<LiveQuote>,
    merged_state: Mutex<HashMap<String, LiveQuote>>,
}

impl Handler for QuoteStreamHandler {
    fn handle_events(&self, _event: TradingViewDataEvent, _message: &[Value]) {
        // Quote frames are processed exclusively in `handle_quote_data` to prevent double delivery.
    }

    fn handle_quote_data(&self, message: &[Value]) {
        let Some(payload) = message.get(1) else {
            return;
        };
        let Ok(qd) = QuoteData::deserialize(payload) else {
            return;
        };
        let raw_name = qd.name.as_str().trim();
        if raw_name.is_empty() {
            return;
        }
        let (exchange, symbol, ticker) = match SymbolKey::parse(raw_name) {
            Ok(key) => {
                // Audit Fix #1: Reverse-map TradingView continuous futures wire aliases
                // (`HNX:VN301!` -> `HNX:VN30F1!`, `HNX:VN302!` -> `HNX:VN30F2!`) so Watchlist
                // quotesMap keys match canonical Vietnamese derivatives tickers.
                let canonical = map_from_tv_wire_symbol(&key);
                (
                    canonical.exchange.clone(),
                    canonical.symbol.clone(),
                    canonical.ticker(),
                )
            }
            Err(_) => {
                let ex = qd
                    .value
                    .exchange
                    .map(|u| normalize_exchange_alias(u.as_str()))
                    .unwrap_or_default();
                let sym = qd
                    .value
                    .symbol
                    .map(|u| u.as_str().trim().to_ascii_uppercase())
                    .unwrap_or_else(|| raw_name.to_ascii_uppercase());
                let canonical = if !ex.is_empty() {
                    map_from_tv_wire_symbol(&SymbolKey {
                        exchange: ex,
                        symbol: sym,
                    })
                } else {
                    SymbolKey {
                        exchange: ex,
                        symbol: sym,
                    }
                };
                let t = if canonical.exchange.is_empty() {
                    canonical.symbol.clone()
                } else {
                    canonical.ticker()
                };
                (canonical.exchange, canonical.symbol, t)
            }
        };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let ts_ms = qd
            .value
            .timestamp
            .filter(|t| t.is_finite() && *t > 0.0)
            .map_or(now_ms, |secs| (secs as i64).saturating_mul(1_000));

        let snapshot = {
            let mut map = self
                .merged_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let entry = map.entry(ticker.clone()).or_insert_with(|| LiveQuote {
                ticker: ticker.clone(),
                exchange: exchange.clone(),
                symbol: symbol.clone(),
                timestamp_ms: ts_ms,
                last_price: None,
                bid: None,
                bid_size: None,
                ask: None,
                ask_size: None,
                open: None,
                high: None,
                low: None,
                prev_close: None,
                change: None,
                change_percent: None,
                volume: None,
                currency: None,
            });
            entry.timestamp_ms = ts_ms;
            if let Some(v) = qd.value.price.filter(|x| x.is_finite()) {
                entry.last_price = Some(v);
            }
            if let Some(v) = qd.value.bid.filter(|x| x.is_finite()) {
                entry.bid = Some(v);
            }
            if let Some(v) = qd.value.bid_size.filter(|x| x.is_finite()) {
                entry.bid_size = Some(v);
            }
            if let Some(v) = qd.value.ask.filter(|x| x.is_finite()) {
                entry.ask = Some(v);
            }
            if let Some(v) = qd.value.ask_size.filter(|x| x.is_finite()) {
                entry.ask_size = Some(v);
            }
            if let Some(v) = qd.value.open.filter(|x| x.is_finite()) {
                entry.open = Some(v);
            }
            if let Some(v) = qd.value.high.filter(|x| x.is_finite()) {
                entry.high = Some(v);
            }
            if let Some(v) = qd.value.low.filter(|x| x.is_finite()) {
                entry.low = Some(v);
            }
            if let Some(v) = qd.value.prev_close.filter(|x| x.is_finite()) {
                entry.prev_close = Some(v);
            }
            if let Some(v) = qd.value.change.filter(|x| x.is_finite()) {
                entry.change = Some(v);
            }
            if let Some(v) = qd.value.change_percent.filter(|x| x.is_finite()) {
                entry.change_percent = Some(v);
            }
            if let Some(v) = qd.value.volume.filter(|x| x.is_finite()) {
                entry.volume = Some(v);
            }
            if let Some(cur) = qd.value.currency {
                let s = cur.as_str().trim();
                if !s.is_empty() {
                    entry.currency = Some(s.to_owned());
                }
            }
            entry.clone()
        };

        let _ = self.tx.try_send(snapshot);
    }

    fn handle_series_data(&self, _event: TradingViewDataEvent, _messages: &[Value]) {}

    fn notify_error(&self, error: tradingview::Error, _message: &[Value]) {
        debug!(%error, "WebSocket transport notification on quote stream");
    }
}

async fn run_quote_stream_supervisor(
    config: Arc<TradingViewConfig>,
    servers: Vec<DataServer>,
    endpoint: Option<Url>,
    symbols: Vec<SymbolKey>,
    btx: broadcast::Sender<LiveQuote>,
    cancel: CancellationToken,
) {
    let tickers: Vec<String> = symbols
        .iter()
        .map(|s| map_to_tv_wire_symbol(s).ticker())
        .collect();
    let mut attempt: u32 = 0;

    while !cancel.is_cancelled() {
        let server = servers[(attempt as usize) % servers.len()];
        let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<LiveQuote>(INTERNAL_MPSC_CAPACITY);
        let handler = QuoteStreamHandler {
            tx: mpsc_tx,
            merged_state: Mutex::new(HashMap::new()),
        };

        let ws = match WebSocketClient::builder()
            .auth_token(config.auth_token.as_str())
            .server(server)
            .handler(handler)
            .maybe_endpoint(endpoint.clone())
            .build()
            .await
        {
            Ok(client) => client,
            Err(err) => {
                attempt = attempt.saturating_add(1);
                let delay = backoff_delay(
                    config.reconnect_base_delay,
                    config.reconnect_max_delay,
                    attempt,
                );
                warn!(?server, attempt, ?delay, %err, "quote stream connect failed; reconnecting");
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => break,
                    () = tokio::time::sleep(delay) => continue,
                }
            }
        };

        let quote_session = gen_session_id("qs");
        let ticker_refs: Vec<&str> = tickers.iter().map(String::as_str).collect();
        let setup_ok = async {
            ws.create_quote_session(&quote_session).await?;
            ws.set_fields(&quote_session).await?;
            ws.add_symbols(&quote_session, &ticker_refs).await?;
            Ok::<(), tradingview::Error>(())
        }
        .await;

        if let Err(err) = setup_ok {
            let _ = ws.delete_quote_session(&quote_session).await;
            let _ = ws.close().await;
            attempt = attempt.saturating_add(1);
            let delay = backoff_delay(
                config.reconnect_base_delay,
                config.reconnect_max_delay,
                attempt,
            );
            warn!(%err, ?delay, "quote session setup failed; reconnecting");
            tokio::select! {
                biased;
                () = cancel.cancelled() => break,
                () = tokio::time::sleep(delay) => continue,
            }
        }

        info!(
            symbols = ?tickers,
            ?server,
            session = %quote_session,
            "live quote stream connected"
        );
        attempt = 0;

        let forward_btx = btx.clone();
        let forward_task = tokio::spawn(async move {
            while let Some(q) = mpsc_rx.recv().await {
                let _ = forward_btx.send(q);
            }
        });

        let reader_ws = Arc::clone(&ws);
        let reader_exit = tokio::select! {
            biased;
            () = cancel.cancelled() => {
                debug!(session = %quote_session, "unsubscribing live quote stream");
                None
            }
            res = reader_ws.subscribe() => Some(res),
        };

        let _ = ws.delete_quote_session(&quote_session).await;
        let _ = ws.close().await;
        forward_task.abort();

        if cancel.is_cancelled() {
            break;
        }

        attempt = attempt.saturating_add(1);
        let delay = backoff_delay(
            config.reconnect_base_delay,
            config.reconnect_max_delay,
            attempt,
        );
        warn!(exit = ?reader_exit, attempt, ?delay, "quote stream disconnected; auto-reconnecting");
        tokio::select! {
            biased;
            () = cancel.cancelled() => break,
            () = tokio::time::sleep(delay) => {}
        }
    }
}

/// Maps common Vietnamese broker futures aliases (`HNX:VN30F1!` / `HNX:VN30F2!`)
/// to TradingView's canonical wire continuous-contract identifiers (`HNX:VN301!` / `HNX:VN302!`).
fn map_to_tv_wire_symbol(symbol: &SymbolKey) -> SymbolKey {
    if symbol.exchange == "HNX" {
        match symbol.symbol.as_str() {
            "VN30F1!" | "VN30F1M" => {
                return SymbolKey {
                    exchange: "HNX".to_owned(),
                    symbol: "VN301!".to_owned(),
                };
            }
            "VN30F2!" | "VN30F2M" => {
                return SymbolKey {
                    exchange: "HNX".to_owned(),
                    symbol: "VN302!".to_owned(),
                };
            }
            _ => {}
        }
    }
    symbol.clone()
}

/// Reverse-maps TradingView's wire continuous-contract identifiers (`HNX:VN301!` / `HNX:VN302!`)
/// back to canonical Vietnamese derivatives tickers (`HNX:VN30F1!` / `HNX:VN30F2!`) (`Audit Fix #1`).
#[must_use]
pub fn map_from_tv_wire_symbol(symbol: &SymbolKey) -> SymbolKey {
    if symbol.exchange == "HNX" {
        match symbol.symbol.as_str() {
            "VN301!" => {
                return SymbolKey {
                    exchange: "HNX".to_owned(),
                    symbol: "VN30F1!".to_owned(),
                };
            }
            "VN302!" => {
                return SymbolKey {
                    exchange: "HNX".to_owned(),
                    symbol: "VN30F2!".to_owned(),
                };
            }
            _ => {}
        }
    }
    symbol.clone()
}

fn datapoint_to_ohlcv_bar(dp: &DataPoint, interval: Interval) -> Option<OhlcvBar> {
    if dp.value.len() < 5 {
        return None;
    }
    let ts_secs = dp.timestamp();
    let open = dp.open();
    let high = dp.high();
    let low = dp.low();
    let close = dp.close();
    if !open.is_finite() || !high.is_finite() || !low.is_finite() || !close.is_finite() {
        return None;
    }
    let raw_vol = dp.volume();
    let volume = if raw_vol.is_finite() && raw_vol >= 0.0 {
        raw_vol
    } else {
        0.0
    };
    let ts_ms = canonicalize_bar_timestamp_ms(ts_secs.saturating_mul(1_000), interval);
    Some(OhlcvBar::new(ts_ms, open, high, low, close, volume))
}

fn backoff_delay(base: Duration, max: Duration, attempt: u32) -> Duration {
    let exp = attempt.saturating_sub(1).min(8);
    let factor = 1_u32 << exp;
    base.saturating_mul(factor).min(max)
}

fn map_tv_error(err: tradingview::Error) -> DataError {
    match err {
        tradingview::Error::Timeout(msg) => DataError::Timeout(msg.to_string()),
        tradingview::Error::TradingView {
            source: tradingview::error::TradingViewError::SymbolError,
        } => DataError::SymbolNotFound("symbol rejected by TradingView".to_owned()),
        other => DataError::Provider(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vn30_futures_wire_alias_round_trips_canonically() {
        let f1 = SymbolKey::new("HNX", "VN30F1!").unwrap();
        let wire1 = map_to_tv_wire_symbol(&f1);
        assert_eq!(wire1.ticker(), "HNX:VN301!");
        assert_eq!(map_from_tv_wire_symbol(&wire1).ticker(), "HNX:VN30F1!");

        let f2 = SymbolKey::new("HNX", "VN30F2!").unwrap();
        let wire2 = map_to_tv_wire_symbol(&f2);
        assert_eq!(wire2.ticker(), "HNX:VN302!");
        assert_eq!(map_from_tv_wire_symbol(&wire2).ticker(), "HNX:VN30F2!");

        // Equities and non-HNX symbols pass through untouched.
        let fpt = SymbolKey::new("HOSE", "FPT").unwrap();
        assert_eq!(map_to_tv_wire_symbol(&fpt), fpt);
        assert_eq!(map_from_tv_wire_symbol(&fpt), fpt);
    }
}
