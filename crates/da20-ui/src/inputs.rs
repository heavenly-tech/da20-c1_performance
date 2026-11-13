use da20_core::altitude::isa_troposphere_density_altitude_ft;
use da20_core::altitude::{clear_pa_override, effective_pressure_altitude_ft, set_pa_override};
use da20_core::chart_bounds::{limits, CHART_EDGE_TOL};
use da20_core::cruise::isa_temp_c_at_pa;
use da20_core::types::CruiseSolveMode;
use da20_core::types::{ChartPhase, ClimbFigure, FlightInputs, PropellerKind};
use da20_core::wind::{
    apply_signed_along_runway_wind, runway_heading_deg, runway_heading_status,
    signed_along_runway_kt, wind_chart_panel_needs_crosswind_note, wind_components,
    RunwayHeadingStatus,
};
use da20_core::wind_format::{format_wind, parse_wind};
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::ground_oat::display_disa_c;
use crate::i18n::{copy, format_along_runway_component, format_crosswind_component, Locale};
use crate::phase_state::{apply_cruise_ground_elevation, apply_cruise_ground_oat, cruise_tab_mut};
use crate::phases::Phase;
use crate::prefs::{
    apply_estimated_cruise_oat, clamp_cruise_oat_for_propeller, persist_climb_tab_from_inputs,
    persist_landing_tab_from_inputs, persist_manual_cruise_oat, persist_takeoff_tab_from_inputs,
    snapshot_and_save_flight_prefs, FlightPrefs,
};

/// Takeoff vs climb vs landing (full) airfield blocks.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AirfieldInputsMode {
    #[default]
    Full,
    Takeoff,
    Climb,
    Landing,
}

pub fn shared_airfield_writes_tab_prefs(mode: AirfieldInputsMode) -> bool {
    matches!(
        mode,
        AirfieldInputsMode::Takeoff | AirfieldInputsMode::Climb | AirfieldInputsMode::Landing
    )
}

/// Strip chart + SharedAirfieldInputs use the same mode/phase pairing.
pub fn strip_airfield_target(phase: Phase) -> (AirfieldInputsMode, Option<ChartPhase>) {
    match phase {
        Phase::Takeoff => (AirfieldInputsMode::Takeoff, Some(ChartPhase::Takeoff)),
        Phase::Climb => (AirfieldInputsMode::Climb, Some(ChartPhase::Climb)),
        Phase::Landing => (AirfieldInputsMode::Landing, Some(ChartPhase::Landing)),
        _ => (AirfieldInputsMode::Full, None),
    }
}

/// Same path as the takeoff weight NumField after chart drag.
pub fn takeoff_set_weight_lb(prefs: &mut FlightPrefs, inputs: &mut FlightInputs, lb: f64) {
    let (kg_min, kg_max) = limits::FIG54_WEIGHT_KG;
    let lb_min = kg_min / 0.45359237;
    let lb_max = kg_max / 0.45359237;
    inputs.weight_lb = lb.clamp(lb_min, lb_max);
    prefs.takeoff.weight_lb = inputs.weight_lb;
}

/// Same path as the takeoff obstacle NumField after chart drag.
pub fn takeoff_set_obstacle_height_m(
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    height_m: f64,
) {
    inputs.obstacle_height_m = height_m.clamp(0.0, da20_core::types::defaults::OBSTACLE_HEIGHT_MAX_M);
    prefs.takeoff.obstacle_height_m = inputs.obstacle_height_m;
}

/// Wind strip drag — mirrors post-drag wind correction on takeoff inputs.
pub fn takeoff_apply_wind_from_strip(
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    wind_dir_deg: f64,
    wind_speed_kt: f64,
    crosswind_clamped: bool,
) {
    inputs.wind_dir_deg = wind_dir_deg;
    inputs.wind_speed_kt = wind_speed_kt;
    inputs.wind_strip_crosswind_clamped = crosswind_clamped;
    clamp_along_runway_wind(inputs);
    prefs.takeoff.wind_dir_deg = inputs.wind_dir_deg;
    prefs.takeoff.wind_speed_kt = inputs.wind_speed_kt;
}

pub fn shared_airfield_pressure_altitude_display(
    inputs: &FlightInputs,
    chart_phase: Option<ChartPhase>,
) -> f64 {
    if chart_phase.is_some() {
        effective_pressure_altitude_ft(inputs, chart_phase)
    } else {
        inputs.pressure_altitude_input_ft
    }
}

/// Same logic as the SharedAirfieldInputs PA field setter (takeoff ground prefs vs chart override).
pub fn shared_airfield_set_pressure_altitude(
    mode: AirfieldInputsMode,
    chart_phase: Option<ChartPhase>,
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    pa_ft: f64,
) {
    match mode {
        AirfieldInputsMode::Takeoff => {
            prefs.takeoff.pressure_altitude_ft = pa_ft;
            inputs.pressure_altitude_input_ft = pa_ft;
            clear_pa_override(inputs);
        }
        AirfieldInputsMode::Climb => {
            if let Some(phase) = chart_phase.filter(|p| *p == ChartPhase::Climb) {
                set_pa_override(inputs, pa_ft, phase);
                prefs.climb.pa_override = inputs.pa_override;
            } else {
                prefs.climb.pressure_altitude_ft = pa_ft;
                inputs.pressure_altitude_input_ft = pa_ft;
                clear_pa_override(inputs);
                prefs.climb.pa_override = None;
            }
        }
        AirfieldInputsMode::Landing => {
            if let Some(phase) = chart_phase.filter(|p| *p == ChartPhase::Landing) {
                set_pa_override(inputs, pa_ft, phase);
                prefs.landing.pa_override = inputs.pa_override;
            } else {
                prefs.landing.pressure_altitude_ft = pa_ft;
                inputs.pressure_altitude_input_ft = pa_ft;
                clear_pa_override(inputs);
                prefs.landing.pa_override = None;
            }
        }
        AirfieldInputsMode::Full => {
            inputs.pressure_altitude_input_ft = pa_ft;
            clear_pa_override(inputs);
        }
    }
}

/// Same logic as the SharedAirfieldInputs OAT field setter.
pub fn shared_airfield_set_oat_c(
    mode: AirfieldInputsMode,
    prefs: &mut FlightPrefs,
    inputs: &mut FlightInputs,
    oat_c: f64,
) {
    inputs.oat_c = oat_c;
    match mode {
        AirfieldInputsMode::Takeoff => prefs.takeoff.oat_c = oat_c,
        AirfieldInputsMode::Climb => prefs.climb.oat_c = oat_c,
        AirfieldInputsMode::Landing => prefs.landing.oat_c = oat_c,
        AirfieldInputsMode::Full => {}
    }
}

pub(crate) const CHART_CLAMP_UNSET: f64 = f64::NAN;

/// Fig 5.4 panel: tailwind up to 5 kt (signed −5), headwind up to 20 kt.
fn clamp_along_runway_wind(inputs: &mut da20_core::types::FlightInputs) {
    if runway_heading_status(&inputs.runway) != RunwayHeadingStatus::Valid {
        return;
    }
    let Some(signed) = signed_along_runway_kt(inputs) else {
        return;
    };
    let lo = -limits::FIG54_TAILWIND_KT.1;
    let hi = limits::FIG54_HEADWIND_KT.1;
    let clamped = signed.clamp(lo, hi);
    if (clamped - signed).abs() > 1e-9 {
        apply_signed_along_runway_wind(inputs, clamped);
    }
}

fn clamp_chart(v: f64, min: f64, max: f64) -> f64 {
    let mut x = v;
    if min.is_finite() {
        x = x.max(min);
    }
    if max.is_finite() {
        x = x.min(max);
    }
    x
}

#[component]
pub fn NumField(
    label: impl Fn() -> String + Send + 'static,
    get: impl Fn() -> f64 + Send + 'static,
    set: impl Fn(f64) + Send + Copy + 'static,
    #[prop(default = CHART_CLAMP_UNSET)] min: f64,
    #[prop(default = CHART_CLAMP_UNSET)] max: f64,
    /// Display formatting only; stored value keeps full precision.
    #[prop(default = None)]
    display_fraction_digits: Option<u32>,
) -> impl IntoView {
    view! {
        <div class="f">
            <label>{move || label()}</label>
            <input type="number" step="any" class="mono"
                min=min.is_finite().then_some(min)
                max=max.is_finite().then_some(max)
                prop:value=move || match display_fraction_digits {
                    Some(d) => format!("{:.*}", d as usize, get()),
                    None => format!("{}", get()),
                }
                on:input=move |ev| {
                    if let Ok(v) = input_value(&ev).parse() {
                        set(clamp_chart(v, min, max));
                    }
                }
            />
        </div>
    }
}

#[component]
pub fn CruiseGroundInputs(
    editor_phase: Phase,
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            let phase = editor_phase;
            let ground_elev = move || match phase {
                Phase::Cruise => flight_prefs.get().cruise.ground_elevation_ft,
                Phase::Endurance => flight_prefs.get().endurance.cruise.ground_elevation_ft,
                _ => flight_prefs.get().cruise.ground_elevation_ft,
            };
            let ground_oat = move || match phase {
                Phase::Cruise => flight_prefs.get().cruise.ground_oat_c,
                Phase::Endurance => flight_prefs.get().endurance.cruise.ground_oat_c,
                _ => flight_prefs.get().cruise.ground_oat_c,
            };
            view! {
                <p class="lab"><strong>{c.ground_section_title}</strong></p>
                <NumField label=move || c.ground_elevation_ft.to_string()
                    min=limits::FIG52_PA_FT.0
                    max=limits::FIG52_PA_FT.1
                    get=ground_elev
                    set=move |v| {
                        flight_prefs.update(|p| {
                            inputs.update(|i| apply_cruise_ground_elevation(phase, p, i, v));
                        });
                    }/>
                <NumField label=move || c.ground_oat_c.to_string()
                    display_fraction_digits=Some(1)
                    min=limits::FIG52_OAT_C.0
                    max=limits::FIG52_OAT_C.1
                    get=ground_oat
                    set=move |v| {
                        flight_prefs.update(|p| {
                            inputs.update(|i| apply_cruise_ground_oat(phase, p, i, v));
                        });
                    }/>
            }
        }}
    }
}

#[component]
pub fn SharedAirfieldInputs(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
    chart_phase: Option<ChartPhase>,
    #[prop(default = AirfieldInputsMode::Full)] mode: AirfieldInputsMode,
    #[prop(default = false)] hide_pa_oat: bool,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            let i = inputs.get();
            let pa = effective_pressure_altitude_ft(&i, chart_phase);
            let override_active = chart_phase.is_some_and(|p| {
                i.pa_override.as_ref().is_some_and(|o| o.phase == p)
            });
            let (pa_min, pa_max, oat_min, oat_max) = match chart_phase {
                Some(ChartPhase::Takeoff) => (
                    limits::FIG54_PA_FT.0,
                    limits::FIG54_PA_FT.1,
                    limits::FIG54_OAT_C.0,
                    limits::FIG54_OAT_C.1,
                ),
                Some(ChartPhase::Landing) => (
                    limits::TABLE4_PA_FT.0,
                    limits::TABLE4_PA_FT.1,
                    CHART_CLAMP_UNSET,
                    CHART_CLAMP_UNSET,
                ),
                Some(ChartPhase::Climb) | None => match mode {
                    AirfieldInputsMode::Takeoff => (
                        limits::FIG54_PA_FT.0,
                        limits::FIG54_PA_FT.1,
                        limits::FIG54_OAT_C.0,
                        limits::FIG54_OAT_C.1,
                    ),
                    AirfieldInputsMode::Climb => (
                        limits::FIG52_PA_FT.0,
                        limits::FIG52_PA_FT.1,
                        limits::FIG52_OAT_C.0,
                        limits::FIG52_OAT_C.1,
                    ),
                    AirfieldInputsMode::Full | AirfieldInputsMode::Landing => (
                        limits::FIG52_PA_FT.0,
                        limits::FIG52_PA_FT.1,
                        limits::FIG52_OAT_C.0,
                        limits::FIG52_OAT_C.1,
                    ),
                },
            };
            view! {
                {(!hide_pa_oat).then(|| view! {
                    <NumField label=move || c.pressure_altitude_ft.to_string()
                        min=pa_min
                        max=pa_max
                        get=move || {
                            shared_airfield_pressure_altitude_display(&inputs.get(), chart_phase)
                        }
                        set=move |v| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| {
                                    shared_airfield_set_pressure_altitude(
                                        mode,
                                        chart_phase,
                                        p,
                                        i,
                                        v,
                                    );
                                });
                                if shared_airfield_writes_tab_prefs(mode) {
                                    p.save();
                                }
                            });
                        }/>
                    <NumField label=move || c.oat.to_string()
                        min=oat_min
                        max=oat_max
                        get=move || inputs.get().oat_c
                        set=move |v| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| shared_airfield_set_oat_c(mode, p, i, v));
                                if shared_airfield_writes_tab_prefs(mode) {
                                    p.save();
                                }
                            });
                        }/>
                })}
                {(mode == AirfieldInputsMode::Takeoff).then(|| {
                    view! {
                        <div class="f">
                            <label>{c.runway}</label>
                            <input class="mono" maxlength="3"
                                prop:value=move || inputs.get().runway.clone()
                                on:input=move |ev| {
                                    let raw = input_value(&ev);
                                    flight_prefs.update(|p| {
                                        inputs.update(|i| {
                                            i.runway = raw;
                                            persist_takeoff_tab_from_inputs(p, i);
                                        });
                                    });
                                }/>
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
                        <WindField locale=locale inputs=inputs flight_prefs=flight_prefs/>
                    }
                })}
                {(!override_active).then(|| view! {
                    <p class="lab readout-text">
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
                                on:click=move |_| {
                                    flight_prefs.update(|p| {
                                        inputs.update(|i| {
                                            clear_pa_override(i);
                                            match mode {
                                                AirfieldInputsMode::Climb => {
                                                    persist_climb_tab_from_inputs(p, i);
                                                }
                                                AirfieldInputsMode::Landing => {
                                                    persist_landing_tab_from_inputs(p, i);
                                                }
                                                _ => {}
                                            }
                                        });
                                    });
                                }>
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
pub fn SharedPropellerSelector(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            view! {
                <div class="f wheel-fairings-shared">
                    <label>{c.propeller_toggle_label}</label>
                    <div class="toggle-row wheel-fairings-row">
                        <button type="button" class="btn g"
                            class:on=move || inputs.get().propeller == PropellerKind::Sensenich
                            on:click=move |_| inputs.update(|i| {
                                i.propeller = PropellerKind::Sensenich;
                                flight_prefs.update(|p| snapshot_and_save_flight_prefs(i, p));
                            })>
                            {c.propeller_sensenich}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || inputs.get().propeller == PropellerKind::Mt175R
                            on:click=move |_| inputs.update(|i| {
                                i.propeller = PropellerKind::Mt175R;
                                flight_prefs.update(|p| snapshot_and_save_flight_prefs(i, p));
                            })>
                            {c.propeller_mt175r}
                        </button>
                    </div>
                </div>
            }
        }}
    }
}

#[component]
pub fn SharedLandingGroundIdleToggle(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            let on = inputs.get().landing_ground_idle_1000_rpm;
            view! {
                <div class="f">
                    <label>{c.landing_ground_idle_toggle}</label>
                    <div class="toggle-row wheel-fairings-row">
                        <button type="button" class="btn g"
                            class:on=move || !on
                            on:click=move |_| {
                                flight_prefs.update(|p| {
                                    inputs.update(|i| {
                                        i.landing_ground_idle_1000_rpm = false;
                                        persist_landing_tab_from_inputs(p, i);
                                    });
                                });
                            }>
                            {c.landing_ground_idle_off}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || on
                            on:click=move |_| {
                                flight_prefs.update(|p| {
                                    inputs.update(|i| {
                                        i.landing_ground_idle_1000_rpm = true;
                                        persist_landing_tab_from_inputs(p, i);
                                    });
                                });
                            }>
                            {c.landing_ground_idle_on}
                        </button>
                    </div>
                </div>
            }
        }}
    }
}

/// Wheel fairings: one shared `FlightInputs` flag (climb −3% ROC; cruise −5% TAS per AFM).
#[component]
pub fn SharedWheelFairingToggle(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
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
                            on:click=move |_| inputs.update(|i| {
                                i.wheel_fairings_installed = true;
                                flight_prefs.update(|p| snapshot_and_save_flight_prefs(i, p));
                            })>
                            {c.wheel_fairings_installed_yes}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || !inputs.get().wheel_fairings_installed
                            on:click=move |_| inputs.update(|i| {
                                i.wheel_fairings_installed = false;
                                flight_prefs.update(|p| snapshot_and_save_flight_prefs(i, p));
                            })>
                            {c.wheel_fairings_installed_no}
                        </button>
                    </div>
                </div>
            }
        }}
    }
}

#[component]
pub fn CruiseInputs(
    editor_phase: Phase,
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
) -> impl IntoView {
    let phase = editor_phase;
    let set_cruise_oat_from_isa = move |deviation: f64| {
        flight_prefs.update(|p| {
            let tab = cruise_tab_mut(p, phase);
            tab.cruise_oat_manual = true;
            inputs.update(|i| {
                let isa = isa_temp_c_at_pa(i.cruise_pressure_alt_ft);
                i.cruise_oat_c = clamp_cruise_oat_for_propeller(i, isa + deviation);
            });
            persist_manual_cruise_oat(tab, &inputs.get_untracked());
            p.save();
        });
    };
    view! {
        {move || {
            let c = copy(locale.get());
            let mt = inputs.get().propeller == PropellerKind::Mt175R;
            let manual_oat = match phase {
                Phase::Cruise => flight_prefs.get().cruise.cruise_oat_manual,
                Phase::Endurance => flight_prefs.get().endurance.cruise.cruise_oat_manual,
                _ => false,
            };
            let disa = display_disa_c(
                inputs.get().cruise_oat_c,
                inputs.get().cruise_pressure_alt_ft,
            );
            view! {
                <NumField label=move || c.cruise_pa.to_string()
                    min=limits::TABLE3_PA_FT.0
                    max=limits::TABLE3_PA_FT.1
                    get=move || inputs.get().cruise_pressure_alt_ft
                    set=move |v| {
                        flight_prefs.update(|p| {
                            let tab = cruise_tab_mut(p, phase);
                            inputs.update(|i| {
                                i.cruise_pressure_alt_ft = v;
                                tab.cruise_pressure_alt_ft = v;
                                if !tab.cruise_oat_manual {
                                    apply_estimated_cruise_oat(tab, i);
                                } else if i.propeller == PropellerKind::Sensenich {
                                    let isa = isa_temp_c_at_pa(v);
                                    let (dmin, dmax) = limits::TABLE3_DISA_C;
                                    i.cruise_oat_c =
                                        i.cruise_oat_c.clamp(isa + dmin, isa + dmax);
                                }
                            });
                            p.save();
                        });
                    }/>
                <NumField label=move || {
                    if manual_oat {
                        c.cruise_oat.to_string()
                    } else {
                        c.cruise_oat_estimated_label.to_string()
                    }
                }
                    display_fraction_digits=Some(1)
                    min=limits::FIG52_OAT_C.0
                    max=limits::FIG52_OAT_C.1
                    get=move || inputs.get().cruise_oat_c
                    set=move |v| {
                        flight_prefs.update(|p| {
                            let tab = cruise_tab_mut(p, phase);
                            tab.cruise_oat_manual = true;
                            inputs.update(|i| {
                                i.cruise_oat_c = clamp_cruise_oat_for_propeller(i, v);
                            });
                            persist_manual_cruise_oat(tab, &inputs.get_untracked());
                            p.save();
                        });
                    }/>
                <p class="lab readout-text">
                    {c.cruise_disa_readout}": "{format!("{:.1}", disa)}" °C"
                </p>
                {manual_oat.then(|| view! {
                    <button type="button" class="btn g"
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                let tab = cruise_tab_mut(p, phase);
                                tab.cruise_oat_manual = false;
                                tab.cruise_oat_c_saved = None;
                                inputs.update(|i| apply_estimated_cruise_oat(tab, i));
                                p.save();
                            });
                        }>
                        {c.cruise_oat_use_estimate}
                    </button>
                })}
                {mt.then(|| {
                    let da = isa_troposphere_density_altitude_ft(
                        inputs.get().cruise_pressure_alt_ft,
                        inputs.get().cruise_oat_c,
                    );
                    let (lo, hi) = limits::FIG4_DA_FT;
                    let da_txt = if da >= lo - CHART_EDGE_TOL && da <= hi + CHART_EDGE_TOL {
                        format!("{}", da.round() as i64)
                    } else {
                        "—".into()
                    };
                    view! {
                        <p class="lab readout-text">{c.density_altitude_readout}": "{da_txt}" ft"</p>
                    }
                })}
                <div class="toggle-row">
                    <button type="button" class="btn g"
                        class:on=move || inputs.get().cruise_solve_mode == CruiseSolveMode::SetRpm
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| i.cruise_solve_mode = CruiseSolveMode::SetRpm);
                                cruise_tab_mut(p, phase)
                                    .mark_cruise_solve_saved(CruiseSolveMode::SetRpm);
                                p.save();
                            });
                        }>
                        {c.cruise_mode_set_rpm}
                    </button>
                    <button type="button" class="btn g"
                        class:on=move || inputs.get().cruise_solve_mode == CruiseSolveMode::SetPercentBhp
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| {
                                    i.cruise_solve_mode = CruiseSolveMode::SetPercentBhp
                                });
                                cruise_tab_mut(p, phase)
                                    .mark_cruise_solve_saved(CruiseSolveMode::SetPercentBhp);
                                p.save();
                            });
                        }>
                        {c.cruise_mode_set_bhp}
                    </button>
                </div>
                {move || {
                    if inputs.get().cruise_solve_mode == CruiseSolveMode::SetRpm {
                        view! {
                            <NumField label=move || c.cruise_rpm.to_string()
                                get=move || inputs.get().cruise_rpm
                                set=move |v| {
                                    flight_prefs.update(|p| {
                                        inputs.update(|i| {
                                            i.cruise_rpm = v;
                                            cruise_tab_mut(p, phase).cruise_rpm = v;
                                        });
                                        p.save();
                                    });
                                }/>
                        }.into_any()
                    } else {
                        view! {
                            <NumField label=move || c.cruise_pct_bhp.to_string()
                                get=move || inputs.get().cruise_percent_bhp
                                set=move |v| {
                                    flight_prefs.update(|p| {
                                        inputs.update(|i| i.cruise_percent_bhp = v);
                                        cruise_tab_mut(p, phase).mark_cruise_percent_saved(v);
                                        p.save();
                                    });
                                }/>
                        }.into_any()
                    }
                }}
                {move || {
                    let c = copy(locale.get());
                    let installed = inputs.get().wheel_fairings_installed;
                    (!installed).then(|| view! {
                        <p class="lab warn afm-quote">{c.afm_wheel_fairings_note_cruise}</p>
                    })
                }}
                {(!mt).then(|| view! {
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
                })}
            }
        }}
    }
}

#[component]
pub fn WindField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    #[prop(optional)] flight_prefs: Option<RwSignal<FlightPrefs>>,
) -> impl IntoView {
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
                        if let Some(fp) = flight_prefs {
                            fp.update(|p| {
                                inputs.update(|i| {
                                    i.wind_dir_deg = d;
                                    i.wind_speed_kt = s;
                                    i.wind_strip_crosswind_clamped = false;
                                    clamp_along_runway_wind(i);
                                    persist_takeoff_tab_from_inputs(p, i);
                                });
                            });
                        } else {
                            inputs.update(|i| {
                                i.wind_dir_deg = d;
                                i.wind_speed_kt = s;
                                i.wind_strip_crosswind_clamped = false;
                                clamp_along_runway_wind(i);
                            });
                        }
                    }
                }
                on:blur=move |ev| {
                    let raw = input_value(&ev);
                    if let Some((d, s)) = parse_wind(&raw) {
                        if let Some(fp) = flight_prefs {
                            fp.update(|p| {
                                inputs.update(|i| {
                                    i.wind_dir_deg = d;
                                    i.wind_speed_kt = s;
                                    i.wind_strip_crosswind_clamped = false;
                                    clamp_along_runway_wind(i);
                                    persist_takeoff_tab_from_inputs(p, i);
                                });
                            });
                        } else {
                            inputs.update(|i| {
                                i.wind_dir_deg = d;
                                i.wind_speed_kt = s;
                                i.wind_strip_crosswind_clamped = false;
                                clamp_along_runway_wind(i);
                            });
                        }
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
                        {along.map(|line| view! { <p class="lab readout-text">{line}</p> })}
                        {(comp.crosswind_kt.abs() > 0.05).then(|| view! {
                            <p class="lab readout-text">{cross}</p>
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
    flight_prefs: RwSignal<FlightPrefs>,
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
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| {
                                    i.climb_figure = ClimbFigure::Fig56TakeoffFlaps;
                                    persist_climb_tab_from_inputs(p, i);
                                });
                            });
                        }>
                        {c.climb_flaps_to}
                    </button>
                    <button type="button"
                        class="btn g"
                        class:on=move || inputs.get().climb_figure == ClimbFigure::Fig55CruiseFlaps
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| {
                                    i.climb_figure = ClimbFigure::Fig55CruiseFlaps;
                                    persist_climb_tab_from_inputs(p, i);
                                });
                            });
                        }>
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
                                flight_prefs.update(|p| {
                                    inputs.update(|i| {
                                        i.climb_figure = ClimbFigure::Fig58BalkedLanding;
                                        persist_climb_tab_from_inputs(p, i);
                                    });
                                });
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

#[cfg(test)]
mod airfield_setter_tests {
    use super::*;
    use crate::phase_state::{assemble_takeoff_inputs, hydrate_tab_inputs};
    use crate::phases::Phase;
    use crate::prefs::FlightPrefs;
    use da20_core::altitude::effective_pressure_altitude_ft;
    use da20_core::takeoff::compute_takeoff;
    use da20_core::PerformanceData;

    fn seeded_prefs() -> (FlightPrefs, FlightInputs) {
        let mut prefs = FlightPrefs::default();
        prefs.takeoff.pressure_altitude_ft = 2130.0;
        prefs.takeoff.oat_c = 20.0;
        prefs.cruise.ground_elevation_ft = 2130.0;
        prefs.cruise.ground_oat_c = 20.0;
        let data = PerformanceData::production();
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Climb, &prefs, &data, &mut inputs);
        (prefs, inputs)
    }

    #[test]
    fn climb_pa_setter_leaves_takeoff_prefs_and_takeoff_compute() {
        let data = PerformanceData::production();
        let (mut prefs, mut inputs) = seeded_prefs();
        let takeoff_roll_before = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;

        shared_airfield_set_pressure_altitude(
            AirfieldInputsMode::Climb,
            Some(ChartPhase::Climb),
            &mut prefs,
            &mut inputs,
            8000.0,
        );

        assert_eq!(prefs.takeoff.pressure_altitude_ft, 2130.0);
        assert_eq!(prefs.takeoff.oat_c, 20.0);
        assert_eq!(
            effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Climb)),
            8000.0
        );
        let takeoff_roll_after = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        assert!((takeoff_roll_before - takeoff_roll_after).abs() < f64::EPSILON);
    }

    #[test]
    fn landing_pa_setter_leaves_takeoff_prefs() {
        let (mut prefs, mut inputs) = seeded_prefs();

        shared_airfield_set_pressure_altitude(
            AirfieldInputsMode::Landing,
            Some(ChartPhase::Landing),
            &mut prefs,
            &mut inputs,
            3500.0,
        );

        assert_eq!(prefs.takeoff.pressure_altitude_ft, 2130.0);
        assert_eq!(
            effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Landing)),
            3500.0
        );
        assert_eq!(
            prefs.landing.pa_override.as_ref().map(|o| o.pa_ft),
            Some(3500.0)
        );
    }

    #[test]
    fn landing_oat_setter_updates_landing_not_takeoff() {
        let (mut prefs, mut inputs) = seeded_prefs();

        shared_airfield_set_oat_c(AirfieldInputsMode::Landing, &mut prefs, &mut inputs, -5.0);

        assert_eq!(prefs.takeoff.oat_c, 20.0);
        assert_eq!(prefs.landing.oat_c, -5.0);
        assert_eq!(inputs.oat_c, -5.0);
    }
}
