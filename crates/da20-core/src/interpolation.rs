use crate::types::CalculationWarning;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtrapolationPolicy {
    /// Return error when query is outside grid bounds.
    Strict,
    /// Clamp to nearest edge and emit warning.
    Clamp,
    /// Linear extrapolation beyond edges and emit warning.
    AllowExtrapolate,
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
    if axis.is_empty() || values.is_empty() {
        return Err(InterpError::EmptyAxis);
    }
    if axis.len() != values.len() {
        return Err(InterpError::LengthMismatch);
    }
    if axis.len() == 1 {
        return Ok((values[0], vec![]));
    }

    let mut warnings = Vec::new();
    let (q, extrapolated) = clamp_or_extrapolate_query(axis, query, policy, &mut warnings)?;
    if extrapolated {
        // already warned in helper
    }

    let idx = bracket_index(axis, q);
    if idx + 1 >= axis.len() {
        return Ok((values[axis.len() - 1], warnings));
    }
    let y = lerp(axis[idx], values[idx], axis[idx + 1], values[idx + 1], q);
    Ok((y, warnings))
}

fn bracket_index(axis: &[f64], q: f64) -> usize {
    let mut i = 0usize;
    while i + 1 < axis.len() && axis[i + 1] <= q {
        i += 1;
    }
    i.min(axis.len().saturating_sub(2))
}

fn clamp_or_extrapolate_query(
    axis: &[f64],
    query: f64,
    policy: ExtrapolationPolicy,
    warnings: &mut Vec<CalculationWarning>,
) -> Result<(f64, bool), InterpError> {
    let min = axis[0];
    let max = axis[axis.len() - 1];
    if query < min {
        return match policy {
            ExtrapolationPolicy::Strict => Err(InterpError::BelowMin { min }),
            ExtrapolationPolicy::Clamp => {
                warnings.push(CalculationWarning {
                    code: "CLAMP_LOW".into(),
                    message: format!("Query {query} below axis min {min}; clamped."),
                });
                Ok((min, true))
            }
            ExtrapolationPolicy::AllowExtrapolate => {
                warnings.push(CalculationWarning {
                    code: "EXTRAP_LOW".into(),
                    message: format!("Query {query} below axis min {min}; extrapolating."),
                });
                Ok((query, true))
            }
        };
    }
    if query > max {
        return match policy {
            ExtrapolationPolicy::Strict => Err(InterpError::AboveMax { max }),
            ExtrapolationPolicy::Clamp => {
                warnings.push(CalculationWarning {
                    code: "CLAMP_HIGH".into(),
                    message: format!("Query {query} above axis max {max}; clamped."),
                });
                Ok((max, true))
            }
            ExtrapolationPolicy::AllowExtrapolate => {
                warnings.push(CalculationWarning {
                    code: "EXTRAP_HIGH".into(),
                    message: format!("Query {query} above axis max {max}; extrapolating."),
                });
                Ok((query, true))
            }
        };
    }
    Ok((query, false))
}

/// Bilinear interpolation on a rectilinear grid.
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

    let mut warnings = Vec::new();
    let (xq_eff, _) = clamp_or_extrapolate_query(x_axis, xq, policy, &mut warnings)?;
    let (yq_eff, _) = clamp_or_extrapolate_query(y_axis, yq, policy, &mut warnings)?;

    let xi = bracket_index(x_axis, xq_eff);
    let yi = bracket_index(y_axis, yq_eff);

    if x_axis.len() == 1 && y_axis.len() == 1 {
        return Ok((grid[0][0], warnings));
    }
    if x_axis.len() == 1 {
        let (v, w) = interpolate_1d(y_axis, &grid[0], yq_eff, policy)?;
        warnings.extend(w);
        return Ok((v, warnings));
    }
    if y_axis.len() == 1 {
        let row = &grid[0];
        let (v, w) = interpolate_1d(x_axis, row, xq_eff, policy)?;
        warnings.extend(w);
        return Ok((v, warnings));
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
    Ok((v, warnings))
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
    fn interpolate_1d_clamp_warns() {
        let (v, w) = interpolate_1d(
            &[0.0, 10.0],
            &[0.0, 100.0],
            -5.0,
            ExtrapolationPolicy::Clamp,
        )
        .unwrap();
        assert!((v - 0.0).abs() < 1e-9);
        assert_eq!(w.len(), 1);
    }
}
