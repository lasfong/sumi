use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use sumi_data::{
    Adjustment, BarQueryMode, BarsRequest, CachePolicy, CafeFImportConfig, CafeFImporter, Interval,
    MarketDataProvider, MarketDataService, SeriesKey, SqliteMarketStore, SymbolKey,
    TradingViewConfig, TradingViewFeed, init_tracing,
};

#[derive(Debug, Parser)]
#[command(
    name = "sumi-data-cli",
    version,
    about = "Sumi v2.0 Market Data CLI (TradingView pure-Rust feed + SQLite Cache-First + CafeF Importer)"
)]
struct Cli {
    /// Path to local SQLite cache database.
    #[arg(long, default_value = "data/cache/sumi_market.db", global = true)]
    db: PathBuf,

    /// Force 100% offline operation using only local SQLite cache.
    #[arg(long, default_value_t = false, global = true)]
    offline: bool,

    /// Emit raw JSON instead of formatted human tables.
    #[arg(long, default_value_t = false, global = true)]
    json: bool,

    /// Tracing log level (`error`, `warn`, `info`, `debug`, `trace`; WebSocket frames always capped per RECOMMENDED_LOG_DIRECTIVES).
    #[arg(long, default_value = "warn", global = true)]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Bulk-import CafeF historical CSV archives (`CafeF.INDEX*.csv`, `CafeF.SolieuGD.*.csv`) into local SQLite cache.
    ImportCafef {
        /// Directory containing CafeF CSV files.
        #[arg(long, default_value = "data/raw/cafef")]
        dir: PathBuf,

        /// Also import `CafeF.SolieuGD.Raw*.csv` into the `unadjusted` bucket.
        #[arg(long, default_value_t = false)]
        include_raw: bool,

        /// Price multiplier applied to CafeF stock quotes (default 1000.0 converts thousand-VND -> raw VND).
        #[arg(long, default_value_t = 1000.0)]
        stock_multiplier: f64,
    },

    /// Search instruments across TradingView & local cache (optional exchange filter: HOSE, HNX, UPCOM, BINANCE, NASDAQ, ...).
    Search {
        /// Search query (e.g. `FPT`, `BTCUSDT`, `VN30F`).
        query: String,

        /// Optional exchange filter (e.g. `HOSE`, `HNX`, `BINANCE`, `NASDAQ`).
        #[arg(long)]
        exchange: Option<String>,

        /// Maximum results to return.
        #[arg(long, default_value_t = 15)]
        limit: usize,
    },

    /// Resolve full instrument metadata (precision, tick ladder, variable tick size, Vietnam price band).
    Resolve {
        /// Instrument identifier in `EXCHANGE:SYMBOL` format (e.g. `HOSE:FPT`, `HNX:VN30F1!`, `FX_IDC:USDVND`).
        symbol: String,
    },

    /// Fetch cache-first paginated OHLCV bars (`Init`, `Forward { before_ms }`, or `Backward { after_ms }`).
    Bars {
        /// Instrument identifier in `EXCHANGE:SYMBOL` format.
        symbol: String,

        /// Bar timeframe (`1m`, `5m`, `15m`, `30m`, `1h`, `4h`, `1D`, `1W`, `1M`).
        #[arg(long, default_value = "15m")]
        interval: String,

        /// Maximum number of bars to return.
        #[arg(long, default_value_t = 200)]
        limit: usize,

        /// Corporate action adjustment (`splits`, `dividends`, `unadjusted`).
        #[arg(long, default_value = "splits")]
        adjustment: String,

        /// KLineChart `Forward` pagination: load bars strictly older than this Unix millisecond timestamp.
        #[arg(long, conflicts_with = "after")]
        before: Option<i64>,

        /// KLineChart `Backward` pagination: load bars strictly newer than this Unix millisecond timestamp.
        #[arg(long, conflicts_with = "before")]
        after: Option<i64>,
    },

    /// Stream live real-time candlestick updates (or live quotes with `--quotes`) over WebSocket.
    Stream {
        /// Instrument identifier in `EXCHANGE:SYMBOL` format.
        symbol: String,

        /// Bar timeframe (`1m`, `5m`, `15m`, `30m`, `1h`, `4h`, `1D`, `1W`, `1M`).
        #[arg(long, default_value = "1m")]
        interval: String,

        /// Corporate action adjustment (`splits`, `dividends`, `unadjusted`).
        #[arg(long, default_value = "splits")]
        adjustment: String,

        /// Stream live quotes (`qsd`) instead of OHLCV chart bars.
        #[arg(long, default_value_t = false)]
        quotes: bool,

        /// Optional maximum duration in seconds before unsubscribing cleanly.
        #[arg(long)]
        duration_secs: Option<u64>,

        /// Optional maximum number of events to receive before unsubscribing cleanly.
        #[arg(long)]
        max_events: Option<usize>,

        /// Disable automatic SQLite archival of incoming live bars.
        #[arg(long, default_value_t = false)]
        no_archive: bool,
    },
}

fn format_ms_utc(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms).map_or_else(
        || ms.to_string(),
        |dt| dt.format("%Y-%m-%d %H:%M UTC").to_string(),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level);

    let store = SqliteMarketStore::open(&cli.db)?;
    let feed = Arc::new(TradingViewFeed::new(TradingViewConfig::default()));
    let service = MarketDataService::new(
        feed,
        store.clone(),
        CachePolicy {
            offline_mode: cli.offline,
            ..CachePolicy::default()
        },
    );

    match cli.command {
        Commands::ImportCafef {
            dir,
            include_raw,
            stock_multiplier,
        } => {
            let importer = CafeFImporter::new(CafeFImportConfig {
                stock_price_multiplier: stock_multiplier,
                include_raw,
                ..CafeFImportConfig::default()
            });
            let summary =
                tokio::task::spawn_blocking(move || importer.import_dir(&dir, &store)).await??;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Imported CafeF archives into {} in {} ms",
                    cli.db.display(),
                    summary.elapsed_ms
                );
                println!("  Files:            {}", summary.files_processed.join(", "));
                println!("  Distinct symbols: {}", summary.distinct_symbols);
                println!("  Total series:     {}", summary.total_series);
                println!("  Total bars:       {}", summary.total_bars_written);
                println!("  Quarantined rows: {}", summary.quarantined_rows);
            }
        }

        Commands::Search {
            query,
            exchange,
            limit,
        } => {
            let t0 = Instant::now();
            let hits = service
                .search_symbols(&query, exchange.as_deref(), limit)
                .await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else {
                println!(
                    "Found {} symbol(s) for '{}' (exchange={}) in {} ms:",
                    hits.len(),
                    query,
                    exchange.as_deref().unwrap_or("*"),
                    t0.elapsed().as_millis()
                );
                println!(
                    "{:<22} {:<10} {:<10} {:<8} DESCRIPTION",
                    "TICKER", "TYPE", "CURRENCY", "COUNTRY"
                );
                for h in hits {
                    println!(
                        "{:<22} {:<10} {:<10} {:<8} {}",
                        h.ticker, h.market_type, h.currency_code, h.country_code, h.description
                    );
                }
            }
        }

        Commands::Resolve { symbol } => {
            let key = SymbolKey::parse(&symbol)?;
            let t0 = Instant::now();
            let info = service.resolve_symbol(&key).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!(
                    "Resolved {} in {} ms:",
                    info.ticker,
                    t0.elapsed().as_millis()
                );
                println!("  Description:       {}", info.description);
                println!(
                    "  Exchange / Listed: {} / {}",
                    info.exchange, info.listed_exchange
                );
                println!("  Market Type:       {}", info.market_type);
                println!("  Currency / TZ:     {} / {}", info.currency, info.timezone);
                println!("  Session:           {}", info.session);
                println!(
                    "  Precision (P / V): {} / {}",
                    info.price_precision, info.volume_precision
                );
                println!(
                    "  Scale / MinMov:    {} / {} (minmove2={}, tick={:?})",
                    info.pricescale, info.minmov, info.minmove2, info.tick_size
                );
                if !info.variable_tick_size.is_empty() {
                    println!("  Variable Ticks:    {}", info.variable_tick_size);
                }
                if let Some(band) = &info.vn_price_band {
                    println!("  VN Price Band:     {} ({})", band.label, band.venue_label);
                }
                println!(
                    "  PointValue / Delay: {} / {}s (ISIN={})",
                    info.point_value, info.delay_secs, info.isin
                );
            }
        }

        Commands::Bars {
            symbol,
            interval,
            limit,
            adjustment,
            before,
            after,
        } => {
            let sym_key = SymbolKey::parse(&symbol)?;
            let iv: Interval = interval.parse()?;
            let adj: Adjustment = adjustment.parse()?;
            let series_key = SeriesKey::new(&sym_key, iv, adj);
            let mode = if let Some(b) = before {
                BarQueryMode::Forward { before_ms: b }
            } else if let Some(a) = after {
                BarQueryMode::Backward { after_ms: a }
            } else {
                BarQueryMode::Init
            };

            let t0 = Instant::now();
            let page = service
                .get_bars(&BarsRequest {
                    key: series_key.clone(),
                    mode,
                    limit,
                })
                .await?;
            let elapsed_ms = t0.elapsed().as_secs_f64() * 1_000.0;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&page)?);
            } else {
                println!(
                    "{} | bars={} | source={:?} | has_older={} | has_newer={} | {:.2} ms",
                    series_key,
                    page.bars.len(),
                    page.source,
                    page.has_older,
                    page.has_newer,
                    elapsed_ms
                );
                if let (Some(first), Some(last)) = (page.bars.first(), page.bars.last()) {
                    println!(
                        "  Range: {} ({}) -> {} ({})",
                        format_ms_utc(first.timestamp),
                        first.timestamp,
                        format_ms_utc(last.timestamp),
                        last.timestamp
                    );
                }
                let preview_start = page.bars.len().saturating_sub(5);
                println!(
                    "  Last {} bar(s):",
                    page.bars.len().saturating_sub(preview_start)
                );
                for b in &page.bars[preview_start..] {
                    println!(
                        "    {} | O={:.2} H={:.2} L={:.2} C={:.2} V={:.2} Turnover={:.2}",
                        format_ms_utc(b.timestamp),
                        b.open,
                        b.high,
                        b.low,
                        b.close,
                        b.volume,
                        b.turnover
                    );
                }
            }
        }

        Commands::Stream {
            symbol,
            interval,
            adjustment,
            quotes,
            duration_secs,
            max_events,
            no_archive,
        } => {
            let archive = !no_archive;
            let sym_key = SymbolKey::parse(&symbol)?;
            let deadline =
                duration_secs.map(|s| tokio::time::Instant::now() + Duration::from_secs(s));
            let mut count = 0_usize;

            if quotes {
                let mut sub = service.subscribe_quotes(vec![sym_key.clone()]).await?;
                println!("Streaming live quotes for {} (Ctrl+C to stop)...", sym_key);
                loop {
                    if max_events.is_some_and(|max| count >= max) {
                        break;
                    }
                    let next_event = async {
                        if let Some(dl) = deadline {
                            tokio::time::timeout_at(dl, sub.recv()).await.ok().flatten()
                        } else {
                            sub.recv().await
                        }
                    };
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => break,
                        ev = next_event => {
                            let Some(q) = ev else { break };
                            count += 1;
                            if cli.json {
                                println!("{}", serde_json::to_string(&q)?);
                            } else {
                                println!(
                                    "[#{count}] {} @ {} | lp={:?} bid={:?} ask={:?} ch={:?} ({:?}%) vol={:?}",
                                    q.ticker,
                                    format_ms_utc(q.timestamp_ms),
                                    q.last_price,
                                    q.bid,
                                    q.ask,
                                    q.change,
                                    q.change_percent,
                                    q.volume
                                );
                            }
                        }
                    }
                }
                sub.unsubscribe().await;
            } else {
                let iv: Interval = interval.parse()?;
                let adj: Adjustment = adjustment.parse()?;
                let series_key = SeriesKey::new(&sym_key, iv, adj);
                let (mut sub, archiver) = if archive {
                    let (s, h) = service
                        .subscribe_bars_with_archival(series_key.clone())
                        .await?;
                    (s, Some(h))
                } else {
                    (
                        service
                            .provider()
                            .subscribe_bars(series_key.clone())
                            .await?,
                        None,
                    )
                };

                println!(
                    "Streaming live bars for {} (archive={archive}, Ctrl+C to stop)...",
                    series_key
                );
                loop {
                    if max_events.is_some_and(|max| count >= max) {
                        break;
                    }
                    let next_event = async {
                        if let Some(dl) = deadline {
                            tokio::time::timeout_at(dl, sub.recv()).await.ok().flatten()
                        } else {
                            sub.recv().await
                        }
                    };
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => break,
                        ev = next_event => {
                            let Some(bar_ev) = ev else { break };
                            count += 1;
                            if cli.json {
                                println!("{}", serde_json::to_string(&bar_ev)?);
                            } else {
                                let b = &bar_ev.bar;
                                println!(
                                    "[#{count}] {} ({}) | {} | O={:.2} H={:.2} L={:.2} C={:.2} V={:.2} T={:.2}",
                                    bar_ev.series,
                                    if bar_ev.is_update { "update" } else { "snapshot" },
                                    format_ms_utc(b.timestamp),
                                    b.open,
                                    b.high,
                                    b.low,
                                    b.close,
                                    b.volume,
                                    b.turnover
                                );
                            }
                        }
                    }
                }
                sub.unsubscribe().await;
                if let Some(h) = archiver {
                    let _ = tokio::time::timeout(Duration::from_secs(2), h).await;
                }
            }
            println!("Stream closed cleanly after {count} event(s).");
        }
    }

    Ok(())
}
