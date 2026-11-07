use crate::rev29::engine::{ft_to_m, m_to_ft};
use crate::types::TakeoffResult;
use serde::{Deserialize, Serialize};

/// Display rounding for takeoff ground roll and over-obstacle distance (raw solver first).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TakeoffDistanceRounding {
    #[default]
    Raw,
    /// Round up both distances to the next 10 m (exact multiples unchanged).
    Ceil10Meters,
    /// Round up both distances to the next 50 ft, then derive meters from rounded ft.
    Ceil50Feet,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundedTakeoffDistances {
    pub ground_roll_m: f64,
    pub ground_roll_ft: f64,
    pub over_obstacle_m: f64,
    pub over_obstacle_ft: f64,
}

/// Ceil `value` to the next positive multiple of `step`, leaving exact multiples unchanged.
pub fn ceil_up_step(value: f64, step: f64) -> f64 {
    assert!(step > 0.0, "step must be positive");
    if !value.is_finite() {
        return value;
    }
    if value <= 0.0 {
        return 0.0;
    }
    let quotient = value / step;
    let rounded_q = quotient.round();
    if (quotient - rounded_q).abs() < 1e-9 && (rounded_q * step - value).abs() < step * 1e-9 {
        value
    } else {
        quotient.ceil() * step
    }
}

pub fn apply_takeoff_distance_rounding(
    result: &TakeoffResult,
    mode: TakeoffDistanceRounding,
) -> RoundedTakeoffDistances {
    let raw_gr_m = ft_to_m(result.ground_roll_ft);
    let raw_over_m = result.distance_over_obstacle_raw_m;
    let raw_gr_ft = result.ground_roll_ft;
    let raw_over_ft = result.distance_over_obstacle_raw_ft;
    match mode {
        TakeoffDistanceRounding::Raw => RoundedTakeoffDistances {
            ground_roll_m: raw_gr_m,
            ground_roll_ft: raw_gr_ft,
            over_obstacle_m: raw_over_m,
            over_obstacle_ft: raw_over_ft,
        },
        TakeoffDistanceRounding::Ceil10Meters => {
            let ground_roll_m = ceil_up_step(raw_gr_m, 10.0);
            let over_obstacle_m = ceil_up_step(raw_over_m, 10.0);
            RoundedTakeoffDistances {
                ground_roll_m,
                ground_roll_ft: m_to_ft(ground_roll_m),
                over_obstacle_m,
                over_obstacle_ft: m_to_ft(over_obstacle_m),
            }
        }
        TakeoffDistanceRounding::Ceil50Feet => {
            let ground_roll_ft = ceil_up_step(raw_gr_ft, 50.0);
            let over_obstacle_ft = ceil_up_step(raw_over_ft, 50.0);
            RoundedTakeoffDistances {
                ground_roll_ft,
                ground_roll_m: ft_to_m(ground_roll_ft),
                over_obstacle_ft,
                over_obstacle_m: ft_to_m(over_obstacle_ft),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{defaults, AfmLookupTag, ProvenanceBlock, TakeoffResult};

    fn dummy_result(gr_ft: f64, over_m: f64, over_ft: f64) -> TakeoffResult {
        TakeoffResult {
            provenance: ProvenanceBlock {
                afm_document_number: String::new(),
                afm_revision: String::new(),
                afm_date: String::new(),
                data_status: String::new(),
                citation: crate::types::Citation {
                    figure_or_table: String::new(),
                    page: None,
                    afm_revision: None,
                    notes: None,
                },
            },
            afm_lookup_tag: AfmLookupTag::Interpolated,
            pressure_altitude_ft: 0.0,
            ground_roll_ft: gr_ft,
            distance_over_obstacle_raw_m: over_m,
            distance_over_obstacle_raw_ft: over_ft,
            distance_over_obstacle_conservative_m: 0.0,
            distance_over_obstacle_conservative_ft: 0.0,
            distance_over_50ft_ft: over_ft,
            wind_correction_ft: 0.0,
            chart_off_afm: false,
            warnings: vec![],
        }
    }

    #[test]
    fn ceil_step_exact_multiples_unchanged() {
        assert_eq!(ceil_up_step(300.0, 10.0), 300.0);
        assert_eq!(ceil_up_step(1200.0, 50.0), 1200.0);
    }

    #[test]
    fn ceil_step_just_over_boundary() {
        assert_eq!(ceil_up_step(300.01, 10.0), 310.0);
        assert_eq!(ceil_up_step(1154.0, 50.0), 1200.0);
    }

    #[test]
    fn rounding_both_columns_10m() {
        let r = dummy_result(800.0, 351.0, 1154.0);
        let out = apply_takeoff_distance_rounding(&r, TakeoffDistanceRounding::Ceil10Meters);
        assert_eq!(out.ground_roll_m, 250.0); // 800 ft ≈ 243.84 → 250
        assert_eq!(out.over_obstacle_m, 360.0);
    }

    #[test]
    fn rounding_both_columns_50ft() {
        let r = dummy_result(743.0, 351.0, 1154.0);
        let out = apply_takeoff_distance_rounding(&r, TakeoffDistanceRounding::Ceil50Feet);
        assert_eq!(out.ground_roll_ft, 750.0);
        assert_eq!(out.over_obstacle_ft, 1200.0);
        assert!((out.over_obstacle_m - ft_to_m(1200.0)).abs() < 0.01);
    }

    #[test]
    fn raw_mode_preserves_solver_values() {
        let r = dummy_result(800.0, 351.0, 1154.0);
        let out = apply_takeoff_distance_rounding(&r, TakeoffDistanceRounding::Raw);
        assert_eq!(out.ground_roll_ft, 800.0);
        assert_eq!(out.over_obstacle_ft, 1154.0);
        let _ = defaults::WEIGHT_LB;
    }
}
