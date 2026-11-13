use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

use crate::palette::clear_chart_palette_inline_overrides;

const THEME_KEY: &str = "da20_theme_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    Light,
    Dark,
    Night,
}

impl ThemePreference {
    pub fn data_attr(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Night => "night",
        }
    }

    pub fn from_data_attr(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "night" => Some(Self::Night),
            _ => None,
        }
    }

    /// When `stored` is `None`, uses OS `prefers-color-scheme` (light or dark only).
    pub fn resolve_default(stored: Option<Self>) -> Self {
        stored.unwrap_or_else(|| {
            if system_prefers_dark() {
                Self::Dark
            } else {
                Self::Light
            }
        })
    }

    pub fn load() -> Self {
        let stored: Option<Self> = LocalStorage::get(THEME_KEY).ok();
        Self::resolve_default(stored)
    }

    /// Boot `index.html` may set `data-theme` before WASM loads; prefer that when valid.
    pub fn load_initial() -> Self {
        if let Some(pref) = read_dom_theme() {
            return pref;
        }
        Self::load()
    }

    /// Keep the theme `<select>` in sync when `data-theme` was set outside Leptos (e.g. boot script).
    pub fn sync_signal_from_dom(theme: RwSignal<Self>) {
        if let Some(pref) = read_dom_theme() {
            if theme.get_untracked() != pref {
                theme.set(pref);
            }
        }
    }

    pub fn save(self) {
        let _ = LocalStorage::set(THEME_KEY, self);
    }
}

fn system_prefers_dark() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
            .flatten()
            .map(|mq| mq.matches())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn color_scheme_value(pref: ThemePreference) -> &'static str {
    match pref {
        ThemePreference::Light => "light only",
        ThemePreference::Dark | ThemePreference::Night => "dark only",
    }
}

fn read_dom_theme() -> Option<ThemePreference> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
            .and_then(|html| html.get_attribute("data-theme"))
            .and_then(|attr| ThemePreference::from_data_attr(&attr))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

pub fn apply_theme(pref: ThemePreference) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(html) = doc.document_element() {
            let _ = html.set_attribute("data-theme", pref.data_attr());
            let scheme = color_scheme_value(pref);
            let _ = html.set_attribute("data-color-scheme", scheme);
            if let Ok(Some(meta)) = doc.query_selector("meta[name='color-scheme']") {
                let _ = meta.set_attribute("content", scheme);
            }
            if let Ok(el) = html.dyn_into::<web_sys::HtmlElement>() {
                let _ = el.style().set_property("color-scheme", scheme);
                if pref == ThemePreference::Night {
                    clear_chart_palette_inline_overrides(&el);
                }
            }
        }
    }
}

/// SVG overlay polylines: token-based stroke colors (no hardcoded hex in chart code).
pub fn overlay_trace_var(index: usize) -> &'static str {
    const VARS: [&str; 5] = [
        "var(--trace-1)",
        "var(--trace-2)",
        "var(--trace-3)",
        "var(--trace-4)",
        "var(--trace-5)",
    ];
    VARS[index % VARS.len()]
}

#[allow(dead_code)]
#[component]
pub fn ThemeEffect(theme: RwSignal<ThemePreference>) -> impl IntoView {
    Effect::new(move |_| {
        apply_theme(theme.get());
    });
    view! { <span class="sr-only">"theme"</span> }
}

#[component]
pub fn ThemeDomSync(theme: RwSignal<ThemePreference>) -> impl IntoView {
    Effect::new(move |_| {
        ThemePreference::sync_signal_from_dom(theme);
    });
    view! { <span class="sr-only">"theme-dom-sync"</span> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_serde_roundtrip() {
        for pref in [
            ThemePreference::Light,
            ThemePreference::Dark,
            ThemePreference::Night,
        ] {
            let json = serde_json::to_string(&pref).unwrap();
            let back: ThemePreference = serde_json::from_str(&json).unwrap();
            assert_eq!(back, pref);
        }
    }

    #[test]
    fn resolve_default_without_storage_is_light_in_unit_tests() {
        assert_eq!(
            ThemePreference::resolve_default(None),
            ThemePreference::Light
        );
    }

    #[test]
    fn resolve_default_uses_stored_night() {
        assert_eq!(
            ThemePreference::resolve_default(Some(ThemePreference::Night)),
            ThemePreference::Night
        );
    }

    #[test]
    fn data_attr_roundtrip() {
        for pref in [
            ThemePreference::Light,
            ThemePreference::Dark,
            ThemePreference::Night,
        ] {
            assert_eq!(
                ThemePreference::from_data_attr(pref.data_attr()),
                Some(pref)
            );
        }
    }

    #[test]
    fn theme_menu_values_match_data_attr() {
        assert_eq!(ThemePreference::Light.data_attr(), "light");
        assert_eq!(ThemePreference::Night.data_attr(), "night");
    }

    #[test]
    fn overlay_vars_use_css_tokens() {
        assert!(overlay_trace_var(0).starts_with("var(--trace-"));
    }
}
