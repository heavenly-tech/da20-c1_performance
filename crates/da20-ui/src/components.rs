use leptos::prelude::*;

use crate::i18n::{copy, Locale};
use crate::phases::Phase;
use crate::theme::ThemePreference;

#[component]
pub fn Disclaimer(locale: RwSignal<Locale>) -> impl IntoView {
    view! {
        <aside class="disc" role="note">
            <b>"⚠ "</b>
            {move || copy(locale.get()).disclaimer}
        </aside>
    }
}

#[component]
pub fn Nav(
    phase: RwSignal<Phase>,
    locale: RwSignal<Locale>,
    theme: RwSignal<ThemePreference>,
) -> impl IntoView {
    view! {
        <nav class="nav">
            <PhaseTab phase=phase target=Phase::Takeoff locale=locale/>
            <PhaseTab phase=phase target=Phase::Climb locale=locale/>
            <PhaseTab phase=phase target=Phase::Cruise locale=locale/>
            <PhaseTab phase=phase target=Phase::Endurance locale=locale/>
            <PhaseTab phase=phase target=Phase::Landing locale=locale/>
            <PhaseTab phase=phase target=Phase::Readings locale=locale/>
            <PhaseTab phase=phase target=Phase::Manual locale=locale/>
            <button class="lang-btn" on:click=move |_| {
                let next = locale.get().toggle();
                locale.set(next);
                next.save();
            }>{move || format!("{} · {}", copy(locale.get()).lang_toggle, locale.get().label())}</button>
            <select class="lang-btn" prop:value=move || theme_label(theme.get())
                on:change=move |ev| {
                    let v = select_value(&ev);
                    let pref = match v.as_str() {
                        "light" => ThemePreference::Light,
                        "dark" => ThemePreference::Dark,
                        _ => ThemePreference::System,
                    };
                    theme.set(pref);
                    pref.save();
                    crate::theme::apply_theme(pref);
                }>
                <option value="system">{move || copy(locale.get()).theme_system}</option>
                <option value="light">{move || copy(locale.get()).theme_light}</option>
                <option value="dark">{move || copy(locale.get()).theme_dark}</option>
            </select>
        </nav>
    }
}

fn theme_label(t: ThemePreference) -> String {
    match t {
        ThemePreference::System => "system".into(),
        ThemePreference::Light => "light".into(),
        ThemePreference::Dark => "dark".into(),
    }
}

#[component]
fn PhaseTab(phase: RwSignal<Phase>, target: Phase, locale: RwSignal<Locale>) -> impl IntoView {
    view! {
        <button type="button"
            class="nav-btn"
            class:on=move || phase.get() == target
            on:click=move |_| phase.set(target)>
            {move || tab_label(locale.get(), target)}
        </button>
    }
}

fn tab_label(locale: Locale, p: Phase) -> &'static str {
    let c = copy(locale);
    match p {
        Phase::Takeoff => c.tab_takeoff,
        Phase::Climb => c.tab_climb,
        Phase::Cruise => c.tab_cruise,
        Phase::Endurance => c.tab_endurance,
        Phase::Landing => c.tab_landing,
        Phase::Readings => c.tab_readings,
        Phase::Manual => c.tab_manual,
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
            {cite.figure_or_table}
            {match cite.page {
                Some(p) => format!(" p.{p}"),
                None => String::new(),
            }}
            " · "
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

fn select_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}
