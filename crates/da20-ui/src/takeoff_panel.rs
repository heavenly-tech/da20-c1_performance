//! Takeoff numeric inputs (unit prefs), distance rounding, and parameter sweep table.

use da20_core::chart_bounds::ChartOutOfRange;
use da20_core::rev29::engine::{ft_to_m, kg_to_lb, lb_to_kg, m_to_ft};
use da20_core::takeoff_rounding::{apply_takeoff_distance_rounding, TakeoffDistanceRounding};
use da20_core::takeoff_sweep::{
    compute_takeoff_sweep, TakeoffSweepAxis, TakeoffSweepError, TakeoffSweepRow, TakeoffSweepVar,
};
use da20_core::types::{defaults, FlightInputs};
use da20_core::wind::{runway_heading_status, signed_along_runway_kt, RunwayHeadingStatus};
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::chart_field_ooc::{field_ooc_active, takeoff_wind_axis_limits, ChartInputField};
use crate::i18n::{afm_lookup_tag_label, copy, format_takeoff_sweep_var_obs, Locale};
use crate::inputs::{format_num_display, input_value, note_num_draft, NumField};
use crate::prefs::{
    persist_takeoff_tab_from_inputs, FlightPrefs, ObstacleUnit, TakeoffPrefs, WeightUnit,
};

/// Commit a typed takeoff weight field value (kg or lb display) into `weight_lb`.
/// Accepts a decimal point or comma. Does not rewrite the in-progress draft.
pub fn apply_takeoff_weight_field_value(weight_lb: &mut f64, unit: WeightUnit, raw: &str) -> bool {
    let normalized = raw.trim().replace(',', ".");
    let v: f64 = match normalized.parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => v,
        _ => return false,
    };
    *weight_lb = match unit {
        WeightUnit::Kg => kg_to_lb(v),
        WeightUnit::Lb => v,
    };
    true
}

/// Keystroke handler for the weight field. Stores the raw draft and does not
/// touch `weight_lb`. Committing each prefix and rewriting `{:.2}` (the old
/// `prop:value` + `on:input` loop) freezes the field on the first digit.
pub fn note_takeoff_weight_draft(draft: &mut String, raw: &str) {
    *draft = raw.to_string();
}

pub fn takeoff_weight_field_display(
    editing: bool,
    draft: &str,
    weight_lb: f64,
    unit: WeightUnit,
) -> String {
    if editing {
        draft.to_string()
    } else {
        format_takeoff_weight_display(weight_lb, unit)
    }
}

fn format_takeoff_weight_display(weight_lb: f64, unit: WeightUnit) -> String {
    let v = match unit {
        WeightUnit::Kg => lb_to_kg(weight_lb),
        WeightUnit::Lb => weight_lb,
    };
    format!("{v:.2}")
}

#[derive(Clone, Copy, Default)]
struct SweepVarDraft {
    enabled: bool,
    start: f64,
    end: f64,
    step: f64,
}

/// Sweep obstacle axis values are stored in metres (core unit).
pub fn obstacle_sweep_display_value(value_m: f64, unit: ObstacleUnit) -> f64 {
    match unit {
        ObstacleUnit::Meters => value_m,
        ObstacleUnit::Feet => m_to_ft(value_m),
    }
}

pub fn obstacle_sweep_commit_display(display: f64, unit: ObstacleUnit) -> f64 {
    let m = match unit {
        ObstacleUnit::Meters => display,
        ObstacleUnit::Feet => ft_to_m(display),
    };
    if m.is_finite() {
        m.max(0.0)
    } else {
        0.0
    }
}

/// Max shown in obstacle labels/errors (matches `TakeoffObstacleUnitField` 0–15 m / 0–50 ft).
pub fn obstacle_unit_range_display_max(unit: ObstacleUnit) -> f64 {
    match unit {
        ObstacleUnit::Meters => defaults::OBSTACLE_HEIGHT_MAX_M,
        ObstacleUnit::Feet => 50.0,
    }
}

/// Avoid re-committing a 1-decimal display value on blur (ft→m drift).
pub fn obstacle_sweep_commit_if_changed(current_m: f64, display: f64, unit: ObstacleUnit) -> f64 {
    let current_display = obstacle_sweep_display_value(current_m, unit);
    let formatted = format_num_display(current_display, Some(1));
    let Ok(formatted_val) = formatted.parse::<f64>() else {
        return obstacle_sweep_commit_display(display, unit);
    };
    if (display - formatted_val).abs() < 1e-9 {
        return current_m;
    }
    obstacle_sweep_commit_display(display, unit)
}

pub fn format_sweep_obstacle_cell(value_m: f64, unit: ObstacleUnit) -> String {
    match unit {
        ObstacleUnit::Meters => format!("{:.1}", value_m),
        ObstacleUnit::Feet => format!("{:.1}", m_to_ft(value_m)),
    }
}

#[component]
pub fn TakeoffNumericBlock(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
    flight_prefs: RwSignal<FlightPrefs>,
) -> impl IntoView {
    let takeoff_ooc =
        use_context::<Memo<Option<ChartOutOfRange>>>().unwrap_or_else(|| Memo::new(move |_| None));
    view! {
        <TakeoffWeightField locale=locale inputs=inputs prefs=prefs flight_prefs=flight_prefs takeoff_ooc=takeoff_ooc/>
        <TakeoffSignedHeadwindField locale=locale inputs=inputs flight_prefs=flight_prefs takeoff_ooc=takeoff_ooc/>
        <TakeoffObstacleUnitField locale=locale inputs=inputs prefs=prefs flight_prefs=flight_prefs takeoff_ooc=takeoff_ooc/>
        <TakeoffRoundingToggle locale=locale prefs=prefs/>
    }
}

fn commit_takeoff_weight_raw(
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
    flight_prefs: RwSignal<FlightPrefs>,
    raw: &str,
) {
    let unit = prefs.get_untracked().weight;
    flight_prefs.update(|p| {
        inputs.update(|i| {
            apply_takeoff_weight_field_value(&mut i.weight_lb, unit, raw);
            persist_takeoff_tab_from_inputs(p, i);
        });
    });
}

#[component]
fn TakeoffWeightField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
    flight_prefs: RwSignal<FlightPrefs>,
    takeoff_ooc: Memo<Option<ChartOutOfRange>>,
) -> impl IntoView {
    let weight_ooc = Memo::new(move |_| {
        field_ooc_active(takeoff_ooc.get().as_ref(), ChartInputField::TakeoffWeight)
    });
    let editing = RwSignal::new(false);
    let draft = RwSignal::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();

    // While the field is focused the browser owns the text. Writing `prop:value`
    // on each key (especially `{:.2}` on `type=number`) rewrote "800" into "8.00".
    Effect::new(move |_| {
        let el = input_ref.get();
        let editing_now = editing.get();
        let unit = prefs.get().weight;
        let text = takeoff_weight_field_display(false, "", inputs.get().weight_lb, unit);
        if editing_now {
            return;
        }
        if let Some(el) = el {
            el.set_value(&text);
        }
    });

    let commit_and_close = move |raw: String| {
        commit_takeoff_weight_raw(inputs, prefs, flight_prefs, &raw);
        editing.set(false);
    };

    view! {
        <div class="f" class:out-of-chart=move || weight_ooc.get()>
            <label>
                {move || {
                    let c = copy(locale.get());
                    match prefs.get().weight {
                        WeightUnit::Kg => c.takeoff_weight_kg.to_string(),
                        WeightUnit::Lb => c.weight.to_string(),
                    }
                }}
            </label>
            <div class="unit-row">
                <input node_ref=input_ref type="text" inputmode="decimal" autocomplete="off" class="mono"
                    on:focus=move |_| {
                        editing.set(true);
                    }
                    on:input=move |ev| {
                        let mut next = String::new();
                        note_takeoff_weight_draft(&mut next, &input_value(&ev));
                        draft.set(next);
                    }
                    on:blur=move |ev| {
                        commit_and_close(input_value(&ev));
                    }
                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            commit_and_close(input_value(&ev));
                        }
                    }
                />
                <div class="toggle-row unit-toggle">
                    <button type="button" class="btn g"
                        class:on=move || prefs.get().weight == WeightUnit::Kg
                        on:click=move |_| {
                            if editing.get_untracked() {
                                if let Some(el) = input_ref.get_untracked() {
                                    commit_takeoff_weight_raw(inputs, prefs, flight_prefs, &el.value());
                                }
                                editing.set(false);
                            }
                            prefs.update(|p| { p.weight = WeightUnit::Kg; p.save(); });
                        }>
                        {move || copy(locale.get()).unit_kg.to_string()}
                    </button>
                    <button type="button" class="btn g"
                        class:on=move || prefs.get().weight == WeightUnit::Lb
                        on:click=move |_| {
                            if editing.get_untracked() {
                                if let Some(el) = input_ref.get_untracked() {
                                    commit_takeoff_weight_raw(inputs, prefs, flight_prefs, &el.value());
                                }
                                editing.set(false);
                            }
                            prefs.update(|p| { p.weight = WeightUnit::Lb; p.save(); });
                        }>
                        {move || copy(locale.get()).unit_lb.to_string()}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn TakeoffSignedHeadwindField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    flight_prefs: RwSignal<FlightPrefs>,
    takeoff_ooc: Memo<Option<ChartOutOfRange>>,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let draft = RwSignal::new(String::new());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let wind_ooc = Memo::new(move |_| {
        field_ooc_active(
            takeoff_ooc.get().as_ref(),
            ChartInputField::TakeoffSignedWind,
        )
    });

    Effect::new(move |_| {
        if editing.get() {
            return;
        }
        let Some(el) = input_ref.get() else {
            return;
        };
        let text = if runway_heading_status(&inputs.get().runway) != RunwayHeadingStatus::Valid {
            String::new()
        } else {
            signed_along_runway_kt(&inputs.get())
                .map(|v| format!("{v:.1}"))
                .unwrap_or_default()
        };
        el.set_value(&text);
    });

    let commit = move |raw: String| {
        let Ok(v) = raw.trim().replace(',', ".").parse::<f64>() else {
            editing.set(false);
            return;
        };
        if !v.is_finite() {
            editing.set(false);
            return;
        }
        flight_prefs.update(|p| {
            inputs.update(|i| {
                da20_core::wind::apply_signed_along_runway_wind(i, v);
                persist_takeoff_tab_from_inputs(p, i);
            });
        });
        editing.set(false);
    };

    view! {
        <div class="f" class:out-of-chart=move || wind_ooc.get()>
            <label>{move || copy(locale.get()).takeoff_signed_headwind.to_string()}</label>
            <input node_ref=input_ref type="text" inputmode="decimal" autocomplete="off" class="mono"
                on:focus=move |_| editing.set(true)
                on:input=move |ev| {
                    let mut next = String::new();
                    note_num_draft(&mut next, &input_value(&ev));
                    draft.set(next);
                }
                on:blur=move |ev| commit(input_value(&ev))
                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        commit(input_value(&ev));
                    }
                }
            />
            {move || wind_ooc.get().then(|| view! {
                <p class="lab field-error">{takeoff_wind_axis_limits(locale.get())}</p>
            })}
        </div>
    }
}

#[component]
fn TakeoffObstacleUnitField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
    flight_prefs: RwSignal<FlightPrefs>,
    takeoff_ooc: Memo<Option<ChartOutOfRange>>,
) -> impl IntoView {
    let obstacle_ooc = Memo::new(move |_| {
        field_ooc_active(takeoff_ooc.get().as_ref(), ChartInputField::TakeoffObstacle)
    });
    view! {
        {move || {
            let c = copy(locale.get());
            let unit = prefs.get().obstacle;
            let label = match unit {
                ObstacleUnit::Meters => format!("{} (0–15)", c.obstacle_m),
                ObstacleUnit::Feet => format!("{} (0–50)", c.obstacle_ft),
            };
            view! {
                <div class="f" class:out-of-chart=move || obstacle_ooc.get()>
                    <label>{label}</label>
                    <div class="unit-row">
                        <input type="number" step="any" class="mono"
                            min=0.0
                            prop:value=move || {
                                let unit = prefs.get().obstacle;
                                let v = match unit {
                                    ObstacleUnit::Meters => inputs.get().obstacle_height_m,
                                    ObstacleUnit::Feet => m_to_ft(inputs.get().obstacle_height_m),
                                };
                                format!("{v:.1}")
                            }
                            on:input=move |ev| {
                                if let Ok(v) = input_value(&ev).parse::<f64>() {
                                    if !v.is_finite() {
                                        return;
                                    }
                                    flight_prefs.update(|p| {
                                        inputs.update(|i| {
                                            let m = match prefs.get_untracked().obstacle {
                                                ObstacleUnit::Meters => v,
                                                ObstacleUnit::Feet => ft_to_m(v),
                                            };
                                            i.obstacle_height_m = m.max(0.0);
                                            persist_takeoff_tab_from_inputs(p, i);
                                        });
                                    });
                                }
                            }
                        />
                        <div class="toggle-row unit-toggle">
                            <button type="button" class="btn g"
                                class:on=move || prefs.get().obstacle == ObstacleUnit::Meters
                                on:click=move |_| prefs.update(|p| { p.obstacle = ObstacleUnit::Meters; p.save(); })>
                                {c.unit_m}
                            </button>
                            <button type="button" class="btn g"
                                class:on=move || prefs.get().obstacle == ObstacleUnit::Feet
                                on:click=move |_| prefs.update(|p| { p.obstacle = ObstacleUnit::Feet; p.save(); })>
                                {c.unit_ft}
                            </button>
                        </div>
                    </div>
                </div>
                <div class="toggle-row">
                    <button type="button" class="btn g"
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| {
                                    i.obstacle_height_m = defaults::OBSTACLE_PRESET_AFM_EXAMPLE_M;
                                    persist_takeoff_tab_from_inputs(p, i);
                                });
                            });
                        }>
                        {c.obstacle_preset_afm}
                    </button>
                    <button type="button" class="btn g"
                        on:click=move |_| {
                            flight_prefs.update(|p| {
                                inputs.update(|i| {
                                    i.obstacle_height_m = defaults::OBSTACLE_HEIGHT_DEFAULT_M;
                                    persist_takeoff_tab_from_inputs(p, i);
                                });
                            });
                        }>
                        {c.obstacle_preset_screen}
                    </button>
                </div>
            }
        }}
    }
}

#[component]
fn TakeoffRoundingToggle(locale: RwSignal<Locale>, prefs: RwSignal<TakeoffPrefs>) -> impl IntoView {
    view! {
        <div class="f">
            <label>{move || copy(locale.get()).takeoff_rounding_label.to_string()}</label>
            <div class="toggle-row">
                {move || {
                    let c = copy(locale.get());
                    view! {
                        <button type="button" class="btn g"
                            class:on=move || prefs.get().distance_rounding == TakeoffDistanceRounding::Raw
                            on:click=move |_| prefs.update(|p| {
                                p.distance_rounding = TakeoffDistanceRounding::Raw;
                                p.save();
                            })>
                            {c.takeoff_rounding_raw}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || prefs.get().distance_rounding == TakeoffDistanceRounding::Ceil10Meters
                            on:click=move |_| prefs.update(|p| {
                                p.distance_rounding = TakeoffDistanceRounding::Ceil10Meters;
                                p.save();
                            })>
                            {c.takeoff_rounding_ceil_10m}
                        </button>
                        <button type="button" class="btn g"
                            class:on=move || prefs.get().distance_rounding == TakeoffDistanceRounding::Ceil50Feet
                            on:click=move |_| prefs.update(|p| {
                                p.distance_rounding = TakeoffDistanceRounding::Ceil50Feet;
                                p.save();
                            })>
                            {c.takeoff_rounding_ceil_50ft}
                        </button>
                    }
                }}
            </div>
        </div>
    }
}

pub fn format_takeoff_distances_pair(
    locale: Locale,
    ground_m: f64,
    ground_ft: f64,
    over_m: f64,
    over_ft: f64,
) -> (String, String) {
    let c = copy(locale);
    let gr = format!(
        "{} {:.0} m / {:.0} ft",
        c.takeoff_ground_roll, ground_m, ground_ft
    );
    let to = format!(
        "{} {:.0} m / {:.0} ft",
        c.takeoff_over_obstacle, over_m, over_ft
    );
    (gr, to)
}

#[component]
pub fn TakeoffSweepPanel(
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
) -> impl IntoView {
    let oat = RwSignal::new(SweepVarDraft {
        enabled: false,
        start: 15.0,
        end: 25.0,
        step: 5.0,
    });
    let hw = RwSignal::new(SweepVarDraft {
        enabled: false,
        start: 0.0,
        end: 10.0,
        step: 5.0,
    });
    let obs = RwSignal::new(SweepVarDraft {
        enabled: false,
        start: 5.0,
        end: 15.0,
        step: 5.0,
    });
    let error = RwSignal::new(Option::<String>::None);
    let rows = RwSignal::new(Vec::<TakeoffSweepRow>::new());

    let run = move |_| {
        error.set(None);
        rows.set(Vec::new());
        let base = inputs.get();
        if runway_heading_status(&base.runway) != RunwayHeadingStatus::Valid {
            error.set(Some(sweep_err_msg(
                locale.get(),
                TakeoffSweepError::InvalidRunway,
                prefs.get_untracked().obstacle,
            )));
            return;
        }
        let Some(hw_base) = signed_along_runway_kt(&base) else {
            error.set(Some(sweep_err_msg(
                locale.get(),
                TakeoffSweepError::InvalidRunway,
                prefs.get_untracked().obstacle,
            )));
            return;
        };
        let mut axes = Vec::new();
        for (draft, var) in [
            (oat.get(), TakeoffSweepVar::OatC),
            (hw.get(), TakeoffSweepVar::SignedHeadwindKt),
            (obs.get(), TakeoffSweepVar::ObstacleHeightM),
        ] {
            if draft.enabled {
                axes.push(TakeoffSweepAxis {
                    var,
                    start: draft.start,
                    end: draft.end,
                    step: draft.step,
                });
            }
        }
        match compute_takeoff_sweep(&data.get(), &base, &axes, hw_base) {
            Ok(r) => rows.set(r),
            Err(e) => error.set(Some(sweep_err_msg(
                locale.get(),
                e,
                prefs.get_untracked().obstacle,
            ))),
        }
    };

    view! {
        <div class="card sweep-card">
            <h3>{move || copy(locale.get()).takeoff_sweep_title.to_string()}</h3>
            <SweepVarRow locale=locale label=move || copy(locale.get()).takeoff_sweep_var_oat.to_string() draft=oat/>
            <SweepVarRow locale=locale label=move || copy(locale.get()).takeoff_sweep_var_hw.to_string() draft=hw range_min=-5.0 range_max=20.0/>
            <ObstacleSweepVarRow locale=locale prefs=prefs draft=obs/>
            <button type="button" class="btn g" on:click=run>
                {move || copy(locale.get()).takeoff_sweep_run.to_string()}
            </button>
            {move || error.get().map(|e| view! { <p class="lab field-error">{e}</p> })}
            <TakeoffSweepTable locale=locale rows=rows prefs=prefs/>
        </div>
    }
}

#[component]
fn ObstacleSweepVarRow(
    locale: RwSignal<Locale>,
    prefs: RwSignal<TakeoffPrefs>,
    draft: RwSignal<SweepVarDraft>,
) -> impl IntoView {
    view! {
        <div class="sweep-var">
            <label class="sweep-check">
                <input type="checkbox"
                    prop:checked=move || draft.get().enabled
                    on:change=move |ev| {
                        let on = event_checked(&ev);
                        draft.update(|d| d.enabled = on);
                    }
                />
                {move || format_takeoff_sweep_var_obs(locale.get(), prefs.get().obstacle)}
            </label>
            {move || {
                let unit = prefs.get().obstacle;
                let min = obstacle_sweep_display_value(0.0, unit);
                let max = obstacle_sweep_display_value(defaults::OBSTACLE_HEIGHT_MAX_M, unit);
                view! {
                    <div class="sweep-range">
                        <NumField label=move || copy(locale.get()).takeoff_sweep_start.to_string()
                            min=min
                            max=max
                            display_fraction_digits=Some(1)
                            get=move || {
                                obstacle_sweep_display_value(draft.get().start, prefs.get().obstacle)
                            }
                            set=move |display| {
                                draft.update(|d| {
                                    d.start = obstacle_sweep_commit_if_changed(
                                        d.start,
                                        display,
                                        prefs.get_untracked().obstacle,
                                    );
                                });
                            }/>
                        <NumField label=move || copy(locale.get()).takeoff_sweep_end.to_string()
                            min=min
                            max=max
                            display_fraction_digits=Some(1)
                            get=move || {
                                obstacle_sweep_display_value(draft.get().end, prefs.get().obstacle)
                            }
                            set=move |display| {
                                draft.update(|d| {
                                    d.end = obstacle_sweep_commit_if_changed(
                                        d.end,
                                        display,
                                        prefs.get_untracked().obstacle,
                                    );
                                });
                            }/>
                        <NumField label=move || copy(locale.get()).takeoff_sweep_step.to_string()
                            display_fraction_digits=Some(1)
                            get=move || {
                                obstacle_sweep_display_value(draft.get().step, prefs.get().obstacle)
                            }
                            set=move |display| {
                                draft.update(|d| {
                                    d.step = obstacle_sweep_commit_if_changed(
                                        d.step,
                                        display,
                                        prefs.get_untracked().obstacle,
                                    );
                                });
                            }/>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn SweepVarRow(
    locale: RwSignal<Locale>,
    label: impl Fn() -> String + Send + 'static,
    draft: RwSignal<SweepVarDraft>,
    #[prop(default = crate::inputs::CHART_CLAMP_UNSET)] range_min: f64,
    #[prop(default = crate::inputs::CHART_CLAMP_UNSET)] range_max: f64,
) -> impl IntoView {
    view! {
        <div class="sweep-var">
            <label class="sweep-check">
                <input type="checkbox"
                    prop:checked=move || draft.get().enabled
                    on:change=move |ev| {
                        let on = event_checked(&ev);
                        draft.update(|d| d.enabled = on);
                    }
                />
                {move || label()}
            </label>
            <div class="sweep-range">
                <NumField label=move || copy(locale.get()).takeoff_sweep_start.to_string()
                    min=range_min
                    max=range_max
                    get=move || draft.get().start
                    set=move |v| draft.update(|d| d.start = v)/>
                <NumField label=move || copy(locale.get()).takeoff_sweep_end.to_string()
                    min=range_min
                    max=range_max
                    get=move || draft.get().end
                    set=move |v| draft.update(|d| d.end = v)/>
                <NumField label=move || copy(locale.get()).takeoff_sweep_step.to_string()
                    get=move || draft.get().step
                    set=move |v| draft.update(|d| d.step = v)/>
            </div>
        </div>
    }
}

#[component]
fn TakeoffSweepTable(
    locale: RwSignal<Locale>,
    rows: RwSignal<Vec<TakeoffSweepRow>>,
    prefs: RwSignal<TakeoffPrefs>,
) -> impl IntoView {
    view! {
        {move || {
            let list = rows.get();
            if list.is_empty() {
                return ().into_any();
            }
            let c = copy(locale.get());
            let mode = prefs.get().distance_rounding;
            let obs_unit = prefs.get().obstacle;
            let obs_col = format_takeoff_sweep_var_obs(locale.get(), obs_unit);
            view! {
                <div class="sweep-table-wrap">
                    <table class="sweep-table mono">
                        <thead>
                            <tr>
                                <th>{c.takeoff_sweep_var_oat}</th>
                                <th>{c.takeoff_sweep_var_hw}</th>
                                <th>{obs_col}</th>
                                <th>{c.takeoff_sweep_col_gr}</th>
                                <th>{c.takeoff_sweep_col_to}</th>
                                <th>{c.takeoff_sweep_col_tag}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {list.iter().map(|row| {
                                let loc = locale.get();
                                let Some(ref result) = row.result else {
                                    let obs_display =
                                        format_sweep_obstacle_cell(row.obstacle_height_m, obs_unit);
                                    return view! {
                                        <tr>
                                            <td>{format!("{:.1}", row.oat_c)}</td>
                                            <td>{format!("{:.1}", row.signed_headwind_kt)}</td>
                                            <td>{obs_display}</td>
                                            <td>"—"</td>
                                            <td>"—"</td>
                                            <td><span class="chip">"—"</span></td>
                                        </tr>
                                    }
                                    .into_any();
                                };
                                let rounded = apply_takeoff_distance_rounding(result, mode);
                                let tag = afm_lookup_tag_label(loc, result.afm_lookup_tag);
                                let obs_display =
                                    format_sweep_obstacle_cell(row.obstacle_height_m, obs_unit);
                                view! {
                                    <tr>
                                        <td>{format!("{:.1}", row.oat_c)}</td>
                                        <td>{format!("{:.1}", row.signed_headwind_kt)}</td>
                                        <td>{obs_display}</td>
                                        <td>{format!("{:.0} m / {:.0} ft", rounded.ground_roll_m, rounded.ground_roll_ft)}</td>
                                        <td>{format!("{:.0} m / {:.0} ft", rounded.over_obstacle_m, rounded.over_obstacle_ft)}</td>
                                        <td><span class="chip">{tag}</span></td>
                                    </tr>
                                }
                                .into_any()
                            }).collect_view()}
                        </tbody>
                    </table>
                    {move || {
                        let c = copy(locale.get());
                        if rows.get().iter().any(|r| r.result.is_none()) {
                            view! { <p class="lab results-muted">{c.takeoff_sweep_oob_footnote}</p> }.into_any()
                        } else {
                            ().into_any()
                        }
                    }}
                </div>
            }
            .into_any()
        }}
    }
}

fn sweep_err_msg(locale: Locale, err: TakeoffSweepError, obstacle_unit: ObstacleUnit) -> String {
    let obs_max_display = obstacle_unit_range_display_max(obstacle_unit);
    let obs_unit_suffix = match obstacle_unit {
        ObstacleUnit::Meters => "m",
        ObstacleUnit::Feet => "ft",
    };
    match locale {
        Locale::Es => match err {
            TakeoffSweepError::NoAxes => "Seleccione 1–3 variables para el barrido.".into(),
            TakeoffSweepError::TooManyAxes => "Máximo 3 variables.".into(),
            TakeoffSweepError::DuplicateVar => "Variable duplicada.".into(),
            TakeoffSweepError::NonFinite => "Valores no finitos.".into(),
            TakeoffSweepError::ZeroStep => "El paso no puede ser cero.".into(),
            TakeoffSweepError::RangeDirection => "Paso incompatible con desde/hasta.".into(),
            TakeoffSweepError::EmptyRange => "Rango vacío.".into(),
            TakeoffSweepError::TooManyCombinations { max } => {
                format!("Demasiadas combinaciones (máx. {max}).")
            }
            TakeoffSweepError::InvalidRunway => "Rumbo de pista inválido.".into(),
            TakeoffSweepError::ObstacleOutOfRange { max: _ } => {
                format!("Obstáculo fuera de rango (0–{obs_max_display:.0} {obs_unit_suffix}).")
            }
        },
        Locale::En => match err {
            TakeoffSweepError::NoAxes => "Select 1–3 variables to sweep.".into(),
            TakeoffSweepError::TooManyAxes => "At most 3 variables.".into(),
            TakeoffSweepError::DuplicateVar => "Duplicate variable.".into(),
            TakeoffSweepError::NonFinite => "Non-finite values.".into(),
            TakeoffSweepError::ZeroStep => "Step cannot be zero.".into(),
            TakeoffSweepError::RangeDirection => "Step sign does not match from/to.".into(),
            TakeoffSweepError::EmptyRange => "Empty range.".into(),
            TakeoffSweepError::TooManyCombinations { max } => {
                format!("Too many combinations (max {max}).")
            }
            TakeoffSweepError::InvalidRunway => "Invalid runway heading.".into(),
            TakeoffSweepError::ObstacleOutOfRange { max: _ } => {
                format!("Obstacle out of range (0–{obs_max_display:.0} {obs_unit_suffix}).")
            }
        },
    }
}

fn event_checked(ev: &leptos::ev::Event) -> bool {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.checked())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::data::PerformanceData;
    use da20_core::takeoff::compute_takeoff;
    use da20_core::takeoff_rounding::{apply_takeoff_distance_rounding, TakeoffDistanceRounding};
    use da20_core::wind::{apply_signed_along_runway_wind, signed_along_runway_kt};

    #[test]
    fn signed_headwind_sync_matches_solver() {
        let data = PerformanceData::production();
        let mut inputs = FlightInputs {
            pa_override: Some(da20_core::types::PaOverride {
                pa_ft: 1000.0,
                phase: da20_core::types::ChartPhase::Takeoff,
            }),
            oat_c: 21.0,
            weight_lb: defaults::TAKEOFF_REF_WEIGHT_KG / 0.45359237,
            runway: "360".into(),
            wind_dir_deg: 0.0,
            wind_speed_kt: 4.0,
            obstacle_height_m: 5.0,
            ..Default::default()
        };
        apply_signed_along_runway_wind(&mut inputs, 4.0);
        assert!((signed_along_runway_kt(&inputs).unwrap() - 4.0).abs() < 0.1);
        let a = compute_takeoff(&data, &inputs).unwrap();
        inputs.wind_speed_kt = 4.0;
        inputs.wind_dir_deg = 0.0;
        let b = compute_takeoff(&data, &inputs).unwrap();
        assert!((a.ground_roll_ft - b.ground_roll_ft).abs() < 1e-6);
    }

    fn legacy_keystroke_reformat(unit: WeightUnit, typed: &str) -> f64 {
        let mut lb = defaults::WEIGHT_LB;
        let mut field = String::new();
        for ch in typed.chars() {
            field.push(ch);
            apply_takeoff_weight_field_value(&mut lb, unit, &field);
            field = format_takeoff_weight_display(lb, unit);
        }
        lb
    }

    fn type_weight_draft(unit: WeightUnit, typed: &str) -> (Vec<String>, f64) {
        let start = defaults::WEIGHT_LB;
        let mut lb = start;
        let mut draft = format_takeoff_weight_display(lb, unit);
        let mut shown = Vec::new();
        let mut buf = String::new();
        for ch in typed.chars() {
            buf.push(ch);
            note_takeoff_weight_draft(&mut draft, &buf);
            assert!(
                (lb - start).abs() < 1e-9,
                "draft keystrokes must not commit {typed}"
            );
            shown.push(takeoff_weight_field_display(true, &draft, lb, unit));
        }
        assert!(apply_takeoff_weight_field_value(&mut lb, unit, &draft));
        let committed = takeoff_weight_field_display(false, &draft, lb, unit);
        shown.push(committed);
        (shown, lb)
    }

    #[test]
    fn typing_800_and_650_keeps_every_digit_in_kg_and_lb() {
        let (shown, lb) = type_weight_draft(WeightUnit::Kg, "800");
        assert_eq!(&shown[..3], ["8", "80", "800"]);
        assert!((lb_to_kg(lb) - 800.0).abs() < 0.01, "kg {}", lb_to_kg(lb));
        assert_eq!(shown.last().unwrap(), "800.00");

        let (shown, lb) = type_weight_draft(WeightUnit::Kg, "650");
        assert_eq!(&shown[..3], ["6", "65", "650"]);
        assert!((lb_to_kg(lb) - 650.0).abs() < 0.01);
        assert_eq!(shown.last().unwrap(), "650.00");

        let (shown, lb) = type_weight_draft(WeightUnit::Lb, "800");
        assert_eq!(&shown[..3], ["8", "80", "800"]);
        assert!((lb - 800.0).abs() < 0.01);
        assert_eq!(shown.last().unwrap(), "800.00");

        let (shown, lb) = type_weight_draft(WeightUnit::Lb, "650");
        assert_eq!(&shown[..3], ["6", "65", "650"]);
        assert!((lb - 650.0).abs() < 0.01);

        let stuck = legacy_keystroke_reformat(WeightUnit::Kg, "800");
        assert!(
            (lb_to_kg(stuck) - 800.0).abs() > 100.0,
            "old per-keystroke {{:.2}} rewrite stored {} kg",
            lb_to_kg(stuck)
        );
        let stuck_lb = legacy_keystroke_reformat(WeightUnit::Lb, "650");
        assert!(
            (stuck_lb - 650.0).abs() > 100.0,
            "old lb rewrite stored {stuck_lb}"
        );
    }

    #[test]
    fn weight_decimal_and_unit_toggle_round_trip() {
        let mut lb = defaults::WEIGHT_LB;
        assert!(apply_takeoff_weight_field_value(
            &mut lb,
            WeightUnit::Kg,
            "650,5"
        ));
        assert!((lb_to_kg(lb) - 650.5).abs() < 0.01);
        assert!(apply_takeoff_weight_field_value(
            &mut lb,
            WeightUnit::Kg,
            "800.25"
        ));
        assert!((lb_to_kg(lb) - 800.25).abs() < 0.01);
        let as_lb = takeoff_weight_field_display(false, "", lb, WeightUnit::Lb);
        let as_kg = takeoff_weight_field_display(false, "", lb, WeightUnit::Kg);
        assert_eq!(as_kg, "800.25");
        let mut round = lb;
        assert!(apply_takeoff_weight_field_value(
            &mut round,
            WeightUnit::Lb,
            &as_lb
        ));
        assert!((round - lb).abs() < 0.05);
        assert!(apply_takeoff_weight_field_value(
            &mut round,
            WeightUnit::Kg,
            &as_kg
        ));
        assert!((lb_to_kg(round) - 800.25).abs() < 0.01);

        let mut other = defaults::WEIGHT_LB;
        assert!(apply_takeoff_weight_field_value(
            &mut other,
            WeightUnit::Lb,
            "1763.70"
        ));
        let kg = takeoff_weight_field_display(false, "", other, WeightUnit::Kg);
        let back = takeoff_weight_field_display(false, "", other, WeightUnit::Lb);
        assert!(apply_takeoff_weight_field_value(
            &mut other,
            WeightUnit::Kg,
            &kg
        ));
        let restored = takeoff_weight_field_display(false, "", other, WeightUnit::Lb);
        assert_eq!(restored, back);
    }

    #[test]
    fn obstacle_sweep_blur_without_edit_keeps_exact_metres() {
        let m = 5.0;
        let display_ft =
            format_num_display(obstacle_sweep_display_value(m, ObstacleUnit::Feet), Some(1))
                .parse::<f64>()
                .unwrap();
        let after = obstacle_sweep_commit_if_changed(m, display_ft, ObstacleUnit::Feet);
        assert_eq!(after, m);
        let naive = obstacle_sweep_commit_display(display_ft, ObstacleUnit::Feet);
        assert!(
            (naive - m).abs() > 1e-6,
            "naive commit drifts: {naive} vs {m}"
        );
    }

    #[test]
    fn obstacle_sweep_oob_error_max_matches_main_field_labels() {
        let es_ft = sweep_err_msg(
            Locale::Es,
            TakeoffSweepError::ObstacleOutOfRange {
                max: defaults::OBSTACLE_HEIGHT_MAX_M,
            },
            ObstacleUnit::Feet,
        );
        assert!(es_ft.contains("0–50 ft"), "{es_ft}");
        let en_m = sweep_err_msg(
            Locale::En,
            TakeoffSweepError::ObstacleOutOfRange {
                max: defaults::OBSTACLE_HEIGHT_MAX_M,
            },
            ObstacleUnit::Meters,
        );
        assert!(en_m.contains("0–15 m"), "{en_m}");
    }

    #[test]
    fn obstacle_sweep_display_round_trip_preserves_metres() {
        let samples = [0.0, 5.0, 10.0, defaults::OBSTACLE_HEIGHT_MAX_M];
        for &m in &samples {
            for unit in [ObstacleUnit::Meters, ObstacleUnit::Feet] {
                let display = obstacle_sweep_display_value(m, unit);
                let back = obstacle_sweep_commit_display(display, unit);
                assert!(
                    (back - m).abs() < 1e-9,
                    "m={m} unit={unit:?} display={display} back={back}"
                );
            }
        }
    }

    #[test]
    fn obstacle_sweep_labels_es_en_ft_and_m() {
        use crate::i18n::format_takeoff_sweep_var_obs;
        assert_eq!(
            format_takeoff_sweep_var_obs(Locale::Es, ObstacleUnit::Meters),
            "Obstáculo (m)"
        );
        assert_eq!(
            format_takeoff_sweep_var_obs(Locale::Es, ObstacleUnit::Feet),
            "Obstáculo (ft)"
        );
        assert_eq!(
            format_takeoff_sweep_var_obs(Locale::En, ObstacleUnit::Meters),
            "Obstacle (m)"
        );
        assert_eq!(
            format_takeoff_sweep_var_obs(Locale::En, ObstacleUnit::Feet),
            "Obstacle (ft)"
        );
    }

    #[test]
    fn obstacle_sweep_ft_and_m_axes_produce_identical_core_results() {
        use da20_core::takeoff_sweep::{compute_takeoff_sweep, TakeoffSweepAxis, TakeoffSweepVar};

        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pa_override: Some(da20_core::types::PaOverride {
                pa_ft: 1000.0,
                phase: da20_core::types::ChartPhase::Takeoff,
            }),
            oat_c: 21.0,
            weight_lb: defaults::TAKEOFF_REF_WEIGHT_KG / 0.45359237,
            runway: "360".into(),
            wind_dir_deg: 0.0,
            wind_speed_kt: 4.0,
            obstacle_height_m: 5.0,
            ..Default::default()
        };
        let hw = signed_along_runway_kt(&inputs).unwrap();
        let axes_m = vec![TakeoffSweepAxis {
            var: TakeoffSweepVar::ObstacleHeightM,
            start: 5.0,
            end: 10.0,
            step: 5.0,
        }];
        let rows_m = compute_takeoff_sweep(&data, &inputs, &axes_m, hw).unwrap();

        let start_ft = obstacle_sweep_display_value(5.0, ObstacleUnit::Feet);
        let end_ft = obstacle_sweep_display_value(10.0, ObstacleUnit::Feet);
        let step_ft = obstacle_sweep_display_value(5.0, ObstacleUnit::Feet);
        let axes_ft = vec![TakeoffSweepAxis {
            var: TakeoffSweepVar::ObstacleHeightM,
            start: obstacle_sweep_commit_display(start_ft, ObstacleUnit::Feet),
            end: obstacle_sweep_commit_display(end_ft, ObstacleUnit::Feet),
            step: obstacle_sweep_commit_display(step_ft, ObstacleUnit::Feet),
        }];
        let rows_ft = compute_takeoff_sweep(&data, &inputs, &axes_ft, hw).unwrap();

        assert_eq!(rows_m.len(), rows_ft.len());
        for (a, b) in rows_m.iter().zip(rows_ft.iter()) {
            assert!((a.obstacle_height_m - b.obstacle_height_m).abs() < 1e-9);
            match (&a.result, &b.result) {
                (Some(ra), Some(rb)) => {
                    assert!((ra.ground_roll_ft - rb.ground_roll_ft).abs() < 1e-6);
                    assert!(
                        (ra.distance_over_obstacle_raw_ft - rb.distance_over_obstacle_raw_ft).abs()
                            < 1e-6
                    );
                }
                (None, None) => {}
                _ => panic!("result mismatch for obs {}", a.obstacle_height_m),
            }
        }
    }

    #[test]
    fn rounding_idempotent_on_unit_switch_display() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pa_override: Some(da20_core::types::PaOverride {
                pa_ft: 1000.0,
                phase: da20_core::types::ChartPhase::Takeoff,
            }),
            oat_c: 21.0,
            weight_lb: defaults::TAKEOFF_REF_WEIGHT_KG / 0.45359237,
            runway: "360".into(),
            wind_dir_deg: 0.0,
            wind_speed_kt: 4.0,
            obstacle_height_m: 5.0,
            ..Default::default()
        };
        let r = compute_takeoff(&data, &inputs).unwrap();
        let m1 = apply_takeoff_distance_rounding(&r, TakeoffDistanceRounding::Ceil50Feet);
        let m2 = apply_takeoff_distance_rounding(&r, TakeoffDistanceRounding::Ceil50Feet);
        assert_eq!(m1.over_obstacle_ft, m2.over_obstacle_ft);
        assert_eq!(m1.ground_roll_ft, m2.ground_roll_ft);
    }
}
