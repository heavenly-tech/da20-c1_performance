use crate::data::PerformanceData;
use crate::takeoff::compute_takeoff;
use crate::types::{defaults, FlightInputs, TakeoffResult};
use thiserror::Error;

/// Maximum sweep rows before the UI rejects the request (avoid browser freeze).
pub const TAKEOFF_SWEEP_MAX_ROWS: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeoffSweepVar {
    OatC,
    SignedHeadwindKt,
    ObstacleHeightM,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakeoffSweepAxis {
    pub var: TakeoffSweepVar,
    pub start: f64,
    pub end: f64,
    pub step: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakeoffSweepRow {
    pub oat_c: f64,
    pub signed_headwind_kt: f64,
    pub obstacle_height_m: f64,
    pub result: Option<TakeoffResult>,
}

#[derive(Debug, Error, PartialEq)]
pub enum TakeoffSweepError {
    #[error("no sweep axes selected")]
    NoAxes,
    #[error("too many sweep axes (max 3)")]
    TooManyAxes,
    #[error("duplicate sweep variable")]
    DuplicateVar,
    #[error("non-finite sweep value")]
    NonFinite,
    #[error("step must be non-zero")]
    ZeroStep,
    #[error("step sign does not match start/end range")]
    RangeDirection,
    #[error("empty sweep range")]
    EmptyRange,
    #[error("too many combinations (max {max})")]
    TooManyCombinations { max: usize },
    #[error("invalid runway heading for sweep")]
    InvalidRunway,
    #[error("obstacle height out of range (0–{max} m)")]
    ObstacleOutOfRange { max: f64 },
}

pub fn build_value_series(start: f64, end: f64, step: f64) -> Result<Vec<f64>, TakeoffSweepError> {
    if !(start.is_finite() && end.is_finite() && step.is_finite()) {
        return Err(TakeoffSweepError::NonFinite);
    }
    if step == 0.0 {
        return Err(TakeoffSweepError::ZeroStep);
    }
    let mut values = Vec::new();
    if step > 0.0 {
        if start > end {
            return Err(TakeoffSweepError::RangeDirection);
        }
        let mut v = start;
        let mut guard = 0usize;
        while v <= end + step.abs() * 1e-9 {
            values.push(v);
            v += step;
            guard += 1;
            if guard > TAKEOFF_SWEEP_MAX_ROWS + 10 {
                break;
            }
        }
    } else {
        if start < end {
            return Err(TakeoffSweepError::RangeDirection);
        }
        let mut v = start;
        let mut guard = 0usize;
        while v >= end - step.abs() * 1e-9 {
            values.push(v);
            v += step;
            guard += 1;
            if guard > TAKEOFF_SWEEP_MAX_ROWS + 10 {
                break;
            }
        }
    }
    if values.is_empty() {
        return Err(TakeoffSweepError::EmptyRange);
    }
    Ok(values)
}

pub fn validate_sweep_axes(axes: &[TakeoffSweepAxis]) -> Result<(), TakeoffSweepError> {
    if axes.is_empty() {
        return Err(TakeoffSweepError::NoAxes);
    }
    if axes.len() > 3 {
        return Err(TakeoffSweepError::TooManyAxes);
    }
    let mut seen = [false; 3];
    for ax in axes {
        let idx = match ax.var {
            TakeoffSweepVar::OatC => 0,
            TakeoffSweepVar::SignedHeadwindKt => 1,
            TakeoffSweepVar::ObstacleHeightM => 2,
        };
        if seen[idx] {
            return Err(TakeoffSweepError::DuplicateVar);
        }
        seen[idx] = true;
        build_value_series(ax.start, ax.end, ax.step)?;
    }
    Ok(())
}

fn cartesian_product(axes: &[TakeoffSweepAxis]) -> Result<Vec<[f64; 3]>, TakeoffSweepError> {
    validate_sweep_axes(axes)?;
    let mut dims: Vec<(TakeoffSweepVar, Vec<f64>)> = Vec::new();
    for ax in axes {
        dims.push((ax.var, build_value_series(ax.start, ax.end, ax.step)?));
    }
    let count: usize = dims.iter().map(|(_, v)| v.len()).product();
    if count > TAKEOFF_SWEEP_MAX_ROWS {
        return Err(TakeoffSweepError::TooManyCombinations {
            max: TAKEOFF_SWEEP_MAX_ROWS,
        });
    }
    let mut combos = Vec::with_capacity(count);
    let mut idx = vec![0usize; dims.len()];
    loop {
        let mut oat = 0.0;
        let mut hw = 0.0;
        let mut obs = 0.0;
        for (i, (var, vals)) in dims.iter().enumerate() {
            let v = vals[idx[i]];
            match var {
                TakeoffSweepVar::OatC => oat = v,
                TakeoffSweepVar::SignedHeadwindKt => hw = v,
                TakeoffSweepVar::ObstacleHeightM => obs = v,
            }
        }
        combos.push([oat, hw, obs]);
        let mut carry = true;
        for i in (0..dims.len()).rev() {
            if carry {
                idx[i] += 1;
                if idx[i] < dims[i].1.len() {
                    carry = false;
                } else {
                    idx[i] = 0;
                }
            }
        }
        if carry {
            break;
        }
    }
    Ok(combos)
}

/// Apply a signed along-runway wind (kt) to `inputs` without changing wind direction when possible.
pub fn apply_sweep_headwind(inputs: &mut FlightInputs, signed_headwind_kt: f64) {
    use crate::wind::{
        back_solve_wind_for_headwind, back_solve_wind_for_tailwind, runway_heading_deg,
    };
    if let Some(rwy) = runway_heading_deg(&inputs.runway) {
        let (dir, speed, clamp) = if signed_headwind_kt >= 0.0 {
            back_solve_wind_for_headwind(inputs.wind_dir_deg, rwy, signed_headwind_kt.max(0.0))
        } else {
            back_solve_wind_for_tailwind(inputs.wind_dir_deg, rwy, (-signed_headwind_kt).max(0.0))
        };
        inputs.wind_dir_deg = dir;
        inputs.wind_speed_kt = speed;
        inputs.wind_strip_crosswind_clamped = clamp;
    }
}

pub fn compute_takeoff_sweep(
    data: &PerformanceData,
    base: &FlightInputs,
    axes: &[TakeoffSweepAxis],
    signed_headwind_base: f64,
) -> Result<Vec<TakeoffSweepRow>, TakeoffSweepError> {
    if crate::wind::runway_heading_status(&base.runway) != crate::wind::RunwayHeadingStatus::Valid {
        return Err(TakeoffSweepError::InvalidRunway);
    }
    let combos = cartesian_product(axes)?;
    let base_oat = base.oat_c;
    let base_hw = signed_headwind_base;
    let base_obs = base.obstacle_height_m;
    let has_oat = axes.iter().any(|a| a.var == TakeoffSweepVar::OatC);
    let has_hw = axes
        .iter()
        .any(|a| a.var == TakeoffSweepVar::SignedHeadwindKt);
    let has_obs = axes
        .iter()
        .any(|a| a.var == TakeoffSweepVar::ObstacleHeightM);

    let mut rows = Vec::with_capacity(combos.len());
    for [oat_v, hw_v, obs_v] in combos {
        let oat = if has_oat { oat_v } else { base_oat };
        let hw = if has_hw { hw_v } else { base_hw };
        let obs = if has_obs { obs_v } else { base_obs };
        if !(0.0..=defaults::OBSTACLE_HEIGHT_MAX_M).contains(&obs) {
            return Err(TakeoffSweepError::ObstacleOutOfRange {
                max: defaults::OBSTACLE_HEIGHT_MAX_M,
            });
        }
        let mut inputs = base.clone();
        inputs.oat_c = oat;
        inputs.obstacle_height_m = obs;
        apply_sweep_headwind(&mut inputs, hw);
        let result = compute_takeoff(data, &inputs).ok();
        rows.push(TakeoffSweepRow {
            oat_c: oat,
            signed_headwind_kt: hw,
            obstacle_height_m: obs,
            result,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;
    use crate::types::defaults;
    use crate::wind::signed_along_runway_kt;

    fn base_inputs() -> FlightInputs {
        FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            oat_c: 21.0,
            weight_lb: defaults::TAKEOFF_REF_WEIGHT_KG / 0.45359237,
            runway: "360".into(),
            wind_dir_deg: 0.0,
            wind_speed_kt: 4.0,
            obstacle_height_m: 5.0,
            ..Default::default()
        }
    }

    #[test]
    fn sweep_single_point_matches_normal_solver() {
        let data = PerformanceData::production();
        let base = base_inputs();
        let hw = signed_along_runway_kt(&base).unwrap();
        let axes = vec![TakeoffSweepAxis {
            var: TakeoffSweepVar::OatC,
            start: 21.0,
            end: 21.0,
            step: 1.0,
        }];
        let rows = compute_takeoff_sweep(&data, &base, &axes, hw).unwrap();
        assert_eq!(rows.len(), 1);
        let direct = compute_takeoff(&data, &base).unwrap();
        let row = rows[0].result.as_ref().unwrap();
        assert!((row.ground_roll_ft - direct.ground_roll_ft).abs() < 1e-6);
        assert!(
            (row.distance_over_obstacle_raw_ft - direct.distance_over_obstacle_raw_ft).abs() < 1e-6
        );
    }

    #[test]
    fn sweep_two_variable_count() {
        let data = PerformanceData::production();
        let base = base_inputs();
        let hw = signed_along_runway_kt(&base).unwrap();
        let axes = vec![
            TakeoffSweepAxis {
                var: TakeoffSweepVar::OatC,
                start: 20.0,
                end: 21.0,
                step: 1.0,
            },
            TakeoffSweepAxis {
                var: TakeoffSweepVar::SignedHeadwindKt,
                start: 0.0,
                end: 4.0,
                step: 2.0,
            },
        ];
        let rows = compute_takeoff_sweep(&data, &base, &axes, hw).unwrap();
        assert_eq!(rows.len(), 6);
    }

    #[test]
    fn sweep_rejects_oversized_grid() {
        let axes = vec![TakeoffSweepAxis {
            var: TakeoffSweepVar::OatC,
            start: 0.0,
            end: 100.0,
            step: 0.1,
        }];
        let err = cartesian_product(&axes).unwrap_err();
        assert!(matches!(err, TakeoffSweepError::TooManyCombinations { .. }));
    }

    #[test]
    fn sweep_three_variables_cartesian() {
        let data = PerformanceData::production();
        let base = base_inputs();
        let hw = signed_along_runway_kt(&base).unwrap();
        let axes = vec![
            TakeoffSweepAxis {
                var: TakeoffSweepVar::OatC,
                start: 20.0,
                end: 21.0,
                step: 1.0,
            },
            TakeoffSweepAxis {
                var: TakeoffSweepVar::SignedHeadwindKt,
                start: 0.0,
                end: 2.0,
                step: 2.0,
            },
            TakeoffSweepAxis {
                var: TakeoffSweepVar::ObstacleHeightM,
                start: 5.0,
                end: 5.0,
                step: 1.0,
            },
        ];
        let rows = compute_takeoff_sweep(&data, &base, &axes, hw).unwrap();
        assert_eq!(rows.len(), 4);
    }

    #[test]
    fn sweep_invalid_range_direction() {
        let err = build_value_series(30.0, 10.0, 1.0).unwrap_err();
        assert_eq!(err, TakeoffSweepError::RangeDirection);
    }
}
