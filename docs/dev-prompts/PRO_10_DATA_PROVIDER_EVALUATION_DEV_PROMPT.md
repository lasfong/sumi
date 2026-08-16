# PRO-10 DEV Prompt — Market Data Provider Decision

You are the dedicated DEV session for **PRO-10 — Market Data Provider Decision**. Implement this batch from the current workspace checkout; do not rely on chat history. Stop at the Independent Reviewer Gate when implementation and verification are complete. Do not approve your own work, commit, push, or start PRO-11.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-10, PRO-PROV-01..06)
6. `docs/program/PRO_10_MARKET_DATA_PROVIDER_DECISION.md`
7. `docs/exec-plans/PRO_10_DATA_PROVIDER_EVALUATION.md`

## Outcome

A comprehensive, evidence-backed Architectural Decision Record (`docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`) evaluates Vietnam market data provider candidates across licensing, security, data quality/coverage, corporate action adjustments, failure modes, provider boundary isolation, and local-first privacy invariants. The ADR delivers a definitive `APPROVE` or `REJECT` verdict unlocking PRO-11 or solidifying local file import/catalog as the stable path.

## Implementation tasks

1. **Provider Candidates Spike & Analysis (`PRO-PROV-01`, `PRO-PROV-02`):**
   - Survey major Vietnam data providers (SSI FastConnect/iBoard, TCBS, DNSE Open API, Vietstock, CafeF, Entrade).
   - Document licensing, commercial rights, redistribution rules, and secret management design (env variables, secure local storage, no hardcoded keys).

2. **Historical Coverage, Timezones & Corporate Actions (`PRO-PROV-03`, `PRO-PROV-04`):**
   - Document Daily & Weekly coverage depth for HOSE, HNX, and UPCOM.
   - Document timestamp/timezone semantics (Asia/Ho_Chi_Minh UTC+7 aligned with Sumi `WeeklyAggregator`).
   - Define exact adjustment rules (split/dividend adjusted vs unadjusted raw prices) and audit trail requirements.

3. **Error Resilience, Throttling & Offline Invariants (`PRO-PROV-05`):**
   - Design fail-closed retry logic, rate limit handling, and graceful offline fallback.
   - Preserve zero-telemetry and local-first invariants.

4. **Architectural Decision Record Authoring (`PRO-PROV-06`):**
   - Author `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` with:
     - Context and problem statement
     - Evaluated candidates comparison matrix
     - Decision verdict (`APPROVE` with selected provider or `REJECT` with local file import retention)
     - Provider boundary adapter interface design (`MarketDataProviderAdapter` abstract class specification in docs / provider contract)
     - Implementation requirements for PRO-11 (or local file retention plan)
     - Consequences and compliance with Sumi non-negotiable invariants

5. **Technical Gates Verification:**
   - Run backend pytest tests and vitest frontend tests.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_10_DATA_PROVIDER_EVALUATION.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact ADR path, verification commands, and test counts. Do not commit, push, or start PRO-11.
