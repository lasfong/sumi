# Execution queue and gates

The detailed task cards/formulas in `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` remain authoritative. This routing table schedules them; it does not widen or replace their dependencies. The orchestrator must check each task card's dependency list before coding. If a card requires a dependency beyond STATE.json, add it and record why; never remove a semantic dependency just to unlock work.

| Tasks | Work and exit gate |
| --- | --- |
| BOOTSTRAP | Current-tree evidence, executable baseline, causal harness readiness, independent reviewer capability. |
| P1-SIG-01, P1-SIG-02 | Already reviewer-sealed; do not reimplement. Recheck identity after migration. |
| P2-BT-01 | Pure next-event execution with hand-calculated fills/ledger and future-invariance. No same-close fills. |
| P2-MKT-02 | Effective-dated VN market rules; verify authoritative source/assumptions. Unknown rules block historical-correctness claims. |
| P3-PRICE-01, P3-VSA-02 | Price/volume signals and causal support/resistance; independent of BB/provider upgrades. |
| P4-BATCH-01, P4-MET-02 | Independent capital, compute-once phase runner, approved phase-end policy; exact nine metrics from verified ledger. |
| P5-DATA-01, P5-BB-02 | Provider-neutral contracts then Symbol OHLCV_PROXY; fixtures for contracts, targeted fresh evidence for real bars and adjustment/value-source policy. |
| P6-MEASURE-01 | Real audited sample measurement validation, not P&L optimization. No promotion from synthetic fixtures. |
| P7-UNI-01, P7-MKT-02 | Universe/aggregate frameworks can use labeled fixtures. Publishing SUMI420_v1/Market BB requires approved membership/effective dates, coverage and comparable methods. Framework completion is not task publication acceptance. |
| P8-ICHI-01, P8-DIV-02 | Can progress independently of blocked BB branches; causal displacement and confirmed-pivot tests. |
| P8-COMP-03 | Composition/health/BB events; separate completed technical pieces from BB promotion blocked by measurement evidence. |
| P9-UI-01 | Unified validation of implemented capabilities; unavailable data/capabilities explicit, never invented. Partial UI progress is not whole-task acceptance. |
| P10-PERF-01, P10-SEAL-02 | Optimize measured verified hotspots; final integrated correctness, evidence and migration package. No whole-plan seal while mandatory tasks are blocked. |

State dependencies are conservative scheduling prerequisites; blocked task frameworks may be developed in scoped submilestones once independent review identifies inputs satisfied, but parent task stays blocked until all gates pass. The orchestrator may split cards into local checkpoints without a new owner prompt or loosening the parent contract.

## Evidence economy

Reuse accepted baseline and unchanged test evidence by exact content identity. Focused tests during repair; full applicable suites at stable batch/final boundaries. Do not reread historical P1R prompts on every batch, duplicate giant reports, or repeat full Doraemon research. Checkpoint holds paths and actionable findings, not source/log dumps.
