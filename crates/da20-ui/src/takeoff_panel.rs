//! Takeoff numeric inputs (unit prefs), distance rounding, and parameter sweep table.

use da20_core::rev29::engine::{ft_to_m, kg_to_lb, lb_to_kg, m_to_ft};
use da20_core::takeoff_rounding::{apply_takeoff_distance_rounding, TakeoffDistanceRounding};
use da20_core::takeoff_sweep::{
    compute_takeoff_sweep, TakeoffSweepAxis, TakeoffSweepError, TakeoffSweepRow, TakeoffSweepVar,
};
use da20_core::types::{defaults, FlightInputs};
use da20_core::wind::{runway_heading_status, signed_along_runway_kt, RunwayHeadingStatus};
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::i18n::{afm_lookup_tag_label, copy, Locale};
use crate::inputs::{input_value, NumField};
use crate::prefs::{ObstacleUnit, TakeoffPrefs, WeightUnit};

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

#[component]
pub fn TakeoffNumericBlock(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
) -> impl IntoView {
    view! {
        <TakeoffWeightField locale=locale inputs=inputs prefs=prefs/>
        <TakeoffSignedHeadwindField locale=locale inputs=inputs/>
        <TakeoffObstacleUnitField locale=locale inputs=inputs prefs=prefs/>
        <TakeoffRoundingToggle locale=locale prefs=prefs/>
    }
}

#[component]
fn TakeoffWeightField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let draft = RwSignal::new(String::new());

    view! {
        <div class="f">
            {move || {
                let c = copy(locale.get());
                let unit = prefs.get().weight;
                let label = match unit {
                    WeightUnit::Kg => c.takeoff_weight_kg,
                    WeightUnit::Lb => c.weight,
                };
                view! {
                    <label>{label}</label>
                    <div class="unit-row">
                        <input type="number" step="any" class="mono"
                            prop:value=move || {
                                takeoff_weight_field_display(
                                    editing.get(),
                                    &draft.get(),
                                    inputs.get().weight_lb,
                                    unit,
                                )
                            }
                            on:focus=move |_| {
                                let u = prefs.get_untracked().weight;
                                draft.set(format_takeoff_weight_display(inputs.get_untracked().weight_lb, u));
                                editing.set(true);
                            }
                            on:input=move |ev| {
                                let mut next = draft.get_untracked();
                                note_takeoff_weight_draft(&mut next, &input_value(&ev));
                                draft.set(next);
                            }
                            on:blur=move |ev| {
                                let u = prefs.get_untracked().weight;
                                let raw = input_value(&ev);
                                inputs.update(|i| {
                                    apply_takeoff_weight_field_value(&mut i.weight_lb, u, &raw);
                                });
                                editing.set(false);
                            }
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                if ev.key() == "Enter" {
                                    let u = prefs.get_untracked().weight;
                                    let raw = input_value(&ev);
                                    inputs.update(|i| {
                                        apply_takeoff_weight_field_value(&mut i.weight_lb, u, &raw);
                                    });
                                    editing.set(false);
                                }
                            }
                        />
                        <div class="toggle-row unit-toggle">
                            <button type="button" class="btn g"
                                class:on=move || prefs.get().weight == WeightUnit::Kg
                                on:click=move |_| prefs.update(|p| { p.weight = WeightUnit::Kg; p.save(); })>
                                {c.unit_kg}
                            </button>
                            <button type="button" class="btn g"
                                class:on=move || prefs.get().weight == WeightUnit::Lb
                                on:click=move |_| prefs.update(|p| { p.weight = WeightUnit::Lb; p.save(); })>
                                {c.unit_lb}
                            </button>
                        </div>
                    </div>
                }
            }}
        </div>
    }
}

#[component]
fn TakeoffSignedHeadwindField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        <div class="f">
            <label>{move || copy(locale.get()).takeoff_signed_headwind.to_string()}</label>
            <input type="number" step="any" class="mono"
                prop:value=move || {
                    if runway_heading_status(&inputs.get().runway) != RunwayHeadingStatus::Valid {
                        String::new()
                    } else {
                        signed_along_runway_kt(&inputs.get())
                            .map(|v| format!("{v:.1}"))
                            .unwrap_or_default()
                    }
                }
                on:input=move |ev| {
                    if let Ok(v) = input_value(&ev).parse::<f64>() {
                        inputs.update(|i| {
                            da20_core::wind::apply_signed_along_runway_wind(i, v);
                        });
                    }
                }
            />
        </div>
    }
}

#[component]
fn TakeoffObstacleUnitField(
    locale: RwSignal<Locale>,
    inputs: RwSignal<FlightInputs>,
    prefs: RwSignal<TakeoffPrefs>,
) -> impl IntoView {
    view! {
        {move || {
            let c = copy(locale.get());
            let unit = prefs.get().obstacle;
            let label = match unit {
                ObstacleUnit::Meters => format!("{} (0–15)", c.obstacle_m),
                ObstacleUnit::Feet => format!("{} (0–50)", c.obstacle_ft),
            };
            view! {
                <div class="f">
                    <label>{label}</label>
                    <div class="unit-row">
                        <input type="number" step="any" class="mono"
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
                                    inputs.update(|i| {
                                        let m = match prefs.get_untracked().obstacle {
                                            ObstacleUnit::Meters => v,
                                            ObstacleUnit::Feet => ft_to_m(v),
                                        };
                                        i.obstacle_height_m =
                                            m.clamp(0.0, defaults::OBSTACLE_HEIGHT_MAX_M);
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
            )));
            return;
        }
        let Some(hw_base) = signed_along_runway_kt(&base) else {
            error.set(Some(sweep_err_msg(
                locale.get(),
                TakeoffSweepError::InvalidRunway,
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
            Err(e) => error.set(Some(sweep_err_msg(locale.get(), e))),
        }
    };

    view! {
        <div class="card sweep-card">
            <h3>{move || copy(locale.get()).takeoff_sweep_title.to_string()}</h3>
            <SweepVarRow locale=locale label=move || copy(locale.get()).takeoff_sweep_var_oat.to_string() draft=oat/>
            <SweepVarRow locale=locale label=move || copy(locale.get()).takeoff_sweep_var_hw.to_string() draft=hw/>
            <SweepVarRow locale=locale label=move || copy(locale.get()).takeoff_sweep_var_obs.to_string() draft=obs/>
            <button type="button" class="btn g" on:click=run>
                {move || copy(locale.get()).takeoff_sweep_run.to_string()}
            </button>
            {move || error.get().map(|e| view! { <p class="lab field-error">{e}</p> })}
            <TakeoffSweepTable locale=locale rows=rows prefs=prefs/>
        </div>
    }
}

#[component]
fn SweepVarRow(
    locale: RwSignal<Locale>,
    label: impl Fn() -> String + Send + 'static,
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
                {move || label()}
            </label>
            <div class="sweep-range">
                <NumField label=move || copy(locale.get()).takeoff_sweep_start.to_string()
                    get=move || draft.get().start
                    set=move |v| draft.update(|d| d.start = v)/>
                <NumField label=move || copy(locale.get()).takeoff_sweep_end.to_string()
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
            view! {
                <div class="sweep-table-wrap">
                    <table class="sweep-table mono">
                        <thead>
                            <tr>
                                <th>{c.takeoff_sweep_var_oat}</th>
                                <th>{c.takeoff_sweep_var_hw}</th>
                                <th>{c.takeoff_sweep_var_obs}</th>
                                <th>{c.takeoff_sweep_col_gr}</th>
                                <th>{c.takeoff_sweep_col_to}</th>
                                <th>{c.takeoff_sweep_col_tag}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {list.iter().map(|row| {
                                let rounded = apply_takeoff_distance_rounding(&row.result, mode);
                                let loc = locale.get();
                                let tag = afm_lookup_tag_label(loc, row.result.afm_lookup_tag);
                                let obs_display = match prefs.get_untracked().obstacle {
                                    ObstacleUnit::Meters => format!("{:.0} m", row.obstacle_height_m),
                                    ObstacleUnit::Feet => format!("{:.0} ft", m_to_ft(row.obstacle_height_m)),
                                };
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
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            }
            .into_any()
        }}
    }
}

fn sweep_err_msg(locale: Locale, err: TakeoffSweepError) -> String {
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
            TakeoffSweepError::ObstacleOutOfRange { max } => {
                format!("Obstáculo fuera de rango (0–{max:.0} m).")
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
            TakeoffSweepError::ObstacleOutOfRange { max } => {
                format!("Obstacle out of range (0–{max:.0} m).")
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
