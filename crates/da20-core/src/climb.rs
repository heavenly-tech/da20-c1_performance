use crate::data::PerformanceData;
use crate::equivalent_altitude::equivalent_altitude_ft;
use crate::interpolation::{interpolate_1d, ExtrapolationPolicy};
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, ClimbResult, FlightInputs};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ClimbError {
    #[error("equivalent altitude: {0}")]
    EqAlt(#[from] crate::equivalent_altitude::EqAltError),
    #[error("interpolation: {0}")]
    Interp(#[from] crate::interpolation::InterpError),
    #[error("missing climb segment {0}")]
    MissingSegment(String),
}

fn segment_by_figure<'a>(
    data: &'a PerformanceData,
    figure: &str,
) -> Result<&'a crate::data::ClimbSegment, ClimbError> {
    data.climb
        .segments
        .iter()
        .find(|s| s.figure == figure)
        .ok_or_else(|| ClimbError::MissingSegment(figure.to_string()))
}

pub fn compute_climb(
    data: &PerformanceData,
    inputs: &FlightInputs,
    target_pressure_alt_ft: f64,
) -> Result<ClimbResult, ClimbError> {
    use crate::altitude::pressure_altitude_ft;

    let pa = pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa);
    let (eq, mut warnings) =
        equivalent_altitude_ft(data, pa, inputs.oat_c, ExtrapolationPolicy::Clamp)?;
    let (target_eq, w2) =
        equivalent_altitude_ft(data, target_pressure_alt_ft, inputs.oat_c, ExtrapolationPolicy::Clamp)?;
    warnings.extend(w2);

    let seg_rate = segment_by_figure(data, "5.5")?;
    let (roc, w3) = interpolate_1d(
        &seg_rate.equivalent_altitude_ft,
        &seg_rate.rate_of_climb_fpm,
        eq,
        ExtrapolationPolicy::Clamp,
    )?;
    warnings.extend(w3);

    let seg_profile = segment_by_figure(data, "5.8").ok();
    let mut time_min = None;
    let mut fuel = None;
    let mut distance = None;
    if let Some(seg) = seg_profile {
        if !seg.time_min.is_empty() {
            let (t, w) = interpolate_1d(&seg.equivalent_altitude_ft, &seg.time_min, target_eq, ExtrapolationPolicy::Clamp)?;
            time_min = Some(t);
            warnings.extend(w);
        }
        if !seg.fuel_us_gal.is_empty() {
            let (f, w) = interpolate_1d(&seg.equivalent_altitude_ft, &seg.fuel_us_gal, target_eq, ExtrapolationPolicy::Clamp)?;
            fuel = Some(f);
            warnings.extend(w);
        }
        if !seg.distance_nm.is_empty() {
            let (d, w) = interpolate_1d(&seg.equivalent_altitude_ft, &seg.distance_nm, target_eq, ExtrapolationPolicy::Clamp)?;
            distance = Some(d);
            warnings.extend(w);
        }
    }

    if data.meta.is_synthetic() {
        warnings.push(CalculationWarning {
            code: "SYNTHETIC_DATA".into(),
            message: "Climb performance from SYNTHETIC fixture — not for flight use.".into(),
        });
    }

    Ok(ClimbResult {
        provenance: provenance_for(data, &data.climb.citations.figure_5_5),
        equivalent_altitude_ft: eq,
        rate_of_climb_fpm: roc,
        time_to_altitude_min: time_min,
        fuel_to_altitude_us_gal: fuel,
        distance_to_altitude_nm: distance,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_climb() {
        let data = PerformanceData::synthetic_fixture();
        let inputs = FlightInputs::default();
        let r = compute_climb(&data, &inputs, inputs.cruise_pressure_alt_ft).unwrap();
        assert!(r.rate_of_climb_fpm > 0.0);
    }
}
