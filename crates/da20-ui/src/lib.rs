use leptos::prelude::*;

mod app;
mod components;
mod hand_plot;
mod planner;
mod readings;
mod storage;

use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}
