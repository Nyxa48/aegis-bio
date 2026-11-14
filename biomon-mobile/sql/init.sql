-- ============================================================================
-- AEGIS-BIO: BIOMON EDGE DATABASE SCHEMA (biomon_init.sql -> aegis_biomon.db)
-- Target: Rust Local Node (rusqlite) on Astronaut Arm Tablet
-- ============================================================================

PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

-- 1. Astronaut Profile & Active State
CREATE TABLE IF NOT EXISTS astronaut_config (
    astronaut_id      TEXT PRIMARY KEY,
    callsign          TEXT NOT NULL,
    weight            REAL NOT NULL,                  -- Body weight in kg
    height            REAL NOT NULL,                  -- Height in cm
    age               INTEGER NOT NULL DEFAULT 35,    -- Age in years (Keytel & max HR)
    sex               TEXT NOT NULL DEFAULT 'M' CHECK (sex IN ('M', 'F')),
    active_phase      TEXT NOT NULL,                  -- LAUNCH, ORBIT, EVA, REENTRY
    mission_start_utc TEXT NOT NULL,                  -- Mission start reference for mission_day
    sleep_target_min  INTEGER NOT NULL DEFAULT 480    -- Daily sleep target (minutes)
);

-- 2. 1 Hz Local Telemetry Ring Buffer
CREATE TABLE IF NOT EXISTS telemetry_ring_buffer (
    seq_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id        TEXT NOT NULL,
    recorded_at_utc     TEXT NOT NULL,
    mission_day         INTEGER NOT NULL,
    mission_phase       TEXT NOT NULL,
    hr_bpm              REAL NOT NULL,
    sbp_mmhg            REAL NOT NULL,
    resp_brpm           REAL NOT NULL,
    spo2_pct            REAL NOT NULL,
    tissue_o2_pct       REAL,                         -- Hypoxia module (#11)
    blood_n2_idx        REAL,                         -- Decompression R-value (#9)
    g_load              REAL NOT NULL,                -- Static/dynamic G-load (#4)
    imu_activity_idx    REAL NOT NULL DEFAULT 0.0,    -- Dynamic movement intensity (#7, #14, #16)
    skin_temp_c         REAL NOT NULL,
    radiation_usv_hr    REAL NOT NULL,
    cabin_co2_ppm       REAL NOT NULL,
    cabin_temp_c        REAL,
    cabin_humidity_pct  REAL,
    cabin_pressure_kpa  REAL,
    cabin_o2_pct        REAL,
    cabin_motion_idx    REAL NOT NULL DEFAULT 0.0,    -- Cabin movement sensor (#2 Isolation)
    suit_pressure_kpa   REAL,                         -- NULL outside EVA
    suit_humidity_pct   REAL,                         -- NULL outside EVA
    eva_elapsed_sec     INTEGER NOT NULL DEFAULT 0,   -- Active EVA / prebreathe timer (#9)
    kcal_burn_delta     REAL NOT NULL,
    sweat_loss_delta_ml REAL NOT NULL,
    is_synced           INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (astronaut_id) REFERENCES astronaut_config(astronaut_id)
);

CREATE INDEX IF NOT EXISTS idx_biomon_telemetry_unsynced
    ON telemetry_ring_buffer(is_synced);

CREATE INDEX IF NOT EXISTS idx_biomon_telemetry_day
    ON telemetry_ring_buffer(astronaut_id, mission_day);

-- 3. Deterministic Offline Countermeasure Protocols
CREATE TABLE IF NOT EXISTS offline_protocols (
    protocol_id      TEXT PRIMARY KEY,
    hazard_category  TEXT NOT NULL, -- CARDIO, HYDRATION, RADIATION, HYPOXIA, SANS, DECOMP, OSTEO
    applicable_phase TEXT NOT NULL, -- Target phase or ALL
    title            TEXT NOT NULL,
    immediate_action TEXT NOT NULL,
    rationale        TEXT NOT NULL
);

-- 4. Local Physiological Alerts
CREATE TABLE IF NOT EXISTS local_alerts (
    alert_id         INTEGER PRIMARY KEY AUTOINCREMENT,
    seq_id           INTEGER NOT NULL,
    protocol_id      TEXT NOT NULL,
    triggered_at_utc TEXT NOT NULL,
    severity         TEXT NOT NULL, -- INFO, CAUTION, CRITICAL
    acknowledged     INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (seq_id) REFERENCES telemetry_ring_buffer(seq_id),
    FOREIGN KEY (protocol_id) REFERENCES offline_protocols(protocol_id)
);

-- 5. Crew Manual Input Queue (Input Mode)
CREATE TABLE IF NOT EXISTS crew_input_queue (
    input_id        INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id    TEXT NOT NULL,
    logged_at_utc   TEXT NOT NULL,
    mission_day     INTEGER NOT NULL,
    input_type      TEXT NOT NULL, -- MEAL_LOG, FLUID_LOG, MEDICATION, INJURY, EXERCISE, ISOLATION_SURVEY
    reference_code  TEXT,          -- e.g., iss_food_cache.food_id or symptom/med code
    consumption_pct INTEGER,       -- 100, 50, 25, 0
    notes           TEXT,
    is_synced       INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (astronaut_id) REFERENCES astronaut_config(astronaut_id)
);

CREATE INDEX IF NOT EXISTS idx_biomon_inputs_unsynced
    ON crew_input_queue(is_synced);

CREATE INDEX IF NOT EXISTS idx_biomon_inputs_day
    ON crew_input_queue(astronaut_id, mission_day, input_type);

-- 6. Local ISS Food Catalog Cache (Synced from Hub for Offline Nutrition Math)
CREATE TABLE IF NOT EXISTS iss_food_cache (
    food_id       TEXT PRIMARY KEY,
    item_name     TEXT NOT NULL,
    kcal          REAL NOT NULL DEFAULT 0.0,
    sodium_mg     REAL NOT NULL,
    calcium_mg    REAL NOT NULL DEFAULT 0.0,
    vit_d3_iu     REAL NOT NULL DEFAULT 0.0,
    fluid_ml      REAL NOT NULL DEFAULT 0.0,
    is_low_sodium INTEGER NOT NULL
);

-- 7. Sleep Epochs (60-second Actigraphy + HR Windows)
CREATE TABLE IF NOT EXISTS sleep_epochs (
    epoch_id         INTEGER PRIMARY KEY AUTOINCREMENT,
    astronaut_id     TEXT NOT NULL,
    epoch_start_utc  TEXT NOT NULL,
    mission_day      INTEGER NOT NULL,
    activity_count   REAL NOT NULL,
    hr_bpm_avg       REAL,
    sleep_state      TEXT NOT NULL CHECK (sleep_state IN ('SLEEP', 'WAKE')),
    is_synced        INTEGER NOT NULL DEFAULT 0,
    UNIQUE (astronaut_id, epoch_start_utc),
    FOREIGN KEY (astronaut_id) REFERENCES astronaut_config(astronaut_id)
);

CREATE INDEX IF NOT EXISTS idx_biomon_sleep_unsynced
    ON sleep_epochs(is_synced);

CREATE INDEX IF NOT EXISTS idx_biomon_sleep_day
    ON sleep_epochs(astronaut_id, mission_day);

-- 8. Daily SANS State (Derived from Sodium + Cabin CO2 + Flight Phase)
CREATE TABLE IF NOT EXISTS sans_daily_state (
    astronaut_id        TEXT NOT NULL,
    mission_day         INTEGER NOT NULL,
    sodium_mg_total     REAL NOT NULL,
    avg_cabin_co2_ppm   REAL NOT NULL,
    max_cabin_co2_ppm   REAL NOT NULL,
    sans_score          REAL NOT NULL,
    sans_risk           TEXT NOT NULL CHECK (sans_risk IN ('LOW', 'MODERATE', 'HIGH')),
    computed_at_utc     TEXT NOT NULL,
    is_synced           INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (astronaut_id, mission_day),
    FOREIGN KEY (astronaut_id) REFERENCES astronaut_config(astronaut_id)
);

CREATE INDEX IF NOT EXISTS idx_biomon_sans_unsynced
    ON sans_daily_state(is_synced);

-- 9. Daily Osteoporosis State (Derived from Exercise Load + Calcium + Vitamin D3)
CREATE TABLE IF NOT EXISTS osteoporosis_daily_state (
    astronaut_id       TEXT NOT NULL,
    mission_day        INTEGER NOT NULL,
    exercise_load_idx  REAL NOT NULL,
    calcium_mg_total   REAL NOT NULL,
    vit_d3_iu_total    REAL NOT NULL,
    protection_score   REAL NOT NULL,
    osteo_risk         TEXT NOT NULL CHECK (osteo_risk IN ('LOW', 'MODERATE', 'HIGH')),
    computed_at_utc    TEXT NOT NULL,
    is_synced          INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (astronaut_id, mission_day),
    FOREIGN KEY (astronaut_id) REFERENCES astronaut_config(astronaut_id)
);

CREATE INDEX IF NOT EXISTS idx_biomon_osteo_unsynced
    ON osteoporosis_daily_state(is_synced);

-- 10. Views for Daily Derived Models
CREATE VIEW IF NOT EXISTS v_daily_sodium AS
SELECT
    q.astronaut_id,
    q.mission_day,
    COALESCE(SUM(f.sodium_mg * COALESCE(q.consumption_pct, 0) / 100.0), 0.0) AS sodium_mg_total
FROM crew_input_queue q
JOIN iss_food_cache f ON f.food_id = q.reference_code
WHERE q.input_type = 'MEAL_LOG'
GROUP BY q.astronaut_id, q.mission_day;

CREATE VIEW IF NOT EXISTS v_daily_nutrition AS
SELECT
    q.astronaut_id,
    q.mission_day,
    COALESCE(SUM(f.kcal * COALESCE(q.consumption_pct, 0) / 100.0), 0.0)       AS kcal_total,
    COALESCE(SUM(f.sodium_mg * COALESCE(q.consumption_pct, 0) / 100.0), 0.0)  AS sodium_mg_total,
    COALESCE(SUM(f.calcium_mg * COALESCE(q.consumption_pct, 0) / 100.0), 0.0) AS calcium_mg_total,
    COALESCE(SUM(f.vit_d3_iu * COALESCE(q.consumption_pct, 0) / 100.0), 0.0)  AS vit_d3_iu_total,
    COALESCE(SUM(f.fluid_ml * COALESCE(q.consumption_pct, 0) / 100.0), 0.0)   AS fluid_ml_total
FROM crew_input_queue q
JOIN iss_food_cache f ON f.food_id = q.reference_code
WHERE q.input_type = 'MEAL_LOG'
GROUP BY q.astronaut_id, q.mission_day;

CREATE VIEW IF NOT EXISTS v_daily_cabin_co2 AS
SELECT
    astronaut_id,
    mission_day,
    AVG(cabin_co2_ppm) AS avg_cabin_co2_ppm,
    MAX(cabin_co2_ppm) AS max_cabin_co2_ppm
FROM telemetry_ring_buffer
GROUP BY astronaut_id, mission_day;
