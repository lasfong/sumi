# Documentation Index

This file is the starting point for anyone working on Sumi.

## Canonical Documents

Read these in order:

1. `docs/PRODUCT_SPEC.md`
   - Canonical product specification for the current MVP.
   - The project currently follows the local-first SQLite/manual replay direction in this document.

2. `docs/DECISIONS.md`
   - Records current architecture decisions.
   - If implementation conflicts with old specs, follow this file and `PRODUCT_SPEC.md`.

3. `docs/IMPLEMENTATION_PLAYBOOK.md`
   - Step-by-step execution plan for junior developers.
   - Follow the phases in order. Do not jump to later phases before the previous phase passes its Definition of Done.

4. `docs/PROJECT_AUDIT_2026-06-26.md`
   - Current-state audit of the project as of 2026-06-26.
   - Use this to understand what is real, what is incomplete, and what needs hardening.

## Vision / Future Documents

5. `docs/SPEC.md`
   - Long-term vision and full PRD.
   - Not the implementation contract for the immediate MVP.
   - Features such as PostgreSQL/TimescaleDB, Celery/Redis workers, and full algorithmic strategy execution are future scope unless explicitly promoted in `DECISIONS.md`.

6. `docs/FUTURE_ROADMAP.md`
   - Clean summary of future phases after the MVP is stable.

## Operational Notes

- CafeF sample/raw data should live under `data/raw/`, not under `docs/`.
- Local databases such as `backend/sumi.db` are runtime artifacts and should not be committed.
- Research clones and POC repositories should live under `research_repos/` locally only and should not be committed.
- Generated folders such as `__pycache__`, `.pytest_cache`, `node_modules`, and build outputs are ignored.

## Sprint Plans

7. `docs/sprints/sprint_plan_overview.md`
   - Tổng quan 9 sprints (0–8) với dependency graph, quy trình bắt buộc.
   - Đây là entry point cho toàn bộ sprint plans chi tiết.

Sprint plans chi tiết nằm trong `docs/sprints/`:
- Sprint 0: Environment Setup
- Sprint 1: Frontend Build Green
- Sprint 2: Backend Tests Green
- Sprint 3: No-Future-Leak Lock
- Sprint 4: Trade Lifecycle Hardening
- Sprint 5: Limit Order & Market Constraints
- Sprint 6: Analytics Standardization
- Sprint 7: Schema & Migration Cleanup
- Sprint 8: Algorithmic Backtest Engine MVP

## Current Priority

The current priority is not new feature development.

The current priority is:

```text
Frontend build green
-> Backend tests green
-> No future leak for candles and indicators
-> Correct T+2 trade lifecycle
-> Limit orders and market constraints
-> Correct analytics
-> Clean migration/schema
-> Algorithmic engine
```

