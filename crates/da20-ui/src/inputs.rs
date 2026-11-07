use da20_core::altitude::{clear_pa_override, effective_pressure_altitude_ft, set_pa_override};
use da20_core::cruise::isa_temp_c_at_pa;
use da20_core::types::{ChartPhase, ClimbFigure, FlightInputs};
use da20_core::wind::{
    runway_heading_deg, runway_heading_status, wind_chart_panel_needs_crosswind_note,
    wind_components, RunwayHeadingStatus,
};
use da20_core::wind_format::{format_wind, parse_wind};
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::i18n::{copy, format_along_runway_component, format_crosswind_component, Locale};

/// Takeoff vs climb vs landing (full) airfield blocks.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AirfieldInputsMode {
    #[default]
    Full,
    Takeoff,
    Climb,
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
    chart_phase: Option<ChartPhase>,
    #[prop(default = AirfieldInputsMode::Full)] mode: AirfieldInputsMode,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            let i = inputs.get();
            let pa = effective_pressure_altitude_ft(&i, chart_phase);
            let override_active = chart_phase.is_some_and(|p| {
                i.pa_override.as_ref().is_some_and(|o| o.phase == p)
            });
            view! {
                <NumField label=move || c.pressure_altitude_ft.to_string()
                    get=move || inputs.get().pressure_altitude_input_ft
                    set=move |v| {
                        inputs.update(|i| {
                            i.pressure_altitude_input_ft = v;
                            clear_pa_override(i);
                        });
                    }/>
                <NumField label=move || c.oat.to_string()
                    get=move || inputs.get().oat_c
                    set=move |v| inputs.update(|i| i.oat_c = v)/>
                {(mode == AirfieldInputsMode::Takeoff || mode == AirfieldInputsMode::Full).then(|| {
                    view! {
                        <div class="f">
                            <label>{c.runway}</label>
                            <input class="mono" maxlength="3"
                                prop:value=move || inputs.get().runway.clone()
                                on:input=move |ev| inputs.update(|i| i.runway = input_value(&ev))/>
                            {move || {
                                let c = copy(locale.get());
                                match runway_heading_status(&inputs.get().runway) {
                                    RunwayHeadingStatus::AmbiguousShort => view! {
                                        <p class="lab field-error">{c.runway_heading_ambiguous}</p>
                                    }.into_any(),
                                    RunwayHeadingStatus::Invalid => view! {
                                        <p class="lab field-error">{c.runway_heading_invalid}</p>
                                    }.into_any(),
                                    _ => ().into_any(),
                                }
                            }}
                        </div>
                        <WindField locale=locale inputs=inputs/>
                    }
                })}
                {(!override_active).then(|| view! {
                    <p class="lab mono">
                        "PA "
                        {pa.round() as i64}
                        " ft"
                    </p>
                })}
                {override_active.then(|| {
                    let pa_ov = i.pa_override.as_ref().map(|o| o.pa_ft.round() as i64).unwrap_or(0);
                    view! {
                        <span class="chip pa-override">
                            {c.pa_override_chip}" "{pa_ov}" ft"
                            <button type="button" class="btn g chip-reset"
                                on:click=move |_| inputs.update(clear_pa_override)>
                                {c.pa_override_reset}
                            </button>
                        </span>
                    }
                })}
            }
        }}
    }
}

/// Wheel fairings: one shared `FlightInputs` flag (climb −3% ROC; cruise −5% TAS per AFM).
#[component]
pub fn SharedWheelFairingToggle(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            view! {
                <div class="f wheel-fairings-shared">
                    <label>{c.wheel_fairings_toggle_label}</label>
                    <div class="toggle-row wheel-fairings-row">
                        <button type="button" class="btn g"
                            class:on=move || inputs.get().wheel_fairings_installed
                            on:click=move |_| inputs.update(|i| i.wheel_fairings_installed = true)>
                            {c.wheel_fairings_installed_yes}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || !inputs.get().wheel_fairings_installed
                            on:click=move |_| inputs.update(|i| i.wheel_fairings_installed = false)>
                            {c.wheel_fairings_installed_no}
                        </button>
                    </div>
                </div>
            }
        }}
    }
}

#[component]
pub fn CruiseInputs(locale: RwSignal<Locale>, inputs: RwSignal<FlightInputs>) -> impl IntoView {
    let set_cruise_oat_from_isa = move |deviation: f64| {
        inputs.update(|i| {
            i.cruise_oat_c = isa_temp_c_at_pa(i.cruise_pressure_alt_ft) + deviation;
        });
    };
    view! {
        {move || {
            let c = copy(locale.get());
            view! {
                <NumField label=move || c.cruise_pa.to_string()
                    get=move || inputs.get().cruise_pressure_alt_ft
                    set=move |v| inputs.update(|i| {
                        i.cruise_pressure_alt_ft = v;
                        i.cruise_oat_c = isa_temp_c_at_pa(v);
                    })/>
                <NumField label=move || c.cruise_oat.to_string()
                    get=move || inputs.get().cruise_oat_c
                    set=move |v| inputs.update(|i| i.cruise_oat_c = v)/>
                <NumField label=move || c.cruise_rpm.to_string()
                    get=move || inputs.get().cruise_rpm
                    set=move |v| inputs.update(|i| i.cruise_rpm = v)/>
                {move || {
                    let c = copy(locale.get());
                    let installed = inputs.get().wheel_fairings_installed;
                    (!installed).then(|| view! {
                        <p class="lab warn afm-quote">{c.afm_wheel_fairings_note_cruise}</p>
                    })
                }}
                <div class="toggle-row">
                    <button type="button" class="btn g"
                        on:click=move |_| set_cruise_oat_from_isa(0.0)>
                        {c.cruise_isa}
                    </button>
                    <button type="button" class="btn g"
                        on:click=move |_| set_cruise_oat_from_isa(-20.0)>
                        {c.cruise_disa_minus20}
                    </button>
                    <button type="button" class="btn g"
                        on:click=move |_| set_cruise_oat_from_isa(10.0)>
                        {c.cruise_disa_plus10}
                    </button>
                </div>
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
                            i.wind_strip_crosswind_clamped = false;
                        });
                    }
                }
                on:blur=move |ev| {
                    let raw = input_value(&ev);
                    if let Some((d, s)) = parse_wind(&raw) {
                        inputs.update(|i| {
                            i.wind_dir_deg = d;
                            i.wind_speed_kt = s;
                            i.wind_strip_crosswind_clamped = false;
                        });
                    }
                }
            />
            {move || {
                let c = copy(locale.get());
                let i = inputs.get();
                if runway_heading_status(&i.runway) != RunwayHeadingStatus::Valid {
                    return ().into_any();
                }
                let rwy = runway_heading_deg(&i.runway).unwrap_or(0.0);
                let comp = wind_components(i.wind_dir_deg, i.wind_speed_kt, rwy);
                let along = format_along_runway_component(locale.get(), &comp);
                let cross = format_crosswind_component(locale.get(), comp.crosswind_kt);
                let show_xw_note = wind_chart_panel_needs_crosswind_note(&i);
                view! {
                    <div class="wind-components">
                        {along.map(|line| view! { <p class="lab mono">{line}</p> })}
                        {(comp.crosswind_kt.abs() > 0.05).then(|| view! {
                            <p class="lab mono">{cross}</p>
                        })}
                        {show_xw_note.then(|| view! {
                            <p class="lab field-error">{c.wind_strip_crosswind_note}</p>
                        })}
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}

#[component]
pub fn ClimbFlapToggle(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    data: RwSignal<PerformanceData>,
) -> impl IntoView {
    view! {
        <div class="toggle-row">
            {move || {
                let c = copy(locale.get());
                let has_balked = data
                    .get()
                    .inner()
                    .roc
                    .value
                    .contains_key("balked_5_8");
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
                        prop:disabled=move || !has_balked
                        prop:title=move || {
                            if has_balked {
                                String::new()
                            } else {
                                c.climb_flaps_balked_unavailable.to_string()
                            }
                        }
                        on:click=move |_| {
                            if has_balked {
                                inputs.update(|i| i.climb_figure = ClimbFigure::Fig58BalkedLanding);
                            }
                        }>
                        {c.climb_flaps_ldg}
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

pub fn set_chart_pa(inputs: RwSignal<FlightInputs>, pa_ft: f64, phase: ChartPhase) {
    inputs.update(|i| set_pa_override(i, pa_ft, phase));
}
