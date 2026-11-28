//! Wave 3 Quantitative Analytics, Volume Structure, Market Breadth/RRG, Trade Planner,
//! Synthetic DOM Ladder, Journal Analytics, and Replay Session Debrief Tauri Commands.
//!
//! Unlocks the complete `sumi-core` engine suite (`vwap`, `volume_profile`, `market_profile`,
//! `patterns`, `breadth`, `planner`, `ledger`, `ladder`, `coaching`) for the React UI.

use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sumi_core::data::Timeframe;
use sumi_core::indicators::{
    CandlestickPatternKind, MarketProfileConfig, VolumeProfileConfig, compute_anchored_vwap,
    compute_atr, compute_candlestick_patterns_and_scores, compute_session_market_profiles,
    compute_volume_profile,
};
use sumi_core::journal::{
    ChecklistState, JournalAnalytics, JournalEntry, JournalEntrySource, JournalEntryStatus,
    MistakeTag, SetupTag, TradeJournalStore, TradePlanInput, VnPriceUnit, compute_trade_plan,
};
use sumi_core::model::Candle;
use sumi_core::scanner::breadth::{
    BreadthFeedInput, MarketBreadthSnapshot, RrgQuadrant, compute_market_breadth_and_rrg,
};
use sumi_core::scanner::{QmvMoneyTier, QmvSector, QmvSectorGroup, lookup_qmv_profile};
use sumi_core::trading::{
    DomLadderConfig, DomLadderSnapshot, OrderStatus, PositionSide, PracticeDecisionLog,
    SessionDebriefReport, compute_causal_dom_ladder,
};
use sumi_data::{Adjustment, BarQueryMode, Interval, OhlcvBar, SeriesKey, SymbolKey};
use tauri::State;

use crate::commands::broker::{ensure_broker_synced, infer_venue_for_ticker};
use crate::error::{DesktopError, DesktopResult};
use crate::state::AppState;

// ============================================================================
// 1. Market Breadth, 4-Quadrant RRG & 11-Sector QMV Heatmap
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorHeatmapTileDto {
    pub ticker: String,
    pub symbol: String,
    pub sector_code: String,
    pub sector_label: String,
    pub last_close: f64,
    pub change_pct: f64,
    pub traded_value: f64,
    pub buy_flow_ratio_pct: f64,
    pub rs_ratio: f64,
    pub rs_momentum: f64,
    pub quadrant: RrgQuadrant,
    pub in_qmv30: bool,
    pub in_qmv60: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorHeatmapGroupDto {
    pub sector_code: String,
    pub sector_label: String,
    pub sector_group_label: String,
    pub stock_count: usize,
    pub advancers: usize,
    pub decliners: usize,
    pub unchanged: usize,
    pub avg_change_pct: f64,
    pub weighted_change_pct: f64,
    pub total_traded_value: f64,
    pub avg_buy_flow_pct: f64,
    pub stocks: Vec<SectorHeatmapTileDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketBreadthRrgResponse {
    pub scope: String,
    pub as_of_date: Option<String>,
    pub replay_locked: bool,
    pub snapshot: MarketBreadthSnapshot,
    pub sectors: Vec<SectorHeatmapGroupDto>,
    pub elapsed_ms: u64,
}

fn sector_group_for(sec: QmvSector) -> QmvSectorGroup {
    match sec {
        QmvSector::Banking
        | QmvSector::Brokerage
        | QmvSector::RealEstate
        | QmvSector::ConstructionSteel => QmvSectorGroup::Group1FinancialLeverage,
        QmvSector::UtilitiesDefensive => QmvSectorGroup::Group3Defensive,
        QmvSector::MarketIndex => QmvSectorGroup::MarketIndex,
        _ => QmvSectorGroup::Group2OperatingCycle,
    }
}

/// Evaluates causal Market Breadth, 4-Quadrant RRG (vs `HOSE:VNINDEX`), and 11-Sector QMV Heatmap.
/// Strictly honors active Bar Replay cutoff OR Yumeko PIT `as_of_date` (`DEF-001` zero future leak).
#[tauri::command]
pub async fn get_market_breadth_and_rrg(
    state: State<'_, AppState>,
    as_of_date: Option<String>,
    scope: Option<String>,
) -> DesktopResult<MarketBreadthRrgResponse> {
    let started = std::time::Instant::now();
    let scope_upper = scope
        .as_deref()
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "ALL_VN".to_owned());

    // Resolve causal cutoff in seconds: active Replay cutoff takes highest precedence, then PIT `as_of_date`.
    let replay_cutoff_secs: Option<i64> = {
        let rg = state.replay.lock().await;
        rg.as_ref().and_then(|rt| {
            rt.controller
                .is_active()
                .then(|| rt.controller.cutoff_timestamp(&rt.base_candles))
                .flatten()
        })
    };
    let replay_locked = replay_cutoff_secs.is_some();

    let pit_cutoff_secs = as_of_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp());
    let effective_cutoff_secs = replay_cutoff_secs.or(pit_cutoff_secs);

    let store = state.market_store.clone();
    let scope_clone = scope_upper.clone();
    let as_of_clone = as_of_date.clone();

    tokio::task::spawn_blocking(move || {
        let bench_sym = SymbolKey::new("HOSE", "VNINDEX")?;
        let bench_bars = store
            .query_bars(
                &SeriesKey::new(&bench_sym, Interval::D1, Adjustment::Splits),
                BarQueryMode::Init,
                320,
            )
            .map(|p| p.bars)
            .unwrap_or_default();
        let bench_candles: Vec<Candle> = bench_bars
            .iter()
            .map(|b| b.to_core_candle(Interval::D1))
            .collect();

        let ex_filter = match scope_clone.as_str() {
            "HOSE" | "HNX" | "UPCOM" => Some(scope_clone.as_str()),
            _ => None,
        };
        let symbols = store.search_local_symbols("", ex_filter, 2_000)?;

        struct OwnedFeed {
            ticker: String,
            symbol: String,
            exchange: String,
            candles: Vec<Candle>,
        }

        let mut owned = Vec::new();
        for s in symbols {
            if !matches!(s.exchange.as_str(), "HOSE" | "HNX" | "UPCOM")
                || s.market_type == "index"
                || s.symbol.ends_with("F1!")
                || s.symbol.starts_with("VN30F")
            {
                continue;
            }
            let prof = lookup_qmv_profile(&s.symbol);
            let keep = match scope_clone.as_str() {
                "QMV30" => prof.is_some_and(|p| p.in_qmv30),
                "QMV60" => prof.is_some_and(|p| p.in_qmv60),
                "DT1" => prof.is_some_and(|p| p.money_tier == QmvMoneyTier::Dt1),
                "DT2" => prof.is_some_and(|p| p.money_tier == QmvMoneyTier::Dt2),
                "DT3" => prof.is_some_and(|p| p.money_tier == QmvMoneyTier::Dt3),
                _ => true,
            };
            if !keep {
                continue;
            }
            let Ok(key) = SymbolKey::parse(&s.ticker) else {
                continue;
            };
            let Ok(page) = store.query_bars(
                &SeriesKey::new(&key, Interval::D1, Adjustment::Splits),
                BarQueryMode::Init,
                220,
            ) else {
                continue;
            };
            if page.bars.len() < 30 {
                continue;
            }
            let candles: Vec<Candle> = page
                .bars
                .iter()
                .map(|b| b.to_core_candle(Interval::D1))
                .collect();
            owned.push(OwnedFeed {
                ticker: s.ticker,
                symbol: s.symbol,
                exchange: s.exchange,
                candles,
            });
        }

        let feeds: Vec<BreadthFeedInput<'_>> = owned
            .iter()
            .map(|f| BreadthFeedInput {
                instrument_id: &f.ticker,
                symbol: &f.symbol,
                preset_label: &f.exchange,
                timeframe: Timeframe::D1,
                candles: &f.candles,
                exclude_incomplete_tail: false,
            })
            .collect();

        let snapshot = compute_market_breadth_and_rrg(
            &feeds,
            &bench_candles,
            Timeframe::D1,
            "VNINDEX",
            effective_cutoff_secs,
        );

        // Aggregate 11 QMV Sectors Heatmap from causal breadth rotations
        let mut by_sector: BTreeMap<String, (QmvSector, Vec<SectorHeatmapTileDto>)> =
            BTreeMap::new();
        for rot in &snapshot.rotations {
            let prof = lookup_qmv_profile(&rot.symbol);
            let sec = prof.map_or(QmvSector::OtherCyclical, |p| p.sector);
            let code = sec.code().to_owned();
            let tile = SectorHeatmapTileDto {
                ticker: rot.instrument_id.clone(),
                symbol: rot.symbol.clone(),
                sector_code: code.clone(),
                sector_label: sec.label_vi().to_owned(),
                last_close: rot.last_close,
                change_pct: rot.return_1bar_pct,
                traded_value: rot.traded_value,
                buy_flow_ratio_pct: rot.buy_flow_ratio_pct,
                rs_ratio: rot.rs_ratio,
                rs_momentum: rot.rs_momentum,
                quadrant: rot.quadrant,
                in_qmv30: prof.is_some_and(|p| p.in_qmv30),
                in_qmv60: prof.is_some_and(|p| p.in_qmv60),
            };
            by_sector
                .entry(code)
                .or_insert_with(|| (sec, Vec::new()))
                .1
                .push(tile);
        }

        let mut sectors: Vec<SectorHeatmapGroupDto> = by_sector
            .into_iter()
            .map(|(code, (sec, mut tiles))| {
                tiles.sort_by(|a, b| {
                    b.traded_value
                        .partial_cmp(&a.traded_value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let stock_count = tiles.len();
                let advancers = tiles.iter().filter(|t| t.change_pct > 0.05).count();
                let decliners = tiles.iter().filter(|t| t.change_pct < -0.05).count();
                let unchanged = stock_count.saturating_sub(advancers + decliners);
                let total_val: f64 = tiles.iter().map(|t| t.traded_value.max(0.0)).sum();
                let avg_change = if stock_count > 0 {
                    tiles.iter().map(|t| t.change_pct).sum::<f64>() / stock_count as f64
                } else {
                    0.0
                };
                let weighted_change = if total_val > 1.0 {
                    tiles
                        .iter()
                        .map(|t| t.change_pct * t.traded_value.max(0.0))
                        .sum::<f64>()
                        / total_val
                } else {
                    avg_change
                };
                let avg_flow = if stock_count > 0 {
                    tiles.iter().map(|t| t.buy_flow_ratio_pct).sum::<f64>() / stock_count as f64
                } else {
                    50.0
                };
                SectorHeatmapGroupDto {
                    sector_code: code,
                    sector_label: sec.label_vi().to_owned(),
                    sector_group_label: sector_group_for(sec).label_vi().to_owned(),
                    stock_count,
                    advancers,
                    decliners,
                    unchanged,
                    avg_change_pct: avg_change,
                    weighted_change_pct: weighted_change,
                    total_traded_value: total_val,
                    avg_buy_flow_pct: avg_flow,
                    stocks: tiles,
                }
            })
            .collect();

        sectors.sort_by(|a, b| {
            b.total_traded_value
                .partial_cmp(&a.total_traded_value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(MarketBreadthRrgResponse {
            scope: scope_clone,
            as_of_date: as_of_clone,
            replay_locked,
            snapshot,
            sectors,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

// ============================================================================
// 2. Volume Structure (VRVP/FRVP, TPO Market Profile, AVWAP, 12 Patterns)
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeProfileBinDto {
    pub price_low: f64,
    pub price_high: f64,
    pub price_mid: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub in_value_area: bool,
    pub is_poc: bool,
    pub is_hvn: bool,
    pub is_lvn: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeProfileDto {
    pub range_low: f64,
    pub range_high: f64,
    pub poc_price: f64,
    pub vah_price: f64,
    pub val_price: f64,
    pub total_volume: f64,
    pub total_buy_volume: f64,
    pub total_sell_volume: f64,
    pub net_delta: f64,
    pub uses_tpo_fallback: bool,
    pub bins: Vec<VolumeProfileBinDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TpoSessionDto {
    pub start_timestamp_ms: i64,
    pub end_timestamp_ms: i64,
    pub poc_price: f64,
    pub vah_price: f64,
    pub val_price: f64,
    pub ib_high: f64,
    pub ib_low: f64,
    pub is_naked_poc: bool,
    pub naked_poc_end_timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternHitDto {
    pub timestamp_ms: i64,
    pub price: f64,
    pub badge: String,
    pub name: String,
    pub is_bullish: bool,
    pub candle_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvwapPointDto {
    pub timestamp_ms: i64,
    pub vwap: f64,
    pub upper1: f64,
    pub lower1: f64,
    pub upper2: f64,
    pub lower2: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeStructureResponse {
    pub ticker: String,
    pub interval: String,
    pub volume_profile: Option<VolumeProfileDto>,
    pub tpo_sessions: Vec<TpoSessionDto>,
    pub avwap: Vec<AvwapPointDto>,
    pub pattern_hits: Vec<PatternHitDto>,
}

/// Pure computation helper for Volume Profile (`VRVP`/`FRVP`), Session TPO, Anchored VWAP,
/// and 12 Japanese Candlestick Patterns — shared across live tickers, Blind Drill (`"MÃ ẨN #XXXX"`),
/// and synthetic ratio/spread expressions (`W6-03` & `W6-05`).
pub(crate) fn compute_volume_structure_from_bars(
    norm_ticker: String,
    iv: Interval,
    bars: &[OhlcvBar],
    row_count: usize,
    start_timestamp_ms: Option<i64>,
    end_timestamp_ms: Option<i64>,
    anchor_timestamp_ms: Option<i64>,
) -> VolumeStructureResponse {
    let tf = iv.to_core_timeframe().unwrap_or(Timeframe::D1);
    let iv_str = iv.as_str().to_owned();
    let candles: Vec<Candle> = bars.iter().map(|b| b.to_core_candle(iv)).collect();
    if candles.is_empty() {
        return VolumeStructureResponse {
            ticker: norm_ticker,
            interval: iv_str,
            volume_profile: None,
            tpo_sessions: Vec::new(),
            avwap: Vec::new(),
            pattern_hits: Vec::new(),
        };
    }

    let vp_slice: Vec<Candle> = candles
        .iter()
        .filter(|c| {
            let ts_ms = c.timestamp * 1_000;
            start_timestamp_ms.is_none_or(|s| ts_ms >= s)
                && end_timestamp_ms.is_none_or(|e| ts_ms <= e)
        })
        .cloned()
        .collect();
    let vp_input = if vp_slice.is_empty() {
        &candles[..]
    } else {
        &vp_slice[..]
    };

    let volume_profile = compute_volume_profile(
        vp_input,
        VolumeProfileConfig {
            row_count,
            value_area_pct: 0.70,
        },
    )
    .map(|vp| VolumeProfileDto {
        range_low: vp.range_low,
        range_high: vp.range_high,
        poc_price: vp.poc_price,
        vah_price: vp.vah_price,
        val_price: vp.val_price,
        total_volume: vp.total_volume,
        total_buy_volume: vp.total_buy_volume,
        total_sell_volume: vp.total_sell_volume,
        net_delta: vp.net_delta,
        uses_tpo_fallback: vp.uses_tpo_fallback,
        bins: vp
            .bins
            .into_iter()
            .map(|b| VolumeProfileBinDto {
                price_low: b.price_low,
                price_high: b.price_high,
                price_mid: b.price_mid,
                buy_volume: b.buy_volume,
                sell_volume: b.sell_volume,
                total_volume: b.total_volume,
                in_value_area: b.in_value_area,
                is_poc: b.is_poc,
                is_hvn: b.is_hvn,
                is_lvn: b.is_lvn,
            })
            .collect(),
    });

    let tpo_cfg = MarketProfileConfig {
        utc_offset_seconds: 7 * 3_600,
        num_bins: 24,
        max_sessions: 12,
        ..MarketProfileConfig::default()
    };
    let tpo_out = compute_session_market_profiles(&candles, tf, tpo_cfg);
    let tpo_sessions: Vec<TpoSessionDto> = tpo_out
        .profiles
        .into_iter()
        .map(|p| TpoSessionDto {
            start_timestamp_ms: candles[p.start_index.min(candles.len() - 1)].timestamp * 1_000,
            end_timestamp_ms: candles[p.end_index.min(candles.len() - 1)].timestamp * 1_000,
            poc_price: p.poc_price,
            vah_price: p.vah_price,
            val_price: p.val_price,
            ib_high: p.ib_high,
            ib_low: p.ib_low,
            is_naked_poc: p.naked_poc_mitigated_index.is_none(),
            naked_poc_end_timestamp_ms: candles[p.naked_poc_end_index.min(candles.len() - 1)]
                .timestamp
                * 1_000,
        })
        .collect();

    let mut avwap = Vec::new();
    if let Some(anchor_ms) = anchor_timestamp_ms {
        let anchor_sec = anchor_ms / 1_000;
        let anchor_idx = candles
            .iter()
            .position(|c| c.timestamp >= anchor_sec)
            .unwrap_or(candles.len().saturating_sub(1));
        let bands = compute_anchored_vwap(&candles, anchor_idx, 1.0, 2.0);
        for (i, bar) in candles.iter().enumerate().skip(anchor_idx) {
            if let Some(v) = bands.vwap[i] {
                avwap.push(AvwapPointDto {
                    timestamp_ms: bar.timestamp * 1_000,
                    vwap: v,
                    upper1: bands.upper_1[i].unwrap_or(v),
                    lower1: bands.lower_1[i].unwrap_or(v),
                    upper2: bands.upper_2[i].unwrap_or(v),
                    lower2: bands.lower_2[i].unwrap_or(v),
                });
            }
        }
    }

    let atr_14 = compute_atr(&candles, 14);
    let (_geoms, pat_bars) = compute_candlestick_patterns_and_scores(&candles, &atr_14);
    let start_p = candles.len().saturating_sub(120);
    let mut pattern_hits = Vec::new();
    for i in start_p..candles.len() {
        let mask = pat_bars[i].active_mask;
        if mask == 0 {
            continue;
        }
        for kind in CandlestickPatternKind::ALL {
            if (mask & kind.bit()) != 0 {
                pattern_hits.push(PatternHitDto {
                    timestamp_ms: candles[i].timestamp * 1_000,
                    price: if kind.is_bullish() {
                        candles[i].low
                    } else {
                        candles[i].high
                    },
                    badge: kind.short_badge().to_owned(),
                    name: kind.short_snake_name().to_owned(),
                    is_bullish: kind.is_bullish(),
                    candle_score: pat_bars[i].candle_score,
                });
            }
        }
    }

    VolumeStructureResponse {
        ticker: norm_ticker,
        interval: iv_str,
        volume_profile,
        tpo_sessions,
        avwap,
        pattern_hits,
    }
}

/// Computes `sumi-core` Volume Profile (`VRVP`/`FRVP`), Session TPO Market Profile,
/// Anchored VWAP (`±1σ/2σ`), and 12 Japanese Candlestick Patterns causally.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn compute_volume_structure(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    rows: Option<usize>,
    start_timestamp_ms: Option<i64>,
    end_timestamp_ms: Option<i64>,
    anchor_timestamp_ms: Option<i64>,
) -> DesktopResult<VolumeStructureResponse> {
    let iv: Interval = interval.parse()?;
    let row_count = rows.unwrap_or(32).clamp(8, 120);

    // 1. Check active Bar Replay (including unrevealed True Blind Drill `"MÃ ẨN #XXXX"`, `W6-03`) BEFORE `SymbolKey::parse`!
    let blind_or_replay: Option<(String, Vec<OhlcvBar>)> = {
        let rg = state.replay.lock().await;
        rg.as_ref().and_then(|rt| {
            if !rt.controller.is_active() {
                return None;
            }
            if let Some(ref bd) = rt.blind_drill
                && bd.enabled
                && !bd.revealed
                && (bd.alias_code == ticker || rt.ticker.eq_ignore_ascii_case(&ticker))
            {
                let bars =
                    crate::commands::replay::materialize_replay_bars(rt, iv, &state.market_store);
                return Some((bd.alias_code.clone(), bars));
            }
            if rt.ticker.eq_ignore_ascii_case(ticker.trim()) {
                let bars =
                    crate::commands::replay::materialize_replay_bars(rt, iv, &state.market_store);
                return Some((rt.ticker.clone(), bars));
            }
            None
        })
    };

    if let Some((norm_ticker, bars)) = blind_or_replay {
        return tokio::task::spawn_blocking(move || {
            Ok(compute_volume_structure_from_bars(
                norm_ticker,
                iv,
                &bars,
                row_count,
                start_timestamp_ms,
                end_timestamp_ms,
                anchor_timestamp_ms,
            ))
        })
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?;
    }

    // 2. Support synthetic ratio/spread expressions (`HOSE:SSI / HOSE:VNINDEX`, `(HOSE:FPT + HOSE:MWG) / 2`, `W6-05`)
    if sumi_data::is_synthetic_expression(&ticker) {
        let parsed = sumi_data::parse_expression(&ticker)?;
        let norm_ticker = parsed.canonical.clone();
        let bars = crate::commands::market::load_expression_bars(
            &state,
            &parsed,
            iv,
            Adjustment::Splits,
            600,
            None,
        )
        .await?;
        return tokio::task::spawn_blocking(move || {
            Ok(compute_volume_structure_from_bars(
                norm_ticker,
                iv,
                &bars,
                row_count,
                start_timestamp_ms,
                end_timestamp_ms,
                anchor_timestamp_ms,
            ))
        })
        .await
        .map_err(|e| DesktopError::Internal(e.to_string()))?;
    }

    // 3. Standard cached symbol path
    let sym = SymbolKey::parse(&ticker)?;
    let store = state.market_store.clone();
    let norm_ticker = sym.ticker();

    tokio::task::spawn_blocking(move || {
        let bars = store
            .query_bars(
                &SeriesKey::new(&sym, iv, Adjustment::Splits),
                BarQueryMode::Init,
                600,
            )
            .map(|p| p.bars)
            .unwrap_or_default();
        Ok(compute_volume_structure_from_bars(
            norm_ticker,
            iv,
            &bars,
            row_count,
            start_timestamp_ms,
            end_timestamp_ms,
            anchor_timestamp_ms,
        ))
    })
    .await
    .map_err(|e| DesktopError::Internal(e.to_string()))?
}

// ============================================================================
// 3. Trade Planner (`sumi_core::journal::compute_trade_plan`)
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePlanRequestDto {
    pub ticker: String,
    pub side: String,
    pub account_equity: f64,
    pub available_cash: f64,
    pub risk_pct: f64,
    pub max_position_pct: Option<f64>,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub target_price: f64,
    pub reference_price: Option<f64>,
    pub commission_bps: Option<f64>,
    pub tax_sell_bps: Option<f64>,
    pub slippage_bps: Option<f64>,
    /// Optional explicit entry timestamp (ms) for accurate historical T+2/T+3 settlement (`W6-04`).
    #[serde(default)]
    pub entry_timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePlanResponseDto {
    pub ticker: String,
    pub venue: String,
    pub side: String,
    pub snapped_entry: f64,
    pub snapped_stop: f64,
    pub snapped_target: f64,
    pub breakeven_price: f64,
    pub max_risk_amount: f64,
    pub planned_quantity: f64,
    pub position_notional: f64,
    pub margin_required: f64,
    pub planned_risk_amount: f64,
    pub planned_risk_pct_actual: f64,
    pub estimated_net_profit: f64,
    pub estimated_round_trip_friction: f64,
    pub gross_r_multiple: f64,
    pub net_r_multiple: f64,
    pub required_win_rate_pct: f64,
    pub settlement_timestamp_ms: Option<i64>,
    pub affordable: bool,
    pub warnings: Vec<String>,
}

pub(crate) fn compute_trade_plan_inner(
    input: TradePlanRequestDto,
    replay_cutoff_secs: Option<i64>,
    is_blind_drill_masked: bool,
) -> DesktopResult<TradePlanResponseDto> {
    let trimmed_ticker = input.ticker.trim();
    let is_masked_blind = is_blind_drill_masked || trimmed_ticker.starts_with("MÃ ẨN");
    let venue = if is_masked_blind {
        // Match `reset_replay_broker_for_blind_drill` (`0.01` decimal tick, `1` share lot on 100.00-normalized scale)
        sumi_core::journal::ExchangeVenue::UsEquities
    } else {
        infer_venue_for_ticker(trimmed_ticker)
    };
    let side = if input.side.trim().eq_ignore_ascii_case("short")
        || input.side.trim().eq_ignore_ascii_case("sell")
    {
        PositionSide::Short
    } else {
        PositionSide::Long
    };
    let is_vn30f = venue == sumi_core::journal::ExchangeVenue::Vn30Futures;
    let price_unit = if is_masked_blind {
        VnPriceUnit::Auto
    } else if is_vn30f {
        VnPriceUnit::ThousandVnd
    } else if venue.is_vn_cash_equity() {
        VnPriceUnit::RawVnd
    } else {
        VnPriceUnit::Auto
    };

    // Never leak real historical dates via `settlement_timestamp_ms` while Blind Drill is unrevealed (`M-01`)!
    let effective_entry_secs = if is_masked_blind {
        None
    } else {
        Some(
            input
                .entry_timestamp_ms
                .map(|ms| ms.div_euclid(1_000))
                .or(replay_cutoff_secs)
                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
        )
    };

    let default_comm_bps = if is_masked_blind {
        15.0
    } else {
        venue.default_commission_bps()
    };
    let default_tax_bps = if is_masked_blind {
        10.0
    } else {
        venue.default_tax_sell_bps()
    };

    let core_in = TradePlanInput {
        venue,
        price_unit,
        side,
        account_equity: input.account_equity,
        available_cash: input.available_cash,
        leverage: 1.0,
        contract_multiplier: if is_vn30f { 100_000.0 } else { 1.0 },
        initial_margin_pct: venue.default_initial_margin_pct(),
        max_position_pct: input.max_position_pct.unwrap_or(100.0),
        risk_pct: input.risk_pct,
        entry_price: input.entry_price,
        stop_loss: input.stop_loss,
        target_price: input.target_price,
        reference_price: if is_masked_blind {
            None
        } else {
            input.reference_price
        },
        commission_bps: input.commission_bps.unwrap_or(default_comm_bps),
        tax_sell_bps: input.tax_sell_bps.unwrap_or(default_tax_bps),
        slippage_bps: input.slippage_bps.unwrap_or(2.0),
        snap_to_ticks: true,
        entry_timestamp: effective_entry_secs,
    };

    let plan =
        compute_trade_plan(&core_in).map_err(|e| DesktopError::Validation(e.message().to_owned()))?;

    Ok(TradePlanResponseDto {
        ticker: if is_masked_blind {
            trimmed_ticker.to_owned()
        } else {
            trimmed_ticker.to_ascii_uppercase()
        },
        venue: if is_masked_blind {
            "BLIND".to_owned()
        } else {
            venue.short_label().to_owned()
        },
        side: match plan.side {
            PositionSide::Long => "LONG".to_owned(),
            PositionSide::Short => "SHORT".to_owned(),
        },
        snapped_entry: plan.snapped_entry,
        snapped_stop: plan.snapped_stop,
        snapped_target: plan.snapped_target,
        breakeven_price: plan.breakeven_price,
        max_risk_amount: plan.max_risk_amount,
        planned_quantity: plan.planned_quantity,
        position_notional: plan.position_notional,
        margin_required: plan.margin_required,
        planned_risk_amount: plan.planned_risk_amount,
        planned_risk_pct_actual: plan.planned_risk_pct_actual,
        estimated_net_profit: plan.estimated_net_profit,
        estimated_round_trip_friction: plan.estimated_round_trip_friction,
        gross_r_multiple: plan.gross_r_multiple,
        net_r_multiple: plan.net_r_multiple,
        required_win_rate_pct: plan.required_win_rate_pct,
        settlement_timestamp_ms: if is_masked_blind {
            None
        } else {
            plan.settlement_timestamp.map(|s| s * 1_000)
        },
        affordable: plan.affordable,
        warnings: plan.warnings.into_iter().map(|w| w.description()).collect(),
    })
}

#[tauri::command]
pub async fn compute_trade_plan_cmd(
    state: State<'_, AppState>,
    input: TradePlanRequestDto,
) -> DesktopResult<TradePlanResponseDto> {
    let req_ticker = input.ticker.trim().to_owned();
    let (replay_cutoff_secs, is_blind_masked) = {
        let rg = state.replay.lock().await;
        match rg.as_ref() {
            Some(rt) if rt.controller.is_active() => {
                let masked = rt.blind_drill.as_ref().is_some_and(|b| {
                    b.enabled
                        && !b.revealed
                        && (b.alias_code == req_ticker
                            || rt.ticker.eq_ignore_ascii_case(&req_ticker))
                });
                if masked {
                    (None, true)
                } else {
                    let idx = rt
                        .controller
                        .base_cursor_index
                        .min(rt.base_bars.len().saturating_sub(1));
                    let ts_ms = rt
                        .raw_base_bars
                        .as_ref()
                        .and_then(|raw| raw.get(idx))
                        .or_else(|| rt.base_bars.get(idx))
                        .map(|b| b.timestamp);
                    (ts_ms.map(|ms| ms.div_euclid(1_000)), false)
                }
            }
            _ => (None, false),
        }
    };
    compute_trade_plan_inner(input, replay_cutoff_secs, is_blind_masked)
}

// ============================================================================
// 4. Synthetic DOM Ladder (`OHLCV_SYNTH_DOM_PROXY`)
// ============================================================================

#[tauri::command]
pub async fn compute_dom_ladder_cmd(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
    half_rows: Option<usize>,
    lookback_bars: Option<usize>,
) -> DesktopResult<DomLadderSnapshot> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let cutoff_ts = broker.base_candles.get(idx).map(|c| c.timestamp);
    let venue = broker.session.config.venue;
    let vn_unit = if venue == sumi_core::journal::ExchangeVenue::Vn30Futures {
        VnPriceUnit::ThousandVnd
    } else if venue.is_vn_cash_equity() {
        VnPriceUnit::RawVnd
    } else {
        VnPriceUnit::Auto
    };
    let cfg = DomLadderConfig {
        venue,
        vn_unit,
        half_rows: half_rows.unwrap_or(8).clamp(4, 20),
        lookback_bars: lookback_bars.unwrap_or(100).clamp(10, 300),
        center_price_override: None,
        reference_price_override: None,
    };
    let pending: Vec<_> = broker
        .session
        .orders()
        .iter()
        .filter(|o| matches!(o.status, OrderStatus::Pending))
        .cloned()
        .collect();
    compute_causal_dom_ladder(
        &broker.base_candles,
        &cfg,
        &pending,
        broker.session.open_position(),
        cutoff_ts,
    )
    .map_err(|e| DesktopError::Validation(format!("Không thể tạo DOM Ladder: {e:?}")))
}

// ============================================================================
// 5. Structured Trade Journal Analytics & Replay Session Debrief
// ============================================================================

fn parse_setup_tag(raw: &str) -> SetupTag {
    match raw.trim().to_ascii_lowercase().as_str() {
        "trendpullback" | "pullback" => SetupTag::TrendPullback,
        "squeezebreakout" | "squeeze bo" | "breakout" => SetupTag::SqueezeBreakout,
        "meanreversion" | "mean rev" => SetupTag::MeanReversion,
        "supportbounce" | "s/r bounce" | "support" => SetupTag::SupportBounce,
        "momentumcontinuation" | "momentum" => SetupTag::MomentumContinuation,
        "liquiditysweep" | "liq sweep" => SetupTag::LiquiditySweep,
        _ => SetupTag::Untagged,
    }
}

fn parse_mistake_tag(raw: &str) -> MistakeTag {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fomochase" | "fomo chase" | "fomo" => MistakeTag::FomoChase,
        "movedstoploss" | "moved stop" => MistakeTag::MovedStopLoss,
        "earlyexit" | "early exit" => MistakeTag::EarlyExit,
        "oversizedrisk" | "oversized" => MistakeTag::OversizedRisk,
        "countertrendimpulse" | "counter-trend" => MistakeTag::CounterTrendImpulse,
        "ignoreddailyband" | "band / t+2 violation" => MistakeTag::IgnoredDailyBand,
        "skippedchecklist" | "no checklist" => MistakeTag::SkippedChecklist,
        _ => MistakeTag::None,
    }
}

fn parse_checklist(val: Option<&serde_json::Value>, has_sl: bool) -> ChecklistState {
    val.and_then(|v| serde_json::from_value::<ChecklistState>(v.clone()).ok())
        .unwrap_or_else(|| ChecklistState::automated_import_default(has_sl))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEquityPointDto {
    pub trade_index: usize,
    pub timestamp_ms: i64,
    pub cumulative_pnl: f64,
    pub trade_pnl: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalAnalyticsResponseDto {
    pub ticker: String,
    pub analytics: JournalAnalytics,
    pub equity_points: Vec<JournalEquityPointDto>,
}

fn hydrate_journal_entries(
    state: &AppState,
    broker: &crate::state::PaperBrokerRuntime,
) -> (TradeJournalStore, Vec<JournalEntry>) {
    let anns: HashMap<u64, crate::workspace_db::JournalAnnotationRecord> = state
        .workspace
        .list_journal_annotations(&broker.ticker)
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.trade_id, a))
        .collect();

    let mut store = TradeJournalStore::new();
    let venue = broker.session.config.venue;
    for tr in broker.session.closed_trades() {
        let ann = anns.get(&tr.trade_id);
        let setup = ann.map_or(SetupTag::Untagged, |a| parse_setup_tag(&a.setup_tag));
        let mistake = ann.map_or(MistakeTag::None, |a| parse_mistake_tag(&a.mistake_tag));
        let checklist = parse_checklist(
            ann.and_then(|a| a.checklist_json.as_ref()),
            tr.had_initial_sl_at_open,
        );
        let notes = ann.map_or_else(String::new, |a| a.note.clone());
        let entry = JournalEntry {
            id: tr.trade_id.max(1),
            source: JournalEntrySource::PaperTrade {
                symbol: broker.ticker.clone(),
                trade_id: tr.trade_id,
                entry_timestamp: tr.entry_timestamp,
            },
            status: JournalEntryStatus::Closed,
            symbol: broker.ticker.clone(),
            venue,
            side: tr.side,
            created_at_timestamp: tr.entry_timestamp,
            entry_timestamp: tr.entry_timestamp,
            exit_timestamp: Some(tr.exit_timestamp),
            planned_entry: Some(tr.entry_price),
            planned_stop: tr.initial_stop_loss,
            planned_target: None,
            planned_quantity: Some(tr.quantity),
            planned_net_r: None,
            executed_entry: Some(tr.entry_price),
            executed_exit: Some(tr.exit_price),
            executed_quantity: Some(tr.quantity),
            gross_pnl: tr.gross_pnl,
            fees_and_taxes: tr.total_fees,
            net_pnl: tr.net_pnl,
            realized_r: tr.r_multiple,
            setup,
            mistake,
            conviction: 3,
            checklist,
            notes,
        };
        store.entries.push(entry);
    }
    store.sanitize_deserialized();
    let entries = store.entries.clone();
    (store, entries)
}

#[tauri::command]
pub async fn broker_get_journal_analytics(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
) -> DesktopResult<JournalAnalyticsResponseDto> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let cutoff = replay_linked
        .then(|| broker.base_candles.get(idx).map(|c| c.timestamp))
        .flatten();
    let (store, _) = hydrate_journal_entries(&state, &broker);
    let analytics = store.compute_analytics(cutoff);

    let mut cum = 0.0_f64;
    let equity_points: Vec<JournalEquityPointDto> = broker
        .session
        .closed_trades()
        .iter()
        .enumerate()
        .map(|(i, tr)| {
            cum += tr.net_pnl;
            JournalEquityPointDto {
                trade_index: i + 1,
                timestamp_ms: tr.exit_timestamp * 1_000,
                cumulative_pnl: cum,
                trade_pnl: tr.net_pnl,
            }
        })
        .collect();

    Ok(JournalAnalyticsResponseDto {
        ticker: broker.ticker.clone(),
        analytics,
        equity_points,
    })
}

#[tauri::command]
pub async fn compute_replay_debrief_cmd(
    state: State<'_, AppState>,
    ticker: String,
    interval: String,
) -> DesktopResult<SessionDebriefReport> {
    let replay_linked = ensure_broker_synced(&state, &ticker, &interval).await?;
    let broker = if replay_linked {
        state.replay_broker.lock().await
    } else {
        state.broker.lock().await
    };
    let idx = broker
        .cursor_index
        .min(broker.base_candles.len().saturating_sub(1));
    let mark_price = broker
        .base_candles
        .get(idx)
        .map_or(0.0, |c| c.close);
    let cutoff = replay_linked
        .then(|| broker.base_candles.get(idx).map(|c| c.timestamp))
        .flatten();
    let tf = broker.interval.to_core_timeframe().unwrap_or(Timeframe::D1);
    let (_, entries) = hydrate_journal_entries(&state, &broker);
    let decision_log = PracticeDecisionLog::new();
    Ok(SessionDebriefReport::build(
        &broker.ticker,
        tf,
        &broker.session,
        mark_price,
        &entries,
        &decision_log,
        replay_linked,
        idx,
        cutoff,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_plan_computes_hose_board_lot_and_net_rr() {
        // Use a historical Replay date (2024-03-15) to verify settlement_timestamp_ms tracks entry_timestamp_ms
        let hist_ms = 1_710_460_800_000_i64; // 2024-03-15T00:00:00Z
        let res = compute_trade_plan_inner(
            TradePlanRequestDto {
                ticker: "HOSE:FPT".to_owned(),
                side: "LONG".to_owned(),
                account_equity: 500_000_000.0,
                available_cash: 500_000_000.0,
                risk_pct: 1.0,
                max_position_pct: Some(50.0),
                entry_price: 130_000.0,
                stop_loss: 124_000.0,
                target_price: 145_000.0,
                reference_price: Some(129_500.0),
                commission_bps: None,
                tax_sell_bps: None,
                slippage_bps: Some(2.0),
                entry_timestamp_ms: Some(hist_ms),
            },
            None,
            false,
        )
        .unwrap();

        assert_eq!(res.venue, "HOSE");
        assert!(res.planned_quantity >= 100.0);
        assert_eq!((res.planned_quantity as i64) % 100, 0);
        assert!(res.net_r_multiple > 1.8 && res.net_r_multiple < 2.6);
        assert!(res.planned_risk_amount <= 5_000_000.0 + 1.0);
        if let Some(settle_ms) = res.settlement_timestamp_ms {
            assert!(
                settle_ms >= hist_ms && settle_ms <= hist_ms + 7 * 86_400_000,
                "Settlement must align with historical entry date, got {settle_ms}"
            );
        }
    }

    #[test]
    fn wave6_volume_structure_runs_on_blind_alias_and_synthetic_bars() {
        let base_ms = 946_857_600_000_i64;
        let bars: Vec<OhlcvBar> = (0..40_i64)
            .map(|i| {
                let p = 98.0 + (i as f64) * 0.15;
                OhlcvBar::new(base_ms + i * 86_400_000, p - 0.5, p + 1.0, p - 1.0, p, 250_000.0)
            })
            .collect();
        let res = compute_volume_structure_from_bars(
            "MÃ ẨN #4242".to_owned(),
            Interval::D1,
            &bars,
            24,
            None,
            None,
            Some(bars[5].timestamp),
        );
        assert_eq!(res.ticker, "MÃ ẨN #4242");
        let vp = res.volume_profile.expect("volume profile computed on Blind Drill alias");
        assert!(vp.poc_price > 95.0 && vp.poc_price < 110.0);
        assert!(!res.avwap.is_empty(), "AVWAP computed cleanly on Blind Drill alias");
    }

    #[test]
    fn wave6_1_trade_plan_blind_drill_preserves_decimal_ticks_and_hides_settlement_date() {
        let leaked_hist_secs = 1_668_556_800_i64; // 2022-11-16T00:00:00Z
        let res = compute_trade_plan_inner(
            TradePlanRequestDto {
                ticker: "MÃ ẨN #4242".to_owned(),
                side: "LONG".to_owned(),
                account_equity: 100_000.0,
                available_cash: 100_000.0,
                risk_pct: 1.0,
                max_position_pct: Some(50.0),
                entry_price: 100.25,
                stop_loss: 97.53,
                target_price: 106.80,
                reference_price: Some(100.00),
                commission_bps: None,
                tax_sell_bps: None,
                slippage_bps: Some(2.0),
                entry_timestamp_ms: None,
            },
            Some(leaked_hist_secs),
            true,
        )
        .unwrap();

        assert_eq!(res.venue, "BLIND");
        assert_eq!(res.ticker, "MÃ ẨN #4242");
        assert!(
            (res.snapped_entry - 100.25).abs() < 1e-6,
            "Blind drill must preserve 0.01 decimal tick instead of snapping to 10 VND: got {}",
            res.snapped_entry
        );
        assert!(
            (res.snapped_stop - 97.53).abs() < 1e-6,
            "Blind drill stop must preserve 0.01 decimal tick: got {}",
            res.snapped_stop
        );
        assert!(
            res.planned_quantity > 100.0 && (res.planned_quantity % 100.0).abs() > 0.5,
            "Blind drill must size to 1-share increments instead of 100-share HOSE board lot: got {}",
            res.planned_quantity
        );
        assert!(
            res.settlement_timestamp_ms.is_none(),
            "Blind drill must never leak historical settlement_timestamp_ms while unrevealed"
        );
    }
}
