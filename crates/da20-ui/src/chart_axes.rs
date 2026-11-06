//! AFM-style axis tick labels and major gridlines at those ticks.

use leptos::prelude::*;

use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::i18n::Copy;
use da20_core::rev29::charts::map_to_svg;
use da20_core::rev29::hand_plot::{
    x_headwind_kt, x_oat, x_obstacle_m, x_roc_fpm, x_tailwind_kt, x_weight_kg, y_dist, y_ea_ft,
    CLIMB_EQ, CLIMB_ROC, DIST_MAX_M, DIST_MIN_M, STRIP_BOTTOM, STRIP_TOP, TAKEOFF_HW, TAKEOFF_OAT,
    TAKEOFF_OBS, TAKEOFF_TW, TAKEOFF_WEIGHT,
};

const STRIP_TICK_Y: f64 = 518.0;
const OAT_TICKS_C: [f64; 5] = [-20.0, 0.0, 20.0, 40.0, 55.0];
const WEIGHT_TICKS_KG: [f64; 5] = [600.0, 650.0, 700.0, 750.0, 800.0];
const TW_TICKS_KT: [f64; 6] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
const HW_TICKS_KT: [f64; 5] = [0.0, 5.0, 10.0, 15.0, 20.0];
const OBS_TICKS_M: [f64; 4] = [0.0, 5.0, 10.0, 15.0];
const EA_TICKS_FT: [f64; 5] = [0.0, 3500.0, 7000.0, 10_500.0, 14_000.0];
const ROC_TICKS_FPM: [f64; 5] = [0.0, 275.0, 550.0, 825.0, 1100.0];

fn major_vline(x: f64, y0: f64, y1: f64) -> impl IntoView {
    view! {
        <line x1=x y1=y0 x2=x y2=y1 class="chart-stroke-grid-major"/>
    }
}

fn major_hline(x0: f64, x1: f64, y: f64) -> impl IntoView {
    view! {
        <line x1=x0 y1=y x2=x1 y2=y class="chart-stroke-grid-major"/>
    }
}

fn thin_vline(x: f64, y0: f64, y1: f64) -> impl IntoView {
    view! {
        <line x1=x y1=y0 x2=x y2=y1 class="chart-stroke-grid"/>
    }
}

fn thin_hline(x0: f64, x1: f64, y: f64) -> impl IntoView {
    view! {
        <line x1=x0 y1=y x2=x1 y2=y class="chart-stroke-grid"/>
    }
}

fn strip_distance_hlines(x0: f64, x1: f64) -> impl IntoView {
    view! {
        {(0..=8).map(|i| {
            let m = DIST_MIN_M + (DIST_MAX_M - DIST_MIN_M) * i as f64 / 8.0;
            major_hline(x0, x1, y_dist(m))
        }).collect_view()}
    }
}

fn strip_tick(x: f64, y: f64, text: String) -> impl IntoView {
    guide_line_label(x, y, text, LabelAnchor::Middle)
}

/// Major grid at labelled x ticks (+ distance y ticks) for Fig 5.4 strip.
pub fn takeoff_strip_major_grid() -> impl IntoView {
    let x0 = TAKEOFF_OAT.x;
    let x1 = TAKEOFF_OBS.x + TAKEOFF_OBS.w;
    view! {
        {strip_distance_hlines(x0, x1)}
        {OAT_TICKS_C.into_iter().map(|t| major_vline(x_oat(t), STRIP_TOP, STRIP_BOTTOM)).collect_view()}
        {WEIGHT_TICKS_KG.into_iter().map(|kg| major_vline(x_weight_kg(kg), STRIP_TOP, STRIP_BOTTOM)).collect_view()}
        {TW_TICKS_KT.into_iter().map(|kt| major_vline(x_tailwind_kt(kt), STRIP_TOP, STRIP_BOTTOM)).collect_view()}
        {HW_TICKS_KT.into_iter().map(|kt| major_vline(x_headwind_kt(kt), STRIP_TOP, STRIP_BOTTOM)).collect_view()}
        {OBS_TICKS_M.into_iter().map(|m| major_vline(x_obstacle_m(m), STRIP_TOP, STRIP_BOTTOM)).collect_view()}
    }
}

pub fn climb_strip_major_grid() -> impl IntoView {
    let roc_right = CLIMB_ROC.x + CLIMB_ROC.w;
    view! {
        {EA_TICKS_FT.into_iter().map(|ea| {
            let y = y_ea_ft(ea);
            view! {
                <>
                    // Same EA level as Fig 5.2 fan chart; split at panel join so lines align exactly.
                    {thin_hline(CLIMB_EQ.x, CLIMB_ROC.x, y)}
                    {thin_hline(CLIMB_ROC.x, roc_right, y)}
                </>
            }
        }).collect_view()}
        {OAT_TICKS_C.into_iter().map(|t| {
            let x = CLIMB_EQ.x + (t + 20.0) / 75.0 * CLIMB_EQ.w;
            thin_vline(x, STRIP_TOP, STRIP_BOTTOM)
        }).collect_view()}
        {ROC_TICKS_FPM.into_iter().map(|roc| {
            thin_vline(x_roc_fpm(roc), STRIP_TOP, STRIP_BOTTOM)
        }).collect_view()}
    }
}

pub fn interactive_chart_major_grid(
    kind: crate::interactive_chart::ChartKind,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> impl IntoView {
    let (x_vals, y_vals) = interactive_tick_values(kind);
    view! {
        {x_vals.into_iter().map(|v| {
            let (sx, _) = map_to_svg(v, y_min, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0);
            major_vline(sx, 20.0, 270.0)
        }).collect_view()}
        {y_vals.into_iter().map(|v| {
            let (_, sy) = map_to_svg(x_min, v, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0);
            major_hline(40.0, 380.0, sy)
        }).collect_view()}
    }
}

fn interactive_tick_values(kind: crate::interactive_chart::ChartKind) -> (Vec<f64>, Vec<f64>) {
    let x_vals = match kind {
        crate::interactive_chart::ChartKind::Takeoff => OAT_TICKS_C.to_vec(),
        // Climb preview: OAT (x) vs EA (y) on Fig 5.2 fan chart.
        crate::interactive_chart::ChartKind::Climb => OAT_TICKS_C.to_vec(),
        crate::interactive_chart::ChartKind::Landing => vec![0.0, 1750.0, 3500.0, 5250.0, 7000.0],
        crate::interactive_chart::ChartKind::Wind => vec![0.0, 30.0, 60.0, 90.0],
    };
    let y_vals = match kind {
        crate::interactive_chart::ChartKind::Takeoff => vec![150.0, 400.0, 650.0, 900.0],
        crate::interactive_chart::ChartKind::Climb => EA_TICKS_FT.to_vec(),
        crate::interactive_chart::ChartKind::Landing => vec![600.0, 900.0, 1200.0, 1600.0],
        crate::interactive_chart::ChartKind::Wind => vec![0.0, 3.0, 6.0, 9.0, 12.0],
    };
    (x_vals, y_vals)
}

/// Fig 5.4 strip — x-axis ticks on every panel (incl. TW sub-panel).
pub fn takeoff_strip_x_axis_ticks(c: &Copy) -> impl IntoView {
    view! {
        {OAT_TICKS_C.into_iter().map(|t| {
            strip_tick(x_oat(t), STRIP_TICK_Y, format!("{t:.0} °C"))
        }).collect_view()}
        {WEIGHT_TICKS_KG.into_iter().map(|kg| {
            strip_tick(x_weight_kg(kg), STRIP_TICK_Y, format!("{kg:.0} kg"))
        }).collect_view()}
        {TW_TICKS_KT.into_iter().map(|kt| {
            strip_tick(x_tailwind_kt(kt), STRIP_TICK_Y, format!("{kt:.0} kt"))
        }).collect_view()}
        {HW_TICKS_KT.into_iter().map(|kt| {
            strip_tick(x_headwind_kt(kt), STRIP_TICK_Y, format!("{kt:.0} kt"))
        }).collect_view()}
        {OBS_TICKS_M.into_iter().map(|m| {
            strip_tick(x_obstacle_m(m), STRIP_TICK_Y, format!("{m:.0} m"))
        }).collect_view()}
        {(0..=8).map(|i| {
            let m = DIST_MIN_M + (DIST_MAX_M - DIST_MIN_M) * i as f64 / 8.0;
            guide_line_label(
                TAKEOFF_OAT.x - 3.0,
                y_dist(m),
                format!("{m:.0} m"),
                LabelAnchor::End,
            )
        }).collect_view()}
        {strip_tick(
            TAKEOFF_OAT.x - 18.0,
            (STRIP_BOTTOM + STRIP_TOP) / 2.0,
            c.axis_dist_m.to_string(),
        )}
    }
}

/// Climb strip — OAT (°C) on fan panel; shared EA (ft) on Y; ROC (fpm) on ROC panel X.
pub fn climb_strip_axis_ticks(c: &Copy) -> impl IntoView {
    view! {
        {OAT_TICKS_C.into_iter().map(|t| {
            let x = CLIMB_EQ.x + (t + 20.0) / 75.0 * CLIMB_EQ.w;
            strip_tick(x, STRIP_TICK_Y, format!("{t:.0} °C"))
        }).collect_view()}
        {EA_TICKS_FT.into_iter().map(|ea| {
            guide_line_label(
                CLIMB_EQ.x - 3.0,
                y_ea_ft(ea),
                format!("{ea:.0} ft"),
                LabelAnchor::End,
            )
        }).collect_view()}
        {EA_TICKS_FT.into_iter().map(|ea| {
            guide_line_label(
                CLIMB_ROC.x + CLIMB_ROC.w + 3.0,
                y_ea_ft(ea),
                format!("{ea:.0} ft"),
                LabelAnchor::Start,
            )
        }).collect_view()}
        {ROC_TICKS_FPM.into_iter().map(|roc| {
            strip_tick(x_roc_fpm(roc), STRIP_TICK_Y, format!("{roc:.0} fpm"))
        }).collect_view()}
        {strip_tick(
            CLIMB_EQ.x - 18.0,
            (STRIP_BOTTOM + STRIP_TOP) / 2.0,
            c.strip_climb_roc_ea_y_axis.to_string(),
        )}
        {strip_tick(
            CLIMB_ROC.x + CLIMB_ROC.w + 18.0,
            (STRIP_BOTTOM + STRIP_TOP) / 2.0,
            c.strip_climb_roc_ea_y_axis.to_string(),
        )}
    }
}

/// Interactive phase charts (400×300 viewBox).
pub fn interactive_chart_axis_ticks(
    kind: crate::interactive_chart::ChartKind,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> impl IntoView {
    let (x_vals, y_vals) = interactive_tick_values(kind);
    let x_fmt = |v: f64| match kind {
        crate::interactive_chart::ChartKind::Takeoff => format!("{v:.0} °C"),
        crate::interactive_chart::ChartKind::Climb => format!("{v:.0} °C"),
        crate::interactive_chart::ChartKind::Landing => format!("{v:.0} ft"),
        crate::interactive_chart::ChartKind::Wind => format!("{v:.0}°"),
    };
    let y_fmt = |v: f64| match kind {
        crate::interactive_chart::ChartKind::Takeoff => format!("{v:.0} m"),
        crate::interactive_chart::ChartKind::Climb => format!("{v:.0} ft"),
        crate::interactive_chart::ChartKind::Landing => format!("{v:.0} ft"),
        crate::interactive_chart::ChartKind::Wind => format!("{v:.0} kt"),
    };

    view! {
        {x_vals.into_iter().map(|v| {
            let (sx, _) = map_to_svg(v, y_min, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0);
            guide_line_label(sx, 278.0, x_fmt(v), LabelAnchor::Middle)
        }).collect_view()}
        {y_vals.into_iter().map(|v| {
            let (_, sy) = map_to_svg(x_min, v, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0);
            guide_line_label(44.0, sy, y_fmt(v), LabelAnchor::End)
        }).collect_view()}
    }
}

pub fn takeoff_strip_panel_titles(c: &Copy) -> impl IntoView {
    view! {
        <text x=TAKEOFF_OAT.x+TAKEOFF_OAT.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.strip_panel_oat_pa}</text>
        <text x=TAKEOFF_WEIGHT.x+TAKEOFF_WEIGHT.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.axis_weight_kg}</text>
        <text x=TAKEOFF_TW.x+TAKEOFF_TW.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.strip_panel_tw}</text>
        <text x=TAKEOFF_HW.x+TAKEOFF_HW.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.strip_panel_hw}</text>
        <text x=TAKEOFF_OBS.x+TAKEOFF_OBS.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.axis_obstacle_m}</text>
    }
}

pub fn climb_strip_panel_titles(c: &Copy, roc_title: &'static str) -> impl IntoView {
    view! {
        <text x=CLIMB_EQ.x+CLIMB_EQ.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.axis_oat_c}</text>
        <text x=CLIMB_ROC.x+CLIMB_ROC.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.strip_climb_roc_x_axis}</text>
        <text x=CLIMB_EQ.x - 8.0 y="36" class="chart-axis-title" text-anchor="end">{c.strip_climb_roc_ea_y_axis}</text>
        <text x=CLIMB_ROC.x + 8.0 y="36" class="chart-axis-title" text-anchor="start">{roc_title}</text>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{copy, Locale};

    #[test]
    fn tw_subpanel_ticks_use_tailwind_x_mapper() {
        assert!((x_tailwind_kt(0.0) - (TAKEOFF_TW.x + TAKEOFF_TW.w)).abs() < 0.01);
        assert!((x_tailwind_kt(5.0) - TAKEOFF_TW.x).abs() < 0.01);
        let c = copy(Locale::En);
        assert!(!c.axis_weight_kg.is_empty());
    }

    use da20_core::rev29::hand_plot::{x_roc_fpm, CLIMB_ROC};

    #[test]
    fn climb_roc_strip_x_axis_ticks_at_standard_fpm_values() {
        assert_eq!(ROC_TICKS_FPM, [0.0, 275.0, 550.0, 825.0, 1100.0]);
        for roc in ROC_TICKS_FPM {
            let x = x_roc_fpm(roc);
            assert!(
                (CLIMB_ROC.x..=CLIMB_ROC.x + CLIMB_ROC.w).contains(&x),
                "ROC tick {roc} fpm maps outside ROC panel"
            );
        }
        assert!((x_roc_fpm(0.0) - CLIMB_ROC.x).abs() < 0.01);
        assert!((x_roc_fpm(1100.0) - (CLIMB_ROC.x + CLIMB_ROC.w)).abs() < 0.01);
    }

    #[test]
    fn climb_roc_strip_has_no_altitude_labels_on_roc_line() {
        use da20_core::rev29::hand_plot::ROC_PANEL_MAX_FPM;
        assert!((ROC_PANEL_MAX_FPM - 1100.0).abs() < f64::EPSILON);
        assert_eq!(ROC_TICKS_FPM.first(), Some(&0.0));
        assert_eq!(ROC_TICKS_FPM.last(), Some(&1100.0));
    }

    #[test]
    fn climb_roc_strip_x_axis_title_es_en() {
        assert_eq!(
            copy(Locale::Es).strip_climb_roc_x_axis,
            "Razón de ascenso (fpm)"
        );
        assert_eq!(
            copy(Locale::En).strip_climb_roc_x_axis,
            "Rate of climb (fpm)"
        );
    }

    #[test]
    fn climb_strip_ea_y_axis_title_full_es_en() {
        let es = copy(Locale::Es);
        let en = copy(Locale::En);
        assert_eq!(es.strip_climb_roc_ea_y_axis, "Altitud equivalente (ft)");
        assert_eq!(en.strip_climb_roc_ea_y_axis, "Equivalent altitude (ft)");
    }

    #[test]
    fn climb_major_grid_includes_roc_verticals_at_same_ticks() {
        assert_eq!(ROC_TICKS_FPM.len(), EA_TICKS_FT.len());
    }

    #[test]
    fn climb_roc_panel_ea_tick_y_matches_fan_ea_axis() {
        for &ea in &EA_TICKS_FT {
            let y_fan = y_ea_ft(ea);
            let y_roc_right = y_ea_ft(ea);
            assert!(
                (y_fan - y_roc_right).abs() < 1e-9,
                "EA {ea} ft: fan and ROC EA ticks must share y_ea_ft"
            );
        }
    }

    #[test]
    fn takeoff_major_grid_includes_tw_and_oat_verticals() {
        assert_eq!(TW_TICKS_KT.len(), 6);
        assert!((x_oat(OAT_TICKS_C[0]) - TAKEOFF_OAT.x).abs() < 0.01);
    }
}
