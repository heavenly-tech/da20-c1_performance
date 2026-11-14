use crate::cruise::compute_cruise;
use crate::data::PerformanceData;
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
        provenance: cruise.provenance.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cruise::{compute_cruise, isa_temp_c_at_pa};
    use crate::types::{CruiseSolveMode, FlightInputs, PropellerKind};

    fn inputs_pa_pct(pa: f64, pct: f64, prop: PropellerKind) -> FlightInputs {
        FlightInputs {
            propeller: prop,
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: isa_temp_c_at_pa(pa),
            cruise_percent_bhp: pct,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        }
    }

    #[test]
    fn mt_endurance_8000_55_matches_mt_cruise_chain() {
        let data = PerformanceData::production();
        let inputs = inputs_pa_pct(8000.0, 55.0, PropellerKind::Mt175R);
        let sensenich = inputs_pa_pct(8000.0, 55.0, PropellerKind::Sensenich);
        let cruise = compute_cruise(&data, &inputs).expect("MT in chart");
        let end = compute_endurance(&data, &inputs).expect("endurance");
        let s_cruise = compute_cruise(&data, &sensenich).unwrap();
        assert!(
            (cruise.fuel_flow_us_gal_h - s_cruise.fuel_flow_us_gal_h).abs() > 1e-6
                || (cruise.tas_kt - s_cruise.tas_kt).abs() > 0.1,
            "MT chain must not reuse Table3 GPH/TAS: mt gph={} tas={} vs table3 gph={} tas={}",
            cruise.fuel_flow_us_gal_h,
            cruise.tas_kt,
            s_cruise.fuel_flow_us_gal_h,
            s_cruise.tas_kt,
        );
        assert!(!cruise.mt_gph_out_of_chart);
        assert!(
            (end.fuel_flow_us_gal_h - cruise.fuel_flow_us_gal_h).abs() < 1e-9,
            "gph"
        );
        let expected_h =
            (inputs.fuel_usable_us_gal - inputs.fuel_reserve_us_gal) / cruise.fuel_flow_us_gal_h;
        assert!((end.endurance_h - expected_h).abs() < 1e-6);
        assert!((end.range_nm - end.endurance_h * cruise.tas_kt).abs() < 1e-6);
        assert!(cruise.table3_source.is_none());
    }

    #[test]
    fn mt_endurance_off_chart_no_numeric_result() {
        let data = PerformanceData::production();
        let inputs = inputs_pa_pct(12_000.0, 65.0, PropellerKind::Mt175R);
        let cruise = compute_cruise(&data, &inputs).unwrap();
        assert!(cruise.mt_gph_out_of_chart);
        assert!(compute_endurance(&data, &inputs).is_err());
    }

    #[test]
    fn sensenich_endurance_uses_table3_cruise() {
        let data = PerformanceData::production();
        let inputs = inputs_pa_pct(8000.0, 55.0, PropellerKind::Sensenich);
        let cruise = compute_cruise(&data, &inputs).unwrap();
        let end = compute_endurance(&data, &inputs).unwrap();
        assert!(cruise.table3_source.is_some());
        assert!((end.fuel_flow_us_gal_h - cruise.fuel_flow_us_gal_h).abs() < 1e-9);
    }

    #[test]
    fn mt_endurance_provenance_matches_mt_cruise_not_table3() {
        let data = PerformanceData::production();
        let inputs = inputs_pa_pct(8000.0, 55.0, PropellerKind::Mt175R);
        let cruise = compute_cruise(&data, &inputs).unwrap();
        let end = compute_endurance(&data, &inputs).unwrap();
        assert_eq!(
            end.provenance.citation.figure_or_table,
            cruise.provenance.citation.figure_or_table
        );
        assert!(!end.provenance.citation.figure_or_table.contains("Table 3"));
    }
}
