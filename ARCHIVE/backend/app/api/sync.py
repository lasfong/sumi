from typing import List, Optional
from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.schemas.sync_schema import (
    ProviderMetadataSchema,
    TestConnectionRequest,
    TestConnectionResponse,
    SyncPreviewRequest,
    SyncPreviewResponse,
    SyncExecuteRequest,
    SyncExecuteResponse,
    SyncRollbackRequest,
    SyncRollbackResponse,
    SyncManifestSchema,
)
from app.services.sync_workflow_service import SyncWorkflowService

router = APIRouter()

@router.get("/providers", response_model=List[ProviderMetadataSchema])
def list_providers():
    """Lists available market data providers and configuration status."""
    return SyncWorkflowService.list_providers()


@router.post("/test-connection", response_model=TestConnectionResponse)
def test_connection(request: TestConnectionRequest):
    """Tests connectivity to a specific data provider."""
    return SyncWorkflowService.test_connection(request.provider_id, request.credentials)


@router.post("/preview", response_model=SyncPreviewResponse)
def preview_sync(request: SyncPreviewRequest, db: Session = Depends(get_db)):
    """Fetches market data and generates a dry-run sync preview with conflict detection."""
    return SyncWorkflowService.generate_sync_preview(db, request)


@router.post("/execute", response_model=SyncExecuteResponse)
def execute_sync(request: SyncExecuteRequest, db: Session = Depends(get_db)):
    """Atomically commits previewed data, derives weekly candles, and records an audit manifest."""
    return SyncWorkflowService.execute_sync(db, request.sync_id, request.content_sha256)


@router.post("/rollback", response_model=SyncRollbackResponse)
def rollback_sync(request: SyncRollbackRequest, db: Session = Depends(get_db)):
    """Rolls back an accepted sync run and re-derives weekly series."""
    return SyncWorkflowService.rollback_sync(db, request.sync_id)


@router.get("/history", response_model=List[SyncManifestSchema])
def get_sync_history(limit: int = Query(50, ge=1, le=200), db: Session = Depends(get_db)):
    """Returns immutable audit manifests for past sync operations."""
    return SyncWorkflowService.get_sync_history(db, limit)


@router.get("/{sync_id}/manifest", response_model=SyncManifestSchema)
def get_sync_manifest(sync_id: str, db: Session = Depends(get_db)):
    """Returns detailed audit manifest for a specific sync run."""
    run = SyncWorkflowService.get_sync_run(db, sync_id)
    if not run:
        raise HTTPException(status_code=404, detail=f"Không tìm thấy lượt đồng bộ {sync_id}")
    return SyncManifestSchema(
        sync_id=run.id,
        created_at=run.created_at.isoformat(),
        provider_id=run.provider_id,
        symbol=run.symbol,
        start_date=run.start_date,
        end_date=run.end_date,
        timeframe=run.timeframe,
        adjustment_type=run.adjustment_type,
        status=run.status,
        parsed_count=run.parsed_count,
        duplicate_count=run.duplicate_count,
        conflicting_count=run.conflicting_count,
        accepted_count=run.accepted_count,
        duration_ms=run.duration_ms,
        accepted_at=run.accepted_at.isoformat() if run.accepted_at else None,
        rolled_back_at=run.rolled_back_at.isoformat() if run.rolled_back_at else None,
        manifest=run.manifest_json
    )
