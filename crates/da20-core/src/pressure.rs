//! QNH unit conversion and pressure-altitude derivation (AFM performance inputs).

pub const HPA_PER_INHG: f64 = 33.8639;
pub const ISA_SEA_LEVEL_PRESSURE_HPA: f64 = 1013.25;
pub const HPA_TO_PA_FT: f64 = 30.0;

pub fn qnh_hpa_to_inhg(hpa: f64) -> f64 {
    hpa / HPA_PER_INHG
}

pub fn qnh_inhg_to_hpa(inhg: f64) -> f64 {
    inhg * HPA_PER_INHG
}

pub fn format_qnh_dual(hpa: f64) -> String {
    format!("{:.0} hPa / {:.2} inHg", hpa, qnh_hpa_to_inhg(hpa))
}

/// Inline derivation: PA (ft) = elev + (1013.25 − QNH) × 30.
pub fn pressure_altitude_derivation(elev_ft: f64, qnh_hpa: f64) -> String {
    let delta = (ISA_SEA_LEVEL_PRESSURE_HPA - qnh_hpa) * HPA_TO_PA_FT;
    let pa = elev_ft + delta;
    format!(
        "PA = {elev:.0} + (1013.25 − {qnh:.0}) × 30 = {pa:.1} ft",
        elev = elev_ft,
        qnh = qnh_hpa,
        pa = pa
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_qnh_inhg() {
        let inhg = qnh_hpa_to_inhg(1013.0);
        assert!((inhg - 29.91).abs() < 0.01);
    }

    #[test]
    fn sctb_pa_derivation() {
        let pa: f64 = 2130.0 + (1013.25 - 1013.0) * 30.0;
        assert!((pa - 2137.5).abs() < 0.01);
        let s = pressure_altitude_derivation(2130.0, 1013.0);
        assert!(s.contains("2137.5"));
    }
}
