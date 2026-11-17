//! Altitude integration along the climb ROC chart path (Simpson, 100 ft substeps).

use crate::altitude::clear_pa_override;
use crate::altitude_sweep::oat_at_pa_with_lapse;
use crate::climb::compute_climb;
use crate::climb::tas_from_ias_at_pa;
use crate::cruise::{compute_cruise, TABLE3_RPM_MAX};
use crate::data::PerformanceData;
use crate::types::{CruiseSolveMode, FlightInputs, PropellerKind};

pub const CLIMB_INTEGRATE_SUBSTEP_FT: f64 = 100.0;

fn inputs_at_pa(
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    pa_ft: f64,
    lapse: f64,
) -> FlightInputs {
    let mut inputs = base.clone();
    clear_pa_override(&mut inputs);
    inputs.pressure_altitude_input_ft = pa_ft;
    inputs.oat_c = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, pa_ft, lapse);
    inputs
}

/// Penalized ROC (fpm) at `pa_ft` via `compute_climb` (Fig 5.2 EA + roc fit + fairings).
pub fn climb_roc_fpm_at(
    data: &PerformanceData,
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    pa_ft: f64,
    lapse: f64,
    target_pa_ft: f64,
) -> Option<f64> {
    let inputs = inputs_at_pa(base, pa_start_ft, oat_start_c, pa_ft, lapse);
    let r = compute_climb(data, &inputs, target_pa_ft).ok()?;
    let roc = r.rate_of_climb_fpm;
    if roc <= 0.0 {
        None
    } else {
        Some(roc)
    }
}

fn simpson_steps(from_pa: f64, to_pa: f64) -> (usize, f64) {
    let dh = to_pa - from_pa;
    let mut n = ((dh / CLIMB_INTEGRATE_SUBSTEP_FT).ceil() as usize).max(2);
    if !n.is_multiple_of(2) {
        n += 1;
    }
    (n, dh / n as f64)
}

/// ∫ (1/ROC(h)) dh in minutes (ROC in ft/min).
#[allow(clippy::too_many_arguments)]
pub fn integrate_climb_time_min(
    data: &PerformanceData,
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    lapse: f64,
    target_pa_ft: f64,
    from_pa: f64,
    to_pa: f64,
) -> Option<f64> {
    if to_pa <= from_pa + 1e-9 {
        return Some(0.0);
    }
    let (n, step) = simpson_steps(from_pa, to_pa);
    let mut sum = 0.0;
    for i in 0..=n {
        let h = from_pa + step * i as f64;
        let roc = climb_roc_fpm_at(data, base, pa_start_ft, oat_start_c, h, lapse, target_pa_ft)?;
        let f = 1.0 / roc;
        let weight = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight * f;
    }
    Some(sum * step / 3.0)
}

/// Still-air horizontal distance (NM): ∫ TAS dt with TAS from Vy IAS and density ratio.
#[allow(clippy::too_many_arguments)]
pub fn integrate_climb_still_air_nm(
    data: &PerformanceData,
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    lapse: f64,
    target_pa_ft: f64,
    from_pa: f64,
    to_pa: f64,
    vy_ias_kt: f64,
) -> Option<f64> {
    if to_pa <= from_pa + 1e-9 {
        return Some(0.0);
    }
    let (n, step) = simpson_steps(from_pa, to_pa);
    let mut sum = 0.0;
    for i in 0..=n {
        let h = from_pa + step * i as f64;
        let oat = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, h, lapse);
        let roc = climb_roc_fpm_at(data, base, pa_start_ft, oat_start_c, h, lapse, target_pa_ft)?;
        let tas = tas_from_ias_at_pa(vy_ias_kt, h, oat);
        // d(nm) = TAS_kt * dh / (ROC_fpm * 60)
        let f = tas / (roc * 60.0);
        let weight = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight * f;
    }
    Some(sum * step / 3.0)
}

fn table3_climb_gph_at(
    data: &PerformanceData,
    base: &FlightInputs,
    pa_ft: f64,
    oat_c: f64,
) -> Option<f64> {
    if base.propeller != PropellerKind::Sensenich {
        return None;
    }
    let mut ci = base.clone();
    clear_pa_override(&mut ci);
    ci.cruise_pressure_alt_ft = pa_ft;
    ci.cruise_oat_c = oat_c;
    ci.cruise_rpm = TABLE3_RPM_MAX;
    ci.cruise_solve_mode = CruiseSolveMode::SetRpm;
    compute_cruise(data, &ci).ok().map(|r| r.fuel_flow_us_gal_h)
}

/// Estimated climb fuel (US gal) from Table 3 GPH at 2800 RPM when available on entire segment.
#[allow(clippy::too_many_arguments)]
pub fn integrate_climb_fuel_us_gal(
    data: &PerformanceData,
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    lapse: f64,
    target_pa_ft: f64,
    from_pa: f64,
    to_pa: f64,
) -> Option<f64> {
    if base.propeller != PropellerKind::Sensenich {
        return None;
    }
    if to_pa <= from_pa + 1e-9 {
        return Some(0.0);
    }
    let (n, step) = simpson_steps(from_pa, to_pa);
    let mut sum = 0.0;
    for i in 0..=n {
        let h = from_pa + step * i as f64;
        let oat = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, h, lapse);
        let roc = climb_roc_fpm_at(data, base, pa_start_ft, oat_start_c, h, lapse, target_pa_ft)?;
        let gph = table3_climb_gph_at(data, base, h, oat)?;
        let f = gph / (roc * 60.0);
        let weight = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight * f;
    }
    Some(sum * step / 3.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::altitude::density_ratio_sigma;
    use crate::climb::AFM_CLIMB_VY_IAS_KT;

    #[test]
    fn tas_increases_with_altitude_on_standard_day() {
        let ias = AFM_CLIMB_VY_IAS_KT;
        let oat0 = 15.0;
        let tas0 = tas_from_ias_at_pa(ias, 0.0, oat0);
        let tas5k = tas_from_ias_at_pa(ias, 5000.0, oat0 - 0.0019812 * 5000.0);
        assert!(tas5k > tas0);
        let sigma = density_ratio_sigma(0.0, oat0);
        assert!((tas0 - ias / sigma.sqrt()).abs() < 1e-6);
    }
}
