use crate::cruise::compute_cruise;
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, EnduranceResult, FlightInputs};

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
    if cruise.mt_gph_out_of_chart {
        return Err(EnduranceError::CruiseFailed);
    }
    let tas_chart = cruise.tas_chart_kt.unwrap_or(cruise.tas_kt);
    let usable = inputs.fuel_usable_us_gal;
    let reserve = inputs.fuel_reserve_us_gal;
    let burn = (usable - reserve).max(0.0);
    let endurance_h = if cruise.fuel_flow_us_gal_h > 0.0 {
        burn / cruise.fuel_flow_us_gal_h
    } else {
        0.0
    };
    let range_nm = endurance_h * cruise.tas_kt;
    let range_chart_nm = endurance_h * tas_chart;

    let mut warnings = cruise.warnings;
    warnings.push(CalculationWarning::code_only("ENDURANCE_DERIVED"));
    warnings.push(CalculationWarning::code_only("AFM_FUEL_CAPACITY"));

    Ok(EnduranceResult {
        provenance: provenance_for(data.inner(), data.cruise_source()),
        afm_lookup_tag: cruise.afm_lookup_tag,
        fuel_flow_us_gal_h: cruise.fuel_flow_us_gal_h,
        tas_kt: cruise.tas_kt,
        tas_chart_kt: cruise.tas_chart_kt,
        endurance_h,
        range_nm,
        range_chart_nm: Some(range_chart_nm),
        usable_us_gal: usable,
        reserve_us_gal: reserve,
        warnings,
    })
}
