use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::components::{Disclaimer, Nav};
use crate::i18n::Locale;
use crate::palette::{apply_palette, PaletteChoice, PaletteThemeEffect};
use crate::phase_state::{hydrate_tab_inputs, persist_tab_inputs};
use crate::phases::{Phase, PhaseShell};
use crate::prefs::{FlightPrefs, TakeoffPrefs};
use crate::routing::{
    read_phase_from_location, redirect_legacy_manual_hash, write_phase_to_location,
};
use crate::storage::ReadingsStore;
use crate::theme::ThemePreference;
use da20_core::types::FlightInputs;

#[component]
pub fn App() -> impl IntoView {
    redirect_legacy_manual_hash();
    let initial_phase = read_phase_from_location().unwrap_or(Phase::Takeoff);
    let phase = RwSignal::new(initial_phase);
    let locale = RwSignal::new(Locale::load());
    let theme = RwSignal::new(ThemePreference::load_initial());
    let palette = RwSignal::new(PaletteChoice::for_theme(theme.get_untracked()));
    let data = RwSignal::new(PerformanceData::production());
    let flight_prefs = RwSignal::new(FlightPrefs::load());
    let inputs = RwSignal::new({
        let prefs = flight_prefs.get_untracked();
        let mut i = FlightInputs::default();
        hydrate_tab_inputs(initial_phase, &prefs, &data.get_untracked(), &mut i);
        i
    });
    let last_phase = RwSignal::new(initial_phase);
    let store = RwSignal::new(ReadingsStore::load());
    let takeoff_prefs = RwSignal::new(TakeoffPrefs::load());

    crate::theme::apply_theme(theme.get());
    apply_palette(&palette.get(), theme.get());

    Effect::new(move |_| {
        write_phase_to_location(phase.get());
    });

    Effect::new(move |_| {
        let new_phase = phase.get();
        let old_phase = last_phase.get_untracked();
        if old_phase == new_phase {
            return;
        }
        let d = data.get_untracked();
        flight_prefs.update(|prefs| {
            let snap = inputs.get_untracked();
            persist_tab_inputs(old_phase, prefs, &snap);
        });
        flight_prefs.update(|prefs| {
            inputs.update(|i| hydrate_tab_inputs(new_phase, prefs, &d, i));
        });
        last_phase.set(new_phase);
    });

    Effect::new(move |_| {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(html) = doc.document_element() {
                let _ = html.set_attribute("lang", locale.get().html_lang());
            }
        }
    });

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(resp) = gloo_net::http::Request::get("/data/performance.json")
                .send()
                .await
            {
                if let Ok(text) = resp.text().await {
                    if let Ok(parsed) = PerformanceData::from_json_str(&text) {
                        if parsed.meta().data_status.as_deref() == Some("production") {
                            data.set(parsed);
                        }
                    }
                }
            }
        });
    });

    view! {
        <PaletteThemeEffect theme=theme palette=palette/>
        <crate::theme::ThemeDomSync theme=theme/>
        <div class="app">
            <Disclaimer locale=locale/>
            <header class="header">
                <div class="logo">"DA20-C1 "<span>{move || crate::i18n::copy(locale.get()).logo_suffix}</span></div>
                <p class="subtitle">{move || crate::i18n::copy(locale.get()).subtitle}</p>
                <Nav phase=phase locale=locale theme=theme palette=palette/>
            </header>
            <main class="main">
                <PhaseShell locale=locale phase=phase data=data inputs=inputs store=store takeoff_prefs=takeoff_prefs flight_prefs=flight_prefs/>
            </main>
            <footer class="footer">
                <p>
                    "AFM "
                    {move || data.get().meta().doc.clone()}
                    " rev "
                    {move || data.get().meta().revision.to_string()}
                    " · "
                    <a href=move || data.get().meta().afm_official_url.clone().unwrap_or_default()>
                        {move || crate::i18n::copy(locale.get()).afm_link}
                    </a>
                    " · "
                    <a href=move || data.get().meta().technical_publications_url.clone().unwrap_or_default()>
                        {move || crate::i18n::copy(locale.get()).portal_link}
                    </a>
                    " · "
                    <a href=move || data.get().meta().amm_official_url.clone().unwrap_or_default()>
                        {move || crate::i18n::copy(locale.get()).amm_link}
                    </a>
                </p>
            </footer>
        </div>
    }
}
