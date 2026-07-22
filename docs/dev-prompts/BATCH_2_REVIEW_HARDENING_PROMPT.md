# DEV prompt — Batch 2 Reviewer hardening

Continue in the existing Sumi checkout and working tree. Do not create or switch a branch/worktree. Do not commit, push, merge, reset, clean, retag, discard, or overwrite existing Reviewer/user changes.

This is a bounded continuation of Batch 2 only. Do not begin Batch 3.

## Read before editing

Read completely:

1. `AGENTS.md`
2. `PLANS.md`
3. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
4. `docs/exec-plans/BATCH_2_PROFESSIONAL_INDICATOR_MANAGER.md`
5. `docs/reviews/BATCH_2_REVIEW_2026-07-16.md`
6. `scripts/product-uat.mjs`
7. Current indicator controller/domain/request/chart/pane/series/manager code and tests.

Update the existing Batch 2 ExecPlan before code changes. Add B2-R01 through B2-R04, the reproduced independent result, exact planned modules/tests, rollback, decisions, and progress. Do not weaken or remove accepted behavior.

## Required closure

### B2-R01 — Deterministic responsive pane layout

- Fix the creation/order/reconciliation race so the documented 4:1 price-to-each-visible-subpane policy is true after asynchronous request completion, add/remove, visibility toggle, reorder, reload, ten remounts, and resize.
- Layout must not depend on which indicator request resolves last.
- Reconcile through official Lightweight Charts v5 pane APIs only.
- Price must remain approximately four times each visible subpane; visible subpanes must be mutually consistent within a documented tolerance.
- Define and record a minimum readable-height/overflow policy for 1280×800. The current 24px CCI pane is unacceptable.
- Add focused tests with panes created in multiple asynchronous orders and repeated layout calls.

### B2-R02 — Correct request/render/layout failure semantics

- Separate transport/abort/stale handling from semantic mapping and chart/layout application.
- Do not label a post-HTTP-200 render/layout exception as `Request failed`.
- Do not leave partially applied series/panes while presenting a misleading runtime status.
- Make render/layout failures observable to focused tests and browser UAT; the gate must fail rather than silently accepting an error card.
- Retain isolated per-instance recovery and valid prior data where safe, with honest user-facing wording.
- Prove final active instances have no error state after initial load, reload, lifecycle churn, and compact resize.

### B2-R03 — Real visible pane chrome and reference evidence

- Implement visible pane-associated title, legend/current values, settings, visibility, and close affordances for RSI, MACD, CCI, and Volume. Keep one Sumi domain owner; pane chrome must dispatch existing manager commands rather than create parallel state.
- Make RSI 30/50/70 and CCI -100/0/100 reference values visibly understandable in the actual product. Avoid label/current-value overlap.
- MACD line/signal/histogram/zero labels and values must remain readable.
- Remove hidden hard-coded acceptance-surrogate nodes as evidence. Test-only snapshots may expose actual chart state, but visible-UX criteria require visible DOM/browser evidence.
- Preserve the always-visible active manager list and all accepted add/edit/cancel/hide/remove/order/persistence behavior.

### B2-R04 — Strengthen UAT without changing the contract

Update `scripts/product-uat.mjs` additively. Do not delete, rename away, weaken, or relabel any existing assertion.

At minimum assert:

- actual pane order matches the ordered visible indicator document;
- the fixed 4:1 ratio within a recorded tolerance after initial render, reload, every one of ten remount cycles, and 1280×800 resize;
- the documented minimum readable-height/overflow policy at both viewports;
- no visible active indicator is in `error` at any evidence checkpoint;
- failed/aborted indicator requests are observed, not omitted from a success-only list;
- visible pane chrome contains the correct instance title/params/current values and working settings/hide/remove controls tied to that stable ID;
- actual chart snapshot contains RSI/CCI/MACD reference definitions and the corresponding values are visibly labeled in the pane UX;
- the final 1440×1000 and 1280×800 screenshots contain no clipped required controls, collapsed pane, misleading error, or overlapping legend/reference labels.

Keep the seven retained Batch 3/legacy drawing failures visible:

- `drawings.tool-trendline`
- `drawings.tool-ray`
- `drawings.tool-rectangle`
- `drawings.tool-fibonacci`
- `drawings.tool-text`
- `drawings.selection-contract`
- `drawings.persist-after-create`

## Required regression scope

- I-01–I-07, I-10, and I-13 remain green.
- Backend IndicatorEngine/session-scoped endpoint remains authoritative; no future candles.
- Batch 1 Horizontal create/select/move/cancel/undo/redo/persistence/remount behavior remains green.
- Production `backend/sumi.db` remains byte-for-byte unchanged.
- No new dependency, backend contract, migration, community provider, drawing tool, broad visual redesign, or Batch 3 work.

## Verification

Run:

```bash
git diff --check
cd frontend && npm test -- --run
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

Retain machine results and screenshots. Review both images manually before handoff. Report actual pane heights/order/ratios and runtime status at initial, reload, ten-remount, 1440×1000, and 1280×800 checkpoints.

## Final handoff

End with:

`BATCH 2 HARDENING COMPLETE — STOPPED AT REVIEWER GATE`

Report B2-R01–B2-R04 separately, exact focused/full test counts, independent-style product-UAT counts, every retained failure ID, pane height/ratio evidence at all checkpoints, error/request counts, artifact paths, DB before/after hash, deviations, and known limitations. Do not claim Batch 2 closed and do not start Batch 3.
