# ADR-001 — Controlled Replay/Chart UI rebuild

- Status: Accepted for V3 planning
- Date: 2026-07-15
- Decision owner: Product reviewer/orchestrator

## Context

The RC2 implementation uses Lightweight Charts v5 correctly at a basic API level, and the backend replay/indicator/backtest foundations are real. The user-facing chart workspace is nevertheless prototype-quality: `ReplayPage.tsx` owns too many concerns, indicator lifecycle UI is absent, and custom drawing tools are ordinary series/price-line approximations without a professional interaction model.

Incrementally adding controls to the current surface (Option B as originally stated) would preserve weak ownership boundaries. Rebuilding the entire project or changing chart libraries (broad Option C) would discard working backend and rendering foundations without evidence that Lightweight Charts is the blocker.

## Decision

Choose a **controlled Option C for the Replay/Chart frontend**, while retaining the stable system foundations:

### Keep

- Lightweight Charts v5 as the base renderer.
- Backend replay/session APIs and no-future-leak model.
- `IndicatorEngine` and `StrategyIndicatorAdapter` as authoritative calculation path.
- Trade lifecycle, scanner evaluator, backtest cleanup, and analytics contracts unless a batch proves a specific defect.
- Existing persisted sessions through explicit compatibility/migration layers.

### Rebuild or replace

- Replay workspace composition and responsive information hierarchy.
- Indicator Manager and pane chrome/legends/settings.
- Drawing interaction subsystem using a real provider behind a Sumi-owned adapter.
- Chart state ownership, persistence contracts, and browser acceptance harness.

### Refactor behind contracts

- Pane and series management.
- Indicator render definitions.
- Replay orchestration currently embedded in `ReplayPage.tsx`.

## Drawing provider decision gate

Run a separate spike before adopting a provider:

1. Evaluate `deepentropy/lightweight-charts-drawing` first.
2. Benchmark `difurious/lightweight-charts-line-tools-core` plus required companion tools.
3. Test only the required V3 tool set initially: cursor/select, horizontal, trendline, ray, rectangle, Fibonacci retracement, text/note.
4. Reject a provider if it cannot reliably select, move, edit, delete, serialize, restore, pan/zoom, clean up React listeners, and operate without runtime errors on Sumi data.
5. Review license obligations and pin an exact revision/version.

If both providers fail, do not immediately switch chart libraries. First estimate a Sumi primitive-plugin implementation using official Lightweight Charts primitives. Reconsider the base chart library only if that estimate or a verified platform limitation makes the V3 acceptance criteria infeasible.

## Consequences

- More frontend code will be replaced than in Option B, but backend risk stays bounded.
- V3 delivery will proceed as vertical batches with explicit compatibility seams.
- The current UI remains a reference/baseline, not the target architecture.
- “Done” is determined by product UAT and acceptance IDs, not file presence or passing unit tests.
