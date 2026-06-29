from app.services.strategy_lab_history_service import StrategyLabHistoryService


def test_strategy_lab_history_create_list_get_delete(db_session):
    payload = {
        "run_type": "comparison",
        "label": "Two strategy comparison",
        "request_config": {"symbols": ["FPT", "SSI"]},
        "result_payload": {"results": [{"name": "Trend"}]},
        "metrics": {"net_pnl": 1234.5},
    }

    created = StrategyLabHistoryService.create_run(db_session, payload)

    assert created["id"] is not None
    assert created["label"] == "Two strategy comparison"
    assert created["request_config"]["symbols"] == ["FPT", "SSI"]
    assert created["metrics"]["net_pnl"] == 1234.5

    listed = StrategyLabHistoryService.list_runs(db_session)
    assert len(listed) == 1
    assert listed[0]["id"] == created["id"]

    fetched = StrategyLabHistoryService.get_run(db_session, created["id"])
    assert fetched["result_payload"]["results"][0]["name"] == "Trend"

    assert StrategyLabHistoryService.delete_run(db_session, created["id"]) is True
    assert StrategyLabHistoryService.get_run(db_session, created["id"]) is None
    assert StrategyLabHistoryService.delete_run(db_session, created["id"]) is False
