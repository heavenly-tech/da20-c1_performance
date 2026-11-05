use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::{copy, data_status_label, warn_text, Locale};
use crate::palette::{apply_palette, PaletteChoice};
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
    palette: RwSignal<PaletteChoice>,
) -> impl IntoView {
    view! {
        <nav class="nav">
            <PhaseTab phase=phase target=Phase::Takeoff locale=locale/>
            <PhaseTab phase=phase target=Phase::Climb locale=locale/>
            <PhaseTab phase=phase target=Phase::Cruise locale=locale/>
            <PhaseTab phase=phase target=Phase::Endurance locale=locale/>
            <PhaseTab phase=phase target=Phase::Landing locale=locale/>
            <PhaseTab phase=phase target=Phase::Manual locale=locale/>
            <button class="lang-btn" on:click=move |_| {
                let next = locale.get().toggle();
                locale.set(next);
                next.save();
            }>{move || format!("{} · {}", copy(locale.get()).lang_toggle, locale.get().label())}</button>
            <button class="lang-btn" type="button" on:click=move |_| {
                let current = theme.get();
                let next = current.toggle();
                theme.set(next);
                next.save();
                crate::theme::apply_theme(next);
                let p = PaletteChoice::for_theme(next);
                palette.set(p.clone());
                apply_palette(&p, next);
            }>{move || format!(
                "{} · {}",
                copy(locale.get()).theme_toggle,
                match theme.get() {
                    ThemePreference::Light => copy(locale.get()).theme_light,
                    ThemePreference::Dark => copy(locale.get()).theme_dark,
                }
            )}</button>
            <label class="palette-picker lang-btn">
                <span class="palette-picker-label">{move || copy(locale.get()).palette_toggle}</span>
                <select class="palette-select" prop:value=move || palette.get().id.clone()
                    on:change=move |ev| {
                        let id = ev
                            .target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                            .map(|s| s.value())
                            .unwrap_or_default();
                        let choice = PaletteChoice { id: id.clone() };
                        palette.set(choice.clone());
                        choice.save_for_theme(theme.get());
                        apply_palette(&choice, theme.get());
                    }>
                    {move || {
                        let loc = locale.get();
                        PaletteChoice::menu_options(loc)
                            .into_iter()
                            .map(|(id, label)| {
                                view! {
                                    <option value=id.clone() selected={palette.get().id == id}>
                                        {label}
                                    </option>
                                }
                            })
                            .collect_view()
                    }}
                </select>
            </label>
        </nav>
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
        Phase::Manual => c.tab_method,
    }
}

#[component]
pub fn ProvenanceLine(
    locale: Locale,
    provenance: da20_core::types::ProvenanceBlock,
) -> impl IntoView {
    let cite = provenance.citation.clone();
    let rev = copy(locale).provenance_rev;
    let status = data_status_label(locale, &provenance.data_status);
    view! {
        <p class="provenance">
            {provenance.afm_document_number}
            {rev}
            {provenance.afm_revision}
            " · "
            {cite.figure_or_table}
            {match cite.page {
                Some(p) => format!(" p.{p}"),
                None => String::new(),
            }}
            " · "
            {status}
        </p>
    }
}

#[component]
pub fn WarningsList(
    locale: Locale,
    warnings: Vec<da20_core::types::CalculationWarning>,
) -> impl IntoView {
    let empty = warnings.is_empty();
    view! {
        {(!empty).then(|| {
            view! {
                <ul class="warnings">
                    {warnings.into_iter().map(|w| {
                        let text = warn_text(locale, &w);
                        view! {
                            <li>{text}</li>
                        }
                    }).collect_view()}
                </ul>
            }
        })}
    }
}
