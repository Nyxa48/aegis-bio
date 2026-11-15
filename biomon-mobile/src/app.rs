use crate::calc::{self, PanelAlertState, SafetyEvalInput};
use crate::db::{
    AstronautConfig, CrewInput, DailyNutritionTotals, Database, FoodCacheItem, LocalAlert,
    NewCrewInput, OfflineProtocol, OsteoporosisDailyState, SansDailyState, SleepEpoch,
    UnsyncedCounts,
};
use crate::net::{BiomonLanClient, ConnectionState, HubDownlinkMessage, LinkEvent, OutboundFrame};
use crate::ui;
use crate::ui_components;
use eframe::egui;
use serde_json::json;
use std::path::PathBuf;

const CREW_PRESETS: [(&str, &str, f64, f64, i64, &str); 6] = [
    ("AST-01", "ARES-1", 78.0, 180.0, 36, "M"),
    ("AST-02", "ARES-2", 74.0, 176.0, 34, "M"),
    ("AST-03", "ARES-3", 64.0, 168.0, 33, "F"),
    ("AST-04", "ARES-4", 81.0, 183.0, 39, "M"),
    ("AST-05", "ARES-5", 69.0, 172.0, 31, "F"),
    ("AST-06", "ARES-6", 76.0, 178.0, 37, "M"),
];

fn requested_crew_id_from_env_or_args() -> &'static str {
    let env_val = std::env::var("AEGIS_CREW_ID").ok();
    let mut args = std::env::args();
    let mut arg_val: Option<String> = None;
    while let Some(arg) = args.next() {
        if arg == "--crew"
            && let Some(val) = args.next()
        {
            arg_val = Some(val);
            break;
        } else if let Some(val) = arg.strip_prefix("--crew=") {
            arg_val = Some(val.to_string());
            break;
        }
    }
    let raw = arg_val.or(env_val).unwrap_or_default().to_ascii_uppercase();
    match raw.as_str() {
        "AST-02" => "AST-02",
        "AST-03" => "AST-03",
        "AST-04" => "AST-04",
        "AST-05" => "AST-05",
        "AST-06" => "AST-06",
        _ => "AST-01",
    }
}

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
            AppMode::ViewMode => "GÖRÜNTÜLE",
            AppMode::InputMode => "VERİ GİRİŞİ",
        }
    }

    pub fn detail_mode_badge(self, is_alert: bool) -> &'static str {
        if is_alert {
            "KRİTİK UYARI ETKİN"
        } else {
            match self {
                AppMode::ViewMode => "İZLEME MODU — TELEMETRİ",
                AppMode::InputMode => "GİRİŞ MODU — ELDİVEN KONTROLÜ",
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
            QuadrantId::HealthAndSuit => "BİYOMETRİ",
            QuadrantId::HubAndNav => "KONUM",
            QuadrantId::CommsAndDisplay => "BAĞLANTI",
            QuadrantId::HazardAndEnv => "KABİN ORTAMI",
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
    pub hub_endpoint: String,
    pub pending_sync_batch: Option<u64>,
    pub last_live_push_time: f64,
    pub snapshot: UiSnapshot,
    pub db: Database,
    pub lan_client: BiomonLanClient,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_db_dir(cc, None)
    }

    pub fn new_with_db_dir(cc: &eframe::CreationContext<'_>, db_dir: Option<PathBuf>) -> Self {
        ui_components::install_aerospace_fonts(&cc.egui_ctx);
        ui::apply_mandated_visuals(&cc.egui_ctx);

        let mut db = match db_dir {
            Some(dir) => Database::open_in_dir(dir),
            None => Database::open_default(),
        }
        .unwrap_or_else(|_| Database::open_in_memory_fallback());
        let _ = db.seed_demo_data();

        let target_crew = requested_crew_id_from_env_or_args();
        let preset = CREW_PRESETS
            .iter()
            .find(|p| p.0 == target_crew)
            .copied()
            .unwrap_or(CREW_PRESETS[0]);

        let config = db
            .ensure_crew_profile(
                preset.0, preset.1, preset.2, preset.3, preset.4, preset.5, "ORBIT",
            )
            .unwrap_or(AstronautConfig {
                astronaut_id: preset.0.to_string(),
                callsign: preset.1.to_string(),
                weight: preset.2,
                height: preset.3,
                age: preset.4,
                sex: preset.5.to_string(),
                active_phase: "ORBIT".to_string(),
                mission_start_utc: "2026-11-14T00:00:00Z".to_string(),
                sleep_target_min: 480,
            });

        let lan_client =
            BiomonLanClient::new_auto(&config.astronaut_id, &config.callsign, &config.active_phase);
        let comms_state = lan_client.connection_state();
        let hub_endpoint = lan_client.target_hub_addr();

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
            .unwrap_or_else(|| "2026-11-14T20:00:00Z".to_string());
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
        let recent_inputs = db
            .load_recent_crew_inputs(&config.astronaut_id, 4)
            .unwrap_or_default();
        let local_alerts = db.load_local_alerts().unwrap_or_default();
        let sleep_epochs = db
            .load_sleep_epochs(&config.astronaut_id, mission_day)
            .unwrap_or_default();
        let unsynced = db.count_unsynced().unwrap_or_default();

        // NASA JPL Horizons 2026-11-14 -> 2026-11-15 Earth-Mars ephemeris (AU)
        let (earth_mars_au, one_way_light_min) =
            calc::interpolate_ephemeris(1.324362, 1.315854, 0.0);

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

        Self {
            mode: AppMode::ViewMode,
            screen: Screen::MainHud,
            selected_food_idx: 0,
            selected_meal_pct: 100,
            selected_protocol_idx: 0,
            status_banner: format!("{} WAL READY", db_label.to_ascii_uppercase()),
            hub_endpoint,
            pending_sync_batch: None,
            last_live_push_time: -1.0,
            snapshot,
            db,
            lan_client,
        }
    }

    pub fn refresh_db_state(&mut self) {
        let id = &self.snapshot.astronaut_id;
        let day = self.snapshot.mission_day;
        self.snapshot.sans_state = self.db.load_sans_state(id, day).ok().flatten();
        self.snapshot.osteo_state = self.db.load_osteo_state(id, day).ok().flatten();
        self.snapshot.daily_nutrition = self.db.load_daily_nutrition(id, day).unwrap_or_default();
        self.snapshot.recent_inputs = self.db.load_recent_crew_inputs(id, 4).unwrap_or_default();
        self.snapshot.local_alerts = self.db.load_local_alerts().unwrap_or_default();
        self.snapshot.sleep_epochs = self.db.load_sleep_epochs(id, day).unwrap_or_default();
        self.snapshot.unsynced = self.db.count_unsynced().unwrap_or_default();
    }

    pub fn log_selected_meal(&mut self) {
        if let Some(food) = self
            .snapshot
            .food_cache
            .get(self.selected_food_idx)
            .cloned()
        {
            let pct = self.selected_meal_pct as i64;
            let saved = self.db.insert_crew_input(&NewCrewInput {
                astronaut_id: &self.snapshot.astronaut_id,
                mission_day: self.snapshot.mission_day,
                input_type: "MEAL_LOG",
                reference_code: Some(&food.food_id),
                consumption_pct: Some(pct),
                notes: Some(&food.item_name),
                phase: &self.snapshot.active_phase,
                avg_co2_ppm: self.snapshot.cabin_co2_ppm,
            });
            if let Ok(saved_row) = saved {
                let batch_id = self.lan_client.next_batch_id();
                let sent = self.lan_client.queue_frame(OutboundFrame {
                    crew_id: self.snapshot.astronaut_id.clone(),
                    packet_type: "CREW_INPUT".to_string(),
                    payload: json!({
                        "batch_id": batch_id,
                        "astronaut_id": self.snapshot.astronaut_id,
                        "edge_input_id": saved_row.input_id,
                        "logged_at_utc": saved_row.logged_at_utc,
                        "mission_day": saved_row.mission_day,
                        "input_type": "MEAL_LOG",
                        "reference_code": food.food_id,
                        "consumption_pct": pct,
                        "notes": food.item_name,
                    }),
                });
                if sent {
                    self.pending_sync_batch = Some(batch_id);
                }
            }
            self.refresh_db_state();
            self.status_banner = format!("LOGGED {} AT {}%", food.food_id, pct);
        }
    }

    pub fn log_hydration_pouch(&mut self) {
        let saved = self.db.insert_crew_input(&NewCrewInput {
            astronaut_id: &self.snapshot.astronaut_id,
            mission_day: self.snapshot.mission_day,
            input_type: "FLUID_LOG",
            reference_code: Some("POUCH-250ML"),
            consumption_pct: Some(100),
            notes: Some("250 ML ELECTROLYTE POUCH"),
            phase: &self.snapshot.active_phase,
            avg_co2_ppm: self.snapshot.cabin_co2_ppm,
        });
        if let Ok(saved_row) = saved {
            let batch_id = self.lan_client.next_batch_id();
            let sent = self.lan_client.queue_frame(OutboundFrame {
                crew_id: self.snapshot.astronaut_id.clone(),
                packet_type: "CREW_INPUT".to_string(),
                payload: json!({
                    "batch_id": batch_id,
                    "astronaut_id": self.snapshot.astronaut_id,
                    "edge_input_id": saved_row.input_id,
                    "logged_at_utc": saved_row.logged_at_utc,
                    "mission_day": saved_row.mission_day,
                    "input_type": "FLUID_LOG",
                    "reference_code": "POUCH-250ML",
                    "consumption_pct": 100,
                    "notes": "250 ML ELECTROLYTE POUCH",
                    "fluid_ml": 250,
                }),
            });
            if sent {
                self.pending_sync_batch = Some(batch_id);
            }
        }
        self.refresh_db_state();
        self.status_banner = "LOGGED +250 ML HYDRATION POUCH".to_string();
    }

    pub fn set_mission_phase(&mut self, phase: &str) {
        self.snapshot.active_phase = phase.to_string();
        let _ = self
            .db
            .update_active_phase(&self.snapshot.astronaut_id, phase);
        self.snapshot.recompute_derived();
        self.status_banner = format!("ACTIVE PHASE SET TO {}", phase);
    }

    pub fn step_prebreathe_washout(&mut self, dt_sec: f64) {
        let suit_p = self.snapshot.suit_pressure_kpa.unwrap_or(29.6);
        let cur_r = self.snapshot.blood_n2_idx.unwrap_or(1.08);
        let cur_p_n2 = cur_r * suit_p;
        let (_, new_r) = calc::update_tissue_n2(cur_p_n2, 0.0, suit_p, dt_sec);
        self.snapshot.blood_n2_idx = Some(new_r);
        self.snapshot.eva_elapsed_sec += dt_sec as i64;
        self.snapshot.recompute_derived();
        self.status_banner = format!("O2 PREBREATHE +10M — N2 R={:.2}", new_r);
    }

    pub fn sync_queued_packets(&mut self) {
        let queued = self.snapshot.unsynced.total();
        if self.snapshot.comms_state != ConnectionState::Online {
            self.status_banner = format!("OFFLINE — {} REC BUFFERED IN WAL", queued);
            return;
        }
        let unsynced_inputs = self
            .db
            .load_unsynced_inputs(&self.snapshot.astronaut_id)
            .unwrap_or_default();
        let inputs_json: Vec<serde_json::Value> = unsynced_inputs
            .iter()
            .map(|inp| {
                json!({
                    "edge_input_id": inp.input_id,
                    "logged_at_utc": inp.logged_at_utc,
                    "mission_day": inp.mission_day,
                    "input_type": inp.input_type,
                    "reference_code": inp.reference_code,
                    "consumption_pct": inp.consumption_pct,
                    "notes": inp.notes,
                })
            })
            .collect();

        let batch_id = self.lan_client.next_batch_id();
        let sent = self.lan_client.queue_frame(OutboundFrame {
            crew_id: self.snapshot.astronaut_id.clone(),
            packet_type: "TELEMETRY_BATCH_SYNC".to_string(),
            payload: json!({
                "batch_id": batch_id,
                "seq_id": self.snapshot.seq_id,
                "astronaut_id": self.snapshot.astronaut_id,
                "callsign": self.snapshot.callsign,
                "active_phase": self.snapshot.active_phase,
                "mission_day": self.snapshot.mission_day,
                "hr_bpm": self.snapshot.hr_bpm,
                "sbp_mmhg": self.snapshot.sbp_mmhg,
                "resp_brpm": self.snapshot.resp_brpm,
                "spo2_pct": self.snapshot.spo2_pct,
                "tissue_o2_pct": self.snapshot.tissue_o2_pct,
                "blood_n2_idx": self.snapshot.blood_n2_idx,
                "g_load": self.snapshot.g_load,
                "imu_activity_idx": self.snapshot.imu_activity_idx,
                "skin_temp_c": self.snapshot.skin_temp_c,
                "radiation_usv_hr": self.snapshot.radiation_usv_hr,
                "cabin_co2_ppm": self.snapshot.cabin_co2_ppm,
                "cabin_o2_pct": self.snapshot.cabin_o2_pct,
                "suit_pressure_kpa": self.snapshot.suit_pressure_kpa,
                "suit_humidity_pct": self.snapshot.suit_humidity_pct,
                "eva_elapsed_sec": self.snapshot.eva_elapsed_sec,
                "hub_distance_m": self.snapshot.hub_distance_m,
                "heading_deg": self.snapshot.heading_deg,
                "timestamp_utc": self.snapshot.recorded_at_utc,
                "flushed_records": queued,
                "inputs": inputs_json,
            }),
        });
        if sent {
            self.pending_sync_batch = Some(batch_id);
            self.status_banner = format!("SENT BATCH #{batch_id} ({queued} REC) — AWAITING ACK");
        } else {
            self.status_banner = format!("OFFLINE — {queued} REC HELD IN WAL");
        }
    }

    pub fn rescan_lan(&mut self) {
        self.lan_client.trigger_rescan();
        self.hub_endpoint = self.lan_client.target_hub_addr();
        self.status_banner = "SCANNING LOCAL NETWORK FOR HUB...".to_string();
    }

    fn adopt_crew_identity(&mut self, crew_id: &str, callsign_hint: &str) {
        let preset = CREW_PRESETS
            .iter()
            .find(|p| p.0 == crew_id)
            .copied()
            .unwrap_or((
                "AST-01",
                if callsign_hint.is_empty() { "ARES-1" } else { callsign_hint },
                75.0,
                176.0,
                35,
                "M",
            ));
        let callsign = if callsign_hint.is_empty() {
            preset.1
        } else {
            callsign_hint
        };
        let cfg = self
            .db
            .ensure_crew_profile(
                crew_id,
                callsign,
                preset.2,
                preset.3,
                preset.4,
                preset.5,
                &self.snapshot.active_phase,
            )
            .unwrap_or(AstronautConfig {
                astronaut_id: crew_id.to_string(),
                callsign: callsign.to_string(),
                weight: preset.2,
                height: preset.3,
                age: preset.4,
                sex: preset.5.to_string(),
                active_phase: self.snapshot.active_phase.clone(),
                mission_start_utc: "2026-11-14T00:00:00Z".to_string(),
                sleep_target_min: 480,
            });
        self.snapshot.astronaut_id = cfg.astronaut_id.clone();
        self.snapshot.callsign = cfg.callsign.clone();
        self.snapshot.weight_kg = cfg.weight;
        self.snapshot.height_cm = cfg.height;
        self.snapshot.age_yr = cfg.age;
        self.snapshot.sex = cfg.sex.clone();
        self.refresh_db_state();
        self.snapshot.recompute_derived();
    }

    pub fn cycle_crew_identity(&mut self) {
        let cur_idx = CREW_PRESETS
            .iter()
            .position(|p| p.0 == self.snapshot.astronaut_id)
            .unwrap_or(0);
        let next = CREW_PRESETS[(cur_idx + 1) % CREW_PRESETS.len()];
        self.adopt_crew_identity(next.0, next.1);
        self.lan_client.set_identity(
            &self.snapshot.astronaut_id,
            &self.snapshot.callsign,
            &self.snapshot.active_phase,
        );
        self.status_banner = format!(
            "IDENTITY REQUESTED: {} ({})",
            self.snapshot.astronaut_id, self.snapshot.callsign
        );
    }

    pub fn acknowledge_alerts(&mut self) {
        let _ = self.db.acknowledge_all_alerts();
        self.refresh_db_state();
        self.status_banner = "LOCAL ALERTS ACKNOWLEDGED".to_string();
    }

    fn push_live_telemetry(&mut self) {
        self.snapshot.seq_id += 1;
        let _ = self.lan_client.queue_frame(OutboundFrame {
            crew_id: self.snapshot.astronaut_id.clone(),
            packet_type: "TELEMETRY_LIVE".to_string(),
            payload: json!({
                "seq_id": self.snapshot.seq_id,
                "mission_day": self.snapshot.mission_day,
                "active_phase": self.snapshot.active_phase,
                "hr_bpm": self.snapshot.hr_bpm,
                "sbp_mmhg": self.snapshot.sbp_mmhg,
                "resp_brpm": self.snapshot.resp_brpm,
                "spo2_pct": self.snapshot.spo2_pct,
                "tissue_o2_pct": self.snapshot.tissue_o2_pct,
                "blood_n2_idx": self.snapshot.blood_n2_idx,
                "g_load": self.snapshot.g_load,
                "imu_activity_idx": self.snapshot.imu_activity_idx,
                "skin_temp_c": self.snapshot.skin_temp_c,
                "radiation_usv_hr": self.snapshot.radiation_usv_hr,
                "cabin_co2_ppm": self.snapshot.cabin_co2_ppm,
                "cabin_o2_pct": self.snapshot.cabin_o2_pct,
                "suit_pressure_kpa": self.snapshot.suit_pressure_kpa,
                "suit_humidity_pct": self.snapshot.suit_humidity_pct,
                "eva_elapsed_sec": self.snapshot.eva_elapsed_sec,
                "hub_distance_m": self.snapshot.hub_distance_m,
                "heading_deg": self.snapshot.heading_deg,
                "timestamp_utc": self.snapshot.recorded_at_utc,
            }),
        });
    }

    fn poll_network(&mut self, time_sec: f64) {
        self.hub_endpoint = self.lan_client.target_hub_addr();

        while let Some(ev) = self.lan_client.try_recv_event() {
            match ev {
                LinkEvent::Connected(addr) => {
                    self.snapshot.comms_state = ConnectionState::Online;
                    self.hub_endpoint = addr.clone();
                    self.last_live_push_time = -1.0;
                    self.status_banner = format!("HUB CONNECTED: {addr}");
                    self.snapshot.recompute_derived();
                }
                LinkEvent::Disconnected => {
                    self.snapshot.comms_state = ConnectionState::Offline;
                    self.status_banner = "HUB LINK OFFLINE — LOCAL WAL BUFFERING".to_string();
                    self.snapshot.recompute_derived();
                }
                LinkEvent::Downlink(msg) => match msg {
                    HubDownlinkMessage::HelloAck {
                        hub_id,
                        mission_phase,
                        distance_au,
                        one_way_light_min,
                        assigned_crew_id,
                        assigned_callsign,
                    } => {
                        if !assigned_crew_id.is_empty()
                            && assigned_crew_id != self.snapshot.astronaut_id
                        {
                            self.adopt_crew_identity(&assigned_crew_id, &assigned_callsign);
                        }
                        self.snapshot.earth_mars_au = distance_au;
                        self.snapshot.one_way_light_min = one_way_light_min;
                        if !mission_phase.is_empty() && self.snapshot.active_phase != mission_phase
                        {
                            self.set_mission_phase(&mission_phase);
                        } else {
                            self.snapshot.recompute_derived();
                        }
                        if self.snapshot.unsynced.total() > 0 {
                            self.sync_queued_packets();
                        }
                        self.status_banner = format!(
                            "SLOT {} ({}) -> {} ({})",
                            self.snapshot.astronaut_id,
                            self.snapshot.callsign,
                            hub_id,
                            mission_phase
                        );
                    }
                    HubDownlinkMessage::SlotFull { max_slots, reason } => {
                        self.snapshot.comms_state = ConnectionState::Offline;
                        self.snapshot.recompute_derived();
                        self.status_banner = format!("{reason} (MAX {max_slots})");
                    }
                    HubDownlinkMessage::SyncAck {
                        batch_id,
                        synced_records,
                    } => {
                        let _ = self.db.mark_all_synced();
                        self.pending_sync_batch = None;
                        self.refresh_db_state();
                        self.status_banner =
                            format!("HUB ACK #{batch_id} ({synced_records} REC SYNCED)");
                    }
                    HubDownlinkMessage::PhaseSync {
                        phase,
                        apply_to_all,
                        target_crew_id,
                    } => {
                        if apply_to_all
                            || target_crew_id.as_deref()
                                == Some(self.snapshot.astronaut_id.as_str())
                        {
                            self.set_mission_phase(&phase);
                            self.status_banner = format!("HUB PHASE SYNC: {phase}");
                        }
                    }
                    HubDownlinkMessage::EphemerisSync {
                        distance_au,
                        one_way_light_min,
                        ..
                    } => {
                        self.snapshot.earth_mars_au = distance_au;
                        self.snapshot.one_way_light_min = one_way_light_min;
                        self.snapshot.recompute_derived();
                    }
                },
            }
        }

        let live_comms = self.lan_client.connection_state();
        if self.snapshot.comms_state != live_comms {
            self.snapshot.comms_state = live_comms;
            self.snapshot.recompute_derived();
        }

        if self.snapshot.comms_state == ConnectionState::Online
            && (self.last_live_push_time < 0.0 || time_sec - self.last_live_push_time >= 1.0)
        {
            self.last_live_push_time = time_sec;
            self.push_live_telemetry();
        }
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let time_sec = ui.input(|i| i.time);
        self.poll_network(time_sec);
        ui::render(ui, self);
    }
}
