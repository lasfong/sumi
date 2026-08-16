# PRO-08 — Trade Planning and Journal

Status: `PREPARED — USER AUTHORIZED`

## Outcome

Replay trading decisions become a measurable risk, execution, and learning workflow: versioned trade plans, deterministic position sizing (lot, tick, fees, taxes, available cash), Sumi-owned Long/Short Risk-Reward drawing integration, immutable checklist snapshots, rich journal review taxonomy (setup, regime, confidence, emotion, process mistake, rule violation), and faithful T+2 settlement modeling across replay stepping, reload, and navigation.

## Context and problem

PRO-07 is independently approved and closed. Previously, replay trading supported basic buy/sell/close orders and basic journal notes. Serious technical practice requires disciplined risk-first trade planning before execution (defining entry, stop loss, profit target, risk percentage, lot rounding, expected R-multiple), synchronizing chart risk-reward drawings with the trade order form, enforcing pre-trade checklists, tracking psychological and rule-violation factors in the journal, and accurately accounting for Vietnam market T+2 settlement availability without hindsight leakage.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs `PRO-TRADE-01` through `PRO-TRADE-10`; `docs/program/PRO_08_TRADE_PLANNING_AND_JOURNAL.md`; V3 G-01..05 regression.

## In scope

1. **Trade Planning & Sizing (`PRO-TRADE-01`, `PRO-TRADE-02`, `PRO-TRADE-09`):**
   - Plan schema recording entry price, stop price, target price, direction (long), account risk percentage, planned quantity, estimated fees/taxes, and expected R-multiple.
   - Deterministic position sizing engine adhering to Vietnam market rules: standard 100-share lot increments, minimum lot constraints, tick size step rules, fee & tax modeling, and available cash checks.
2. **Long/Short Risk-Reward Drawing Integration (`PRO-TRADE-03`):**
   - Versioned Sumi domain contract connecting chart Risk-Reward tool geometry (entry, stop, target) directly to the trade planning panel without provider-native dependency.
3. **Order & Trade Lifecycle Sync (`PRO-TRADE-04`, `PRO-TRADE-05`):**
   - Synchronize planned, pending, filled, rejected, cancelled, settled, and closed states across replay stepping, rewind, reload, and route switching.
   - T+2 settlement tracking: clear UI and API feedback for blocked shares, available shares, and release dates.
4. **Checklist & Decision Review (`PRO-TRADE-06`, `PRO-TRADE-07`, `PRO-TRADE-08`):**
   - Editable, versioned checklist templates saved as an immutable snapshot per decision.
   - Rich journal taxonomy: setup type, market regime, confidence level, emotion/mindset, process mistakes (e.g. FOMO, early exit, chase), and rule violations.
   - Review comparing planned vs executed entry, stop, target, size, and outcome without hindsight leakage during blind practice.
5. **Local Privacy & Export (`PRO-TRADE-10`):**
   - Local-first JSON/CSV journal export and backup preserving stable identifiers and zero external transmission.
6. **Automated Testing & Browser Evidence:**
   - Backend unit/integration tests for trade lifecycle, position sizing, T+2 rules, and fee calculations.
   - Frontend unit tests for planning calculator, checklist snapshots, and journal components.
   - Deterministic Product UAT assertions (`pro08.*`) and retained 1440×1000 and 1280×800 screenshots.

## Out of scope

- Strategy Research UX & Parameter Sweeps (PRO-09).
- Market data provider licensing/sync (PRO-10, PRO-11).
- Direct brokerage integration or live order routing.

## Invariants

- Accounting authority resides authoritatively in backend domain services (`trade_lifecycle_service.py`), avoiding duplicate or conflicting calculations.
- Blind practice mode never leaks future candles or trade outcomes before reveal.
- No test or UAT mutates `backend/sumi.db`; before/after SHA-256 is recorded.
- Checklist snapshots and journal entries remain immutable once recorded.
- All product features operate strictly local-first.

## Milestones

1. **Backend domain & APIs:** Extend `trade_lifecycle_service.py`, `journal_service.py`, and API endpoints for versioned plans, position sizing calculation, T+2 availability, checklist templates/snapshots, and journal taxonomy; add comprehensive tests in `test_trade_lifecycle.py`.
2. **Frontend Trade Planning & Position Sizing:** Implement `TradePlanningPanel`, risk calculator (lot 100 rounding, risk %, target R-multiple), and sync with `TradeControls.tsx`.
3. **Risk-Reward Chart Tool Sync:** Connect Risk-Reward drawing tool in `DrawingToolRegistry.ts` / `SumiPrimitiveDrawingProvider.ts` with the planning panel.
4. **Checklist & Rich Journal Review:** Extend `PracticeJournal.tsx`, `DecisionJournal.tsx`, and `JournalPage.tsx` with checklist snapshots, taxonomy tags, and planned vs executed comparison.
5. **Product UAT & Verification:** Extend `product-uat.mjs` with PRO-08 trade planning and journal review assertions; run fast technical gate and full Product UAT; capture 1440×1000 and 1280×800 screenshots; stop at Reviewer Gate.

## Acceptance mapping

| ID | Requirement |
| --- | --- |
| PRO-TRADE-01 | A plan records entry, stop, target, direction, account risk, planned quantity, fees, and expected R multiple. |
| PRO-TRADE-02 | Position sizing uses documented tick, lot, fee, tax, and available-cash rules with deterministic rounding. |
| PRO-TRADE-03 | Long/Short Risk-Reward drawing and trade plan share a versioned Sumi domain contract without provider-native persistence. |
| PRO-TRADE-04 | Planned, pending, filled, rejected, cancelled, settled, and closed states remain synchronized through replay and reload. |
| PRO-TRADE-05 | T+2 availability and rejection feedback state the blocked quantity, available quantity, and release date. |
| PRO-TRADE-06 | Checklist templates are editable, versioned, and captured as an immutable per-decision snapshot. |
| PRO-TRADE-07 | Journal taxonomy supports setup, regime, confidence, emotion, process mistake, rule violation, and review notes. |
| PRO-TRADE-08 | Review compares planned versus executed entry, stop, target, size, R, and outcome without hindsight leakage during practice. |
| PRO-TRADE-09 | Hand-calculated fixtures verify risk sizing, fees, taxes, T+2, partial availability, and R-multiple calculations. |
| PRO-TRADE-10 | Journal export and backup preserve local privacy and stable identifiers. |

## Verification commands

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/test_trade_lifecycle.py app/tests/ -v
Set-Location ..\frontend
npm.cmd test -- --run
Set-Location ..
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

## Progress log

- 2026-08-16: User authorized PRO-08. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
