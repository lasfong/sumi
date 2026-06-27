from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from sqlalchemy.orm import Session
from app.db import SessionLocal
from app.services.replay_service import ReplayService
import asyncio
import json
import logging

logger = logging.getLogger(__name__)
router = APIRouter()

class ConnectionManager:
    def __init__(self):
        self.active_connections: dict[int, WebSocket] = {}
        self.playback_tasks: dict[int, asyncio.Task] = {}

    async def connect(self, websocket: WebSocket, session_id: int):
        await websocket.accept()
        self.active_connections[session_id] = websocket

    def disconnect(self, session_id: int):
        if session_id in self.active_connections:
            del self.active_connections[session_id]
        self.stop_playback(session_id)

    def stop_playback(self, session_id: int):
        if session_id in self.playback_tasks:
            self.playback_tasks[session_id].cancel()
            del self.playback_tasks[session_id]

    async def send_candle_update(self, session_id: int, candle_data: dict):
        if session_id in self.active_connections:
            try:
                await self.active_connections[session_id].send_json({
                    "type": "new_candle",
                    "data": candle_data
                })
            except Exception as e:
                logger.error(f"Error sending candle to session {session_id}: {e}")
                self.disconnect(session_id)

manager = ConnectionManager()

def format_candle(candle):
    return {
        "time": int(candle.timestamp.timestamp()),
        "open": candle.open,
        "high": candle.high,
        "low": candle.low,
        "close": candle.close,
        "volume": candle.volume
    }

async def playback_loop(session_id: int, speed_ms: int):
    """Background task to push candles continuously"""
    db = SessionLocal()
    try:
        while True:
            # 1. Advance the session
            session = ReplayService.next_candle(db, session_id, 1)
            
            # 2. Fetch the latest candle
            candles = ReplayService.get_candles(db, session_id)
            if candles:
                latest_candle = candles[-1]
                candle_dict = format_candle(latest_candle)
                
                # 3. Send via WebSocket
                await manager.send_candle_update(session_id, candle_dict)
            
            # Stop if session is complete
            if session.status == "COMPLETED":
                break
                
            # Wait for next tick
            await asyncio.sleep(speed_ms / 1000.0)
            
    except asyncio.CancelledError:
        pass # Task cancelled by user
    except Exception as e:
        logger.error(f"Error in playback loop for session {session_id}: {e}")
    finally:
        db.close()

@router.websocket("/ws/replay/{session_id}")
async def websocket_replay_endpoint(websocket: WebSocket, session_id: int):
    await manager.connect(websocket, session_id)
    try:
        while True:
            data = await websocket.receive_text()
            payload = json.loads(data)
            action = payload.get("action")
            
            if action == "start":
                speed_ms = payload.get("speed", 1000)
                manager.stop_playback(session_id) # Stop existing task if any
                task = asyncio.create_task(playback_loop(session_id, speed_ms))
                manager.playback_tasks[session_id] = task
                
            elif action == "pause":
                manager.stop_playback(session_id)
                
            elif action == "next":
                manager.stop_playback(session_id) # Stop auto-play if running
                db = SessionLocal()
                try:
                    session = ReplayService.next_candle(db, session_id, 1)
                    candles = ReplayService.get_candles(db, session_id)
                    if candles:
                        latest_candle = candles[-1]
                        await manager.send_candle_update(session_id, format_candle(latest_candle))
                finally:
                    db.close()
                    
    except WebSocketDisconnect:
        manager.disconnect(session_id)
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
        manager.disconnect(session_id)
