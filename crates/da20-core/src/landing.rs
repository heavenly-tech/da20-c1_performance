use crate::altitude::pressure_altitude_ft;
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::rev29::engine::{landing_lookup_ft, m_to_ft, wind_panel_hw_m, wind_panel_tw_m};
use crate::types::{CalculationWarning, FlightInputs, LandingResult};
use crate::wind::runway_heading_deg;

pub const LANDING_TABLE_50FT_M_NOTE: &str =
    "Table 4 lists 1360 ft (≈415 m rounded) at sea level; AFM narrative text cites 414 m for the same condition.";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LandingError {
    #[error("landing lookup failed")]
    Failed,
}

pub fn compute_landing(data: &PerformanceData, inputs: &FlightInputs) -> Result<LandingResult, LandingError> {
    let pa = pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa);
    let (roll_ft, over_50_ft) = landing_lookup_ft(data.inner(), pa);

    let rwy = runway_heading_deg(&inputs.runway).unwrap_or(0.0);
    let angle = (inputs.wind_dir_deg - rwy).to_radians();
    let headwind = inputs.wind_speed_kt * angle.cos();
    let headwind_kt = headwind.max(0.0);
    let tailwind_kt = (-headwind).max(0.0);

    // Wind correction via Fig 5.4 wind panels applied to table ground roll baseline (meters).
    let base_m = roll_ft / 3.280839895;
    let mut corrected_m = base_m;
    if headwind_kt > 0.0 {
        corrected_m = wind_panel_hw_m(data.inner(), base_m, headwind_kt);
    } else if tailwind_kt > 0.0 {
        corrected_m = wind_panel_tw_m(data.inner(), base_m, tailwind_kt);
    }
    let wind_delta_ft = m_to_ft(corrected_m - base_m);
    let ground_ft = roll_ft + wind_delta_ft;
    let dist_ft = over_50_ft + wind_delta_ft;

    let warnings = vec![CalculationWarning {
        code: "LANDING_TABLE_50FT".into(),
        message: LANDING_TABLE_50FT_M_NOTE.into(),
    }];

    Ok(LandingResult {
        provenance: provenance_for(data.inner(), data.landing_source()),
        ground_roll_ft: ground_ft,
        distance_over_50ft_ft: dist_ft,
        wind_correction_ft: wind_delta_ft,
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
