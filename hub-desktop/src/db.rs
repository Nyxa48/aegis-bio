use crate::calc;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_HUB_DB_PATH: &str = "aegis_hub.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewMember {
    pub astronaut_id: String,
    pub full_name: String,
    pub role: String,
    pub weight: f64,
    pub height: f64,
    pub age: i64,
    pub sex: String,
    pub active_phase_code: String,
    pub lan_status: String,
    pub last_heartbeat_utc: String,
    pub mission_start_utc: String,
    pub sleep_target_min: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryHistoryRecord {
    pub record_id: i64,
    pub astronaut_id: String,
    pub phase_code: String,
    pub mission_day: i64,
    pub edge_seq_id: i64,
    pub recorded_at_utc: String,
    pub received_at_utc: String,
    pub hr_bpm: f64,
    pub sbp_mmhg: f64,
    pub resp_brpm: f64,
    pub spo2_pct: f64,
    pub tissue_o2_pct: Option<f64>,
    pub blood_n2_idx: Option<f64>,
    pub g_load: f64,
    pub imu_activity_idx: f64,
    pub cabin_co2_ppm: f64,
    pub cabin_temp_c: Option<f64>,
    pub cabin_humidity_pct: Option<f64>,
    pub cabin_pressure_kpa: Option<f64>,
    pub cabin_o2_pct: Option<f64>,
    pub cabin_motion_idx: f64,
    pub suit_pressure_kpa: Option<f64>,
    pub suit_humidity_pct: Option<f64>,
    pub eva_elapsed_sec: i64,
    pub radiation_usv_hr: f64,
    pub shock_index: f64,
    pub orthostatic_risk: String,
    pub sans_risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodCatalogItem {
    pub food_id: String,
    pub item_name: String,
    pub category: String,
    pub kcal: f64,
    pub protein_g: f64,
    pub sodium_mg: f64,
    pub potassium_mg: f64,
    pub calcium_mg: f64,
    pub vit_d3_iu: f64,
    pub fluid_ml: f64,
    pub is_low_sodium: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewInputHistoryRecord {
    pub record_id: i64,
    pub astronaut_id: String,
    pub edge_input_id: i64,
    pub logged_at_utc: String,
    pub mission_day: i64,
    pub input_type: String,
    pub reference_code: Option<String>,
    pub consumption_pct: Option<i64>,
    pub notes: Option<String>,
    pub received_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeonIncident {
    pub incident_id: i64,
    pub astronaut_id: String,
    pub telemetry_record_id: Option<i64>,
    pub created_at_utc: String,
    pub hazard_type: String,
    pub injury_details: Option<String>,
    pub medication_administered: Option<String>,
    pub medical_notes: Option<String>,
    pub surgeon_ack: i64,
}

pub struct NewSurgeonIncident<'a> {
    pub astronaut_id: &'a str,
    pub telemetry_record_id: Option<i64>,
    pub created_at_utc: &'a str,
    pub hazard_type: &'a str,
    pub injury_details: Option<&'a str>,
    pub medication_administered: Option<&'a str>,
    pub medical_notes: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepDailySummary {
    pub astronaut_id: String,
    pub mission_day: i64,
    pub total_sleep_min: f64,
    pub target_sleep_min: i64,
    pub sleep_efficiency_pct: Option<f64>,
    pub awakenings: Option<i64>,
    pub computed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SansDailyHistory {
    pub astronaut_id: String,
    pub mission_day: i64,
    pub sodium_mg_total: f64,
    pub avg_cabin_co2_ppm: f64,
    pub max_cabin_co2_ppm: f64,
    pub sans_score: f64,
    pub sans_risk: String,
    pub computed_at_utc: String,
    pub received_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsteoporosisDailyHistory {
    pub astronaut_id: String,
    pub mission_day: i64,
    pub exercise_load_idx: f64,
    pub calcium_mg_total: f64,
    pub vit_d3_iu_total: f64,
    pub protection_score: f64,
    pub osteo_risk: String,
    pub computed_at_utc: String,
    pub received_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionEphemeris {
    pub mission_date: String,
    pub distance_au: f64,
    pub one_way_light_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubLabReport {
    pub report_id: i64,
    pub astronaut_id: Option<String>,
    pub source_dataset: String,
    pub domain: String,
    pub sample_date: String,
    pub biomarker_name: String,
    pub measured_value: f64,
    pub unit: String,
    pub status_flag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceRadiationPoint {
    pub timestamp_utc: String,
    pub instrument_id: String,
    pub absorbed_dose_rate_ugy_hr: f64,
    pub interval_sec: Option<i64>,
    pub interval_dose_ugy: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceEdaPoint {
    pub recorded_at_utc: String,
    pub co2_iss_ppm: Option<f64>,
    pub co2_ground_ppm: Option<f64>,
    pub temp_iss_c: Option<f64>,
    pub temp_ground_c: Option<f64>,
    pub humidity_iss_pct: Option<f64>,
    pub humidity_ground_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchLabObservation {
    pub obs_id: i64,
    pub study_id: String,
    pub domain: String,
    pub sample_name: String,
    pub subject_code: String,
    pub sample_matrix: String,
    pub timepoint: String,
    pub sort_order: i64,
    pub biomarker_name: String,
    pub measured_value: f64,
    pub unit: String,
    pub ref_min: Option<f64>,
    pub ref_max: Option<f64>,
    pub status_flag: String,
    pub source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProvenance {
    pub file_name: String,
    pub dataset_id: String,
    pub sha256: String,
    pub source_url: String,
    pub byte_size: i64,
    pub imported_at_utc: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HubCrewDailyNutrition {
    pub kcal_total: f64,
    pub sodium_mg_total: f64,
    pub calcium_mg_total: f64,
    pub vit_d3_iu_total: f64,
    pub fluid_ml_total: f64,
    pub meal_count: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct InboundTelemetryFrame<'a> {
    pub astronaut_id: &'a str,
    pub phase_code: &'a str,
    pub mission_day: i64,
    pub edge_seq_id: i64,
    pub recorded_at_utc: &'a str,
    pub hr_bpm: f64,
    pub sbp_mmhg: f64,
    pub resp_brpm: f64,
    pub spo2_pct: f64,
    pub tissue_o2_pct: Option<f64>,
    pub blood_n2_idx: Option<f64>,
    pub g_load: f64,
    pub imu_activity_idx: f64,
    pub cabin_co2_ppm: f64,
    pub cabin_temp_c: Option<f64>,
    pub cabin_humidity_pct: Option<f64>,
    pub cabin_pressure_kpa: Option<f64>,
    pub cabin_o2_pct: Option<f64>,
    pub cabin_motion_idx: f64,
    pub suit_pressure_kpa: Option<f64>,
    pub suit_humidity_pct: Option<f64>,
    pub eva_elapsed_sec: i64,
    pub radiation_usv_hr: f64,
    pub shock_index: f64,
    pub orthostatic_risk: &'a str,
    pub sans_risk: &'a str,
}

pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        Self::open(DEFAULT_HUB_DB_PATH)
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

    pub fn seed_demo_data(&mut self) -> Result<()> {
        // 1. Seed primary crew member AST-01 plus 3 habitat crewmates
        let crew = [
            (
                "AST-01",
                "ARES-1 (CDR EMIR)",
                "COMMANDER",
                78.0_f64,
                180.0_f64,
                36_i64,
                "M",
                "ORBIT",
                "ONLINE",
            ),
            (
                "AST-02",
                "ARES-2 (PLT LYKA)",
                "PILOT",
                74.0_f64,
                176.0_f64,
                34_i64,
                "M",
                "ORBIT",
                "ONLINE",
            ),
            (
                "AST-03",
                "ARES-3 (MS NOVA)",
                "MISSION SPECIALIST",
                64.0_f64,
                168.0_f64,
                33_i64,
                "F",
                "ORBIT",
                "ONLINE",
            ),
            (
                "AST-04",
                "ARES-4 (FS ORION)",
                "FLIGHT SURGEON",
                81.0_f64,
                183.0_f64,
                39_i64,
                "M",
                "ORBIT",
                "ONLINE",
            ),
        ];

        for (id, name, role, weight, height, age, sex, phase, lan) in crew {
            self.conn.execute(
                "INSERT OR IGNORE INTO crew_members
                 (astronaut_id, full_name, role, weight, height, age, sex, active_phase_code, lan_status, last_heartbeat_utc, mission_start_utc, sleep_target_min)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    name,
                    role,
                    weight,
                    height,
                    age,
                    sex,
                    phase,
                    lan,
                    "2026-11-14T20:00:00Z",
                    "2026-11-14T00:00:00Z",
                    480_i64,
                ],
            )?;
        }

        // 2. Seed sample ISS food items in iss_food_catalog
        let foods = [
            (
                "FOOD-01",
                "GRILLED CHICKEN & RICE (LOW NA)",
                "ENTREE",
                340.0,
                28.0,
                210.0,
                420.0,
                85.0,
                120.0,
                140.0,
                1_i64,
            ),
            (
                "FOOD-02",
                "FORTIFIED CALCIUM OATMEAL",
                "ENTREE",
                260.0,
                11.0,
                140.0,
                310.0,
                420.0,
                320.0,
                180.0,
                1_i64,
            ),
            (
                "FOOD-03",
                "BEEF FAJITA TORTILLA PACK",
                "ENTREE",
                430.0,
                26.0,
                690.0,
                380.0,
                110.0,
                80.0,
                95.0,
                0_i64,
            ),
            (
                "FOOD-04",
                "ELECTROLYTE CITRUS HYDRATION POUCH",
                "REHYDRATION",
                90.0,
                1.0,
                240.0,
                510.0,
                180.0,
                280.0,
                250.0,
                1_i64,
            ),
            (
                "FOOD-05",
                "FREEZE-DRIED SALMON & QUINOA",
                "ENTREE",
                390.0,
                32.0,
                230.0,
                490.0,
                210.0,
                440.0,
                150.0,
                1_i64,
            ),
            (
                "FOOD-06",
                "CALCIUM + VIT D3 CHEWABLE TAB",
                "SUPPLEMENT",
                15.0,
                0.0,
                5.0,
                10.0,
                600.0,
                800.0,
                0.0,
                1_i64,
            ),
        ];

        for (id, name, cat, kcal, prot, na, k, ca, d3, fluid, low_na) in foods {
            self.conn.execute(
                "INSERT OR IGNORE INTO iss_food_catalog
                 (food_id, item_name, category, kcal, protein_g, sodium_mg, potassium_mg, calcium_mg, vit_d3_iu, fluid_ml, is_low_sodium)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![id, name, cat, kcal, prot, na, k, ca, d3, fluid, low_na],
            )?;
        }

        // 3. Seed mission_ephemeris & NASA reference/research datasets via nasa_seed.sql
        self.seed_nasa_data()?;

        // 4. Seed initial telemetry_history rows for the crew roster
        let crew_telemetry = [
            ("AST-01", "ORBIT", 88.0, 118.0, 14.0, 98.0, 1150.0, 0.42),
            ("AST-02", "ORBIT", 74.0, 120.0, 13.0, 99.0, 1120.0, 0.38),
            ("AST-03", "ORBIT", 92.0, 114.0, 15.0, 97.5, 1180.0, 0.41),
            ("AST-04", "ORBIT", 79.0, 122.0, 13.5, 98.5, 1100.0, 0.39),
        ];

        for (ast_id, phase, hr, sbp, resp, spo2, co2, rad) in crew_telemetry {
            let shock_idx = calc::calc_shock_index(hr, sbp);
            let ortho_risk = calc::calc_orthostatic_risk(hr, sbp, 1.0);
            let (_, sans_risk) = calc::calc_sans_risk(1140.0, co2, phase);

            self.conn.execute(
                "INSERT OR IGNORE INTO telemetry_history (
                    astronaut_id, phase_code, mission_day, edge_seq_id,
                    recorded_at_utc, received_at_utc,
                    hr_bpm, sbp_mmhg, resp_brpm, spo2_pct, tissue_o2_pct, blood_n2_idx,
                    g_load, imu_activity_idx, cabin_co2_ppm, cabin_temp_c,
                    cabin_humidity_pct, cabin_pressure_kpa, cabin_o2_pct, cabin_motion_idx,
                    suit_pressure_kpa, suit_humidity_pct, eva_elapsed_sec,
                    radiation_usv_hr, shock_index, orthostatic_risk, sans_risk
                ) VALUES (
                    ?1, ?2, ?3, ?4,
                    ?5, ?6,
                    ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20,
                    ?21, ?22, ?23,
                    ?24, ?25, ?26, ?27
                )",
                params![
                    ast_id,
                    phase,
                    1_i64,
                    1_i64,
                    "2026-11-14T20:00:00Z",
                    "2026-11-14T20:00:01Z",
                    hr,
                    sbp,
                    resp,
                    spo2,
                    Some(93.5_f64),
                    Some(1.08_f64),
                    1.0_f64,
                    0.35_f64,
                    co2,
                    Some(21.5_f64),
                    Some(44.0_f64),
                    Some(101.3_f64),
                    Some(20.9_f64),
                    0.40_f64,
                    Some(29.6_f64),
                    Some(38.0_f64),
                    0_i64,
                    rad,
                    shock_idx,
                    ortho_risk,
                    sans_risk,
                ],
            )?;
        }

        // 5. Seed crew_input_history (idempotent via UNIQUE(astronaut_id, edge_input_id))
        let inputs = [
            (
                "AST-01",
                101_i64,
                "2026-11-14T08:15:00Z",
                1_i64,
                "MEAL_LOG",
                Some("FOOD-01"),
                Some(100_i64),
                Some("Nominal breakfast ration"),
                "2026-11-14T08:15:02Z",
            ),
            (
                "AST-01",
                102_i64,
                "2026-11-14T12:30:00Z",
                1_i64,
                "FLUID_LOG",
                Some("FOOD-04"),
                Some(100_i64),
                Some("Post-exercise electrolyte pouch"),
                "2026-11-14T12:30:02Z",
            ),
            (
                "AST-02",
                101_i64,
                "2026-11-14T08:45:00Z",
                1_i64,
                "MEAL_LOG",
                Some("FOOD-02"),
                Some(100_i64),
                Some("Fortified oatmeal complete"),
                "2026-11-14T08:45:03Z",
            ),
            (
                "AST-03",
                101_i64,
                "2026-11-14T13:10:00Z",
                1_i64,
                "MEAL_LOG",
                Some("FOOD-03"),
                Some(75_i64),
                Some("High-sodium entree logged"),
                "2026-11-14T13:10:01Z",
            ),
            (
                "AST-04",
                101_i64,
                "2026-11-14T15:00:00Z",
                1_i64,
                "EXERCISE",
                Some("ARED-RESIST"),
                Some(100_i64),
                Some("60 min resistive load protocol"),
                "2026-11-14T15:00:02Z",
            ),
        ];

        for (ast, edge_id, logged_at, day, itype, ref_code, pct, notes, recv_at) in inputs {
            self.conn.execute(
                "INSERT OR IGNORE INTO crew_input_history
                 (astronaut_id, edge_input_id, logged_at_utc, mission_day, input_type, reference_code, consumption_pct, notes, received_at_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![ast, edge_id, logged_at, day, itype, ref_code, pct, notes, recv_at],
            )?;
        }

        // 6. Seed surgeon_incidents (idempotent explicit incident_id)
        let incidents = [
            (
                1_i64,
                "AST-03",
                Some(3_i64),
                "2026-11-14T14:22:00Z",
                "ORTHOSTATIC_RISK",
                Some("Transient dizziness post-treadmill transition"),
                Some("250mL Electrolyte Bolus"),
                Some("Monitor SBP & Shock Index during next orbit pass"),
                0_i64,
            ),
            (
                2_i64,
                "AST-01",
                Some(1_i64),
                "2026-11-14T16:05:00Z",
                "RADIATION_SPIKE",
                Some("Minor solar proton flux elevation during airlock check"),
                Some("None"),
                Some("Dosimeter verified within nominal daily corridor"),
                1_i64,
            ),
            (
                3_i64,
                "AST-02",
                Some(2_i64),
                "2026-11-14T18:40:00Z",
                "DECOMP",
                Some("Pre-breathe N2 washout verification alert"),
                Some("100% O2 Mask Protocol 15m"),
                Some("Blood N2 R-value stabilized at 1.08"),
                1_i64,
            ),
        ];

        for (inc_id, ast, tel_id, created, hazard, injury, med, notes, ack) in incidents {
            self.conn.execute(
                "INSERT OR IGNORE INTO surgeon_incidents
                 (incident_id, astronaut_id, telemetry_record_id, created_at_utc, hazard_type, injury_details, medication_administered, medical_notes, surgeon_ack)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![inc_id, ast, tel_id, created, hazard, injury, med, notes, ack],
            )?;
        }

        // 7. Seed sleep_daily_summary (PRIMARY KEY (astronaut_id, mission_day))
        let sleeps = [
            (
                "AST-01",
                1_i64,
                452.0,
                480_i64,
                Some(91.5),
                Some(2_i64),
                "2026-11-14T07:00:00Z",
            ),
            (
                "AST-02",
                1_i64,
                474.0,
                480_i64,
                Some(94.2),
                Some(1_i64),
                "2026-11-14T07:00:00Z",
            ),
            (
                "AST-03",
                1_i64,
                410.0,
                480_i64,
                Some(84.0),
                Some(4_i64),
                "2026-11-14T07:00:00Z",
            ),
            (
                "AST-04",
                1_i64,
                468.0,
                480_i64,
                Some(93.0),
                Some(1_i64),
                "2026-11-14T07:00:00Z",
            ),
        ];

        for (ast, day, total, target, eff, awake, comp) in sleeps {
            self.conn.execute(
                "INSERT OR IGNORE INTO sleep_daily_summary
                 (astronaut_id, mission_day, total_sleep_min, target_sleep_min, sleep_efficiency_pct, awakenings, computed_at_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![ast, day, total, target, eff, awake, comp],
            )?;
        }

        // 8. Seed sans_daily_history (PRIMARY KEY (astronaut_id, mission_day))
        let sans_rows = [
            ("AST-01", 1_i64, 1450.0, 1150.0, 1420.0, "ORBIT"),
            ("AST-02", 1_i64, 1280.0, 1120.0, 1360.0, "ORBIT"),
            ("AST-03", 1_i64, 2420.0, 1380.0, 1890.0, "ORBIT"),
            ("AST-04", 1_i64, 1310.0, 1100.0, 1290.0, "ORBIT"),
        ];

        for (ast, day, na_mg, avg_co2, max_co2, phase) in sans_rows {
            let (score, risk) = calc::calc_sans_risk(na_mg, avg_co2, phase);
            self.conn.execute(
                "INSERT OR IGNORE INTO sans_daily_history
                 (astronaut_id, mission_day, sodium_mg_total, avg_cabin_co2_ppm, max_cabin_co2_ppm, sans_score, sans_risk, computed_at_utc, received_at_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    ast,
                    day,
                    na_mg,
                    avg_co2,
                    max_co2,
                    score,
                    risk,
                    "2026-11-14T19:55:00Z",
                    "2026-11-14T19:55:01Z",
                ],
            )?;
        }

        // 9. Seed osteoporosis_daily_history (PRIMARY KEY (astronaut_id, mission_day))
        let osteo_rows = [
            ("AST-01", 1_i64, 88.0, 1120.0, 760.0),
            ("AST-02", 1_i64, 92.0, 1180.0, 820.0),
            ("AST-03", 1_i64, 62.0, 840.0, 520.0),
            ("AST-04", 1_i64, 95.0, 1240.0, 840.0),
        ];

        for (ast, day, ex_idx, ca_mg, d3_iu) in osteo_rows {
            let (prot_score, risk) = calc::calc_osteoporosis_risk(ex_idx, ca_mg, d3_iu);
            self.conn.execute(
                "INSERT OR IGNORE INTO osteoporosis_daily_history
                 (astronaut_id, mission_day, exercise_load_idx, calcium_mg_total, vit_d3_iu_total, protection_score, osteo_risk, computed_at_utc, received_at_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    ast,
                    day,
                    ex_idx,
                    ca_mg,
                    d3_iu,
                    prot_score,
                    risk,
                    "2026-11-14T19:55:00Z",
                    "2026-11-14T19:55:01Z",
                ],
            )?;
        }

        // 10. Seed hub_lab_reports (idempotent explicit report_id)
        let labs = [
            (
                1_i64,
                Some("AST-01"),
                "OSD-575",
                "IMMUNE",
                "2026-11-14",
                "IL-6 CYTOKINE",
                2.4_f64,
                "pg/mL",
                "NOMINAL",
            ),
            (
                2_i64,
                Some("AST-03"),
                "OSD-656",
                "IMMUNE",
                "2026-11-14",
                "CORTISOL SALIVARY",
                19.8_f64,
                "nmol/L",
                "ELEVATED",
            ),
            (
                3_i64,
                None,
                "OSD-573",
                "MICROBE",
                "2026-11-14",
                "AIRLOCK SURFACE CFU",
                120.0_f64,
                "CFU/100cm2",
                "NOMINAL",
            ),
            (
                4_i64,
                Some("AST-02"),
                "OSD-570",
                "IMMUNE",
                "2026-11-14",
                "CD4/CD8 T-CELL RATIO",
                1.75_f64,
                "ratio",
                "NOMINAL",
            ),
            (
                5_i64,
                None,
                "OSD-280",
                "MICROBE",
                "2026-11-14",
                "WATER DISPENSER BIOFILM",
                45.0_f64,
                "CFU/100mL",
                "NOMINAL",
            ),
        ];

        for (rep_id, ast, ds, dom, date, marker, val, unit, flag) in labs {
            self.conn.execute(
                "INSERT OR IGNORE INTO hub_lab_reports
                 (report_id, astronaut_id, source_dataset, domain, sample_date, biomarker_name, measured_value, unit, status_flag)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![rep_id, ast, ds, dom, date, marker, val, unit, flag],
            )?;
        }

        Ok(())
    }

    pub fn load_crew_members(&self) -> Result<Vec<CrewMember>> {
        let mut stmt = self.conn.prepare(
            "SELECT astronaut_id, full_name, role, weight, height, age, sex, active_phase_code, lan_status, last_heartbeat_utc, mission_start_utc, sleep_target_min
             FROM crew_members ORDER BY astronaut_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CrewMember {
                astronaut_id: row.get(0)?,
                full_name: row.get(1)?,
                role: row.get(2)?,
                weight: row.get(3)?,
                height: row.get(4)?,
                age: row.get(5)?,
                sex: row.get(6)?,
                active_phase_code: row.get(7)?,
                lan_status: row.get(8)?,
                last_heartbeat_utc: row.get(9)?,
                mission_start_utc: row.get(10)?,
                sleep_target_min: row.get(11)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_latest_telemetry(
        &self,
        astronaut_id: &str,
    ) -> Result<Option<TelemetryHistoryRecord>> {
        self.conn
            .query_row(
                "SELECT
                    record_id, astronaut_id, phase_code, mission_day, edge_seq_id,
                    recorded_at_utc, received_at_utc,
                    hr_bpm, sbp_mmhg, resp_brpm, spo2_pct, tissue_o2_pct, blood_n2_idx,
                    g_load, imu_activity_idx, cabin_co2_ppm, cabin_temp_c,
                    cabin_humidity_pct, cabin_pressure_kpa, cabin_o2_pct, cabin_motion_idx,
                    suit_pressure_kpa, suit_humidity_pct, eva_elapsed_sec,
                    radiation_usv_hr, shock_index, orthostatic_risk, sans_risk
                 FROM telemetry_history
                 WHERE astronaut_id = ?1
                 ORDER BY record_id DESC
                 LIMIT 1",
                params![astronaut_id],
                |row| {
                    Ok(TelemetryHistoryRecord {
                        record_id: row.get(0)?,
                        astronaut_id: row.get(1)?,
                        phase_code: row.get(2)?,
                        mission_day: row.get(3)?,
                        edge_seq_id: row.get(4)?,
                        recorded_at_utc: row.get(5)?,
                        received_at_utc: row.get(6)?,
                        hr_bpm: row.get(7)?,
                        sbp_mmhg: row.get(8)?,
                        resp_brpm: row.get(9)?,
                        spo2_pct: row.get(10)?,
                        tissue_o2_pct: row.get(11)?,
                        blood_n2_idx: row.get(12)?,
                        g_load: row.get(13)?,
                        imu_activity_idx: row.get(14)?,
                        cabin_co2_ppm: row.get(15)?,
                        cabin_temp_c: row.get(16)?,
                        cabin_humidity_pct: row.get(17)?,
                        cabin_pressure_kpa: row.get(18)?,
                        cabin_o2_pct: row.get(19)?,
                        cabin_motion_idx: row.get(20)?,
                        suit_pressure_kpa: row.get(21)?,
                        suit_humidity_pct: row.get(22)?,
                        eva_elapsed_sec: row.get(23)?,
                        radiation_usv_hr: row.get(24)?,
                        shock_index: row.get(25)?,
                        orthostatic_risk: row.get(26)?,
                        sans_risk: row.get(27)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_latest_ephemeris(&self) -> Result<Option<MissionEphemeris>> {
        self.conn
            .query_row(
                "SELECT mission_date, distance_au, one_way_light_min
                 FROM mission_ephemeris
                 ORDER BY mission_date DESC
                 LIMIT 1",
                [],
                |row| {
                    Ok(MissionEphemeris {
                        mission_date: row.get(0)?,
                        distance_au: row.get(1)?,
                        one_way_light_min: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_ephemeris_all(&self) -> Result<Vec<MissionEphemeris>> {
        let mut stmt = self.conn.prepare(
            "SELECT mission_date, distance_au, one_way_light_min
             FROM mission_ephemeris
             ORDER BY mission_date ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MissionEphemeris {
                mission_date: row.get(0)?,
                distance_au: row.get(1)?,
                one_way_light_min: row.get(2)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_food_catalog(&self) -> Result<Vec<FoodCatalogItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT food_id, item_name, category, kcal, protein_g, sodium_mg, potassium_mg, calcium_mg, vit_d3_iu, fluid_ml, is_low_sodium
             FROM iss_food_catalog ORDER BY food_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FoodCatalogItem {
                food_id: row.get(0)?,
                item_name: row.get(1)?,
                category: row.get(2)?,
                kcal: row.get(3)?,
                protein_g: row.get(4)?,
                sodium_mg: row.get(5)?,
                potassium_mg: row.get(6)?,
                calcium_mg: row.get(7)?,
                vit_d3_iu: row.get(8)?,
                fluid_ml: row.get(9)?,
                is_low_sodium: row.get(10)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_surgeon_incidents(&self) -> Result<Vec<SurgeonIncident>> {
        let mut stmt = self.conn.prepare(
            "SELECT incident_id, astronaut_id, telemetry_record_id, created_at_utc, hazard_type, injury_details, medication_administered, medical_notes, surgeon_ack
             FROM surgeon_incidents ORDER BY incident_id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SurgeonIncident {
                incident_id: row.get(0)?,
                astronaut_id: row.get(1)?,
                telemetry_record_id: row.get(2)?,
                created_at_utc: row.get(3)?,
                hazard_type: row.get(4)?,
                injury_details: row.get(5)?,
                medication_administered: row.get(6)?,
                medical_notes: row.get(7)?,
                surgeon_ack: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn resolve_surgeon_incident(&self, incident_id: i64, resolved: bool) -> Result<()> {
        let ack_val = if resolved { 1_i64 } else { 0_i64 };
        self.conn.execute(
            "UPDATE surgeon_incidents SET surgeon_ack = ?1 WHERE incident_id = ?2",
            params![ack_val, incident_id],
        )?;
        Ok(())
    }

    pub fn insert_surgeon_incident(&self, inc: &NewSurgeonIncident<'_>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO surgeon_incidents
             (astronaut_id, telemetry_record_id, created_at_utc, hazard_type, injury_details, medication_administered, medical_notes, surgeon_ack)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                inc.astronaut_id,
                inc.telemetry_record_id,
                inc.created_at_utc,
                inc.hazard_type,
                inc.injury_details,
                inc.medication_administered,
                inc.medical_notes,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn load_hub_lab_reports(&self) -> Result<Vec<HubLabReport>> {
        let mut stmt = self.conn.prepare(
            "SELECT report_id, astronaut_id, source_dataset, domain, sample_date, biomarker_name, measured_value, unit, status_flag
             FROM hub_lab_reports ORDER BY report_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HubLabReport {
                report_id: row.get(0)?,
                astronaut_id: row.get(1)?,
                source_dataset: row.get(2)?,
                domain: row.get(3)?,
                sample_date: row.get(4)?,
                biomarker_name: row.get(5)?,
                measured_value: row.get(6)?,
                unit: row.get(7)?,
                status_flag: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn seed_nasa_data(&mut self) -> Result<()> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM dataset_provenance",
            [],
            |row| row.get(0),
        )?;
        if count == 0 {
            self.conn.execute_batch(include_str!("../data/nasa_seed.sql"))?;
        }
        Ok(())
    }

    pub fn load_radiation_series(&self) -> Result<Vec<ReferenceRadiationPoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp_utc, instrument_id, absorbed_dose_rate_ugy_hr, interval_sec, interval_dose_ugy
             FROM reference_radiation_series ORDER BY timestamp_utc ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ReferenceRadiationPoint {
                timestamp_utc: row.get(0)?,
                instrument_id: row.get(1)?,
                absorbed_dose_rate_ugy_hr: row.get(2)?,
                interval_sec: row.get(3)?,
                interval_dose_ugy: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_eda_environment(&self) -> Result<Vec<ReferenceEdaPoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT recorded_at_utc, co2_iss_ppm, co2_ground_ppm, temp_iss_c, temp_ground_c, humidity_iss_pct, humidity_ground_pct
             FROM reference_eda_environment ORDER BY recorded_at_utc ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ReferenceEdaPoint {
                recorded_at_utc: row.get(0)?,
                co2_iss_ppm: row.get(1)?,
                co2_ground_ppm: row.get(2)?,
                temp_iss_c: row.get(3)?,
                temp_ground_c: row.get(4)?,
                humidity_iss_pct: row.get(5)?,
                humidity_ground_pct: row.get(6)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_research_lab_observations(
        &self,
        subject_code: Option<&str>,
        domain: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ResearchLabObservation>> {
        let mut stmt = self.conn.prepare(
            "SELECT obs_id, study_id, domain, sample_name, subject_code, sample_matrix, timepoint,
                    sort_order, biomarker_name, measured_value, unit, ref_min, ref_max, status_flag, source_file
             FROM research_lab_observations
             WHERE (?1 IS NULL OR subject_code = ?1)
               AND (?2 IS NULL OR domain = ?2)
             ORDER BY subject_code ASC, sort_order ASC, biomarker_name ASC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![subject_code, domain, limit as i64], |row| {
            Ok(ResearchLabObservation {
                obs_id: row.get(0)?,
                study_id: row.get(1)?,
                domain: row.get(2)?,
                sample_name: row.get(3)?,
                subject_code: row.get(4)?,
                sample_matrix: row.get(5)?,
                timepoint: row.get(6)?,
                sort_order: row.get(7)?,
                biomarker_name: row.get(8)?,
                measured_value: row.get(9)?,
                unit: row.get(10)?,
                ref_min: row.get(11)?,
                ref_max: row.get(12)?,
                status_flag: row.get(13)?,
                source_file: row.get(14)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_dataset_provenance(&self) -> Result<Vec<DatasetProvenance>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_name, dataset_id, sha256, source_url, byte_size, imported_at_utc
             FROM dataset_provenance ORDER BY dataset_id ASC, file_name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DatasetProvenance {
                file_name: row.get(0)?,
                dataset_id: row.get(1)?,
                sha256: row.get(2)?,
                source_url: row.get(3)?,
                byte_size: row.get(4)?,
                imported_at_utc: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_sleep_summaries(&self) -> Result<Vec<SleepDailySummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT astronaut_id, mission_day, total_sleep_min, target_sleep_min, sleep_efficiency_pct, awakenings, computed_at_utc
             FROM sleep_daily_summary ORDER BY mission_day DESC, astronaut_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SleepDailySummary {
                astronaut_id: row.get(0)?,
                mission_day: row.get(1)?,
                total_sleep_min: row.get(2)?,
                target_sleep_min: row.get(3)?,
                sleep_efficiency_pct: row.get(4)?,
                awakenings: row.get(5)?,
                computed_at_utc: row.get(6)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_sans_history(&self) -> Result<Vec<SansDailyHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT astronaut_id, mission_day, sodium_mg_total, avg_cabin_co2_ppm, max_cabin_co2_ppm, sans_score, sans_risk, computed_at_utc, received_at_utc
             FROM sans_daily_history ORDER BY mission_day DESC, astronaut_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SansDailyHistory {
                astronaut_id: row.get(0)?,
                mission_day: row.get(1)?,
                sodium_mg_total: row.get(2)?,
                avg_cabin_co2_ppm: row.get(3)?,
                max_cabin_co2_ppm: row.get(4)?,
                sans_score: row.get(5)?,
                sans_risk: row.get(6)?,
                computed_at_utc: row.get(7)?,
                received_at_utc: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_osteo_history(&self) -> Result<Vec<OsteoporosisDailyHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT astronaut_id, mission_day, exercise_load_idx, calcium_mg_total, vit_d3_iu_total, protection_score, osteo_risk, computed_at_utc, received_at_utc
             FROM osteoporosis_daily_history ORDER BY mission_day DESC, astronaut_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(OsteoporosisDailyHistory {
                astronaut_id: row.get(0)?,
                mission_day: row.get(1)?,
                exercise_load_idx: row.get(2)?,
                calcium_mg_total: row.get(3)?,
                vit_d3_iu_total: row.get(4)?,
                protection_score: row.get(5)?,
                osteo_risk: row.get(6)?,
                computed_at_utc: row.get(7)?,
                received_at_utc: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn load_crew_input_history(&self) -> Result<Vec<CrewInputHistoryRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT record_id, astronaut_id, edge_input_id, logged_at_utc, mission_day, input_type, reference_code, consumption_pct, notes, received_at_utc
             FROM crew_input_history ORDER BY record_id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CrewInputHistoryRecord {
                record_id: row.get(0)?,
                astronaut_id: row.get(1)?,
                edge_input_id: row.get(2)?,
                logged_at_utc: row.get(3)?,
                mission_day: row.get(4)?,
                input_type: row.get(5)?,
                reference_code: row.get(6)?,
                consumption_pct: row.get(7)?,
                notes: row.get(8)?,
                received_at_utc: row.get(9)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn update_crew_phase(&self, astronaut_id: &str, phase_code: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE crew_members SET active_phase_code = ?1 WHERE astronaut_id = ?2",
            params![phase_code, astronaut_id],
        )?;
        Ok(())
    }

    pub fn count_unacked_incidents(&self) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM surgeon_incidents WHERE surgeon_ack = 0",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn has_open_incident(&self, astronaut_id: &str, hazard_type: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM surgeon_incidents
             WHERE astronaut_id = ?1 AND hazard_type = ?2 AND surgeon_ack = 0",
            params![astronaut_id, hazard_type],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn next_internal_edge_seq_id(&self, astronaut_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COALESCE((SELECT MIN(edge_seq_id) FROM telemetry_history
                                  WHERE astronaut_id = ?1 AND edge_seq_id < 0), 0) - 1",
                params![astronaut_id],
                |r| r.get(0),
            )
            .map_err(Into::into)
    }

    pub fn next_internal_edge_input_id(&self, astronaut_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT COALESCE((SELECT MIN(edge_input_id) FROM crew_input_history
                                  WHERE astronaut_id = ?1 AND edge_input_id < 0), 0) - 1",
                params![astronaut_id],
                |r| r.get(0),
            )
            .map_err(Into::into)
    }

    pub fn update_crew_lan_heartbeat(
        &self,
        astronaut_id: &str,
        phase_code: Option<&str>,
        lan_status: &str,
        heartbeat_utc: &str,
    ) -> Result<()> {
        if let Some(phase) = phase_code {
            self.conn.execute(
                "UPDATE crew_members
                 SET active_phase_code = ?1, lan_status = ?2, last_heartbeat_utc = ?3
                 WHERE astronaut_id = ?4",
                params![phase, lan_status, heartbeat_utc, astronaut_id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE crew_members
                 SET lan_status = ?1, last_heartbeat_utc = ?2
                 WHERE astronaut_id = ?3",
                params![lan_status, heartbeat_utc, astronaut_id],
            )?;
        }
        Ok(())
    }

    pub fn upsert_telemetry_frame(&self, frame: &InboundTelemetryFrame<'_>) -> Result<i64> {
        let received_at = calc::utc_now_iso8601();
        self.conn.execute(
            "INSERT INTO telemetry_history (
                astronaut_id, phase_code, mission_day, edge_seq_id,
                recorded_at_utc, received_at_utc,
                hr_bpm, sbp_mmhg, resp_brpm, spo2_pct, tissue_o2_pct, blood_n2_idx,
                g_load, imu_activity_idx, cabin_co2_ppm, cabin_temp_c,
                cabin_humidity_pct, cabin_pressure_kpa, cabin_o2_pct, cabin_motion_idx,
                suit_pressure_kpa, suit_humidity_pct, eva_elapsed_sec,
                radiation_usv_hr, shock_index, orthostatic_risk, sans_risk
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6,
                ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20,
                ?21, ?22, ?23,
                ?24, ?25, ?26, ?27
            )
            ON CONFLICT(astronaut_id, edge_seq_id) DO NOTHING",
            params![
                frame.astronaut_id,
                frame.phase_code,
                frame.mission_day,
                frame.edge_seq_id,
                frame.recorded_at_utc,
                received_at,
                frame.hr_bpm,
                frame.sbp_mmhg,
                frame.resp_brpm,
                frame.spo2_pct,
                frame.tissue_o2_pct,
                frame.blood_n2_idx,
                frame.g_load,
                frame.imu_activity_idx,
                frame.cabin_co2_ppm,
                frame.cabin_temp_c,
                frame.cabin_humidity_pct,
                frame.cabin_pressure_kpa,
                frame.cabin_o2_pct,
                frame.cabin_motion_idx,
                frame.suit_pressure_kpa,
                frame.suit_humidity_pct,
                frame.eva_elapsed_sec,
                frame.radiation_usv_hr,
                frame.shock_index,
                frame.orthostatic_risk,
                frame.sans_risk,
            ],
        )?;
        let record_id: i64 = self.conn.query_row(
            "SELECT record_id FROM telemetry_history WHERE astronaut_id = ?1 AND edge_seq_id = ?2",
            params![frame.astronaut_id, frame.edge_seq_id],
            |r| r.get(0),
        )?;
        Ok(record_id)
    }

    pub fn load_crew_daily_nutrition(
        &self,
        astronaut_id: &str,
        mission_day: i64,
    ) -> Result<HubCrewDailyNutrition> {
        self.conn
            .query_row(
                "SELECT
                    COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.kcal * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                    COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.sodium_mg * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                    COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.calcium_mg * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                    COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.vit_d3_iu * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                    COALESCE(SUM(
                        CASE
                            WHEN c.input_type = 'FLUID_LOG' AND f.food_id IS NULL THEN 250.0 * COALESCE(c.consumption_pct, 100) / 100.0
                            WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.fluid_ml * COALESCE(c.consumption_pct, 100) / 100.0
                            ELSE 0.0
                        END
                    ), 0.0),
                    COALESCE(SUM(CASE WHEN c.input_type = 'MEAL_LOG' THEN 1 ELSE 0 END), 0)
                 FROM crew_input_history c
                 LEFT JOIN iss_food_catalog f ON c.reference_code = f.food_id
                 WHERE c.astronaut_id = ?1 AND c.mission_day = ?2",
                params![astronaut_id, mission_day],
                |row| {
                    Ok(HubCrewDailyNutrition {
                        kcal_total: row.get(0)?,
                        sodium_mg_total: row.get(1)?,
                        calcium_mg_total: row.get(2)?,
                        vit_d3_iu_total: row.get(3)?,
                        fluid_ml_total: row.get(4)?,
                        meal_count: row.get(5)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn load_crew_daily_sodium_map(&self, mission_day: i64) -> Result<HashMap<String, f64>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                c.astronaut_id,
                COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.sodium_mg * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0)
             FROM crew_input_history c
             LEFT JOIN iss_food_catalog f ON c.reference_code = f.food_id
             WHERE c.mission_day = ?1
             GROUP BY c.astronaut_id",
        )?;
        let rows = stmt.query_map(params![mission_day], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })?;
        let mut out = HashMap::new();
        for r in rows {
            let (id, na) = r?;
            out.insert(id, na);
        }
        Ok(out)
    }

    pub fn load_all_crew_daily_nutrition(
        &self,
        mission_day: i64,
    ) -> Result<HashMap<String, HubCrewDailyNutrition>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                c.astronaut_id,
                COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.kcal * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.sodium_mg * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.calcium_mg * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                COALESCE(SUM(CASE WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.vit_d3_iu * COALESCE(c.consumption_pct, 100) / 100.0 ELSE 0.0 END), 0.0),
                COALESCE(SUM(
                    CASE
                        WHEN c.input_type = 'FLUID_LOG' AND f.food_id IS NULL THEN 250.0 * COALESCE(c.consumption_pct, 100) / 100.0
                        WHEN c.input_type IN ('MEAL_LOG', 'FLUID_LOG') AND f.food_id IS NOT NULL THEN f.fluid_ml * COALESCE(c.consumption_pct, 100) / 100.0
                        ELSE 0.0
                    END
                ), 0.0),
                COALESCE(SUM(CASE WHEN c.input_type = 'MEAL_LOG' THEN 1 ELSE 0 END), 0)
             FROM crew_input_history c
             LEFT JOIN iss_food_catalog f ON c.reference_code = f.food_id
             WHERE c.mission_day = ?1
             GROUP BY c.astronaut_id",
        )?;
        let rows = stmt.query_map(params![mission_day], |row| {
            Ok((
                row.get::<_, String>(0)?,
                HubCrewDailyNutrition {
                    kcal_total: row.get(1)?,
                    sodium_mg_total: row.get(2)?,
                    calcium_mg_total: row.get(3)?,
                    vit_d3_iu_total: row.get(4)?,
                    fluid_ml_total: row.get(5)?,
                    meal_count: row.get(6)?,
                },
            ))
        })?;
        let mut out = HashMap::new();
        for r in rows {
            let (id, nut) = r?;
            out.insert(id, nut);
        }
        Ok(out)
    }

    pub fn upsert_sans_daily(
        &self,
        astronaut_id: &str,
        mission_day: i64,
        sodium_mg_total: f64,
        avg_cabin_co2_ppm: f64,
        max_cabin_co2_ppm: f64,
        sans_score: f64,
        sans_risk: &str,
        computed_at_utc: &str,
    ) -> Result<()> {
        let received_at = calc::utc_now_iso8601();
        self.conn.execute(
            "INSERT INTO sans_daily_history
             (astronaut_id, mission_day, sodium_mg_total, avg_cabin_co2_ppm, max_cabin_co2_ppm, sans_score, sans_risk, computed_at_utc, received_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(astronaut_id, mission_day) DO UPDATE SET
                 sodium_mg_total = excluded.sodium_mg_total,
                 avg_cabin_co2_ppm = excluded.avg_cabin_co2_ppm,
                 max_cabin_co2_ppm = excluded.max_cabin_co2_ppm,
                 sans_score = excluded.sans_score,
                 sans_risk = excluded.sans_risk,
                 computed_at_utc = excluded.computed_at_utc,
                 received_at_utc = excluded.received_at_utc",
            params![
                astronaut_id,
                mission_day,
                sodium_mg_total,
                avg_cabin_co2_ppm,
                max_cabin_co2_ppm,
                sans_score,
                sans_risk,
                computed_at_utc,
                received_at,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_osteoporosis_daily(
        &self,
        astronaut_id: &str,
        mission_day: i64,
        exercise_load_idx: f64,
        calcium_mg_total: f64,
        vit_d3_iu_total: f64,
        protection_score: f64,
        osteo_risk: &str,
        computed_at_utc: &str,
    ) -> Result<()> {
        let received_at = calc::utc_now_iso8601();
        self.conn.execute(
            "INSERT INTO osteoporosis_daily_history
             (astronaut_id, mission_day, exercise_load_idx, calcium_mg_total, vit_d3_iu_total, protection_score, osteo_risk, computed_at_utc, received_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(astronaut_id, mission_day) DO UPDATE SET
                 exercise_load_idx = excluded.exercise_load_idx,
                 calcium_mg_total = excluded.calcium_mg_total,
                 vit_d3_iu_total = excluded.vit_d3_iu_total,
                 protection_score = excluded.protection_score,
                 osteo_risk = excluded.osteo_risk,
                 computed_at_utc = excluded.computed_at_utc,
                 received_at_utc = excluded.received_at_utc",
            params![
                astronaut_id,
                mission_day,
                exercise_load_idx,
                calcium_mg_total,
                vit_d3_iu_total,
                protection_score,
                osteo_risk,
                computed_at_utc,
                received_at,
            ],
        )?;
        Ok(())
    }

    pub fn recompute_crew_daily_states(
        &self,
        astronaut_id: &str,
        mission_day: i64,
        phase: &str,
        fallback_co2_ppm: f64,
    ) -> Result<()> {
        let nut = self.load_crew_daily_nutrition(astronaut_id, mission_day)?;
        let (avg_co2, max_co2): (f64, f64) = self
            .conn
            .query_row(
                "SELECT COALESCE(AVG(cabin_co2_ppm), ?3), COALESCE(MAX(cabin_co2_ppm), ?3)
                 FROM telemetry_history
                 WHERE astronaut_id = ?1 AND mission_day = ?2",
                params![astronaut_id, mission_day, fallback_co2_ppm],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((fallback_co2_ppm, fallback_co2_ppm));
        let now_utc = calc::utc_now_iso8601();
        let (sans_score, sans_risk) = calc::calc_sans_risk(nut.sodium_mg_total, avg_co2, phase);
        self.upsert_sans_daily(
            astronaut_id,
            mission_day,
            nut.sodium_mg_total,
            avg_co2,
            max_co2,
            sans_score,
            sans_risk,
            &now_utc,
        )?;

        let ex_units: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(COALESCE(consumption_pct, 100) / 100.0), 0.0)
             FROM crew_input_history
             WHERE astronaut_id = ?1 AND mission_day = ?2 AND input_type = 'EXERCISE'",
            params![astronaut_id, mission_day],
            |r| r.get(0),
        )?;
        let ex_load = (75.0 + ex_units * 15.0).clamp(0.0, 120.0);
        let (prot_score, osteo_risk) =
            calc::calc_osteoporosis_risk(ex_load, nut.calcium_mg_total, nut.vit_d3_iu_total);
        self.upsert_osteoporosis_daily(
            astronaut_id,
            mission_day,
            ex_load,
            nut.calcium_mg_total,
            nut.vit_d3_iu_total,
            prot_score,
            osteo_risk,
            &now_utc,
        )?;
        Ok(())
    }

    pub fn upsert_crew_input(
        &self,
        astronaut_id: &str,
        edge_input_id: i64,
        logged_at_utc: &str,
        mission_day: i64,
        input_type: &str,
        reference_code: Option<&str>,
        consumption_pct: Option<i64>,
        notes: Option<&str>,
        current_phase: &str,
        fallback_co2_ppm: f64,
    ) -> Result<()> {
        let received_at = calc::utc_now_iso8601();
        self.conn.execute(
            "INSERT INTO crew_input_history
             (astronaut_id, edge_input_id, logged_at_utc, mission_day, input_type, reference_code, consumption_pct, notes, received_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(astronaut_id, edge_input_id) DO NOTHING",
            params![
                astronaut_id,
                edge_input_id,
                logged_at_utc,
                mission_day,
                input_type,
                reference_code,
                consumption_pct,
                notes,
                received_at,
            ],
        )?;
        self.recompute_crew_daily_states(astronaut_id, mission_day, current_phase, fallback_co2_ppm)?;
        Ok(())
    }

    pub fn insert_manual_meal_log(
        &self,
        astronaut_id: &str,
        mission_day: i64,
        food_id: &str,
        consumption_pct: i64,
        current_phase: &str,
        fallback_co2_ppm: f64,
    ) -> Result<()> {
        let edge_input_id = self.next_internal_edge_input_id(astronaut_id)?;
        let item_name: Option<String> = self
            .conn
            .query_row(
                "SELECT item_name FROM iss_food_catalog WHERE food_id = ?1",
                params![food_id],
                |r| r.get(0),
            )
            .optional()?;
        let now_utc = calc::utc_now_iso8601();
        self.upsert_crew_input(
            astronaut_id,
            edge_input_id,
            &now_utc,
            mission_day,
            "MEAL_LOG",
            Some(food_id),
            Some(consumption_pct),
            item_name.as_deref(),
            current_phase,
            fallback_co2_ppm,
        )
    }

    pub fn upsert_sleep_epoch(
        &self,
        astronaut_id: &str,
        edge_epoch_id: i64,
        epoch_start_utc: &str,
        mission_day: i64,
        activity_count: f64,
        hr_bpm_avg: Option<f64>,
        raw_sleep_state: &str,
    ) -> Result<()> {
        let normalized_state = if raw_sleep_state.eq_ignore_ascii_case("WAKE") {
            "WAKE"
        } else {
            "SLEEP"
        };
        let received_at = calc::utc_now_iso8601();
        let inserted = self.conn.execute(
            "INSERT INTO sleep_epoch_history
             (astronaut_id, edge_epoch_id, epoch_start_utc, mission_day, activity_count, hr_bpm_avg, sleep_state, received_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(astronaut_id, edge_epoch_id) DO NOTHING",
            params![
                astronaut_id,
                edge_epoch_id,
                epoch_start_utc,
                mission_day,
                activity_count,
                hr_bpm_avg,
                normalized_state,
                received_at,
            ],
        )?;
        if inserted > 0 {
            let target_min: i64 = self
                .conn
                .query_row(
                    "SELECT sleep_target_min FROM crew_members WHERE astronaut_id = ?1",
                    params![astronaut_id],
                    |r| r.get(0),
                )
                .unwrap_or(480);
            let (cur_sleep, cur_awake): (f64, i64) = self
                .conn
                .query_row(
                    "SELECT total_sleep_min, COALESCE(awakenings, 0) FROM sleep_daily_summary
                     WHERE astronaut_id = ?1 AND mission_day = ?2",
                    params![astronaut_id, mission_day],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap_or((0.0, 0));
            let next_sleep = if normalized_state == "SLEEP" {
                cur_sleep + 1.0
            } else {
                cur_sleep
            };
            let next_awake = if normalized_state == "WAKE" {
                cur_awake + 1
            } else {
                cur_awake
            };
            let eff = (next_sleep / target_min.max(1) as f64 * 100.0).clamp(0.0, 100.0);
            let now_utc = calc::utc_now_iso8601();
            self.conn.execute(
                "INSERT INTO sleep_daily_summary
                 (astronaut_id, mission_day, total_sleep_min, target_sleep_min, sleep_efficiency_pct, awakenings, computed_at_utc)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(astronaut_id, mission_day) DO UPDATE SET
                     total_sleep_min = excluded.total_sleep_min,
                     sleep_efficiency_pct = excluded.sleep_efficiency_pct,
                     awakenings = excluded.awakenings,
                     computed_at_utc = excluded.computed_at_utc",
                params![
                    astronaut_id,
                    mission_day,
                    next_sleep,
                    target_min,
                    eff,
                    next_awake,
                    now_utc,
                ],
            )?;
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_and_load_all_tables() {
        let mut db = Database::open_in_memory_fallback();
        db.seed_demo_data().expect("seed_demo_data must succeed");
        // Verify idempotency by seeding a second time
        db.seed_demo_data()
            .expect("second seed_demo_data must succeed");

        let crew = db.load_crew_members().expect("load crew");
        assert_eq!(crew.len(), 4);

        let food = db.load_food_catalog().expect("load food");
        assert!(food.len() >= 4);

        let ephem_all = db.load_ephemeris_all().expect("load ephemeris all");
        assert!(ephem_all.len() >= 3);
        let latest_ephem = db
            .load_latest_ephemeris()
            .expect("load latest ephemeris")
            .expect("must have ephemeris");
        assert_eq!(
            latest_ephem.mission_date,
            ephem_all.last().unwrap().mission_date
        );

        let incidents = db.load_surgeon_incidents().expect("load incidents");
        assert_eq!(incidents.len(), 3);
        let unacked_before = db.count_unacked_incidents().expect("count unacked");
        assert_eq!(unacked_before, 1);

        db.resolve_surgeon_incident(1, true)
            .expect("resolve incident 1");
        assert_eq!(db.count_unacked_incidents().unwrap(), 0);

        let new_id = db
            .insert_surgeon_incident(&NewSurgeonIncident {
                astronaut_id: "AST-01",
                telemetry_record_id: Some(1),
                created_at_utc: "2026-11-14T21:00:00Z",
                hazard_type: "HYPOXIA",
                injury_details: Some("Simulated SpO2 drop"),
                medication_administered: Some("O2 Flow Boost"),
                medical_notes: Some("Flight surgeon drill"),
            })
            .expect("insert incident");
        assert!(new_id > 0);
        assert_eq!(db.count_unacked_incidents().unwrap(), 1);

        let labs = db.load_hub_lab_reports().expect("load labs");
        assert!(!labs.is_empty());

        let sleeps = db.load_sleep_summaries().expect("load sleep");
        assert_eq!(sleeps.len(), 4);

        let sans = db.load_sans_history().expect("load sans");
        assert_eq!(sans.len(), 4);

        let osteo = db.load_osteo_history().expect("load osteo");
        assert_eq!(osteo.len(), 4);

        let inputs = db.load_crew_input_history().expect("load crew inputs");
        assert!(!inputs.is_empty());

        // Verify real NASA datasets imported via nasa_seed.sql
        assert_eq!(ephem_all.len(), 8);
        assert!((ephem_all[0].distance_au - 1.324362).abs() < 1e-4);

        let rad = db.load_radiation_series().expect("load radlab");
        assert_eq!(rad.len(), 1044);

        let eda = db.load_eda_environment().expect("load eda");
        assert_eq!(eda.len(), 289);

        let cbc = db
            .load_research_lab_observations(Some("C001"), Some("CBC"), 500)
            .expect("load research labs");
        assert!(!cbc.is_empty());
        assert!(cbc.windows(2).all(|w| w[0].sort_order <= w[1].sort_order));

        let prov = db.load_dataset_provenance().expect("load provenance");
        assert_eq!(prov.len(), 11);

        // Idempotency check: re-running seed_nasa_data must keep counts unchanged
        db.seed_nasa_data().expect("idempotent reseed");
        assert_eq!(db.load_dataset_provenance().unwrap().len(), 11);
    }

    #[test]
    fn test_nasa_data_through_calc_logic() {
        let mut db = Database::open_in_memory_fallback();
        db.seed_demo_data().expect("seed_demo_data");

        // 1. Ephemeris: Compare JPL one_way_light_min vs calc::calc_one_way_light_min
        let ephem = db.load_ephemeris_all().expect("ephem");
        let mut max_err_min = 0.0_f64;
        for row in &ephem {
            let calc_min = calc::calc_one_way_light_min(row.distance_au);
            let err = (calc_min - row.one_way_light_min).abs();
            if err > max_err_min {
                max_err_min = err;
            }
            assert!(err < 0.01, "light delay mismatch on {}: {}", row.mission_date, err);
        }
        let (interp_au, interp_min) =
            calc::interpolate_ephemeris(ephem[0].distance_au, ephem[1].distance_au, 0.5);
        println!(
            "[EPHEMERIS] rows={}, day0={} ({:.6} AU, JPL={:.4} min, calc={:.4} min), max_err={:.6} min, mid_interp=({:.6} AU, {:.4} min)",
            ephem.len(),
            ephem[0].mission_date,
            ephem[0].distance_au,
            ephem[0].one_way_light_min,
            calc::calc_one_way_light_min(ephem[0].distance_au),
            max_err_min,
            interp_au,
            interp_min
        );

        // 2. EDA RR-1 Environment (289 pts): evaluate_safety & calc_sans_risk
        let eda = db.load_eda_environment().expect("eda");
        let iss_co2: Vec<f64> = eda.iter().filter_map(|e| e.co2_iss_ppm).collect();
        let gnd_co2: Vec<f64> = eda.iter().filter_map(|e| e.co2_ground_ppm).collect();
        let iss_min = iss_co2.iter().copied().fold(f64::INFINITY, f64::min);
        let iss_max = iss_co2.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let iss_avg = iss_co2.iter().sum::<f64>() / iss_co2.len() as f64;
        let gnd_avg = gnd_co2.iter().sum::<f64>() / gnd_co2.len() as f64;

        let mut hazard_triggers = 0_usize;
        for &co2 in &iss_co2 {
            let alerts = calc::evaluate_safety(&calc::SafetyEvalInput {
                phase: "ORBIT",
                hr_bpm: 78.0,
                resting_hr_bpm: 68.0,
                sbp_mmhg: 118.0,
                spo2_pct: 98.0,
                tissue_o2_pct: Some(94.0),
                blood_n2_idx: Some(1.0),
                suit_pressure_kpa: None,
                radiation_usv_hr: 0.40,
                cabin_co2_ppm: co2,
                cabin_o2_pct: Some(20.9),
                hub_distance_m: 150.0,
                comms_online: true,
            });
            if alerts.hazard_danger {
                hazard_triggers += 1;
            }
        }
        let (sans_low_na, tier_low_na) = calc::calc_sans_risk(1500.0, iss_avg, "ORBIT");
        let (sans_high_na, tier_high_na) = calc::calc_sans_risk(3200.0, iss_max, "ORBIT");
        println!(
            "[EDA CO2] pts={}, ISS min/avg/max={:.1}/{:.1}/{:.1} ppm, Ground avg={:.1} ppm, hazard_triggers(>3000ppm)={}, SANS(1500mg Na, avg CO2)={:.2} ({}), SANS(3200mg Na, max CO2)={:.2} ({})",
            eda.len(),
            iss_min,
            iss_avg,
            iss_max,
            gnd_avg,
            hazard_triggers,
            sans_low_na,
            tier_low_na,
            sans_high_na,
            tier_high_na
        );
        assert_eq!(eda.len(), 289);
        assert!(sans_low_na > 0.0 && sans_low_na <= 100.0);

        // 3. RadLab 2013-01-01 (1044 pts): cumulative dose & safety check
        let rad = db.load_radiation_series().expect("rad");
        let mut rad_min = f64::INFINITY;
        let mut rad_max = f64::NEG_INFINITY;
        let mut rad_sum = 0.0_f64;
        let mut cum_dose_ugy = 0.0_f64;
        let mut valid_intervals = 0_usize;
        let mut rad_hazards = 0_usize;
        for pt in &rad {
            let r = pt.absorbed_dose_rate_ugy_hr;
            rad_min = rad_min.min(r);
            rad_max = rad_max.max(r);
            rad_sum += r;
            if let Some(d) = pt.interval_dose_ugy {
                cum_dose_ugy += d;
                valid_intervals += 1;
            }
            let alerts = calc::evaluate_safety(&calc::SafetyEvalInput {
                phase: "ORBIT",
                hr_bpm: 78.0,
                resting_hr_bpm: 68.0,
                sbp_mmhg: 118.0,
                spo2_pct: 98.0,
                tissue_o2_pct: Some(94.0),
                blood_n2_idx: Some(1.0),
                suit_pressure_kpa: None,
                radiation_usv_hr: r,
                cabin_co2_ppm: 1200.0,
                cabin_o2_pct: Some(20.9),
                hub_distance_m: 150.0,
                comms_online: true,
            });
            if alerts.hazard_danger {
                rad_hazards += 1;
            }
        }
        println!(
            "[RADLAB] pts={}, min/avg/max={:.2}/{:.2}/{:.2} uGy/hr, valid_intervals={}, cum_dose={:.2} uGy, hazard_triggers(>50)={}",
            rad.len(),
            rad_min,
            rad_sum / rad.len() as f64,
            rad_max,
            valid_intervals,
            cum_dose_ugy,
            rad_hazards
        );
        assert_eq!(rad.len(), 1044);
        assert!(cum_dose_ugy > 0.0);

        // 4. Research Lab Observations (23,096 rows): domain breakdown & ref-range flags
        let all_obs = db
            .load_research_lab_observations(None, None, 30_000)
            .expect("all obs");
        let above = all_obs.iter().filter(|o| o.status_flag == "ABOVE_REF").count();
        let below = all_obs.iter().filter(|o| o.status_flag == "BELOW_REF").count();
        let in_rng = all_obs.iter().filter(|o| o.status_flag == "IN_RANGE").count();
        let no_ref = all_obs.iter().filter(|o| o.status_flag == "NO_REF").count();
        println!(
            "[RESEARCH LABS] total={}, IN_RANGE={}, ABOVE_REF={}, BELOW_REF={}, NO_REF={}",
            all_obs.len(),
            in_rng,
            above,
            below,
            no_ref
        );
        assert_eq!(all_obs.len(), 23096);

        // 5. Crew & Nutrition / Osteoporosis / Haldane N2 logic with seeded DB
        let (p_n2, n2_idx) = calc::update_tissue_n2(79.0, 0.0, 29.6, 2700.0);
        let (prot_score, osteo_tier) = calc::calc_osteoporosis_risk(85.0, 950.0, 720.0);
        let kcal_s = calc::calc_kcal_per_sec(130.0, 78.0, 35, "M");
        let sweat_s = calc::calc_sweat_ml_per_sec(kcal_s * 60.0, 37.4);
        println!(
            "[PHYSIOLOGY] Haldane 45m prebreathe(29.6kPa): pN2={:.2} kPa, R={:.2} | Osteo(85,950mg,720IU)={:.1} ({}) | Keytel(130bpm,78kg,35M)={:.3} kcal/s, sweat={:.4} mL/s",
            p_n2, n2_idx, prot_score, osteo_tier, kcal_s, sweat_s
        );
        assert!(n2_idx > 1.5);
        assert_eq!(osteo_tier, "LOW");
    }

    #[test]
    fn test_ingestion_dedup_nutrition_sans_and_sleep() {
        let mut db = Database::open_in_memory_fallback();
        db.seed_demo_data().expect("seed_demo_data");

        let before_nut = db.load_crew_daily_nutrition("AST-01", 1).unwrap();
        let na_map = db.load_crew_daily_sodium_map(1).unwrap();
        assert!(
            (*na_map.get("AST-01").unwrap() - before_nut.sodium_mg_total).abs() < 1e-6
        );

        // Upsert telemetry frame twice with identical edge_seq_id -> deduped cleanly
        let frame = InboundTelemetryFrame {
            astronaut_id: "AST-01",
            phase_code: "EVA",
            mission_day: 1,
            edge_seq_id: 99,
            recorded_at_utc: "2026-11-15T10:15:00Z",
            hr_bpm: 126.0,
            sbp_mmhg: 130.0,
            resp_brpm: 22.0,
            spo2_pct: 98.0,
            tissue_o2_pct: Some(93.5),
            blood_n2_idx: Some(1.06),
            g_load: 0.01,
            imu_activity_idx: 0.55,
            cabin_co2_ppm: 1250.0,
            cabin_temp_c: Some(21.5),
            cabin_humidity_pct: Some(44.0),
            cabin_pressure_kpa: Some(101.3),
            cabin_o2_pct: Some(20.9),
            cabin_motion_idx: 0.2,
            suit_pressure_kpa: Some(29.6),
            suit_humidity_pct: Some(39.0),
            eva_elapsed_sec: 120,
            radiation_usv_hr: 0.65,
            shock_index: 0.97,
            orthostatic_risk: "HIGH",
            sans_risk: "LOW",
        };
        let rec1 = db.upsert_telemetry_frame(&frame).unwrap();
        let rec2 = db.upsert_telemetry_frame(&frame).unwrap();
        assert_eq!(rec1, rec2);

        // Log manual meal FOOD-03 (690mg Na) + test SANS recalculation
        db.insert_manual_meal_log("AST-01", 1, "FOOD-03", 100, "EVA", 1250.0)
            .unwrap();
        let after_nut = db.load_crew_daily_nutrition("AST-01", 1).unwrap();
        assert!(after_nut.sodium_mg_total > before_nut.sodium_mg_total + 600.0);

        let sans_rows = db.load_sans_history().unwrap();
        let ast1_sans = sans_rows
            .iter()
            .find(|s| s.astronaut_id == "AST-01" && s.mission_day == 1)
            .unwrap();
        assert!((ast1_sans.sodium_mg_total - after_nut.sodium_mg_total).abs() < 1e-6);

        // Sleep epoch normalization ("DEEP" -> "SLEEP") increments summary once on dedup
        let prev_sleep = db
            .load_sleep_summaries()
            .unwrap()
            .into_iter()
            .find(|s| s.astronaut_id == "AST-01" && s.mission_day == 1)
            .unwrap()
            .total_sleep_min;
        db.upsert_sleep_epoch("AST-01", 501, "2026-11-15T10:16:00Z", 1, 2.5, Some(54.0), "DEEP")
            .unwrap();
        db.upsert_sleep_epoch("AST-01", 501, "2026-11-15T10:16:00Z", 1, 2.5, Some(54.0), "DEEP")
            .unwrap();
        let next_sleep = db
            .load_sleep_summaries()
            .unwrap()
            .into_iter()
            .find(|s| s.astronaut_id == "AST-01" && s.mission_day == 1)
            .unwrap()
            .total_sleep_min;
        assert!((next_sleep - (prev_sleep + 1.0)).abs() < 1e-6);
    }
}
