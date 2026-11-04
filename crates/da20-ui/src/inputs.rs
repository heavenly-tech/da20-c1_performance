use da20_core::altitude::{clear_pa_override, effective_pressure_altitude_ft, set_pa_override};
use da20_core::cruise::isa_temp_c_at_pa;
use da20_core::pressure::{
    format_qnh_dual, pressure_altitude_derivation, qnh_hpa_to_inhg, qnh_inhg_to_hpa,
};
use da20_core::types::defaults;
use da20_core::types::{ChartPhase, ClimbFigure, FlightInputs};
use da20_core::wind_format::{format_wind, parse_wind};
use leptos::prelude::*;

use crate::i18n::{copy, Locale};
use crate::qnh_unit::QnhUnit;

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
                <NumField label=move || c.field_elev.to_string()
                    get=move || inputs.get().field_elevation_ft
                    set=move |v| {
                        inputs.update(|i| {
                            i.field_elevation_ft = v;
                            clear_pa_override(i);
                        });
                    }/>
                <QnhField locale=locale inputs=inputs/>
                <NumField label=move || c.oat.to_string()
                    get=move || inputs.get().oat_c
                    set=move |v| inputs.update(|i| i.oat_c = v)/>
                <div class="f">
                    <label>{c.runway}</label>
                    <input prop:value=move || inputs.get().runway.clone()
                        on:input=move |ev| inputs.update(|i| i.runway = input_value(&ev))/>
                </div>
                <WindField locale=locale inputs=inputs/>
                <p class="lab mono">{format_qnh_dual(i.qnh_hpa)}</p>
                {(!override_active).then(|| {
                    let deriv = pressure_altitude_derivation(i.field_elevation_ft, i.qnh_hpa);
                    view! { <p class="lab mono pa-deriv">{deriv}</p> }
                })}
                <p class="lab mono">
                    "PA "
                    {pa.round() as i64}
                    " ft"
                </p>
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

#[component]
pub fn QnhField(locale: RwSignal<Locale>, inputs: RwSignal<FlightInputs>) -> impl IntoView {
    let unit = RwSignal::new(QnhUnit::load());
    view! {
        {move || {
            let c = copy(locale.get());
            let u = unit.get();
            let hpa = inputs.get().qnh_hpa;
            view! {
                <div class="f qnh-field">
                    <label>{c.qnh}" ("{u.label()}")"</label>
                    <div class="qnh-row">
                        <input type="number" step="any" class="mono"
                            prop:value=move || match unit.get() {
                                QnhUnit::Hpa => format!("{:.1}", hpa),
                                QnhUnit::InHg => format!("{:.2}", qnh_hpa_to_inhg(hpa)),
                            }
                            on:input=move |ev| {
                                if let Ok(v) = input_value(&ev).parse::<f64>() {
                                    inputs.update(|i| {
                                        i.qnh_hpa = match unit.get() {
                                            QnhUnit::Hpa => v,
                                            QnhUnit::InHg => qnh_inhg_to_hpa(v),
                                        };
                                        clear_pa_override(i);
                                    });
                                }
                            }
                        />
                        <button type="button" class="btn g"
                            class:on=move || unit.get() == QnhUnit::Hpa
                            on:click=move |_| {
                                let next = QnhUnit::Hpa;
                                unit.set(next);
                                next.save();
                            }>"hPa"</button>
                        <button type="button" class="btn g"
                            class:on=move || unit.get() == QnhUnit::InHg
                            on:click=move |_| {
                                let next = QnhUnit::InHg;
                                unit.set(next);
                                next.save();
                            }>"inHg"</button>
                    </div>
                </div>
            }
        }}
    }
}

#[component]
pub fn ObstacleField(locale: RwSignal<Locale>, inputs: RwSignal<FlightInputs>) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            view! {
                <NumField
                    label=move || format!("{} (0–15)", c.obstacle_m)
                    get=move || inputs.get().obstacle_height_m
                    set=move |v| inputs.update(|i| {
                        i.obstacle_height_m = v.clamp(0.0, defaults::OBSTACLE_HEIGHT_MAX_M);
                    })
                />
                <div class="toggle-row">
                    <button type="button" class="btn g"
                        on:click=move |_| inputs.update(|i| {
                            i.obstacle_height_m = defaults::OBSTACLE_PRESET_AFM_EXAMPLE_M;
                        })>
                        {c.obstacle_preset_afm}
                    </button>
                    <button type="button" class="btn g"
                        on:click=move |_| inputs.update(|i| {
                            i.obstacle_height_m = defaults::OBSTACLE_HEIGHT_DEFAULT_M;
                        })>
                        {c.obstacle_preset_screen}
                    </button>
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

pub fn set_chart_pa(inputs: RwSignal<FlightInputs>, pa_ft: f64, phase: ChartPhase) {
    inputs.update(|i| set_pa_override(i, pa_ft, phase));
}
