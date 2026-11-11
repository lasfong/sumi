from sqlalchemy.orm import Session
from sqlalchemy import func
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.schemas.readiness_schema import DataReadinessResponse


class ReadinessService:
    @staticmethod
    def get_readiness(db: Session) -> DataReadinessResponse:
        total_candles = db.query(func.count(Candle.id)).scalar() or 0
        symbols_count = db.query(func.count(Symbol.symbol)).scalar() or 0

        raw_symbols_with_candles = (
            db.query(Candle.symbol)
            .distinct()
            .order_by(Candle.symbol)
            .all()
        )
        symbols_with_candles = [r[0] for r in raw_symbols_with_candles if r[0]]

        raw_timeframes = (
            db.query(Candle.timeframe)
            .distinct()
            .order_by(Candle.timeframe)
            .all()
        )
        timeframes = [r[0] for r in raw_timeframes if r[0]]

        earliest_raw = db.query(func.min(Candle.timestamp)).scalar()
        latest_raw = db.query(func.max(Candle.timestamp)).scalar()

        earliest_timestamp = str(earliest_raw)[:10] if earliest_raw else None
        latest_timestamp = str(latest_raw)[:10] if latest_raw else None

        if total_candles == 0 or len(symbols_with_candles) == 0:
            status = 'empty'
        elif symbols_count > 0 and len(symbols_with_candles) < symbols_count:
            status = 'partial'
        else:
            status = 'ready'

        return DataReadinessResponse(
            status=status,
            symbols_count=symbols_count,
            symbols_with_candles=symbols_with_candles,
            timeframes=timeframes,
            total_candles=total_candles,
            earliest_timestamp=earliest_timestamp,
            latest_timestamp=latest_timestamp,
        )
