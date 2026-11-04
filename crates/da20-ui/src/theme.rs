use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const THEME_KEY: &str = "da20_theme_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn load() -> Self {
        LocalStorage::get(THEME_KEY).unwrap_or(Self::System)
    }

    pub fn save(self) {
        let _ = LocalStorage::set(THEME_KEY, self);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EffectiveTheme {
    Light,
    Dark,
}

pub fn effective_theme(pref: ThemePreference) -> EffectiveTheme {
    match pref {
        ThemePreference::Light => EffectiveTheme::Light,
        ThemePreference::Dark => EffectiveTheme::Dark,
        ThemePreference::System => {
            if system_prefers_dark() {
                EffectiveTheme::Dark
            } else {
                EffectiveTheme::Light
            }
        }
    }
}

fn system_prefers_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
        .flatten()
        .map(|mq| mq.matches())
        .unwrap_or(true)
}

pub fn apply_theme(pref: ThemePreference) {
    let effective = effective_theme(pref);
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(html) = doc.document_element() {
            let _ = html.set_attribute(
                "data-theme",
                match effective {
                    EffectiveTheme::Light => "light",
                    EffectiveTheme::Dark => "dark",
                },
            );
        }
    }
}

#[component]
pub fn ThemeEffect(theme: RwSignal<ThemePreference>) -> impl IntoView {
    Effect::new(move |_| {
        let t = theme.get();
        apply_theme(t);
    });
    view! { <span class="sr-only">"theme"</span> }
}
