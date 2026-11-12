//! Property: solvers never return numbers outside printed chart hulls (checklist C/CL/T/L).

#[cfg(test)]
mod checklist {
    use crate::climb::{compute_climb, ClimbError};
    use crate::cruise::{compute_cruise, isa_temp_c_at_pa, CruiseError};
    use crate::data::PerformanceData;
    use crate::landing::{compute_landing, LandingError};
    use crate::rev29::engine::{
        equivalent_altitude_ft, landing_lookup_ft, obstacle_add_m, takeoff_base_m, weight_factor,
    };
    use crate::rev29::Rev29File;
    use crate::takeoff::{compute_takeoff, TakeoffError};
    use crate::types::CruiseSolveMode;
    use crate::types::{defaults, FlightInputs};

    fn d() -> PerformanceData {
        PerformanceData::production()
    }

    fn rev() -> Rev29File {
        Rev29File::production()
    }

    #[test]
    fn c1_disa_plus20_out_of_chart() {
        let data = d();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6000.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6000.0) + 20.0,
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&data, &inputs),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 ΔISA",
                ..
            })
        ));
    }

    #[test]
    fn c2_rpm_2400_at_6500_out_of_chart() {
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2400.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&d(), &inputs),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                ..
            })
        ));
    }

    #[test]
    fn c3_pa_below_2000_out_of_chart() {
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 1500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(1500.0),
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&d(), &inputs),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 PA",
                ..
            })
        ));
    }

    #[test]
    fn c4_bhp_inverse_within_rpm_intersection_only() {
        let data = d();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 56.25,
            ..Default::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        let rpm = r.rpm.unwrap();
        assert!((rpm - 2600.0).abs() < 1.0);
        let low_inputs = FlightInputs {
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 5.0,
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            ..Default::default()
        };
        assert_eq!(
            compute_cruise(&data, &low_inputs),
            Err(CruiseError::BhpNotAchievable)
        );
    }

    #[test]
    fn cl1_fig52_pa_out_of_chart() {
        assert!(equivalent_altitude_ft(&rev(), -1.0, 15.0).is_err());
        assert!(equivalent_altitude_ft(&rev(), 16_001.0, 15.0).is_err());
    }

    #[test]
    fn cl2_fig52_oat_out_of_chart_climb() {
        let inputs = FlightInputs {
            oat_c: -21.0,
            ..Default::default()
        };
        assert!(matches!(
            compute_climb(&d(), &inputs, inputs.cruise_pressure_alt_ft),
            Err(ClimbError::OutOfChart(_))
        ));
    }

    #[test]
    fn t1_takeoff_pa_out_of_chart() {
        assert!(takeoff_base_m(&rev(), -1.0, 15.0).is_err());
        assert!(takeoff_base_m(&rev(), 10_001.0, 15.0).is_err());
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: -1.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        assert!(matches!(
            compute_takeoff(&d(), &inputs),
            Err(TakeoffError::OutOfChart(_))
        ));
    }

    #[test]
    fn t2_takeoff_oat_out_of_chart() {
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            oat_c: 56.0,
            ..Default::default()
        };
        assert!(compute_takeoff(&d(), &inputs).is_err());
    }

    #[test]
    fn t3_weight_out_of_chart() {
        let inputs = FlightInputs {
            weight_lb: 400.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        assert!(compute_takeoff(&d(), &inputs).is_err());
    }

    #[test]
    fn t4_headwind_over_20_takeoff_out_of_chart() {
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            wind_dir_deg: 0.0,
            wind_speed_kt: 25.0,
            runway: "360".into(),
            ..Default::default()
        };
        assert!(compute_takeoff(&d(), &inputs).is_err());
    }

    #[test]
    fn t5_tailwind_over_5_takeoff_out_of_chart() {
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            wind_dir_deg: 180.0,
            wind_speed_kt: 8.0,
            runway: "360".into(),
            ..Default::default()
        };
        assert!(compute_takeoff(&d(), &inputs).is_err());
    }

    #[test]
    fn t6_obstacle_panel_entry_out_of_chart() {
        let rev = rev();
        let (wf, _) = weight_factor(&rev, defaults::TAKEOFF_REF_WEIGHT_KG).unwrap();
        let (base, _) = takeoff_base_m(&rev, 1000.0, 21.0).unwrap();
        let entry = base * wf;
        let min_line = rev.obs_lines.value[0];
        assert!(obstacle_add_m(&rev, min_line - 1.0, 5.0).is_err());
        assert!(obstacle_add_m(&rev, entry, 5.0).is_ok());
        let max_line = *rev.obs_lines.value.last().unwrap();
        assert!(obstacle_add_m(&rev, max_line + 1.0, 5.0).is_err());
    }

    #[test]
    fn l1_landing_pa_out_of_chart() {
        assert!(landing_lookup_ft(&rev(), -1.0).is_err());
        assert!(landing_lookup_ft(&rev(), 7001.0).is_err());
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 8000.0,
                phase: crate::types::ChartPhase::Landing,
            }),
            ..Default::default()
        };
        assert!(matches!(
            compute_landing(&d(), &inputs),
            Err(LandingError::OutOfChart(_))
        ));
    }
}
