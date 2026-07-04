"""Create deterministic local demo data without external market-data services."""

from datetime import datetime, timedelta, timezone
from math import sin
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from app.db import Base, SessionLocal, engine
import app.models  # noqa: F401 - register the complete application schema
from app.models.candle import Candle
from app.models.symbol import Symbol


DEMO_SYMBOLS = [
    {
        "symbol": "FPT",
        "company_name": "FPT Corporation (Demo)",
        "sector": "Technology",
        "industry": "Software",
        "base_price": 80_000.0,
        "trend": 42,
        "phase": 0.0,
    },
    {
        "symbol": "SSI",
        "company_name": "SSI Securities Corporation (Demo)",
        "sector": "Financials",
        "industry": "Securities",
        "base_price": 28_000.0,
        "trend": 18,
        "phase": 1.3,
    },
    {
        "symbol": "VCI",
        "company_name": "Vietcap Securities (Demo)",
        "sector": "Financials",
        "industry": "Securities",
        "base_price": 36_000.0,
        "trend": 24,
        "phase": 2.1,
    },
    {
        "symbol": "VNINDEX",
        "company_name": "VNINDEX Benchmark (Demo)",
        "sector": "Index",
        "industry": "Benchmark",
        "base_price": 1_050.0,
        "trend": 0.45,
        "phase": 0.7,
        "asset_type": "index",
    },
]


def seed_symbol(config: dict, sessions: int = 520) -> int:
    symbol = config["symbol"]
    Base.metadata.create_all(bind=engine)
    db = SessionLocal()
    try:
        existing = db.query(Candle).filter(
            Candle.symbol == symbol,
            Candle.timeframe == "1D",
            Candle.adjustment_type == "unadjusted",
        ).count()
        if existing >= sessions:
            return existing

        db.merge(Symbol(
            symbol=symbol,
            exchange="HOSE",
            company_name=config["company_name"],
            sector=config["sector"],
            industry=config["industry"],
            asset_type=config.get("asset_type", "stock"),
            is_active=True,
        ))

        db.query(Candle).filter(Candle.symbol == symbol, Candle.timeframe == "1D").delete(synchronize_session=False)

        day = datetime(2023, 1, 2, tzinfo=timezone.utc)
        base_price = float(config["base_price"])
        trend_step = float(config["trend"])
        phase = float(config["phase"])
        previous_close = base_price
        rows = []
        index = 0
        while len(rows) < sessions:
            if day.weekday() < 5:
                amplitude = max(base_price * 0.045, 120.0)
                trend = index * trend_step
                cycle = sin(index / 9 + phase) * amplitude + sin(index / 31 + phase) * amplitude * 1.6
                close = round(base_price + trend + cycle, 2)
                open_price = round(previous_close + sin(index / 4 + phase) * amplitude * 0.2, 2)
                high = round(max(open_price, close) + amplitude * 0.35 + abs(sin(index + phase)) * amplitude * 0.25, 2)
                low = round(min(open_price, close) - amplitude * 0.35 - abs(sin(index / 2 + phase)) * amplitude * 0.2, 2)
                rows.append(Candle(
                    symbol=symbol,
                    timeframe="1D",
                    timestamp=day,
                    open=open_price,
                    high=high,
                    low=low,
                    close=close,
                    volume=1_000_000 + (index % 30) * 45_000,
                    source="sumi_demo",
                    adjustment_type="unadjusted",
                ))
                previous_close = close
                index += 1
            day += timedelta(days=1)

        db.add_all(rows)
        db.commit()
        return len(rows)
    finally:
        db.close()


def seed_demo(sessions: int = 520) -> dict[str, int]:
    return {config["symbol"]: seed_symbol(config, sessions) for config in DEMO_SYMBOLS}


if __name__ == "__main__":
    counts = seed_demo()
    total = sum(counts.values())
    detail = ", ".join(f"{symbol}={count}" for symbol, count in counts.items())
    print(f"Seeded {total} deterministic daily candles ({detail}).")
