use leptos::prelude::*;

use crate::components::{ProvenanceLine, WarningsList};
use crate::hand_plot::HandPlotPanel;
use crate::storage::{compute_all, new_reading, ReadingsStore};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryMode {
    Numeric,
    HandPlot,
}

#[component]
pub fn Planner(
    data: RwSignal<da20_core::PerformanceData>,
    store: RwSignal<ReadingsStore>,
) -> impl IntoView {
    let inputs = RwSignal::new(da20_core::types::FlightInputs::default());
    let mode = RwSignal::new(EntryMode::Numeric);
    let label = RwSignal::new(String::from("Reading"));

    let bundle = Memo::new(move |_| {
        let d = data.get();
        let i = inputs.get();
        compute_all(&d, &i)
    });

    view! {
        <section class="planner">
            <div class="mode-toggle">
                <button class="nav-btn" on:click=move |_| mode.set(EntryMode::Numeric)>"Numeric entry"</button>
                <button class="nav-btn" on:click=move |_| mode.set(EntryMode::HandPlot)>"Hand plot"</button>
            </div>

            {move || if mode.get() == EntryMode::Numeric {
                view! { <NumericForm inputs=inputs/> }.into_any()
            } else {
                view! { <HandPlotPanel data=data inputs=inputs/> }.into_any()
            }}

            <div class="results">
                <h2>"Results"</h2>
                {move || {
                    let b = bundle.get();
                    view! {
                        <TakeoffCard takeoff=b.takeoff.clone()/>
                        <ClimbCard climb=b.climb.clone()/>
                        <CruiseCard cruise=b.cruise.clone()/>
                        <LandingCard landing=b.landing.clone()/>
                        <WindCard wind=b.wind.clone()/>
                    }
                }}

                <div class="save-row">
                    <label>"Label "
                        <input prop:value=move || label.get()
                            on:input=move |ev| label.set(input_value(&ev))/>
                    </label>
                    <button class="primary" on:click=move |_| {
                        let b = bundle.get();
                        let reading = new_reading(label.get(), inputs.get(), b);
                        store.update(|s| s.push(reading));
                    }>"Save reading"</button>
                </div>
            </div>
        </section>
    }
}

#[component]
fn NumericForm(inputs: RwSignal<da20_core::types::FlightInputs>) -> impl IntoView {
    view! {
        <form class="form-grid" on:submit=|ev| ev.prevent_default()>
            <NumField label="Field elevation (ft)" get=move || inputs.get().field_elevation_ft
                set=move |v| inputs.update(|i| i.field_elevation_ft = v)/>
            <NumField label="QNH (hPa)" get=move || inputs.get().qnh_hpa
                set=move |v| inputs.update(|i| i.qnh_hpa = v)/>
            <NumField label="OAT (°C)" get=move || inputs.get().oat_c
                set=move |v| inputs.update(|i| i.oat_c = v)/>
            <label>"Runway "
                <input prop:value=move || inputs.get().runway.clone()
                    on:input=move |ev| inputs.update(|i| i.runway = input_value(&ev))/>
            </label>
            <NumField label="Wind dir (°)" get=move || inputs.get().wind_dir_deg
                set=move |v| inputs.update(|i| i.wind_dir_deg = v)/>
            <NumField label="Wind speed (kt)" get=move || inputs.get().wind_speed_kt
                set=move |v| inputs.update(|i| i.wind_speed_kt = v)/>
            <NumField label="Weight (lb)" get=move || inputs.get().weight_lb
                set=move |v| inputs.update(|i| i.weight_lb = v)/>
            <NumField label="Cruise PA (ft)" get=move || inputs.get().cruise_pressure_alt_ft
                set=move |v| inputs.update(|i| i.cruise_pressure_alt_ft = v)/>
            <NumField label="Cruise RPM" get=move || inputs.get().cruise_rpm
                set=move |v| inputs.update(|i| i.cruise_rpm = v)/>
            <NumField label="Obstacle height (m)" get=move || inputs.get().obstacle_height_m
                set=move |v| inputs.update(|i| i.obstacle_height_m = v)/>
        </form>
    }
}

#[component]
fn NumField(
    label: &'static str,
    get: impl Fn() -> f64 + Send + 'static,
    set: impl Fn(f64) + Send + Copy + 'static,
) -> impl IntoView {
    view! {
        <label>{label}
            <input type="number" step="any"
                prop:value=move || format!("{}", get())
                on:input=move |ev| {
                    if let Ok(v) = input_value(&ev).parse() { set(v); }
                }
            />
        </label>
    }
}

#[component]
fn TakeoffCard(takeoff: Option<da20_core::types::TakeoffResult>) -> impl IntoView {
    view! {
        <article class="result-card">
            <h3>"Takeoff (Fig 5.4)"</h3>
            {match takeoff {
                Some(t) => view! {
                    <ProvenanceLine provenance=t.provenance.clone()/>
                    <p>"PA "{t.pressure_altitude_ft.round()} " ft · Ground "{t.ground_roll_ft.round()} " ft · obstacle distance "{t.distance_over_50ft_ft.round()} " ft"</p>
                    <WarningsList warnings=t.warnings/>
                }.into_any(),
                None => view! { <p class="error">"Unavailable"</p> }.into_any(),
            }}
        </article>
    }
}

#[component]
fn ClimbCard(climb: Option<da20_core::types::ClimbResult>) -> impl IntoView {
    view! {
        <article class="result-card">
            <h3>"Climb (5.5/5.8 via 5.2)"</h3>
            {match climb {
                Some(c) => view! {
                    <ProvenanceLine provenance=c.provenance.clone()/>
                    <p>"Eq alt "{c.equivalent_altitude_ft.round()} " ft · ROC "{c.rate_of_climb_fpm.round()} " fpm (Fig 5.5)"</p>
                    {c.time_to_altitude_min.map(|t| view! {
                        <p class="derived">"Time to alt (derived estimate, no AFM chart): "{t.round()} " min"</p>
                    })}
                    <WarningsList warnings=c.warnings/>
                }.into_any(),
                None => view! { <p class="error">"Unavailable"</p> }.into_any(),
            }}
        </article>
    }
}

#[component]
fn CruiseCard(cruise: Option<da20_core::types::CruiseResult>) -> impl IntoView {
    view! {
        <article class="result-card">
            <h3>"Cruise (Table 3)"</h3>
            {match cruise {
                Some(c) => view! {
                    <ProvenanceLine provenance=c.provenance.clone()/>
                    <p>"TAS "{c.tas_kt.round()} " kt · FF "{c.fuel_flow_us_gal_h.round()} " US gal/h"</p>
                    <WarningsList warnings=c.warnings/>
                }.into_any(),
                None => view! { <p class="error">"Unavailable"</p> }.into_any(),
            }}
        </article>
    }
}

#[component]
fn LandingCard(landing: Option<da20_core::types::LandingResult>) -> impl IntoView {
    view! {
        <article class="result-card">
            <h3>"Landing (Table 4)"</h3>
            {match landing {
                Some(l) => view! {
                    <ProvenanceLine provenance=l.provenance.clone()/>
                    <p>"Ground "{l.ground_roll_ft.round()} " ft · 50 ft "{l.distance_over_50ft_ft.round()} " ft"</p>
                    <WarningsList warnings=l.warnings/>
                }.into_any(),
                None => view! { <p class="error">"Unavailable"</p> }.into_any(),
            }}
        </article>
    }
}

#[component]
fn WindCard(wind: Option<da20_core::types::WindResult>) -> impl IntoView {
    view! {
        <article class="result-card">
            <h3>"Wind (Fig 5.3)"</h3>
            {match wind {
                Some(w) => view! {
                    <ProvenanceLine provenance=w.provenance.clone()/>
                    <p>"Headwind "{w.components.headwind_kt.round()} " kt · Crosswind "{w.components.crosswind_kt.round()} " kt"</p>
                    <WarningsList warnings=w.warnings/>
                }.into_any(),
                None => view! { <p class="error">"Unavailable"</p> }.into_any(),
            }}
        </article>
    }
}

fn input_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}
