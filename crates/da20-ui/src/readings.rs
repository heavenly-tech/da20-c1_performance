use leptos::prelude::*;

use crate::storage::ReadingsStore;

#[component]
pub fn ReadingsPage(store: RwSignal<ReadingsStore>) -> impl IntoView {
    let compare_a = RwSignal::new(String::new());
    let compare_b = RwSignal::new(String::new());

    view! {
        <section class="readings">
            <h2>"Saved readings"</h2>
            <p>"Stored in browser localStorage — unlimited entries."</p>
            <div class="export-row">
                <button on:click=move |_| {
                    let json = store.get().export_json();
                    download_text("da20-readings.json", &json);
                }>"Export JSON"</button>
                <button on:click=move |_| {
                    let csv = store.get().export_csv();
                    download_text("da20-readings.csv", &csv);
                }>"Export CSV"</button>
            </div>

            <ul class="reading-list">
                {move || store.get().readings.iter().rev().map(|r| {
                    let id = r.id.clone();
                    view! {
                        <li>
                            <strong>{r.label.clone()}</strong>" · "{r.created_at_iso.clone()}
                            <button class="danger" on:click={
                                let id = id.clone();
                                move |_| store.update(|s| s.delete(&id))
                            }>"Delete"</button>
                            <details>
                                <summary>"Details"</summary>
                                <pre>{serde_json::to_string_pretty(r).unwrap_or_default()}</pre>
                            </details>
                        </li>
                    }
                }).collect_view()}
            </ul>

            <h3>"Compare"</h3>
            <div class="compare-row">
                <label>"A "<input prop:value=move || compare_a.get() on:input=move |ev| compare_a.set(input_value(&ev))/></label>
                <label>"B "<input prop:value=move || compare_b.get() on:input=move |ev| compare_b.set(input_value(&ev))/></label>
            </div>
            {move || {
                let a = compare_a.get();
                let b = compare_b.get();
                let readings = store.get().readings;
                let ra = readings.iter().find(|r| r.id == a);
                let rb = readings.iter().find(|r| r.id == b);
                match (ra, rb) {
                    (Some(ra), Some(rb)) => view! {
                        <pre class="compare">{diff_json(ra, rb)}</pre>
                    }.into_any(),
                    _ => view! { <p>"Select two reading IDs to compare."</p> }.into_any(),
                }
            }}
        </section>
    }
}

fn diff_json(a: &da20_core::types::SavedReading, b: &da20_core::types::SavedReading) -> String {
    format!(
        "Takeoff ground: {:?} vs {:?}\nCruise TAS: {:?} vs {:?}",
        a.takeoff.as_ref().map(|t| t.ground_roll_ft),
        b.takeoff.as_ref().map(|t| t.ground_roll_ft),
        a.cruise.as_ref().map(|c| c.tas_kt),
        b.cruise.as_ref().map(|c| c.tas_kt),
    )
}

fn input_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
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
