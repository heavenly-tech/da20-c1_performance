//! Pure helpers for MT Fig 2/3 cruise chart geometry, drag mapping, and marker guides.

use da20_core::altitude::isa_troposphere_density_altitude_ft;
use da20_core::mt175r::charts::{
    fig2_family_chart_points, fig2_pct_bhp_chart_points, fig2_pct_bhp_da_bounds,
    fig3_family_chart_points, fig3_full_throttle_chart_points, fig3_pct_bhp_chart_points,
    fig3_pct_bhp_da_bounds, pct_bhp_from_fig2_drag_rpm, PRINTED_PCT_CURVES,
};
use da20_core::mt175r::Mt175rData;
use da20_core::rev29::charts::{map_from_svg, map_to_svg};
use da20_core::types::{CruiseSolveMode, FlightInputs};

use crate::chart_guides::LabelAnchor;

/// Left gutter: Y tick labels + marker readout.
pub const MT_MARGIN_L: f64 = 56.0;
pub const MT_MARGIN_B: f64 = 64.0;
pub const MT_MARGIN_T: f64 = 14.0;
/// Right gutter: %BHP family labels.
pub const MT_MARGIN_R: f64 = 52.0;
pub const MT_W: f64 = 320.0;
pub const MT_H: f64 = 200.0;

pub const FIG2_DA_MIN: f64 = 0.0;
pub const FIG2_DA_MAX: f64 = 12_000.0;
/// AFMS Fig 2 printed axis (`domains.fig2_power.rpm` in mt175r_cruise_data.json).
pub const FIG2_RPM_MIN: f64 = 1900.0;
pub const FIG2_RPM_MAX: f64 = 2800.0;
pub const FIG3_TAS_MIN: f64 = 90.0;
pub const FIG3_TAS_MAX: f64 = 150.0;

/// Rough single-line label box for viewBox fit tests (9px chart-guide-label).
#[cfg_attr(not(test), allow(dead_code))]
pub const LABEL_FONT_PX: f64 = 9.0;
#[cfg_attr(not(test), allow(dead_code))]
pub const LABEL_CHAR_PX: f64 = 5.4;
/// Minimum vertical separation between curve-end label bounding boxes.
pub const END_LABEL_MIN_GAP_PX: f64 = 11.0;

#[cfg_attr(not(test), allow(dead_code))]
pub fn estimate_text_width(text: &str) -> f64 {
    text.chars().count() as f64 * LABEL_CHAR_PX
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn label_bbox_start(x: f64, y: f64, text: &str) -> (f64, f64, f64, f64) {
    let w = estimate_text_width(text);
    let h = LABEL_FONT_PX;
    (x, y - h * 0.5, x + w, y + h * 0.5)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn label_bboxes_overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 - 0.5 && a.2 > b.0 + 0.5 && a.1 < b.3 - 0.5 && a.3 > b.1 + 0.5
}

pub fn stagger_end_label_ys(base_ys: &[f64]) -> Vec<f64> {
    let n = base_ys.len();
    if n == 0 {
        return vec![];
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| base_ys[a].total_cmp(&base_ys[b]));
    let mut out = base_ys.to_vec();
    let mut last = f64::NEG_INFINITY;
    for idx in order {
        let mut y = base_ys[idx];
        if y < last + END_LABEL_MIN_GAP_PX {
            y = last + END_LABEL_MIN_GAP_PX;
        }
        out[idx] = y;
        last = y;
    }
    out
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn label_box_fits_viewbox(x: f64, y: f64, text: &str, anchor: LabelAnchor) -> bool {
    let w = estimate_text_width(text);
    let h = LABEL_FONT_PX;
    let (x0, x1) = match anchor {
        LabelAnchor::Start => (x, x + w),
        LabelAnchor::Middle => (x - w * 0.5, x + w * 0.5),
        LabelAnchor::End => (x - w, x),
    };
    let vb_w = MT_W + MT_MARGIN_L + MT_MARGIN_R;
    let vb_h = MT_H + MT_MARGIN_B + MT_MARGIN_T;
    x0 >= 0.0 && x1 <= vb_w && y - h * 0.5 >= 0.0 && y + h * 0.5 <= vb_h
}

/// Drop off-chart samples so polylines are not flattened by axis clamping.
pub fn filter_plot_points(
    points: &[(f64, f64)],
    da_min: f64,
    da_max: f64,
    y_min: f64,
    y_max: f64,
) -> Vec<(f64, f64)> {
    points
        .iter()
        .copied()
        .filter(|(da, y)| (da_min..=da_max).contains(da) && (y_min..=y_max).contains(y))
        .collect()
}

pub fn map_mt_chart_xy(
    x: f64,
    y: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> (f64, f64) {
    map_to_svg(
        x,
        y,
        x_min,
        x_max,
        y_min,
        y_max,
        MT_MARGIN_L,
        MT_MARGIN_T,
        MT_W,
        MT_H,
    )
}

pub fn map_mt_chart_from_svg_y(sy: f64, y_min: f64, y_max: f64) -> f64 {
    map_from_svg(
        MT_MARGIN_L,
        sy,
        0.0,
        1.0,
        y_min,
        y_max,
        MT_MARGIN_L,
        MT_MARGIN_T,
        MT_W,
        MT_H,
    )
    .1
}

pub fn mt_cruise_density_altitude_ft(inputs: &FlightInputs) -> f64 {
    isa_troposphere_density_altitude_ft(inputs.cruise_pressure_alt_ft, inputs.cruise_oat_c)
}

pub fn selected_pct_bhp(inputs: &FlightInputs, solved_pct: Option<f64>) -> f64 {
    match inputs.cruise_solve_mode {
        CruiseSolveMode::SetPercentBhp => inputs.cruise_percent_bhp,
        CruiseSolveMode::SetRpm => solved_pct.unwrap_or(inputs.cruise_percent_bhp),
    }
}

pub fn format_pct_bhp_label(pct: f64) -> String {
    if (pct - pct.round()).abs() < 0.05 {
        format!("{:.0}%", pct.round())
    } else {
        format!("{pct:.1}%")
    }
}

/// Apply Fig 2 vertical drag at chart Y → set %BHP mode and value (clamped to 45–75).
pub fn apply_mt_fig2_pct_drag(svg_y: f64, mt: &Mt175rData, inputs: &mut FlightInputs) -> bool {
    let da = mt_cruise_density_altitude_ft(inputs);
    let rpm = map_mt_chart_from_svg_y(svg_y, FIG2_RPM_MIN, FIG2_RPM_MAX);
    let Some(pct) = pct_bhp_from_fig2_drag_rpm(mt, da, rpm) else {
        return false;
    };
    inputs.cruise_solve_mode = CruiseSolveMode::SetPercentBhp;
    inputs.cruise_percent_bhp = (pct * 100.0).round() / 100.0;
    true
}

#[derive(Debug, Clone, PartialEq)]
pub struct MtChartMarkerGuides {
    pub cx: f64,
    pub cy: f64,
    pub x_axis_y: f64,
    pub y_axis_x: f64,
    pub x_label: String,
    pub y_label: String,
    pub x_label_x: f64,
    pub x_label_y: f64,
    pub y_label_x: f64,
    pub y_label_y: f64,
    pub on_chart: bool,
    pub draw_guides: bool,
    pub notice: Option<String>,
}

pub fn mt_chart_da_on_fig2_axes(da_ft: f64) -> bool {
    (FIG2_DA_MIN - 0.5..=FIG2_DA_MAX + 0.5).contains(&da_ft)
}

pub fn plot_bottom_y() -> f64 {
    MT_MARGIN_T + MT_H
}

pub fn plot_left_x() -> f64 {
    MT_MARGIN_L
}

pub fn plot_right_x() -> f64 {
    MT_MARGIN_L + MT_W
}

pub fn plot_top_y() -> f64 {
    MT_MARGIN_T
}

pub fn fig2_marker_guides(
    locale: crate::i18n::Locale,
    da_ft: f64,
    rpm: Option<f64>,
    out_of_chart: bool,
) -> MtChartMarkerGuides {
    let axes_ok = mt_chart_da_on_fig2_axes(da_ft);
    let on_chart = axes_ok && !out_of_chart && rpm.is_some() && rpm.unwrap().is_finite();
    let draw_guides = on_chart;
    let notice = if !on_chart {
        Some(crate::i18n::format_mt_cruise_chart_ooc_notice(
            locale, da_ft,
        ))
    } else {
        None
    };
    let da_clamped = da_ft.clamp(FIG2_DA_MIN, FIG2_DA_MAX);
    let rpm_v = if on_chart { rpm.unwrap() } else { f64::NAN };
    let (cx, cy) = if on_chart {
        map_mt_chart_xy(
            da_clamped,
            rpm_v,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG2_RPM_MIN,
            FIG2_RPM_MAX,
        )
    } else {
        (MT_MARGIN_L + MT_W * 0.5, MT_MARGIN_T + MT_H * 0.5)
    };
    let x_label = format!("DA {:.0} ft", da_ft);
    let y_label = if on_chart {
        format!("{:.0} RPM", rpm_v)
    } else {
        match locale {
            crate::i18n::Locale::Es => "Fuera de carta".into(),
            crate::i18n::Locale::En => "Off chart".into(),
        }
    };
    let axis_y = plot_bottom_y();
    let axis_x = plot_left_x();
    MtChartMarkerGuides {
        cx,
        cy,
        x_axis_y: axis_y,
        y_axis_x: axis_x,
        x_label: x_label.clone(),
        y_label: y_label.clone(),
        x_label_x: cx,
        x_label_y: axis_y - 10.0,
        y_label_x: axis_x + 6.0,
        y_label_y: cy,
        on_chart,
        draw_guides,
        notice,
    }
}

pub fn fig3_marker_guides(
    locale: crate::i18n::Locale,
    da_ft: f64,
    tas_kt: Option<f64>,
    pct_bhp: f64,
    out_of_chart: bool,
) -> MtChartMarkerGuides {
    let axes_ok = mt_chart_da_on_fig2_axes(da_ft);
    let on_chart = axes_ok && !out_of_chart && tas_kt.is_some() && tas_kt.unwrap().is_finite();
    let draw_guides = on_chart;
    let notice = if !on_chart {
        Some(crate::i18n::format_mt_cruise_chart_ooc_notice(
            locale, da_ft,
        ))
    } else {
        None
    };
    let da_clamped = da_ft.clamp(FIG2_DA_MIN, FIG2_DA_MAX);
    let tas_v = if on_chart { tas_kt.unwrap() } else { f64::NAN };
    let (cx, cy) = if on_chart {
        map_mt_chart_xy(
            da_clamped,
            tas_v,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG3_TAS_MIN,
            FIG3_TAS_MAX,
        )
    } else {
        (MT_MARGIN_L + MT_W * 0.5, MT_MARGIN_T + MT_H * 0.5)
    };
    let x_label = format!("DA {:.0} ft", da_ft);
    let pct_label = format_pct_bhp_label(pct_bhp);
    let value_lbl = if on_chart {
        format!("{:.0} kt", tas_v)
    } else {
        match locale {
            crate::i18n::Locale::Es => "Fuera de carta".into(),
            crate::i18n::Locale::En => "Off chart".into(),
        }
    };
    let axis_y = plot_bottom_y();
    let axis_x = plot_left_x();
    let y_label = format!("{value_lbl} · {pct_label}");
    MtChartMarkerGuides {
        cx,
        cy,
        x_axis_y: axis_y,
        y_axis_x: axis_x,
        x_label: x_label.clone(),
        y_label: y_label.clone(),
        x_label_x: cx,
        x_label_y: axis_y - 10.0,
        y_label_x: axis_x + 6.0,
        y_label_y: cy,
        on_chart,
        draw_guides,
        notice,
    }
}

/// Serialize guide geometry for unit tests (no NaN in finite paths).
#[cfg_attr(not(test), allow(dead_code))]
pub fn marker_guides_svg_snippet(g: &MtChartMarkerGuides) -> String {
    let mut lines = Vec::new();
    if g.draw_guides {
        lines.push(format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            g.cx, g.cy, g.cx, g.x_axis_y
        ));
        lines.push(format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            g.cx, g.cy, g.y_axis_x, g.cy
        ));
        lines.push(format!("<text>{}</text>", g.x_label));
        lines.push(format!("<text>{}</text>", g.y_label));
        if g.on_chart {
            lines.push(format!("<circle cx=\"{}\" cy=\"{}\" r=\"5\"/>", g.cx, g.cy));
        }
    }
    if let Some(n) = &g.notice {
        lines.push(format!("<text class=\"notice\">{}</text>", n));
    }
    lines.join("")
}

pub fn fig2_family_polylines(mt: &Mt175rData) -> Vec<(i32, Vec<(f64, f64)>)> {
    PRINTED_PCT_CURVES
        .iter()
        .map(|&p| (p, fig2_family_chart_points(mt, p)))
        .collect()
}

pub fn fig2_selected_polyline(mt: &Mt175rData, pct: f64) -> Vec<(f64, f64)> {
    fig2_pct_bhp_chart_points(mt, pct)
}

pub fn fig3_family_polylines(mt: &Mt175rData) -> Vec<(i32, Vec<(f64, f64)>)> {
    PRINTED_PCT_CURVES
        .iter()
        .map(|&p| (p, fig3_family_chart_points(mt, p)))
        .collect()
}

pub fn fig3_selected_polyline(mt: &Mt175rData, pct: f64) -> Vec<(f64, f64)> {
    fig3_pct_bhp_chart_points(mt, pct)
}

fn end_label_x(da: f64, y: f64, y_min: f64, y_max: f64) -> f64 {
    let (sx, _) = map_mt_chart_xy(da, y, FIG2_DA_MIN, FIG2_DA_MAX, y_min, y_max);
    (sx + 10.0).min(plot_right_x() + 8.0)
}

/// Curve-end labels for Fig 2 (`text`, `x`, `y`, `highlight`).
pub fn fig2_curve_end_label_positions(
    mt: &Mt175rData,
    selected_pct: f64,
    skip_matching_printed_label: bool,
) -> Vec<(String, f64, f64, bool)> {
    let mut specs: Vec<(String, f64, f64, bool)> = Vec::new();
    for p in PRINTED_PCT_CURVES {
        if skip_matching_printed_label && (selected_pct - p as f64).abs() < 0.01 {
            continue;
        }
        let pts = fig2_family_chart_points(mt, p);
        let Some((da, rpm)) = pts.last().copied() else {
            continue;
        };
        let (sx, sy) = map_mt_chart_xy(
            da,
            rpm,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG2_RPM_MIN,
            FIG2_RPM_MAX,
        );
        let x = (sx + 10.0).min(plot_right_x() + 8.0);
        specs.push((format!("{p}%"), x, sy, false));
    }
    let selected_pts = fig2_pct_bhp_chart_points(mt, selected_pct);
    if !selected_pts.is_empty() {
        let da_max = fig2_pct_bhp_da_bounds(mt, selected_pct)
            .map(|(_, d1)| d1)
            .unwrap_or_else(|| selected_pts.last().unwrap().0);
        let rpm = selected_pts
            .iter()
            .find(|(d, _)| (*d - da_max).abs() < 1.0)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| selected_pts.last().unwrap().1);
        let (sx, sy) = map_mt_chart_xy(
            da_max,
            rpm,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG2_RPM_MIN,
            FIG2_RPM_MAX,
        );
        let x = (sx + 10.0).min(plot_right_x() + 8.0);
        let y = (sy - 12.0).max(plot_top_y() + 8.0);
        let on_printed = PRINTED_PCT_CURVES
            .iter()
            .any(|p| (selected_pct - *p as f64).abs() < 0.01);
        let show_selected = skip_matching_printed_label || !on_printed || selected_pts.len() > 1;
        if show_selected {
            specs.push((format_pct_bhp_label(selected_pct), x, y, true));
        }
    }
    let svg_ys: Vec<f64> = specs.iter().map(|(_, _, y, _)| *y).collect();
    let adj = stagger_end_label_ys(&svg_ys);
    specs
        .into_iter()
        .zip(adj)
        .map(|((text, x, _, hi), y)| (text, x, y, hi))
        .collect()
}

/// Curve-end labels for Fig 3 (`text`, `x`, `y`, `highlight`).
pub fn fig3_curve_end_label_positions(
    mt: &Mt175rData,
    selected_pct: f64,
    skip_matching_printed_label: bool,
) -> Vec<(String, f64, f64, bool)> {
    let mut specs: Vec<(String, f64, f64, bool)> = Vec::new();
    for p in PRINTED_PCT_CURVES {
        if skip_matching_printed_label && (selected_pct - p as f64).abs() < 0.01 {
            continue;
        }
        let pts = fig3_family_chart_points(mt, p);
        let Some((da, tas)) = pts.last().copied() else {
            continue;
        };
        let x = end_label_x(da, tas, FIG3_TAS_MIN, FIG3_TAS_MAX);
        let sy = map_mt_chart_xy(
            da,
            tas,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG3_TAS_MIN,
            FIG3_TAS_MAX,
        )
        .1;
        specs.push((format!("{p}%"), x, sy, false));
    }
    let ft_pts = fig3_full_throttle_chart_points(mt);
    if let Some((da, tas)) = ft_pts.last().copied() {
        let x = end_label_x(da, tas, FIG3_TAS_MIN, FIG3_TAS_MAX);
        let sy = map_mt_chart_xy(
            da,
            tas,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG3_TAS_MIN,
            FIG3_TAS_MAX,
        )
        .1;
        specs.push(("Full Throttle".into(), x, sy, false));
    }
    let selected_pts = fig3_pct_bhp_chart_points(mt, selected_pct);
    if !selected_pts.is_empty() {
        let da_max = fig3_pct_bhp_da_bounds(mt, selected_pct)
            .map(|(_, d1)| d1)
            .unwrap_or_else(|| selected_pts.last().unwrap().0);
        let tas = selected_pts
            .iter()
            .find(|(d, _)| (*d - da_max).abs() < 1.0)
            .map(|(_, t)| *t)
            .unwrap_or_else(|| selected_pts.last().unwrap().1);
        let x = end_label_x(da_max, tas, FIG3_TAS_MIN, FIG3_TAS_MAX);
        let sy = map_mt_chart_xy(
            da_max,
            tas,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG3_TAS_MIN,
            FIG3_TAS_MAX,
        )
        .1;
        let y = (sy - 12.0).max(plot_top_y() + 8.0);
        let on_printed = PRINTED_PCT_CURVES
            .iter()
            .any(|p| (selected_pct - *p as f64).abs() < 0.01);
        let show_selected = skip_matching_printed_label || !on_printed || selected_pts.len() > 1;
        if show_selected {
            specs.push((format_pct_bhp_label(selected_pct), x, y, true));
        }
    }
    let svg_ys: Vec<f64> = specs.iter().map(|(_, _, y, _)| *y).collect();
    let adj = stagger_end_label_ys(&svg_ys);
    specs
        .into_iter()
        .zip(adj)
        .map(|((text, x, _, hi), y)| (text, x, y, hi))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;
    use da20_core::altitude::isa_troposphere_density_altitude_ft;
    use da20_core::cruise::isa_temp_c_at_pa;
    use da20_core::mt175r::charts::{lookup_fig2_rpm_at_pct, MT_PCT_BHP_MAX, MT_PCT_BHP_MIN};
    use da20_core::mt175r::cruise::compute_mt175r_cruise;
    use da20_core::types::PropellerKind;

    const DA6000_PA: f64 = 6000.0;

    fn mt_inputs(pct: f64) -> FlightInputs {
        let pa = DA6000_PA;
        FlightInputs {
            propeller: PropellerKind::Mt175R,
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: isa_temp_c_at_pa(pa),
            cruise_percent_bhp: pct,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..Default::default()
        }
    }

    #[test]
    fn fig2_drag_y_pct_round_trip_including_clamp() {
        let mt = Mt175rData::production();
        let inputs_probe = mt_inputs(65.0);
        let da = isa_troposphere_density_altitude_ft(
            inputs_probe.cruise_pressure_alt_ft,
            inputs_probe.cruise_oat_c,
        );
        for pct in [45.0, 55.0, 65.0, 75.0, 60.0] {
            let rpm = lookup_fig2_rpm_at_pct(&mt, da, pct).unwrap().rpm;
            let sy = map_mt_chart_xy(
                da,
                rpm,
                FIG2_DA_MIN,
                FIG2_DA_MAX,
                FIG2_RPM_MIN,
                FIG2_RPM_MAX,
            )
            .1;
            let mut inputs = mt_inputs(pct);
            assert!(apply_mt_fig2_pct_drag(sy, &mt, &mut inputs));
            assert!(
                (inputs.cruise_percent_bhp - pct).abs() < 0.11,
                "pct={pct} got={}",
                inputs.cruise_percent_bhp
            );
            let sy_back = map_mt_chart_xy(
                da,
                lookup_fig2_rpm_at_pct(&mt, da, inputs.cruise_percent_bhp)
                    .unwrap()
                    .rpm,
                FIG2_DA_MIN,
                FIG2_DA_MAX,
                FIG2_RPM_MIN,
                FIG2_RPM_MAX,
            )
            .1;
            assert!((sy - sy_back).abs() < 0.5, "y round-trip pct={pct}");
        }
        let r45 = lookup_fig2_rpm_at_pct(&mt, da, 45.0).unwrap().rpm;
        let sy_low = map_mt_chart_xy(
            da,
            r45 - 80.0,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG2_RPM_MIN,
            FIG2_RPM_MAX,
        )
        .1;
        let mut low = mt_inputs(65.0);
        apply_mt_fig2_pct_drag(sy_low, &mt, &mut low);
        assert!((low.cruise_percent_bhp - MT_PCT_BHP_MIN).abs() < f64::EPSILON);

        let r75 = lookup_fig2_rpm_at_pct(&mt, da, 75.0).unwrap().rpm;
        let sy_high = map_mt_chart_xy(
            da,
            r75 + 80.0,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG2_RPM_MIN,
            FIG2_RPM_MAX,
        )
        .1;
        let mut high = mt_inputs(65.0);
        apply_mt_fig2_pct_drag(sy_high, &mt, &mut high);
        assert!((high.cruise_percent_bhp - MT_PCT_BHP_MAX).abs() < f64::EPSILON);
    }

    #[test]
    fn fig2_marker_rpm_matches_core_at_da6000_pct65() {
        let mt = Mt175rData::production();
        let inputs = mt_inputs(65.0);
        let cr = compute_mt175r_cruise(&mt, &inputs).unwrap();
        let da = cr.density_altitude_ft.unwrap();
        let rpm = cr.rpm.unwrap();
        let core_rpm = lookup_fig2_rpm_at_pct(&mt, da, 65.0).unwrap().rpm;
        assert!((rpm - core_rpm).abs() < 0.01, "ui={rpm} core={core_rpm}");
        let g = fig2_marker_guides(Locale::Es, da, Some(rpm), false);
        let svg = marker_guides_svg_snippet(&g);
        assert!(svg.contains(&format!("DA {:.0} ft", da.round())));
        assert!(svg.contains(&format!("{:.0} RPM", rpm.round())));
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn fig2_off_envelope_da_shows_fuera_de_carta() {
        let mt = Mt175rData::production();
        let da = 13_000.0;
        let rpm = lookup_fig2_rpm_at_pct(&mt, da, 65.0);
        assert!(rpm.is_none());
        let g = fig2_marker_guides(Locale::Es, da, None, true);
        assert!(!g.draw_guides);
        let notice = g.notice.as_ref().expect("notice");
        assert!(notice.contains("13000"));
        let svg = marker_guides_svg_snippet(&g);
        assert!(svg.contains("Fuera de carta"));
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn marker_and_axis_labels_fit_viewbox() {
        let g = fig2_marker_guides(Locale::Es, 7572.0, Some(2472.0), false);
        assert!(label_box_fits_viewbox(
            g.x_label_x,
            g.x_label_y,
            &g.x_label,
            LabelAnchor::Middle
        ));
        assert!(label_box_fits_viewbox(
            g.y_label_x,
            g.y_label_y,
            &g.y_label,
            LabelAnchor::Start
        ));
        let vb_w = MT_W + MT_MARGIN_L + MT_MARGIN_R;
        assert!(g.cx <= vb_w - MT_MARGIN_R);
        assert!(g.cx >= MT_MARGIN_L);
    }

    #[test]
    fn fig2_curve_end_labels_no_overlap_at_60_and_65_pct() {
        let mt = Mt175rData::production();
        for pct in [60.0, 65.0] {
            for skip in [false, true] {
                let labels = fig2_curve_end_label_positions(&mt, pct, skip);
                let boxes: Vec<_> = labels
                    .iter()
                    .map(|(t, x, y, _)| label_bbox_start(*x, *y, t))
                    .collect();
                for (i, a) in boxes.iter().enumerate() {
                    for b in boxes.iter().skip(i + 1) {
                        assert!(
                            !label_bboxes_overlap(*a, *b),
                            "overlap pct={pct} skip={skip} labels={labels:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn fig3_curve_end_labels_no_overlap_at_60_and_65_pct() {
        let mt = Mt175rData::production();
        for pct in [60.0, 65.0] {
            let labels = fig3_curve_end_label_positions(&mt, pct, false);
            let boxes: Vec<_> = labels
                .iter()
                .map(|(t, x, y, _)| label_bbox_start(*x, *y, t))
                .collect();
            for (i, a) in boxes.iter().enumerate() {
                for b in boxes.iter().skip(i + 1) {
                    assert!(
                        !label_bboxes_overlap(*a, *b),
                        "overlap pct={pct} labels={labels:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn fig3_selected_polyline_nonempty_at_60_pct() {
        let mt = Mt175rData::production();
        let pts = fig3_selected_polyline(&mt, 60.0);
        assert!(
            pts.len() > 2,
            "expected interpolated Fig3 curve, got {}",
            pts.len()
        );
    }

    #[test]
    fn fig3_marker_guides_include_tas_and_pct() {
        let mt = Mt175rData::production();
        let inputs = mt_inputs(65.0);
        let cr = compute_mt175r_cruise(&mt, &inputs).unwrap();
        let da = cr.density_altitude_ft.unwrap();
        let tas = cr.tas_chart_kt.unwrap();
        let g = fig3_marker_guides(Locale::Es, da, Some(tas), 65.0, false);
        let svg = marker_guides_svg_snippet(&g);
        assert!(svg.contains(&format!("DA {:.0} ft", da.round())));
        assert!(svg.contains(&format!("{:.0} kt", tas.round())));
        assert!(svg.contains("65%"));
        assert!(!svg.contains("NaN"));
    }
}
