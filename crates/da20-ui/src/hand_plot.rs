//! Vector-redrawn performance charts for hand-plot mode (not AFM scans).

use da20_core::altitude::pressure_altitude_ft;
use da20_core::types::FlightInputs;
use da20_core::PerformanceData;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlotKind {
    TakeoffDistance,
    EquivalentAltitude,
}

#[component]
pub fn HandPlotPanel(
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    let kind = RwSignal::new(PlotKind::TakeoffDistance);
    let cursor_x = RwSignal::new(0.5f64);
    let cursor_y = RwSignal::new(0.5f64);
    let dragging = RwSignal::new(false);

    let on_move = move |ev: leptos::ev::MouseEvent| {
        if !dragging.get() {
            return;
        }
        update_cursor_from_event(ev, cursor_x, cursor_y);
        apply_cursor_to_inputs(kind.get(), cursor_x.get(), cursor_y.get(), data.get(), inputs);
    };

    view! {
        <div class="hand-plot">
            <p class="hint">
                "Drag the crosshair along reference-style guide lines. Charts are "
                <strong>"original vector drawings"</strong>
                " — not AFM reproductions."
            </p>
            <select prop:value=move || match kind.get() {
                PlotKind::TakeoffDistance => "takeoff",
                PlotKind::EquivalentAltitude => "eq",
            } on:change=move |ev| {
                let v = event_select_value(&ev);
                kind.set(if v == "eq" { PlotKind::EquivalentAltitude } else { PlotKind::TakeoffDistance });
            }>
                <option value="takeoff">"Takeoff distance (Fig 5.4 style)"</option>
                <option value="eq">"Equivalent altitude (Fig 5.2 style)"</option>
            </select>

            <svg
                class="plot-svg"
                viewBox="0 0 400 300"
                on:mousedown=move |_| dragging.set(true)
                on:mouseup=move |_| dragging.set(false)
                on:mouseleave=move |_| dragging.set(false)
                on:mousemove=on_move
            >
                <HandPlotArt kind=kind/>
                <GuideLines cursor_x=cursor_x cursor_y=cursor_y/>
                <Crosshair x=move || cursor_x.get() y=move || cursor_y.get()/>
            </svg>
            <p class="cursor-readout">
                {move || format_hand_plot_readout(kind.get(), cursor_x.get(), cursor_y.get(), data.get(), inputs.get())}
            </p>
        </div>
    }
}

#[component]
fn HandPlotArt(kind: RwSignal<PlotKind>) -> impl IntoView {
    view! {
        <rect x="40" y="20" width="340" height="250" fill="#0b1220" stroke="#334155"/>
        {move || match kind.get() {
            PlotKind::TakeoffDistance => view! {
                <text x="50" y="40" fill="#94a3b8" font-size="12">"OAT →"</text>
                <text x="350" y="270" fill="#94a3b8" font-size="12">"PA ↑"</text>
                <path d="M 60 240 Q 200 180 360 80" stroke="#38bdf8" fill="none" stroke-width="2"/>
                <path d="M 60 260 Q 220 210 360 120" stroke="#f472b6" fill="none" stroke-width="2"/>
            }.into_any(),
            PlotKind::EquivalentAltitude => view! {
                <text x="50" y="40" fill="#94a3b8" font-size="12">"OAT →"</text>
                <text x="350" y="270" fill="#94a3b8" font-size="12">"PA ↑"</text>
                <path d="M 60 230 L 360 90" stroke="#a78bfa" fill="none" stroke-width="2"/>
                <path d="M 60 250 L 360 130" stroke="#34d399" fill="none" stroke-width="2"/>
            }.into_any(),
        }}
    }
}

#[component]
fn GuideLines(cursor_x: RwSignal<f64>, cursor_y: RwSignal<f64>) -> impl IntoView {
    view! {
        {move || {
            let x = 40.0 + cursor_x.get() * 340.0;
            let y = 20.0 + (1.0 - cursor_y.get()) * 250.0;
            view! {
                <line x1=x y1="20" x2=x y2="270" stroke="#64748b" stroke-dasharray="4 3"/>
                <line x1="40" y1=y x2="380" y2=y stroke="#64748b" stroke-dasharray="4 3"/>
            }
        }}
    }
}

#[component]
fn Crosshair(x: impl Fn() -> f64 + Send + 'static, y: impl Fn() -> f64 + Send + 'static) -> impl IntoView {
    view! {
        {move || {
            let cx = 40.0 + x() * 340.0;
            let cy = 20.0 + (1.0 - y()) * 250.0;
            view! {
                <circle cx=cx cy=cy r="6" fill="#fbbf24" stroke="#0f172a" stroke-width="2"/>
            }
        }}
    }
}

fn update_cursor_from_event(ev: leptos::ev::MouseEvent, cursor_x: RwSignal<f64>, cursor_y: RwSignal<f64>) {
    let ox = ev.offset_x() as f64;
    let oy = ev.offset_y() as f64;
    let nx = ((ox - 40.0) / 340.0).clamp(0.0, 1.0);
    let ny = (1.0 - ((oy - 20.0) / 250.0)).clamp(0.0, 1.0);
    cursor_x.set(nx);
    cursor_y.set(ny);
}

fn apply_cursor_to_inputs(
    kind: PlotKind,
    nx: f64,
    ny: f64,
    data: PerformanceData,
    inputs: RwSignal<FlightInputs>,
) {
    let inner = data.inner();
    match kind {
        PlotKind::TakeoffDistance => {
            let to_pa = &inner.to_pa.value;
            let to_t = &inner.to_t.value;
            let pa_min = *to_pa.first().unwrap_or(&0.0);
            let pa_max = *to_pa.last().unwrap_or(&8000.0);
            let t_min = *to_t.first().unwrap_or(&0.0);
            let t_max = *to_t.last().unwrap_or(&40.0);
            let pa = pa_min + ny * (pa_max - pa_min);
            let oat = t_min + nx * (t_max - t_min);
            inputs.update(|i| {
                i.oat_c = oat;
                i.field_elevation_ft = pa;
            });
        }
        PlotKind::EquivalentAltitude => {
            let fan_pa = &inner.fan_pa.value;
            let fan_t = &inner.fan_t.value;
            let pa_min = *fan_pa.first().unwrap_or(&0.0);
            let pa_max = *fan_pa.last().unwrap_or(&8000.0);
            let t_min = *fan_t.first().unwrap_or(&0.0);
            let t_max = *fan_t.last().unwrap_or(&40.0);
            inputs.update(|i| {
                i.oat_c = t_min + nx * (t_max - t_min);
                i.field_elevation_ft = pa_min + ny * (pa_max - pa_min);
            });
        }
    }
    let _ = pressure_altitude_ft(inputs.get().field_elevation_ft, inputs.get().qnh_hpa);
}

fn format_hand_plot_readout(
    kind: PlotKind,
    nx: f64,
    ny: f64,
    _data: PerformanceData,
    inputs: FlightInputs,
) -> String {
    let pa = pressure_altitude_ft(inputs.field_elevation_ft, inputs.qnh_hpa);
    match kind {
        PlotKind::TakeoffDistance => format!(
            "Hand plot → OAT {:.0} °C, PA {:.0} ft ({:.0}%, {:.0}%)",
            inputs.oat_c,
            pa,
            nx * 100.0,
            ny * 100.0
        ),
        PlotKind::EquivalentAltitude => format!(
            "Hand plot Fig 5.2 → OAT {:.0} °C, PA {:.0} ft",
            inputs.oat_c, pa
        ),
    }
}

fn event_select_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}
