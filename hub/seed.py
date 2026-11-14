from __future__ import annotations

import csv
import sqlite3
from datetime import datetime, timezone
from pathlib import Path


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def seed_if_empty(conn: sqlite3.Connection, nasa_dir: Path | None = None) -> None:
    """Seed crew, simulated ISS pantry, and historical NASA/demo datasets if DB is empty."""
    crew_count = conn.execute("SELECT COUNT(*) FROM astronauts").fetchone()[0]
    if crew_count == 0:
        now = _now_iso()
        crew = [
            ("C001", "Cmdr. Alex Mercer", "Mission Commander", "ORBIT_IVA", 74.0, 98.4, 1.00, 21.8, 2600.0, 110.0, 2300.0, 1150.0, 48.0, 980.0, now),
            ("C002", "Dr. Elena Rostova", "Flight Surgeon / Payload", "ORBIT_IVA", 68.0, 99.1, 1.00, 22.1, 2200.0, 95.0, 2300.0, 920.0, 41.0, 810.0, now),
            ("C003", "Maj. Kenji Sato", "EVA Specialist", "EVA_SPACEWALK", 128.0, 96.5, 0.72, 23.4, 3100.0, 130.0, 2500.0, 1480.0, 62.0, 1290.0, now),
            ("C004", "Lt. Maya Lin", "Systems & Bio Engineer", "REENTRY_LANDING", 104.0, 97.8, 0.98, 22.6, 2400.0, 100.0, 3200.0, 1050.0, 44.0, 1650.0, now),
        ]
        conn.executemany(
            """
            INSERT INTO astronauts (
                subject_id, name, role, mission_phase, heart_rate, oxygen_level,
                suit_pressure, temperature, daily_kcal_target, daily_protein_target,
                daily_sodium_cap_mg, kcal_consumed, protein_consumed, sodium_consumed_mg,
                last_updated
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            crew,
        )

    pantry_count = conn.execute("SELECT COUNT(*) FROM food_pantry").fetchone()[0]
    if pantry_count == 0:
        pantry = [
            ("ISS-01", "Thermostabilized Beef Stew", "Entree", "220g pouch", 340.0, 26.0, 480.0, 14),
            ("ISS-02", "Rehydratable Teriyaki Chicken", "Entree", "180g pouch", 290.0, 28.0, 520.0, 11),
            ("ISS-03", "Space Tofu & Quinoa Bowl", "Entree", "200g pouch", 310.0, 22.0, 310.0, 9),
            ("ISS-04", "Freeze-Dried Scrambled Eggs", "Breakfast", "110g pouch", 240.0, 19.0, 390.0, 16),
            ("ISS-05", "Almond & Whey Recovery Bar", "Snack", "70g bar", 280.0, 21.0, 160.0, 22),
            ("ISS-06", "Low-Sodium Lentil Soup", "Side", "210g pouch", 210.0, 14.0, 190.0, 12),
            ("ISS-07", "Electrolyte NaCl Hydration Pack", "Countermeasure", "250ml + 1g NaCl", 45.0, 0.0, 393.0, 30),
            ("ISS-08", "Freeze-Dried Strawberries & Granola", "Breakfast", "95g pouch", 260.0, 8.0, 95.0, 15),
        ]
        conn.executemany(
            """
            INSERT INTO food_pantry (id, name, category, serving_size, kcal, protein_g, sodium_mg, stock_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            pantry,
        )

    meas_count = conn.execute("SELECT COUNT(*) FROM measurements").fetchone()[0]
    if meas_count == 0:
        loaded_from_csv = _try_load_nasa_csvs(conn, nasa_dir)
        if not loaded_from_csv:
            sample_measurements = [
                ("C001", "C001_whole-blood_L-3_cbc", "absolute_basophils", "cells/uL", 49.0, "launch", -3, None, "OSD-569", "CBC_TRANSFORMED.csv (demo)", "ok"),
                ("C001", "C001_whole-blood_L-3_cbc", "white_blood_cells", "10^3/uL", 6.2, "launch", -3, None, "OSD-569", "CBC_TRANSFORMED.csv (demo)", "ok"),
                ("C001", "C001_whole-blood_R+1_cbc", "white_blood_cells", "10^3/uL", 9.8, "return", 1, None, "OSD-569", "CBC_TRANSFORMED.csv (demo)", "review"),
                ("C001", "C001_serum_L-3", "albumin", "g/dL", 4.9, "launch", -3, None, "OSD-575", "CMP_TRANSFORMED.csv (demo)", "ok"),
                ("C001", "C001_serum_R+1", "albumin", "g/dL", 4.4, "return", 1, None, "OSD-575", "CMP_TRANSFORMED.csv (demo)", "ok"),
                ("C002", "C002_serum_L-3", "albumin", "g/dL", 4.6, "launch", -3, None, "OSD-575", "CMP_TRANSFORMED.csv (demo)", "ok"),
                ("C002", "C002_whole-blood_L-3_cbc", "hemoglobin", "g/dL", 13.8, "launch", -3, None, "OSD-569", "CBC_TRANSFORMED.csv (demo)", "ok"),
                ("C003", "C003_whole-blood_L-3_cbc", "hemoglobin", "g/dL", 15.1, "launch", -3, None, "OSD-569", "CBC_TRANSFORMED.csv (demo)", "ok"),
                ("C003", "C003_serum_R+1", "potassium", "mmol/L", None, "return", 1, None, "OSD-575", "CMP_TRANSFORMED.csv (demo)", "missing"),
                ("C004", "C004_serum_L-3", "sodium", "mmol/L", 141.0, "launch", -3, None, "OSD-575", "CMP_TRANSFORMED.csv (demo)", "ok"),
                ("C004", "C004_serum_R+1", "sodium", "mmol/L", 138.0, "return", 1, None, "OSD-575", "CMP_TRANSFORMED.csv (demo)", "ok"),
            ]
            conn.executemany(
                """
                INSERT INTO measurements (
                    subject_id, sample_id, metric, unit, value, event,
                    day_offset, measured_at, source, source_file, quality_flag
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                sample_measurements,
            )

    rad_count = conn.execute("SELECT COUNT(*) FROM radiation_readings").fetchone()[0]
    if rad_count == 0:
        loaded_rad = _try_load_radlab_csv(conn, nasa_dir)
        if not loaded_rad:
            # Representative non-1Hz Liulin-5-D3 slice (gaps of 90s - 240s preserved)
            sample_rad = [
                ("2013-01-01T00:00:13", "Liulin-5-D3", 3.5311, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:01:43", "Liulin-5-D3", 4.1200, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:03:13", "Liulin-5-D3", 12.8400, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:07:13", "Liulin-5-D3", 148.6200, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:08:43", "Liulin-5-D3", 369.1124, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:11:03", "Liulin-5-D3", 86.4100, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:12:33", "Liulin-5-D3", 5.9100, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
                ("2013-01-01T00:14:03", "Liulin-5-D3", 0.8400, "uGy/saat", "RadLab_2013_01_01.csv (demo)"),
            ]
            conn.executemany(
                """
                INSERT INTO radiation_readings (timestamp_raw, instrument_id, absorbed_dose_rate, unit, source_file)
                VALUES (?, ?, ?, ?, ?)
                """,
                sample_rad,
            )

    conn.commit()


def _try_load_nasa_csvs(conn: sqlite3.Connection, nasa_dir: Path | None) -> bool:
    """Placeholder hook for real OSD-569/OSD-575 TRANSFORMED CSV ingestion if local NASA dir exists."""
    if not nasa_dir or not nasa_dir.exists():
        return False
    return False


def _try_load_radlab_csv(conn: sqlite3.Connection, nasa_dir: Path | None) -> bool:
    if not nasa_dir:
        return False
    rad_path = nasa_dir / "RadLab_2013_01_01.csv"
    if not rad_path.exists():
        return False
    rows = []
    with rad_path.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            ts = row.get("timestamp")
            rate_raw = row.get("absorbed_dose_rate")
            if not ts or rate_raw in (None, "", "NA"):
                continue
            rows.append((ts, "Liulin-5-D3", float(rate_raw), "uGy/saat", rad_path.name))
    if not rows:
        return False
    conn.executemany(
        """
        INSERT INTO radiation_readings (timestamp_raw, instrument_id, absorbed_dose_rate, unit, source_file)
        VALUES (?, ?, ?, ?, ?)
        """,
        rows,
    )
    return True
