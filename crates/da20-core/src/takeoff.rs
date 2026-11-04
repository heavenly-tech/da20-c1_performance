use crate::altitude::pressure_altitude_ft;
use crate::data::PerformanceData;
use crate::interpolation::{interpolate_1d, ExtrapolationPolicy};
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, FlightInputs, TakeoffResult};
use crate::wind::compute_wind;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TakeoffError {
    #[error("interpolation: {0}")]
    Interp(#[from] crate::interpolation::InterpError),
    #[error("no takeoff chart for pressure altitude")]
    NoChart,
}

fn interpolate_takeoff_at_pa(
    charts: &[crate::data::TakeoffChart],
    pa: f64,
    oat_c: f64,
) -> Result<(f64, f64, Vec<CalculationWarning>), TakeoffError> {
    if charts.is_empty() {
        return Err(TakeoffError::NoChart);
    }
    let mut warnings = Vec::new();
    let mut sorted: Vec<_> = charts.iter().collect();
    sorted.sort_by(|a, b| {
        a.pressure_altitude_ft
            .partial_cmp(&b.pressure_altitude_ft)
            .unwrap()
    });

    let (ground, dist50) = if pa <= sorted[0].pressure_altitude_ft {
        warnings.push(CalculationWarning {
            code: "CLAMP_PA".into(),
            message: format!(
                "Pressure altitude {pa:.0} ft at/below lowest chart {} ft.",
                sorted[0].pressure_altitude_ft
            ),
        });
        let c = sorted[0];
        let (g, w1) = interpolate_1d(&c.temperature_c, &c.ground_roll_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        let (d, w2) = interpolate_1d(&c.temperature_c, &c.distance_50ft_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        warnings.extend(w1);
        warnings.extend(w2);
        (g, d)
    } else if pa >= sorted[sorted.len() - 1].pressure_altitude_ft {
        let c = sorted[sorted.len() - 1];
        let (g, w1) = interpolate_1d(&c.temperature_c, &c.ground_roll_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        let (d, w2) = interpolate_1d(&c.temperature_c, &c.distance_50ft_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        warnings.extend(w1);
        warnings.extend(w2);
        (g, d)
    } else {
        let mut upper_idx = 1usize;
        while upper_idx < sorted.len() && sorted[upper_idx].pressure_altitude_ft < pa {
            upper_idx += 1;
        }
        let lower = sorted[upper_idx - 1];
        let upper = sorted[upper_idx];
        warnings.push(CalculationWarning {
            code: "INTERP_PA".into(),
            message: format!(
                "Interpolating takeoff between {} and {} ft PA.",
                lower.pressure_altitude_ft, upper.pressure_altitude_ft
            ),
        });
        let (g0, w1) = interpolate_1d(&lower.temperature_c, &lower.ground_roll_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        let (d0, w2) = interpolate_1d(&lower.temperature_c, &lower.distance_50ft_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        let (g1, w3) = interpolate_1d(&upper.temperature_c, &upper.ground_roll_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        let (d1, w4) = interpolate_1d(&upper.temperature_c, &upper.distance_50ft_ft, oat_c, ExtrapolationPolicy::Clamp)?;
        warnings.extend(w1);
        warnings.extend(w2);
        warnings.extend(w3);
        warnings.extend(w4);
        (
            crate::interpolation::lerp(
                lower.pressure_altitude_ft,
                g0,
                upper.pressure_altitude_ft,
                g1,
                pa,
            ),
            crate::interpolation::lerp(
                lower.pressure_altitude_ft,
                d0,
                upper.pressure_altitude_ft,
                d1,
                pa,
            ),
        )
    };
    Ok((ground, dist50, warnings))
}

pub fn compute_takeoff(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<TakeoffResult, TakeoffError> {
    let pa = pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa);
    let (ground, dist50, mut warnings) =
        interpolate_takeoff_at_pa(&data.takeoff.charts, pa, inputs.oat_c)?;

    let wind = compute_wind(data, inputs).map_err(TakeoffError::Interp)?;
    warnings.extend(wind.warnings.clone());
    let ground_adj = ground + wind.takeoff_landing_correction_ft;
    let dist_adj = dist50 + wind.takeoff_landing_correction_ft;

    if data.meta.is_synthetic() {
        warnings.push(CalculationWarning {
            code: "SYNTHETIC_DATA".into(),
            message: "Takeoff distances from SYNTHETIC fixture — not for flight use.".into(),
        });
    }

    Ok(TakeoffResult {
        provenance: provenance_for(data, &data.takeoff.citation),
        pressure_altitude_ft: pa,
        ground_roll_ft: ground_adj,
        distance_over_50ft_ft: dist_adj,
        wind_correction_ft: wind.takeoff_landing_correction_ft,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FlightInputs;

    #[test]
    fn synthetic_takeoff_runs() {
        let data = PerformanceData::synthetic_fixture();
        let r = compute_takeoff(&data, &FlightInputs::default()).unwrap();
        assert!(r.ground_roll_ft > 0.0);
        assert!(r.distance_over_50ft_ft >= r.ground_roll_ft);
    }
}
