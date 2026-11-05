use std::collections::HashSet;

use da20_core::endurance::compute_endurance;
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::components::{ProvenanceLine, WarningsList};
use crate::i18n::{
    copy, format_along_runway_component, format_crosswind_component, format_obstacle_over,
    format_takeoff_conservative_estimate, format_takeoff_over_obstacle_result, Locale,
};
use crate::inputs::{ClimbFlapToggle, CruiseInputs, NumField, ObstacleField, SharedAirfieldInputs};
use crate::interactive_chart::{ChartKind, CruiseTableView, InteractiveChart, LandingTableView};
use crate::phase_readouts::PhaseReadoutsPanel;
use crate::storage::{compute_all, ReadingsStore, SavedBundle};
use crate::strip_plot::{StripHandPlot, StripKind};
use da20_core::types::ChartPhase;
use da20_core::wind::{runway_heading_status, RunwayHeadingStatus};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Takeoff,
    Climb,
    Cruise,
    Endurance,
    Landing,
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
                    <ObstacleField locale=locale inputs=inputs/>
                    <NumField label=move || copy(locale.get()).weight.to_string()
                        get=move || inputs.get().weight_lb
                        set=move |v| inputs.update(|i| i.weight_lb = v)/>
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
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Cruise airfield=false>
                    <CruiseInputs locale=locale inputs=inputs/>
                </PhaseGrid>
            }.into_any(),
            Phase::Endurance => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Endurance airfield=false>
                    <CruiseInputs locale=locale inputs=inputs/>
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
    #[prop(default = true)] airfield: bool,
    children: Children,
) -> impl IntoView {
    let overlay_ids = RwSignal::new(HashSet::<String>::new());
    let bundle = Memo::new(move |_| {
        let d = data.get();
        let i = inputs.get();
        let mut b = compute_all(&d, &i);
        b.endurance = compute_endurance(&d, &i).ok();
        b
    });

    view! {
        <section class="phase scr on">
            <div class="grid">
                <div class="card">
                    <h3>{move || copy(locale.get()).inputs_title}</h3>
                    {airfield.then(|| view! {
                        <SharedAirfieldInputs
                            locale=locale
                            inputs=inputs
                            chart_phase=chart_phase_for(phase)
                        />
                    })}
                    {(!airfield).then(|| view! { <span></span> })}
                    {children()}
                </div>
                <div class="card chart-card">
                    <h3>{phase_title(locale, phase)}</h3>
                    {phase_chart(phase, locale, data, inputs, store, overlay_ids)}
                    {move || phase_results(locale.get(), phase, bundle.get(), inputs.get())}
                    <PhaseReadoutsPanel
                        locale=locale
                        phase=phase
                        data=data
                        inputs=inputs
                        store=store
                        overlay_ids=overlay_ids
                    />
                </div>
            </div>
        </section>
    }
}

fn chart_phase_for(phase: Phase) -> Option<ChartPhase> {
    match phase {
        Phase::Takeoff => Some(ChartPhase::Takeoff),
        Phase::Climb => Some(ChartPhase::Climb),
        Phase::Landing => Some(ChartPhase::Landing),
        _ => None,
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
            Phase::Manual => "",
        }
    }
}

pub(crate) fn phase_storage_name(phase: Phase) -> String {
    match phase {
        Phase::Takeoff => "takeoff",
        Phase::Climb => "climb",
        Phase::Cruise => "cruise",
        Phase::Endurance => "endurance",
        Phase::Landing => "landing",
        Phase::Manual => "method",
    }
    .into()
}

pub(crate) fn phase_default_label(phase: Phase) -> String {
    match phase {
        Phase::Takeoff => "Takeoff".into(),
        Phase::Climb => "Climb".into(),
        Phase::Cruise => "Cruise".into(),
        Phase::Endurance => "Endurance".into(),
        Phase::Landing => "Landing".into(),
        Phase::Manual => "Method".into(),
    }
}

fn phase_chart(
    phase: Phase,
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
    store: RwSignal<ReadingsStore>,
    overlay_ids: RwSignal<HashSet<String>>,
) -> impl IntoView {
    let phase_key = phase_storage_name(phase);
    match phase {
        Phase::Takeoff => view! {
            <StripHandPlot
                locale=locale
                kind=StripKind::Takeoff
                data=data
                inputs=inputs
                store=store
                phase_key=phase_key
                overlay_ids=overlay_ids
            />
        }
        .into_any(),
        Phase::Climb => view! {
            <StripHandPlot
                locale=locale
                kind=StripKind::Climb
                data=data
                inputs=inputs
                store=store
                phase_key=phase_key
                overlay_ids=overlay_ids
            />
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
        Phase::Manual => view! { <span></span> }.into_any(),
    }
}

fn phase_results(
    locale: Locale,
    phase: Phase,
    b: SavedBundle,
    inputs: da20_core::types::FlightInputs,
) -> impl IntoView {
    let c = copy(locale);
    match phase {
        Phase::Takeoff => {
            let runway_ok =
                runway_heading_status(&inputs.runway) == RunwayHeadingStatus::Valid;
            view! {
            {if runway_ok {
                match &b.takeoff {
                Some(t) => view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=t.provenance.clone()/>
                        <p class="mono">{c.takeoff_ground_roll_at_chart_zero}" "{t.ground_roll_ft.round() as i64}" ft"</p>
                        <p class="mono">{format_takeoff_over_obstacle_result(
                            locale,
                            t.distance_over_obstacle_raw_ft,
                            t.distance_over_obstacle_raw_m,
                        )}</p>
                        <p class="mono">{format_takeoff_conservative_estimate(
                            locale,
                            t.distance_over_obstacle_conservative_ft,
                            t.distance_over_obstacle_conservative_m,
                        )}</p>
                        <p class="lab">{format_obstacle_over(locale, inputs.obstacle_height_m)}</p>
                        <WarningsList locale=locale warnings=t.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }} else {
                view! { <p class="lab results-muted">{c.results_runway_required}</p> }.into_any()
            }}
            {runway_ok.then(|| match &b.wind {
                Some(w) => {
                    let along = format_along_runway_component(locale, &w.components);
                    let cross = format_crosswind_component(locale, w.components.crosswind_kt);
                    view! {
                        <div class="wind-components">
                            {along.map(|line| view! { <p class="mono">{line}</p> })}
                            <p class="mono">{cross}</p>
                        </div>
                    }.into_any()
                }
                None => view! { <span></span> }.into_any(),
            })}
            }
            .into_any()
        }
        Phase::Climb => view! {
            {match &b.climb {
                Some(cl) => view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=cl.provenance.clone()/>
                        <p class="big">{cl.rate_of_climb_fpm.round() as i64}<span class="u">" fpm"</span></p>
                        <p>"EA "{cl.equivalent_altitude_ft.round() as i64}" ft"</p>
                        <WarningsList locale=locale warnings=cl.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Cruise => view! {
            {match &b.cruise {
                Some(cr) => view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=cr.provenance.clone()/>
                        <p class="big">{cr.tas_kt.round() as i64}<span class="u">" kt TAS"</span></p>
                        <p>{format!("{:.1}", cr.fuel_flow_us_gal_h)}" US gal/h"</p>
                        <WarningsList locale=locale warnings=cr.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Endurance => view! {
            {match &b.endurance {
                Some(e) => view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=e.provenance.clone()/>
                        <p class="chip">{copy(locale).derived_not_afm}</p>
                        <p class="big">{format!("{:.1}", e.endurance_h)}<span class="u">" h"</span></p>
                        <p>{format!("{:.0}", e.range_nm)}" NM · "{format!("{:.1}", e.fuel_flow_us_gal_h)}" gph @ "{e.tas_kt.round()}" kt"</p>
                        <WarningsList locale=locale warnings=e.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Landing => view! {
            {match &b.landing {
                Some(l) => view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=l.provenance.clone()/>
                        <p class="big">{l.ground_roll_ft.round() as i64}<span class="u">" ft"</span></p>
                        <p>"50 ft: "{l.distance_over_50ft_ft.round() as i64}" ft"</p>
                        <WarningsList locale=locale warnings=l.warnings.clone()/>
                    </div>
                }.into_any(),
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Manual => view! { <span></span> }.into_any(),
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

use da20_core::types::FlightInputs;
