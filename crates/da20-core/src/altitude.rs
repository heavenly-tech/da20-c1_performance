//! Pressure and density altitude from field elevation, QNH, and OAT.

use crate::types::FlightInputs;

const ISA_SEA_LEVEL_PRESSURE_HPA: f64 = 1013.25;
const ISA_LAPSE_RATE_C_PER_FT: f64 = 0.0019812;

/// Pressure altitude (ft) from field elevation and QNH (hPa).
pub fn pressure_altitude_ft(field_elevation_ft: f64, qnh_hpa: f64) -> f64 {
    field_elevation_ft + (ISA_SEA_LEVEL_PRESSURE_HPA - qnh_hpa) * 30.0
}

/// Density altitude (ft) from pressure altitude and OAT (°C).
pub fn density_altitude_ft(pressure_altitude_ft: f64, oat_c: f64) -> f64 {
    let isa_temp_c = 15.0 - ISA_LAPSE_RATE_C_PER_FT * pressure_altitude_ft;
    pressure_altitude_ft + (120.0 * (oat_c - isa_temp_c))
}

/// Performance planning PA: direct chart entry when set, else from field elevation and QNH.
pub fn effective_pressure_altitude_ft(inputs: &FlightInputs) -> f64 {
    inputs
        .direct_pressure_altitude_ft
        .unwrap_or_else(|| pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sctb_pressure_altitude_at_standard_qnh() {
        let pa = pressure_altitude_ft(2130.0, 1013.0);
        assert!((pa - (2130.0 + 7.5)).abs() < 1.0);
    }

    #[test]
    fn direct_pa_override() {
        let i = FlightInputs {
            direct_pressure_altitude_ft: Some(5000.0),
            ..Default::default()
        };
        assert_eq!(effective_pressure_altitude_ft(&i), 5000.0);
    }
}
