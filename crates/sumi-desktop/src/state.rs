use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sumi_core::indicators::AnalogMemoryBank;
use sumi_core::model::Candle;
use sumi_core::replay::ReplayController;
use sumi_core::trading::{AccountConfig, TradingSession};
use sumi_data::{
    CachePolicy, CafeFImportConfig, CafeFImporter, Interval, MarketDataService, OhlcvBar,
    SqliteMarketStore, SqliteMigrationReceipt, TradingViewConfig, TradingViewFeed,
    legacy_sumi_app_data_dirs, sqlite_backup_atomic, sumi_config_dir, sumi_data_dir,
};
use tokio::sync::{Mutex, broadcast};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::commands::market::LiveBarPayload;
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

/// Metadata for a True Blind Replay Drill (random historical symbol, normalized to `100.00` at cut bar).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlindDrillMeta {
    pub enabled: bool,
    pub alias_code: String,
    pub price_scale: f64,
    pub revealed: bool,
    pub real_ticker: String,
    pub cut_date: String,
}

impl BlindDrillMeta {
    /// Returns a spoiler-free copy with `real_ticker` and `cut_date` masked while `!revealed`.
    #[must_use]
    pub fn masked_for_ui(&self) -> Self {
        if self.revealed {
            self.clone()
        } else {
            Self {
                enabled: self.enabled,
                alias_code: self.alias_code.clone(),
                price_scale: self.price_scale,
                revealed: false,
                real_ticker: String::new(),
                cut_date: String::new(),
            }
        }
    }
}

/// Active causal Multi-Timeframe Bar Replay session (`sumi-core` `ReplayController`).
pub struct ReplayRuntime {
    pub ticker: String,
    /// Finest timeframe driving causal replay stepping (e.g. `1h` when observing `1h` + `4h`).
    pub interval: Interval,
    /// All active timeframes tracked for synchronized multi-pane MTF stepping (e.g. `["1h", "4h", "1D"]`).
    pub tracked_intervals: Vec<Interval>,
    pub base_bars: Vec<OhlcvBar>,
    pub base_candles: Vec<Candle>,
    pub controller: ReplayController,
    pub speed_multiplier: f64,
    pub blind_drill: Option<BlindDrillMeta>,
}

/// Active Paper Broker session (either live chart or synchronized with Bar Replay).
pub struct PaperBrokerRuntime {
    pub ticker: String,
    pub interval: Interval,
    pub base_bars: Vec<OhlcvBar>,
    pub base_candles: Vec<Candle>,
    pub cursor_index: usize,
    pub session: TradingSession,
    /// Maps order_id -> VN order type label (`"LO"`, `"MP"`, `"ATO"`, `"ATC"`, `"STOP"`).
    pub order_types_meta: HashMap<u64, String>,
}

impl Default for PaperBrokerRuntime {
    fn default() -> Self {
        let cfg = AccountConfig {
            initial_cash: 500_000_000.0,
            commission_bps: 15.0,
            tax_sell_bps: 10.0,
            slippage_bps: 2.0,
            apply_slippage_to_limits: false,
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

pub struct CachedAnalogMemoryBank {
    pub bank: Arc<AnalogMemoryBank>,
    pub built_epoch: u64,
    pub built_total_bars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V3MigrationBundleReceipt {
    pub schema_version: String,
    pub market_db: Option<SqliteMigrationReceipt>,
    pub workspace_db: Option<SqliteMigrationReceipt>,
}

pub struct AppState {
    pub market_service: MarketDataService<TradingViewFeed>,
    pub market_store: SqliteMarketStore,
    pub workspace: WorkspaceStore,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub offline_mode: AtomicBool,
    pub alert_seq: AtomicU64,
    pub bar_updates_tx: broadcast::Sender<LiveBarPayload>,
    pub bar_streams: Mutex<HashMap<String, ActiveBarStream>>,
    pub bar_stream_generations: Mutex<HashMap<String, u64>>,
    pub quote_stream: Mutex<Option<ActiveQuoteStream>>,
    pub replay: Mutex<Option<ReplayRuntime>>,
    /// Live Paper Broker (persisted per `(ticker, interval)` in SQLite — never overwritten by Replay!).
    pub broker: Mutex<PaperBrokerRuntime>,
    /// Isolated sandbox Paper Broker used strictly while Bar Replay is active (`BUG-006`).
    pub replay_broker: Mutex<PaperBrokerRuntime>,
    /// Lazy in-RAM cross-market `AnalogMemoryBank` cache (`AFC — Analog Forecast Cone`).
    pub analog_bank: std::sync::RwLock<Option<CachedAnalogMemoryBank>>,
    /// Serializes cold-start `AnalogMemoryBank` builds so concurrent chart panes share one build.
    pub analog_build_lock: Mutex<()>,
    /// Monotonic invalidation counter bumped whenever CafeF archives are imported.
    pub analog_cache_epoch: AtomicU64,
}

impl AppState {
    pub fn initialize(market_db_path: PathBuf, workspace_db_path: PathBuf) -> DesktopResult<Self> {
        let data_dir = market_db_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(sumi_data_dir);
        let config_dir = sumi_config_dir();
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(&config_dir);

        let market_store = SqliteMarketStore::open(&market_db_path)?;
        maybe_auto_seed_cafef(&market_store);
        let workspace = WorkspaceStore::open(&workspace_db_path)?;
        let tv_feed = Arc::new(TradingViewFeed::new(TradingViewConfig::default()));
        let policy = CachePolicy::default();
        let market_service = MarketDataService::new(tv_feed, market_store.clone(), policy);
        let (bar_updates_tx, _) = broadcast::channel(512);

        Ok(Self {
            market_service,
            market_store,
            workspace,
            data_dir,
            config_dir,
            offline_mode: AtomicBool::new(false),
            alert_seq: AtomicU64::new(1),
            bar_updates_tx,
            bar_streams: Mutex::new(HashMap::new()),
            bar_stream_generations: Mutex::new(HashMap::new()),
            quote_stream: Mutex::new(None),
            replay: Mutex::new(None),
            broker: Mutex::new(PaperBrokerRuntime::default()),
            replay_broker: Mutex::new(PaperBrokerRuntime::default()),
            analog_bank: std::sync::RwLock::new(None),
            analog_build_lock: Mutex::new(()),
            analog_cache_epoch: AtomicU64::new(1),
        })
    }

    /// Invalidates the cached `AnalogMemoryBank` so the next `compute_analog_cone` call rebuilds
    /// cleanly from the updated `SqliteMarketStore` (e.g. after `import_cafef_dir`).
    pub fn invalidate_analog_memory_bank(&self) {
        self.analog_cache_epoch.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut guard) = self.analog_bank.write() {
            *guard = None;
        }
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

/// Collects candidate local ancestor directories from `current_dir()` and `current_exe()` for
/// optional cold-start seeding (never overrides canonical OS data directories).
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

/// Resolves canonical cross-platform paths for `sumi_market.db` and `sumi_workspace.db`
/// (`~/.local/share/sumi/` on macOS/Linux, `%LOCALAPPDATA%\sumi\` on Windows, or `SUMI_DATA_DIR`).
///
/// On first v3.0 launch, automatically and non-destructively migrates any existing v2.x databases
/// from legacy OS app support folders (`~/Library/Application Support/com.sumi.supercharts/`,
/// `%APPDATA%\com.sumi.supercharts\`, or the passed `legacy_app_data_dir`) using SQLite's WAL-safe
/// Online Backup API with SHA-256 verification. Never deletes legacy files.
pub fn resolve_database_paths(legacy_app_data_dir: Option<PathBuf>) -> (PathBuf, PathBuf) {
    let target_dir = sumi_data_dir();
    let _ = migrate_legacy_databases_into(&target_dir, legacy_app_data_dir.as_deref());
    (
        target_dir.join("sumi_market.db"),
        target_dir.join("sumi_workspace.db"),
    )
}

/// Performs non-destructive WAL-safe SQLite migration from legacy v2.x directories into `target_dir`.
pub fn migrate_legacy_databases_into(
    target_dir: &Path,
    extra_legacy_dir: Option<&Path>,
) -> Option<V3MigrationBundleReceipt> {
    let target_market = target_dir.join("sumi_market.db");
    let target_workspace = target_dir.join("sumi_workspace.db");
    if target_market.is_file() && target_workspace.is_file() {
        return None;
    }

    let mut legacy_candidates = Vec::new();
    if let Some(extra) = extra_legacy_dir
        && extra != target_dir
    {
        legacy_candidates.push(extra.to_path_buf());
    }
    for dir in legacy_sumi_app_data_dirs() {
        if dir != target_dir && !legacy_candidates.contains(&dir) {
            legacy_candidates.push(dir);
        }
    }

    let mut market_receipt = None;
    let mut workspace_receipt = None;

    if !target_market.is_file() {
        for cand_dir in &legacy_candidates {
            let src = cand_dir.join("sumi_market.db");
            if src.is_file() {
                match sqlite_backup_atomic(&src, &target_market) {
                    Ok(rec) => {
                        info!(
                            src = %src.display(),
                            dst = %target_market.display(),
                            bytes = rec.target_bytes,
                            sha256 = %rec.target_sha256,
                            "migrated legacy sumi_market.db non-destructively to canonical SUMI_DATA_DIR"
                        );
                        market_receipt = Some(rec);
                        break;
                    }
                    Err(err) => {
                        warn!(src = %src.display(), error = %err, "legacy market DB migration attempt failed");
                    }
                }
            }
        }
    }

    if !target_workspace.is_file() {
        for cand_dir in &legacy_candidates {
            let src = cand_dir.join("sumi_workspace.db");
            if src.is_file() {
                match sqlite_backup_atomic(&src, &target_workspace) {
                    Ok(rec) => {
                        info!(
                            src = %src.display(),
                            dst = %target_workspace.display(),
                            bytes = rec.target_bytes,
                            sha256 = %rec.target_sha256,
                            "migrated legacy sumi_workspace.db non-destructively to canonical SUMI_DATA_DIR"
                        );
                        workspace_receipt = Some(rec);
                        break;
                    }
                    Err(err) => {
                        warn!(src = %src.display(), error = %err, "legacy workspace DB migration attempt failed");
                    }
                }
            }
        }
    }

    if market_receipt.is_some() || workspace_receipt.is_some() {
        let bundle = V3MigrationBundleReceipt {
            schema_version: "sumi_v3_migration_receipt_v1".to_owned(),
            market_db: market_receipt,
            workspace_db: workspace_receipt,
        };
        let receipt_path = target_dir.join(".migration_v3_receipt.json");
        if let Ok(json) = serde_json::to_vec_pretty(&bundle) {
            let _ = std::fs::write(&receipt_path, json);
        }
        return Some(bundle);
    }

    None
}

/// Collects candidate cold-start seed roots strictly from explicit env overrides (`SUMI_SEED_DIR`),
/// packaged bundle `Resources/`, or active working tree (0% hardcoded personal user paths).
fn seed_candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut push_unique = |p: PathBuf| {
        if !roots.contains(&p) {
            roots.push(p);
        }
    };

    if let Ok(explicit) = std::env::var("SUMI_SEED_DIR")
        && !explicit.trim().is_empty()
    {
        push_unique(PathBuf::from(explicit.trim()));
    }

    // macOS bundle Resources directory: `<App>.app/Contents/MacOS/sumi-desktop` -> `../Resources`
    // Windows portable directory: `<Portable>/sumi-desktop.exe` -> `<Portable>/seed`
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        push_unique(exe_dir.to_path_buf());
        push_unique(exe_dir.join("seed"));
        push_unique(exe_dir.join("data"));
        if let Some(contents_dir) = exe_dir.parent() {
            push_unique(contents_dir.join("Resources"));
            push_unique(contents_dir.join("Resources").join("data"));
        }
    }

    for anc in workspace_ancestor_dirs() {
        push_unique(anc);
    }

    roots
}

/// On a truly empty Clean-Slate install (`total_symbols < 100`), optionally hydrates from a
/// co-located seed SQLite DB or CafeF CSV directory if present (never required on Clean-Slate
/// Windows PCs, which hydrate directly via Yumeko Read-Only + TradingView Catalog/EOD sync!).
fn maybe_auto_seed_cafef(market_store: &SqliteMarketStore) {
    let symbol_count = market_store.stats().map(|s| s.total_symbols).unwrap_or(0);
    if symbol_count >= 100 {
        return;
    }

    let roots = seed_candidate_roots();

    // 1. Prefer merging an already-seeded SQLite market DB (instant ~300ms for 473 symbols / 950k bars).
    for root in &roots {
        for rel in ["data/cache/sumi_market.db", "sumi_market.db"] {
            let candidate = root.join(rel);
            if !candidate.exists() || candidate == market_store.path() {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&candidate) else {
                continue;
            };
            if meta.len() < 1_000_000 {
                continue;
            }
            match market_store.merge_from_sqlite_path(&candidate) {
                Ok(inserted) if inserted > 0 => {
                    info!(
                        seed = %candidate.display(),
                        inserted_bars = inserted,
                        "auto-merged seeded SQLite market history into canonical SUMI_DATA_DIR database"
                    );
                    return;
                }
                Ok(_) => {}
                Err(err) => {
                    warn!(
                        seed = %candidate.display(),
                        error = %err,
                        "failed to merge seeded SQLite database; trying next candidate"
                    );
                }
            }
        }
    }

    // 2. Fallback: import CafeF CSV archives directly if present.
    for root in &roots {
        for rel in ["data/raw/cafef", "cafef"] {
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
                info!(dir = %candidate.display(), "cold-start SQLite sparse; auto-seeding CafeF CSV directory");
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
