from __future__ import annotations

import json
import os
import random
import sqlite3
import uuid
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from hub.db import DEFAULT_DB_PATH, get_connection, init_db
from hub.logic import (
    evaluate_biomon_alerts,
    ingest_telemetry_idempotent,
    log_meal_idempotent,
    recommend_rations,
)
from hub.seed import seed_if_empty

STATIC_DIR = Path(__file__).resolve().parent / "static"


class TelemetryPacket(BaseModel):
    event_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    subject_id: str
    heart_rate: float
    oxygen_level: float = 98.0
    suit_pressure: float = 1.0
    temperature: float = 22.0
    mission_phase: str = "ORBIT_IVA"


class MealRequest(BaseModel):
    event_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    subject_id: str
    food_id: str
    consumed_ratio: float = Field(ge=0.0, le=1.0)


class ConnectionManager:
    def __init__(self) -> None:
        self.hub_clients: list[WebSocket] = []

    async def connect_hub(self, ws: WebSocket) -> None:
        await ws.accept()
        self.hub_clients.append(ws)

    def disconnect_hub(self, ws: WebSocket) -> None:
        if ws in self.hub_clients:
            self.hub_clients.remove(ws)

    async def broadcast_hub(self, payload: dict[str, Any]) -> None:
        dead: list[WebSocket] = []
        message = json.dumps(payload)
        for ws in self.hub_clients:
            try:
                await ws.send_text(message)
            except Exception:
                dead.append(ws)
        for ws in dead:
            self.disconnect_hub(ws)


def create_app(db_path: Path | str | None = None, nasa_dir: Path | None = None) -> FastAPI:
    resolved_db = Path(db_path or os.environ.get("AEGIS_HUB_DB", DEFAULT_DB_PATH))
    resolved_nasa = nasa_dir or (Path(__file__).resolve().parent.parent / "data" / "NASA")
    manager = ConnectionManager()

    def get_db() -> sqlite3.Connection:
        return app.state.conn

    @asynccontextmanager
    async def lifespan(_app: FastAPI):
        conn = get_connection(resolved_db)
        init_db(conn)
        seed_if_empty(conn, nasa_dir=resolved_nasa)
        _app.state.conn = conn
        yield
        conn.close()

    app = FastAPI(title="AEGIS-BIO Mission Hub Terminal", version="0.1.0", lifespan=lifespan)
    if STATIC_DIR.exists():
        app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

    @app.get("/")
    def index():
        return FileResponse(STATIC_DIR / "index.html")

    @app.get("/api/v1/crew")
    def list_crew():
        conn = get_db()
        rows = conn.execute("SELECT * FROM astronauts ORDER BY subject_id").fetchall()
        crew = []
        for r in rows:
            alerts = evaluate_biomon_alerts(
                r["heart_rate"], r["oxygen_level"], r["suit_pressure"], r["mission_phase"]
            )
            crew.append({**dict(r), "alerts": alerts})
        return {"crew": crew}

    @app.get("/api/v1/crew/{subject_id}")
    def get_crew_profile(subject_id: str):
        conn = get_db()
        astro = conn.execute(
            "SELECT * FROM astronauts WHERE subject_id = ?", (subject_id,)
        ).fetchone()
        if not astro:
            raise HTTPException(status_code=404, detail="Astronaut not found")

        alerts = evaluate_biomon_alerts(
            astro["heart_rate"],
            astro["oxygen_level"],
            astro["suit_pressure"],
            astro["mission_phase"],
        )
        measurements = conn.execute(
            """
            SELECT sample_id, metric, unit, value, event, day_offset,
                   measured_at, source, source_file, quality_flag
            FROM measurements
            WHERE subject_id = ?
            ORDER BY CASE event WHEN 'launch' THEN 0 ELSE 1 END, day_offset ASC, metric ASC
            """,
            (subject_id,),
        ).fetchall()
        meals = conn.execute(
            """
            SELECT m.event_id, f.name AS food_name, m.consumed_ratio,
                   m.kcal_added, m.protein_added, m.sodium_added, m.logged_at
            FROM meal_logs m
            JOIN food_pantry f ON f.id = m.food_id
            WHERE m.subject_id = ?
            ORDER BY m.logged_at DESC
            LIMIT 10
            """,
            (subject_id,),
        ).fetchall()

        return {
            "profile": {**dict(astro), "alerts": alerts},
            "measurements": [dict(m) for m in measurements],
            "meal_logs": [dict(ml) for ml in meals],
            "ration_advice": recommend_rations(conn, subject_id),
        }

    @app.get("/api/v1/pantry")
    def get_pantry():
        conn = get_db()
        items = conn.execute("SELECT * FROM food_pantry ORDER BY id").fetchall()
        return {"items": [dict(i) for i in items]}

    @app.get("/api/v1/hub-only/radiation")
    def get_hub_only_radiation():
        conn = get_db()
        rows = conn.execute(
            "SELECT timestamp_raw, instrument_id, absorbed_dose_rate, unit, source_file FROM radiation_readings ORDER BY timestamp_raw ASC"
        ).fetchall()
        return {
            "scope": "HUB_ONLY_CREW_RESTRICTED",
            "note": "Tarihsel Liulin-5-D3 kesiti (2013-01-01). 1 Hz canlı akış değildir; aradaki ölçüm boşlukları korunur.",
            "readings": [dict(r) for r in rows],
        }

    @app.post("/api/v1/meals")
    async def create_meal_log(req: MealRequest):
        conn = get_db()
        try:
            result = log_meal_idempotent(
                conn, req.event_id, req.subject_id, req.food_id, req.consumed_ratio
            )
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc
        await manager.broadcast_hub({"type": "meal_updated", "subject_id": req.subject_id})
        return result

    @app.post("/api/v1/telemetry/sync")
    async def sync_telemetry(packets: list[TelemetryPacket]):
        conn = get_db()
        results = []
        for p in packets:
            res = ingest_telemetry_idempotent(
                conn,
                p.event_id,
                p.subject_id,
                p.heart_rate,
                p.oxygen_level,
                p.suit_pressure,
                p.temperature,
                p.mission_phase,
            )
            results.append(res)
        await manager.broadcast_hub({"type": "crew_updated"})
        return {"processed": len(results), "results": results}

    @app.post("/api/v1/simulate/tick")
    async def simulate_tick():
        conn = get_db()
        rows = conn.execute("SELECT * FROM astronauts").fetchall()
        for r in rows:
            delta_hr = random.uniform(-4.0, 5.0)
            new_hr = round(max(48.0, min(172.0, r["heart_rate"] + delta_hr)), 1)
            ingest_telemetry_idempotent(
                conn,
                str(uuid.uuid4()),
                r["subject_id"],
                new_hr,
                r["oxygen_level"],
                r["suit_pressure"],
                r["temperature"],
                r["mission_phase"],
            )
        await manager.broadcast_hub({"type": "crew_updated"})
        return {"status": "tick_applied"}

    @app.websocket("/ws/edge")
    async def ws_edge(ws: WebSocket):
        await ws.accept()
        conn = get_db()
        try:
            while True:
                raw = await ws.receive_text()
                data = json.loads(raw)
                packet = TelemetryPacket(**data)
                res = ingest_telemetry_idempotent(
                    conn,
                    packet.event_id,
                    packet.subject_id,
                    packet.heart_rate,
                    packet.oxygen_level,
                    packet.suit_pressure,
                    packet.temperature,
                    packet.mission_phase,
                )
                await ws.send_text(json.dumps(res))
                await manager.broadcast_hub({"type": "crew_updated", "subject_id": packet.subject_id})
        except WebSocketDisconnect:
            pass

    @app.websocket("/ws/hub-ui")
    async def ws_hub_ui(ws: WebSocket):
        await manager.connect_hub(ws)
        try:
            while True:
                await ws.receive_text()
        except WebSocketDisconnect:
            manager.disconnect_hub(ws)

    return app


app = create_app()
