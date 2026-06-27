from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from app.api import health, import_data, replay, decisions, analytics, symbols, journal
from app.db import Base, engine

import app.models  # ensure models are registered

# Database tables are managed by alembic migrations for production
# But for local SQLite testing, create them automatically:
Base.metadata.create_all(bind=engine)

app = FastAPI(
    title="Sumi - VN Replay Trading Lab API",
    description="API for local-first trading replay and journaling",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(health.router, prefix="/api")
app.include_router(import_data.router, prefix="/api/import", tags=["import"])
app.include_router(replay.router, prefix="/api/replay", tags=["replay"])
app.include_router(decisions.router, prefix="/api/replay", tags=["decisions"])
app.include_router(analytics.router, prefix="/api/replay", tags=["analytics"])
app.include_router(symbols.router, prefix="/api", tags=["symbols"])
app.include_router(journal.router, prefix="/api/replay", tags=["journal"])

from app.api import indicators, ws_replay
app.include_router(indicators.router, prefix="/api/indicators", tags=["indicators"])
app.include_router(ws_replay.router, prefix="/api", tags=["websocket"])
