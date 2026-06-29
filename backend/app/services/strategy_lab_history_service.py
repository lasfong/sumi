import json
from typing import Any

from sqlalchemy.orm import Session

from app.models.strategy_lab_run import StrategyLabRun


class StrategyLabHistoryService:
    @staticmethod
    def create_run(db: Session, payload: dict[str, Any]) -> dict[str, Any]:
        run = StrategyLabRun(
            run_type=payload["run_type"],
            label=payload["label"],
            request_config=json.dumps(payload.get("request_config", {}), ensure_ascii=False),
            result_payload=json.dumps(payload.get("result_payload", {}), ensure_ascii=False),
            metrics_payload=json.dumps(payload.get("metrics", {}), ensure_ascii=False),
        )
        db.add(run)
        db.commit()
        db.refresh(run)
        return StrategyLabHistoryService.to_dict(run)

    @staticmethod
    def list_runs(db: Session, limit: int = 50) -> list[dict[str, Any]]:
        runs = db.query(StrategyLabRun).order_by(StrategyLabRun.created_at.desc()).limit(limit).all()
        return [StrategyLabHistoryService.to_dict(run) for run in runs]

    @staticmethod
    def get_run(db: Session, run_id: int) -> dict[str, Any] | None:
        run = db.query(StrategyLabRun).filter(StrategyLabRun.id == run_id).first()
        return StrategyLabHistoryService.to_dict(run) if run else None

    @staticmethod
    def delete_run(db: Session, run_id: int) -> bool:
        run = db.query(StrategyLabRun).filter(StrategyLabRun.id == run_id).first()
        if not run:
            return False
        db.delete(run)
        db.commit()
        return True

    @staticmethod
    def to_dict(run: StrategyLabRun) -> dict[str, Any]:
        return {
            "id": run.id,
            "run_type": run.run_type,
            "label": run.label,
            "request_config": StrategyLabHistoryService._decode(run.request_config),
            "result_payload": StrategyLabHistoryService._decode(run.result_payload),
            "metrics": StrategyLabHistoryService._decode(run.metrics_payload),
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
