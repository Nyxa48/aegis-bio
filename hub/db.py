from __future__ import annotations

import sqlite3
from pathlib import Path

DEFAULT_DB_PATH = Path(__file__).resolve().parent.parent / "data" / "aegis_hub.db"


def get_connection(db_path: Path | str = DEFAULT_DB_PATH) -> sqlite3.Connection:
    path = Path(db_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(path), check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON;")
    return conn


def init_db(conn: sqlite3.Connection) -> None:
    """Initialize SQLite tables following the AEGIS-BIO data specification."""
    conn.executescript(
        """
        CREATE TABLE IF NOT EXISTS astronauts (
            subject_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            mission_phase TEXT NOT NULL,
            heart_rate REAL NOT NULL,
            oxygen_level REAL NOT NULL,
            suit_pressure REAL NOT NULL,
            temperature REAL NOT NULL,
            daily_kcal_target REAL NOT NULL,
            daily_protein_target REAL NOT NULL,
            daily_sodium_cap_mg REAL NOT NULL,
            kcal_consumed REAL NOT NULL DEFAULT 0.0,
            protein_consumed REAL NOT NULL DEFAULT 0.0,
            sodium_consumed_mg REAL NOT NULL DEFAULT 0.0,
            last_updated TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS measurements (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subject_id TEXT NOT NULL,
            sample_id TEXT NOT NULL,
            metric TEXT NOT NULL,
            unit TEXT NOT NULL,
            value REAL,
            event TEXT NOT NULL,
            day_offset INTEGER NOT NULL,
            measured_at TEXT,
            source TEXT NOT NULL,
            source_file TEXT NOT NULL,
            quality_flag TEXT NOT NULL DEFAULT 'ok'
        );

        CREATE TABLE IF NOT EXISTS radiation_readings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_raw TEXT NOT NULL,
            instrument_id TEXT NOT NULL,
            absorbed_dose_rate REAL NOT NULL,
            unit TEXT NOT NULL,
            source_file TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS food_pantry (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            serving_size TEXT NOT NULL,
            kcal REAL NOT NULL,
            protein_g REAL NOT NULL,
            sodium_mg REAL NOT NULL,
            stock_count INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS meal_logs (
            event_id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL,
            food_id TEXT NOT NULL,
            consumed_ratio REAL NOT NULL,
            kcal_added REAL NOT NULL,
            protein_added REAL NOT NULL,
            sodium_added REAL NOT NULL,
            logged_at TEXT NOT NULL,
            FOREIGN KEY (subject_id) REFERENCES astronauts(subject_id),
            FOREIGN KEY (food_id) REFERENCES food_pantry(id)
        );

        CREATE TABLE IF NOT EXISTS telemetry_events (
            event_id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL,
            heart_rate REAL NOT NULL,
            oxygen_level REAL NOT NULL,
            suit_pressure REAL NOT NULL,
            temperature REAL NOT NULL,
            mission_phase TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        );
        """
    )
    conn.commit()
