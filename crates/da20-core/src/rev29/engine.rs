use crate::interpolation::{interpolate_1d, ExtrapolationPolicy};
use crate::rev29::types::Rev29File;
use crate::types::CalculationWarning;

const M_TO_FT: f64 = 3.280839895;

const CLAMP_TOLERANCE: f64 = 0.2;

/// Clamp a chart distance query to the AFM axis; emit at most one conservative low/high warning.
pub fn clamp_chart_distance_m(
    axis: &[f64],
    query_m: f64,
    chart_label: &str,
) -> (f64, Vec<CalculationWarning>) {
    if axis.is_empty() {
        return (query_m, vec![]);
    }
    let min = axis[0];
    let max = axis[axis.len() - 1];
    if query_m < min - CLAMP_TOLERANCE {
        return (
            min,
            vec![CalculationWarning::with_args(
                "CHART_CLAMP_LOW",
                vec![
                    chart_label.to_string(),
                    format!("{:.0}", query_m.round()),
                    format!("{:.0}", min.round()),
                ],
            )],
        );
    }
    if query_m > max + CLAMP_TOLERANCE {
        return (
            max,
            vec![CalculationWarning::with_args(
                "CHART_CLAMP_HIGH",
                vec![
                    chart_label.to_string(),
                    format!("{:.0}", query_m.round()),
                    format!("{:.0}", max.round()),
                ],
            )],
        );
    }
    (query_m, vec![])
}

fn linear_interp_quiet(axis: &[f64], values: &[f64], x: f64) -> f64 {
    match interpolate_1d(axis, values, x, ExtrapolationPolicy::Strict) {
        Ok((v, _)) => v,
        Err(_) => {
            if axis.is_empty() {
                0.0
            } else if x <= axis[0] {
                values[0]
            } else {
                *values.last().unwrap_or(&values[0])
            }
        }
    }
}

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
    let raw = if n == 1 || pa_ft <= to_pa[0] {
        base_at_row(0)
    } else if pa_ft >= to_pa[n - 1] {
        base_at_row(n - 1)
    } else {
        let j = bracket_pa_index(to_pa, pa_ft);
        let v0 = base_at_row(j);
        let v1 = base_at_row(j + 1);
        crate::interpolation::lerp(to_pa[j], v0, to_pa[j + 1], v1, pa_ft)
    };
    (raw, warnings)
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
) -> (f64, f64, Vec<CalculationWarning>) {
    wind_panel_kt_rows(
        &data.hw_lines.value,
        &data.hw_kt.value,
        &data.hw_m.value,
        base_distance_m,
        headwind_kt,
        "Fig 5.4 headwind panel",
        data.hw_kt.value.last().copied().unwrap_or(20.0),
        "HEADWIND_CHART_LIMIT",
    )
}

pub fn wind_panel_tw_m(
    data: &Rev29File,
    base_distance_m: f64,
    tailwind_kt: f64,
) -> (f64, f64, Vec<CalculationWarning>) {
    wind_panel_kt_rows(
        &data.tw_lines.value,
        &data.tw_kt.value,
        &data.tw_m.value,
        base_distance_m,
        tailwind_kt,
        "Fig 5.4 tailwind panel",
        data.tw_kt.value.last().copied().unwrap_or(5.0),
        "TAILWIND_CHART_LIMIT",
    )
}

/// Headwind grid: each row is a wind speed; columns are distance guide lines.
#[allow(clippy::too_many_arguments)]
fn wind_panel_kt_rows(
    lines: &[f64],
    wind_kt: &[f64],
    grid: &[Vec<f64>],
    entry_distance_m: f64,
    wind_component_kt: f64,
    label: &str,
    max_chart_kt: f64,
    limit_code: &str,
) -> (f64, f64, Vec<CalculationWarning>) {
    let (entry_eff, clamp_w) = clamp_chart_distance_m(lines, entry_distance_m, label);
    let mut warnings = clamp_w;
    let at_wind: Vec<f64> = grid
        .iter()
        .map(|row| linear_interp_quiet(lines, row, entry_eff))
        .collect();
    let (ground, delta_w) = wind_panel_delta(
        wind_kt,
        &at_wind,
        entry_eff,
        wind_component_kt,
        label,
        max_chart_kt,
        limit_code,
        &mut warnings,
    );
    (ground, entry_eff, delta_w)
}

#[allow(clippy::too_many_arguments)]
fn wind_panel_delta(
    wind_kt: &[f64],
    at_wind: &[f64],
    entry_distance_m: f64,
    wind_component_kt: f64,
    label: &str,
    max_chart_kt: f64,
    limit_code: &str,
    warnings: &mut Vec<CalculationWarning>,
) -> (f64, Vec<CalculationWarning>) {
    let sample_at_kt = |kt: f64| -> (f64, Vec<CalculationWarning>) {
        let (v, w) = linear_interp(wind_kt, at_wind, kt, label);
        (v, w)
    };
    if wind_component_kt <= 0.0 {
        return (entry_distance_m, std::mem::take(warnings));
    }
    let kt_sample = if wind_component_kt > max_chart_kt {
        warnings.push(CalculationWarning::with_args(
            limit_code,
            vec![
                format!("{:.1}", max_chart_kt),
                format!("{:.1}", wind_component_kt),
            ],
        ));
        max_chart_kt
    } else {
        wind_component_kt
    };
    let (v0, w0) = sample_at_kt(0.0);
    warnings.extend(w0);
    let (v_w, w_w) = sample_at_kt(kt_sample);
    warnings.extend(w_w);
    (entry_distance_m + (v_w - v0), std::mem::take(warnings))
}

pub fn obstacle_add_m(
    data: &Rev29File,
    ground_after_wind_m: f64,
    obstacle_m: f64,
) -> (f64, f64, Vec<CalculationWarning>) {
    if obstacle_m <= 0.0 {
        return (ground_after_wind_m, ground_after_wind_m, vec![]);
    }
    let (ground_eff, clamp_w) = clamp_chart_distance_m(
        &data.obs_lines.value,
        ground_after_wind_m,
        "Fig 5.4 obstacle panel",
    );
    let slope = linear_interp_quiet(
        &data.obs_lines.value,
        &data.obs_slope_m_per_m.value,
        ground_eff,
    );
    (
        ground_after_wind_m + slope * obstacle_m,
        ground_eff,
        clamp_w,
    )
}

pub struct TakeoffDistancesM {
    pub base_m: f64,
    pub after_weight_m: f64,
    /// Distance row used on the wind panel (may be chart-clamped above `after_weight_m`).
    pub wind_panel_entry_m: f64,
    /// Ground-roll row used on the obstacle slope axis (may be chart-clamped).
    pub obstacle_panel_entry_m: f64,
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
    let (ground, wind_panel_entry_m, w2) = if headwind_kt > 0.0 {
        wind_panel_hw_m(data, after_weight, headwind_kt)
    } else if tailwind_kt > 0.0 {
        wind_panel_tw_m(data, after_weight, tailwind_kt)
    } else {
        (after_weight, after_weight, vec![])
    };
    warnings.extend(w2);
    let (over_obstacle, obstacle_panel_entry_m, w3) = obstacle_add_m(data, ground, obstacle_m);
    warnings.extend(w3);
    warnings = crate::warnings::dedupe_warnings(warnings);
    TakeoffDistancesM {
        base_m: base,
        after_weight_m: after_weight,
        wind_panel_entry_m,
        obstacle_panel_entry_m,
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

pub fn ft_to_m(ft: f64) -> f64 {
    ft / M_TO_FT
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
        let t = takeoff_distances_m(&d, 1000.0, 21.0, 725.0, 4.0, 0.0, 5.0);
        let raw_ft = m_to_ft(t.over_obstacle_m);
        assert!(
            (raw_ft - 1154.0).abs() / 1154.0 <= 0.015,
            "raw chart {raw_ft:.0} ft"
        );
        assert_eq!((raw_ft / 50.0).ceil() as i64 * 50, 1200);
    }

    #[test]
    fn wind_panel_anchors_at_entry_distance() {
        let d = Rev29File::production();
        let entry = 296.0;
        let (d0, entry_eff, _) = wind_panel_hw_m(&d, entry, 0.0);
        assert!((d0 - entry_eff).abs() < f64::EPSILON);
        let (d4, _, _) = wind_panel_hw_m(&d, entry, 4.0);
        assert!(d4.is_finite() && d4 > 0.0);
    }

    #[test]
    fn wind_panel_output_depends_only_on_entry_and_component() {
        let d = Rev29File::production();
        let entry = 520.0;
        let hw = 10.0;
        let once = wind_panel_hw_m(&d, entry, hw).0;
        let twice = wind_panel_hw_m(&d, entry, hw).0;
        assert_eq!(once, twice);
        assert!((once - entry - (wind_panel_hw_m(&d, entry, 4.0).0 - entry)).abs() > 0.0);
    }

    #[test]
    fn headwind_panel_clamp_warning_emitted_once_not_per_row() {
        let data = Rev29File::production();
        let min_line = data.hw_lines.value[0];
        let (_, _, single) = wind_panel_hw_m(&data, 206.0, 16.7);
        let hw_clamps: Vec<_> = single
            .iter()
            .filter(|w| w.code == "CHART_CLAMP_LOW")
            .collect();
        assert_eq!(hw_clamps.len(), 1);

        let cases = [
            (2500.0, 15.0, 680.0),
            (5000.0, 45.0, 600.0),
            (1000.0, 50.0, 600.0),
        ];
        for (pa, oat, kg) in cases {
            let dist = takeoff_distances_m(&data, pa, oat, kg, 16.7, 0.0, 10.0);
            if dist.after_weight_m >= min_line {
                continue;
            }
            let n = dist
                .warnings
                .iter()
                .filter(|w| w.code == "CHART_CLAMP_LOW")
                .count();
            assert_eq!(n, 1, "takeoff_distances_m should emit one clamp warning");
            return;
        }
    }

    #[test]
    fn chart_clamp_low_is_conservative_for_headwind_panel() {
        let data = Rev29File::production();
        let dist = takeoff_distances_m(&data, 2500.0, 15.0, 680.0, 16.7, 0.0, 10.0);
        if dist.wind_panel_entry_m <= dist.after_weight_m + 0.01 {
            return;
        }
        assert!(dist.wind_panel_entry_m >= dist.after_weight_m);
        assert!(dist.ground_roll_m >= dist.after_weight_m);
    }

    #[test]
    fn obstacle_add_depends_only_on_ground_entry_and_height() {
        let d = Rev29File::production();
        let ground = 410.0;
        let h = 5.0;
        assert_eq!(
            obstacle_add_m(&d, ground, h).0,
            obstacle_add_m(&d, ground, h).0
        );
        assert!((obstacle_add_m(&d, ground, 0.0).0 - ground).abs() < 1e-9);
    }
}
