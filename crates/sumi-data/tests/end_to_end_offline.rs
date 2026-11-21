use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use sumi_data::{
    Adjustment, BarSubscription, BarsRequest, BatchBarsResult, CachePolicy, CafeFImportConfig,
    CafeFImporter, DataError, DataSourceOrigin, HistoryFetchOutput, Interval, MarketDataProvider,
    MarketDataService, OhlcvBar, QuoteSubscription, RateLimitConfig, ResolvedSymbol, Result,
    SeriesKey, SqliteMarketStore, SymbolKey, SymbolSearchResult, TradingViewConfig,
    TradingViewFeed, canonicalize_bar_timestamp_ms,
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

fn tv_frame(val: &Value) -> Message {
    let text = val.to_string();
    let utf16_len = text.encode_utf16().count();
    Message::Text(format!("~m~{utf16_len}~m~{text}").into())
}

fn parse_tv_frames(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(after_prefix) = rest.strip_prefix("~m~") {
        let Some(end) = after_prefix.find("~m~") else {
            break;
        };
        let Ok(len) = after_prefix[..end].parse::<usize>() else {
            break;
        };
        let body = &after_prefix[end + 3..];
        let byte_len = body
            .char_indices()
            .scan(0usize, |acc, (idx, ch)| {
                let next = *acc + ch.len_utf16();
                *acc = next;
                Some((idx + ch.len_utf8(), next))
            })
            .find(|&(_, u16_count)| u16_count >= len)
            .map_or(body.len(), |(b, _)| b);
        let (payload, tail) = body.split_at(byte_len);
        if let Ok(val) = serde_json::from_str::<Value>(payload) {
            out.push(val);
        }
        rest = tail;
    }
    out
}

/// Spins up a local loopback TradingView-protocol WebSocket server to verify
/// `TradingViewFeed::{resolve_symbol, fetch_history, subscribe_bars, subscribe_quotes}`
/// including auto-reconnect after forced disconnect and clean `unsubscribe`.
async fn spawn_mock_tv_server(
    drop_first_bar_conn: bool,
    send_live_du: bool,
) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let connections = Arc::new(AtomicUsize::new(0));
    let conn_counter = Arc::clone(&connections);

    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            let conn_id = conn_counter.fetch_add(1, Ordering::SeqCst);
            let should_drop_bars = drop_first_bar_conn && conn_id == 0;
            let emit_du = send_live_du;
            tokio::spawn(async move {
                let Ok(mut ws) = tokio_tungstenite::accept_async(tcp).await else {
                    return;
                };
                while let Some(Ok(msg)) = ws.next().await {
                    let Message::Text(text) = msg else { continue };
                    for frame in parse_tv_frames(&text) {
                        let method = frame.get("m").and_then(Value::as_str).unwrap_or("");
                        let params = frame
                            .get("p")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default();

                        match method {
                            "resolve_symbol" => {
                                let cs = params.first().and_then(Value::as_str).unwrap_or("cs");
                                let sid = params.get(1).and_then(Value::as_str).unwrap_or("sds");
                                let info = json!({
                                    "pro_name": "HOSE:FPT",
                                    "name": "FPT",
                                    "exchange": "HOSE",
                                    "listed_exchange": "HOSE",
                                    "description": "FPT Corporation",
                                    "type": "stock",
                                    "currency_code": "VND",
                                    "timezone": "Asia/Ho_Chi_Minh",
                                    "session": "0900-1500",
                                    "pricescale": 1,
                                    "minmov": 10,
                                    "minmove2": 0,
                                    "variable_tick_size": "10 10000 50 50000 100",
                                    "pointvalue": 1.0,
                                    "delay": 0,
                                    "isin": "VN000000FPT1"
                                });
                                let _ = ws
                                    .send(tv_frame(&json!({
                                        "m": "symbol_resolved",
                                        "p": [cs, sid, info]
                                    })))
                                    .await;
                            }
                            "create_series" => {
                                let cs = params.first().and_then(Value::as_str).unwrap_or("cs");
                                let series_id =
                                    params.get(1).and_then(Value::as_str).unwrap_or("sds_1");
                                // 2026-11-05 02:00 UTC and 2026-11-06 02:00 UTC (seconds).
                                let rows = json!([
                                    {"i": 0, "v": [1793844000.0, 54500.0, 54600.0, 52800.0, 52900.0, 8006200.0]},
                                    {"i": 1, "v": [1793930400.0, 51400.0, 51800.0, 50800.0, 50800.0, 12162200.0]}
                                ]);
                                let _ = ws
                                    .send(tv_frame(&json!({
                                        "m": "timescale_update",
                                        "p": [cs, { series_id: { "s": rows, "ns": { "d": "", "indexes": [] } } }]
                                    })))
                                    .await;
                                let _ = ws
                                    .send(tv_frame(&json!({
                                        "m": "series_completed",
                                        "p": [cs, series_id, "streaming", "s1", { "data_completed": "end" }]
                                    })))
                                    .await;

                                if should_drop_bars {
                                    // Simulate sudden server disconnect to verify supervisor reconnect!
                                    let _ = ws.send(Message::Close(None)).await;
                                    return;
                                } else if emit_du {
                                    // Push one live `du` intra-bar tick update.
                                    let du_rows = json!([
                                        {"i": 1, "v": [1793930400.0, 51400.0, 52000.0, 50800.0, 51900.0, 12500000.0]}
                                    ]);
                                    let _ = ws
                                        .send(tv_frame(&json!({
                                            "m": "du",
                                            "p": [cs, { series_id: { "s": du_rows } }]
                                        })))
                                        .await;
                                }
                            }
                            "quote_add_symbols" => {
                                let qs = params.first().and_then(Value::as_str).unwrap_or("qs");
                                let sym =
                                    params.get(1).and_then(Value::as_str).unwrap_or("HOSE:FPT");
                                // Delta 1: bid/ask only
                                let _ = ws
                                    .send(tv_frame(&json!({
                                        "m": "qsd",
                                        "p": [qs, { "n": sym, "s": "ok", "v": { "bid": 51800.0, "ask": 51900.0 } }]
                                    })))
                                    .await;
                                // Delta 2: last price + volume (must merge with bid/ask!)
                                let _ = ws
                                    .send(tv_frame(&json!({
                                        "m": "qsd",
                                        "p": [qs, { "n": sym, "s": "ok", "v": { "lp": 51900.0, "volume": 12500000.0, "ch": 1100.0, "chp": 2.16 } }]
                                    })))
                                    .await;
                            }
                            "chart_delete_session" | "quote_delete_session" => {
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            });
        }
    });

    (format!("ws://{addr}/socket.io/websocket"), connections)
}

#[tokio::test]
async fn tradingview_feed_resolve_history_and_daily_canonicalization_over_mock_ws() {
    let (endpoint, _) = spawn_mock_tv_server(false, false).await;
    let feed = TradingViewFeed::new(TradingViewConfig {
        endpoint_override: Some(endpoint),
        fallback_server: None,
        rate_limit: RateLimitConfig::fast_test(),
        request_timeout: Duration::from_secs(5),
        ..TradingViewConfig::default()
    });

    let sym = SymbolKey::new("HOSE", "FPT").unwrap();
    let resolved = feed.resolve_symbol(&sym).await.unwrap();
    assert_eq!(resolved.ticker, "HOSE:FPT");
    assert_eq!(resolved.description, "FPT Corporation");
    assert_eq!(resolved.price_precision, 0);
    assert_eq!(resolved.volume_precision, 0);
    assert_eq!(resolved.variable_tick_size, "10 10000 50 50000 100");
    assert_eq!(resolved.tick_size_at(9_000.0), Some(10.0));
    assert_eq!(resolved.tick_size_at(30_000.0), Some(50.0));
    assert_eq!(resolved.tick_size_at(55_000.0), Some(100.0));
    assert_eq!(resolved.vn_price_band.as_ref().unwrap().label, "±7%");
    assert_eq!(resolved.isin, "VN000000FPT1");

    // Fetch 1D history -> 02:00 UTC timestamps must canonicalize to 00:00 UTC (no CafeF duplicates!)
    let series = SeriesKey::new(&sym, Interval::D1, Adjustment::Splits);
    let hist = feed.fetch_history(&series, 50, None).await.unwrap();
    assert_eq!(hist.bars.len(), 2);
    assert_eq!(
        hist.bars[0].timestamp,
        canonicalize_bar_timestamp_ms(1_793_844_000_000, Interval::D1)
    );
    assert_eq!(hist.bars[0].timestamp % 86_400_000, 0);
    assert!((hist.bars[1].turnover - 50_800.0 * 12_162_200.0).abs() < 1.0);
}

#[tokio::test]
async fn tradingview_feed_live_bar_reconnect_and_merged_quotes_over_mock_ws() {
    let (endpoint, connections) = spawn_mock_tv_server(true, true).await;
    let feed = TradingViewFeed::new(TradingViewConfig {
        endpoint_override: Some(endpoint),
        fallback_server: None,
        rate_limit: RateLimitConfig::fast_test(),
        reconnect_base_delay: Duration::from_millis(50),
        reconnect_max_delay: Duration::from_millis(100),
        request_timeout: Duration::from_secs(5),
        ..TradingViewConfig::default()
    });

    let sym = SymbolKey::new("HOSE", "FPT").unwrap();
    let series = SeriesKey::new(&sym, Interval::M15, Adjustment::Splits);

    // 1. Live bars: first connection emits snapshot then drops; supervisor must reconnect & deliver live `du` update!
    let mut bar_sub = feed.subscribe_bars(series).await.unwrap();
    let mut saw_update = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(ev)) = tokio::time::timeout(Duration::from_secs(2), bar_sub.recv()).await
            && ev.is_update
            && (ev.bar.close - 51_900.0).abs() < 1e-6
        {
            saw_update = true;
            break;
        }
    }
    bar_sub.unsubscribe().await;
    assert!(
        saw_update,
        "supervisor must auto-reconnect and deliver post-reconnect update"
    );
    assert!(
        connections.load(Ordering::SeqCst) >= 2,
        "expected >= 2 connections due to auto-reconnect"
    );

    // 2. Live quotes: verify delta updates merge into a complete snapshot!
    let mut quote_sub = feed.subscribe_quotes(vec![sym]).await.unwrap();
    let mut saw_merged = false;
    let q_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < q_deadline {
        if let Ok(Some(q)) = tokio::time::timeout(Duration::from_secs(2), quote_sub.recv()).await
            && q.last_price == Some(51_900.0)
            && q.bid == Some(51_800.0)
            && q.ask == Some(51_900.0)
        {
            saw_merged = true;
            break;
        }
    }
    quote_sub.unsubscribe().await;
    assert!(
        saw_merged,
        "expected quote stream to merge bid/ask with lp delta"
    );
}

struct PaginatedMockProvider {
    all_bars: Vec<OhlcvBar>,
    calls: AtomicUsize,
}

#[async_trait]
impl MarketDataProvider for PaginatedMockProvider {
    async fn search_symbols(
        &self,
        _query: &str,
        _exchange: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<SymbolSearchResult>> {
        Ok(Vec::new())
    }

    async fn resolve_symbol(&self, symbol: &SymbolKey) -> Result<ResolvedSymbol> {
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
        self.calls.fetch_add(1, Ordering::SeqCst);
        let filtered: Vec<OhlcvBar> = match from_ms {
            Some(from) => self
                .all_bars
                .iter()
                .copied()
                .filter(|b| b.timestamp >= from)
                .collect(),
            None => self.all_bars.clone(),
        };
        let start = filtered.len().saturating_sub(num_bars);
        let slice = filtered[start..].to_vec();
        let reached_start =
            slice.first().map(|b| b.timestamp) == self.all_bars.first().map(|b| b.timestamp);
        Ok(HistoryFetchOutput {
            bars: slice,
            symbol_info: Some(ResolvedSymbol::from_cafef_vn_ticker(
                &series.exchange,
                &series.symbol,
                false,
            )),
            reached_start_of_history: reached_start,
            pages_fetched: 1,
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
        for s in symbols {
            let sk = SeriesKey::new(s, interval, adjustment);
            match self.fetch_history(&sk, num_bars, None).await {
                Ok(out) => successful.push((s.clone(), out.bars)),
                Err(e) => failed.push((s.clone(), e.to_string())),
            }
        }
        BatchBarsResult {
            successful,
            failed,
            elapsed_ms: 1,
        }
    }

    async fn subscribe_bars(&self, _series: SeriesKey) -> Result<BarSubscription> {
        Err(DataError::Offline("not used".to_owned()))
    }

    async fn subscribe_quotes(&self, symbols: Vec<SymbolKey>) -> Result<QuoteSubscription> {
        let (tx, _) = broadcast::channel(8);
        let cancel = CancellationToken::new();
        let c = cancel.clone();
        let task = tokio::spawn(async move { c.cancelled().await });
        Ok(QuoteSubscription::new(symbols, tx, cancel, task))
    }
}

#[tokio::test]
async fn cafef_seed_plus_daily_gap_fill_has_zero_duplicate_dates_and_forward_backfill_works() {
    // 1. Seed CafeF Adjusted + Raw CSV into SQLite.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("CafeF.SolieuGD.Adjusted.Upto06112026.csv"),
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n\
         FPT,20261106,51.4,51.8,50.8,50.8,12162200\n\
         FPT,20261105,54.5,54.6,52.8,52.9,8006200\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("CafeF.SolieuGD.Raw.Upto06112026.csv"),
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n\
         FPT,20261106,51.4,51.8,50.8,50.8,12162200\n",
    )
    .unwrap();

    let store = SqliteMarketStore::open_in_memory().unwrap();
    let importer = CafeFImporter::new(CafeFImportConfig {
        include_raw: true,
        ..CafeFImportConfig::default()
    });
    let summary = importer.import_dir(dir.path(), &store).unwrap();
    assert_eq!(summary.files_processed.len(), 2);
    assert_eq!(summary.total_bars_written, 3);

    // 2. Verify forward backfill & sync_batch on a 30-bar intraday series.
    let now_ms = chrono::Utc::now().timestamp_millis();
    let step = Interval::M15.duration_ms();
    let all_bars: Vec<OhlcvBar> = (0..30)
        .map(|i| {
            let ts = now_ms - ((29 - i) as i64) * step;
            let px = 60_000.0 + (i as f64) * 100.0;
            OhlcvBar::new(ts, px, px + 50.0, px - 50.0, px + 20.0, 2_000.0)
        })
        .collect();
    let provider = Arc::new(PaginatedMockProvider {
        all_bars: all_bars.clone(),
        calls: AtomicUsize::new(0),
    });
    let svc = MarketDataService::new(
        Arc::clone(&provider),
        store.clone(),
        CachePolicy {
            cold_fetch_min_bars: 10,
            forward_backfill_multiplier: 1,
            ..CachePolicy::default()
        },
    );

    let sym = SymbolKey::new("HOSE", "VCB").unwrap();
    let key = SeriesKey::new(&sym, Interval::M15, Adjustment::Splits);

    // Cold Init fetches only newest 10 bars (indices 20..=29) -> has_older = true!
    let init = svc
        .get_bars(&BarsRequest::init(key.clone(), 10))
        .await
        .unwrap();
    assert_eq!(init.source, DataSourceOrigin::RemoteLive);
    assert_eq!(init.bars.len(), 10);
    assert!(init.has_older);
    assert_eq!(init.bars[0].timestamp, all_bars[20].timestamp);

    // Forward before index 20 triggers online backfill -> retrieves indices 10..=19!
    let fwd1 = svc
        .get_bars(&BarsRequest::forward(
            key.clone(),
            init.bars[0].timestamp,
            10,
        ))
        .await
        .unwrap();
    assert_eq!(fwd1.source, DataSourceOrigin::CacheWithGapFill);
    assert_eq!(fwd1.bars.len(), 10);
    assert_eq!(fwd1.bars[0].timestamp, all_bars[10].timestamp);
    assert!(fwd1.has_older);
    assert!(fwd1.has_newer);

    // Forward before index 10 hits start of history -> indices 0..=9, has_older = false!
    let fwd2 = svc
        .get_bars(&BarsRequest::forward(
            key.clone(),
            fwd1.bars[0].timestamp,
            10,
        ))
        .await
        .unwrap();
    assert_eq!(fwd2.bars.len(), 10);
    assert_eq!(fwd2.bars[0].timestamp, all_bars[0].timestamp);
    assert!(!fwd2.has_older);
    assert!(fwd2.has_newer);

    // sync_batch persists multiple symbols into SQLite cleanly.
    let batch_syms = vec![
        SymbolKey::new("HOSE", "SSI").unwrap(),
        SymbolKey::new("HOSE", "HPG").unwrap(),
    ];
    let batch = svc
        .sync_batch(&batch_syms, Interval::M15, Adjustment::Splits, 10)
        .await;
    assert_eq!(batch.successful.len(), 2);
    assert!(batch.failed.is_empty());
    let ssi_offline = svc
        .with_offline_mode(true)
        .get_bars(&BarsRequest::init(
            SeriesKey::new(&batch_syms[0], Interval::M15, Adjustment::Splits),
            10,
        ))
        .await
        .unwrap();
    assert_eq!(ssi_offline.source, DataSourceOrigin::OfflineCache);
    assert_eq!(ssi_offline.bars.len(), 10);
}
