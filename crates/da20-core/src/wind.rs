use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::types::{CalculationWarning, FlightInputs, WindComponents, WindResult};

/// Parse runway orientation: designator (1–2 digits, optional L/R/C) × 10°, or 3-digit magnetic heading.
pub fn runway_heading_deg(runway: &str) -> Option<f64> {
    let s = runway.trim();
    if s.is_empty() {
        return None;
    }
    let upper: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    let designator = upper.trim_end_matches(['L', 'R', 'C']);
    let digits: String = designator
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    match digits.len() {
        3 => {
            let deg: f64 = digits.parse().ok()?;
            if (0.0..=360.0).contains(&deg) {
                Some(deg)
            } else {
                None
            }
        }
        1 | 2 => {
            let n: f64 = digits.parse().ok()?;
            if (0.0..=36.0).contains(&n) {
                Some(n * 10.0)
            } else {
                None
            }
        }
        _ => None,
    }
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
    let mut warnings = Vec::new();
    let rwy = match runway_heading_deg(&inputs.runway) {
        Some(h) => h,
        None => {
            warnings.push(CalculationWarning::with_args(
                "RUNWAY_HEADING",
                vec![inputs.runway.clone()],
            ));
            0.0
        }
    };
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
    fn runway_designator_and_magnetic_heading() {
        assert_eq!(runway_heading_deg("19"), Some(190.0));
        assert_eq!(runway_heading_deg("193"), Some(193.0));
        assert_eq!(runway_heading_deg("09L"), Some(90.0));
        assert_eq!(runway_heading_deg(" 19R "), Some(190.0));
        assert!((runway_heading_deg("193").unwrap() - runway_heading_deg("19").unwrap()).abs() > 0.5);
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
