use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::components::{Disclaimer, Nav};
use crate::i18n::Locale;
use crate::palette::{apply_palette, PaletteChoice, PaletteThemeEffect};
use crate::phases::{Phase, PhaseShell};
use crate::storage::ReadingsStore;
use crate::theme::ThemePreference;

#[component]
pub fn App() -> impl IntoView {
    let phase = RwSignal::new(Phase::Takeoff);
    let locale = RwSignal::new(Locale::load());
    let theme = RwSignal::new(ThemePreference::load());
    let palette = RwSignal::new(PaletteChoice::load());
    let data = RwSignal::new(PerformanceData::production());
    let inputs = RwSignal::new(da20_core::types::FlightInputs::default());
    let store = RwSignal::new(ReadingsStore::load());

    crate::theme::apply_theme(theme.get());
    apply_palette(&palette.get(), theme.get());

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
        <div class="app">
            <Disclaimer locale=locale/>
            <header class="header">
                <div class="logo">"DA20-C1 "<span>{move || crate::i18n::copy(locale.get()).logo_suffix}</span></div>
                <p class="subtitle">{move || crate::i18n::copy(locale.get()).subtitle}</p>
                <Nav phase=phase locale=locale theme=theme palette=palette/>
            </header>
            <main class="main">
                <PhaseShell locale=locale phase=phase data=data inputs=inputs store=store/>
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
