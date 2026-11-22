use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sumi_core::model::Candle;
use sumi_core::replay::ReplayController;
use sumi_core::trading::{AccountConfig, TradingSession};
use sumi_data::{
    CachePolicy, CafeFImportConfig, CafeFImporter, Interval, MarketDataService, OhlcvBar,
    SqliteMarketStore, TradingViewConfig, TradingViewFeed,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::info;

use crate::error::DesktopResult;
use crate::workspace_db::WorkspaceStore;

pub struct ActiveBarStream {
    pub cancel_tx: tokio::sync::oneshot::Sender<()>,
    pub forwarder: JoinHandle<()>,
}

pub struct ActiveQuoteStream {
    pub cancel_tx: tokio::sync::oneshot::Sender<()>,
    pub forwarder: JoinHandle<()>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedBrokerSnapshot {
    pub ticker: String,
    pub interval: String,
    pub config: AccountConfig,
    pub session_start_base_index: usize,
    pub commands: Vec<sumi_core::trading::RecordedCommand>,
    pub order_types_meta: HashMap<u64, String>,
}

/// Active causal Bar Replay + linked Paper Broker session (`sumi-core` `ReplayController` + `TradingSession`).
pub struct ReplayRuntime {
    pub ticker: String,
    pub interval: Interval,
    pub base_bars: Vec<OhlcvBar>,
    pub base_candles: Vec<Candle>,
    pub controller: ReplayController,
    pub speed_multiplier: f64,
}

/// Active Paper Broker session (either live chart or synchronized with Bar Replay).
pub struct PaperBrokerRuntime {
    pub ticker: String,
    pub interval: Interval,
    pub base_bars: Vec<OhlcvBar>,
    pub base_candles: Vec<Candle>,
    pub cursor_index: usize,
    pub session: TradingSession,
    /// Maps order_id -> VN order type label (`"LO"`, `"MP"`, `"ATO"`, `"ATC"`).
    pub order_types_meta: HashMap<u64, String>,
}

impl Default for PaperBrokerRuntime {
    fn default() -> Self {
        let cfg = AccountConfig {
            initial_cash: 500_000_000.0,
            commission_bps: 15.0,
            tax_sell_bps: 10.0,
            slippage_bps: 2.0,
            allow_short: false,
            enforce_vn_t2_lot_lock: true,
            venue: sumi_core::journal::ExchangeVenue::Hose,
            ..AccountConfig::default()
        };
        Self {
            ticker: "HOSE:FPT".to_owned(),
            interval: Interval::D1,
            base_bars: Vec::new(),
            base_candles: Vec::new(),
            cursor_index: 0,
            session: TradingSession::new(cfg),
            order_types_meta: HashMap::new(),
        }
    }
}

pub struct AppState {
    pub market_service: MarketDataService<TradingViewFeed>,
    pub market_store: SqliteMarketStore,
    pub workspace: WorkspaceStore,
    pub offline_mode: AtomicBool,
    pub bar_streams: Mutex<HashMap<String, ActiveBarStream>>,
    pub quote_stream: Mutex<Option<ActiveQuoteStream>>,
    pub replay: Mutex<Option<ReplayRuntime>>,
    pub broker: Mutex<PaperBrokerRuntime>,
}

impl AppState {
    pub fn initialize(market_db_path: PathBuf, workspace_db_path: PathBuf) -> DesktopResult<Self> {
        let market_store = SqliteMarketStore::open(&market_db_path)?;
        maybe_auto_seed_cafef(&market_store);
        let workspace = WorkspaceStore::open(&workspace_db_path)?;
        let tv_feed = Arc::new(TradingViewFeed::new(TradingViewConfig::default()));
        let policy = CachePolicy::default();
        let market_service = MarketDataService::new(tv_feed, market_store.clone(), policy);

        Ok(Self {
            market_service,
            market_store,
            workspace,
            offline_mode: AtomicBool::new(false),
            bar_streams: Mutex::new(HashMap::new()),
            quote_stream: Mutex::new(None),
            replay: Mutex::new(None),
            broker: Mutex::new(PaperBrokerRuntime::default()),
        })
    }

    pub fn active_market_service(&self) -> MarketDataService<TradingViewFeed> {
        let offline = self.offline_mode.load(Ordering::Relaxed);
        self.market_service.with_offline_mode(offline)
    }

    pub fn set_offline(&self, offline: bool) {
        self.offline_mode.store(offline, Ordering::Relaxed);
    }

    pub fn is_offline(&self) -> bool {
        self.offline_mode.load(Ordering::Relaxed)
    }
}

/// Collects candidate workspace root directories from both `current_dir()` and `current_exe()`
/// so double-clicking `dist/Sumi Supercharts.app` in macOS Finder (where `cwd` is `/`) or
/// `dist\SumiSupercharts-win64\sumi-desktop.exe` in Windows Explorer still locates `data/cache`
/// and `data/raw/cafef` inside the repository.
pub fn workspace_ancestor_dirs() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        for anc in cwd.ancestors().take(4) {
            let p = anc.to_path_buf();
            if !roots.contains(&p) {
                roots.push(p);
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        for anc in exe.ancestors().skip(1).take(7) {
            let p = anc.to_path_buf();
            if !roots.contains(&p) {
                roots.push(p);
            }
        }
    }
    roots
}

/// Resolves default local paths for `sumi_market.db` and `sumi_workspace.db`.
/// Prefers the workspace's seeded `data/cache/sumi_market.db` during dev/repo execution,
/// falling back to the OS app data directory when packaged outside the repo.
pub fn resolve_database_paths(app_data_dir: Option<PathBuf>) -> (PathBuf, PathBuf) {
    for root in workspace_ancestor_dirs() {
        let cand = root.join("data/cache");
        let market_candidate = cand.join("sumi_market.db");
        if market_candidate.exists() {
            return (market_candidate, cand.join("sumi_workspace.db"));
        }
    }

    let base = app_data_dir.unwrap_or_else(|| PathBuf::from("data/cache"));
    (base.join("sumi_market.db"), base.join("sumi_workspace.db"))
}

/// On a fresh install (`total_symbols == 0`), automatically imports local CafeF CSV archives
/// if `data/raw/cafef` or `work/CafeFData` is present in the workspace hierarchy.
fn maybe_auto_seed_cafef(market_store: &SqliteMarketStore) {
    let is_empty = market_store
        .stats()
        .map(|s| s.total_symbols == 0)
        .unwrap_or(false);
    if !is_empty {
        return;
    }

    for root in workspace_ancestor_dirs() {
        for rel in ["data/raw/cafef", "work/CafeFData"] {
            let candidate = root.join(rel);
            let has_csv = std::fs::read_dir(&candidate)
                .ok()
                .map(|entries| {
                    entries.flatten().any(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
                    })
                })
                .unwrap_or(false);
            if has_csv {
                info!(dir = %candidate.display(), "cold-start SQLite empty; auto-seeding CafeF CSV directory");
                let importer = CafeFImporter::new(CafeFImportConfig::default());
                if let Ok(summary) = importer.import_dir(&candidate, market_store) {
                    info!(
                        symbols = summary.distinct_symbols,
                        bars = summary.total_bars_written,
                        "completed automatic cold-start CafeF seed"
                    );
                }
                return;
            }
        }
    }
}
