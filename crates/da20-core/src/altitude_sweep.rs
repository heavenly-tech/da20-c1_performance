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
}
