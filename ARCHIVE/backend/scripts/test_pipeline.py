import sys
import os
from datetime import datetime, timedelta
import pytz

# Add the parent directory to sys.path to allow imports from app
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from app.db import SessionLocal
from app.domain.data_feeds.yahoo_feed import YahooFinanceFeed
from app.domain.data_feeds.binance_feed import BinanceFeed
from app.services.data_service import DataFeedService
from app.models.candle import Candle

def run_test():
    print("=== Testing Data Pipeline ===")
    
    db = SessionLocal()
    try:
        # 1. Test Yahoo Finance Feed (AAPL)
        print("\n1. Testing YahooFinanceFeed (AAPL - 1D)")
        yahoo_feed = YahooFinanceFeed()
        yahoo_service = DataFeedService(db, yahoo_feed)
        
        start_date = datetime.now(pytz.UTC) - timedelta(days=30)
        
        try:
            records_saved = yahoo_service.sync_historical_data("AAPL", "1D", start_date)
            print(f"-> Successfully fetched and saved {records_saved} candles for AAPL.")
        except Exception as e:
            print(f"-> Error fetching Yahoo data: {e}")
            
        # 2. Test Binance Feed (BTCUSDT)
        print("\n2. Testing BinanceFeed (BTCUSDT - 1H)")
        binance_feed = BinanceFeed()
        binance_service = DataFeedService(db, binance_feed)
        
        start_date_binance = datetime.now(pytz.UTC) - timedelta(days=2) # 48 hours
        
        try:
            records_saved = binance_service.sync_historical_data("BTCUSDT", "1H", start_date_binance)
            print(f"-> Successfully fetched and saved {records_saved} candles for BTCUSDT.")
        except Exception as e:
            print(f"-> Error fetching Binance data: {e}")
            
        # 3. Verify Database records
        print("\n3. Verifying Database Records")
        aapl_count = db.query(Candle).filter(Candle.symbol == "AAPL").count()
        btc_count = db.query(Candle).filter(Candle.symbol == "BTCUSDT").count()
        
        print(f"-> Database now contains {aapl_count} AAPL candles.")
        print(f"-> Database now contains {btc_count} BTCUSDT candles.")
        
    finally:
        db.close()

if __name__ == "__main__":
    run_test()
