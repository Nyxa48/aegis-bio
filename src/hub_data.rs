use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionPhase {
    LaunchAscent,
    OrbitIva,
    EvaSpacewalk,
    ReentryLanding,
}

impl MissionPhase {
    pub const ALL: [MissionPhase; 4] = [
        MissionPhase::LaunchAscent,
        MissionPhase::OrbitIva,
        MissionPhase::EvaSpacewalk,
        MissionPhase::ReentryLanding,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::LaunchAscent => "LAUNCH_ASCENT",
            Self::OrbitIva => "ORBIT_IVA",
            Self::EvaSpacewalk => "EVA_SPACEWALK",
            Self::ReentryLanding => "REENTRY_LANDING",
        }
    }

    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "LAUNCH_ASCENT" => Some(Self::LaunchAscent),
            "ORBIT_IVA" => Some(Self::OrbitIva),
            "EVA_SPACEWALK" => Some(Self::EvaSpacewalk),
            "REENTRY_LANDING" => Some(Self::ReentryLanding),
            _ => None,
        }
    }

    pub fn hr_limit(self) -> f32 {
        match self {
            Self::EvaSpacewalk | Self::LaunchAscent => 160.0,
            Self::OrbitIva | Self::ReentryLanding => 140.0,
        }
    }

    pub fn sodium_cap_mg(self) -> f32 {
        match self {
            Self::ReentryLanding => 3200.0, // Re-entry fluid/salt loading protocol
            _ => 2500.0,                    // Microgravity cap (<2500 mg/day)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityFlag {
    Ok,
    Missing,
    Review,
}

impl QualityFlag {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Missing => "missing",
            Self::Review => "review",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Critical,
    Warning,
}

#[derive(Debug, Clone)]
pub struct AlertItem {
    pub level: AlertLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstronautProfile {
    pub subject_id: String,
    pub name: String,
    pub role: String,
    pub mission_phase: MissionPhase,
    pub bpm: f32,
    pub oxygen: f32,
    pub pressure: f32,
    pub nitrogen: f32,
    pub hemoglobin: f32,
    pub temperature: f32,
    pub panic: bool,
    pub daily_kcal_target: f32,
    pub daily_protein_target: f32,
    pub daily_sodium_cap_mg: f32,
    pub kcal_consumed: f32,
    pub protein_consumed: f32,
    pub sodium_consumed_mg: f32,
    #[serde(skip)]
    pub hr_history: VecDeque<[f64; 2]>,
}

impl AstronautProfile {
    pub fn push_hr_sample(&mut self, t: f64) {
        self.hr_history.push_back([t, self.bpm as f64]);
        while self.hr_history.len() > 180 {
            self.hr_history.pop_front();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementRecord {
    pub subject_id: String,
    pub sample_id: String,
    pub metric: String,
    pub unit: String,
    pub value: Option<f64>,
    pub event: String,
    pub day_offset: i32,
    pub source: String,
    pub source_file: String,
    pub quality_flag: QualityFlag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiationReading {
    pub timestamp_raw: String,
    pub elapsed_min: f64,
    pub instrument_id: String,
    pub absorbed_dose_rate: f64,
    pub unit: String,
    pub source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub serving_size: String,
    pub kcal: f32,
    pub protein_g: f32,
    pub sodium_mg: f32,
    pub stock_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealLogEntry {
    pub event_id: String,
    pub subject_id: String,
    pub food_name: String,
    pub consumed_ratio: f32,
    pub kcal_added: f32,
    pub protein_added: f32,
    pub sodium_added: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryPacket {
    pub event_id: Option<String>,
    pub subject_id: String,
    pub bpm: f32,
    pub oxygen: Option<f32>,
    pub pressure: Option<f32>,
    pub nitrogen: Option<f32>,
    pub hemoglobin: Option<f32>,
    pub temperature: Option<f32>,
    pub panic: Option<bool>,
    pub mission_phase: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RationCandidate {
    pub id: String,
    pub name: String,
    pub serving_size: String,
    pub kcal: f32,
    pub protein_g: f32,
    pub sodium_mg: f32,
    pub stock_count: u32,
    pub fit_score: f32,
}

#[derive(Debug, Clone)]
pub struct RationAdvice {
    pub remaining_kcal: f32,
    pub remaining_protein: f32,
    pub remaining_sodium_mg: f32,
    pub recommendations: Vec<RationCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubState {
    pub crew: Vec<AstronautProfile>,
    pub pantry: Vec<FoodItem>,
    pub measurements: Vec<MeasurementRecord>,
    pub radiation_readings: Vec<RadiationReading>,
    pub meal_logs: Vec<MealLogEntry>,
    pub seen_event_ids: HashSet<String>,
    #[serde(skip)]
    pub audit_logs: Vec<String>,
    #[serde(skip)]
    pub packets_received: u64,
    #[serde(skip)]
    pub sim_time: f64,
}

impl HubState {
    pub fn load_or_seed(state_path: &Path, nasa_dir: Option<&Path>) -> Self {
        let mut state = if let Ok(raw) = fs::read_to_string(state_path) {
            serde_json::from_str::<HubState>(&raw).unwrap_or_else(|_| Self::seeded())
        } else {
            Self::seeded()
        };

        if let Some(dir) = nasa_dir {
            state.try_load_radlab_csv(&dir.join("RadLab_2013_01_01.csv"));
        }

        for a in &mut state.crew {
            if a.hr_history.is_empty() {
                a.push_hr_sample(0.0);
            }
        }
        state.push_audit("[SİSTEM] Rust Mission Hub Terminal başlatıldı (HTTPS yok, yerel LAN TCP aktif).");
        state
    }

    pub fn save(&self, state_path: &Path) {
        if let Some(parent) = state_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(state_path, json);
        }
    }

    pub fn seeded() -> Self {
        let crew = vec![
            AstronautProfile {
                subject_id: "C001".into(),
                name: "Cmdr. Alex Mercer".into(),
                role: "Mission Commander".into(),
                mission_phase: MissionPhase::OrbitIva,
                bpm: 74.0,
                oxygen: 98.4,
                pressure: 120.0,
                nitrogen: 78.0,
                hemoglobin: 14.9,
                temperature: 21.8,
                panic: false,
                daily_kcal_target: 2600.0,
                daily_protein_target: 110.0,
                daily_sodium_cap_mg: 2300.0,
                kcal_consumed: 1150.0,
                protein_consumed: 48.0,
                sodium_consumed_mg: 980.0,
                hr_history: VecDeque::new(),
            },
            AstronautProfile {
                subject_id: "C002".into(),
                name: "Dr. Elena Rostova".into(),
                role: "Flight Surgeon / Payload".into(),
                mission_phase: MissionPhase::OrbitIva,
                bpm: 68.0,
                oxygen: 99.1,
                pressure: 118.0,
                nitrogen: 78.2,
                hemoglobin: 13.8,
                temperature: 22.1,
                panic: false,
                daily_kcal_target: 2200.0,
                daily_protein_target: 95.0,
                daily_sodium_cap_mg: 2300.0,
                kcal_consumed: 920.0,
                protein_consumed: 41.0,
                sodium_consumed_mg: 810.0,
                hr_history: VecDeque::new(),
            },
            AstronautProfile {
                subject_id: "C003".into(),
                name: "Maj. Kenji Sato".into(),
                role: "EVA Specialist".into(),
                mission_phase: MissionPhase::EvaSpacewalk,
                bpm: 128.0,
                oxygen: 96.5,
                pressure: 132.0,
                nitrogen: 76.5,
                hemoglobin: 15.1,
                temperature: 23.4,
                panic: false,
                daily_kcal_target: 3100.0,
                daily_protein_target: 130.0,
                daily_sodium_cap_mg: 2500.0,
                kcal_consumed: 1480.0,
                protein_consumed: 62.0,
                sodium_consumed_mg: 1290.0,
                hr_history: VecDeque::new(),
            },
            AstronautProfile {
                subject_id: "C004".into(),
                name: "Lt. Maya Lin".into(),
                role: "Systems & Bio Engineer".into(),
                mission_phase: MissionPhase::ReentryLanding,
                bpm: 104.0,
                oxygen: 97.8,
                pressure: 124.0,
                nitrogen: 77.9,
                hemoglobin: 14.2,
                temperature: 22.6,
                panic: false,
                daily_kcal_target: 2400.0,
                daily_protein_target: 100.0,
                daily_sodium_cap_mg: 3200.0,
                kcal_consumed: 1050.0,
                protein_consumed: 44.0,
                sodium_consumed_mg: 1650.0,
                hr_history: VecDeque::new(),
            },
        ];

        let pantry = vec![
            FoodItem {
                id: "ISS-01".into(),
                name: "Thermostabilized Beef Stew".into(),
                category: "Entree".into(),
                serving_size: "220g pouch".into(),
                kcal: 340.0,
                protein_g: 26.0,
                sodium_mg: 480.0,
                stock_count: 14,
            },
            FoodItem {
                id: "ISS-02".into(),
                name: "Rehydratable Teriyaki Chicken".into(),
                category: "Entree".into(),
                serving_size: "180g pouch".into(),
                kcal: 290.0,
                protein_g: 28.0,
                sodium_mg: 520.0,
                stock_count: 11,
            },
            FoodItem {
                id: "ISS-03".into(),
                name: "Space Tofu & Quinoa Bowl".into(),
                category: "Entree".into(),
                serving_size: "200g pouch".into(),
                kcal: 310.0,
                protein_g: 22.0,
                sodium_mg: 310.0,
                stock_count: 9,
            },
            FoodItem {
                id: "ISS-04".into(),
                name: "Freeze-Dried Scrambled Eggs".into(),
                category: "Breakfast".into(),
                serving_size: "110g pouch".into(),
                kcal: 240.0,
                protein_g: 19.0,
                sodium_mg: 390.0,
                stock_count: 16,
            },
            FoodItem {
                id: "ISS-05".into(),
                name: "Almond & Whey Recovery Bar".into(),
                category: "Snack".into(),
                serving_size: "70g bar".into(),
                kcal: 280.0,
                protein_g: 21.0,
                sodium_mg: 160.0,
                stock_count: 22,
            },
            FoodItem {
                id: "ISS-06".into(),
                name: "Low-Sodium Lentil Soup".into(),
                category: "Side".into(),
                serving_size: "210g pouch".into(),
                kcal: 210.0,
                protein_g: 14.0,
                sodium_mg: 190.0,
                stock_count: 12,
            },
            FoodItem {
                id: "ISS-07".into(),
                name: "Electrolyte NaCl Hydration Pack".into(),
                category: "Countermeasure".into(),
                serving_size: "250ml + 1g NaCl".into(),
                kcal: 45.0,
                protein_g: 0.0,
                sodium_mg: 393.0,
                stock_count: 30,
            },
            FoodItem {
                id: "ISS-08".into(),
                name: "Freeze-Dried Strawberries & Granola".into(),
                category: "Breakfast".into(),
                serving_size: "95g pouch".into(),
                kcal: 260.0,
                protein_g: 8.0,
                sodium_mg: 95.0,
                stock_count: 15,
            },
        ];

        let measurements = vec![
            meas("C001", "C001_whole-blood_L-3_cbc", "absolute_basophils", "cells/uL", Some(49.0), "launch", -3, "OSD-569", "CBC_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C001", "C001_whole-blood_L-3_cbc", "white_blood_cells", "10^3/uL", Some(6.2), "launch", -3, "OSD-569", "CBC_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C001", "C001_whole-blood_R+1_cbc", "white_blood_cells", "10^3/uL", Some(9.8), "return", 1, "OSD-569", "CBC_TRANSFORMED.csv (demo)", QualityFlag::Review),
            meas("C001", "C001_serum_L-3", "albumin", "g/dL", Some(4.9), "launch", -3, "OSD-575", "CMP_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C001", "C001_serum_R+1", "albumin", "g/dL", Some(4.4), "return", 1, "OSD-575", "CMP_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C002", "C002_serum_L-3", "albumin", "g/dL", Some(4.6), "launch", -3, "OSD-575", "CMP_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C002", "C002_whole-blood_L-3_cbc", "hemoglobin", "g/dL", Some(13.8), "launch", -3, "OSD-569", "CBC_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C003", "C003_whole-blood_L-3_cbc", "hemoglobin", "g/dL", Some(15.1), "launch", -3, "OSD-569", "CBC_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C003", "C003_serum_R+1", "potassium", "mmol/L", None, "return", 1, "OSD-575", "CMP_TRANSFORMED.csv (demo)", QualityFlag::Missing),
            meas("C004", "C004_serum_L-3", "sodium", "mmol/L", Some(141.0), "launch", -3, "OSD-575", "CMP_TRANSFORMED.csv (demo)", QualityFlag::Ok),
            meas("C004", "C004_serum_R+1", "sodium", "mmol/L", Some(138.0), "return", 1, "OSD-575", "CMP_TRANSFORMED.csv (demo)", QualityFlag::Ok),
        ];

        let radiation_readings = vec![
            rad("2013-01-01T00:00:13", 0.2, 3.5311, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:01:43", 1.7, 4.1200, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:03:13", 3.2, 12.8400, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:07:13", 7.2, 148.6200, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:08:43", 8.7, 369.1124, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:11:03", 11.0, 86.4100, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:12:33", 12.5, 5.9100, "RadLab_2013_01_01.csv (demo)"),
            rad("2013-01-01T00:14:03", 14.0, 0.8400, "RadLab_2013_01_01.csv (demo)"),
        ];

        Self {
            crew,
            pantry,
            measurements,
            radiation_readings,
            meal_logs: Vec::new(),
            seen_event_ids: HashSet::new(),
            audit_logs: Vec::new(),
            packets_received: 0,
            sim_time: 0.0,
        }
    }

    pub fn try_load_radlab_csv(&mut self, path: &Path) {
        let Ok(raw) = fs::read_to_string(path) else {
            return;
        };
        let cleaned = raw.trim_start_matches('\u{feff}');
        let mut lines = cleaned.lines();
        let Some(header) = lines.next() else {
            return;
        };
        let cols: Vec<&str> = header.split(',').map(str::trim).collect();
        let Some(ts_idx) = cols.iter().position(|&c| c == "timestamp") else {
            return;
        };
        let Some(dose_idx) = cols.iter().position(|&c| c == "absorbed_dose_rate") else {
            return;
        };

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("RadLab_2013_01_01.csv")
            .to_string();
        let mut loaded = Vec::new();
        for (i, line) in lines.enumerate() {
            let parts: Vec<&str> = line.split(',').map(str::trim).collect();
            if parts.len() <= ts_idx.max(dose_idx) {
                continue;
            }
            if let Ok(rate) = parts[dose_idx].parse::<f64>() {
                loaded.push(rad(parts[ts_idx], i as f64 * 1.5, rate, &file_name));
            }
        }
        if !loaded.is_empty() {
            self.radiation_readings = loaded;
            self.push_audit(format!(
                "[VERİ] {} dosyasından {} radyasyon kaydı yüklendi.",
                file_name,
                self.radiation_readings.len()
            ));
        }
    }

    pub fn push_audit(&mut self, msg: impl Into<String>) {
        self.audit_logs.push(msg.into());
        if self.audit_logs.len() > 150 {
            self.audit_logs.remove(0);
        }
    }

    pub fn apply_telemetry(&mut self, pkt: TelemetryPacket) -> &'static str {
        if let Some(ref eid) = pkt.event_id {
            if !self.seen_event_ids.insert(eid.clone()) {
                self.push_audit(format!("[DTN] Tekrarlanan event_id reddedildi: {eid}"));
                return "duplicate_ignored";
            }
        }

        let Some(astro) = self.crew.iter_mut().find(|a| a.subject_id == pkt.subject_id) else {
            self.push_audit(format!("[UYARI] Bilinmeyen astronot ID: {}", pkt.subject_id));
            return "unknown_subject";
        };

        astro.bpm = pkt.bpm.clamp(30.0, 220.0);
        if let Some(o2) = pkt.oxygen {
            astro.oxygen = o2.clamp(0.0, 100.0);
        }
        if let Some(p) = pkt.pressure {
            astro.pressure = p;
        }
        if let Some(n2) = pkt.nitrogen {
            astro.nitrogen = n2;
        }
        if let Some(hb) = pkt.hemoglobin {
            astro.hemoglobin = hb;
        }
        if let Some(temp) = pkt.temperature {
            astro.temperature = temp;
        }
        if let Some(panic) = pkt.panic {
            astro.panic = panic;
        }
        if let Some(ref phase_str) = pkt.mission_phase {
            if let Some(phase) = MissionPhase::from_str_lossy(phase_str) {
                astro.mission_phase = phase;
                astro.daily_sodium_cap_mg = phase.sodium_cap_mg();
            }
        }
        astro.push_hr_sample(self.sim_time);
        self.packets_received += 1;
        "accepted"
    }

    pub fn log_meal(
        &mut self,
        event_id: &str,
        subject_id: &str,
        food_id: &str,
        consumed_ratio: f32,
    ) -> Result<String, String> {
        if !self.seen_event_ids.insert(event_id.to_string()) {
            return Ok("duplicate_ignored".into());
        }

        let ratio = consumed_ratio.clamp(0.0, 1.0);
        let food_idx = self
            .pantry
            .iter()
            .position(|f| f.id == food_id)
            .ok_or_else(|| format!("Paket bulunamadı: {food_id}"))?;
        if self.pantry[food_idx].stock_count == 0 {
            self.seen_event_ids.remove(event_id);
            return Err(format!("Stok tükendi: {}", self.pantry[food_idx].name));
        }

        let astro_idx = self
            .crew
            .iter()
            .position(|a| a.subject_id == subject_id)
            .ok_or_else(|| format!("Astronot bulunamadı: {subject_id}"))?;

        self.pantry[food_idx].stock_count -= 1;
        let food = &self.pantry[food_idx];
        let kcal_added = (food.kcal * ratio * 10.0).round() / 10.0;
        let protein_added = (food.protein_g * ratio * 10.0).round() / 10.0;
        let sodium_added = (food.sodium_mg * ratio * 10.0).round() / 10.0;

        let astro = &mut self.crew[astro_idx];
        astro.kcal_consumed += kcal_added;
        astro.protein_consumed += protein_added;
        astro.sodium_consumed_mg += sodium_added;

        self.meal_logs.push(MealLogEntry {
            event_id: event_id.to_string(),
            subject_id: subject_id.to_string(),
            food_name: food.name.clone(),
            consumed_ratio: ratio,
            kcal_added,
            protein_added,
            sodium_added,
        });
        self.push_audit(format!(
            "[BESİN] {} için {} (%{:.0}) kaydedildi (+{:.0} kcal).",
            subject_id,
            food.name,
            ratio * 100.0,
            kcal_added
        ));
        Ok("logged".into())
    }

    pub fn recommend_rations(&self, subject_id: &str) -> RationAdvice {
        let Some(astro) = self.crew.iter().find(|a| a.subject_id == subject_id) else {
            return RationAdvice {
                remaining_kcal: 0.0,
                remaining_protein: 0.0,
                remaining_sodium_mg: 0.0,
                recommendations: Vec::new(),
            };
        };

        let rem_kcal = (astro.daily_kcal_target - astro.kcal_consumed).max(0.0);
        let rem_protein = (astro.daily_protein_target - astro.protein_consumed).max(0.0);
        let rem_sodium = (astro.daily_sodium_cap_mg - astro.sodium_consumed_mg).max(0.0);

        let mut recs: Vec<RationCandidate> = self
            .pantry
            .iter()
            .filter(|f| f.stock_count > 0 && f.sodium_mg <= rem_sodium)
            .map(|f| {
                let mut score = 0.0_f32;
                if rem_kcal > 0.0 {
                    score += f.kcal.min(rem_kcal) / rem_kcal.max(1.0);
                }
                if rem_protein > 0.0 {
                    score += 1.5 * (f.protein_g.min(rem_protein) / rem_protein.max(1.0));
                }
                RationCandidate {
                    id: f.id.clone(),
                    name: f.name.clone(),
                    serving_size: f.serving_size.clone(),
                    kcal: f.kcal,
                    protein_g: f.protein_g,
                    sodium_mg: f.sodium_mg,
                    stock_count: f.stock_count,
                    fit_score: score,
                }
            })
            .collect();

        recs.sort_by(|a, b| b.fit_score.total_cmp(&a.fit_score));
        recs.truncate(4);

        RationAdvice {
            remaining_kcal: rem_kcal,
            remaining_protein: rem_protein,
            remaining_sodium_mg: rem_sodium,
            recommendations: recs,
        }
    }

    pub fn measurements_for(&self, subject_id: &str) -> Vec<&MeasurementRecord> {
        let mut list: Vec<&MeasurementRecord> = self
            .measurements
            .iter()
            .filter(|m| m.subject_id == subject_id)
            .collect();
        list.sort_by(|a, b| {
            let ea = if a.event == "launch" { 0 } else { 1 };
            let eb = if b.event == "launch" { 0 } else { 1 };
            ea.cmp(&eb)
                .then(a.day_offset.cmp(&b.day_offset))
                .then(a.metric.cmp(&b.metric))
        });
        list
    }
}

pub fn evaluate_alerts(astro: &AstronautProfile) -> Vec<AlertItem> {
    let mut alerts = Vec::new();
    if astro.panic {
        alerts.push(AlertItem {
            level: AlertLevel::Critical,
            message: "PANİK ALARMI AKTİF!".into(),
        });
    }
    if astro.oxygen < 90.0 {
        alerts.push(AlertItem {
            level: AlertLevel::Critical,
            message: format!("DÜŞÜK OKSİJEN (%{:.1})", astro.oxygen),
        });
    }
    let hr_limit = astro.mission_phase.hr_limit();
    if astro.bpm > hr_limit {
        alerts.push(AlertItem {
            level: AlertLevel::Warning,
            message: format!("YÜKSEK NABIZ (>{:.0} BPM)", hr_limit),
        });
    } else if astro.bpm < 45.0 {
        alerts.push(AlertItem {
            level: AlertLevel::Warning,
            message: "DÜŞÜK NABIZ (<45 BPM)".into(),
        });
    }
    alerts
}

#[allow(clippy::too_many_arguments)]
fn meas(
    subject_id: &str,
    sample_id: &str,
    metric: &str,
    unit: &str,
    value: Option<f64>,
    event: &str,
    day_offset: i32,
    source: &str,
    source_file: &str,
    quality_flag: QualityFlag,
) -> MeasurementRecord {
    MeasurementRecord {
        subject_id: subject_id.into(),
        sample_id: sample_id.into(),
        metric: metric.into(),
        unit: unit.into(),
        value,
        event: event.into(),
        day_offset,
        source: source.into(),
        source_file: source_file.into(),
        quality_flag,
    }
}

fn rad(timestamp_raw: &str, elapsed_min: f64, absorbed_dose_rate: f64, source_file: &str) -> RadiationReading {
    RadiationReading {
        timestamp_raw: timestamp_raw.into(),
        elapsed_min,
        instrument_id: "Liulin-5-D3".into(),
        absorbed_dose_rate,
        unit: "uGy/saat".into(),
        source_file: source_file.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_crew_has_live_bpm_and_preserves_null_measurement() {
        let state = HubState::seeded();
        assert_eq!(state.crew.len(), 4);
        assert!(state.crew.iter().all(|c| c.bpm > 0.0));

        let c003_meas = state.measurements_for("C003");
        let missing = c003_meas.iter().find(|m| m.quality_flag == QualityFlag::Missing);
        assert!(missing.is_some());
        assert_eq!(missing.unwrap().value, None);
    }

    #[test]
    fn meal_logging_is_idempotent_and_deducts_stock() {
        let mut state = HubState::seeded();
        let initial_stock = state.pantry[0].stock_count;
        let initial_kcal = state.crew[0].kcal_consumed;

        let res1 = state.log_meal("meal-1", "C001", "ISS-01", 0.5).unwrap();
        assert_eq!(res1, "logged");
        assert_eq!(state.pantry[0].stock_count, initial_stock - 1);
        assert!((state.crew[0].kcal_consumed - (initial_kcal + 170.0)).abs() < 0.01);

        let res2 = state.log_meal("meal-1", "C001", "ISS-01", 0.5).unwrap();
        assert_eq!(res2, "duplicate_ignored");
        assert_eq!(state.pantry[0].stock_count, initial_stock - 1);
    }

    #[test]
    fn telemetry_packet_updates_bpm_idempotently() {
        let mut state = HubState::seeded();
        let pkt = TelemetryPacket {
            event_id: Some("evt-100".into()),
            subject_id: "C002".into(),
            bpm: 118.0,
            oxygen: Some(97.0),
            pressure: None,
            nitrogen: None,
            hemoglobin: None,
            temperature: None,
            panic: Some(false),
            mission_phase: None,
        };
        assert_eq!(state.apply_telemetry(pkt.clone()), "accepted");
        assert_eq!(state.crew[1].bpm, 118.0);
        assert_eq!(state.apply_telemetry(pkt), "duplicate_ignored");
    }

    #[test]
    fn ration_solver_respects_sodium_cap() {
        let mut state = HubState::seeded();
        state.crew[0].sodium_consumed_mg = 2200.0; // cap is 2300.0 -> max 100mg left
        let advice = state.recommend_rations("C001");
        assert!(advice.recommendations.iter().all(|r| r.sodium_mg <= 100.0));
    }
}
