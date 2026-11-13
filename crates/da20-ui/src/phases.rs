use std::collections::HashSet;

use da20_core::endurance::compute_endurance;
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::components::{ProvenanceLine, WarningsList};
use crate::i18n::{
    afm_lookup_tag_label, copy, format_along_runway_component, format_crosswind_component,
    format_mt_cruise_bhp_not_achievable, format_obstacle_over, format_out_of_chart,
    format_takeoff_conservative_estimate, Locale,
};
use crate::inputs::{
    AirfieldInputsMode, ClimbFlapToggle, CruiseInputs, NumField, SharedAirfieldInputs,
    SharedPropellerSelector, SharedWheelFairingToggle,
};
use crate::interactive_chart::{ChartKind, CruiseTableView, InteractiveChart, LandingTableView};
use crate::mt_cruise_panel::MtCruisePanel;
use crate::phase_readouts::PhaseReadoutsPanel;
use crate::prefs::TakeoffPrefs;
use crate::storage::{compute_all, ReadingsStore, SavedBundle};
use crate::strip_plot::{StripHandPlot, StripKind};
use crate::table_source_panel::{
    MtFig2Fig3SourcePanel, MtFig4SourcePanel, Table3SourcePanel, Table4SourcePanel,
};
use crate::takeoff_panel::{format_takeoff_distances_pair, TakeoffNumericBlock, TakeoffSweepPanel};
use da20_core::takeoff_rounding::{apply_takeoff_distance_rounding, TakeoffDistanceRounding};
use da20_core::types::{ChartPhase, CruiseSolveMode, PropellerKind};
use da20_core::wind::{runway_heading_status, RunwayHeadingStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    takeoff_prefs: RwSignal<TakeoffPrefs>,
) -> impl IntoView {
    view! {
        {move || match phase.get() {
            Phase::Takeoff => view! {
                <>
                    <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Takeoff takeoff_prefs=takeoff_prefs>
                        <SharedPropellerSelector locale=locale inputs=inputs/>
                        <SharedWheelFairingToggle locale=locale inputs=inputs/>
                        <TakeoffNumericBlock locale=locale inputs=inputs prefs=takeoff_prefs/>
                    </PhaseGrid>
                    <TakeoffSweepPanel locale=locale data=data inputs=inputs prefs=takeoff_prefs/>
                </>
            }.into_any(),
            Phase::Climb => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Climb>
                    <NumField label=move || copy(locale.get()).weight.to_string()
                        get=move || inputs.get().weight_lb
                        set=move |v| inputs.update(|i| {
                            let kg_min = da20_core::chart_bounds::limits::FIG54_WEIGHT_KG.0;
                            let kg_max = da20_core::chart_bounds::limits::FIG54_WEIGHT_KG.1;
                            let lb_min = kg_min / 0.45359237;
                            let lb_max = kg_max / 0.45359237;
                            i.weight_lb = v.clamp(lb_min, lb_max);
                        })/>
                    <ClimbFlapToggle locale=locale inputs=inputs data=data/>
                    <SharedPropellerSelector locale=locale inputs=inputs/>
                    <SharedWheelFairingToggle locale=locale inputs=inputs/>
                    {move || {
                        let c = copy(locale.get());
                        (!inputs.get().wheel_fairings_installed).then(|| view! {
                            <p class="lab warn afm-quote mono">{c.afm_wheel_fairings_note_climb}</p>
                        })
                    }}
                </PhaseGrid>
            }.into_any(),
            Phase::Cruise => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Cruise airfield=false>
                    <SharedPropellerSelector locale=locale inputs=inputs/>
                    <SharedWheelFairingToggle locale=locale inputs=inputs/>
                    <CruiseInputs locale=locale inputs=inputs/>
                </PhaseGrid>
            }.into_any(),
            Phase::Endurance => view! {
                <PhaseGrid locale=locale inputs=inputs data=data store=store phase=Phase::Endurance airfield=false>
                    <SharedPropellerSelector locale=locale inputs=inputs/>
                    <SharedWheelFairingToggle locale=locale inputs=inputs/>
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
                    <SharedPropellerSelector locale=locale inputs=inputs/>
                    <SharedWheelFairingToggle locale=locale inputs=inputs/>
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
    #[prop(optional)] takeoff_prefs: Option<RwSignal<TakeoffPrefs>>,
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
                            mode=airfield_mode_for(phase)
                        />
                    })}
                    {(!airfield).then(|| view! { <span></span> })}
                    {children()}
                </div>
                <div class="card chart-card">
                    <h3>{phase_title(locale, phase, inputs)}</h3>
                    {phase_chart(phase, locale, data, inputs, store, overlay_ids)}
                    {move || phase_results(locale.get(), phase, bundle.get(), inputs.get(), takeoff_prefs.map(|p| p.get()))}
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

fn airfield_mode_for(phase: Phase) -> AirfieldInputsMode {
    match phase {
        Phase::Takeoff => AirfieldInputsMode::Takeoff,
        Phase::Climb => AirfieldInputsMode::Climb,
        _ => AirfieldInputsMode::Full,
    }
}

fn phase_title(
    locale: RwSignal<Locale>,
    phase: Phase,
    inputs: RwSignal<da20_core::types::FlightInputs>,
) -> impl IntoView {
    move || {
        let c = copy(locale.get());
        match phase {
            Phase::Takeoff => c.takeoff.into(),
            Phase::Climb => c.climb.into(),
            Phase::Cruise => {
                let sub = match inputs.get().propeller {
                    PropellerKind::Mt175R => c.cruise_subtitle_mt,
                    PropellerKind::Sensenich => c.cruise_subtitle_table3,
                };
                format!("{} · {sub}", c.cruise)
            }
            Phase::Endurance => c.tab_endurance.into(),
            Phase::Landing => c.landing.into(),
            Phase::Manual => String::new(),
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
            {move || match inputs.get().propeller {
                PropellerKind::Mt175R => view! {
                    <MtCruisePanel locale=locale data=data inputs=inputs/>
                }.into_any(),
                PropellerKind::Sensenich => view! {
                    <CruiseTableView locale=locale data=data inputs=inputs/>
                }.into_any(),
            }}
        }
        .into_any(),
        Phase::Endurance => view! {
            <p class="chip">{move || copy(locale.get()).derived_not_afm}</p>
        }
        .into_any(),
        Phase::Landing => view! {
            <LandingTableView locale=locale data=data inputs=inputs/>
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
    takeoff_prefs: Option<TakeoffPrefs>,
) -> impl IntoView {
    let c = copy(locale);
    match phase {
        Phase::Takeoff => {
            let runway_ok =
                runway_heading_status(&inputs.runway) == RunwayHeadingStatus::Valid;
            view! {
            {if runway_ok {
                match &b.takeoff {
                Some(t) => {
                    let rounding = takeoff_prefs
                        .map(|p| p.distance_rounding)
                        .unwrap_or(TakeoffDistanceRounding::Raw);
                    let dist = apply_takeoff_distance_rounding(t, rounding);
                    let (gr_line, to_line) = format_takeoff_distances_pair(
                        locale,
                        dist.ground_roll_m,
                        dist.ground_roll_ft,
                        dist.over_obstacle_m,
                        dist.over_obstacle_ft,
                    );
                    view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=t.provenance.clone()/>
                        <p class="chip">{afm_lookup_tag_label(locale, t.afm_lookup_tag)}</p>
                        <p class="readout-text">{gr_line}</p>
                        <p class="readout-text">{to_line}</p>
                        {(rounding == TakeoffDistanceRounding::Raw).then(|| view! {
                            <p class="readout-text">{format_takeoff_conservative_estimate(
                                locale,
                                t.distance_over_obstacle_conservative_ft,
                                t.distance_over_obstacle_conservative_m,
                            )}</p>
                        })}
                        <p class="lab">{format_obstacle_over(locale, inputs.obstacle_height_m)}</p>
                        <WarningsList locale=locale warnings=t.warnings.clone()/>
                        {(inputs.propeller == PropellerKind::Mt175R).then(|| view! {
                            <p class="lab afm-quote">{c.propeller_no_change_note}</p>
                        })}
                    </div>
                    }.into_any()
                },
                None => {
                    if let Some(ooc) = &b.takeoff_ooc {
                        view! { <p class="error warn">{format_out_of_chart(locale, ooc)}</p> }.into_any()
                    } else {
                        view! { <p class="error">"—"</p> }.into_any()
                    }
                }
            }} else {
                view! { <p class="lab results-muted">{c.results_runway_required}</p> }.into_any()
            }}
            {runway_ok.then(|| match &b.wind {
                Some(w) => {
                    let along = format_along_runway_component(locale, &w.components);
                    let cross = format_crosswind_component(locale, w.components.crosswind_kt);
                    view! {
                        <div class="wind-components">
                            {along.map(|line| view! { <p class="readout-text">{line}</p> })}
                            <p class="readout-text">{cross}</p>
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
                Some(cl) => {
                    let c_loc = copy(locale);
                    let chart_roc = cl.rate_of_climb_chart_fpm;
                    let penalized = cl.rate_of_climb_fpm;
                    let fair_on = inputs.wheel_fairings_installed;
                    let chart_i = chart_roc.map(|x| x.round() as i64).unwrap_or(penalized.round() as i64);
                    let adj_i = penalized.round() as i64;
                    view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=cl.provenance.clone()/>
                        <p class="chip">{afm_lookup_tag_label(locale, cl.afm_lookup_tag)}</p>
                        {if fair_on {
                            view! {
                                <p class="big">{chart_i}<span class="u">" fpm"</span></p>
                            }.into_any()
                        } else if chart_roc.is_some() {
                            view! {
                                <>
                                    <p class="big mono">{chart_i}" fpm "{c_loc.climb_roc_fairing_penalty_suffix}</p>
                                    <p class="big mono">{c_loc.climb_roc_adjusted_label}": "{adj_i}<span class="u">" fpm"</span></p>
                                </>
                            }.into_any()
                        } else {
                            view! {
                                <p class="big mono">{adj_i}" fpm "{c_loc.climb_roc_fairing_penalty_suffix}</p>
                            }.into_any()
                        }}
                        <p>"EA "{cl.equivalent_altitude_ft.round() as i64}" ft"</p>
                        <WarningsList locale=locale warnings=cl.warnings.clone()/>
                        {(inputs.propeller == PropellerKind::Mt175R).then(|| view! {
                            <p class="lab afm-quote">{c_loc.propeller_no_change_note}</p>
                        })}
                    </div>
                    }.into_any()
                },
                None => {
                    if let Some(ooc) = &b.climb_ooc {
                        view! { <p class="error warn">{format_out_of_chart(locale, ooc)}</p> }.into_any()
                    } else {
                        view! { <p class="error">"—"</p> }.into_any()
                    }
                }
            }}
        }.into_any(),
        Phase::Cruise => view! {
            {match &b.cruise {
                Some(cr) => {
                    let c_loc = copy(locale);
                    let tas_c = cr.tas_chart_kt;
                    let pen = cr.tas_kt;
                    let ff = if cr.mt_gph_out_of_chart {
                        None
                    } else {
                        Some(cr.fuel_flow_us_gal_h)
                    };
                    let gph_ooc = cr.mt_gph_out_of_chart;
                    let fair_on = inputs.wheel_fairings_installed;
                    let t_chart = tas_c.map(|x| x.round() as i64).unwrap_or(pen.round() as i64);
                    let t_pen = pen.round() as i64;
                    let bhp = cr.percent_bhp.unwrap_or(0.0);
                    let rpm_out = cr.rpm.map(|x| x.round() as i64).unwrap_or(0);
                    let bhp_mode = inputs.cruise_solve_mode == CruiseSolveMode::SetPercentBhp;
                    let requested_bhp = inputs.cruise_percent_bhp;
                    let mt_prop = inputs.propeller == PropellerKind::Mt175R;
                    view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=cr.provenance.clone()/>
                        {mt_prop.then(|| view! {
                            <p class="chip warn">{c_loc.mt_tcca_badge}</p>
                        })}
                        {mt_prop.then(|| cr.density_altitude_ft.map(|da| view! {
                            <p class="readout-text">{c_loc.density_altitude_readout}": "{da.round() as i64}" ft"</p>
                        }))}
                        <p class="chip">{afm_lookup_tag_label(locale, cr.afm_lookup_tag)}</p>
                        {if bhp_mode {
                            if fair_on {
                                view! {
                                    <>
                                        <p class="big mono">{format!("{requested_bhp:.2}")}" % BHP"</p>
                                        <p class="big mono">{rpm_out}" RPM · "{t_chart}<span class="u">" kt TAS"</span></p>
                                        <p class="mono">{if let Some(ff) = ff { format!("{ff:.1} US gal/h") } else { "— US gal/h".into() }}</p>
                                    </>
                                }.into_any()
                            } else if tas_c.is_some() {
                                view! {
                                    <>
                                        <p class="big mono">{format!("{requested_bhp:.2}")}" % BHP"</p>
                                        <p class="big mono">{rpm_out}" RPM · "{t_chart}" kt TAS "{c_loc.cruise_tas_fairing_penalty_suffix}</p>
                                        <p class="big mono">{c_loc.cruise_tas_adjusted_label}": "{t_pen}<span class="u">" kt TAS"</span></p>
                                        <p class="mono">{if let Some(ff) = ff { format!("{ff:.1} US gal/h") } else { "— US gal/h".into() }}</p>
                                    </>
                                }.into_any()
                            } else {
                                view! {
                                    <>
                                        <p class="big mono">{format!("{requested_bhp:.2}")}" % BHP"</p>
                                        <p class="big mono">{rpm_out}" RPM · "{t_pen}" kt TAS "{c_loc.cruise_tas_fairing_penalty_suffix}</p>
                                        <p class="mono">{if let Some(ff) = ff { format!("{ff:.1} US gal/h") } else { "— US gal/h".into() }}</p>
                                    </>
                                }.into_any()
                            }
                        } else if fair_on {
                            view! {
                                <>
                                    <p class="big mono">{format!("{bhp:.2}")}" % BHP · "{t_chart}<span class="u">" kt TAS"</span></p>
                                    <p>{if let Some(ff) = ff { format!("{ff:.1} US gal/h") } else { "— US gal/h".into() }}</p>
                                </>
                            }.into_any()
                        } else if tas_c.is_some() {
                            view! {
                                <>
                                    <p class="big mono">{format!("{bhp:.2}")}" % BHP · "{t_chart}" kt TAS "{c_loc.cruise_tas_fairing_penalty_suffix}</p>
                                    <p class="big mono">{c_loc.cruise_tas_adjusted_label}": "{t_pen}<span class="u">" kt TAS"</span></p>
                                    <p>{if let Some(ff) = ff { format!("{ff:.1} US gal/h") } else { "— US gal/h".into() }}</p>
                                </>
                            }.into_any()
                        } else {
                            view! {
                                <>
                                    <p class="big mono">{format!("{bhp:.2}")}" % BHP · "{t_pen}" kt TAS "{c_loc.cruise_tas_fairing_penalty_suffix}</p>
                                    <p>{if let Some(ff) = ff { format!("{ff:.1} US gal/h") } else { "— US gal/h".into() }}</p>
                                </>
                            }.into_any()
                        }}
                        {mt_prop.then(|| cr.mt_fig4_ktas_kt.map(|kt| view! {
                            <p class="readout-text mono">{format!("{}: {:.0} kt", c_loc.mt_fig4_ktas_compare, kt)}</p>
                        }))}
                        {gph_ooc.then(|| b.cruise_ooc.as_ref().map(|ooc| view! {
                            <p class="error warn">{format_out_of_chart(locale, ooc)}</p>
                        }))}
                        <WarningsList locale=locale warnings=cr.warnings.clone()/>
                        {if mt_prop {
                            view! {
                                <MtFig2Fig3SourcePanel
                                    locale=locale
                                    fig2=cr.mt_fig2_source.clone()
                                    fig3=cr.mt_fig3_source.clone()
                                />
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if mt_prop {
                            cr.mt_fig4_source.clone().map(|trace| {
                                view! { <MtFig4SourcePanel locale=locale trace=trace /> }
                            }).into_any()
                        } else {
                            cr.table3_source.clone().map(|trace| {
                                view! { <Table3SourcePanel locale=locale trace=trace /> }
                            }).into_any()
                        }}
                    </div>
                    }.into_any()
                },
                None => {
                    let c_loc = copy(locale);
                    let mt_prop = inputs.propeller == PropellerKind::Mt175R;
                    if let Some(ooc) = &b.cruise_ooc {
                        let msg = if mt_prop
                            && inputs.cruise_solve_mode == CruiseSolveMode::SetPercentBhp
                            && ooc.label == "Fig 2 %BHP"
                        {
                            format_mt_cruise_bhp_not_achievable(
                                locale,
                                inputs.cruise_percent_bhp,
                                45.0,
                                75.0,
                            )
                        } else {
                            format_out_of_chart(locale, ooc)
                        };
                        let bhp_line = (inputs.cruise_solve_mode == CruiseSolveMode::SetPercentBhp)
                            .then(|| {
                                view! {
                                    <p class="big mono">{format!("{:.2}", inputs.cruise_percent_bhp)}" % BHP"</p>
                                }
                            });
                        view! {
                            <div class="results-block">
                                {bhp_line}
                                <p class="error warn">{msg}</p>
                            </div>
                        }
                        .into_any()
                    } else if b.cruise_bhp_not_achievable
                        && inputs.cruise_solve_mode == CruiseSolveMode::SetPercentBhp
                        && !mt_prop
                    {
                        view! {
                            <div class="results-block">
                                <p class="big mono">{format!("{:.2}", inputs.cruise_percent_bhp)}" % BHP"</p>
                                <p class="error warn">{c_loc.cruise_bhp_not_achievable}</p>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <p class="error">"—"</p> }.into_any()
                    }
                }
            }}
        }.into_any(),
        Phase::Endurance => view! {
            {match &b.endurance {
                Some(e) => {
                    let loc = locale;
                    let c_loc = copy(loc);
                    let fair_on = inputs.wheel_fairings_installed;
                    let h = e.endurance_h;
                    let g = e.fuel_flow_us_gal_h;
                    let r_pen = e.range_nm;
                    let r_chart = e.range_chart_nm;
                    let t_pen = e.tas_kt;
                    let t_chart = e.tas_chart_kt;
                    view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=e.provenance.clone()/>
                        <p class="chip">{copy(locale).derived_not_afm}</p>
                        <p class="chip">{afm_lookup_tag_label(loc, e.afm_lookup_tag)}</p>
                        <p class="big">{format!("{h:.1}")}<span class="u">" h"</span></p>
                        {if fair_on {
                            let rc = r_chart.unwrap_or(r_pen).round() as i64;
                            let tc = t_chart.unwrap_or(t_pen).round() as i64;
                            view! {
                                <p>{format!("{rc}")}" NM · "{format!("{g:.1}")}" gph @ "{tc}" kt"</p>
                            }.into_any()
                        } else if t_chart.is_some() && r_chart.is_some() {
                            let rc = r_chart.unwrap().round() as i64;
                            let rp = r_pen.round() as i64;
                            let tc = t_chart.unwrap().round() as i64;
                            let tp = t_pen.round() as i64;
                            view! {
                                <>
                                    <p class="mono">{format!("{rc}")}" NM · "{format!("{g:.1}")}" gph @ "{tc}" kt "{c_loc.cruise_tas_fairing_penalty_suffix}</p>
                                    <p class="mono">{c_loc.endurance_range_adjusted_label}": "{format!("{rp}")}" NM @ "{tp}" kt"</p>
                                </>
                            }.into_any()
                        } else {
                            let rp = r_pen.round() as i64;
                            let tp = t_pen.round() as i64;
                            view! {
                                <p class="mono">{format!("{rp}")}" NM · "{format!("{g:.1}")}" gph @ "{tp}" kt "{c_loc.cruise_tas_fairing_penalty_suffix}</p>
                            }.into_any()
                        }}
                        <WarningsList locale=locale warnings=e.warnings.clone()/>
                    </div>
                    }.into_any()
                },
                None => view! { <p class="error">"—"</p> }.into_any(),
            }}
        }.into_any(),
        Phase::Landing => view! {
            {match &b.landing {
                Some(l) => view! {
                    <div class="results-block">
                        <ProvenanceLine locale=locale provenance=l.provenance.clone()/>
                        <p class="chip">{afm_lookup_tag_label(locale, l.afm_lookup_tag)}</p>
                        <p class="big">{l.ground_roll_ft.round() as i64}<span class="u">" ft"</span></p>
                        <p>"50 ft: "{l.distance_over_50ft_ft.round() as i64}" ft"</p>
                        <WarningsList locale=locale warnings=l.warnings.clone()/>
                        {(inputs.propeller == PropellerKind::Mt175R).then(|| view! {
                            <p class="lab afm-quote">{c.propeller_no_change_note}</p>
                        })}
                        {l.table4_source.clone().map(|trace| {
                            view! { <Table4SourcePanel locale=locale trace=trace /> }
                        })}
                    </div>
                }.into_any(),
                None => {
                    if let Some(ooc) = &b.landing_ooc {
                        view! { <p class="error warn">{format_out_of_chart(locale, ooc)}</p> }.into_any()
                    } else {
                        view! { <p class="error">"—"</p> }.into_any()
                    }
                }
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
