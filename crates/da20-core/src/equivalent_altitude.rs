use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::CalculationWarning;

pub fn equivalent_altitude_ft(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
) -> (f64, Vec<CalculationWarning>) {
    crate::rev29::engine::equivalent_altitude_ft(data.inner(), pressure_altitude_ft, oat_c)
}

pub fn equivalent_altitude_with_provenance(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
) -> (f64, crate::types::ProvenanceBlock, Vec<CalculationWarning>) {
    let (eq, warnings) = equivalent_altitude_ft(data, pressure_altitude_ft, oat_c);
    let prov = provenance_for(data.inner(), data.equivalent_altitude_source());
    (eq, prov, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altitude::{effective_pressure_altitude_ft, pressure_altitude_ft};
    use crate::data::PerformanceData;

    #[test]
    fn sctb_equivalent_altitude_finite() {
        let data = PerformanceData::production();
        let inputs = crate::types::FlightInputs::default();
        let pa = effective_pressure_altitude_ft(&inputs, None);
        let (eq, _) = equivalent_altitude_ft(&data, pa, 20.0);
        assert!(eq.is_finite());
        let pa2 = pressure_altitude_ft(2130.0, 1013.0);
        assert!((pa - pa2).abs() < 1.0);
    }
}
