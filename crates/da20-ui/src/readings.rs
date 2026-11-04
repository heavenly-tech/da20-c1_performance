use da20_core::wind_format::format_wind;
use leptos::prelude::*;

use crate::i18n::{copy, Locale};
use crate::storage::ReadingsStore;

#[component]
pub fn ReadingsPage(locale: RwSignal<Locale>, store: RwSignal<ReadingsStore>) -> impl IntoView {
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
                        view! {
                            <li>
                                <strong>{r.label.clone()}</strong>" · "{r.phase.clone()}" · "{r.created_at_iso.clone()}
                                <span class="mono">" · "{wind}</span>
                                <button class="btn g" on:click={
                                    let id = id.clone();
                                    move |_| store.update(|s| s.delete(&id))
                                }>"×"</button>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            </div>
        </section>
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
