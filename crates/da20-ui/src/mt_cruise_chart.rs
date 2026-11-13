//! Pure helpers for MT Fig 2/3 cruise chart geometry, drag mapping, and marker guides.

use da20_core::altitude::isa_troposphere_density_altitude_ft;
use da20_core::mt175r::charts::{
    fig2_family_chart_points, fig2_pct_bhp_chart_points, fig3_curve_chart_points,
    pct_bhp_from_fig2_drag_rpm, PRINTED_PCT_CURVES,
};
use da20_core::mt175r::Mt175rData;
use da20_core::rev29::charts::{map_from_svg, map_to_svg};
use da20_core::types::{CruiseSolveMode, FlightInputs};

pub const MT_MARGIN_L: f64 = 44.0;
pub const MT_MARGIN_B: f64 = 28.0;
pub const MT_MARGIN_T: f64 = 12.0;
pub const MT_MARGIN_R: f64 = 10.0;
pub const MT_W: f64 = 320.0;
pub const MT_H: f64 = 200.0;

pub const FIG2_DA_MIN: f64 = 0.0;
pub const FIG2_DA_MAX: f64 = 12_000.0;
pub const FIG2_RPM_MIN: f64 = 2100.0;
pub const FIG2_RPM_MAX: f64 = 2900.0;
pub const FIG3_TAS_MIN: f64 = 90.0;
pub const FIG3_TAS_MAX: f64 = 140.0;

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
        MT_MARGIN_B,
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
        MT_MARGIN_B,
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
    pub on_chart: bool,
}

fn chart_axis_y() -> f64 {
    MT_MARGIN_B + MT_H
}

fn chart_axis_x() -> f64 {
    MT_MARGIN_L
}

pub fn fig2_marker_guides(da_ft: f64, rpm: Option<f64>, out_of_chart: bool) -> MtChartMarkerGuides {
    let da_clamped = da_ft.clamp(FIG2_DA_MIN, FIG2_DA_MAX);
    let on_chart = !out_of_chart && rpm.is_some() && rpm.unwrap().is_finite();
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
        let sy = map_mt_chart_xy(
            da_clamped,
            FIG2_RPM_MIN,
            FIG2_DA_MIN,
            FIG2_DA_MAX,
            FIG2_RPM_MIN,
            FIG2_RPM_MAX,
        )
        .1;
        (
            map_mt_chart_xy(
                da_clamped,
                FIG2_RPM_MIN,
                FIG2_DA_MIN,
                FIG2_DA_MAX,
                FIG2_RPM_MIN,
                FIG2_RPM_MAX,
            )
            .0,
            sy,
        )
    };
    let x_label = format!("DA {:.0} ft", da_ft);
    let y_label = if on_chart {
        format!("{:.0} RPM", rpm_v)
    } else {
        "Fuera de carta".into()
    };
    MtChartMarkerGuides {
        cx,
        cy,
        x_axis_y: chart_axis_y(),
        y_axis_x: chart_axis_x(),
        x_label,
        y_label,
        on_chart,
    }
}

pub fn fig3_marker_guides(
    da_ft: f64,
    tas_kt: Option<f64>,
    pct_bhp: f64,
    out_of_chart: bool,
) -> MtChartMarkerGuides {
    let da_clamped = da_ft.clamp(FIG2_DA_MIN, FIG2_DA_MAX);
    let on_chart = !out_of_chart && tas_kt.is_some() && tas_kt.unwrap().is_finite();
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
        (
            map_mt_chart_xy(
                da_clamped,
                FIG3_TAS_MIN,
                FIG2_DA_MIN,
                FIG2_DA_MAX,
                FIG3_TAS_MIN,
                FIG3_TAS_MAX,
            )
            .0,
            map_mt_chart_xy(
                da_clamped,
                FIG3_TAS_MIN,
                FIG2_DA_MIN,
                FIG2_DA_MAX,
                FIG3_TAS_MIN,
                FIG3_TAS_MAX,
            )
            .1,
        )
    };
    let x_label = format!("DA {:.0} ft", da_ft);
    let pct_label = format_pct_bhp_label(pct_bhp);
    let value_lbl = if on_chart {
        format!("{:.0} kt", tas_v)
    } else {
        "Fuera de carta".into()
    };
    MtChartMarkerGuides {
        cx,
        cy,
        x_axis_y: chart_axis_y(),
        y_axis_x: chart_axis_x(),
        x_label,
        y_label: format!("{value_lbl} · {pct_label}"),
        on_chart,
    }
}

/// Serialize guide geometry for unit tests (no NaN in finite paths).
#[cfg_attr(not(test), allow(dead_code))]
pub fn marker_guides_svg_snippet(g: &MtChartMarkerGuides) -> String {
    let mut lines = vec![
        format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            g.cx, g.cy, g.cx, g.x_axis_y
        ),
        format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            g.cx, g.cy, g.y_axis_x, g.cy
        ),
        format!("<text>{}</text>", g.x_label),
        format!("<text>{}</text>", g.y_label),
    ];
    if g.on_chart {
        lines.push(format!("<circle cx=\"{}\" cy=\"{}\" r=\"5\"/>", g.cx, g.cy));
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
        .map(|&p| (p, fig3_curve_chart_points(mt, p)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::altitude::isa_troposphere_density_altitude_ft;
    use da20_core::cruise::isa_temp_c_at_pa;
    use da20_core::mt175r::charts::{
        lookup_fig2_rpm_at_pct, MT_PCT_BHP_MAX, MT_PCT_BHP_MIN,
    };
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
        let g = fig2_marker_guides(da, Some(rpm), false);
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
        let g = fig2_marker_guides(da, None, true);
        assert!(g.y_label.contains("Fuera de carta"));
        let svg = marker_guides_svg_snippet(&g);
        assert!(!svg.contains("NaN"));
    }

    #[test]
    fn fig3_marker_guides_include_tas_and_pct() {
        let mt = Mt175rData::production();
        let inputs = mt_inputs(65.0);
        let cr = compute_mt175r_cruise(&mt, &inputs).unwrap();
        let da = cr.density_altitude_ft.unwrap();
        let tas = cr.tas_chart_kt.unwrap();
        let g = fig3_marker_guides(da, Some(tas), 65.0, false);
        let svg = marker_guides_svg_snippet(&g);
        assert!(svg.contains(&format!("DA {:.0} ft", da.round())));
        assert!(svg.contains(&format!("{:.0} kt", tas.round())));
        assert!(svg.contains("65%"));
        assert!(!svg.contains("NaN"));
    }
}
