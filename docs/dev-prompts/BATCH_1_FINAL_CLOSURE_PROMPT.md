# DEV prompt — Batch 1 final closure

Return to the existing Sumi V3 Batch 1 checkout. Do not create/switch a branch or worktree. Do not commit, push, merge, reset, clean, retag, or discard existing changes.

Read completely before editing:

1. `AGENTS.md`
2. `docs/exec-plans/BATCH_1_REPLAY_WORKSPACE_FOUNDATION.md`
3. `docs/reviews/BATCH_1_REVIEW_2026-07-16.md`, especially “Reviewer re-inspection”

Update the existing ExecPlan before code changes. This continuation is limited to B1-R10 and B1-R11. Do not refactor already accepted work and do not begin Batch 2.

## B1-R10 — Native pointer cancellation

- Do not route `pointercancel` through the commit/pointer-up handler.
- During an active drag, native `pointercancel` must restore the exact pre-drag drawing.
- It must emit no `change-committed` event and create no history/persistence revision.
- Restore chart scrolling.
- Release pointer capture safely only if applicable; browser automatic capture loss must not throw.
- Preserve the intended selection/tool state and publish the final interaction snapshot.
- Remove the dedicated listener correctly during idempotent destroy.

Required focused test:

- start drag;
- preview a changed price;
- dispatch `pointercancel`;
- assert exact rollback;
- assert zero commit;
- assert capture/scroll cleanup;
- assert no event after destroy.

Add a scoped browser UAT check that dispatches a real `PointerEvent('pointercancel')` during drag and proves domain document, localStorage, revision, and history availability remain unchanged.

## B1-R11 — Repository identity collision

Current failure to resolve:

1. storage contains session 7 / FPT / revision N;
2. application opens session 7 / SSI;
3. `load(7, 'SSI')` returns null;
4. first SSI save at expected revision 0 conflicts against the old FPT revision forever.

Define an explicit safe policy using Sumi-owned storage only. Requirements:

- never load FPT drawings into SSI;
- allow SSI to initialize and save normally;
- do not silently destroy the old FPT document—preserve it under an identity-specific or backup key if replacing the current key;
- keep current same-session/same-symbol CAS behavior;
- document compatibility/rollback consequences;
- do not change backend contracts or migrate backend legacy drawings.

Required tests:

- mismatched symbol does not load;
- new identity can initialize and save;
- old identity remains recoverable according to the recorded policy;
- same identity stale revision still conflicts;
- existing same-symbol stored documents still load without data loss.

Update UAT only as needed to retain the new pointer-cancel assertion. Do not remove, weaken, rename away, or relabel any existing assertion. All 13 product gaps must remain visible.

## Verification

Run:

```bash
git diff --check
cd frontend && npm test -- --run src/features/drawings
cd frontend && npm run lint
cd frontend && npm run build
./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
shasum -a 256 backend/sumi.db
```

Use the isolated temporary UAT database and retain machine results/screenshots. Production DB SHA-256 must remain `60198b0962edb2e56e91a5107875f4ff5f2b69d9df40126e3194ceaa68195f8d`.

## Prohibited scope

- No Batch 2 or Indicator Manager work.
- No new drawing tool, magnet, community provider, dependency, backend persistence, or legacy migration.
- No cosmetic redesign.
- No broad refactor of the accepted controller/view/provider work.
- No weakening tests or acceptance criteria.

End with `BATCH 1 FINAL CLOSURE COMPLETE — STOPPED AT REVIEWER GATE`, list B1-R10/R11 evidence, exact test/UAT result paths, DB before/after hash, and stop.
