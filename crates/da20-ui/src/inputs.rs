use da20_core::altitude::effective_pressure_altitude_ft;
use da20_core::types::{ClimbFigure, FlightInputs};
use da20_core::wind_format::{format_wind, parse_wind};
use leptos::prelude::*;

use crate::i18n::{copy, Locale};

pub fn clear_direct_pa(inputs: RwSignal<FlightInputs>) {
    inputs.update(|i| i.direct_pressure_altitude_ft = None);
}

#[component]
pub fn NumField(
    label: impl Fn() -> String + Send + 'static,
    get: impl Fn() -> f64 + Send + 'static,
    set: impl Fn(f64) + Send + Copy + 'static,
) -> impl IntoView {
    view! {
        <div class="f">
            <label>{move || label()}</label>
            <input type="number" step="any" class="mono"
                prop:value=move || format!("{}", get())
                on:input=move |ev| {
                    if let Ok(v) = input_value(&ev).parse() {
                        set(v);
                    }
                }
            />
        </div>
    }
}

#[component]
pub fn SharedAirfieldInputs(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            view! {
                <NumField label=move || c.field_elev.to_string()
                    get=move || inputs.get().field_elevation_ft
                    set=move |v| {
                        inputs.update(|i| i.field_elevation_ft = v);
                        clear_direct_pa(inputs);
                    }/>
                <NumField label=move || c.qnh.to_string()
                    get=move || inputs.get().qnh_hpa
                    set=move |v| {
                        inputs.update(|i| i.qnh_hpa = v);
                        clear_direct_pa(inputs);
                    }/>
                <NumField label=move || c.oat.to_string()
                    get=move || inputs.get().oat_c
                    set=move |v| inputs.update(|i| i.oat_c = v)/>
                <div class="f">
                    <label>{c.runway}</label>
                    <input prop:value=move || inputs.get().runway.clone()
                        on:input=move |ev| inputs.update(|i| i.runway = input_value(&ev))/>
                </div>
                <WindField locale=locale inputs=inputs/>
                <p class="lab mono">
                    "PA "
                    {move || effective_pressure_altitude_ft(&inputs.get()).round() as i64}
                    " ft"
                </p>
            }
        }}
    }
}

#[component]
pub fn WindField(locale: RwSignal<Locale>, inputs: RwSignal<FlightInputs>) -> impl IntoView {
    view! {
        <div class="f">
            <label>{move || copy(locale.get()).wind}</label>
            <input class="mono"
                prop:value=move || {
                    let i = inputs.get();
                    format_wind(i.wind_dir_deg, i.wind_speed_kt)
                }
                on:change=move |ev| {
                    let raw = input_value(&ev);
                    if let Some((d, s)) = parse_wind(&raw) {
                        inputs.update(|i| {
                            i.wind_dir_deg = d;
                            i.wind_speed_kt = s;
                        });
                    }
                }
                on:blur=move |ev| {
                    let raw = input_value(&ev);
                    if let Some((d, s)) = parse_wind(&raw) {
                        inputs.update(|i| {
                            i.wind_dir_deg = d;
                            i.wind_speed_kt = s;
                        });
                    }
                }
            />
        </div>
    }
}

#[component]
pub fn ClimbFlapToggle(locale: RwSignal<Locale>, inputs: RwSignal<FlightInputs>) -> impl IntoView {
    view! {
        <div class="toggle-row">
            {move || {
                let c = copy(locale.get());
                view! {
                    <button type="button"
                        class="btn g"
                        class:on=move || inputs.get().climb_figure == ClimbFigure::Fig56TakeoffFlaps
                        on:click=move |_| inputs.update(|i| i.climb_figure = ClimbFigure::Fig56TakeoffFlaps)>
                        {c.climb_flaps_to}
                    </button>
                    <button type="button"
                        class="btn g"
                        class:on=move || inputs.get().climb_figure == ClimbFigure::Fig55CruiseFlaps
                        on:click=move |_| inputs.update(|i| i.climb_figure = ClimbFigure::Fig55CruiseFlaps)>
                        {c.climb_flaps_up}
                    </button>
                    <button type="button"
                        class="btn g"
                        class:on=move || inputs.get().climb_figure == ClimbFigure::Fig58BalkedLanding
                        on:click=move |_| inputs.update(|i| i.climb_figure = ClimbFigure::Fig58BalkedLanding)>
                        {c.climb_fig_58}
                    </button>
                }
            }}
        </div>
    }
}

pub fn input_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}
