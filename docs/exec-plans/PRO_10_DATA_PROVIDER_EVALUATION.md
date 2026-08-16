# PRO-10 — Market Data Provider Decision

Status: `PREPARED — USER AUTHORIZED`

## Outcome

A thorough, evidence-backed Architectural Decision Record (`docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`) evaluates Vietnam market data provider candidates (e.g. SSI iBoard / FastConnect API, TCBS, DNSE Open API, Vietstock, CafeF, Entrade) across licensing, authentication/secrets security, Daily/Weekly historical coverage, corporate action adjustments, rate limits, network failure fallbacks, provider boundary isolation, and local-first privacy invariants. The ADR delivers a definitive `APPROVE` or `REJECT` verdict unlocking PRO-11 synchronization or solidifying local file import/catalog as the stable path.

## Context and problem

PRO-09 is independently approved and closed. Sumi's non-negotiable invariant is local-first architecture: no telemetry and no user trading data sent externally. Live or on-demand network data integration must not violate privacy, create hidden subscription traps, expose insecure credentials, or leak untrusted provider payloads into core domain services. Before writing provider integration code in PRO-11, PRO-10 conducts a rigorous evaluation spike and documents formal provider boundaries.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs `PRO-PROV-01` through `PRO-PROV-06`; `docs/program/PRO_10_MARKET_DATA_PROVIDER_DECISION.md`; V3 G-01..05 regression.

## In scope

1. **Provider Candidate Evaluation (`PRO-PROV-01`):**
   - Survey and analyze primary Vietnam market data APIs/sources (e.g. SSI Open API / iBoard, TCBS, DNSE Open API, Entrade / vnstock community adapters, Vietstock, CafeF).
   - Document licensing terms, commercial redistribution restrictions, rate limits, and attribution requirements.
2. **Security & Secret Handling Architecture (`PRO-PROV-02`):**
   - Define strict credential storage policies (environment variables, local secure key storage; never hardcoded or committed to git).
   - Ensure local-first isolation: outbound traffic is strictly limited to data fetching without sending user replay sessions, trades, or strategies.
3. **Data Quality, Coverage & Corporate Actions (`PRO-PROV-03`, `PRO-PROV-04`):**
   - Evaluate Daily & Weekly historical coverage depth (e.g., minimum 5–10 years for HOSE/HNX/UPCOM).
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

- Direct implementation of automated sync UI / background workers (belongs to PRO-11 if approved).
- Publish release candidate (PRO-12).

## Invariants

- Local-first privacy: no user trading, journal, replay, or strategy data is ever sent externally.
- No third-party provider types/schemas leak into core `Candle`, `IndicatorEngine`, or `Replay` domain services.
- `backend/sumi.db` SHA-256 remains untouched during tests/UAT.

## Acceptance mapping

| ID | Requirement |
| --- | --- |
| PRO-PROV-01 | Terms, licensing, redistribution, and attribution obligations for every considered candidate are recorded in a decision record. |
| PRO-PROV-02 | Authentication, credential storage, and secret-handling design are documented without hardcoded keys or insecure persistence. |
| PRO-PROV-03 | Daily and weekly history coverage, lookback depth, rate limits, and latency are evaluated against sample Vietnam equity datasets. |
| PRO-PROV-04 | Corporate action handling (dividends, splits, adjustments) and timestamp/timezone semantics are explicitly verified. |
| PRO-PROV-05 | Error handling, retry/throttling strategy, network failure fallbacks, and offline local-first behavior are designed to fail closed. |
| PRO-PROV-06 | Decision outcome is recorded in `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` with explicit approve/reject verdict, boundary isolation design, and implementation plan for PRO-11 (or local-file retention plan). |

## Verification commands

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/ -v
Set-Location ..\frontend
npm.cmd test -- --run
Set-Location ..
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

## Progress log

- 2026-08-16: User authorized PRO-10. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
