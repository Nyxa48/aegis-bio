use crate::calc;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_BIOMON_DB_PATH: &str = "aegis_biomon.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstronautConfig {
    pub astronaut_id: String,
    pub callsign: String,
    pub weight: f64,
    pub height: f64,
    pub age: i64,
    pub sex: String,
    pub active_phase: String,
    pub mission_start_utc: String,
    pub sleep_target_min: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryFrame {
    pub seq_id: i64,
    pub astronaut_id: String,
    pub recorded_at_utc: String,
    pub mission_day: i64,
    pub mission_phase: String,
    pub hr_bpm: f64,
    pub sbp_mmhg: f64,
    pub resp_brpm: f64,
    pub spo2_pct: f64,
    pub tissue_o2_pct: Option<f64>,
    pub blood_n2_idx: Option<f64>,
    pub g_load: f64,
    pub imu_activity_idx: f64,
    pub skin_temp_c: f64,
    pub radiation_usv_hr: f64,
    pub cabin_co2_ppm: f64,
    pub cabin_temp_c: Option<f64>,
    pub cabin_humidity_pct: Option<f64>,
    pub cabin_pressure_kpa: Option<f64>,
    pub cabin_o2_pct: Option<f64>,
    pub cabin_motion_idx: f64,
    pub suit_pressure_kpa: Option<f64>,
    pub suit_humidity_pct: Option<f64>,
    pub eva_elapsed_sec: i64,
    pub kcal_burn_delta: f64,
    pub sweat_loss_delta_ml: f64,
    pub is_synced: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineProtocol {
    pub protocol_id: String,
    pub hazard_category: String,
    pub applicable_phase: String,
    pub title: String,
    pub immediate_action: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAlert {
    pub alert_id: i64,
    pub seq_id: i64,
    pub protocol_id: String,
    pub triggered_at_utc: String,
    pub severity: String,
    pub acknowledged: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewInput {
    pub input_id: i64,
    pub astronaut_id: String,
    pub logged_at_utc: String,
    pub mission_day: i64,
    pub input_type: String,
    pub reference_code: Option<String>,
    pub consumption_pct: Option<i64>,
    pub notes: Option<String>,
    pub is_synced: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct NewCrewInput<'a> {
    pub astronaut_id: &'a str,
    pub mission_day: i64,
    pub input_type: &'a str,
    pub reference_code: Option<&'a str>,
    pub consumption_pct: Option<i64>,
    pub notes: Option<&'a str>,
    pub phase: &'a str,
    pub avg_co2_ppm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodCacheItem {
    pub food_id: String,
    pub item_name: String,
    pub kcal: f64,
    pub sodium_mg: f64,
    pub calcium_mg: f64,
    pub vit_d3_iu: f64,
    pub fluid_ml: f64,
    pub is_low_sodium: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepEpoch {
    pub epoch_id: i64,
    pub astronaut_id: String,
    pub epoch_start_utc: String,
    pub mission_day: i64,
    pub activity_count: f64,
    pub hr_bpm_avg: Option<f64>,
    pub sleep_state: String,
    pub is_synced: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SansDailyState {
    pub astronaut_id: String,
    pub mission_day: i64,
    pub sodium_mg_total: f64,
    pub avg_cabin_co2_ppm: f64,
    pub max_cabin_co2_ppm: f64,
    pub sans_score: f64,
    pub sans_risk: String,
    pub computed_at_utc: String,
    pub is_synced: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsteoporosisDailyState {
    pub astronaut_id: String,
    pub mission_day: i64,
    pub exercise_load_idx: f64,
    pub calcium_mg_total: f64,
    pub vit_d3_iu_total: f64,
    pub protection_score: f64,
    pub osteo_risk: String,
    pub computed_at_utc: String,
    pub is_synced: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyNutritionTotals {
    pub kcal_total: f64,
    pub sodium_mg_total: f64,
    pub calcium_mg_total: f64,
    pub vit_d3_iu_total: f64,
    pub fluid_ml_total: f64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct UnsyncedCounts {
    pub telemetry_frames: i64,
    pub crew_inputs: i64,
    pub sleep_epochs: i64,
    pub daily_states: i64,
}

impl UnsyncedCounts {
    pub fn total(self) -> i64 {
        self.telemetry_frames + self.crew_inputs + self.sleep_epochs + self.daily_states
    }
}

pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        Self::open(DEFAULT_BIOMON_DB_PATH)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let conn = Connection::open(&path_buf)?;
        Self::configure_connection(&conn)?;

        let mut db = Self {
            conn,
            path: path_buf,
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory_fallback() -> Self {
        let conn = Connection::open_in_memory().expect("in-memory sqlite must succeed");
        let _ = Self::configure_connection(&conn);
        let mut db = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        let _ = db.init_schema();
        db
    }

    fn configure_connection(conn: &Connection) -> Result<()> {
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        Ok(())
    }

    pub fn init_schema(&mut self) -> Result<()> {
        self.conn.execute_batch(include_str!("../sql/init.sql"))?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn seed_demo_data(&mut self) -> Result<()> {
        // 1. Seed AST-01 (Callsign "ARES-1", weight 78.0, height 180.0, age 36, sex "M", phase "ORBIT", mission_start_utc "2026-11-14T00:00:00Z")
        self.conn.execute(
            "INSERT OR IGNORE INTO astronaut_config
             (astronaut_id, callsign, weight, height, age, sex, active_phase, mission_start_utc, sleep_target_min)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "AST-01",
                "ARES-1",
                78.0_f64,
                180.0_f64,
                36_i64,
                "M",
                "ORBIT",
                "2026-11-14T00:00:00Z",
                480_i64,
            ],
        )?;

        // 2. Seed 5 core offline_protocols (CARDIO, HYDRATION, RADIATION, HYPOXIA, DECOMP)
        let protocols = [
            (
                "PROT-CARDIO-01",
                "CARDIO",
                "ALL",
                "TACHYCARDIA / SHOCK INDEX OVERRIDE",
                "CEASE EXERTION, ASSUME REST POSTURE, VERIFY CUFF SBP",
                "Prevents orthostatic syncope and excessive myocardial oxygen demand under microgravity or EVA load.",
            ),
            (
                "PROT-HYDRATION-01",
                "HYDRATION",
                "ALL",
                "EVAPORATIVE SWEAT DEFICIT",
                "INGEST 250 ML ELECTROLYTE REHYDRATION POUCH IMMEDIATELY",
                "Restores plasma volume depleted by thermal sweat loss and microgravity diuresis.",
            ),
            (
                "PROT-RADIATION-01",
                "RADIATION",
                "ALL",
                "SOLAR PARTICLE FLUX SPIKE (>50 USV/HR)",
                "ABORT EVA / RETREAT TO POLYETHYLENE STORM SHELTER",
                "Minimizes acute ionizing dose equivalent during SPE/GCR flux excursion.",
            ),
            (
                "PROT-HYPOXIA-01",
                "HYPOXIA",
                "ALL",
                "ARTERIAL OR TISSUE HYPOXIA",
                "PURGE SUIT O2 LOOP / VERIFY CABIN O2 PARTIAL PRESSURE",
                "Restores alveolar PO2 before cognitive impairment occurs (<92% SpO2 or <85% tissue O2).",
            ),
            (
                "PROT-DECOMP-01",
                "DECOMP",
                "EVA",
                "NITROGEN SUPERSATURATION BREACH (R > 1.50)",
                "EXTEND PURE O2 PREBREATHE OR REPRESSURIZE SUIT TO 40 KPA",
                "Accelerates Haldane 360-min tissue N2 washout to prevent venous gas emboli.",
            ),
        ];

        for (id, cat, phase, title, action, rationale) in protocols {
            self.conn.execute(
                "INSERT OR IGNORE INTO offline_protocols
                 (protocol_id, hazard_category, applicable_phase, title, immediate_action, rationale)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, cat, phase, title, action, rationale],
            )?;
        }

        // 3. Seed 4 sample ISS food items in iss_food_cache
        let foods = [
            (
                "FOOD-01",
                "GRILLED CHICKEN & RICE (LOW NA)",
                340.0,
                210.0,
                85.0,
                120.0,
                140.0,
                1_i64,
            ),
            (
                "FOOD-02",
                "FORTIFIED CALCIUM OATMEAL",
                260.0,
                140.0,
                420.0,
                320.0,
                180.0,
                1_i64,
            ),
            (
                "FOOD-03",
                "BEEF FAJITA TORTILLA PACK",
                430.0,
                690.0,
                110.0,
                80.0,
                95.0,
                0_i64,
            ),
            (
                "FOOD-04",
                "ELECTROLYTE CITRUS HYDRATION POUCH",
                90.0,
                240.0,
                180.0,
                280.0,
                250.0,
                1_i64,
            ),
        ];

        for (food_id, name, kcal, na, ca, d3, fluid, low_na) in foods {
            self.conn.execute(
                "INSERT OR IGNORE INTO iss_food_cache
                 (food_id, item_name, kcal, sodium_mg, calcium_mg, vit_d3_iu, fluid_ml, is_low_sodium)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![food_id, name, kcal, na, ca, d3, fluid, low_na],
            )?;
        }

        // 4. Seed initial telemetry frame if telemetry_ring_buffer is empty
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM telemetry_ring_buffer", [], |r| {
                    r.get(0)
                })?;
        if count == 0 {
            let hr = 88.0;
            let kcal_sec = calc::calc_kcal_per_sec(hr, 78.0, 36, "M");
            let sweat_sec = calc::calc_sweat_ml_per_sec(kcal_sec * 60.0, 36.4);
            let (_, r_n2) = calc::update_tissue_n2(32.0, 0.0, 29.6, 1.0);

            self.conn.execute(
                "INSERT INTO telemetry_ring_buffer (
                    astronaut_id, recorded_at_utc, mission_day, mission_phase,
                    hr_bpm, sbp_mmhg, resp_brpm, spo2_pct, tissue_o2_pct, blood_n2_idx,
                    g_load, imu_activity_idx, skin_temp_c, radiation_usv_hr,
                    cabin_co2_ppm, cabin_temp_c, cabin_humidity_pct, cabin_pressure_kpa,
                    cabin_o2_pct, cabin_motion_idx, suit_pressure_kpa, suit_humidity_pct,
                    eva_elapsed_sec, kcal_burn_delta, sweat_loss_delta_ml, is_synced
                ) VALUES (
                    ?1, ?2, ?3, ?4,
                    ?5, ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18,
                    ?19, ?20, ?21, ?22,
                    ?23, ?24, ?25, ?26
                )",
                params![
                    "AST-01",
                    "2026-11-14T20:00:00Z",
                    1_i64,
                    "ORBIT",
                    hr,
                    118.0_f64,
                    14.0_f64,
                    98.0_f64,
                    Some(93.5_f64),
                    Some(r_n2),
                    1.0_f64,
                    0.35_f64,
                    36.4_f64,
                    0.42_f64,
                    1150.0_f64,
                    Some(21.5_f64),
                    Some(44.0_f64),
                    Some(101.3_f64),
                    Some(20.9_f64),
                    0.40_f64,
                    Some(29.6_f64),
                    Some(38.0_f64),
                    0_i64,
                    kcal_sec,
                    sweat_sec,
                    0_i64,
                ],
            )?;
        }

        // 5. Seed initial crew inputs if empty
        let input_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM crew_input_queue", [], |r| r.get(0))?;
        if input_count == 0 {
            let seed_inputs = [
                ("FOOD-01", 100_i64, "BREAKFAST PACK"),
                ("FOOD-02", 100_i64, "MIDDAY OATMEAL"),
                ("FOOD-04", 100_i64, "HYDRATION POUCH"),
            ];
            for (food_id, pct, note) in seed_inputs {
                self.conn.execute(
                    "INSERT INTO crew_input_queue
                     (astronaut_id, logged_at_utc, mission_day, input_type, reference_code, consumption_pct, notes, is_synced)
                     VALUES ('AST-01', '2026-11-14T18:30:00Z', 1, 'MEAL_LOG', ?1, ?2, ?3, 0)",
                    params![food_id, pct, note],
                )?;
            }
        }

        // 6. Seed sample sleep epochs if empty
        let sleep_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM sleep_epochs", [], |r| r.get(0))?;
        if sleep_count == 0 {
            let epochs = [
                ("2026-11-14T06:00:00Z", 2.1, 56.0, "SLEEP"),
                ("2026-11-14T06:01:00Z", 1.8, 55.0, "SLEEP"),
                ("2026-11-14T06:02:00Z", 18.4, 72.0, "WAKE"),
            ];
            for (ts, act, hr_avg, state) in epochs {
                self.conn.execute(
                    "INSERT OR IGNORE INTO sleep_epochs
                     (astronaut_id, epoch_start_utc, mission_day, activity_count, hr_bpm_avg, sleep_state, is_synced)
                     VALUES ('AST-01', ?1, 1, ?2, ?3, ?4, 0)",
                    params![ts, act, hr_avg, state],
                )?;
            }
        }

        // 7. Seed sample local alert if empty
        let alert_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM local_alerts", [], |r| r.get(0))?;
        if alert_count == 0
            && let Some(latest) = self.load_latest_telemetry("AST-01")?
        {
            self.conn.execute(
                "INSERT INTO local_alerts
                 (seq_id, protocol_id, triggered_at_utc, severity, acknowledged)
                 VALUES (?1, 'PROT-HYDRATION-01', '2026-11-14T19:45:00Z', 'CAUTION', 0)",
                params![latest.seq_id],
            )?;
        }

        // 8. Seed daily SANS and Osteoporosis states for day 1
        let (sans_score, sans_risk) = calc::calc_sans_risk(1140.0, 1150.0, "ORBIT");
        self.conn.execute(
            "INSERT OR IGNORE INTO sans_daily_state
             (astronaut_id, mission_day, sodium_mg_total, avg_cabin_co2_ppm, max_cabin_co2_ppm, sans_score, sans_risk, computed_at_utc, is_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "AST-01",
                1_i64,
                1140.0_f64,
                1150.0_f64,
                1320.0_f64,
                sans_score,
                sans_risk,
                "2026-11-14T20:00:00Z",
                0_i64,
            ],
        )?;

        let (prot_score, osteo_risk) = calc::calc_osteoporosis_risk(82.0, 1050.0, 800.0);
        self.conn.execute(
            "INSERT OR IGNORE INTO osteoporosis_daily_state
             (astronaut_id, mission_day, exercise_load_idx, calcium_mg_total, vit_d3_iu_total, protection_score, osteo_risk, computed_at_utc, is_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "AST-01",
                1_i64,
                82.0_f64,
                1050.0_f64,
                800.0_f64,
                prot_score,
                osteo_risk,
                "2026-11-14T20:00:00Z",
                0_i64,
            ],
        )?;

        Ok(())
    }

    pub fn load_astronaut_config(&self, astronaut_id: &str) -> Result<Option<AstronautConfig>> {
        self.conn
            .query_row(
                "SELECT astronaut_id, callsign, weight, height, age, sex, active_phase, mission_start_utc, sleep_target_min
                 FROM astronaut_config WHERE astronaut_id = ?1",
                params![astronaut_id],
                |row| {
                    Ok(AstronautConfig {
                        astronaut_id: row.get(0)?,
                        callsign: row.get(1)?,
                        weight: row.get(2)?,
                        height: row.get(3)?,
                        age: row.get(4)?,
                        sex: row.get(5)?,
                        active_phase: row.get(6)?,
                        mission_start_utc: row.get(7)?,
                        sleep_target_min: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn update_active_phase(&self, astronaut_id: &str, phase: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE astronaut_config SET active_phase = ?2 WHERE astronaut_id = ?1",
            params![astronaut_id, phase],
        )?;
        Ok(())
    }

    pub fn load_latest_telemetry(&self, astronaut_id: &str) -> Result<Option<TelemetryFrame>> {
        self.conn
            .query_row(
                "SELECT
                    seq_id, astronaut_id, recorded_at_utc, mission_day, mission_phase,
                    hr_bpm, sbp_mmhg, resp_brpm, spo2_pct, tissue_o2_pct, blood_n2_idx,
                    g_load, imu_activity_idx, skin_temp_c, radiation_usv_hr,
                    cabin_co2_ppm, cabin_temp_c, cabin_humidity_pct, cabin_pressure_kpa,
                    cabin_o2_pct, cabin_motion_idx, suit_pressure_kpa, suit_humidity_pct,
                    eva_elapsed_sec, kcal_burn_delta, sweat_loss_delta_ml, is_synced
                 FROM telemetry_ring_buffer
                 WHERE astronaut_id = ?1
                 ORDER BY seq_id DESC
                 LIMIT 1",
                params![astronaut_id],
                |row| {
                    Ok(TelemetryFrame {
                        seq_id: row.get(0)?,
                        astronaut_id: row.get(1)?,
                        recorded_at_utc: row.get(2)?,
                        mission_day: row.get(3)?,
                        mission_phase: row.get(4)?,
                        hr_bpm: row.get(5)?,
                        sbp_mmhg: row.get(6)?,
                        resp_brpm: row.get(7)?,
                        spo2_pct: row.get(8)?,
                        tissue_o2_pct: row.get(9)?,
                        blood_n2_idx: row.get(10)?,
                        g_load: row.get(11)?,
                        imu_activity_idx: row.get(12)?,
                        skin_temp_c: row.get(13)?,
                        radiation_usv_hr: row.get(14)?,
                        cabin_co2_ppm: row.get(15)?,
                        cabin_temp_c: row.get(16)?,
                        cabin_humidity_pct: row.get(17)?,
                        cabin_pressure_kpa: row.get(18)?,
                        cabin_o2_pct: row.get(19)?,
                        cabin_motion_idx: row.get(20)?,
                        suit_pressure_kpa: row.get(21)?,
                        suit_humidity_pct: row.get(22)?,
                        eva_elapsed_sec: row.get(23)?,
                        kcal_burn_delta: row.get(24)?,
                        sweat_loss_delta_ml: row.get(25)?,
                        is_synced: row.get(26)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_food_cache(&self) -> Result<Vec<FoodCacheItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT food_id, item_name, kcal, sodium_mg, calcium_mg, vit_d3_iu, fluid_ml, is_low_sodium
             FROM iss_food_cache ORDER BY food_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FoodCacheItem {
                food_id: row.get(0)?,
                item_name: row.get(1)?,
                kcal: row.get(2)?,
                sodium_mg: row.get(3)?,
                calcium_mg: row.get(4)?,
                vit_d3_iu: row.get(5)?,
                fluid_ml: row.get(6)?,
                is_low_sodium: row.get(7)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_offline_protocols(&self) -> Result<Vec<OfflineProtocol>> {
        let mut stmt = self.conn.prepare(
            "SELECT protocol_id, hazard_category, applicable_phase, title, immediate_action, rationale
             FROM offline_protocols ORDER BY protocol_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(OfflineProtocol {
                protocol_id: row.get(0)?,
                hazard_category: row.get(1)?,
                applicable_phase: row.get(2)?,
                title: row.get(3)?,
                immediate_action: row.get(4)?,
                rationale: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_sans_state(
        &self,
        astronaut_id: &str,
        mission_day: i64,
    ) -> Result<Option<SansDailyState>> {
        self.conn
            .query_row(
                "SELECT astronaut_id, mission_day, sodium_mg_total, avg_cabin_co2_ppm,
                        max_cabin_co2_ppm, sans_score, sans_risk, computed_at_utc, is_synced
                 FROM sans_daily_state
                 WHERE astronaut_id = ?1 AND mission_day = ?2",
                params![astronaut_id, mission_day],
                |row| {
                    Ok(SansDailyState {
                        astronaut_id: row.get(0)?,
                        mission_day: row.get(1)?,
                        sodium_mg_total: row.get(2)?,
                        avg_cabin_co2_ppm: row.get(3)?,
                        max_cabin_co2_ppm: row.get(4)?,
                        sans_score: row.get(5)?,
                        sans_risk: row.get(6)?,
                        computed_at_utc: row.get(7)?,
                        is_synced: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_osteo_state(
        &self,
        astronaut_id: &str,
        mission_day: i64,
    ) -> Result<Option<OsteoporosisDailyState>> {
        self.conn
            .query_row(
                "SELECT astronaut_id, mission_day, exercise_load_idx, calcium_mg_total,
                        vit_d3_iu_total, protection_score, osteo_risk, computed_at_utc, is_synced
                 FROM osteoporosis_daily_state
                 WHERE astronaut_id = ?1 AND mission_day = ?2",
                params![astronaut_id, mission_day],
                |row| {
                    Ok(OsteoporosisDailyState {
                        astronaut_id: row.get(0)?,
                        mission_day: row.get(1)?,
                        exercise_load_idx: row.get(2)?,
                        calcium_mg_total: row.get(3)?,
                        vit_d3_iu_total: row.get(4)?,
                        protection_score: row.get(5)?,
                        osteo_risk: row.get(6)?,
                        computed_at_utc: row.get(7)?,
                        is_synced: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_recent_crew_inputs(
        &self,
        astronaut_id: &str,
        limit: usize,
    ) -> Result<Vec<CrewInput>> {
        let mut stmt = self.conn.prepare(
            "SELECT input_id, astronaut_id, logged_at_utc, mission_day, input_type,
                    reference_code, consumption_pct, notes, is_synced
             FROM crew_input_queue
             WHERE astronaut_id = ?1
             ORDER BY input_id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![astronaut_id, limit as i64], |row| {
            Ok(CrewInput {
                input_id: row.get(0)?,
                astronaut_id: row.get(1)?,
                logged_at_utc: row.get(2)?,
                mission_day: row.get(3)?,
                input_type: row.get(4)?,
                reference_code: row.get(5)?,
                consumption_pct: row.get(6)?,
                notes: row.get(7)?,
                is_synced: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn insert_crew_input(&self, input: &NewCrewInput<'_>) -> Result<CrewInput> {
        let logged_at_utc = "2026-11-14T20:15:00Z".to_string();
        self.conn.execute(
            "INSERT INTO crew_input_queue
             (astronaut_id, logged_at_utc, mission_day, input_type, reference_code, consumption_pct, notes, is_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                input.astronaut_id,
                logged_at_utc,
                input.mission_day,
                input.input_type,
                input.reference_code,
                input.consumption_pct,
                input.notes
            ],
        )?;
        let input_id = self.conn.last_insert_rowid();

        // Recompute daily SANS & Osteoporosis states from view totals
        let totals = self.load_daily_nutrition(input.astronaut_id, input.mission_day)?;
        let (sans_score, sans_risk) =
            calc::calc_sans_risk(totals.sodium_mg_total, input.avg_co2_ppm, input.phase);
        self.conn.execute(
            "INSERT INTO sans_daily_state
             (astronaut_id, mission_day, sodium_mg_total, avg_cabin_co2_ppm, max_cabin_co2_ppm, sans_score, sans_risk, computed_at_utc, is_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)
             ON CONFLICT(astronaut_id, mission_day) DO UPDATE SET
                 sodium_mg_total = excluded.sodium_mg_total,
                 avg_cabin_co2_ppm = excluded.avg_cabin_co2_ppm,
                 sans_score = excluded.sans_score,
                 sans_risk = excluded.sans_risk,
                 is_synced = 0",
            params![
                input.astronaut_id,
                input.mission_day,
                totals.sodium_mg_total,
                input.avg_co2_ppm,
                input.avg_co2_ppm.max(1320.0),
                sans_score,
                sans_risk,
                logged_at_utc,
            ],
        )?;

        let ex_load = self
            .load_osteo_state(input.astronaut_id, input.mission_day)?
            .map(|o| o.exercise_load_idx)
            .unwrap_or(82.0);
        let (prot_score, osteo_risk) =
            calc::calc_osteoporosis_risk(ex_load, totals.calcium_mg_total, totals.vit_d3_iu_total);
        self.conn.execute(
            "INSERT INTO osteoporosis_daily_state
             (astronaut_id, mission_day, exercise_load_idx, calcium_mg_total, vit_d3_iu_total, protection_score, osteo_risk, computed_at_utc, is_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)
             ON CONFLICT(astronaut_id, mission_day) DO UPDATE SET
                 calcium_mg_total = excluded.calcium_mg_total,
                 vit_d3_iu_total = excluded.vit_d3_iu_total,
                 protection_score = excluded.protection_score,
                 osteo_risk = excluded.osteo_risk,
                 is_synced = 0",
            params![
                input.astronaut_id,
                input.mission_day,
                ex_load,
                totals.calcium_mg_total,
                totals.vit_d3_iu_total,
                prot_score,
                osteo_risk,
                logged_at_utc,
            ],
        )?;

        Ok(CrewInput {
            input_id,
            astronaut_id: input.astronaut_id.to_string(),
            logged_at_utc,
            mission_day: input.mission_day,
            input_type: input.input_type.to_string(),
            reference_code: input.reference_code.map(ToString::to_string),
            consumption_pct: input.consumption_pct,
            notes: input.notes.map(ToString::to_string),
            is_synced: 0,
        })
    }

    pub fn load_daily_nutrition(
        &self,
        astronaut_id: &str,
        mission_day: i64,
    ) -> Result<DailyNutritionTotals> {
        let meal_totals = self
            .conn
            .query_row(
                "SELECT kcal_total, sodium_mg_total, calcium_mg_total, vit_d3_iu_total, fluid_ml_total
                 FROM v_daily_nutrition
                 WHERE astronaut_id = ?1 AND mission_day = ?2",
                params![astronaut_id, mission_day],
                |row| {
                    Ok(DailyNutritionTotals {
                        kcal_total: row.get(0)?,
                        sodium_mg_total: row.get(1)?,
                        calcium_mg_total: row.get(2)?,
                        vit_d3_iu_total: row.get(3)?,
                        fluid_ml_total: row.get(4)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();

        // Add any direct FLUID_LOG pouches (250 mL * consumption_pct / 100)
        let extra_fluid_ml: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(250.0 * COALESCE(consumption_pct, 100) / 100.0), 0.0)
             FROM crew_input_queue
             WHERE astronaut_id = ?1 AND mission_day = ?2 AND input_type = 'FLUID_LOG'",
            params![astronaut_id, mission_day],
            |r| r.get(0),
        )?;

        Ok(DailyNutritionTotals {
            fluid_ml_total: meal_totals.fluid_ml_total + extra_fluid_ml,
            ..meal_totals
        })
    }

    pub fn load_local_alerts(&self) -> Result<Vec<LocalAlert>> {
        let mut stmt = self.conn.prepare(
            "SELECT alert_id, seq_id, protocol_id, triggered_at_utc, severity, acknowledged
             FROM local_alerts ORDER BY alert_id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LocalAlert {
                alert_id: row.get(0)?,
                seq_id: row.get(1)?,
                protocol_id: row.get(2)?,
                triggered_at_utc: row.get(3)?,
                severity: row.get(4)?,
                acknowledged: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn acknowledge_all_alerts(&self) -> Result<()> {
        self.conn
            .execute("UPDATE local_alerts SET acknowledged = 1", [])?;
        Ok(())
    }

    pub fn load_sleep_epochs(
        &self,
        astronaut_id: &str,
        mission_day: i64,
    ) -> Result<Vec<SleepEpoch>> {
        let mut stmt = self.conn.prepare(
            "SELECT epoch_id, astronaut_id, epoch_start_utc, mission_day, activity_count,
                    hr_bpm_avg, sleep_state, is_synced
             FROM sleep_epochs
             WHERE astronaut_id = ?1 AND mission_day = ?2
             ORDER BY epoch_id DESC",
        )?;
        let rows = stmt.query_map(params![astronaut_id, mission_day], |row| {
            Ok(SleepEpoch {
                epoch_id: row.get(0)?,
                astronaut_id: row.get(1)?,
                epoch_start_utc: row.get(2)?,
                mission_day: row.get(3)?,
                activity_count: row.get(4)?,
                hr_bpm_avg: row.get(5)?,
                sleep_state: row.get(6)?,
                is_synced: row.get(7)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn count_unsynced(&self) -> Result<UnsyncedCounts> {
        let telemetry_frames: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM telemetry_ring_buffer WHERE is_synced = 0",
            [],
            |r| r.get(0),
        )?;
        let crew_inputs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM crew_input_queue WHERE is_synced = 0",
            [],
            |r| r.get(0),
        )?;
        let sleep_epochs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sleep_epochs WHERE is_synced = 0",
            [],
            |r| r.get(0),
        )?;
        let sans_unsynced: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sans_daily_state WHERE is_synced = 0",
            [],
            |r| r.get(0),
        )?;
        let osteo_unsynced: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM osteoporosis_daily_state WHERE is_synced = 0",
            [],
            |r| r.get(0),
        )?;
        Ok(UnsyncedCounts {
            telemetry_frames,
            crew_inputs,
            sleep_epochs,
            daily_states: sans_unsynced + osteo_unsynced,
        })
    }

    pub fn mark_all_synced(&self) -> Result<usize> {
        let mut total = 0;
        total += self.conn.execute(
            "UPDATE telemetry_ring_buffer SET is_synced = 1 WHERE is_synced = 0",
            [],
        )?;
        total += self.conn.execute(
            "UPDATE crew_input_queue SET is_synced = 1 WHERE is_synced = 0",
            [],
        )?;
        total += self.conn.execute(
            "UPDATE sleep_epochs SET is_synced = 1 WHERE is_synced = 0",
            [],
        )?;
        total += self.conn.execute(
            "UPDATE sans_daily_state SET is_synced = 1 WHERE is_synced = 0",
            [],
        )?;
        total += self.conn.execute(
            "UPDATE osteoporosis_daily_state SET is_synced = 1 WHERE is_synced = 0",
            [],
        )?;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_and_nutrition_roundtrip() {
        let mut db = Database::open_in_memory_fallback();
        db.seed_demo_data().expect("seed must succeed");

        let cfg = db
            .load_astronaut_config("AST-01")
            .expect("config query")
            .expect("AST-01 exists");
        assert_eq!(cfg.callsign, "ARES-1");

        let nut = db.load_daily_nutrition("AST-01", 1).expect("nutrition");
        assert!(nut.kcal_total > 600.0);
        assert!(nut.sodium_mg_total > 500.0);

        db.insert_crew_input(&NewCrewInput {
            astronaut_id: "AST-01",
            mission_day: 1,
            input_type: "FLUID_LOG",
            reference_code: Some("POUCH-250ML"),
            consumption_pct: Some(100),
            notes: Some("EVA REHYDRATION"),
            phase: "ORBIT",
            avg_co2_ppm: 1150.0,
        })
        .expect("insert fluid log");

        let nut_after = db
            .load_daily_nutrition("AST-01", 1)
            .expect("nutrition after");
        assert!((nut_after.fluid_ml_total - (nut.fluid_ml_total + 250.0)).abs() < 1e-4);

        let unsynced = db.count_unsynced().expect("unsynced count");
        assert!(unsynced.total() > 0);
        let synced_n = db.mark_all_synced().expect("mark synced");
        assert!(synced_n > 0);
        assert_eq!(db.count_unsynced().expect("after sync").total(), 0);
    }
}
