use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::components::{Disclaimer, Nav};
use crate::planner::Planner;
use crate::readings::ReadingsPage;
use crate::storage::ReadingsStore;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Planner,
    Readings,
    Methods,
}

#[component]
pub fn App() -> impl IntoView {
    let route = RwSignal::new(Route::Planner);
    let data = RwSignal::new(PerformanceData::synthetic_fixture());
    let store = RwSignal::new(ReadingsStore::load());

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(resp) = gloo_net::http::Request::get("/data/performance.json").send().await {
                if let Ok(text) = resp.text().await {
                    if let Ok(parsed) = PerformanceData::from_json_str(&text) {
                        if parsed.meta.data_status == "production" {
                            data.set(parsed);
                        }
                    }
                }
            }
        });
    });

    view! {
        <div class="app">
            <Disclaimer/>
            <header class="header">
                <h1>"DA20-C1 Eclipse Performance Planner"</h1>
                <p class="subtitle">"AFM DA202-C1 Chapter 5 — architecture preview (data TBD)"</p>
                <Nav route=route/>
            </header>
            <main class="main">
                {move || match route.get() {
                    Route::Planner => view! { <Planner data=data store=store/> }.into_any(),
                    Route::Readings => view! { <ReadingsPage store=store/> }.into_any(),
                    Route::Methods => view! { <MethodsManual/> }.into_any(),
                }}
            </main>
            <footer class="footer">
                <p>
                    "AFM: "
                    {move || data.get().meta.afm_document_number.clone()}
                    " rev "
                    {move || data.get().meta.afm_revision.clone()}
                    " ("
                    {move || data.get().meta.afm_date.clone()}
                    ") — "
                    <a href=move || data.get().meta.afm_official_url.clone()>"Official AFM (URL TBD)"</a>
                    " · "
                    <a href=move || data.get().meta.amm_official_url.clone()>"Official AMM (URL TBD)"</a>
                </p>
            </footer>
        </div>
    }
}

#[component]
fn MethodsManual() -> impl IntoView {
    let entries = da20_core::methods::all_methods();
    view! {
        <section class="methods">
            <h2>"Methods manual"</h2>
            <p>"Each calculation path, AFM source reference, interpolation policy, and limitations."</p>
            <ul class="method-list">
                {entries.into_iter().map(|e| view! {
                    <li>
                        <h3>{e.title}</h3>
                        <p><strong>"AFM: "</strong>{e.afm_reference}</p>
                        <p>{e.summary}</p>
                        <p><em>"Interpolation: "</em>{e.interpolation}</p>
                        <p><em>"Limitations: "</em>{e.limitations}</p>
                    </li>
                }).collect_view()}
            </ul>
        </section>
    }
}
