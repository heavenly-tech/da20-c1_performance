//! Derive AFM grid-vs-interpolation tags for phase readouts (bilingual labels in UI).

use crate::types::AfmLookupTag;
use crate::types::CalculationWarning;

pub fn landing_tag_from_pa(pa_ft: f64, published_pa: &[f64]) -> AfmLookupTag {
    let exact = published_pa.iter().any(|p| (p - pa_ft).abs() < 0.5);
    if exact {
        AfmLookupTag::Table
    } else {
        AfmLookupTag::Interpolated
    }
}

pub fn takeoff_tag() -> AfmLookupTag {
    AfmLookupTag::Interpolated
}

pub fn climb_tag_from_warnings(_warnings: &[CalculationWarning]) -> AfmLookupTag {
    AfmLookupTag::Interpolated
}
