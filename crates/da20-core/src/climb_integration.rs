//! Altitude integration along the climb ROC chart path (Simpson, 100 ft substeps).

use crate::altitude::clear_pa_override;
use crate::altitude_sweep::oat_at_pa_with_lapse;
use crate::chart_bounds::limits;
use crate::climb::compute_climb;
use crate::climb::{tas_from_vy_held, AFM_SERVICE_CEILING_ROC_FPM};
use crate::cruise::{compute_cruise, disa_c, TABLE3_RPM_MAX};
use crate::data::PerformanceData;
use crate::types::{ClimbFigure, CruiseSolveMode, FlightInputs, PropellerKind};

pub const CLIMB_INTEGRATE_SUBSTEP_FT: f64 = 100.0;

/// Flight inputs at a lapse-adjusted PA along a climb segment (clears chart PA override).
pub fn climb_inputs_at_pa(
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
    let inputs = climb_inputs_at_pa(base, pa_start_ft, oat_start_c, pa_ft, lapse);
    let r = compute_climb(data, &inputs, target_pa_ft).ok()?;
    let roc = r.rate_of_climb_fpm;
    if roc <= AFM_SERVICE_CEILING_ROC_FPM {
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
    vy_figure: ClimbFigure,
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
        let tas = tas_from_vy_held(vy_figure, h, oat);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClimbFuelOmitReason {
    MtPropeller,
    BelowTable3Pa,
    DisaOutsideTable3,
    Table3OffChart,
}

pub fn climb_fuel_omit_reason_for_segment(
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    lapse: f64,
    from_pa: f64,
    to_pa: f64,
) -> Option<ClimbFuelOmitReason> {
    if base.propeller != PropellerKind::Sensenich {
        return Some(ClimbFuelOmitReason::MtPropeller);
    }
    let step = CLIMB_INTEGRATE_SUBSTEP_FT;
    let mut h = from_pa;
    while h <= to_pa + 1e-9 {
        let oat = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, h, lapse);
        let disa = disa_c(oat, h);
        if h < limits::TABLE3_PA_FT.0 - 1e-6 || h > limits::TABLE3_PA_FT.1 + 1e-6 {
            return Some(ClimbFuelOmitReason::BelowTable3Pa);
        }
        if disa < limits::TABLE3_DISA_C.0 - 1e-6 || disa > limits::TABLE3_DISA_C.1 + 1e-6 {
            return Some(ClimbFuelOmitReason::DisaOutsideTable3);
        }
        h += step;
    }
    None
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
    if climb_fuel_omit_reason_for_segment(base, pa_start_ft, oat_start_c, lapse, from_pa, to_pa)
        .is_some()
    {
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
    use crate::climb::afm_vy_cas_kt;
    use crate::climb::tas_from_vy_cas_at_pa;
    use crate::types::ClimbFigure;

    #[test]
    fn tas_increases_with_altitude_on_standard_day() {
        let cas = afm_vy_cas_kt(ClimbFigure::Fig55CruiseFlaps);
        let oat0 = 15.0;
        let tas0 = tas_from_vy_cas_at_pa(cas, 0.0, oat0);
        let tas5k = tas_from_vy_cas_at_pa(cas, 5000.0, oat0 - 0.0019812 * 5000.0);
        assert!(tas5k > tas0);
    }
}
