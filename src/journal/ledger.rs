use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::backtest::{BacktestTrade, PNL_BREAKEVEN_EPS};
use crate::data::format_utc_datetime_hm;
use crate::trading::{ClosedTrade, PositionSide};

use super::market_rules::ExchangeVenue;
use super::planner::TradePlanComputed;

/// Maximum persisted journal entries retained in workstation storage.
pub const MAX_JOURNAL_ENTRIES: usize = 2_048;

/// Maximum UTF-8 byte length stored per journal note.
pub const MAX_JOURNAL_NOTE_BYTES: usize = 512;

/// Structured trade setup taxonomy (`REA-R10-04` / `JournalPage.tsx` upgrade).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SetupTag {
    #[default]
    Untagged,
    TrendPullback,
    SqueezeBreakout,
    MeanReversion,
    SupportBounce,
    MomentumContinuation,
    LiquiditySweep,
}

impl SetupTag {
    pub const ALL: [Self; 7] = [
        Self::Untagged,
        Self::TrendPullback,
        Self::SqueezeBreakout,
        Self::MeanReversion,
        Self::SupportBounce,
        Self::MomentumContinuation,
        Self::LiquiditySweep,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Untagged => "Untagged",
            Self::TrendPullback => "Pullback",
            Self::SqueezeBreakout => "Squeeze BO",
            Self::MeanReversion => "Mean Rev",
            Self::SupportBounce => "S/R Bounce",
            Self::MomentumContinuation => "Momentum",
            Self::LiquiditySweep => "Liq Sweep",
        }
    }
}

/// Behavioral execution / discipline mistake taxonomy (`REA-R10-04`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MistakeTag {
    #[default]
    None,
    FomoChase,
    MovedStopLoss,
    EarlyExit,
    OversizedRisk,
    CounterTrendImpulse,
    IgnoredDailyBand,
    SkippedChecklist,
}

impl MistakeTag {
    pub const ALL: [Self; 8] = [
        Self::None,
        Self::FomoChase,
        Self::MovedStopLoss,
        Self::EarlyExit,
        Self::OversizedRisk,
        Self::CounterTrendImpulse,
        Self::IgnoredDailyBand,
        Self::SkippedChecklist,
    ];

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::None => "Clean / Plan",
            Self::FomoChase => "FOMO Chase",
            Self::MovedStopLoss => "Moved Stop",
            Self::EarlyExit => "Early Exit",
            Self::OversizedRisk => "Oversized",
            Self::CounterTrendImpulse => "Counter-Trend",
            Self::IgnoredDailyBand => "Band / T+2 Violation",
            Self::SkippedChecklist => "No Checklist",
        }
    }
}

/// Typed 6-point pre-trade execution checklist (`REA-R10-07` replacement for opaque JSON strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistState {
    pub trend_aligned: bool,
    pub level_confirmed: bool,
    pub catalyst_or_volume_ok: bool,
    pub stop_and_lot_sized: bool,
    pub band_and_t2_checked: bool,
    pub calm_state: bool,
}

impl Default for ChecklistState {
    fn default() -> Self {
        Self::all_checked()
    }
}

impl ChecklistState {
    #[inline]
    #[must_use]
    pub const fn all_checked() -> Self {
        Self {
            trend_aligned: true,
            level_confirmed: true,
            catalyst_or_volume_ok: true,
            stop_and_lot_sized: true,
            band_and_t2_checked: true,
            calm_state: true,
        }
    }

    /// Baseline checklist assigned to unreviewed automated imports (verifies system-enforced
    /// trend/band/risk rules while leaving discretionary level, volume, and psychology items unchecked
    /// until trader debrief).
    #[inline]
    #[must_use]
    pub const fn automated_import_default(has_risk_stop: bool) -> Self {
        Self {
            trend_aligned: true,
            level_confirmed: false,
            catalyst_or_volume_ok: false,
            stop_and_lot_sized: has_risk_stop,
            band_and_t2_checked: true,
            calm_state: false,
        }
    }

    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            trend_aligned: false,
            level_confirmed: false,
            catalyst_or_volume_ok: false,
            stop_and_lot_sized: false,
            band_and_t2_checked: false,
            calm_state: false,
        }
    }

    #[inline]
    #[must_use]
    pub const fn checked_count(self) -> u8 {
        (self.trend_aligned as u8)
            + (self.level_confirmed as u8)
            + (self.catalyst_or_volume_ok as u8)
            + (self.stop_and_lot_sized as u8)
            + (self.band_and_t2_checked as u8)
            + (self.calm_state as u8)
    }

    #[inline]
    #[must_use]
    pub fn pass_ratio(self) -> f64 {
        f64::from(self.checked_count()) / 6.0
    }
}

/// Provenance key enabling idempotent ingestion from Paper Trading and Strategy Backtest Lab.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JournalEntrySource {
    PaperTrade {
        symbol: String,
        trade_id: u64,
        #[serde(default)]
        entry_timestamp: i64,
    },
    BacktestTrade {
        symbol: String,
        preset_label: String,
        trade_id: u64,
        entry_timestamp: i64,
    },
    PlannedSetup {
        symbol: String,
    },
}

impl JournalEntrySource {
    #[inline]
    #[must_use]
    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::PaperTrade { .. } => "PAPER",
            Self::BacktestTrade { .. } => "BACKTEST",
            Self::PlannedSetup { .. } => "PLAN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalEntryStatus {
    Planned,
    Closed,
}

/// Structured, causally gated trade journal record (`Milestone R10`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: u64,
    pub source: JournalEntrySource,
    pub status: JournalEntryStatus,
    pub symbol: String,
    pub venue: ExchangeVenue,
    pub side: PositionSide,
    /// Causal frontier timestamp at which this entry becomes observable (`exit_timestamp` for closed
    /// trades; `entry_timestamp` / creation cutoff for planned setups — strictly gated by `DEF-001`).
    pub created_at_timestamp: i64,
    pub entry_timestamp: i64,
    pub exit_timestamp: Option<i64>,
    pub planned_entry: Option<f64>,
    pub planned_stop: Option<f64>,
    pub planned_target: Option<f64>,
    pub planned_quantity: Option<f64>,
    pub planned_net_r: Option<f64>,
    pub executed_entry: Option<f64>,
    pub executed_exit: Option<f64>,
    pub executed_quantity: Option<f64>,
    pub gross_pnl: f64,
    pub fees_and_taxes: f64,
    pub net_pnl: f64,
    pub realized_r: Option<f64>,
    pub setup: SetupTag,
    pub mistake: MistakeTag,
    pub conviction: u8,
    pub checklist: ChecklistState,
    pub notes: String,
}

impl JournalEntry {
    /// Returns `true` when this journal entry is causally observable at `replay_close_cutoff` (`DEF-001`).
    #[inline]
    #[must_use]
    pub fn is_causally_visible(&self, replay_close_cutoff: Option<i64>) -> bool {
        match replay_close_cutoff {
            None => true,
            Some(cutoff) => {
                let effective_ts = self
                    .exit_timestamp
                    .unwrap_or(self.created_at_timestamp)
                    .max(self.created_at_timestamp);
                effective_ts <= cutoff
            }
        }
    }

    /// Adverse entry slippage relative to `planned_entry` (`> 0` = worse than planned, `< 0` = better fill).
    #[must_use]
    pub fn entry_drift_pct(&self) -> Option<f64> {
        let planned = self.planned_entry.filter(|v| v.is_finite() && *v > 0.0)?;
        let exec = self.executed_entry.filter(|v| v.is_finite() && *v > 0.0)?;
        Some(self.side.sign() * ((exec - planned) / planned) * 100.0)
    }
}

/// Per-setup attribution row inside [`JournalAnalytics`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalSetupBreakdown {
    pub setup: SetupTag,
    pub trade_count: usize,
    pub win_rate_pct: f64,
    pub net_pnl: f64,
    pub avg_r: Option<f64>,
}

/// Behavioral & statistical performance analytics computed over causally visible journal entries.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JournalAnalytics {
    pub visible_entry_count: usize,
    pub planned_count: usize,
    pub closed_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub win_rate_pct: f64,
    pub total_net_pnl: f64,
    pub profit_factor: Option<f64>,
    pub expectancy_r: Option<f64>,
    /// Van Tharp System Quality Number (`sqrt(min(N, 100)) * mean(R) / std_sample(R)` for `N >= 5`).
    pub sqn: Option<f64>,
    pub discipline_score_pct: f64,
    pub mistake_trade_count: usize,
    pub cost_of_mistakes: f64,
    pub avg_entry_drift_pct: Option<f64>,
    pub current_streak: i32,
    pub setup_attribution: Vec<JournalSetupBreakdown>,
}

const fn default_next_journal_id() -> u64 {
    1
}

/// Persistent, deterministic structured trade journal store (`Milestone R10`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeJournalStore {
    #[serde(default)]
    pub entries: Vec<JournalEntry>,
    #[serde(default = "default_next_journal_id")]
    next_id: u64,
    #[serde(default)]
    pub selected_entry_id: Option<u64>,
}

impl Default for TradeJournalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeJournalStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
            selected_entry_id: None,
        }
    }

    pub fn sanitize_deserialized(&mut self) {
        if self.entries.len() > MAX_JOURNAL_ENTRIES {
            let drop_n = self.entries.len() - MAX_JOURNAL_ENTRIES;
            self.entries.drain(0..drop_n);
        }
        let mut seen_ids = HashSet::with_capacity(self.entries.len());
        let mut seen_sources = HashSet::with_capacity(self.entries.len());
        let mut max_id = 0_u64;
        self.entries.retain_mut(|e| {
            if e.id == 0
                || !seen_ids.insert(e.id)
                || !e.gross_pnl.is_finite()
                || !e.fees_and_taxes.is_finite()
                || !e.net_pnl.is_finite()
            {
                return false;
            }
            if !matches!(e.source, JournalEntrySource::PlannedSetup { .. })
                && !seen_sources.insert(e.source.clone())
            {
                return false;
            }
            e.planned_entry = e.planned_entry.filter(|v| v.is_finite() && *v > 0.0);
            e.planned_stop = e.planned_stop.filter(|v| v.is_finite() && *v > 0.0);
            e.planned_target = e.planned_target.filter(|v| v.is_finite() && *v > 0.0);
            e.planned_quantity = e.planned_quantity.filter(|v| v.is_finite() && *v > 0.0);
            e.planned_net_r = e.planned_net_r.filter(|v| v.is_finite());
            e.executed_entry = e.executed_entry.filter(|v| v.is_finite() && *v > 0.0);
            e.executed_exit = e.executed_exit.filter(|v| v.is_finite() && *v > 0.0);
            e.executed_quantity = e.executed_quantity.filter(|v| v.is_finite() && *v > 0.0);
            e.realized_r = e.realized_r.filter(|v| v.is_finite());
            if let Some(ex_ts) = e.exit_timestamp
                && ex_ts < e.entry_timestamp
            {
                e.exit_timestamp = Some(e.entry_timestamp);
            }
            e.conviction = e.conviction.clamp(1, 5);
            e.symbol = truncate_utf8(&e.symbol, 32);
            e.notes = truncate_utf8(&e.notes, MAX_JOURNAL_NOTE_BYTES);
            max_id = max_id.max(e.id);
            true
        });
        self.next_id = self.next_id.max(max_id.saturating_add(1)).max(1);
        if let Some(sel) = self.selected_entry_id
            && !self.entries.iter().any(|e| e.id == sel)
        {
            self.selected_entry_id = None;
        }
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id.saturating_add(1);
        id
    }

    fn enforce_capacity(&mut self) {
        if self.entries.len() > MAX_JOURNAL_ENTRIES {
            let drop_n = self.entries.len() - MAX_JOURNAL_ENTRIES;
            self.entries.drain(0..drop_n);
            if let Some(sel) = self.selected_entry_id
                && !self.entries.iter().any(|e| e.id == sel)
            {
                self.selected_entry_id = self.entries.last().map(|e| e.id);
            }
        }
    }

    /// Logs a pre-trade [`TradePlanComputed`] setup into the journal at `causal_timestamp`.
    #[allow(clippy::too_many_arguments)]
    pub fn record_trade_plan(
        &mut self,
        symbol: &str,
        plan: &TradePlanComputed,
        causal_timestamp: i64,
        setup: SetupTag,
        conviction: u8,
        checklist: ChecklistState,
        notes: &str,
    ) -> u64 {
        let id = self.alloc_id();
        let clean_sym = truncate_utf8(symbol.trim(), 32);
        let entry = JournalEntry {
            id,
            source: JournalEntrySource::PlannedSetup {
                symbol: clean_sym.clone(),
            },
            status: JournalEntryStatus::Planned,
            symbol: clean_sym,
            venue: plan.venue,
            side: plan.side,
            created_at_timestamp: causal_timestamp,
            entry_timestamp: causal_timestamp,
            exit_timestamp: None,
            planned_entry: Some(plan.snapped_entry),
            planned_stop: Some(plan.snapped_stop),
            planned_target: Some(plan.snapped_target),
            planned_quantity: Some(plan.planned_quantity),
            planned_net_r: Some(plan.net_r_multiple),
            executed_entry: None,
            executed_exit: None,
            executed_quantity: None,
            gross_pnl: 0.0,
            fees_and_taxes: 0.0,
            net_pnl: 0.0,
            realized_r: None,
            setup,
            mistake: MistakeTag::None,
            conviction: conviction.clamp(1, 5),
            checklist,
            notes: truncate_utf8(notes.trim(), MAX_JOURNAL_NOTE_BYTES),
        };
        self.entries.push(entry);
        self.selected_entry_id = Some(id);
        self.enforce_capacity();
        id
    }

    /// Idempotently synchronizes Paper Trading [`ClosedTrade`] records into the journal,
    /// pruning invalidated post-rewind paper trades for `symbol` while preserving user-curated
    /// `setup`, `mistake`, `conviction`, `checklist`, `planned_*`, and `notes` on surviving trades.
    pub fn sync_paper_closed_trades(
        &mut self,
        symbol: &str,
        venue: ExchangeVenue,
        trades: &[ClosedTrade],
    ) -> usize {
        let clean_sym = truncate_utf8(symbol.trim(), 32);
        let live_keys: HashSet<(u64, i64)> = trades
            .iter()
            .map(|tr| (tr.trade_id, tr.entry_timestamp))
            .collect();

        // Prune paper trades for `clean_sym` that were undone by Bar Replay rewind or session reset.
        self.entries.retain(|e| match &e.source {
            JournalEntrySource::PaperTrade {
                symbol: s,
                trade_id,
                entry_timestamp,
            } if s == &clean_sym => live_keys.contains(&(*trade_id, *entry_timestamp)),
            _ => true,
        });
        if let Some(sel) = self.selected_entry_id
            && !self.entries.iter().any(|e| e.id == sel)
        {
            self.selected_entry_id = self.entries.last().map(|e| e.id);
        }

        let mut added = 0_usize;
        for tr in trades {
            let src = JournalEntrySource::PaperTrade {
                symbol: clean_sym.clone(),
                trade_id: tr.trade_id,
                entry_timestamp: tr.entry_timestamp,
            };
            if let Some(existing) = self.entries.iter_mut().find(|e| e.source == src) {
                existing.venue = venue;
                existing.side = tr.side;
                existing.status = JournalEntryStatus::Closed;
                existing.created_at_timestamp = tr.exit_timestamp;
                existing.entry_timestamp = tr.entry_timestamp;
                existing.exit_timestamp = Some(tr.exit_timestamp);
                existing.executed_entry = Some(tr.entry_price);
                existing.executed_exit = Some(tr.exit_price);
                existing.executed_quantity = Some(tr.quantity);
                existing.gross_pnl = tr.gross_pnl;
                existing.fees_and_taxes = tr.total_fees;
                existing.net_pnl = tr.net_pnl;
                existing.realized_r = tr.r_multiple.filter(|v| v.is_finite());
            } else {
                let id = self.alloc_id();
                self.entries.push(JournalEntry {
                    id,
                    source: src,
                    status: JournalEntryStatus::Closed,
                    symbol: clean_sym.clone(),
                    venue,
                    side: tr.side,
                    created_at_timestamp: tr.exit_timestamp,
                    entry_timestamp: tr.entry_timestamp,
                    exit_timestamp: Some(tr.exit_timestamp),
                    planned_entry: Some(tr.entry_price),
                    planned_stop: None,
                    planned_target: None,
                    planned_quantity: Some(tr.quantity),
                    planned_net_r: None,
                    executed_entry: Some(tr.entry_price),
                    executed_exit: Some(tr.exit_price),
                    executed_quantity: Some(tr.quantity),
                    gross_pnl: tr.gross_pnl,
                    fees_and_taxes: tr.total_fees,
                    net_pnl: tr.net_pnl,
                    realized_r: tr.r_multiple.filter(|v| v.is_finite()),
                    setup: SetupTag::Untagged,
                    mistake: MistakeTag::None,
                    conviction: 3,
                    checklist: ChecklistState::automated_import_default(tr.r_multiple.is_some()),
                    notes: String::new(),
                });
                self.selected_entry_id = Some(id);
                added += 1;
            }
        }
        self.enforce_capacity();
        added
    }

    /// Idempotently imports up to `max_import` most recent [`BacktestTrade`] records into the journal.
    pub fn import_backtest_trades(
        &mut self,
        symbol: &str,
        venue: ExchangeVenue,
        preset_label: &str,
        setup_hint: SetupTag,
        trades: &[BacktestTrade],
        max_import: usize,
    ) -> usize {
        if max_import == 0 || trades.is_empty() {
            return 0;
        }
        let clean_sym = truncate_utf8(symbol.trim(), 32);
        let clean_preset = truncate_utf8(preset_label.trim(), 32);
        let start = trades.len().saturating_sub(max_import);
        let mut added = 0_usize;

        for tr in &trades[start..] {
            let src = JournalEntrySource::BacktestTrade {
                symbol: clean_sym.clone(),
                preset_label: clean_preset.clone(),
                trade_id: tr.trade_id,
                entry_timestamp: tr.entry_timestamp,
            };
            if self.entries.iter().any(|e| e.source == src) {
                continue;
            }
            let id = self.alloc_id();
            self.entries.push(JournalEntry {
                id,
                source: src,
                status: JournalEntryStatus::Closed,
                symbol: clean_sym.clone(),
                venue,
                side: tr.side,
                created_at_timestamp: tr.exit_timestamp,
                entry_timestamp: tr.entry_timestamp,
                exit_timestamp: Some(tr.exit_timestamp),
                planned_entry: Some(tr.entry_price),
                planned_stop: None,
                planned_target: None,
                planned_quantity: Some(tr.quantity),
                planned_net_r: None,
                executed_entry: Some(tr.entry_price),
                executed_exit: Some(tr.exit_price),
                executed_quantity: Some(tr.quantity),
                gross_pnl: tr.gross_pnl,
                fees_and_taxes: tr.entry_fee + tr.exit_fee + tr.exit_tax,
                net_pnl: tr.net_pnl,
                realized_r: tr.r_multiple.filter(|v| v.is_finite()),
                setup: setup_hint,
                mistake: MistakeTag::None,
                conviction: 3,
                checklist: ChecklistState::automated_import_default(tr.r_multiple.is_some()),
                notes: format!("{} · {}", clean_preset, tr.exit_reason.short_label()),
            });
            self.selected_entry_id = Some(id);
            added += 1;
        }
        self.enforce_capacity();
        added
    }

    pub fn delete_entry(&mut self, id: u64) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        if self.selected_entry_id == Some(id) {
            self.selected_entry_id = self.entries.last().map(|e| e.id);
        }
        self.entries.len() < len_before
    }

    pub fn entry_mut(&mut self, id: u64) -> Option<&mut JournalEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    /// Returns causally visible entries (`DEF-001`: future entries beyond `replay_close_cutoff` are masked).
    #[must_use]
    pub fn visible_entries(&self, replay_close_cutoff: Option<i64>) -> Vec<&JournalEntry> {
        self.entries
            .iter()
            .filter(|e| e.is_causally_visible(replay_close_cutoff))
            .collect()
    }

    /// Computes behavioral & statistical journal analytics strictly over causally visible entries (`DEF-001`).
    #[must_use]
    pub fn compute_analytics(&self, replay_close_cutoff: Option<i64>) -> JournalAnalytics {
        let visible = self.visible_entries(replay_close_cutoff);
        if visible.is_empty() {
            return JournalAnalytics {
                discipline_score_pct: 100.0,
                ..JournalAnalytics::default()
            };
        }

        let mut planned_count = 0_usize;
        let mut closed_count = 0_usize;
        let mut win_count = 0_usize;
        let mut loss_count = 0_usize;
        let mut total_net_pnl = 0.0_f64;
        let mut gross_win_sum = 0.0_f64;
        let mut gross_loss_abs_sum = 0.0_f64;
        let mut mistake_trade_count = 0_usize;
        let mut cost_of_mistakes = 0.0_f64;
        let mut closed_discipline_sum = 0.0_f64;
        let mut planned_discipline_sum = 0.0_f64;
        let mut r_values = Vec::new();
        let mut drift_sum = 0.0_f64;
        let mut drift_cnt = 0_usize;
        let mut current_streak = 0_i32;

        // Ordered chronologically by completion/creation for accurate streak calculation.
        let mut ordered: Vec<&JournalEntry> = visible;
        ordered.sort_by(|a, b| {
            let ta = a.exit_timestamp.unwrap_or(a.created_at_timestamp);
            let tb = b.exit_timestamp.unwrap_or(b.created_at_timestamp);
            ta.cmp(&tb).then_with(|| a.id.cmp(&b.id))
        });

        for e in &ordered {
            let item_disc = if e.mistake == MistakeTag::None {
                0.5 + 0.5 * e.checklist.pass_ratio()
            } else {
                0.25 * e.checklist.pass_ratio()
            };

            match e.status {
                JournalEntryStatus::Planned => {
                    planned_count += 1;
                    planned_discipline_sum += item_disc;
                }
                JournalEntryStatus::Closed => {
                    closed_count += 1;
                    closed_discipline_sum += item_disc;
                    total_net_pnl += e.net_pnl;
                    if e.net_pnl > PNL_BREAKEVEN_EPS {
                        win_count += 1;
                        gross_win_sum += e.net_pnl;
                        current_streak = if current_streak >= 0 {
                            current_streak + 1
                        } else {
                            1
                        };
                    } else if e.net_pnl < -PNL_BREAKEVEN_EPS {
                        loss_count += 1;
                        gross_loss_abs_sum += e.net_pnl.abs();
                        current_streak = if current_streak <= 0 {
                            current_streak - 1
                        } else {
                            -1
                        };
                    } else {
                        current_streak = 0;
                    }
                    if e.mistake != MistakeTag::None {
                        mistake_trade_count += 1;
                        if e.net_pnl < 0.0 {
                            cost_of_mistakes += e.net_pnl.abs();
                        }
                    }
                    if let Some(r) = e.realized_r.filter(|v| v.is_finite()) {
                        r_values.push(r);
                    }
                    if let Some(d) = e.entry_drift_pct().filter(|v| v.is_finite()) {
                        drift_sum += d;
                        drift_cnt += 1;
                    }
                }
            }
        }

        let win_rate_pct = if closed_count > 0 {
            (win_count as f64 / closed_count as f64) * 100.0
        } else {
            0.0
        };
        let profit_factor = if gross_loss_abs_sum > 1.0e-9 {
            Some(gross_win_sum / gross_loss_abs_sum)
        } else {
            None
        };
        let discipline_score_pct = if closed_count > 0 {
            (closed_discipline_sum / (closed_count as f64)) * 100.0
        } else if planned_count > 0 {
            (planned_discipline_sum / (planned_count as f64)) * 100.0
        } else {
            100.0
        };

        let (expectancy_r, sqn) = if r_values.is_empty() {
            (None, None)
        } else {
            let n = r_values.len() as f64;
            let mean_r = r_values.iter().sum::<f64>() / n;
            let sqn_val = if r_values.len() >= 5 {
                let var = r_values
                    .iter()
                    .map(|r| {
                        let d = *r - mean_r;
                        d * d
                    })
                    .sum::<f64>()
                    / (n - 1.0);
                let std_r = var.sqrt();
                if std_r > 1.0e-12 {
                    Some((n.min(100.0)).sqrt() * (mean_r / std_r))
                } else {
                    None
                }
            } else {
                None
            };
            (Some(mean_r), sqn_val)
        };

        let avg_entry_drift_pct = (drift_cnt > 0).then(|| drift_sum / (drift_cnt as f64));

        let mut setup_attribution = Vec::new();
        for tag in SetupTag::ALL {
            let mut cnt = 0_usize;
            let mut wins = 0_usize;
            let mut pnl = 0.0_f64;
            let mut r_sum = 0.0_f64;
            let mut r_cnt = 0_usize;
            for e in &ordered {
                if e.status == JournalEntryStatus::Closed && e.setup == tag {
                    cnt += 1;
                    pnl += e.net_pnl;
                    if e.net_pnl > PNL_BREAKEVEN_EPS {
                        wins += 1;
                    }
                    if let Some(r) = e.realized_r.filter(|v| v.is_finite()) {
                        r_sum += r;
                        r_cnt += 1;
                    }
                }
            }
            if cnt > 0 {
                setup_attribution.push(JournalSetupBreakdown {
                    setup: tag,
                    trade_count: cnt,
                    win_rate_pct: (wins as f64 / cnt as f64) * 100.0,
                    net_pnl: pnl,
                    avg_r: (r_cnt > 0).then(|| r_sum / (r_cnt as f64)),
                });
            }
        }
        setup_attribution.sort_by(|a, b| {
            b.net_pnl
                .total_cmp(&a.net_pnl)
                .then_with(|| b.trade_count.cmp(&a.trade_count))
        });

        JournalAnalytics {
            visible_entry_count: ordered.len(),
            planned_count,
            closed_count,
            win_count,
            loss_count,
            win_rate_pct,
            total_net_pnl,
            profit_factor,
            expectancy_r,
            sqn,
            discipline_score_pct,
            mistake_trade_count,
            cost_of_mistakes,
            avg_entry_drift_pct,
            current_streak,
            setup_attribution,
        }
    }

    /// Exports causally visible journal entries as deterministic single-line-per-record RFC-4180 CSV
    /// with spreadsheet formula-injection neutralization (`sanitize_csv_cell`).
    #[must_use]
    pub fn export_csv(&self, replay_close_cutoff: Option<i64>) -> String {
        let visible = self.visible_entries(replay_close_cutoff);
        let mut out = String::from(
            "id,source,status,symbol,venue,side,entry_utc,exit_utc,planned_entry,executed_entry,executed_exit,quantity,net_pnl,realized_r,setup,mistake,conviction,checklist_6,notes\n",
        );
        for e in visible {
            let entry_dt = format_utc_datetime_hm(e.entry_timestamp);
            let exit_dt = e
                .exit_timestamp
                .map_or_else(String::new, format_utc_datetime_hm);
            let qty = e.executed_quantity.or(e.planned_quantity);
            let status_str = match e.status {
                JournalEntryStatus::Planned => "PLANNED",
                JournalEntryStatus::Closed => "CLOSED",
            };
            let cols = [
                e.id.to_string(),
                sanitize_csv_cell(e.source.kind_label()),
                sanitize_csv_cell(status_str),
                sanitize_csv_cell(&e.symbol),
                sanitize_csv_cell(e.venue.short_label()),
                sanitize_csv_cell(e.side.short_label()),
                sanitize_csv_cell(&entry_dt),
                sanitize_csv_cell(&exit_dt),
                fmt_opt_f64(e.planned_entry, 4),
                fmt_opt_f64(e.executed_entry, 4),
                fmt_opt_f64(e.executed_exit, 4),
                fmt_opt_f64(qty, 4),
                format!("{:.2}", e.net_pnl),
                fmt_opt_f64(e.realized_r, 3),
                sanitize_csv_cell(e.setup.short_label()),
                sanitize_csv_cell(e.mistake.short_label()),
                e.conviction.to_string(),
                format!("{}/6", e.checklist.checked_count()),
                sanitize_csv_cell(&e.notes),
            ];
            out.push_str(&cols.join(","));
            out.push('\n');
        }
        out
    }

    /// Exports causally visible journal analytics and entries as a deterministic Markdown debrief report.
    #[must_use]
    pub fn export_markdown(&self, replay_close_cutoff: Option<i64>) -> String {
        let stats = self.compute_analytics(replay_close_cutoff);
        let visible = self.visible_entries(replay_close_cutoff);
        let mut md = String::new();
        md.push_str("# Sumi Structured Trade Journal & Discipline Report\n\n");
        md.push_str(&format!(
            "- **Visible Entries:** `{}` (`{}` closed, `{}` planned)\n",
            stats.visible_entry_count, stats.closed_count, stats.planned_count
        ));
        let pf_str = stats
            .profit_factor
            .map_or_else(|| "n/a".to_owned(), |pf| format!("{pf:.2}"));
        md.push_str(&format!(
            "- **Win Rate:** `{:.1}%` (`{}W / {}L`, `PF: {pf_str}`) | **Total Net PnL:** `{:+.2}`\n",
            stats.win_rate_pct, stats.win_count, stats.loss_count, stats.total_net_pnl
        ));
        md.push_str(&format!(
            "- **Discipline Score:** `{:.1}%` | **Mistakes:** `{}` (`Cost: {:.2}`)\n",
            stats.discipline_score_pct, stats.mistake_trade_count, stats.cost_of_mistakes
        ));
        if let Some(exp_r) = stats.expectancy_r {
            let sqn_str = stats
                .sqn
                .map_or_else(|| "n/a (<5 R samples)".to_owned(), |s| format!("{s:.2}"));
            md.push_str(&format!(
                "- **Expectancy:** `{exp_r:+.2}R` | **Van Tharp SQN:** `{sqn_str}`\n"
            ));
        }
        md.push_str(
            "\n| ID | Src | Symbol | Side | Entry UTC | Net PnL | R | Setup | Mistake | Notes |\n",
        );
        md.push_str("| :--- | :--- | :--- | :--- | :--- | ---: | ---: | :--- | :--- | :--- |\n");
        for e in visible {
            let r_str = e
                .realized_r
                .or(e.planned_net_r)
                .map_or_else(|| "-".to_owned(), |r| format!("{r:+.2}R"));
            let clean_note = e.notes.replace('|', "\\|").replace(['\n', '\r'], " ");
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:+.2} | {} | {} | {} | {} |\n",
                e.id,
                e.source.kind_label(),
                e.symbol.replace('|', "\\|"),
                e.side.short_label(),
                format_utc_datetime_hm(e.entry_timestamp),
                e.net_pnl,
                r_str,
                e.setup.short_label(),
                e.mistake.short_label(),
                clean_note
            ));
        }
        md
    }
}

fn fmt_opt_f64(val: Option<f64>, decimals: usize) -> String {
    match val.filter(|v| v.is_finite()) {
        Some(v) => format!("{v:.decimals$}"),
        None => String::new(),
    }
}

fn truncate_utf8(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    input[..end].to_owned()
}

/// Sanitizes an arbitrary text cell for RFC-4180 CSV export while neutralizing spreadsheet
/// formula injection (`=`, `+`, `-`, `@`, `|`, `\t`, `\r`, `\n` — including after leading
/// ASCII/Unicode whitespace or control characters) and normalizing embedded newlines to spaces.
#[must_use]
pub fn sanitize_csv_cell(raw: &str) -> String {
    let had_leading_ctrl = raw
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '\t' | '\r' | '\n' | '\x0B' | '\x0C'));
    let flat = if raw.contains(['\r', '\n']) {
        raw.replace("\r\n", " ").replace(['\r', '\n'], " ")
    } else {
        raw.to_owned()
    };
    let trimmed_start = flat.trim_start_matches(|c: char| c.is_whitespace() || c.is_control());
    let needs_formula_prefix = had_leading_ctrl
        || trimmed_start
            .bytes()
            .next()
            .is_some_and(|b| matches!(b, b'=' | b'+' | b'-' | b'@' | b'|' | b'\t'));

    let safe = if needs_formula_prefix {
        format!("'{flat}")
    } else {
        flat
    };

    let needs_quotes = safe
        .bytes()
        .any(|b| matches!(b, b',' | b'"' | b'\n' | b'\r'));
    if needs_quotes {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}
