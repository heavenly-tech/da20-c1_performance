use crate::interpolation::{interpolate_1d, ExtrapolationPolicy};
use crate::rev29::types::Rev29File;

const M_TO_FT: f64 = 3.280839895;

pub fn linear_interp(axis: &[f64], values: &[f64], x: f64) -> f64 {
    interpolate_1d(axis, values, x, ExtrapolationPolicy::AllowExtrapolate)
        .map(|(v, _)| v)
        .unwrap_or_else(|_| {
            if axis.is_empty() {
                return 0.0;
            }
            if x <= axis[0] {
                values[0]
            } else {
                *values.last().unwrap()
            }
        })
}

pub fn equivalent_altitude_ft(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    let fan_t = &data.fan_t.value;
    let fan_pa = &data.fan_pa.value;
    let fan_ea = &data.fan_ea.value;
    let row_values: Vec<f64> = fan_ea
        .iter()
        .map(|row| linear_interp(fan_t, row, oat_c))
        .collect();
    linear_interp(fan_pa, &row_values, pa_ft)
}

pub fn takeoff_base_m(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    let to_t = &data.to_t.value;
    let to_pa = &data.to_pa.value;
    let to_base = &data.to_base_m.value;
    let row_values: Vec<f64> = to_base
        .iter()
        .map(|row| {
            let xs: Vec<f64> = to_t
                .iter()
                .zip(row.iter())
                .filter_map(|(t, v)| v.map(|x| (*t, x)))
                .map(|(t, _)| t)
                .collect();
            let ys: Vec<f64> = row.iter().filter_map(|v| *v).collect();
            linear_interp(&xs, &ys, oat_c)
        })
        .collect();
    linear_interp(to_pa, &row_values, pa_ft)
}

pub fn weight_factor(data: &Rev29File, weight_kg: f64) -> f64 {
    linear_interp(&data.wt_kg.value, &data.wt_f.value, weight_kg)
}

pub fn wind_panel_hw_m(data: &Rev29File, base_distance_m: f64, headwind_kt: f64) -> f64 {
    wind_panel(&data.hw_lines.value, &data.hw_kt.value, &data.hw_m.value, base_distance_m, headwind_kt)
}

pub fn wind_panel_tw_m(data: &Rev29File, base_distance_m: f64, tailwind_kt: f64) -> f64 {
    wind_panel(&data.tw_lines.value, &data.tw_kt.value, &data.tw_m.value, base_distance_m, tailwind_kt)
}

fn wind_panel(
    lines: &[f64],
    wind_kt: &[f64],
    grid: &[Vec<f64>],
    base_distance_m: f64,
    wind_component_kt: f64,
) -> f64 {
    let at_wind: Vec<f64> = grid
        .iter()
        .map(|row| linear_interp(lines, row, base_distance_m))
        .collect();
    linear_interp(wind_kt, &at_wind, wind_component_kt)
}

pub fn obstacle_add_m(data: &Rev29File, ground_after_wind_m: f64, obstacle_m: f64) -> f64 {
    let slope = linear_interp(
        &data.obs_lines.value,
        &data.obs_slope_m_per_m.value,
        ground_after_wind_m,
    );
    ground_after_wind_m + slope * obstacle_m
}

pub struct TakeoffDistancesM {
    pub base_m: f64,
    pub after_weight_m: f64,
    pub ground_roll_m: f64,
    pub over_obstacle_m: f64,
}

pub fn takeoff_distances_m(
    data: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
    weight_kg: f64,
    headwind_kt: f64,
    tailwind_kt: f64,
    obstacle_m: f64,
) -> TakeoffDistancesM {
    let base = takeoff_base_m(data, pa_ft, oat_c);
    let after_weight = base * weight_factor(data, weight_kg);
    let mut ground = after_weight;
    if headwind_kt > 0.0 {
        ground = wind_panel_hw_m(data, after_weight, headwind_kt);
    } else if tailwind_kt > 0.0 {
        ground = wind_panel_tw_m(data, after_weight, tailwind_kt);
    }
    let over_obstacle = obstacle_add_m(data, ground, obstacle_m);
    TakeoffDistancesM {
        base_m: base,
        after_weight_m: after_weight,
        ground_roll_m: ground,
        over_obstacle_m: over_obstacle,
    }
}

pub fn rate_of_climb_fpm(data: &Rev29File, roc_key: &str, pa_ft: f64, oat_c: f64) -> Option<f64> {
    let [a, b] = *data.roc.value.get(roc_key)?;
    Some(a + b * equivalent_altitude_ft(data, pa_ft, oat_c))
}

pub fn landing_lookup_ft(data: &Rev29File, pa_ft: f64) -> (f64, f64) {
    let roll = linear_interp(&data.land_alt.value, &data.land_roll.value, pa_ft);
    let over_50 = linear_interp(&data.land_alt.value, &data.land_50ft.value, pa_ft);
    (roll, over_50)
}

pub fn landing_lookup_m(data: &Rev29File, pa_ft: f64) -> (f64, f64) {
    let (roll_ft, over_ft) = landing_lookup_ft(data, pa_ft);
    (roll_ft / M_TO_FT, over_ft / M_TO_FT)
}

pub fn lb_to_kg(lb: f64) -> f64 {
    lb * 0.45359237
}

pub fn m_to_ft(m: f64) -> f64 {
    m * M_TO_FT
}

pub fn fahrenheit_to_celsius(f: f64) -> f64 {
    (f - 32.0) / 1.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rev29_worked_takeoff_chain_matches_reference_script() {
        let d = Rev29File::production();
        let t = takeoff_distances_m(&d, 1000.0, fahrenheit_to_celsius(72.0), 725.0, 4.0, 0.0, 5.0);
        assert!((t.over_obstacle_m - 356.0).abs() < 0.5);
    }
}
