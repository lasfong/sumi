//! Strictly Read-Only SQLite & Research Export reader for upstream **Yumeko v3.0**.
//!
//! Hard Guards enforced unconditionally:
//! 1. **Read-Only Isolation:** Always opens `yumeko-app.sqlite` (and optional `qmv-shared.sqlite`)
//!    with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` + `PRAGMA query_only = ON`. Never mutates
//!    any Yumeko table, lease, or secret.
//! 2. **F01 Portfolio vs Watchlist Separation:** Real holdings (`personal_portfolio_snapshots`) are
//!    strictly separated from `yr_personal_watchlist` (`★`) and never conflated.
//! 3. **F02 Provenance & PIT Mode:** Verifies `raw_sha256` on `dnse_mcp_observations` and flags
//!    `HistoricalMode::PointInTime` vs `HistoricalMode::Retrospective`.
//! 4. **kVND <-> VND Scale Guard:** All QMV/DNSE equity prices (`StopLoss`, `TargetPrice1/2`, `Close`,
//!    DNSE OHLCV) are explicitly normalized to Sumi's chart unit while indices & VN30F stay `1.0x`.
//! 5. **Clean-Slate Graceful Degradation:** When `yumeko-app.sqlite` is absent on a fresh Windows/Mac
//!    installation, returns `available = false` with zero errors.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use sumi_data::{OhlcvBar, yumeko_data_dir};

use crate::contract::{
    Basis, BridgeResult, CandleBarV1, CandleSeriesV1, HistoricalMode, QuantityUnit,
    SUMI_CONTRACT_VERSION_V1, TimeframeV1, candle_series_v1_to_sumi_bars, canonical_sha256_value,
    normalize_price_to_sumi_vnd, parse_provider_price_unit, sha256_str_hex, strip_exchange_prefix,
};

// ============================================================================
// Bridge Configuration & Status DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YumekoBridgeStatus {
    pub available: bool,
    pub sqlite_path: String,
    pub qmv_shared_path: Option<String>,
    pub latest_as_of_dates: Vec<String>,
    pub qmv_technical_symbols: usize,
    pub qmv_active_money_symbols: usize,
    pub qmv_foreign_flow_symbols: usize,
    pub valuation_symbols: usize,
    pub dnse_history_symbols: usize,
    pub macro_observations_count: usize,
    pub portfolio_snapshots_count: usize,
    pub watchlist_symbols_count: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroObservationDto {
    pub indicator_code: String,
    pub period_date: String,
    pub release_date: String,
    pub value: Option<f64>,
    pub value_text: Option<String>,
    pub unit: String,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorBreadthSnapshotDto {
    pub icb2_code: String,
    pub icb2_name: String,
    pub change_pct: Option<f64>,
    pub advancers: Option<i64>,
    pub decliners: Option<i64>,
    pub unchanged: Option<i64>,
    pub total_stocks: Option<i64>,
    pub advancing_ratio_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vm01MarketContextDto {
    pub effective_as_of: Option<String>,
    pub market_sentiment: Option<f64>,
    pub market_bbm_05: Option<f64>,
    pub market_bbm_20: Option<f64>,
    pub market_bbmz_05: Option<f64>,
    pub market_bbmz_20: Option<f64>,
    pub market_foreign_net_value: Option<f64>,
    pub top_sectors: Vec<SectorBreadthSnapshotDto>,
    pub macro_highlights: Vec<MacroObservationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QmvStockBlackboxSnapshotDto {
    pub content_date: String,
    /// Always `"qmv_real"` for authentic decoded rows from Yumeko `qmv_compact_rows`.
    pub source_provenance: String,
    pub is_provider_proprietary: bool,
    pub close_vnd: Option<f64>,
    pub close_kvnd: Option<f64>,
    pub tmc: Option<f64>,
    pub tma20: Option<f64>,
    pub tma50: Option<f64>,
    pub dspi: Option<f64>,
    pub dspi_ma5: Option<f64>,
    pub dspi_ma20: Option<f64>,
    pub mpicp_all: Option<f64>,
    pub mpicp_z: Option<f64>,
    pub dm: Option<f64>,
    pub ds: Option<f64>,
    pub trading_score: Option<f64>,
    pub price_trend: Option<String>,
    pub stop_loss_vnd: Option<f64>,
    pub target_price_1_vnd: Option<f64>,
    pub target_price_2_vnd: Option<f64>,
    pub money_in_today: bool,
    pub money_in_2_sessions: bool,
    pub money_in_3_sessions: bool,
    pub reversal_up_t_plus: bool,
    pub reversal_up_swing: bool,
    pub active_flags: Vec<String>,
    pub qmv_rating: Option<String>,
    pub trading_suitability: Option<String>,
    pub division_allocation_group: Option<String>,
    pub division_money_flow_label: Option<String>,
    pub in_qmv60: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuationQuarterDto {
    pub period: String,
    pub content_date: Option<String>,
    pub source_family: String,
    pub pe: Option<f64>,
    pub pb: Option<f64>,
    pub ps: Option<f64>,
    pub ev_ebitda: Option<f64>,
    pub eps: Option<f64>,
    pub bvps: Option<f64>,
    pub roe: Option<f64>,
    pub roa: Option<f64>,
    pub debt_to_equity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dm01HoldingDto {
    pub as_of: String,
    pub effective_as_of: String,
    pub sha256_verified: bool,
    pub quantity: f64,
    pub avg_cost_vnd: f64,
    pub current_price_vnd: f64,
    pub current_value_vnd: f64,
    pub unrealized_pnl_vnd: f64,
    pub unrealized_pnl_pct: f64,
    pub weight_pct: f64,
    pub days_held: Option<i64>,
    pub sector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dm01WatchlistEntryDto {
    pub symbol: String,
    pub strategy_tag: String,
    pub note: String,
    pub target_buy_vnd: Option<f64>,
    pub invalidation_level_vnd: Option<f64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodicThesisDto {
    pub decision_id: String,
    pub as_of: String,
    pub function_name: String,
    pub gate_step: Option<String>,
    pub stance: String,
    pub confidence: String,
    pub thesis_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopDownSymbolContextDto {
    pub symbol: String,
    pub requested_as_of: Option<String>,
    pub effective_as_of: Option<String>,
    pub bridge_available: bool,
    /// `"qmv_real"` when authentic QMV Blackbox rows exist for this symbol, `"local_proxy"` otherwise.
    pub flow_provenance: String,
    pub vm01_market: Vm01MarketContextDto,
    pub hp01_blackbox: Option<QmvStockBlackboxSnapshotDto>,
    pub dn01_valuations: Vec<ValuationQuarterDto>,
    pub dm01_real_holding: Option<Dm01HoldingDto>,
    pub dm01_yumeko_watchlist: Option<Dm01WatchlistEntryDto>,
    pub episodic_theses: Vec<EpisodicThesisDto>,
    pub dnse_pit_available: bool,
    pub gaps: Vec<String>,
}

// ============================================================================
// Chart Time-Series DTOs for Real QMV Indicators (`QMV_STOCK_BB`, `ACTIVE_MONEY`, `FOREIGN_FLOW`)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QmvStockBbPointDto {
    pub date: String,
    pub timestamp_ms: i64,
    pub close_vnd: Option<f64>,
    pub tmc: Option<f64>,
    pub tma20: Option<f64>,
    pub tma50: Option<f64>,
    pub dspi: Option<f64>,
    pub dspi_ma5: Option<f64>,
    pub dspi_ma20: Option<f64>,
    pub mpicp_all: Option<f64>,
    pub mpicp_z: Option<f64>,
    pub dm: Option<f64>,
    pub ds: Option<f64>,
    pub stop_loss_vnd: Option<f64>,
    pub tp1_vnd: Option<f64>,
    pub tp2_vnd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QmvActiveMoneyPointDto {
    pub date: String,
    pub timestamp_ms: i64,
    pub price_vnd: Option<f64>,
    pub shark: Option<f64>,
    pub wolf: Option<f64>,
    pub sheep: Option<f64>,
    pub trend_shark: Option<f64>,
    pub trend_wolf: Option<f64>,
    pub trend_sheep: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QmvForeignFlowPointDto {
    pub date: String,
    pub timestamp_ms: i64,
    pub price_vnd: Option<f64>,
    pub buy_value: Option<f64>,
    pub sell_value: Option<f64>,
    pub net_value: Option<f64>,
    pub buy_volume: Option<f64>,
    pub sell_volume: Option<f64>,
    pub net_volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QmvRealChartSeriesDto {
    pub symbol: String,
    pub as_of: Option<String>,
    /// `"qmv_real"` when >= 1 authentic series exists, `"local_proxy"` otherwise.
    pub source_provenance: String,
    pub is_provider_proprietary: bool,
    pub stock_bb: Vec<QmvStockBbPointDto>,
    pub active_money: Vec<QmvActiveMoneyPointDto>,
    pub foreign_flow: Vec<QmvForeignFlowPointDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnseLoadedSeries {
    pub contract_series: CandleSeriesV1,
    pub sumi_bars: Vec<OhlcvBar>,
    pub sha256_verified: bool,
    pub historical_mode: HistoricalMode,
    pub observation_as_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchExportVerificationDto {
    pub export_dir: String,
    pub manifest_found: bool,
    pub verified_files: usize,
    pub failed_files: Vec<String>,
    pub as_of: Option<String>,
    pub valid: bool,
}

// ============================================================================
// Internal Compact Schema Wire Decoder (`qmv-compact-domain-v1`)
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct CompactSchemaWire {
    fields: Vec<String>,
}

struct DecodedRow {
    content_date: Option<String>,
    entity: Option<String>,
    values: BTreeMap<String, Value>,
}

/// Strictly Read-Only handle to Yumeko's local SQLite research warehouse.
#[derive(Debug, Clone)]
pub struct YumekoReadOnlyBridge {
    sqlite_path: PathBuf,
    qmv_shared_path: Option<PathBuf>,
}

impl YumekoReadOnlyBridge {
    /// Creates a bridge pointing at explicit paths (used in tests or `YUMEKO_SQLITE_PATH` overrides).
    #[must_use]
    pub fn new(sqlite_path: PathBuf, qmv_shared_path: Option<PathBuf>) -> Self {
        Self {
            sqlite_path,
            qmv_shared_path,
        }
    }

    /// Auto-discovers Yumeko's cross-platform `yumeko-app.sqlite` via `YUMEKO_SQLITE_PATH` or
    /// `sumi_data::yumeko_data_dir()` (`~/.local/share/yumeko` on macOS/Linux, `%LOCALAPPDATA%\yumeko\data` on Windows).
    #[must_use]
    pub fn discover_default() -> Self {
        let sqlite_path = std::env::var("YUMEKO_SQLITE_PATH")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| yumeko_data_dir().join("yumeko-app.sqlite"));
        let qmv_shared_path = std::env::var("YUMEKO_QMV_SHARED_SQLITE")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_file());
        Self {
            sqlite_path,
            qmv_shared_path,
        }
    }

    #[must_use]
    pub fn sqlite_path(&self) -> &Path {
        &self.sqlite_path
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.sqlite_path.is_file()
    }

    /// Opens `yumeko-app.sqlite` in **strict read-only mode** (`SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = ON`).
    pub fn open_readonly_conn(&self) -> BridgeResult<Option<Connection>> {
        if !self.sqlite_path.is_file() {
            return Ok(None);
        }
        let conn = Connection::open_with_flags(
            &self.sqlite_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.pragma_update(None, "query_only", "ON")?;
        let _ = conn.pragma_update(None, "busy_timeout", 3_000_i64);
        Ok(Some(conn))
    }

    /// Inspects Yumeko DB availability, latest PIT `as_of` dates, and table coverage.
    pub fn get_status(&self) -> BridgeResult<YumekoBridgeStatus> {
        let Some(conn) = self.open_readonly_conn()? else {
            return Ok(YumekoBridgeStatus {
                available: false,
                sqlite_path: self.sqlite_path.display().to_string(),
                qmv_shared_path: self
                    .qmv_shared_path
                    .as_ref()
                    .map(|p| p.display().to_string()),
                latest_as_of_dates: Vec::new(),
                qmv_technical_symbols: 0,
                qmv_active_money_symbols: 0,
                qmv_foreign_flow_symbols: 0,
                valuation_symbols: 0,
                dnse_history_symbols: 0,
                macro_observations_count: 0,
                portfolio_snapshots_count: 0,
                watchlist_symbols_count: 0,
                note: "Chưa phát hiện yumeko-app.sqlite — Sumi hoạt động độc lập (Clean-Slate mode với chỉ báo OHLCV Proxy).".to_owned(),
            });
        };

        let mut dates = BTreeSet::new();
        if table_exists(&conn, "qmv_compact_rows")? {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT content_date FROM qmv_compact_rows
                 WHERE content_date IS NOT NULL AND length(content_date) = 10
                 ORDER BY content_date DESC LIMIT 20",
            )?;
            let iter = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for d in iter.flatten() {
                dates.insert(d);
            }
        }
        if table_exists(&conn, "personal_portfolio_snapshots")? {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT effective_as_of FROM personal_portfolio_snapshots
                 WHERE effective_as_of IS NOT NULL AND length(effective_as_of) = 10
                 ORDER BY effective_as_of DESC LIMIT 10",
            )?;
            let iter = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for d in iter.flatten() {
                dates.insert(d);
            }
        }
        let mut latest_as_of_dates: Vec<String> = dates.into_iter().collect();
        latest_as_of_dates.sort_by(|a, b| b.cmp(a));
        latest_as_of_dates.truncate(20);

        let qmv_technical_symbols = count_distinct_qmv_entities(&conn, "technical_metrics")?;
        let qmv_active_money_symbols = count_distinct_qmv_entities(&conn, "active_money")?;
        let qmv_foreign_flow_symbols =
            count_distinct_qmv_entities(&conn, "symbol_foreign_flow_current")?;
        let valuation_symbols = count_query(
            &conn,
            "symbol_valuation_snapshots",
            "SELECT COUNT(DISTINCT symbol) FROM symbol_valuation_snapshots",
        )?;
        let dnse_history_symbols = count_query(
            &conn,
            "dnse_mcp_observations",
            "SELECT COUNT(DISTINCT symbol) FROM dnse_mcp_observations WHERE tool_name = 'price_get_history'",
        )?;
        let macro_observations_count = count_query(
            &conn,
            "yr_macro_observations",
            "SELECT COUNT(*) FROM yr_macro_observations",
        )?;
        let portfolio_snapshots_count = count_query(
            &conn,
            "personal_portfolio_snapshots",
            "SELECT COUNT(*) FROM personal_portfolio_snapshots",
        )?;
        let watchlist_symbols_count = count_query(
            &conn,
            "yr_personal_watchlist",
            "SELECT COUNT(DISTINCT symbol) FROM yr_personal_watchlist",
        )?;

        Ok(YumekoBridgeStatus {
            available: true,
            sqlite_path: self.sqlite_path.display().to_string(),
            qmv_shared_path: self
                .qmv_shared_path
                .as_ref()
                .map(|p| p.display().to_string()),
            latest_as_of_dates,
            qmv_technical_symbols,
            qmv_active_money_symbols,
            qmv_foreign_flow_symbols,
            valuation_symbols,
            dnse_history_symbols,
            macro_observations_count,
            portfolio_snapshots_count,
            watchlist_symbols_count,
            note: "Đã kết nối Chỉ Đọc (PRAGMA query_only = ON) với kho nghiên cứu Yumeko v3.0."
                .to_owned(),
        })
    }

    /// Loads full Top-Down context (`VM01 -> HP01 -> DN01 -> DM01`) for `ticker_or_symbol`
    /// strictly respecting optional Point-in-Time cutoff `as_of` (`YYYY-MM-DD`).
    pub fn load_topdown_context(
        &self,
        ticker_or_symbol: &str,
        as_of: Option<&str>,
    ) -> BridgeResult<TopDownSymbolContextDto> {
        let sym = strip_exchange_prefix(ticker_or_symbol);
        let as_of_clean = as_of
            .map(str::trim)
            .filter(|s| s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok())
            .map(ToOwned::to_owned);

        let Some(conn) = self.open_readonly_conn()? else {
            return Ok(TopDownSymbolContextDto {
                symbol: sym,
                requested_as_of: as_of_clean,
                effective_as_of: None,
                bridge_available: false,
                flow_provenance: "local_proxy".to_owned(),
                vm01_market: Vm01MarketContextDto {
                    effective_as_of: None,
                    market_sentiment: None,
                    market_bbm_05: None,
                    market_bbm_20: None,
                    market_bbmz_05: None,
                    market_bbmz_20: None,
                    market_foreign_net_value: None,
                    top_sectors: Vec::new(),
                    macro_highlights: Vec::new(),
                },
                hp01_blackbox: None,
                dn01_valuations: Vec::new(),
                dm01_real_holding: None,
                dm01_yumeko_watchlist: None,
                episodic_theses: Vec::new(),
                dnse_pit_available: false,
                gaps: vec![
                    "yumeko_db_absent_clean_slate: Sử dụng dữ liệu biểu đồ cục bộ & chỉ báo OHLCV Proxy."
                        .to_owned(),
                ],
            });
        };

        let schemas = load_all_compact_schemas(&conn)?;
        let as_of_ref = as_of_clean.as_deref();
        let mut gaps = Vec::new();

        // 1. VM01 Macro & Market Context
        let vm01_market = load_vm01_market_context(&conn, &schemas, as_of_ref)?;

        // 2. HP01 QMV Stock Blackbox (`qmv_real`)
        let hp01_blackbox = load_hp01_stock_blackbox(&conn, &schemas, &sym, as_of_ref)?;
        let flow_provenance = if hp01_blackbox.is_some() {
            "qmv_real".to_owned()
        } else {
            gaps.push(format!(
                "qmv_blackbox_missing_for_{sym}: Chưa có bản ghi technical_metrics trong Yumeko — tự động dùng OHLCV Proxy."
            ));
            "local_proxy".to_owned()
        };

        // 3. DN01 8-Quarter Valuation (`symbol_valuation_snapshots`)
        let dn01_valuations = load_dn01_valuations(&conn, &sym, as_of_ref)?;
        if dn01_valuations.is_empty() {
            gaps.push(format!(
                "dn01_valuation_missing_for_{sym}: Chưa có ảnh chụp định giá CSTC 8 quý trong Yumeko."
            ));
        }

        // 4. DM01 Personal Portfolio Holding vs Watchlist (Strictly separated F01!)
        let dm01_real_holding = load_dm01_real_holding(&conn, &sym, as_of_ref)?;
        let dm01_yumeko_watchlist = load_dm01_watchlist_entry(&conn, &sym)?;

        // 5. Episodic Theses (`yr_episodic_decisions`)
        let episodic_theses = load_episodic_theses(&conn, &sym, as_of_ref)?;

        // 6. Check if DNSE OHLCV PIT series is present
        let dnse_pit_available = has_dnse_price_history(&conn, &sym)?;

        let effective_as_of = hp01_blackbox
            .as_ref()
            .map(|b| b.content_date.clone())
            .or_else(|| vm01_market.effective_as_of.clone())
            .or_else(|| as_of_clean.clone());

        Ok(TopDownSymbolContextDto {
            symbol: sym,
            requested_as_of: as_of_clean,
            effective_as_of,
            bridge_available: true,
            flow_provenance,
            vm01_market,
            hp01_blackbox,
            dn01_valuations,
            dm01_real_holding,
            dm01_yumeko_watchlist,
            episodic_theses,
            dnse_pit_available,
            gaps,
        })
    }

    /// Loads authentic QMV Blackbox time series (`QMV_STOCK_BB`, `QMV_ACTIVE_MONEY`, `QMV_FOREIGN_FLOW`)
    /// for chart pane overlays (`source_provenance = "qmv_real"`, `is_provider_proprietary = true`).
    pub fn load_qmv_stock_series(
        &self,
        ticker_or_symbol: &str,
        as_of: Option<&str>,
    ) -> BridgeResult<QmvRealChartSeriesDto> {
        let sym = strip_exchange_prefix(ticker_or_symbol);
        let as_of_clean = as_of
            .map(str::trim)
            .filter(|s| s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok())
            .map(ToOwned::to_owned);

        let Some(conn) = self.open_readonly_conn()? else {
            return Ok(QmvRealChartSeriesDto {
                symbol: sym,
                as_of: as_of_clean,
                source_provenance: "local_proxy".to_owned(),
                is_provider_proprietary: false,
                stock_bb: Vec::new(),
                active_money: Vec::new(),
                foreign_flow: Vec::new(),
            });
        };

        let schemas = load_all_compact_schemas(&conn)?;
        let as_of_ref = as_of_clean.as_deref();

        // 1. QMV_STOCK_BB (`technical_metrics`)
        let tech_rows = query_decoded_qmv_rows(&conn, &schemas, "technical_metrics", Some(&sym), as_of_ref, 400)?;
        let mut stock_bb_by_date: BTreeMap<String, QmvStockBbPointDto> = BTreeMap::new();
        for row in tech_rows {
            let Some(date_str) = row.content_date.as_deref().and_then(normalize_iso_date) else {
                continue;
            };
            let Some(ts_ms) = iso_date_to_utc_ms(&date_str) else {
                continue;
            };
            let close_kvnd = val_f64(&row.values, "Close");
            let sl_kvnd = val_f64(&row.values, "StopLoss");
            let tp1_kvnd = val_f64(&row.values, "TargetPrice1");
            let tp2_kvnd = val_f64(&row.values, "TargetPrice2");
            let unit = if crate::contract::is_index_or_futures_symbol(&sym) {
                QuantityUnit::Points
            } else {
                QuantityUnit::ThousandVnd
            };
            stock_bb_by_date.insert(
                date_str.clone(),
                QmvStockBbPointDto {
                    date: date_str,
                    timestamp_ms: ts_ms,
                    close_vnd: close_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, &sym)),
                    tmc: val_f64(&row.values, "TMC"),
                    tma20: val_f64(&row.values, "TMA20"),
                    tma50: val_f64(&row.values, "TMA50"),
                    dspi: val_f64(&row.values, "DSPI"),
                    dspi_ma5: val_f64(&row.values, "DSPIMA5"),
                    dspi_ma20: val_f64(&row.values, "DSPIMA20"),
                    mpicp_all: val_f64(&row.values, "MPICPALL"),
                    mpicp_z: val_f64(&row.values, "MPICPZ"),
                    dm: val_f64(&row.values, "DM"),
                    ds: val_f64(&row.values, "DS"),
                    stop_loss_vnd: sl_kvnd
                        .filter(|v| *v > 0.0)
                        .map(|v| normalize_price_to_sumi_vnd(v, unit, &sym)),
                    tp1_vnd: tp1_kvnd
                        .filter(|v| *v > 0.0)
                        .map(|v| normalize_price_to_sumi_vnd(v, unit, &sym)),
                    tp2_vnd: tp2_kvnd
                        .filter(|v| *v > 0.0)
                        .map(|v| normalize_price_to_sumi_vnd(v, unit, &sym)),
                },
            );
        }

        // 2. QMV_ACTIVE_MONEY (`active_money` — Shark/Cáo, Wolf/Sói, Sheep/Thỏ)
        let am_rows = query_decoded_qmv_rows(&conn, &schemas, "active_money", Some(&sym), as_of_ref, 400)?;
        let mut active_by_date: BTreeMap<String, QmvActiveMoneyPointDto> = BTreeMap::new();
        for row in am_rows {
            let Some(date_str) = row.content_date.as_deref().and_then(normalize_iso_date) else {
                continue;
            };
            let Some(ts_ms) = iso_date_to_utc_ms(&date_str) else {
                continue;
            };
            let price_kvnd = val_f64(&row.values, "Price");
            let unit = if crate::contract::is_index_or_futures_symbol(&sym) {
                QuantityUnit::Points
            } else {
                QuantityUnit::ThousandVnd
            };
            active_by_date.insert(
                date_str.clone(),
                QmvActiveMoneyPointDto {
                    date: date_str,
                    timestamp_ms: ts_ms,
                    price_vnd: price_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, &sym)),
                    shark: val_f64(&row.values, "Shark"),
                    wolf: val_f64(&row.values, "Wolf"),
                    sheep: val_f64(&row.values, "Sheep"),
                    trend_shark: val_f64(&row.values, "TrendShark"),
                    trend_wolf: val_f64(&row.values, "TrendWolf"),
                    trend_sheep: val_f64(&row.values, "TrendSheep"),
                    volume: val_f64(&row.values, "Volumn").or_else(|| val_f64(&row.values, "Volume")),
                },
            );
        }

        // 3. QMV_FOREIGN_FLOW (`symbol_foreign_flow_current` or `market_foreign_flow_current` for VNINDEX)
        let ff_family = if sym == "VNINDEX" {
            "market_foreign_flow_current"
        } else {
            "symbol_foreign_flow_current"
        };
        let ff_entity = if sym == "VNINDEX" { None } else { Some(sym.as_str()) };
        let ff_rows = query_decoded_qmv_rows(&conn, &schemas, ff_family, ff_entity, as_of_ref, 400)?;
        let mut ff_by_date: BTreeMap<String, QmvForeignFlowPointDto> = BTreeMap::new();
        for row in ff_rows {
            let Some(date_str) = row.content_date.as_deref().and_then(normalize_iso_date) else {
                continue;
            };
            let Some(ts_ms) = iso_date_to_utc_ms(&date_str) else {
                continue;
            };
            let buy_val = val_f64(&row.values, "BuyValue");
            let sell_val = val_f64(&row.values, "SellValue");
            let net_val = match (buy_val, sell_val) {
                (Some(b), Some(s)) => Some(b - s),
                _ => val_f64(&row.values, "TotalValue"),
            };
            let buy_vol = val_f64(&row.values, "BuyVolumn").or_else(|| val_f64(&row.values, "BuyVolume"));
            let sell_vol = val_f64(&row.values, "SellVolumn").or_else(|| val_f64(&row.values, "SellVolume"));
            let net_vol = match (buy_vol, sell_vol) {
                (Some(b), Some(s)) => Some(b - s),
                _ => val_f64(&row.values, "TotalVolumn"),
            };
            let price_kvnd = val_f64(&row.values, "Price");
            let unit = if crate::contract::is_index_or_futures_symbol(&sym) {
                QuantityUnit::Points
            } else {
                QuantityUnit::ThousandVnd
            };
            ff_by_date.insert(
                date_str.clone(),
                QmvForeignFlowPointDto {
                    date: date_str,
                    timestamp_ms: ts_ms,
                    price_vnd: price_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, &sym)),
                    buy_value: buy_val,
                    sell_value: sell_val,
                    net_value: net_val,
                    buy_volume: buy_vol,
                    sell_volume: sell_vol,
                    net_volume: net_vol,
                },
            );
        }

        let stock_bb: Vec<_> = stock_bb_by_date.into_values().collect();
        let active_money: Vec<_> = active_by_date.into_values().collect();
        let foreign_flow: Vec<_> = ff_by_date.into_values().collect();
        let has_real = !stock_bb.is_empty() || !active_money.is_empty() || !foreign_flow.is_empty();

        Ok(QmvRealChartSeriesDto {
            symbol: sym,
            as_of: as_of_clean,
            source_provenance: if has_real {
                "qmv_real".to_owned()
            } else {
                "local_proxy".to_owned()
            },
            is_provider_proprietary: has_real,
            stock_bb,
            active_money,
            foreign_flow,
        })
    }

    /// Loads DNSE `price_get_history` OHLCV bars from Yumeko's `dnse_mcp_observations` with
    /// F02 SHA-256 verification, PIT `as_of` clipping, and `kVND -> VND` Scale Guard normalization.
    pub fn load_dnse_ohlcv_series(
        &self,
        ticker_or_symbol: &str,
        as_of: Option<&str>,
    ) -> BridgeResult<Option<DnseLoadedSeries>> {
        let sym = strip_exchange_prefix(ticker_or_symbol);
        let cutoff_date = as_of
            .map(str::trim)
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let Some(conn) = self.open_readonly_conn()? else {
            return Ok(None);
        };
        if !table_exists(&conn, "dnse_mcp_observations")? {
            return Ok(None);
        }

        let mut stmt = conn.prepare(
            "SELECT observation_id, as_of, response_json, raw_sha256, stored_at
             FROM dnse_mcp_observations
             WHERE tool_name = 'price_get_history' AND UPPER(symbol) = ?1
             ORDER BY stored_at DESC LIMIT 5",
        )?;
        let rows = stmt.query_map(params![&sym], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;

        for row_res in rows {
            let (_obs_id, obs_as_of, resp_json, expected_sha, stored_at) = row_res?;
            let Ok(parsed) = serde_json::from_str::<Value>(&resp_json) else {
                continue;
            };
            let canonical_sha = canonical_sha256_value(&parsed);
            let raw_sha = sha256_str_hex(&resp_json);
            let sha_ok = expected_sha.eq_ignore_ascii_case(&canonical_sha)
                || expected_sha.eq_ignore_ascii_case(&raw_sha);
            if !sha_ok {
                continue;
            }

            let Some(data) = parsed.get("data").and_then(Value::as_object) else {
                continue;
            };
            let ts_arr = data.get("t").and_then(Value::as_array);
            let o_arr = data.get("o").and_then(Value::as_array);
            let h_arr = data.get("h").and_then(Value::as_array);
            let l_arr = data.get("l").and_then(Value::as_array);
            let c_arr = data.get("c").and_then(Value::as_array);
            let v_arr = data.get("v").and_then(Value::as_array);
            let (Some(ts), Some(os), Some(hs), Some(ls), Some(cs), Some(vs)) =
                (ts_arr, o_arr, h_arr, l_arr, c_arr, v_arr)
            else {
                continue;
            };
            let n = ts
                .len()
                .min(os.len())
                .min(hs.len())
                .min(ls.len())
                .min(cs.len())
                .min(vs.len());
            if n == 0 {
                continue;
            }

            let raw_unit = parsed.get("unit").and_then(Value::as_str);
            let (price_unit, price_scale) = parse_provider_price_unit(raw_unit, &sym);
            let basis = match parsed
                .get("price_basis")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "unadjusted" | "raw" => Basis::UnadjustedClose,
                _ => Basis::AdjustedClose,
            };

            let mut by_date: BTreeMap<NaiveDate, CandleBarV1> = BTreeMap::new();
            for i in 0..n {
                let Some(t_sec) = ts[i].as_i64() else {
                    continue;
                };
                let (Some(o), Some(h), Some(l), Some(c), Some(v)) = (
                    os[i].as_f64(),
                    hs[i].as_f64(),
                    ls[i].as_f64(),
                    cs[i].as_f64(),
                    vs[i].as_f64(),
                ) else {
                    continue;
                };
                if !(o.is_finite() && h.is_finite() && l.is_finite() && c.is_finite() && v.is_finite())
                    || h < l
                    || v < 0.0
                {
                    continue;
                }
                let Some(dt_utc) = Utc.timestamp_opt(t_sec, 0).single() else {
                    continue;
                };
                // Convert to ICT (UTC+7) trading date so daily bar dates align cleanly with VN sessions.
                let ict_dt = dt_utc + chrono::Duration::hours(7);
                let bar_date = ict_dt.date_naive();
                if let Some(cut) = cutoff_date
                    && bar_date > cut
                {
                    continue;
                }
                let hi = h.max(o).max(c);
                let lo = l.min(o).min(c);
                by_date.insert(
                    bar_date,
                    CandleBarV1 {
                        bar_date,
                        timestamp_utc: None,
                        open: o,
                        high: hi,
                        low: lo,
                        close: c,
                        volume: v,
                        value_traded: None,
                    },
                );
            }
            if by_date.is_empty() {
                continue;
            }
            let bars: Vec<CandleBarV1> = by_date.into_values().collect();
            let effective_as_of = cutoff_date.unwrap_or_else(|| bars.last().unwrap().bar_date);

            // F02 PIT check: if observation was captured strictly after requested `as_of` date, tag `Retrospective`.
            let captured_date = DateTime::parse_from_rfc3339(&stored_at)
                .ok()
                .map(|dt| (dt.with_timezone(&Utc) + chrono::Duration::hours(7)).date_naive());
            let historical_mode = match (cutoff_date, captured_date) {
                (Some(cut), Some(cap)) if cap > cut => HistoricalMode::Retrospective,
                _ => HistoricalMode::PointInTime,
            };

            let contract_series = CandleSeriesV1 {
                schema_version: SUMI_CONTRACT_VERSION_V1.to_owned(),
                symbol: sym.clone(),
                timeframe: TimeframeV1::Daily1d,
                price_unit,
                price_scale,
                basis,
                historical_mode,
                source_provider: parsed
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or("dnse_real")
                    .to_owned(),
                source_sha256: expected_sha,
                as_of: effective_as_of,
                bars,
            };
            let sumi_bars = candle_series_v1_to_sumi_bars(&contract_series)?;
            return Ok(Some(DnseLoadedSeries {
                contract_series,
                sumi_bars,
                sha256_verified: true,
                historical_mode,
                observation_as_of: obs_as_of,
            }));
        }

        Ok(None)
    }

    /// Verifies a `yumeko research export` directory (9 PIT tables + `manifest.json` SHA-256).
    pub fn verify_research_export_dir(dir: &Path) -> BridgeResult<ResearchExportVerificationDto> {
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.is_file() {
            return Ok(ResearchExportVerificationDto {
                export_dir: dir.display().to_string(),
                manifest_found: false,
                verified_files: 0,
                failed_files: vec!["manifest.json không tồn tại".to_owned()],
                as_of: None,
                valid: false,
            });
        }
        let raw = fs::read_to_string(&manifest_path)?;
        let manifest: Value = serde_json::from_str(&raw)?;
        let as_of = manifest
            .get("as_of")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        let mut verified = 0usize;
        let mut failed = Vec::new();
        if let Some(files) = manifest
            .get("files")
            .or_else(|| manifest.get("tables"))
            .and_then(Value::as_array)
        {
            for item in files {
                let rel = item
                    .get("path")
                    .or_else(|| item.get("file"))
                    .or_else(|| item.get("filename"))
                    .and_then(Value::as_str);
                let expected_sha = item
                    .get("sha256")
                    .or_else(|| item.get("sha256_hex"))
                    .and_then(Value::as_str);
                if let (Some(r), Some(exp)) = (rel, expected_sha) {
                    let target = dir.join(r);
                    if !target.is_file() {
                        failed.push(format!("{r}: missing"));
                        continue;
                    }
                    let bytes = fs::read(&target)?;
                    let actual = hex::encode(sha2::Sha256::digest(&bytes));
                    if actual.eq_ignore_ascii_case(exp) {
                        verified += 1;
                    } else {
                        failed.push(format!("{r}: sha256 mismatch"));
                    }
                }
            }
        }
        let valid = failed.is_empty() && verified > 0;
        Ok(ResearchExportVerificationDto {
            export_dir: dir.display().to_string(),
            manifest_found: true,
            verified_files: verified,
            failed_files: failed,
            as_of,
            valid,
        })
    }
}

// ============================================================================
// Private SQLite Read-Only Helpers
// ============================================================================

fn table_exists(conn: &Connection, table_name: &str) -> BridgeResult<bool> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table_name],
        |r| r.get(0),
    )?;
    Ok(exists > 0)
}

fn count_query(conn: &Connection, table: &str, sql: &str) -> BridgeResult<usize> {
    if !table_exists(conn, table)? {
        return Ok(0);
    }
    let cnt: i64 = conn.query_row(sql, [], |r| r.get(0))?;
    Ok(cnt.max(0) as usize)
}

fn count_distinct_qmv_entities(conn: &Connection, family: &str) -> BridgeResult<usize> {
    if !table_exists(conn, "qmv_compact_rows")? {
        return Ok(0);
    }
    let cnt: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT UPPER(entity)) FROM qmv_compact_rows
         WHERE family = ?1 AND entity IS NOT NULL AND TRIM(entity) <> ''",
        params![family],
        |r| r.get(0),
    )?;
    Ok(cnt.max(0) as usize)
}

fn load_all_compact_schemas(conn: &Connection) -> BridgeResult<HashMap<String, CompactSchemaWire>> {
    let mut map = HashMap::new();
    if !table_exists(conn, "qmv_compact_schemas")? {
        return Ok(map);
    }
    let mut stmt = conn.prepare("SELECT id, body FROM qmv_compact_schemas")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for row in rows.flatten() {
        if let Ok(parsed) = serde_json::from_str::<CompactSchemaWire>(&row.1) {
            map.insert(row.0, parsed);
        }
    }
    Ok(map)
}

fn query_decoded_qmv_rows(
    conn: &Connection,
    schemas: &HashMap<String, CompactSchemaWire>,
    family: &str,
    entity: Option<&str>,
    as_of: Option<&str>,
    limit: usize,
) -> BridgeResult<Vec<DecodedRow>> {
    if !table_exists(conn, "qmv_compact_rows")? {
        return Ok(Vec::new());
    }
    let lim = limit.clamp(1, 2_000) as i64;
    let raw_rows: Vec<(Option<String>, Option<String>, String, String)> = match (entity, as_of) {
        (Some(ent), Some(cut)) => {
            let mut stmt = conn.prepare(
                "SELECT entity, content_date, schema_ref, body FROM qmv_compact_rows
                 WHERE family = ?1 AND UPPER(entity) = ?2
                   AND (content_date IS NULL OR content_date <= ?3)
                 ORDER BY content_date DESC, revision DESC LIMIT ?4",
            )?;
            let mut out = Vec::new();
            let iter = stmt.query_map(params![family, ent.to_ascii_uppercase(), cut, lim], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?;
            for item in iter.flatten() {
                out.push(item);
            }
            out
        }
        (Some(ent), None) => {
            let mut stmt = conn.prepare(
                "SELECT entity, content_date, schema_ref, body FROM qmv_compact_rows
                 WHERE family = ?1 AND UPPER(entity) = ?2
                 ORDER BY content_date DESC, revision DESC LIMIT ?3",
            )?;
            let mut out = Vec::new();
            let iter = stmt.query_map(params![family, ent.to_ascii_uppercase(), lim], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?;
            for item in iter.flatten() {
                out.push(item);
            }
            out
        }
        (None, Some(cut)) => {
            let mut stmt = conn.prepare(
                "SELECT entity, content_date, schema_ref, body FROM qmv_compact_rows
                 WHERE family = ?1
                   AND (content_date IS NULL OR content_date <= ?2)
                 ORDER BY content_date DESC, revision DESC LIMIT ?3",
            )?;
            let mut out = Vec::new();
            let iter = stmt.query_map(params![family, cut, lim], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?;
            for item in iter.flatten() {
                out.push(item);
            }
            out
        }
        (None, None) => {
            let mut stmt = conn.prepare(
                "SELECT entity, content_date, schema_ref, body FROM qmv_compact_rows
                 WHERE family = ?1
                 ORDER BY content_date DESC, revision DESC LIMIT ?2",
            )?;
            let mut out = Vec::new();
            let iter = stmt.query_map(params![family, lim], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?;
            for item in iter.flatten() {
                out.push(item);
            }
            out
        }
    };

    let mut decoded = Vec::with_capacity(raw_rows.len());
    for (ent, cdate, schema_ref, body_str) in raw_rows {
        let Some(schema) = schemas.get(&schema_ref) else {
            continue;
        };
        let Ok(raw) = serde_json::from_str::<Value>(&body_str) else {
            continue;
        };
        let missing: BTreeSet<usize> = raw
            .get("missing_fields")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_default();
        let vals = raw
            .get("values")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut map = BTreeMap::new();
        for (idx, fname) in schema.fields.iter().enumerate() {
            if missing.contains(&idx) {
                continue;
            }
            if let Some(v) = vals.get(idx) {
                map.insert(fname.clone(), v.clone());
            }
        }
        decoded.push(DecodedRow {
            content_date: cdate.or_else(|| {
                raw.get("content_date")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }),
            entity: ent.or_else(|| {
                raw.get("entity")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }),
            values: map,
        });
    }
    Ok(decoded)
}

fn load_vm01_market_context(
    conn: &Connection,
    schemas: &HashMap<String, CompactSchemaWire>,
    as_of: Option<&str>,
) -> BridgeResult<Vm01MarketContextDto> {
    let sent_rows = query_decoded_qmv_rows(conn, schemas, "market_sentiment_current", None, as_of, 1)?;
    let market_sentiment = sent_rows.first().and_then(|r| val_f64(&r.values, "Value"));
    let mut effective_as_of = sent_rows.first().and_then(|r| r.content_date.clone());

    let bb_rows = query_decoded_qmv_rows(conn, schemas, "bbmarket_v2_horizons_current", None, as_of, 10)?;
    let bb_pick = bb_rows
        .iter()
        .find(|r| {
            r.entity
                .as_deref()
                .is_some_and(|e| e.eq_ignore_ascii_case("VNINDEX") || e == "0000" || e.contains("MARKET"))
        })
        .or_else(|| bb_rows.first());
    if effective_as_of.is_none() {
        effective_as_of = bb_pick.and_then(|r| r.content_date.clone());
    }

    let ff_rows = query_decoded_qmv_rows(conn, schemas, "market_foreign_flow_current", None, as_of, 1)?;
    let market_foreign_net_value = ff_rows.first().and_then(|r| {
        match (val_f64(&r.values, "BuyValue"), val_f64(&r.values, "SellValue")) {
            (Some(b), Some(s)) => Some(b - s),
            _ => val_f64(&r.values, "TotalValue"),
        }
    });

    // Sector breadth from latest `valuation_report_current`
    let sector_rows =
        query_decoded_qmv_rows(conn, schemas, "valuation_report_current", None, as_of, 40)?;
    let latest_sec_date = sector_rows.first().and_then(|r| r.content_date.clone());
    let mut seen_icb = BTreeSet::new();
    let mut top_sectors = Vec::new();
    for r in sector_rows {
        if latest_sec_date.is_some() && r.content_date != latest_sec_date {
            continue;
        }
        let code = val_str(&r.values, "ICB2Code").unwrap_or_default();
        let name = val_str(&r.values, "ICB2Name").unwrap_or_default();
        if name.is_empty() || !seen_icb.insert(code.clone()) {
            continue;
        }
        top_sectors.push(SectorBreadthSnapshotDto {
            icb2_code: code,
            icb2_name: name,
            change_pct: val_f64(&r.values, "PhanTramTangDiem"),
            advancers: val_i64(&r.values, "TongCoPhieuTang"),
            decliners: val_i64(&r.values, "TongCoPhieuGiam"),
            unchanged: val_i64(&r.values, "TongCoPhieuDungGia"),
            total_stocks: val_i64(&r.values, "TongCoPhieu"),
            advancing_ratio_pct: val_f64(&r.values, "TyLeCoPhieuTang"),
        });
    }
    top_sectors.sort_by(|a, b| {
        b.change_pct
            .unwrap_or(-999.0)
            .total_cmp(&a.change_pct.unwrap_or(-999.0))
    });
    top_sectors.truncate(8);

    // Macro highlights (`yr_macro_observations` strictly `release_date <= as_of`)
    let mut macro_highlights = Vec::new();
    if table_exists(conn, "yr_macro_observations")? {
        let mut seen_codes = BTreeSet::new();
        let sql = if as_of.is_some() {
            "SELECT indicator_code, period_date, release_date, value, value_text, unit, source_name
             FROM yr_macro_observations
             WHERE release_date <= ?1
             ORDER BY release_date DESC, period_date DESC LIMIT 120"
        } else {
            "SELECT indicator_code, period_date, release_date, value, value_text, unit, source_name
             FROM yr_macro_observations
             ORDER BY release_date DESC, period_date DESC LIMIT 120"
        };
        let mut stmt = conn.prepare(sql)?;
        let mut push_row = |row: MacroObservationDto| {
            if seen_codes.insert(row.indicator_code.clone()) && macro_highlights.len() < 8 {
                macro_highlights.push(row);
            }
        };
        if let Some(cut) = as_of {
            let iter = stmt.query_map(params![cut], map_macro_row)?;
            for item in iter.flatten() {
                push_row(item);
            }
        } else {
            let iter = stmt.query_map([], map_macro_row)?;
            for item in iter.flatten() {
                push_row(item);
            }
        }
    }

    Ok(Vm01MarketContextDto {
        effective_as_of,
        market_sentiment,
        market_bbm_05: bb_pick.and_then(|r| val_f64(&r.values, "BBM05")),
        market_bbm_20: bb_pick.and_then(|r| val_f64(&r.values, "BBM20")),
        market_bbmz_05: bb_pick.and_then(|r| val_f64(&r.values, "BBMZ05")),
        market_bbmz_20: bb_pick.and_then(|r| val_f64(&r.values, "BBMZ20")),
        market_foreign_net_value,
        top_sectors,
        macro_highlights,
    })
}

fn map_macro_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<MacroObservationDto> {
    Ok(MacroObservationDto {
        indicator_code: r.get(0)?,
        period_date: r.get(1)?,
        release_date: r.get(2)?,
        value: r.get(3)?,
        value_text: r.get(4)?,
        unit: r.get(5)?,
        source_name: r.get(6)?,
    })
}

fn load_hp01_stock_blackbox(
    conn: &Connection,
    schemas: &HashMap<String, CompactSchemaWire>,
    sym: &str,
    as_of: Option<&str>,
) -> BridgeResult<Option<QmvStockBlackboxSnapshotDto>> {
    let tech_rows = query_decoded_qmv_rows(conn, schemas, "technical_metrics", Some(sym), as_of, 1)?;
    let Some(row) = tech_rows.into_iter().next() else {
        return Ok(None);
    };

    let unit = if crate::contract::is_index_or_futures_symbol(sym) {
        QuantityUnit::Points
    } else {
        QuantityUnit::ThousandVnd
    };
    let close_kvnd = val_f64(&row.values, "Close");
    let sl_kvnd = val_f64(&row.values, "StopLoss").filter(|v| *v > 0.0);
    let tp1_kvnd = val_f64(&row.values, "TargetPrice1").filter(|v| *v > 0.0);
    let tp2_kvnd = val_f64(&row.values, "TargetPrice2").filter(|v| *v > 0.0);

    let mut active_flags = Vec::new();
    for (field, label) in [
        ("TIENVAO_TODAY", "Tiền vào hôm nay"),
        ("TIENVAO_2PHIEN", "Tiền vào 2 phiên"),
        ("TIENVAO_3PHIEN", "Tiền vào 3 phiên"),
        ("TIEN_DAO_CHIEU_TANG_T_PLUS", "Đảo chiều tăng T+"),
        ("TIEN_DAO_CHIEU_TANG", "Đảo chiều tăng Sóng"),
        ("TIENRA_TODAY", "Tiền ra hôm nay"),
        ("TIEN_DAO_CHIEU_GIAM", "Đảo chiều giảm"),
        ("DSPIKhoe", "DSPI Khỏe"),
        ("DSPIDuytri", "DSPI Duy trì"),
        ("DSPIBaohoa", "DSPI Bão hòa"),
        ("Spring", "VSA Spring"),
        ("Upthrust", "VSA Upthrust"),
        ("DemandUp", "Cầu tăng"),
        ("SupplyUp", "Cung tăng"),
        ("NewHigh", "Đỉnh mới"),
        ("IchimokusystemBuy", "Ichimoku Mua"),
        ("IchimokusystemSell", "Ichimoku Bán"),
        ("RegularDivergeBuy", "Phân kỳ dương"),
        ("RegularDivergeSell", "Phân kỳ âm"),
    ] {
        if val_truthy(&row.values, field) {
            active_flags.push(label.to_owned());
        }
    }

    // Enrich with `trading_ideas`, `division_current`, and `qmv60_membership`
    let ideas = query_decoded_qmv_rows(conn, schemas, "trading_ideas", Some(sym), as_of, 1)?;
    let qmv_rating = ideas.first().and_then(|r| val_str(&r.values, "qmv_rating"));
    let trading_suitability = ideas
        .first()
        .and_then(|r| val_str(&r.values, "trading_suitability"));

    let divs = query_decoded_qmv_rows(conn, schemas, "division_current", Some(sym), as_of, 1)?;
    let division_allocation_group = divs
        .first()
        .and_then(|r| val_str(&r.values, "allocation_group"));
    let division_money_flow_label = divs
        .first()
        .and_then(|r| val_str(&r.values, "money_flow_group_1_label"));

    let qmv60 = query_decoded_qmv_rows(conn, schemas, "qmv60_membership", Some(sym), as_of, 1)?;

    Ok(Some(QmvStockBlackboxSnapshotDto {
        content_date: row.content_date.unwrap_or_default(),
        source_provenance: "qmv_real".to_owned(),
        is_provider_proprietary: true,
        close_vnd: close_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, sym)),
        close_kvnd,
        tmc: val_f64(&row.values, "TMC"),
        tma20: val_f64(&row.values, "TMA20"),
        tma50: val_f64(&row.values, "TMA50"),
        dspi: val_f64(&row.values, "DSPI"),
        dspi_ma5: val_f64(&row.values, "DSPIMA5"),
        dspi_ma20: val_f64(&row.values, "DSPIMA20"),
        mpicp_all: val_f64(&row.values, "MPICPALL"),
        mpicp_z: val_f64(&row.values, "MPICPZ"),
        dm: val_f64(&row.values, "DM"),
        ds: val_f64(&row.values, "DS"),
        trading_score: val_f64(&row.values, "TradingScore"),
        price_trend: val_str(&row.values, "PriceTrend"),
        stop_loss_vnd: sl_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, sym)),
        target_price_1_vnd: tp1_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, sym)),
        target_price_2_vnd: tp2_kvnd.map(|v| normalize_price_to_sumi_vnd(v, unit, sym)),
        money_in_today: val_truthy(&row.values, "TIENVAO_TODAY"),
        money_in_2_sessions: val_truthy(&row.values, "TIENVAO_2PHIEN"),
        money_in_3_sessions: val_truthy(&row.values, "TIENVAO_3PHIEN"),
        reversal_up_t_plus: val_truthy(&row.values, "TIEN_DAO_CHIEU_TANG_T_PLUS"),
        reversal_up_swing: val_truthy(&row.values, "TIEN_DAO_CHIEU_TANG"),
        active_flags,
        qmv_rating,
        trading_suitability,
        division_allocation_group,
        division_money_flow_label,
        in_qmv60: !qmv60.is_empty(),
    }))
}

fn load_dn01_valuations(
    conn: &Connection,
    sym: &str,
    as_of: Option<&str>,
) -> BridgeResult<Vec<ValuationQuarterDto>> {
    if !table_exists(conn, "symbol_valuation_snapshots")? {
        return Ok(Vec::new());
    }
    let mut rows_out = Vec::new();
    let sql = if as_of.is_some() {
        "SELECT period, content_date, source_family, pe, pb, ps, ev_ebitda, eps, bvps, roe, roa, debt_to_equity
         FROM symbol_valuation_snapshots
         WHERE UPPER(symbol) = ?1 AND (content_date IS NULL OR content_date <= ?2)
         ORDER BY period DESC LIMIT 16"
    } else {
        "SELECT period, content_date, source_family, pe, pb, ps, ev_ebitda, eps, bvps, roe, roa, debt_to_equity
         FROM symbol_valuation_snapshots
         WHERE UPPER(symbol) = ?1
         ORDER BY period DESC LIMIT 16"
    };
    let mut stmt = conn.prepare(sql)?;
    let mut seen_periods = BTreeSet::new();
    let map_val = |r: &rusqlite::Row<'_>| -> rusqlite::Result<ValuationQuarterDto> {
        Ok(ValuationQuarterDto {
            period: r.get(0)?,
            content_date: r.get(1)?,
            source_family: r.get(2)?,
            pe: r.get(3)?,
            pb: r.get(4)?,
            ps: r.get(5)?,
            ev_ebitda: r.get(6)?,
            eps: r.get(7)?,
            bvps: r.get(8)?,
            roe: r.get(9)?,
            roa: r.get(10)?,
            debt_to_equity: r.get(11)?,
        })
    };
    if let Some(cut) = as_of {
        let iter = stmt.query_map(params![sym, cut], map_val)?;
        for item in iter.flatten() {
            if seen_periods.insert(item.period.clone()) && rows_out.len() < 8 {
                rows_out.push(item);
            }
        }
    } else {
        let iter = stmt.query_map(params![sym], map_val)?;
        for item in iter.flatten() {
            if seen_periods.insert(item.period.clone()) && rows_out.len() < 8 {
                rows_out.push(item);
            }
        }
    }
    Ok(rows_out)
}

fn load_dm01_real_holding(
    conn: &Connection,
    sym: &str,
    as_of: Option<&str>,
) -> BridgeResult<Option<Dm01HoldingDto>> {
    if !table_exists(conn, "personal_portfolio_snapshots")? {
        return Ok(None);
    }
    let row: Option<(String, String, String, String)> = if let Some(cut) = as_of {
        conn.query_row(
            "SELECT requested_as_of, effective_as_of, portfolio_json, snapshot_sha256
             FROM personal_portfolio_snapshots
             WHERE effective_as_of <= ?1
             ORDER BY effective_as_of DESC, stored_at DESC LIMIT 1",
            params![cut],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok()
    } else {
        conn.query_row(
            "SELECT requested_as_of, effective_as_of, portfolio_json, snapshot_sha256
             FROM personal_portfolio_snapshots
             ORDER BY effective_as_of DESC, stored_at DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok()
    };

    let Some((req_as_of, eff_as_of, port_json, expected_sha)) = row else {
        return Ok(None);
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&port_json) else {
        return Ok(None);
    };
    let sha_ok = expected_sha.eq_ignore_ascii_case(&canonical_sha256_value(&parsed))
        || expected_sha.eq_ignore_ascii_case(&sha256_str_hex(&port_json));

    let Some(holdings) = parsed.get("audited_holdings").and_then(Value::as_array) else {
        return Ok(None);
    };
    for h in holdings {
        let Some(h_sym) = h.get("symbol").and_then(Value::as_str) else {
            continue;
        };
        if !h_sym.trim().eq_ignore_ascii_case(sym) {
            continue;
        }
        let qty = h.get("quantity").and_then(Value::as_f64).unwrap_or(0.0);
        if qty <= 0.0 {
            continue;
        }
        return Ok(Some(Dm01HoldingDto {
            as_of: req_as_of,
            effective_as_of: eff_as_of,
            sha256_verified: sha_ok,
            quantity: qty,
            avg_cost_vnd: h.get("avg_cost").and_then(Value::as_f64).unwrap_or(0.0),
            current_price_vnd: h.get("current_price").and_then(Value::as_f64).unwrap_or(0.0),
            current_value_vnd: h.get("current_value").and_then(Value::as_f64).unwrap_or(0.0),
            unrealized_pnl_vnd: h
                .get("unrealized_pnl")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            unrealized_pnl_pct: h
                .get("unrealized_pnl_pct")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            weight_pct: h.get("weight_pct").and_then(Value::as_f64).unwrap_or(0.0),
            days_held: h.get("days_held").and_then(Value::as_i64),
            sector: h
                .get("sector")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        }));
    }
    Ok(None)
}

fn load_dm01_watchlist_entry(
    conn: &Connection,
    sym: &str,
) -> BridgeResult<Option<Dm01WatchlistEntryDto>> {
    if !table_exists(conn, "yr_personal_watchlist")? {
        return Ok(None);
    }
    let row = conn
        .query_row(
            "SELECT symbol, strategy_tag, note, target_buy, invalidation_level, updated_at
             FROM yr_personal_watchlist
             WHERE UPPER(symbol) = ?1
             ORDER BY updated_at DESC LIMIT 1",
            params![sym],
            |r| {
                let target_buy: Option<f64> = r.get(3)?;
                let inv_level: Option<f64> = r.get(4)?;
                Ok(Dm01WatchlistEntryDto {
                    symbol: r.get(0)?,
                    strategy_tag: r.get(1)?,
                    note: r.get(2)?,
                    target_buy_vnd: target_buy,
                    invalidation_level_vnd: inv_level,
                    updated_at: r.get(5)?,
                })
            },
        )
        .ok();
    Ok(row)
}

fn load_episodic_theses(
    conn: &Connection,
    sym: &str,
    as_of: Option<&str>,
) -> BridgeResult<Vec<EpisodicThesisDto>> {
    if !table_exists(conn, "yr_episodic_decisions")? {
        return Ok(Vec::new());
    }
    let sql = if as_of.is_some() {
        "SELECT decision_id, as_of, function_name, gate_step, stance, confidence, thesis_summary
         FROM yr_episodic_decisions
         WHERE UPPER(symbol) = ?1 AND as_of <= ?2
         ORDER BY as_of DESC, recorded_at DESC LIMIT 5"
    } else {
        "SELECT decision_id, as_of, function_name, gate_step, stance, confidence, thesis_summary
         FROM yr_episodic_decisions
         WHERE UPPER(symbol) = ?1
         ORDER BY as_of DESC, recorded_at DESC LIMIT 5"
    };
    let mut stmt = conn.prepare(sql)?;
    let map_ep = |r: &rusqlite::Row<'_>| -> rusqlite::Result<EpisodicThesisDto> {
        Ok(EpisodicThesisDto {
            decision_id: r.get(0)?,
            as_of: r.get(1)?,
            function_name: r.get(2)?,
            gate_step: r.get(3)?,
            stance: r.get(4)?,
            confidence: r.get(5)?,
            thesis_summary: r.get(6)?,
        })
    };
    let mut out = Vec::new();
    if let Some(cut) = as_of {
        for item in stmt.query_map(params![sym, cut], map_ep)?.flatten() {
            out.push(item);
        }
    } else {
        for item in stmt.query_map(params![sym], map_ep)?.flatten() {
            out.push(item);
        }
    }
    Ok(out)
}

fn has_dnse_price_history(conn: &Connection, sym: &str) -> BridgeResult<bool> {
    if !table_exists(conn, "dnse_mcp_observations")? {
        return Ok(false);
    }
    let cnt: i64 = conn.query_row(
        "SELECT COUNT(*) FROM dnse_mcp_observations
         WHERE tool_name = 'price_get_history' AND UPPER(symbol) = ?1",
        params![sym],
        |r| r.get(0),
    )?;
    Ok(cnt > 0)
}

fn val_f64(map: &BTreeMap<String, Value>, key: &str) -> Option<f64> {
    let v = map.get(key)?;
    if let Some(n) = v.as_f64() {
        return n.is_finite().then_some(n);
    }
    if let Some(s) = v.as_str() {
        let clean = s.trim().replace([',', '%'], "");
        if let Ok(n) = clean.parse::<f64>()
            && n.is_finite()
        {
            return Some(n);
        }
    }
    None
}

fn val_i64(map: &BTreeMap<String, Value>, key: &str) -> Option<i64> {
    let v = map.get(key)?;
    v.as_i64()
        .or_else(|| v.as_f64().map(|f| f.round() as i64))
}

fn val_str(map: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    let v = map.get(key)?;
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn val_truthy(map: &BTreeMap<String, Value>, key: &str) -> bool {
    let Some(v) = map.get(key) else {
        return false;
    };
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x > 0.0),
        Value::String(s) => {
            let l = s.trim().to_ascii_lowercase();
            matches!(l.as_str(), "1" | "true" | "yes" | "y" | "x" | "có")
        }
        _ => false,
    }
}

fn normalize_iso_date(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.len() >= 10 {
        let prefix = &s[..10];
        if NaiveDate::parse_from_str(prefix, "%Y-%m-%d").is_ok() {
            return Some(prefix.to_owned());
        }
    }
    None
}

fn iso_date_to_utc_ms(iso: &str) -> Option<i64> {
    let d = NaiveDate::parse_from_str(iso, "%Y-%m-%d").ok()?;
    Some(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_slate_missing_yumeko_db_degrades_gracefully_with_local_proxy() {
        let tmp = tempfile::tempdir().unwrap();
        let bridge = YumekoReadOnlyBridge::new(tmp.path().join("nonexistent.sqlite"), None);
        assert!(!bridge.is_available());

        let status = bridge.get_status().expect("status succeeds cleanly");
        assert!(!status.available);
        assert_eq!(status.qmv_technical_symbols, 0);

        let ctx = bridge
            .load_topdown_context("HOSE:FPT", Some("2026-11-25"))
            .expect("context degrades cleanly");
        assert!(!ctx.bridge_available);
        assert_eq!(ctx.flow_provenance, "local_proxy");
        assert!(ctx.hp01_blackbox.is_none());

        let series = bridge
            .load_qmv_stock_series("HOSE:FPT", Some("2026-11-25"))
            .expect("series degrades cleanly");
        assert_eq!(series.source_provenance, "local_proxy");
        assert!(!series.is_provider_proprietary);
        assert!(series.stock_bb.is_empty());
    }

    #[test]
    fn hard_guard_readonly_connection_rejects_any_write_attempt() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("mock_yumeko.sqlite");
        {
            let rw = Connection::open(&db_path).unwrap();
            rw.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY);")
                .unwrap();
        }
        let bridge = YumekoReadOnlyBridge::new(db_path, None);
        let ro = bridge.open_readonly_conn().unwrap().expect("opened");
        let write_attempt = ro.execute("INSERT INTO t (id) VALUES (1)", []);
        assert!(
            write_attempt.is_err(),
            "Read-only bridge MUST reject any SQLite write mutation"
        );
    }

    #[test]
    fn real_yumeko_db_readonly_integration_if_present_on_host() {
        let bridge = YumekoReadOnlyBridge::discover_default();
        if !bridge.is_available() {
            // Clean-Slate CI environment: skip gracefully.
            return;
        }
        let status = bridge.get_status().expect("real status");
        assert!(status.available);
        assert!(status.qmv_technical_symbols > 0);

        let ctx = bridge
            .load_topdown_context("HOSE:FPT", Some("2026-11-25"))
            .expect("real FPT context");
        assert!(ctx.bridge_available);
        assert_eq!(ctx.flow_provenance, "qmv_real");
        let bb = ctx.hp01_blackbox.expect("FPT real QMV blackbox");
        assert!(bb.is_provider_proprietary);
        if let Some(close_vnd) = bb.close_vnd {
            assert!(
                (10_000.0..=500_000.0).contains(&close_vnd),
                "FPT close_vnd must be scaled to VND (got {close_vnd})"
            );
        }
        assert!(
            !ctx.dn01_valuations.is_empty(),
            "FPT 8Q CSTC valuations must decode"
        );

        let qmv_series = bridge
            .load_qmv_stock_series("HOSE:FPT", Some("2026-11-25"))
            .expect("FPT QMV series");
        assert_eq!(qmv_series.source_provenance, "qmv_real");
        assert!(qmv_series.is_provider_proprietary);
        assert!(!qmv_series.stock_bb.is_empty());
        assert!(!qmv_series.active_money.is_empty());
    }
}
