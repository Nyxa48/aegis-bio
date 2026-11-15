use hub_desktop::calc::{
    self, BASAL_KCAL_PER_MIN, HALDANE_HALF_TIME_MIN, LIGHT_MIN_PER_AU, PanelAlertState,
    SafetyEvalInput,
};
use hub_desktop::db::Database;

#[test]
fn validate_ephemeris_jpl_horizons_and_interpolation() {
    let mut db = Database::open_in_memory_fallback();
    db.seed_demo_data().expect("seed_demo_data");

    let ephem = db.load_ephemeris_all().expect("load_ephemeris_all");
    assert_eq!(
        ephem.len(),
        8,
        "Expected 8 JPL Horizons daily ephemeris rows"
    );

    // Verify monotonic closing range (Earth-Mars distance shrinking 2026-11-14 .. 2026-11-21)
    assert!(
        ephem
            .windows(2)
            .all(|w| w[1].distance_au < w[0].distance_au),
        "Expected distance_au to decrease monotonically across seeded week"
    );

    let mut max_err_min = 0.0_f64;
    for row in &ephem {
        let calc_min = calc::calc_one_way_light_min(row.distance_au);
        let err_min = (calc_min - row.one_way_light_min).abs();
        max_err_min = max_err_min.max(err_min);
        assert!(
            err_min < 1e-5,
            "JPL vs calc_one_way_light_min drift too high on {}: err={} min",
            row.mission_date,
            err_min
        );
    }

    // 48-hour linear interpolation accuracy: Day 0..Day 2 midpoint vs actual Day 1 JPL row
    let (mid_au, mid_min) =
        calc::interpolate_ephemeris(ephem[0].distance_au, ephem[2].distance_au, 0.5);
    let day1_au_err = (mid_au - ephem[1].distance_au).abs();
    let day1_min_err = (mid_min - ephem[1].one_way_light_min).abs();
    assert!(day1_au_err < 5e-5, "48h interp AU err={day1_au_err}");
    assert!(day1_min_err < 5e-4, "48h interp min err={day1_min_err}");

    // Clamping checks for out-of-range t
    let (clamp_lo, _) =
        calc::interpolate_ephemeris(ephem[0].distance_au, ephem[1].distance_au, -0.5);
    let (clamp_hi, _) =
        calc::interpolate_ephemeris(ephem[0].distance_au, ephem[1].distance_au, 1.5);
    assert!((clamp_lo - ephem[0].distance_au).abs() < 1e-12);
    assert!((clamp_hi - ephem[1].distance_au).abs() < 1e-12);

    println!(
        "[TEST-EPHEMERIS] 8 rows | LIGHT_MIN_PER_AU={} | max_err={:.7} min ({:.3} ms) | 48h_mid_err={:.7} AU ({:.3} ms)",
        LIGHT_MIN_PER_AU,
        max_err_min,
        max_err_min * 60_000.0,
        day1_au_err,
        day1_min_err * 60_000.0
    );
}

#[test]
fn validate_eda_environment_sans_osteoporosis_and_safety() {
    let mut db = Database::open_in_memory_fallback();
    db.seed_demo_data().expect("seed_demo_data");

    let eda = db.load_eda_environment().expect("load_eda_environment");
    let foods = db.load_food_catalog().expect("load_food_catalog");
    assert_eq!(eda.len(), 289);
    assert!(foods.len() >= 6);

    let iss_co2: Vec<f64> = eda.iter().filter_map(|e| e.co2_iss_ppm).collect();
    let gnd_co2: Vec<f64> = eda.iter().filter_map(|e| e.co2_ground_ppm).collect();
    let iss_temps: Vec<f64> = eda.iter().filter_map(|e| e.temp_iss_c).collect();
    let iss_hum: Vec<f64> = eda.iter().filter_map(|e| e.humidity_iss_pct).collect();
    assert_eq!(iss_co2.len(), 289);
    assert_eq!(gnd_co2.len(), 289);
    assert_eq!(iss_temps.len(), 289);
    assert_eq!(iss_hum.len(), 289);

    let iss_co2_avg = iss_co2.iter().sum::<f64>() / iss_co2.len() as f64;
    let gnd_co2_avg = gnd_co2.iter().sum::<f64>() / gnd_co2.len() as f64;
    let iss_temp_min = iss_temps.iter().copied().fold(f64::INFINITY, f64::min);
    let iss_temp_max = iss_temps.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    // Every EDA ISS sample should stay below 3000 ppm acute emergency cutoff
    let mut iss_hazard_count = 0_usize;
    for pt in &eda {
        let alerts = calc::evaluate_safety(&SafetyEvalInput {
            phase: "ORBIT",
            hr_bpm: 74.0,
            resting_hr_bpm: 66.0,
            sbp_mmhg: 118.0,
            spo2_pct: 98.5,
            tissue_o2_pct: Some(94.0),
            blood_n2_idx: Some(1.0),
            radiation_usv_hr: 1.5,
            cabin_co2_ppm: pt.co2_iss_ppm.unwrap(),
            cabin_o2_pct: Some(20.9),
            suit_pressure_kpa: None,
            hub_distance_m: 0.0,
            comms_online: true,
        });
        if alerts.hazard_danger {
            iss_hazard_count += 1;
        }
    }
    assert_eq!(iss_hazard_count, 0);

    // Build realistic diets from seeded iss_food_catalog
    let by_id = |id: &str| {
        foods
            .iter()
            .find(|f| f.food_id == id)
            .unwrap_or_else(|| panic!("missing food {id}"))
    };
    let low_na_menu = [
        by_id("FOOD-01"),
        by_id("FOOD-02"),
        by_id("FOOD-04"),
        by_id("FOOD-05"),
        by_id("FOOD-06"),
    ];
    let low_na_mg: f64 = low_na_menu.iter().map(|f| f.sodium_mg).sum();
    let low_ca_mg: f64 = low_na_menu.iter().map(|f| f.calcium_mg).sum();
    let low_d3_iu: f64 = low_na_menu.iter().map(|f| f.vit_d3_iu).sum();
    // 210 + 140 + 240 + 230 + 5 = 825 mg Na; 85+420+180+210+600 = 1495 mg Ca; 120+320+280+440+800 = 1960 IU D3
    assert!((low_na_mg - 825.0).abs() < 1e-6);

    let high_na_mg = by_id("FOOD-03").sodium_mg * 3.0 + by_id("FOOD-01").sodium_mg; // 2280 mg

    // Compare SANS risk on Ground CO2 vs real ISS RR-1 CO2 (2646 ppm)
    let (gnd_low_s, gnd_low_t) = calc::calc_sans_risk(low_na_mg, gnd_co2_avg, "ORBIT");
    let (iss_low_s, iss_low_t) = calc::calc_sans_risk(low_na_mg, iss_co2_avg, "ORBIT");
    let (iss_high_s, iss_high_t) = calc::calc_sans_risk(high_na_mg, iss_co2_avg, "ORBIT");
    let (iss_launch_s, _) = calc::calc_sans_risk(high_na_mg, iss_co2_avg, "LAUNCH");

    assert_eq!(gnd_low_t, "LOW");
    assert_eq!(iss_low_t, "MODERATE");
    assert_eq!(iss_high_t, "MODERATE");
    assert!(iss_low_s > gnd_low_s);
    assert!(iss_high_s > iss_low_s);
    assert!((iss_launch_s - iss_high_s * 0.85).abs() < 1e-9);

    // At 2605 mg Na + ISS 2646 ppm CO2, score crosses >= 70 HIGH threshold
    let (iss_crit_s, iss_crit_t) = calc::calc_sans_risk(2605.0, iss_co2_avg, "ORBIT");
    assert_eq!(iss_crit_t, "HIGH");
    assert!(iss_crit_s >= 70.0);

    // Verify AST-03 seeded telemetry_history <-> sans_daily_history parity and ISS CO2 response
    let ast03_tel = db
        .load_latest_telemetry("AST-03")
        .expect("load_latest_telemetry AST-03")
        .expect("AST-03 telemetry exists");
    let sans_hist = db.load_sans_history().expect("load_sans_history");
    let ast03_sans = sans_hist
        .iter()
        .find(|s| s.astronaut_id == "AST-03")
        .expect("AST-03 sans_daily_history exists");
    assert_eq!(ast03_tel.sans_risk, "MODERATE");
    assert_eq!(ast03_sans.sans_risk, "MODERATE");

    let (ast03_on_iss_co2, ast03_iss_tier) =
        calc::calc_sans_risk(ast03_sans.sodium_mg_total, iss_co2_avg, "ORBIT");
    let (ast01_on_iss_co2, ast01_iss_tier) = calc::calc_sans_risk(1450.0, iss_co2_avg, "ORBIT");
    assert!((ast03_on_iss_co2 - 67.54666666666667).abs() < 0.05);
    assert_eq!(ast03_iss_tier, "MODERATE");
    assert!((ast01_on_iss_co2 - 54.61333333333333).abs() < 0.05);
    assert_eq!(ast01_iss_tier, "MODERATE");

    let (ast03_high_s, ast03_high_t) =
        calc::calc_sans_risk(ast03_sans.sodium_mg_total, 2850.0, "ORBIT");
    assert_eq!(ast03_high_t, "HIGH");
    assert!(ast03_high_s >= 70.0);

    // Osteoporosis protection score from low_na_menu nutrients across exercise loads
    let (osteo_no_ex_s, osteo_no_ex_t) = calc::calc_osteoporosis_risk(0.0, low_ca_mg, low_d3_iu);
    let (osteo_hi_ex_s, osteo_hi_ex_t) = calc::calc_osteoporosis_risk(85.0, low_ca_mg, low_d3_iu);
    assert_eq!(osteo_no_ex_t, "MODERATE"); // ca/d3 alone capped at 1.2*(30+20)=60.0 -> MODERATE without ARED
    assert!((osteo_no_ex_s - 60.0).abs() < 1e-6);
    assert_eq!(osteo_hi_ex_t, "LOW");
    assert!((osteo_hi_ex_s - 100.0).abs() < 1e-6);

    println!(
        "[TEST-EDA] iss_co2={:.1}ppm (flat, hazards={}), gnd_co2_avg={:.1}ppm, temp={:.1}..{:.1}C | SANS(825mg Na): GND={:.2}({}) vs ISS={:.2}({}) | SANS(2280mg Na, ISS)={:.2}({}) | AST-03(2420mg): ISS={:.2}({}), 2850ppm={:.2}({}) | Osteo(menu Ca={:.0}mg,D3={:.0}IU): ex=0->{:.1}({}), ex=85->{:.1}({})",
        iss_co2_avg,
        iss_hazard_count,
        gnd_co2_avg,
        iss_temp_min,
        iss_temp_max,
        gnd_low_s,
        gnd_low_t,
        iss_low_s,
        iss_low_t,
        iss_high_s,
        iss_high_t,
        ast03_on_iss_co2,
        ast03_iss_tier,
        ast03_high_s,
        ast03_high_t,
        low_ca_mg,
        low_d3_iu,
        osteo_no_ex_s,
        osteo_no_ex_t,
        osteo_hi_ex_s,
        osteo_hi_ex_t
    );
}

#[test]
fn validate_radlab_dose_integral_and_quality_factor_sensitivity() {
    let mut db = Database::open_in_memory_fallback();
    db.seed_demo_data().expect("seed_demo_data");

    let rad = db.load_radiation_series().expect("load_radiation_series");
    assert_eq!(rad.len(), 1044);

    let mut rates: Vec<f64> = rad.iter().map(|r| r.absorbed_dose_rate_ugy_hr).collect();
    rates.sort_by(|a, b| a.total_cmp(b));
    let p50 = rates[rates.len() / 2];
    let p95 = rates[(rates.len() * 95) / 100];
    let max_rate = *rates.last().unwrap();

    // Independent verification of interval_dose_ugy = absorbed_dose_rate_ugy_hr * interval_sec / 3600
    let mut recomputed_dose = 0.0_f64;
    let mut stored_dose = 0.0_f64;
    let mut max_interval_diff = 0.0_f64;
    let mut gap_count = 0_usize;

    for pt in &rad {
        match (pt.interval_sec, pt.interval_dose_ugy) {
            (Some(sec), Some(d)) => {
                let calc_d = pt.absorbed_dose_rate_ugy_hr * (sec as f64) / 3600.0;
                max_interval_diff = max_interval_diff.max((calc_d - d).abs());
                recomputed_dose += calc_d;
                stored_dose += d;
            }
            _ => gap_count += 1,
        }
    }
    assert_eq!(gap_count, 2, "Expected initial row + 1 >300s telemetry gap");
    assert!(max_interval_diff < 1e-5);
    assert!((recomputed_dose - stored_dose).abs() < 1e-3);

    // Check evaluate_safety radiation threshold (> 50 uSv/hr) across quality factors Q=1.0, Q=2.0, Q=3.0
    let count_hazards_with_q = |q: f64| -> usize {
        rad.iter()
            .filter(|pt| {
                calc::evaluate_safety(&SafetyEvalInput {
                    phase: "ORBIT",
                    hr_bpm: 75.0,
                    resting_hr_bpm: 65.0,
                    sbp_mmhg: 120.0,
                    spo2_pct: 98.0,
                    tissue_o2_pct: Some(93.0),
                    blood_n2_idx: Some(1.0),
                    radiation_usv_hr: pt.absorbed_dose_rate_ugy_hr * q,
                    cabin_co2_ppm: 1200.0,
                    cabin_o2_pct: Some(20.9),
                    suit_pressure_kpa: None,
                    hub_distance_m: 100.0,
                    comms_online: true,
                })
                .hazard_danger
            })
            .count()
    };

    let trig_q1 = count_hazards_with_q(1.0);
    let trig_q2 = count_hazards_with_q(2.0);
    let trig_q3 = count_hazards_with_q(3.0);
    assert_eq!(trig_q1, 42);
    assert_eq!(trig_q2, 51);
    assert_eq!(trig_q3, 58);

    println!(
        "[TEST-RADLAB] pts={} (gaps={}) | p50={:.2}, p95={:.2}, max={:.2} uGy/hr | cum_dose={:.3} uGy (max_step_err={:.6}) | hazard(>50uSv/h): Q=1.0->{}, Q=2.0->{}, Q=3.0->{}",
        rad.len(),
        gap_count,
        p50,
        p95,
        max_rate,
        recomputed_dose,
        max_interval_diff,
        trig_q1,
        trig_q2,
        trig_q3
    );
}

#[test]
fn validate_physiology_haldane_safety_boundaries_and_osd_labs() {
    let mut db = Database::open_in_memory_fallback();
    db.seed_demo_data().expect("seed_demo_data");

    // 1. Keytel & sweat for seeded crew (male AST-01 vs female AST-03)
    let crew = db.load_crew_members().expect("load_crew_members");
    assert_eq!(crew.len(), 6);
    for m in &crew {
        let kcal_rest = calc::calc_kcal_per_sec(65.0, m.weight, m.age as i32, &m.sex);
        let kcal_eva = calc::calc_kcal_per_sec(140.0, m.weight, m.age as i32, &m.sex);
        let sweat_eva = calc::calc_sweat_ml_per_sec(kcal_eva * 60.0, 36.8);
        assert!(kcal_rest >= BASAL_KCAL_PER_MIN / 60.0);
        assert!(kcal_eva > kcal_rest);
        assert!(sweat_eva > 0.15 && sweat_eva < 0.70);
    }
    // Verify female vs male branch differs and basal clamp engages at low HR
    let f_eva = calc::calc_kcal_per_sec(130.0, 64.0, 33, "F");
    let m_eva = calc::calc_kcal_per_sec(130.0, 64.0, 33, "M");
    assert!((f_eva - m_eva).abs() > 0.01);
    assert!(
        (calc::calc_kcal_per_sec(20.0, 50.0, 20, "F") - BASAL_KCAL_PER_MIN / 60.0).abs() < 1e-9
    );

    // 2. Shock index NaN/negative guards + Orthostatic risk tiers
    assert!((calc::calc_shock_index(80.0, 0.0) - 9.99).abs() < 1e-9);
    assert!((calc::calc_shock_index(80.0, f64::NAN) - 9.99).abs() < 1e-9);
    assert!((calc::calc_shock_index(f64::NAN, 120.0) - 0.0).abs() < 1e-9);
    assert!((calc::calc_shock_index(-10.0, 120.0) - 0.0).abs() < 1e-9);
    assert_eq!(calc::calc_orthostatic_risk(70.0, 120.0, 1.0), "LOW");
    assert_eq!(calc::calc_orthostatic_risk(85.0, 102.0, 1.0), "MODERATE");
    assert_eq!(calc::calc_orthostatic_risk(80.0, 90.0, 3.0), "HIGH");

    // 3. Haldane 1 Hz incremental vs closed-form 2700s + exact time to R <= 1.50
    let (single_p, single_r) = calc::update_tissue_n2(79.0, 0.0, 29.6, 2700.0);
    let mut step_p = 79.0;
    let mut step_r = 79.0 / 29.6;
    for _ in 0..2700 {
        (step_p, step_r) = calc::update_tissue_n2(step_p, 0.0, 29.6, 1.0);
    }
    assert!((single_p - step_p).abs() < 1e-10);
    assert!((single_r - step_r).abs() < 1e-10);
    assert!((single_p - 72.441).abs() < 0.01);
    assert!(single_r > 1.50);

    // Analytical time for R = 1.50 (pN2 = 1.50 * 29.6 = 44.4 kPa): t = 360 * ln(79 / 44.4) / ln(2)
    let expected_min = HALDANE_HALF_TIME_MIN * (79.0_f64 / 44.4_f64).ln() / std::f64::consts::LN_2;
    let (p_at_target, r_at_target) = calc::update_tissue_n2(79.0, 0.0, 29.6, expected_min * 60.0);
    assert!((p_at_target - 44.4).abs() < 1e-9);
    assert!((r_at_target - 1.50).abs() < 1e-9);

    // 4. Exhaustive SafetyEvalInput boundary checks (including EVA resting+50 spike and NaN fail-safe)
    let nominal = SafetyEvalInput {
        phase: "ORBIT",
        hr_bpm: 75.0,
        resting_hr_bpm: 65.0,
        sbp_mmhg: 120.0,
        spo2_pct: 92.0, // exact >= 92.0 boundary
        tissue_o2_pct: Some(85.0),
        blood_n2_idx: Some(1.50),
        radiation_usv_hr: 50.0,
        cabin_co2_ppm: 3000.0,
        cabin_o2_pct: Some(19.5),
        suit_pressure_kpa: Some(20.0), // ignored in ORBIT, triggers only in EVA
        hub_distance_m: 1500.0,
        comms_online: true,
    };
    let res_nom = calc::evaluate_safety(&nominal);
    assert!(
        !res_nom.health_danger
            && !res_nom.hazard_danger
            && !res_nom.hub_danger
            && !res_nom.comms_danger
    );

    // EVA corridor is 75..160 BPM, but resting 68 + 50 = 118 BPM must still trip spike guard
    let res_eva_spike = calc::evaluate_safety(&SafetyEvalInput {
        phase: "EVA",
        hr_bpm: 118.0,
        resting_hr_bpm: 68.0,
        suit_pressure_kpa: Some(29.6),
        ..nominal
    });
    assert!(res_eva_spike.health_danger);
    assert!(!res_eva_spike.hazard_danger);

    // NaN inputs fail-safe instead of evaluating false in IEEE 754 comparisons
    let res_nan = calc::evaluate_safety(&SafetyEvalInput {
        spo2_pct: f64::NAN,
        radiation_usv_hr: f64::NAN,
        hub_distance_m: f64::NAN,
        ..nominal
    });
    assert_eq!(
        res_nan,
        PanelAlertState {
            health_danger: true,
            hub_danger: true,
            comms_danger: false,
            hazard_danger: true,
        }
    );

    let res_eva_suit_loss = calc::evaluate_safety(&SafetyEvalInput {
        phase: "EVA",
        ..nominal
    });
    assert!(res_eva_suit_loss.health_danger && res_eva_suit_loss.hazard_danger);
    assert!(
        calc::evaluate_safety(&SafetyEvalInput {
            hub_distance_m: 1500.1,
            comms_online: false,
            ..nominal
        }) == PanelAlertState {
            health_danger: false,
            hub_danger: true,
            comms_danger: true,
            hazard_danger: false,
        }
    );

    // 5. OSD-569/575/656 Research Lab Observations integrity & physiological range check
    let obs = db
        .load_research_lab_observations(None, None, 30_000)
        .expect("load_research_lab_observations");
    assert_eq!(obs.len(), 23_096);
    for o in &obs {
        assert!(o.measured_value.is_finite() && o.measured_value >= 0.0);
        if let Some(lo) = o.ref_min
            && o.measured_value < lo
        {
            assert_eq!(o.status_flag, "BELOW_REF");
        }
        if let Some(hi) = o.ref_max
            && o.measured_value > hi
        {
            assert_eq!(o.status_flag, "ABOVE_REF");
        }
    }

    let cmp_na: Vec<f64> = obs
        .iter()
        .filter(|o| o.domain == "CMP" && o.biomarker_name == "sodium")
        .map(|o| o.measured_value)
        .collect();
    let cmp_ca: Vec<f64> = obs
        .iter()
        .filter(|o| o.domain == "CMP" && o.biomarker_name == "calcium")
        .map(|o| o.measured_value)
        .collect();
    let cmp_glu_min = obs
        .iter()
        .filter(|o| o.domain == "CMP" && o.biomarker_name == "glucose")
        .map(|o| o.measured_value)
        .fold(f64::INFINITY, f64::min);
    assert_eq!(cmp_na.len(), 28);
    assert_eq!(cmp_ca.len(), 28);
    assert!((cmp_glu_min - 36.0).abs() < 1e-9);

    println!(
        "[TEST-PHYS-LABS] Haldane 45m={:.2}kPa (R={:.2}) | R=1.50 prebreathe={:.1}min | Labs={} (0 NaNs/mismatches) | CMP Na={:.1}..{:.1} mmol/L, Ca={:.1}..{:.1} mg/dL, Glu_min={:.1} mg/dL",
        single_p,
        single_r,
        expected_min,
        obs.len(),
        cmp_na.iter().copied().fold(f64::INFINITY, f64::min),
        cmp_na.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        cmp_ca.iter().copied().fold(f64::INFINITY, f64::min),
        cmp_ca.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        cmp_glu_min
    );
}
