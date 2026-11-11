import sys
import os
from datetime import datetime, timedelta
import pytz

# Add the parent directory to sys.path to allow imports from app
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from app.domain.data_feeds.yahoo_feed import YahooFinanceFeed
from app.domain.data_feeds.binance_feed import BinanceFeed

def test_feeds():
    print("=== Testing YahooFinanceFeed ===")
    yahoo_feed = YahooFinanceFeed()
    start_date = datetime.now(pytz.UTC) - timedelta(days=5)
    
    try:
        df = yahoo_feed.fetch_historical_data("AAPL", "1D", start_date)
        print(f"Yahoo AAPL 1D Data Shape: {df.shape}")
        if not df.empty:
            print(df.head(2))
        else:
            print("Warning: DataFrame is empty!")
            
        realtime = yahoo_feed.fetch_realtime_data("AAPL")
        print(f"Realtime: {realtime['price']}")
        print("Yahoo Feed OK.\n")
    except Exception as e:
        print(f"Error testing Yahoo Feed: {e}")

    print("=== Testing BinanceFeed ===")
    binance_feed = BinanceFeed()
    start_date_binance = datetime.now(pytz.UTC) - timedelta(days=2)
    
    try:
        df_btc = binance_feed.fetch_historical_data("BTCUSDT", "1H", start_date_binance)
        print(f"Binance BTCUSDT 1H Data Shape: {df_btc.shape}")
        if not df_btc.empty:
            print(df_btc.head(2))
        else:
            print("Warning: DataFrame is empty!")
            
        realtime_btc = binance_feed.fetch_realtime_data("BTCUSDT")
        print(f"Realtime: {realtime_btc['price']}")
        print("Binance Feed OK.\n")
    except Exception as e:
        print(f"Error testing Binance Feed: {e}")

if __name__ == "__main__":
    test_feeds()
