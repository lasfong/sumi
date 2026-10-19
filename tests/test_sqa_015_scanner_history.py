import sys
from pathlib import Path
import pytest

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from app.db import Base
import app.models
from app.services.scanner_history_service import ScannerHistoryService


def test_sqa_015_scanner_history_persistence_and_isolation():
    """SQA-015: Saved scan runs persist independent result_payloads without cross-run contamination."""
    engine = create_engine("sqlite:///:memory:")
    Base.metadata.create_all(engine)
    Session = sessionmaker(bind=engine)
    db = Session()

    try:
        run1_payload = {
            "status": "succeeded",
            "total_results": 2,
            "truncated": False,
            "warnings": [],
            "results": [
                {"symbol": "FPT", "timestamp": "2024-01-10T00:00:00", "signal_type": "entry", "price": 100000.0, "regime": "bullish"},
                {"symbol": "SSI", "timestamp": "2024-01-10T00:00:00", "signal_type": "entry", "price": 35000.0, "regime": "bullish"},
            ],
        }
        run2_payload = {
            "status": "succeeded",
            "total_results": 1,
            "truncated": False,
            "warnings": [],
            "results": [
                {"symbol": "VCI", "timestamp": "2024-01-15T00:00:00", "signal_type": "exit", "price": 42000.0, "regime": "bearish"},
            ],
        }

        # Create run 1 via ScannerHistoryService
        run1_dict = ScannerHistoryService.create_run(
            db=db,
            request_config={"symbols": ["FPT", "SSI"], "strategy": {"name": "EMA Cross"}},
            result_payload=run1_payload,
        )

        # Create run 2 via ScannerHistoryService
        run2_dict = ScannerHistoryService.create_run(
            db=db,
            request_config={"symbols": ["VCI"], "strategy": {"name": "RSI Oversold"}},
            result_payload=run2_payload,
        )

        # List runs
        runs = ScannerHistoryService.list_runs(db, limit=10)
        assert len(runs) == 2

        # Verify run 1 payload is preserved exactly and not overwritten by run 2
        retrieved_run1 = next(r for r in runs if r["id"] == run1_dict["id"])
        assert retrieved_run1["total_results"] == 2
        assert retrieved_run1["result_payload"]["total_results"] == 2
        assert [r["symbol"] for r in retrieved_run1["result_payload"]["results"]] == ["FPT", "SSI"]

        # Verify run 2 payload
        retrieved_run2 = next(r for r in runs if r["id"] == run2_dict["id"])
        assert retrieved_run2["total_results"] == 1
        assert retrieved_run2["result_payload"]["total_results"] == 1
        assert [r["symbol"] for r in retrieved_run2["result_payload"]["results"]] == ["VCI"]

        # Fetch individually by ID
        single_run1 = ScannerHistoryService.get_run(db, run1_dict["id"])
        assert single_run1 is not None
        assert single_run1["total_results"] == 2
        assert [r["symbol"] for r in single_run1["result_payload"]["results"]] == ["FPT", "SSI"]
    finally:
        db.close()
