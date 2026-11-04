use crate::data::PerformanceData;
use crate::interpolation::lerp;
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, CruiseResult, FlightInputs};

pub const CRUISE_TABLE3_GPH_NOTE: &str = "Table 3 GPH values are transcribed as printed in AFM Rev 29 (includes known oddities, e.g. 2000 ft / 2700 RPM ISA 6.8 GPH vs −20 °C 7.7 GPH; 4000 ft / 2800 RPM +10 °C 6.8 GPH).";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CruiseError {
    #[error("cruise row not found")]
    NotFound,
}

pub fn isa_temp_c_at_pa(pa_ft: f64) -> f64 {
    15.0 - 0.0019812 * pa_ft
}

#[derive(Clone, Copy)]
enum TempBand {
    DisaMinus20,
    Isa,
    DisaPlus10,
}

fn select_band(oat_c: f64, pa_ft: f64) -> TempBand {
    let isa = isa_temp_c_at_pa(pa_ft);
    let d = oat_c - isa;
    if d <= -10.0 {
        TempBand::DisaMinus20
    } else if d >= 5.0 {
        TempBand::DisaPlus10
    } else {
        TempBand::Isa
    }
}

fn band_columns(band: TempBand) -> (usize, usize, usize) {
    match band {
        TempBand::DisaMinus20 => (2, 3, 4),
        TempBand::Isa => (5, 6, 7),
        TempBand::DisaPlus10 => (8, 9, 10),
    }
}

fn row_matches_rpm(row: &[f64], rpm: f64) -> bool {
    row.len() >= 11 && (row[1] - rpm).abs() < 0.5
}

fn interpolate_cruise_at(
    rows: &[Vec<f64>],
    pa: f64,
    rpm: f64,
    oat_c: f64,
) -> Result<(f64, f64), CruiseError> {
    let band = select_band(oat_c, pa);
    let (_, tas_idx, gph_idx) = band_columns(band);

    let mut at_rpm: Vec<&Vec<f64>> = rows.iter().filter(|r| row_matches_rpm(r, rpm)).collect();
    if at_rpm.is_empty() {
        return Err(CruiseError::NotFound);
    }
    at_rpm.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());

    if at_rpm.len() == 1 || pa <= at_rpm[0][0] {
        let row = at_rpm[0];
        return Ok((row[tas_idx], row[gph_idx]));
    }
    if pa >= at_rpm[at_rpm.len() - 1][0] {
        let row = at_rpm[at_rpm.len() - 1];
        return Ok((row[tas_idx], row[gph_idx]));
    }

    let mut upper = 1usize;
    while upper < at_rpm.len() && at_rpm[upper][0] < pa {
        upper += 1;
    }
    let lower = at_rpm[upper - 1];
    let upper = at_rpm[upper];
    let tas = lerp(lower[0], lower[tas_idx], upper[0], upper[tas_idx], pa);
    let gph = lerp(lower[0], lower[gph_idx], upper[0], upper[gph_idx], pa);
    Ok((tas, gph))
}

pub fn compute_cruise(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    let pa = inputs.cruise_pressure_alt_ft;
    let rpm = inputs.cruise_rpm;
    let rows = &data.inner().cruise_table.value.rows;
    let (tas, ff) = interpolate_cruise_at(rows, pa, rpm, inputs.cruise_oat_c)?;

    let warnings = vec![CalculationWarning::code_only("TABLE3_GPH")];

    Ok(CruiseResult {
        provenance: provenance_for(data.inner(), data.cruise_source()),
        tas_kt: tas,
        fuel_flow_us_gal_h: ff,
        warnings,
    })
}

pub fn lookup_cruise_row(data: &PerformanceData, pa: f64, rpm: f64) -> Option<&Vec<f64>> {
    data.inner()
        .cruise_table
        .value
        .rows
        .iter()
        .find(|r| r.len() >= 11 && (r[0] - pa).abs() < 0.5 && (r[1] - rpm).abs() < 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;

    #[test]
    fn cruise_interpolates_at_6500ft() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            ..FlightInputs::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        assert!(r.tas_kt > 110.0);
    }

    #[test]
    fn cruise_ignores_aerodrome_oat() {
        let data = PerformanceData::production();
        let mut inputs = FlightInputs {
            oat_c: 40.0,
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            ..FlightInputs::default()
        };
        let r1 = compute_cruise(&data, &inputs).unwrap();
        inputs.oat_c = -10.0;
        let r2 = compute_cruise(&data, &inputs).unwrap();
        assert!((r1.tas_kt - r2.tas_kt).abs() < 1e-9);
    }
}
