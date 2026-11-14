#!/usr/bin/env python3
"""
ETL script: Normalizes local NASA dataset files into hub-desktop/data/nasa_seed.sql.

Rules enforced (from Uygulama_Rehberi.md & YENI_EKLENENLER.txt):
- Only *_TRANSFORMED.csv files are ingested for OSD-569/575/656 (never SUBMITTED.csv).
- Empty / NA / N/A cells are skipped, never coerced to 0.0.
- Historical/research data stays in reference_* / research_* tables, separate from live crew telemetry.
- Timepoints (e.g., L-44, L-3, R+1) get numeric sort_order for chronological ordering.
- RadLab interval dose is skipped across long data gaps (> 300s).
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
import re
from datetime import datetime
from pathlib import Path

SOURCE_ROOT = Path(r"C:\Users\Emir\Downloads\Yeni klasör (2)\Yeni klasör (2)")
NASA_ROOT = SOURCE_ROOT / "NASA"
REPO_ROOT = Path(__file__).resolve().parent.parent
OUT_SQL = REPO_ROOT / "hub-desktop" / "data" / "nasa_seed.sql"

NA_VALUES = {"", "NA", "N/A", "NAN", "NONE", "NULL", "-"}


def sql_str(val: str | None) -> str:
    if val is None:
        return "NULL"
    return "'" + val.replace("'", "''") + "'"


def sql_num(val: float | int | None) -> str:
    if val is None:
        return "NULL"
    return repr(val)


def parse_float(raw: str | None) -> float | None:
    if raw is None:
        return None
    s = raw.strip()
    if s.upper() in NA_VALUES:
        return None
    try:
        v = float(s)
        if not math.isfinite(v):
            return None
        return v
    except ValueError:
        return None


def file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_sample_name(sample_name: str) -> tuple[str, str, str, int]:
    """
    Examples:
      C001_whole-blood_L-3_cbc -> ('C001', 'whole-blood', 'L-3', -3)
      C001_serum_L-44          -> ('C001', 'serum', 'L-44', -44)
      C002_urine_R+1           -> ('C002', 'urine', 'R+1', 1001)
    """
    parts = sample_name.strip().split("_")
    subject = parts[0] if parts else sample_name.strip()
    matrix = parts[1] if len(parts) > 1 else "unknown"
    timepoint = "UNKNOWN"
    for p in parts[1:]:
        if re.match(r"^[LR][+-]\d+$", p, re.IGNORECASE) or re.match(r"^FD\d+$", p, re.IGNORECASE):
            timepoint = p.upper()
            break
    if timepoint == "UNKNOWN" and len(parts) >= 3:
        timepoint = parts[2]

    sort_order = 0
    m_l = re.match(r"^L([+-]?\d+)$", timepoint)
    m_fd = re.match(r"^FD(\d+)$", timepoint)
    m_r = re.match(r"^R([+-]?\d+)$", timepoint)
    if m_l:
        sort_order = int(m_l.group(1))
    elif m_fd:
        sort_order = 100 + int(m_fd.group(1))
    elif m_r:
        sort_order = 1000 + int(m_r.group(1))
    return subject, matrix, timepoint, sort_order


def split_metric_column(col: str) -> tuple[str, str, str] | None:
    """
    Classify wide-table column into (biomarker_key, role, unit).
    role in {'value', 'range_min', 'range_max'}.
    """
    c = col.strip()
    if not c or c == "Sample Name":
        return None
    if "_range_min" in c:
        idx = c.index("_range_min")
        marker = c[:idx]
        unit = c[idx + len("_range_min"):].lstrip("_")
        return marker, "range_min", unit
    if "_range_max" in c:
        idx = c.index("_range_max")
        marker = c[:idx]
        unit = c[idx + len("_range_max"):].lstrip("_")
        return marker, "range_max", unit
    if "_value" in c:
        idx = c.index("_value")
        marker = c[:idx]
        unit = c[idx + len("_value"):].lstrip("_")
        return marker, "value", unit
    if "_concentration" in c:
        idx = c.index("_concentration")
        marker = c[:idx]
        unit = c[idx + len("_concentration"):].lstrip("_")
        return marker, "value", unit
    if c.endswith("_percent"):
        return c[: -len("_percent")], "percent_aux", "percent"
    return None


def clean_unit(unit: str) -> str:
    replacements = {
        "cells_per_microliter": "cells/uL",
        "thousand_per_microliter": "10^3/uL",
        "million_per_microliter": "10^6/uL",
        "gram_per_deciliter": "g/dL",
        "grams_per_deciliter": "g/dL",
        "milligram_per_deciliter": "mg/dL",
        "millimol_per_liter": "mmol/L",
        "units_per_liter": "U/L",
        "picogram_per_milliliter": "pg/mL",
        "nanogram_per_milliliter": "ng/mL",
        "milliliter_per_minute_per_1.73_meter_squared": "mL/min/1.73m2",
        "percent": "%",
        "picogram": "pg",
        "femtoliter": "fL",
        "npq": "NPQ",
    }
    return replacements.get(unit, unit.replace("_", " ") if unit else "ratio")


def build_ephemeris(lines: list[str]) -> int:
    path = NASA_ROOT / "Horizons" / "Dunya_Mars_2026_11_14_21.csv"
    count = 0
    with path.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            dt = datetime.strptime(row["time_UT"].strip(), "%Y-%b-%d %H:%M")
            date_iso = dt.strftime("%Y-%m-%d")
            dist_au = float(row["apparent_range_au"])
            light_min = float(row["one_way_down_light_time_minutes"])
            lines.append(
                f"INSERT OR REPLACE INTO mission_ephemeris (mission_date, distance_au, one_way_light_min) "
                f"VALUES ({sql_str(date_iso)}, {sql_num(dist_au)}, {sql_num(light_min)});"
            )
            count += 1
    return count


def build_radlab(lines: list[str]) -> int:
    path = NASA_ROOT / "RadLab_2013_01_01.csv"
    count = 0
    prev_dt: datetime | None = None
    with path.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            ts = row["timestamp"].strip()
            inst = row["instrument_id"].strip()
            dose_rate = parse_float(row["absorbed_dose_rate"])
            if dose_rate is None:
                continue
            dt = datetime.fromisoformat(ts)
            interval_sec: int | None = None
            interval_dose: float | None = None
            if prev_dt is not None:
                delta = int((dt - prev_dt).total_seconds())
                if 0 < delta <= 300:
                    interval_sec = delta
                    interval_dose = round(dose_rate * delta / 3600.0, 6)
            prev_dt = dt
            lines.append(
                f"INSERT OR IGNORE INTO reference_radiation_series "
                f"(timestamp_utc, instrument_id, absorbed_dose_rate_ugy_hr, interval_sec, interval_dose_ugy) "
                f"VALUES ({sql_str(ts)}, {sql_str(inst)}, {sql_num(dose_rate)}, {sql_num(interval_sec)}, {sql_num(interval_dose)});"
            )
            count += 1
    return count


def build_eda(lines: list[str]) -> int:
    eda_dir = NASA_ROOT / "EDA"
    merged: dict[str, dict[str, float | None]] = {}

    def read_pair(fname: str, key_iss: str, key_gnd: str, col_iss: str, col_gnd: str) -> None:
        with (eda_dir / fname).open("r", encoding="utf-8-sig", newline="") as f:
            for row in csv.DictReader(f):
                t = row["time"].strip()
                entry = merged.setdefault(t, {})
                entry[key_iss] = parse_float(row.get(col_iss))
                entry[key_gnd] = parse_float(row.get(col_gnd))

    read_pair("RR1_co2_2014_09_25.csv", "co2_iss", "co2_gnd", "co2_iss", "co2_ground")
    read_pair("RR1_temperature_2014_09_25.csv", "temp_iss", "temp_gnd", "temperature_iss", "temperature_ground")
    read_pair("RR1_humidity_2014_09_25.csv", "hum_iss", "hum_gnd", "humidity_iss", "humidity_ground")

    count = 0
    for t in sorted(merged):
        d = merged[t]
        lines.append(
            "INSERT OR IGNORE INTO reference_eda_environment "
            "(recorded_at_utc, co2_iss_ppm, co2_ground_ppm, temp_iss_c, temp_ground_c, humidity_iss_pct, humidity_ground_pct) "
            f"VALUES ({sql_str(t)}, {sql_num(d.get('co2_iss'))}, {sql_num(d.get('co2_gnd'))}, "
            f"{sql_num(d.get('temp_iss'))}, {sql_num(d.get('temp_gnd'))}, "
            f"{sql_num(d.get('hum_iss'))}, {sql_num(d.get('hum_gnd'))});"
        )
        count += 1
    return count


def build_lab_panels(lines: list[str]) -> int:
    panels = [
        ("OSD-569", "CBC", NASA_ROOT / "OSD-569" / "LSDS-7_Complete_Blood_Count_CBC_TRANSFORMED.csv"),
        ("OSD-575", "CMP", NASA_ROOT / "OSD-575" / "LSDS-8_Comprehensive_Metabolic_Panel_CMP_TRANSFORMED.csv"),
        ("OSD-575", "SERUM_CARDIO", NASA_ROOT / "OSD-575" / "LSDS-8_Multiplex_serum_cardiovascular_EvePanel_TRANSFORMED.csv"),
        ("OSD-575", "SERUM_IMMUNE", NASA_ROOT / "OSD-575" / "LSDS-8_Multiplex_serum_immune_EvePanel_TRANSFORMED.csv"),
        ("OSD-575", "SERUM_ALAMAR", NASA_ROOT / "OSD-575" / "LSDS-8_Multiplex_serum.immune.AlamarPanel_TRANSFORMED.csv"),
        ("OSD-656", "URINE_IMMUNE", NASA_ROOT / "OSD-656" / "LSDS-64_Multiplex_urine.immune.AlamarPanel_TRANSFORMED.csv"),
    ]
    count = 0
    for study_id, domain, path in panels:
        with path.open("r", encoding="utf-8-sig", newline="") as f:
            reader = csv.reader(f)
            headers = next(reader)
            col_meta: dict[str, dict[str, tuple[int, str]]] = {}
            for idx, h in enumerate(headers):
                parsed = split_metric_column(h)
                if not parsed:
                    continue
                marker, role, unit = parsed
                if role == "percent_aux":
                    continue
                col_meta.setdefault(marker, {})[role] = (idx, clean_unit(unit))

            for row in reader:
                if not row or not row[0].strip():
                    continue
                sample_name = row[0].strip()
                subject, matrix, timepoint, sort_order = parse_sample_name(sample_name)
                for marker, roles in col_meta.items():
                    if "value" not in roles:
                        continue
                    val_idx, unit = roles["value"]
                    if val_idx >= len(row):
                        continue
                    val = parse_float(row[val_idx])
                    if val is None:
                        continue  # Never coerce NA/empty to 0.0
                    rmin = parse_float(row[roles["range_min"][0]]) if "range_min" in roles and roles["range_min"][0] < len(row) else None
                    rmax = parse_float(row[roles["range_max"][0]]) if "range_max" in roles and roles["range_max"][0] < len(row) else None

                    flag = "IN_RANGE"
                    if rmin is not None and val < rmin:
                        flag = "BELOW_REF"
                    elif rmax is not None and val > rmax:
                        flag = "ABOVE_REF"
                    elif rmin is None and rmax is None:
                        flag = "NO_REF"

                    lines.append(
                        "INSERT OR IGNORE INTO research_lab_observations "
                        "(study_id, domain, sample_name, subject_code, sample_matrix, timepoint, sort_order, "
                        "biomarker_name, measured_value, unit, ref_min, ref_max, status_flag, source_file) "
                        f"VALUES ({sql_str(study_id)}, {sql_str(domain)}, {sql_str(sample_name)}, {sql_str(subject)}, "
                        f"{sql_str(matrix)}, {sql_str(timepoint)}, {sort_order}, {sql_str(marker)}, "
                        f"{sql_num(val)}, {sql_str(unit)}, {sql_num(rmin)}, {sql_num(rmax)}, {sql_str(flag)}, {sql_str(path.name)});"
                    )
                    count += 1
    return count


def build_provenance(lines: list[str]) -> int:
    tracked_files = [
        ("Horizons", NASA_ROOT / "Horizons" / "Dunya_Mars_2026_11_14_21.csv", "https://ssd-api.jpl.nasa.gov/doc/horizons.html"),
        ("RadLab", NASA_ROOT / "RadLab_2013_01_01.csv", "https://visualization.osdr.nasa.gov/radlab/gui/data-api/"),
        ("EDA", NASA_ROOT / "EDA" / "RR1_co2_2014_09_25.csv", "https://visualization.osdr.nasa.gov/eda/api_reference"),
        ("EDA", NASA_ROOT / "EDA" / "RR1_temperature_2014_09_25.csv", "https://visualization.osdr.nasa.gov/eda/api_reference"),
        ("EDA", NASA_ROOT / "EDA" / "RR1_humidity_2014_09_25.csv", "https://visualization.osdr.nasa.gov/eda/api_reference"),
        ("OSD-569", NASA_ROOT / "OSD-569" / "LSDS-7_Complete_Blood_Count_CBC_TRANSFORMED.csv", "https://osdr.nasa.gov/bio/repo/data/studies/OSD-569"),
        ("OSD-575", NASA_ROOT / "OSD-575" / "LSDS-8_Comprehensive_Metabolic_Panel_CMP_TRANSFORMED.csv", "https://osdr.nasa.gov/bio/repo/data/studies/OSD-575"),
        ("OSD-575", NASA_ROOT / "OSD-575" / "LSDS-8_Multiplex_serum_cardiovascular_EvePanel_TRANSFORMED.csv", "https://osdr.nasa.gov/bio/repo/data/studies/OSD-575"),
        ("OSD-575", NASA_ROOT / "OSD-575" / "LSDS-8_Multiplex_serum_immune_EvePanel_TRANSFORMED.csv", "https://osdr.nasa.gov/bio/repo/data/studies/OSD-575"),
        ("OSD-575", NASA_ROOT / "OSD-575" / "LSDS-8_Multiplex_serum.immune.AlamarPanel_TRANSFORMED.csv", "https://osdr.nasa.gov/bio/repo/data/studies/OSD-575"),
        ("OSD-656", NASA_ROOT / "OSD-656" / "LSDS-64_Multiplex_urine.immune.AlamarPanel_TRANSFORMED.csv", "https://osdr.nasa.gov/bio/repo/data/studies/OSD-656"),
    ]
    now_utc = "2026-11-15T00:00:00Z"
    for ds, path, url in tracked_files:
        sha = file_sha256(path)
        sz = path.stat().st_size
        lines.append(
            "INSERT OR IGNORE INTO dataset_provenance "
            "(file_name, dataset_id, sha256, source_url, byte_size, imported_at_utc) "
            f"VALUES ({sql_str(path.name)}, {sql_str(ds)}, {sql_str(sha)}, {sql_str(url)}, {sz}, {sql_str(now_utc)});"
        )
    return len(tracked_files)


def main() -> None:
    lines: list[str] = [
        "-- Auto-generated by scripts/build_nasa_seed.py from NASA open datasets.",
        "BEGIN IMMEDIATE;",
    ]
    n_eph = build_ephemeris(lines)
    n_rad = build_radlab(lines)
    n_eda = build_eda(lines)
    n_lab = build_lab_panels(lines)
    n_prov = build_provenance(lines)
    lines.append("COMMIT;")

    OUT_SQL.parent.mkdir(parents=True, exist_ok=True)
    OUT_SQL.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(
        f"Generated {OUT_SQL} ({OUT_SQL.stat().st_size} bytes): "
        f"ephemeris={n_eph}, radlab={n_rad}, eda={n_eda}, lab_obs={n_lab}, provenance={n_prov}"
    )


if __name__ == "__main__":
    main()
