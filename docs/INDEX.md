# Documentation Index

This is the starting point for Sumi development after the V2 cleanup.

## Canonical V2 Documents

Read these in order:

1. `docs/PRODUCT_STRATEGY_V2.md`
   - Product thesis, priority tiers, current repo reality, and product decisions.

2. `docs/SPEC_V2.md`
   - Umbrella product/technical spec separating Manual Replay and Automated Backtest.

3. `docs/MANUAL_REPLAY_SPEC.md`
   - Manual replay UX, no-future-leak rules, indicators, drawings, trading simulation, journal, and manual analytics.

4. `docs/BACKTEST_ENGINE_SPEC.md`
   - Safe automated backtest engine, strategy model, result slicing, analytics outputs, and security rules.

5. `docs/ACCEPTANCE_CRITERIA_V2.md`
   - P0/P1 release gates, command gates, manual replay UAT, backtest UAT, and analytics acceptance.

6. `docs/ROADMAP_TO_COMPLETION.md`
   - Phase-by-phase developer execution plan.

7. `docs/PROGRESS_V2.md`
   - Active execution tracker: completed batches, phase status, and remaining large batches.

8. `docs/FEATURE_MATRIX_RESEARCH.md`
   - Research matrix and links for comparable professional platforms.

9. `docs/DECISIONS.md`
   - Architecture and product decisions. If implementation conflicts with V2 specs, update this file explicitly.

## Supporting Materials

- `docs/BacktestSample/Sample.md`
  - Example output shape for symbol/period/regime-oriented backtest analytics.

- `docs/AGENTS.md`
  - Agent/developer operating guidance.

## Archived Pre-V2 Documents

Historical documents were moved to:

```text
docs/archive/pre_v2/
```

They are useful for context but are no longer the implementation contract. Do not follow archived sprint plans or old specs if they conflict with V2 documents.

## Current Development Priority

Do not add broad new product surface until P0/P1 blockers from `ACCEPTANCE_CRITERIA_V2.md` pass.

Required order:

```text
Fix P0/P1 blockers
-> Lock manual replay MVP
-> Standardize analytics
-> Build indicator/signal registry
-> Build safe backtest MVP
-> Add regime slicing
-> Add scanner
-> Add strategy comparison
```

## Operational Notes

- Raw CafeF files belong under `data/raw/`, not `docs/`.
- Local databases such as `backend/sumi.db` are runtime artifacts and should not be committed.
- Research clones and POC repositories belong under `research_repos/` locally only.
- Generated folders such as `__pycache__`, `.pytest_cache`, `node_modules`, and build outputs are ignored.
