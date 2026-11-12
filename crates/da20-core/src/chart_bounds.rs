//! Interpolation-only: values exist only inside digitized chart hulls.

use crate::types::CalculationWarning;

/// Published AFM axis limits used for validation and UI clamps (Rev 29 digitization).
pub mod limits {
    pub const TABLE3_PA_FT: (f64, f64) = (2000.0, 12000.0);
    pub const TABLE3_DISA_C: (f64, f64) = (-20.0, 10.0);
    pub const FIG52_PA_FT: (f64, f64) = (0.0, 16000.0);
    pub const FIG52_OAT_C: (f64, f64) = (-20.0, 60.0);
    pub const FIG54_PA_FT: (f64, f64) = (0.0, 10000.0);
    pub const FIG54_OAT_C: (f64, f64) = (-20.0, 55.0);
    pub const FIG54_WEIGHT_KG: (f64, f64) = (600.0, 800.0);
    pub const FIG54_HEADWIND_KT: (f64, f64) = (0.0, 20.0);
    pub const FIG54_TAILWIND_KT: (f64, f64) = (0.0, 5.0);
    pub const TABLE4_PA_FT: (f64, f64) = (0.0, 7000.0);
}

#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[error("{label} out of chart ({min}..{max})")]
pub struct ChartOutOfRange {
    pub label: &'static str,
    pub min: f64,
    pub max: f64,
}

impl ChartOutOfRange {
    pub fn warning(&self) -> CalculationWarning {
        CalculationWarning::with_args(
            "CHART_OUT_OF_RANGE",
            vec![
                self.label.to_string(),
                format_axis(self.min),
                format_axis(self.max),
            ],
        )
    }

    pub fn contains(&self, value: f64, tol: f64) -> bool {
        value >= self.min - tol && value <= self.max + tol
    }
}

fn format_axis(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

pub fn axis_contains(axis: &[f64], value: f64, tol: f64) -> bool {
    if axis.is_empty() {
        return false;
    }
    value >= axis[0] - tol && value <= axis[axis.len() - 1] + tol
}

pub fn require_in_range(
    label: &'static str,
    min: f64,
    max: f64,
    value: f64,
    tol: f64,
) -> Result<(), ChartOutOfRange> {
    if value < min - tol || value > max + tol {
        Err(ChartOutOfRange { label, min, max })
    } else {
        Ok(())
    }
}

pub fn chart_out_of_range_warnings(warnings: &[CalculationWarning]) -> bool {
    warnings.iter().any(|w| w.code == "CHART_OUT_OF_RANGE")
}
