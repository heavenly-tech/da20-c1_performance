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

/// Magnetic wind direction in degrees, 1–360 (never 0).
fn normalize_wind_dir_deg(deg: f64) -> f64 {
    let d = deg.rem_euclid(360.0);
    if d < 1.0 {
        360.0
    } else {
        d
    }
}

fn wind_dir_opposite(deg: f64) -> f64 {
    normalize_wind_dir_deg(deg + 180.0)
}

/// Solve speed (and direction when crossing HW↔TW) for a target along-runway headwind (strip/chart drag).
/// Signed along-runway component (kt): positive headwind, negative tailwind.
pub fn signed_along_runway_kt(inputs: &FlightInputs) -> Option<f64> {
    let rwy = runway_heading_deg(&inputs.runway)?;
    Some(wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy).along_runway_headwind_kt)
}

/// Update `FlightInputs` wind for a signed along-runway value (direction may flip 180° when crossing zero).
pub fn apply_signed_along_runway_wind(inputs: &mut FlightInputs, signed_kt: f64) {
    let Some(rwy) = runway_heading_deg(&inputs.runway) else {
        return;
    };
    let (dir, speed, clamp) = if signed_kt >= 0.0 {
        back_solve_wind_for_headwind(inputs.wind_dir_deg, rwy, signed_kt)
    } else {
        back_solve_wind_for_tailwind(inputs.wind_dir_deg, rwy, (-signed_kt).max(0.0))
    };
    inputs.wind_dir_deg = dir;
    inputs.wind_speed_kt = speed;
    inputs.wind_strip_crosswind_clamped = clamp;
}

pub fn back_solve_wind_for_headwind(
    wind_dir_deg: f64,
    runway_heading_deg: f64,
    target_headwind_kt: f64,
) -> (f64, f64, bool) {
    let target = target_headwind_kt.max(0.0);
    if target < f64::EPSILON {
        return (wind_dir_deg, 0.0, false);
    }
    let angle = (wind_dir_deg - runway_heading_deg).to_radians();
    let cos = angle.cos();
    if cos.abs() > COS_EPS {
        if cos > 0.0 {
            let speed = (target / cos).max(0.0);
            (wind_dir_deg, speed, false)
        } else {
            let speed = (target / cos.abs()).max(0.0);
            (wind_dir_opposite(wind_dir_deg), speed, false)
        }
    } else {
        // Nearly pure crosswind: cannot rotate direction; treat target as speed (crosswind-limited).
        (wind_dir_deg, target, true)
    }
}

/// Solve speed (and direction when crossing HW↔TW) for a target tailwind (strip TW sub-panel drag).
pub fn back_solve_wind_for_tailwind(
    wind_dir_deg: f64,
    runway_heading_deg: f64,
    target_tailwind_kt: f64,
) -> (f64, f64, bool) {
    let target = target_tailwind_kt.max(0.0);
    if target < f64::EPSILON {
        return (wind_dir_deg, 0.0, false);
    }
    let angle = (wind_dir_deg - runway_heading_deg).to_radians();
    let cos = angle.cos();
    if cos.abs() > COS_EPS {
        if cos < 0.0 {
            let speed = (target / -cos).max(0.0);
            (wind_dir_deg, speed, false)
        } else {
            let speed = (target / cos).max(0.0);
            (wind_dir_opposite(wind_dir_deg), speed, false)
        }
    } else {
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

/// Chart/strip wind panels use along-runway component only; show a note when crosswind dominates or strip drag clamped.
pub fn wind_chart_panel_needs_crosswind_note(inputs: &FlightInputs) -> bool {
    if inputs.wind_strip_crosswind_clamped {
        return true;
    }
    if runway_heading_status(&inputs.runway) != RunwayHeadingStatus::Valid {
        return false;
    }
    let Some(rwy) = runway_heading_deg(&inputs.runway) else {
        return false;
    };
    if inputs.wind_speed_kt < 0.05 {
        return false;
    }
    let comp = wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy);
    let (hw, tw) = headwind_tailwind_kt(&comp);
    hw < 0.05 && tw < 0.05 && comp.crosswind_kt.abs() > 0.05
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
    fn pure_crosswind_triggers_chart_panel_note() {
        let inputs = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 280.0,
            wind_speed_kt: 10.0,
            ..Default::default()
        };
        assert!(wind_chart_panel_needs_crosswind_note(&inputs));
        let mut clamped = inputs.clone();
        clamped.wind_strip_crosswind_clamped = true;
        assert!(wind_chart_panel_needs_crosswind_note(&clamped));
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

    #[test]
    fn signed_along_runway_negative_is_tailwind() {
        let inputs = FlightInputs {
            runway: "360".into(),
            wind_dir_deg: 180.0,
            wind_speed_kt: 5.0,
            ..Default::default()
        };
        let signed = signed_along_runway_kt(&inputs).unwrap();
        assert!(signed < 0.0);
    }

    #[test]
    fn apply_signed_along_runway_round_trip() {
        let mut inputs = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 200.0,
            wind_speed_kt: 8.0,
            ..Default::default()
        };
        let before = signed_along_runway_kt(&inputs).unwrap();
        apply_signed_along_runway_wind(&mut inputs, 3.0);
        let after = signed_along_runway_kt(&inputs).unwrap();
        assert!((after - 3.0).abs() < 0.15);
        apply_signed_along_runway_wind(&mut inputs, before);
        assert!((signed_along_runway_kt(&inputs).unwrap() - before).abs() < 0.15);
    }

    #[test]
    fn apply_signed_tailwind_flips_dir_from_default_headwind() {
        let mut inputs = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 200.0,
            wind_speed_kt: 8.0,
            ..Default::default()
        };
        apply_signed_along_runway_wind(&mut inputs, -3.0);
        assert!(
            (inputs.wind_dir_deg - 20.0).abs() < 0.5,
            "dir {}",
            inputs.wind_dir_deg
        );
        assert!(inputs.wind_speed_kt > 0.0);
        let along = signed_along_runway_kt(&inputs).unwrap();
        assert!((along - (-3.0)).abs() < 0.15, "along {along}");
    }

    #[test]
    fn back_solve_zero_kt_keeps_direction_from_both_hw_and_tw() {
        let rwy = 190.0;
        let dir_hw = 200.0;
        let (d_hw, s_hw, _) = back_solve_wind_for_headwind(dir_hw, rwy, 0.0);
        assert_eq!(d_hw, dir_hw);
        assert_eq!(s_hw, 0.0);
        let (d_tw, s_tw, _) = back_solve_wind_for_tailwind(dir_hw, rwy, 0.0);
        assert_eq!(d_tw, dir_hw);
        assert_eq!(s_tw, 0.0);

        let dir_tw = 20.0;
        let (d2, s2, _) = back_solve_wind_for_tailwind(dir_tw, rwy, 0.0);
        assert_eq!(d2, dir_tw);
        assert_eq!(s2, 0.0);
    }

    #[test]
    fn flip_side_reverses_crosswind_sign() {
        let rwy = 190.0;
        let dir = 200.0;
        let speed = 8.0;
        let before = wind_components(dir, speed, rwy);
        assert!(before.crosswind_kt > 0.1);
        let (dir2, speed2, _) = back_solve_wind_for_tailwind(dir, rwy, 3.0);
        assert!((dir2 - 20.0).abs() < 0.5);
        let after = wind_components(dir2, speed2, rwy);
        assert!(after.crosswind_kt < -0.05);
        assert!((after.along_runway_headwind_kt - (-3.0)).abs() < 0.15);
    }

    #[test]
    fn apply_signed_zero_from_hw_and_tw_same_state() {
        use crate::data::PerformanceData;
        use crate::takeoff::compute_takeoff;
        use crate::types::{ChartPhase, PaOverride};

        let data = PerformanceData::production();
        let pa = Some(PaOverride {
            pa_ft: 1000.0,
            phase: ChartPhase::Takeoff,
        });
        let mut from_hw = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 200.0,
            wind_speed_kt: 6.0,
            pa_override: pa,
            oat_c: 21.0,
            ..Default::default()
        };
        apply_signed_along_runway_wind(&mut from_hw, 0.0);
        let mut from_tw = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 20.0,
            wind_speed_kt: 4.0,
            pa_override: pa,
            oat_c: 21.0,
            ..Default::default()
        };
        apply_signed_along_runway_wind(&mut from_tw, 0.0);
        assert_eq!(from_hw.wind_speed_kt, 0.0);
        assert_eq!(from_tw.wind_speed_kt, 0.0);
        assert_eq!(signed_along_runway_kt(&from_hw).unwrap(), 0.0);
        assert_eq!(signed_along_runway_kt(&from_tw).unwrap(), 0.0);
        let t_hw = compute_takeoff(&data, &from_hw).unwrap();
        let t_tw = compute_takeoff(&data, &from_tw).unwrap();
        assert!((t_hw.ground_roll_ft - t_tw.ground_roll_ft).abs() < 1e-6);
        assert!(
            (t_hw.distance_over_obstacle_raw_ft - t_tw.distance_over_obstacle_raw_ft).abs() < 1e-6
        );
    }

    #[test]
    fn wind_dir_opposite_twice_from_north_is_360_not_000() {
        let d = wind_dir_opposite(wind_dir_opposite(0.0));
        assert!((d - 360.0).abs() < 0.01, "dir {d}");
        assert!(d >= 1.0);
    }
}
