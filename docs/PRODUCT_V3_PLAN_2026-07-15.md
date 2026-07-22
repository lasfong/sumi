# Sumi V3 product development plan

## Product outcome

Turn Sumi into a dependable local-first manual replay and backtesting workstation for professional personal technical-analysis practice on Vietnam market data. The V3 objective is not feature-count parity with TradingView; it is a coherent workflow in which replay, indicators, drawings, trades, and journaling work together without friction or runtime errors.

## Chosen strategy

Use the controlled frontend rebuild defined by ADR-001:

- rebuild Replay/Chart UX and state ownership;
- keep Lightweight Charts v5;
- keep backend engines and data contracts unless a verified defect requires a scoped change;
- replace the custom drawing interaction layer through a provider adapter;
- make product UAT the release authority.

This is closer to Option C for the frontend, but it is not a whole-project rewrite.

## Success measures

1. All acceptance IDs in `PRODUCT_ACCEPTANCE_CRITERIA_V3.md` pass for released scope.
2. A 30-minute FPT replay practice session can be completed without fighting indicator/drawing controls.
3. Browser UAT retains visual evidence and fails on runtime errors or missing interaction contracts.
4. No-future-leak and backend indicator parity remain green.
5. Existing replay sessions remain readable or receive an explicit migration path.

## Delivery batches

### Batch 0 — Development foundation and provider decision

Outcome: deterministic harness, canonical instructions, provider spike, and approved target design.

- Establish root `AGENTS.md`, ExecPlan standard, acceptance IDs, isolated product UAT, and artifact retention.
- Inventory stale/duplicate docs; mark canonical V3 sources without deleting historical evidence.
- Spike deepentropy drawing provider against D-01 through D-11 required subset.
- Benchmark difurious only where the first provider fails or creates unacceptable risk.
- Record exact version/revision, license, bundle impact, lifecycle behavior, and persistence mapping.
- Produce an approved component/state architecture for subsequent batches.

Exit: provider decision accepted; harness baseline failures documented; no feature claim.

### Batch 1 — Replay workspace foundation and chart provider boundary

Outcome: new Replay workspace shell renders deterministic candles/volume with stable state ownership and compatibility seams.

- Extract replay application controller/state from `ReplayPage`.
- Introduce chart workspace facade and provider interfaces.
- Preserve markers, websocket updates, no-future-leak behavior, and resume.
- Establish pane chrome/layout primitives and responsive desktop geometry.
- Integrate selected drawing provider for one end-to-end tool only, including persistence adapter.

Primary acceptance: R-01–R-05, G-01–G-03, initial D-08/D-11.

### Batch 2 — Professional Indicator Manager

Outcome: complete add/manage/settings/persistence workflow for EMA, RSI, MACD, CCI, and Volume.

- Explicit indicator domain state and stable IDs.
- Add/search/configure flow driven by backend registry.
- Active list, individual remove, visibility, settings, multiple instances, order.
- Pane title/legend/controls, reference lines, scale policies, responsive sizing.
- Request cancellation/deduplication and reload/resume restoration.

Primary acceptance: I-01–I-13.

### Batch 3 — Professional drawing MVP

Outcome: reliable lifecycle for the required TA tools.

- Cursor/select, horizontal, trendline, ray, rectangle, Fibonacci, text/note.
- Select/move/edit/delete, keyboard lifecycle, cancel, magnet, undo/redo.
- Versioned persistence and migration from current four-type schema.
- Pan/zoom/replay/reload/unmount stability.

Primary acceptance: D-01–D-11.

### Batch 4 — Integrated trading-practice workflow

Outcome: chart, trade controls, position/order state, journal, and checklist form one coherent workspace.

- Rebalance chart/sidebar/header information hierarchy.
- Make current date/context obvious.
- Keep journal/checklist accessible without navigation loss.
- Verify markers, T+2 feedback, pending orders, and replay state synchronization.
- Compact-laptop layout and explicit mobile limitation.

Primary acceptance: T-01–T-05 plus R-02/R-04.

### Batch 5 — Product hardening and V3 release candidate

Outcome: evidence-backed release candidate suitable for sustained personal use.

- 30-minute practice-session UAT and regression scenarios.
- Performance/memory checks with long histories and many drawings.
- Migration/backup/restore verification.
- Accessibility and keyboard pass for core workflow.
- Documentation cleanup, release notes, known limits, and rollback instructions.

Exit: all released acceptance IDs green; reviewer signs off browser evidence.

## Architecture target

```text
Replay route/page
  -> Replay application controller
      -> replay API/query adapters
      -> workspace state + persistence
      -> trade/journal state
  -> Replay workspace UI
      -> ChartWorkspace facade
          -> Lightweight Charts provider
          -> Indicator renderer definitions
          -> Drawing provider adapter
      -> Indicator Manager
      -> Drawing toolbar/context controls
      -> Trade and journal panels
```

Backend indicator values remain authoritative. Frontend render definitions own display semantics such as titles, colors, scales, formatters, and reference lines.

## Documentation policy

Canonical V3 docs are this plan, V3 acceptance criteria, ADR-001, the operating model, and active ExecPlans. V2 release documents remain evidence of the old baseline and must not be edited to imply V3 completion.

## Major risks

| Risk | Mitigation |
| --- | --- |
| Community drawing provider is immature | Time-boxed spike, exact revision pin, Sumi contract suite, adapter isolation. |
| Rebuild repeats superficial completion | Acceptance IDs and browser artifacts are release authority. |
| Scope expands into full TradingView clone | Fixed V3 required tool/indicator set and explicit out-of-scope list per batch. |
| Existing sessions break | Versioned workspace/drawing schema, migration fixtures, backup/restore tests. |
| Reviewer and DEV conflict | Separate tasks, shared-checkout one-writer rule; use worktree only when explicitly requested. |
| Backend regressions during frontend work | Preserve API contracts; full backend suite in every product gate. |

## Immediate next action

Start a dedicated local DEV task for **Batch 0 only** on the current checkout and branch. It must read/update `docs/exec-plans/BATCH_0_FOUNDATION_AND_DRAWING_SPIKE.md`, run the current red product UAT baseline, then perform the provider spike. It must not create a branch/worktree or begin the Indicator Manager/Replay UI implementation until the reviewer accepts the provider and architecture decision evidence.
