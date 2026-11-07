//! Per-phase interactive chart (numeric entry + drag on same SVG).

use da20_core::altitude::{effective_pressure_altitude_ft, set_pa_override};
use da20_core::climb::compute_climb;
use da20_core::cruise::cruise_cell_tag_for_column;
use da20_core::landing::compute_landing;
use da20_core::rev29::charts::{
    equivalent_altitude_curves, landing_table_curves, map_to_svg, sample_landing_ft,
    sample_roc_fpm_with_inputs, sample_takeoff_base_m, takeoff_base_curves, wind_headwind_vs_angle,
    Polyline,
};
use da20_core::rev29::ea_inverse::pressure_altitude_for_equivalent_altitude;
use da20_core::rev29::engine::{equivalent_altitude_ft, lb_to_kg};
use da20_core::rev29::hand_plot::climb_fan_pa_curve_chart_points;
use da20_core::takeoff::compute_takeoff;
use da20_core::types::{AfmLookupTag, ChartPhase, FlightInputs};
use da20_core::wind::{compute_wind, runway_heading_deg};
use da20_core::PerformanceData;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::chart_axes::{interactive_chart_axis_ticks, interactive_chart_major_grid};
use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::chart_inputs::round_after_chart_drag;
use crate::i18n::{
    afm_lookup_tag_label, copy, format_climb_readout_interactive_pair,
    format_takeoff_strip_pa_trace_label, Locale,
};
use crate::inputs::set_chart_pa;

fn chart_phase(kind: ChartKind) -> Option<ChartPhase> {
    match kind {
        ChartKind::Takeoff => Some(ChartPhase::Takeoff),
        ChartKind::Climb => Some(ChartPhase::Climb),
        ChartKind::Landing => Some(ChartPhase::Landing),
        ChartKind::Wind => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ChartKind {
    Takeoff,
    Climb,
    Wind,
    Landing,
}

#[component]
pub fn InteractiveChart(
    locale: RwSignal<Locale>,
    kind: ChartKind,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    let dragging = RwSignal::new(false);

    let sync_cursor_from_inputs = move || {
        let d = data.get();
        let inner = d.inner();
        let i = inputs.get();
        let pa = effective_pressure_altitude_ft(&i, chart_phase(kind));
        match kind {
            ChartKind::Takeoff => {
                let to_t = &inner.to_t.value;
                let to_pa = &inner.to_pa.value;
                (norm_axis(to_t, i.oat_c), norm_axis(to_pa, pa))
            }
            ChartKind::Climb => {
                let fan_t = &inner.fan_t.value;
                let (ea, _) = equivalent_altitude_ft(inner, pa, i.oat_c);
                (norm_axis(fan_t, i.oat_c), norm_axis_ea(ea))
            }
            ChartKind::Landing => {
                let alts = &inner.land_alt.value;
                let (_, dist) = sample_landing_ft(inner, pa);
                let y_min = 600.0;
                let y_max = 1600.0;
                let ny = ((dist - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
                (norm_axis(alts, pa), ny)
            }
            ChartKind::Wind => {
                let angle = wind_angle_from_inputs(&i);
                (angle / 90.0, 0.5)
            }
        }
    };

    let cursor_x = RwSignal::new(0.5f64);
    let cursor_y = RwSignal::new(0.5f64);

    Effect::new(move |_| {
        let (nx, ny) = sync_cursor_from_inputs();
        cursor_x.set(nx);
        cursor_y.set(ny);
    });

    let on_pointer_down = move |ev: web_sys::PointerEvent| {
        dragging.set(true);
        update_cursor(&ev, cursor_x, cursor_y);
        apply_drag(kind, cursor_x.get(), cursor_y.get(), data.get(), inputs);
        if let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
    };
    let on_pointer_up = move |ev: web_sys::PointerEvent| {
        if dragging.get() {
            inputs.update(round_after_chart_drag);
        }
        dragging.set(false);
        if let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = el.release_pointer_capture(ev.pointer_id());
        }
    };
    let on_pointer_move = move |ev: web_sys::PointerEvent| {
        if !dragging.get() {
            return;
        }
        update_cursor(&ev, cursor_x, cursor_y);
        apply_drag(kind, cursor_x.get(), cursor_y.get(), data.get(), inputs);
    };

    view! {
        <div class="chart-wrap">
            <p class="hint">{move || copy(locale.get()).plot_hint}</p>
            <svg
                class="plot-svg"
                viewBox="0 0 400 300"
                on:pointerdown=on_pointer_down
                on:pointerup=on_pointer_up
                on:pointerleave=move |_| {
                    if dragging.get() {
                        inputs.update(round_after_chart_drag);
                    }
                    dragging.set(false);
                }
                on:pointermove=on_pointer_move
            >
                {move || {
                    let d = data.get();
                    let i = inputs.get();
                    render_svg(kind, locale.get(), &d, &i, cursor_x.get(), cursor_y.get())
                }}
            </svg>
            <p class="cursor-readout mono">
                {move || {
                    let c = copy(locale.get());
                    let d = data.get();
                    let i = inputs.get();
                    let (chart, core) = readout_pair(locale.get(), kind, &d, &i);
                    format!(
                        "{}: {chart} | {}: {core}",
                        c.plot_readout_chart,
                        c.plot_readout_core,
                    )
                }}
            </p>
        </div>
    }
}

fn wind_angle_from_inputs(i: &FlightInputs) -> f64 {
    let rwy = runway_heading_deg(&i.runway).unwrap_or(0.0);
    (i.wind_dir_deg - rwy).rem_euclid(360.0).min(90.0)
}

fn norm_axis(axis: &[f64], value: f64) -> f64 {
    if axis.len() < 2 {
        return 0.5;
    }
    let min = axis[0];
    let max = axis[axis.len() - 1];
    if (max - min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn norm_axis_ea(ea_ft: f64) -> f64 {
    const EA_MIN: f64 = 0.0;
    const EA_MAX: f64 = 14_000.0;
    ((ea_ft - EA_MIN) / (EA_MAX - EA_MIN)).clamp(0.0, 1.0)
}

fn render_svg(
    kind: ChartKind,
    locale: Locale,
    data: &PerformanceData,
    inputs: &FlightInputs,
    cx: f64,
    cy: f64,
) -> impl IntoView {
    let inner = data.inner();
    let (x_label, y_label) = chart_axis_labels(locale, kind);
    let (polylines, x_min, x_max, y_min, y_max, highlight_idx) = chart_spec(kind, inner, inputs);

    let phase = chart_phase(kind);
    let pa = effective_pressure_altitude_ft(inputs, phase);
    let oat_line_x = cx;
    let oat_x_data = x_min + oat_line_x * (x_max - x_min);
    let (oat_sx, _) = map_to_svg(
        oat_x_data, y_min, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0,
    );

    let climb_pa_guide: Option<(String, String)> = if kind == ChartKind::Climb {
        let raw = climb_fan_pa_curve_chart_points(inner, pa);
        if raw.is_empty() {
            None
        } else {
            let pts: String = raw
                .iter()
                .map(|(t, ea)| {
                    let (sx, sy) = map_to_svg(
                        *t, *ea, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0,
                    );
                    format!("{sx:.1},{sy:.1}")
                })
                .collect::<Vec<_>>()
                .join(" ");
            Some((pts, format_takeoff_strip_pa_trace_label(locale, pa)))
        }
    } else {
        None
    };

    let paths = polylines.into_iter().enumerate().map(|(idx, pl)| {
        let class = if Some(idx) == highlight_idx {
            "chart-stroke-line-hi"
        } else {
            "chart-stroke-line"
        };
        let pts: String = pl
            .points
            .iter()
            .map(|(x, y)| {
                let (sx, sy) =
                    map_to_svg(*x, *y, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0);
                format!("{sx:.1},{sy:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let label = polyline_guide_label(kind, inner, idx, &pl, x_min, x_max, y_min, y_max);
        view! {
            <>
                <polyline points=pts class=class/>
                {label}
            </>
        }
    });

    let (gx, gy) = {
        let x = x_min + cx * (x_max - x_min);
        let y = y_min + cy * (y_max - y_min);
        map_to_svg(x, y, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0)
    };

    let ea_at_cursor = y_min + cy * (y_max - y_min);
    let (ea_gx1, ea_gy) = map_to_svg(
        x_min,
        ea_at_cursor,
        x_min,
        x_max,
        y_min,
        y_max,
        40.0,
        20.0,
        340.0,
        250.0,
    );
    let (ea_gx2, _) = map_to_svg(
        x_max,
        ea_at_cursor,
        x_min,
        x_max,
        y_min,
        y_max,
        40.0,
        20.0,
        340.0,
        250.0,
    );

    view! {
        <rect x="40" y="20" width="340" height="250" class="chart-panel"/>
        {interactive_chart_major_grid(kind, x_min, x_max, y_min, y_max)}
        {paths.collect_view()}
        {match kind {
            ChartKind::Climb => view! {
                <line
                    x1=ea_gx1
                    y1=ea_gy
                    x2=ea_gx2
                    y2=ea_gy
                    class="chart-stroke-guide"
                    stroke-dasharray="3 4"
                    opacity="0.92"
                />
            }.into_any(),
            _ => view! { <g/> }.into_any(),
        }}
        {climb_pa_guide.as_ref().map(|(pts, _)| view! {
            <polyline points=pts.clone() fill="none" class="chart-stroke-guide-active" stroke-dasharray="5 4"/>
        })}
        {(kind != ChartKind::Climb).then(|| view! {
            <line x1=oat_sx y1="20" x2=oat_sx y2="270" class="chart-stroke-guide-active" stroke-dasharray="5 4"/>
        })}
        {climb_pa_guide.map(|(_, lbl)| view! {
            <>
                <line x1=gx y1="20" x2=gx y2="270" class="chart-stroke-guide-active" stroke-dasharray="5 4"/>
                {guide_line_label(gx + 4.0, gy, lbl, LabelAnchor::Start)}
            </>
        })}
        <circle cx=gx cy=gy r="6" class="chart-handle"/>
        <text x="200" y="292" class="chart-axis-title" text-anchor="middle">{x_label}</text>
        <text x="44" y="16" class="chart-axis-title" text-anchor="start">{y_label}</text>
        <text x="50" y="288" fill="var(--chart-label)" font-size="10">
            {match kind {
                ChartKind::Takeoff => {
                    format!("OAT {:.0} °C · PA {:.0} ft", inputs.oat_c, pa)
                }
                ChartKind::Climb => {
                    format!("OAT {:.0} °C", inputs.oat_c)
                }
                _ => String::new(),
            }}
        </text>
        {takeoff_overlays(kind, data, inner, inputs)}
        {interactive_chart_axis_ticks(kind, x_min, x_max, y_min, y_max)}
    }
}

#[allow(clippy::too_many_arguments)]
fn polyline_guide_label(
    kind: ChartKind,
    inner: &da20_core::rev29::types::Rev29File,
    idx: usize,
    pl: &Polyline,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> impl IntoView {
    match kind {
        ChartKind::Climb => view! { <g></g> }.into_any(),
        _ => {
            let (x, y, text) = match kind {
                ChartKind::Takeoff => {
                    let pa = inner.to_pa.value.get(idx).copied().unwrap_or(0.0);
                    let (x, y) = pl.points.last().copied().unwrap_or((x_max, y_min));
                    (x, y, format!("{pa:.0} ft"))
                }
                ChartKind::Landing => {
                    let text = if idx == 0 {
                        "Roll".to_string()
                    } else {
                        "50 ft".to_string()
                    };
                    let (x, y) = pl.points.last().copied().unwrap_or((x_max, y_min));
                    (x, y, text)
                }
                ChartKind::Wind => {
                    let (x, y) = pl.points.last().copied().unwrap_or((x_max, y_min));
                    (x, y, format!("{y:.0} kt"))
                }
                ChartKind::Climb => unreachable!(),
            };
            let (sx, sy) = map_to_svg(x, y, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0);
            guide_line_label(sx + 4.0, sy, text, LabelAnchor::Start).into_any()
        }
    }
}

fn takeoff_overlays(
    kind: ChartKind,
    data: &PerformanceData,
    inner: &da20_core::rev29::types::Rev29File,
    inputs: &FlightInputs,
) -> impl IntoView {
    if kind != ChartKind::Takeoff {
        return view! { <g></g> }.into_any();
    }
    let kg = lb_to_kg(inputs.weight_lb);
    let hw = compute_wind(data, inputs)
        .map(|w| w.components.headwind_kt)
        .unwrap_or(0.0);
    let wt_n = norm_axis(&inner.wt_kg.value, kg);
    let wt_x = 40.0 + wt_n * 340.0;
    view! {
        <line x1=wt_x y1="20" x2=wt_x y2="270" class="chart-stroke-guide-active" stroke-dasharray="3 4" opacity="0.85"/>
        <text x="250" y="36" class="chart-axis-title" text-anchor="end">
            {format!("{kg:.0} kg · {hw:.1} kt")}
        </text>
    }
    .into_any()
}

fn readout_pair(
    locale: Locale,
    kind: ChartKind,
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> (String, String) {
    let inner = data.inner();
    let phase = chart_phase(kind);
    let pa = effective_pressure_altitude_ft(inputs, phase);
    let chart = match kind {
        ChartKind::Takeoff => format!(
            "{:.0} m base",
            sample_takeoff_base_m(inner, pa, inputs.oat_c)
        ),
        ChartKind::Climb => {
            let (ea, _) = equivalent_altitude_ft(inner, pa, inputs.oat_c);
            let roc_u =
                sample_roc_fpm_with_inputs(inner, inputs.climb_figure.roc_key(), pa, inputs);
            let (chart_line, _) = format_climb_readout_interactive_pair(
                locale,
                roc_u,
                Some(roc_u),
                inputs.wheel_fairings_installed,
                AfmLookupTag::Interpolated,
            );
            format!("{:.0} ft EA · {chart_line}", ea.round())
        }
        ChartKind::Landing => {
            let (r, d) = da20_core::rev29::charts::sample_landing_ft(inner, pa);
            format!("{:.0} / {:.0} ft", r, d)
        }
        ChartKind::Wind => format!("{:.1} kt HW", {
            compute_wind(data, inputs)
                .map(|w| w.components.headwind_kt)
                .unwrap_or(0.0)
        }),
    };
    let core = match kind {
        ChartKind::Takeoff => compute_takeoff(data, inputs)
            .map(|t| format!("{:.0} ft roll", t.ground_roll_ft))
            .unwrap_or_else(|_| "—".into()),
        ChartKind::Climb => compute_climb(data, inputs, inputs.cruise_pressure_alt_ft)
            .map(|cl| {
                let (_, core) = format_climb_readout_interactive_pair(
                    locale,
                    cl.rate_of_climb_fpm,
                    cl.rate_of_climb_chart_fpm,
                    inputs.wheel_fairings_installed,
                    cl.afm_lookup_tag,
                );
                core
            })
            .unwrap_or_else(|_| "—".into()),
        ChartKind::Landing => compute_landing(data, inputs)
            .map(|l| format!("{:.0} ft", l.distance_over_50ft_ft))
            .unwrap_or_else(|_| "—".into()),
        ChartKind::Wind => compute_wind(data, inputs)
            .map(|w| {
                format!(
                    "{:.1} / {:.1} kt",
                    w.components.headwind_kt, w.components.crosswind_kt
                )
            })
            .unwrap_or_else(|_| "—".into()),
    };
    (chart, core)
}

fn chart_axis_labels(locale: Locale, kind: ChartKind) -> (&'static str, &'static str) {
    let c = copy(locale);
    match kind {
        ChartKind::Takeoff => (c.axis_oat_c, c.axis_dist_m),
        ChartKind::Climb => (c.axis_oat_c, c.strip_climb_roc_ea_y_axis),
        ChartKind::Landing => (c.axis_pa_ft, c.axis_dist_ft),
        ChartKind::Wind => (c.axis_angle_deg, c.axis_hw_kt),
    }
}

fn chart_spec(
    kind: ChartKind,
    inner: &da20_core::rev29::types::Rev29File,
    inputs: &FlightInputs,
) -> (Vec<Polyline>, f64, f64, f64, f64, Option<usize>) {
    let phase = chart_phase(kind);
    let pa = effective_pressure_altitude_ft(inputs, phase);
    match kind {
        ChartKind::Takeoff => {
            let pl = takeoff_base_curves(inner);
            let hi = nearest_pa_index(&inner.to_pa.value, pa);
            (pl, -20.0, 55.0, 150.0, 1000.0, hi)
        }
        ChartKind::Climb => {
            let pl = equivalent_altitude_curves(inner);
            let hi = nearest_pa_index(&inner.fan_pa.value, pa);
            (pl, -20.0, 55.0, 0.0, 14_000.0, hi)
        }
        ChartKind::Landing => {
            let (roll, dist) = landing_table_curves(inner);
            (vec![roll, dist], 0.0, 7000.0, 600.0, 1600.0, None)
        }
        ChartKind::Wind => (
            vec![wind_headwind_vs_angle(inputs.wind_speed_kt.max(1.0))],
            0.0,
            90.0,
            0.0,
            inputs.wind_speed_kt.max(12.0),
            None,
        ),
    }
}

fn nearest_pa_index(axis: &[f64], pa: f64) -> Option<usize> {
    axis.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (*a - pa)
                .abs()
                .partial_cmp(&(*b - pa).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
}

fn apply_drag(
    kind: ChartKind,
    nx: f64,
    ny: f64,
    data: PerformanceData,
    inputs: RwSignal<FlightInputs>,
) {
    let inner = data.inner();
    match kind {
        ChartKind::Takeoff => {
            let pa = lerp_axis(&inner.to_pa.value, ny);
            inputs.update(|i| {
                i.oat_c = lerp_axis(&inner.to_t.value, nx);
                set_pa_override(i, pa, ChartPhase::Takeoff);
            });
        }
        ChartKind::Climb => {
            let oat_c = lerp_axis(&inner.fan_t.value, nx);
            let ea = ny * 14_000.0;
            let pa = pressure_altitude_for_equivalent_altitude(inner, ea, oat_c);
            inputs.update(|i| {
                i.oat_c = oat_c;
                set_pa_override(i, pa, ChartPhase::Climb);
            });
        }
        ChartKind::Landing => {
            let pa = lerp_axis(&inner.land_alt.value, nx);
            set_chart_pa(inputs, pa, ChartPhase::Landing);
        }
        ChartKind::Wind => {
            let angle = nx * 90.0;
            let rwy = runway_heading_deg(&inputs.get().runway).unwrap_or(0.0);
            inputs.update(|i| {
                i.wind_dir_deg = (rwy + angle).rem_euclid(360.0);
            });
        }
    }
}

fn lerp_axis(axis: &[f64], n: f64) -> f64 {
    if axis.is_empty() {
        return 0.0;
    }
    if axis.len() == 1 {
        return axis[0];
    }
    let f = n.clamp(0.0, 1.0) * (axis.len() - 1) as f64;
    let i = f.floor() as usize;
    let j = (i + 1).min(axis.len() - 1);
    let t = f - i as f64;
    axis[i] + t * (axis[j] - axis[i])
}

fn update_cursor(ev: &web_sys::PointerEvent, cursor_x: RwSignal<f64>, cursor_y: RwSignal<f64>) {
    if let Some(target) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let rect = target.get_bounding_client_rect();
        let nx = ((ev.client_x() as f64 - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let ny = (1.0 - ((ev.client_y() as f64 - rect.top()) / rect.height())).clamp(0.0, 1.0);
        cursor_x.set(((nx * 340.0) / 340.0).clamp(0.0, 1.0));
        cursor_y.set(ny);
    }
}

#[component]
pub fn CruiseTableView(
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        <div class="table-scroll">
            <table class="data-table cruise-table-full">
                <thead>
                    <tr>
                        <th rowspan=2>"PA ft"</th>
                        <th rowspan=2>"RPM"</th>
                        <th colspan=3>{move || copy(locale.get()).cruise_tbl_group_disa_m20}</th>
                        <th colspan=3>{move || copy(locale.get()).cruise_tbl_group_isa}</th>
                        <th colspan=3>{move || copy(locale.get()).cruise_tbl_group_disa_p10}</th>
                    </tr>
                    <tr>
                        {move || {
                            let c = copy(locale.get());
                            [c.cruise_tbl_pct_bhp, c.cruise_tbl_ktas, c.cruise_tbl_gph].into_iter().cycle().take(9).map(|h| view! { <th>{h}</th> }).collect_view()
                        }}
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        let loc = locale.get();
                        let d = data.get();
                        let rows = &d.inner().cruise_table.value.rows;
                        let i = inputs.get();
                        rows.iter().filter(|r| r.len() >= 11).map(|r| {
                            let pa = r[0];
                            let rpm = r[1];
                            let highlight = (pa - i.cruise_pressure_alt_ft).abs() < 500.0
                                && (rpm - i.cruise_rpm).abs() < 0.5;
                            let tag_m20 = afm_lookup_tag_label(loc, cruise_cell_tag_for_column(pa, rpm, -20.0));
                            let tag_isa = afm_lookup_tag_label(loc, cruise_cell_tag_for_column(pa, rpm, 0.0));
                            let tag_p10 = afm_lookup_tag_label(loc, cruise_cell_tag_for_column(pa, rpm, 10.0));
                            view! {
                                <tr class:row-hi=highlight>
                                    <td>{pa as i64}</td>
                                    <td>{rpm as i64}</td>
                                    <td class="mono">{format!("{:.0}", r[2])}<br/><span class="cell-tag">{tag_m20}</span></td>
                                    <td class="mono">{format!("{:.0}", r[3])}</td>
                                    <td class="mono">{format!("{:.1}", r[4])}</td>
                                    <td class="mono">{format!("{:.0}", r[5])}<br/><span class="cell-tag">{tag_isa}</span></td>
                                    <td class="mono">{format!("{:.0}", r[6])}</td>
                                    <td class="mono">{format!("{:.1}", r[7])}</td>
                                    <td class="mono">{format!("{:.0}", r[8])}<br/><span class="cell-tag">{tag_p10}</span></td>
                                    <td class="mono">{format!("{:.0}", r[9])}</td>
                                    <td class="mono">{format!("{:.1}", r[10])}</td>
                                </tr>
                            }
                        }).collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}

#[component]
pub fn LandingTableView(
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        <div class="table-scroll">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"PA ft"</th>
                        <th>"Roll ft"</th>
                        <th>"50 ft ft"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        let d = data.get();
                        let inner = d.inner();
                        let pa = effective_pressure_altitude_ft(&inputs.get(), Some(ChartPhase::Landing));
                        inner.land_alt.value.iter().enumerate().map(|(idx, alt)| {
                            let highlight = (*alt - pa).abs()
                                < 500.0;
                            view! {
                                <tr class:row-hi=highlight>
                                    <td>{*alt as i64}</td>
                                    <td>{inner.land_roll.value[idx] as i64}</td>
                                    <td>{inner.land_50ft.value[idx] as i64}</td>
                                </tr>
                            }
                        }).collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}
