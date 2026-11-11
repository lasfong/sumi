from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from starlette.exceptions import HTTPException as StarletteHTTPException
from app.api import health, import_data, replay, decisions, analytics, symbols, journal
from app.db import Base, engine

from app.config import settings, verify_database_path_security
import app.models  # ensure models are registered
from contextlib import asynccontextmanager


_HTTP_STATUS_CODES = {
    400: "BAD_REQUEST",
    401: "UNAUTHORIZED",
    403: "FORBIDDEN",
    404: "NOT_FOUND",
    405: "METHOD_NOT_ALLOWED",
    409: "CONFLICT",
    422: "VALIDATION_ERROR",
    500: "INTERNAL_ERROR",
}


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Guard against insecure or unanchored database paths
    verify_database_path_security(str(engine.url))
    if settings.AUTO_CREATE_TABLES:
        Base.metadata.create_all(bind=engine)
    yield


app = FastAPI(
    title="Sumi - VN Replay Trading Lab API",
    description="API for local-first trading replay and journaling",
    version=settings.VERSION,
    lifespan=lifespan,
)


@app.exception_handler(RequestValidationError)
async def validation_exception_handler(request: Request, exc: RequestValidationError) -> JSONResponse:
    field_errors = []
    messages = []
    for err in exc.errors():
        loc_parts = [str(part) for part in err.get("loc", []) if part != "body"]
        field_name = ".".join(loc_parts) if loc_parts else "body"
        msg = str(err.get("msg", "Validation error"))
        err_type = str(err.get("type", "validation_error"))
        field_errors.append({
            "field": field_name,
            "message": msg,
            "type": err_type,
        })
        messages.append(f"{field_name}: {msg}" if field_name and field_name != "body" else msg)

    summary_message = "; ".join(messages) if messages else "Request validation failed"
    return JSONResponse(
        status_code=422,
        content={
            "code": "VALIDATION_ERROR",
            "message": summary_message,
            "field_errors": field_errors,
            "detail": summary_message,
        },
    )


@app.exception_handler(StarletteHTTPException)
async def http_exception_handler(request: Request, exc: StarletteHTTPException) -> JSONResponse:
    default_code = _HTTP_STATUS_CODES.get(exc.status_code, f"HTTP_{exc.status_code}")
    raw_detail = exc.detail
    field_errors = []

    if isinstance(raw_detail, dict):
        code = str(raw_detail.get("error_code") or raw_detail.get("code") or default_code)
        message = str(raw_detail.get("message") or raw_detail.get("detail") or default_code)
        if isinstance(raw_detail.get("field_errors"), list):
            field_errors = raw_detail["field_errors"]
        return JSONResponse(
            status_code=exc.status_code,
            content={
                "code": code,
                "message": message,
                "field_errors": field_errors,
                "detail": raw_detail,
            },
            headers=exc.headers,
        )

    if isinstance(raw_detail, list):
        messages = []
        for item in raw_detail:
            if isinstance(item, dict):
                loc_parts = [str(part) for part in item.get("loc", []) if part != "body"]
                field_name = ".".join(loc_parts) if loc_parts else "body"
                msg = str(item.get("msg") or item.get("message") or str(item))
                field_errors.append({
                    "field": field_name,
                    "message": msg,
                    "type": str(item.get("type", "error")),
                })
                messages.append(f"{field_name}: {msg}" if field_name != "body" else msg)
            else:
                messages.append(str(item))
        message = "; ".join(messages) if messages else default_code
    else:
        message = str(raw_detail) if raw_detail is not None else default_code

    return JSONResponse(
        status_code=exc.status_code,
        content={
            "code": default_code,
            "message": message,
            "field_errors": field_errors,
            "detail": message,
        },
        headers=exc.headers,
    )


@app.exception_handler(Exception)
async def unhandled_exception_handler(request: Request, exc: Exception) -> JSONResponse:
    return JSONResponse(
        status_code=500,
        content={
            "code": "INTERNAL_ERROR",
            "message": "Internal server error",
            "field_errors": [],
            "detail": "Internal server error",
        },
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
app.include_router(decisions.router, prefix="/api", tags=["decisions-root"])
app.include_router(analytics.router, prefix="/api/replay", tags=["analytics"])
app.include_router(symbols.router, prefix="/api", tags=["symbols"])
app.include_router(journal.router, prefix="/api/replay", tags=["journal"])

from app.api import indicators, ws_replay, backtest, scanner, strategy_lab, sync, signals, bb, universe
app.include_router(indicators.router, prefix="/api/indicators", tags=["indicators"])
app.include_router(ws_replay.router, prefix="/api", tags=["websocket"])
app.include_router(backtest.router, prefix="/api/backtest", tags=["backtest"])
app.include_router(scanner.router, prefix="/api/scanner", tags=["scanner"])
app.include_router(strategy_lab.router, prefix="/api/strategy-lab", tags=["strategy-lab"])
app.include_router(sync.router, prefix="/api/sync", tags=["sync"])
app.include_router(signals.router, prefix="/api/signals", tags=["signals"])
app.include_router(bb.router, prefix="/api/bb", tags=["money-flow"])
app.include_router(universe.router)
