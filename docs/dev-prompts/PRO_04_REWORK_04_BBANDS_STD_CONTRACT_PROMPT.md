# DEV prompt — PRO-04 REWORK-04: Bollinger non-default standard-deviation contract

You are the DEV session.  Implement only this bounded correction in the existing checkout.  Do not approve your own work, commit, push, change acceptance criteria, or start PRO-05.

## Read first, in order

1. `AGENTS.md`
2. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md` (if present; otherwise `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`)
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `docs/reviews/PRO_04_REVIEW_2026-08-15_R4.md`
5. `docs/exec-plans/PRO_04_CORE_INDICATOR_EXPANSION.md`
6. `docs/program/PRO_04_CORE_INDICATOR_EXPANSION.md`

## Authority and defect

The public released parameter remains `bbands.std` (`float`, 0.1..10).  The pinned `pandas-ta.bbands` function accepts `lower_std` and `upper_std`, not `std`.  Current backend forwarding silently uses the library default, while the frontend expects a requested-value column name.  A valid non-default user setting such as `std=2.25` therefore computes default `BB*_20_2.0_2.0` output and renders no bands.

## Required implementation

1. In the backend `IndicatorEngine`, translate the public normalized `std` only for Bollinger computation to `lower_std=std` and `upper_std=std`.  Keep the public registry/API/persisted product parameter named `std`; do not expose the library parameter names.
2. Replace the frontend Bollinger `toFixed(1)` assumption with one deterministic formatter matching the pinned backend's actual output representation: integral float values retain `.0`; fractional values retain the meaningful submitted float precision (e.g. `2.25`, `1.15`).  Build all exact BBU/BBM/BBL names through that formatter.  Never use a prefix, alias, alternate spelling, or arbitrary-column fallback.
3. Preserve the R3 all-or-nothing contract: any missing or mismatched band component produces `[]` and a truthful non-ready state.
4. Add tests before/with the implementation:
   - backend: `std=2.25` exact BBL/BBM/BBU names and values materially distinct from default `2.0`;
   - frontend: exact `2.25` columns render three semantic components; rounded (`2.2`), default (`2.0`), alias, partial, or mismatched columns return `[]`;
   - parity: scoped replay response honors the non-default public parameter;
   - Product UAT: add a `std=2.25` Bollinger instance and assert its rendered upper/middle/lower values equal the three exact scoped backend response values, are ready, and are visibly retained in the two required screenshots.
5. Update only PRO-04 execution evidence/state after all gates pass.  State must say `IMPLEMENTED — REVIEW PENDING (REWORK-04 COMPLETE)` and link this prompt and R4 review record.  Do not mark PRO-04 approved/closed.

## Invariants

- Do not mutate `backend/sumi.db`; record SHA-256 before and after.
- No future-candle leakage; scoped replay responses must end at `current_index`.
- Backend remains indicator-calculation authority.
- Preserve all currently unrelated dirty changes.
- Do not weaken/remove an assertion or change criteria merely to make a gate green.

## Required evidence and verification

Run and record exact commands/results:

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/test_indicators.py app/tests/test_indicator_parity_e2e.py -q
Set-Location ..\frontend
npm.cmd test -- --run src/features/indicators src/components/chart/__tests__/IndicatorRenderRegistry.test.ts src/components/chart/__tests__/IndicatorPaneChrome.test.tsx src/components/chart/__tests__/SeriesManager.test.ts src/components/chart/__tests__/PaneManager.test.ts
Set-Location ..
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
& 'C:\Program Files\Git\bin\bash.exe' -lc 'cd /e/Workspace/sumi && SUMI_PYTHON=/e/Workspace/sumi/backend/.venv/Scripts/python.exe ./scripts/verify-product.sh'
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

Retain Product UAT `results.json`, manifest reconciliation, screenshot paths/hashes at 1440×1000 and 1280×800, no-page/no-console-error result, DB before/after hash, process cleanup, and focused/full counts.  Inspect both screenshots yourself.

## Stop condition / exact final report

When all implementation and evidence are complete, stop at the Independent Reviewer Gate and report only:

```text
Execution stops at the Independent Reviewer Gate. The codebase, documentation, and evidence artifacts are ready for R5 Independent Reviewer audit. PRO-05 remains unauthorized.
```
