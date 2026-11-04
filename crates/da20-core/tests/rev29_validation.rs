//! Ports `validate_rev29.py` reference checks into Rust tests.

use da20_core::data::PerformanceData;
use da20_core::rev29::engine::{fahrenheit_to_celsius, rate_of_climb_fpm, takeoff_distances_m};
use da20_core::wind::wind_components;

#[test]
fn validation_rev29_takeoff_and_roc_and_wind() {
    let data = PerformanceData::production();
    let inner = data.inner();

    let d = takeoff_distances_m(inner, 1000.0, 21.0, 725.0, 4.0, 0.0, 5.0);
    let err_pct = (d.over_obstacle_m - 365.76) / 365.76 * 100.0;
    assert!(
        (d.over_obstacle_m - 360.9).abs() < 1.0,
        "over-obstacle {:.2} m",
        d.over_obstacle_m
    );
    assert!(
        err_pct > -2.0 && err_pct < 0.0,
        "AFM Rev 29 worked example err {err_pct:.2}%"
    );

    let t65 = fahrenheit_to_celsius(65.0);
    let roc_55 = rate_of_climb_fpm(inner, "cruise_5_5", 2000.0, t65)
        .unwrap()
        .0;
    assert!((roc_55 - 800.0).abs() / 800.0 <= 0.01);
    let roc_56 = rate_of_climb_fpm(inner, "takeoff_5_6", 2000.0, t65)
        .unwrap()
        .0;
    assert!((roc_56 - 744.0).abs() / 744.0 <= 0.01);
    let roc_58 = rate_of_climb_fpm(inner, "balked_5_8", 2000.0, fahrenheit_to_celsius(70.0))
        .unwrap()
        .0;
    assert!((roc_58 - 374.0).abs() / 374.0 <= 0.01);

    let hw = 11.0 * (30.0f64.to_radians().cos());
    let xw = 11.0 * (30.0f64.to_radians().sin());
    assert!((hw - 9.5).abs() <= 0.1);
    assert!((xw - 5.5).abs() <= 0.1);

    let c = wind_components(30.0, 11.0, 0.0);
    assert!((c.headwind_kt - 9.5).abs() <= 0.1);
    assert!((c.crosswind_kt - 5.5).abs() <= 0.1);
}
