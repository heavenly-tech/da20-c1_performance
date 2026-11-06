use da20_core::types::FlightInputs;

/// Round values after chart/strip drag (pilot display precision).
pub fn round_after_chart_drag(i: &mut FlightInputs) {
    i.oat_c = (i.oat_c * 10.0).round() / 10.0;
    i.weight_lb = i.weight_lb.round();
    i.wind_speed_kt = i.wind_speed_kt.round();
    i.obstacle_height_m = (i.obstacle_height_m * 10.0).round() / 10.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::types::FlightInputs;

    #[test]
    fn rounds_oat_weight_wind() {
        let mut i = FlightInputs {
            oat_c: 21.04,
            weight_lb: 1598.6,
            wind_speed_kt: 7.8,
            ..Default::default()
        };
        round_after_chart_drag(&mut i);
        assert!((i.oat_c - 21.0).abs() < f64::EPSILON);
        assert_eq!(i.weight_lb, 1599.0);
        assert_eq!(i.wind_speed_kt, 8.0);
        i.obstacle_height_m = 5.04;
        round_after_chart_drag(&mut i);
        assert!((i.obstacle_height_m - 5.0).abs() < f64::EPSILON);
    }
}
