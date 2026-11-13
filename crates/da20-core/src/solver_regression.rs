//! Fixed baseline outputs for default planning inputs (guards unrelated numeric drift).

#[cfg(test)]
mod tests {
    use crate::climb::compute_climb;
    use crate::cruise::compute_cruise;
    use crate::data::PerformanceData;
    use crate::landing::compute_landing;
    use crate::takeoff::compute_takeoff;
    use crate::types::{defaults, ChartPhase, FlightInputs, PaOverride};

    #[test]
    fn baseline_takeoff_climb_cruise_landing_unchanged() {
        let data = PerformanceData::production();

        let takeoff = FlightInputs {
            pa_override: Some(PaOverride {
                pa_ft: 1000.0,
                phase: ChartPhase::Takeoff,
            }),
            oat_c: 21.0,
            weight_lb: defaults::TAKEOFF_REF_WEIGHT_KG / 0.45359237,
            ..Default::default()
        };
        let to = compute_takeoff(&data, &takeoff).unwrap();
        assert!((to.ground_roll_ft - 942.2494233660218).abs() < 1e-6);
        assert!((to.distance_over_obstacle_raw_ft - 1202.6218022726364).abs() < 1e-6);

        let climb = compute_climb(
            &data,
            &FlightInputs::default(),
            defaults::CRUISE_PRESSURE_ALT_FT,
        )
        .unwrap();
        assert!((climb.rate_of_climb_fpm - 792.5479212).abs() < 1e-6);

        let cruise = compute_cruise(&data, &FlightInputs::default()).unwrap();
        assert!((cruise.tas_kt - 116.0).abs() < 1e-6);
        assert!((cruise.fuel_flow_us_gal_h - 5.5249999999999995).abs() < 1e-6);

        let landing = FlightInputs {
            pa_override: Some(PaOverride {
                pa_ft: 2000.0,
                phase: ChartPhase::Landing,
            }),
            ..Default::default()
        };
        let ld = compute_landing(&data, &landing).unwrap();
        assert!((ld.ground_roll_ft - 750.07).abs() < 1e-6);
        assert!((ld.distance_over_50ft_ft - 1487.8500000000001).abs() < 1e-6);
    }
}
