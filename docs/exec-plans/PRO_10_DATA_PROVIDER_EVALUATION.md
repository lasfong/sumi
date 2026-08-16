# PRO-10 — Market Data Provider Decision

Status: `CLOSED — INDEPENDENTLY APPROVED`

## Outcome

A thorough, evidence-backed Architectural Decision Record (`docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`) evaluates Vietnam market data provider candidates (e.g. SSI iBoard / FastConnect API, TCBS, DNSE Open API, Vietstock, CafeF, Entrade) across licensing, authentication/secrets security, Daily/Weekly historical coverage, corporate action adjustments, rate limits, network failure fallbacks, provider boundary isolation, and local-first privacy invariants. The ADR delivers a definitive `APPROVE` verdict unlocking PRO-11 synchronization under the `MarketDataProviderAdapter` boundary while solidifying local file import/catalog (`PRO-03`) as the permanent offline baseline.

## Context and problem

PRO-09 is independently approved and closed. Sumi's non-negotiable invariant is local-first architecture: no telemetry and no user trading data sent externally. Live or on-demand network data integration must not violate privacy, create hidden subscription traps, expose insecure credentials, or leak untrusted provider payloads into core domain services. Before writing provider integration code in PRO-11, PRO-10 conducts a rigorous evaluation spike and documents formal provider boundaries.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs `PRO-PROV-01` through `PRO-PROV-06`; `docs/program/PRO_10_MARKET_DATA_PROVIDER_DECISION.md`; V3 G-01..05 regression.

## In scope

1. **Provider Candidate Evaluation (`PRO-PROV-01`):**
   - Survey and analyze primary Vietnam market data APIs/sources (SSI FastConnect / Open API, DNSE Open API, TCBS Web Endpoints, `vnstock` community library, Commercial feeds: Vietstock/FiinPro/FireAnt, and CafeF EOD file archives).
   - Document licensing terms, commercial redistribution restrictions, rate limits, and attribution requirements.
2. **Security & Secret Handling Architecture (`PRO-PROV-02`):**
   - Define strict credential storage policies (environment variables, local secure key storage; never hardcoded or committed to git).
   - Ensure local-first isolation: outbound traffic is strictly limited to data fetching without sending user replay sessions, trades, or strategies.
3. **Data Quality, Coverage & Corporate Actions (`PRO-PROV-03`, `PRO-PROV-04`):**
   - Evaluate Daily & Weekly historical coverage depth (minimum 5–10 years for HOSE/HNX/UPCOM and benchmark indices VNINDEX, VN30).
   - Verify timestamp/timezone semantics (Asia/Ho_Chi_Minh UTC+7 aligned with existing `WeeklyAggregator` and daily candle conventions).
   - Define corporate actions policy: unadjusted vs adjusted (dividend, split) prices, ensuring replay and backtest reproducibility.
4. **Resilience, Throttling & Offline Invariants (`PRO-PROV-05`):**
   - Design fail-closed retry, exponential backoff, rate limit handling, and graceful offline fallback.
   - Sumi must function 100% offline using existing local SQLite database / file imports if no network or provider is available.
5. **Architectural Decision Record & Provider Boundary (`PRO-PROV-06`):**
   - Author `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` documenting evaluated candidates, decision matrix, chosen candidate (or rejection), provider adapter boundary interface (`MarketDataProviderAdapter`), and migration roadmap for PRO-11.
6. **Verification & Gates:**
   - Ensure existing fast technical gate (`verify-v2.ps1`) and product UAT (`run-product-uat.ps1`) continue passing with 0 regressions.
   - Verify `backend/sumi.db` SHA-256 remains untouched.

## Out of scope

- Direct implementation of automated sync UI / background workers (belongs to PRO-11).
- Direct implementation of provider client network code in core backend (belongs to PRO-11).
- Publish release candidate (PRO-12).

## Invariants

- Local-first privacy: no user trading, journal, replay, or strategy data is ever sent externally.
- No third-party provider types/schemas leak into core `Candle`, `IndicatorEngine`, or `Replay` domain services.
- `backend/sumi.db` SHA-256 remains untouched during tests/UAT.

## Acceptance mapping

| ID | Requirement | Status | Evidence |
| --- | --- | :---: | --- |
| PRO-PROV-01 | Terms, licensing, redistribution, and attribution obligations for every considered candidate are recorded in a decision record. | PASS | Detailed survey of SSI FastConnect, DNSE Open API, TCBS, vnstock, Vietstock/FiinPro, and CafeF recorded in `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` (Section 2 & Section 3.1). |
| PRO-PROV-02 | Authentication, credential storage, and secret-handling design are documented without hardcoded keys or insecure persistence. | PASS | Secret lifecycle, env variable isolation, local storage encryption, frontend key masking, and ephemeral in-memory JWT token caching documented in ADR-002 (Section 3.2). |
| PRO-PROV-03 | Daily and weekly history coverage, lookback depth, rate limits, and latency are evaluated against sample Vietnam equity datasets. | PASS | Historical depth (5–15+ years for HOSE/HNX/UPCOM, VNINDEX, VN30), rate limits, and latency benchmarks documented in ADR-002 (Section 3.3). |
| PRO-PROV-04 | Corporate action handling (dividends, splits, adjustments) and timestamp/timezone semantics are explicitly verified. | PASS | Strict isolation between `adjusted` and `unadjusted` candles, `Asia/Ho_Chi_Minh` UTC+7 timezone semantics, and `WeeklyAggregator` internal derivation documented in ADR-002 (Section 3.4). |
| PRO-PROV-05 | Error handling, retry/throttling strategy, network failure fallbacks, and offline local-first behavior are designed to fail closed. | PASS | Fail-closed retry with exponential backoff, rate limit handling, user-triggered on-demand sync invariant, and 100% offline local SQLite parity documented in ADR-002 (Section 3.5). |
| PRO-PROV-06 | Decision outcome is recorded in `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` with explicit approve/reject verdict, boundary isolation design, and implementation plan for PRO-11 (or local-file retention plan). | PASS | Authored `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` with definitive `APPROVE` verdict, `MarketDataProviderAdapter` abstract interface specification, and detailed PRO-11 delivery roadmap. |

## Verification commands & results

```powershell
# 1. Database SHA-256 Before
Get-FileHash -Algorithm SHA256 backend\sumi.db
# Output: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A

# 2. Backend Pytest
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/ -v
# Output: 176 passed, 1 warning in 5.59s

# 3. Frontend Vitest
Set-Location ..\frontend
npm.cmd test -- --run
# Output: 27 test files passed, 182 tests passed in 30.53s

# 4. Fast Technical Gate
Set-Location ..
.\scripts\verify-v2.ps1
# Output:
# - Backend pytest: 176 passed
# - Alembic migration: clean (0 drift)
# - ESLint: 0 errors, 0 warnings
# - Frontend vitest: 182 passed
# - Frontend production build: clean (0 errors, 683ms)

# 5. Product UAT Suite
.\scripts\run-product-uat.ps1
# Output:
# - Directory: test-results/product-uat/2026-08-16T14-51-49-955Z/
# - results.json SHA-256: 20AEFF4435C1A1D615F06E3E2AF059827972F88E8C0F31C33D9216AE22212DD1
# - Assertions: 333 / 333 passed, 0 failed, 0 blocking failed
# - Manifest reconciliation: pass: true
# - Runtime errors: 0
# - Provider errors: 0

# 6. Formatting & Diff Check
git diff --check
# Output: 0 errors

# 7. Database SHA-256 After
Get-FileHash -Algorithm SHA256 backend\sumi.db
# Output: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A (0 bytes mutated)
```

## Progress log

- 2026-08-16: User authorized PRO-10. Reviewer prepared ExecPlan and standalone DEV prompt.
- 2026-08-16: Conducted deep evaluation spike across Vietnam market data providers (SSI FastConnect, DNSE Open API, TCBS, `vnstock`, Commercial vendors, and CafeF baseline).
- 2026-08-16: Formulated and authored `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` covering legal licensing, credential security, historical depth, corporate actions, error resilience, abstract `MarketDataProviderAdapter` interface design, and PRO-11 roadmap.
- 2026-08-16: Executed full verification suite: backend pytest (176 passed), frontend vitest (182 passed), `verify-v2.ps1` (clean lint, build, tests), deterministic Product UAT (333/333 passed, reconciliation pass), DB invariant verified (0 bytes mutated). Batch is ready for Independent Reviewer Gate audit.
- 2026-08-16: Independent Reviewer audited ADR-002, provider contracts, and test evidence. Verdict: `APPROVE` recorded in `docs/reviews/PRO_10_REVIEW_2026-08-16.md`. PRO-10 is closed; PRO-11 remains unauthorized.
