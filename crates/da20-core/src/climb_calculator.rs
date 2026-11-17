//! Climb segment calculator: time, still-air distance, optional fuel, per-segment table.

use crate::altitude::isa_troposphere_density_altitude_ft;
use crate::altitude_sweep::{
    build_altitude_series, oat_at_pa_with_lapse, sweep_pa_series_with_terminal,
    AltitudeSweepRangeError, DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
};
use crate::chart_bounds::{limits, CHART_EDGE_TOL};
use crate::climb::compute_climb;
use crate::climb::{
    afm_vy_cas_kt, vy_kias_for_figure, VySpeedPolicy, AFM_SERVICE_CEILING_ROC_FPM,
    CLIMB_FUEL_ESTIMATE_BASIS,
};
use crate::climb_integration::{
    climb_fuel_omit_reason_for_segment, climb_inputs_at_pa, integrate_climb_fuel_us_gal,
    integrate_climb_still_air_nm, integrate_climb_time_min, ClimbFuelOmitReason,
    CLIMB_INTEGRATE_SUBSTEP_FT,
};
use crate::climb_sweep::ClimbSweepStopReason;
use crate::cruise::disa_c;
use crate::data::PerformanceData;
use crate::types::{defaults, ClimbFigure, FlightInputs};
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
    #[error("invalid lapse rate")]
    InvalidLapse,
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
    pub fuel_omit_reason: Option<ClimbFuelOmitReason>,
    pub average_roc_fpm: Option<f64>,
    pub stop_reason: Option<ClimbSweepStopReason>,
    /// Chart envelope edge PA (not the first off-chart table row).
    pub out_of_chart_at_pa_ft: Option<f64>,
    pub fuel_basis: Option<&'static str>,
    pub climb_figure: ClimbFigure,
    pub vy_kias: f64,
    pub vy_cas_kt: f64,
    pub vy_policy: VySpeedPolicy,
    pub wheel_fairings_installed: bool,
    pub weight_lb: f64,
}

fn climb_point_in_fig52_hull(pa_ft: f64, oat_c: f64) -> bool {
    pa_ft >= limits::FIG52_PA_FT.0 - CHART_EDGE_TOL
        && pa_ft <= limits::FIG52_PA_FT.1 + CHART_EDGE_TOL
        && oat_c >= limits::FIG52_OAT_C.0 - CHART_EDGE_TOL
        && oat_c <= limits::FIG52_OAT_C.1 + CHART_EDGE_TOL
}

/// Report the published chart limit PA when OAT/PA leaves the Fig 5.2 hull.
fn climb_ooc_edge_pa_ft(pa_ft: f64, _oat_c: f64) -> f64 {
    if pa_ft > limits::FIG52_PA_FT.1 {
        limits::FIG52_PA_FT.1
    } else if pa_ft < limits::FIG52_PA_FT.0 {
        limits::FIG52_PA_FT.0
    } else {
        pa_ft
    }
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
    if !params.lapse_rate_c_per_ft.is_finite() || params.lapse_rate_c_per_ft == 0.0 {
        return Err(ClimbCalculatorError::InvalidLapse);
    }

    let figure = base.climb_figure;
    let (vy_kias, vy_policy) = vy_kias_for_figure(figure);
    let vy_cas_kt = afm_vy_cas_kt(figure);

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
        let (rate_of_climb_fpm, row_valid) = if !climb_point_in_fig52_hull(*pa_ft, oat_c) {
            if stop_reason.is_none() {
                stop_reason = Some(ClimbSweepStopReason::OutOfChart);
                out_of_chart_at_pa_ft = Some(climb_ooc_edge_pa_ft(*pa_ft, oat_c));
            }
            (None, false)
        } else {
            let inputs = climb_inputs_at_pa(base, pa_start_ft, oat_start_c, *pa_ft, lapse);
            match compute_climb(data, &inputs, target_pa_ft) {
                Ok(r) if r.rate_of_climb_fpm > AFM_SERVICE_CEILING_ROC_FPM => {
                    (Some(r.rate_of_climb_fpm), true)
                }
                Ok(_) => {
                    if stop_reason.is_none() {
                        stop_reason = Some(ClimbSweepStopReason::ServiceCeiling);
                        out_of_chart_at_pa_ft = Some(climb_ooc_edge_pa_ft(*pa_ft, oat_c));
                    }
                    (None, false)
                }
                Err(_) => {
                    if stop_reason.is_none() {
                        stop_reason = Some(ClimbSweepStopReason::OutOfChart);
                        out_of_chart_at_pa_ft = Some(climb_ooc_edge_pa_ft(*pa_ft, oat_c));
                    }
                    (None, false)
                }
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
                    if stop_reason.is_none() {
                        stop_reason = Some(ClimbSweepStopReason::OutOfChart);
                        out_of_chart_at_pa_ft = Some(climb_ooc_edge_pa_ft(*pa_ft, oat_c));
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
            figure,
        )
    } else {
        None
    };

    let pre_omit = climb_fuel_omit_reason_for_segment(
        base,
        pa_start_ft,
        oat_start_c,
        lapse,
        pa_start_ft,
        target_pa_ft,
    );
    let fuel_us_gal = if completed && pre_omit.is_none() {
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
    let fuel_omit_reason = if fuel_us_gal.is_some() {
        None
    } else if let Some(r) = pre_omit {
        Some(r)
    } else if completed {
        Some(ClimbFuelOmitReason::Table3OffChart)
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
        fuel_omit_reason: if fuel_us_gal.is_some() {
            None
        } else {
            fuel_omit_reason
        },
        average_roc_fpm,
        stop_reason,
        out_of_chart_at_pa_ft,
        fuel_basis: fuel_us_gal.map(|_| CLIMB_FUEL_ESTIMATE_BASIS),
        climb_figure: figure,
        vy_kias,
        vy_cas_kt,
        vy_policy,
        wheel_fairings_installed: base.wheel_fairings_installed,
        weight_lb: base.weight_lb,
    })
}

/// AFM chart datum weight for conservative ROC read (800 kg).
pub fn climb_chart_max_weight_lb() -> f64 {
    defaults::TAKEOFF_CHART_WEIGHT_KG * 2.2046226218
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climb_sweep::{
        compute_climb_roc_sweep, ClimbSweepParams, CLIMB_SWEEP_DEFAULT_STEP_FT,
    };
    use crate::cruise::isa_temp_c_at_pa;
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

    #[test]
    fn invalid_lapse_rejected() {
        let data = PerformanceData::production();
        let inputs = FlightInputs::default();
        let err = compute_climb_calculator(
            &data,
            &inputs,
            &ClimbCalculatorParams {
                from_pa_ft: 0.0,
                to_pa_ft: 1000.0,
                lapse_rate_c_per_ft: 0.0,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err, ClimbCalculatorError::InvalidLapse);
    }

    /// Kepler / AFM Rev 29 hand validation: 2000→6000 ft ISA, flaps CRUISE, no fairings.
    ///
    /// Documented expectations (Vy 75 KIAS → 79 KCAS, Table 1; ROC penalized ×0.97):
    /// - ROC @ 2000 ft ≈ 796.2 fpm; @ 6000 ft ≈ 586.6 fpm
    /// - Total time ≈ 350.0 s; average ROC ≈ 685.6 fpm; still-air ≈ 8.17 NM; fuel ≈ 0.78 US gal
    #[test]
    fn kepler_hand_calc_2000_to_6000_isa_cruise_flaps_no_fairings() {
        let data = PerformanceData::production();
        let from = 2000.0;
        let to = 6000.0;
        let oat = isa_temp_c_at_pa(from);
        let inputs = FlightInputs {
            pressure_altitude_input_ft: from,
            oat_c: oat,
            climb_figure: ClimbFigure::Fig55CruiseFlaps,
            wheel_fairings_installed: false,
            ..Default::default()
        };
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

        let roc_2k = res.rows[0].rate_of_climb_fpm.unwrap();
        let roc_6k = res
            .rows
            .iter()
            .find(|r| (r.pa_ft - to).abs() < 1e-3)
            .and_then(|r| r.rate_of_climb_fpm)
            .unwrap();
        assert!((roc_2k - 796.2).abs() < 2.0, "roc_2k={roc_2k}");
        assert!((roc_6k - 586.6).abs() < 2.0, "roc_6k={roc_6k}");

        let t = res.total_time_s.expect("time");
        assert!((t - 350.0).abs() < 3.0, "t={t}");

        let avg = res.average_roc_fpm.expect("avg roc");
        assert!((avg - 685.6).abs() < 3.0, "avg={avg}");

        let nm = res.still_air_distance_nm.expect("nm");
        assert!((nm - 8.17).abs() < 0.08, "nm={nm}");

        let fuel = res.fuel_us_gal.expect("fuel");
        assert!((fuel - 0.78).abs() < 0.05, "fuel={fuel}");
    }

    /// Consistency with sweep integral (not independent validation).
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
    fn out_of_chart_reports_chart_pa_edge_not_row() {
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
                from_pa_ft: 8_000.0,
                to_pa_ft: 16_050.0,
                oat_at_from_c: isa_temp_c_at_pa(8_000.0),
                step_ft: 4_025.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(res.stop_reason, Some(ClimbSweepStopReason::OutOfChart));
        assert_eq!(res.out_of_chart_at_pa_ft, Some(limits::FIG52_PA_FT.1));
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
