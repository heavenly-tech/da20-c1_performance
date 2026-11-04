//! Pressure and density altitude from field elevation, QNH, and OAT.
//!
//! Standard atmosphere relationships used for planning inputs; chart lookups use
//! reported pressure altitude from these formulas unless overridden.

const ISA_SEA_LEVEL_PRESSURE_HPA: f64 = 1013.25;
const ISA_LAPSE_RATE_C_PER_FT: f64 = 0.0019812;

/// Pressure altitude (ft) from field elevation and QNH (hPa).
pub fn pressure_altitude_ft(field_elevation_ft: f64, qnh_hpa: f64) -> f64 {
    // AFM-style: PA = elevation + (1013 - QNH) * 30 (approx 30 ft/hPa)
    field_elevation_ft + (ISA_SEA_LEVEL_PRESSURE_HPA - qnh_hpa) * 30.0
}

/// Density altitude (ft) from pressure altitude and OAT (°C).
pub fn density_altitude_ft(pressure_altitude_ft: f64, oat_c: f64) -> f64 {
    let isa_temp_c = 15.0 - ISA_LAPSE_RATE_C_PER_FT * pressure_altitude_ft;
    pressure_altitude_ft + (120.0 * (oat_c - isa_temp_c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sctb_pressure_altitude_at_standard_qnh() {
        let pa = pressure_altitude_ft(2130.0, 1013.0);
        // 1013 hPa vs ISA 1013.25 → small offset
        assert!((pa - (2130.0 + 7.5)).abs() < 1.0);
    }

    #[test]
    fn density_altitude_increases_with_warmth() {
        let pa = 2000.0;
        let da_cold = density_altitude_ft(pa, 0.0);
        let da_hot = density_altitude_ft(pa, 35.0);
        assert!(da_hot > da_cold);
    }
}
