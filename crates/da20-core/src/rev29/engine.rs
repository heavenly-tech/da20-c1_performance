use crate::interpolation::{interpolate_1d, ExtrapolationPolicy};
use crate::rev29::types::Rev29File;
use crate::types::CalculationWarning;

const M_TO_FT: f64 = 3.280839895;

pub fn linear_interp(
    axis: &[f64],
    values: &[f64],
    x: f64,
    axis_label: &str,
) -> (f64, Vec<CalculationWarning>) {
    match interpolate_1d(axis, values, x, ExtrapolationPolicy::Clamp) {
        Ok((v, mut w)) => {
            for warn in &mut w {
                if !axis_label.is_empty() {
                    warn.args.insert(0, axis_label.to_string());
                }
            }
            (v, w)
        }
        Err(_) => {
            if axis.is_empty() {
                (0.0, vec![])
            } else if x <= axis[0] {
                (values[0], vec![])
            } else {
                (*values.last().unwrap_or(&values[0]), vec![])
            }
        }
    }
}

pub fn equivalent_altitude_ft(
    data: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
) -> (f64, Vec<CalculationWarning>) {
    let fan_t = &data.fan_t.value;
    let fan_pa = &data.fan_pa.value;
    let fan_ea = &data.fan_ea.value;
    let mut warnings = Vec::new();

    let mut ea_at_row = |row_idx: usize| -> f64 {
        let row = &fan_ea[row_idx];
        let (v, w) = linear_interp(fan_t, row, oat_c, "Fig 5.2 OAT");
        warnings.extend(w);
        v
    };

    let n = fan_pa.len();
    if n == 0 {
        return (0.0, warnings);
    }
    if n == 1 || pa_ft <= fan_pa[0] {
        return (ea_at_row(0), warnings);
    }
    if pa_ft >= fan_pa[n - 1] {
        return (ea_at_row(n - 1), warnings);
    }
    let j = bracket_pa_index(fan_pa, pa_ft);
    let v0 = ea_at_row(j);
    let v1 = ea_at_row(j + 1);
    let ea = crate::interpolation::lerp(fan_pa[j], v0, fan_pa[j + 1], v1, pa_ft);
    (ea, warnings)
}

fn bracket_pa_index(axis: &[f64], pa_ft: f64) -> usize {
    let mut j = 0usize;
    while j + 1 < axis.len() && axis[j + 1] <= pa_ft {
        j += 1;
    }
    j.min(axis.len().saturating_sub(2))
}

pub fn takeoff_base_m(data: &Rev29File, pa_ft: f64, oat_c: f64) -> (f64, Vec<CalculationWarning>) {
    let to_t = &data.to_t.value;
    let to_pa = &data.to_pa.value;
    let to_base = &data.to_base_m.value;
    let mut warnings = Vec::new();

    let mut base_at_row = |row_idx: usize| -> f64 {
        let row = &to_base[row_idx];
        let xs: Vec<f64> = to_t
            .iter()
            .zip(row.iter())
            .filter_map(|(t, v)| v.map(|_| *t))
            .collect();
        let ys: Vec<f64> = row.iter().filter_map(|v| *v).collect();
        let (v, w) = linear_interp(&xs, &ys, oat_c, "Fig 5.4 OAT");
        warnings.extend(w);
        v
    };

    let n = to_pa.len();
    if n == 0 {
        return (0.0, warnings);
    }
    if n == 1 || pa_ft <= to_pa[0] {
        return (base_at_row(0), warnings);
    }
    if pa_ft >= to_pa[n - 1] {
        return (base_at_row(n - 1), warnings);
    }
    let j = bracket_pa_index(to_pa, pa_ft);
    let v0 = base_at_row(j);
    let v1 = base_at_row(j + 1);
    let base = crate::interpolation::lerp(to_pa[j], v0, to_pa[j + 1], v1, pa_ft);
    (base, warnings)
}

pub fn weight_factor(data: &Rev29File, weight_kg: f64) -> (f64, Vec<CalculationWarning>) {
    linear_interp(
        &data.wt_kg.value,
        &data.wt_f.value,
        weight_kg,
        "Fig 5.4 weight",
    )
}

pub fn wind_panel_hw_m(
    data: &Rev29File,
    base_distance_m: f64,
    headwind_kt: f64,
) -> (f64, Vec<CalculationWarning>) {
    wind_panel(
        &data.hw_lines.value,
        &data.hw_kt.value,
        &data.hw_m.value,
        base_distance_m,
        headwind_kt,
        "Fig 5.4 headwind panel",
    )
}

pub fn wind_panel_tw_m(
    data: &Rev29File,
    base_distance_m: f64,
    tailwind_kt: f64,
) -> (f64, Vec<CalculationWarning>) {
    wind_panel(
        &data.tw_lines.value,
        &data.tw_kt.value,
        &data.tw_m.value,
        base_distance_m,
        tailwind_kt,
        "Fig 5.4 tailwind panel",
    )
}

fn wind_panel(
    lines: &[f64],
    wind_kt: &[f64],
    grid: &[Vec<f64>],
    base_distance_m: f64,
    wind_component_kt: f64,
    label: &str,
) -> (f64, Vec<CalculationWarning>) {
    let mut warnings = Vec::new();
    let at_wind: Vec<f64> = grid
        .iter()
        .map(|row| {
            let (v, w) = linear_interp(lines, row, base_distance_m, label);
            warnings.extend(w);
            v
        })
        .collect();
    let (v, w) = linear_interp(wind_kt, &at_wind, wind_component_kt, label);
    warnings.extend(w);
    (v, warnings)
}

pub fn obstacle_add_m(
    data: &Rev29File,
    ground_after_wind_m: f64,
    obstacle_m: f64,
) -> (f64, Vec<CalculationWarning>) {
    let (slope, w) = linear_interp(
        &data.obs_lines.value,
        &data.obs_slope_m_per_m.value,
        ground_after_wind_m,
        "Fig 5.4 obstacle slope",
    );
    let warnings = w;
    (ground_after_wind_m + slope * obstacle_m, warnings)
}

pub struct TakeoffDistancesM {
    pub base_m: f64,
    pub after_weight_m: f64,
    pub ground_roll_m: f64,
    pub over_obstacle_m: f64,
    pub warnings: Vec<CalculationWarning>,
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
    let (base, mut warnings) = takeoff_base_m(data, pa_ft, oat_c);
    let (wf, w1) = weight_factor(data, weight_kg);
    warnings.extend(w1);
    let after_weight = base * wf;
    let (ground, w2) = if headwind_kt > 0.0 {
        wind_panel_hw_m(data, after_weight, headwind_kt)
    } else if tailwind_kt > 0.0 {
        wind_panel_tw_m(data, after_weight, tailwind_kt)
    } else {
        (after_weight, vec![])
    };
    warnings.extend(w2);
    let (over_obstacle, w3) = obstacle_add_m(data, ground, obstacle_m);
    warnings.extend(w3);
    TakeoffDistancesM {
        base_m: base,
        after_weight_m: after_weight,
        ground_roll_m: ground,
        over_obstacle_m: over_obstacle,
        warnings,
    }
}

pub fn rate_of_climb_fpm(
    data: &Rev29File,
    roc_key: &str,
    pa_ft: f64,
    oat_c: f64,
) -> Option<(f64, Vec<CalculationWarning>)> {
    let [a, b] = *data.roc.value.get(roc_key)?;
    let (ea, warnings) = equivalent_altitude_ft(data, pa_ft, oat_c);
    Some((a + b * ea, warnings))
}

pub fn landing_lookup_ft(data: &Rev29File, pa_ft: f64) -> (f64, f64, Vec<CalculationWarning>) {
    let (roll, w1) = linear_interp(
        &data.land_alt.value,
        &data.land_roll.value,
        pa_ft,
        "Table 4 PA",
    );
    let (over_50, w2) = linear_interp(
        &data.land_alt.value,
        &data.land_50ft.value,
        pa_ft,
        "Table 4 PA",
    );
    let mut warnings = w1;
    warnings.extend(w2);
    (roll, over_50, warnings)
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

// Back-compat helpers for tests expecting f64 only
pub fn equivalent_altitude_ft_value(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    equivalent_altitude_ft(data, pa_ft, oat_c).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rev29_worked_takeoff_chain_matches_reference_script() {
        let d = Rev29File::production();
        let t = takeoff_distances_m(
            &d,
            1000.0,
            fahrenheit_to_celsius(72.0),
            725.0,
            4.0,
            0.0,
            5.0,
        );
        assert!((t.over_obstacle_m - 356.0).abs() < 0.5);
    }
}
