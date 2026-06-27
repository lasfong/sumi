from sqlalchemy.orm import Session
from datetime import datetime
from typing import Optional

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.domain.data_feeds.base import BaseFeed

class DataFeedService:
    """
    Service responsible for orchestrating data feeds and database persistence.
    """
    def __init__(self, db: Session, feed: BaseFeed):
        self.db = db
        self.feed = feed

    def sync_historical_data(
        self, 
        symbol_name: str, 
        timeframe: str, 
        start_date: datetime, 
        end_date: Optional[datetime] = None
    ):
        """
        Fetch historical data from feed and save to DB.
        """
        # Ensure symbol exists
        symbol = self.db.query(Symbol).filter(Symbol.symbol == symbol_name).first()
        if not symbol:
            symbol = Symbol(symbol=symbol_name, is_active=True)
            self.db.add(symbol)
            self.db.commit()

        # Fetch data
        df = self.feed.fetch_historical_data(symbol_name, timeframe, start_date, end_date)
        
        if df.empty:
            return 0

        # In a real production system, we should use bulk insert (e.g. pg_bulkload, or pandas to_sql)
        # Here we use sqlalchemy core bulk insert for better performance than ORM
        
        records = df.to_dict('records')
        
        # We need to map dataframe columns to Candle model columns
        candle_objects = []
        for row in records:
            candle_objects.append({
                "symbol": symbol_name,
                "timeframe": timeframe,
                "timestamp": row['timestamp'],
                "open": float(row['open']),
                "high": float(row['high']),
                "low": float(row['low']),
                "close": float(row['close']),
                "volume": float(row['volume']),
                "source": self.feed.__class__.__name__,
                "adjustment_type": "unadjusted"
            })
            
        # Optional: Handle duplicates (UPSERT).
        # For simplicity, we just do a bulk insert and assume we aren't overlapping.
        # TimescaleDB hypertable allows upserts using ON CONFLICT. For local testing we fallback to sqlite.
        from sqlalchemy.dialects.sqlite import insert
        
        stmt = insert(Candle).values(candle_objects)
        # On conflict do nothing for now
        stmt = stmt.on_conflict_do_nothing(
            index_elements=['symbol', 'timeframe', 'timestamp', 'adjustment_type']
        )
        
        self.db.execute(stmt)
        self.db.commit()
        
        return len(candle_objects)
