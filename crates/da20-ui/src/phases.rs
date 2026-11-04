use da20_core::endurance::compute_endurance;
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::components::{ProvenanceLine, WarningsList};
use crate::i18n::{copy, Locale};
use crate::inputs::{input_value, ClimbFlapToggle, NumField, SharedAirfieldInputs};
use crate::interactive_chart::{ChartKind, CruiseTableView, InteractiveChart, LandingTableView};
use crate::storage::{compute_all, new_reading, ReadingsStore, SavedBundle};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Takeoff,
    Climb,
    Cruise,
    Endurance,
    Landing,
    Readings,
    Manual,
}

#[component]
pub fn PhaseShell(
    locale: RwSignal<Locale>,
    phase: RwSignal<Phase>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<da20_core::types::FlightInputs>,
    store: RwSignal<ReadingsStore>,
) -> impl IntoView {
    view! {
        {move || match phase.get() {
            Phase::Takeoff => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Takeoff>
                    <NumField label=move || copy(locale.get()).weight.to_string()
                        get=move || inputs.get().weight_lb
                        set=move |v| inputs.update(|i| i.weight_lb = v)/>
                    <NumField label=move || copy(locale.get()).obstacle_m.to_string()
                        get=move || inputs.get().obstacle_height_m
                        set=move |v| inputs.update(|i| i.obstacle_height_m = v)/>
                </PhaseGrid>
            }.into_any(),
            Phase::Climb => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Climb>
                    <NumField label=move || copy(locale.get()).weight.to_string()
                        get=move || inputs.get().weight_lb
                        set=move |v| inputs.update(|i| i.weight_lb = v)/>
                    <ClimbFlapToggle locale=locale inputs=inputs/>
                </PhaseGrid>
            }.into_any(),
            Phase::Cruise => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Cruise>
                    <NumField label=move || copy(locale.get()).cruise_pa.to_string()
                        get=move || inputs.get().cruise_pressure_alt_ft
                        set=move |v| inputs.update(|i| i.cruise_pressure_alt_ft = v)/>
                    <NumField label=move || copy(locale.get()).cruise_rpm.to_string()
                        get=move || inputs.get().cruise_rpm
                        set=move |v| inputs.update(|i| i.cruise_rpm = v)/>
                </PhaseGrid>
            }.into_any(),
            Phase::Endurance => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Endurance>
                    <NumField label=move || copy(locale.get()).cruise_pa.to_string()
                        get=move || inputs.get().cruise_pressure_alt_ft
                        set=move |v| inputs.update(|i| i.cruise_pressure_alt_ft = v)/>
                    <NumField label=move || copy(locale.get()).cruise_rpm.to_string()
                        get=move || inputs.get().cruise_rpm
                        set=move |v| inputs.update(|i| i.cruise_rpm = v)/>
                    <NumField label=move || copy(locale.get()).fuel_usable.to_string()
                        get=move || inputs.get().fuel_usable_us_gal
                        set=move |v| inputs.update(|i| i.fuel_usable_us_gal = v)/>
                    <NumField label=move || copy(locale.get()).fuel_reserve.to_string()
                        get=move || inputs.get().fuel_reserve_us_gal
                        set=move |v| inputs.update(|i| i.fuel_reserve_us_gal = v)/>
                </PhaseGrid>
            }.into_any(),
            Phase::Landing => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Landing>
                    <p class="lab warn">{move || copy(locale.get()).landing_no_corr}</p>
                </PhaseGrid>
            }.into_any(),
            Phase::Readings => view! { <crate::readings::ReadingsPage locale=locale store=store/> }.into_any(),
            Phase::Manual => view! { <MethodsManual locale=locale data=data/> }.into_any(),
        }}
    }
}

#[component]
fn PhaseGrid(
    locale: RwSignal<Locale>,
    phase: Phase,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<da20_core::types::FlightInputs>,
    store: RwSignal<ReadingsStore>,
    children: Children,
) -> impl IntoView {
    let bundle = Memo::new(move |_| {
        let d = data.get();
        let i = inputs.get();
        let mut b = compute_all(&d, &i);
        b.endurance = compute_endurance(&d, &i).ok();
        b
    });
    let label = RwSignal::new(String::new());

    view! {
        <section class="phase scr on">
            <div class="grid">
                <div class="card">
                    <h3>{move || copy(locale.get()).inputs_title}</h3>
                    <SharedAirfieldInputs locale=locale inputs=inputs/>
                    {children()}
                </div>
                <div class="card chart-card">
                    <h3>{phase_title(locale, phase)}</h3>
                    {phase_chart(phase, locale, data, inputs)}
                    {move || phase_results(locale.get(), phase, bundle.get())}
                    <div class="save-row">
                        <label>
                            <input prop:value=move || label.get()
                                on:input=move |ev| label.set(input_value(&ev))/>
                        </label>
                        <button class="btn" on:click=move |_| {
                            let mut b = bundle.get();
                            if b.endurance.is_none() {
                                b.endurance = compute_endurance(&data.get(), &inputs.get()).ok();
                            }
                            let reading = new_reading(
                                if label.get().is_empty() {
                                    phase_default_label(phase)
                                } else {
                                    label.get()
                                },
                                phase_storage_name(phase),
                                inputs.get(),
                                b,
                            );
                            store.update(|s| s.push(reading));
                        }>{move || copy(locale.get()).save_reading}</button>
                    </div>
                </div>
            </div>
        </section>
    }
}

fn phase_title(locale: RwSignal<Locale>, phase: Phase) -> impl IntoView {
    move || {
        let c = copy(locale.get());
        match phase {
            Phase::Takeoff => c.takeoff,
            Phase::Climb => c.climb,
            Phase::Cruise => c.cruise,
            Phase::Endurance => c.tab_endurance,
            Phase::Landing => c.landing,
            _ => "",
        }
    }
}

fn phase_storage_name(phase: Phase) -> String {
    match phase {
        Phase::Takeoff => "takeoff",
        Phase::Climb => "climb",
        Phase::Cruise => "cruise",
        Phase::Endurance => "endurance",
        Phase::Landing => "landing",
        Phase::Readings => "readings",
        Phase::Manual => "manual",
    }
    .into()
}

fn phase_default_label(phase: Phase) -> String {
    match phase {
        Phase::Takeoff => "Takeoff".into(),
        Phase::Climb => "Climb".into(),
        Phase::Cruise => "Cruise".into(),
        Phase::Endurance => "Endurance".into(),
        Phase::Landing => "Landing".into(),
        _ => "Reading".into(),
    }
}

fn phase_chart(
    phase: Phase,
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<da20_core::types::FlightInputs>,
) -> impl IntoView {
    match phase {
        Phase::Takeoff => view! {
            <InteractiveChart locale=locale kind=ChartKind::Takeoff data=data inputs=inputs/>
            <InteractiveChart locale=locale kind=ChartKind::Wind data=data inputs=inputs/>
        }
        .into_any(),
        Phase::Climb => view! {
            <InteractiveChart locale=locale kind=ChartKind::Climb data=data inputs=inputs/>
        }
        .into_any(),
        Phase::Cruise => view! {
            <CruiseTableView data=data inputs=inputs/>
        }
        .into_any(),
        Phase::Endurance => view! {
            <p class="chip">{move || copy(locale.get()).derived_not_afm}</p>
        }
        .into_any(),
        Phase::Landing => view! {
            <LandingTableView data=data inputs=inputs/>
            <InteractiveChart locale=locale kind=ChartKind::Landing data=data inputs=inputs/>
        }
        .into_any(),
        _ => view! { <span></span> }.into_any(),
    }
}

fn phase_results(locale: Locale, phase: Phase, b: SavedBundle) -> impl IntoView {
    match phase {
        Phase::Takeoff => view! {
            {match &b.takeoff {
                Some(t) => view! {
                    <div class="results-block">
                        <ProvenanceLine provenance=t.provenance.clone()/>
                        <p class="big">{t.ground_roll_ft.round() as i64}<span class="u">" ft roll"</span></p>
                        <p>"50 ft: "{t.distance_over_50ft_ft.round() as i64}" ft"</p>
                        <WarningsList warnings=t.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
            {match &b.wind {
                Some(w) => view! {
                    <p class="mono">"HW "{w.components.headwind_kt.round()}" kt · XW "{w.components.crosswind_kt.round()}" kt"</p>
                }.into_any(),
                None => view! { <span></span> }.into_any(),
            }}
        }.into_any(),
        Phase::Climb => view! {
            {match &b.climb {
                Some(c) => view! {
                    <div class="results-block">
                        <ProvenanceLine provenance=c.provenance.clone()/>
                        <p class="big">{c.rate_of_climb_fpm.round() as i64}<span class="u">" fpm"</span></p>
                        <p>"EA "{c.equivalent_altitude_ft.round() as i64}" ft"</p>
                        <WarningsList warnings=c.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Cruise => view! {
            {match &b.cruise {
                Some(c) => view! {
                    <div class="results-block">
                        <ProvenanceLine provenance=c.provenance.clone()/>
                        <p class="big">{c.tas_kt.round() as i64}<span class="u">" kt TAS"</span></p>
                        <p>{c.fuel_flow_us_gal_h.round()}" US gal/h"</p>
                        <WarningsList warnings=c.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Endurance => view! {
            {match &b.endurance {
                Some(e) => view! {
                    <div class="results-block">
                        <ProvenanceLine provenance=e.provenance.clone()/>
                        <p class="chip">{copy(locale).derived_not_afm}</p>
                        <p class="big">{format!("{:.1}", e.endurance_h)}<span class="u">" h"</span></p>
                        <p>{format!("{:.0}", e.range_nm)}" NM · "{e.fuel_flow_us_gal_h.round()}" gph @ "{e.tas_kt.round()}" kt"</p>
                        <WarningsList warnings=e.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Landing => view! {
            {match &b.landing {
                Some(l) => view! {
                    <div class="results-block">
                        <ProvenanceLine provenance=l.provenance.clone()/>
                        <p class="big">{l.ground_roll_ft.round() as i64}<span class="u">" ft"</span></p>
                        <p>"50 ft: "{l.distance_over_50ft_ft.round() as i64}" ft"</p>
                        <WarningsList warnings=l.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        _ => view! { <span></span> }.into_any(),
    }
}

#[component]
fn MethodsManual(locale: RwSignal<Locale>, data: RwSignal<PerformanceData>) -> impl IntoView {
    view! {
        <section class="methods scr on">
            <div class="grid">
                <div class="card">
                    <h3>"Document"</h3>
                    <p><b>{move || data.get().meta().doc.clone()}</b>" rev "{move || data.get().meta().revision.to_string()}</p>
                    <p class="lab">{move || copy(locale.get()).manual_intro}</p>
                </div>
                <div class="card man">
                    <h3>{move || copy(locale.get()).manual_title}</h3>
                    <ul class="method-list">
                        {move || copy(locale.get()).methods.iter().map(|e| view! {
                            <li>
                                <h4>{e.title}</h4>
                                <p><strong>"AFM: "</strong>{e.afm}</p>
                                <p>{e.summary}</p>
                                <p><em>{e.interpolation}</em></p>
                                <p class="warn">{e.limitations}</p>
                            </li>
                        }).collect_view()}
                    </ul>
                </div>
            </div>
        </section>
    }
}
