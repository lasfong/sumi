//! Whole-Market Money Flow Blackbox (`MarketBBAggregator`) & Independent Flow Breadth Engine
//! (`Milestone R18 / Wave 17` — `B4`).
//!
//! Strictly enforces:
//! - `AT09` Aggregate-Before-Ratio: Sums constituent active buy and sell trading values across
//!   `UniverseResolver`-eligible members on each session BEFORE computing `100 * rolling_buy / rolling_total`.
//! - `AT10` Anti-Averaging Principle: Market Money Flow is strictly trading-value weighted, never
//!   an unweighted arithmetic average of symbol `ProxyBB` scores.
//! - `AT11` Point-in-Time (`PIT`) Constituent Boundary Enforcement.
//! - `AT17` Independent Count & Trading-Value Flow Breadth (`> 50 + buf`, `< 50 - buf`, neutral).
//! - Publication Gate (`CanonicalPublished` vs `ResearchRetrospective` vs `UnavailableDegraded`).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::data::Timeframe;
use crate::model::Candle;

pub use crate::indicators::proxy_bb::{
    BBDirection, BBHorizon, BBRegime, DecomposedBarFlow, PROXY_BB_FIXED_BOUNDS_HINT,
    PROXY_BB_METHODOLOGY_VERSION, PROXY_BB_REFERENCE_LEVELS, ProxyBBBarPoint, ProxyBBHorizonPoint,
    ProxyBBSeriesOutput, compute_proxy_bb_series, decompose_bar_ohlcv_proxy,
};

use super::universe::{
    CoverageStatus, SessionDate, UniverseCoverageReport, UniverseDefinition, UniverseMode,
    UniverseResolver, UniverseStatus,
};

pub const MARKET_BB_METHODOLOGY_VERSION: &str = "bb_v1_market_aggregate";

/// Governance publication gate status for market-wide Money Flow metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketPublicationStatus {
    CanonicalPublished,
    ResearchRetrospective,
    UnavailableDegraded,
}

impl MarketPublicationStatus {
    #[inline]
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::CanonicalPublished => "CANONICAL_PUBLISHED",
            Self::ResearchRetrospective => "RESEARCH_RETROSPECTIVE",
            Self::UnavailableDegraded => "UNAVAILABLE_DEGRADED",
        }
    }
}

/// Rolling whole-market value-weighted `ProxyBB` point for one horizon (`AT09`, `AT10`).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct MarketBBHorizonPoint {
    pub horizon: BBHorizon,
    pub market_bb_value: Option<f64>,
    pub market_buy_value: f64,
    pub market_sell_value: f64,
    pub market_total_value: f64,
    pub market_oib: Option<f64>,
    pub is_warmup: bool,
    pub direction: BBDirection,
    pub regime: BBRegime,
}

/// Independent constituent count & trading-value participation breadth for one horizon (`AT17`).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct FlowBreadthHorizonPoint {
    pub horizon: BBHorizon,
    pub positive_count: usize,
    pub neutral_count: usize,
    pub negative_count: usize,
    pub total_count: usize,
    pub positive_count_ratio: f64,
    pub neutral_count_ratio: f64,
    pub negative_count_ratio: f64,
    pub positive_value: f64,
    pub neutral_value: f64,
    pub negative_value: f64,
    pub total_value: f64,
    pub positive_value_ratio: f64,
    pub neutral_value_ratio: f64,
    pub negative_value_ratio: f64,
    pub buffer: f64,
}

/// Complete whole-market Money Flow & Flow Breadth snapshot for one session date.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketBBSessionPoint {
    pub date: SessionDate,
    pub horizons: [MarketBBHorizonPoint; 6],
    pub breadth: [FlowBreadthHorizonPoint; 6],
    pub symbol_coverage: f64,
    pub reporting_symbols_count: usize,
    pub total_target_count: usize,
    pub coverage_status: CoverageStatus,
}

impl MarketBBSessionPoint {
    #[inline]
    #[must_use]
    pub const fn horizon(&self, h: BBHorizon) -> MarketBBHorizonPoint {
        self.horizons[h.index()]
    }

    #[inline]
    #[must_use]
    pub const fn breadth_at(&self, h: BBHorizon) -> FlowBreadthHorizonPoint {
        self.breadth[h.index()]
    }
}

/// Input feed descriptor for [`MarketBBAggregator::calculate_market_bb`].
#[derive(Debug, Clone, Copy)]
pub struct MarketBBFeedInput<'a> {
    pub symbol: &'a str,
    pub timeframe: Timeframe,
    pub candles: &'a [Candle],
}

/// Complete series result returned by [`MarketBBAggregator::calculate_market_bb`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketBBSeriesResult {
    pub universe_id: String,
    pub universe_version: String,
    pub universe_mode: UniverseMode,
    pub is_point_in_time: bool,
    pub publication_status: MarketPublicationStatus,
    pub methodology_version: &'static str,
    pub survivor_bias_caveat: Option<String>,
    pub points: Vec<MarketBBSessionPoint>,
    pub gate_reasons: Vec<String>,
}

pub struct MarketBBAggregator;

impl MarketBBAggregator {
    /// Evaluates the market-level publication gate (`ARCHIVE/backend/app/domain/bb/market_aggregator.py:86-127`).
    #[must_use]
    pub fn evaluate_publication_gate(
        universe: &UniverseDefinition,
        eval_mode: UniverseMode,
        coverage_report: &UniverseCoverageReport,
    ) -> (MarketPublicationStatus, Vec<String>) {
        let mut reasons = Vec::new();
        if coverage_report.status == CoverageStatus::Failed || coverage_report.target_count == 0 {
            reasons.push(format!(
                "Coverage failed minimum threshold ({:.1}% available: {}/{}).",
                coverage_report.coverage_ratio * 100.0,
                coverage_report.available_count,
                coverage_report.target_count
            ));
            return (MarketPublicationStatus::UnavailableDegraded, reasons);
        }
        if eval_mode == UniverseMode::RetrospectiveFixed {
            reasons.push(
                "Evaluated under RETROSPECTIVE_FIXED mode; carries survivorship bias and cannot be published as canonical point-in-time market truth."
                    .to_owned(),
            );
        }
        if universe.status != UniverseStatus::Approved {
            reasons.push(format!(
                "Universe '{}' status is '{}', not APPROVED canonical universe.",
                universe.universe_id,
                universe.status.code()
            ));
        }
        if coverage_report.status == CoverageStatus::Degraded {
            reasons.push(format!(
                "Coverage is DEGRADED ({:.1}% < {:.1}%).",
                coverage_report.coverage_ratio * 100.0,
                coverage_report.threshold * 100.0
            ));
        }
        if reasons.is_empty() {
            (MarketPublicationStatus::CanonicalPublished, reasons)
        } else {
            (MarketPublicationStatus::ResearchRetrospective, reasons)
        }
    }

    /// Computes whole-market aggregate `ProxyBB` (`AT09`, `AT10`), `PIT` constituent gating (`AT11`),
    /// and independent count/value `FlowBreadth` (`AT17`), strictly gated by `replay_causal_close_timestamp` (`DEF-001`).
    #[must_use]
    pub fn calculate_market_bb(
        universe: &UniverseDefinition,
        feeds: &[MarketBBFeedInput<'_>],
        mode: Option<UniverseMode>,
        replay_causal_close_timestamp: Option<i64>,
        breadth_buffer: f64,
        coverage_threshold: f64,
    ) -> MarketBBSeriesResult {
        let eval_mode = mode.unwrap_or(universe.default_mode);
        let clean_buf = if breadth_buffer.is_finite() {
            breadth_buffer.clamp(0.0, 49.0)
        } else {
            0.0
        };
        let clean_thresh = if coverage_threshold.is_finite() {
            coverage_threshold.clamp(0.0, 1.0)
        } else {
            0.80
        };

        if let Err(err) = UniverseResolver::validate_definition(universe) {
            return MarketBBSeriesResult {
                universe_id: universe.universe_id.clone(),
                universe_version: universe.version.clone(),
                universe_mode: eval_mode,
                is_point_in_time: eval_mode == UniverseMode::PointInTime,
                publication_status: MarketPublicationStatus::UnavailableDegraded,
                methodology_version: MARKET_BB_METHODOLOGY_VERSION,
                survivor_bias_caveat: None,
                points: Vec::new(),
                gate_reasons: vec![err.to_string()],
            };
        }

        // Per-symbol causal slice, single-symbol ProxyBB series, and per-date decomposed flow.
        struct SymbolDateFlow {
            decomposed: DecomposedBarFlow,
            bar_point: ProxyBBBarPoint,
        }

        let mut symbol_by_date: BTreeMap<String, BTreeMap<SessionDate, SymbolDateFlow>> =
            BTreeMap::new();
        let mut all_dates = BTreeSet::new();

        for feed in feeds {
            let norm_sym = feed.symbol.trim().to_ascii_uppercase();
            if norm_sym.is_empty()
                || !universe
                    .members
                    .iter()
                    .any(|m| m.symbol.trim().eq_ignore_ascii_case(&norm_sym))
            {
                continue;
            }
            let causal_base_slice =
                crate::backtest::engine::slice_causal_series_for_symbol_and_options(
                    Some(&norm_sym),
                    feed.candles,
                    feed.timeframe,
                    crate::backtest::BacktestRunOptions {
                        replay_cutoff_timestamp: replay_causal_close_timestamp,
                        incomplete_tail_bar: false,
                    },
                );
            if causal_base_slice.is_empty() {
                continue;
            }

            // Roll intraday constituent feeds (`1m`..`4h`) up to completed `D1` session bars before
            // computing single-symbol `ProxyBB` and indexing `SessionDate` (`[CRITIC-R18-BLOCKER-02]`),
            // withholding any trailing unclosed `D1` session at `replay_causal_close_timestamp`.
            let d1_cow: Vec<Candle>;
            let raw_session_slice: &[Candle] = if feed.timeframe != Timeframe::D1
                && feed.timeframe.can_aggregate_to(Timeframe::D1)
            {
                let Ok(mut agg) = crate::data::timeframe::aggregate_ohlcv(
                    causal_base_slice,
                    feed.timeframe,
                    Timeframe::D1,
                ) else {
                    continue;
                };
                if let (Some(cut), Some(last_d1)) = (replay_causal_close_timestamp, agg.last())
                    && crate::backtest::is_bucket_incomplete_at_close_for_symbol(
                        &norm_sym,
                        last_d1.timestamp,
                        cut,
                        feed.timeframe,
                        Timeframe::D1,
                    )
                {
                    agg.pop();
                }
                d1_cow = agg;
                &d1_cow
            } else {
                causal_base_slice
            };
            if raw_session_slice.is_empty() {
                continue;
            }

            // Coalesce any duplicate `SessionDate` rows chronologically so `date_map` never silently
            // overwrites earlier traded value on the same date (`AT09` + `AT17` conservation).
            let mut session_bars: Vec<Candle> = Vec::with_capacity(raw_session_slice.len());
            let mut session_dates: Vec<SessionDate> = Vec::with_capacity(raw_session_slice.len());
            for bar in raw_session_slice {
                let d = SessionDate::from_utc_timestamp(bar.timestamp);
                if session_dates.last() == Some(&d) {
                    if let Some(prev_bar) = session_bars.last_mut() {
                        prev_bar.high = if prev_bar.high.is_nan() || bar.high.is_nan() {
                            f64::NAN
                        } else {
                            prev_bar.high.max(bar.high)
                        };
                        prev_bar.low = if prev_bar.low.is_nan() || bar.low.is_nan() {
                            f64::NAN
                        } else {
                            prev_bar.low.min(bar.low)
                        };
                        prev_bar.close = bar.close;
                        prev_bar.volume += bar.volume;
                    }
                } else {
                    session_dates.push(d);
                    session_bars.push(bar.clone());
                }
            }

            let sym_bb = compute_proxy_bb_series(&session_bars);
            let mut date_map = BTreeMap::new();
            let mut prev_close: Option<f64> = None;
            for (i, bar) in session_bars.iter().enumerate() {
                let d = session_dates[i];
                let dec = decompose_bar_ohlcv_proxy(bar, prev_close);
                if dec.is_valid {
                    prev_close = Some(bar.close);
                } else {
                    prev_close = None;
                }
                all_dates.insert(d);
                date_map.insert(
                    d,
                    SymbolDateFlow {
                        decomposed: dec,
                        bar_point: sym_bb.points[i],
                    },
                );
            }
            symbol_by_date.insert(norm_sym, date_map);
        }

        let chronological_dates: Vec<SessionDate> = all_dates.into_iter().collect();
        if chronological_dates.is_empty() {
            return MarketBBSeriesResult {
                universe_id: universe.universe_id.clone(),
                universe_version: universe.version.clone(),
                universe_mode: eval_mode,
                is_point_in_time: eval_mode == UniverseMode::PointInTime,
                publication_status: MarketPublicationStatus::UnavailableDegraded,
                methodology_version: MARKET_BB_METHODOLOGY_VERSION,
                survivor_bias_caveat: None,
                points: Vec::new(),
                gate_reasons: vec!["No causal market bars available for calculation.".to_owned()],
            };
        }

        let n_sessions = chronological_dates.len();
        let mut daily_buy = Vec::with_capacity(n_sessions);
        let mut daily_sell = Vec::with_capacity(n_sessions);
        let mut resolutions = Vec::with_capacity(n_sessions);
        let mut coverages = Vec::with_capacity(n_sessions);

        for &d in &chronological_dates {
            let res = UniverseResolver::resolve(universe, d, Some(eval_mode))
                .expect("universe already validated");
            let mut reporting = Vec::with_capacity(res.active_symbols.len());
            let mut m_buy = 0.0_f64;
            let mut m_sell = 0.0_f64;

            for sym in &res.active_symbols {
                if let Some(flow) = symbol_by_date.get(sym).and_then(|m| m.get(&d))
                    && flow.decomposed.is_valid
                {
                    reporting.push(sym.as_str());
                    m_buy += flow.decomposed.buy_value;
                    m_sell += flow.decomposed.sell_value;
                }
            }

            let cov = UniverseResolver::check_coverage(&res, &reporting, clean_thresh)
                .expect("threshold clamped to [0, 1]");
            daily_buy.push(m_buy);
            daily_sell.push(m_sell);
            resolutions.push(res);
            coverages.push(cov);
        }

        let mut prev_scores: [Option<f64>; 6] = [None; 6];
        let mut session_points = Vec::with_capacity(n_sessions);

        for t in 0..n_sessions {
            let d = chronological_dates[t];
            let res = &resolutions[t];
            let cov = &coverages[t];

            let mut horizons_arr = [MarketBBHorizonPoint::default(); 6];
            let mut breadth_arr = [FlowBreadthHorizonPoint::default(); 6];

            for h in BBHorizon::ALL {
                let hi = h.index();
                let lookback = h.lookback_bars();

                if t + 1 < lookback {
                    let buy_s: f64 = daily_buy[..=t].iter().sum();
                    let sell_s: f64 = daily_sell[..=t].iter().sum();
                    horizons_arr[hi] = MarketBBHorizonPoint {
                        horizon: h,
                        market_bb_value: None,
                        market_buy_value: buy_s,
                        market_sell_value: sell_s,
                        market_total_value: buy_s + sell_s,
                        market_oib: None,
                        is_warmup: true,
                        direction: BBDirection::Unknown,
                        regime: BBRegime::Unknown,
                    };
                } else {
                    let win_start = t + 1 - lookback;
                    let rolling_buy: f64 = daily_buy[win_start..=t].iter().sum();
                    let rolling_sell: f64 = daily_sell[win_start..=t].iter().sum();
                    let rolling_total = rolling_buy + rolling_sell;

                    let (score, oib, regime) = if rolling_total <= 0.0 {
                        (None, None, BBRegime::Neutral)
                    } else {
                        let s = (100.0 * (rolling_buy / rolling_total)).clamp(0.0, 100.0);
                        let o = ((rolling_buy - rolling_sell) / rolling_total).clamp(-1.0, 1.0);
                        let reg = if s > 50.0 + 1.0e-7 {
                            BBRegime::Positive
                        } else if s < 50.0 - 1.0e-7 {
                            BBRegime::Negative
                        } else {
                            BBRegime::Neutral
                        };
                        (Some(s), Some(o), reg)
                    };

                    let direction = match (prev_scores[hi], score) {
                        (None, _) | (_, None) => BBDirection::Unknown,
                        (Some(prev), Some(curr)) if curr > prev + 1.0e-7 => BBDirection::Rising,
                        (Some(prev), Some(curr)) if curr < prev - 1.0e-7 => BBDirection::Falling,
                        (Some(_), Some(_)) => BBDirection::Flat,
                    };
                    if let Some(curr) = score {
                        prev_scores[hi] = Some(curr);
                    } else {
                        prev_scores[hi] = None;
                    }

                    horizons_arr[hi] = MarketBBHorizonPoint {
                        horizon: h,
                        market_bb_value: score,
                        market_buy_value: rolling_buy,
                        market_sell_value: rolling_sell,
                        market_total_value: rolling_total,
                        market_oib: oib,
                        is_warmup: false,
                        direction,
                        regime,
                    };
                }

                // AT17 Independent Flow Breadth across active universe constituents on date `d`:
                let mut pos_c = 0_usize;
                let mut neu_c = 0_usize;
                let mut neg_c = 0_usize;
                let mut pos_v = 0.0_f64;
                let mut neu_v = 0.0_f64;
                let mut neg_v = 0.0_f64;

                for sym in &res.active_symbols {
                    if let Some(flow) = symbol_by_date.get(sym).and_then(|m| m.get(&d)) {
                        let h_pt = flow.bar_point.horizon(h);
                        if !h_pt.is_warmup
                            && let Some(sc) = h_pt.bb_value
                        {
                            let v = if h_pt.raw_denominator > 0.0 {
                                h_pt.raw_denominator
                            } else {
                                flow.bar_point.daily_trading_value
                            };
                            if sc > 50.0 + clean_buf {
                                pos_c += 1;
                                pos_v += v;
                            } else if sc < 50.0 - clean_buf {
                                neg_c += 1;
                                neg_v += v;
                            } else {
                                neu_c += 1;
                                neu_v += v;
                            }
                        }
                    }
                }

                let tot_c = pos_c + neu_c + neg_c;
                let tot_v = pos_v + neu_v + neg_v;
                breadth_arr[hi] = FlowBreadthHorizonPoint {
                    horizon: h,
                    positive_count: pos_c,
                    neutral_count: neu_c,
                    negative_count: neg_c,
                    total_count: tot_c,
                    positive_count_ratio: if tot_c > 0 {
                        (pos_c as f64) / (tot_c as f64)
                    } else {
                        0.0
                    },
                    neutral_count_ratio: if tot_c > 0 {
                        (neu_c as f64) / (tot_c as f64)
                    } else {
                        0.0
                    },
                    negative_count_ratio: if tot_c > 0 {
                        (neg_c as f64) / (tot_c as f64)
                    } else {
                        0.0
                    },
                    positive_value: pos_v,
                    neutral_value: neu_v,
                    negative_value: neg_v,
                    total_value: tot_v,
                    positive_value_ratio: if tot_v > 0.0 { pos_v / tot_v } else { 0.0 },
                    neutral_value_ratio: if tot_v > 0.0 { neu_v / tot_v } else { 0.0 },
                    negative_value_ratio: if tot_v > 0.0 { neg_v / tot_v } else { 0.0 },
                    buffer: clean_buf,
                };
            }

            session_points.push(MarketBBSessionPoint {
                date: d,
                horizons: horizons_arr,
                breadth: breadth_arr,
                symbol_coverage: cov.coverage_ratio,
                reporting_symbols_count: cov.available_count,
                total_target_count: cov.target_count,
                coverage_status: cov.status,
            });
        }

        let latest_cov = coverages.last().expect("non-empty sessions");
        let (pub_status, gate_reasons) =
            Self::evaluate_publication_gate(universe, eval_mode, latest_cov);
        let survivor_caveat = (eval_mode == UniverseMode::RetrospectiveFixed).then(|| {
            "RETROSPECTIVE_FIXED MODE: Evaluated on fixed universe snapshot. Carries survivorship bias and must not be used as canonical point-in-time market truth."
                .to_owned()
        });

        MarketBBSeriesResult {
            universe_id: universe.universe_id.clone(),
            universe_version: universe.version.clone(),
            universe_mode: eval_mode,
            is_point_in_time: eval_mode == UniverseMode::PointInTime,
            publication_status: pub_status,
            methodology_version: MARKET_BB_METHODOLOGY_VERSION,
            survivor_bias_caveat: survivor_caveat,
            points: session_points,
            gate_reasons,
        }
    }
}
