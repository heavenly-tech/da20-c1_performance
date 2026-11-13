//! MT AFMS Fig 2/3 reference charts and Fig 4 table for cruise phase.

use da20_core::mt175r::charts::{
    fig2_curve_chart_points, fig3_curve_chart_points, fig3_full_throttle_chart_points,
};
use da20_core::mt175r::Mt175rData;
use da20_core::rev29::charts::map_to_svg;
use da20_core::types::{CruiseSolveMode, FlightInputs};
use leptos::prelude::*;

use crate::i18n::{copy, Locale};
use crate::storage::compute_all;
use crate::strip_plot::format_svg_polyline;
use crate::table_source_panel::{mt_fig4_table_cell_highlight, mt_fig4_table_row_highlight};

const MT_MARGIN_L: f64 = 44.0;
const MT_MARGIN_B: f64 = 28.0;
const MT_MARGIN_T: f64 = 12.0;
const MT_MARGIN_R: f64 = 10.0;
const MT_W: f64 = 320.0;
const MT_H: f64 = 200.0;

fn highlighted_curves(
    cr: Option<&da20_core::types::CruiseResult>,
    inputs: &FlightInputs,
    solved_pct: Option<f64>,
) -> Vec<(i32, f64)> {
    if let Some(f2) = cr.and_then(|c| c.mt_fig2_source.as_ref()) {
        return f2
            .curve_brackets
            .iter()
            .filter(|b| b.weight > 1e-6)
            .map(|b| (b.pct_bhp, b.weight))
            .collect();
    }
    let pct = chart_pct_bhp(inputs, solved_pct);
    vec![(pct, 1.0)]
}

fn bracket_caption(locale: Locale, brackets: &[(i32, f64)], pct_bhp_lbl: &str) -> String {
    if brackets.len() <= 1 {
        return String::new();
    }
    let parts: Vec<String> = brackets
        .iter()
        .map(|(p, w)| format!("{p}%={w:.3}"))
        .collect();
    match locale {
        Locale::Es => format!(" · {pct_bhp_lbl}: {}", parts.join(", ")),
        Locale::En => format!(" · {pct_bhp_lbl}: {}", parts.join(", ")),
    }
}

fn chart_pct_bhp(inputs: &FlightInputs, solved_pct: Option<f64>) -> i32 {
    let pct = match inputs.cruise_solve_mode {
        CruiseSolveMode::SetPercentBhp => inputs.cruise_percent_bhp,
        CruiseSolveMode::SetRpm => solved_pct.unwrap_or(inputs.cruise_percent_bhp),
    };
    [45, 55, 65, 75]
        .iter()
        .min_by(|a, b| {
            ((**a as f64 - pct).abs())
                .partial_cmp(&(**b as f64 - pct).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or(65)
}

fn map_da_y(da: f64, y: f64, da_min: f64, da_max: f64, y_min: f64, y_max: f64) -> (f64, f64) {
    map_to_svg(
        da,
        y,
        da_min,
        da_max,
        y_min,
        y_max,
        MT_MARGIN_L,
        MT_MARGIN_B,
        MT_W,
        MT_H,
    )
}

fn polyline_svg(points: &[(f64, f64)], da_min: f64, da_max: f64, y_min: f64, y_max: f64) -> String {
    let svg_pts: Vec<(f64, f64)> = points
        .iter()
        .map(|(da, y)| map_da_y(*da, *y, da_min, da_max, y_min, y_max))
        .collect();
    format_svg_polyline(&svg_pts)
}

fn mt_chart_cursor_readout(
    _locale: Locale,
    chart_ok: bool,
    da: Option<f64>,
    rpm: Option<f64>,
    tas: Option<f64>,
) -> String {
    if !chart_ok {
        return "—".into();
    }
    let da = da.unwrap_or(f64::NAN);
    let rpm = rpm.unwrap_or(f64::NAN);
    let tas = tas.unwrap_or(f64::NAN);
    if !da.is_finite() || !rpm.is_finite() || !tas.is_finite() {
        return "—".into();
    }
    format!("DA {:.0} ft · {:.0} RPM · {:.0} kt", da, rpm, tas)
}

#[component]
pub fn MtCruisePanel(
    locale: RwSignal<Locale>,
    data: RwSignal<da20_core::PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        <p class="lab plot-hint">{move || copy(locale.get()).plot_hint}</p>
        {move || {
            let mt = Mt175rData::production();
            let loc = locale.get();
            let i = inputs.get();
            let b = compute_all(&data.get(), &i);
            let chart_ok = b.cruise.is_some();
            let cr = b.cruise.as_ref();
            let pct = chart_pct_bhp(&i, cr.and_then(|c| c.percent_bhp));
            let fig2_brackets = highlighted_curves(cr, &i, cr.and_then(|c| c.percent_bhp));
            let fig3_brackets = cr
                .and_then(|c| c.mt_fig3_source.as_ref())
                .map(|f3| {
                    f3.curve_brackets
                        .iter()
                        .filter(|b| b.weight > 1e-6)
                        .map(|b| (b.pct_bhp, b.weight))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| fig2_brackets.clone());
            let da = cr.and_then(|c| c.density_altitude_ft);
            let cursor_rpm = cr.and_then(|c| c.rpm);
            let cursor_tas = cr.and_then(|c| c.tas_chart_kt);

            let da_min = 0.0;
            let da_max = 12_000.0;
            let rpm_min = 2100.0;
            let rpm_max = 2900.0;
            let tas_min = 90.0;
            let tas_max = 140.0;

            let fig2_lines: Vec<(String, String)> = fig2_brackets
                .iter()
                .map(|(p, w)| {
                    let cls = if *w > 0.99 {
                        "chart-stroke-line-hi"
                    } else {
                        "chart-stroke-line"
                    };
                    (
                        cls.to_string(),
                        polyline_svg(
                            &fig2_curve_chart_points(&mt, *p),
                            da_min,
                            da_max,
                            rpm_min,
                            rpm_max,
                        ),
                    )
                })
                .collect();
            let fig3_lines: Vec<(String, String)> = fig3_brackets
                .iter()
                .map(|(p, w)| {
                    let cls = if *w > 0.99 {
                        "chart-stroke-line-hi"
                    } else {
                        "chart-stroke-line"
                    };
                    (
                        cls.to_string(),
                        polyline_svg(
                            &fig3_curve_chart_points(&mt, *p),
                            da_min,
                            da_max,
                            tas_min,
                            tas_max,
                        ),
                    )
                })
                .collect();
            let fig3_ft_line = polyline_svg(
                &fig3_full_throttle_chart_points(&mt),
                da_min,
                da_max,
                tas_min,
                tas_max,
            );

            let cursor_readout =
                mt_chart_cursor_readout(loc, chart_ok, da, cursor_rpm, cursor_tas);

            let fig2_cursor = if chart_ok {
                let da = da.expect("chart_ok");
                let rpm = cursor_rpm.expect("chart_ok");
                let (cx, cy_rpm) =
                    map_da_y(da, rpm, da_min, da_max, rpm_min, rpm_max);
                view! { <circle class="chart-marker" cx=cx cy=cy_rpm r="4"/> }.into_any()
            } else {
                view! { <span></span> }.into_any()
            };

            let vb_w = MT_W + MT_MARGIN_L + MT_MARGIN_R;
            let vb_h = MT_H + MT_MARGIN_B + MT_MARGIN_T;
            let c = copy(loc);

            view! {
                <p class="lab mono">{format!(
                    "{} · {pct}% BHP{}",
                    c.mt_fig2_title,
                    bracket_caption(loc, &fig2_brackets, c.mt_pct_bhp_weight_lbl),
                )}</p>
                <svg class="plot-svg strip-svg mt-fig-svg" viewBox=format!("0 0 {vb_w} {vb_h}")>
                    <rect class="chart-panel" x="0" y="0" width=vb_w height=vb_h/>
                    {fig2_lines.iter().map(|(cls, pts)| view! {
                        <polyline class=cls.clone() points=pts.clone() fill="none"/>
                    }).collect_view()}
                    {fig2_cursor}
                </svg>
                <p class="lab mono">{format!(
                    "{} · {pct}% BHP{}",
                    c.mt_fig3_title,
                    bracket_caption(loc, &fig3_brackets, c.mt_pct_bhp_weight_lbl),
                )}</p>
                <svg class="plot-svg strip-svg mt-fig-svg" viewBox=format!("0 0 {vb_w} {vb_h}")>
                    <rect class="chart-panel" x="0" y="0" width=vb_w height=vb_h/>
                    <polyline class="chart-stroke-line" points=fig3_ft_line fill="none"/>
                    {fig3_lines.iter().map(|(cls, pts)| view! {
                        <polyline class=cls.clone() points=pts.clone() fill="none"/>
                    }).collect_view()}
                    {if chart_ok {
                        let da = da.expect("chart_ok");
                        let tas = cursor_tas.expect("chart_ok");
                        let (cx2, cy_tas) =
                            map_da_y(da, tas, da_min, da_max, tas_min, tas_max);
                        view! { <circle class="chart-marker" cx=cx2 cy=cy_tas r="4"/> }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
                </svg>
                <p class="cursor-readout readout-text">{cursor_readout}</p>
                <MtFig4TableView locale=locale data=data inputs=inputs/>
            }
        }}
    }
}

#[component]
pub fn MtFig4TableView(
    locale: RwSignal<Locale>,
    data: RwSignal<da20_core::PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    let mt_fig4_source = Memo::new(move |_| {
        compute_all(&data.get(), &inputs.get())
            .cruise
            .and_then(|c| c.mt_fig4_source)
    });
    view! {
        <div class="table-scroll">
            <table class="data-table cruise-table-full">
                <thead>
                    <tr>
                        <th>{move || copy(locale.get()).mt_fig4_col_da}</th>
                        <th>{move || copy(locale.get()).mt_fig4_col_rpm}</th>
                        <th>{move || copy(locale.get()).cruise_tbl_pct_bhp}</th>
                        <th>{move || copy(locale.get()).cruise_tbl_ktas}</th>
                        <th>{move || copy(locale.get()).cruise_tbl_gph}</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        let mt = Mt175rData::production();
                        let rows = mt.fig4_rows();
                        let trace = mt_fig4_source.get();
                        rows.iter().filter(|r| r.len() >= 5).map(|r| {
                            let da = r[0];
                            let rpm = r[1];
                            let row_band = mt_fig4_table_row_highlight(trace.as_ref(), da, rpm);
                            let cell = if mt_fig4_table_cell_highlight(trace.as_ref(), da, rpm) {
                                "mono cell-used-emphasis"
                            } else {
                                "mono"
                            };
                            view! {
                                <tr class:row-source-band=row_band>
                                    <td class=cell>{da as i64}</td>
                                    <td class=cell>{rpm as i64}</td>
                                    <td class=cell>{format!("{:.0}", r[2])}</td>
                                    <td class=cell>{format!("{:.0}", r[3])}</td>
                                    <td class=cell>{format!("{:.1}", r[4])}</td>
                                </tr>
                            }
                        }).collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}
