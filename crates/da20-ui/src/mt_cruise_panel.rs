//! MT AFMS Fig 2/3 reference charts and Fig 4 table for cruise phase.

use da20_core::mt175r::charts::{
    fig3_full_throttle_chart_points, lookup_fig2_rpm_at_pct, lookup_fig3_tas_at_pct,
};
use da20_core::mt175r::Mt175rData;
use da20_core::types::FlightInputs;
use leptos::prelude::*;

use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::i18n::{copy, format_mt_cruise_interp_caption, Locale};
use crate::mt_cruise_axes::{
    mt_chart_axes, mt_fig2_axis_spec, mt_fig3_axis_spec, viewbox_h, viewbox_w, MtAxisSpec,
    RPM_TICKS, TAS_TICKS_KT,
};
use crate::mt_cruise_chart::{
    apply_mt_fig2_pct_drag, fig2_curve_end_label_positions, fig2_family_polylines,
    fig2_marker_guides, fig2_selected_polyline, fig3_curve_end_label_positions,
    fig3_family_polylines, fig3_marker_guides, fig3_selected_polyline, filter_plot_points,
    format_pct_bhp_label, map_mt_chart_xy, mt_chart_da_on_fig2_axes, mt_cruise_density_altitude_ft,
    selected_pct_bhp,
};
use crate::phase_state::persist_tab_inputs;
use crate::phases::Phase;
use crate::prefs::FlightPrefs;
use crate::storage::compute_all;
use crate::strip_plot::{
    capture_pointer, format_svg_polyline, release_pointer, svg_pointer_coords,
};
use crate::table_source_panel::{mt_fig4_table_cell_highlight, mt_fig4_table_row_highlight};

fn polyline_svg(points: &[(f64, f64)], spec: MtAxisSpec) -> String {
    let plot_pts = filter_plot_points(points, spec.da_min, spec.da_max, spec.y_min, spec.y_max);
    let svg_pts: Vec<(f64, f64)> = plot_pts
        .iter()
        .map(|(da, y)| map_mt_chart_xy(*da, *y, spec.da_min, spec.da_max, spec.y_min, spec.y_max))
        .collect();
    format_svg_polyline(&svg_pts)
}

fn render_curve_end_labels(labels: Vec<(String, f64, f64, bool)>) -> impl IntoView {
    view! {
        {labels.into_iter().map(|(text, x, y, highlight)| {
            if highlight {
                view! {
                    <text
                        x=x
                        y=y
                        class="chart-guide-label-hi"
                        font-size="9"
                        text-anchor="start"
                        dominant-baseline="middle"
                    >
                        {text}
                    </text>
                }
                .into_any()
            } else {
                view! { {guide_line_label(x, y, text, LabelAnchor::Start)} }.into_any()
            }
        }).collect_view()}
    }
}

fn render_marker_guides(g: &crate::mt_cruise_chart::MtChartMarkerGuides) -> impl IntoView {
    view! {
        {g.draw_guides.then(|| view! {
            <>
                <line
                    x1=g.cx
                    y1=g.cy
                    x2=g.cx
                    y2=g.x_axis_y
                    class="chart-stroke-guide-active"
                    stroke-dasharray="4 4"
                />
                <line
                    x1=g.cx
                    y1=g.cy
                    x2=g.y_axis_x
                    y2=g.cy
                    class="chart-stroke-guide-active"
                    stroke-dasharray="4 4"
                />
                {guide_line_label(
                    g.x_label_x,
                    g.x_label_y,
                    g.x_label.clone(),
                    LabelAnchor::Middle,
                )}
                {guide_line_label(
                    g.y_label_x,
                    g.y_label_y,
                    g.y_label.clone(),
                    LabelAnchor::Start,
                )}
                {g.on_chart.then(|| view! {
                    <circle class="chart-marker" cx=g.cx cy=g.cy r="5"/>
                })}
            </>
        })}
        {g.notice.as_ref().map(|notice| view! {
            <text
                x=g.cx
                y=g.cy
                class="chart-guide-label-hi"
                font-size="11"
                text-anchor="middle"
                dominant-baseline="middle"
            >
                {notice.clone()}
            </text>
        })}
    }
}

#[component]
pub fn MtCruisePanel(
    locale: RwSignal<Locale>,
    data: RwSignal<da20_core::PerformanceData>,
    inputs: RwSignal<FlightInputs>,
    #[prop(optional)] flight_prefs: Option<RwSignal<FlightPrefs>>,
) -> impl IntoView {
    let fig2_dragging = RwSignal::new(false);
    let fig2_spec = mt_fig2_axis_spec();
    let fig3_spec = mt_fig3_axis_spec();
    let vb_w = viewbox_w();
    let vb_h = viewbox_h();

    view! {
        <p class="lab plot-hint">{move || copy(locale.get()).plot_hint}</p>
        {move || {
            let mt = Mt175rData::production();
            let loc = locale.get();
            let i = inputs.get();
            let b = compute_all(&data.get(), &i);
            let cr = b.cruise.as_ref();
            let da = mt_cruise_density_altitude_ft(&i);
            let selected_pct = selected_pct_bhp(&i, cr.and_then(|c| c.percent_bhp));
            let fig2_brackets = cr
                .and_then(|c| c.mt_fig2_source.as_ref())
                .map(|f2| {
                    f2.curve_brackets
                        .iter()
                        .filter(|b| b.weight > 1e-6)
                        .map(|b| (b.pct_bhp, b.weight))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| vec![(selected_pct.round() as i32, 1.0)]);

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

            let fig2_rpm_marker = if cr.is_some() {
                cr.and_then(|c| c.rpm)
            } else {
                lookup_fig2_rpm_at_pct(&mt, da, selected_pct).map(|h| h.rpm)
            };
            let fig2_ooc = !mt_chart_da_on_fig2_axes(da) || fig2_rpm_marker.is_none();
            let fig2_guides = fig2_marker_guides(loc, da, fig2_rpm_marker, fig2_ooc);

            let fig3_tas_marker = cr.and_then(|c| c.tas_chart_kt).or_else(|| {
                lookup_fig3_tas_at_pct(&mt, da, selected_pct).map(|h| h.tas_kt)
            });
            let fig3_ooc =
                !mt_chart_da_on_fig2_axes(da) || fig3_tas_marker.is_none();
            let fig3_guides =
                fig3_marker_guides(loc, da, fig3_tas_marker, selected_pct, fig3_ooc);

            let family = fig2_family_polylines(&mt);
            let selected_pts = fig2_selected_polyline(&mt, selected_pct);
            let selected_label = format_pct_bhp_label(selected_pct);

            let fig3_family = fig3_family_polylines(&mt);
            let fig3_selected_pts = fig3_selected_polyline(&mt, selected_pct);
            let fig3_ft_line = polyline_svg(&fig3_full_throttle_chart_points(&mt), fig3_spec);
            let skip_printed_end_label = fig2_brackets.len() <= 1
                && (selected_pct - selected_pct.round() as i32 as f64).abs() < 0.01;
            let fig2_end_labels =
                fig2_curve_end_label_positions(&mt, selected_pct, skip_printed_end_label);
            let fig3_end_labels =
                fig3_curve_end_label_positions(&mt, selected_pct, skip_printed_end_label);

            let c = copy(loc);
            let da_axis_title = c.mt_da_axis_title.to_string();

            let on_fig2_down = move |ev: web_sys::PointerEvent| {
                fig2_dragging.set(true);
                if let Some((_sx, sy)) = svg_pointer_coords(&ev) {
                    let mt = Mt175rData::production();
                    inputs.update(|inp| {
                        apply_mt_fig2_pct_drag(sy, &mt, inp);
                    });
                }
                capture_pointer(&ev);
            };
            let on_fig2_up = move |ev: web_sys::PointerEvent| {
                if fig2_dragging.get() {
                    if let Some(fp) = flight_prefs {
                        fp.update(|p| {
                            persist_tab_inputs(Phase::Cruise, p, &inputs.get_untracked());
                            p.save();
                        });
                    }
                }
                fig2_dragging.set(false);
                release_pointer(&ev);
            };
            let on_fig2_move = move |ev: web_sys::PointerEvent| {
                if !fig2_dragging.get() {
                    return;
                }
                if let Some((_sx, sy)) = svg_pointer_coords(&ev) {
                    let mt = Mt175rData::production();
                    inputs.update(|inp| {
                        apply_mt_fig2_pct_drag(sy, &mt, inp);
                    });
                }
            };

            view! {
                <p class="lab mono">{format!(
                    "{} · {selected_label}{}",
                    c.mt_fig2_title,
                    format_mt_cruise_interp_caption(loc, &fig2_brackets),
                )}</p>
                <svg
                    class="plot-svg strip-svg mt-fig-svg"
                    viewBox=format!("0 0 {vb_w} {vb_h}")
                    on:pointerdown=on_fig2_down
                    on:pointerup=on_fig2_up
                    on:pointerleave=move |_| {
                        if fig2_dragging.get() {
                            if let Some(fp) = flight_prefs {
                                fp.update(|p| {
                                    persist_tab_inputs(Phase::Cruise, p, &inputs.get_untracked());
                                    p.save();
                                });
                            }
                        }
                        fig2_dragging.set(false);
                    }
                    on:pointermove=on_fig2_move
                >
                    <rect class="chart-panel" x="0" y="0" width=vb_w height=vb_h/>
                    {mt_chart_axes(fig2_spec, &RPM_TICKS, da_axis_title.clone())}
                    {family.iter().map(|(_pct, pts)| {
                        let line = polyline_svg(pts, fig2_spec);
                        view! { <polyline class="chart-stroke-line" points=line fill="none"/> }
                    }).collect_view()}
                    {(!selected_pts.is_empty()).then(|| {
                        let line = polyline_svg(&selected_pts, fig2_spec);
                        view! { <polyline class="chart-stroke-line-hi" points=line fill="none"/> }
                    })}
                    {render_curve_end_labels(fig2_end_labels)}
                    {render_marker_guides(&fig2_guides)}
                </svg>
                <p class="lab mono">{format!(
                    "{} · {selected_label}{}",
                    c.mt_fig3_title,
                    format_mt_cruise_interp_caption(loc, &fig3_brackets),
                )}</p>
                <svg class="plot-svg strip-svg mt-fig-svg" viewBox=format!("0 0 {vb_w} {vb_h}")>
                    <rect class="chart-panel" x="0" y="0" width=vb_w height=vb_h/>
                    {mt_chart_axes(fig3_spec, &TAS_TICKS_KT, da_axis_title)}
                    <polyline class="chart-stroke-line" points=fig3_ft_line fill="none"/>
                    {fig3_family.iter().map(|(_pct, pts)| {
                        let line = polyline_svg(pts, fig3_spec);
                        view! { <polyline class="chart-stroke-line" points=line fill="none"/> }
                    }).collect_view()}
                    {(!fig3_selected_pts.is_empty()).then(|| {
                        let line = polyline_svg(&fig3_selected_pts, fig3_spec);
                        view! { <polyline class="chart-stroke-line-hi" points=line fill="none"/> }
                    })}
                    {render_curve_end_labels(fig3_end_labels)}
                    {render_marker_guides(&fig3_guides)}
                </svg>
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

#[cfg(test)]
mod panel_tests {
    use super::*;
    use crate::mt_cruise_chart::marker_guides_svg_snippet;

    #[test]
    fn fig2_guide_snippet_has_no_nan_for_valid_point() {
        let g = fig2_marker_guides(Locale::Es, 6600.0, Some(2451.0), false);
        let s = marker_guides_svg_snippet(&g);
        assert!(s.contains("DA 6600 ft"));
        assert!(s.contains("2451 RPM"));
        assert!(!s.contains("NaN"));
    }
}
