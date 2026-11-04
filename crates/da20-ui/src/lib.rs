use leptos::prelude::*;

mod app;
mod components;
mod i18n;
mod inputs;
mod interactive_chart;
mod phases;
mod qnh_unit;
mod readings;
mod storage;
mod strip_plot;
mod theme;

use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}
