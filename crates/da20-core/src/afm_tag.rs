//! Derive AFM grid-vs-interpolation tags for phase readouts (bilingual labels in UI).

use crate::types::AfmLookupTag;
use crate::types::CalculationWarning;

/// Landing Table 4: clamp to table edge counts as extrapolated/off-chart; interior interpolation otherwise.
pub fn landing_tag_from_pa(
    warnings: &[CalculationWarning],
    pa_ft: f64,
    published_pa: &[f64],
) -> AfmLookupTag {
    let clamped = warnings.iter().any(|w| {
        matches!(w.code.as_str(), "CLAMP_LOW" | "CLAMP_HIGH")
            && w.args
                .first()
                .map(|a| a.contains("Table 4"))
                .unwrap_or(false)
    });
    if clamped {
        return AfmLookupTag::Extrapolated;
    }
    let exact = published_pa.iter().any(|p| (p - pa_ft).abs() < 0.5);
    if exact {
        AfmLookupTag::Table
    } else {
        AfmLookupTag::Interpolated
    }
}

/// Takeoff: off-chart clamp → extrapolated; otherwise chart chain is between tabulated points.
pub fn takeoff_tag(chart_off_afm: bool) -> AfmLookupTag {
    if chart_off_afm {
        AfmLookupTag::Extrapolated
    } else {
        AfmLookupTag::Interpolated
    }
}

/// Climb ROC: extrapolated if Fig 5.2 equivalent-altitude path used clamp/extrapolation; else interpolated.
pub fn climb_tag_from_warnings(warnings: &[CalculationWarning]) -> AfmLookupTag {
    let fig52 = |w: &CalculationWarning| {
        w.args
            .first()
            .map(|s| s.contains("5.2") || s.contains("Fig 5.2"))
            .unwrap_or(false)
    };
    let bad = warnings.iter().any(|w| match w.code.as_str() {
        "CHART_CLAMP_LOW" | "CHART_CLAMP_HIGH" => fig52(w),
        "CLAMP_LOW" | "CLAMP_HIGH" | "EXTRAP_LOW" | "EXTRAP_HIGH" => fig52(w),
        _ => false,
    });
    if bad {
        AfmLookupTag::Extrapolated
    } else {
        AfmLookupTag::Interpolated
    }
}
