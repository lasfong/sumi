from fastapi import APIRouter, UploadFile, File, Depends, HTTPException, Form
from sqlalchemy.orm import Session
from app.dependencies import get_db
from app.schemas.import_schema import ImportResponse
from app.services.cafef_importer import CafeFImporter
from app.services.event_logging_service import EventLoggingService

router = APIRouter()

@router.post("/cafef", response_model=ImportResponse)
async def import_cafef_data(
    file: UploadFile = File(...),
    adjustment_type: str = Form("unadjusted"),
    db: Session = Depends(get_db)
):
    """Import CafeF data from CSV, TXT, or ZIP file."""
    if not file.filename.endswith(('.csv', '.txt', '.zip')):
        raise HTTPException(status_code=400, detail="Only .csv, .txt, or .zip files are supported")
        
    content = await file.read()
    response = CafeFImporter.import_data(db, content, file.filename, adjustment_type)
    
    EventLoggingService.log_event(
        db=db,
        event_type="IMPORT_COMPLETED",
        message=f"Imported {response.imported_rows} rows for {response.symbols_count} symbols",
        details={
            "filename": file.filename,
            "imported_rows": response.imported_rows,
            "symbols_count": response.symbols_count,
            "skipped_rows": response.skipped_rows
        }
    )
    
    return response

@router.post("/benchmark", response_model=ImportResponse)
def import_benchmark_data(
    start_date: str = Form("2000-01-01"),
    end_date: str = Form(None),
    db: Session = Depends(get_db)
):
    """Import VNINDEX benchmark data using vnstock."""
    try:
        from vnstock import stock_historical_data
        from datetime import datetime
        from app.models.candle import Candle
        
        if not end_date:
            end_date = datetime.now().strftime("%Y-%m-%d")
            
        # Call vnstock
        df = stock_historical_data("VNINDEX", start_date, end_date, "1D", "index")
        
        if df is None or df.empty:
            raise HTTPException(status_code=400, detail="No data returned from vnstock")
            
        # DataFrame columns typically: ['time', 'open', 'high', 'low', 'close', 'volume', 'ticker']
        # Convert to candles
        candles = []
        for _, row in df.iterrows():
            candles.append(Candle(
                symbol="VNINDEX",
                timeframe="D",
                adjustment_type="split",
                timestamp=datetime.strptime(str(row['time'])[:10], "%Y-%m-%d"),
                open=float(row['open']),
                high=float(row['high']),
                low=float(row['low']),
                close=float(row['close']),
                volume=float(row['volume'])
            ))
            
        # Delete old benchmark data
        db.query(Candle).filter(Candle.symbol == "VNINDEX").delete()
        
        # Insert new
        db.bulk_save_objects(candles)
        db.commit()
        
        EventLoggingService.log_event(
            db=db,
            event_type="IMPORT_BENCHMARK",
            message=f"Imported {len(candles)} rows for VNINDEX",
            details={
                "start_date": start_date,
                "end_date": end_date,
                "imported_rows": len(candles)
            }
        )
        
        return ImportResponse(
            imported_rows=len(candles),
            symbols_count=1,
            skipped_rows=0,
            message="VNINDEX benchmark data imported successfully"
        )
    except Exception as e:
        db.rollback()
        raise HTTPException(status_code=500, detail=str(e))
