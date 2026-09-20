# DEV P1R-03A — Incremental scope inventory

Status: **DEV COMPLETED; REVIEW REQUIRES P1R-03B CLOSURE**. Historical evidence only; do not rerun.

Read `AGENTS.md`, `docs/exec-plans/P0_BASELINE_FREEZE.md`, and the latest addendum in `docs/reviews/P1_SIGNAL_VOLUME_SPIKE_REVIEW_2026-09-12.md`.

## Permission boundary

You may create only `docs/reviews/P1R_03_SCOPE_INVENTORY.md`. Temporary reconstruction is allowed only under git-ignored `scratch/p1r03-*` and must be removed afterward. Do not modify application/test/UAT files, other docs, the Git index, commits, branches, database, or baseline artifacts. Do not run servers or UAT.

## Task

Compare the current checkout with the exact frozen working-tree state in `scratch/baseline-freeze-20260912-200500/`, not merely with `HEAD`.

Record:

1. Current branch/HEAD and staged/unstaged/untracked counts. Confirm whether the staged set remains exactly the original 159 deletions.
2. Verify all 93 paths in `sha256-manifest.txt`: unchanged, changed after freeze, or missing. List every new untracked path after freeze.
3. For each changed baseline path, identify the exact post-freeze hunks and classify them as: authorized Phase 1 integration, reviewer/documentation change, or out-of-scope drift. Use the frozen unstaged patch to reconstruct tracked baseline content and the path-preserving ZIP for baseline-untracked content.
4. Audit the incremental Phase 1 changes in `backend/app/main.py`, `frontend/src/components/replay/ReplayWorkspace.tsx`, and `scripts/product-uat.mjs`. For `ReplayWorkspace`, separate the approved SignalInspector import/mount/currentTimestamp from any other post-freeze edits. For product UAT, identify Phase 1 assertions separately from any unrelated changed assertions.
5. List the 15 Phase 1 production/test files created after freeze and flag any extra new code file.
6. Recheck production DB hash and WAL/SHM without opening SQLite.

For every drift, give a reviewer recommendation: `KEEP_AS_P1`, `KEEP_SEPARATE`, or `RESTORE_TO_FROZEN_BASELINE`. Do not execute the recommendation. Include commands/evidence sufficient for independent reproduction and state any uncertainty explicitly.

End exactly:

`P1R-03A INVENTORY DONE; NO FILES CLEANED; PHASE 2 NOT STARTED`
