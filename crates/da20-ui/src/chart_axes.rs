//! AFM-style axis tick labels (values + units), legible on all themes/palettes.

use leptos::prelude::*;

use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::i18n::Copy;
use da20_core::rev29::charts::map_to_svg;
use da20_core::rev29::hand_plot::{
    x_ea_ft, x_headwind_kt, x_oat, x_obstacle_m, x_tailwind_kt, x_weight_kg, y_roc_fpm, CLIMB_EQ,
    CLIMB_ROC, STRIP_BOTTOM, TAKEOFF_HW, TAKEOFF_OAT, TAKEOFF_OBS, TAKEOFF_TW, TAKEOFF_WEIGHT,
};

const STRIP_TICK_Y: f64 = 518.0;

fn strip_tick(x: f64, y: f64, text: String) -> impl IntoView {
    guide_line_label(x, y, text, LabelAnchor::Middle)
}

/// Fig 5.4 strip — x-axis ticks on every panel (incl. TW sub-panel).
pub fn takeoff_strip_x_axis_ticks(c: &Copy) -> impl IntoView {
    view! {
        {[-20.0, 0.0, 20.0, 40.0, 55.0].into_iter().map(|t| {
            strip_tick(x_oat(t), STRIP_TICK_Y, format!("{t:.0} °C"))
        }).collect_view()}
        {[600.0, 650.0, 700.0, 750.0, 800.0].into_iter().map(|kg| {
            strip_tick(x_weight_kg(kg), STRIP_TICK_Y, format!("{kg:.0} kg"))
        }).collect_view()}
        {[0.0, 1.0, 2.0, 3.0, 4.0, 5.0].into_iter().map(|kt| {
            strip_tick(x_tailwind_kt(kt), STRIP_TICK_Y, format!("{kt:.0} kt"))
        }).collect_view()}
        {[0.0, 5.0, 10.0, 15.0, 20.0].into_iter().map(|kt| {
            strip_tick(x_headwind_kt(kt), STRIP_TICK_Y, format!("{kt:.0} kt"))
        }).collect_view()}
        {[0.0, 5.0, 10.0, 15.0].into_iter().map(|m| {
            strip_tick(x_obstacle_m(m), STRIP_TICK_Y, format!("{m:.0} m"))
        }).collect_view()}
        {strip_tick(
            TAKEOFF_OAT.x - 18.0,
            (STRIP_BOTTOM + 40.0) / 2.0,
            c.axis_dist_m.to_string(),
        )}
    }
}

/// Climb strip — OAT (°C) on fan panel, EA (ft) on ROC panel x, ROC (fpm) on ROC y.
pub fn climb_strip_axis_ticks(c: &Copy) -> impl IntoView {
    view! {
        {[-20.0, 0.0, 20.0, 40.0, 55.0].into_iter().map(|t| {
            let x = CLIMB_EQ.x + (t + 20.0) / 75.0 * CLIMB_EQ.w;
            strip_tick(x, STRIP_TICK_Y, format!("{t:.0} °C"))
        }).collect_view()}
        {[0.0, 3500.0, 7000.0, 10_500.0, 14_000.0].into_iter().map(|ea| {
            strip_tick(x_ea_ft(ea), STRIP_TICK_Y, format!("{ea:.0} ft"))
        }).collect_view()}
        {[0.0, 250.0, 500.0, 750.0, 1000.0].into_iter().map(|roc| {
            strip_tick(
                CLIMB_ROC.x - 6.0,
                y_roc_fpm(roc),
                format!("{roc:.0} fpm"),
            )
        }).collect_view()}
        {strip_tick(
            TAKEOFF_OAT.x - 18.0,
            (STRIP_BOTTOM + 40.0) / 2.0,
            c.axis_dist_m.to_string(),
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
    let (x_vals, x_fmt): (Vec<f64>, fn(f64) -> String) = match kind {
        crate::interactive_chart::ChartKind::Takeoff => {
            (vec![-20.0, 0.0, 20.0, 40.0, 55.0], |v| format!("{v:.0} °C"))
        }
        crate::interactive_chart::ChartKind::Climb => {
            (vec![0.0, 3500.0, 7000.0, 10_500.0, 14_000.0], |v| {
                format!("{v:.0} ft")
            })
        }
        crate::interactive_chart::ChartKind::Landing => {
            (vec![0.0, 1750.0, 3500.0, 5250.0, 7000.0], |v| {
                format!("{v:.0} ft")
            })
        }
        crate::interactive_chart::ChartKind::Wind => {
            (vec![0.0, 30.0, 60.0, 90.0], |v| format!("{v:.0}°"))
        }
    };
    let y_vals: Vec<f64> = match kind {
        crate::interactive_chart::ChartKind::Takeoff => vec![150.0, 400.0, 650.0, 900.0],
        crate::interactive_chart::ChartKind::Climb => vec![0.0, 250.0, 500.0, 750.0, 1000.0],
        crate::interactive_chart::ChartKind::Landing => vec![600.0, 900.0, 1200.0, 1600.0],
        crate::interactive_chart::ChartKind::Wind => vec![0.0, 3.0, 6.0, 9.0, 12.0],
    };
    let y_fmt = |v: f64| match kind {
        crate::interactive_chart::ChartKind::Takeoff => format!("{v:.0} m"),
        crate::interactive_chart::ChartKind::Climb => format!("{v:.0} fpm"),
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
        <text x=CLIMB_ROC.x+CLIMB_ROC.w/2.0 y="538" class="chart-axis-title" text-anchor="middle">{c.axis_ea_ft}</text>
        <text x=CLIMB_ROC.x - 8.0 y="36" class="chart-axis-title" text-anchor="end">{c.axis_roc_fpm}</text>
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
}
