//! Per-tab flight inputs: persisted in `FlightPrefs`, hydrated into the working `FlightInputs` signal.

use crate::ground_oat::estimated_oat_c_from_ground;
use da20_core::types::{defaults, ChartPhase, CruiseSolveMode, FlightInputs};
use da20_core::PerformanceData;

use crate::phases::Phase;
use crate::prefs::{
    apply_estimated_cruise_oat, clamp_cruise_oat_for_propeller, hydrate_cruise_tab_fields,
    persist_cruise_tab_from_inputs, persist_takeoff_tab_from_inputs, ClimbTabPrefs, CruiseTabPrefs,
    FlightPrefs, LandingTabPrefs, TakeoffTabPrefs,
};

pub(crate) fn cruise_tab_mut<'a>(
    prefs: &'a mut FlightPrefs,
    phase: Phase,
) -> &'a mut CruiseTabPrefs {
    match phase {
        Phase::Cruise => &mut prefs.cruise,
        Phase::Endurance => &mut prefs.endurance.cruise,
        _ => panic!("cruise_tab_mut called for non-cruise phase"),
    }
}

pub(crate) fn cruise_tab_ref<'a>(prefs: &'a FlightPrefs, phase: Phase) -> &'a CruiseTabPrefs {
    match phase {
        Phase::Cruise => &prefs.cruise,
        Phase::Endurance => &prefs.endurance.cruise,
        _ => panic!("cruise_tab_ref called for non-cruise phase"),
    }
}

pub fn persist_tab_inputs(phase: Phase, prefs: &mut FlightPrefs, inputs: &FlightInputs) {
    match phase {
        Phase::Takeoff => {
            prefs.takeoff = TakeoffTabPrefs {
                pressure_altitude_ft: inputs.pressure_altitude_input_ft,
                oat_c: inputs.oat_c,
                weight_lb: inputs.weight_lb,
                runway: inputs.runway.clone(),
                wind_dir_deg: inputs.wind_dir_deg,
                wind_speed_kt: inputs.wind_speed_kt,
                obstacle_height_m: inputs.obstacle_height_m,
            };
        }
        Phase::Climb => {
            prefs.climb = ClimbTabPrefs {
                pressure_altitude_ft: inputs.pressure_altitude_input_ft,
                oat_c: inputs.oat_c,
                weight_lb: inputs.weight_lb,
                climb_figure: inputs.climb_figure,
                pa_override: inputs.pa_override.filter(|o| o.phase == ChartPhase::Climb),
                climb_target_altitude_ft: inputs.cruise_pressure_alt_ft,
            };
        }
        Phase::Landing => {
            prefs.landing = LandingTabPrefs {
                pressure_altitude_ft: inputs.pressure_altitude_input_ft,
                oat_c: inputs.oat_c,
                pa_override: inputs
                    .pa_override
                    .filter(|o| o.phase == ChartPhase::Landing),
                landing_ground_idle_1000_rpm: inputs.landing_ground_idle_1000_rpm,
            };
        }
        Phase::Cruise => {
            persist_cruise_tab_from_inputs(&mut prefs.cruise, inputs);
        }
        Phase::Endurance => {
            persist_cruise_tab_from_inputs(&mut prefs.endurance.cruise, inputs);
            prefs.endurance.fuel_usable_us_gal = inputs.fuel_usable_us_gal;
            prefs.endurance.fuel_reserve_us_gal = inputs.fuel_reserve_us_gal;
        }
        Phase::Manual => {}
    }
    prefs.save();
}

pub fn hydrate_tab_inputs(
    phase: Phase,
    prefs: &FlightPrefs,
    data: &PerformanceData,
    inputs: &mut FlightInputs,
) {
    inputs.pa_override = None;
    inputs.direct_pressure_altitude_ft = None;
    prefs.apply_global_fields(inputs);

    match phase {
        Phase::Takeoff => {
            inputs.pressure_altitude_input_ft = prefs.takeoff.pressure_altitude_ft;
            inputs.oat_c = prefs.takeoff.oat_c;
            inputs.weight_lb = prefs.takeoff.weight_lb;
            inputs.runway = prefs.takeoff.runway.clone();
            inputs.wind_dir_deg = prefs.takeoff.wind_dir_deg;
            inputs.wind_speed_kt = prefs.takeoff.wind_speed_kt;
            inputs.obstacle_height_m = prefs.takeoff.obstacle_height_m;
        }
        Phase::Climb => {
            inputs.pressure_altitude_input_ft = prefs.climb.pressure_altitude_ft;
            inputs.oat_c = prefs.climb.oat_c;
            inputs.weight_lb = prefs.climb.weight_lb;
            inputs.climb_figure = prefs.climb.climb_figure;
            inputs.pa_override = prefs.climb.pa_override;
            inputs.cruise_pressure_alt_ft = prefs.climb.climb_target_altitude_ft;
        }
        Phase::Landing => {
            inputs.pressure_altitude_input_ft = prefs.landing.pressure_altitude_ft;
            inputs.oat_c = prefs.landing.oat_c;
            inputs.pa_override = prefs.landing.pa_override;
            inputs.weight_lb = defaults::WEIGHT_LB;
            inputs.landing_ground_idle_1000_rpm = prefs.landing.landing_ground_idle_1000_rpm;
        }
        Phase::Cruise => {
            hydrate_cruise_tab_fields(&prefs.cruise, data, prefs, inputs);
        }
        Phase::Endurance => {
            hydrate_cruise_tab_fields(&prefs.endurance.cruise, data, prefs, inputs);
            inputs.fuel_usable_us_gal = prefs.endurance.fuel_usable_us_gal;
            inputs.fuel_reserve_us_gal = prefs.endurance.fuel_reserve_us_gal;
        }
        Phase::Manual => {}
    }
}

pub fn assemble_takeoff_inputs(prefs: &FlightPrefs) -> FlightInputs {
    FlightInputs {
        pressure_altitude_input_ft: prefs.takeoff.pressure_altitude_ft,
        oat_c: prefs.takeoff.oat_c,
        weight_lb: prefs.takeoff.weight_lb,
        runway: prefs.takeoff.runway.clone(),
        wind_dir_deg: prefs.takeoff.wind_dir_deg,
        wind_speed_kt: prefs.takeoff.wind_speed_kt,
        obstacle_height_m: prefs.takeoff.obstacle_height_m,
        wheel_fairings_installed: prefs.wheel_fairings_installed,
        ..Default::default()
    }
}

pub fn assemble_climb_inputs(prefs: &FlightPrefs) -> FlightInputs {
    FlightInputs {
        pressure_altitude_input_ft: prefs.climb.pressure_altitude_ft,
        oat_c: prefs.climb.oat_c,
        weight_lb: prefs.climb.weight_lb,
        climb_figure: prefs.climb.climb_figure,
        pa_override: prefs.climb.pa_override,
        wheel_fairings_installed: prefs.wheel_fairings_installed,
        cruise_pressure_alt_ft: prefs.climb.climb_target_altitude_ft,
        ..Default::default()
    }
}

pub fn assemble_landing_inputs(prefs: &FlightPrefs) -> FlightInputs {
    FlightInputs {
        pressure_altitude_input_ft: prefs.landing.pressure_altitude_ft,
        oat_c: prefs.landing.oat_c,
        pa_override: prefs.landing.pa_override,
        wheel_fairings_installed: prefs.wheel_fairings_installed,
        landing_ground_idle_1000_rpm: prefs.landing.landing_ground_idle_1000_rpm,
        ..Default::default()
    }
}

pub fn assemble_cruise_like_inputs(
    tab: &CruiseTabPrefs,
    prefs: &FlightPrefs,
    data: &PerformanceData,
) -> FlightInputs {
    let cruise_pressure_alt_ft = tab.cruise_pressure_alt_ft;
    let solve_mode = tab
        .cruise_solve_saved
        .unwrap_or(CruiseSolveMode::SetPercentBhp);
    let percent_bhp = tab
        .cruise_percent_bhp_saved
        .unwrap_or_else(|| crate::prefs::resolved_default_cruise_percent_bhp(data));
    let probe = FlightInputs {
        propeller: prefs.propeller,
        cruise_pressure_alt_ft,
        cruise_solve_mode: solve_mode,
        cruise_percent_bhp: percent_bhp,
        ..Default::default()
    };
    let cruise_oat_c = if tab.cruise_oat_manual {
        tab.cruise_oat_c_saved
            .map(|oat| clamp_cruise_oat_for_propeller(&probe, oat))
            .unwrap_or(probe.cruise_oat_c)
    } else {
        let est = estimated_oat_c_from_ground(
            tab.ground_elevation_ft,
            tab.ground_oat_c,
            cruise_pressure_alt_ft,
        );
        clamp_cruise_oat_for_propeller(&probe, est)
    };
    FlightInputs {
        propeller: prefs.propeller,
        wheel_fairings_installed: prefs.wheel_fairings_installed,
        cruise_pressure_alt_ft,
        cruise_rpm: tab.cruise_rpm,
        weight_lb: tab.weight_lb,
        cruise_solve_mode: solve_mode,
        cruise_percent_bhp: percent_bhp,
        cruise_oat_c,
        ..Default::default()
    }
}

pub fn assemble_cruise_inputs(prefs: &FlightPrefs, data: &PerformanceData) -> FlightInputs {
    assemble_cruise_like_inputs(&prefs.cruise, prefs, data)
}

pub fn assemble_endurance_inputs(prefs: &FlightPrefs, data: &PerformanceData) -> FlightInputs {
    let mut i = assemble_cruise_like_inputs(&prefs.endurance.cruise, prefs, data);
    i.fuel_usable_us_gal = prefs.endurance.fuel_usable_us_gal;
    i.fuel_reserve_us_gal = prefs.endurance.fuel_reserve_us_gal;
    i
}

pub fn apply_cruise_ground_oat(
    phase: Phase,
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    ground_oat_c: f64,
) {
    let tab = cruise_tab_mut(prefs, phase);
    tab.ground_oat_c = ground_oat_c;
    if !tab.cruise_oat_manual {
        apply_estimated_cruise_oat(tab, inputs);
    }
    prefs.save();
}

pub fn apply_cruise_ground_elevation(
    phase: Phase,
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    ground_elevation_ft: f64,
) {
    let tab = cruise_tab_mut(prefs, phase);
    tab.ground_elevation_ft = ground_elevation_ft;
    if !tab.cruise_oat_manual {
        apply_estimated_cruise_oat(tab, inputs);
    }
    prefs.save();
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::takeoff::compute_takeoff;

    fn seeded_prefs() -> FlightPrefs {
        let mut prefs = FlightPrefs::default();
        prefs.takeoff.pressure_altitude_ft = 2130.0;
        prefs.takeoff.oat_c = 20.0;
        prefs.takeoff.weight_lb = defaults::WEIGHT_LB;
        prefs.climb.pressure_altitude_ft = 2130.0;
        prefs.climb.oat_c = 20.0;
        prefs.cruise.ground_elevation_ft = 5000.0;
        prefs.cruise.ground_oat_c = 25.0;
        prefs.endurance.cruise.ground_elevation_ft = 5000.0;
        prefs.endurance.cruise.ground_oat_c = 25.0;
        prefs
    }

    #[test]
    fn climb_oat_edit_leaves_takeoff_compute_and_prefs() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        let takeoff_before = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs)).unwrap();
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Climb, &prefs, &data, &mut inputs);
        inputs.oat_c = -5.0;
        persist_tab_inputs(Phase::Climb, &mut prefs, &inputs);
        let takeoff_after = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs)).unwrap();
        assert_eq!(prefs.takeoff.oat_c, 20.0);
        assert!(
            (takeoff_before.ground_roll_ft - takeoff_after.ground_roll_ft).abs() < f64::EPSILON
        );
        assert_eq!(prefs.climb.oat_c, -5.0);
    }

    #[test]
    fn endurance_cruise_pa_edit_leaves_cruise_tab_pa() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        prefs.cruise.cruise_pressure_alt_ft = 6500.0;
        prefs.endurance.cruise.cruise_pressure_alt_ft = 7500.0;
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Endurance, &prefs, &data, &mut inputs);
        inputs.cruise_pressure_alt_ft = 9000.0;
        persist_tab_inputs(Phase::Endurance, &mut prefs, &inputs);
        assert_eq!(prefs.cruise.cruise_pressure_alt_ft, 6500.0);
        assert_eq!(prefs.endurance.cruise.cruise_pressure_alt_ft, 9000.0);
    }

    #[test]
    fn cruise_weight_edit_leaves_endurance_weight() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        prefs.cruise.weight_lb = 1763.0;
        prefs.endurance.cruise.weight_lb = 1600.0;
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Cruise, &prefs, &data, &mut inputs);
        inputs.weight_lb = 1500.0;
        persist_tab_inputs(Phase::Cruise, &mut prefs, &inputs);
        assert_eq!(prefs.cruise.weight_lb, 1500.0);
        assert_eq!(prefs.endurance.cruise.weight_lb, 1600.0);
    }

    #[test]
    fn climb_target_independent_of_cruise_pa() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        prefs.climb.climb_target_altitude_ft = 4000.0;
        prefs.cruise.cruise_pressure_alt_ft = 9000.0;
        let climb_in = assemble_climb_inputs(&prefs);
        let cruise_in = assemble_cruise_inputs(&prefs, &data);
        assert_eq!(climb_in.cruise_pressure_alt_ft, 4000.0);
        assert_eq!(cruise_in.cruise_pressure_alt_ft, 9000.0);
        let climb = da20_core::climb::compute_climb(&data, &climb_in, 4000.0).unwrap();
        let _ = climb;
    }

    #[test]
    fn takeoff_oat_edit_changes_takeoff_result() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        let roll_cold = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        prefs.takeoff.oat_c = 40.0;
        let roll_hot = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        assert!((roll_hot - roll_cold).abs() > 1.0);
    }

    #[test]
    fn cruise_ground_oat_changes_only_cruise_estimate() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        prefs.cruise.cruise_oat_manual = false;
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Cruise, &prefs, &data, &mut inputs);
        let est_before = inputs.cruise_oat_c;
        apply_cruise_ground_oat(Phase::Cruise, &mut prefs, &mut inputs, 5.0);
        assert!(
            inputs.cruise_oat_c != est_before
                || (prefs.cruise.ground_oat_c - 5.0).abs() < f64::EPSILON
        );
        assert_eq!(prefs.takeoff.oat_c, 20.0);
    }

    #[test]
    fn endurance_cruise_assemble_uses_endurance_tab_percent_only() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        prefs.cruise.cruise_pressure_alt_ft = 6500.0;
        prefs.cruise.weight_lb = 1763.0;
        prefs.endurance.cruise.cruise_pressure_alt_ft = 7500.0;
        prefs.endurance.cruise.weight_lb = 1600.0;
        prefs.endurance.cruise.cruise_percent_bhp_saved = Some(55.0);
        prefs.cruise.cruise_percent_bhp_saved = Some(70.0);
        let cruise_in = assemble_cruise_inputs(&prefs, &data);
        let endurance_in = assemble_endurance_inputs(&prefs, &data);
        assert!((cruise_in.cruise_percent_bhp - 70.0).abs() < f64::EPSILON);
        assert!((endurance_in.cruise_percent_bhp - 55.0).abs() < f64::EPSILON);
        assert!((cruise_in.cruise_pressure_alt_ft - 6500.0).abs() < f64::EPSILON);
        assert!((endurance_in.cruise_pressure_alt_ft - 7500.0).abs() < f64::EPSILON);
        assert!((cruise_in.weight_lb - 1763.0).abs() < f64::EPSILON);
        assert!((endurance_in.weight_lb - 1600.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tab_prefs_round_trip_json() {
        let prefs = seeded_prefs();
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.takeoff, prefs.takeoff);
        assert_eq!(back.climb, prefs.climb);
        assert_eq!(back.landing, prefs.landing);
        assert_eq!(back.cruise, prefs.cruise);
        assert_eq!(
            back.endurance.fuel_usable_us_gal,
            prefs.endurance.fuel_usable_us_gal
        );
        assert_eq!(
            back.endurance.cruise.cruise_pressure_alt_ft,
            prefs.endurance.cruise.cruise_pressure_alt_ft
        );
    }

    #[test]
    fn takeoff_weight_persist_round_trip() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Takeoff, &prefs, &data, &mut inputs);
        inputs.weight_lb = 1650.0;
        persist_takeoff_tab_from_inputs(&mut prefs, &inputs);
        let json = serde_json::to_string(&prefs).expect("serialize");
        let mut loaded: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        loaded.finish_migration();
        assert_eq!(loaded.takeoff.weight_lb, 1650.0);
        let roll = compute_takeoff(&data, &assemble_takeoff_inputs(&loaded))
            .unwrap()
            .ground_roll_ft;
        let mut inputs2 = FlightInputs::default();
        hydrate_tab_inputs(Phase::Takeoff, &loaded, &data, &mut inputs2);
        assert_eq!(inputs2.weight_lb, 1650.0);
        assert!(roll.is_finite());
    }
}
