use crate::calc::{self, PanelAlertState, SafetyEvalInput};
use crate::db::{
    CrewInputHistoryRecord, CrewMember, Database, DatasetProvenance, FoodCatalogItem, HubLabReport,
    LanCrewInputInsert, LanTelemetryInsert, MissionEphemeris, NewSurgeonIncident,
    OsteoporosisDailyHistory, ReferenceEdaPoint, ReferenceRadiationPoint, ResearchLabObservation,
    SansDailyHistory, SleepDailySummary, SurgeonIncident,
};
use crate::net::{HubDownlinkMessage, HubLanServer, HubNetEvent, InboundPacket};
use crate::ui;
use crate::ui_components;
use eframe::egui;

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
    pub peer_addr: Option<String>,
    pub record_id: i64,
    pub mission_day: i64,
    pub edge_seq_id: i64,
    pub recorded_at_utc: String,
    pub received_at_utc: String,
    pub hr_bpm: f64,
    pub resting_hr_bpm: f64,
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
    pub daily_sodium_mg: f64,
    pub shock_index: f64,
    pub orthostatic_risk: String,
    pub sans_risk: String,
    pub skin_temp_c: f64,
    pub hub_distance_m: f64,
    pub heading_deg: f32,
    pub target_deg: f32,
    pub active_anomaly: Option<StressScenario>,
    pub alerts: PanelAlertState,
}

impl CrewSummarySnapshot {
    pub fn recompute_derived(&mut self) {
        self.shock_index = calc::calc_shock_index(self.hr_bpm, self.sbp_mmhg);
        self.orthostatic_risk =
            calc::calc_orthostatic_risk(self.hr_bpm, self.sbp_mmhg, self.g_load).to_string();
        let (_, sans_tier) = calc::calc_sans_risk(
            self.daily_sodium_mg,
            self.cabin_co2_ppm,
            &self.member.active_phase_code,
        );
        self.sans_risk = sans_tier.to_string();
        let comms_online = self.member.lan_status == "ONLINE";
        self.alerts = calc::evaluate_safety(&SafetyEvalInput {
            phase: &self.member.active_phase_code,
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
    pub lan_discovery_ready: bool,
    pub lan_local_ips: Vec<String>,
    pub lan_tcp_port: u16,
    pub lan_discovery_port: u16,
    pub lan_error: Option<String>,
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
    pub snapshot: HubUiSnapshot,
    pub db: Database,
    pub lan_server: HubLanServer,
}

impl AppState {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui_components::install_aerospace_fonts(&cc.egui_ctx);
        ui::apply_mandated_visuals(&cc.egui_ctx);

        let mut db =
            Database::open_default().unwrap_or_else(|_| Database::open_in_memory_fallback());
        let _ = db.seed_demo_data();

        let lan_server = HubLanServer::start("0.0.0.0:7878");
        let snapshot = Self::build_snapshot_from_db(&db, &lan_server, 0, "AWAITING LAN CLIENTS");
        let initial_phase = snapshot
            .crew_summaries
            .first()
            .map(|c| MissionPhase::from_code(&c.member.active_phase_code))
            .unwrap_or(MissionPhase::Orbit);

        lan_server.update_hub_metadata(
            initial_phase.label(),
            snapshot.distance_au,
            snapshot.one_way_light_min,
        );

        Self {
            screen: HubScreen::MainMenu,
            selected_crew_idx: 0,
            mission_phase: initial_phase,
            stress_gain: 100.0,
            apply_phase_to_all: true,
            snapshot,
            db,
            lan_server,
        }
    }

    fn build_snapshot_from_db(
        db: &Database,
        lan_server: &HubLanServer,
        packets_received: u64,
        last_packet_info: &str,
    ) -> HubUiSnapshot {
        let members = db.load_crew_members().unwrap_or_default();
        let peers = lan_server.online_crew_peers();
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
        let distance_au = ephem.as_ref().map(|e| e.distance_au).unwrap_or(1.42);
        let one_way_light_min = ephem
            .as_ref()
            .map(|e| e.one_way_light_min)
            .unwrap_or_else(|| calc::calc_one_way_light_min(distance_au));

        let mut crew_summaries = Vec::with_capacity(members.len());
        let headings = [38.0_f32, 112.0, 215.0, 304.0, 154.0, 268.0];
        let distances = [222.0_f64, 185.0, 340.0, 95.0, 148.0, 276.0];

        for (i, mut member) in members.into_iter().enumerate() {
            let peer_addr = peers.get(&member.astronaut_id).cloned();
            member.lan_status = if peer_addr.is_some() {
                "ONLINE".to_string()
            } else {
                "OFFLINE".to_string()
            };

            let latest = db
                .load_latest_telemetry(&member.astronaut_id)
                .ok()
                .flatten();

            let daily_sodium_mg = sans_history
                .iter()
                .filter(|s| s.astronaut_id == member.astronaut_id)
                .max_by_key(|s| s.mission_day)
                .map(|s| s.sodium_mg_total)
                .unwrap_or(1450.0);

            let record_id = latest
                .as_ref()
                .map(|t| t.record_id)
                .unwrap_or((i + 1) as i64);
            let mission_day = latest.as_ref().map(|t| t.mission_day).unwrap_or(1);
            let edge_seq_id = latest.as_ref().map(|t| t.edge_seq_id).unwrap_or(1);
            let recorded_at_utc = latest
                .as_ref()
                .map(|t| t.recorded_at_utc.clone())
                .unwrap_or_else(|| "2026-11-14T20:00:00Z".to_string());
            let received_at_utc = latest
                .as_ref()
                .map(|t| t.received_at_utc.clone())
                .unwrap_or_else(|| "2026-11-14T20:00:01Z".to_string());

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
                    calc::calc_sans_risk(daily_sodium_mg, cabin_co2_ppm, &member.active_phase_code)
                        .1
                        .to_string()
                });

            crew_summaries.push(CrewSummarySnapshot {
                member,
                peer_addr,
                record_id,
                mission_day,
                edge_seq_id,
                recorded_at_utc,
                received_at_utc,
                hr_bpm,
                resting_hr_bpm: 68.0,
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
                daily_sodium_mg,
                shock_index,
                orthostatic_risk,
                sans_risk,
                skin_temp_c: 34.2,
                hub_distance_m: distances[i % distances.len()],
                heading_deg: headings[i % headings.len()],
                target_deg: 0.0,
                active_anomaly: None,
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
            lan_discovery_ready: lan_server.discovery_ready(),
            lan_local_ips: lan_server.local_ipv4s().to_vec(),
            lan_tcp_port: lan_server.tcp_port(),
            lan_discovery_port: lan_server.discovery_port(),
            lan_error: lan_server.last_error(),
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
        let target_crew_id = if self.apply_phase_to_all {
            for crew in &mut self.snapshot.crew_summaries {
                crew.member.active_phase_code = phase_code.to_string();
                let _ = self
                    .db
                    .update_crew_phase(&crew.member.astronaut_id, phase_code);
            }
            None
        } else if let Some(crew) = self.snapshot.crew_summaries.get_mut(self.selected_crew_idx) {
            crew.member.active_phase_code = phase_code.to_string();
            let _ = self
                .db
                .update_crew_phase(&crew.member.astronaut_id, phase_code);
            Some(crew.member.astronaut_id.clone())
        } else {
            None
        };

        self.lan_server.update_hub_metadata(
            phase_code,
            self.snapshot.distance_au,
            self.snapshot.one_way_light_min,
        );
        self.lan_server
            .broadcast_downlink(HubDownlinkMessage::PhaseSync {
                phase: phase_code.to_string(),
                apply_to_all: self.apply_phase_to_all,
                target_crew_id,
            });
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
        let pkts = self.snapshot.packets_received;
        let last_info = self.snapshot.last_packet_info.clone();
        self.snapshot = Self::build_snapshot_from_db(&self.db, &self.lan_server, pkts, &last_info);
        if let Some(crew) = self.snapshot.crew_summaries.get(self.selected_crew_idx) {
            self.mission_phase = MissionPhase::from_code(&crew.member.active_phase_code);
        }
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

        let _ = self.db.insert_surgeon_incident(&NewSurgeonIncident {
            astronaut_id: &crew.member.astronaut_id,
            telemetry_record_id: Some(crew.record_id),
            created_at_utc: "2026-11-14T20:15:00Z",
            hazard_type: hazard,
            injury_details: Some(&details),
            medication_administered: Some(med),
            medical_notes: Some(&notes),
        });
        self.refresh_incidents_from_db();
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
        let (crew_id, next_seq, day) = self
            .snapshot
            .crew_summaries
            .get(self.selected_crew_idx)
            .map(|c| {
                (
                    c.member.astronaut_id.clone(),
                    c.edge_seq_id + 1,
                    c.mission_day,
                )
            })
            .unwrap_or_else(|| ("AST-01".to_string(), 2, 1));
        let pkt = InboundPacket {
            crew_id,
            packet_type: "TELEMETRY_BATCH_SYNC".to_string(),
            payload: serde_json::json!({
                "batch_id": 9999,
                "seq_id": next_seq,
                "mission_day": day,
                "active_phase": self.mission_phase.label(),
                "hr_bpm": 94.0,
                "sbp_mmhg": 121.0,
                "spo2_pct": 98.5,
                "timestamp_utc": "2026-11-14T20:20:00Z",
                "flushed_records": 1,
            }),
        };
        self.lan_server.inject_test_packet(pkt);
    }

    fn apply_telemetry_payload(
        crew: &mut CrewSummarySnapshot,
        payload: &serde_json::Value,
        peer_label: Option<&str>,
    ) {
        if let Some(label) = peer_label {
            crew.peer_addr = Some(label.to_string());
            crew.member.lan_status = "ONLINE".to_string();
        }
        if let Some(hr) = payload.get("hr_bpm").and_then(|v| v.as_f64()) {
            crew.hr_bpm = hr;
        }
        if let Some(sbp) = payload.get("sbp_mmhg").and_then(|v| v.as_f64()) {
            crew.sbp_mmhg = sbp;
        }
        if let Some(resp) = payload.get("resp_brpm").and_then(|v| v.as_f64()) {
            crew.resp_brpm = resp;
        }
        if let Some(spo2) = payload.get("spo2_pct").and_then(|v| v.as_f64()) {
            crew.spo2_pct = spo2;
        }
        if let Some(tis) = payload.get("tissue_o2_pct").and_then(|v| v.as_f64()) {
            crew.tissue_o2_pct = Some(tis);
        }
        if let Some(n2) = payload.get("blood_n2_idx").and_then(|v| v.as_f64()) {
            crew.blood_n2_idx = Some(n2);
        }
        if let Some(g) = payload.get("g_load").and_then(|v| v.as_f64()) {
            crew.g_load = g;
        }
        if let Some(imu) = payload.get("imu_activity_idx").and_then(|v| v.as_f64()) {
            crew.imu_activity_idx = imu;
        }
        if let Some(rad) = payload.get("radiation_usv_hr").and_then(|v| v.as_f64()) {
            crew.radiation_usv_hr = rad;
        }
        if let Some(co2) = payload.get("cabin_co2_ppm").and_then(|v| v.as_f64()) {
            crew.cabin_co2_ppm = co2;
        }
        if let Some(o2) = payload.get("cabin_o2_pct").and_then(|v| v.as_f64()) {
            crew.cabin_o2_pct = Some(o2);
        }
        if let Some(sp) = payload.get("suit_pressure_kpa").and_then(|v| v.as_f64()) {
            crew.suit_pressure_kpa = Some(sp);
        }
        if let Some(sh) = payload.get("suit_humidity_pct").and_then(|v| v.as_f64()) {
            crew.suit_humidity_pct = Some(sh);
        }
        if let Some(eva) = payload.get("eva_elapsed_sec").and_then(|v| v.as_i64()) {
            crew.eva_elapsed_sec = eva;
        }
        if let Some(temp) = payload.get("skin_temp_c").and_then(|v| v.as_f64()) {
            crew.skin_temp_c = temp;
        }
        if let Some(dist) = payload.get("hub_distance_m").and_then(|v| v.as_f64()) {
            crew.hub_distance_m = dist;
        }
        if let Some(hdg) = payload.get("heading_deg").and_then(|v| v.as_f64()) {
            crew.heading_deg = hdg as f32;
        }
        if let Some(day) = payload.get("mission_day").and_then(|v| v.as_i64()) {
            crew.mission_day = day;
        }
        if let Some(seq) = payload.get("seq_id").and_then(|v| v.as_i64()) {
            crew.edge_seq_id = crew.edge_seq_id.max(seq);
        } else {
            crew.edge_seq_id += 1;
        }
        if let Some(phase) = payload.get("active_phase").and_then(|v| v.as_str()) {
            crew.member.active_phase_code = phase.to_string();
        }
        if let Some(ts) = payload.get("timestamp_utc").and_then(|v| v.as_str()) {
            crew.member.last_heartbeat_utc = ts.to_string();
            crew.recorded_at_utc = ts.to_string();
            crew.received_at_utc = ts.to_string();
        }
    }

    fn persist_crew_input_value(
        &self,
        crew_id: &str,
        fallback_day: i64,
        val: &serde_json::Value,
    ) -> bool {
        let edge_input_id = val
            .get("edge_input_id")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(1)
            });
        let logged_at_utc = val
            .get("logged_at_utc")
            .and_then(|v| v.as_str())
            .unwrap_or("2026-11-14T20:20:00Z");
        let mission_day = val
            .get("mission_day")
            .and_then(|v| v.as_i64())
            .unwrap_or(fallback_day);
        let input_type = val
            .get("input_type")
            .and_then(|v| v.as_str())
            .unwrap_or("MEAL_LOG");
        let reference_code = val.get("reference_code").and_then(|v| v.as_str());
        let consumption_pct = val.get("consumption_pct").and_then(|v| v.as_i64());
        let notes = val.get("notes").and_then(|v| v.as_str());

        self.db
            .upsert_crew_input_from_lan(&LanCrewInputInsert {
                astronaut_id: crew_id,
                edge_input_id,
                logged_at_utc,
                mission_day,
                input_type,
                reference_code,
                consumption_pct,
                notes,
                received_at_utc: logged_at_utc,
            })
            .unwrap_or(false)
    }

    fn poll_network(&mut self) {
        self.snapshot.lan_ready = self.lan_server.is_ready();
        self.snapshot.lan_discovery_ready = self.lan_server.discovery_ready();
        self.snapshot.lan_error = self.lan_server.last_error();
        self.snapshot.connected_clients = self.lan_server.connected_clients();

        let mut changed = false;
        let mut refresh_inputs = false;

        while let Some(ev) = self.lan_server.try_recv_event() {
            match ev {
                HubNetEvent::ClientConnected {
                    peer,
                    crew_id,
                    callsign,
                } => {
                    let label = format!("{peer} ({callsign})");
                    if let Some(crew) = self
                        .snapshot
                        .crew_summaries
                        .iter_mut()
                        .find(|c| c.member.astronaut_id == crew_id)
                    {
                        crew.member.lan_status = "ONLINE".to_string();
                        crew.peer_addr = Some(label.clone());
                    }
                    let _ = self.db.update_crew_lan_status(&crew_id, "ONLINE", None);
                    self.snapshot.last_packet_info = format!("{crew_id} // CONNECTED ({label})");
                    changed = true;
                }
                HubNetEvent::ClientDisconnected { peer, crew_id } => {
                    if !self.lan_server.crew_has_active_session(&crew_id) {
                        if let Some(crew) = self
                            .snapshot
                            .crew_summaries
                            .iter_mut()
                            .find(|c| c.member.astronaut_id == crew_id)
                        {
                            crew.member.lan_status = "OFFLINE".to_string();
                            crew.peer_addr = None;
                        }
                        let _ = self.db.update_crew_lan_status(&crew_id, "OFFLINE", None);
                    }
                    self.snapshot.last_packet_info = format!("{crew_id} // DISCONNECTED ({peer})");
                    changed = true;
                }
                HubNetEvent::Packet { peer, packet: pkt } => {
                    self.snapshot.packets_received += 1;
                    let ts_label = pkt
                        .payload
                        .get("timestamp_utc")
                        .and_then(|v| v.as_str())
                        .unwrap_or("LIVE");
                    self.snapshot.last_packet_info =
                        format!("{} // {} @ {}", pkt.crew_id, pkt.packet_type, ts_label);

                    let peer_label = if peer.port() == 0 {
                        "LOCAL SIM".to_string()
                    } else {
                        peer.to_string()
                    };

                    match pkt.packet_type.as_str() {
                        "HELLO" | "HEARTBEAT" => {
                            if let Some(crew) = self
                                .snapshot
                                .crew_summaries
                                .iter_mut()
                                .find(|c| c.member.astronaut_id == pkt.crew_id)
                            {
                                crew.member.lan_status = "ONLINE".to_string();
                                if crew.peer_addr.is_none() {
                                    crew.peer_addr = Some(peer_label);
                                }
                            }
                            let _ = self.db.update_crew_lan_status(&pkt.crew_id, "ONLINE", None);
                        }
                        "CREW_INPUT" => {
                            let fallback_day = self
                                .snapshot
                                .crew_summaries
                                .iter()
                                .find(|c| c.member.astronaut_id == pkt.crew_id)
                                .map(|c| c.mission_day)
                                .unwrap_or(1);
                            if self.persist_crew_input_value(
                                &pkt.crew_id,
                                fallback_day,
                                &pkt.payload,
                            ) {
                                refresh_inputs = true;
                            }
                            if let Some(batch_id) =
                                pkt.payload.get("batch_id").and_then(|v| v.as_u64())
                            {
                                self.lan_server.send_to_crew(
                                    &pkt.crew_id,
                                    HubDownlinkMessage::SyncAck {
                                        batch_id,
                                        synced_records: 1,
                                    },
                                );
                            }
                        }
                        _ => {
                            // TELEMETRY_LIVE, TELEMETRY_BATCH_SYNC, TELEMETRY_SYNC
                            let mut persisted_crew: Option<CrewSummarySnapshot> = None;

                            if let Some(crew) = self
                                .snapshot
                                .crew_summaries
                                .iter_mut()
                                .find(|c| c.member.astronaut_id == pkt.crew_id)
                            {
                                Self::apply_telemetry_payload(
                                    crew,
                                    &pkt.payload,
                                    Some(&peer_label),
                                );
                                if pkt.packet_type == "TELEMETRY_BATCH_SYNC"
                                    || pkt.packet_type == "TELEMETRY_SYNC"
                                {
                                    persisted_crew = Some(crew.clone());
                                }
                            }

                            if let Some(crew) = persisted_crew {
                                let _ = self.db.upsert_telemetry_from_lan(&LanTelemetryInsert {
                                    astronaut_id: &crew.member.astronaut_id,
                                    phase_code: &crew.member.active_phase_code,
                                    mission_day: crew.mission_day,
                                    edge_seq_id: crew.edge_seq_id,
                                    recorded_at_utc: &crew.recorded_at_utc,
                                    received_at_utc: &crew.received_at_utc,
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
                                    daily_sodium_mg: crew.daily_sodium_mg,
                                });
                            }

                            let mut synced_n = 1_usize;
                            if let Some(arr) = pkt.payload.get("inputs").and_then(|v| v.as_array())
                            {
                                let fallback_day = self
                                    .snapshot
                                    .crew_summaries
                                    .iter()
                                    .find(|c| c.member.astronaut_id == pkt.crew_id)
                                    .map(|c| c.mission_day)
                                    .unwrap_or(1);
                                for item in arr {
                                    if self.persist_crew_input_value(
                                        &pkt.crew_id,
                                        fallback_day,
                                        item,
                                    ) {
                                        refresh_inputs = true;
                                    }
                                    synced_n += 1;
                                }
                            }

                            if pkt.packet_type == "TELEMETRY_BATCH_SYNC" {
                                let batch_id = pkt
                                    .payload
                                    .get("batch_id")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1);
                                self.lan_server.send_to_crew(
                                    &pkt.crew_id,
                                    HubDownlinkMessage::SyncAck {
                                        batch_id,
                                        synced_records: synced_n,
                                    },
                                );
                            }
                        }
                    }
                    changed = true;
                }
            }
        }

        if refresh_inputs {
            self.snapshot.crew_inputs = self.db.load_crew_input_history().unwrap_or_default();
        }
        if changed {
            self.snapshot.recompute_alerts();
        }
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_network();
        ui::render(ui, self);
    }
}
