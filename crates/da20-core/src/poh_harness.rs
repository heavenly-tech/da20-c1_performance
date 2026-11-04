//! POH worked-example regression harness (AFM Rev 29 reference checks).

use crate::data::PerformanceData;
use crate::rev29::engine::{
    fahrenheit_to_celsius, rate_of_climb_fpm, takeoff_distances_m,
};
use crate::types::FlightInputs;
use crate::wind::wind_components;

#[derive(Debug, Clone)]
pub struct HarnessResult {
    pub example_id: String,
    pub passed: bool,
    pub message: String,
}

pub fn run_rev29_reference_checks(data: &PerformanceData) -> Vec<HarnessResult> {
    let mut out = Vec::new();
    let inner = data.inner();

    let d = takeoff_distances_m(
        inner,
        1000.0,
        fahrenheit_to_celsius(72.0),
        725.0,
        4.0,
        0.0,
        5.0,
    );
    let err_pct = (d.over_obstacle_m - 365.76) / 365.76 * 100.0;
    out.push(HarnessResult {
        example_id: "takeoff_5_4_rev29".into(),
        passed: err_pct > -5.0 && err_pct < 5.0,
        message: format!(
            "over-obstacle {:.1} m vs 365.76 m ({err_pct:.1}%, known ~2.7% short bias)",
            d.over_obstacle_m
        ),
    });

    let t65 = fahrenheit_to_celsius(65.0);
    for (key, expected, label) in [
        ("cruise_5_5", 800.0, "Fig 5.5"),
        ("takeoff_5_6", 744.0, "Fig 5.6"),
        ("balked_5_8", 374.0, "Fig 5.8"),
    ] {
        let roc = rate_of_climb_fpm(inner, key, 2000.0, if key == "balked_5_8" {
            fahrenheit_to_celsius(70.0)
        } else {
            t65
        })
        .unwrap_or(0.0);
        let err_pct = (roc - expected).abs() / expected * 100.0;
        out.push(HarnessResult {
            example_id: format!("roc_{key}"),
            passed: err_pct <= 1.0,
            message: format!("{label} ROC {roc:.1} fpm vs {expected} ({err_pct:.2}%)"),
        });
    }

    let c = wind_components(30.0, 11.0, 0.0);
    out.push(HarnessResult {
        example_id: "wind_5_3_hw".into(),
        passed: (c.headwind_kt - 9.5).abs() <= 0.1,
        message: format!("HW {:.2} kt (ref 9.5)", c.headwind_kt),
    });
    out.push(HarnessResult {
        example_id: "wind_5_3_xw".into(),
        passed: (c.crosswind_kt - 5.5).abs() <= 0.1,
        message: format!("XW {:.2} kt (ref 5.5)", c.crosswind_kt),
    });

    let land_50 = crate::landing::table_landing_50ft_ft(data, 0.0);
    out.push(HarnessResult {
        example_id: "landing_table4_sl".into(),
        passed: (land_50 - 1360.0).abs() < f64::EPSILON,
        message: format!("Table 4 sea-level 50 ft distance {land_50:.0} ft"),
    });

    out
}

pub fn run_all(_data: &PerformanceData, _inputs: &FlightInputs) -> Vec<HarnessResult> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rev29_reference_checks_pass() {
        let data = PerformanceData::production();
        for r in run_rev29_reference_checks(&data) {
            assert!(r.passed, "{}: {}", r.example_id, r.message);
        }
    }
}
