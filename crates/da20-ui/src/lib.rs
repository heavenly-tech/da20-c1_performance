use leptos::prelude::*;

mod app;
mod chart_axes;
mod chart_guides;
mod chart_inputs;
mod components;
mod i18n;
mod inputs;
mod interactive_chart;
mod palette;
mod phase_readouts;
mod phases;
mod routing;
mod storage;
mod strip_plot;
mod theme;

use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}
