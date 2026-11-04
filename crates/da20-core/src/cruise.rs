use crate::data::PerformanceData;
use crate::interpolation::{interpolate_1d, interpolate_2d, ExtrapolationPolicy};
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, CruiseResult, FlightInputs};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CruiseError {
    #[error("interpolation: {0}")]
    Interp(#[from] crate::interpolation::InterpError),
}

pub fn compute_cruise(data: &PerformanceData, inputs: &FlightInputs) -> Result<CruiseResult, CruiseError> {
    let table = &data.cruise.table;
    let y_axis: Vec<f64> = table.rows.iter().map(|r| r.oat_c).collect();

    let n_pa = table.pressure_altitude_ft.len();
    let n_rpm = table.rpm.len();
    let mut tas_grid = Vec::with_capacity(y_axis.len());
    let mut ff_grid = Vec::with_capacity(y_axis.len());
    for row in &table.rows {
        let mut tas_row = Vec::with_capacity(n_pa);
        let mut ff_row = Vec::with_capacity(n_pa);
        for pa_idx in 0..n_pa {
            let base = pa_idx * n_rpm;
            tas_row.push(row.tas_kt.get(base).copied().unwrap_or(0.0));
            ff_row.push(row.fuel_flow_us_gal_h.get(base).copied().unwrap_or(0.0));
        }
        tas_grid.push(tas_row);
        ff_grid.push(ff_row);
    }

    let pa = inputs.cruise_pressure_alt_ft;
    let rpm = inputs.cruise_rpm;
    let oat = inputs.oat_c;

    let (tas, mut warnings) = interpolate_2d(
        &table.pressure_altitude_ft,
        &y_axis,
        &tas_grid,
        pa,
        oat,
        ExtrapolationPolicy::Clamp,
    )?;

    let rpm_axis = &table.rpm;
    let tas_at_rpm: Vec<f64> = (0..y_axis.len())
        .map(|yi| {
            interpolate_1d(rpm_axis, &tas_grid[yi], rpm, ExtrapolationPolicy::Clamp)
                .map(|(v, _)| v)
                .unwrap_or(tas_grid[yi][0])
        })
        .collect();
    let (tas_final, w2) = interpolate_1d(&y_axis, &tas_at_rpm, oat, ExtrapolationPolicy::Clamp)?;
    warnings.extend(w2);

    let ff_at_rpm: Vec<f64> = (0..y_axis.len())
        .map(|yi| {
            interpolate_1d(rpm_axis, &ff_grid[yi], rpm, ExtrapolationPolicy::Clamp)
                .map(|(v, _)| v)
                .unwrap_or(ff_grid[yi][0])
        })
        .collect();
    let (ff, w3) = interpolate_1d(&y_axis, &ff_at_rpm, oat, ExtrapolationPolicy::Clamp)?;
    warnings.extend(w3);

    let _ = tas;
    if data.meta.is_synthetic() {
        warnings.push(CalculationWarning {
            code: "SYNTHETIC_DATA".into(),
            message: "Cruise values from SYNTHETIC fixture — not for flight use.".into(),
        });
    }

    Ok(CruiseResult {
        provenance: provenance_for(data, &data.cruise.citation),
        tas_kt: tas_final,
        fuel_flow_us_gal_h: ff,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_cruise() {
        let data = PerformanceData::synthetic_fixture();
        let r = compute_cruise(&data, &FlightInputs::default()).unwrap();
        assert!(r.tas_kt > 0.0);
    }
}
