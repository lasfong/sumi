import asyncio
import json
import logging
from dataclasses import dataclass, field
from datetime import timezone
from typing import Any, Callable, Optional

from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from sqlalchemy.orm import Session

from app.db import SessionLocal
from app.models.replay_session import ReplaySession
from app.services.replay_service import ReplayService

logger = logging.getLogger(__name__)
router = APIRouter()

MIN_SPEED_MS = 100
MAX_SPEED_MS = 4000
DEFAULT_SPEED_MS = 1000

VALID_ACTIONS = {"start", "play", "pause", "next", "step", "set_speed", "ping", "sync", "resume"}


def clamp_speed_ms(raw_speed: Any, raw_multiplier: Any = None) -> tuple[int, bool]:
    """
    Validate and clamp playback interval in milliseconds to [100, 4000].
    Also supports optional speed_multiplier in [0.25, 10.0] where 1x = 1000ms.
    Returns (clamped_speed_ms, was_clamped).
    """
    if raw_multiplier is not None and raw_speed is None:
        try:
            mult = float(raw_multiplier)
            if mult <= 0:
                return MAX_SPEED_MS, True
            computed = int(round(1000.0 / mult))
            clamped = max(MIN_SPEED_MS, min(MAX_SPEED_MS, computed))
            return clamped, clamped != computed
        except (TypeError, ValueError):
            return DEFAULT_SPEED_MS, True

    if raw_speed is None:
        return DEFAULT_SPEED_MS, False

    try:
        val = int(round(float(raw_speed)))
    except (TypeError, ValueError):
        return DEFAULT_SPEED_MS, True

    clamped = max(MIN_SPEED_MS, min(MAX_SPEED_MS, val))
    return clamped, clamped != val


def format_candle(candle: Any) -> dict[str, Any]:
    timestamp = candle.timestamp
    if timestamp.tzinfo is None:
        # Candle timestamps are stored as timezone-free market dates. Treat them
        # as UTC before producing the wire epoch so the host timezone cannot
        # shift a daily candle onto the preceding date in the browser.
        timestamp = timestamp.replace(tzinfo=timezone.utc)
    return {
        "id": getattr(candle, "id", None),
        "symbol": getattr(candle, "symbol", None),
        "timeframe": getattr(candle, "timeframe", "1D"),
        "timestamp": candle.timestamp.isoformat() if hasattr(candle.timestamp, "isoformat") else str(candle.timestamp),
        "time": int(timestamp.timestamp()),
        "open": candle.open,
        "high": candle.high,
        "low": candle.low,
        "close": candle.close,
        "volume": candle.volume,
        "source": getattr(candle, "source", "vnstock"),
        "adjustment_type": getattr(candle, "adjustment_type", "unadjusted"),
    }


@dataclass
class ClientConnection:
    websocket: WebSocket
    session_id: int
    seq: int = 0
    speed_ms: int = DEFAULT_SPEED_MS
    playback_state: str = "PAUSED"
    playback_task: Optional[asyncio.Task] = field(default=None, repr=False)
    speed_updated_event: asyncio.Event = field(default_factory=asyncio.Event, repr=False)

    def next_seq(self) -> int:
        self.seq += 1
        return self.seq


class ConnectionManager:
    def __init__(self, session_factory: Callable[[], Session] = SessionLocal):
        self.session_factory = session_factory
        # Map session_id -> list of independent ClientConnection instances (WS-10 multi-tab support)
        self.connections_by_session: dict[int, list[ClientConnection]] = {}
        # Legacy single-socket view retained for backward-compatible inspection
        self.active_connections: dict[int, WebSocket] = {}
        self.playback_tasks: dict[int, asyncio.Task] = {}

    async def connect(self, websocket: WebSocket, session_id: int) -> ClientConnection:
        await websocket.accept()
        conn = ClientConnection(websocket=websocket, session_id=session_id)
        bucket = self.connections_by_session.setdefault(session_id, [])
        bucket.append(conn)
        self.active_connections[session_id] = websocket
        return conn

    def disconnect(self, session_id: int, websocket: WebSocket):
        self.stop_connection_playback_by_ws(session_id, websocket)
        bucket = self.connections_by_session.get(session_id, [])
        was_in_bucket = any(c.websocket is websocket for c in bucket)
        self.connections_by_session[session_id] = [c for c in bucket if c.websocket is not websocket]
        if not self.connections_by_session.get(session_id):
            self.connections_by_session.pop(session_id, None)
            if self.active_connections.get(session_id) is websocket:
                self.active_connections.pop(session_id, None)
                self.stop_playback(session_id)
            elif was_in_bucket:
                self.playback_tasks.pop(session_id, None)
        elif self.active_connections.get(session_id) is websocket:
            self.active_connections[session_id] = self.connections_by_session[session_id][-1].websocket

    def stop_playback(self, session_id: int, conn: Optional[ClientConnection] = None):
        if conn is not None:
            if conn.playback_task is not None:
                conn.playback_task.cancel()
                conn.playback_task = None
            if self.playback_tasks.get(session_id) is not None and not any(
                c.playback_task is not None for c in self.connections_by_session.get(session_id, [])
            ):
                self.playback_tasks.pop(session_id, None)
            return

        for c in self.connections_by_session.get(session_id, []):
            if c.playback_task is not None:
                c.playback_task.cancel()
                c.playback_task = None
        task = self.playback_tasks.pop(session_id, None)
        if task is not None:
            task.cancel()

    def stop_connection_playback_by_ws(self, session_id: int, websocket: WebSocket):
        for c in self.connections_by_session.get(session_id, []):
            if c.websocket is websocket:
                self.stop_playback(session_id, c)

    async def send_frame(
        self,
        conn: ClientConnection,
        frame_type: str,
        current_index: int,
        payload: dict[str, Any],
        source_context: Optional[dict[str, Any]] = None,
        data: Optional[dict[str, Any]] = None,
    ) -> bool:
        seq = conn.next_seq()
        envelope: dict[str, Any] = {
            "type": frame_type,
            "seq": seq,
            "session_id": conn.session_id,
            "current_index": current_index,
            "playback_state": conn.playback_state,
            "speed_ms": conn.speed_ms,
            "payload": payload,
            "data": data if data is not None else payload,
        }
        if source_context is not None:
            envelope["source_context"] = source_context
        try:
            await conn.websocket.send_json(envelope)
            return True
        except Exception as e:
            logger.error(f"Error sending frame {frame_type} to session {conn.session_id}: {e}")
            self.disconnect(conn.session_id, conn.websocket)
            return False

    async def send_candle_update(
        self,
        session_id: int,
        candle_data: dict[str, Any],
        source_context: dict[str, Any],
        conn: Optional[ClientConnection] = None,
        current_index: int = 0,
    ) -> bool:
        targets = [conn] if conn is not None else list(self.connections_by_session.get(session_id, []))
        if not targets and conn is None:
            legacy_ws = self.active_connections.get(session_id)
            if legacy_ws is not None:
                try:
                    await legacy_ws.send_json(
                        {
                            "type": "new_candle",
                            "data": candle_data,
                            "source_context": source_context,
                        }
                    )
                    return True
                except Exception as e:
                    logger.error(f"Error sending candle to session {session_id}: {e}")
                    self.disconnect(session_id, legacy_ws)
                    return False
            return False

        sent_any = False
        for target in targets:
            if target is None:
                continue
            ok = await self.send_frame(
                target,
                frame_type="new_candle",
                current_index=current_index,
                payload={"candle": candle_data, **candle_data},
                source_context=source_context,
                data=candle_data,
            )
            sent_any = sent_any or ok
        return sent_any


manager = ConnectionManager()


def _is_session_completed(status: Optional[str]) -> bool:
    return str(status or "").strip().lower() in ("completed", "bankrupt")


def _read_session_snapshot(db: Session, session_id: int) -> tuple[Optional[ReplaySession], Optional[dict[str, Any]], Optional[dict[str, Any]]]:
    session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
    if not session:
        return None, None, None
    candles = ReplayService.get_candles(db, session_id)
    latest_candle = format_candle(candles[-1]) if candles else None
    source_ctx = ReplayService.source_context(db, session).model_dump(mode="json")
    return session, latest_candle, source_ctx


async def connection_playback_loop(conn: ClientConnection, session_factory: Callable[[], Session]):
    """Per-connection background task to push candles continuously at conn.speed_ms."""
    try:
        while True:
            db = session_factory()
            try:
                prev_session = db.query(ReplaySession).filter(ReplaySession.id == conn.session_id).first()
                if not prev_session:
                    conn.playback_state = "ERROR"
                    await manager.send_frame(
                        conn,
                        frame_type="error",
                        current_index=0,
                        payload={"code": "SESSION_NOT_FOUND", "message": f"Session {conn.session_id} not found"},
                    )
                    break

                prev_index = prev_session.current_index
                if _is_session_completed(prev_session.status):
                    conn.playback_state = "COMPLETED"
                    await manager.send_frame(
                        conn,
                        frame_type="completed",
                        current_index=prev_session.current_index,
                        payload={
                            "reason": "END_OF_DATA",
                            "current_index": prev_session.current_index,
                            "speed_ms": conn.speed_ms,
                        },
                    )
                    break

                session = ReplayService.next_candle(db, conn.session_id, 1)
                candles = ReplayService.get_candles(db, conn.session_id)
                source_ctx = ReplayService.source_context(db, session).model_dump(mode="json")

                if candles and session.current_index != prev_index:
                    latest_candle = candles[-1]
                    candle_dict = format_candle(latest_candle)
                    if _is_session_completed(session.status):
                        conn.playback_state = "COMPLETED"
                    await manager.send_candle_update(
                        conn.session_id,
                        candle_dict,
                        source_ctx,
                        conn=conn,
                        current_index=session.current_index,
                    )

                if _is_session_completed(session.status) or session.current_index == prev_index:
                    conn.playback_state = "COMPLETED"
                    await manager.send_frame(
                        conn,
                        frame_type="completed",
                        current_index=session.current_index,
                        payload={
                            "reason": "END_OF_DATA",
                            "current_index": session.current_index,
                            "speed_ms": conn.speed_ms,
                        },
                        source_context=source_ctx,
                    )
                    break
            finally:
                db.close()

            # Wait for next tick or immediate speed adjustment wakeup
            conn.speed_updated_event.clear()
            try:
                await asyncio.wait_for(conn.speed_updated_event.wait(), timeout=conn.speed_ms / 1000.0)
                # If speed was updated mid-wait, sleep the updated interval before next tick so ticks are never lost
                await asyncio.sleep(conn.speed_ms / 1000.0)
            except asyncio.TimeoutError:
                pass

    except asyncio.CancelledError:
        pass
    except Exception as e:
        logger.error(f"Error in playback loop for session {conn.session_id}: {e}")
    finally:
        if conn.playback_Task_is_current():
            conn.playback_task = None


def _attach_task_helper():
    def playback_Task_is_current(self: ClientConnection) -> bool:
        return self.playback_task is asyncio.current_task()
    setattr(ClientConnection, "playback_Task_is_current", playback_Task_is_current)


_attach_task_helper()


@router.websocket("/ws/replay/{session_id}")
async def websocket_replay_endpoint(websocket: WebSocket, session_id: int):
    conn = await manager.connect(websocket, session_id)
    session_factory = manager.session_factory
    try:
        while True:
            raw_text = await websocket.receive_text()

            # 1. Validate JSON syntax & object structure (WS-03)
            try:
                payload = json.loads(raw_text)
            except (json.JSONDecodeError, TypeError):
                await manager.send_frame(
                    conn,
                    frame_type="error",
                    current_index=0,
                    payload={"code": "INVALID_JSON", "message": "Malformed JSON payload"},
                )
                continue

            if not isinstance(payload, dict):
                await manager.send_frame(
                    conn,
                    frame_type="error",
                    current_index=0,
                    payload={"code": "INVALID_PAYLOAD", "message": "WebSocket payload must be a JSON object"},
                )
                continue

            action = payload.get("action")
            if not isinstance(action, str) or action not in VALID_ACTIONS:
                await manager.send_frame(
                    conn,
                    frame_type="error",
                    current_index=0,
                    payload={"code": "INVALID_ACTION", "message": f"Unsupported action: {action!r}"},
                )
                continue

            # 2. Heartbeat ping/pong
            if action == "ping":
                db = session_factory()
                try:
                    sess = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
                    cur_idx = sess.current_index if sess else 0
                finally:
                    db.close()
                await manager.send_frame(
                    conn,
                    frame_type="pong",
                    current_index=cur_idx,
                    payload={"ts": payload.get("ts"), "speed_ms": conn.speed_ms},
                )
                continue

            # 3. State sync / resume handshake (WS-01, WS-02)
            if action in ("sync", "resume"):
                db = session_factory()
                try:
                    sess, latest_candle, source_ctx = _read_session_snapshot(db, session_id)
                    if not sess:
                        await manager.send_frame(
                            conn,
                            frame_type="error",
                            current_index=0,
                            payload={"code": "SESSION_NOT_FOUND", "message": f"Session {session_id} not found"},
                        )
                        continue
                    if _is_session_completed(sess.status):
                        conn.playback_state = "COMPLETED"
                    cur_idx = sess.current_index
                finally:
                    db.close()

                await manager.send_frame(
                    conn,
                    frame_type="playback_state",
                    current_index=cur_idx,
                    payload={
                        "playback_state": conn.playback_state,
                        "speed_ms": conn.speed_ms,
                        "current_index": cur_idx,
                        "candle": latest_candle,
                    },
                    source_context=source_ctx,
                )
                continue

            # 4. Dynamic speed adjustment (WS-04, WS-05)
            if action == "set_speed":
                clamped_speed, was_clamped = clamp_speed_ms(
                    payload.get("speed"), payload.get("speed_multiplier")
                )
                conn.speed_ms = clamped_speed
                conn.speed_updated_event.set()
                db = session_factory()
                try:
                    sess = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
                    cur_idx = sess.current_index if sess else 0
                finally:
                    db.close()
                await manager.send_frame(
                    conn,
                    frame_type="playback_state",
                    current_index=cur_idx,
                    payload={
                        "playback_state": conn.playback_state,
                        "speed_ms": conn.speed_ms,
                        "clamped": was_clamped,
                        "current_index": cur_idx,
                    },
                )
                continue

            # 5. Start / Play (WS-01, WS-04, WS-09)
            if action in ("start", "play"):
                if "speed" in payload or "speed_multiplier" in payload:
                    clamped_speed, was_clamped = clamp_speed_ms(
                        payload.get("speed"), payload.get("speed_multiplier")
                    )
                    conn.speed_ms = clamped_speed
                else:
                    was_clamped = False

                db = session_factory()
                try:
                    sess = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
                    if not sess:
                        await manager.send_frame(
                            conn,
                            frame_type="error",
                            current_index=0,
                            payload={"code": "SESSION_NOT_FOUND", "message": f"Session {session_id} not found"},
                        )
                        continue
                    cur_idx = sess.current_index
                    sess_status = sess.status
                finally:
                    db.close()

                manager.stop_playback(session_id, conn)
                if _is_session_completed(sess_status):
                    conn.playback_state = "COMPLETED"
                    await manager.send_frame(
                        conn,
                        frame_type="completed",
                        current_index=cur_idx,
                        payload={
                            "reason": "END_OF_DATA",
                            "current_index": cur_idx,
                            "speed_ms": conn.speed_ms,
                        },
                    )
                    continue

                conn.playback_state = "PLAYING"
                await manager.send_frame(
                    conn,
                    frame_type="playback_state",
                    current_index=cur_idx,
                    payload={
                        "playback_state": conn.playback_state,
                        "speed_ms": conn.speed_ms,
                        "clamped": was_clamped,
                        "current_index": cur_idx,
                    },
                )
                task = asyncio.create_task(connection_playback_loop(conn, session_factory))
                conn.playback_task = task
                manager.playback_tasks[session_id] = task
                continue

            # 6. Pause
            if action == "pause":
                manager.stop_playback(session_id, conn)
                conn.playback_state = "PAUSED"
                db = session_factory()
                try:
                    sess = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
                    cur_idx = sess.current_index if sess else 0
                finally:
                    db.close()
                await manager.send_frame(
                    conn,
                    frame_type="playback_state",
                    current_index=cur_idx,
                    payload={
                        "playback_state": conn.playback_state,
                        "speed_ms": conn.speed_ms,
                        "current_index": cur_idx,
                    },
                )
                continue

            # 7. Next / Step single or multi-bar via WS
            if action in ("next", "step"):
                manager.stop_playback(session_id, conn)
                conn.playback_state = "PAUSED"
                steps = 1
                try:
                    steps = max(1, min(100, int(payload.get("steps", 1))))
                except (TypeError, ValueError):
                    steps = 1

                db = session_factory()
                try:
                    prev_sess = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
                    if not prev_sess:
                        await manager.send_frame(
                            conn,
                            frame_type="error",
                            current_index=0,
                            payload={"code": "SESSION_NOT_FOUND", "message": f"Session {session_id} not found"},
                        )
                        continue
                    prev_idx = prev_sess.current_index
                    session = ReplayService.next_candle(db, session_id, steps)
                    candles = ReplayService.get_candles(db, session_id)
                    source_ctx = ReplayService.source_context(db, session).model_dump(mode="json")
                    if candles and session.current_index != prev_idx:
                        latest_candle = candles[-1]
                        if _is_session_completed(session.status):
                            conn.playback_state = "COMPLETED"
                        await manager.send_candle_update(
                            session_id,
                            format_candle(latest_candle),
                            source_ctx,
                            conn=conn,
                            current_index=session.current_index,
                        )
                    if _is_session_completed(session.status) or session.current_index == prev_idx:
                        conn.playback_state = "COMPLETED"
                        await manager.send_frame(
                            conn,
                            frame_type="completed",
                            current_index=session.current_index,
                            payload={
                                "reason": "END_OF_DATA",
                                "current_index": session.current_index,
                                "speed_ms": conn.speed_ms,
                            },
                            source_context=source_ctx,
                        )
                finally:
                    db.close()

    except WebSocketDisconnect:
        manager.disconnect(session_id, websocket)
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
        manager.disconnect(session_id, websocket)
