use crate::data::PerformanceData;
use crate::interpolation::{interpolate_1d, ExtrapolationPolicy};
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, FlightInputs, WindComponents, WindResult};

/// Runway heading in degrees true/magnetic (per AFM examples — user aligns with airfield data).
pub fn runway_heading_deg(runway: &str) -> Option<f64> {
    let digits: String = runway.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let base: f64 = digits.parse().ok()?;
    Some(base * 10.0)
}

pub fn wind_components(
    wind_dir_deg: f64,
    wind_speed_kt: f64,
    runway_heading_deg: f64,
) -> WindComponents {
    let angle_deg = (wind_dir_deg - runway_heading_deg).to_radians();
    let headwind = wind_speed_kt * angle_deg.cos();
    let crosswind = wind_speed_kt * angle_deg.sin();
    WindComponents {
        headwind_kt: headwind,
        crosswind_kt: crosswind,
        along_runway_headwind_kt: headwind,
    }
}

pub fn takeoff_landing_wind_correction_ft(
    data: &PerformanceData,
    headwind_component_kt: f64,
) -> Result<(f64, Vec<CalculationWarning>), crate::interpolation::InterpError> {
    let model = &data.wind.correction_model;
    let grid = model
        .grid
        .as_ref()
        .ok_or(crate::interpolation::InterpError::EmptyAxis)?;
    interpolate_1d(
        &grid.headwind_component_kt,
        &grid.correction_ft,
        headwind_component_kt,
        ExtrapolationPolicy::Clamp,
    )
}

pub fn compute_wind(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<WindResult, crate::interpolation::InterpError> {
    let rwy = runway_heading_deg(&inputs.runway).unwrap_or(0.0);
    let components = wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy);
    let (correction, mut warnings) =
        takeoff_landing_wind_correction_ft(data, components.along_runway_headwind_kt)?;
    if data.meta.is_synthetic() {
        warnings.push(CalculationWarning {
            code: "SYNTHETIC_DATA".into(),
            message: "Wind correction from SYNTHETIC fixture — not for flight use.".into(),
        });
    }
    Ok(WindResult {
        provenance: provenance_for(data, &data.wind.citation),
        components,
        takeoff_landing_correction_ft: correction,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_headwind() {
        let c = wind_components(190.0, 10.0, 190.0);
        assert!((c.headwind_kt - 10.0).abs() < 1e-6);
        assert!(c.crosswind_kt.abs() < 1e-6);
    }

    #[test]
    fn crosswind_from_left() {
        let c = wind_components(280.0, 10.0, 190.0);
        assert!(c.crosswind_kt.abs() > 0.0);
    }
}
