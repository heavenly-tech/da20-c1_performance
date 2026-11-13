use leptos::prelude::*;

mod app;
mod chart_axes;
mod chart_guides;
mod chart_inputs;
mod components;
mod ground_oat;
mod i18n;
mod inputs;
mod interactive_chart;
mod mt_cruise_chart;
mod mt_cruise_panel;
mod palette;
mod phase_readouts;
mod phase_state;
mod phases;
mod prefs;
mod routing;
mod storage;
mod strip_plot;
mod table_source_panel;
mod takeoff_panel;
mod theme;
#[cfg(test)]
mod theme_contrast;

use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| view! { <App/> });
}
