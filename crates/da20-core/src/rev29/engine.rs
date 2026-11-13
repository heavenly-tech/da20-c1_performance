use crate::chart_bounds::{self, limits, ChartOutOfRange, CHART_EDGE_TOL};
use crate::interpolation::{interpolate_1d, ExtrapolationPolicy, InterpError};
use crate::rev29::types::Rev29File;
use crate::types::CalculationWarning;

const M_TO_FT: f64 = 3.280839895;
const AXIS_TOL: f64 = CHART_EDGE_TOL;

fn interp_err_to_chart(label: &'static str, axis: &[f64], err: InterpError) -> ChartOutOfRange {
    let min = axis.first().copied().unwrap_or(0.0);
    let max = axis.last().copied().unwrap_or(min);
    match err {
        InterpError::BelowMin { min } => ChartOutOfRange { label, min, max },
        InterpError::AboveMax { max } => ChartOutOfRange { label, min, max },
        _ => ChartOutOfRange { label, min, max },
    }
}

pub fn linear_interp(
    axis: &[f64],
    values: &[f64],
    x: f64,
    axis_label: &'static str,
) -> Result<(f64, Vec<CalculationWarning>), ChartOutOfRange> {
    match interpolate_1d(axis, values, x, ExtrapolationPolicy::Strict) {
        Ok(pair) => Ok(pair),
        Err(e) => Err(interp_err_to_chart(axis_label, axis, e)),
    }
}

fn bracket_pa_index(axis: &[f64], pa_ft: f64) -> usize {
    let mut j = 0usize;
    while j + 1 < axis.len() && axis[j + 1] <= pa_ft {
        j += 1;
    }
    j.min(axis.len().saturating_sub(2))
}

/// Printed PA row match (no interpolation to the next row).
fn exact_pa_row_index(axis: &[f64], pa_ft: f64) -> Option<usize> {
    axis.iter().position(|&p| (p - pa_ft).abs() <= AXIS_TOL)
}

/// CL1 + CL2 — Fig 5.2 equivalent altitude (PA and OAT strict).
pub fn equivalent_altitude_ft(
    data: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
) -> Result<(f64, Vec<CalculationWarning>), ChartOutOfRange> {
    chart_bounds::require_in_range(
        "Fig 5.2 PA",
        limits::FIG52_PA_FT.0,
        limits::FIG52_PA_FT.1,
        pa_ft,
        AXIS_TOL,
    )?;
    chart_bounds::require_in_range(
        "Fig 5.2 OAT",
        limits::FIG52_OAT_C.0,
        limits::FIG52_OAT_C.1,
        oat_c,
        AXIS_TOL,
    )?;

    let fan_t = &data.fan_t.value;
    let fan_pa = &data.fan_pa.value;
    let fan_ea = &data.fan_ea.value;

    let ea_at_row = |row_idx: usize| -> Result<f64, ChartOutOfRange> {
        let row = &fan_ea[row_idx];
        let (v, _) = linear_interp(fan_t, row, oat_c, "Fig 5.2 OAT")?;
        Ok(v)
    };

    let n = fan_pa.len();
    if n == 0 {
        return Ok((0.0, vec![]));
    }
    if n == 1 {
        return Ok((ea_at_row(0)?, vec![]));
    }
    if let Some(j) = exact_pa_row_index(fan_pa, pa_ft) {
        return Ok((ea_at_row(j)?, vec![]));
    }
    let j = bracket_pa_index(fan_pa, pa_ft);
    let v0 = ea_at_row(j)?;
    let v1 = ea_at_row(j + 1)?;
    let ea = crate::interpolation::lerp(fan_pa[j], v0, fan_pa[j + 1], v1, pa_ft);
    Ok((ea, vec![]))
}

/// T1 + T2 — Fig 5.4 base ground roll.
pub fn takeoff_base_m(
    data: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
) -> Result<(f64, Vec<CalculationWarning>), ChartOutOfRange> {
    chart_bounds::require_in_range(
        "Fig 5.4 PA",
        limits::FIG54_PA_FT.0,
        limits::FIG54_PA_FT.1,
        pa_ft,
        AXIS_TOL,
    )?;
    chart_bounds::require_in_range(
        "Fig 5.4 OAT",
        limits::FIG54_OAT_C.0,
        limits::FIG54_OAT_C.1,
        oat_c,
        AXIS_TOL,
    )?;

    let to_t = &data.to_t.value;
    let to_pa = &data.to_pa.value;
    let to_base = &data.to_base_m.value;

    let base_at_row = |row_idx: usize| -> Result<f64, ChartOutOfRange> {
        let row = &to_base[row_idx];
        let xs: Vec<f64> = to_t
            .iter()
            .zip(row.iter())
            .filter_map(|(t, v)| v.map(|_| *t))
            .collect();
        let ys: Vec<f64> = row.iter().filter_map(|v| *v).collect();
        let (v, _) = linear_interp(&xs, &ys, oat_c, "Fig 5.4 OAT")?;
        Ok(v)
    };

    let n = to_pa.len();
    if n == 0 {
        return Ok((0.0, vec![]));
    }
    if n == 1 {
        return Ok((base_at_row(0)?, vec![]));
    }
    if let Some(j) = exact_pa_row_index(to_pa, pa_ft) {
        return Ok((base_at_row(j)?, vec![]));
    }
    let j = bracket_pa_index(to_pa, pa_ft);
    let v0 = base_at_row(j)?;
    let v1 = base_at_row(j + 1)?;
    let raw = crate::interpolation::lerp(to_pa[j], v0, to_pa[j + 1], v1, pa_ft);
    Ok((raw, vec![]))
}

/// T3 — weight factor panel.
pub fn weight_factor(
    data: &Rev29File,
    weight_kg: f64,
) -> Result<(f64, Vec<CalculationWarning>), ChartOutOfRange> {
    chart_bounds::require_in_range(
        "Fig 5.4 weight",
        limits::FIG54_WEIGHT_KG.0,
        limits::FIG54_WEIGHT_KG.1,
        weight_kg,
        AXIS_TOL,
    )?;
    linear_interp(
        &data.wt_kg.value,
        &data.wt_f.value,
        weight_kg,
        "Fig 5.4 weight",
    )
}

pub const TAILWIND_PANEL_MAX_KT: f64 = 5.0;

fn require_panel_entry_m(lines: &[f64], entry_m: f64) -> Result<(), ChartOutOfRange> {
    if lines.is_empty() {
        return Err(ChartOutOfRange {
            label: "Fig 5.4 panel distance",
            min: 0.0,
            max: 0.0,
        });
    }
    chart_bounds::require_in_range(
        "Fig 5.4 panel distance",
        lines[0],
        lines[lines.len() - 1],
        entry_m,
        AXIS_TOL,
    )
}

fn interp_panel_row(lines: &[f64], row: &[f64], entry_m: f64) -> Result<f64, ChartOutOfRange> {
    let (v, _) = linear_interp(lines, row, entry_m, "Fig 5.4 panel distance")?;
    Ok(v)
}

/// T4 — headwind panel (0–20 kt, entry distance on grid lines only).
pub fn wind_panel_hw_m(
    data: &Rev29File,
    base_distance_m: f64,
    headwind_kt: f64,
) -> Result<(f64, f64, Vec<CalculationWarning>), ChartOutOfRange> {
    wind_panel_kt_rows(
        &data.hw_lines.value,
        &data.hw_kt.value,
        &data.hw_m.value,
        base_distance_m,
        headwind_kt,
        limits::FIG54_HEADWIND_KT.1,
    )
}

/// T5 — tailwind panel (0–5 kt).
pub fn wind_panel_tw_m(
    data: &Rev29File,
    base_distance_m: f64,
    tailwind_kt: f64,
) -> Result<(f64, f64, Vec<CalculationWarning>), ChartOutOfRange> {
    wind_panel_kt_rows(
        &data.tw_lines.value,
        &data.tw_kt.value,
        &data.tw_m.value,
        base_distance_m,
        tailwind_kt,
        limits::FIG54_TAILWIND_KT.1,
    )
}

fn wind_panel_kt_rows(
    lines: &[f64],
    wind_kt: &[f64],
    grid: &[Vec<f64>],
    entry_distance_m: f64,
    wind_component_kt: f64,
    max_chart_kt: f64,
) -> Result<(f64, f64, Vec<CalculationWarning>), ChartOutOfRange> {
    require_panel_entry_m(lines, entry_distance_m)?;
    let at_wind: Vec<f64> = grid
        .iter()
        .map(|row| interp_panel_row(lines, row, entry_distance_m))
        .collect::<Result<_, _>>()?;
    let ground = wind_panel_delta(
        wind_kt,
        &at_wind,
        entry_distance_m,
        wind_component_kt,
        max_chart_kt,
    )?;
    Ok((ground, entry_distance_m, vec![]))
}

fn wind_panel_delta(
    wind_kt: &[f64],
    at_wind: &[f64],
    entry_distance_m: f64,
    wind_component_kt: f64,
    max_chart_kt: f64,
) -> Result<f64, ChartOutOfRange> {
    if wind_component_kt <= 0.0 {
        return Ok(entry_distance_m);
    }
    if wind_component_kt > max_chart_kt + AXIS_TOL {
        let label = if (max_chart_kt - limits::FIG54_TAILWIND_KT.1).abs() < 0.01 {
            "Fig 5.4 tailwind"
        } else {
            "Fig 5.4 headwind"
        };
        return Err(ChartOutOfRange {
            label,
            min: 0.0,
            max: max_chart_kt,
        });
    }
    let (v0, _) = linear_interp(wind_kt, at_wind, 0.0, label_if_hw(max_chart_kt))?;
    let (v_w, _) = linear_interp(
        wind_kt,
        at_wind,
        wind_component_kt,
        label_if_hw(max_chart_kt),
    )?;
    Ok(entry_distance_m + (v_w - v0))
}

fn label_if_hw(max_chart_kt: f64) -> &'static str {
    if (max_chart_kt - limits::FIG54_TAILWIND_KT.1).abs() < 0.01 {
        "Fig 5.4 tailwind"
    } else {
        "Fig 5.4 headwind"
    }
}

/// T6 — obstacle add-on (entry on obstacle panel axis only).
pub fn obstacle_add_m(
    data: &Rev29File,
    ground_after_wind_m: f64,
    obstacle_m: f64,
) -> Result<(f64, f64, Vec<CalculationWarning>), ChartOutOfRange> {
    if obstacle_m <= 0.0 {
        return Ok((ground_after_wind_m, ground_after_wind_m, vec![]));
    }
    let lines = &data.obs_lines.value;
    require_panel_entry_m(lines, ground_after_wind_m)?;
    let (slope, _) = linear_interp(
        lines,
        &data.obs_slope_m_per_m.value,
        ground_after_wind_m,
        "Fig 5.4 obstacle panel",
    )?;
    Ok((
        ground_after_wind_m + slope * obstacle_m,
        ground_after_wind_m,
        vec![],
    ))
}

pub struct TakeoffDistancesM {
    pub base_m: f64,
    pub after_weight_m: f64,
    pub wind_panel_entry_m: f64,
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
) -> Result<TakeoffDistancesM, ChartOutOfRange> {
    let (base, _) = takeoff_base_m(data, pa_ft, oat_c)?;
    let (wf, _) = weight_factor(data, weight_kg)?;
    let after_weight = base * wf;
    let (ground, wind_panel_entry_m) = if headwind_kt > 0.0 {
        let (g, e, _) = wind_panel_hw_m(data, after_weight, headwind_kt)?;
        (g, e)
    } else if tailwind_kt > 0.0 {
        let (g, e, _) = wind_panel_tw_m(data, after_weight, tailwind_kt)?;
        (g, e)
    } else {
        (after_weight, after_weight)
    };
    let (over_obstacle, obstacle_panel_entry_m, _) = obstacle_add_m(data, ground, obstacle_m)?;
    Ok(TakeoffDistancesM {
        base_m: base,
        after_weight_m: after_weight,
        wind_panel_entry_m,
        obstacle_panel_entry_m,
        ground_roll_m: ground,
        over_obstacle_m: over_obstacle,
        warnings: vec![CalculationWarning::code_only("UNITS")],
    })
}

pub fn rate_of_climb_fpm(
    data: &Rev29File,
    roc_key: &str,
    pa_ft: f64,
    oat_c: f64,
) -> Result<(f64, Vec<CalculationWarning>), ChartOutOfRange> {
    let [a, b] = *data.roc.value.get(roc_key).ok_or(ChartOutOfRange {
        label: "ROC figure",
        min: 0.0,
        max: 0.0,
    })?;
    let (ea, warnings) = equivalent_altitude_ft(data, pa_ft, oat_c)?;
    Ok((a + b * ea, warnings))
}

/// L1 — Table 4 landing distances (PA strict 0–7000 ft).
pub fn landing_lookup_ft(
    data: &Rev29File,
    pa_ft: f64,
) -> Result<(f64, f64, Vec<CalculationWarning>), ChartOutOfRange> {
    chart_bounds::require_in_range(
        "Table 4 PA",
        limits::TABLE4_PA_FT.0,
        limits::TABLE4_PA_FT.1,
        pa_ft,
        AXIS_TOL,
    )?;
    let (roll, _) = linear_interp(
        &data.land_alt.value,
        &data.land_roll.value,
        pa_ft,
        "Table 4 PA",
    )?;
    let (over_50, _) = linear_interp(
        &data.land_alt.value,
        &data.land_50ft.value,
        pa_ft,
        "Table 4 PA",
    )?;
    Ok((roll, over_50, vec![]))
}

pub fn lb_to_kg(lb: f64) -> f64 {
    lb * 0.45359237
}

pub fn kg_to_lb(kg: f64) -> f64 {
    kg / 0.45359237
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

pub fn equivalent_altitude_ft_value(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    equivalent_altitude_ft(data, pa_ft, oat_c)
        .map(|(v, _)| v)
        .unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_bounds::limits;

    fn prod() -> Rev29File {
        Rev29File::production()
    }

    #[test]
    fn rev29_worked_takeoff_chain_matches_reference_script() {
        let d = prod();
        let t = takeoff_distances_m(&d, 1000.0, 21.0, 725.0, 4.0, 0.0, 5.0).unwrap();
        let raw_ft = m_to_ft(t.over_obstacle_m);
        assert!((raw_ft - 1154.0).abs() / 1154.0 <= 0.015);
        assert_eq!((raw_ft / 50.0).ceil() as i64 * 50, 1200);
    }

    #[test]
    fn cl1_pa_below_fig52_out_of_chart() {
        let d = prod();
        let err = equivalent_altitude_ft(&d, -1.0, 15.0).unwrap_err();
        assert_eq!(err.label, "Fig 5.2 PA");
        assert_eq!(err.min, limits::FIG52_PA_FT.0);
    }

    #[test]
    fn cl1_pa_above_fig52_out_of_chart() {
        let d = prod();
        assert!(equivalent_altitude_ft(&d, 16001.0, 15.0).is_err());
    }

    #[test]
    fn cl2_oat_outside_fig52_out_of_chart() {
        let d = prod();
        assert!(equivalent_altitude_ft(&d, 5000.0, -21.0).is_err());
        assert!(equivalent_altitude_ft(&d, 5000.0, 60.1).is_err());
    }

    #[test]
    fn t1_pa_out_of_chart() {
        let d = prod();
        assert!(takeoff_base_m(&d, -1.0, 15.0).is_err());
        assert!(takeoff_base_m(&d, 10001.0, 15.0).is_err());
    }

    #[test]
    fn t2_oat_out_of_chart() {
        let d = prod();
        assert!(takeoff_base_m(&d, 1000.0, -21.0).is_err());
        assert!(takeoff_base_m(&d, 1000.0, 56.0).is_err());
    }

    #[test]
    fn takeoff_base_on_exact_pa_row_uses_row_only() {
        let d = prod();
        let (m40, _) = takeoff_base_m(&d, 6000.0, 40.0).unwrap();
        let (m55, _) = takeoff_base_m(&d, 6000.0, 55.0).unwrap();
        assert!((m40 - 857.0).abs() < 0.5, "6000 ft / 40 °C: {m40}");
        assert!((m55 - 970.0).abs() < 0.5, "6000 ft / 55 °C: {m55}");
        let (m8035, _) = takeoff_base_m(&d, 8000.0, 35.0).unwrap();
        assert!((m8035 - 981.0).abs() < 0.5, "8000 ft / 35 °C: {m8035}");
        assert!(takeoff_base_m(&d, 8000.0, 40.0).is_err());
        assert!(takeoff_base_m(&d, 7000.0, 40.0).is_err());
    }

    #[test]
    fn takeoff_base_between_rows_high_oat_stays_out_of_chart() {
        let d = prod();
        assert!(takeoff_base_m(&d, 5999.0, 55.0).is_ok());
        assert!(takeoff_base_m(&d, 5000.0, 55.0).is_ok());
        assert!(takeoff_base_m(&d, 7000.0, 40.0).is_err());
    }

    #[test]
    fn t3_weight_out_of_chart() {
        let d = prod();
        assert!(weight_factor(&d, 599.0).is_err());
        assert!(weight_factor(&d, 801.0).is_err());
    }

    #[test]
    fn t4_headwind_over_20_out_of_chart() {
        let d = prod();
        let base = takeoff_base_m(&d, 1000.0, 21.0).unwrap().0;
        let wf = weight_factor(&d, 725.0).unwrap().0;
        let entry = base * wf;
        assert!(wind_panel_hw_m(&d, entry, 20.1).is_err());
    }

    #[test]
    fn t5_tailwind_over_5_out_of_chart() {
        let d = prod();
        let entry = 400.0;
        assert!(wind_panel_tw_m(&d, entry, 5.01).is_err());
    }

    #[test]
    fn t6_obstacle_entry_below_panel_lines_out_of_chart() {
        let d = prod();
        let min_line = d.obs_lines.value[0];
        assert!(obstacle_add_m(&d, min_line - 1.0, 5.0).is_err());
    }

    #[test]
    fn t4_panel_entry_below_hw_lines_out_of_chart() {
        let d = prod();
        let min = d.hw_lines.value[0];
        assert!(wind_panel_hw_m(&d, min - 1.0, 4.0).is_err());
    }

    #[test]
    fn l1_landing_pa_out_of_chart() {
        let d = prod();
        assert!(landing_lookup_ft(&d, -1.0).is_err());
        assert!(landing_lookup_ft(&d, 7001.0).is_err());
    }

    #[test]
    fn wind_panel_anchors_at_entry_distance() {
        let d = prod();
        let entry = 400.0;
        let (d0, entry_eff, _) = wind_panel_hw_m(&d, entry, 0.0).unwrap();
        assert!((d0 - entry_eff).abs() < f64::EPSILON);
    }
}
