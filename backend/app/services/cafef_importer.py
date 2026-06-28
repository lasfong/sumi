import pandas as pd
import zipfile
from typing import List, Tuple
from io import BytesIO
from sqlalchemy.orm import Session
from sqlalchemy.dialects.sqlite import insert

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.schemas.import_schema import ImportResponse, ImportWarning
from app.services.data_quality_service import DataQualityService

class CafeFImporter:
    @staticmethod
    def parse_file(file_content: bytes, filename: str) -> pd.DataFrame:
        # Check if CSV or TXT
        # CafeF format: <Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>, sometimes <OI>
        try:
            df = pd.read_csv(BytesIO(file_content))
        except Exception:
            # Fallback for malformed CSVs (e.g. trailing commas, extra columns like <OI> without header)
            df = pd.read_csv(BytesIO(file_content), on_bad_lines='skip', engine='python')

        # Standardize columns
        col_map = {}
        for col in df.columns:
            lower_col = col.lower().strip('<>')
            if 'ticker' in lower_col or 'symbol' in lower_col:
                col_map[col] = 'symbol'
            elif 'dt' in lower_col or 'date' in lower_col:
                col_map[col] = 'timestamp'
            elif 'open' in lower_col:
                col_map[col] = 'open'
            elif 'high' in lower_col:
                col_map[col] = 'high'
            elif 'low' in lower_col:
                col_map[col] = 'low'
            elif 'close' in lower_col:
                col_map[col] = 'close'
            elif 'volume' in lower_col or 'vol' in lower_col:
                col_map[col] = 'volume'

        df.rename(columns=col_map, inplace=True)

        # Parse date
        if 'timestamp' in df.columns:
            # CafeF format is usually YYYYMMDD
            try:
                df['timestamp'] = pd.to_datetime(df['timestamp'], format='%Y%m%d').dt.date
            except Exception:
                try:
                    df['timestamp'] = pd.to_datetime(df['timestamp']).dt.date
                except Exception:
                    pass # Data quality service will catch it

        return df

    @staticmethod
    def parse_zip(file_content: bytes) -> pd.DataFrame:
        """Extract and parse CSV/TXT files from a ZIP archive."""
        frames = []
        with zipfile.ZipFile(BytesIO(file_content)) as zf:
            for name in zf.namelist():
                if name.lower().endswith(('.csv', '.txt')) and not name.startswith('__MACOSX'):
                    with zf.open(name) as f:
                        content = f.read()
                        df = CafeFImporter.parse_file(content, name)
                        frames.append(df)
        if not frames:
            raise ValueError("No CSV/TXT files found inside ZIP archive")
        return pd.concat(frames, ignore_index=True)

    @staticmethod
    def _detect_exchange_from_filename(filename: str) -> str:
        """Detect exchange from CafeF filename."""
        filename_upper = filename.upper()
        if "HSX" in filename_upper or "HOSE" in filename_upper:
            return "HOSE"
        elif "HNX" in filename_upper:
            return "HNX"
        elif "UPCOM" in filename_upper:
            return "UPCOM"
        return None

    @staticmethod
    def import_data(db: Session, file_content: bytes, filename: str, adjustment_type: str = 'unadjusted') -> ImportResponse:
        try:
            if filename.lower().endswith('.zip'):
                df = CafeFImporter.parse_zip(file_content)
            else:
                df = CafeFImporter.parse_file(file_content, filename)
        except Exception as e:
            return ImportResponse(
                imported_rows=0, skipped_rows=0, duplicate_rows=0, symbols_count=0,
                warnings=[ImportWarning(row_index=0, message=f"Failed to parse file: {str(e)}")]
            )

        warnings = DataQualityService.validate_dataframe(df)

        # Drop rows with critical errors to continue
        error_rows = set([w.row_index - 1 for w in warnings if w.row_index > 0])
        valid_df = df.drop(index=list(error_rows)).copy()

        # Filter duplicates (keep last)
        duplicates_count = len(valid_df) - len(valid_df.drop_duplicates(subset=['symbol', 'timestamp']))
        valid_df.drop_duplicates(subset=['symbol', 'timestamp'], keep='last', inplace=True)

        imported = 0
        symbols_found = set()
        start_date = None
        end_date = None

        if not valid_df.empty:
            symbols_found = set(valid_df['symbol'].unique())
            start_date = str(valid_df['timestamp'].min())
            end_date = str(valid_df['timestamp'].max())

            exchange = CafeFImporter._detect_exchange_from_filename(filename)

            # Upsert Symbols
            for sym in symbols_found:
                symbol_record = db.query(Symbol).filter(Symbol.symbol == sym).first()
                if symbol_record:
                    if exchange and not symbol_record.exchange:
                        symbol_record.exchange = exchange
                else:
                    symbol_record = Symbol(symbol=sym, exchange=exchange)
                    db.add(symbol_record)
            db.flush()

            # Upsert Candles
            records = valid_df.to_dict(orient='records')
            for rec in records:
                rec['timeframe'] = '1D'
                rec['source'] = 'cafef'
                rec['adjustment_type'] = adjustment_type

            # SQLite upsert chunking (max 32766 variables / 8 columns ~ 4000 rows)
            chunk_size = 2000
            for i in range(0, len(records), chunk_size):
                chunk = records[i:i + chunk_size]
                stmt = insert(Candle).values(chunk)
                stmt = stmt.on_conflict_do_update(
                    index_elements=['symbol', 'timeframe', 'timestamp', 'adjustment_type'],
                    set_={
                        'open': stmt.excluded.open,
                        'high': stmt.excluded.high,
                        'low': stmt.excluded.low,
                        'close': stmt.excluded.close,
                        'volume': stmt.excluded.volume,
                    }
                )
                db.execute(stmt)
            db.commit()
            imported = len(records)

        return ImportResponse(
            imported_rows=imported,
            skipped_rows=len(error_rows),
            duplicate_rows=duplicates_count,
            symbols_count=len(symbols_found),
            start_date=start_date,
            end_date=end_date,
            warnings=warnings[:100] # Cap warnings to not overload JSON
        )
