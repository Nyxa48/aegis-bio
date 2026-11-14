use serde::{Deserialize, Serialize};

pub const BASAL_KCAL_PER_MIN: f64 = 1.2;
pub const HALDANE_HALF_TIME_MIN: f64 = 360.0;
pub const LIGHT_MIN_PER_AU: f64 = 8.31675;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PanelAlertState {
    pub health_danger: bool,
    pub hub_danger: bool,
    pub comms_danger: bool,
    pub hazard_danger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SafetyEvalInput<'a> {
    pub phase: &'a str,
    pub hr_bpm: f64,
    pub resting_hr_bpm: f64,
    pub sbp_mmhg: f64,
    pub spo2_pct: f64,
    pub tissue_o2_pct: Option<f64>,
    pub blood_n2_idx: Option<f64>,
    pub radiation_usv_hr: f64,
    pub cabin_co2_ppm: f64,
    pub cabin_o2_pct: Option<f64>,
    pub suit_pressure_kpa: Option<f64>,
    pub hub_distance_m: f64,
    pub comms_online: bool,
}

/// 1. Keytel et al. (2005) energy expenditure in kcal/sec (clamped at basal floor `1.2 / 60.0`).
///
/// Male:   ((-55.0969 + 0.6309 * HR + 0.1988 * W + 0.2017 * A) / 4.184) / 60.0
/// Female: ((-20.4022 + 0.4472 * HR - 0.1263 * W + 0.0740 * A) / 4.184) / 60.0
pub fn calc_kcal_per_sec(hr_bpm: f64, weight_kg: f64, age: i32, sex: &str) -> f64 {
    let age_f = age as f64;
    let kcal_per_min = if sex.eq_ignore_ascii_case("F") {
        (-20.4022 + 0.4472 * hr_bpm - 0.1263 * weight_kg + 0.0740 * age_f) / 4.184
    } else {
        (-55.0969 + 0.6309 * hr_bpm + 0.1988 * weight_kg + 0.2017 * age_f) / 4.184
    };
    kcal_per_min.max(BASAL_KCAL_PER_MIN) / 60.0
}

/// Evaporative fluid/sweat loss in mL/sec from metabolic rate (`kcal_per_min`) and skin temperature (`skin_temp_c`).
pub fn calc_sweat_ml_per_sec(kcal_per_min: f64, skin_temp_c: f64) -> f64 {
    let clamped_kcal_min = kcal_per_min.max(BASAL_KCAL_PER_MIN);
    let thermal_factor = 1.0 + ((skin_temp_c - 33.5).max(-2.0) * 0.12);
    let ml_per_min = (clamped_kcal_min * 1.45 * thermal_factor.max(0.5)).max(0.25);
    ml_per_min / 60.0
}

/// Returns the NASA-STD-3001 dynamic heart-rate corridor `(min_bpm, max_bpm)` for the given mission phase.
pub fn phase_hr_corridor(phase: &str) -> (f64, f64) {
    match phase.to_ascii_uppercase().as_str() {
        "EVA" => (75.0, 160.0),
        "LAUNCH" | "REENTRY" => (70.0, 150.0),
        _ => (45.0, 100.0), // ORBIT and default
    }
}

/// Computes Shock Index (`hr_bpm / sbp_mmhg`).
pub fn calc_shock_index(hr_bpm: f64, sbp_mmhg: f64) -> f64 {
    if sbp_mmhg <= 1.0 {
        9.99
    } else {
        hr_bpm / sbp_mmhg
    }
}

/// Evaluates orthostatic risk tier from Shock Index and G-load.
pub fn calc_orthostatic_risk(hr_bpm: f64, sbp_mmhg: f64, g_load: f64) -> &'static str {
    let si = calc_shock_index(hr_bpm, sbp_mmhg);
    if si > 0.90 || (g_load > 2.5 && sbp_mmhg < 95.0) {
        "HIGH"
    } else if si >= 0.75 || sbp_mmhg < 105.0 {
        "MODERATE"
    } else {
        "LOW"
    }
}

/// 2. Dynamic Phase Thresholds & Danger Overrides (`evaluate_safety`).
pub fn evaluate_safety(input: &SafetyEvalInput<'_>) -> PanelAlertState {
    let (hr_min, hr_max) = phase_hr_corridor(input.phase);
    let hr_out_of_corridor = input.hr_bpm < hr_min || input.hr_bpm > hr_max;
    let hr_spike = input.hr_bpm >= input.resting_hr_bpm + 50.0;
    let shock_index = calc_shock_index(input.hr_bpm, input.sbp_mmhg);

    let hypoxia_spo2 = input.spo2_pct < 92.0;
    let hypoxia_tissue = input.tissue_o2_pct.is_some_and(|t| t < 85.0);
    let shock_breach = shock_index > 0.90;
    let decomp_breach = input.blood_n2_idx.is_some_and(|r| r > 1.50);

    let is_eva = input.phase.eq_ignore_ascii_case("EVA");
    let suit_loss = is_eva && input.suit_pressure_kpa.is_some_and(|p| p < 25.0);

    let health_danger = hr_out_of_corridor
        || hr_spike
        || hypoxia_spo2
        || hypoxia_tissue
        || shock_breach
        || decomp_breach
        || suit_loss;

    let hazard_danger = input.radiation_usv_hr > 50.0
        || input.cabin_co2_ppm > 3000.0
        || input.cabin_o2_pct.is_some_and(|o2| o2 < 19.5)
        || suit_loss;

    let hub_danger = input.hub_distance_m > 1500.0;
    let comms_danger = !input.comms_online;

    PanelAlertState {
        health_danger,
        hub_danger,
        comms_danger,
        hazard_danger,
    }
}

/// 3. Decompression 1-compartment Haldane nitrogen washout step (`half_time_min = 360.0`).
///
/// Updates dissolved tissue nitrogen partial pressure `p_n2` at `dt_sec` (default 1 Hz)
/// and returns `(updated_p_n2_kpa, blood_n2_idx)` where `blood_n2_idx = updated_p_n2_kpa / suit_pressure_kpa`.
pub fn update_tissue_n2(
    current_p_n2_kpa: f64,
    inspired_n2_kpa: f64,
    suit_pressure_kpa: f64,
    dt_sec: f64,
) -> (f64, f64) {
    let k_per_sec = std::f64::consts::LN_2 / (HALDANE_HALF_TIME_MIN * 60.0);
    let decay = (-k_per_sec * dt_sec.max(0.0)).exp();
    let updated_p_n2 = inspired_n2_kpa + (current_p_n2_kpa - inspired_n2_kpa) * decay;
    let denom = suit_pressure_kpa.max(1.0);
    let blood_n2_idx = updated_p_n2 / denom;
    (updated_p_n2, blood_n2_idx)
}

/// 4. Daily SANS risk model from total dietary sodium (`mg`), average cabin CO2 (`ppm`), and flight phase.
///
/// Returns `(sans_score, "LOW" | "MODERATE" | "HIGH")`.
pub fn calc_sans_risk(sodium_mg_total: f64, avg_co2_ppm: f64, phase: &str) -> (f64, &'static str) {
    let sodium_ratio = (sodium_mg_total.max(0.0) / 3000.0).clamp(0.0, 2.5);
    let co2_ratio = (avg_co2_ppm.max(0.0) / 3000.0).clamp(0.0, 2.5);
    let phase_weight = match phase.to_ascii_uppercase().as_str() {
        "ORBIT" | "EVA" => 1.0,
        _ => 0.85,
    };

    let raw_score = (sodium_ratio * 40.0 + co2_ratio * 40.0) * phase_weight;
    let sans_score = raw_score.clamp(0.0, 100.0);
    let risk_tier = if sans_score >= 70.0 {
        "HIGH"
    } else if sans_score >= 40.0 {
        "MODERATE"
    } else {
        "LOW"
    };
    (sans_score, risk_tier)
}

/// 4b. Daily Osteoporosis protection/risk model from exercise load index, calcium (`mg`), and vitamin D3 (`IU`).
///
/// Returns `(protection_score, "LOW" | "MODERATE" | "HIGH")` where a high `protection_score` yields `"LOW"` bone-loss risk.
pub fn calc_osteoporosis_risk(
    exercise_load_idx: f64,
    calcium_mg: f64,
    vit_d3_iu: f64,
) -> (f64, &'static str) {
    let ex_norm = (exercise_load_idx.max(0.0) / 100.0).clamp(0.0, 1.2);
    let ca_norm = (calcium_mg.max(0.0) / 1200.0).clamp(0.0, 1.2);
    let d3_norm = (vit_d3_iu.max(0.0) / 800.0).clamp(0.0, 1.2);

    let protection_score = (ex_norm * 50.0 + ca_norm * 30.0 + d3_norm * 20.0).clamp(0.0, 100.0);
    let osteo_risk = if protection_score >= 70.0 {
        "LOW"
    } else if protection_score >= 40.0 {
        "MODERATE"
    } else {
        "HIGH"
    };
    (protection_score, osteo_risk)
}

/// 5. Earth-Mars one-way light delay in minutes (`distance_au * 8.31675`).
pub fn calc_one_way_light_min(distance_au: f64) -> f64 {
    distance_au.max(0.0) * LIGHT_MIN_PER_AU
}

/// Linear interpolation helper between two ephemeris distance samples (`t` in `0.0..=1.0`).
/// Returns `(interpolated_distance_au, interpolated_one_way_light_min)`.
pub fn interpolate_ephemeris(start_au: f64, end_au: f64, t: f64) -> (f64, f64) {
    let clamped_t = t.clamp(0.0, 1.0);
    let dist_au = start_au + (end_au - start_au) * clamped_t;
    (dist_au, calc_one_way_light_min(dist_au))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keytel_calorie_and_sweat_output() {
        let kcal_sec_m = calc_kcal_per_sec(120.0, 78.0, 36, "M");
        assert!(kcal_sec_m > 0.16 && kcal_sec_m < 0.19);

        let basal_sec = calc_kcal_per_sec(30.0, 50.0, 25, "F");
        assert!((basal_sec - (BASAL_KCAL_PER_MIN / 60.0)).abs() < 1e-6);

        let sweat_sec = calc_sweat_ml_per_sec(kcal_sec_m * 60.0, 35.0);
        assert!(sweat_sec > 0.05);
    }

    #[test]
    fn test_eva_vs_orbit_hr_alert_thresholds() {
        let base_input = SafetyEvalInput {
            phase: "ORBIT",
            hr_bpm: 115.0,
            resting_hr_bpm: 75.0,
            sbp_mmhg: 130.0,
            spo2_pct: 98.0,
            tissue_o2_pct: Some(92.0),
            blood_n2_idx: Some(1.10),
            radiation_usv_hr: 12.0,
            cabin_co2_ppm: 1200.0,
            cabin_o2_pct: Some(20.9),
            suit_pressure_kpa: Some(29.6),
            hub_distance_m: 222.0,
            comms_online: true,
        };

        let orbit_eval = evaluate_safety(&base_input);
        assert!(orbit_eval.health_danger);
        assert!(!orbit_eval.hazard_danger);

        let eva_input = SafetyEvalInput {
            phase: "EVA",
            ..base_input
        };
        let eva_eval = evaluate_safety(&eva_input);
        assert!(!eva_eval.health_danger);

        let eva_spike = SafetyEvalInput {
            phase: "EVA",
            hr_bpm: 130.0,
            ..base_input
        };
        assert!(evaluate_safety(&eva_spike).health_danger);
    }

    #[test]
    fn test_sans_and_osteoporosis_risk_tiers() {
        let (low_score, low_tier) = calc_sans_risk(1200.0, 900.0, "ORBIT");
        assert_eq!(low_tier, "LOW");
        assert!(low_score < 40.0);

        let (high_score, high_tier) = calc_sans_risk(3800.0, 3200.0, "ORBIT");
        assert_eq!(high_tier, "HIGH");
        assert!(high_score >= 70.0);

        let (prot_high, osteo_low) = calc_osteoporosis_risk(90.0, 1200.0, 800.0);
        assert_eq!(osteo_low, "LOW");
        assert!(prot_high >= 70.0);
    }

    #[test]
    fn test_haldane_decompression_ratio_and_light_delay() {
        let (p_n2_after, r_val) = update_tissue_n2(79.0, 0.0, 29.6, 60.0);
        assert!(p_n2_after < 79.0);
        assert!(r_val > 1.50);

        let delay = calc_one_way_light_min(1.42);
        assert!((delay - 11.809785).abs() < 1e-3);

        let (interp_au, interp_min) = interpolate_ephemeris(1.40, 1.44, 0.5);
        assert!((interp_au - 1.42).abs() < 1e-6);
        assert!((interp_min - calc_one_way_light_min(1.42)).abs() < 1e-6);
    }
}
