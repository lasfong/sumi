# Sumi V3 product acceptance criteria

These are release requirements, not aspirational notes. `PASS` requires browser evidence on deterministic demo data and no console/page errors.

## Global quality

| ID | Requirement |
| --- | --- |
| G-01 | Full backend tests, frontend tests, lint, type/build, and deterministic product UAT pass. |
| G-02 | Product UAT uses a temporary database and does not mutate `backend/sumi.db`. |
| G-03 | Successful and failed UAT runs retain machine-readable results and screenshots. |
| G-04 | No P0/P1 issue remains in the released workflow. |
| G-05 | The product remains local-first with no telemetry or external transmission of user trading data. |

## Replay integrity

| ID | Requirement |
| --- | --- |
| R-01 | Future candles are absent from API payloads, chart series, indicators, markers, and derived UI. |
| R-02 | Current symbol, candle date, bar index, and OHLCV context are obvious without using the crosshair. |
| R-03 | Previous/next, ±5, autoplay/pause, speed, and keyboard navigation behave consistently. |
| R-04 | Advancing or rewinding updates chart, positions, orders, markers, and active indicators without duplicates or races. |
| R-05 | Reload/resume restores replay index and complete workspace state. |

## Indicator Manager

| ID | Requirement |
| --- | --- |
| I-01 | Active indicator list is always visible and distinguishes type, parameters, pane, visibility, and color. |
| I-02 | Add flow supports search/category and parameter entry before confirmation. |
| I-03 | Each indicator can be removed with one obvious action. |
| I-04 | Each indicator can be shown/hidden without losing its settings. |
| I-05 | Parameters can be edited and validated from backend registry metadata. |
| I-06 | Multiple instances of the same indicator type with different settings are distinguishable and independently managed. |
| I-07 | Complete indicator state persists and restores after reload/resume. |
| I-08 | Each oscillator pane has a title, legend, values, close/settings/visibility controls, and usable default height. |
| I-09 | Panes can be resized, reordered, or have an explicitly accepted fixed responsive layout. |
| I-10 | MACD clearly renders line, signal, histogram, zero line, names, and non-overlapping current values. |
| I-11 | RSI uses a sensible 0–100 scale and visible 30/50/70 references. |
| I-12 | CCI has visible -100/0/100 references. |
| I-13 | Warmup/null data does not crash, draw invalid segments, or leave misleading empty active state. |

## Drawing system

| ID | Requirement |
| --- | --- |
| D-01 | Toolbar exposes Cursor/Select, Horizontal, Trendline, Ray, Rectangle, Fibonacci Retracement, and Text/Note with labels/tooltips. |
| D-02 | Active drawing mode is visually obvious and Escape/Cursor cancels it predictably. |
| D-03 | Drawings can be selected with clear handles/bounds and deselected without accidental creation. |
| D-04 | Selected drawings can be moved and anchor points edited. |
| D-05 | Selected drawings can be deleted by UI and keyboard; Clear All requires confirmation or undo. |
| D-06 | Undo/redo covers create, move, edit, and delete. |
| D-07 | Required tools remain attached correctly through pan, zoom, pane resize, replay advance, and reload. |
| D-08 | Drawing state is versioned, persisted, and restored without provider-specific data loss. |
| D-09 | Fibonacci shows usable levels/labels and supports direction/editing. |
| D-10 | Magnet/snapping behavior is predictable and can be disabled or constrained. |
| D-11 | No duplicate-time, recursion, stale-listener, or unmount errors occur. |

## Trading practice workflow

| ID | Requirement |
| --- | --- |
| T-01 | Chart remains the primary workspace; trade panels do not reduce it below an accepted usable size. |
| T-02 | Buy/sell/hold/skip, pending orders, position, and T+2 feedback are understandable in context. |
| T-03 | Decision journal and checklist are reachable without losing chart context or replay state. |
| T-04 | Trade markers and current position state remain synchronized through replay navigation and reload. |
| T-05 | A user can complete a 30-minute practice session without encountering a missing core action or runtime error. |

## Required viewport evidence

- Primary desktop: 1440×1000.
- Compact laptop: 1280×800.
- Minimum supported desktop width must be stated and enforced.
- Mobile may remain read-only/limited for V3, but the limitation must be explicit; it must not masquerade as full trading functionality.

## Release language

The phrase “professional manual replay and TA practice product” may be used only when all G/R/I/D/T criteria in the released scope pass. The phrase “TradingView-like” remains prohibited unless a future decision defines and verifies a broader parity contract.
