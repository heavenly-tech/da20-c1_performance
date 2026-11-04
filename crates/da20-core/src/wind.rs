use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::{FlightInputs, WindComponents, WindResult};

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

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum WindError {
    #[error("wind calculation failed")]
    Failed,
}

pub fn compute_wind(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<WindResult, WindError> {
    let rwy = runway_heading_deg(&inputs.runway).unwrap_or(0.0);
    let components = wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy);
    Ok(WindResult {
        provenance: provenance_for(data.inner(), data.wind_source()),
        components,
        takeoff_landing_correction_ft: 0.0,
        warnings: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fig_5_3_worked_example_components() {
        let c = wind_components(30.0, 11.0, 0.0);
        assert!((c.headwind_kt - 9.5).abs() <= 0.1);
        assert!((c.crosswind_kt - 5.5).abs() <= 0.1);
    }
}
