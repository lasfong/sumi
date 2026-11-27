use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sumi_core::data::{
    DataLoadOptions, InvalidRowPolicy, OhlcEnvelopePolicy, load_ohlcv_csv_multi_symbol,
};
use tracing::info;

use crate::error::{DataError, Result};
use crate::storage::SqliteMarketStore;
use crate::types::{
    Adjustment, Interval, OhlcvBar, ResolvedSymbol, SeriesKey, SymbolKey,
    infer_vn_exchange_for_ticker, normalize_bars,
};

/// Configuration for [`CafeFImporter`].
#[derive(Debug, Clone)]
pub struct CafeFImportConfig {
    /// Multiplier applied to CafeF stock CSV prices (`SolieuGD` quotes are in thousands of VND,
    /// so `1000.0` normalizes them to raw VND matching TradingView `HOSE:*` / `HNX:*` / `UPCOM:*`).
    pub stock_price_multiplier: f64,
    /// Whether to also import `CafeF.SolieuGD.Raw*.csv` into `Adjustment::Unadjusted`.
    pub include_raw: bool,
    /// Target adjustment bucket for `CafeF.SolieuGD.Adjusted*.csv` (default: `Adjustment::Splits`).
    pub adjusted_bucket: Adjustment,
}

impl Default for CafeFImportConfig {
    fn default() -> Self {
        Self {
            stock_price_multiplier: 1_000.0,
            include_raw: true,
            adjusted_bucket: Adjustment::Splits,
        }
    }
}

/// Summary report returned after importing a CafeF CSV directory into [`SqliteMarketStore`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CafeFImportSummary {
    pub files_processed: Vec<String>,
    pub distinct_symbols: usize,
    pub total_series: usize,
    pub total_bars_written: usize,
    pub quarantined_rows: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CafeFFileKind {
    Index,
    AdjustedStocks,
    RawStocks,
}

/// High-speed batch importer for official CafeF CSV archives (`CafeF.INDEX*.csv`,
/// `CafeF.SolieuGD.Adjusted*.csv`, `CafeF.SolieuGD.Raw*.csv`).
///
/// Reuses `sumi-core`'s single-pass RFC-4180 streaming parser with
/// `OhlcEnvelopePolicy::CafeFSplitAware` and `InvalidRowPolicy::QuarantineRow`, seeding
/// 471+ Vietnamese equities and benchmark indices into local SQLite storage in a few seconds.
pub struct CafeFImporter {
    config: CafeFImportConfig,
}

impl Default for CafeFImporter {
    fn default() -> Self {
        Self::new(CafeFImportConfig::default())
    }
}

impl CafeFImporter {
    #[must_use]
    pub const fn new(config: CafeFImportConfig) -> Self {
        Self { config }
    }

    /// Scans `dir` for CafeF CSV files and imports them atomically into `store`.
    pub fn import_dir(
        &self,
        dir: impl AsRef<Path>,
        store: &SqliteMarketStore,
    ) -> Result<CafeFImportSummary> {
        let dir_path = dir.as_ref();
        if !dir_path.is_dir() {
            return Err(DataError::Import(format!(
                "CafeF directory does not exist or is not a directory: {}",
                dir_path.display()
            )));
        }

        let started = Instant::now();
        let mut discovered: Vec<(CafeFFileKind, PathBuf)> = Vec::new();

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Some(kind) = classify_cafef_filename(name) {
                if kind == CafeFFileKind::RawStocks && !self.config.include_raw {
                    continue;
                }
                discovered.push((kind, path));
            }
        }

        discovered.sort_by(|a, b| a.1.cmp(&b.1));
        if discovered.is_empty() {
            return Err(DataError::Import(format!(
                "no CafeF CSV files (CafeF.INDEX*.csv / CafeF.SolieuGD.*.csv) found in {}",
                dir_path.display()
            )));
        }

        let mut files_processed = Vec::with_capacity(discovered.len());
        let mut seen_symbols: HashSet<String> = HashSet::new();
        let mut total_series = 0_usize;
        let mut total_bars_written = 0_usize;
        let mut quarantined_rows = 0_usize;

        for (kind, path) in discovered {
            let file_label = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.csv")
                .to_owned();
            let (is_index, adjustment, price_multiplier, source_tag) = match kind {
                CafeFFileKind::Index => (true, Adjustment::Splits, 1.0, "cafef_index"),
                CafeFFileKind::AdjustedStocks => (
                    false,
                    self.config.adjusted_bucket,
                    self.config.stock_price_multiplier,
                    "cafef_adjusted",
                ),
                CafeFFileKind::RawStocks => (
                    false,
                    Adjustment::Unadjusted,
                    self.config.stock_price_multiplier,
                    "cafef_raw",
                ),
            };

            let csv_text = std::fs::read_to_string(&path)?;
            let opts = DataLoadOptions {
                envelope_policy: OhlcEnvelopePolicy::CafeFSplitAware,
                invalid_row_policy: InvalidRowPolicy::QuarantineRow,
                price_multiplier,
                ..DataLoadOptions::default()
            };

            let reports = load_ohlcv_csv_multi_symbol(&csv_text, &opts).map_err(|e| {
                DataError::Import(format!("failed parsing CafeF file '{file_label}': {e:?}"))
            })?;

            let mut series_batches: Vec<(SeriesKey, Vec<OhlcvBar>, String)> =
                Vec::with_capacity(reports.len());
            let mut symbol_metas: Vec<ResolvedSymbol> = Vec::with_capacity(reports.len());

            for rep in reports {
                quarantined_rows = quarantined_rows.saturating_add(rep.quarantined_row_count);
                if rep.candles.is_empty() {
                    continue;
                }
                let raw_sym = normalize_cafef_ticker(&rep.symbol, is_index);
                let exchange = infer_vn_exchange_for_ticker(&raw_sym, is_index);
                let Ok(sym_key) = SymbolKey::new(exchange, &raw_sym) else {
                    continue;
                };

                let mut raw_bars: Vec<OhlcvBar> =
                    Vec::with_capacity(rep.candles.len());
                for candle in &rep.candles {
                    let bar = OhlcvBar::from_core_candle(candle);
                    // Vietnamese exchanges (`HOSE`, `HNX`, `UPCOM`) never trade on Saturdays or Sundays;
                    // quarantine weekend bars (e.g. inverted DD/MM rows in CafeF INDEX CSVs — BUG-001!).
                    if is_vn_weekend_timestamp_ms(bar.timestamp) {
                        quarantined_rows = quarantined_rows.saturating_add(1);
                        continue;
                    }
                    raw_bars.push(bar);
                }
                let bars = normalize_bars(raw_bars);
                if bars.is_empty() {
                    continue;
                }

                seen_symbols.insert(sym_key.ticker());
                let series_key = SeriesKey::new(&sym_key, Interval::D1, adjustment);
                series_batches.push((series_key, bars, source_tag.to_owned()));
                symbol_metas.push(ResolvedSymbol::from_cafef_vn_ticker(
                    &sym_key.exchange,
                    &sym_key.symbol,
                    is_index,
                ));
            }

            let written =
                store.bulk_import_series_and_symbols(&series_batches, &symbol_metas, true)?;

            total_series = total_series.saturating_add(series_batches.len());
            total_bars_written = total_bars_written.saturating_add(written);
            files_processed.push(file_label.clone());

            info!(
                file = %file_label,
                series = series_batches.len(),
                bars = written,
                "imported CafeF archive into SQLite cache"
            );
        }

        Ok(CafeFImportSummary {
            files_processed,
            distinct_symbols: seen_symbols.len(),
            total_series,
            total_bars_written,
            quarantined_rows,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }
}

fn classify_cafef_filename(name: &str) -> Option<CafeFFileKind> {
    let upper = name.to_ascii_uppercase();
    if !upper.ends_with(".CSV") {
        return None;
    }
    if upper.contains("CAFEF.INDEX") {
        Some(CafeFFileKind::Index)
    } else if upper.contains("CAFEF.SOLIEUGD.ADJUSTED")
        || (upper.contains("SOLIEUGD") && upper.contains("ADJ"))
    {
        Some(CafeFFileKind::AdjustedStocks)
    } else if upper.contains("CAFEF.SOLIEUGD.RAW")
        || (upper.contains("SOLIEUGD") && upper.contains("RAW"))
    {
        Some(CafeFFileKind::RawStocks)
    } else {
        None
    }
}

fn is_vn_weekend_timestamp_ms(timestamp_ms: i64) -> bool {
    // 1970-01-01 (day 0) was Thursday (weekday = 3 in Mon=0..Sun=6); ICT = UTC+7.
    let ict_epoch_days = (timestamp_ms.div_euclid(1_000) + 7 * 3_600).div_euclid(86_400);
    (ict_epoch_days + 3).rem_euclid(7) >= 5
}

fn normalize_cafef_ticker(raw: &str, is_index: bool) -> String {
    let upper = raw.trim().to_ascii_uppercase();
    if is_index {
        match upper.as_str() {
            "HNX-INDEX" | "HNXINDEX" => return "HNXINDEX".to_owned(),
            "VN-INDEX" | "VNINDEX" => return "VNINDEX".to_owned(),
            "UPCOM-INDEX" | "UPCOMINDEX" => return "UPCOMINDEX".to_owned(),
            _ => {}
        }
    }
    upper
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BarQueryMode;

    #[test]
    fn cafef_importer_ingests_index_and_adjusted_stocks_normalized_to_vnd() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("CafeF.INDEX.Upto06112026.csv"),
            "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n\
             VNINDEX,20261106,1713.46,1734.42,1713.46,1725.36,785347700\n\
             VNINDEX,20261105,1745.03,1753.15,1736.26,1740.73,742461800\n\
             HNX-INDEX,20261106,255.03,256.81,252.97,256.73,69064001\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("CafeF.SolieuGD.Adjusted.Upto06112026.csv"),
            "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n\
             FPT,20261106,51.4,51.8,50.8,50.8,12162200\n\
             FPT,20261105,54.5,54.6,52.8,52.9,8006200\n\
             SHS,20261106,14.2,14.5,14.0,14.4,5000000\n\
             ACV,20261106,110.0,112.0,109.0,111.5,250000\n",
        )
        .unwrap();

        let store = SqliteMarketStore::open_in_memory().unwrap();
        let importer = CafeFImporter::default();
        let summary = importer.import_dir(dir.path(), &store).unwrap();

        assert_eq!(summary.files_processed.len(), 2);
        assert_eq!(summary.distinct_symbols, 5);
        assert_eq!(summary.total_bars_written, 7);

        // FPT mapped to HOSE and normalized from thousand VND (50.8) -> raw VND (50,800).
        let fpt_key = SeriesKey::new(
            &SymbolKey::new("HOSE", "FPT").unwrap(),
            Interval::D1,
            Adjustment::Splits,
        );
        let fpt_page = store.query_bars(&fpt_key, BarQueryMode::Init, 10).unwrap();
        assert_eq!(fpt_page.bars.len(), 2);
        assert!((fpt_page.bars[1].close - 50_800.0).abs() < 1e-6);
        assert!((fpt_page.bars[1].turnover - 50_800.0 * 12_162_200.0).abs() < 1.0);
        // Timestamp must be Unix milliseconds (2026-11-06 00:00:00 UTC > 1.79e12 ms).
        assert!(fpt_page.bars[1].timestamp > 1_700_000_000_000);

        // VNINDEX stays in index points (not multiplied by 1,000).
        let vni_key = SeriesKey::new(
            &SymbolKey::new("HOSE", "VNINDEX").unwrap(),
            Interval::D1,
            Adjustment::Splits,
        );
        let vni_page = store.query_bars(&vni_key, BarQueryMode::Init, 10).unwrap();
        assert_eq!(vni_page.bars.len(), 2);
        assert!((vni_page.bars[1].close - 1725.36).abs() < 1e-6);

        // SHS mapped to HNX (±10%) and ACV mapped to UPCOM (±15%).
        let (shs_meta, _) = store
            .get_symbol(&SymbolKey::new("HNX", "SHS").unwrap())
            .unwrap()
            .expect("SHS resolved offline");
        assert_eq!(shs_meta.vn_price_band.unwrap().label, "±10%");

        let (acv_meta, _) = store
            .get_symbol(&SymbolKey::new("UPCOM", "ACV").unwrap())
            .unwrap()
            .expect("ACV resolved offline");
        assert_eq!(acv_meta.vn_price_band.unwrap().label, "±15%");
    }
}
