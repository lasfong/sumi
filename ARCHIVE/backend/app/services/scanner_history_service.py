import json
from typing import Any

from sqlalchemy.orm import Session

from app.models.scanner_run import ScannerRun


class ScannerHistoryService:
    @staticmethod
    def create_run(db: Session, request_config: dict[str, Any], result_payload: dict[str, Any]) -> dict[str, Any]:
        strategy = request_config.get("strategy", {})
        strategy_name = strategy.get("name") if isinstance(strategy, dict) else None
        symbols = request_config.get("symbols") or []
        label = f"{strategy_name or 'Scanner'} on {len(symbols)} symbol(s)"

        run = ScannerRun(
            label=label,
            status=result_payload.get("status", "unknown"),
            total_results=int(result_payload.get("total_results") or 0),
            request_config=json.dumps(request_config, ensure_ascii=False),
            result_payload=json.dumps(result_payload, ensure_ascii=False),
        )
        db.add(run)
        db.commit()
        db.refresh(run)
        return ScannerHistoryService.to_dict(run)

    @staticmethod
    def list_runs(db: Session, limit: int = 30) -> list[dict[str, Any]]:
        clean_limit = max(1, min(int(limit), 100))
        runs = db.query(ScannerRun).order_by(ScannerRun.created_at.desc(), ScannerRun.id.desc()).limit(clean_limit).all()
        return [ScannerHistoryService.to_dict(run) for run in runs]

    @staticmethod
    def get_run(db: Session, run_id: int) -> dict[str, Any] | None:
        run = db.query(ScannerRun).filter(ScannerRun.id == run_id).first()
        return ScannerHistoryService.to_dict(run) if run else None

    @staticmethod
    def to_dict(run: ScannerRun) -> dict[str, Any]:
        return {
            "id": run.id,
            "label": run.label,
            "status": run.status,
            "total_results": run.total_results,
            "request_config": ScannerHistoryService._decode(run.request_config),
            "result_payload": ScannerHistoryService._decode(run.result_payload),
            "created_at": run.created_at.isoformat() if run.created_at else None,
        }

    @staticmethod
    def _decode(value: str | None) -> dict[str, Any]:
        if not value:
            return {}
        try:
            return json.loads(value)
        except json.JSONDecodeError:
            return {}
