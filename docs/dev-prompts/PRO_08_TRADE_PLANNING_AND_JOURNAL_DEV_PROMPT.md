# PRO-08 DEV Prompt — Trade Planning and Journal

You are the dedicated DEV session for **PRO-08 — Trade Planning and Journal**. Implement this batch from the current workspace checkout; do not rely on chat history. Stop at the Independent Reviewer Gate when implementation and verification are complete. Do not approve your own work, commit, push, or start PRO-09.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-08, PRO-TRADE-01..10)
6. `docs/program/PRO_08_TRADE_PLANNING_AND_JOURNAL.md`
7. `docs/exec-plans/PRO_08_TRADE_PLANNING_AND_JOURNAL.md`

## Outcome

Replay trading decisions become a measurable risk, execution, and learning workflow: versioned trade plans, deterministic position sizing (lot 100, tick size, fees, taxes, available cash), Sumi-owned Long/Short Risk-Reward drawing integration, immutable checklist snapshots, rich journal review taxonomy (setup, regime, confidence, emotion, process mistake, rule violation), and faithful T+2 settlement modeling across replay stepping, reload, and navigation.

## Implementation tasks

1. **Backend Trade Planning & Position Sizing Engine (`PRO-TRADE-01`, `PRO-TRADE-02`, `PRO-TRADE-09`):**
   - Extend `backend/app/services/trade_lifecycle_service.py` to support trade planning schemas:
     - Entry, stop loss, profit target, planned quantity, account risk percentage, estimated commission/tax, and expected R-multiple (\(R = \frac{\text{Target} - \text{Entry}}{\text{Entry} - \text{Stop}}\)).
     - Position sizing calculation based on account equity, risk per trade %, lot size (100 shares), tick size constraints, and available buying power.
   - Extend T+2 settlement tracking: explicit feedback for locked quantity vs tradable quantity and release dates (`PRO-TRADE-05`).
   - Add hand-calculated fixtures and unit tests in `backend/app/tests/test_trade_lifecycle.py`.

2. **Backend Journal Taxonomy & Checklist Snapshots (`PRO-TRADE-06`, `PRO-TRADE-07`, `PRO-TRADE-08`, `PRO-TRADE-10`):**
   - Extend `backend/app/services/journal_service.py` and models:
     - Editable checklist templates; create immutable per-decision checklist snapshot on trade execution.
     - Journal taxonomy tags: setup type, market regime, confidence (1-5), emotion/mindset, process mistakes (e.g. FOMO, early cut, oversized), rule violations.
     - Planned vs executed variance tracking (entry drift, exit variance, realized R vs planned R).
     - Local export endpoint/utility for JSON/CSV backup.

3. **Frontend Trade Planning & Position Sizing UI (`TradeControls.tsx`, `TradePlanningPanel`):**
   - Provide explicit Trade Planning UI on the replay sidebar:
     - Input/adjust Entry, Stop Loss, Target Price.
     - Auto-calculate position size from risk % (or cash amount) with 100-share lot rounding.
     - Real-time R-multiple and Risk:Reward ratio display.
   - Sync with Long/Short Risk-Reward drawing tool on chart (`DrawingToolRegistry.ts`, `SumiPrimitiveDrawingProvider.ts`).

4. **Frontend Checklist & Rich Journal Review (`PracticeJournal.tsx`, `JournalPage.tsx`):**
   - Checklist modal/dropdown before order placement capturing checklist state.
   - Rich journal entry modal with taxonomy selectors (setup, regime, emotion, mistake).
   - Journal page table / detail view displaying planned vs executed comparisons and R-multiples.

5. **Product UAT & Screenshot Evidence:**
   - Add deterministic UAT checks in `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json` for trade planning, sizing calculation, checklist capture, T+2 availability, and journal review.
   - Retain `pro08-trade-planning-1440x1000.png` and `pro08-trade-planning-1280x800.png`.

6. **Technical Gates & Hand-off:**
   - Run pytest and vitest suites.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_08_TRADE_PLANNING_AND_JOURNAL.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact commands, test counts, artifact hashes, and screenshot evidence. Do not commit, push, or start PRO-09.
