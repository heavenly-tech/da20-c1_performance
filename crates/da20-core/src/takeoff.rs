use crate::afm_tag::takeoff_tag;
use crate::altitude::effective_pressure_altitude_ft;
use crate::chart_bounds::ChartOutOfRange;
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::rev29::engine::{ft_to_m, lb_to_kg, m_to_ft, takeoff_distances_m};
use crate::types::{FlightInputs, TakeoffResult};
use crate::wind::runway_heading_deg;

pub const TAKEOFF_AFM_ROUNDING_NOTE: &str = "AFM worked examples often round chart distances up to the next 50 ft. The planner shows the raw Fig 5.4 chain result and a conservative value (next 50 ft up, with meters converted from that ft value).";

/// Round up to the next 50 ft (AFM-style conservative distance).
pub fn afm_conservative_distance_ft(raw_ft: f64) -> f64 {
    (raw_ft / 50.0).ceil() * 50.0
}

/// Meters equivalent of the conservative ft value (no separate m rounding).
pub fn afm_conservative_distance_m_from_ft(conservative_ft: f64) -> f64 {
    ft_to_m(conservative_ft)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TakeoffError {
    #[error("invalid runway heading")]
    InvalidRunwayHeading,
    #[error(transparent)]
    OutOfChart(ChartOutOfRange),
}

pub fn compute_takeoff(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<TakeoffResult, TakeoffError> {
    if crate::wind::runway_heading_status(&inputs.runway) != crate::wind::RunwayHeadingStatus::Valid
    {
        return Err(TakeoffError::InvalidRunwayHeading);
    }
    let pa = effective_pressure_altitude_ft(inputs, Some(crate::types::ChartPhase::Takeoff));
    let rwy = runway_heading_deg(&inputs.runway).expect("validated above");
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
    )
    .map_err(TakeoffError::OutOfChart)?;

    let ground_ft = m_to_ft(dist.ground_roll_m);
    let over_raw_m = dist.over_obstacle_m;
    let over_raw_ft = m_to_ft(over_raw_m);
    let over_conservative_ft = afm_conservative_distance_ft(over_raw_ft);
    let over_conservative_m = afm_conservative_distance_m_from_ft(over_conservative_ft);
    let wind_delta_ft = m_to_ft(dist.ground_roll_m - dist.after_weight_m);

    let warnings = dist.warnings;
    let afm_lookup_tag = takeoff_tag();

    Ok(TakeoffResult {
        provenance: provenance_for(data.inner(), data.takeoff_source()),
        afm_lookup_tag,
        pressure_altitude_ft: pa,
        ground_roll_ft: ground_ft,
        distance_over_obstacle_raw_m: over_raw_m,
        distance_over_obstacle_raw_ft: over_raw_ft,
        distance_over_obstacle_conservative_m: over_conservative_m,
        distance_over_obstacle_conservative_ft: over_conservative_ft,
        distance_over_50ft_ft: over_raw_ft,
        wind_correction_ft: wind_delta_ft,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;
    use crate::rev29::engine::takeoff_distances_m;

    #[test]
    fn takeoff_worked_example_raw_and_afm_rounding() {
        let data = PerformanceData::production();
        let d = takeoff_distances_m(data.inner(), 1000.0, 21.0, 725.0, 4.0, 0.0, 5.0).unwrap();
        let raw_ft = m_to_ft(d.over_obstacle_m);
        assert!(
            (raw_ft - 1154.0).abs() / 1154.0 <= 0.015,
            "raw {raw_ft:.1} ft"
        );
        assert_eq!(afm_conservative_distance_ft(raw_ft), 1200.0);
        assert!((afm_conservative_distance_m_from_ft(1200.0) - 365.76).abs() < 0.01);

        let t = compute_takeoff(
            &data,
            &FlightInputs {
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 1000.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                oat_c: 21.0,
                weight_lb: 725.0 / 0.45359237,
                wind_dir_deg: 0.0,
                wind_speed_kt: 4.0,
                runway: "360".into(),
                obstacle_height_m: 5.0,
                ..Default::default()
            },
        )
        .expect("takeoff");
        assert!((t.distance_over_obstacle_raw_ft - raw_ft).abs() < 0.01);
        assert_eq!(t.distance_over_obstacle_conservative_ft, 1200.0);
        assert!((t.distance_over_obstacle_conservative_m - 365.76).abs() < 0.05);
    }
}
