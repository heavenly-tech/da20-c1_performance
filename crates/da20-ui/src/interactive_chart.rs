//! Per-phase interactive chart (numeric entry + drag on same SVG).

use da20_core::altitude::effective_pressure_altitude_ft;
use da20_core::rev29::charts::{
    climb_roc_vs_ea, landing_table_curves, map_to_svg, takeoff_base_curves,
    wind_headwind_vs_angle, Polyline,
};
use da20_core::types::FlightInputs;
use da20_core::PerformanceData;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::{copy, Locale};

#[derive(Clone, Copy, PartialEq, Eq)]
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
        let pa = effective_pressure_altitude_ft(&i);
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
                (norm_axis(alts, pa), 0.5)
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
                    render_svg(kind, &d, &i, cursor_x.get(), cursor_y.get())
                }}
            </svg>
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
    data: &PerformanceData,
    inputs: &FlightInputs,
    cx: f64,
    cy: f64,
) -> impl IntoView {
    let inner = data.inner();
    let (polylines, x_label, y_label, x_min, x_max, y_min, y_max, highlight_idx) =
        chart_spec(kind, inner, inputs);

    let pa = effective_pressure_altitude_ft(inputs);
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
            {format!("OAT {:.0} °C · PA {:.0} ft", inputs.oat_c, pa)}
        </text>
    }
}

fn chart_spec(
    kind: ChartKind,
    inner: &da20_core::rev29::types::Rev29File,
    inputs: &FlightInputs,
) -> (
    Vec<Polyline>,
    &'static str,
    &'static str,
    f64,
    f64,
    f64,
    f64,
    Option<usize>,
) {
    let pa = effective_pressure_altitude_ft(inputs);
    match kind {
        ChartKind::Takeoff => {
            let pl = takeoff_base_curves(inner);
            let hi = nearest_pa_index(&inner.to_pa.value, pa);
            (pl, "OAT °C", "Base roll m", -20.0, 55.0, 150.0, 1000.0, hi)
        }
        ChartKind::Climb => (
            vec![climb_roc_vs_ea(inner, inputs.climb_figure.roc_key())],
            "EA ft",
            "ROC fpm",
            0.0,
            14_000.0,
            0.0,
            1000.0,
            None,
        ),
        ChartKind::Landing => {
            let (roll, dist) = landing_table_curves(inner);
            (
                vec![roll, dist],
                "PA ft",
                "ft",
                0.0,
                7000.0,
                600.0,
                1600.0,
                None,
            )
        }
        ChartKind::Wind => (
            vec![wind_headwind_vs_angle(inputs.wind_speed_kt.max(1.0))],
            "Angle °",
            "HW kt",
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
            inputs.update(|i| {
                i.oat_c = lerp_axis(&inner.to_t.value, nx);
                i.direct_pressure_altitude_ft = Some(lerp_axis(&inner.to_pa.value, ny));
            });
        }
        ChartKind::Climb => {
            inputs.update(|i| {
                i.oat_c = lerp_axis(&inner.fan_t.value, nx);
                i.direct_pressure_altitude_ft = Some(lerp_axis(&inner.fan_pa.value, ny));
            });
        }
        ChartKind::Landing => {
            inputs.update(|i| {
                i.direct_pressure_altitude_ft = Some(lerp_axis(&inner.land_alt.value, nx));
            });
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
                        let pa = effective_pressure_altitude_ft(&inputs.get());
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
