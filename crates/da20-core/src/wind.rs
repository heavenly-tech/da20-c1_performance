use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, FlightInputs, WindComponents, WindResult};

/// Validation state for the runway heading field (3-digit magnetic only, 001–360).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunwayHeadingStatus {
    Empty,
    Valid,
    /// One or two digits only (e.g. `19` — could mean 019°).
    AmbiguousShort,
    Invalid,
}

/// Classify runway heading input without parsing a degree value.
pub fn runway_heading_status(runway: &str) -> RunwayHeadingStatus {
    let s = runway.trim();
    if s.is_empty() {
        return RunwayHeadingStatus::Empty;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        match s.len() {
            1 | 2 => RunwayHeadingStatus::AmbiguousShort,
            3 => {
                let deg: u32 = s.parse().unwrap_or(0);
                if (1..=360).contains(&deg) {
                    RunwayHeadingStatus::Valid
                } else {
                    RunwayHeadingStatus::Invalid
                }
            }
            _ => RunwayHeadingStatus::Invalid,
        }
    } else {
        RunwayHeadingStatus::Invalid
    }
}

/// Magnetic runway heading in degrees (001–360), or `None` if not exactly three valid digits.
pub fn runway_heading_deg(runway: &str) -> Option<f64> {
    let s = runway.trim();
    if runway_heading_status(s) != RunwayHeadingStatus::Valid {
        return None;
    }
    s.parse().ok()
}

pub fn wind_components(
    wind_dir_deg: f64,
    wind_speed_kt: f64,
    runway_heading_deg: f64,
) -> WindComponents {
    let angle_deg = (wind_dir_deg - runway_heading_deg).to_radians();
    let headwind = wind_speed_kt * angle_deg.cos();
    let crosswind = wind_speed_kt * angle_deg.sin();
    WindComponents {
        headwind_kt: headwind,
        crosswind_kt: crosswind,
        along_runway_headwind_kt: headwind,
    }
}

/// Headwind and tailwind magnitudes (kt), each clamped to ≥ 0.
pub fn headwind_tailwind_kt(components: &WindComponents) -> (f64, f64) {
    let hw = components.headwind_kt.max(0.0);
    let tw = (-components.headwind_kt).max(0.0);
    (hw, tw)
}

const COS_EPS: f64 = 0.05;

/// Keep wind direction fixed; solve speed for a target along-runway headwind (strip/chart drag).
pub fn back_solve_wind_for_headwind(
    wind_dir_deg: f64,
    runway_heading_deg: f64,
    target_headwind_kt: f64,
) -> (f64, f64, bool) {
    let target = target_headwind_kt.max(0.0);
    let angle = (wind_dir_deg - runway_heading_deg).to_radians();
    let cos = angle.cos();
    if cos.abs() > COS_EPS {
        if cos > 0.0 {
            let speed = (target / cos).max(0.0);
            (wind_dir_deg, speed, false)
        } else {
            (wind_dir_deg, 0.0, false)
        }
    } else {
        // Nearly pure crosswind: cannot rotate direction; treat target as speed (crosswind-limited).
        (wind_dir_deg, target, true)
    }
}

pub fn compute_wind(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<WindResult, WindError> {
    let status = runway_heading_status(&inputs.runway);
    if status != RunwayHeadingStatus::Valid {
        return Err(if status == RunwayHeadingStatus::AmbiguousShort {
            WindError::AmbiguousRunwayHeading
        } else {
            WindError::InvalidRunwayHeading
        });
    }
    let mut warnings = Vec::new();
    let rwy = runway_heading_deg(&inputs.runway).expect("validated above");
    let components = wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy);
    if components.along_runway_headwind_kt < 0.0 {
        warnings.push(CalculationWarning::with_args(
            "TAILWIND",
            vec![format!("{:.1}", -components.along_runway_headwind_kt)],
        ));
    }
    Ok(WindResult {
        provenance: provenance_for(data.inner(), data.wind_source()),
        components,
        takeoff_landing_correction_ft: 0.0,
        warnings,
    })
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum WindError {
    #[error("wind calculation failed")]
    Failed,
    #[error("invalid runway heading")]
    InvalidRunwayHeading,
    #[error("ambiguous runway heading")]
    AmbiguousRunwayHeading,
}

/// One decimal place for component readout (kt).
pub fn format_component_kt(kt: f64) -> String {
    format!("{:.1}", kt)
}

pub fn along_runway_display_kt(components: &WindComponents) -> (bool, f64) {
    let (hw, tw) = headwind_tailwind_kt(components);
    const EPS: f64 = 0.05;
    if hw >= EPS {
        (true, hw)
    } else if tw >= EPS {
        (false, tw)
    } else {
        (true, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fig_5_3_worked_example_components() {
        let c = wind_components(30.0, 11.0, 0.0);
        assert!((c.headwind_kt - 9.5).abs() <= 0.1);
        assert!((c.crosswind_kt - 5.5).abs() <= 0.1);
    }

    #[test]
    fn runway_heading_three_digit_magnetic_only() {
        for ok in ["190", "193", "019", "360", "001", " 360 "] {
            assert_eq!(
                runway_heading_status(ok),
                RunwayHeadingStatus::Valid,
                "{ok}"
            );
            assert!(runway_heading_deg(ok).is_some(), "{ok}");
        }
        assert_eq!(runway_heading_deg("019").unwrap(), 19.0);
        assert_eq!(runway_heading_deg("360").unwrap(), 360.0);

        for bad in ["19", "9", "000", "361", "09L", "abc"] {
            assert_ne!(
                runway_heading_status(bad),
                RunwayHeadingStatus::Valid,
                "{bad}"
            );
            assert!(runway_heading_deg(bad).is_none(), "{bad}");
        }
        assert_eq!(
            runway_heading_status("19"),
            RunwayHeadingStatus::AmbiguousShort
        );
        assert_eq!(
            runway_heading_status("9"),
            RunwayHeadingStatus::AmbiguousShort
        );
        assert_eq!(runway_heading_status("000"), RunwayHeadingStatus::Invalid);
        assert_eq!(runway_heading_status("09L"), RunwayHeadingStatus::Invalid);
    }

    #[test]
    fn headwind_tailwind_sign_clamp() {
        let c = wind_components(180.0, 10.0, 0.0);
        let (hw, tw) = headwind_tailwind_kt(&c);
        assert_eq!(hw, 0.0);
        assert!((tw - 10.0).abs() < 0.01);
    }

    #[test]
    fn back_solve_preserves_direction() {
        let rwy = 190.0;
        let dir = 220.0;
        let (d, s, clamp) = back_solve_wind_for_headwind(dir, rwy, 8.0);
        assert_eq!(d, dir);
        assert!(!clamp);
        let c = wind_components(d, s, rwy);
        assert!((c.headwind_kt - 8.0).abs() < 0.2);
    }

    #[test]
    fn back_solve_crosswind_limited_without_rotating_dir() {
        let rwy = 0.0;
        let dir = 88.0;
        let (d, s, clamp) = back_solve_wind_for_headwind(dir, rwy, 6.0);
        assert_eq!(d, dir);
        assert!(clamp);
        assert!((s - 6.0).abs() < f64::EPSILON);
    }
}
