use crate::cruise::compute_cruise;
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, EnduranceResult, FlightInputs};

pub const ENDURANCE_DERIVED_NOTE: &str =
    "Endurance and range are derived from Table 3 fuel flow and user-entered usable fuel and reserve — not an AFM chart.";

pub const AFM_FUEL_CAPACITY_NOTE: &str =
    "AFM DA202-C1 Rev 29 Section 1 (aircraft specifications): total fuel capacity 24.0 US gal, usable 23.5 US gal.";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EnduranceError {
    #[error("cruise lookup failed")]
    CruiseFailed,
}

pub fn compute_endurance(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<EnduranceResult, EnduranceError> {
    let cruise = compute_cruise(data, inputs).map_err(|_| EnduranceError::CruiseFailed)?;
    let usable = inputs.fuel_usable_us_gal;
    let reserve = inputs.fuel_reserve_us_gal;
    let burn = (usable - reserve).max(0.0);
    let endurance_h = if cruise.fuel_flow_us_gal_h > 0.0 {
        burn / cruise.fuel_flow_us_gal_h
    } else {
        0.0
    };
    let range_nm = endurance_h * cruise.tas_kt;

    let mut warnings = cruise.warnings;
    warnings.push(CalculationWarning {
        code: "ENDURANCE_DERIVED".into(),
        message: ENDURANCE_DERIVED_NOTE.into(),
    });
    warnings.push(CalculationWarning {
        code: "AFM_FUEL_CAPACITY".into(),
        message: AFM_FUEL_CAPACITY_NOTE.into(),
    });

    Ok(EnduranceResult {
        provenance: provenance_for(data.inner(), data.cruise_source()),
        fuel_flow_us_gal_h: cruise.fuel_flow_us_gal_h,
        tas_kt: cruise.tas_kt,
        endurance_h,
        range_nm,
        usable_us_gal: usable,
        reserve_us_gal: reserve,
        warnings,
    })
}
