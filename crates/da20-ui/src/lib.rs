use leptos::prelude::*;

mod app;
mod components;
mod i18n;
mod inputs;
mod interactive_chart;
mod phases;
mod readings;
mod storage;
mod theme;

use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}
