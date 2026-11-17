//! Out-of-chart at chart limit + small epsilon (interpolate-only; no clamp).

#[cfg(test)]
mod epsilon {
    use crate::chart_bounds::limits;
    use crate::climb::compute_climb;
    use crate::cruise::{compute_cruise, isa_temp_c_at_pa, CruiseError};
    use crate::data::PerformanceData;
    use crate::landing::{compute_landing, LandingError};
    use crate::rev29::engine::{kg_to_lb, takeoff_distances_m};
    use crate::takeoff::{compute_takeoff, TakeoffError};
    use crate::types::{ChartPhase, FlightInputs, PaOverride};
    const EPS: f64 = 0.1;

    fn d() -> PerformanceData {
        PerformanceData::production()
    }

    fn takeoff_in_chart() -> FlightInputs {
        FlightInputs {
            pa_override: Some(PaOverride {
                pa_ft: 2130.0,
                phase: ChartPhase::Takeoff,
            }),
            oat_c: 15.0,
            weight_lb: kg_to_lb(800.0),
            wind_dir_deg: 0.0,
            wind_speed_kt: 0.0,
            runway: "360".into(),
            obstacle_height_m: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn takeoff_tailwind_at_limit_ok_over_limit_ooc() {
        let data = d();
        let mut ok = takeoff_in_chart();
        ok.wind_dir_deg = 180.0;
        ok.wind_speed_kt = limits::FIG54_TAILWIND_KT.1;
        assert!(compute_takeoff(&data, &ok).is_ok());

        let mut ooc = ok.clone();
        ooc.wind_speed_kt = limits::FIG54_TAILWIND_KT.1 + EPS;
        assert!(matches!(
            compute_takeoff(&data, &ooc),
            Err(TakeoffError::OutOfChart(_))
        ));
    }

    #[test]
    fn takeoff_headwind_at_limit_ok_over_limit_ooc() {
        let data = d();
        let mut ok = takeoff_in_chart();
        ok.wind_dir_deg = 0.0;
        ok.wind_speed_kt = limits::FIG54_HEADWIND_KT.1;
        assert!(compute_takeoff(&data, &ok).is_ok());

        let mut ooc = ok.clone();
        ooc.wind_speed_kt = limits::FIG54_HEADWIND_KT.1 + EPS;
        assert!(matches!(
            compute_takeoff(&data, &ooc),
            Err(TakeoffError::OutOfChart(_))
        ));
    }

    #[test]
    fn takeoff_weight_at_limit_ok_over_limit_ooc() {
        let data = d();
        let mut ok = takeoff_in_chart();
        ok.weight_lb = kg_to_lb(limits::FIG54_WEIGHT_KG.1);
        assert!(compute_takeoff(&data, &ok).is_ok());

        let mut ooc = ok;
        ooc.weight_lb = kg_to_lb(limits::FIG54_WEIGHT_KG.1) + 0.01;
        assert!(matches!(
            compute_takeoff(&data, &ooc),
            Err(TakeoffError::OutOfChart(_))
        ));
    }

    #[test]
    fn takeoff_pa_oat_epsilon_ooc() {
        let data = d();
        let rev = data.inner();
        let (wf, _) = crate::rev29::engine::weight_factor(rev, 800.0).unwrap();
        let (base, _) = crate::rev29::engine::takeoff_base_m(rev, 1000.0, 15.0).unwrap();
        let entry = base * wf;

        assert!(
            takeoff_distances_m(rev, limits::FIG54_PA_FT.1, 15.0, 800.0, 0.0, 0.0, 0.0,).is_ok()
        );
        assert!(
            takeoff_distances_m(rev, limits::FIG54_PA_FT.1 + EPS, 15.0, 800.0, 0.0, 0.0, 0.0,)
                .is_err()
        );

        assert!(
            takeoff_distances_m(rev, 1000.0, limits::FIG54_OAT_C.1, 800.0, 0.0, 0.0, 0.0,).is_ok()
        );
        assert!(takeoff_distances_m(
            rev,
            1000.0,
            limits::FIG54_OAT_C.1 + EPS,
            800.0,
            0.0,
            0.0,
            0.0,
        )
        .is_err());
        let _ = entry;
    }

    #[test]
    fn climb_fig52_pa_epsilon() {
        let data = d();
        use crate::equivalent_altitude::equivalent_altitude_ft;
        assert!(equivalent_altitude_ft(&data, limits::FIG52_PA_FT.1, 15.0).is_ok());
        assert!(equivalent_altitude_ft(&data, limits::FIG52_PA_FT.1 + EPS, 15.0).is_err());
        assert!(equivalent_altitude_ft(&data, 4000.0, limits::FIG52_OAT_C.1).is_ok());
        assert!(equivalent_altitude_ft(&data, 4000.0, limits::FIG52_OAT_C.1 + EPS).is_err());
        let ok = FlightInputs {
            oat_c: 15.0,
            ..Default::default()
        };
        assert!(compute_climb(&data, &ok, 8000.0).is_ok());
    }

    #[test]
    fn cruise_table3_pa_epsilon() {
        let data = d();
        let pa = 10_000.0;
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: isa_temp_c_at_pa(pa),
            cruise_rpm: 2600.0,
            ..Default::default()
        };
        assert!(compute_cruise(&data, &inputs).is_ok());
        let ooc = FlightInputs {
            cruise_pressure_alt_ft: limits::TABLE3_PA_FT.1 + EPS,
            cruise_oat_c: isa_temp_c_at_pa(limits::TABLE3_PA_FT.1),
            cruise_rpm: 2600.0,
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&data, &ooc),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 PA",
                ..
            })
        ));
    }

    #[test]
    fn landing_pa_epsilon() {
        let data = d();
        let mut ok = FlightInputs {
            pa_override: Some(PaOverride {
                pa_ft: limits::TABLE4_PA_FT.1,
                phase: ChartPhase::Landing,
            }),
            ..Default::default()
        };
        assert!(compute_landing(&data, &ok).is_ok());
        ok.pa_override = Some(PaOverride {
            pa_ft: limits::TABLE4_PA_FT.1 + EPS,
            phase: ChartPhase::Landing,
        });
        assert!(matches!(
            compute_landing(&data, &ok),
            Err(LandingError::OutOfChart(_))
        ));
    }

}
