//! Per-tab flight inputs: persisted in `FlightPrefs`, hydrated into the working `FlightInputs` signal.

use crate::ground_oat::estimated_oat_c_from_ground;
use da20_core::types::{defaults, ChartPhase, CruiseSolveMode, FlightInputs};
use da20_core::PerformanceData;

use crate::phases::Phase;
use crate::prefs::{
    apply_estimated_cruise_oat, clamp_cruise_oat_for_propeller, ClimbTabPrefs, EnduranceTabPrefs,
    FlightPrefs, LandingTabPrefs, TakeoffTabPrefs,
};

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
            };
        }
        Phase::Landing => {
            prefs.landing = LandingTabPrefs {
                pressure_altitude_ft: inputs.pressure_altitude_input_ft,
                oat_c: inputs.oat_c,
                pa_override: inputs
                    .pa_override
                    .filter(|o| o.phase == ChartPhase::Landing),
            };
        }
        Phase::Cruise => {
            prefs.cruise_planning.cruise_pressure_alt_ft = inputs.cruise_pressure_alt_ft;
            prefs.cruise_planning.weight_lb = inputs.weight_lb;
            prefs.cruise_planning.cruise_rpm = inputs.cruise_rpm;
        }
        Phase::Endurance => {
            prefs.endurance = EnduranceTabPrefs {
                fuel_usable_us_gal: inputs.fuel_usable_us_gal,
                fuel_reserve_us_gal: inputs.fuel_reserve_us_gal,
            };
            // endurance uses cruise planning + cruise fields (see hydrate)
            prefs.cruise_planning.weight_lb = inputs.weight_lb;
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
    prefs.apply_global_fields(data, inputs);

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
        }
        Phase::Landing => {
            inputs.pressure_altitude_input_ft = prefs.landing.pressure_altitude_ft;
            inputs.oat_c = prefs.landing.oat_c;
            inputs.pa_override = prefs.landing.pa_override;
            inputs.weight_lb = defaults::WEIGHT_LB;
        }
        Phase::Cruise | Phase::Endurance => {
            inputs.cruise_pressure_alt_ft = prefs.cruise_planning.cruise_pressure_alt_ft;
            inputs.cruise_rpm = prefs.cruise_planning.cruise_rpm;
            inputs.weight_lb = prefs.cruise_planning.weight_lb;
            inputs.fuel_usable_us_gal = prefs.endurance.fuel_usable_us_gal;
            inputs.fuel_reserve_us_gal = prefs.endurance.fuel_reserve_us_gal;
            if prefs.cruise_oat_manual {
                if let Some(oat) = prefs.cruise_oat_c_saved {
                    inputs.cruise_oat_c = clamp_cruise_oat_for_propeller(inputs, oat);
                }
            } else {
                apply_estimated_cruise_oat(inputs, prefs);
            }
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
        cruise_pressure_alt_ft: prefs.cruise_planning.cruise_pressure_alt_ft,
        ..Default::default()
    }
}

pub fn assemble_landing_inputs(prefs: &FlightPrefs) -> FlightInputs {
    FlightInputs {
        pressure_altitude_input_ft: prefs.landing.pressure_altitude_ft,
        oat_c: prefs.landing.oat_c,
        pa_override: prefs.landing.pa_override,
        wheel_fairings_installed: prefs.wheel_fairings_installed,
        landing_ground_idle_1000_rpm: prefs.landing_ground_idle_1000_rpm,
        ..Default::default()
    }
}

pub fn assemble_cruise_inputs(prefs: &FlightPrefs, data: &PerformanceData) -> FlightInputs {
    let cruise_pressure_alt_ft = prefs.cruise_planning.cruise_pressure_alt_ft;
    let solve_mode = prefs
        .cruise_solve_saved
        .unwrap_or(CruiseSolveMode::SetPercentBhp);
    let percent_bhp = prefs
        .cruise_percent_bhp_saved
        .unwrap_or_else(|| crate::prefs::resolved_default_cruise_percent_bhp(data));
    let probe = FlightInputs {
        propeller: prefs.propeller,
        cruise_pressure_alt_ft,
        cruise_solve_mode: solve_mode,
        cruise_percent_bhp: percent_bhp,
        ..Default::default()
    };
    let cruise_oat_c = if prefs.cruise_oat_manual {
        prefs
            .cruise_oat_c_saved
            .map(|oat| clamp_cruise_oat_for_propeller(&probe, oat))
            .unwrap_or(probe.cruise_oat_c)
    } else {
        let est = estimated_oat_c_from_ground(
            prefs.cruise_planning.ground_elevation_ft,
            prefs.cruise_planning.ground_oat_c,
            cruise_pressure_alt_ft,
        );
        clamp_cruise_oat_for_propeller(&probe, est)
    };
    FlightInputs {
        propeller: prefs.propeller,
        wheel_fairings_installed: prefs.wheel_fairings_installed,
        cruise_pressure_alt_ft,
        cruise_rpm: prefs.cruise_planning.cruise_rpm,
        weight_lb: prefs.cruise_planning.weight_lb,
        cruise_solve_mode: solve_mode,
        cruise_percent_bhp: percent_bhp,
        cruise_oat_c,
        ..Default::default()
    }
}

pub fn assemble_endurance_inputs(prefs: &FlightPrefs, data: &PerformanceData) -> FlightInputs {
    FlightInputs {
        fuel_usable_us_gal: prefs.endurance.fuel_usable_us_gal,
        fuel_reserve_us_gal: prefs.endurance.fuel_reserve_us_gal,
        ..assemble_cruise_inputs(prefs, data)
    }
}

/// Cruise ground OAT edit helper used by cruise-only ground fields.
pub fn apply_cruise_ground_oat(
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    ground_oat_c: f64,
) {
    prefs.cruise_planning.ground_oat_c = ground_oat_c;
    if !prefs.cruise_oat_manual {
        apply_estimated_cruise_oat(inputs, prefs);
    }
    prefs.save();
}

pub fn apply_cruise_ground_elevation(
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    ground_elevation_ft: f64,
) {
    prefs.cruise_planning.ground_elevation_ft = ground_elevation_ft;
    if !prefs.cruise_oat_manual {
        apply_estimated_cruise_oat(inputs, prefs);
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
        prefs.cruise_planning.ground_elevation_ft = 5000.0;
        prefs.cruise_planning.ground_oat_c = 25.0;
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
    fn landing_oat_edit_leaves_takeoff_compute() {
        let data = PerformanceData::production();
        let mut prefs = seeded_prefs();
        let roll_before = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Landing, &prefs, &data, &mut inputs);
        inputs.oat_c = -10.0;
        persist_tab_inputs(Phase::Landing, &mut prefs, &inputs);
        let roll_after = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        assert_eq!(prefs.takeoff.oat_c, 20.0);
        assert!((roll_before - roll_after).abs() < f64::EPSILON);
        assert_eq!(prefs.landing.oat_c, -10.0);
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
        prefs.cruise_oat_manual = false;
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Cruise, &prefs, &data, &mut inputs);
        let est_before = inputs.cruise_oat_c;
        apply_cruise_ground_oat(&mut prefs, &mut inputs, 5.0);
        assert!(
            inputs.cruise_oat_c != est_before
                || (prefs.cruise_planning.ground_oat_c - 5.0).abs() < f64::EPSILON
        );
        let takeoff_roll = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        let _ = takeoff_roll;
        assert_eq!(prefs.takeoff.oat_c, 20.0);
    }

    #[test]
    fn tab_prefs_round_trip_json() {
        let prefs = seeded_prefs();
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.takeoff, prefs.takeoff);
        assert_eq!(back.climb, prefs.climb);
        assert_eq!(back.landing, prefs.landing);
        assert_eq!(back.cruise_planning, prefs.cruise_planning);
    }
}
