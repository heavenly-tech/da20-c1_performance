use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::CalculationWarning;

pub fn equivalent_altitude_ft(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
) -> Result<(f64, Vec<CalculationWarning>), crate::chart_bounds::ChartOutOfRange> {
    crate::rev29::engine::equivalent_altitude_ft(data.inner(), pressure_altitude_ft, oat_c)
}

pub fn equivalent_altitude_with_provenance(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
) -> Result<
    (f64, crate::types::ProvenanceBlock, Vec<CalculationWarning>),
    crate::chart_bounds::ChartOutOfRange,
> {
    let (eq, warnings) = equivalent_altitude_ft(data, pressure_altitude_ft, oat_c)?;
    let prov = provenance_for(data.inner(), data.equivalent_altitude_source());
    Ok((eq, prov, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altitude::effective_pressure_altitude_ft;
    use crate::data::PerformanceData;

    #[test]
    fn sctb_equivalent_altitude_finite() {
        let data = PerformanceData::production();
        let inputs = crate::types::FlightInputs::default();
        let pa = effective_pressure_altitude_ft(&inputs, None);
        let (eq, _) = equivalent_altitude_ft(&data, pa, 20.0).unwrap();
        assert!(eq.is_finite());
        assert!((pa - inputs.pressure_altitude_input_ft).abs() < f64::EPSILON);
    }
}
