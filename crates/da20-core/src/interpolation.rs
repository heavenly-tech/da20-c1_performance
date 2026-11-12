use crate::types::CalculationWarning;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtrapolationPolicy {
    /// Return error when query is outside grid bounds (only policy used in this app).
    Strict,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum InterpError {
    #[error("empty axis")]
    EmptyAxis,
    #[error("axis/value length mismatch")]
    LengthMismatch,
    #[error("query below axis minimum ({min})")]
    BelowMin { min: f64 },
    #[error("query above axis maximum ({max})")]
    AboveMax { max: f64 },
}

/// One-dimensional linear interpolation.
pub fn lerp(x0: f64, y0: f64, x1: f64, y1: f64, x: f64) -> f64 {
    if (x1 - x0).abs() < f64::EPSILON {
        return y0;
    }
    let t = (x - x0) / (x1 - x0);
    y0 + t * (y1 - y0)
}

pub fn interpolate_1d(
    axis: &[f64],
    values: &[f64],
    query: f64,
    policy: ExtrapolationPolicy,
) -> Result<(f64, Vec<CalculationWarning>), InterpError> {
    let _ = policy;
    if axis.is_empty() || values.is_empty() {
        return Err(InterpError::EmptyAxis);
    }
    if axis.len() != values.len() {
        return Err(InterpError::LengthMismatch);
    }
    if axis.len() == 1 {
        if (query - axis[0]).abs() > 1e-9 {
            return Err(if query < axis[0] {
                InterpError::BelowMin { min: axis[0] }
            } else {
                InterpError::AboveMax { max: axis[0] }
            });
        }
        return Ok((values[0], vec![]));
    }

    let min = axis[0];
    let max = axis[axis.len() - 1];
    if query < min - 1e-9 {
        return Err(InterpError::BelowMin { min });
    }
    if query > max + 1e-9 {
        return Err(InterpError::AboveMax { max });
    }

    let idx = bracket_index(axis, query);
    if idx + 1 >= axis.len() {
        return Ok((values[axis.len() - 1], vec![]));
    }
    let y = lerp(
        axis[idx],
        values[idx],
        axis[idx + 1],
        values[idx + 1],
        query,
    );
    Ok((y, vec![]))
}

fn bracket_index(axis: &[f64], q: f64) -> usize {
    let mut i = 0usize;
    while i + 1 < axis.len() && axis[i + 1] <= q {
        i += 1;
    }
    i.min(axis.len().saturating_sub(2))
}

/// Bilinear interpolation on a rectilinear grid (strict axes only).
pub fn interpolate_2d(
    x_axis: &[f64],
    y_axis: &[f64],
    grid: &[Vec<f64>],
    xq: f64,
    yq: f64,
    policy: ExtrapolationPolicy,
) -> Result<(f64, Vec<CalculationWarning>), InterpError> {
    if grid.is_empty() || grid[0].is_empty() {
        return Err(InterpError::EmptyAxis);
    }
    if grid.len() != y_axis.len() {
        return Err(InterpError::LengthMismatch);
    }
    for row in grid {
        if row.len() != x_axis.len() {
            return Err(InterpError::LengthMismatch);
        }
    }

    let min_x = x_axis[0];
    let max_x = x_axis[x_axis.len() - 1];
    if xq < min_x - 1e-9 {
        return Err(InterpError::BelowMin { min: min_x });
    }
    if xq > max_x + 1e-9 {
        return Err(InterpError::AboveMax { max: max_x });
    }
    let min_y = y_axis[0];
    let max_y = y_axis[y_axis.len() - 1];
    if yq < min_y - 1e-9 {
        return Err(InterpError::BelowMin { min: min_y });
    }
    if yq > max_y + 1e-9 {
        return Err(InterpError::AboveMax { max: max_y });
    }
    let xq_eff = xq;
    let yq_eff = yq;

    let xi = bracket_index(x_axis, xq_eff);
    let yi = bracket_index(y_axis, yq_eff);

    if x_axis.len() == 1 && y_axis.len() == 1 {
        return Ok((grid[0][0], vec![]));
    }
    if x_axis.len() == 1 {
        let (v, w) = interpolate_1d(y_axis, &grid[0], yq_eff, policy)?;
        return Ok((v, w));
    }
    if y_axis.len() == 1 {
        let row = &grid[0];
        let (v, w) = interpolate_1d(x_axis, row, xq_eff, policy)?;
        return Ok((v, w));
    }

    let x0 = x_axis[xi];
    let x1 = x_axis[xi + 1];
    let y0 = y_axis[yi];
    let y1 = y_axis[yi + 1];

    let q11 = grid[yi][xi];
    let q21 = grid[yi][xi + 1];
    let q12 = grid[yi + 1][xi];
    let q22 = grid[yi + 1][xi + 1];

    let r1 = lerp(x0, q11, x1, q21, xq_eff);
    let r2 = lerp(x0, q12, x1, q22, xq_eff);
    let v = lerp(y0, r1, y1, r2, yq_eff);
    Ok((v, vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_midpoint() {
        assert!((lerp(0.0, 0.0, 10.0, 100.0, 5.0) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn interpolate_1d_interior() {
        let (v, w) = interpolate_1d(
            &[0.0, 10.0],
            &[0.0, 100.0],
            5.0,
            ExtrapolationPolicy::Strict,
        )
        .unwrap();
        assert!((v - 50.0).abs() < 1e-9);
        assert!(w.is_empty());
    }

    #[test]
    fn interpolate_1d_below_strict_errors() {
        let err = interpolate_1d(
            &[0.0, 10.0],
            &[0.0, 100.0],
            -5.0,
            ExtrapolationPolicy::Strict,
        )
        .unwrap_err();
        assert_eq!(err, InterpError::BelowMin { min: 0.0 });
    }
}
