import os
from datetime import datetime, timezone
import pytz
from app.db import SessionLocal, Base, engine
from app.services.data_service import DataFeedService
from app.domain.data_feeds.yahoo_feed import YahooFinanceFeed

def seed_data():
    # Make sure tables exist
    Base.metadata.create_all(bind=engine)
    db = SessionLocal()
    
    feed = YahooFinanceFeed()
    service = DataFeedService(db, feed)
    
    symbols = ['FPT.VN', 'VCB.VN', 'AAPL']
    start_date = datetime(2020, 1, 1, tzinfo=pytz.UTC)
    end_date = datetime.now(pytz.UTC)
    
    print("Fetching historical data from Yahoo Finance...")
    for symbol in symbols:
        try:
            print(f"Syncing {symbol}...")
            count = service.sync_historical_data(symbol, '1D', start_date, end_date)
            print(f"Imported {count} candles for {symbol}")
        except Exception as e:
            print(f"Failed to sync {symbol}: {e}")
            
    db.close()
    print("Seed complete! You can now test replay sessions with: ", symbols)

if __name__ == "__main__":
    seed_data()
