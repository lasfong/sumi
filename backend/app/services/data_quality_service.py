from typing import List, Dict, Any
import pandas as pd
from app.schemas.import_schema import ImportWarning

class DataQualityService:
    @staticmethod
    def validate_dataframe(df: pd.DataFrame) -> List[ImportWarning]:
        warnings = []
        
        # Ensure required columns exist
        required_cols = {'symbol', 'timestamp', 'open', 'high', 'low', 'close', 'volume'}
        missing_cols = required_cols - set(df.columns)
        if missing_cols:
            warnings.append(ImportWarning(row_index=-1, message=f"Missing columns: {missing_cols}"))
            return warnings

        # Iterate rows for logical checks
        # Pandas apply or vectorized is faster, but for detailed warnings row by row:
        for idx, row in df.iterrows():
            row_num = int(idx) + 1 # 1-based index for user reading
            
            # Missing checks
            if pd.isna(row['symbol']) or not str(row['symbol']).strip():
                warnings.append(ImportWarning(row_index=row_num, message="Missing symbol"))
            if pd.isna(row['timestamp']):
                warnings.append(ImportWarning(row_index=row_num, message="Missing timestamp"))
            
            # Price <= 0 checks
            prices = [row['open'], row['high'], row['low'], row['close']]
            if any(pd.isna(p) for p in prices):
                warnings.append(ImportWarning(row_index=row_num, message="Missing price values"))
            elif any(p <= 0 for p in prices):
                warnings.append(ImportWarning(row_index=row_num, message="Negative or zero price values"))
                
            # Volume < 0
            if pd.isna(row['volume']) or row['volume'] < 0:
                warnings.append(ImportWarning(row_index=row_num, message="Negative volume"))
                
            # High < Low (skip if any NaN)
            if not any(pd.isna(p) for p in prices):
                if row['high'] < row['low']:
                    warnings.append(ImportWarning(row_index=row_num, message="High is less than Low"))
                
                # Open/Close outside High/Low
                if not (row['low'] <= row['open'] <= row['high']):
                    warnings.append(ImportWarning(row_index=row_num, message="Open is outside High/Low range"))
                if not (row['low'] <= row['close'] <= row['high']):
                    warnings.append(ImportWarning(row_index=row_num, message="Close is outside High/Low range"))

        # Duplicate checks
        duplicates = df.duplicated(subset=['symbol', 'timestamp'], keep=False)
        for idx in df[duplicates].index:
            warnings.append(ImportWarning(row_index=int(idx) + 1, message="Duplicate timestamp for symbol"))

        return warnings
