from fastapi import APIRouter, UploadFile, File, Depends, HTTPException, Form
from sqlalchemy.orm import Session
from typing import List, Optional

from app.dependencies import get_db
from app.schemas.import_schema import (
    ImportResponse,
    ImportWarning,
    ImportPreviewResponse,
    ImportAcceptRequest,
    ImportAcceptResponse,
    ImportRollbackResponse,
    CatalogItemSchema
)
from app.services.import_workflow_service import ImportWorkflowService
from app.services.event_logging_service import EventLoggingService

router = APIRouter()

@router.post("/preview", response_model=ImportPreviewResponse)
async def preview_import_data(
    file: UploadFile = File(...),
    source_type: str = Form("cafef"),
    timeframe: str = Form("1D"),
    adjustment_type: str = Form("unadjusted"),
    timezone: str = Form("Asia/Ho_Chi_Minh"),
    timezone_str: Optional[str] = Form(None),
    db: Session = Depends(get_db)
):
    """Preview an import file without mutating accepted candle data."""
    if not file.filename.endswith(('.csv', '.txt', '.zip')):
        raise HTTPException(status_code=400, detail="Chỉ hỗ trợ tập tin dạng .csv, .txt hoặc .zip")

    selected_timezone = timezone_str if timezone_str is not None else timezone
    content = await file.read()
    response = ImportWorkflowService.preview_import(
        db=db,
        file_content=content,
        filename=file.filename,
        source_type=source_type,
        timeframe=timeframe,
        adjustment_type=adjustment_type,
        timezone_str=selected_timezone
    )
    return response

@router.post("/runs/{run_id}/accept", response_model=ImportAcceptResponse)
def accept_import_run(
    run_id: str,
    payload: ImportAcceptRequest,
    db: Session = Depends(get_db)
):
    """Explicitly accept a previewed import run and apply mutations atomically."""
    res = ImportWorkflowService.accept_import(db, run_id, payload.content_sha256)
    
    EventLoggingService.log_event(
        db=db,
        event_type="IMPORT_ACCEPTED",
        message=res.message,
        details={"run_id": run_id, "accepted_count": res.accepted_count, "status": res.status}
    )
    return res

@router.get("/runs")
def list_import_runs(db: Session = Depends(get_db)):
    """List historical import run manifests."""
    runs = ImportWorkflowService.list_import_runs(db)
    return [
        {
            "run_id": r.id,
            "file_name": r.file_name,
            "created_at": r.created_at.isoformat() if r.created_at else None,
            "source_type": r.source_type,
            "timeframe": r.timeframe,
            "adjustment_type": r.adjustment_type,
            "status": r.status,
            "parsed_count": r.parsed_count,
            "rejected_count": r.rejected_count,
            "duplicate_count": r.duplicate_count,
            "conflicting_count": r.conflicting_count,
            "accepted_count": r.accepted_count,
            "can_accept": r.can_accept,
            "accepted_at": r.accepted_at.isoformat() if r.accepted_at else None,
            "rolled_back_at": r.rolled_back_at.isoformat() if r.rolled_back_at else None
        }
        for r in runs
    ]

@router.get("/runs/{run_id}")
def get_import_run_detail(run_id: str, db: Session = Depends(get_db)):
    """Get details of a specific import run manifest."""
    run = ImportWorkflowService.get_import_run(db, run_id)
    if not run:
        raise HTTPException(status_code=404, detail=f"Không tìm thấy lượt nhập {run_id}")
    return run

@router.post("/runs/{run_id}/rollback", response_model=ImportRollbackResponse)
def rollback_import_run(
    run_id: str,
    db: Session = Depends(get_db)
):
    """Roll back an accepted import run restoring exact before-images."""
    res = ImportWorkflowService.rollback_import(db, run_id)
    
    EventLoggingService.log_event(
        db=db,
        event_type="IMPORT_ROLLED_BACK",
        message=res.message,
        details={"run_id": run_id, "restored_mutations_count": res.restored_mutations_count}
    )
    return res

@router.get("/catalog", response_model=List[CatalogItemSchema])
def get_data_catalog(db: Session = Depends(get_db)):
    """Get complete market data catalog coverage and provenance."""
    return ImportWorkflowService.get_catalog(db)

@router.post("/cafef", response_model=ImportResponse)
async def import_cafef_data(
    file: UploadFile = File(...),
    adjustment_type: str = Form("unadjusted"),
    db: Session = Depends(get_db)
):
    """Legacy CafeF import route disabled to enforce explicit preview -> user confirmation -> accept contract."""
    raise HTTPException(
        status_code=400,
        detail="Tuyến API legacy /api/import/cafef tự động chấp nhận đã bị vô hiệu hóa để bảo vệ an toàn dữ liệu. Vui lòng sử dụng quy trình /api/import/preview và /api/import/runs/{run_id}/accept với xác nhận từ người dùng."
    )

@router.post("/benchmark", response_model=ImportResponse)
def import_benchmark_data(
    start_date: str = Form("2000-01-01"),
    end_date: str = Form(None),
    db: Session = Depends(get_db)
):
    """Legacy benchmark import route - network sync requires explicit approval (PRO-10/11)."""
    raise HTTPException(
        status_code=400,
        detail="Đồng bộ chỉ số VNINDEX qua mạng yêu cầu nhà cung cấp dữ liệu được phê duyệt (PRO-10/PRO-11)."
    )
