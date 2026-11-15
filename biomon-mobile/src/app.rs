use crate::calc::{self, PanelAlertState, SafetyEvalInput};
use crate::db::{
    AstronautConfig, CrewInput, DailyNutritionTotals, DailyPhysioTotals, Database, FoodCacheItem,
    LocalAlert, NewCrewInput, NewTelemetryFrame, OfflineProtocol, OsteoporosisDailyState,
    SansDailyState, SleepEpoch, UnsyncedCounts,
};
use crate::net::{BiomonLanClient, ConnectionState, OutboundFrame, SyncBatch};
use crate::sim::BioSimulator;
use crate::ui;
use crate::ui_components;
use eframe::egui;
use serde_json::json;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    ViewMode,
    InputMode,
}

impl AppMode {
    pub fn toggle(&mut self) {
        *self = match self {
            AppMode::ViewMode => AppMode::InputMode,
            AppMode::InputMode => AppMode::ViewMode,
        };
    }

    pub fn cycle_badge_label(self) -> &'static str {
        match self {
            AppMode::ViewMode => "VIEW",
            AppMode::InputMode => "INPUT",
        }
    }

    pub fn detail_mode_badge(self, is_alert: bool) -> &'static str {
        if is_alert {
            "CRITICAL OVERRIDE ACTIVE"
        } else {
            match self {
                AppMode::ViewMode => "VIEW MODE — LIVE TELEMETRY",
                AppMode::InputMode => "INPUT MODE — GLOVE CONTROLS",
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadrantId {
    HealthAndSuit,
    HubAndNav,
    CommsAndDisplay,
    HazardAndEnv,
}

impl QuadrantId {
    pub fn title(self) -> &'static str {
        match self {
            QuadrantId::HealthAndSuit => "HEALTH & SUIT",
            QuadrantId::HubAndNav => "HUB & NAV",
            QuadrantId::CommsAndDisplay => "COMMS",
            QuadrantId::HazardAndEnv => "HAZARD & ENV",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainHud,
    Detail(QuadrantId),
}

/// Non-blocking snapshot read by the 60 FPS `eframe` render loop without touching SQLite per frame.
#[derive(Debug, Clone)]
pub struct UiSnapshot {
    pub astronaut_id: String,
    pub callsign: String,
    pub weight_kg: f64,
    pub height_cm: f64,
    pub age_yr: i64,
    pub sex: String,
    pub sleep_target_min: i64,
    pub active_phase: String,
    pub mission_day: i64,
    pub seq_id: i64,
    pub recorded_at_utc: String,
    pub hr_bpm: f64,
    pub resting_hr_bpm: f64,
    pub sbp_mmhg: f64,
    pub resp_brpm: f64,
    pub spo2_pct: f64,
    pub tissue_o2_pct: Option<f64>,
    pub blood_n2_idx: Option<f64>,
    pub g_load: f64,
    pub imu_activity_idx: f64,
    pub skin_temp_c: f64,
    pub suit_pressure_kpa: Option<f64>,
    pub suit_humidity_pct: Option<f64>,
    pub eva_elapsed_sec: i64,
    pub cabin_co2_ppm: f64,
    pub cabin_temp_c: Option<f64>,
    pub cabin_humidity_pct: Option<f64>,
    pub cabin_pressure_kpa: Option<f64>,
    pub cabin_o2_pct: Option<f64>,
    pub cabin_motion_idx: f64,
    pub radiation_usv_hr: f64,
    pub kcal_per_sec: f64,
    pub sweat_ml_per_sec: f64,
    pub shock_index: f64,
    pub orthostatic_risk: &'static str,
    pub hub_distance_m: u32,
    pub heading_deg: f32,
    pub target_bearing_deg: f32,
    pub earth_mars_au: f64,
    pub one_way_light_min: f64,
    pub comms_state: ConnectionState,
    pub alerts: PanelAlertState,
    pub food_cache: Vec<FoodCacheItem>,
    pub protocols: Vec<OfflineProtocol>,
    pub sans_state: Option<SansDailyState>,
    pub osteo_state: Option<OsteoporosisDailyState>,
    pub daily_nutrition: DailyNutritionTotals,
    pub daily_physio: DailyPhysioTotals,
    pub recent_inputs: Vec<CrewInput>,
    pub local_alerts: Vec<LocalAlert>,
    pub sleep_epochs: Vec<SleepEpoch>,
    pub unsynced: UnsyncedCounts,
}

impl UiSnapshot {
    pub fn cardinal_heading(&self) -> &'static str {
        let deg = self.heading_deg.rem_euclid(360.0);
        match ((deg + 22.5) / 45.0).floor() as i32 % 8 {
            0 => "N",
            1 => "NE",
            2 => "E",
            3 => "SE",
            4 => "S",
            5 => "SW",
            6 => "W",
            _ => "NW",
        }
    }

    pub fn recompute_derived(&mut self) {
        self.kcal_per_sec =
            calc::calc_kcal_per_sec(self.hr_bpm, self.weight_kg, self.age_yr as i32, &self.sex);
        self.sweat_ml_per_sec =
            calc::calc_sweat_ml_per_sec(self.kcal_per_sec * 60.0, self.skin_temp_c);
        self.shock_index = calc::calc_shock_index(self.hr_bpm, self.sbp_mmhg);
        self.orthostatic_risk =
            calc::calc_orthostatic_risk(self.hr_bpm, self.sbp_mmhg, self.g_load);
        self.one_way_light_min = calc::calc_one_way_light_min(self.earth_mars_au);

        self.alerts = calc::evaluate_safety(&SafetyEvalInput {
            phase: &self.active_phase,
            hr_bpm: self.hr_bpm,
            resting_hr_bpm: self.resting_hr_bpm,
            sbp_mmhg: self.sbp_mmhg,
            spo2_pct: self.spo2_pct,
            tissue_o2_pct: self.tissue_o2_pct,
            blood_n2_idx: self.blood_n2_idx,
            radiation_usv_hr: self.radiation_usv_hr,
            cabin_co2_ppm: self.cabin_co2_ppm,
            cabin_o2_pct: self.cabin_o2_pct,
            suit_pressure_kpa: self.suit_pressure_kpa,
            hub_distance_m: self.hub_distance_m as f64,
            comms_online: self.comms_state == ConnectionState::Online,
        });
    }
}

pub type BiomonApp = AppState;

pub struct AppState {
    pub mode: AppMode,
    pub screen: Screen,
    pub selected_food_idx: usize,
    pub selected_meal_pct: u8,
    pub selected_protocol_idx: usize,
    pub status_banner: String,
    pub snapshot: UiSnapshot,
    pub db: Database,
    pub lan_client: BiomonLanClient,
    pub sim: BioSimulator,
    last_sim_tick: Instant,
    last_sync_attempt: Instant,
    ticks_since_state_recompute: u32,
    sync_inflight: bool,
    ephemeris_start_au: f64,
    ephemeris_end_au: f64,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui_components::install_aerospace_fonts(&cc.egui_ctx);
        ui::apply_mandated_visuals(&cc.egui_ctx);

        let mut db =
            Database::open_default().unwrap_or_else(|_| Database::open_in_memory_fallback());
        let _ = db.seed_demo_data();
        let lan_client = BiomonLanClient::new("127.0.0.1:7878");
        Self::from_db_and_client(db, lan_client)
    }

    pub fn from_db_and_client(db: Database, lan_client: BiomonLanClient) -> Self {
        let comms_state = lan_client.connection_state();

        let config = db
            .load_astronaut_config("AST-01")
            .ok()
            .flatten()
            .unwrap_or(AstronautConfig {
                astronaut_id: "AST-01".to_string(),
                callsign: "ARES-1".to_string(),
                weight: 78.0,
                height: 180.0,
                age: 36,
                sex: "M".to_string(),
                active_phase: "ORBIT".to_string(),
                mission_start_utc: "2026-11-14T00:00:00Z".to_string(),
                sleep_target_min: 480,
            });

        let latest = db
            .load_latest_telemetry(&config.astronaut_id)
            .ok()
            .flatten();
        let food_cache = db.load_food_cache().unwrap_or_default();
        let protocols = db.load_offline_protocols().unwrap_or_default();

        let mission_day = latest.as_ref().map(|t| t.mission_day).unwrap_or(1);
        let seq_id = latest.as_ref().map(|t| t.seq_id).unwrap_or(1);
        let recorded_at_utc = latest
            .as_ref()
            .map(|t| t.recorded_at_utc.clone())
            .unwrap_or_else(calc::utc_now_iso8601);
        let hr_bpm = latest.as_ref().map(|t| t.hr_bpm).unwrap_or(88.0);
        let sbp_mmhg = latest.as_ref().map(|t| t.sbp_mmhg).unwrap_or(118.0);
        let resp_brpm = latest.as_ref().map(|t| t.resp_brpm).unwrap_or(14.0);
        let spo2_pct = latest.as_ref().map(|t| t.spo2_pct).unwrap_or(98.0);
        let tissue_o2_pct = latest.as_ref().and_then(|t| t.tissue_o2_pct).or(Some(93.5));
        let blood_n2_idx = latest.as_ref().and_then(|t| t.blood_n2_idx).or(Some(1.08));
        let g_load = latest.as_ref().map(|t| t.g_load).unwrap_or(1.0);
        let imu_activity_idx = latest.as_ref().map(|t| t.imu_activity_idx).unwrap_or(0.35);
        let skin_temp_c = latest.as_ref().map(|t| t.skin_temp_c).unwrap_or(36.4);
        let suit_pressure_kpa = latest
            .as_ref()
            .and_then(|t| t.suit_pressure_kpa)
            .or(Some(29.6));
        let suit_humidity_pct = latest
            .as_ref()
            .and_then(|t| t.suit_humidity_pct)
            .or(Some(38.0));
        let eva_elapsed_sec = latest.as_ref().map(|t| t.eva_elapsed_sec).unwrap_or(1420);
        let cabin_co2_ppm = latest.as_ref().map(|t| t.cabin_co2_ppm).unwrap_or(1150.0);
        let cabin_temp_c = latest.as_ref().and_then(|t| t.cabin_temp_c).or(Some(21.5));
        let cabin_humidity_pct = latest
            .as_ref()
            .and_then(|t| t.cabin_humidity_pct)
            .or(Some(44.0));
        let cabin_pressure_kpa = latest
            .as_ref()
            .and_then(|t| t.cabin_pressure_kpa)
            .or(Some(101.3));
        let cabin_o2_pct = latest.as_ref().and_then(|t| t.cabin_o2_pct).or(Some(20.9));
        let cabin_motion_idx = latest.as_ref().map(|t| t.cabin_motion_idx).unwrap_or(0.40);
        let radiation_usv_hr = latest.as_ref().map(|t| t.radiation_usv_hr).unwrap_or(0.42);

        let sans_state = db
            .load_sans_state(&config.astronaut_id, mission_day)
            .ok()
            .flatten();
        let osteo_state = db
            .load_osteo_state(&config.astronaut_id, mission_day)
            .ok()
            .flatten();
        let daily_nutrition = db
            .load_daily_nutrition(&config.astronaut_id, mission_day)
            .unwrap_or_default();
        let daily_physio = db
            .load_daily_physio_totals(&config.astronaut_id, mission_day)
            .unwrap_or_default();
        let recent_inputs = db
            .load_recent_crew_inputs(&config.astronaut_id, 6)
            .unwrap_or_default();
        let local_alerts = db.load_local_alerts().unwrap_or_default();
        let sleep_epochs = db
            .load_sleep_epochs(&config.astronaut_id, mission_day)
            .unwrap_or_default();
        let unsynced = db.count_unsynced().unwrap_or_default();

        // NASA JPL Horizons 2026-11-14 -> 2026-11-15 Earth-Mars ephemeris (AU)
        let ephemeris_start_au = 1.324362;
        let ephemeris_end_au = 1.315854;
        let (earth_mars_au, one_way_light_min) = calc::interpolate_ephemeris(
            ephemeris_start_au,
            ephemeris_end_au,
            calc::utc_day_fraction(),
        );

        let sim = BioSimulator::new(blood_n2_idx, suit_pressure_kpa);

        let mut snapshot = UiSnapshot {
            astronaut_id: config.astronaut_id,
            callsign: config.callsign,
            weight_kg: config.weight,
            height_cm: config.height,
            age_yr: config.age,
            sex: config.sex,
            sleep_target_min: config.sleep_target_min,
            active_phase: config.active_phase,
            mission_day,
            seq_id,
            recorded_at_utc,
            hr_bpm,
            resting_hr_bpm: 68.0,
            sbp_mmhg,
            resp_brpm,
            spo2_pct,
            tissue_o2_pct,
            blood_n2_idx,
            g_load,
            imu_activity_idx,
            skin_temp_c,
            suit_pressure_kpa,
            suit_humidity_pct,
            eva_elapsed_sec,
            cabin_co2_ppm,
            cabin_temp_c,
            cabin_humidity_pct,
            cabin_pressure_kpa,
            cabin_o2_pct,
            cabin_motion_idx,
            radiation_usv_hr,
            kcal_per_sec: 0.0,
            sweat_ml_per_sec: 0.0,
            shock_index: 0.0,
            orthostatic_risk: "LOW",
            hub_distance_m: 222,
            heading_deg: 42.0,
            target_bearing_deg: 0.0,
            earth_mars_au,
            one_way_light_min,
            comms_state,
            alerts: PanelAlertState::default(),
            food_cache,
            protocols,
            sans_state,
            osteo_state,
            daily_nutrition,
            daily_physio,
            recent_inputs,
            local_alerts,
            sleep_epochs,
            unsynced,
        };
        snapshot.recompute_derived();

        let db_label = db
            .path()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("SQLITE");

        let now = Instant::now();
        Self {
            mode: AppMode::ViewMode,
            screen: Screen::MainHud,
            selected_food_idx: 0,
            selected_meal_pct: 100,
            selected_protocol_idx: 0,
            status_banner: format!("{} WAL READY", db_label.to_ascii_uppercase()),
            snapshot,
            db,
            lan_client,
            sim,
            last_sim_tick: now,
            last_sync_attempt: now,
            ticks_since_state_recompute: 0,
            sync_inflight: false,
            ephemeris_start_au,
            ephemeris_end_au,
        }
    }

    pub fn refresh_db_state(&mut self) {
        let id = &self.snapshot.astronaut_id;
        let day = self.snapshot.mission_day;
        self.snapshot.sans_state = self.db.load_sans_state(id, day).ok().flatten();
        self.snapshot.osteo_state = self.db.load_osteo_state(id, day).ok().flatten();
        self.snapshot.daily_nutrition = self.db.load_daily_nutrition(id, day).unwrap_or_default();
        self.snapshot.daily_physio = self.db.load_daily_physio_totals(id, day).unwrap_or_default();
        self.snapshot.recent_inputs = self.db.load_recent_crew_inputs(id, 6).unwrap_or_default();
        self.snapshot.local_alerts = self.db.load_local_alerts().unwrap_or_default();
        self.snapshot.sleep_epochs = self.db.load_sleep_epochs(id, day).unwrap_or_default();
        self.snapshot.unsynced = self.db.count_unsynced().unwrap_or_default();
    }

    fn persist_snapshot_telemetry_frame(&mut self, kcal_delta: f64, sweat_delta_ml: f64) {
        let ts = if self.snapshot.recorded_at_utc.is_empty() {
            calc::utc_now_iso8601()
        } else {
            self.snapshot.recorded_at_utc.clone()
        };
        let frame_in = NewTelemetryFrame {
            astronaut_id: &self.snapshot.astronaut_id,
            recorded_at_utc: &ts,
            mission_day: self.snapshot.mission_day,
            mission_phase: &self.snapshot.active_phase,
            hr_bpm: self.snapshot.hr_bpm,
            sbp_mmhg: self.snapshot.sbp_mmhg,
            resp_brpm: self.snapshot.resp_brpm,
            spo2_pct: self.snapshot.spo2_pct,
            tissue_o2_pct: self.snapshot.tissue_o2_pct,
            blood_n2_idx: self.snapshot.blood_n2_idx,
            g_load: self.snapshot.g_load,
            imu_activity_idx: self.snapshot.imu_activity_idx,
            skin_temp_c: self.snapshot.skin_temp_c,
            radiation_usv_hr: self.snapshot.radiation_usv_hr,
            cabin_co2_ppm: self.snapshot.cabin_co2_ppm,
            cabin_temp_c: self.snapshot.cabin_temp_c,
            cabin_humidity_pct: self.snapshot.cabin_humidity_pct,
            cabin_pressure_kpa: self.snapshot.cabin_pressure_kpa,
            cabin_o2_pct: self.snapshot.cabin_o2_pct,
            cabin_motion_idx: self.snapshot.cabin_motion_idx,
            suit_pressure_kpa: self.snapshot.suit_pressure_kpa,
            suit_humidity_pct: self.snapshot.suit_humidity_pct,
            eva_elapsed_sec: self.snapshot.eva_elapsed_sec,
            kcal_burn_delta: kcal_delta,
            sweat_loss_delta_ml: sweat_delta_ml,
        };
        if let Ok(saved) = self.db.insert_telemetry_frame(&frame_in) {
            self.snapshot.seq_id = saved.seq_id;
            self.snapshot.recorded_at_utc = saved.recorded_at_utc;
        }
        self.evaluate_and_record_local_alerts();
    }

    fn evaluate_and_record_local_alerts(&self) {
        let seq = self.snapshot.seq_id;
        if self.snapshot.alerts.health_danger {
            if self.snapshot.spo2_pct < 92.0
                || self.snapshot.tissue_o2_pct.is_some_and(|t| t < 85.0)
            {
                let _ = self
                    .db
                    .trigger_local_alert_if_new(seq, "PROT-HYPOXIA-01", "CRITICAL");
            } else if self.snapshot.blood_n2_idx.is_some_and(|r| r > 1.50)
                || (self.snapshot.active_phase.eq_ignore_ascii_case("EVA")
                    && self.snapshot.suit_pressure_kpa.is_some_and(|p| p < 25.0))
            {
                let _ = self
                    .db
                    .trigger_local_alert_if_new(seq, "PROT-DECOMP-01", "CRITICAL");
            } else {
                let _ = self
                    .db
                    .trigger_local_alert_if_new(seq, "PROT-CARDIO-01", "HIGH");
            }
        }
        if self.snapshot.alerts.hazard_danger {
            if self.snapshot.radiation_usv_hr > 50.0 {
                let _ = self
                    .db
                    .trigger_local_alert_if_new(seq, "PROT-RAD-01", "CRITICAL");
            } else if self.snapshot.cabin_co2_ppm > 3000.0
                || self.snapshot.cabin_o2_pct.is_some_and(|o| o < 19.5)
            {
                let _ = self
                    .db
                    .trigger_local_alert_if_new(seq, "PROT-HYPOXIA-01", "HIGH");
            }
        }
        let hydration_deficit = (self.snapshot.daily_physio.sweat_loss_ml_total
            - self.snapshot.daily_nutrition.fluid_ml_total)
            .max(0.0);
        if hydration_deficit > 1500.0 {
            let _ = self
                .db
                .trigger_local_alert_if_new(seq, "PROT-HYDR-01", "WARN");
        }
    }

    pub fn step_simulation_1hz(&mut self) {
        let hydration_deficit = (self.snapshot.daily_physio.sweat_loss_ml_total
            - self.snapshot.daily_nutrition.fluid_ml_total)
            .max(0.0);
        let tick_out = self.sim.tick_1hz(&mut self.snapshot, hydration_deficit);
        self.persist_snapshot_telemetry_frame(
            self.snapshot.kcal_per_sec,
            self.snapshot.sweat_ml_per_sec,
        );

        if let Some(ep) = tick_out.sleep_epoch {
            let _ = self.db.insert_sleep_epoch(
                &self.snapshot.astronaut_id,
                self.snapshot.mission_day,
                ep.activity_count,
                ep.hr_bpm_avg,
                &ep.sleep_state,
            );
            self.snapshot.sleep_epochs = self
                .db
                .load_sleep_epochs(&self.snapshot.astronaut_id, self.snapshot.mission_day)
                .unwrap_or_default();
        }

        self.ticks_since_state_recompute += 1;
        if self.ticks_since_state_recompute >= 10 {
            self.ticks_since_state_recompute = 0;
            let _ = self.db.recompute_daily_derived_states(
                &self.snapshot.astronaut_id,
                self.snapshot.mission_day,
                &self.snapshot.active_phase,
                self.snapshot.cabin_co2_ppm,
                &calc::utc_now_iso8601(),
            );
            self.refresh_db_state();
        } else {
            self.snapshot.daily_physio = self
                .db
                .load_daily_physio_totals(&self.snapshot.astronaut_id, self.snapshot.mission_day)
                .unwrap_or_default();
            self.snapshot.local_alerts = self.db.load_local_alerts().unwrap_or_default();
            self.snapshot.unsynced = self.db.count_unsynced().unwrap_or_default();
        }

        let (dist_au, light_min) = calc::interpolate_ephemeris(
            self.ephemeris_start_au,
            self.ephemeris_end_au,
            calc::utc_day_fraction(),
        );
        self.snapshot.earth_mars_au = dist_au;
        self.snapshot.one_way_light_min = light_min;
    }

    pub fn log_selected_meal(&mut self) {
        if let Some(food) = self
            .snapshot
            .food_cache
            .get(self.selected_food_idx)
            .cloned()
        {
            let pct = self.selected_meal_pct as i64;
            let _ = self.db.insert_crew_input(&NewCrewInput {
                astronaut_id: &self.snapshot.astronaut_id,
                mission_day: self.snapshot.mission_day,
                input_type: "MEAL_LOG",
                reference_code: Some(&food.food_id),
                consumption_pct: Some(pct),
                notes: Some(&food.item_name),
                phase: &self.snapshot.active_phase,
                avg_co2_ppm: self.snapshot.cabin_co2_ppm,
            });
            self.refresh_db_state();
            if self.snapshot.comms_state == ConnectionState::Online {
                self.dispatch_unsynced_batches(false);
                self.status_banner = format!("LOGGED {} AT {}%", food.food_id, pct);
            } else {
                self.status_banner =
                    format!("LOGGED {} AT {}% (WAL BUFFERED)", food.food_id, pct);
            }
        }
    }

    pub fn log_hydration_pouch(&mut self) {
        let _ = self.db.insert_crew_input(&NewCrewInput {
            astronaut_id: &self.snapshot.astronaut_id,
            mission_day: self.snapshot.mission_day,
            input_type: "FLUID_LOG",
            reference_code: Some("POUCH-250ML"),
            consumption_pct: Some(100),
            notes: Some("250 ML ELECTROLYTE POUCH"),
            phase: &self.snapshot.active_phase,
            avg_co2_ppm: self.snapshot.cabin_co2_ppm,
        });
        self.refresh_db_state();
        if self.snapshot.comms_state == ConnectionState::Online {
            self.dispatch_unsynced_batches(false);
            self.status_banner = "LOGGED +250 ML HYDRATION POUCH".to_string();
        } else {
            self.status_banner = "LOGGED +250 ML POUCH (WAL BUFFERED)".to_string();
        }
    }

    pub fn log_exercise_session(&mut self, code: &str, label: &str) {
        let _ = self.db.insert_crew_input(&NewCrewInput {
            astronaut_id: &self.snapshot.astronaut_id,
            mission_day: self.snapshot.mission_day,
            input_type: "EXERCISE",
            reference_code: Some(code),
            consumption_pct: Some(100),
            notes: Some(label),
            phase: &self.snapshot.active_phase,
            avg_co2_ppm: self.snapshot.cabin_co2_ppm,
        });
        self.refresh_db_state();
        if self.snapshot.comms_state == ConnectionState::Online {
            self.dispatch_unsynced_batches(false);
        }
        self.status_banner = format!("LOGGED EXERCISE: {}", label);
    }

    pub fn set_mission_phase(&mut self, phase: &str) {
        self.snapshot.active_phase = phase.to_string();
        let _ = self
            .db
            .update_active_phase(&self.snapshot.astronaut_id, phase);
        self.snapshot.recorded_at_utc = calc::utc_now_iso8601();
        self.snapshot.recompute_derived();
        self.persist_snapshot_telemetry_frame(0.0, 0.0);
        let _ = self.db.recompute_daily_derived_states(
            &self.snapshot.astronaut_id,
            self.snapshot.mission_day,
            &self.snapshot.active_phase,
            self.snapshot.cabin_co2_ppm,
            &calc::utc_now_iso8601(),
        );
        self.refresh_db_state();
        self.status_banner = format!("ACTIVE PHASE SET TO {}", phase);
    }

    pub fn step_prebreathe_washout(&mut self, dt_sec: f64) {
        let suit_p = self.snapshot.suit_pressure_kpa.unwrap_or(29.6);
        let cur_r = self.snapshot.blood_n2_idx.unwrap_or(1.08);
        let cur_p_n2 = cur_r * suit_p;
        let (_, new_r) = calc::update_tissue_n2(cur_p_n2, 0.0, suit_p, dt_sec);
        self.snapshot.blood_n2_idx = Some(new_r);
        self.sim
            .sync_tissue_n2_from_snapshot(Some(new_r), Some(suit_p));
        self.snapshot.eva_elapsed_sec += dt_sec as i64;
        self.snapshot.recorded_at_utc = calc::utc_now_iso8601();
        self.snapshot.recompute_derived();
        self.persist_snapshot_telemetry_frame(0.0, 0.0);
        self.refresh_db_state();
        self.status_banner = format!("O2 PREBREATHE +10M — N2 R={:.2}", new_r);
    }

    pub fn cycle_sim_scenario(&mut self) {
        self.sim.scenario = self.sim.scenario.next();
        self.status_banner = format!("SIM SCENARIO: {}", self.sim.scenario.label());
    }

    pub fn toggle_sim_enabled(&mut self) {
        self.sim.enabled = !self.sim.enabled;
        self.status_banner = if self.sim.enabled {
            format!("1HZ SIM RESUMED ({})", self.sim.scenario.label())
        } else {
            "1HZ SIM PAUSED".to_string()
        };
    }

    pub fn drain_network_acks(&mut self) {
        let mut any_marked = false;
        let mut any_failed = false;
        while let Some(ack) = self.lan_client.try_recv_ack() {
            if ack.delivered {
                if let Some(batch) = ack.sync_batch {
                    if !batch.telemetry_seq_ids.is_empty() {
                        let _ = self.db.mark_telemetry_synced(&batch.telemetry_seq_ids);
                        any_marked = true;
                    }
                    if !batch.crew_input_ids.is_empty() {
                        let _ = self.db.mark_crew_inputs_synced(&batch.crew_input_ids);
                        any_marked = true;
                    }
                    if !batch.sleep_epoch_ids.is_empty() {
                        let _ = self.db.mark_sleep_epochs_synced(&batch.sleep_epoch_ids);
                        any_marked = true;
                    }
                    if let Some((astro_id, day)) = batch.daily_state_key {
                        let _ = self.db.mark_daily_states_synced(&astro_id, day);
                        any_marked = true;
                    }
                }
                self.sync_inflight = false;
            } else {
                self.sync_inflight = false;
                any_failed = true;
            }
        }
        if any_marked {
            self.refresh_db_state();
            if self.snapshot.unsynced.total() == 0 {
                self.status_banner = format!(
                    "HUB SYNCED ({} PACKETS TOTAL)",
                    self.lan_client.frames_sent()
                );
            }
        } else if any_failed {
            self.snapshot.comms_state = self.lan_client.connection_state();
        }
    }

    pub fn dispatch_unsynced_batches(&mut self, manual_request: bool) {
        if self.lan_client.is_manual_blackout() {
            if manual_request {
                self.status_banner = format!(
                    "BLACKOUT ACTIVE — {} RECORDS BUFFERED IN WAL",
                    self.snapshot.unsynced.total()
                );
            }
            return;
        }
        if !manual_request
            && (self.sync_inflight || self.lan_client.connection_state() != ConnectionState::Online)
        {
            return;
        }

        let crew_id = self.snapshot.astronaut_id.clone();
        let day = self.snapshot.mission_day;
        let mut dispatched = false;

        let inputs = self.db.load_unsynced_crew_inputs(20).unwrap_or_default();
        if !inputs.is_empty() {
            let ids: Vec<i64> = inputs.iter().map(|i| i.input_id).collect();
            self.lan_client.queue_frame(OutboundFrame {
                crew_id: crew_id.clone(),
                packet_type: "CREW_INPUT".to_string(),
                payload: json!({ "items": inputs }),
                sync_batch: Some(SyncBatch {
                    crew_input_ids: ids,
                    ..Default::default()
                }),
            });
            dispatched = true;
        }

        let epochs = self.db.load_unsynced_sleep_epochs(20).unwrap_or_default();
        if !epochs.is_empty() {
            let ids: Vec<i64> = epochs.iter().map(|e| e.epoch_id).collect();
            self.lan_client.queue_frame(OutboundFrame {
                crew_id: crew_id.clone(),
                packet_type: "SLEEP_EPOCH".to_string(),
                payload: json!({ "epochs": epochs }),
                sync_batch: Some(SyncBatch {
                    sleep_epoch_ids: ids,
                    ..Default::default()
                }),
            });
            dispatched = true;
        }

        let sans = self.db.load_unsynced_sans_states().unwrap_or_default();
        let osteo = self.db.load_unsynced_osteo_states().unwrap_or_default();
        if !sans.is_empty() || !osteo.is_empty() {
            self.lan_client.queue_frame(OutboundFrame {
                crew_id: crew_id.clone(),
                packet_type: "DAILY_STATES".to_string(),
                payload: json!({
                    "mission_day": day,
                    "sans": sans,
                    "osteo": osteo,
                }),
                sync_batch: Some(SyncBatch {
                    daily_state_key: Some((crew_id.clone(), day)),
                    ..Default::default()
                }),
            });
            dispatched = true;
        }

        let telemetry = self.db.load_unsynced_telemetry(60).unwrap_or_default();
        if !telemetry.is_empty() {
            let seq_ids: Vec<i64> = telemetry.iter().map(|t| t.seq_id).collect();
            self.lan_client.queue_frame(OutboundFrame {
                crew_id: crew_id.clone(),
                packet_type: "TELEMETRY_SYNC".to_string(),
                payload: json!({ "frames": telemetry }),
                sync_batch: Some(SyncBatch {
                    telemetry_seq_ids: seq_ids,
                    ..Default::default()
                }),
            });
            dispatched = true;
        } else if manual_request {
            self.lan_client.queue_frame(OutboundFrame {
                crew_id: crew_id.clone(),
                packet_type: "TELEMETRY_BATCH_SYNC".to_string(),
                payload: json!({
                    "astronaut_id": crew_id,
                    "callsign": self.snapshot.callsign,
                    "mission_day": day,
                    "active_phase": self.snapshot.active_phase,
                    "hr_bpm": self.snapshot.hr_bpm,
                    "sbp_mmhg": self.snapshot.sbp_mmhg,
                    "spo2_pct": self.snapshot.spo2_pct,
                    "radiation_usv_hr": self.snapshot.radiation_usv_hr,
                    "cabin_co2_ppm": self.snapshot.cabin_co2_ppm,
                    "flushed_records": 0,
                }),
                sync_batch: None,
            });
            dispatched = true;
        }

        if dispatched {
            self.sync_inflight = true;
            self.last_sync_attempt = Instant::now();
            if manual_request {
                self.status_banner = if self.snapshot.comms_state == ConnectionState::Online {
                    "DISPATCHING WAL PACKETS TO HUB...".to_string()
                } else {
                    format!(
                        "HUB PROBING ({} RECORDS IN WAL)...",
                        self.snapshot.unsynced.total()
                    )
                };
            }
        }
    }

    pub fn sync_queued_packets(&mut self) {
        self.lan_client.trigger_probe();
        self.sync_inflight = false;
        self.dispatch_unsynced_batches(true);
    }

    pub fn acknowledge_alerts(&mut self) {
        let _ = self.db.acknowledge_all_alerts();
        self.refresh_db_state();
        self.status_banner = "LOCAL ALERTS ACKNOWLEDGED".to_string();
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_network_acks();
        let live_comms = self.lan_client.connection_state();
        if self.snapshot.comms_state != live_comms {
            self.snapshot.comms_state = live_comms;
            self.snapshot.recompute_derived();
        }

        let now = Instant::now();
        if self.sim.enabled {
            let mut catchup = 0;
            while now.duration_since(self.last_sim_tick) >= Duration::from_secs(1) && catchup < 3
            {
                self.last_sim_tick += Duration::from_secs(1);
                self.step_simulation_1hz();
                catchup += 1;
            }
            if now.duration_since(self.last_sim_tick) >= Duration::from_secs(1) {
                self.last_sim_tick = now;
            }
        } else {
            self.last_sim_tick = now;
        }

        if !self.sync_inflight
            && self.snapshot.comms_state == ConnectionState::Online
            && self.snapshot.unsynced.total() > 0
            && now.duration_since(self.last_sync_attempt) >= Duration::from_secs(2)
        {
            self.dispatch_unsynced_batches(false);
        } else if self.sync_inflight
            && now.duration_since(self.last_sync_attempt) >= Duration::from_secs(5)
        {
            self.sync_inflight = false;
        }

        ui.ctx().request_repaint_after(Duration::from_millis(200));
        ui::render(ui, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::SimScenario;

    #[test]
    fn test_app_state_sim_meal_exercise_and_offline_buffering() {
        let mut db = Database::open_in_memory_fallback();
        db.seed_demo_data().unwrap();
        let lan_client = BiomonLanClient::new("127.0.0.1:0");
        let mut app = AppState::from_db_and_client(db, lan_client);

        let initial_seq = app.snapshot.seq_id;
        let initial_kcal = app.snapshot.daily_nutrition.kcal_total;

        // 1. Run 10 1Hz sim ticks in Exercise scenario -> inserts frames, accumulates burn/sweat
        app.sim.scenario = SimScenario::Exercise;
        for _ in 0..10 {
            app.step_simulation_1hz();
        }
        assert_eq!(app.snapshot.seq_id, initial_seq + 10);
        assert!(app.snapshot.daily_physio.kcal_burned_total > 0.5);
        assert!(app.snapshot.daily_physio.sweat_loss_ml_total > 1.0);
        assert!(app.snapshot.unsynced.telemetry_frames >= 10);

        // 2. Log meal & exercise -> updates SQLite nutrition & osteo load, stays unsynced offline
        app.selected_food_idx = 0;
        app.selected_meal_pct = 100;
        app.log_selected_meal();
        assert!(app.snapshot.daily_nutrition.kcal_total > initial_kcal);

        app.log_exercise_session("ARED-15M", "15M ARED RESISTANCE");
        assert!(app.snapshot.osteo_state.as_ref().unwrap().exercise_load_idx > 75.0);

        // 3. Attempting sync with unreachable port keeps records safely buffered in WAL
        app.sync_queued_packets();
        std::thread::sleep(Duration::from_millis(120));
        app.drain_network_acks();
        assert!(app.snapshot.unsynced.total() > 0);
    }
}
