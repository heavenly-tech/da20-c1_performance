//! Climb segment calculator: time, still-air distance, optional fuel, per-segment table.

use crate::altitude::{clear_pa_override, isa_troposphere_density_altitude_ft};
use crate::altitude_sweep::{
    build_altitude_series, oat_at_pa_with_lapse, sweep_pa_series_with_terminal,
    AltitudeSweepRangeError, DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
};
use crate::climb::compute_climb;
use crate::climb::{AFM_CLIMB_VY_IAS_KT, CLIMB_FUEL_ESTIMATE_BASIS};
use crate::climb_integration::{
    integrate_climb_fuel_us_gal, integrate_climb_still_air_nm, integrate_climb_time_min,
    CLIMB_INTEGRATE_SUBSTEP_FT,
};
use crate::climb_sweep::ClimbSweepStopReason;
use crate::cruise::disa_c;
use crate::data::PerformanceData;
use crate::types::FlightInputs;
use thiserror::Error;

pub const CLIMB_CALC_DEFAULT_STEP_FT: f64 = CLIMB_INTEGRATE_SUBSTEP_FT;
pub const CLIMB_CALC_MAX_ROWS: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct ClimbCalculatorParams {
    pub from_pa_ft: f64,
    pub to_pa_ft: f64,
    pub oat_at_from_c: f64,
    pub step_ft: f64,
    pub lapse_rate_c_per_ft: f64,
}

impl Default for ClimbCalculatorParams {
    fn default() -> Self {
        Self {
            from_pa_ft: 0.0,
            to_pa_ft: 0.0,
            oat_at_from_c: 15.0,
            step_ft: CLIMB_CALC_DEFAULT_STEP_FT,
            lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ClimbCalculatorError {
    #[error("from PA must be strictly below to PA")]
    InvalidRange,
    #[error(transparent)]
    Sweep(#[from] AltitudeSweepRangeError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClimbCalculatorRow {
    pub pa_ft: f64,
    pub oat_c: f64,
    pub isa_dev_c: f64,
    pub density_altitude_ft: Option<f64>,
    pub rate_of_climb_fpm: Option<f64>,
    pub cumulative_time_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClimbCalculatorResult {
    pub rows: Vec<ClimbCalculatorRow>,
    pub total_time_s: Option<f64>,
    pub still_air_distance_nm: Option<f64>,
    pub fuel_us_gal: Option<f64>,
    pub average_roc_fpm: Option<f64>,
    pub stop_reason: Option<ClimbSweepStopReason>,
    /// First PA where the climb leaves the chart envelope (no clamping).
    pub out_of_chart_at_pa_ft: Option<f64>,
    pub fuel_basis: Option<&'static str>,
}

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

pub fn compute_climb_calculator(
    data: &PerformanceData,
    base: &FlightInputs,
    params: &ClimbCalculatorParams,
) -> Result<ClimbCalculatorResult, ClimbCalculatorError> {
    if params.from_pa_ft >= params.to_pa_ft {
        return Err(ClimbCalculatorError::InvalidRange);
    }
    if !(params.from_pa_ft.is_finite()
        && params.to_pa_ft.is_finite()
        && params.oat_at_from_c.is_finite()
        && params.step_ft.is_finite())
    {
        return Err(ClimbCalculatorError::Sweep(
            AltitudeSweepRangeError::NonFinite,
        ));
    }

    let step = params.step_ft.max(CLIMB_INTEGRATE_SUBSTEP_FT / 2.0);
    let pa_series = sweep_pa_series_with_terminal(
        build_altitude_series(
            params.from_pa_ft,
            params.to_pa_ft,
            step,
            CLIMB_CALC_MAX_ROWS,
        )?,
        params.to_pa_ft,
        CLIMB_CALC_MAX_ROWS,
    )?;

    let pa_start_ft = params.from_pa_ft;
    let oat_start_c = params.oat_at_from_c;
    let lapse = params.lapse_rate_c_per_ft;
    let target_pa_ft = params.to_pa_ft;

    let mut rows = Vec::with_capacity(pa_series.len());
    let mut stop_reason = None;
    let mut out_of_chart_at_pa_ft = None;
    let mut cumulative_min = 0.0;
    let mut prev_valid_pa: Option<f64> = None;
    let mut accumulate = true;

    for (idx, pa_ft) in pa_series.iter().enumerate() {
        let oat_c = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, *pa_ft, lapse);
        let isa_dev_c = disa_c(oat_c, *pa_ft);
        let da = isa_troposphere_density_altitude_ft(*pa_ft, oat_c);
        let inputs = inputs_at_pa(base, pa_start_ft, oat_start_c, *pa_ft, lapse);

        let (rate_of_climb_fpm, row_valid) = match compute_climb(data, &inputs, target_pa_ft) {
            Ok(r) if r.rate_of_climb_fpm > 0.0 => (Some(r.rate_of_climb_fpm), true),
            Ok(_) => {
                if stop_reason.is_none() {
                    stop_reason = Some(ClimbSweepStopReason::ServiceCeiling);
                    out_of_chart_at_pa_ft = Some(*pa_ft);
                }
                (None, false)
            }
            Err(_) => {
                if stop_reason.is_none() {
                    stop_reason = Some(ClimbSweepStopReason::OutOfChart);
                    out_of_chart_at_pa_ft = Some(*pa_ft);
                }
                (None, false)
            }
        };

        let cumulative_time_s = if idx == 0 {
            Some(0.0)
        } else if !accumulate || !row_valid {
            accumulate = false;
            None
        } else if let Some(prev_pa) = prev_valid_pa {
            match integrate_climb_time_min(
                data,
                base,
                pa_start_ft,
                oat_start_c,
                lapse,
                target_pa_ft,
                prev_pa,
                *pa_ft,
            ) {
                Some(seg_min) => {
                    cumulative_min += seg_min;
                    Some(cumulative_min * 60.0)
                }
                None => {
                    accumulate = false;
                    if out_of_chart_at_pa_ft.is_none() {
                        out_of_chart_at_pa_ft = Some(*pa_ft);
                    }
                    if stop_reason.is_none() {
                        stop_reason = Some(ClimbSweepStopReason::OutOfChart);
                    }
                    None
                }
            }
        } else {
            Some(0.0)
        };

        if row_valid {
            prev_valid_pa = Some(*pa_ft);
        }

        rows.push(ClimbCalculatorRow {
            pa_ft: *pa_ft,
            oat_c,
            isa_dev_c,
            density_altitude_ft: Some(da),
            rate_of_climb_fpm,
            cumulative_time_s,
        });

        if !row_valid {
            break;
        }
    }

    let completed = accumulate
        && rows
            .last()
            .map(|r| (r.pa_ft - target_pa_ft).abs() < 1e-3)
            .unwrap_or(false)
        && stop_reason.is_none();

    let total_time_s = if completed {
        integrate_climb_time_min(
            data,
            base,
            pa_start_ft,
            oat_start_c,
            lapse,
            target_pa_ft,
            pa_start_ft,
            target_pa_ft,
        )
        .map(|m| m * 60.0)
    } else {
        None
    };

    let still_air_distance_nm = if completed {
        integrate_climb_still_air_nm(
            data,
            base,
            pa_start_ft,
            oat_start_c,
            lapse,
            target_pa_ft,
            pa_start_ft,
            target_pa_ft,
            AFM_CLIMB_VY_IAS_KT,
        )
    } else {
        None
    };

    let fuel_us_gal = if completed {
        integrate_climb_fuel_us_gal(
            data,
            base,
            pa_start_ft,
            oat_start_c,
            lapse,
            target_pa_ft,
            pa_start_ft,
            target_pa_ft,
        )
    } else {
        None
    };

    let delta_ft = target_pa_ft - pa_start_ft;
    let average_roc_fpm = total_time_s.and_then(|t| {
        if t > 0.0 {
            Some(delta_ft / (t / 60.0))
        } else {
            None
        }
    });

    Ok(ClimbCalculatorResult {
        rows,
        total_time_s,
        still_air_distance_nm,
        fuel_us_gal,
        average_roc_fpm,
        stop_reason,
        out_of_chart_at_pa_ft,
        fuel_basis: fuel_us_gal.map(|_| CLIMB_FUEL_ESTIMATE_BASIS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climb_sweep::{
        compute_climb_roc_sweep, ClimbSweepParams, CLIMB_SWEEP_DEFAULT_STEP_FT,
    };
    use crate::types::{climb_roc_fairing_multiplier, ClimbFigure, FlightInputs};

    #[test]
    fn from_ge_to_is_error() {
        let data = PerformanceData::production();
        let inputs = FlightInputs::default();
        let err = compute_climb_calculator(
            &data,
            &inputs,
            &ClimbCalculatorParams {
                from_pa_ft: 4000.0,
                to_pa_ft: 2000.0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err, ClimbCalculatorError::InvalidRange);

        let err_eq = compute_climb_calculator(
            &data,
            &inputs,
            &ClimbCalculatorParams {
                from_pa_ft: 3000.0,
                to_pa_ft: 3000.0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err_eq, ClimbCalculatorError::InvalidRange);
    }

    /// Hand check at two altitudes (documented arithmetic).
    ///
    /// At PA 2000 ft, OAT 18.035 °C (65 °F), cruise flaps, fairings on:
    /// AFM Fig 5.5 gives ROC ≈ 800 fpm (see `climb::tests::roc_figures_within_one_percent`).
    /// One 100 ft segment: Δt = 100/800 min = 0.125 min = 7.5 s.
    #[test]
    fn hand_calculation_two_point_100ft_segment() {
        use crate::rev29::engine::fahrenheit_to_celsius;

        let data = PerformanceData::production();
        let oat = fahrenheit_to_celsius(65.0);
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 2000.0,
            oat_c: oat,
            climb_figure: ClimbFigure::Fig55CruiseFlaps,
            wheel_fairings_installed: true,
            ..Default::default()
        };
        let from = 2000.0;
        let to = 2100.0;
        let res = compute_climb_calculator(
            &data,
            &inputs,
            &ClimbCalculatorParams {
                from_pa_ft: from,
                to_pa_ft: to,
                oat_at_from_c: oat,
                step_ft: 100.0,
                lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
            },
        )
        .unwrap();
        let roc_start = res.rows[0].rate_of_climb_fpm.unwrap();
        assert!((roc_start - 800.0).abs() < 8.0, "roc_start={roc_start}");
        let t_s = res.total_time_s.expect("time");
        let expected_s = 100.0 / roc_start * 60.0;
        assert!(
            (t_s - expected_s).abs() < 0.5,
            "t={t_s}s expected≈{expected_s}s (100/ROC)"
        );
    }

    #[test]
    fn total_time_matches_roc_sweep_over_same_range() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 2130.0,
            oat_c: 20.0,
            ..Default::default()
        };
        let lapse = DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT;
        let from = 2130.0;
        let to = 4000.0;
        let calc = compute_climb_calculator(
            &data,
            &inputs,
            &ClimbCalculatorParams {
                from_pa_ft: from,
                to_pa_ft: to,
                oat_at_from_c: 20.0,
                step_ft: CLIMB_SWEEP_DEFAULT_STEP_FT,
                lapse_rate_c_per_ft: lapse,
            },
        )
        .unwrap();
        let sweep = compute_climb_roc_sweep(
            &data,
            &inputs,
            &ClimbSweepParams {
                from_pa_ft: from,
                to_pa_ft: to,
                step_ft: CLIMB_SWEEP_DEFAULT_STEP_FT,
                lapse_rate_c_per_ft: lapse,
            },
        )
        .unwrap();
        let t_calc = calc.total_time_s.expect("calc time");
        let t_sweep = sweep.total_time_s.expect("sweep time");
        assert!(
            (t_calc - t_sweep).abs() < 2.0,
            "calc {t_calc}s vs sweep {t_sweep}s"
        );
    }

    #[test]
    fn out_of_chart_reports_leave_altitude() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 15_500.0,
            oat_c: 15.0,
            ..Default::default()
        };
        let res = compute_climb_calculator(
            &data,
            &inputs,
            &ClimbCalculatorParams {
                from_pa_ft: 15_500.0,
                to_pa_ft: 16_500.0,
                oat_at_from_c: 15.0,
                step_ft: 500.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(res.stop_reason, Some(ClimbSweepStopReason::OutOfChart));
        assert!(res.out_of_chart_at_pa_ft.is_some());
        assert!(res.total_time_s.is_none());
    }

    #[test]
    fn fairings_reduce_roc_three_percent_in_calculator_rows() {
        let data = PerformanceData::production();
        let base = FlightInputs::default();
        let params = ClimbCalculatorParams {
            from_pa_ft: 2000.0,
            to_pa_ft: 2200.0,
            oat_at_from_c: base.oat_c,
            step_ft: 200.0,
            ..Default::default()
        };
        let mut off = base.clone();
        off.wheel_fairings_installed = false;
        let on = base;
        let r_on = compute_climb_calculator(&data, &on, &params).unwrap();
        let r_off = compute_climb_calculator(&data, &off, &params).unwrap();
        let roc_on = r_on.rows[0].rate_of_climb_fpm.unwrap();
        let roc_off = r_off.rows[0].rate_of_climb_fpm.unwrap();
        let m = climb_roc_fairing_multiplier(false);
        assert!((roc_off - roc_on * m).abs() < 0.05);
    }
}
