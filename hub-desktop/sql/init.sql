-- ============================================================================
-- AEGIS-BIO: TERMINAL HUB DATABASE SCHEMA (hub_init.sql -> aegis_hub.db)
-- Target: Mission Control / Habitat Terminal (Pure Rust Desktop App)
-- ============================================================================

PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

-- 1. Master Crew Roster
CREATE TABLE IF NOT EXISTS crew_members (
    astronaut_id       TEXT PRIMARY KEY,
    full_name          TEXT NOT NULL,
    role               TEXT NOT NULL,
    weight             REAL NOT NULL,                  -- Body weight in kg
    height             REAL NOT NULL,                  -- Height in cm
    age                INTEGER NOT NULL DEFAULT 35,    -- Age in years
    sex                TEXT NOT NULL DEFAULT 'M' CHECK (sex IN ('M', 'F')),
    active_phase_code  TEXT NOT NULL,                  -- LAUNCH, ORBIT, EVA, REENTRY
    lan_status         TEXT NOT NULL,                  -- ONLINE, BLACKOUT_BUFFERED
    last_heartbeat_utc TEXT NOT NULL,
    mission_start_utc  TEXT NOT NULL,
    sleep_target_min   INTEGER NOT NULL DEFAULT 480
);

-- 2. Persistent Multi-Crew Telemetry History (Full Parity with Biomon)
CREATE TABLE IF NOT EXISTS telemetry_history (
    record_id          INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id       TEXT NOT NULL,
    phase_code         TEXT NOT NULL,
    mission_day        INTEGER NOT NULL,
    edge_seq_id        INTEGER NOT NULL,
    recorded_at_utc    TEXT NOT NULL,
    received_at_utc    TEXT NOT NULL,
    hr_bpm             REAL NOT NULL,
    sbp_mmhg           REAL NOT NULL,
    resp_brpm          REAL NOT NULL,
    spo2_pct           REAL NOT NULL,
    tissue_o2_pct      REAL,
    blood_n2_idx       REAL,
    g_load             REAL NOT NULL,
    imu_activity_idx   REAL NOT NULL DEFAULT 0.0,
    cabin_co2_ppm      REAL NOT NULL,
    cabin_temp_c       REAL,
    cabin_humidity_pct REAL,
    cabin_pressure_kpa REAL,
    cabin_o2_pct       REAL,
    cabin_motion_idx   REAL NOT NULL DEFAULT 0.0,
    suit_pressure_kpa  REAL,
    suit_humidity_pct  REAL,
    eva_elapsed_sec    INTEGER NOT NULL DEFAULT 0,
    radiation_usv_hr   REAL NOT NULL,
    shock_index        REAL NOT NULL,
    orthostatic_risk   TEXT NOT NULL, -- LOW, MODERATE, HIGH
    sans_risk          TEXT NOT NULL,
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

CREATE INDEX IF NOT EXISTS idx_hub_telemetry_recorded
    ON telemetry_history(recorded_at_utc);

CREATE UNIQUE INDEX IF NOT EXISTS idx_hub_telemetry_dedup
    ON telemetry_history(astronaut_id, edge_seq_id);

CREATE INDEX IF NOT EXISTS idx_hub_telemetry_day
    ON telemetry_history(astronaut_id, mission_day);

-- 3. ISS Space Food Catalog
CREATE TABLE IF NOT EXISTS iss_food_catalog (
    food_id       TEXT PRIMARY KEY,
    item_name     TEXT NOT NULL,
    category      TEXT NOT NULL, -- ENTREE, SNACK, REHYDRATION, SUPPLEMENT
    kcal          REAL NOT NULL,
    protein_g     REAL NOT NULL,
    sodium_mg     REAL NOT NULL,
    potassium_mg  REAL NOT NULL,
    calcium_mg    REAL NOT NULL,
    vit_d3_iu     REAL NOT NULL,
    fluid_ml      REAL NOT NULL,
    is_low_sodium INTEGER NOT NULL
);

-- 4. Synced Crew Manual Input History (Destination for Biomon crew_input_queue)
CREATE TABLE IF NOT EXISTS crew_input_history (
    record_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id    TEXT NOT NULL,
    edge_input_id   INTEGER NOT NULL,
    logged_at_utc   TEXT NOT NULL,
    mission_day     INTEGER NOT NULL,
    input_type      TEXT NOT NULL, -- MEAL_LOG, FLUID_LOG, MEDICATION, INJURY, EXERCISE, ISOLATION_SURVEY
    reference_code  TEXT,
    consumption_pct INTEGER,
    notes           TEXT,
    received_at_utc TEXT NOT NULL,
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_hub_inputs_dedup
    ON crew_input_history(astronaut_id, edge_input_id);

CREATE INDEX IF NOT EXISTS idx_hub_inputs_day
    ON crew_input_history(astronaut_id, mission_day);

-- 5. Flight Surgeon Medical & Incident Log
CREATE TABLE IF NOT EXISTS surgeon_incidents (
    incident_id             INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id            TEXT NOT NULL,
    telemetry_record_id     INTEGER,
    created_at_utc          TEXT NOT NULL,
    hazard_type             TEXT NOT NULL, -- ORTHOSTATIC_RISK, RADIATION_SPIKE, HYPOXIA, INJURY, DECOMP
    injury_details          TEXT,
    medication_administered TEXT,
    medical_notes           TEXT,
    surgeon_ack             INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id),
    FOREIGN KEY (telemetry_record_id) REFERENCES telemetry_history(record_id)
);

-- 6. Sleep Epoch History (Synced 60s Actigraphy Windows)
CREATE TABLE IF NOT EXISTS sleep_epoch_history (
    record_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id     TEXT NOT NULL,
    edge_epoch_id    INTEGER NOT NULL,
    epoch_start_utc  TEXT NOT NULL,
    mission_day      INTEGER NOT NULL,
    activity_count   REAL NOT NULL,
    hr_bpm_avg       REAL,
    sleep_state      TEXT NOT NULL CHECK (sleep_state IN ('SLEEP', 'WAKE')),
    received_at_utc  TEXT NOT NULL,
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_hub_sleep_dedup
    ON sleep_epoch_history(astronaut_id, edge_epoch_id);

CREATE INDEX IF NOT EXISTS idx_hub_sleep_day
    ON sleep_epoch_history(astronaut_id, mission_day);

-- 7. Daily Sleep Summary
CREATE TABLE IF NOT EXISTS sleep_daily_summary (
    astronaut_id         TEXT NOT NULL,
    mission_day          INTEGER NOT NULL,
    total_sleep_min      REAL NOT NULL,
    target_sleep_min     INTEGER NOT NULL,
    sleep_efficiency_pct REAL,
    awakenings           INTEGER,
    computed_at_utc      TEXT NOT NULL,
    PRIMARY KEY (astronaut_id, mission_day),
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

-- 8. Daily SANS History
CREATE TABLE IF NOT EXISTS sans_daily_history (
    astronaut_id        TEXT NOT NULL,
    mission_day         INTEGER NOT NULL,
    sodium_mg_total     REAL NOT NULL,
    avg_cabin_co2_ppm   REAL NOT NULL,
    max_cabin_co2_ppm   REAL NOT NULL,
    sans_score          REAL NOT NULL,
    sans_risk           TEXT NOT NULL CHECK (sans_risk IN ('LOW', 'MODERATE', 'HIGH')),
    computed_at_utc     TEXT NOT NULL,
    received_at_utc     TEXT NOT NULL,
    PRIMARY KEY (astronaut_id, mission_day),
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

-- 9. Daily Osteoporosis History
CREATE TABLE IF NOT EXISTS osteoporosis_daily_history (
    astronaut_id       TEXT NOT NULL,
    mission_day        INTEGER NOT NULL,
    exercise_load_idx  REAL NOT NULL,
    calcium_mg_total   REAL NOT NULL,
    vit_d3_iu_total    REAL NOT NULL,
    protection_score   REAL NOT NULL,
    osteo_risk         TEXT NOT NULL CHECK (osteo_risk IN ('LOW', 'MODERATE', 'HIGH')),
    computed_at_utc    TEXT NOT NULL,
    received_at_utc    TEXT NOT NULL,
    PRIMARY KEY (astronaut_id, mission_day),
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

-- 10. Earth-Mars Ephemeris Lookup (Module #3: NASA Horizons CSV)
CREATE TABLE IF NOT EXISTS mission_ephemeris (
    mission_date      TEXT PRIMARY KEY, -- YYYY-MM-DD
    distance_au       REAL NOT NULL,
    one_way_light_min REAL NOT NULL
);

-- 11. Batch Lab & Microbiome Reports (Modules #6 Immune & #10 Microbes)
CREATE TABLE IF NOT EXISTS hub_lab_reports (
    report_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id   TEXT,             -- NULL for cabin surface swabs (OSD-573)
    source_dataset TEXT NOT NULL,    -- OSD-575, OSD-656, OSD-570, OSD-573, OSD-280
    domain         TEXT NOT NULL CHECK (domain IN ('IMMUNE', 'MICROBE')),
    sample_date    TEXT NOT NULL,
    biomarker_name TEXT NOT NULL,
    measured_value REAL NOT NULL,
    unit           TEXT NOT NULL,
    status_flag    TEXT NOT NULL CHECK (status_flag IN ('NOMINAL', 'ELEVATED', 'CRITICAL')),
    FOREIGN KEY (astronaut_id) REFERENCES crew_members(astronaut_id)
);

-- 12. Historical Reference Radiation Series (NASA/RadLab_2013_01_01.csv)
CREATE TABLE IF NOT EXISTS reference_radiation_series (
    timestamp_utc              TEXT NOT NULL,
    instrument_id              TEXT NOT NULL,
    absorbed_dose_rate_ugy_hr  REAL NOT NULL,
    interval_sec               INTEGER,
    interval_dose_ugy          REAL,
    PRIMARY KEY (timestamp_utc, instrument_id)
);

-- 13. Historical Reference ISS vs Ground Environment (NASA/EDA RR-1 2014-09-25)
CREATE TABLE IF NOT EXISTS reference_eda_environment (
    recorded_at_utc    TEXT PRIMARY KEY,
    co2_iss_ppm        REAL,
    co2_ground_ppm     REAL,
    temp_iss_c         REAL,
    temp_ground_c      REAL,
    humidity_iss_pct   REAL,
    humidity_ground_pct REAL
);

-- 14. Historical Research Lab Observations (Unpivoted OSD-569/575/656 TRANSFORMED.csv)
CREATE TABLE IF NOT EXISTS research_lab_observations (
    obs_id          INTEGER PRIMARY KEY AUTOINCREMENT,
    study_id        TEXT NOT NULL,
    domain          TEXT NOT NULL, -- CBC, CMP, SERUM_CARDIO, SERUM_IMMUNE, SERUM_ALAMAR, URINE_IMMUNE
    sample_name     TEXT NOT NULL,
    subject_code    TEXT NOT NULL,
    sample_matrix   TEXT NOT NULL,
    timepoint       TEXT NOT NULL,
    sort_order      INTEGER NOT NULL,
    biomarker_name  TEXT NOT NULL,
    measured_value  REAL NOT NULL,
    unit            TEXT NOT NULL,
    ref_min         REAL,
    ref_max         REAL,
    status_flag     TEXT NOT NULL CHECK (status_flag IN ('IN_RANGE', 'BELOW_REF', 'ABOVE_REF', 'NO_REF')),
    source_file     TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_research_lab_dedup
    ON research_lab_observations(study_id, domain, sample_name, biomarker_name);

CREATE INDEX IF NOT EXISTS idx_research_lab_subject_time
    ON research_lab_observations(subject_code, sort_order);

-- 15. Dataset Provenance & Idempotency Manifest
CREATE TABLE IF NOT EXISTS dataset_provenance (
    file_name       TEXT PRIMARY KEY,
    dataset_id      TEXT NOT NULL,
    sha256          TEXT NOT NULL,
    source_url      TEXT NOT NULL,
    byte_size       INTEGER NOT NULL,
    imported_at_utc TEXT NOT NULL
);
