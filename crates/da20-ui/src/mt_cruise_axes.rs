//! Axis frame, grid, and tick labels for MT AFMS Fig 2 / Fig 3 (DA vs RPM / TAS).

use leptos::prelude::*;

use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::mt_cruise_chart::{
    map_mt_chart_xy, MT_H, MT_MARGIN_B, MT_MARGIN_L, MT_MARGIN_R, MT_MARGIN_T, MT_W,
};

pub const DA_TICKS_FT: [f64; 7] = [0.0, 2000.0, 4000.0, 6000.0, 8000.0, 10_000.0, 12_000.0];
pub const RPM_TICKS: [f64; 10] = [
    1900.0, 2000.0, 2100.0, 2200.0, 2300.0, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0,
];
pub const TAS_TICKS_KT: [f64; 7] = [90.0, 100.0, 110.0, 120.0, 130.0, 140.0, 150.0];

pub fn plot_right() -> f64 {
    MT_MARGIN_L + MT_W
}

pub fn plot_bottom() -> f64 {
    MT_MARGIN_T + MT_H
}

pub fn plot_top() -> f64 {
    MT_MARGIN_T
}

pub fn viewbox_w() -> f64 {
    MT_W + MT_MARGIN_L + MT_MARGIN_R
}

pub fn viewbox_h() -> f64 {
    MT_H + MT_MARGIN_B + MT_MARGIN_T
}

fn major_vline(x: f64, y0: f64, y1: f64) -> impl IntoView {
    view! { <line x1=x y1=y0 x2=x y2=y1 class="chart-stroke-grid-major"/> }
}

fn major_hline(x0: f64, x1: f64, y: f64) -> impl IntoView {
    view! { <line x1=x0 y1=y x2=x1 y2=y class="chart-stroke-grid-major"/> }
}

#[derive(Clone, Copy)]
pub struct MtAxisSpec {
    pub da_min: f64,
    pub da_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub y_tick_fmt: fn(f64) -> String,
    pub y_axis_title: &'static str,
}

pub fn mt_fig2_axis_spec() -> MtAxisSpec {
    MtAxisSpec {
        da_min: 0.0,
        da_max: 12_000.0,
        y_min: 1900.0,
        y_max: 2800.0,
        y_tick_fmt: |v| format!("{:.0}", v),
        y_axis_title: "RPM",
    }
}

pub fn mt_fig3_axis_spec() -> MtAxisSpec {
    MtAxisSpec {
        da_min: 0.0,
        da_max: 12_000.0,
        y_min: 90.0,
        y_max: 150.0,
        y_tick_fmt: |v| format!("{:.0}", v),
        y_axis_title: "TAS (kt)",
    }
}

pub fn mt_chart_axes(spec: MtAxisSpec, y_ticks: &[f64], da_axis_title: String) -> impl IntoView {
    let x0 = MT_MARGIN_L;
    let x1 = plot_right();
    let y0 = plot_top();
    let y1 = plot_bottom();
    let tick_y = y1 + 14.0;
    let da_title_y = y1 + 36.0;
    let y_title_x = 18.0;
    let y_title_y = (y0 + y1) * 0.5;
    view! {
        {DA_TICKS_FT.into_iter().map(|da| {
            let (sx, _) = map_mt_chart_xy(da, spec.y_min, spec.da_min, spec.da_max, spec.y_min, spec.y_max);
            view! {
                <>
                    {major_vline(sx, y0, y1)}
                    {guide_line_label(sx, tick_y, format!("{:.0}", da), LabelAnchor::Middle)}
                </>
            }
        }).collect_view()}
        {y_ticks.iter().copied().map(|yv| {
            let (_, sy) = map_mt_chart_xy(spec.da_min, yv, spec.da_min, spec.da_max, spec.y_min, spec.y_max);
            view! {
                <>
                    {major_hline(x0, x1, sy)}
                    {guide_line_label(x0 - 8.0, sy, (spec.y_tick_fmt)(yv), LabelAnchor::End)}
                </>
            }
        }).collect_view()}
        <line x1=x0 y1=y1 x2=x1 y2=y1 class="chart-stroke-line"/>
        <line x1=x0 y1=y0 x2=x0 y2=y1 class="chart-stroke-line"/>
        {guide_line_label(
            (x0 + x1) * 0.5,
            da_title_y,
            da_axis_title,
            LabelAnchor::Middle,
        )}
        <text
            x=y_title_x
            y=y_title_y
            class="chart-guide-label"
            font-size="10"
            text-anchor="middle"
            dominant-baseline="middle"
            transform=format!("rotate(-90 {y_title_x} {y_title_y})")
        >
            {spec.y_axis_title}
        </text>
    }
}
