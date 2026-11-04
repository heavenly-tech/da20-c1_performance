use crate::altitude::effective_pressure_altitude_ft;
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::rev29::engine::{lb_to_kg, m_to_ft, takeoff_distances_m};
use crate::types::{CalculationWarning, FlightInputs, TakeoffResult};
use crate::wind::runway_heading_deg;

pub const TAKEOFF_OBSTACLE_DIGITIZATION_NOTE: &str = "Digitized Fig 5.4 sheet 2 reads ~2.7% short vs AFM Rev 29 worked example (1200 ft / 365.76 m at 1000 ft PA, 21 °C, 725 kg, 4 kt HW, 5 m obstacle).";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TakeoffError {
    #[error("takeoff calculation failed")]
    Failed,
}

pub fn compute_takeoff(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<TakeoffResult, TakeoffError> {
    let pa = effective_pressure_altitude_ft(inputs, Some(crate::types::ChartPhase::Takeoff));
    let rwy = runway_heading_deg(&inputs.runway).unwrap_or(0.0);
    let angle = (inputs.wind_dir_deg - rwy).to_radians();
    let headwind = inputs.wind_speed_kt * angle.cos();
    let headwind_kt = headwind.max(0.0);
    let tailwind_kt = (-headwind).max(0.0);

    let weight_kg = lb_to_kg(inputs.weight_lb);
    let dist = takeoff_distances_m(
        data.inner(),
        pa,
        inputs.oat_c,
        weight_kg,
        headwind_kt,
        tailwind_kt,
        inputs.obstacle_height_m,
    );

    let ground_ft = m_to_ft(dist.ground_roll_m);
    let over_ft = m_to_ft(dist.over_obstacle_m);
    let wind_delta_ft = m_to_ft(dist.ground_roll_m - dist.after_weight_m);

    let mut warnings = dist.warnings;
    warnings.push(CalculationWarning::code_only("TAKEOFF_OBSTACLE_BIAS"));
    warnings.push(CalculationWarning::code_only("UNITS"));

    Ok(TakeoffResult {
        provenance: provenance_for(data.inner(), data.takeoff_source()),
        pressure_altitude_ft: pa,
        ground_roll_ft: ground_ft,
        distance_over_50ft_ft: over_ft,
        wind_correction_ft: wind_delta_ft,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use crate::data::PerformanceData;
    use crate::rev29::engine::{fahrenheit_to_celsius, takeoff_distances_m};

    #[test]
    fn takeoff_worked_example_within_five_percent() {
        let data = PerformanceData::production();
        let d = takeoff_distances_m(
            data.inner(),
            1000.0,
            fahrenheit_to_celsius(72.0),
            725.0,
            4.0,
            0.0,
            5.0,
        );
        let err_pct = (d.over_obstacle_m - 365.76) / 365.76 * 100.0;
        assert!(err_pct > -5.0 && err_pct < 5.0, "err {err_pct}%");
        assert!(err_pct < 0.0, "expected short bias, got {err_pct}%");
    }
}
