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

use crate::i18n::{copy, format_along_runway_component, format_crosswind_component, Locale};
use crate::prefs::save_flight_prefs_from_inputs;

/// Takeoff vs climb vs landing (full) airfield blocks.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AirfieldInputsMode {
    #[default]
    Full,
    Takeoff,
    Climb,
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

pub fn clamp_chart(v: f64, min: f64, max: f64) -> f64 {
    let mut x = v;
    if min.is_finite() {
        x = x.max(min);
    }
    if max.is_finite() {
        x = x.min(max);
    }
    x
}

/// Keystroke handler: store raw draft only; do not parse, clamp, or rewrite display.
pub fn note_num_draft(draft: &mut String, raw: &str) {
    *draft = raw.to_string();
}

pub fn format_num_display(value: f64, display_fraction_digits: Option<u32>) -> String {
    match display_fraction_digits {
        Some(d) => format!("{:.*}", d as usize, value),
        None => format!("{}", value),
    }
}

pub fn num_field_display(
    editing: bool,
    draft: &str,
    value: f64,
    display_fraction_digits: Option<u32>,
) -> String {
    if editing {
        draft.to_string()
    } else {
        format_num_display(value, display_fraction_digits)
    }
}

/// Parse and clamp on commit (blur / Enter / change). Empty input is ignored.
pub fn commit_num_raw(raw: &str, min: f64, max: f64) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let v = trimmed.parse::<f64>().ok()?;
    Some(clamp_chart(v, min, max))
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
    let editing = RwSignal::new(false);
    // Draft is non-reactive so keystrokes do not re-render (and remount) the <input>.
    let draft = StoredValue::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();

    // While focused the browser owns the text; prop:value rewrites mid-keystroke
    // (especially clamps and `{:.1}`) freeze multi-digit entry.
    Effect::new(move |_| {
        if editing.get() {
            return;
        }
        let Some(el) = input_ref.get() else {
            return;
        };
        let text = num_field_display(false, "", get(), display_fraction_digits);
        el.set_value(&text);
    });

    let commit_and_close = move |raw: String| {
        if let Some(v) = commit_num_raw(&raw, min, max) {
            set(v);
        }
        editing.set(false);
    };

    view! {
        <div class="f">
            <label>{move || label()}</label>
            <input node_ref=input_ref type="text" inputmode="decimal" autocomplete="off" class="mono"
                on:focus=move |_| editing.set(true)
                on:input=move |ev| {
                    let mut next = String::new();
                    note_num_draft(&mut next, &input_value(&ev));
                    draft.set_value(next);
                }
                on:blur=move |ev| commit_and_close(input_value(&ev))
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        commit_and_close(input_value(&ev));
                    }
                }
            />
        </div>
    }
}

fn airfield_chart_limits(
    chart_phase: Option<ChartPhase>,
    mode: AirfieldInputsMode,
) -> (f64, f64, f64, f64) {
    match chart_phase {
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
            AirfieldInputsMode::Full => (
                limits::FIG52_PA_FT.0,
                limits::FIG52_PA_FT.1,
                limits::FIG52_OAT_C.0,
                limits::FIG52_OAT_C.1,
            ),
        },
    }
}

#[component]
fn SharedAirfieldPaReadout(
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
pub fn SharedAirfieldInputs(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    chart_phase: Option<ChartPhase>,
    #[prop(default = AirfieldInputsMode::Full)] mode: AirfieldInputsMode,
) -> impl IntoView {
    let (pa_min, pa_max, oat_min, oat_max) = airfield_chart_limits(chart_phase, mode);
    let show_runway_wind = mode == AirfieldInputsMode::Takeoff || mode == AirfieldInputsMode::Full;
    view! {
        <NumField label=move || copy(locale.get()).pressure_altitude_ft.to_string()
            min=pa_min
            max=pa_max
            get=move || inputs.get().pressure_altitude_input_ft
            set=move |v| {
                inputs.update(|i| {
                    i.pressure_altitude_input_ft = v;
                    clear_pa_override(i);
                });
            }/>
        <NumField label=move || copy(locale.get()).oat.to_string()
            min=oat_min
            max=oat_max
            get=move || inputs.get().oat_c
            set=move |v| inputs.update(|i| i.oat_c = v)/>
        {show_runway_wind.then(|| {
            view! {
                <div class="f">
                    <label>{move || copy(locale.get()).runway}</label>
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
        <SharedAirfieldPaReadout locale=locale inputs=inputs chart_phase=chart_phase/>
    }
}

#[component]
pub fn SharedPropellerSelector(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
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
                                save_flight_prefs_from_inputs(i);
                            })>
                            {c.propeller_sensenich}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || inputs.get().propeller == PropellerKind::Mt175R
                            on:click=move |_| inputs.update(|i| {
                                i.propeller = PropellerKind::Mt175R;
                                save_flight_prefs_from_inputs(i);
                            })>
                            {c.propeller_mt175r}
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
                                save_flight_prefs_from_inputs(i);
                            })>
                            {c.wheel_fairings_installed_yes}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || !inputs.get().wheel_fairings_installed
                            on:click=move |_| inputs.update(|i| {
                                i.wheel_fairings_installed = false;
                                save_flight_prefs_from_inputs(i);
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
fn CruiseDensityAltitudeReadout(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
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
    let cruise_mode = Memo::new(move |_| inputs.get().cruise_solve_mode);
    let is_mt = Memo::new(move |_| inputs.get().propeller == PropellerKind::Mt175R);
    let fairings_off = Memo::new(move |_| !inputs.get().wheel_fairings_installed);

    view! {
        <NumField label=move || copy(locale.get()).cruise_pa.to_string()
            min=limits::TABLE3_PA_FT.0
            max=limits::TABLE3_PA_FT.1
            get=move || inputs.get().cruise_pressure_alt_ft
            set=move |v| inputs.update(|i| {
                i.cruise_pressure_alt_ft = v;
                if i.propeller == PropellerKind::Sensenich {
                    let isa = isa_temp_c_at_pa(v);
                    let (dmin, dmax) = limits::TABLE3_DISA_C;
                    i.cruise_oat_c = i.cruise_oat_c.clamp(isa + dmin, isa + dmax);
                }
            })/>
        <NumField label=move || copy(locale.get()).cruise_oat.to_string()
            display_fraction_digits=Some(1)
            min=limits::FIG52_OAT_C.0
            max=limits::FIG52_OAT_C.1
            get=move || inputs.get().cruise_oat_c
            set=move |v| inputs.update(|i| {
                if i.propeller == PropellerKind::Mt175R {
                    i.cruise_oat_c = v.clamp(limits::FIG52_OAT_C.0, limits::FIG52_OAT_C.1);
                } else {
                    let isa = isa_temp_c_at_pa(i.cruise_pressure_alt_ft);
                    let (dmin, dmax) = limits::TABLE3_DISA_C;
                    i.cruise_oat_c = v.clamp(isa + dmin, isa + dmax);
                }
            })/>
        <Show when=move || is_mt.get()>
            <CruiseDensityAltitudeReadout locale=locale inputs=inputs/>
        </Show>
        <div class="toggle-row">
            <button type="button" class="btn g"
                class:on=move || cruise_mode.get() == CruiseSolveMode::SetRpm
                on:click=move |_| inputs.update(|i| i.cruise_solve_mode = CruiseSolveMode::SetRpm)>
                {move || copy(locale.get()).cruise_mode_set_rpm}
            </button>
            <button type="button" class="btn g"
                class:on=move || cruise_mode.get() == CruiseSolveMode::SetPercentBhp
                on:click=move |_| inputs.update(|i| i.cruise_solve_mode = CruiseSolveMode::SetPercentBhp)>
                {move || copy(locale.get()).cruise_mode_set_bhp}
            </button>
        </div>
        <Show
            when=move || cruise_mode.get() == CruiseSolveMode::SetRpm
            fallback=move || view! {
                <NumField label=move || copy(locale.get()).cruise_pct_bhp.to_string()
                    get=move || inputs.get().cruise_percent_bhp
                    set=move |v| inputs.update(|i| i.cruise_percent_bhp = v)/>
            }
        >
            <NumField label=move || copy(locale.get()).cruise_rpm.to_string()
                get=move || inputs.get().cruise_rpm
                set=move |v| inputs.update(|i| i.cruise_rpm = v)/>
        </Show>
        <Show when=move || fairings_off.get()>
            <p class="lab warn afm-quote">{move || copy(locale.get()).afm_wheel_fairings_note_cruise}</p>
        </Show>
        <Show when=move || !is_mt.get()>
            <div class="toggle-row">
                <button type="button" class="btn g"
                    on:click=move |_| set_cruise_oat_from_isa(0.0)>
                    {move || copy(locale.get()).cruise_isa}
                </button>
                <button type="button" class="btn g"
                    on:click=move |_| set_cruise_oat_from_isa(-20.0)>
                    {move || copy(locale.get()).cruise_disa_minus20}
                </button>
                <button type="button" class="btn g"
                    on:click=move |_| set_cruise_oat_from_isa(10.0)>
                    {move || copy(locale.get()).cruise_disa_plus10}
                </button>
            </div>
        </Show>
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
                            clamp_along_runway_wind(i);
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
                            clamp_along_runway_wind(i);
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

#[cfg(test)]
mod num_field_tests {
    use super::*;
    use da20_core::chart_bounds::limits;

    fn legacy_num_keystroke(
        start: f64,
        min: f64,
        max: f64,
        typed: &str,
        display_fraction_digits: Option<u32>,
    ) -> (Vec<String>, f64) {
        let mut v = start;
        let mut shown = Vec::new();
        let mut field = String::new();
        for ch in typed.chars() {
            field.push(ch);
            if let Ok(parsed) = field.parse::<f64>() {
                v = clamp_chart(parsed, min, max);
            }
            field = format_num_display(v, display_fraction_digits);
            shown.push(field.clone());
        }
        (shown, v)
    }

    fn type_num_draft(
        start: f64,
        min: f64,
        max: f64,
        typed: &str,
        display_fraction_digits: Option<u32>,
    ) -> (Vec<String>, f64) {
        let mut v = start;
        let mut draft = format_num_display(v, display_fraction_digits);
        let mut shown = Vec::new();
        let mut buf = String::new();
        for ch in typed.chars() {
            buf.push(ch);
            note_num_draft(&mut draft, &buf);
            shown.push(num_field_display(true, &draft, v, display_fraction_digits));
        }
        if let Some(next) = commit_num_raw(&draft, min, max) {
            v = next;
        }
        shown.push(num_field_display(false, &draft, v, display_fraction_digits));
        (shown, v)
    }

    #[test]
    fn typing_multi_digit_keeps_every_character_before_commit() {
        let unset = CHART_CLAMP_UNSET;
        let (shown, v) = type_num_draft(0.0, unset, unset, "65", None);
        assert_eq!(&shown[..2], ["6", "65"]);
        assert!((v - 65.0).abs() < 1e-9);

        let (shown, v) = type_num_draft(0.0, unset, unset, "2450", None);
        assert_eq!(&shown[..4], ["2", "24", "245", "2450"]);
        assert!((v - 2450.0).abs() < 1e-9);
    }

    #[test]
    fn cruise_pa_does_not_clamp_mid_typing() {
        let (min, max) = limits::TABLE3_PA_FT;
        let (shown, v) = type_num_draft(2000.0, min, max, "5000", None);
        assert_eq!(&shown[..4], ["5", "50", "500", "5000"]);
        assert!((v - 5000.0).abs() < 1e-9);

        let (legacy_shown, legacy_v) = legacy_num_keystroke(2000.0, min, max, "5000", None);
        assert_eq!(legacy_shown.first().map(String::as_str), Some("2000"));
        assert!(
            (legacy_v - 5000.0).abs() > 1e-9,
            "legacy per-keystroke clamp/format stored {legacy_v}"
        );
    }

    #[test]
    fn cruise_oat_one_decimal_does_not_rewrite_prefix() {
        let (min, max) = (limits::FIG52_OAT_C.0, limits::FIG52_OAT_C.1);
        let (shown, v) = type_num_draft(15.0, min, max, "15", Some(1));
        assert_eq!(&shown[..2], ["1", "15"]);
        assert_eq!(shown.last().map(String::as_str), Some("15.0"));
        assert!((v - 15.0).abs() < 1e-9);

        let (legacy_shown, _) = legacy_num_keystroke(15.0, min, max, "15", Some(1));
        assert_eq!(legacy_shown.first().map(String::as_str), Some("1.0"));
    }

    #[test]
    fn out_of_range_clamps_only_on_commit() {
        let (min, max) = limits::TABLE3_PA_FT;
        let (_, v) = type_num_draft(5000.0, min, max, "99999", None);
        assert!((v - max).abs() < 1e-9);

        let (_, v) = type_num_draft(5000.0, min, max, "1", None);
        assert!((v - min).abs() < 1e-9);
    }
}
