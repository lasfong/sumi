import asyncio
import os
import sys

# Add the parent directory to sys.path to allow imports from app
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from sqlalchemy import text
from app.db import engine

def setup_timescaledb():
    print("Setting up TimescaleDB extension and hypertable...")
    
    with engine.connect() as conn:
        # Create TimescaleDB extension if not exists
        conn.execute(text("CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;"))
        conn.commit()
        print("- Extension timescaledb created or already exists.")
        
        # Check if hypertable already exists
        result = conn.execute(text(
            "SELECT count(*) FROM _timescaledb_catalog.hypertable WHERE table_name = 'candles';"
        )).scalar()
        
        if result == 0:
            # Convert candles table to hypertable
            conn.execute(text("SELECT create_hypertable('candles', 'timestamp');"))
            conn.commit()
            print("- Converted 'candles' table to hypertable.")
        else:
            print("- Table 'candles' is already a hypertable.")
            
    print("Database setup complete.")

if __name__ == "__main__":
    setup_timescaledb()
