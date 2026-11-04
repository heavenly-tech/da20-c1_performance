use crate::data::PerformanceData;
use crate::interpolation::{interpolate_2d, ExtrapolationPolicy};
use crate::provenance::provenance_for;
use crate::types::CalculationWarning;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EqAltError {
    #[error("interpolation: {0}")]
    Interp(#[from] crate::interpolation::InterpError),
}

/// POH Figure 5.2 — equivalent altitude for climb performance.
pub fn equivalent_altitude_ft(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
    policy: ExtrapolationPolicy,
) -> Result<(f64, Vec<CalculationWarning>), EqAltError> {
    let grid = &data.equivalent_altitude.grid;
    let x_axis = &grid.pressure_altitude_ft;
    let y_axis: Vec<f64> = grid.rows.iter().map(|r| r.oat_c).collect();
    let values: Vec<Vec<f64>> = grid
        .rows
        .iter()
        .map(|r| r.equivalent_altitude_ft.clone())
        .collect();

    let (eq, mut warnings) = interpolate_2d(x_axis, &y_axis, &values, pressure_altitude_ft, oat_c, policy)?;
    if data.meta.is_synthetic() {
        warnings.push(CalculationWarning {
            code: "SYNTHETIC_DATA".into(),
            message: "Equivalent altitude from SYNTHETIC fixture — not for flight use.".into(),
        });
    }
    Ok((eq, warnings))
}

pub fn equivalent_altitude_with_provenance(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
) -> Result<(f64, crate::types::ProvenanceBlock, Vec<CalculationWarning>), EqAltError> {
    let (eq, warnings) =
        equivalent_altitude_ft(data, pressure_altitude_ft, oat_c, ExtrapolationPolicy::Clamp)?;
    let prov = provenance_for(data, &data.equivalent_altitude.citation);
    Ok((eq, prov, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altitude::pressure_altitude_ft;

    #[test]
    fn synthetic_equivalent_altitude_at_sctb() {
        let data = PerformanceData::synthetic_fixture();
        let pa = pressure_altitude_ft(2130.0, 1013.0);
        let (eq, _) = equivalent_altitude_ft(&data, pa, 20.0, ExtrapolationPolicy::Clamp).unwrap();
        assert!(eq.is_finite());
    }
}
