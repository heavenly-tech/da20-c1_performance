use leptos::prelude::*;

use crate::app::Route;

#[component]
pub fn Disclaimer() -> impl IntoView {
    view! {
        <aside class="disclaimer" role="note">
            <strong>"Not for operational use."</strong>
            " This tool is "
            <em>"not"</em>
            " Values follow AFM DA202-C1 Rev 29 digitized Chapter 5 data (approximate). "
            "Takeoff over obstacle distances read ~2.7% short vs the AFM worked example — see Methods. "
            "The approved Aircraft Flight Manual (AFM) in the aircraft always governs."
        </aside>
    }
}

#[component]
pub fn Nav(route: RwSignal<Route>) -> impl IntoView {
    view! {
        <nav class="nav">
            <button class="nav-btn" on:click=move |_| route.set(Route::Planner)>"Planner"</button>
            <button class="nav-btn" on:click=move |_| route.set(Route::Readings)>"Saved readings"</button>
            <button class="nav-btn" on:click=move |_| route.set(Route::Methods)>"Methods"</button>
        </nav>
    }
}

#[component]
pub fn ProvenanceLine(provenance: da20_core::types::ProvenanceBlock) -> impl IntoView {
    let cite = provenance.citation.clone();
    view! {
        <p class="provenance">
            {provenance.afm_document_number}
            " rev "
            {provenance.afm_revision}
            " · "
            {provenance.afm_date}
            " · "
            {cite.figure_or_table}
            {match cite.page {
                Some(p) => format!(" p.{p}"),
                None => String::new(),
            }}
            " · data: "
            {provenance.data_status}
        </p>
    }
}

#[component]
pub fn WarningsList(warnings: Vec<da20_core::types::CalculationWarning>) -> impl IntoView {
    let empty = warnings.is_empty();
    view! {
        {(!empty).then(|| {
            view! {
                <ul class="warnings">
                    {warnings.into_iter().map(|w| view! {
                        <li><code>{w.code}</code>" — "{w.message}</li>
                    }).collect_view()}
                </ul>
            }
        })}
    }
}
