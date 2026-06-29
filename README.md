# Sumi Replay Trading Lab

Sumi is a local-first trading practice and research platform for manual replay, journaling, analytics, scanner workflows and automated backtesting.

## What Sumi V2 Supports

- Manual bar replay with no-future-leak candles and session-scoped indicators.
- Trading decisions, order lifecycle, T+2 constraints, pending limit orders, positions and trades.
- Drawing persistence, journal fields and replay session resume.
- Analytics for equity, drawdown, benchmark, setup/symbol/mistake grouping, trade distribution and outlier impact.
- Safe declarative backtesting without API-controlled `eval()`.
- Sample MA Crossover and MACD RSI Momentum strategies.
- Historical signal scanner with saved scan runs and replay links.
- Strategy Lab comparison, parameter sweep and persisted run history.

## Tech Stack

- Backend: FastAPI, SQLAlchemy, SQLite, Alembic, pandas.
- Frontend: React, TypeScript, Vite, TanStack Query, Zustand, Lightweight Charts.
- Default database: local SQLite.

## Run Locally

Backend:

```powershell
cd backend
.\.venv\Scripts\python.exe -m uvicorn app.main:app --reload --port 8000
```

Frontend:

```powershell
cd frontend
npm.cmd install
npm.cmd run dev
```

Open the Vite URL shown by the frontend dev server.

## Import Data

CafeF batch import:

```powershell
cd backend
.\.venv\Scripts\python.exe scripts\import_batch.py ..\data\raw\cafef_sample
```

Raw data should live under `data/raw/`. Local databases such as `backend/sumi.db` are runtime artifacts and should not be committed.

## Verify Release Gates

Run all V2 gates from the repository root:

```powershell
.\scripts\verify-v2.ps1
```

The script runs backend tests, Alembic upgrade, frontend lint, frontend tests and frontend production build.

## Canonical Docs

Start with:

- `docs/INDEX.md`
- `docs/PROGRESS_V2.md`
- `docs/RELEASE_CHECKLIST_V2.md`
- `docs/ACCEPTANCE_CRITERIA_V2.md`

## Known V2 Limits

- Daily candles are the supported happy path.
- SQLite is the supported V2 database.
- Sector strength and advanced scanner filters remain future scope unless sector metadata is available.
- No multi-user authentication; Sumi is intentionally local-first.
