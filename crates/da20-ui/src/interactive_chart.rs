//! Per-phase interactive chart (numeric entry + drag on same SVG).

use da20_core::altitude::{effective_pressure_altitude_ft, set_pa_override};
use da20_core::climb::compute_climb;
use da20_core::landing::compute_landing;
use da20_core::rev29::charts::{
    climb_roc_vs_ea, landing_table_curves, map_to_svg, sample_landing_ft, sample_roc_fpm,
    sample_takeoff_base_m, takeoff_base_curves, wind_headwind_vs_angle, Polyline,
};
use da20_core::rev29::engine::lb_to_kg;
use da20_core::takeoff::compute_takeoff;
use da20_core::types::{ChartPhase, FlightInputs};
use da20_core::wind::compute_wind;
use da20_core::PerformanceData;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::{copy, Locale};
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
                let fan_pa = &inner.fan_pa.value;
                (norm_axis(fan_t, i.oat_c), norm_axis(fan_pa, pa))
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
                on:pointerleave=move |_| dragging.set(false)
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
                    let (chart, core) = readout_pair(kind, &d, &i);
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
    let rwy = i.runway.parse::<f64>().unwrap_or(0.0) * 10.0;
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

    let paths = polylines.into_iter().enumerate().map(|(idx, pl)| {
        let stroke = if Some(idx) == highlight_idx {
            "var(--chart-highlight)"
        } else {
            "var(--chart-line)"
        };
        let sw = if Some(idx) == highlight_idx {
            "2.4"
        } else {
            "1.6"
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
        view! {
            <polyline points=pts fill="none" stroke=stroke stroke-width=sw/>
        }
    });

    let (gx, gy) = {
        let x = x_min + cx * (x_max - x_min);
        let y = y_min + cy * (y_max - y_min);
        map_to_svg(x, y, x_min, x_max, y_min, y_max, 40.0, 20.0, 340.0, 250.0)
    };

    view! {
        <rect x="40" y="20" width="340" height="250" fill="var(--chart-bg)" stroke="var(--chart-border)"/>
        {paths.collect_view()}
        <line x1=oat_sx y1="20" x2=oat_sx y2="270" stroke="var(--chart-guide)" stroke-width="2" stroke-dasharray="5 4"/>
        <line x1=gx y1="20" x2=gx y2="270" stroke="var(--chart-grid)" stroke-dasharray="4 3"/>
        <line x1="40" y1=gy x2="380" y2=gy stroke="var(--chart-grid)" stroke-dasharray="4 3"/>
        <circle cx=gx cy=gy r="7" fill="var(--chart-guide)" stroke="var(--chart-bg)" stroke-width="2"/>
        <text x="50" y="36" fill="var(--chart-label)" font-size="11">{x_label}</text>
        <text x="330" y="285" fill="var(--chart-label)" font-size="11">{y_label}</text>
        <text x="50" y="288" fill="var(--chart-label)" font-size="10">
            {match kind {
                ChartKind::Takeoff | ChartKind::Climb => {
                    format!("OAT {:.0} °C · PA {:.0} ft", inputs.oat_c, pa)
                }
                _ => String::new(),
            }}
        </text>
        {takeoff_overlays(kind, data, inner, inputs)}
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
        <line x1=wt_x y1="20" x2=wt_x y2="270" stroke="var(--chart-guide)" stroke-width="1.5" stroke-dasharray="3 4" opacity="0.85"/>
        <text x="250" y="36" fill="var(--chart-guide)" font-size="10" text-anchor="end">
            {format!("Wt {kg:.0} kg · HW {hw:.1} kt")}
        </text>
    }
    .into_any()
}

fn readout_pair(
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
        ChartKind::Climb => format!(
            "{:.0} fpm",
            sample_roc_fpm(inner, inputs.climb_figure.roc_key(), pa, inputs.oat_c)
        ),
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
            .map(|c| format!("{:.0} fpm", c.rate_of_climb_fpm))
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
        ChartKind::Takeoff => (c.axis_oat_c, c.axis_base_roll_m),
        ChartKind::Climb => (c.axis_ea_ft, c.axis_roc_fpm),
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
        ChartKind::Climb => (
            vec![climb_roc_vs_ea(inner, inputs.climb_figure.roc_key())],
            0.0,
            14_000.0,
            0.0,
            1000.0,
            None,
        ),
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
            let pa = lerp_axis(&inner.fan_pa.value, ny);
            inputs.update(|i| {
                i.oat_c = lerp_axis(&inner.fan_t.value, nx);
                set_pa_override(i, pa, ChartPhase::Climb);
            });
        }
        ChartKind::Landing => {
            let pa = lerp_axis(&inner.land_alt.value, nx);
            set_chart_pa(inputs, pa, ChartPhase::Landing);
        }
        ChartKind::Wind => {
            let angle = nx * 90.0;
            let rwy = inputs.get().runway.parse::<f64>().unwrap_or(0.0) * 10.0;
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
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        <div class="table-scroll">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"PA ft"</th>
                        <th>"RPM"</th>
                        <th>"TAS"</th>
                        <th>"GPH"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        let d = data.get();
                        let rows = &d.inner().cruise_table.value.rows;
                        let i = inputs.get();
                        rows.iter().filter(|r| r.len() >= 11).map(|r| {
                            let pa = r[0];
                            let rpm = r[1];
                            let highlight = (pa - i.cruise_pressure_alt_ft).abs() < 500.0
                                && (rpm - i.cruise_rpm).abs() < 0.5;
                            view! {
                                <tr class:row-hi=highlight>
                                    <td>{pa as i64}</td>
                                    <td>{rpm as i64}</td>
                                    <td>{format!("{:.0}", r[6])}</td>
                                    <td>{format!("{:.1}", r[7])}</td>
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
