use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const THEME_KEY: &str = "da20_theme_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    Light,
    Dark,
}

impl ThemePreference {
    pub fn load() -> Self {
        LocalStorage::get(THEME_KEY).unwrap_or(Self::Light)
    }

    pub fn save(self) {
        let _ = LocalStorage::set(THEME_KEY, self);
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

pub fn apply_theme(pref: ThemePreference) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(html) = doc.document_element() {
            let _ = html.set_attribute(
                "data-theme",
                match pref {
                    ThemePreference::Light => "light",
                    ThemePreference::Dark => "dark",
                },
            );
        }
    }
}

#[allow(dead_code)]
#[component]
pub fn ThemeEffect(theme: RwSignal<ThemePreference>) -> impl IntoView {
    Effect::new(move |_| {
        apply_theme(theme.get());
    });
    view! { <span class="sr-only">"theme"</span> }
}
