//! Per-phase saved readouts (list, restore, export, overlay selection).

use std::collections::HashSet;

use da20_core::pressure::format_qnh_dual;
use da20_core::types::FlightInputs;
use da20_core::wind_format::format_wind;
use leptos::prelude::*;

use crate::i18n::{
    copy, format_takeoff_conservative_estimate, format_takeoff_over_obstacle_result, Locale,
};
use crate::phases::{phase_default_label, phase_storage_name, Phase};
use crate::storage::{compute_all, new_reading, ReadingsStore};
use da20_core::endurance::compute_endurance;
use da20_core::PerformanceData;

pub const OVERLAY_STROKE_COLORS: [&str; 5] =
    ["#0072B2", "#E69F00", "#009E73", "#CC79A7", "#56B4E9"];

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
                        let summary = readout_inputs_summary(&r.inputs);
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
                                    <span class="lab">{r.created_at_iso.clone()}</span>
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

pub fn readout_inputs_summary(inputs: &FlightInputs) -> String {
    let wind = format_wind(inputs.wind_dir_deg, inputs.wind_speed_kt);
    let qnh = format_qnh_dual(inputs.qnh_hpa);
    format!(
        "OAT {:.0} °C · {qnh} · {:.0} lb · {wind} · RWY {}",
        inputs.oat_c, inputs.weight_lb, inputs.runway
    )
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
                format!(
                    "{:.0} ft roll · {} · {}",
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
                )
            } else {
                c.results_unavailable.to_string()
            }
        }
        Phase::Climb => r
            .climb
            .as_ref()
            .map(|cl| {
                format!(
                    "{:.0} fpm · EA {:.0} ft",
                    cl.rate_of_climb_fpm, cl.equivalent_altitude_ft
                )
            })
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Cruise => r
            .cruise
            .as_ref()
            .map(|cr| format!("{:.0} kt TAS · {:.1} gph", cr.tas_kt, cr.fuel_flow_us_gal_h))
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Endurance => r
            .endurance
            .as_ref()
            .map(|e| format!("{:.1} h · {:.0} NM", e.endurance_h, e.range_nm))
            .unwrap_or_else(|| c.results_unavailable.to_string()),
        Phase::Landing => r
            .landing
            .as_ref()
            .map(|l| {
                format!(
                    "{:.0} ft roll · 50 ft {:.0} ft",
                    l.ground_roll_ft, l.distance_over_50ft_ft
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
        let s = readout_inputs_summary(&inputs);
        assert!(s.contains("22"));
        assert!(s.contains("190"));
        assert!(s.contains("200"));
    }
}
