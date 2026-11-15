use crate::calc::{self, PanelAlertState, SafetyEvalInput};
use crate::db::{
    CrewInputHistoryRecord, CrewMember, Database, DatasetProvenance, FoodCatalogItem, HubLabReport,
    MissionEphemeris, NewSurgeonIncident, OsteoporosisDailyHistory, ReferenceEdaPoint,
    ReferenceRadiationPoint, ResearchLabObservation, SansDailyHistory, SleepDailySummary,
    SurgeonIncident,
};
use crate::net::{HubLanServer, InboundPacket};
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
            HubModule::CrewVitalsMonitor => "EKİP BİYOMETRİSİ",
            HubModule::PhaseAndStressControl => "FAZ VE STRES",
            HubModule::MedicalAndIncidentLog => "SAĞLIK KAYITLARI",
            HubModule::FoodCatalogAndSync => "BESLENME VE EŞİTLEME",
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
    pub alerts: PanelAlertState,
}

impl CrewSummarySnapshot {
    pub fn recompute_derived(&mut self) {
        self.shock_index = calc::calc_shock_index(self.hr_bpm, self.sbp_mmhg);
        self.orthostatic_risk =
            calc::calc_orthostatic_risk(self.hr_bpm, self.sbp_mmhg, self.g_load).to_string();
        let (_, sans_tier) =
            calc::calc_sans_risk(1450.0, self.cabin_co2_ppm, &self.member.active_phase_code);
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
                    calc::calc_sans_risk(1140.0, cabin_co2_ppm, &member.active_phase_code)
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
                "timestamp_utc": "2026-11-14T20:20:00Z"
            }),
        };
        self.lan_server.inject_test_packet(pkt);
    }

    fn poll_network(&mut self) {
        self.snapshot.lan_ready = self.lan_server.is_ready();
        self.snapshot.connected_clients = self.lan_server.connected_clients();
        let mut changed = false;
        while let Some(pkt) = self.lan_server.try_recv_packet() {
            self.snapshot.packets_received += 1;
            self.snapshot.last_packet_info = format!(
                "{} // {} @ 2026-11-14T20:20:00Z",
                pkt.crew_id, pkt.packet_type
            );
            if let Some(crew) = self
                .snapshot
                .crew_summaries
                .iter_mut()
                .find(|c| c.member.astronaut_id == pkt.crew_id)
            {
                if let Some(hr) = pkt.payload.get("hr_bpm").and_then(|v| v.as_f64()) {
                    crew.hr_bpm = hr;
                }
                if let Some(sbp) = pkt.payload.get("sbp_mmhg").and_then(|v| v.as_f64()) {
                    crew.sbp_mmhg = sbp;
                }
                if let Some(spo2) = pkt.payload.get("spo2_pct").and_then(|v| v.as_f64()) {
                    crew.spo2_pct = spo2;
                }
                if let Some(ts) = pkt.payload.get("timestamp_utc").and_then(|v| v.as_str()) {
                    crew.member.last_heartbeat_utc = ts.to_string();
                    crew.received_at_utc = ts.to_string();
                }
                crew.edge_seq_id += 1;
            }
            changed = true;
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
