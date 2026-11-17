//! Climb ROC vs pressure-altitude sweep (Fig 5.x).

use crate::altitude::{effective_pressure_altitude_ft, isa_troposphere_density_altitude_ft};
use crate::altitude_sweep::{
    build_altitude_series, oat_at_pa_with_lapse, sweep_pa_series_with_terminal,
    AltitudeSweepRangeError, DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
};
use crate::climb::compute_climb;
use crate::climb::AFM_SERVICE_CEILING_ROC_FPM;
use crate::climb_integration::{climb_inputs_at_pa, integrate_climb_time_min};
use crate::cruise::disa_c;
use crate::data::PerformanceData;
use crate::types::{ChartPhase, FlightInputs};

pub const CLIMB_SWEEP_MAX_ROWS: usize = 60;
pub const CLIMB_SWEEP_DEFAULT_STEP_FT: f64 = 500.0;
/// Default climb-sweep upper PA (Fig 5.2 panel), independent of climb-target input.
pub const CLIMB_SWEEP_DEFAULT_TO_PA_FT: f64 = 4000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ClimbSweepParams {
    pub from_pa_ft: f64,
    pub to_pa_ft: f64,
    pub step_ft: f64,
    pub lapse_rate_c_per_ft: f64,
}

impl Default for ClimbSweepParams {
    fn default() -> Self {
        Self {
            from_pa_ft: 0.0,
            to_pa_ft: 0.0,
            step_ft: CLIMB_SWEEP_DEFAULT_STEP_FT,
            lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClimbSweepStopReason {
    OutOfChart,
    ServiceCeiling,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClimbSweepRow {
    pub pa_ft: f64,
    pub oat_c: f64,
    pub isa_dev_c: f64,
    pub density_altitude_ft: Option<f64>,
    pub rate_of_climb_fpm: Option<f64>,
    /// Elapsed time from sweep start (seconds); `None` when not accumulating (down sweep or after stop).
    pub cumulative_time_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClimbSweepResult {
    pub rows: Vec<ClimbSweepRow>,
    pub total_time_s: Option<f64>,
    pub stop_reason: Option<ClimbSweepStopReason>,
    pub sweeping_up: bool,
}

pub type ClimbSweepError = AltitudeSweepRangeError;

pub fn compute_climb_roc_sweep(
    data: &PerformanceData,
    base: &FlightInputs,
    params: &ClimbSweepParams,
) -> Result<ClimbSweepResult, ClimbSweepError> {
    let pa_series = sweep_pa_series_with_terminal(
        build_altitude_series(
            params.from_pa_ft,
            params.to_pa_ft,
            params.step_ft,
            CLIMB_SWEEP_MAX_ROWS,
        )?,
        params.to_pa_ft,
        CLIMB_SWEEP_MAX_ROWS,
    )?;
    let pa_start_ft = effective_pressure_altitude_ft(base, Some(ChartPhase::Climb));
    let oat_start_c = base.oat_c;
    let lapse = params.lapse_rate_c_per_ft;
    let target_pa_ft = params.to_pa_ft;
    let sweeping_up = params.step_ft > 0.0 && params.to_pa_ft >= params.from_pa_ft;

    let mut rows = Vec::with_capacity(pa_series.len());
    let mut stop_reason = None;
    let mut accumulate_time = sweeping_up;
    let mut cumulative_min = 0.0;
    let mut prev_valid_pa: Option<f64> = None;

    for (idx, pa_ft) in pa_series.iter().enumerate() {
        let oat_c = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, *pa_ft, lapse);
        let isa_dev_c = disa_c(oat_c, *pa_ft);
        let da = isa_troposphere_density_altitude_ft(*pa_ft, oat_c);
        let inputs = climb_inputs_at_pa(base, pa_start_ft, oat_start_c, *pa_ft, lapse);

        let (rate_of_climb_fpm, row_valid) = match compute_climb(data, &inputs, target_pa_ft) {
            Ok(r) if r.rate_of_climb_fpm > AFM_SERVICE_CEILING_ROC_FPM => {
                (Some(r.rate_of_climb_fpm), true)
            }
            Ok(_) => {
                if stop_reason.is_none() && idx > 0 {
                    stop_reason = Some(ClimbSweepStopReason::ServiceCeiling);
                }
                (None, false)
            }
            Err(_) => {
                if stop_reason.is_none() && idx > 0 {
                    stop_reason = Some(ClimbSweepStopReason::OutOfChart);
                }
                (None, false)
            }
        };

        let cumulative_time_s = if !accumulate_time {
            None
        } else if idx == 0 {
            Some(0.0)
        } else if !row_valid {
            accumulate_time = false;
            None
        } else {
            if let Some(prev_pa) = prev_valid_pa {
                if let Some(seg_min) = integrate_climb_time_min(
                    data,
                    base,
                    pa_start_ft,
                    oat_start_c,
                    lapse,
                    target_pa_ft,
                    prev_pa,
                    *pa_ft,
                ) {
                    cumulative_min += seg_min;
                    Some(cumulative_min * 60.0)
                } else {
                    accumulate_time = false;
                    None
                }
            } else {
                Some(0.0)
            }
        };

        if row_valid {
            prev_valid_pa = Some(*pa_ft);
        }

        rows.push(ClimbSweepRow {
            pa_ft: *pa_ft,
            oat_c,
            isa_dev_c,
            density_altitude_ft: Some(da),
            rate_of_climb_fpm,
            cumulative_time_s,
        });

        if !row_valid && idx == 0 {
            break;
        }
        if !row_valid {
            // fill remaining rows as empty
            for rest in pa_series.iter().skip(idx + 1) {
                let oat_c = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, *rest, lapse);
                rows.push(ClimbSweepRow {
                    pa_ft: *rest,
                    oat_c,
                    isa_dev_c: disa_c(oat_c, *rest),
                    density_altitude_ft: Some(isa_troposphere_density_altitude_ft(*rest, oat_c)),
                    rate_of_climb_fpm: None,
                    cumulative_time_s: None,
                });
            }
            break;
        }
    }

    let total_time_s = if sweeping_up && accumulate_time {
        rows.last().and_then(|r| r.cumulative_time_s).or({
            if rows.len() == 1 {
                Some(0.0)
            } else {
                None
            }
        })
    } else {
        None
    };

    Ok(ClimbSweepResult {
        rows,
        total_time_s,
        stop_reason,
        sweeping_up,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climb::compute_climb;
    use crate::cruise::isa_temp_c_at_pa;
    use crate::data::PerformanceData;

    #[test]
    fn fig52_default_hand_check_values() {
        let data = PerformanceData::production();
        let inputs = FlightInputs::default();
        let target = inputs.cruise_pressure_alt_ft;
        let readout = compute_climb(&data, &inputs, target).unwrap();
        let pa = effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Climb));
        let sweep = compute_climb_roc_sweep(
            &data,
            &inputs,
            &ClimbSweepParams {
                from_pa_ft: pa,
                to_pa_ft: target,
                step_ft: CLIMB_SWEEP_DEFAULT_STEP_FT,
                lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
            },
        )
        .unwrap();
        let row = &sweep.rows[0];
        assert!((row.rate_of_climb_fpm.unwrap() - readout.rate_of_climb_fpm).abs() < 1e-6);
        eprintln!(
            "Hand-check: PA={:.0} ft, OAT={:.1} °C, Fig 5.2 ROC readout={:.0} fpm, sweep row={:.0} fpm",
            row.pa_ft,
            row.oat_c,
            readout.rate_of_climb_fpm,
            row.rate_of_climb_fpm.unwrap()
        );
    }

    #[test]
    fn start_row_matches_climb_readout() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 2000.0,
            oat_c: 20.0,
            ..Default::default()
        };
        let target = 8000.0;
        let readout = compute_climb(&data, &inputs, target).unwrap();
        let pa_start = effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Climb));
        let params = ClimbSweepParams {
            from_pa_ft: pa_start,
            to_pa_ft: target,
            step_ft: 500.0,
            lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
        };
        let sweep = compute_climb_roc_sweep(&data, &inputs, &params).unwrap();
        let first = &sweep.rows[0];
        assert!((first.pa_ft - pa_start).abs() < 1e-9);
        let roc = first.rate_of_climb_fpm.unwrap();
        assert!((roc - readout.rate_of_climb_fpm).abs() < 1e-6);
    }

    #[test]
    fn isa_day_sweep_0_to_10k_monotone_decreasing_roc() {
        let data = PerformanceData::production();
        let pa0 = 0.0;
        let inputs = FlightInputs {
            pressure_altitude_input_ft: pa0,
            oat_c: isa_temp_c_at_pa(pa0),
            ..Default::default()
        };
        let params = ClimbSweepParams {
            from_pa_ft: 0.0,
            to_pa_ft: 10_000.0,
            step_ft: 1000.0,
            lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
        };
        let sweep = compute_climb_roc_sweep(&data, &inputs, &params).unwrap();
        let mut prev = f64::MAX;
        for row in &sweep.rows {
            let Some(roc) = row.rate_of_climb_fpm else {
                break;
            };
            assert!(roc <= prev + 1e-6, "roc increased at pa={}", row.pa_ft);
            prev = roc;
        }
    }

    #[test]
    fn downward_sweep_mirrors_upward_at_same_pas() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 6000.0,
            oat_c: 10.0,
            ..Default::default()
        };
        let up = compute_climb_roc_sweep(
            &data,
            &inputs,
            &ClimbSweepParams {
                from_pa_ft: 2000.0,
                to_pa_ft: 6000.0,
                step_ft: 1000.0,
                ..Default::default()
            },
        )
        .unwrap();
        let down = compute_climb_roc_sweep(
            &data,
            &inputs,
            &ClimbSweepParams {
                from_pa_ft: 6000.0,
                to_pa_ft: 2000.0,
                step_ft: -1000.0,
                ..Default::default()
            },
        )
        .unwrap();
        for u in &up.rows {
            if let Some(d) = down.rows.iter().find(|r| (r.pa_ft - u.pa_ft).abs() < 1e-9) {
                assert_eq!(u.rate_of_climb_fpm, d.rate_of_climb_fpm);
            }
        }
    }

    #[test]
    fn out_of_chart_row_has_no_roc() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 17_000.0,
            oat_c: 15.0,
            ..Default::default()
        };
        let sweep = compute_climb_roc_sweep(
            &data,
            &inputs,
            &ClimbSweepParams {
                from_pa_ft: 17_000.0,
                to_pa_ft: 18_000.0,
                step_ft: 1000.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(sweep.rows.iter().any(|r| r.rate_of_climb_fpm.is_none()));
    }

    #[test]
    fn sweep_2130_to_4000_includes_terminal_and_total_time_matches_integral() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 2130.0,
            oat_c: 20.0,
            ..Default::default()
        };
        let lapse = DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT;
        let to = CLIMB_SWEEP_DEFAULT_TO_PA_FT;
        let sweep = compute_climb_roc_sweep(
            &data,
            &inputs,
            &ClimbSweepParams {
                from_pa_ft: 2130.0,
                to_pa_ft: to,
                step_ft: CLIMB_SWEEP_DEFAULT_STEP_FT,
                lapse_rate_c_per_ft: lapse,
            },
        )
        .unwrap();
        assert!(
            sweep.rows.iter().any(|r| (r.pa_ft - to).abs() < 1e-3),
            "last pas: {:?}",
            sweep.rows.iter().map(|r| r.pa_ft).collect::<Vec<_>>()
        );
        let ref_min = integrate_climb_time_min(&data, &inputs, 2130.0, 20.0, lapse, to, 2130.0, to)
            .expect("reference integral");
        let ref_s = ref_min * 60.0;
        let total = sweep.total_time_s.expect("total time");
        assert!(
            (total - ref_s).abs() < 1.0,
            "sweep total {total}s vs reference {ref_s}s"
        );
    }
}
