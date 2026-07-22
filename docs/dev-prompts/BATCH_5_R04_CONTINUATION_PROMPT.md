# Batch 5 R04 correction and final canonical closure — standalone DEV prompt

You are the DEV task continuing the stopped Batch 5 review closure. V3 remains unapproved. Read completely, in repository-mandated order, `AGENTS.md`, its six canonical sources, `docs/dev-prompts/BATCH_5_PRODUCT_HARDENING_V3_RC_PROMPT.md`, `docs/exec-plans/BATCH_5_PRODUCT_HARDENING_V3_RC.md`, `docs/reviews/BATCH_5_REVIEW_2026-07-19.md`, `docs/dev-prompts/BATCH_5_REVIEW_CLOSURE_PROMPT.md`, and the four V3 release/evidence documents. The R04 addendum at the end of the review is the controlling continuation decision.

Preserve the shared dirty checkout. Do not create or switch a branch/worktree; stage, commit, push, merge, reset, clean, checkout or tag; mutate `backend/sumi.db`; add a dependency; transmit data externally; weaken acceptance; or start post-V3 work. Append this continuation scope, method, rollback and commands to the Batch 5 ExecPlan before further code changes.

## Reviewer classification

The stopped artifact is valid red evidence, but it does not authorize a responsive redesign. Its inline `style.zoom = '2'` retains a 1440 CSS-pixel media-query viewport and does not exercise the existing 768px reflow. Its 120-Tab Journal search contradicts the intentional ARIA roving-tabindex pattern. The current focus CSS also postdates the failing result and remains unverified.

R01 is focused-green. R02, R03 and R05 are implemented but require canonical sustained evidence. The 19-case negative self-test is independently green. Do not redo those implementations unless a focused or final gate demonstrates a concrete defect.

## B5-R04 required correction

1. Remove `document.documentElement.style.zoom` from the accessibility verdict.
2. Represent a 1440×1000 physical reference at 200% with an effective 720×500 CSS layout viewport. Record the exact method, `window.innerWidth`, `window.innerHeight`, device-pixel ratio and `matchMedia('(max-width: 768px)').matches`. If a device-scale-two context is used to retain a 1440×1000 bitmap, state clearly that the half-size CSS viewport drives reflow and device scale only controls output density.
3. Starting from body/background, Tab through the real sequential order until the currently selected Trade tab receives focus. Use ArrowRight to focus/select Journal under the tablist contract. Assert the Journal tab and panel are active, keyboard operable, and intersect the viewport after normal browser scrolling. Assert Replay reachability separately. Do not search directly for the roving `tabIndex=-1` Journal tab by repeated Tab.
4. Record document/body/app bounding boxes, scroll width/client width, both axes and the three control/panel boxes. Vertical reflow and vertical scrolling are allowed. Fail on application-wide horizontal clipping or a core panel that cannot be brought into view by its valid keyboard path.
5. Compare unfocused and keyboard-focused computed outline/box-shadow for representative Replay, PracticeRail tab and dialog/opener controls. Composite ancestor backgrounds correctly, treating `transparent` as alpha zero. Require a changed, visible indicator with at least 3:1 contrast against the adjacent rendered background. Retain a focused-control screenshot.
6. Preserve the 1180px full-workstation threshold and sub-1180 limited-workstation behavior. Trade actions may remain disabled with their explanation, but Trade and Journal navigation/content must remain accessible.

First run this as a focused browser check. If the correct 720×500 method passes with the current layout, make no responsive product change. If it reveals a real defect, a narrow CSS containment/reflow correction inside the existing small-viewport breakpoint is authorized. No navigation redesign, feature work, architecture/provider change, desktop-layout change, new dependency or acceptance reinterpretation is authorized. Stop with evidence if that boundary is insufficient.

## Final evidence closure

After focused R04 is green:

- rerun frontend focused/full tests, backend full tests, lint, production build, `git diff --check`, `verify-v2.sh`, standalone product UAT and `verify-product.sh`;
- run `node scripts/batch5-evidence-negative-selftest.mjs` and require all 19 cases to pass;
- run the complete `./scripts/run-batch5-hardening.sh` for at least 1,800 real elapsed seconds;
- require all R02 future-boundary fields at every checkpoint, R03 fail-closed audit/manifest, and R05 category/window/long-history/ten-remount evidence;
- preserve the 272 canonical baseline IDs exactly once and add exactly five unique blocking closure IDs, `batch5.closure.R01.keyboard-isolation` through `batch5.closure.R05.complete-sustained-scope`. The final canonical product result must therefore contain 277 unique passing checks, zero failed/blocking checks and empty unapproved error arrays;
- review new 1440×1000, 1280×800, effective-200%, focused-control, long-history stress and restored-workspace images;
- repoint `docs/V3_ACCEPTANCE_MATRIX.md`, `docs/V3_RELEASE_CANDIDATE_NOTES.md`, `docs/V3_VERIFICATION_AND_RECOVERY.md` and `docs/V3_EVIDENCE_INDEX.md` only to the new green canonical bundle, retaining prior runs as historical evidence.

Report exact counts, baseline comparison, five closure IDs, 30-minute duration/category/window coverage, future-surface samples, performance budgets, 19 negative cases, recovery equality, error arrays, DB hash, HEAD/tag and dirty-scope audit. Do not self-approve, tag, commit, push, or use product-complete/release-approved language. Stop at Reviewer gate.
