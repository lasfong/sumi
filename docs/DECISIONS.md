# Technical Decisions

This document records product and architecture decisions for Sumi.

## Decision 0: Canonical V2 Spec Set

- **Decision**: The active implementation contract is the V2 documentation set: `PRODUCT_STRATEGY_V2.md`, `SPEC_V2.md`, `MANUAL_REPLAY_SPEC.md`, `BACKTEST_ENGINE_SPEC.md`, `ACCEPTANCE_CRITERIA_V2.md`, and `ROADMAP_TO_COMPLETION.md`.
- **Reason**: The pre-V2 specs and sprint plans contained outdated priorities and unsafe assumptions, including accepting `eval()` in backtest MVP.
- **Implication**: Pre-V2 documents in `docs/archive/pre_v2/` are historical context only.

## Decision 1: Database Choice

- **Decision**: Keep SQLite as the default local-first database, abstracted via SQLAlchemy.
- **Reason**: Sumi is currently a personal local-first trading lab. SQLite is simpler for the target user and sufficient for MVP.
- **Implication**: PostgreSQL/TimescaleDB remains optional future scope. Do not add it until SQLite MVP is correct and a separate migration path exists.

## Decision 2: Manual Replay Is The Core Product

- **Decision**: Manual replay is the primary product surface. Automated backtest validates methods learned/practiced through replay.
- **Reason**: The product goal is learning technical analysis and improving decisions, not merely computing strategy PnL.
- **Implication**: Replay correctness, no-future-leak, journal quality, and trade lifecycle correctness outrank new backtest features.

## Decision 3: No Future Data Leak

- **Decision**: Replay candles and replay indicators must be session-scoped and limited by `current_index`.
- **Reason**: Future data leakage invalidates both learning and manual backtest results.
- **Implication**: Frontend filtering is only defensive. The backend must never send future replay data.

## Decision 4: Shared Broker And Accounting

- **Decision**: Manual replay and automated backtest must share broker/accounting rules.
- **Reason**: Separate PnL/execution logic will drift and produce conflicting results.
- **Implication**: T+2, no shorting, fee, tax, price limits, pending orders, executions, positions, and trades must be implemented in one shared path or a clearly shared domain service.

## Decision 5: Safe Strategy Model

- **Decision**: Backtest MVP must use a safe declarative rule model, not Python `eval()` over API-controlled strings.
- **Reason**: The app is local-first and may run on a user's personal machine. `eval()` is an unacceptable code execution risk.
- **Implication**: Strategy rules must be parsed through a whitelist/AST/DSL. User Python code is future trusted-local scope only with subprocess timeout and clear limitations.

## Decision 6: Analytics Contract

- **Decision**: Analytics equity points use the V2 contract: `timestamp`, `equity`, `cash`, `holdings_value`, `drawdown`, and `drawdown_pct`.
- **Reason**: Backend and frontend currently disagree on fields, causing chart/runtime risk.
- **Implication**: Backend schemas, services, frontend types, and chart components must match this contract.

## Decision 7: Data And POC Artifacts

- **Decision**: Raw data, local databases, generated caches, and research clones are not canonical documentation.
- **Reason**: They make the repo noisy and confuse handoff state.
- **Implication**: Raw data belongs under `data/raw/` locally. Research clones belong under `research_repos/`. Runtime databases and caches must not be committed.
