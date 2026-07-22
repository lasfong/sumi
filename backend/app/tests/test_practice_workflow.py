from datetime import date, timedelta

import pytest
from fastapi import HTTPException

from app.domain.enums import DecisionAction
from app.models.candle import Candle
from app.schemas.decision_schema import DecisionCreate
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.journal_schema import JournalEntryCreate
from app.services.journal_service import JournalService
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.models.execution import Execution
from app.models.journal_entry import JournalEntry


def make_session(
    db_session,
    symbol: str = "PRACTICE",
    closes: list[float] | None = None,
    initial_cash: float = 1_000_000.0,
):
    prices = closes or [100.0, 102.0, 104.0, 106.0, 108.0, 110.0]
    for index, close in enumerate(prices):
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=date(2024, 1, 2) + timedelta(days=index),
            open=close,
            high=close + 2,
            low=close - 2,
            close=close,
            volume=1_000_000 + index,
        ))
    db_session.commit()
    return ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol=symbol,
        start_date=date(2024, 1, 2),
        end_date=date(2024, 1, 2) + timedelta(days=len(prices) - 1),
        initial_cash=initial_cash,
    ))


def test_rewind_projects_future_trade_state_without_destroying_ledger(db_session):
    session = make_session(db_session)
    TradeLifecycleService.process_decision(
        db_session, session.id, DecisionCreate(action=DecisionAction.BUY, quantity=100),
    )
    ReplayService.next_candle(db_session, session.id, 2)
    TradeLifecycleService.process_decision(
        db_session, session.id, DecisionCreate(action=DecisionAction.CLOSE),
    )

    tip = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert tip.current_index == 2
    assert len(tip.decisions) == 2
    assert len(tip.executions) == 2
    assert tip.positions == []

    ReplayService.previous_candle(db_session, session.id, 2)
    rewound = PracticeWorkflowService.get_snapshot(db_session, session.id)

    assert rewound.current_index == 0
    assert [decision.action for decision in rewound.decisions] == ["BUY"]
    assert len(rewound.executions) == 1
    assert rewound.positions[0].quantity == 100
    assert rewound.historical is True
    assert rewound.latest_activity_index == 2

    ReplayService.next_candle(db_session, session.id, 2)
    restored = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert len(restored.executions) == 2
    assert restored.positions == []
    assert restored.current_cash == pytest.approx(tip.current_cash)


def test_historical_cursor_rejects_new_accounting_decision(db_session):
    session = make_session(db_session, symbol="HISTORY")
    ReplayService.next_candle(db_session, session.id, 2)
    TradeLifecycleService.process_decision(
        db_session, session.id, DecisionCreate(action=DecisionAction.HOLD, reason="future fact"),
    )
    ReplayService.previous_candle(db_session, session.id)

    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(
            db_session, session.id, DecisionCreate(action=DecisionAction.BUY, quantity=100),
        )

    assert exc_info.value.status_code == 409
    assert "historical" in exc_info.value.detail.lower()


def checklist_content(session, observation: str = "Wait for confirmation") -> str:
    import json
    return json.dumps({
        "schemaVersion": 1,
        "context": {
            "sessionId": session.id,
            "symbol": session.symbol,
            "candleIndex": session.current_index,
            "date": (date(2024, 1, 2) + timedelta(days=session.current_index)).isoformat(),
        },
        "checks": {
            "trendIdentified": True,
            "setupConfirmed": True,
            "entryTriggerDefined": False,
            "riskDefined": True,
            "exitPlanDefined": True,
            "emotionChecked": True,
        },
        "observation": observation,
    })


def test_context_bound_checklist_hides_on_rewind_and_restores_on_forward(db_session):
    session = make_session(db_session, symbol="JOURNAL")
    ReplayService.next_candle(db_session, session.id, 2)
    entry = JournalService.create(db_session, session.id, JournalEntryCreate(
        note_type="practice_checklist",
        content=checklist_content(session),
    ))
    assert [item.id for item in JournalService.list_visible(db_session, session.id)] == [entry.id]

    ReplayService.previous_candle(db_session, session.id, 2)
    assert JournalService.list_visible(db_session, session.id) == []

    ReplayService.next_candle(db_session, session.id, 2)
    assert [item.id for item in JournalService.list_visible(db_session, session.id)] == [entry.id]


def test_checklist_rejects_stale_or_cross_workspace_context(db_session):
    session = make_session(db_session, symbol="CHECKLIST")
    content = checklist_content(session).replace('"candleIndex": 0', '"candleIndex": 1')
    with pytest.raises(HTTPException) as exc_info:
        JournalService.create(db_session, session.id, JournalEntryCreate(
            note_type="practice_checklist",
            content=content,
        ))
    assert exc_info.value.status_code == 409


def test_checklist_accepts_exact_authoritative_current_date(db_session):
    session = make_session(db_session, symbol="CHECKLIST_EXACT")
    ReplayService.next_candle(db_session, session.id, 2)

    entry = JournalService.create(db_session, session.id, JournalEntryCreate(
        note_type="practice_checklist",
        content=checklist_content(session, "Exact date"),
    ))

    assert entry.id is not None
    assert JournalService.list_visible(db_session, session.id)[0].id == entry.id


@pytest.mark.parametrize("bad_date", ["2024-02-30", "not-a-date", "20240102", "2024-1-2"])
def test_checklist_rejects_impossible_or_malformed_date_without_inserting(db_session, bad_date):
    import json

    session = make_session(db_session, symbol=f"BAD_DATE_{bad_date}")
    payload = json.loads(checklist_content(session))
    payload["context"]["date"] = bad_date

    with pytest.raises(HTTPException) as exc_info:
        JournalService.create(db_session, session.id, JournalEntryCreate(
            note_type="practice_checklist",
            content=json.dumps(payload),
        ))

    assert exc_info.value.status_code == 400
    assert db_session.query(JournalEntry).filter_by(session_id=session.id).count() == 0


def test_checklist_rejects_cross_session_and_symbol_without_inserting(db_session):
    import json

    session = make_session(db_session, symbol="CHECKLIST_OWNER")
    other = make_session(db_session, symbol="CHECKLIST_OTHER")
    for field, value in (("sessionId", other.id), ("symbol", other.symbol)):
        payload = json.loads(checklist_content(session))
        payload["context"][field] = value
        with pytest.raises(HTTPException) as exc_info:
            JournalService.create(db_session, session.id, JournalEntryCreate(
                note_type="practice_checklist",
                content=json.dumps(payload),
            ))
        assert exc_info.value.status_code == 409
    assert db_session.query(JournalEntry).filter_by(session_id=session.id).count() == 0


def test_limit_fill_projects_pending_before_fill_and_never_duplicates(db_session):
    from app.models.symbol import Symbol
    db_session.add(Symbol(symbol="LIMIT_PROJECT", exchange="HOSE"))
    session = make_session(db_session, symbol="LIMIT_PROJECT", closes=[100.0, 101.0, 106.0, 107.0])
    ReplayService.next_candle(db_session, session.id)
    TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(
        action=DecisionAction.BUY, quantity=100, order_type="LIMIT", price=105.0,
    ))
    pending = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert pending.orders[0].status == "pending"
    assert pending.executions == []

    ReplayService.next_candle(db_session, session.id)
    filled = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert filled.orders[0].status == "executed"
    assert len(filled.executions) == 1
    execution_count = db_session.query(Execution).filter_by(session_id=session.id).count()

    ReplayService.previous_candle(db_session, session.id)
    rewound = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert rewound.orders[0].status == "pending"
    assert rewound.executions == []

    ReplayService.next_candle(db_session, session.id)
    restored = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert restored.orders[0].status == "executed"
    assert len(restored.executions) == 1
    assert db_session.query(Execution).filter_by(session_id=session.id).count() == execution_count


def test_t2_projection_and_rejection_preserve_exact_state(db_session):
    session = make_session(db_session, symbol="T2")
    TradeLifecycleService.process_decision(
        db_session, session.id, DecisionCreate(action=DecisionAction.BUY, quantity=100),
    )
    before = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert before.available_quantity == 0
    ReplayService.next_candle(db_session, session.id)
    pre_t2 = PracticeWorkflowService.get_snapshot(db_session, session.id)
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(
            db_session, session.id, DecisionCreate(action=DecisionAction.SELL, quantity=100),
        )
    assert "T+2 constraint" in exc_info.value.detail
    assert PracticeWorkflowService.get_snapshot(db_session, session.id) == pre_t2

    ReplayService.next_candle(db_session, session.id)
    settled = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert settled.available_quantity == 100
    assert settled.positions[0].available_quantity == 100


def test_multistep_advance_fills_limit_on_earliest_intermediate_candle(db_session):
    from app.models.symbol import Symbol

    db_session.add(Symbol(symbol="MULTISTEP_LIMIT", exchange="HOSE"))
    session = make_session(
        db_session,
        symbol="MULTISTEP_LIMIT",
        closes=[100.0, 105.0, 120.0, 130.0],
    )
    TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(
        action=DecisionAction.BUY,
        quantity=100,
        order_type="LIMIT",
        price=105.0,
    ))

    ReplayService.next_candle(db_session, session.id, 3)
    snapshot = PracticeWorkflowService.get_snapshot(db_session, session.id)

    assert snapshot.current_index == 3
    assert len(snapshot.executions) == 1
    assert snapshot.executions[0].execution_index == 1
    assert snapshot.executions[0].execution_price == pytest.approx(105.0)
    assert snapshot.orders[0].status == "executed"
    assert snapshot.positions[0].quantity == 100


def test_checklist_rejects_wrong_valid_date_without_inserting(db_session):
    session = make_session(db_session, symbol="CHECKLIST_DATE")
    wrong_date = checklist_content(session).replace('"date": "2024-01-02"', '"date": "2099-12-31"')
    before = db_session.query(JournalEntry).filter_by(session_id=session.id).count()

    with pytest.raises(HTTPException) as exc_info:
        JournalService.create(db_session, session.id, JournalEntryCreate(
            note_type="practice_checklist",
            content=wrong_date,
        ))

    assert exc_info.value.status_code == 409
    assert db_session.query(JournalEntry).filter_by(session_id=session.id).count() == before


def _create_identical_session(db_session, source_session):
    return ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol=source_session.symbol,
        timeframe=source_session.timeframe,
        adjustment_type=source_session.adjustment_type,
        start_date=source_session.start_date,
        end_date=source_session.end_date,
        initial_cash=source_session.initial_cash,
    ))


def _snapshot_signature(snapshot):
    return {
        "index": snapshot.current_index,
        "cash": snapshot.current_cash,
        "available": snapshot.available_quantity,
        "orders": [(item.side, item.order_type, item.requested_price, item.quantity, item.status, item.decision_index) for item in snapshot.orders],
        "executions": [(item.side, item.execution_index, item.execution_date, item.execution_price, item.quantity, item.net_amount) for item in snapshot.executions],
        "positions": [(item.symbol, item.quantity, item.average_price, item.total_cost, item.current_price, item.realized_pnl, item.unrealized_pnl, item.available_quantity) for item in snapshot.positions],
        "trades": [(item.symbol, item.entry_date, item.entry_price, item.quantity, item.exit_date, item.exit_price, item.net_pnl, item.pnl_percent, item.status, item.result) for item in snapshot.trades],
    }


def test_multistep_and_repeated_single_steps_have_equal_lifecycle_state(db_session):
    from app.models.position import Position
    from app.models.symbol import Symbol

    db_session.add(Symbol(symbol="STEP_EQUAL", exchange="HOSE"))
    multi = make_session(db_session, symbol="STEP_EQUAL", closes=[100.0, 105.0, 120.0, 122.0, 124.0, 126.0])
    singles = _create_identical_session(db_session, multi)
    for session in (multi, singles):
        TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(
            action=DecisionAction.BUY, quantity=100, order_type="LIMIT", price=105.0,
        ))

    ReplayService.next_candle(db_session, multi.id, 5)
    for _ in range(5):
        ReplayService.next_candle(db_session, singles.id)

    multi_snapshot = PracticeWorkflowService.get_snapshot(db_session, multi.id)
    singles_snapshot = PracticeWorkflowService.get_snapshot(db_session, singles.id)
    assert _snapshot_signature(multi_snapshot) == _snapshot_signature(singles_snapshot)
    assert ReplayService.get_session(db_session, multi.id).status == ReplayService.get_session(db_session, singles.id).status == "completed"
    assert multi_snapshot.executions[0].execution_index == 1
    assert multi_snapshot.available_quantity == 100
    multi_position = db_session.query(Position).filter_by(session_id=multi.id).first()
    singles_position = db_session.query(Position).filter_by(session_id=singles.id).first()
    assert multi_position.unrealized_pnl == pytest.approx(singles_position.unrealized_pnl)


@pytest.mark.parametrize(
    ("symbol", "closes", "steps", "expected_index", "expected_execution_index"),
    [
        ("STEP_NO_HIT", [100.0, 120.0, 130.0, 140.0], 3, 3, None),
        ("STEP_DESTINATION", [100.0, 120.0, 130.0, 105.0], 3, 3, 3),
        ("STEP_EARLIEST", [100.0, 105.0, 105.5, 130.0], 3, 3, 1),
        ("STEP_COMPLETION", [100.0, 120.0, 130.0, 140.0], 99, 3, None),
    ],
)
def test_multistep_limit_boundaries(db_session, symbol, closes, steps, expected_index, expected_execution_index):
    from app.models.symbol import Symbol

    db_session.add(Symbol(symbol=symbol, exchange="HOSE"))
    session = make_session(db_session, symbol=symbol, closes=closes)
    TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(
        action=DecisionAction.BUY, quantity=100, order_type="LIMIT", price=105.0,
    ))

    ReplayService.next_candle(db_session, session.id, steps)
    snapshot = PracticeWorkflowService.get_snapshot(db_session, session.id)

    assert snapshot.current_index == expected_index
    assert ReplayService.get_session(db_session, session.id).status == "completed"
    if expected_execution_index is None:
        assert snapshot.executions == []
        assert snapshot.orders[0].status == "pending"
    else:
        assert len(snapshot.executions) == 1
        assert snapshot.executions[0].execution_index == expected_execution_index
        assert snapshot.orders[0].status == "executed"


def test_rewind_then_multistep_forward_preserves_execution_identity(db_session):
    from app.models.symbol import Symbol

    db_session.add(Symbol(symbol="STEP_REWIND", exchange="HOSE"))
    session = make_session(db_session, symbol="STEP_REWIND", closes=[100.0, 105.0, 120.0, 130.0])
    TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(
        action=DecisionAction.BUY, quantity=100, order_type="LIMIT", price=105.0,
    ))
    ReplayService.next_candle(db_session, session.id, 3)
    execution_id = db_session.query(Execution).filter_by(session_id=session.id).one().id

    ReplayService.previous_candle(db_session, session.id, 3)
    assert PracticeWorkflowService.get_snapshot(db_session, session.id).executions == []
    ReplayService.next_candle(db_session, session.id, 3)

    executions = db_session.query(Execution).filter_by(session_id=session.id).all()
    assert [item.id for item in executions] == [execution_id]
    assert len(PracticeWorkflowService.get_snapshot(db_session, session.id).executions) == 1


def test_multistep_and_single_steps_stop_at_same_bankruptcy_candle(db_session):
    from app.models.symbol import Symbol

    db_session.add(Symbol(symbol="STEP_BANKRUPT", exchange="HOSE"))
    multi = make_session(db_session, symbol="STEP_BANKRUPT", closes=[100.0, 100.0, 5.0, 130.0], initial_cash=20_000.0)
    singles = _create_identical_session(db_session, multi)
    for session in (multi, singles):
        TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(
            action=DecisionAction.BUY, quantity=100,
        ))
        session.current_cash = -1_000.0
    db_session.commit()

    ReplayService.next_candle(db_session, multi.id, 3)
    ReplayService.next_candle(db_session, singles.id)
    ReplayService.next_candle(db_session, singles.id)

    multi_session = ReplayService.get_session(db_session, multi.id)
    singles_session = ReplayService.get_session(db_session, singles.id)
    assert multi_session.current_index == singles_session.current_index
    assert multi_session.status == singles_session.status
    assert multi_session.current_cash == pytest.approx(singles_session.current_cash)
    assert multi_session.current_index == 2
    assert multi_session.status == "bankrupt"
    assert _snapshot_signature(PracticeWorkflowService.get_snapshot(db_session, multi.id)) == _snapshot_signature(
        PracticeWorkflowService.get_snapshot(db_session, singles.id)
    )
