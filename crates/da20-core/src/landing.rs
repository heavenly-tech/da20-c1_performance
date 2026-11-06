use crate::altitude::effective_pressure_altitude_ft;
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::rev29::engine::landing_lookup_ft;
use crate::afm_tag::landing_tag_from_pa;
use crate::types::{CalculationWarning, FlightInputs, LandingResult};

pub const LANDING_TABLE_50FT_M_NOTE: &str =
    "Table 4 lists 1360 ft (≈415 m rounded) at sea level; AFM narrative text cites 414 m for the same condition.";

pub const LANDING_NO_CORRECTIONS_NOTE: &str =
    "Table 4 has no wind, temperature, or weight corrections — values are pressure-altitude lookup only.";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LandingError {
    #[error("landing lookup failed")]
    Failed,
}

pub fn compute_landing(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<LandingResult, LandingError> {
    let pa = effective_pressure_altitude_ft(inputs, Some(crate::types::ChartPhase::Landing));
    let (roll_ft, over_50_ft, mut warnings) = landing_lookup_ft(data.inner(), pa);

    warnings.push(CalculationWarning::code_only("LANDING_TABLE_50FT"));
    warnings.push(CalculationWarning::code_only("LANDING_NO_WIND"));
    let afm_lookup_tag =
        landing_tag_from_pa(&warnings, pa, &data.inner().land_alt.value);

    Ok(LandingResult {
        provenance: provenance_for(data.inner(), data.landing_source()),
        afm_lookup_tag,
        ground_roll_ft: roll_ft,
        distance_over_50ft_ft: over_50_ft,
        wind_correction_ft: 0.0,
        warnings,
    })
}

pub fn table_landing_50ft_ft(data: &PerformanceData, pa_ft: f64) -> f64 {
    landing_lookup_ft(data.inner(), pa_ft).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;

    #[test]
    fn table4_sea_level_exact_lookup() {
        let data = PerformanceData::production();
        assert_eq!(table_landing_50ft_ft(&data, 0.0), 1360.0);
        let m = 1360.0 * 0.3048;
        assert!((m - 414.528_f64).abs() < 0.01);
    }
}
