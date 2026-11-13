//! Ground OAT lapse helpers (UI / planning layer; AFM charts still use da20-core math).

/// ISA troposphere lapse for user-facing estimates (°C per 1000 ft).
pub const LAPSE_C_PER_1000_FT: f64 = 1.98;

/// ISA temperature (°C) at pressure altitude for display and planning (`15 − 1.98 × alt/1000`).
pub fn isa_temp_c_at_altitude_ft(altitude_ft: f64) -> f64 {
    15.0 - LAPSE_C_PER_1000_FT * altitude_ft / 1000.0
}

/// Estimated OAT (°C) at `cruise_alt_ft` from ground elevation and OAT.
pub fn estimated_oat_c_from_ground(
    ground_elevation_ft: f64,
    ground_oat_c: f64,
    cruise_alt_ft: f64,
) -> f64 {
    let delta_ft = cruise_alt_ft - ground_elevation_ft;
    ground_oat_c - LAPSE_C_PER_1000_FT * delta_ft / 1000.0
}

/// ΔISA for display: cruise OAT minus ISA at cruise pressure altitude (UI lapse).
pub fn display_disa_c(cruise_oat_c: f64, cruise_pressure_alt_ft: f64) -> f64 {
    cruise_oat_c - isa_temp_c_at_altitude_ft(cruise_pressure_alt_ft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_2000_25c_estimates_13_1c_at_8000_ft() {
        let est = estimated_oat_c_from_ground(2000.0, 25.0, 8000.0);
        assert!((est - 13.12).abs() < 0.01);
        assert!((est - 13.1).abs() < 0.05);
    }

    #[test]
    fn isa_temp_at_sea_level() {
        assert!((isa_temp_c_at_altitude_ft(0.0) - 15.0).abs() < f64::EPSILON);
    }
}
