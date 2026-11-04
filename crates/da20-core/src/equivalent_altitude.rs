use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::CalculationWarning;

pub fn equivalent_altitude_ft(
    data: &PerformanceData,
    pressure_altitude_ft: f64,
    oat_c: f64,
) -> (f64, Vec<CalculationWarning>) {
    let eq = crate::rev29::engine::equivalent_altitude_ft(data.inner(), pressure_altitude_ft, oat_c);
    (eq, vec![])
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
    use crate::altitude::pressure_altitude_ft;
    use crate::data::PerformanceData;

    #[test]
    fn sctb_equivalent_altitude_finite() {
        let data = PerformanceData::production();
        let pa = pressure_altitude_ft(2130.0, 1013.0);
        let (eq, _) = equivalent_altitude_ft(&data, pa, 20.0);
        assert!(eq.is_finite());
    }
}
