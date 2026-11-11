# SUMI Batch Reconciliation (B00–B51 + P-series) — 2026-10-12

**Rule honored:** existing IDs preserved; no new roadmap invented before mapping the old one; **B02 NOT CLOSED, B03 NOT READY.**
Two lineages exist — do not conflate them:
- **P-lineage (dev-program, STATE.json `complete`):** BOOTSTRAP + P1-SIG→P10-SEAL, 20 tasks REPORTED accepted with review docs. Covers signal research → kernel → market rules → batch/metrics → data/BB → universe → breadth → Ichimoku/divergence/health → unified UI → perf. This is the *research-production* track.
- **B-lineage (safety/correctness, DEV_BATCHES.md + prompts/):** B00–B51, 15 STOP-gated vertical slices closing GAP_REGISTER. This is the *correctness* track. The assessment's roadmap (file 5) sequences B-lineage; P-lineage is provenance + reusable engines, not a substitute for B acceptance.

## B-lineage status (docs + worktree, no runtime)

| Batch | Scope (DEV_BATCHES) | GAPs | Deps | Status this pass | Missing verification | Overlap |
|---|---|---|---|---|---|---|
| B00 DB path+guard | config, alembic env, startup assert | DB-01 | none | **IN WORKTREE, UNACCEPTED** — config/db/env modified; CWD test present | CWD-launch test + UAT hash-unchanged + reviewer check | feeds B01–B03, B10 |
| B01 Tx+FK | atomic(), get_db rollback, pragma | DB-02/03 | B00 | **IN WORKTREE, UNACCEPTED** — import/sync/weekly rewritten (INFERRED); b01 FK test present | inject-fault atomic + FK reject/cascade + no-PendingRollback | gates everything writing |
| B02 synthetic-deny | guard + quarantine + banner + Settings | DB-04 | B01 | **IN WORKTREE, UNACCEPTED** — deny+quarantine VERIFIED in code; 3 b02 tests + sync test mods present | default-deny 403 test + quarantine-only-on-flag + temp-DB hash + reviewer | — |
| B03 alembic+preview | full models, _has_column, preview-confirm | DB-05/10/14 | B01 | NOT STARTED (blocked: needs B01 green + B02 accepted) | migrate-from-empty 20 tables + preview-no-commit | — |
| B10 replay-ledger | predicate/deprecate + clamp meta | REPLAY-01, EXEC-08 | B01 | NOT STARTED | index-N regression + meta clamp | B11 (shared cursor policy) |
| B11 projected-drop | engine cut + MappingResult type | REPLAY-02 | B10-adjacent (policy) | NOT STARTED | Ichimoku+26 absent payload/render | B32 (FE mapping UI) |
| B20 exec parity | sweep→kernel, fill-est sizing, legacy banner | EXEC-01/07 | B01 | NOT STARTED | parity sweep==batch + golden 6 | B21 (fees/settle out of scope) |
| B21 profile+settle | MarketProfile, SettlementEngine, T+2 inline | EXEC-02/03, U-01 wording | B20 | NOT STARTED | weekend/holiday golden + slippage single-source | needs holiday calendar (DB-11) |
| B22 metrics-unify | net/winner/PF/gate/Sharpe/MaxDD | EXEC-04/05/06 | B21 | NOT STARTED | unify + G-08..G-10 | analytics + practice scoreboard |
| B30 FE keys/nav/switch | key+TF, token, invalidate, QueryClient | FE-01/02/06 | B10 (cursor echo) | NOT STARTED | step-x20 race + no-flash + single-clear grep | B31 (WS out of scope) |
| B31 WS reliability | state machine + backoff + frames | WS-01, FE-10 | B30 | NOT STARTED | chaos matrix | — |
| B32 ChartKernel+god-1+draw-clean | facade, split Header/Dock, MappingResult, delete 3 dead | FE-03/04/05/07 | B30 | NOT STARTED | createChart grep + drawing UAT | B11/B51 |
| B40 contracts | envelope+snapshot+dead routes+422 | API-01/03..11 part | B20 (BB stable post-exec) | NOT STARTED | openapi snapshot CI + 422 UAT | B41 (BB) |
| B41 BB envelope | one envelope + map + N/A reasons | API-02/11 BB | B40 | NOT STARTED | 2-format snapshot | — |
| B50 trading UX | T+2 inline, SL/TP/SHORT/sync, cancel LIMIT | U-01/02/04 part | B21 | NOT STARTED | SL/TP/SHORT/sync matrix UAT | B51 |
| B51 lab UX | builder→battle, undo, orphan nav, assumptions | U-03/04/05, U-06..12 | B50 | NOT STARTED | critical BUILDER→BATTLE + redirect + assumptions | B32/B40 |

Prompt coverage VERIFIED: `docs/plans/prompts/` has all 15 (B00, B01, B02, B10, B11, B20, B21, B22, B30, B31, B32, B40, B41, B50, B51). MASTER_REFACTOR_PLAN dependency graph VERIFIED (§46 mermaid, same ordering as above).

## Historical B00/B01 "CLOSED" vs today
Mission brief notes B00/B01 historically CLOSED in a last verified snapshot. Today's tree shows their files **re-modified and uncommitted** with new isolation tests untracked. Reconciliation: prior closure (if any) predates this rewrite; **treat B00/B01 as RE-OPEN pending re-acceptance on the current tree.** Do not rely on the historical fact.

## B02 quarantine: implemented REPORTED + guard VERIFIED, acceptance pending
Matches ADR-PROV-001 shape in code. Still needs: default-deny + flag-quarantine tests green, temp-DB hash evidence, reviewer sign-off, clean commit. Hence NOT CLOSED.

## DB-SAFETY-02R1
REPORTED implemented, tests unexecuted/unaccepted — consistent with B01 unaccepted. Map it as evidence input to B01 acceptance, not a separate closure.

## P-lineage reconciliation (provenance, not re-litigation)
STATE.json `complete` + per-batch review docs REPORTED. P-engines (signals registry, next-event kernel, market rules, batch runner, BB, universe) are the material B20–B22/B41 must unify — reuse, don't rewrite (TARGET §8). P-acceptance does not cover GAP P0/P1 (different bar: research acceptance vs product G/R/I/D/T). No P-batch is reopened; B-batches cite P-artifacts as inputs.

## Overlaps to manage
- B10↔B11: shared cursor/projected policy — sequence B10 then B11, same reviewer.
- B20→B21→B22: strict order; fee/settle/metrics must not leak into B20.
- B30→B31→B32: state before reliability before facade; B32's MappingResult UI needs B11's server cut first.
- B40↔B41: envelope framework (B40) before BB migration (B41).
- B50/B51 after B21/B20: UX wording depends on settlement/fill truth.
- P2-kernel vs B20: B20 wraps/retires legacy around the P2 kernel, with parity proof.

## New work vs existing absorption
No supplementary IDs proposed: every gap in GAP_REGISTER maps to an existing B-batch (traceability matrix VERIFIED). If later work needs IDs (e.g., holiday-calendar VN data sourcing, min-width enforcement), propose `B52+` only with a note on why B21/B51 cannot absorb it — none needed today.
