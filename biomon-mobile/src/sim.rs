use crate::app::UiSnapshot;
use crate::calc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimScenario {
    Nominal,
    Exercise,
    SleepRest,
    OrthostaticStress,
    Hypoxia,
    SolarFlare,
}

impl SimScenario {
    pub fn all() -> &'static [SimScenario] {
        &[
            SimScenario::Nominal,
            SimScenario::Exercise,
            SimScenario::SleepRest,
            SimScenario::OrthostaticStress,
            SimScenario::Hypoxia,
            SimScenario::SolarFlare,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SimScenario::Nominal => "NOMINAL",
            SimScenario::Exercise => "EXERCISE",
            SimScenario::SleepRest => "SLEEP/REST",
            SimScenario::OrthostaticStress => "ORTHO STRESS",
            SimScenario::Hypoxia => "HYPOXIA",
            SimScenario::SolarFlare => "SOLAR FLARE",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SimScenario::Nominal => SimScenario::Exercise,
            SimScenario::Exercise => SimScenario::SleepRest,
            SimScenario::SleepRest => SimScenario::OrthostaticStress,
            SimScenario::OrthostaticStress => SimScenario::Hypoxia,
            SimScenario::Hypoxia => SimScenario::SolarFlare,
            SimScenario::SolarFlare => SimScenario::Nominal,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SleepEpochEmission {
    pub activity_count: f64,
    pub hr_bpm_avg: f64,
    pub sleep_state: String,
}

#[derive(Debug, Clone, Default)]
pub struct SimTickOutput {
    pub sleep_epoch: Option<SleepEpochEmission>,
}

#[derive(Debug, Clone)]
pub struct BioSimulator {
    pub enabled: bool,
    pub scenario: SimScenario,
    tick_count: u64,
    epoch_ticks: u32,
    epoch_hr_sum: f64,
    epoch_activity_sum: f64,
    tissue_n2_kpa: f64,
}

impl BioSimulator {
    pub fn new(initial_blood_n2_idx: Option<f64>, initial_suit_pressure_kpa: Option<f64>) -> Self {
        let suit_p = initial_suit_pressure_kpa.unwrap_or(29.6).max(1.0);
        let tissue_n2_kpa = initial_blood_n2_idx.unwrap_or(1.08) * suit_p;
        Self {
            enabled: true,
            scenario: SimScenario::Nominal,
            tick_count: 0,
            epoch_ticks: 0,
            epoch_hr_sum: 0.0,
            epoch_activity_sum: 0.0,
            tissue_n2_kpa,
        }
    }

    /// Re-synchronizes internal dissolved nitrogen pressure after an external prebreathe washout step.
    pub fn sync_tissue_n2_from_snapshot(
        &mut self,
        blood_n2_idx: Option<f64>,
        suit_pressure_kpa: Option<f64>,
    ) {
        let suit_p = suit_pressure_kpa.unwrap_or(29.6).max(1.0);
        if let Some(r) = blood_n2_idx {
            self.tissue_n2_kpa = r.max(0.0) * suit_p;
        }
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Advances physiology by 1 second (1 Hz) smoothly toward the active phase + scenario corridor,
    /// updates Haldane dissolved N2 washout, refreshes derived Keytel/sweat/safety fields, and emits a
    /// 60-second actigraphy sleep epoch every 60 ticks.
    pub fn tick_1hz(&mut self, snap: &mut UiSnapshot, hydration_deficit_ml: f64) -> SimTickOutput {
        self.tick_count = self.tick_count.saturating_add(1);
        let t = self.tick_count as f64;

        let wave_fast = (t * 0.41).sin() * 0.65 + (t * 0.17).cos() * 0.35;
        let wave_slow = (t * 0.05).sin();

        let phase_upper = snap.active_phase.to_ascii_uppercase();
        let resting_hr = snap.resting_hr_bpm.clamp(45.0, 95.0);

        let (
            mut target_hr,
            mut target_sbp,
            mut target_resp,
            mut target_spo2,
            mut target_to2,
            mut target_imu,
            mut target_temp,
            mut target_rad,
            mut target_co2,
            mut target_o2,
            mut target_g,
            target_suit_p,
            target_suit_h,
        ) = match phase_upper.as_str() {
            "EVA" => (
                122.0 + 6.0 * wave_slow,
                128.0 + 2.0 * wave_slow,
                21.0,
                98.0,
                94.0,
                0.55,
                36.7,
                0.72 + 0.08 * wave_slow,
                950.0 + 40.0 * wave_slow,
                21.0,
                0.01,
                29.6,
                40.0,
            ),
            "LAUNCH" => (
                124.0 + 4.0 * wave_slow,
                136.0 + 2.0 * wave_slow,
                22.0,
                97.5,
                93.0,
                0.32,
                36.6,
                0.25 + 0.03 * wave_slow,
                1200.0 + 50.0 * wave_slow,
                20.9,
                3.1,
                35.0,
                38.0,
            ),
            "REENTRY" => (
                118.0 + 4.0 * wave_slow,
                132.0 + 2.0 * wave_slow,
                20.0,
                97.0,
                92.5,
                0.28,
                36.6,
                0.30 + 0.04 * wave_slow,
                1300.0 + 60.0 * wave_slow,
                20.9,
                2.8,
                35.0,
                39.0,
            ),
            _ => (
                (resting_hr + 14.0).clamp(76.0, 88.0) + 3.0 * wave_slow,
                118.0 + 1.5 * wave_slow,
                14.0,
                98.2,
                94.5,
                0.22,
                36.4,
                0.42 + 0.04 * wave_slow,
                1150.0 + 45.0 * wave_slow,
                20.9,
                0.0,
                29.6,
                38.0,
            ),
        };

        // Dehydration induces modest compensatory tachycardia and mild SBP drop
        if hydration_deficit_ml > 500.0 {
            let excess_l = ((hydration_deficit_ml - 500.0) / 1000.0).clamp(0.0, 2.0);
            target_hr += excess_l * 6.0;
            target_sbp -= excess_l * 4.0;
        }

        match self.scenario {
            SimScenario::Nominal => {}
            SimScenario::Exercise => {
                target_hr = 146.0 + 5.0 * wave_slow;
                target_sbp = 148.0 + 3.0 * wave_slow;
                target_resp = 28.0;
                target_spo2 = 97.0;
                target_to2 = 91.5;
                target_imu = 0.88;
                target_temp = 37.3;
                target_co2 = 1650.0 + 110.0 * wave_slow;
            }
            SimScenario::SleepRest => {
                target_hr = (resting_hr - 12.0).clamp(50.0, 58.0) + 1.5 * wave_slow;
                target_sbp = 104.0;
                target_resp = 11.0;
                target_spo2 = 97.6;
                target_to2 = 93.8;
                target_imu = 0.02;
                target_temp = 36.1;
                target_co2 = 1050.0 + 25.0 * wave_slow;
            }
            SimScenario::OrthostaticStress => {
                target_hr = 132.0 + 3.0 * wave_slow;
                target_sbp = 90.0;
                target_resp = 22.0;
                target_spo2 = 96.5;
                target_to2 = 90.0;
                target_imu = 0.14;
                target_g = 2.6;
            }
            SimScenario::Hypoxia => {
                target_hr = 118.0 + 3.0 * wave_slow;
                target_sbp = 122.0;
                target_resp = 26.0;
                target_spo2 = 88.5 + 0.6 * wave_slow;
                target_to2 = 82.0 + 0.5 * wave_slow;
                target_o2 = 17.8;
                target_co2 = 3450.0 + 80.0 * wave_slow;
            }
            SimScenario::SolarFlare => {
                target_hr = 102.0 + 3.0 * wave_slow;
                target_sbp = 134.0;
                target_resp = 19.0;
                target_rad = 68.5 + 7.5 * wave_slow;
                target_co2 = 1400.0 + 50.0 * wave_slow;
            }
        }

        let alpha = 0.22;
        snap.hr_bpm =
            (snap.hr_bpm + (target_hr - snap.hr_bpm) * alpha + wave_fast * 1.1).clamp(38.0, 195.0);
        snap.sbp_mmhg =
            (snap.sbp_mmhg + (target_sbp - snap.sbp_mmhg) * alpha + wave_fast * 0.7).clamp(70.0, 190.0);
        snap.resp_brpm =
            (snap.resp_brpm + (target_resp - snap.resp_brpm) * alpha + wave_fast * 0.3).clamp(8.0, 42.0);
        snap.spo2_pct =
            (snap.spo2_pct + (target_spo2 - snap.spo2_pct) * 0.25 + wave_fast * 0.15).clamp(70.0, 100.0);

        let cur_to2 = snap.tissue_o2_pct.unwrap_or(target_to2);
        snap.tissue_o2_pct =
            Some((cur_to2 + (target_to2 - cur_to2) * 0.22 + wave_fast * 0.15).clamp(65.0, 100.0));

        snap.imu_activity_idx =
            (snap.imu_activity_idx + (target_imu - snap.imu_activity_idx) * 0.30).clamp(0.0, 1.0);
        snap.skin_temp_c =
            (snap.skin_temp_c + (target_temp - snap.skin_temp_c) * 0.12).clamp(34.5, 39.5);
        snap.g_load = (snap.g_load + (target_g - snap.g_load) * 0.28).clamp(0.0, 6.0);
        snap.radiation_usv_hr =
            (snap.radiation_usv_hr + (target_rad - snap.radiation_usv_hr) * 0.25).clamp(0.05, 500.0);
        snap.cabin_co2_ppm =
            (snap.cabin_co2_ppm + (target_co2 - snap.cabin_co2_ppm) * 0.18).clamp(350.0, 8000.0);
        let cur_o2 = snap.cabin_o2_pct.unwrap_or(20.9);
        snap.cabin_o2_pct = Some((cur_o2 + (target_o2 - cur_o2) * 0.22).clamp(14.0, 35.0));
        snap.suit_pressure_kpa = Some(target_suit_p);
        snap.suit_humidity_pct = Some(
            (snap.suit_humidity_pct.unwrap_or(target_suit_h)
                + (target_suit_h - snap.suit_humidity_pct.unwrap_or(target_suit_h)) * 0.15)
                .clamp(20.0, 95.0),
        );

        let inspired_n2_kpa = if phase_upper == "EVA" {
            snap.eva_elapsed_sec += 1;
            0.0 // 100% O2 suit washout
        } else {
            // Exploration cabin equilibrium keeps dissolved N2 near safe baseline unless prebreathe lowered it
            31.5
        };
        let (updated_n2_kpa, blood_n2_idx) =
            calc::update_tissue_n2(self.tissue_n2_kpa, inspired_n2_kpa, target_suit_p, 1.0);
        self.tissue_n2_kpa = updated_n2_kpa;
        snap.blood_n2_idx = Some(blood_n2_idx);

        snap.recorded_at_utc = calc::utc_now_iso8601();
        snap.recompute_derived();

        // Accumulate 60-second actigraphy epoch
        self.epoch_ticks += 1;
        self.epoch_hr_sum += snap.hr_bpm;
        self.epoch_activity_sum += snap.imu_activity_idx * 100.0;

        let sleep_epoch = if self.epoch_ticks >= 60 {
            let n = self.epoch_ticks as f64;
            let avg_hr = self.epoch_hr_sum / n;
            let avg_act = self.epoch_activity_sum / n;
            let sleep_state = if avg_act < 8.0 && avg_hr < 58.0 {
                "DEEP"
            } else if avg_act < 18.0 && avg_hr < 68.0 {
                "LIGHT"
            } else {
                "WAKE"
            }
            .to_string();
            self.epoch_ticks = 0;
            self.epoch_hr_sum = 0.0;
            self.epoch_activity_sum = 0.0;
            Some(SleepEpochEmission {
                activity_count: avg_act,
                hr_bpm_avg: avg_hr,
                sleep_state,
            })
        } else {
            None
        };

        SimTickOutput { sleep_epoch }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::PanelAlertState;
    use crate::db::{DailyNutritionTotals, DailyPhysioTotals, UnsyncedCounts};
    use crate::net::ConnectionState;

    fn sample_snapshot() -> UiSnapshot {
        UiSnapshot {
            astronaut_id: "AST-01".to_string(),
            callsign: "ARES-1".to_string(),
            weight_kg: 76.0,
            height_cm: 178.0,
            age_yr: 34,
            sex: "M".to_string(),
            sleep_target_min: 480,
            active_phase: "ORBIT".to_string(),
            mission_day: 1,
            seq_id: 1,
            recorded_at_utc: "2026-11-15T10:00:00Z".to_string(),
            hr_bpm: 78.0,
            resting_hr_bpm: 64.0,
            sbp_mmhg: 118.0,
            resp_brpm: 14.0,
            spo2_pct: 98.0,
            tissue_o2_pct: Some(94.0),
            blood_n2_idx: Some(1.08),
            g_load: 0.0,
            imu_activity_idx: 0.2,
            skin_temp_c: 36.4,
            suit_pressure_kpa: Some(29.6),
            suit_humidity_pct: Some(38.0),
            eva_elapsed_sec: 0,
            cabin_co2_ppm: 1150.0,
            cabin_temp_c: Some(22.0),
            cabin_humidity_pct: Some(44.0),
            cabin_pressure_kpa: Some(101.3),
            cabin_o2_pct: Some(20.9),
            cabin_motion_idx: 0.1,
            radiation_usv_hr: 0.42,
            kcal_per_sec: 0.05,
            sweat_ml_per_sec: 0.1,
            shock_index: 0.66,
            orthostatic_risk: "LOW",
            hub_distance_m: 120,
            heading_deg: 0.0,
            target_bearing_deg: 45.0,
            earth_mars_au: 1.42,
            one_way_light_min: 11.81,
            comms_state: ConnectionState::Offline,
            alerts: PanelAlertState::default(),
            food_cache: Vec::new(),
            protocols: Vec::new(),
            sans_state: None,
            osteo_state: None,
            daily_nutrition: DailyNutritionTotals::default(),
            daily_physio: DailyPhysioTotals::default(),
            recent_inputs: Vec::new(),
            local_alerts: Vec::new(),
            sleep_epochs: Vec::new(),
            unsynced: UnsyncedCounts::default(),
        }
    }

    #[test]
    fn test_sim_exercise_sleep_and_eva_washout() {
        let mut snap = sample_snapshot();
        let mut sim = BioSimulator::new(snap.blood_n2_idx, snap.suit_pressure_kpa);

        // Exercise elevates HR and sweat/kcal burn
        sim.scenario = SimScenario::Exercise;
        for _ in 0..20 {
            sim.tick_1hz(&mut snap, 0.0);
        }
        assert!(snap.hr_bpm > 130.0, "Exercise should elevate HR above 130, got {}", snap.hr_bpm);
        assert!(snap.kcal_per_sec > 0.18, "Exercise should increase Keytel burn");

        // EVA + SleepRest emits sleep epoch at 60th total tick and washes out N2
        let initial_n2 = snap.blood_n2_idx.unwrap();
        snap.active_phase = "EVA".to_string();
        sim.scenario = SimScenario::SleepRest;
        let mut emitted = None;
        for _ in 0..40 {
            let out = sim.tick_1hz(&mut snap, 0.0);
            if out.sleep_epoch.is_some() {
                emitted = out.sleep_epoch;
            }
        }
        assert!(emitted.is_some(), "Should emit one sleep epoch after 60 1Hz ticks");
        assert!(snap.eva_elapsed_sec == 40);
        assert!(
            snap.blood_n2_idx.unwrap() < initial_n2,
            "EVA pure O2 should wash out Haldane tissue N2"
        );
    }
}
