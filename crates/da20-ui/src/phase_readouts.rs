//! Per-phase saved readouts (list, restore, export, overlay selection).

use std::collections::HashSet;

use da20_core::altitude::effective_pressure_altitude_ft;
use da20_core::types::{ChartPhase, ClimbFigure, CruiseSolveMode, FlightInputs};
use da20_core::wind_format::format_wind;
use leptos::prelude::*;

use crate::i18n::{
    afm_lookup_tag_label, copy, format_climb_readout_summary, format_cruise_readout_summary,
    format_endurance_readout_summary, format_takeoff_conservative_estimate,
    format_takeoff_over_obstacle_result, Locale,
};
use crate::phases::{phase_default_label, phase_storage_name, Phase};
use crate::storage::{compute_all, format_readout_timestamp, new_reading, ReadingsStore};
use da20_core::endurance::compute_endurance;
use da20_core::PerformanceData;

pub use crate::theme::overlay_trace_var;

#[component]
pub fn PhaseReadoutsPanel(
    locale: RwSignal<Locale>,
    phase: Phase,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
    store: RwSignal<ReadingsStore>,
    overlay_ids: RwSignal<HashSet<String>>,
) -> impl IntoView {
    let phase_key = phase_storage_name(phase);
    let label = RwSignal::new(String::new());

    let bundle = Memo::new(move |_| {
        let d = data.get();
        let i = inputs.get();
        let mut b = compute_all(&d, &i);
        b.endurance = compute_endurance(&d, &i).ok();
        b
    });

    let phase_key_for_list = phase_key.clone();

    view! {
        <div class="readouts-panel card-inner">
            <h4>{move || copy(locale.get()).readouts_title}</h4>
            <div class="save-row">
                <label class="readout-label-field">
                    <span class="lab">{move || copy(locale.get()).readout_label_hint}</span>
                    <input prop:value=move || label.get()
                        on:input=move |ev| label.set(crate::inputs::input_value(&ev))/>
                </label>
                <button class="btn" type="button" on:click={
                    let phase_key = phase_key.clone();
                    move |_| {
                        let mut b = bundle.get();
                        if b.endurance.is_none() {
                            b.endurance =
                                compute_endurance(&data.get(), &inputs.get()).ok();
                        }
                        let reading = new_reading(
                            if label.get().is_empty() {
                                phase_default_label(phase)
                            } else {
                                label.get()
                            },
                            phase_key.clone(),
                            inputs.get(),
                            b,
                        );
                        store.update(|s| s.push_phase(&phase_key, reading));
                    }
                }>{move || copy(locale.get()).save_readout}</button>
            </div>
            <div class="export-row">
                <button class="btn g" type="button" on:click={
                    let phase_key = phase_key.clone();
                    move |_| {
                        let json = store.get().export_json_phase(&phase_key);
                        download_text(&format!("da20-{phase_key}-readouts.json"), &json);
                    }
                }>{move || copy(locale.get()).export_json}</button>
                <button class="btn g" type="button" on:click={
                    let phase_key = phase_key.clone();
                    move |_| {
                        let csv = store.get().export_csv_phase(&phase_key);
                        download_text(&format!("da20-{phase_key}-readouts.csv"), &csv);
                    }
                }>{move || copy(locale.get()).export_csv}</button>
                <button class="btn g" type="button" on:click={
                    let phase_key = phase_key.clone();
                    move |_| {
                        store.update(|s| s.clear_phase(&phase_key));
                        overlay_ids.update(|set| set.clear());
                    }
                }>{move || copy(locale.get()).clear_readouts}</button>
            </div>
            <ul class="reading-list phase-reading-list">
                {move || {
                    let loc = locale.get();
                    let list = store.get().for_phase(&phase_key_for_list).to_vec();
                    list.into_iter().rev().map(|r| {
                        let id = r.id.clone();
                        let id_restore = id.clone();
                        let id_del = id.clone();
                        let id_overlay_class = id.clone();
                        let id_overlay_toggle = id.clone();
                        let phase_key_restore = phase_key_for_list.clone();
                        let phase_key_del = phase_key_for_list.clone();
                        let summary =
                            readout_inputs_summary_for_phase(loc, phase, &r.inputs);
                        let results = readout_results_summary(loc, phase, &r);
                        let checked = overlay_ids.get().contains(&id);
                        view! {
                            <li class="readout-row"
                                class:readout-selected=move || overlay_ids.get().contains(&id_overlay_class)>
                                <button type="button" class="readout-restore"
                                    on:click=move |_| {
                                        if let Some(found) =
                                            store.get().find_in_phase(&phase_key_restore, &id_restore)
                                        {
                                            inputs.set(found.inputs.clone());
                                        }
                                    }>
                                    <strong>{r.label.clone()}</strong>
                                    <span class="lab">{format_readout_timestamp(&r.created_at_iso, loc)}</span>
                                    <span class="mono readout-summary">{summary}</span>
                                    <span class="mono readout-results">{results}</span>
                                </button>
                                <label class="overlay-toggle" title=copy(loc).overlay_on_chart>
                                    <input type="checkbox" prop:checked=checked
                                        on:change=move |_| {
                                            overlay_ids.update(|set| {
                                                if set.contains(&id_overlay_toggle) {
                                                    set.remove(&id_overlay_toggle);
                                                } else {
                                                    set.insert(id_overlay_toggle.clone());
                                                }
                                            });
                                        }/>
                                </label>
                                <button type="button" class="btn g readout-del"
                                    on:click=move |_| {
                                        store.update(|s| s.delete_phase(&phase_key_del, &id_del));
                                        overlay_ids.update(|set| {
                                            set.remove(&id_del);
                                        });
                                    }>"×"</button>
                            </li>
                        }
                    }).collect_view()
                }}
            </ul>
        </div>
    }
}

pub fn readout_inputs_summary_for_phase(
    locale: Locale,
    phase: Phase,
    inputs: &FlightInputs,
) -> String {
    let c = copy(locale);
    let wind = format_wind(inputs.wind_dir_deg, inputs.wind_speed_kt);
    let fair_on = inputs.wheel_fairings_installed;
    let fair_lbl = if fair_on {
        c.wheel_fairings_installed_yes
    } else {
        c.wheel_fairings_installed_no
    };
    match phase {
        Phase::Takeoff | Phase::Landing => {
            format!(
                "PA {:.0} ft · OAT {:.0} °C · {:.0} lb · {wind} · RWY {}",
                inputs.pressure_altitude_input_ft, inputs.oat_c, inputs.weight_lb, inputs.runway
            )
        }
        Phase::Climb => {
            let pa = effective_pressure_altitude_ft(inputs, Some(ChartPhase::Climb));
            let flap = match inputs.climb_figure {
                ClimbFigure::Fig56TakeoffFlaps => c.climb_flaps_to,
                ClimbFigure::Fig55CruiseFlaps => c.climb_flaps_up,
                ClimbFigure::Fig58BalkedLanding => c.climb_flaps_ldg,
            };
            format!(
                "OAT {:.0} °C · PA {:.0} ft · {:.0} lb · {flap} · {} ({})",
                inputs.oat_c, pa, inputs.weight_lb, fair_lbl, c.wheel_fairings_label_short,
            )
        }
        Phase::Cruise | Phase::Endurance => {
            let thrust = match inputs.cruise_solve_mode {
                CruiseSolveMode::SetRpm => format!("{:.0} RPM", inputs.cruise_rpm),
                CruiseSolveMode::SetPercentBhp => format!("{:.2} % BHP", inputs.cruise_percent_bhp),
            };
            format!(
                "Cr OAT {:.0} °C · PA {:.0} ft · {thrust} · {:.0} lb · {} ({})",
                inputs.cruise_oat_c,
                inputs.cruise_pressure_alt_ft,
                inputs.weight_lb,
                fair_lbl,
                c.wheel_fairings_label_short,
            )
        }
        Phase::Manual => String::new(),
    }
}

pub fn readout_results_summary(
    locale: Locale,
    phase: Phase,
    r: &da20_core::types::SavedReading,
) -> String {
    let c = copy(locale);
    match phase {
        Phase::Takeoff => {
            if let Some(t) = &r.takeoff {
                let tag = afm_lookup_tag_label(locale, t.afm_lookup_tag);
                format!(
                    "{:.0} ft roll · {} · {} · {}",
                    t.ground_roll_ft,
                    format_takeoff_over_obstacle_result(
                        locale,
                        t.distance_over_obstacle_raw_ft,
                        t.distance_over_obstacle_raw_m,
                    ),
                    format_takeoff_conservative_estimate(
                        locale,
                        t.distance_over_obstacle_conservative_ft,
                        t.distance_over_obstacle_conservative_m,
                    ),
                    tag
                )
            } else {
                c.results_unavailable.to_string()
            }
        }
        Phase::Climb => r
            .climb
            .as_ref()
            .map(|cl| {
                let roc = format_climb_readout_summary(
                    locale,
                    cl.rate_of_climb_fpm,
                    cl.rate_of_climb_chart_fpm,
                    r.inputs.wheel_fairings_installed,
                    cl.afm_lookup_tag,
                );
                format!("{roc} · EA {:.0} ft", cl.equivalent_altitude_ft)
            })
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Cruise => r
            .cruise
            .as_ref()
            .map(|cr| {
                format_cruise_readout_summary(
                    locale,
                    cr.tas_kt,
                    cr.tas_chart_kt,
                    r.inputs.wheel_fairings_installed,
                    cr.fuel_flow_us_gal_h,
                    cr.percent_bhp,
                    cr.afm_lookup_tag,
                )
            })
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Endurance => r
            .endurance
            .as_ref()
            .map(|e| {
                format_endurance_readout_summary(
                    locale,
                    e.endurance_h,
                    e.range_nm,
                    e.range_chart_nm,
                    e.tas_kt,
                    e.tas_chart_kt,
                    e.fuel_flow_us_gal_h,
                    r.inputs.wheel_fairings_installed,
                    e.afm_lookup_tag,
                )
            })
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Landing => r
            .landing
            .as_ref()
            .map(|l| {
                let tag = afm_lookup_tag_label(locale, l.afm_lookup_tag);
                format!(
                    "{:.0} ft roll · 50 ft {:.0} ft · {}",
                    l.ground_roll_ft, l.distance_over_50ft_ft, tag
                )
            })
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Manual => String::new(),
    }
}

fn download_text(filename: &str, content: &str) {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let a = document.create_element("a").expect("a");
    let a: web_sys::HtmlElement = a.dyn_into().expect("anchor");
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(content));
    let blob = web_sys::Blob::new_with_str_sequence(&blob_parts).expect("blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("url");
    a.set_attribute("href", &url).ok();
    a.set_attribute("download", filename).ok();
    a.click();
    web_sys::Url::revoke_object_url(&url).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::types::defaults;

    #[test]
    fn readout_summary_includes_oat_wind_runway() {
        let inputs = FlightInputs {
            oat_c: 22.0,
            runway: "190".into(),
            wind_dir_deg: 200.0,
            wind_speed_kt: 8.0,
            weight_lb: defaults::WEIGHT_LB,
            ..Default::default()
        };
        let s = readout_inputs_summary_for_phase(Locale::En, Phase::Takeoff, &inputs);
        assert!(s.contains("22"));
        assert!(s.contains("190"));
        assert!(s.contains("200"));
    }

    #[test]
    fn climb_readout_summary_omits_wind() {
        let inputs = FlightInputs {
            wind_speed_kt: 25.0,
            ..Default::default()
        };
        let s = readout_inputs_summary_for_phase(Locale::En, Phase::Climb, &inputs);
        assert!(!s.contains("25"));
    }
}
