# SUMI Proposed Development Roadmap — 2026-10-12 (PROPOSED)

**Status of this file:** proposal for GPT-6 ratification. Nothing here changes code, ADRs, or batch definitions until approved. Existing B00–B51 IDs/scopes preserved; sequencing follows MASTER_REFACTOR_PLAN graph. No calendar deadlines; no effort estimates claimed as verified.
**Entry rule:** Wave 0 first. No financial/FE/UX batch starts before its safety/correctness predecessors are accepted. Any red → STOP, no weakened tests, no same-batch acceptance-criteria edits (AGENTS.md).

## Wave 0 — Stabilize & baseline (sequential, single writer)
**W0.1 Accept-or-repair B00+B01+B02 slice (PROPOSED first implementation wave).**
- Objective: trustworthy DB + transactions + synthetic isolation on the current tree.
- Entry: clean isolation of the 17-file diff; B00–B02 prompt specs; temp-DB tooling (`verify-v2.sh` VERIFIED).
- Scope: `config, db, dependencies, alembic/env, import/sync/weekly workflows, providers guard/registry, sync FE banner, sync_schema, main wiring, b01/b02/cwd tests, verify script`. Non-goals: replay, execution, FE state, contracts, UX.
- Invariants: no `backend/sumi.db` mutation (hash-unchanged); local-first; single writer; no contract changes outside sync schema.
- Acceptance: CWD-launch creates nothing; inject-fault atomic + no PendingRollback; FK reject/cascade; stub execute default-deny 403 + flag→quarantine-only; full backend suite + migration-from-empty green on temp DBs; reviewer inspects diff + evidence.
- Exit: B00, B01, B02 individually marked accepted with evidence paths; tree clean.
- Risk: high (touches all writes). Team: 1 implementer + independent reviewer. Review: mandatory (architect-level per TARGET §4 sensitivity).
**W0.2 B03 (gated on W0.1).** Full-model env + `_has_column` + preview-confirm. Exit: empty→head 20 tables; preview-without-confirm commits nothing.

## Wave 1 — Data & replay correctness (B10 → B11; B10 needs B01 only)
- **B10 ledger filter + meta clamp.** Objective: zero future activity in replay reads. Acceptance: at index N raw endpoints return nothing >N; `total_bars/release` clamped; regression suite green. Non-goals: indicator projection (B11), FE rework (B30).
- **B11 projected drop.** Objective: server cuts `>max_visible_ts`; FE types warming/mapping/server/projected/transport. Acceptance: Ichimoku+26 (and peers) absent from payload + render; MappingResult surfaces reasons. Depends on B10 cursor policy.
- Parallel-safe with Wave 2 prep (read-only golden-case authoring) but NOT with execution changes.

## Wave 2 — Financial semantics consistency (B20 → B21 → B22, strict)
- **B20 kernel parity.** One next-open kernel; sweep routed via kernel+phases; sizing on fill-estimate; legacy deprecated+bannered. Acceptance: sweep==batch parity + golden G-01..G-05/G-11/G-12. STOP on unexplained PnL delta.
- **B21 MarketProfile + SettlementEngine.** Single fee/slippage/tax/lot + calendar T+1.5 (+ holiday calendar); T+2 inline wording. Acceptance: weekend/holiday golden G-06/G-07; slippage single-source grep; UAT inline availability.
- **B22 metrics unify.** Net PnL, breakeven=not-winner, PF-inf→N/A, N≥30 gate, one Sharpe, close-based MaxDD. Acceptance: G-08..G-10 + analytics/practice/batch agreement.
- Invariants: PnL-history change documented; legacy removal only after parity green + 1 release banner.

## Wave 3 — FE/BE integration (B30 → B31 → B32; needs B10 echo)
- **B30 keys/nav/switch.** Full TF+adjustment keys; nav token/cancel; switch checklist; QueryClient defaults; Zustand→sessionId. Acceptance: step-x20 race, no-flash switch, single-clear grep.
- **B31 WS reliability.** BE state machine + FE remote per ADR-WS-001. Acceptance: chaos matrix (restart-mid-play, offline, bad JSON, speed clamp).
- **B32 ChartKernel + god-split-1 + drawing cleanup.** Thin facade; Header/Dock split; RenderRegistry→MappingResult; delete 3 dead files. Acceptance: `createChart` only in facade; drawing UAT pan/zoom/advance/reload.

## Wave 4 — Contracts & feature completion (B40 → B41; B50/B51 after B21/B20)
- **B40 envelope + snapshot + dead routes + 422.** Acceptance: openapi snapshot CI; no `[object Object]`; single catalog; one signalsClient; dead routes removed.
- **B41 BB envelope unify.** Acceptance: one envelope; `horizons[H].bb_value` map; N/A with reasons.
- **B50 trading UX.** Inline T+2, SL/TP validation + SHORT math + all-direction sync + LIMIT cancel. Acceptance: matrix UAT.
- **B51 lab UX.** Builder→Battle prefill (or honest Copy-YAML), undo discoverability + shortcuts + toast, `/backtest` redirect + Single-vs-Battle note, assumptions everywhere, min-width + mobile banners, empty-state/empty-language cleanup. Acceptance: critical BUILDER→BATTLE + UAT.

## Wave 5 — Regression, acceptance & release readiness
- Full G-01..G-05 gate: backend + FE tests, lint/type/build, deterministic product UAT (temp-DB, both viewports, machine-readable results + screenshots retained), no P0/P1 in released workflow.
- Release notes state scope + limitations explicitly; "professional manual replay and TA practice product" language only if all in-scope G/R/I/D/T pass; "TradingView-like" stays prohibited.
- P2 perf/cleanup list opportunistically per-file (no scope creep).

## Dependency map
```
W0: B00 → B01 → {B02, B03(gated B02-accepted per rule)}
W1: B01 → B10 → B11 ─┐
W2: B01 → B20 → B21 → B22 ├→ W4: B40 → B41 ; B50/B51 (need B20/B21)
W3: B10 → B30 → {B31, B32(B11 for mapping UI)}
W5: all waves → release gate
```
Parallel-safe: W1 vs W2-vs-W3 planning/evidence only; W3 internal B31 vs B32-file-split with disjoint files; B40 vs B50-spec work. Sequential: everything in W0; B20→B21→B22; B10→B11; B30→B31; B40→B41. Single production writer always.

## Per-batch card template (apply to each B above; details in DEV_BATCHES + prompts)
ID/name · objective · dependencies/entry · scope + non-goals · files · invariants · acceptance · tests/evidence · risk · roles (implementer/test/reviewer) · independent review · exit/closure. ExecPlan per PLANS.md before code; verification commands recorded; rollback path (flags: `REPLAY_STRICT`, `EXECUTION_KERNEL=legacy` + banner, FE-only reverts, dual-read VARCHAR/JSON) documented per batch.
