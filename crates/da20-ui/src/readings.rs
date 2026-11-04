use da20_core::pressure::format_qnh_dual;
use da20_core::wind_format::format_wind;
use leptos::prelude::*;

use crate::i18n::{copy, phase_slug_label, Locale};
use crate::storage::ReadingsStore;

#[component]
pub fn ReadingsPage(locale: RwSignal<Locale>, store: RwSignal<ReadingsStore>) -> impl IntoView {
    let compare_a = RwSignal::new(String::new());
    let compare_b = RwSignal::new(String::new());

    view! {
        <section class="readings scr on">
            <div class="card">
                <h3>{move || copy(locale.get()).tab_readings}</h3>
                <div class="export-row">
                    <button class="btn g" on:click=move |_| {
                        let json = store.get().export_json();
                        download_text("da20-readings.json", &json);
                    }>{move || copy(locale.get()).export_json}</button>
                    <button class="btn g" on:click=move |_| {
                        let csv = store.get().export_csv();
                        download_text("da20-readings.csv", &csv);
                    }>{move || copy(locale.get()).export_csv}</button>
                </div>
                <ul class="reading-list">
                    {move || store.get().readings.iter().rev().map(|r| {
                        let id = r.id.clone();
                        let wind = format_wind(r.inputs.wind_dir_deg, r.inputs.wind_speed_kt);
                        let qnh = format_qnh_dual(r.inputs.qnh_hpa);
                        let phase = phase_slug_label(locale.get(), &r.phase);
                        view! {
                            <li>
                                <strong>{r.label.clone()}</strong>" · "{phase}" · "{r.created_at_iso.clone()}
                                <span class="mono">" · "{qnh}" · "{wind}</span>
                                <button class="btn g" on:click={
                                    let id = id.clone();
                                    move |_| store.update(|s| s.delete(&id))
                                }>"×"</button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </div>
            <div class="card compare-card">
                <h4>{move || copy(locale.get()).readings_compare}</h4>
                {move || {
                    let loc = locale.get();
                    let c = copy(loc);
                    let readings = store.get().readings.clone();
                    view! {
                        <div class="compare-picks">
                            <label>{c.compare_pick_a}
                                <select on:change=move |ev| {
                                    compare_a.set(event_value(&ev));
                                }>
                                    <option value="">"—"</option>
                                    {readings.iter().rev().map(|r| {
                                        view! {
                                            <option value=r.id.clone()>{r.label.clone()}</option>
                                        }
                                    }).collect_view()}
                                </select>
                            </label>
                            <label>{c.compare_pick_b}
                                <select on:change=move |ev| {
                                    compare_b.set(event_value(&ev));
                                }>
                                    <option value="">"—"</option>
                                    {readings.iter().rev().map(|r| {
                                        view! {
                                            <option value=r.id.clone()>{r.label.clone()}</option>
                                        }
                                    }).collect_view()}
                                </select>
                            </label>
                        </div>
                        {compare_table(loc, &readings, compare_a.get(), compare_b.get())}
                    }
                }}
            </div>
        </section>
    }
}

fn compare_table(
    _locale: Locale,
    readings: &[da20_core::types::SavedReading],
    id_a: String,
    id_b: String,
) -> impl IntoView {
    if id_a.is_empty() || id_b.is_empty() || id_a == id_b {
        return view! { <p class="lab">"—"</p> }.into_any();
    }
    let a = readings.iter().find(|r| r.id == id_a);
    let b = readings.iter().find(|r| r.id == id_b);
    match (a, b) {
        (Some(ra), Some(rb)) => view! {
            <table class="compare-table mono">
                <thead><tr><th></th><th>{ra.label.clone()}</th><th>{rb.label.clone()}</th></tr></thead>
                <tbody>
                    <CompareRow label="QNH" a=format_qnh_dual(ra.inputs.qnh_hpa) b=format_qnh_dual(rb.inputs.qnh_hpa)/>
                    <CompareRow label="OAT °C" a=format!("{:.1}", ra.inputs.oat_c) b=format!("{:.1}", rb.inputs.oat_c)/>
                    <CompareRow label="Takeoff roll ft" a=fmt_opt_ft(ra.takeoff.as_ref().map(|t| t.ground_roll_ft)) b=fmt_opt_ft(rb.takeoff.as_ref().map(|t| t.ground_roll_ft))/>
                    <CompareRow label="Over obs ft" a=fmt_opt_ft(ra.takeoff.as_ref().map(|t| t.distance_over_50ft_ft)) b=fmt_opt_ft(rb.takeoff.as_ref().map(|t| t.distance_over_50ft_ft))/>
                    <CompareRow label="ROC fpm" a=fmt_opt_ft(ra.climb.as_ref().map(|c| c.rate_of_climb_fpm)) b=fmt_opt_ft(rb.climb.as_ref().map(|c| c.rate_of_climb_fpm))/>
                    <CompareRow label="TAS kt" a=fmt_opt_ft(ra.cruise.as_ref().map(|c| c.tas_kt)) b=fmt_opt_ft(rb.cruise.as_ref().map(|c| c.tas_kt))/>
                </tbody>
            </table>
        }.into_any(),
        _ => view! { <p class="lab">"—"</p> }.into_any(),
    }
}

#[component]
fn CompareRow(label: &'static str, a: String, b: String) -> impl IntoView {
    view! {
        <tr><td>{label}</td><td>{a}</td><td>{b}</td></tr>
    }
}

fn fmt_opt_ft(v: Option<f64>) -> String {
    v.map(|x| format!("{:.0}", x)).unwrap_or_else(|| "—".into())
}

fn event_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
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
