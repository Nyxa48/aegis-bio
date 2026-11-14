from __future__ import annotations

import sqlite3
from datetime import datetime, timezone
from typing import Any


def evaluate_biomon_alerts(
    heart_rate: float,
    oxygen_level: float,
    suit_pressure: float,
    mission_phase: str,
) -> list[dict[str, str]]:
    """Evaluate phase-aware physiological alerts (aligned with NASA-STD-3001 & Biomon rules)."""
    alerts: list[dict[str, str]] = []
    if oxygen_level < 20.0:
        alerts.append({"level": "CRITICAL", "message": "DÜŞÜK OKSİJEN!"})
    if suit_pressure < 0.7:
        alerts.append({"level": "CRITICAL", "message": "BASINÇ DÜŞÜŞÜ!"})

    hr_limit = 160.0 if mission_phase in {"EVA_SPACEWALK", "LAUNCH_ASCENT"} else 140.0
    if heart_rate > hr_limit:
        alerts.append({"level": "WARNING", "message": f"YÜKSEK NABIZ (>{hr_limit:.0f} BPM)"})
    elif heart_rate < 45.0:
        alerts.append({"level": "WARNING", "message": "DÜŞÜK NABIZ (<45 BPM)"})
    return alerts


def recommend_rations(conn: sqlite3.Connection, subject_id: str) -> dict[str, Any]:
    """Rank available ISS rations within remaining kcal/protein and sodium budget."""
    astro = conn.execute(
        "SELECT * FROM astronauts WHERE subject_id = ?", (subject_id,)
    ).fetchone()
    if not astro:
        return {"remaining_kcal": 0, "remaining_protein": 0, "remaining_sodium_mg": 0, "recommendations": []}

    rem_kcal = max(0.0, astro["daily_kcal_target"] - astro["kcal_consumed"])
    rem_protein = max(0.0, astro["daily_protein_target"] - astro["protein_consumed"])
    rem_sodium = max(0.0, astro["daily_sodium_cap_mg"] - astro["sodium_consumed_mg"])

    items = conn.execute(
        "SELECT * FROM food_pantry WHERE stock_count > 0 ORDER BY protein_g DESC"
    ).fetchall()

    candidates: list[dict[str, Any]] = []
    for item in items:
        if item["sodium_mg"] > rem_sodium:
            continue
        score = 0.0
        if rem_kcal > 0:
            score += min(item["kcal"], rem_kcal) / max(rem_kcal, 1.0)
        if rem_protein > 0:
            score += 1.5 * (min(item["protein_g"], rem_protein) / max(rem_protein, 1.0))
        candidates.append(
            {
                "id": item["id"],
                "name": item["name"],
                "category": item["category"],
                "serving_size": item["serving_size"],
                "kcal": item["kcal"],
                "protein_g": item["protein_g"],
                "sodium_mg": item["sodium_mg"],
                "stock_count": item["stock_count"],
                "fit_score": round(score, 3),
            }
        )

    candidates.sort(key=lambda x: x["fit_score"], reverse=True)
    return {
        "remaining_kcal": round(rem_kcal, 1),
        "remaining_protein": round(rem_protein, 1),
        "remaining_sodium_mg": round(rem_sodium, 1),
        "recommendations": candidates[:4],
        "no_solution": len(candidates) == 0,
    }


def log_meal_idempotent(
    conn: sqlite3.Connection,
    event_id: str,
    subject_id: str,
    food_id: str,
    consumed_ratio: float,
) -> dict[str, Any]:
    """Log meal consumption idempotently using event_id so offline replay never double-counts."""
    existing = conn.execute(
        "SELECT * FROM meal_logs WHERE event_id = ?", (event_id,)
    ).fetchone()
    if existing:
        return {"status": "duplicate_ignored", "event_id": event_id}

    food = conn.execute("SELECT * FROM food_pantry WHERE id = ?", (food_id,)).fetchone()
    if not food:
        raise ValueError(f"Food item not found: {food_id}")
    if food["stock_count"] <= 0:
        raise ValueError(f"Out of stock: {food['name']}")

    kcal_added = round(food["kcal"] * consumed_ratio, 2)
    protein_added = round(food["protein_g"] * consumed_ratio, 2)
    sodium_added = round(food["sodium_mg"] * consumed_ratio, 2)
    now = datetime.now(timezone.utc).isoformat()

    conn.execute(
        """
        INSERT INTO meal_logs (
            event_id, subject_id, food_id, consumed_ratio,
            kcal_added, protein_added, sodium_added, logged_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (event_id, subject_id, food_id, consumed_ratio, kcal_added, protein_added, sodium_added, now),
    )
    conn.execute(
        "UPDATE food_pantry SET stock_count = stock_count - 1 WHERE id = ?",
        (food_id,),
    )
    conn.execute(
        """
        UPDATE astronauts
        SET kcal_consumed = kcal_consumed + ?,
            protein_consumed = protein_consumed + ?,
            sodium_consumed_mg = sodium_consumed_mg + ?,
            last_updated = ?
        WHERE subject_id = ?
        """,
        (kcal_added, protein_added, sodium_added, now, subject_id),
    )
    conn.commit()
    return {
        "status": "logged",
        "event_id": event_id,
        "kcal_added": kcal_added,
        "protein_added": protein_added,
        "sodium_added": sodium_added,
    }


def ingest_telemetry_idempotent(
    conn: sqlite3.Connection,
    event_id: str,
    subject_id: str,
    heart_rate: float,
    oxygen_level: float,
    suit_pressure: float,
    temperature: float,
    mission_phase: str,
) -> dict[str, Any]:
    """Ingest live or replayed Biomon telemetry without duplicating event_id."""
    existing = conn.execute(
        "SELECT 1 FROM telemetry_events WHERE event_id = ?", (event_id,)
    ).fetchone()
    if existing:
        return {"status": "duplicate_ignored", "event_id": event_id}

    now = datetime.now(timezone.utc).isoformat()
    conn.execute(
        """
        INSERT INTO telemetry_events (
            event_id, subject_id, heart_rate, oxygen_level,
            suit_pressure, temperature, mission_phase, recorded_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (event_id, subject_id, heart_rate, oxygen_level, suit_pressure, temperature, mission_phase, now),
    )
    conn.execute(
        """
        UPDATE astronauts
        SET heart_rate = ?,
            oxygen_level = ?,
            suit_pressure = ?,
            temperature = ?,
            mission_phase = ?,
            last_updated = ?
        WHERE subject_id = ?
        """,
        (heart_rate, oxygen_level, suit_pressure, temperature, mission_phase, now, subject_id),
    )
    conn.commit()
    return {"status": "accepted", "event_id": event_id, "subject_id": subject_id}
