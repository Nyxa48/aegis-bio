use crate::calc::{self, PanelAlertState, SafetyEvalInput};
use crate::db::{
    CrewInputHistoryRecord, CrewMember, Database, DatasetProvenance, FoodCatalogItem,
    HubCrewDailyNutrition, HubLabReport, InboundTelemetryFrame, MissionEphemeris,
    NewSurgeonIncident, OsteoporosisDailyHistory, ReferenceEdaPoint, ReferenceRadiationPoint,
    ResearchLabObservation, SansDailyHistory, SleepDailySummary, SurgeonIncident,
};
use crate::net::{HubLanServer, InboundPacket};
use crate::ui;
use crate::ui_components;
use eframe::egui;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubModule {
    CrewVitalsMonitor,
    PhaseAndStressControl,
    MedicalAndIncidentLog,
    FoodCatalogAndSync,
}

impl HubModule {
    pub fn title(self) -> &'static str {
        match self {
            HubModule::CrewVitalsMonitor => "CREW VITALS MONITOR",
            HubModule::PhaseAndStressControl => "PHASE & STRESS CONTROL",
            HubModule::MedicalAndIncidentLog => "MEDICAL & INCIDENT LOG",
            HubModule::FoodCatalogAndSync => "FOOD CATALOG & SYNC",
        }
    }

    pub fn is_input_module(self) -> bool {
        matches!(self, HubModule::PhaseAndStressControl)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionPhase {
    Launch,
    Orbit,
    Eva,
    Reentry,
}

impl MissionPhase {
    pub fn label(self) -> &'static str {
        match self {
            MissionPhase::Launch => "LAUNCH",
            MissionPhase::Orbit => "ORBIT",
            MissionPhase::Eva => "EVA",
            MissionPhase::Reentry => "REENTRY",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.to_ascii_uppercase().as_str() {
            "LAUNCH" => MissionPhase::Launch,
            "EVA" => MissionPhase::Eva,
            "REENTRY" => MissionPhase::Reentry,
            _ => MissionPhase::Orbit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubScreen {
    MainMenu,
    Module(HubModule),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StressScenario {
    TachycardiaSpike,
    HypoxiaDrop,
    SolarParticleRadiationSpike,
    CabinCo2ScrubbingFailure,
    SuitDecompressionN2Breach,
    HubRangeExcursion,
}

impl StressScenario {
    pub fn label(self) -> &'static str {
        match self {
            StressScenario::TachycardiaSpike => "TACHYCARDIA SPIKE",
            StressScenario::HypoxiaDrop => "HYPOXIA DROP",
            StressScenario::SolarParticleRadiationSpike => "SOLAR PARTICLE RADIATION SPIKE",
            StressScenario::CabinCo2ScrubbingFailure => "CABIN CO2 SCRUBBING FAILURE",
            StressScenario::SuitDecompressionN2Breach => "SUIT DECOMPRESSION / N2 BREACH",
            StressScenario::HubRangeExcursion => "HUB RANGE EXCURSION",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            StressScenario::TachycardiaSpike => "TACHYCARDIA",
            StressScenario::HypoxiaDrop => "HYPOXIA",
            StressScenario::SolarParticleRadiationSpike => "RADIATION_SPIKE",
            StressScenario::CabinCo2ScrubbingFailure => "CO2_SCRUB_FAIL",
            StressScenario::SuitDecompressionN2Breach => "DECOMP",
            StressScenario::HubRangeExcursion => "RANGE_EXCURSION",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrewSummarySnapshot {
    pub member: CrewMember,
    pub record_id: i64,
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
    pub skin_temp_c: f64,
    pub hub_distance_m: f64,
    pub heading_deg: f32,
    pub target_deg: f32,
    pub active_anomaly: Option<StressScenario>,
    pub daily_nutrition: HubCrewDailyNutrition,
    pub alerts: PanelAlertState,
}

impl CrewSummarySnapshot {
    pub fn recompute_derived(&mut self) {
        self.shock_index = calc::calc_shock_index(self.hr_bpm, self.sbp_mmhg);
        self.orthostatic_risk =
            calc::calc_orthostatic_risk(self.hr_bpm, self.sbp_mmhg, self.g_load).to_string();
        let (_, sans_tier) = calc::calc_sans_risk(
            self.daily_nutrition.sodium_mg_total,
            self.cabin_co2_ppm,
            &self.member.active_phase_code,
        );
        self.sans_risk = sans_tier.to_string();
        let comms_online = self.member.lan_status == "ONLINE";
        self.alerts = calc::evaluate_safety(&SafetyEvalInput {
            phase: &self.member.active_phase_code,
            hr_bpm: self.hr_bpm,
            resting_hr_bpm: 68.0,
            sbp_mmhg: self.sbp_mmhg,
            spo2_pct: self.spo2_pct,
            tissue_o2_pct: self.tissue_o2_pct,
            blood_n2_idx: self.blood_n2_idx,
            radiation_usv_hr: self.radiation_usv_hr,
            cabin_co2_ppm: self.cabin_co2_ppm,
            cabin_o2_pct: self.cabin_o2_pct,
            suit_pressure_kpa: self.suit_pressure_kpa,
            hub_distance_m: self.hub_distance_m,
            comms_online,
        });
    }
}

/// Non-blocking snapshot read by the 60 FPS `eframe` render loop without touching SQLite.
#[derive(Debug, Clone)]
pub struct HubUiSnapshot {
    pub crew_summaries: Vec<CrewSummarySnapshot>,
    pub mission_date: String,
    pub distance_au: f64,
    pub one_way_light_min: f64,
    pub ephemeris_all: Vec<MissionEphemeris>,
    pub unacked_incidents: i64,
    pub surgeon_incidents: Vec<SurgeonIncident>,
    pub lab_reports: Vec<HubLabReport>,
    pub radlab_series: Vec<ReferenceRadiationPoint>,
    pub eda_environment: Vec<ReferenceEdaPoint>,
    pub research_labs: Vec<ResearchLabObservation>,
    pub dataset_provenance: Vec<DatasetProvenance>,
    pub sleep_summaries: Vec<SleepDailySummary>,
    pub sans_history: Vec<SansDailyHistory>,
    pub osteo_history: Vec<OsteoporosisDailyHistory>,
    pub crew_inputs: Vec<CrewInputHistoryRecord>,
    pub food_catalog: Vec<FoodCatalogItem>,
    pub db_path: String,
    pub lan_ready: bool,
    pub connected_clients: usize,
    pub packets_received: u64,
    pub last_packet_info: String,
    pub alerts: PanelAlertState,
}

pub type UiSnapshot = HubUiSnapshot;

impl HubUiSnapshot {
    pub fn recompute_alerts(&mut self) {
        for crew in &mut self.crew_summaries {
            crew.recompute_derived();
        }

        let any_health_danger = self.crew_summaries.iter().any(|c| c.alerts.health_danger);
        let any_hub_danger = self.crew_summaries.iter().any(|c| c.alerts.hub_danger);
        let any_hazard_danger = self.crew_summaries.iter().any(|c| c.alerts.hazard_danger)
            || self.unacked_incidents > 0;
        let any_comms_danger =
            !self.lan_ready || self.crew_summaries.iter().any(|c| c.alerts.comms_danger);

        self.alerts = PanelAlertState {
            health_danger: any_health_danger,
            hub_danger: any_hub_danger,
            comms_danger: any_comms_danger,
            hazard_danger: any_hazard_danger,
        };
    }

    pub fn recompute_for_phase(&mut self, phase: MissionPhase, _selected_idx: usize) {
        let phase_code = phase.label();
        for crew in &mut self.crew_summaries {
            crew.member.active_phase_code = phase_code.to_string();
        }
        self.recompute_alerts();
    }
}

pub type HubApp = AppState;

pub struct AppState {
    pub screen: HubScreen,
    pub selected_crew_idx: usize,
    pub mission_phase: MissionPhase,
    pub stress_gain: f32,
    pub apply_phase_to_all: bool,
    pub crew_sim_enabled: bool,
    pub snapshot: HubUiSnapshot,
    pub db: Database,
    pub lan_server: HubLanServer,
    last_sim_tick: Instant,
    sim_tick_counter: u64,
    last_biomon_rx_at: HashMap<String, Instant>,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui_components::install_aerospace_fonts(&cc.egui_ctx);
        ui::apply_mandated_visuals(&cc.egui_ctx);

        let mut db =
            Database::open_default().unwrap_or_else(|_| Database::open_in_memory_fallback());
        let _ = db.seed_demo_data();

        let lan_server = HubLanServer::start("0.0.0.0:7878");
        Self::from_db_and_server(db, lan_server)
    }

    pub fn from_db_and_server(db: Database, lan_server: HubLanServer) -> Self {
        let snapshot = Self::build_snapshot_from_db(&db, &lan_server, 0, "NONE");
        let initial_phase = snapshot
            .crew_summaries
            .first()
            .map(|c| MissionPhase::from_code(&c.member.active_phase_code))
            .unwrap_or(MissionPhase::Orbit);

        Self {
            screen: HubScreen::MainMenu,
            selected_crew_idx: 0,
            mission_phase: initial_phase,
            stress_gain: 100.0,
            apply_phase_to_all: true,
            crew_sim_enabled: true,
            snapshot,
            db,
            lan_server,
            last_sim_tick: Instant::now(),
            sim_tick_counter: 0,
            last_biomon_rx_at: HashMap::new(),
        }
    }

    fn build_snapshot_from_db(
        db: &Database,
        lan_server: &HubLanServer,
        packets_received: u64,
        last_packet_info: &str,
    ) -> HubUiSnapshot {
        let members = db.load_crew_members().unwrap_or_default();
        let food_catalog = db.load_food_catalog().unwrap_or_default();
        let unacked_incidents = db.count_unacked_incidents().unwrap_or(0);
        let surgeon_incidents = db.load_surgeon_incidents().unwrap_or_default();
        let lab_reports = db.load_hub_lab_reports().unwrap_or_default();
        let radlab_series = db.load_radiation_series().unwrap_or_default();
        let eda_environment = db.load_eda_environment().unwrap_or_default();
        let research_labs = db
            .load_research_lab_observations(None, Some("CBC"), 40)
            .unwrap_or_default();
        let dataset_provenance = db.load_dataset_provenance().unwrap_or_default();
        let sleep_summaries = db.load_sleep_summaries().unwrap_or_default();
        let sans_history = db.load_sans_history().unwrap_or_default();
        let osteo_history = db.load_osteo_history().unwrap_or_default();
        let crew_inputs = db.load_crew_input_history().unwrap_or_default();
        let ephemeris_all = db.load_ephemeris_all().unwrap_or_default();

        let ephem = ephemeris_all
            .first()
            .cloned()
            .or_else(|| db.load_latest_ephemeris().ok().flatten());
        let mission_date = ephem
            .as_ref()
            .map(|e| e.mission_date.clone())
            .unwrap_or_else(|| "2026-11-14".to_string());
        let (distance_au, one_way_light_min) = if ephemeris_all.len() >= 2 {
            calc::interpolate_ephemeris(
                ephemeris_all[0].distance_au,
                ephemeris_all[1].distance_au,
                calc::utc_day_fraction(),
            )
        } else {
            let d = ephem.as_ref().map(|e| e.distance_au).unwrap_or(1.42);
            let l = ephem
                .as_ref()
                .map(|e| e.one_way_light_min)
                .unwrap_or_else(|| calc::calc_one_way_light_min(d));
            (d, l)
        };

        let mut crew_summaries = Vec::with_capacity(members.len());
        let headings = [38.0_f32, 112.0, 215.0, 304.0];
        let distances = [222.0_f64, 185.0, 340.0, 95.0];

        for (i, member) in members.into_iter().enumerate() {
            let latest = db
                .load_latest_telemetry(&member.astronaut_id)
                .ok()
                .flatten();

            let record_id = latest
                .as_ref()
                .map(|t| t.record_id)
                .unwrap_or((i + 1) as i64);
            let mission_day = latest.as_ref().map(|t| t.mission_day).unwrap_or(1);
            let daily_nutrition = db
                .load_crew_daily_nutrition(&member.astronaut_id, mission_day)
                .unwrap_or_default();
            let edge_seq_id = latest.as_ref().map(|t| t.edge_seq_id).unwrap_or(1);
            let recorded_at_utc = latest
                .as_ref()
                .map(|t| t.recorded_at_utc.clone())
                .unwrap_or_else(calc::utc_now_iso8601);
            let received_at_utc = latest
                .as_ref()
                .map(|t| t.received_at_utc.clone())
                .unwrap_or_else(calc::utc_now_iso8601);

            let hr_bpm = latest.as_ref().map(|t| t.hr_bpm).unwrap_or(88.0);
            let sbp_mmhg = latest.as_ref().map(|t| t.sbp_mmhg).unwrap_or(118.0);
            let resp_brpm = latest.as_ref().map(|t| t.resp_brpm).unwrap_or(14.0);
            let spo2_pct = latest.as_ref().map(|t| t.spo2_pct).unwrap_or(98.0);
            let tissue_o2_pct = latest.as_ref().and_then(|t| t.tissue_o2_pct).or(Some(93.5));
            let blood_n2_idx = latest.as_ref().and_then(|t| t.blood_n2_idx).or(Some(1.08));
            let g_load = latest.as_ref().map(|t| t.g_load).unwrap_or(1.0);
            let imu_activity_idx = latest.as_ref().map(|t| t.imu_activity_idx).unwrap_or(0.35);
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
            let suit_pressure_kpa = latest
                .as_ref()
                .and_then(|t| t.suit_pressure_kpa)
                .or(Some(29.6));
            let suit_humidity_pct = latest
                .as_ref()
                .and_then(|t| t.suit_humidity_pct)
                .or(Some(38.0));
            let eva_elapsed_sec = latest.as_ref().map(|t| t.eva_elapsed_sec).unwrap_or(1420);
            let radiation_usv_hr = latest.as_ref().map(|t| t.radiation_usv_hr).unwrap_or(0.42);
            let shock_index = latest
                .as_ref()
                .map(|t| t.shock_index)
                .unwrap_or_else(|| calc::calc_shock_index(hr_bpm, sbp_mmhg));
            let orthostatic_risk = latest
                .as_ref()
                .map(|t| t.orthostatic_risk.clone())
                .unwrap_or_else(|| {
                    calc::calc_orthostatic_risk(hr_bpm, sbp_mmhg, g_load).to_string()
                });
            let sans_risk = latest
                .as_ref()
                .map(|t| t.sans_risk.clone())
                .unwrap_or_else(|| {
                    calc::calc_sans_risk(
                        daily_nutrition.sodium_mg_total,
                        cabin_co2_ppm,
                        &member.active_phase_code,
                    )
                    .1
                    .to_string()
                });

            crew_summaries.push(CrewSummarySnapshot {
                member,
                record_id,
                mission_day,
                edge_seq_id,
                recorded_at_utc,
                received_at_utc,
                hr_bpm,
                sbp_mmhg,
                resp_brpm,
                spo2_pct,
                tissue_o2_pct,
                blood_n2_idx,
                g_load,
                imu_activity_idx,
                cabin_co2_ppm,
                cabin_temp_c,
                cabin_humidity_pct,
                cabin_pressure_kpa,
                cabin_o2_pct,
                cabin_motion_idx,
                suit_pressure_kpa,
                suit_humidity_pct,
                eva_elapsed_sec,
                radiation_usv_hr,
                shock_index,
                orthostatic_risk,
                sans_risk,
                skin_temp_c: 34.2,
                hub_distance_m: distances[i % distances.len()],
                heading_deg: headings[i % headings.len()],
                target_deg: 0.0,
                active_anomaly: None,
                daily_nutrition,
                alerts: PanelAlertState::default(),
            });
        }

        let mut snap = HubUiSnapshot {
            crew_summaries,
            mission_date,
            distance_au,
            one_way_light_min,
            ephemeris_all,
            unacked_incidents,
            surgeon_incidents,
            lab_reports,
            radlab_series,
            eda_environment,
            research_labs,
            dataset_provenance,
            sleep_summaries,
            sans_history,
            osteo_history,
            crew_inputs,
            food_catalog,
            db_path: db.path().display().to_string(),
            lan_ready: lan_server.is_ready(),
            connected_clients: lan_server.connected_clients(),
            packets_received,
            last_packet_info: last_packet_info.to_string(),
            alerts: PanelAlertState::default(),
        };
        snap.recompute_alerts();
        snap
    }

    pub fn set_mission_phase(&mut self, phase: MissionPhase) {
        self.mission_phase = phase;
        let phase_code = phase.label();
        if self.apply_phase_to_all {
            for crew in &mut self.snapshot.crew_summaries {
                crew.member.active_phase_code = phase_code.to_string();
                let _ = self
                    .db
                    .update_crew_phase(&crew.member.astronaut_id, phase_code);
            }
        } else if let Some(crew) = self.snapshot.crew_summaries.get_mut(self.selected_crew_idx) {
            crew.member.active_phase_code = phase_code.to_string();
            let _ = self
                .db
                .update_crew_phase(&crew.member.astronaut_id, phase_code);
        }
        self.snapshot.recompute_alerts();
    }

    pub fn select_crew(&mut self, idx: usize) {
        if idx < self.snapshot.crew_summaries.len() {
            self.selected_crew_idx = idx;
            self.mission_phase = MissionPhase::from_code(
                &self.snapshot.crew_summaries[idx].member.active_phase_code,
            );
            self.snapshot.recompute_alerts();
        }
    }

    pub fn inject_stress_scenario(&mut self, scenario: StressScenario) {
        let scale = (self.stress_gain / 100.0).clamp(0.25, 2.0) as f64;
        let target_indices: Vec<usize> = if self.apply_phase_to_all {
            (0..self.snapshot.crew_summaries.len()).collect()
        } else {
            vec![self.selected_crew_idx]
        };

        for idx in target_indices {
            if let Some(crew) = self.snapshot.crew_summaries.get_mut(idx) {
                crew.active_anomaly = Some(scenario);
                match scenario {
                    StressScenario::TachycardiaSpike => {
                        crew.hr_bpm = (128.0 + 36.0 * scale).clamp(115.0, 210.0);
                        crew.sbp_mmhg = (96.0 - 10.0 * (scale - 1.0)).clamp(78.0, 125.0);
                        crew.resp_brpm = (22.0 + 6.0 * scale).clamp(16.0, 38.0);
                        crew.skin_temp_c = (35.8 + 0.8 * scale).clamp(34.5, 38.5);
                        crew.imu_activity_idx = (0.78 * scale).clamp(0.4, 1.0);
                    }
                    StressScenario::HypoxiaDrop => {
                        crew.spo2_pct = (90.0 - 5.0 * scale).clamp(76.0, 91.5);
                        crew.tissue_o2_pct = Some((82.0 - 6.0 * scale).clamp(68.0, 84.5));
                        crew.cabin_o2_pct = Some((19.1 - 0.8 * scale).clamp(16.5, 19.4));
                        crew.resp_brpm = (24.0 + 4.0 * scale).clamp(18.0, 36.0);
                    }
                    StressScenario::SolarParticleRadiationSpike => {
                        crew.radiation_usv_hr = (65.0 * scale).clamp(52.0, 240.0);
                    }
                    StressScenario::CabinCo2ScrubbingFailure => {
                        crew.cabin_co2_ppm = (3400.0 + 1100.0 * scale).clamp(3100.0, 6800.0);
                        crew.resp_brpm = (20.0 + 3.0 * scale).clamp(16.0, 32.0);
                    }
                    StressScenario::SuitDecompressionN2Breach => {
                        crew.member.active_phase_code = "EVA".to_string();
                        self.mission_phase = MissionPhase::Eva;
                        let suit_p = (22.5 - 4.0 * scale).clamp(14.0, 24.5);
                        crew.suit_pressure_kpa = Some(suit_p);
                        let (_, r_val) = calc::update_tissue_n2(79.0, 0.0, suit_p, 120.0);
                        crew.blood_n2_idx = Some((r_val * scale.max(0.85)).max(1.58));
                    }
                    StressScenario::HubRangeExcursion => {
                        crew.member.active_phase_code = "EVA".to_string();
                        self.mission_phase = MissionPhase::Eva;
                        crew.hub_distance_m = (1650.0 + 450.0 * scale).clamp(1520.0, 3200.0);
                        crew.heading_deg = (crew.heading_deg + 45.0) % 360.0;
                    }
                }
            }
        }
        self.snapshot.recompute_alerts();
    }

    pub fn reset_to_nominal_telemetry(&mut self) {
        let now_utc = calc::utc_now_iso8601();
        let distances = [222.0_f64, 185.0, 340.0, 95.0];
        let mut incident_added = false;
        self.mission_phase = MissionPhase::Orbit;
        for (i, crew) in self.snapshot.crew_summaries.iter_mut().enumerate() {
            crew.active_anomaly = None;
            crew.member.active_phase_code = "ORBIT".to_string();
            let _ = self.db.update_crew_phase(&crew.member.astronaut_id, "ORBIT");
            crew.hr_bpm = 76.0 + (i as f64 * 4.0);
            crew.sbp_mmhg = 118.0;
            crew.resp_brpm = 14.0;
            crew.spo2_pct = 98.2;
            crew.tissue_o2_pct = Some(93.5);
            crew.blood_n2_idx = Some(1.08);
            crew.g_load = 1.0;
            crew.imu_activity_idx = 0.35;
            crew.cabin_co2_ppm = 1150.0;
            crew.cabin_o2_pct = Some(20.9);
            crew.suit_pressure_kpa = Some(29.6);
            crew.radiation_usv_hr = 0.40;
            crew.hub_distance_m = distances[i % distances.len()];
            let edge_seq = self
                .db
                .next_internal_edge_seq_id(&crew.member.astronaut_id)
                .unwrap_or(-1);
            Self::persist_crew_snapshot_row(&self.db, crew, edge_seq, &now_utc);
            if Self::maybe_record_auto_incident(&self.db, crew, &now_utc) {
                incident_added = true;
            }
        }
        if incident_added {
            self.refresh_incidents_from_db();
        }
        self.snapshot.recompute_alerts();
    }

    pub fn log_active_alert_incident(&mut self) {
        let Some(crew) = self.snapshot.crew_summaries.get(self.selected_crew_idx) else {
            return;
        };
        let hazard =
            crew.active_anomaly
                .map(|a| a.code())
                .unwrap_or(if crew.alerts.hazard_danger {
                    "RADIATION_SPIKE"
                } else if crew.alerts.health_danger {
                    "ORTHOSTATIC_RISK"
                } else {
                    "MANUAL_SURGEON_LOG"
                });

        let details = format!(
            "Phase {} | HR {:.0} BPM | SBP {:.0} mmHg | SpO2 {:.1}% | CO2 {:.0} ppm | Rad {:.2} uSv/h",
            crew.member.active_phase_code,
            crew.hr_bpm,
            crew.sbp_mmhg,
            crew.spo2_pct,
            crew.cabin_co2_ppm,
            crew.radiation_usv_hr
        );
        let med = if crew.alerts.health_danger {
            "100% O2 + Isotonic Fluid Protocol"
        } else {
            "Continuous Telemetry Observation"
        };
        let notes = format!(
            "Logged from Phase & Stress Control (Gain {:.0}%)",
            self.stress_gain
        );

        let now_utc = calc::utc_now_iso8601();
        let _ = self.db.insert_surgeon_incident(&NewSurgeonIncident {
            astronaut_id: &crew.member.astronaut_id,
            telemetry_record_id: Some(crew.record_id),
            created_at_utc: &now_utc,
            hazard_type: hazard,
            injury_details: Some(&details),
            medication_administered: Some(med),
            medical_notes: Some(&notes),
        });
        self.refresh_incidents_from_db();
    }

    pub fn log_quick_meal_for_selected_crew(&mut self, food_id: &str, consumption_pct: i64) {
        let Some(crew) = self.snapshot.crew_summaries.get(self.selected_crew_idx) else {
            return;
        };
        let crew_id = crew.member.astronaut_id.clone();
        let day = crew.mission_day;
        let phase = crew.member.active_phase_code.clone();
        let co2 = crew.cabin_co2_ppm;

        if self
            .db
            .insert_manual_meal_log(&crew_id, day, food_id, consumption_pct, &phase, co2)
            .is_ok()
        {
            if let Some(c) = self.snapshot.crew_summaries.get_mut(self.selected_crew_idx) {
                c.daily_nutrition = self
                    .db
                    .load_crew_daily_nutrition(&crew_id, day)
                    .unwrap_or_default();
                c.recompute_derived();
            }
            self.snapshot.crew_inputs = self.db.load_crew_input_history().unwrap_or_default();
            self.snapshot.sans_history = self.db.load_sans_history().unwrap_or_default();
            self.snapshot.osteo_history = self.db.load_osteo_history().unwrap_or_default();
            self.snapshot.recompute_alerts();
        }
    }

    pub fn toggle_crew_sim(&mut self) {
        self.crew_sim_enabled = !self.crew_sim_enabled;
    }

    pub fn toggle_incident_resolved(&mut self, incident_id: i64, resolved: bool) {
        if self
            .db
            .resolve_surgeon_incident(incident_id, resolved)
            .is_ok()
        {
            self.refresh_incidents_from_db();
        }
    }

    pub fn refresh_incidents_from_db(&mut self) {
        self.snapshot.surgeon_incidents = self.db.load_surgeon_incidents().unwrap_or_default();
        self.snapshot.unacked_incidents = self.db.count_unacked_incidents().unwrap_or(0);
        self.snapshot.recompute_alerts();
    }

    pub fn simulate_inbound_biomon_packet(&mut self) {
        let crew_id = self
            .snapshot
            .crew_summaries
            .get(self.selected_crew_idx)
            .map(|c| c.member.astronaut_id.clone())
            .unwrap_or_else(|| "AST-01".to_string());
        let pkt = InboundPacket {
            crew_id,
            packet_type: "TELEMETRY_SYNC".to_string(),
            payload: serde_json::json!({
                "hr_bpm": 94.0,
                "sbp_mmhg": 121.0,
                "spo2_pct": 98.5,
                "recorded_at_utc": calc::utc_now_iso8601(),
            }),
        };
        self.lan_server.inject_test_packet(pkt);
    }

    fn persist_crew_snapshot_row(
        db: &Database,
        crew: &mut CrewSummarySnapshot,
        edge_seq_id: i64,
        recorded_at_utc: &str,
    ) {
        crew.recompute_derived();
        if let Ok(rec_id) = db.upsert_telemetry_frame(&InboundTelemetryFrame {
            astronaut_id: &crew.member.astronaut_id,
            phase_code: &crew.member.active_phase_code,
            mission_day: crew.mission_day,
            edge_seq_id,
            recorded_at_utc,
            hr_bpm: crew.hr_bpm,
            sbp_mmhg: crew.sbp_mmhg,
            resp_brpm: crew.resp_brpm,
            spo2_pct: crew.spo2_pct,
            tissue_o2_pct: crew.tissue_o2_pct,
            blood_n2_idx: crew.blood_n2_idx,
            g_load: crew.g_load,
            imu_activity_idx: crew.imu_activity_idx,
            cabin_co2_ppm: crew.cabin_co2_ppm,
            cabin_temp_c: crew.cabin_temp_c,
            cabin_humidity_pct: crew.cabin_humidity_pct,
            cabin_pressure_kpa: crew.cabin_pressure_kpa,
            cabin_o2_pct: crew.cabin_o2_pct,
            cabin_motion_idx: crew.cabin_motion_idx,
            suit_pressure_kpa: crew.suit_pressure_kpa,
            suit_humidity_pct: crew.suit_humidity_pct,
            eva_elapsed_sec: crew.eva_elapsed_sec,
            radiation_usv_hr: crew.radiation_usv_hr,
            shock_index: crew.shock_index,
            orthostatic_risk: &crew.orthostatic_risk,
            sans_risk: &crew.sans_risk,
        }) {
            crew.record_id = rec_id;
            crew.edge_seq_id = edge_seq_id;
            crew.recorded_at_utc = recorded_at_utc.to_string();
            crew.received_at_utc = calc::utc_now_iso8601();
        }
    }

    fn maybe_record_auto_incident(db: &Database, crew: &CrewSummarySnapshot, now_utc: &str) -> bool {
        if !crew.alerts.health_danger && !crew.alerts.hazard_danger {
            return false;
        }
        let hazard_type = if crew.alerts.hazard_danger && crew.radiation_usv_hr > 50.0 {
            "RADIATION_SPIKE"
        } else if crew.blood_n2_idx.is_some_and(|r| r > 1.50)
            || (crew.member.active_phase_code == "EVA"
                && crew.suit_pressure_kpa.is_some_and(|p| p < 25.0))
        {
            "DECOMP"
        } else if crew.spo2_pct < 92.0 || crew.tissue_o2_pct.is_some_and(|t| t < 85.0) {
            "HYPOXIA"
        } else {
            "ORTHOSTATIC_RISK"
        };

        if db
            .has_open_incident(&crew.member.astronaut_id, hazard_type)
            .unwrap_or(true)
        {
            return false;
        }

        let details = format!(
            "Auto telemetry threshold breach: Phase {} | HR {:.0} BPM | SBP {:.0} mmHg | SpO2 {:.1}% | CO2 {:.0} ppm | Rad {:.2} uSv/h",
            crew.member.active_phase_code,
            crew.hr_bpm,
            crew.sbp_mmhg,
            crew.spo2_pct,
            crew.cabin_co2_ppm,
            crew.radiation_usv_hr
        );
        let _ = db.insert_surgeon_incident(&NewSurgeonIncident {
            astronaut_id: &crew.member.astronaut_id,
            telemetry_record_id: Some(crew.record_id),
            created_at_utc: now_utc,
            hazard_type,
            injury_details: Some(&details),
            medication_administered: Some("Pending Flight Surgeon Review"),
            medical_notes: Some("Auto-generated from live Biomon/Hub threshold breach"),
        });
        true
    }

    fn ingest_telemetry_object(
        &mut self,
        crew_id: &str,
        obj: &serde_json::Value,
        now_utc: &str,
    ) -> bool {
        let Some(crew) = self
            .snapshot
            .crew_summaries
            .iter_mut()
            .find(|c| c.member.astronaut_id == crew_id)
        else {
            return false;
        };

        if let Some(day) = obj.get("mission_day").and_then(|v| v.as_i64()) {
            crew.mission_day = day;
        }
        if let Some(phase) = obj
            .get("mission_phase")
            .or_else(|| obj.get("active_phase"))
            .and_then(|v| v.as_str())
        {
            crew.member.active_phase_code = phase.to_string();
        }
        if let Some(hr) = obj.get("hr_bpm").and_then(|v| v.as_f64()) {
            crew.hr_bpm = hr;
        }
        if let Some(sbp) = obj.get("sbp_mmhg").and_then(|v| v.as_f64()) {
            crew.sbp_mmhg = sbp;
        }
        if let Some(resp) = obj.get("resp_brpm").and_then(|v| v.as_f64()) {
            crew.resp_brpm = resp;
        }
        if let Some(spo2) = obj.get("spo2_pct").and_then(|v| v.as_f64()) {
            crew.spo2_pct = spo2;
        }
        if let Some(to2) = obj.get("tissue_o2_pct").and_then(|v| v.as_f64()) {
            crew.tissue_o2_pct = Some(to2);
        }
        if let Some(r_idx) = obj.get("blood_n2_idx").and_then(|v| v.as_f64()) {
            crew.blood_n2_idx = Some(r_idx);
        }
        if let Some(g) = obj.get("g_load").and_then(|v| v.as_f64()) {
            crew.g_load = g;
        }
        if let Some(imu) = obj.get("imu_activity_idx").and_then(|v| v.as_f64()) {
            crew.imu_activity_idx = imu;
        }
        if let Some(temp) = obj.get("skin_temp_c").and_then(|v| v.as_f64()) {
            crew.skin_temp_c = temp;
        }
        if let Some(rad) = obj.get("radiation_usv_hr").and_then(|v| v.as_f64()) {
            crew.radiation_usv_hr = rad;
        }
        if let Some(co2) = obj.get("cabin_co2_ppm").and_then(|v| v.as_f64()) {
            crew.cabin_co2_ppm = co2;
        }
        if let Some(ct) = obj.get("cabin_temp_c").and_then(|v| v.as_f64()) {
            crew.cabin_temp_c = Some(ct);
        }
        if let Some(ch) = obj.get("cabin_humidity_pct").and_then(|v| v.as_f64()) {
            crew.cabin_humidity_pct = Some(ch);
        }
        if let Some(cp) = obj.get("cabin_pressure_kpa").and_then(|v| v.as_f64()) {
            crew.cabin_pressure_kpa = Some(cp);
        }
        if let Some(co2_pct) = obj.get("cabin_o2_pct").and_then(|v| v.as_f64()) {
            crew.cabin_o2_pct = Some(co2_pct);
        }
        if let Some(cm) = obj.get("cabin_motion_idx").and_then(|v| v.as_f64()) {
            crew.cabin_motion_idx = cm;
        }
        if let Some(sp) = obj.get("suit_pressure_kpa").and_then(|v| v.as_f64()) {
            crew.suit_pressure_kpa = Some(sp);
        }
        if let Some(sh) = obj.get("suit_humidity_pct").and_then(|v| v.as_f64()) {
            crew.suit_humidity_pct = Some(sh);
        }
        if let Some(eva_s) = obj.get("eva_elapsed_sec").and_then(|v| v.as_i64()) {
            crew.eva_elapsed_sec = eva_s;
        }

        let recorded_at = obj
            .get("recorded_at_utc")
            .or_else(|| obj.get("timestamp_utc"))
            .and_then(|v| v.as_str())
            .unwrap_or(now_utc);

        let edge_seq = obj
            .get("seq_id")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| self.db.next_internal_edge_seq_id(crew_id).unwrap_or(-1));

        crew.member.lan_status = "ONLINE".to_string();
        crew.member.last_heartbeat_utc = now_utc.to_string();
        let phase_code = crew.member.active_phase_code.clone();
        Self::persist_crew_snapshot_row(&self.db, crew, edge_seq, recorded_at);
        let _ = self
            .db
            .update_crew_lan_heartbeat(crew_id, Some(&phase_code), "ONLINE", now_utc);
        Self::maybe_record_auto_incident(&self.db, crew, now_utc)
    }

    pub fn poll_network(&mut self) {
        self.snapshot.lan_ready = self.lan_server.is_ready();
        self.snapshot.connected_clients = self.lan_server.connected_clients();
        let mut changed = false;
        let mut refresh_inputs = false;
        let mut refresh_sleep = false;
        let mut refresh_daily = false;
        let mut refresh_incidents = false;

        while let Some(pkt) = self.lan_server.try_recv_packet() {
            let now_utc = calc::utc_now_iso8601();
            self.snapshot.packets_received += 1;
            self.snapshot.last_packet_info =
                format!("{} // {} @ {}", pkt.crew_id, pkt.packet_type, now_utc);
            self.last_biomon_rx_at
                .insert(pkt.crew_id.clone(), Instant::now());

            match pkt.packet_type.as_str() {
                "TELEMETRY_SYNC" | "TELEMETRY_BATCH_SYNC" => {
                    if let Some(frames) = pkt.payload.get("frames").and_then(|v| v.as_array()) {
                        for frame_val in frames {
                            let target_id = frame_val
                                .get("astronaut_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&pkt.crew_id);
                            if self.ingest_telemetry_object(target_id, frame_val, &now_utc) {
                                refresh_incidents = true;
                            }
                        }
                    } else if self.ingest_telemetry_object(&pkt.crew_id, &pkt.payload, &now_utc) {
                        refresh_incidents = true;
                    }
                }
                "CREW_INPUT" => {
                    let items: Vec<serde_json::Value> = pkt
                        .payload
                        .get("items")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_else(|| vec![pkt.payload.clone()]);
                    for item in &items {
                        let ast_id = item
                            .get("astronaut_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&pkt.crew_id);
                        let edge_input_id = item
                            .get("input_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or_else(|| {
                                self.db.next_internal_edge_input_id(ast_id).unwrap_or(-1)
                            });
                        let logged_at = item
                            .get("logged_at_utc")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&now_utc);
                        let day = item.get("mission_day").and_then(|v| v.as_i64()).unwrap_or(1);
                        let input_type = item
                            .get("input_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("MEAL_LOG");
                        let ref_code = item.get("reference_code").and_then(|v| v.as_str());
                        let pct = item.get("consumption_pct").and_then(|v| v.as_i64());
                        let notes = item.get("notes").and_then(|v| v.as_str());
                        let (phase, co2) = self
                            .snapshot
                            .crew_summaries
                            .iter()
                            .find(|c| c.member.astronaut_id == ast_id)
                            .map(|c| (c.member.active_phase_code.clone(), c.cabin_co2_ppm))
                            .unwrap_or_else(|| ("ORBIT".to_string(), 1150.0));
                        let _ = self.db.upsert_crew_input(
                            ast_id,
                            edge_input_id,
                            logged_at,
                            day,
                            input_type,
                            ref_code,
                            pct,
                            notes,
                            &phase,
                            co2,
                        );
                        if let Some(crew) = self
                            .snapshot
                            .crew_summaries
                            .iter_mut()
                            .find(|c| c.member.astronaut_id == ast_id)
                        {
                            crew.daily_nutrition = self
                                .db
                                .load_crew_daily_nutrition(ast_id, day)
                                .unwrap_or_default();
                            crew.member.lan_status = "ONLINE".to_string();
                            crew.member.last_heartbeat_utc = now_utc.clone();
                        }
                    }
                    let _ = self
                        .db
                        .update_crew_lan_heartbeat(&pkt.crew_id, None, "ONLINE", &now_utc);
                    refresh_inputs = true;
                    refresh_daily = true;
                }
                "SLEEP_EPOCH" => {
                    let epochs: Vec<serde_json::Value> = pkt
                        .payload
                        .get("epochs")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_else(|| vec![pkt.payload.clone()]);
                    for ep in &epochs {
                        let ast_id = ep
                            .get("astronaut_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&pkt.crew_id);
                        let edge_epoch_id = ep.get("epoch_id").and_then(|v| v.as_i64()).unwrap_or(1);
                        let start_utc = ep
                            .get("epoch_start_utc")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&now_utc);
                        let day = ep.get("mission_day").and_then(|v| v.as_i64()).unwrap_or(1);
                        let activity = ep
                            .get("activity_count")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(5.0);
                        let hr_avg = ep.get("hr_bpm_avg").and_then(|v| v.as_f64());
                        let state = ep
                            .get("sleep_state")
                            .and_then(|v| v.as_str())
                            .unwrap_or("SLEEP");
                        let _ = self.db.upsert_sleep_epoch(
                            ast_id,
                            edge_epoch_id,
                            start_utc,
                            day,
                            activity,
                            hr_avg,
                            state,
                        );
                    }
                    let _ = self
                        .db
                        .update_crew_lan_heartbeat(&pkt.crew_id, None, "ONLINE", &now_utc);
                    refresh_sleep = true;
                }
                "DAILY_STATES" => {
                    if let Some(sans_arr) = pkt.payload.get("sans").and_then(|v| v.as_array()) {
                        for s in sans_arr {
                            let ast_id = s
                                .get("astronaut_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&pkt.crew_id);
                            let day = s.get("mission_day").and_then(|v| v.as_i64()).unwrap_or(1);
                            let na = s
                                .get("sodium_mg_total")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let avg_co2 = s
                                .get("avg_cabin_co2_ppm")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(1150.0);
                            let max_co2 = s
                                .get("max_cabin_co2_ppm")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(avg_co2);
                            let score = s
                                .get("sans_score")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let risk = s
                                .get("sans_risk")
                                .and_then(|v| v.as_str())
                                .unwrap_or("LOW");
                            let computed_at = s
                                .get("computed_at_utc")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&now_utc);
                            let _ = self.db.upsert_sans_daily(
                                ast_id,
                                day,
                                na,
                                avg_co2,
                                max_co2,
                                score,
                                risk,
                                computed_at,
                            );
                        }
                    }
                    if let Some(osteo_arr) = pkt.payload.get("osteo").and_then(|v| v.as_array()) {
                        for o in osteo_arr {
                            let ast_id = o
                                .get("astronaut_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&pkt.crew_id);
                            let day = o.get("mission_day").and_then(|v| v.as_i64()).unwrap_or(1);
                            let ex = o
                                .get("exercise_load_idx")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(75.0);
                            let ca = o
                                .get("calcium_mg_total")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let d3 = o
                                .get("vit_d3_iu_total")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            let score = o
                                .get("protection_score")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(50.0);
                            let risk = o
                                .get("osteo_risk")
                                .and_then(|v| v.as_str())
                                .unwrap_or("MODERATE");
                            let computed_at = o
                                .get("computed_at_utc")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&now_utc);
                            let _ = self.db.upsert_osteoporosis_daily(
                                ast_id,
                                day,
                                ex,
                                ca,
                                d3,
                                score,
                                risk,
                                computed_at,
                            );
                        }
                    }
                    let _ = self
                        .db
                        .update_crew_lan_heartbeat(&pkt.crew_id, None, "ONLINE", &now_utc);
                    refresh_daily = true;
                }
                _ => {}
            }
            changed = true;
        }

        if refresh_inputs {
            self.snapshot.crew_inputs = self.db.load_crew_input_history().unwrap_or_default();
        }
        if refresh_sleep {
            self.snapshot.sleep_summaries = self.db.load_sleep_summaries().unwrap_or_default();
        }
        if refresh_daily {
            self.snapshot.sans_history = self.db.load_sans_history().unwrap_or_default();
            self.snapshot.osteo_history = self.db.load_osteo_history().unwrap_or_default();
        }
        if refresh_incidents {
            self.snapshot.surgeon_incidents = self.db.load_surgeon_incidents().unwrap_or_default();
            self.snapshot.unacked_incidents = self.db.count_unacked_incidents().unwrap_or(0);
        }
        if changed {
            self.snapshot.recompute_alerts();
        }
    }

    pub fn step_hub_crew_simulation_1hz(&mut self) {
        self.sim_tick_counter = self.sim_tick_counter.saturating_add(1);
        let now = Instant::now();
        let now_utc = calc::utc_now_iso8601();
        let mut incident_added = false;
        let recompute_states = self.sim_tick_counter % 15 == 0;

        for (idx, crew) in self.snapshot.crew_summaries.iter_mut().enumerate() {
            // If Biomon pushed live TCP packets within the last 5s, let Biomon drive this crew member
            if let Some(rx_at) = self.last_biomon_rx_at.get(&crew.member.astronaut_id)
                && now.duration_since(*rx_at) < Duration::from_secs(5)
            {
                continue;
            }

            let t = self.sim_tick_counter as f64 + (idx as f64 * 1.7);
            let wave_fast = (t * 0.45).sin() * 0.7 + (t * 0.19).cos() * 0.3;
            let wave_slow = (t * 0.06).sin();

            if crew.active_anomaly.is_none() {
                let (base_hr, base_sbp, base_resp, base_co2, base_rad) =
                    match crew.member.active_phase_code.to_ascii_uppercase().as_str() {
                        "EVA" => (118.0 + 5.0 * wave_slow, 126.0, 20.0, 960.0, 0.72),
                        "LAUNCH" => (122.0 + 4.0 * wave_slow, 134.0, 22.0, 1180.0, 0.25),
                        "REENTRY" => (116.0 + 4.0 * wave_slow, 130.0, 20.0, 1240.0, 0.30),
                        _ => (
                            74.0 + (idx as f64 * 4.0) + 3.0 * wave_slow,
                            116.0 + (idx as f64 * 2.0),
                            13.5 + (idx as f64 * 0.5),
                            1120.0 + (idx as f64 * 25.0),
                            0.39 + (idx as f64 * 0.015),
                        ),
                    };

                crew.hr_bpm = (crew.hr_bpm + (base_hr - crew.hr_bpm) * 0.22 + wave_fast * 1.1)
                    .clamp(45.0, 175.0);
                crew.sbp_mmhg = (crew.sbp_mmhg + (base_sbp - crew.sbp_mmhg) * 0.20 + wave_fast * 0.6)
                    .clamp(85.0, 165.0);
                crew.resp_brpm =
                    (crew.resp_brpm + (base_resp - crew.resp_brpm) * 0.20 + wave_fast * 0.25)
                        .clamp(10.0, 32.0);
                crew.spo2_pct = (98.3 + wave_slow * 0.4).clamp(95.0, 100.0);
                crew.cabin_co2_ppm = (base_co2 + 40.0 * wave_slow).clamp(400.0, 2500.0);
                crew.radiation_usv_hr = (base_rad + 0.04 * wave_slow).clamp(0.1, 20.0);
                if crew.member.active_phase_code == "EVA" {
                    crew.eva_elapsed_sec += 1;
                }
            } else {
                crew.hr_bpm = (crew.hr_bpm + wave_fast * 0.6).clamp(40.0, 210.0);
            }

            let edge_seq = self
                .db
                .next_internal_edge_seq_id(&crew.member.astronaut_id)
                .unwrap_or(-1);
            Self::persist_crew_snapshot_row(&self.db, crew, edge_seq, &now_utc);
            if Self::maybe_record_auto_incident(&self.db, crew, &now_utc) {
                incident_added = true;
            }
            if recompute_states {
                let _ = self.db.recompute_crew_daily_states(
                    &crew.member.astronaut_id,
                    crew.mission_day,
                    &crew.member.active_phase_code,
                    crew.cabin_co2_ppm,
                );
            }
        }

        if incident_added {
            self.snapshot.surgeon_incidents = self.db.load_surgeon_incidents().unwrap_or_default();
            self.snapshot.unacked_incidents = self.db.count_unacked_incidents().unwrap_or(0);
        }
        if recompute_states {
            self.snapshot.sans_history = self.db.load_sans_history().unwrap_or_default();
            self.snapshot.osteo_history = self.db.load_osteo_history().unwrap_or_default();
        }
        self.snapshot.recompute_alerts();
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_network();

        let now = Instant::now();
        if self.crew_sim_enabled {
            if now.duration_since(self.last_sim_tick) >= Duration::from_secs(1) {
                self.last_sim_tick = now;
                self.step_hub_crew_simulation_1hz();
            }
        } else {
            self.last_sim_tick = now;
        }

        ui.ctx().request_repaint_after(Duration::from_millis(250));
        ui::render(ui, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::HubLanServer;

    #[test]
    fn test_hub_app_network_persistence_and_sodium_sans() {
        let mut db = Database::open_in_memory_fallback();
        db.seed_demo_data().unwrap();
        let lan_server = HubLanServer::start("127.0.0.1:0");
        let mut app = AppState::from_db_and_server(db, lan_server);

        let init_na = app.snapshot.crew_summaries[0].daily_nutrition.sodium_mg_total;
        assert!(init_na > 300.0);

        // Inject batched Biomon telemetry frame with acute hypoxia + high CO2 -> persists + incident
        app.lan_server.inject_test_packet(InboundPacket {
            crew_id: "AST-01".to_string(),
            packet_type: "TELEMETRY_SYNC".to_string(),
            payload: serde_json::json!({
                "frames": [{
                    "seq_id": 500,
                    "astronaut_id": "AST-01",
                    "mission_day": 1,
                    "mission_phase": "ORBIT",
                    "recorded_at_utc": "2026-11-15T10:30:00Z",
                    "hr_bpm": 134.0,
                    "sbp_mmhg": 100.0,
                    "spo2_pct": 87.5,
                    "cabin_co2_ppm": 3400.0,
                    "radiation_usv_hr": 0.45
                }]
            }),
        });

        // Inject CREW_INPUT high sodium meal from FOOD-03 (690 mg Na at 100%)
        app.lan_server.inject_test_packet(InboundPacket {
            crew_id: "AST-01".to_string(),
            packet_type: "CREW_INPUT".to_string(),
            payload: serde_json::json!({
                "items": [{
                    "input_id": 88,
                    "astronaut_id": "AST-01",
                    "logged_at_utc": "2026-11-15T10:31:00Z",
                    "mission_day": 1,
                    "input_type": "MEAL_LOG",
                    "reference_code": "FOOD-03",
                    "consumption_pct": 100,
                    "notes": "Beef Fajita Tortilla Pack"
                }]
            }),
        });

        app.poll_network();

        let ast1 = &app.snapshot.crew_summaries[0];
        assert_eq!(ast1.edge_seq_id, 500);
        assert!((ast1.spo2_pct - 87.5).abs() < 1e-6);
        assert!(ast1.daily_nutrition.sodium_mg_total > init_na + 600.0);
        assert!(ast1.alerts.health_danger);
        assert!(ast1.alerts.hazard_danger);

        // Auto incident + telemetry persisted into SQLite
        let latest = app.db.load_latest_telemetry("AST-01").unwrap().unwrap();
        assert_eq!(latest.edge_seq_id, 500);
        assert!(app.snapshot.unacked_incidents >= 2);
    }
}
