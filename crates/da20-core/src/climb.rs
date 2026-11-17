use crate::afm_tag::climb_tag_from_warnings;
use crate::altitude::effective_pressure_altitude_ft;
use crate::chart_bounds::ChartOutOfRange;
use crate::data::PerformanceData;
use crate::equivalent_altitude::equivalent_altitude_ft;
use crate::provenance::provenance_for;
use crate::rev29::engine::rate_of_climb_fpm;
use crate::types::{
    climb_roc_fairing_multiplier, CalculationWarning, ClimbFigure, ClimbResult, FlightInputs,
};

pub const CLIMB_DERIVED_ESTIMATE_NOTE: &str =
    "Climb time, fuel, and distance are derived estimates (no AFM chart).";

/// AFM §5.3 service ceiling: ROC falls to this value (fpm).
pub const AFM_SERVICE_CEILING_ROC_FPM: f64 = 100.0;

pub const CLIMB_FUEL_ESTIMATE_BASIS: &str =
    "Estimated fuel: Table 3 GPH at 2800 RPM (Sensenich), integrated along climb; climb is full throttle/full rich per AFM — estimate likely low vs rich mixture; no AFM climb-fuel chart.";

/// Whether displayed Vy follows AFM §4.2 or club operating practice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VySpeedPolicy {
    Afm,
    /// Club practice IAS; `afm_kias` is the AFM published value for reference.
    ClubPractice {
        afm_kias: f64,
    },
}

/// Vy (KIAS) held constant for still-air distance: AFM or club (T/O flaps).
pub fn vy_kias_for_figure(figure: ClimbFigure) -> (f64, VySpeedPolicy) {
    match figure {
        ClimbFigure::Fig55CruiseFlaps => (75.0, VySpeedPolicy::Afm),
        ClimbFigure::Fig56TakeoffFlaps => (65.0, VySpeedPolicy::ClubPractice { afm_kias: 68.0 }),
        ClimbFigure::Fig58BalkedLanding => (52.0, VySpeedPolicy::Afm),
    }
}

/// Vy CAS (kt) via AFM Table 1 interpolation for the matching flap curve.
pub fn afm_vy_cas_kt(figure: ClimbFigure) -> f64 {
    let (kias, _) = vy_kias_for_figure(figure);
    crate::airspeed_table1::table1_kcas_from_kias(kias, figure).unwrap_or(kias)
}

/// TAS (kt) with Vy held constant: Table 1 CAS → TAS via σ(PA, OAT).
pub fn tas_from_vy_cas_at_pa(cas_kt: f64, pa_ft: f64, oat_c: f64) -> f64 {
    let sigma = crate::altitude::density_ratio_sigma(pa_ft, oat_c);
    cas_kt / sigma.sqrt()
}

pub fn tas_from_vy_held(figure: ClimbFigure, pa_ft: f64, oat_c: f64) -> f64 {
    tas_from_vy_cas_at_pa(afm_vy_cas_kt(figure), pa_ft, oat_c)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ClimbError {
    #[error("climb segment unavailable")]
    MissingSegment,
    #[error(transparent)]
    OutOfChart(ChartOutOfRange),
}

pub fn compute_climb(
    data: &PerformanceData,
    inputs: &FlightInputs,
    target_pressure_alt_ft: f64,
) -> Result<ClimbResult, ClimbError> {
    let pa = effective_pressure_altitude_ft(inputs, Some(crate::types::ChartPhase::Climb));
    let (eq, mut warnings) =
        equivalent_altitude_ft(data, pa, inputs.oat_c).map_err(ClimbError::OutOfChart)?;
    let key = inputs.climb_figure.roc_key();
    let (roc_base, w) =
        rate_of_climb_fpm(data.inner(), key, pa, inputs.oat_c).map_err(ClimbError::OutOfChart)?;
    warnings.extend(w);
    let m = climb_roc_fairing_multiplier(inputs.wheel_fairings_installed);
    let roc = roc_base * m;

    let delta_ft = (target_pressure_alt_ft - pa).max(0.0);
    let time_min = if roc > 0.0 {
        Some(delta_ft / roc)
    } else {
        None
    };

    warnings.push(CalculationWarning::code_only("DERIVED_ESTIMATE"));
    let afm_lookup_tag = climb_tag_from_warnings(&warnings);

    Ok(ClimbResult {
        provenance: provenance_for(data.inner(), data.climb_roc_source()),
        afm_lookup_tag,
        equivalent_altitude_ft: eq,
        rate_of_climb_chart_fpm: Some(roc_base),
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
        .ok()
        .map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;
    use crate::rev29::engine::fahrenheit_to_celsius;
    use crate::types::climb_roc_fairing_multiplier;

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

    #[test]
    fn climb_roc_without_fairings_is_three_percent_below_chart() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            wheel_fairings_installed: false,
            ..Default::default()
        };
        let target = inputs.cruise_pressure_alt_ft;
        let r = compute_climb(&data, &inputs, target).expect("climb");
        let chart = r.rate_of_climb_chart_fpm.expect("chart ROC stored");
        let m = climb_roc_fairing_multiplier(false);
        assert!((m - 0.97).abs() < 1e-9);
        assert!((r.rate_of_climb_fpm - chart * m).abs() < 1e-6);
        assert!((r.rate_of_climb_fpm - 0.97 * chart).abs() < 1e-6);
    }
}
