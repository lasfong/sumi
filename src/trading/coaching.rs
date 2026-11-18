//! Live Replay Practice Scoreboard, Session Debrief & Causal Decision Coaching, and
//! Zero-Dependency `SavedPracticeSession` + `SavedScanSnapshot` Stores (`Milestone R17 / Wave 16`).

use std::collections::HashMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::alerts::truncate_utf8_bytes;
use crate::backtest::PNL_BREAKEVEN_EPS;
use crate::data::{DataSourcePreset, Timeframe};
use crate::journal::{ChecklistState, JournalEntry, JournalEntrySource, MistakeTag, SetupTag};
use crate::model::Candle;
use crate::scanner::{
    HistoricalSignalHit, MarketRegime, ScanSignalKind, ScannerFilter, ScannerReplayIntent,
    ScannerSignalContext, SignalDirection, WatchlistSortColumn,
};
use crate::viewport::WorkspaceTemplateError;
use crate::viewport::workspace_template::{
    DocValue, escape_str_into, expect_bool, expect_f64, expect_i64_safe, expect_label,
    expect_opt_string, expect_string, expect_struct, expect_u64_safe, expect_usize, find_field,
    fmt_f64, parse_data_source, parse_json_doc, parse_ron_doc, parse_timeframe,
    splitmix64_hash_bytes, strip_leading_ws_and_comments, validate_struct_fields,
};

use super::orders::{AccountConfig, MAX_RECORDED_COMMANDS, OrderKind, OrderRequest, OrderSide};
use super::portfolio::{
    ClosedTrade, PortfolioMetrics, RecordedCommand, RecordedCommandKind, TradingSession,
};

pub const PRACTICE_STORE_SCHEMA_VERSION: u32 = 1;
pub const SCAN_STORE_SCHEMA_VERSION: u32 = 1;
pub const MAX_SAVED_PRACTICE_SESSIONS: usize = 16;
pub const MAX_SAVED_SCAN_SNAPSHOTS: usize = 16;
pub const MAX_DECISION_NOTES: usize = 512;
pub const MAX_DECISION_NOTE_BYTES: usize = 512;
pub const MAX_SNAPSHOT_HITS: usize = 128;

// ============================================================================
// 1. Causal `PracticeDecisionNote` & `PracticeDecisionLog` (`A2` / `DEF-001`)
// ============================================================================

/// Discretionary decision category stamped onto a causal replay bar (`DecisionJournal.tsx` replacement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PracticeDecisionAction {
    #[default]
    ObserveHold,
    PlanLong,
    PlanShort,
    ManagePosition,
    SkipPass,
    PostExitReview,
}

impl PracticeDecisionAction {
    pub const ALL: [Self; 6] = [
        Self::ObserveHold,
        Self::PlanLong,
        Self::PlanShort,
        Self::ManagePosition,
        Self::SkipPass,
        Self::PostExitReview,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::ObserveHold => "Observe",
            Self::PlanLong => "Plan Long",
            Self::PlanShort => "Plan Short",
            Self::ManagePosition => "Manage",
            Self::SkipPass => "Pass/Skip",
            Self::PostExitReview => "Exit Note",
        }
    }

    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::ObserveHold => "ObserveHold",
            Self::PlanLong => "PlanLong",
            Self::PlanShort => "PlanShort",
            Self::ManagePosition => "ManagePosition",
            Self::SkipPass => "SkipPass",
            Self::PostExitReview => "PostExitReview",
        }
    }

    #[must_use]
    pub fn from_id(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "observehold" | "observe" => Some(Self::ObserveHold),
            "planlong" | "long" => Some(Self::PlanLong),
            "planshort" | "short" => Some(Self::PlanShort),
            "manageposition" | "manage" => Some(Self::ManagePosition),
            "skippass" | "pass" | "skip" => Some(Self::SkipPass),
            "postexitreview" | "exit" => Some(Self::PostExitReview),
            _ => None,
        }
    }
}

/// Causal decision note stamped at `(base_bar_index, bar_timestamp)` (`DEF-001` rewind-safe).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticeDecisionNote {
    pub id: u64,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub base_bar_index: usize,
    pub bar_timestamp: i64,
    pub mark_price: f64,
    pub action: PracticeDecisionAction,
    pub setup: SetupTag,
    pub mistake: MistakeTag,
    pub conviction: u8,
    pub checklist: ChecklistState,
    pub note: String,
}

impl PracticeDecisionNote {
    pub fn sanitize(&mut self) {
        truncate_utf8_bytes(&mut self.symbol, 32);
        truncate_utf8_bytes(&mut self.note, MAX_DECISION_NOTE_BYTES);
        if !self.mark_price.is_finite() || self.mark_price < 0.0 {
            self.mark_price = 0.0;
        }
        self.conviction = self.conviction.clamp(1, 5);
    }

    /// Strict `DEF-001` causal visibility gate: during active Bar Replay, notes recorded at future
    /// bars (`base_bar_index > cursor_base_index` or `bar_timestamp > cutoff_timestamp`) are hidden
    /// whenever the user rewinds the replay cursor.
    #[inline]
    #[must_use]
    pub fn is_causally_visible(
        &self,
        replay_active: bool,
        cursor_base_index: usize,
        cutoff_timestamp: Option<i64>,
    ) -> bool {
        if !replay_active {
            return true;
        }
        self.base_bar_index <= cursor_base_index
            && cutoff_timestamp.is_none_or(|cut| self.bar_timestamp <= cut)
    }
}

const fn default_next_note_id() -> u64 {
    1
}

/// Bounded causal decision log attached to the active practice session (`Milestone R17 / A2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticeDecisionLog {
    #[serde(default)]
    pub entries: Vec<PracticeDecisionNote>,
    #[serde(default = "default_next_note_id")]
    pub next_id: u64,
}

impl Default for PracticeDecisionLog {
    fn default() -> Self {
        Self::new()
    }
}

impl PracticeDecisionLog {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn sanitize(&mut self) {
        if self.entries.len() > MAX_DECISION_NOTES {
            let drop_n = self.entries.len() - MAX_DECISION_NOTES;
            self.entries.drain(0..drop_n);
        }
        let mut max_id = 0_u64;
        self.entries.retain_mut(|n| {
            if n.id == 0 {
                return false;
            }
            n.sanitize();
            max_id = max_id.max(n.id);
            true
        });
        self.next_id = self.next_id.max(max_id.saturating_add(1)).max(1);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 1;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_note(
        &mut self,
        symbol: &str,
        timeframe: Timeframe,
        base_bar_index: usize,
        bar_timestamp: i64,
        mark_price: f64,
        action: PracticeDecisionAction,
        setup: SetupTag,
        mistake: MistakeTag,
        conviction: u8,
        checklist: ChecklistState,
        note: &str,
    ) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id.saturating_add(1);
        let mut entry = PracticeDecisionNote {
            id,
            symbol: symbol.trim().to_owned(),
            timeframe,
            base_bar_index,
            bar_timestamp,
            mark_price,
            action,
            setup,
            mistake,
            conviction: conviction.clamp(1, 5),
            checklist,
            note: note.trim().to_owned(),
        };
        entry.sanitize();
        self.entries.push(entry);
        if self.entries.len() > MAX_DECISION_NOTES {
            let drop_n = self.entries.len() - MAX_DECISION_NOTES;
            self.entries.drain(0..drop_n);
        }
        id
    }

    pub fn delete_note(&mut self, id: u64) -> bool {
        let before = self.entries.len();
        self.entries.retain(|n| n.id != id);
        self.entries.len() < before
    }

    #[must_use]
    pub fn visible_notes(
        &self,
        symbol: &str,
        replay_active: bool,
        cursor_base_index: usize,
        cutoff_timestamp: Option<i64>,
    ) -> Vec<PracticeDecisionNote> {
        let clean_sym = symbol.trim();
        self.entries
            .iter()
            .filter(|n| {
                (clean_sym.is_empty() || n.symbol.eq_ignore_ascii_case(clean_sym))
                    && n.is_causally_visible(replay_active, cursor_base_index, cutoff_timestamp)
            })
            .cloned()
            .collect()
    }
}

// ============================================================================
// 2. `PracticeScoreboardSummary` (`A1`) & `SessionDebriefReport` (`A2`)
// ============================================================================

/// Real-time KPI strip for the Live Replay Practice Scoreboard (`PracticeScoreboard.tsx` replacement).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PracticeScoreboardSummary {
    pub closed_trades: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub breakeven_count: usize,
    pub win_rate_pct: f64,
    pub net_pnl: f64,
    pub return_pct: f64,
    pub avg_return_pct: f64,
    pub net_r_multiple: f64,
    pub avg_r_multiple: Option<f64>,
    pub profit_factor: Option<f64>,
    pub max_drawdown_pct: f64,
    pub sl_adherence_pct: Option<f64>,
    pub visible_decision_notes: usize,
    pub has_open_position: bool,
}

impl PracticeScoreboardSummary {
    #[must_use]
    pub fn compute(
        session: &TradingSession,
        mark_price: f64,
        decision_log: &PracticeDecisionLog,
        symbol: &str,
        replay_active: bool,
        cursor_base_index: usize,
        cutoff_timestamp: Option<i64>,
    ) -> Self {
        let m: PortfolioMetrics = session.metrics(mark_price);
        let visible_decision_notes = decision_log
            .entries
            .iter()
            .filter(|n| {
                (symbol.trim().is_empty() || n.symbol.eq_ignore_ascii_case(symbol.trim()))
                    && n.is_causally_visible(replay_active, cursor_base_index, cutoff_timestamp)
            })
            .count();
        Self {
            closed_trades: m.closed_trade_count,
            win_count: m.win_count,
            loss_count: m.loss_count,
            breakeven_count: m.breakeven_count,
            win_rate_pct: m.win_rate_pct,
            net_pnl: m.net_pnl,
            return_pct: m.return_pct,
            avg_return_pct: m.avg_return_pct,
            net_r_multiple: m.net_r_multiple,
            avg_r_multiple: m.avg_r_multiple,
            profit_factor: m.profit_factor,
            max_drawdown_pct: m.max_drawdown_pct,
            sl_adherence_pct: m.sl_adherence_pct,
            visible_decision_notes,
            has_open_position: session.open_position().is_some(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoachingSeverity {
    Positive,
    Neutral,
    Caution,
    Critical,
}

impl CoachingSeverity {
    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Positive => "EDGE",
            Self::Neutral => "INFO",
            Self::Caution => "WATCH",
            Self::Critical => "RISK",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachingCallout {
    pub severity: CoachingSeverity,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MistakeCostBreakdown {
    pub mistake: MistakeTag,
    pub trade_count: usize,
    pub decision_note_count: usize,
    pub net_pnl_impact: f64,
    /// Sum of adverse dollar losses (`(-net_pnl).max(0.0)`) across closed trades tagged with `mistake`.
    pub adverse_cost_abs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetupEdgeBreakdown {
    pub setup: SetupTag,
    pub trade_count: usize,
    pub win_count: usize,
    pub win_rate_pct: f64,
    pub net_pnl: f64,
    pub avg_r: Option<f64>,
}

/// Comprehensive coaching & risk-adherence debrief for `SessionDebriefModal` (`Milestone R17 / A2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDebriefReport {
    pub symbol: String,
    pub timeframe: Timeframe,
    pub closed_trades: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub breakeven_count: usize,
    pub win_rate_pct: f64,
    pub total_net_pnl: f64,
    pub total_return_pct: f64,
    pub avg_return_pct: f64,
    pub profit_factor: Option<f64>,
    pub expectancy_pnl: Option<f64>,
    pub net_r_multiple: f64,
    pub avg_r_multiple: Option<f64>,
    pub max_drawdown_pct: f64,
    pub total_opened_positions: usize,
    pub sl_adherent_positions: usize,
    /// `None` when `total_opened_positions == 0` (eliminates legacy `SessionDebriefModal.tsx:57 vs 93` contradiction).
    pub sl_adherence_pct: Option<f64>,
    pub mistake_breakdown: Vec<MistakeCostBreakdown>,
    pub setup_breakdown: Vec<SetupEdgeBreakdown>,
    pub visible_decision_notes: Vec<PracticeDecisionNote>,
    pub callouts: Vec<CoachingCallout>,
}

impl SessionDebriefReport {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn build(
        symbol: &str,
        timeframe: Timeframe,
        session: &TradingSession,
        mark_price: f64,
        journal_entries: &[JournalEntry],
        decision_log: &PracticeDecisionLog,
        replay_active: bool,
        cursor_base_index: usize,
        cutoff_timestamp: Option<i64>,
    ) -> Self {
        let clean_sym = symbol.trim();
        let m = session.metrics(mark_price);
        let visible_notes = decision_log.visible_notes(
            clean_sym,
            replay_active,
            cursor_base_index,
            cutoff_timestamp,
        );

        // Index causally visible paper journal entries by `(trade_id, entry_timestamp)`.
        let mut journal_by_trade: HashMap<(u64, i64), &JournalEntry> = HashMap::new();
        for je in journal_entries {
            if !je.is_causally_visible(cutoff_timestamp) {
                continue;
            }
            if let JournalEntrySource::PaperTrade {
                symbol: ref s,
                trade_id,
                entry_timestamp,
            } = je.source
                && (clean_sym.is_empty() || s.eq_ignore_ascii_case(clean_sym))
            {
                journal_by_trade.insert((trade_id, entry_timestamp), je);
            }
        }

        // Accumulate MistakeTag & SetupTag metrics over session `closed_trades` (falling back to
        // causal `PracticeDecisionNote`s stamped inside `[entry_base_index..=exit_base_index]`).
        let mut mistake_map: HashMap<MistakeTag, (usize, usize, f64, f64)> = HashMap::new();
        let mut setup_map: HashMap<SetupTag, (usize, usize, f64, f64, usize)> = HashMap::new();

        for n in &visible_notes {
            if n.mistake != MistakeTag::None {
                let slot = mistake_map.entry(n.mistake).or_insert((0, 0, 0.0, 0.0));
                slot.1 += 1;
            }
        }

        let causal_closed_trades: Vec<_> = session
            .closed_trades()
            .iter()
            .filter(|tr| !cutoff_timestamp.is_some_and(|cut| tr.exit_timestamp > cut))
            .collect();
        let open_pos_entry_bar = session
            .open_position()
            .filter(|pos| !cutoff_timestamp.is_some_and(|cut| pos.opened_timestamp > cut))
            .map(|pos| pos.opened_base_index);
        let mut note_claimed = vec![false; visible_notes.len()];

        for tr in &causal_closed_trades {
            let matched_je = journal_by_trade.get(&(tr.trade_id, tr.entry_timestamp));
            let matched_note_idx = visible_notes
                .iter()
                .enumerate()
                .rev()
                .find(|&(ni, n)| {
                    if note_claimed[ni]
                        || n.base_bar_index < tr.entry_base_index
                        || n.base_bar_index > tr.exit_base_index
                    {
                        return false;
                    }
                    // Prevent cross-trade contamination when Trade #1 exits on bar `k` and Trade #2
                    // opens on the same bar `k`: notes stamped on bar `k` (other than explicit
                    // `ExitReview`) belong to the trade opening at `k`, not the exiting trade!
                    if n.base_bar_index == tr.exit_base_index
                        && tr.exit_base_index > tr.entry_base_index
                        && n.action != PracticeDecisionAction::PostExitReview
                    {
                        let next_opens_same_bar = causal_closed_trades.iter().any(|other| {
                            other.trade_id > tr.trade_id
                                && other.entry_base_index == tr.exit_base_index
                        }) || open_pos_entry_bar
                            == Some(tr.exit_base_index);
                        if next_opens_same_bar {
                            return false;
                        }
                    }
                    true
                })
                .map(|(ni, _)| ni);
            if let Some(ni) = matched_note_idx {
                note_claimed[ni] = true;
            }
            let matched_note = matched_note_idx.map(|ni| &visible_notes[ni]);

            let je_curated = matched_je.is_some_and(|j| {
                j.setup != SetupTag::Untagged
                    || j.mistake != MistakeTag::None
                    || !j.notes.trim().is_empty()
                    || j.conviction != 3
            });

            let setup = if let Some(j) = matched_je
                && (j.setup != SetupTag::Untagged || je_curated)
            {
                if j.setup != SetupTag::Untagged {
                    j.setup
                } else {
                    matched_note
                        .map(|n| n.setup)
                        .filter(|&s| s != SetupTag::Untagged)
                        .unwrap_or(SetupTag::Untagged)
                }
            } else {
                matched_note
                    .map(|n| n.setup)
                    .filter(|&s| s != SetupTag::Untagged)
                    .unwrap_or(SetupTag::Untagged)
            };

            let mut mistake = if let Some(j) = matched_je
                && je_curated
            {
                // Curated `JournalEntry` is authoritative — including an explicit `MistakeTag::None`
                // clearing a tentative pre-trade decision note tag (`[CRITIC-R17-MAJOR-01]`).
                j.mistake
            } else {
                matched_note.map_or(MistakeTag::None, |n| n.mistake)
            };
            // Automatically flag naked entries that lost money without an initial stop-loss as `SkippedChecklist`
            // if the user left `MistakeTag::None`, surfacing actionable coaching without mutating journal data.
            if mistake == MistakeTag::None
                && !tr.had_initial_sl_at_open
                && tr.net_pnl < -PNL_BREAKEVEN_EPS
            {
                mistake = MistakeTag::SkippedChecklist;
            }

            if mistake != MistakeTag::None {
                let slot = mistake_map.entry(mistake).or_insert((0, 0, 0.0, 0.0));
                slot.0 += 1;
                slot.2 += tr.net_pnl;
                if tr.net_pnl < 0.0 {
                    slot.3 += tr.net_pnl.abs();
                }
            }

            let s_slot = setup_map.entry(setup).or_insert((0, 0, 0.0, 0.0, 0));
            s_slot.0 += 1;
            if tr.net_pnl > PNL_BREAKEVEN_EPS {
                s_slot.1 += 1;
            }
            s_slot.2 += tr.net_pnl;
            if let Some(r) = tr.r_multiple
                && r.is_finite()
            {
                s_slot.3 += r;
                s_slot.4 += 1;
            }
        }

        let mut mistake_breakdown: Vec<MistakeCostBreakdown> = mistake_map
            .into_iter()
            .map(
                |(
                    mistake,
                    (trade_count, decision_note_count, net_pnl_impact, adverse_cost_abs),
                )| {
                    MistakeCostBreakdown {
                        mistake,
                        trade_count,
                        decision_note_count,
                        net_pnl_impact,
                        adverse_cost_abs,
                    }
                },
            )
            .collect();
        mistake_breakdown.sort_by(|a, b| {
            b.adverse_cost_abs
                .total_cmp(&a.adverse_cost_abs)
                .then_with(|| b.trade_count.cmp(&a.trade_count))
                .then_with(|| a.mistake.short_label().cmp(b.mistake.short_label()))
        });

        let mut setup_breakdown: Vec<SetupEdgeBreakdown> = setup_map
            .into_iter()
            .map(
                |(setup, (cnt, wins, pnl, r_sum, r_cnt))| SetupEdgeBreakdown {
                    setup,
                    trade_count: cnt,
                    win_count: wins,
                    win_rate_pct: if cnt > 0 {
                        (wins as f64 / cnt as f64) * 100.0
                    } else {
                        0.0
                    },
                    net_pnl: pnl,
                    avg_r: (r_cnt > 0).then_some(r_sum / (r_cnt as f64)),
                },
            )
            .collect();
        setup_breakdown.sort_by(|a, b| {
            b.net_pnl
                .total_cmp(&a.net_pnl)
                .then_with(|| b.trade_count.cmp(&a.trade_count))
                .then_with(|| a.setup.short_label().cmp(b.setup.short_label()))
        });

        // Build deterministic coaching callouts (`0` contradictions on empty sessions!).
        let mut callouts = Vec::new();
        match m.sl_adherence_pct {
            None => {
                callouts.push(CoachingCallout {
                    severity: CoachingSeverity::Neutral,
                    title: "No Positions Opened Yet".to_owned(),
                    detail: format!(
                        "0 positions opened ({} bar decision notes logged). Initial Stop-Loss adherence is N/A until an entry executes.",
                        visible_notes.len()
                    ),
                });
            }
            Some(pct) if pct < 80.0 => {
                let naked = m
                    .total_opened_positions
                    .saturating_sub(m.sl_adherent_positions);
                callouts.push(CoachingCallout {
                    severity: CoachingSeverity::Critical,
                    title: format!("Initial Stop-Loss Adherence {pct:.0}% (< 80% Target)"),
                    detail: format!(
                        "{naked} of {} opened position(s) entered without an initial protective Stop-Loss. Post-entry bracket edits never mask naked opens.",
                        m.total_opened_positions
                    ),
                });
            }
            Some(pct) if (pct - 100.0).abs() <= 1e-6 => {
                callouts.push(CoachingCallout {
                    severity: CoachingSeverity::Positive,
                    title: "100% Initial Stop-Loss Discipline".to_owned(),
                    detail: format!(
                        "All {} opened position(s) carried a valid initial Stop-Loss at execution.",
                        m.total_opened_positions
                    ),
                });
            }
            Some(pct) => {
                callouts.push(CoachingCallout {
                    severity: CoachingSeverity::Caution,
                    title: format!("Initial Stop-Loss Adherence {pct:.0}%"),
                    detail: format!(
                        "{} / {} opened position(s) included an initial Stop-Loss at entry.",
                        m.sl_adherent_positions, m.total_opened_positions
                    ),
                });
            }
        }

        if let Some(worst_mk) = mistake_breakdown
            .iter()
            .find(|row| row.adverse_cost_abs > PNL_BREAKEVEN_EPS || row.trade_count > 0)
        {
            callouts.push(CoachingCallout {
                severity: if worst_mk.adverse_cost_abs > 0.0 {
                    CoachingSeverity::Critical
                } else {
                    CoachingSeverity::Caution
                },
                title: format!(
                    "Primary Leak: {} ({} trade(s))",
                    worst_mk.mistake.short_label(),
                    worst_mk.trade_count
                ),
                detail: format!(
                    "Adverse loss cost {:.2} (net PnL impact {:+.2}). Eliminate this pattern to lift session expectancy.",
                    worst_mk.adverse_cost_abs, worst_mk.net_pnl_impact
                ),
            });
        }

        if let Some(best_setup) = setup_breakdown
            .iter()
            .find(|s| s.setup != SetupTag::Untagged && s.net_pnl > PNL_BREAKEVEN_EPS)
        {
            callouts.push(CoachingCallout {
                severity: CoachingSeverity::Positive,
                title: format!(
                    "Top Setup Edge: {} ({:.0}% WR)",
                    best_setup.setup.short_label(),
                    best_setup.win_rate_pct
                ),
                detail: format!(
                    "{} trade(s) generated {:+.2} net PnL{}.",
                    best_setup.trade_count,
                    best_setup.net_pnl,
                    best_setup
                        .avg_r
                        .map_or_else(String::new, |r| format!(" (Avg {r:+.2}R)"))
                ),
            });
        }

        if m.max_drawdown_pct >= 10.0 {
            callouts.push(CoachingCallout {
                severity: CoachingSeverity::Caution,
                title: format!("Elevated Session Drawdown ({:.2}%)", m.max_drawdown_pct),
                detail: "Peak-to-trough equity drawdown exceeded 10%; consider tighter per-trade risk sizing.".to_owned(),
            });
        }

        Self {
            symbol: clean_sym.to_owned(),
            timeframe,
            closed_trades: m.closed_trade_count,
            win_count: m.win_count,
            loss_count: m.loss_count,
            breakeven_count: m.breakeven_count,
            win_rate_pct: m.win_rate_pct,
            total_net_pnl: m.net_pnl,
            total_return_pct: m.return_pct,
            avg_return_pct: m.avg_return_pct,
            profit_factor: m.profit_factor,
            expectancy_pnl: m.expectancy_pnl,
            net_r_multiple: m.net_r_multiple,
            avg_r_multiple: m.avg_r_multiple,
            max_drawdown_pct: m.max_drawdown_pct,
            total_opened_positions: m.total_opened_positions,
            sl_adherent_positions: m.sl_adherent_positions,
            sl_adherence_pct: m.sl_adherence_pct,
            mistake_breakdown,
            setup_breakdown,
            visible_decision_notes: visible_notes,
            callouts,
        }
    }
}

// ============================================================================
// 3. `SavedPracticeSession` & `PracticeSessionStore` (`A3`)
// ============================================================================

///Persisted replay + paper-trading + decision-note practice checkpoint (`Milestone R17 / A3`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedPracticeSession {
    pub slot_id: u64,
    pub label: String,
    pub pinned: bool,
    pub sequence_no: u64,
    pub data_source: DataSourcePreset,
    pub symbol: String,
    pub base_timeframe: Timeframe,
    pub active_timeframe: Timeframe,
    pub expected_bar_count: usize,
    pub expected_first_timestamp: Option<i64>,
    pub expected_last_timestamp: Option<i64>,
    pub replay_active: bool,
    pub replay_start_base_index: usize,
    pub replay_cursor_base_index: usize,
    pub blind_lookback_bars: usize,
    pub scanner_context: Option<ScannerSignalContext>,
    pub account_config: AccountConfig,
    pub session_start_base_index: usize,
    pub commands: Vec<RecordedCommand>,
    pub decision_notes: Vec<PracticeDecisionNote>,
    #[serde(default)]
    pub session_digest: u64,
}

impl SavedPracticeSession {
    pub fn sanitize(&mut self) {
        self.label = self.label.trim().to_owned();
        truncate_utf8_bytes(&mut self.label, 80);
        if self.label.is_empty() {
            self.label = format!("Practice #{}", self.slot_id.max(1));
        }
        truncate_utf8_bytes(&mut self.symbol, 32);
        self.blind_lookback_bars = self.blind_lookback_bars.clamp(1, 500);
        self.account_config.sanitize();
        if self.commands.len() > MAX_RECORDED_COMMANDS {
            self.commands.truncate(MAX_RECORDED_COMMANDS);
        }
        if self.decision_notes.len() > MAX_DECISION_NOTES {
            self.decision_notes.truncate(MAX_DECISION_NOTES);
        }
        for n in &mut self.decision_notes {
            n.sanitize();
        }
        if let Some(ref mut ctx) = self.scanner_context {
            ctx.sanitize_deserialized();
        }
        self.session_digest = self.compute_digest();
    }

    #[must_use]
    pub fn compute_digest(&self) -> u64 {
        let mut copy = self.clone();
        copy.session_digest = 0;
        let mut buf = String::with_capacity(1024);
        encode_ron_practice_slot(&mut buf, &copy, 0, false);
        splitmix64_hash_bytes(buf.as_bytes())
    }

    /// Verifies that the currently loaded instrument matches the dataset on which this session was recorded,
    /// returning a structured warning message when a `CustomFile` or candle span diverges (`DatasetMismatch`).
    #[must_use]
    pub fn verify_dataset_match(
        &self,
        actual_source: DataSourcePreset,
        actual_symbol: &str,
        base_candles: &[Candle],
    ) -> Option<String> {
        if base_candles.is_empty() {
            return Some(format!(
                "DatasetMismatch: '{}' has 0 loaded bars (expected {} bars for {}).",
                actual_symbol, self.expected_bar_count, self.symbol
            ));
        }
        if !actual_symbol
            .trim()
            .eq_ignore_ascii_case(self.symbol.trim())
        {
            return Some(format!(
                "DatasetMismatch: active symbol '{}' != saved session symbol '{}' (source {:?}).",
                actual_symbol, self.symbol, self.data_source
            ));
        }
        let actual_first = base_candles.first().map(|c| c.timestamp);
        let actual_last = base_candles.last().map(|c| c.timestamp);
        if actual_source != self.data_source
            || base_candles.len() != self.expected_bar_count
            || (self.expected_first_timestamp.is_some()
                && actual_first != self.expected_first_timestamp)
            || (self.expected_last_timestamp.is_some()
                && actual_last != self.expected_last_timestamp)
        {
            return Some(format!(
                "DatasetMismatch: loaded {} ({} bars) differs from saved {:?} ({} bars); commands clamped to causal bounds.",
                actual_symbol,
                base_candles.len(),
                self.data_source,
                self.expected_bar_count
            ));
        }
        None
    }
}

const fn default_store_schema_v1() -> u32 {
    1
}

const fn default_next_u64_1() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PracticeSessionStore {
    #[serde(default = "default_store_schema_v1")]
    pub schema_version: u32,
    #[serde(default = "default_next_u64_1")]
    pub next_slot_id: u64,
    #[serde(default = "default_next_u64_1")]
    pub next_sequence_no: u64,
    #[serde(default)]
    pub slots: Vec<SavedPracticeSession>,
}

impl Default for PracticeSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PracticeSessionStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: PRACTICE_STORE_SCHEMA_VERSION,
            next_slot_id: 1,
            next_sequence_no: 1,
            slots: Vec::new(),
        }
    }

    pub fn sanitize(&mut self) {
        self.schema_version = PRACTICE_STORE_SCHEMA_VERSION;
        if self.slots.len() > MAX_SAVED_PRACTICE_SESSIONS {
            self.slots.truncate(MAX_SAVED_PRACTICE_SESSIONS);
        }
        let mut max_id = 0_u64;
        let mut max_seq = 0_u64;
        for (i, s) in self.slots.iter_mut().enumerate() {
            if s.slot_id == 0 {
                s.slot_id = (i as u64) + 1;
            }
            if s.sequence_no == 0 {
                s.sequence_no = (i as u64) + 1;
            }
            s.sanitize();
            max_id = max_id.max(s.slot_id);
            max_seq = max_seq.max(s.sequence_no);
        }
        self.next_slot_id = self.next_slot_id.max(max_id.saturating_add(1)).max(1);
        self.next_sequence_no = self.next_sequence_no.max(max_seq.saturating_add(1)).max(1);
    }

    pub fn save_session(
        &mut self,
        mut session: SavedPracticeSession,
    ) -> Result<u64, WorkspaceTemplateError> {
        let clean_label = session.label.trim().to_owned();
        if let Some(existing) = self
            .slots
            .iter_mut()
            .find(|s| s.label.eq_ignore_ascii_case(&clean_label) && !clean_label.is_empty())
        {
            let id = existing.slot_id;
            let pinned = existing.pinned || session.pinned;
            let seq = self.next_sequence_no.max(1);
            self.next_sequence_no = seq.saturating_add(1);
            session.slot_id = id;
            session.pinned = pinned;
            session.sequence_no = seq;
            session.sanitize();
            *existing = session;
            return Ok(id);
        }

        if self.slots.len() >= MAX_SAVED_PRACTICE_SESSIONS {
            let Some((evict_idx, _)) = self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.pinned)
                .min_by_key(|(_, s)| s.sequence_no)
            else {
                return Err(WorkspaceTemplateError::SnapshotSlotsFull);
            };
            self.slots.remove(evict_idx);
        }

        let id = self.next_slot_id.max(1);
        self.next_slot_id = id.saturating_add(1);
        let seq = self.next_sequence_no.max(1);
        self.next_sequence_no = seq.saturating_add(1);
        session.slot_id = id;
        session.sequence_no = seq;
        session.sanitize();
        self.slots.push(session);
        Ok(id)
    }

    pub fn delete_session(&mut self, slot_id: u64) -> bool {
        let before = self.slots.len();
        self.slots.retain(|s| s.slot_id != slot_id);
        self.slots.len() < before
    }

    pub fn toggle_pin(&mut self, slot_id: u64) -> Option<bool> {
        let s = self.slots.iter_mut().find(|s| s.slot_id == slot_id)?;
        s.pinned = !s.pinned;
        s.session_digest = s.compute_digest();
        Some(s.pinned)
    }

    #[must_use]
    pub fn get(&self, slot_id: u64) -> Option<&SavedPracticeSession> {
        self.slots.iter().find(|s| s.slot_id == slot_id)
    }

    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut copy = self.clone();
        copy.sanitize();
        encode_json_practice_store(&copy)
    }

    #[must_use]
    pub fn to_ron_pretty(&self) -> String {
        let mut copy = self.clone();
        copy.sanitize();
        encode_ron_practice_store(&copy)
    }

    pub fn from_json_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let doc = parse_json_doc(input)?;
        Self::from_doc_value(&doc)
    }

    pub fn from_ron_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let doc = parse_ron_doc(input)?;
        Self::from_doc_value(&doc)
    }

    pub fn from_auto_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let trimmed = strip_leading_ws_and_comments(input)?;
        if trimmed.starts_with('{') {
            Self::from_json_str(trimmed)
        } else {
            Self::from_ron_str(input)
        }
    }

    fn from_doc_value(val: &DocValue) -> Result<Self, WorkspaceTemplateError> {
        let fields = expect_struct(val, "PracticeSessionStore")?;
        validate_struct_fields(
            fields,
            "PracticeSessionStore",
            &[
                "schema_version",
                "next_slot_id",
                "next_sequence_no",
                "slots",
            ],
        )?;
        let ver_val = find_field(fields, "schema_version").ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "schema_version".to_owned(),
                reason: "missing schema_version".to_owned(),
            }
        })?;
        let ver_f = expect_f64(ver_val, "schema_version")?;
        if ver_f < 0.0 || ver_f.fract() != 0.0 || ver_f as u32 != PRACTICE_STORE_SCHEMA_VERSION {
            return Err(WorkspaceTemplateError::UnsupportedSchemaVersion(
                ver_f.max(0.0) as u32,
            ));
        }
        let mut store = Self::new();
        if let Some(v) = find_field(fields, "next_slot_id") {
            store.next_slot_id = expect_u64_safe(v, "next_slot_id")?;
        }
        if let Some(v) = find_field(fields, "next_sequence_no") {
            store.next_sequence_no = expect_u64_safe(v, "next_sequence_no")?;
        }
        if let Some(v) = find_field(fields, "slots") {
            let items = match v {
                DocValue::Array(arr) => arr,
                _ => {
                    return Err(WorkspaceTemplateError::InvalidField {
                        field: "slots".to_owned(),
                        reason: "expected array".to_owned(),
                    });
                }
            };
            if items.len() > MAX_SAVED_PRACTICE_SESSIONS {
                return Err(WorkspaceTemplateError::InvalidField {
                    field: "slots".to_owned(),
                    reason: format!(
                        "too many practice slots ({}, max {MAX_SAVED_PRACTICE_SESSIONS})",
                        items.len()
                    ),
                });
            }
            for (idx, item) in items.iter().enumerate() {
                store
                    .slots
                    .push(decode_practice_slot(item, &format!("slots[{idx}]"))?);
            }
        }
        store.sanitize();
        Ok(store)
    }
}

// ============================================================================
// 4. `SavedScanSnapshot` & `ScanSnapshotStore` (`A3`)
// ============================================================================

/// Persisted scanner filter + causal historical signal hits snapshot (`Milestone R17 / A3`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedScanSnapshot {
    pub slot_id: u64,
    pub label: String,
    pub pinned: bool,
    pub sequence_no: u64,
    pub captured_cutoff_timestamp: Option<i64>,
    pub timeframe: Timeframe,
    pub signal_lookback_bars: usize,
    pub blind_practice_lookback_bars: usize,
    pub filter: ScannerFilter,
    pub sort_col: WatchlistSortColumn,
    pub sort_asc: bool,
    pub hits: Vec<HistoricalSignalHit>,
    #[serde(default)]
    pub snapshot_digest: u64,
}

impl SavedScanSnapshot {
    pub fn sanitize(&mut self) {
        self.label = self.label.trim().to_owned();
        truncate_utf8_bytes(&mut self.label, 80);
        if self.label.is_empty() {
            self.label = format!("Scan #{}", self.slot_id.max(1));
        }
        self.signal_lookback_bars = self.signal_lookback_bars.clamp(5, 1_000);
        self.blind_practice_lookback_bars = self.blind_practice_lookback_bars.clamp(1, 500);
        truncate_utf8_bytes(&mut self.filter.search_query, 64);
        if self.hits.len() > MAX_SNAPSHOT_HITS {
            self.hits.truncate(MAX_SNAPSHOT_HITS);
        }
        for hit in &mut self.hits {
            truncate_utf8_bytes(&mut hit.instrument_id, 128);
            truncate_utf8_bytes(&mut hit.symbol, 64);
            if !hit.price.is_finite() {
                hit.price = 0.0;
            }
            if !hit.metric_value.is_finite() {
                hit.metric_value = 0.0;
            }
        }
        self.snapshot_digest = self.compute_digest();
    }

    #[must_use]
    pub fn compute_digest(&self) -> u64 {
        let mut copy = self.clone();
        copy.snapshot_digest = 0;
        let mut buf = String::with_capacity(1024);
        encode_ron_scan_slot(&mut buf, &copy, 0, false);
        splitmix64_hash_bytes(buf.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanSnapshotStore {
    #[serde(default = "default_store_schema_v1")]
    pub schema_version: u32,
    #[serde(default = "default_next_u64_1")]
    pub next_slot_id: u64,
    #[serde(default = "default_next_u64_1")]
    pub next_sequence_no: u64,
    #[serde(default)]
    pub slots: Vec<SavedScanSnapshot>,
}

impl Default for ScanSnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanSnapshotStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: SCAN_STORE_SCHEMA_VERSION,
            next_slot_id: 1,
            next_sequence_no: 1,
            slots: Vec::new(),
        }
    }

    pub fn sanitize(&mut self) {
        self.schema_version = SCAN_STORE_SCHEMA_VERSION;
        if self.slots.len() > MAX_SAVED_SCAN_SNAPSHOTS {
            self.slots.truncate(MAX_SAVED_SCAN_SNAPSHOTS);
        }
        let mut max_id = 0_u64;
        let mut max_seq = 0_u64;
        for (i, s) in self.slots.iter_mut().enumerate() {
            if s.slot_id == 0 {
                s.slot_id = (i as u64) + 1;
            }
            if s.sequence_no == 0 {
                s.sequence_no = (i as u64) + 1;
            }
            s.sanitize();
            max_id = max_id.max(s.slot_id);
            max_seq = max_seq.max(s.sequence_no);
        }
        self.next_slot_id = self.next_slot_id.max(max_id.saturating_add(1)).max(1);
        self.next_sequence_no = self.next_sequence_no.max(max_seq.saturating_add(1)).max(1);
    }

    pub fn save_snapshot(
        &mut self,
        mut snap: SavedScanSnapshot,
    ) -> Result<u64, WorkspaceTemplateError> {
        let clean_label = snap.label.trim().to_owned();
        if let Some(existing) = self
            .slots
            .iter_mut()
            .find(|s| s.label.eq_ignore_ascii_case(&clean_label) && !clean_label.is_empty())
        {
            let id = existing.slot_id;
            let pinned = existing.pinned || snap.pinned;
            let seq = self.next_sequence_no.max(1);
            self.next_sequence_no = seq.saturating_add(1);
            snap.slot_id = id;
            snap.pinned = pinned;
            snap.sequence_no = seq;
            snap.sanitize();
            *existing = snap;
            return Ok(id);
        }

        if self.slots.len() >= MAX_SAVED_SCAN_SNAPSHOTS {
            let Some((evict_idx, _)) = self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.pinned)
                .min_by_key(|(_, s)| s.sequence_no)
            else {
                return Err(WorkspaceTemplateError::SnapshotSlotsFull);
            };
            self.slots.remove(evict_idx);
        }

        let id = self.next_slot_id.max(1);
        self.next_slot_id = id.saturating_add(1);
        let seq = self.next_sequence_no.max(1);
        self.next_sequence_no = seq.saturating_add(1);
        snap.slot_id = id;
        snap.sequence_no = seq;
        snap.sanitize();
        self.slots.push(snap);
        Ok(id)
    }

    pub fn delete_snapshot(&mut self, slot_id: u64) -> bool {
        let before = self.slots.len();
        self.slots.retain(|s| s.slot_id != slot_id);
        self.slots.len() < before
    }

    pub fn toggle_pin(&mut self, slot_id: u64) -> Option<bool> {
        let s = self.slots.iter_mut().find(|s| s.slot_id == slot_id)?;
        s.pinned = !s.pinned;
        s.snapshot_digest = s.compute_digest();
        Some(s.pinned)
    }

    #[must_use]
    pub fn get(&self, slot_id: u64) -> Option<&SavedScanSnapshot> {
        self.slots.iter().find(|s| s.slot_id == slot_id)
    }

    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut copy = self.clone();
        copy.sanitize();
        encode_json_scan_store(&copy)
    }

    #[must_use]
    pub fn to_ron_pretty(&self) -> String {
        let mut copy = self.clone();
        copy.sanitize();
        encode_ron_scan_store(&copy)
    }

    pub fn from_json_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let doc = parse_json_doc(input)?;
        Self::from_doc_value(&doc)
    }

    pub fn from_ron_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let doc = parse_ron_doc(input)?;
        Self::from_doc_value(&doc)
    }

    pub fn from_auto_str(input: &str) -> Result<Self, WorkspaceTemplateError> {
        let trimmed = strip_leading_ws_and_comments(input)?;
        if trimmed.starts_with('{') {
            Self::from_json_str(trimmed)
        } else {
            Self::from_ron_str(input)
        }
    }

    fn from_doc_value(val: &DocValue) -> Result<Self, WorkspaceTemplateError> {
        let fields = expect_struct(val, "ScanSnapshotStore")?;
        validate_struct_fields(
            fields,
            "ScanSnapshotStore",
            &[
                "schema_version",
                "next_slot_id",
                "next_sequence_no",
                "slots",
            ],
        )?;
        let ver_val = find_field(fields, "schema_version").ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: "schema_version".to_owned(),
                reason: "missing schema_version".to_owned(),
            }
        })?;
        let ver_f = expect_f64(ver_val, "schema_version")?;
        if ver_f < 0.0 || ver_f.fract() != 0.0 || ver_f as u32 != SCAN_STORE_SCHEMA_VERSION {
            return Err(WorkspaceTemplateError::UnsupportedSchemaVersion(
                ver_f.max(0.0) as u32,
            ));
        }
        let mut store = Self::new();
        if let Some(v) = find_field(fields, "next_slot_id") {
            store.next_slot_id = expect_u64_safe(v, "next_slot_id")?;
        }
        if let Some(v) = find_field(fields, "next_sequence_no") {
            store.next_sequence_no = expect_u64_safe(v, "next_sequence_no")?;
        }
        if let Some(v) = find_field(fields, "slots") {
            let items = match v {
                DocValue::Array(arr) => arr,
                _ => {
                    return Err(WorkspaceTemplateError::InvalidField {
                        field: "slots".to_owned(),
                        reason: "expected array".to_owned(),
                    });
                }
            };
            if items.len() > MAX_SAVED_SCAN_SNAPSHOTS {
                return Err(WorkspaceTemplateError::InvalidField {
                    field: "slots".to_owned(),
                    reason: format!(
                        "too many scan snapshot slots ({}, max {MAX_SAVED_SCAN_SNAPSHOTS})",
                        items.len()
                    ),
                });
            }
            for (idx, item) in items.iter().enumerate() {
                store
                    .slots
                    .push(decode_scan_slot(item, &format!("slots[{idx}]"))?);
            }
        }
        store.sanitize();
        Ok(store)
    }
}

// ============================================================================
// 5. Zero-Dependency `JSON` & `RON` Encoders / Decoders (`Practice` & `Scan`)
// ============================================================================

fn checklist_to_mask(c: ChecklistState) -> u8 {
    (u8::from(c.trend_aligned))
        | (u8::from(c.level_confirmed) << 1)
        | (u8::from(c.catalyst_or_volume_ok) << 2)
        | (u8::from(c.stop_and_lot_sized) << 3)
        | (u8::from(c.band_and_t2_checked) << 4)
        | (u8::from(c.calm_state) << 5)
}

fn checklist_from_mask(mask: u8) -> ChecklistState {
    ChecklistState {
        trend_aligned: (mask & (1 << 0)) != 0,
        level_confirmed: (mask & (1 << 1)) != 0,
        catalyst_or_volume_ok: (mask & (1 << 2)) != 0,
        stop_and_lot_sized: (mask & (1 << 3)) != 0,
        band_and_t2_checked: (mask & (1 << 4)) != 0,
        calm_state: (mask & (1 << 5)) != 0,
    }
}

fn setup_tag_id(s: SetupTag) -> &'static str {
    match s {
        SetupTag::Untagged => "Untagged",
        SetupTag::TrendPullback => "TrendPullback",
        SetupTag::SqueezeBreakout => "SqueezeBreakout",
        SetupTag::MeanReversion => "MeanReversion",
        SetupTag::SupportBounce => "SupportBounce",
        SetupTag::MomentumContinuation => "MomentumContinuation",
        SetupTag::LiquiditySweep => "LiquiditySweep",
    }
}

fn parse_setup_tag(raw: &str) -> Option<SetupTag> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "untagged" => Some(SetupTag::Untagged),
        "trendpullback" => Some(SetupTag::TrendPullback),
        "squeezebreakout" => Some(SetupTag::SqueezeBreakout),
        "meanreversion" => Some(SetupTag::MeanReversion),
        "supportbounce" => Some(SetupTag::SupportBounce),
        "momentumcontinuation" => Some(SetupTag::MomentumContinuation),
        "liquiditysweep" => Some(SetupTag::LiquiditySweep),
        _ => None,
    }
}

fn mistake_tag_id(m: MistakeTag) -> &'static str {
    match m {
        MistakeTag::None => "None",
        MistakeTag::FomoChase => "FomoChase",
        MistakeTag::MovedStopLoss => "MovedStopLoss",
        MistakeTag::EarlyExit => "EarlyExit",
        MistakeTag::OversizedRisk => "OversizedRisk",
        MistakeTag::CounterTrendImpulse => "CounterTrendImpulse",
        MistakeTag::IgnoredDailyBand => "IgnoredDailyBand",
        MistakeTag::SkippedChecklist => "SkippedChecklist",
    }
}

fn parse_mistake_tag(raw: &str) -> Option<MistakeTag> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Some(MistakeTag::None),
        "fomochase" => Some(MistakeTag::FomoChase),
        "movedstoploss" => Some(MistakeTag::MovedStopLoss),
        "earlyexit" => Some(MistakeTag::EarlyExit),
        "oversizedrisk" => Some(MistakeTag::OversizedRisk),
        "countertrendimpulse" => Some(MistakeTag::CounterTrendImpulse),
        "ignoreddailyband" => Some(MistakeTag::IgnoredDailyBand),
        "skippedchecklist" => Some(MistakeTag::SkippedChecklist),
        _ => None,
    }
}

fn signal_kind_id(s: ScanSignalKind) -> &'static str {
    match s {
        ScanSignalKind::RsiOversoldCrossUp => "RsiOversoldCrossUp",
        ScanSignalKind::RsiOverboughtCrossDown => "RsiOverboughtCrossDown",
        ScanSignalKind::MacdBullCross => "MacdBullCross",
        ScanSignalKind::MacdBearCross => "MacdBearCross",
        ScanSignalKind::SmaEmaGoldenCross => "SmaEmaGoldenCross",
        ScanSignalKind::SmaEmaDeathCross => "SmaEmaDeathCross",
        ScanSignalKind::BollingerSqueezeBreakout => "BollingerSqueezeBreakout",
        ScanSignalKind::VolumeSpike2x => "VolumeSpike2x",
    }
}

fn parse_signal_kind(raw: &str) -> Option<ScanSignalKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "rsioversoldcrossup" => Some(ScanSignalKind::RsiOversoldCrossUp),
        "rsioverboughtcrossdown" => Some(ScanSignalKind::RsiOverboughtCrossDown),
        "macdbullcross" => Some(ScanSignalKind::MacdBullCross),
        "macdbearcross" => Some(ScanSignalKind::MacdBearCross),
        "smaemagoldencross" => Some(ScanSignalKind::SmaEmaGoldenCross),
        "smaemadeathcross" => Some(ScanSignalKind::SmaEmaDeathCross),
        "bollingersqueezebreakout" => Some(ScanSignalKind::BollingerSqueezeBreakout),
        "volumespike2x" => Some(ScanSignalKind::VolumeSpike2x),
        _ => None,
    }
}

fn regime_id(r: MarketRegime) -> &'static str {
    match r {
        MarketRegime::Uptrend => "Uptrend",
        MarketRegime::Downtrend => "Downtrend",
        MarketRegime::Pullback => "Pullback",
        MarketRegime::Recovery => "Recovery",
        MarketRegime::SidewaysSqueeze => "SidewaysSqueeze",
        MarketRegime::Neutral => "Neutral",
        MarketRegime::InsufficientHistory => "InsufficientHistory",
    }
}

fn parse_regime(raw: &str) -> Option<MarketRegime> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "uptrend" => Some(MarketRegime::Uptrend),
        "downtrend" => Some(MarketRegime::Downtrend),
        "pullback" => Some(MarketRegime::Pullback),
        "recovery" => Some(MarketRegime::Recovery),
        "sidewayssqueeze" => Some(MarketRegime::SidewaysSqueeze),
        "neutral" => Some(MarketRegime::Neutral),
        "insufficienthistory" => Some(MarketRegime::InsufficientHistory),
        _ => None,
    }
}

fn direction_id(d: SignalDirection) -> &'static str {
    match d {
        SignalDirection::Bullish => "Bullish",
        SignalDirection::Bearish => "Bearish",
    }
}

fn parse_direction(raw: &str) -> Option<SignalDirection> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bullish" => Some(SignalDirection::Bullish),
        "bearish" => Some(SignalDirection::Bearish),
        _ => None,
    }
}

fn sort_col_id(c: WatchlistSortColumn) -> &'static str {
    match c {
        WatchlistSortColumn::Symbol => "Symbol",
        WatchlistSortColumn::LastClose => "LastClose",
        WatchlistSortColumn::ChangePct => "ChangePct",
        WatchlistSortColumn::RelativeVolume => "RelativeVolume",
        WatchlistSortColumn::Rsi14 => "Rsi14",
        WatchlistSortColumn::SignalCount => "SignalCount",
    }
}

fn parse_sort_col(raw: &str) -> Option<WatchlistSortColumn> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "symbol" => Some(WatchlistSortColumn::Symbol),
        "lastclose" => Some(WatchlistSortColumn::LastClose),
        "changepct" => Some(WatchlistSortColumn::ChangePct),
        "relativevolume" => Some(WatchlistSortColumn::RelativeVolume),
        "rsi14" => Some(WatchlistSortColumn::Rsi14),
        "signalcount" => Some(WatchlistSortColumn::SignalCount),
        _ => None,
    }
}

fn intent_id(i: ScannerReplayIntent) -> &'static str {
    match i {
        ScannerReplayIntent::BlindPractice => "BlindPractice",
        ScannerReplayIntent::SignalReview => "SignalReview",
    }
}

fn parse_intent(raw: &str) -> Option<ScannerReplayIntent> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "blindpractice" => Some(ScannerReplayIntent::BlindPractice),
        "signalreview" => Some(ScannerReplayIntent::SignalReview),
        _ => None,
    }
}

fn expect_opt_i64(val: &DocValue, field: &str) -> Result<Option<i64>, WorkspaceTemplateError> {
    match val {
        DocValue::None => Ok(None),
        DocValue::Some(inner) => expect_i64_safe(inner, field).map(Some),
        DocValue::Number(_) => expect_i64_safe(val, field).map(Some),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected null/None or integer".to_owned(),
        }),
    }
}

fn expect_opt_f64(val: &DocValue, field: &str) -> Result<Option<f64>, WorkspaceTemplateError> {
    match val {
        DocValue::None => Ok(None),
        DocValue::Some(inner) => expect_f64(inner, field).map(Some),
        DocValue::Number(_) => expect_f64(val, field).map(Some),
        _ => Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: "expected null/None or finite number".to_owned(),
        }),
    }
}

fn parse_hex_u64(val: &DocValue, field: &str) -> Result<u64, WorkspaceTemplateError> {
    let raw = expect_string(val, field)?;
    let hex = raw.strip_prefix("0x").unwrap_or(&raw);
    if hex.len() != 16 {
        return Err(WorkspaceTemplateError::InvalidField {
            field: field.to_owned(),
            reason: format!("expected 16-hex digest, got '{raw}'"),
        });
    }
    u64::from_str_radix(hex, 16).map_err(|_| WorkspaceTemplateError::InvalidField {
        field: field.to_owned(),
        reason: format!("invalid hex digest '{raw}'"),
    })
}

fn encode_json_practice_store(store: &PracticeSessionStore) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("{\n");
    let _ = writeln!(out, "  \"schema_version\": {},", store.schema_version);
    let _ = writeln!(out, "  \"next_slot_id\": {},", store.next_slot_id);
    let _ = writeln!(out, "  \"next_sequence_no\": {},", store.next_sequence_no);
    if store.slots.is_empty() {
        out.push_str("  \"slots\": []\n}");
        return out;
    }
    out.push_str("  \"slots\": [\n");
    for (i, s) in store.slots.iter().enumerate() {
        encode_json_practice_slot(&mut out, s);
        if i + 1 == store.slots.len() {
            out.push('\n');
        } else {
            out.push_str(",\n");
        }
    }
    out.push_str("  ]\n}");
    out
}

fn encode_json_opt_i64(out: &mut String, v: Option<i64>) {
    match v {
        Some(ts) => {
            let _ = write!(out, "{ts}");
        }
        None => out.push_str("null"),
    }
}

fn encode_json_opt_f64(out: &mut String, v: Option<f64>) {
    match v {
        Some(x) => out.push_str(&fmt_f64(x)),
        None => out.push_str("null"),
    }
}

fn encode_json_hit(out: &mut String, h: &HistoricalSignalHit) {
    out.push_str("{ \"instrument_id\": ");
    escape_str_into(out, &h.instrument_id);
    out.push_str(", \"symbol\": ");
    escape_str_into(out, &h.symbol);
    let _ = write!(
        out,
        ", \"timeframe\": \"{:?}\", \"bar_index\": {}, \"timestamp\": {}, \"price\": {}, \"signal\": \"{}\", \"direction\": \"{}\", \"regime\": \"{}\", \"metric_value\": {} }}",
        h.timeframe,
        h.bar_index,
        h.timestamp,
        fmt_f64(h.price),
        signal_kind_id(h.signal),
        direction_id(h.direction),
        regime_id(h.regime),
        fmt_f64(h.metric_value)
    );
}

fn encode_json_practice_slot(out: &mut String, s: &SavedPracticeSession) {
    out.push_str("    {\n");
    let _ = writeln!(out, "      \"slot_id\": {},", s.slot_id);
    out.push_str("      \"label\": ");
    escape_str_into(out, &s.label);
    let _ = writeln!(out, ",\n      \"pinned\": {},", s.pinned);
    let _ = writeln!(out, "      \"sequence_no\": {},", s.sequence_no);
    let _ = writeln!(out, "      \"data_source\": \"{:?}\",", s.data_source);
    out.push_str("      \"symbol\": ");
    escape_str_into(out, &s.symbol);
    let _ = writeln!(
        out,
        ",\n      \"base_timeframe\": \"{:?}\",",
        s.base_timeframe
    );
    let _ = writeln!(
        out,
        "      \"active_timeframe\": \"{:?}\",",
        s.active_timeframe
    );
    let _ = writeln!(
        out,
        "      \"expected_bar_count\": {},",
        s.expected_bar_count
    );
    out.push_str("      \"expected_first_timestamp\": ");
    encode_json_opt_i64(out, s.expected_first_timestamp);
    out.push_str(",\n      \"expected_last_timestamp\": ");
    encode_json_opt_i64(out, s.expected_last_timestamp);
    let _ = writeln!(out, ",\n      \"replay_active\": {},", s.replay_active);
    let _ = writeln!(
        out,
        "      \"replay_start_base_index\": {},",
        s.replay_start_base_index
    );
    let _ = writeln!(
        out,
        "      \"replay_cursor_base_index\": {},",
        s.replay_cursor_base_index
    );
    let _ = writeln!(
        out,
        "      \"blind_lookback_bars\": {},",
        s.blind_lookback_bars
    );
    out.push_str("      \"scanner_context\": ");
    match &s.scanner_context {
        None => out.push_str("null"),
        Some(ctx) => {
            let hit = ctx
                .revealed_signal(Some(i64::MAX))
                .expect("i64::MAX reveals underlying signal for persistence");
            let preroll_json = match ctx.pre_roll_bars() {
                Some(n) => n.to_string(),
                None => "null".to_owned(),
            };
            let _ = write!(
                out,
                "{{ \"intent\": \"{}\", \"reveal_at_timestamp\": {}, \"pre_roll_bars\": {preroll_json}, \"signal\": ",
                intent_id(ctx.intent()),
                ctx.reveal_at_timestamp()
            );
            encode_json_hit(out, hit);
            out.push_str(" }");
        }
    }
    let _ = writeln!(
        out,
        ",\n      \"account_config\": {{ \"initial_cash\": {}, \"commission_bps\": {}, \"slippage_bps\": {}, \"allow_short\": {}, \"max_leverage\": {} }},",
        fmt_f64(s.account_config.initial_cash),
        fmt_f64(s.account_config.commission_bps),
        fmt_f64(s.account_config.slippage_bps),
        s.account_config.allow_short,
        fmt_f64(s.account_config.max_leverage),
    );
    let _ = writeln!(
        out,
        "      \"session_start_base_index\": {},",
        s.session_start_base_index
    );
    out.push_str("      \"commands\": [");
    for (ci, cmd) in s.commands.iter().enumerate() {
        if ci > 0 {
            out.push_str(", ");
        }
        encode_json_command(out, cmd);
    }
    out.push_str("],\n      \"decision_notes\": [");
    for (ni, n) in s.decision_notes.iter().enumerate() {
        if ni > 0 {
            out.push_str(", ");
        }
        encode_json_note(out, n);
    }
    let _ = writeln!(
        out,
        "],\n      \"session_digest_hex\": \"{:016x}\"",
        s.session_digest
    );
    out.push_str("    }");
}

fn encode_json_command(out: &mut String, cmd: &RecordedCommand) {
    let _ = write!(out, "{{ \"base_index\": {}, ", cmd.base_index);
    match &cmd.kind {
        RecordedCommandKind::SubmitOrder(req) => {
            let side = match req.side {
                OrderSide::Buy => "Buy",
                OrderSide::Sell => "Sell",
            };
            let (kind_str, px_opt) = match req.kind {
                OrderKind::Market => ("Market", None),
                OrderKind::Limit { limit_price } => ("Limit", Some(limit_price)),
                OrderKind::Stop { stop_price } => ("Stop", Some(stop_price)),
            };
            let _ = write!(
                out,
                "\"kind\": \"SubmitOrder\", \"side\": \"{side}\", \"order_kind\": \"{kind_str}\", \"trigger_price\": "
            );
            encode_json_opt_f64(out, px_opt);
            let _ = write!(
                out,
                ", \"quantity\": {}, \"stop_loss\": ",
                fmt_f64(req.quantity)
            );
            encode_json_opt_f64(out, req.stop_loss);
            out.push_str(", \"take_profit\": ");
            encode_json_opt_f64(out, req.take_profit);
            out.push_str(" }");
        }
        RecordedCommandKind::CancelOrder { order_id } => {
            let _ = write!(
                out,
                "\"kind\": \"CancelOrder\", \"order_id\": {order_id} }}"
            );
        }
        RecordedCommandKind::UpdateBrackets {
            stop_loss,
            take_profit,
        } => {
            out.push_str("\"kind\": \"UpdateBrackets\", \"stop_loss\": ");
            encode_json_opt_f64(out, *stop_loss);
            out.push_str(", \"take_profit\": ");
            encode_json_opt_f64(out, *take_profit);
            out.push_str(" }");
        }
        RecordedCommandKind::FlattenPosition => {
            out.push_str("\"kind\": \"FlattenPosition\" }");
        }
    }
}

fn encode_json_note(out: &mut String, n: &PracticeDecisionNote) {
    let _ = write!(out, "{{ \"id\": {}, \"symbol\": ", n.id);
    escape_str_into(out, &n.symbol);
    let _ = write!(
        out,
        ", \"timeframe\": \"{:?}\", \"base_bar_index\": {}, \"bar_timestamp\": {}, \"mark_price\": {}, \"action\": \"{}\", \"setup\": \"{}\", \"mistake\": \"{}\", \"conviction\": {}, \"checklist_mask\": {}, \"note\": ",
        n.timeframe,
        n.base_bar_index,
        n.bar_timestamp,
        fmt_f64(n.mark_price),
        n.action.as_id(),
        setup_tag_id(n.setup),
        mistake_tag_id(n.mistake),
        n.conviction,
        checklist_to_mask(n.checklist),
    );
    escape_str_into(out, &n.note);
    out.push_str(" }");
}

fn encode_ron_practice_store(store: &PracticeSessionStore) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("(\n");
    let _ = writeln!(out, "    schema_version: {},", store.schema_version);
    let _ = writeln!(out, "    next_slot_id: {},", store.next_slot_id);
    let _ = writeln!(out, "    next_sequence_no: {},", store.next_sequence_no);
    if store.slots.is_empty() {
        out.push_str("    slots: [],\n)");
        return out;
    }
    out.push_str("    slots: [\n");
    for s in &store.slots {
        encode_ron_practice_slot(&mut out, s, 8, true);
        out.push_str(",\n");
    }
    out.push_str("    ],\n)");
    out
}

fn encode_ron_opt_i64(out: &mut String, v: Option<i64>) {
    match v {
        Some(ts) => {
            let _ = write!(out, "Some({ts})");
        }
        None => out.push_str("None"),
    }
}

fn encode_ron_opt_f64(out: &mut String, v: Option<f64>) {
    match v {
        Some(x) => {
            let _ = write!(out, "Some({})", fmt_f64(x));
        }
        None => out.push_str("None"),
    }
}

fn encode_ron_hit(out: &mut String, h: &HistoricalSignalHit) {
    out.push_str("(instrument_id: ");
    escape_str_into(out, &h.instrument_id);
    out.push_str(", symbol: ");
    escape_str_into(out, &h.symbol);
    let _ = write!(
        out,
        ", timeframe: {:?}, bar_index: {}, timestamp: {}, price: {}, signal: {}, direction: {}, regime: {}, metric_value: {})",
        h.timeframe,
        h.bar_index,
        h.timestamp,
        fmt_f64(h.price),
        signal_kind_id(h.signal),
        direction_id(h.direction),
        regime_id(h.regime),
        fmt_f64(h.metric_value)
    );
}

fn encode_ron_practice_slot(
    out: &mut String,
    s: &SavedPracticeSession,
    indent: usize,
    include_digest: bool,
) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 4);
    let _ = writeln!(out, "{pad}(");
    let _ = writeln!(out, "{inner}slot_id: {},", s.slot_id);
    let _ = write!(out, "{inner}label: ");
    escape_str_into(out, &s.label);
    let _ = writeln!(out, ",");
    let _ = writeln!(out, "{inner}pinned: {},", s.pinned);
    let _ = writeln!(out, "{inner}sequence_no: {},", s.sequence_no);
    let _ = writeln!(out, "{inner}data_source: {:?},", s.data_source);
    let _ = write!(out, "{inner}symbol: ");
    escape_str_into(out, &s.symbol);
    let _ = writeln!(out, ",");
    let _ = writeln!(out, "{inner}base_timeframe: {:?},", s.base_timeframe);
    let _ = writeln!(out, "{inner}active_timeframe: {:?},", s.active_timeframe);
    let _ = writeln!(out, "{inner}expected_bar_count: {},", s.expected_bar_count);
    let _ = write!(out, "{inner}expected_first_timestamp: ");
    encode_ron_opt_i64(out, s.expected_first_timestamp);
    out.push_str(",\n");
    let _ = write!(out, "{inner}expected_last_timestamp: ");
    encode_ron_opt_i64(out, s.expected_last_timestamp);
    out.push_str(",\n");
    let _ = writeln!(out, "{inner}replay_active: {},", s.replay_active);
    let _ = writeln!(
        out,
        "{inner}replay_start_base_index: {},",
        s.replay_start_base_index
    );
    let _ = writeln!(
        out,
        "{inner}replay_cursor_base_index: {},",
        s.replay_cursor_base_index
    );
    let _ = writeln!(
        out,
        "{inner}blind_lookback_bars: {},",
        s.blind_lookback_bars
    );
    let _ = write!(out, "{inner}scanner_context: ");
    match &s.scanner_context {
        None => out.push_str("None"),
        Some(ctx) => {
            let hit = ctx
                .revealed_signal(Some(i64::MAX))
                .expect("i64::MAX reveals underlying signal for persistence");
            let preroll_ron = match ctx.pre_roll_bars() {
                Some(n) => format!("Some({n})"),
                None => "None".to_owned(),
            };
            let _ = write!(
                out,
                "Some((intent: {}, reveal_at_timestamp: {}, pre_roll_bars: {preroll_ron}, signal: ",
                intent_id(ctx.intent()),
                ctx.reveal_at_timestamp()
            );
            encode_ron_hit(out, hit);
            out.push_str("))");
        }
    }
    let _ = writeln!(
        out,
        ",\n{inner}account_config: (initial_cash: {}, commission_bps: {}, slippage_bps: {}, allow_short: {}, max_leverage: {}),",
        fmt_f64(s.account_config.initial_cash),
        fmt_f64(s.account_config.commission_bps),
        fmt_f64(s.account_config.slippage_bps),
        s.account_config.allow_short,
        fmt_f64(s.account_config.max_leverage),
    );
    let _ = writeln!(
        out,
        "{inner}session_start_base_index: {},",
        s.session_start_base_index
    );
    let _ = write!(out, "{inner}commands: [");
    for (ci, cmd) in s.commands.iter().enumerate() {
        if ci > 0 {
            out.push_str(", ");
        }
        encode_ron_command(out, cmd);
    }
    let _ = write!(out, "],\n{inner}decision_notes: [");
    for (ni, n) in s.decision_notes.iter().enumerate() {
        if ni > 0 {
            out.push_str(", ");
        }
        encode_ron_note(out, n);
    }
    out.push_str("],\n");
    if include_digest {
        let _ = writeln!(
            out,
            "{inner}session_digest_hex: \"{:016x}\",",
            s.session_digest
        );
    }
    let _ = write!(out, "{pad})");
}

fn encode_ron_command(out: &mut String, cmd: &RecordedCommand) {
    let _ = write!(out, "(base_index: {}, ", cmd.base_index);
    match &cmd.kind {
        RecordedCommandKind::SubmitOrder(req) => {
            let side = match req.side {
                OrderSide::Buy => "Buy",
                OrderSide::Sell => "Sell",
            };
            let (kind_str, px_opt) = match req.kind {
                OrderKind::Market => ("Market", None),
                OrderKind::Limit { limit_price } => ("Limit", Some(limit_price)),
                OrderKind::Stop { stop_price } => ("Stop", Some(stop_price)),
            };
            let _ = write!(
                out,
                "kind: SubmitOrder, side: {side}, order_kind: {kind_str}, trigger_price: "
            );
            encode_ron_opt_f64(out, px_opt);
            let _ = write!(out, ", quantity: {}, stop_loss: ", fmt_f64(req.quantity));
            encode_ron_opt_f64(out, req.stop_loss);
            out.push_str(", take_profit: ");
            encode_ron_opt_f64(out, req.take_profit);
            out.push(')');
        }
        RecordedCommandKind::CancelOrder { order_id } => {
            let _ = write!(out, "kind: CancelOrder, order_id: {order_id})");
        }
        RecordedCommandKind::UpdateBrackets {
            stop_loss,
            take_profit,
        } => {
            out.push_str("kind: UpdateBrackets, stop_loss: ");
            encode_ron_opt_f64(out, *stop_loss);
            out.push_str(", take_profit: ");
            encode_ron_opt_f64(out, *take_profit);
            out.push(')');
        }
        RecordedCommandKind::FlattenPosition => {
            out.push_str("kind: FlattenPosition)");
        }
    }
}

fn encode_ron_note(out: &mut String, n: &PracticeDecisionNote) {
    let _ = write!(out, "(id: {}, symbol: ", n.id);
    escape_str_into(out, &n.symbol);
    let _ = write!(
        out,
        ", timeframe: {:?}, base_bar_index: {}, bar_timestamp: {}, mark_price: {}, action: {}, setup: {}, mistake: {}, conviction: {}, checklist_mask: {}, note: ",
        n.timeframe,
        n.base_bar_index,
        n.bar_timestamp,
        fmt_f64(n.mark_price),
        n.action.as_id(),
        setup_tag_id(n.setup),
        mistake_tag_id(n.mistake),
        n.conviction,
        checklist_to_mask(n.checklist),
    );
    escape_str_into(out, &n.note);
    out.push(')');
}

fn decode_hit(val: &DocValue, ctx: &str) -> Result<HistoricalSignalHit, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(
        f,
        ctx,
        &[
            "instrument_id",
            "symbol",
            "timeframe",
            "bar_index",
            "timestamp",
            "price",
            "signal",
            "direction",
            "regime",
            "metric_value",
        ],
    )?;
    let instrument_id = expect_string(
        find_field(f, "instrument_id").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.instrument_id"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.instrument_id"),
    )?;
    let symbol = expect_string(
        find_field(f, "symbol").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.symbol"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.symbol"),
    )?;
    let tf_raw = expect_label(
        find_field(f, "timeframe").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.timeframe"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.timeframe"),
    )?;
    let timeframe =
        parse_timeframe(&tf_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.timeframe"),
            reason: format!("unknown Timeframe '{tf_raw}'"),
        })?;
    let bar_index = expect_usize(
        find_field(f, "bar_index").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.bar_index"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.bar_index"),
    )?;
    let timestamp = expect_i64_safe(
        find_field(f, "timestamp").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.timestamp"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.timestamp"),
    )?;
    let price = expect_f64(
        find_field(f, "price").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.price"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.price"),
    )?;
    let sig_raw = expect_label(
        find_field(f, "signal").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.signal"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.signal"),
    )?;
    let signal =
        parse_signal_kind(&sig_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.signal"),
            reason: format!("unknown ScanSignalKind '{sig_raw}'"),
        })?;
    let dir_raw = expect_label(
        find_field(f, "direction").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.direction"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.direction"),
    )?;
    let direction =
        parse_direction(&dir_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.direction"),
            reason: format!("unknown SignalDirection '{dir_raw}'"),
        })?;
    let reg_raw = expect_label(
        find_field(f, "regime").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.regime"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.regime"),
    )?;
    let regime = parse_regime(&reg_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.regime"),
        reason: format!("unknown MarketRegime '{reg_raw}'"),
    })?;
    let metric_value = expect_f64(
        find_field(f, "metric_value").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.metric_value"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.metric_value"),
    )?;

    Ok(HistoricalSignalHit {
        instrument_id,
        symbol,
        timeframe,
        bar_index,
        timestamp,
        price,
        signal,
        direction,
        regime,
        metric_value,
    })
}

fn decode_practice_slot(
    val: &DocValue,
    ctx: &str,
) -> Result<SavedPracticeSession, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(
        f,
        ctx,
        &[
            "slot_id",
            "label",
            "pinned",
            "sequence_no",
            "data_source",
            "symbol",
            "base_timeframe",
            "active_timeframe",
            "expected_bar_count",
            "expected_first_timestamp",
            "expected_last_timestamp",
            "replay_active",
            "replay_start_base_index",
            "replay_cursor_base_index",
            "blind_lookback_bars",
            "scanner_context",
            "account_config",
            "session_start_base_index",
            "commands",
            "decision_notes",
            "session_digest_hex",
        ],
    )?;

    let req_field = |k: &str| {
        find_field(f, k).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.{k}"),
            reason: format!("missing '{k}'"),
        })
    };

    let slot_id = expect_u64_safe(req_field("slot_id")?, &format!("{ctx}.slot_id"))?;
    let label = expect_string(req_field("label")?, &format!("{ctx}.label"))?;
    let pinned = expect_bool(req_field("pinned")?, &format!("{ctx}.pinned"))?;
    let sequence_no = expect_u64_safe(req_field("sequence_no")?, &format!("{ctx}.sequence_no"))?;
    let ds_raw = expect_label(req_field("data_source")?, &format!("{ctx}.data_source"))?;
    let data_source =
        parse_data_source(&ds_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.data_source"),
            reason: format!("unknown DataSourcePreset '{ds_raw}'"),
        })?;
    let symbol = expect_string(req_field("symbol")?, &format!("{ctx}.symbol"))?;
    let btf_raw = expect_label(
        req_field("base_timeframe")?,
        &format!("{ctx}.base_timeframe"),
    )?;
    let base_timeframe =
        parse_timeframe(&btf_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.base_timeframe"),
            reason: format!("unknown Timeframe '{btf_raw}'"),
        })?;
    let atf_raw = expect_label(
        req_field("active_timeframe")?,
        &format!("{ctx}.active_timeframe"),
    )?;
    let active_timeframe =
        parse_timeframe(&atf_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.active_timeframe"),
            reason: format!("unknown Timeframe '{atf_raw}'"),
        })?;
    let expected_bar_count = expect_usize(
        req_field("expected_bar_count")?,
        &format!("{ctx}.expected_bar_count"),
    )?;
    let expected_first_timestamp = match find_field(f, "expected_first_timestamp") {
        Some(v) => expect_opt_i64(v, &format!("{ctx}.expected_first_timestamp"))?,
        None => None,
    };
    let expected_last_timestamp = match find_field(f, "expected_last_timestamp") {
        Some(v) => expect_opt_i64(v, &format!("{ctx}.expected_last_timestamp"))?,
        None => None,
    };
    let replay_active = expect_bool(req_field("replay_active")?, &format!("{ctx}.replay_active"))?;
    let replay_start_base_index = expect_usize(
        req_field("replay_start_base_index")?,
        &format!("{ctx}.replay_start_base_index"),
    )?;
    let replay_cursor_base_index = expect_usize(
        req_field("replay_cursor_base_index")?,
        &format!("{ctx}.replay_cursor_base_index"),
    )?;
    let blind_lookback_bars = expect_usize(
        req_field("blind_lookback_bars")?,
        &format!("{ctx}.blind_lookback_bars"),
    )?;

    let scanner_context = match find_field(f, "scanner_context") {
        None | Some(DocValue::None) => None,
        Some(DocValue::Some(inner)) => Some(decode_scanner_ctx(
            inner,
            &format!("{ctx}.scanner_context"),
        )?),
        Some(other) => Some(decode_scanner_ctx(
            other,
            &format!("{ctx}.scanner_context"),
        )?),
    };

    let ac_fields = expect_struct(
        req_field("account_config")?,
        &format!("{ctx}.account_config"),
    )?;
    validate_struct_fields(
        ac_fields,
        &format!("{ctx}.account_config"),
        &[
            "initial_cash",
            "commission_bps",
            "slippage_bps",
            "allow_short",
            "max_leverage",
        ],
    )?;
    let mut account_config = AccountConfig::default();
    if let Some(v) = find_field(ac_fields, "initial_cash") {
        account_config.initial_cash = expect_f64(v, &format!("{ctx}.account_config.initial_cash"))?;
    }
    if let Some(v) = find_field(ac_fields, "commission_bps") {
        account_config.commission_bps =
            expect_f64(v, &format!("{ctx}.account_config.commission_bps"))?;
    }
    if let Some(v) = find_field(ac_fields, "slippage_bps") {
        account_config.slippage_bps = expect_f64(v, &format!("{ctx}.account_config.slippage_bps"))?;
    }
    if let Some(v) = find_field(ac_fields, "allow_short") {
        account_config.allow_short = expect_bool(v, &format!("{ctx}.account_config.allow_short"))?;
    }
    if let Some(v) = find_field(ac_fields, "max_leverage") {
        account_config.max_leverage = expect_f64(v, &format!("{ctx}.account_config.max_leverage"))?;
    }

    let session_start_base_index = expect_usize(
        req_field("session_start_base_index")?,
        &format!("{ctx}.session_start_base_index"),
    )?;

    let cmd_items = match req_field("commands")? {
        DocValue::Array(arr) => arr,
        _ => {
            return Err(WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.commands"),
                reason: "expected array".to_owned(),
            });
        }
    };
    if cmd_items.len() > MAX_RECORDED_COMMANDS {
        return Err(WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.commands"),
            reason: "too many commands".to_owned(),
        });
    }
    let mut commands = Vec::with_capacity(cmd_items.len());
    for (ci, item) in cmd_items.iter().enumerate() {
        commands.push(decode_command(item, &format!("{ctx}.commands[{ci}]"))?);
    }

    let note_items = match req_field("decision_notes")? {
        DocValue::Array(arr) => arr,
        _ => {
            return Err(WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.decision_notes"),
                reason: "expected array".to_owned(),
            });
        }
    };
    if note_items.len() > MAX_DECISION_NOTES {
        return Err(WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.decision_notes"),
            reason: "too many decision_notes".to_owned(),
        });
    }
    let mut decision_notes = Vec::with_capacity(note_items.len());
    for (ni, item) in note_items.iter().enumerate() {
        decision_notes.push(decode_note(item, &format!("{ctx}.decision_notes[{ni}]"))?);
    }

    let expected_digest = if let Some(v) = find_field(f, "session_digest_hex") {
        Some(parse_hex_u64(v, &format!("{ctx}.session_digest_hex"))?)
    } else {
        None
    };

    let mut slot = SavedPracticeSession {
        slot_id,
        label,
        pinned,
        sequence_no,
        data_source,
        symbol,
        base_timeframe,
        active_timeframe,
        expected_bar_count,
        expected_first_timestamp,
        expected_last_timestamp,
        replay_active,
        replay_start_base_index,
        replay_cursor_base_index,
        blind_lookback_bars,
        scanner_context,
        account_config,
        session_start_base_index,
        commands,
        decision_notes,
        session_digest: 0,
    };
    slot.sanitize();
    if let Some(exp) = expected_digest
        && exp != slot.session_digest
    {
        return Err(WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.session_digest_hex"),
            reason: format!(
                "Corrupt practice session digest (expected {exp:016x}, actual {:016x})",
                slot.session_digest
            ),
        });
    }
    Ok(slot)
}

fn decode_scanner_ctx(
    val: &DocValue,
    ctx: &str,
) -> Result<ScannerSignalContext, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(
        f,
        ctx,
        &["intent", "reveal_at_timestamp", "pre_roll_bars", "signal"],
    )?;
    let intent_raw = expect_label(
        find_field(f, "intent").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.intent"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.intent"),
    )?;
    let intent = parse_intent(&intent_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.intent"),
        reason: format!("unknown ScannerReplayIntent '{intent_raw}'"),
    })?;
    let reveal_at_timestamp = expect_i64_safe(
        find_field(f, "reveal_at_timestamp").ok_or_else(|| {
            WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.reveal_at_timestamp"),
                reason: "missing".to_owned(),
            }
        })?,
        &format!("{ctx}.reveal_at_timestamp"),
    )?;
    let pre_roll_bars = match find_field(f, "pre_roll_bars") {
        None | Some(DocValue::None) => None,
        Some(DocValue::Some(inner)) => {
            let n = expect_usize(inner, &format!("{ctx}.pre_roll_bars"))?;
            if !(1..=500).contains(&n) {
                return Err(WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.pre_roll_bars"),
                    reason: format!("pre_roll_bars {n} must be in 1..=500"),
                });
            }
            Some(n)
        }
        Some(other) => {
            let n = expect_usize(other, &format!("{ctx}.pre_roll_bars"))?;
            if !(1..=500).contains(&n) {
                return Err(WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.pre_roll_bars"),
                    reason: format!("pre_roll_bars {n} must be in 1..=500"),
                });
            }
            Some(n)
        }
    };
    let sig_val = find_field(f, "signal").ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.signal"),
        reason: "missing".to_owned(),
    })?;
    let hit = decode_hit(sig_val, &format!("{ctx}.signal"))?;
    Ok(ScannerSignalContext::with_reveal_and_preroll(
        intent,
        hit,
        reveal_at_timestamp,
        pre_roll_bars,
    ))
}

fn decode_command(val: &DocValue, ctx: &str) -> Result<RecordedCommand, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(
        f,
        ctx,
        &[
            "base_index",
            "kind",
            "side",
            "order_kind",
            "trigger_price",
            "quantity",
            "stop_loss",
            "take_profit",
            "order_id",
        ],
    )?;
    let base_index = expect_usize(
        find_field(f, "base_index").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.base_index"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.base_index"),
    )?;
    let kind_raw = expect_label(
        find_field(f, "kind").ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.kind"),
            reason: "missing".to_owned(),
        })?,
        &format!("{ctx}.kind"),
    )?;
    let kind = match kind_raw.as_str() {
        "SubmitOrder" => {
            let side_raw = expect_label(
                find_field(f, "side").ok_or_else(|| WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.side"),
                    reason: "missing".to_owned(),
                })?,
                &format!("{ctx}.side"),
            )?;
            let side = match side_raw.as_str() {
                "Buy" => OrderSide::Buy,
                "Sell" => OrderSide::Sell,
                _ => {
                    return Err(WorkspaceTemplateError::InvalidField {
                        field: format!("{ctx}.side"),
                        reason: format!("unknown OrderSide '{side_raw}'"),
                    });
                }
            };
            let ok_raw = expect_label(
                find_field(f, "order_kind").ok_or_else(|| {
                    WorkspaceTemplateError::InvalidField {
                        field: format!("{ctx}.order_kind"),
                        reason: "missing".to_owned(),
                    }
                })?,
                &format!("{ctx}.order_kind"),
            )?;
            let trig_opt = match find_field(f, "trigger_price") {
                Some(v) => expect_opt_f64(v, &format!("{ctx}.trigger_price"))?,
                None => None,
            };
            let order_kind = match ok_raw.as_str() {
                "Market" => OrderKind::Market,
                "Limit" => OrderKind::Limit {
                    limit_price: trig_opt.ok_or_else(|| WorkspaceTemplateError::InvalidField {
                        field: format!("{ctx}.trigger_price"),
                        reason: "missing limit_price".to_owned(),
                    })?,
                },
                "Stop" => OrderKind::Stop {
                    stop_price: trig_opt.ok_or_else(|| WorkspaceTemplateError::InvalidField {
                        field: format!("{ctx}.trigger_price"),
                        reason: "missing stop_price".to_owned(),
                    })?,
                },
                _ => {
                    return Err(WorkspaceTemplateError::InvalidField {
                        field: format!("{ctx}.order_kind"),
                        reason: format!("unknown OrderKind '{ok_raw}'"),
                    });
                }
            };
            let quantity = expect_f64(
                find_field(f, "quantity").ok_or_else(|| WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.quantity"),
                    reason: "missing".to_owned(),
                })?,
                &format!("{ctx}.quantity"),
            )?;
            let stop_loss = match find_field(f, "stop_loss") {
                Some(v) => expect_opt_f64(v, &format!("{ctx}.stop_loss"))?,
                None => None,
            };
            let take_profit = match find_field(f, "take_profit") {
                Some(v) => expect_opt_f64(v, &format!("{ctx}.take_profit"))?,
                None => None,
            };
            RecordedCommandKind::SubmitOrder(OrderRequest {
                side,
                kind: order_kind,
                quantity,
                stop_loss,
                take_profit,
            })
        }
        "CancelOrder" => {
            let order_id = expect_u64_safe(
                find_field(f, "order_id").ok_or_else(|| WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.order_id"),
                    reason: "missing".to_owned(),
                })?,
                &format!("{ctx}.order_id"),
            )?;
            RecordedCommandKind::CancelOrder { order_id }
        }
        "UpdateBrackets" => {
            let stop_loss = match find_field(f, "stop_loss") {
                Some(v) => expect_opt_f64(v, &format!("{ctx}.stop_loss"))?,
                None => None,
            };
            let take_profit = match find_field(f, "take_profit") {
                Some(v) => expect_opt_f64(v, &format!("{ctx}.take_profit"))?,
                None => None,
            };
            RecordedCommandKind::UpdateBrackets {
                stop_loss,
                take_profit,
            }
        }
        "FlattenPosition" => RecordedCommandKind::FlattenPosition,
        other => {
            return Err(WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.kind"),
                reason: format!("unknown RecordedCommandKind '{other}'"),
            });
        }
    };
    Ok(RecordedCommand { base_index, kind })
}

fn decode_note(val: &DocValue, ctx: &str) -> Result<PracticeDecisionNote, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(
        f,
        ctx,
        &[
            "id",
            "symbol",
            "timeframe",
            "base_bar_index",
            "bar_timestamp",
            "mark_price",
            "action",
            "setup",
            "mistake",
            "conviction",
            "checklist_mask",
            "note",
        ],
    )?;
    let req = |k: &str| {
        find_field(f, k).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.{k}"),
            reason: format!("missing '{k}'"),
        })
    };
    let id = expect_u64_safe(req("id")?, &format!("{ctx}.id"))?;
    let symbol = expect_string(req("symbol")?, &format!("{ctx}.symbol"))?;
    let tf_raw = expect_label(req("timeframe")?, &format!("{ctx}.timeframe"))?;
    let timeframe =
        parse_timeframe(&tf_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.timeframe"),
            reason: format!("unknown Timeframe '{tf_raw}'"),
        })?;
    let base_bar_index = expect_usize(req("base_bar_index")?, &format!("{ctx}.base_bar_index"))?;
    let bar_timestamp = expect_i64_safe(req("bar_timestamp")?, &format!("{ctx}.bar_timestamp"))?;
    let mark_price = expect_f64(req("mark_price")?, &format!("{ctx}.mark_price"))?;
    let act_raw = expect_label(req("action")?, &format!("{ctx}.action"))?;
    let action = PracticeDecisionAction::from_id(&act_raw).ok_or_else(|| {
        WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.action"),
            reason: format!("unknown PracticeDecisionAction '{act_raw}'"),
        }
    })?;
    let set_raw = expect_label(req("setup")?, &format!("{ctx}.setup"))?;
    let setup = parse_setup_tag(&set_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.setup"),
        reason: format!("unknown SetupTag '{set_raw}'"),
    })?;
    let mk_raw = expect_label(req("mistake")?, &format!("{ctx}.mistake"))?;
    let mistake =
        parse_mistake_tag(&mk_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.mistake"),
            reason: format!("unknown MistakeTag '{mk_raw}'"),
        })?;
    let conviction =
        expect_usize(req("conviction")?, &format!("{ctx}.conviction"))?.clamp(1, 5) as u8;
    let mask = expect_usize(req("checklist_mask")?, &format!("{ctx}.checklist_mask"))? as u8;
    let note = expect_string(req("note")?, &format!("{ctx}.note"))?;

    let mut out = PracticeDecisionNote {
        id,
        symbol,
        timeframe,
        base_bar_index,
        bar_timestamp,
        mark_price,
        action,
        setup,
        mistake,
        conviction,
        checklist: checklist_from_mask(mask),
        note,
    };
    out.sanitize();
    Ok(out)
}

fn encode_json_scan_store(store: &ScanSnapshotStore) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("{\n");
    let _ = writeln!(out, "  \"schema_version\": {},", store.schema_version);
    let _ = writeln!(out, "  \"next_slot_id\": {},", store.next_slot_id);
    let _ = writeln!(out, "  \"next_sequence_no\": {},", store.next_sequence_no);
    if store.slots.is_empty() {
        out.push_str("  \"slots\": []\n}");
        return out;
    }
    out.push_str("  \"slots\": [\n");
    for (i, s) in store.slots.iter().enumerate() {
        encode_json_scan_slot(&mut out, s);
        if i + 1 == store.slots.len() {
            out.push('\n');
        } else {
            out.push_str(",\n");
        }
    }
    out.push_str("  ]\n}");
    out
}

fn encode_json_scan_slot(out: &mut String, s: &SavedScanSnapshot) {
    out.push_str("    {\n");
    let _ = writeln!(out, "      \"slot_id\": {},", s.slot_id);
    out.push_str("      \"label\": ");
    escape_str_into(out, &s.label);
    let _ = writeln!(out, ",\n      \"pinned\": {},", s.pinned);
    let _ = writeln!(out, "      \"sequence_no\": {},", s.sequence_no);
    out.push_str("      \"captured_cutoff_timestamp\": ");
    encode_json_opt_i64(out, s.captured_cutoff_timestamp);
    let _ = writeln!(out, ",\n      \"timeframe\": \"{:?}\",", s.timeframe);
    let _ = writeln!(
        out,
        "      \"signal_lookback_bars\": {},",
        s.signal_lookback_bars
    );
    let _ = writeln!(
        out,
        "      \"blind_practice_lookback_bars\": {},",
        s.blind_practice_lookback_bars
    );
    out.push_str("      \"filter\": { \"search_query\": ");
    escape_str_into(out, &s.filter.search_query);
    out.push_str(", \"signal_filter\": ");
    match s.filter.signal_filter {
        Some(sig) => escape_str_into(out, signal_kind_id(sig)),
        None => out.push_str("null"),
    }
    out.push_str(", \"regime_filter\": ");
    match s.filter.regime_filter {
        Some(reg) => escape_str_into(out, regime_id(reg)),
        None => out.push_str("null"),
    }
    out.push_str(", \"direction_filter\": ");
    match s.filter.direction_filter {
        Some(dir) => escape_str_into(out, direction_id(dir)),
        None => out.push_str("null"),
    }
    let _ = writeln!(
        out,
        ", \"only_active_signals\": {} }},",
        s.filter.only_active_signals
    );
    let _ = writeln!(out, "      \"sort_col\": \"{}\",", sort_col_id(s.sort_col));
    let _ = writeln!(out, "      \"sort_asc\": {},", s.sort_asc);
    out.push_str("      \"hits\": [");
    for (hi, h) in s.hits.iter().enumerate() {
        if hi > 0 {
            out.push_str(", ");
        }
        encode_json_hit(out, h);
    }
    let _ = writeln!(
        out,
        "],\n      \"snapshot_digest_hex\": \"{:016x}\"",
        s.snapshot_digest
    );
    out.push_str("    }");
}

fn encode_ron_scan_store(store: &ScanSnapshotStore) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("(\n");
    let _ = writeln!(out, "    schema_version: {},", store.schema_version);
    let _ = writeln!(out, "    next_slot_id: {},", store.next_slot_id);
    let _ = writeln!(out, "    next_sequence_no: {},", store.next_sequence_no);
    if store.slots.is_empty() {
        out.push_str("    slots: [],\n)");
        return out;
    }
    out.push_str("    slots: [\n");
    for s in &store.slots {
        encode_ron_scan_slot(&mut out, s, 8, true);
        out.push_str(",\n");
    }
    out.push_str("    ],\n)");
    out
}

fn encode_ron_scan_slot(
    out: &mut String,
    s: &SavedScanSnapshot,
    indent: usize,
    include_digest: bool,
) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 4);
    let _ = writeln!(out, "{pad}(");
    let _ = writeln!(out, "{inner}slot_id: {},", s.slot_id);
    let _ = write!(out, "{inner}label: ");
    escape_str_into(out, &s.label);
    let _ = writeln!(out, ",");
    let _ = writeln!(out, "{inner}pinned: {},", s.pinned);
    let _ = writeln!(out, "{inner}sequence_no: {},", s.sequence_no);
    let _ = write!(out, "{inner}captured_cutoff_timestamp: ");
    encode_ron_opt_i64(out, s.captured_cutoff_timestamp);
    out.push_str(",\n");
    let _ = writeln!(out, "{inner}timeframe: {:?},", s.timeframe);
    let _ = writeln!(
        out,
        "{inner}signal_lookback_bars: {},",
        s.signal_lookback_bars
    );
    let _ = writeln!(
        out,
        "{inner}blind_practice_lookback_bars: {},",
        s.blind_practice_lookback_bars
    );
    let _ = write!(out, "{inner}filter: (search_query: ");
    escape_str_into(out, &s.filter.search_query);
    out.push_str(", signal_filter: ");
    match s.filter.signal_filter {
        Some(sig) => {
            let _ = write!(out, "Some({})", signal_kind_id(sig));
        }
        None => out.push_str("None"),
    }
    out.push_str(", regime_filter: ");
    match s.filter.regime_filter {
        Some(reg) => {
            let _ = write!(out, "Some({})", regime_id(reg));
        }
        None => out.push_str("None"),
    }
    out.push_str(", direction_filter: ");
    match s.filter.direction_filter {
        Some(dir) => {
            let _ = write!(out, "Some({})", direction_id(dir));
        }
        None => out.push_str("None"),
    }
    let _ = writeln!(
        out,
        ", only_active_signals: {}),",
        s.filter.only_active_signals
    );
    let _ = writeln!(out, "{inner}sort_col: {},", sort_col_id(s.sort_col));
    let _ = writeln!(out, "{inner}sort_asc: {},", s.sort_asc);
    let _ = write!(out, "{inner}hits: [");
    for (hi, h) in s.hits.iter().enumerate() {
        if hi > 0 {
            out.push_str(", ");
        }
        encode_ron_hit(out, h);
    }
    out.push_str("],\n");
    if include_digest {
        let _ = writeln!(
            out,
            "{inner}snapshot_digest_hex: \"{:016x}\",",
            s.snapshot_digest
        );
    }
    let _ = write!(out, "{pad})");
}

fn decode_scan_slot(
    val: &DocValue,
    ctx: &str,
) -> Result<SavedScanSnapshot, WorkspaceTemplateError> {
    let f = expect_struct(val, ctx)?;
    validate_struct_fields(
        f,
        ctx,
        &[
            "slot_id",
            "label",
            "pinned",
            "sequence_no",
            "captured_cutoff_timestamp",
            "timeframe",
            "signal_lookback_bars",
            "blind_practice_lookback_bars",
            "filter",
            "sort_col",
            "sort_asc",
            "hits",
            "snapshot_digest_hex",
        ],
    )?;
    let req = |k: &str| {
        find_field(f, k).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.{k}"),
            reason: format!("missing '{k}'"),
        })
    };
    let slot_id = expect_u64_safe(req("slot_id")?, &format!("{ctx}.slot_id"))?;
    let label = expect_string(req("label")?, &format!("{ctx}.label"))?;
    let pinned = expect_bool(req("pinned")?, &format!("{ctx}.pinned"))?;
    let sequence_no = expect_u64_safe(req("sequence_no")?, &format!("{ctx}.sequence_no"))?;
    let captured_cutoff_timestamp = match find_field(f, "captured_cutoff_timestamp") {
        Some(v) => expect_opt_i64(v, &format!("{ctx}.captured_cutoff_timestamp"))?,
        None => None,
    };
    let tf_raw = expect_label(req("timeframe")?, &format!("{ctx}.timeframe"))?;
    let timeframe =
        parse_timeframe(&tf_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.timeframe"),
            reason: format!("unknown Timeframe '{tf_raw}'"),
        })?;
    let signal_lookback_bars = expect_usize(
        req("signal_lookback_bars")?,
        &format!("{ctx}.signal_lookback_bars"),
    )?;
    let blind_practice_lookback_bars = expect_usize(
        req("blind_practice_lookback_bars")?,
        &format!("{ctx}.blind_practice_lookback_bars"),
    )?;

    let flt_f = expect_struct(req("filter")?, &format!("{ctx}.filter"))?;
    validate_struct_fields(
        flt_f,
        &format!("{ctx}.filter"),
        &[
            "search_query",
            "signal_filter",
            "regime_filter",
            "direction_filter",
            "only_active_signals",
        ],
    )?;
    let mut filter = ScannerFilter::default();
    if let Some(v) = find_field(flt_f, "search_query") {
        filter.search_query = expect_string(v, &format!("{ctx}.filter.search_query"))?;
    }
    if let Some(v) = find_field(flt_f, "signal_filter")
        && let Some(raw) = decode_opt_label(v, &format!("{ctx}.filter.signal_filter"))?
    {
        filter.signal_filter =
            Some(
                parse_signal_kind(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.filter.signal_filter"),
                    reason: format!("unknown ScanSignalKind '{raw}'"),
                })?,
            );
    }
    if let Some(v) = find_field(flt_f, "regime_filter")
        && let Some(raw) = decode_opt_label(v, &format!("{ctx}.filter.regime_filter"))?
    {
        filter.regime_filter =
            Some(
                parse_regime(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.filter.regime_filter"),
                    reason: format!("unknown MarketRegime '{raw}'"),
                })?,
            );
    }
    if let Some(v) = find_field(flt_f, "direction_filter")
        && let Some(raw) = decode_opt_label(v, &format!("{ctx}.filter.direction_filter"))?
    {
        filter.direction_filter =
            Some(
                parse_direction(&raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
                    field: format!("{ctx}.filter.direction_filter"),
                    reason: format!("unknown SignalDirection '{raw}'"),
                })?,
            );
    }
    if let Some(v) = find_field(flt_f, "only_active_signals") {
        filter.only_active_signals = expect_bool(v, &format!("{ctx}.filter.only_active_signals"))?;
    }

    let sc_raw = expect_label(req("sort_col")?, &format!("{ctx}.sort_col"))?;
    let sort_col = parse_sort_col(&sc_raw).ok_or_else(|| WorkspaceTemplateError::InvalidField {
        field: format!("{ctx}.sort_col"),
        reason: format!("unknown WatchlistSortColumn '{sc_raw}'"),
    })?;
    let sort_asc = expect_bool(req("sort_asc")?, &format!("{ctx}.sort_asc"))?;

    let hit_items = match req("hits")? {
        DocValue::Array(arr) => arr,
        _ => {
            return Err(WorkspaceTemplateError::InvalidField {
                field: format!("{ctx}.hits"),
                reason: "expected array".to_owned(),
            });
        }
    };
    if hit_items.len() > MAX_SNAPSHOT_HITS {
        return Err(WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.hits"),
            reason: format!(
                "too many hits ({}, max {MAX_SNAPSHOT_HITS})",
                hit_items.len()
            ),
        });
    }
    let mut hits = Vec::with_capacity(hit_items.len());
    for (hi, item) in hit_items.iter().enumerate() {
        hits.push(decode_hit(item, &format!("{ctx}.hits[{hi}]"))?);
    }

    let expected_digest = if let Some(v) = find_field(f, "snapshot_digest_hex") {
        Some(parse_hex_u64(v, &format!("{ctx}.snapshot_digest_hex"))?)
    } else {
        None
    };

    let mut snap = SavedScanSnapshot {
        slot_id,
        label,
        pinned,
        sequence_no,
        captured_cutoff_timestamp,
        timeframe,
        signal_lookback_bars,
        blind_practice_lookback_bars,
        filter,
        sort_col,
        sort_asc,
        hits,
        snapshot_digest: 0,
    };
    snap.sanitize();
    if let Some(exp) = expected_digest
        && exp != snap.snapshot_digest
    {
        return Err(WorkspaceTemplateError::InvalidField {
            field: format!("{ctx}.snapshot_digest_hex"),
            reason: format!(
                "Corrupt scan snapshot digest (expected {exp:016x}, actual {:016x})",
                snap.snapshot_digest
            ),
        });
    }
    Ok(snap)
}

fn decode_opt_label(val: &DocValue, field: &str) -> Result<Option<String>, WorkspaceTemplateError> {
    match val {
        DocValue::None => Ok(None),
        DocValue::Some(inner) => expect_label(inner, field).map(Some),
        DocValue::String(_) | DocValue::Ident(_) => expect_label(val, field).map(Some),
        _ => {
            let _ = expect_opt_string(val, field)?;
            Ok(None)
        }
    }
}

#[allow(dead_code)]
fn _assert_closed_trade_used(_t: &ClosedTrade) {}
