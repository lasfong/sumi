from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from app.api import health, import_data, replay, decisions, analytics, symbols, journal
from app.db import Base, engine

from app.config import settings
import app.models  # ensure models are registered
from contextlib import asynccontextmanager


@asynccontextmanager
async def lifespan(app: FastAPI):
    if settings.AUTO_CREATE_TABLES:
        Base.metadata.create_all(bind=engine)
    yield


app = FastAPI(
    title="Sumi - VN Replay Trading Lab API",
    description="API for local-first trading replay and journaling",
    version="0.1.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_allowed_origins_list,
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

from app.api import indicators, ws_replay, backtest
app.include_router(indicators.router, prefix="/api/indicators", tags=["indicators"])
app.include_router(ws_replay.router, prefix="/api", tags=["websocket"])
app.include_router(backtest.router, prefix="/api/backtest", tags=["backtest"])
