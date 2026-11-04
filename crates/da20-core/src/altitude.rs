//! Pressure and density altitude from field elevation, QNH, and OAT.

use crate::pressure::{HPA_TO_PA_FT, ISA_SEA_LEVEL_PRESSURE_HPA};
use crate::types::{ChartPhase, FlightInputs, PaOverride};

const ISA_LAPSE_RATE_C_PER_FT: f64 = 0.0019812;

/// Pressure altitude (ft) from field elevation and QNH (hPa).
pub fn pressure_altitude_ft(field_elevation_ft: f64, qnh_hpa: f64) -> f64 {
    field_elevation_ft + (ISA_SEA_LEVEL_PRESSURE_HPA - qnh_hpa) * HPA_TO_PA_FT
}

/// Density altitude (ft) from pressure altitude and OAT (°C).
pub fn density_altitude_ft(pressure_altitude_ft: f64, oat_c: f64) -> f64 {
    let isa_temp_c = 15.0 - ISA_LAPSE_RATE_C_PER_FT * pressure_altitude_ft;
    pressure_altitude_ft + (120.0 * (oat_c - isa_temp_c))
}

fn resolved_override(inputs: &FlightInputs) -> Option<PaOverride> {
    inputs.pa_override.or_else(|| {
        inputs.direct_pressure_altitude_ft.map(|pa_ft| PaOverride {
            pa_ft,
            phase: ChartPhase::Takeoff,
        })
    })
}

/// Performance planning PA: chart override when active phase matches, else elev+QNH.
pub fn effective_pressure_altitude_ft(inputs: &FlightInputs, phase: Option<ChartPhase>) -> f64 {
    if let Some(active) = phase {
        if let Some(o) = resolved_override(inputs) {
            if o.phase == active {
                return o.pa_ft;
            }
        }
    }
    pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa)
}

pub fn clear_pa_override(inputs: &mut FlightInputs) {
    inputs.pa_override = None;
    inputs.direct_pressure_altitude_ft = None;
}

pub fn set_pa_override(inputs: &mut FlightInputs, pa_ft: f64, phase: ChartPhase) {
    inputs.pa_override = Some(PaOverride { pa_ft, phase });
    inputs.direct_pressure_altitude_ft = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PaOverride;

    #[test]
    fn sctb_pressure_altitude_at_standard_qnh() {
        let pa = pressure_altitude_ft(2130.0, 1013.0);
        assert!((pa - (2130.0 + 7.5)).abs() < 1.0);
    }

    #[test]
    fn scoped_pa_override() {
        let i = FlightInputs {
            pa_override: Some(PaOverride {
                pa_ft: 5000.0,
                phase: ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        assert_eq!(
            effective_pressure_altitude_ft(&i, Some(ChartPhase::Takeoff)),
            5000.0
        );
        assert!(
            (effective_pressure_altitude_ft(&i, Some(ChartPhase::Landing)) - 2137.5).abs() < 1.0
        );
    }
}
