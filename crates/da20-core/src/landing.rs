use crate::altitude::pressure_altitude_ft;
use crate::data::PerformanceData;
use crate::interpolation::{interpolate_2d, ExtrapolationPolicy};
use crate::provenance::provenance_for;
use crate::types::{FlightInputs, LandingResult};
use crate::wind::compute_wind;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LandingError {
    #[error("interpolation: {0}")]
    Interp(#[from] crate::interpolation::InterpError),
}

pub fn compute_landing(data: &PerformanceData, inputs: &FlightInputs) -> Result<LandingResult, LandingError> {
    let table = &data.landing.table;
    let pa = pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa);
    let y_axis: Vec<f64> = table.rows.iter().map(|r| r.oat_c).collect();
    let mut ground_grid = Vec::with_capacity(y_axis.len());
    let mut dist_grid = Vec::with_capacity(y_axis.len());
    for row in &table.rows {
        ground_grid.push(row.ground_roll_ft.clone());
        dist_grid.push(row.distance_50ft_ft.clone());
    }

    let (ground, mut warnings) = interpolate_2d(
        &table.pressure_altitude_ft,
        &y_axis,
        &ground_grid,
        pa,
        inputs.oat_c,
        ExtrapolationPolicy::Clamp,
    )?;
    let (dist50, w2) = interpolate_2d(
        &table.pressure_altitude_ft,
        &y_axis,
        &dist_grid,
        pa,
        inputs.oat_c,
        ExtrapolationPolicy::Clamp,
    )?;
    warnings.extend(w2);

    let wind = compute_wind(data, inputs).map_err(LandingError::Interp)?;
    warnings.extend(wind.warnings);
    let ground_adj = ground + wind.takeoff_landing_correction_ft;
    let dist_adj = dist50 + wind.takeoff_landing_correction_ft;

    if data.meta.is_synthetic() {
        warnings.push(crate::types::CalculationWarning {
            code: "SYNTHETIC_DATA".into(),
            message: "Landing distances from SYNTHETIC fixture — not for flight use.".into(),
        });
    }

    Ok(LandingResult {
        provenance: provenance_for(data, &data.landing.citation),
        ground_roll_ft: ground_adj,
        distance_over_50ft_ft: dist_adj,
        wind_correction_ft: wind.takeoff_landing_correction_ft,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_landing() {
        let data = PerformanceData::synthetic_fixture();
        let r = compute_landing(&data, &FlightInputs::default()).unwrap();
        assert!(r.ground_roll_ft > 0.0);
    }
}
