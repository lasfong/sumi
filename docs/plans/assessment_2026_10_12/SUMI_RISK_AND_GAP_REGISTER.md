# SUMI Risk & Gap Register — 2026-10-12

Read-only; severity = user/data harm × likelihood, confidence = evidence strength this pass.
**Do not mark B02 CLOSED or B03 READY** (rule). GAP_REGISTER (2026-10-09) remains the canonical gap list; this file triages it against today's worktree.

## A. Database integrity & production-data protection — SEV P0 CRITICAL
- **A1 CWD-relative DB path (GAP-DB-01).** Modified `config.py/db.py/alembic/env.py` INFERRED fix; untracked `test_cwd_database_isolation.py` asserts absolute anchoring (VERIFIED header). But: uncommitted, untested, unaccepted; root 0B ghost REPORTED still in lineage. Severity: high (wrong-DB writes). Likelihood: medium (any CWD-variant launch). Confidence: medium. Modules: `config, db, alembic/env, scripts`.
- **A2 Bare transactions (GAP-DB-02).** Write-path `commit/flush` without rollback REPORTED across import/sync/weekly/rollback; `get_db` close-only REPORTED. Working tree rewrites import (377) + sync (465) + weekly (52) — INFERRED remediation, content UNKNOWN. Severity: critical (open tx, partial writes). Confidence: medium. → B01 must prove atomicity via injected-fault test.
- **A3 FK unenforced (GAP-DB-03).** `db.py:1-8` no-pragma REPORTED; `test_b01_transaction_fk.py` asserts listener + reject/cascade (VERIFIED header, UNKNOWN result). Severity: high (orphans, fake cascades). → B01.
- **A4 Preview-commits (GAP-DB-14) + alembic gaps (GAP-DB-05/10).** REPORTED. Severity: medium-high. → B03 (stays NOT READY until B02 accepted + B01 green).
- **A5 Working-tree hazard (NEW).** 17 modified + safety rework uncommitted alongside unknown sync/FE changes; single-writer rule + clean-tree DoD (AGENTS.md) currently VIOLATED by state, not by act. Severity: process-high. Mitigation: commit/isolate B00–B02 slice first; nothing else rides the same tree.

## B. Synthetic/canonical isolation — SEV P0 CRITICAL (improved, not closed)
- **B1 Stub→prod writes (GAP-DB-04).** Guard VERIFIED in tree (`sync_workflow:344-346` deny, `:431-433` quarantine source/dataset, `:378-379` warning; `is_synthetic` metadata VERIFIED both stubs). Tests `test_b02_*` (3 files) + `test_sync_workflow` modifications VERIFIED present, results UNKNOWN. Severity: critical historically (fake candles + derived weekly contaminate prod). Likelihood now: low *if* guard active — but guard is uncommitted so likelihood stays medium until accepted. Confidence: medium-high on code, low on runtime. **Verdict: REPORTED→INFERRED-fixed, acceptance pending. B02 NOT CLOSED.**
- **B2 Secrets + feed bypass (GAP-DB-12/13).** `os.getenv` scatter + `DataFeedService` bypass REPORTED. Severity: medium. → B02/B40.

## C. Replay zero-lookahead — SEV P0 CRITICAL
- **C1 Raw ledger over cursor (GAP-REPLAY-01).** VERIFIED `api/decisions.py:42,49,66,70` unfiltered `.all()`. Severity: critical (future decisions/orders/trades visible). Likelihood: high in any replay with activity. Confidence: high. → B10.
- **C2 Projected indicators served (GAP-REPLAY-02).** `indicator_engine.py:349` reindex VERIFIED; append-all + zero FE filter REPORTED. Severity: critical (future-biased overlays, esp. Ichimoku +26). Confidence: medium-high. → B11.
- **C3 Forward meta (GAP-EXEC-08).** `total_bars/earliest_release_date` REPORTED. Severity: medium. → B10.
- **C4 FE optimistic index + wrong-TF cache (GAP-FE-01/02).** REPORTED. Severity: high (cross-TF/cross-session future flash = lookahead UX). → B30.

## D. Backtest/Strategy financial semantics — SEV P0 CRITICAL
- **D1 Dual timing (GAP-EXEC-01).** Legacy same-close vs kernel next-open (+intraday-same) REPORTED with file:line. Same strategy → different PnL with no domain reason. Severity: critical (money math). → B20 + golden G-01..G-05, G-11, G-12.
- **D2 Slippage/T+2/PnL/metrics multi-truth (GAP-EXEC-02..07).** REPORTED. Severity: critical-each; collectively the release blocker for any "backtest" claim. → B21 (MarketProfile + SettlementEngine + holiday calendar GAP-DB-11) + B22 (net/winner/PF/gate/Sharpe/MaxDD) + golden G-06..G-10.
- **D3 Sizing basis (GAP-EXEC-07).** REPORTED close-vs-fill mismatch. Severity: high (position sizes wrong). → B20.

## E. Transaction/rollback correctness — SEV P0 (see A2; acceptance)
Inject-fault atomicity + no-PendingRollback + FK reject/cascade + temp-DB hash-unchanged required before any financial batch. Without B01 green, B20–B22 results are untrustworthy (they write/read through the same paths).

## F. WebSocket reliability — SEV P1 (P0-adjacent for autoplay users)
No reconnect/backoff/heartbeat/validate/frames; speed skew; fire-and-forget; file pair VERIFIED, behavior REPORTED; chaos matrix doc VERIFIED, results UNKNOWN. Severity: medium-high (dead playback, cursor drift on restart). → B31 after B30. Needs BE state machine + FE remote per ADR-WS-001.

## G. Drawing persistence/compat — SEV P1
7-kind custom stack REPORTED working; 3 dead legacy files REPORTED; versioned-schema doc VERIFIED (`sumi-drawing-document-v1.schema.json`); runtime conformance + reload-through-pan/zoom/advance UNKNOWN; undo invisible (GAP-U-04). Severity: medium (data-loss perception + compat). → B32 + B51.

## H. FE/BE integration — SEV P1 (UX-blocking subset P0)
15 contract mismatches REPORTED (BANKRUPT, CUSTOM_PRICE, BB envelope VERIFIED as misaligned shapes, required/optional flips, TradePlan 7 missing, checklist loss, signal max2/forbid/params, Candle.source, dual catalog, 422 `[object Object]`, `str(e)` leakage, single ErrorBoundary, retry-3). God 1366-line pair REPORTED; silent `return []` mapping REPORTED; dual signal clients converging REPORTED. UX blockers: T+2 gray-out (GAP-U-01), SL/TP vanish + SHORT sign (U-02), builder→battle loss (U-03, critical), undo/cancel (U-04), `/backtest` orphan (U-05). Severity: high for U-01..U-03 (user funds-flow confusion / data loss); medium rest. → B40/B41 (contracts) + B30/B32 (FE) + B50/B51 (UX).

## I. Testing & release safety — SEV P1 PROCESS-BLOCKER
- Inventory VERIFIED (~50 backend test files + `backend/tests` + FE `__tests__` + 6 testing docs + `verify-v2.sh` temp-DB gate + UAT scripts). Results UNKNOWN (execution forbidden + subagents stalled).
- Golden 12 cases doc VERIFIED; parity/chaos/snapshot suites REPORTED-not-green.
- G-01..G-05 release bar: G-02 tooling VERIFIED (`verify-v2.sh` exports temp `DATABASE_URL`, cleanup trap); G-01/G-03/G-04 unmet (open P0/P1 + no evidence run). Per AGENTS.md, **no feature may be called complete and "TradingView-like" language stays prohibited.**
- DB-SAFETY-02R1 REPORTED-implemented, tests unexecuted/unaccepted — consistent with B01/B02 unaccepted status this pass.

## Triage: top 5 risks (ordered)
1. Financial incorrectness (D) — wrong money math ships as product truth.
2. Lookahead edges (C) — future data reaches user despite core slice.
3. DB/synthetic safety acceptance (A/B) — guards exist but unproven.
4. FE/WS determinism (C4/F) — divergent cursor across REST/WS/cache.
5. Contract/UX blockers (H) — data loss + trade confusion without hover/docs.

## What this pass did NOT verify (UNKNOWN, needs owner-visible evidence later)
Runtime pytest/UAT, live DB contents, per-route business-logic audit, FE test contents, holiday calendar correctness, perf claims, exact line numbers cited in prior reports (they may have shifted under the uncommitted rewrite).
