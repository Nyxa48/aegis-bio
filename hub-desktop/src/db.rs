use crate::calc;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
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

        // 3. Seed sample mission_ephemeris rows on the Hub
        let ephemeris = [
            ("2026-11-14", 1.42_f64, 11.81_f64),
            ("2026-11-15", 1.43_f64, calc::calc_one_way_light_min(1.43)),
            ("2026-11-16", 1.45_f64, calc::calc_one_way_light_min(1.45)),
            ("2026-11-17", 1.47_f64, calc::calc_one_way_light_min(1.47)),
            ("2026-11-18", 1.49_f64, calc::calc_one_way_light_min(1.49)),
        ];

        for (date, dist_au, light_min) in ephemeris {
            self.conn.execute(
                "INSERT OR IGNORE INTO mission_ephemeris (mission_date, distance_au, one_way_light_min)
                 VALUES (?1, ?2, ?3)",
                params![date, dist_au, light_min],
            )?;
        }

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
    }
}
