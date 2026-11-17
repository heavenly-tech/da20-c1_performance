//! Shared pressure-altitude sweep helpers (lapse OAT, row series).

use thiserror::Error;

use crate::altitude::ISA_LAPSE_RATE_C_PER_FT;

/// Default lapse for altitude sweeps (ISA).
pub const DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT: f64 = ISA_LAPSE_RATE_C_PER_FT;

#[derive(Debug, Error, PartialEq)]
pub enum AltitudeSweepRangeError {
    #[error("non-finite sweep value")]
    NonFinite,
    #[error("step must be non-zero")]
    ZeroStep,
    #[error("step sign does not match start/end range")]
    RangeDirection,
    #[error("empty sweep range")]
    EmptyRange,
    #[error("too many rows (max {max})")]
    TooManyRows { max: usize },
}

pub fn oat_at_pa_with_lapse(
    oat_start_c: f64,
    pa_start_ft: f64,
    pa_ft: f64,
    lapse_rate_c_per_ft: f64,
) -> f64 {
    oat_start_c - lapse_rate_c_per_ft * (pa_ft - pa_start_ft)
}

pub fn build_altitude_series(
    start: f64,
    end: f64,
    step: f64,
    max_rows: usize,
) -> Result<Vec<f64>, AltitudeSweepRangeError> {
    if !(start.is_finite() && end.is_finite() && step.is_finite()) {
        return Err(AltitudeSweepRangeError::NonFinite);
    }
    if step == 0.0 {
        return Err(AltitudeSweepRangeError::ZeroStep);
    }
    let mut values = Vec::new();
    if step > 0.0 {
        if start > end {
            return Err(AltitudeSweepRangeError::RangeDirection);
        }
        let mut v = start;
        let mut guard = 0usize;
        while v <= end + step.abs() * 1e-9 {
            values.push(v);
            v += step;
            guard += 1;
            if guard > max_rows + 10 {
                break;
            }
        }
    } else {
        if start < end {
            return Err(AltitudeSweepRangeError::RangeDirection);
        }
        let mut v = start;
        let mut guard = 0usize;
        while v >= end - step.abs() * 1e-9 {
            values.push(v);
            v += step;
            guard += 1;
            if guard > max_rows + 10 {
                break;
            }
        }
    }
    if values.is_empty() {
        return Err(AltitudeSweepRangeError::EmptyRange);
    }
    if values.len() > max_rows {
        return Err(AltitudeSweepRangeError::TooManyRows { max: max_rows });
    }
    Ok(values)
}

/// Climb/cruise sweeps only (not takeoff): append `terminal_pa_ft` when the step grid
/// does not land on it, respecting `max_rows` (replace last sample if at cap).
pub fn sweep_pa_series_with_terminal(
    mut series: Vec<f64>,
    terminal_pa_ft: f64,
    max_rows: usize,
) -> Result<Vec<f64>, AltitudeSweepRangeError> {
    const TOL_FT: f64 = 1e-3;
    let needs_terminal = series
        .last()
        .map(|last| (last - terminal_pa_ft).abs() > TOL_FT)
        .unwrap_or(true);
    if needs_terminal {
        if series.len() < max_rows {
            series.push(terminal_pa_ft);
        } else if let Some(last) = series.last_mut() {
            *last = terminal_pa_ft;
        }
    }
    if series.len() > max_rows {
        return Err(AltitudeSweepRangeError::TooManyRows { max: max_rows });
    }
    Ok(series)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lapse_zero_holds_oat() {
        let start_oat = 22.0;
        let pa0 = 3000.0;
        for pa in [3000.0, 4500.0, 9000.0] {
            let oat = oat_at_pa_with_lapse(start_oat, pa0, pa, 0.0);
            assert!((oat - start_oat).abs() < 1e-9, "pa={pa} oat={oat}");
        }
    }

    #[test]
    fn default_lapse_matches_isa_export() {
        assert!((DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT - ISA_LAPSE_RATE_C_PER_FT).abs() < 1e-12);
    }

    #[test]
    fn terminal_appended_when_step_grid_misses_end() {
        let grid = build_altitude_series(2130.0, 4000.0, 500.0, 60).unwrap();
        assert!((grid.last().unwrap() - 3630.0).abs() < 1e-3);
        let with_end = sweep_pa_series_with_terminal(grid, 4000.0, 60).unwrap();
        assert!((with_end.last().unwrap() - 4000.0).abs() < 1e-3);
    }
}
