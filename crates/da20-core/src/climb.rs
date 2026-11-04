use crate::altitude::pressure_altitude_ft;
use crate::data::PerformanceData;
use crate::equivalent_altitude::equivalent_altitude_ft;
use crate::provenance::provenance_for;
use crate::rev29::engine::rate_of_climb_fpm;
use crate::types::{CalculationWarning, ClimbResult, FlightInputs};

pub const CLIMB_DERIVED_ESTIMATE_NOTE: &str =
    "Climb time, fuel, and distance are derived estimates (no AFM chart).";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ClimbError {
    #[error("climb segment unavailable")]
    MissingSegment,
}

pub fn compute_climb(
    data: &PerformanceData,
    inputs: &FlightInputs,
    target_pressure_alt_ft: f64,
) -> Result<ClimbResult, ClimbError> {
    let pa = pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa);
    let (eq, _) = equivalent_altitude_ft(data, pa, inputs.oat_c);
    let roc = rate_of_climb_fpm(data.inner(), "cruise_5_5", pa, inputs.oat_c)
        .ok_or(ClimbError::MissingSegment)?;

    let delta_ft = (target_pressure_alt_ft - pa).max(0.0);
    let time_min = if roc > 0.0 {
        Some(delta_ft / roc)
    } else {
        None
    };

    let warnings = vec![CalculationWarning {
        code: "DERIVED_ESTIMATE".into(),
        message: CLIMB_DERIVED_ESTIMATE_NOTE.into(),
    }];

    Ok(ClimbResult {
        provenance: provenance_for(data.inner(), data.climb_roc_source()),
        equivalent_altitude_ft: eq,
        rate_of_climb_fpm: roc,
        time_to_altitude_min: time_min,
        fuel_to_altitude_us_gal: None,
        distance_to_altitude_nm: None,
        warnings,
    })
}

pub fn rate_of_climb_for_figure(
    data: &PerformanceData,
    figure_key: &str,
    pa_ft: f64,
    oat_c: f64,
) -> Option<f64> {
    rate_of_climb_fpm(data.inner(), figure_key, pa_ft, oat_c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rev29::engine::fahrenheit_to_celsius;
    use crate::data::PerformanceData;

    fn assert_roc(key: &str, pa: f64, oat_c: f64, expected: f64) {
        let data = PerformanceData::production();
        let roc = rate_of_climb_for_figure(&data, key, pa, oat_c).unwrap();
        let err_pct = (roc - expected).abs() / expected * 100.0;
        assert!(err_pct <= 1.0, "roc {roc} vs {expected} ({err_pct}%)");
    }

    #[test]
    fn roc_figures_within_one_percent() {
        let t = fahrenheit_to_celsius(65.0);
        assert_roc("cruise_5_5", 2000.0, t, 800.0);
        assert_roc("takeoff_5_6", 2000.0, t, 744.0);
        assert_roc("balked_5_8", 2000.0, fahrenheit_to_celsius(70.0), 374.0);
    }
}
