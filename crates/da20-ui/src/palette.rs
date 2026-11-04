use std::collections::HashMap;

use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use wasm_bindgen::JsCast;

use crate::i18n::{copy, Locale};
use crate::theme::ThemePreference;

const PALETTE_KEY: &str = "da20_palette_v1";
pub const BUILTIN_PALETTE_ID: &str = "default";
const PALETTES_JSON: &str = include_str!("../../../data/palettes.json");

/// Pilot can change the shipped default in one place (code constant mirrors CSS fallbacks).
pub fn default_palette_id() -> &'static str {
    BUILTIN_PALETTE_ID
}

const PALETTE_ORDER: &[&str] = &["okabe_ito", "vfr_chart", "mono_amber", "cividis"];

#[derive(Debug, Clone, Deserialize)]
struct PaletteEntry {
    name: String,
    light: PaletteColors,
    dark: PaletteColors,
}

#[derive(Debug, Clone, Deserialize)]
struct PaletteColors {
    background: String,
    grid: String,
    axis_text: String,
    guide: String,
    guide_active: String,
    user_path: String,
    markers: String,
    result: String,
    warning: String,
    border: String,
}

type PalettesFile = HashMap<String, PaletteEntry>;

static FILE: std::sync::OnceLock<PalettesFile> = std::sync::OnceLock::new();

fn file() -> &'static PalettesFile {
    FILE.get_or_init(|| serde_json::from_str(PALETTES_JSON).expect("data/palettes.json must parse"))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteChoice {
    pub id: String,
}

impl PaletteChoice {
    pub fn load() -> Self {
        LocalStorage::get(PALETTE_KEY).unwrap_or_else(|_| Self::default_choice())
    }

    pub fn default_choice() -> Self {
        Self {
            id: default_palette_id().to_string(),
        }
    }

    pub fn save(&self) {
        let _ = LocalStorage::set(PALETTE_KEY, self);
    }

    pub fn menu_options(locale: Locale) -> Vec<(String, String)> {
        let c = copy(locale);
        let mut out = vec![(
            BUILTIN_PALETTE_ID.to_string(),
            c.palette_builtin_default.to_string(),
        )];
        for &id in PALETTE_ORDER {
            if let Some(entry) = file().get(id) {
                out.push((
                    id.to_string(),
                    palette_display_name(locale, id, &entry.name),
                ));
            }
        }
        out
    }

    fn entry(&self) -> Option<&PaletteEntry> {
        if self.id == BUILTIN_PALETTE_ID {
            return None;
        }
        file().get(&self.id)
    }

    #[allow(dead_code)]
    pub fn label(&self, locale: Locale) -> String {
        if self.id == BUILTIN_PALETTE_ID {
            return copy(locale).palette_builtin_default.to_string();
        }
        file()
            .get(&self.id)
            .map(|e| palette_display_name(locale, &self.id, &e.name))
            .unwrap_or_else(|| self.id.clone())
    }
}

fn palette_display_name(locale: Locale, id: &str, json_name: &str) -> String {
    let c = copy(locale);
    match (locale, id) {
        (Locale::Es, "okabe_ito") => c.palette_okabe_ito.to_string(),
        (Locale::Es, "vfr_chart") => c.palette_vfr_chart.to_string(),
        (Locale::Es, "mono_amber") => c.palette_mono_amber.to_string(),
        (Locale::Es, "cividis") => c.palette_cividis.to_string(),
        _ => json_name.to_string(),
    }
}

const PALETTE_CSS_VARS: &[&str] = &[
    "--chart-bg",
    "--chart-grid",
    "--chart-label",
    "--chart-line",
    "--chart-highlight",
    "--chart-guide",
    "--chart-guide-active",
    "--chart-user-path",
    "--chart-marker",
    "--chart-border",
    "--ok",
    "--warn",
    "--bg",
    "--panel",
    "--line",
    "--txt",
    "--mut",
    "--amb",
];

fn set_var(html: &web_sys::HtmlElement, name: &str, value: &str) {
    let _ = html.style().set_property(name, value);
}

fn clear_palette_vars(html: &web_sys::HtmlElement) {
    for name in PALETTE_CSS_VARS {
        let _ = html.style().remove_property(name);
    }
}

fn apply_palette_colors(html: &web_sys::HtmlElement, c: &PaletteColors) {
    set_var(html, "--chart-bg", &c.background);
    set_var(html, "--chart-grid", &c.grid);
    set_var(html, "--chart-label", &c.axis_text);
    set_var(html, "--chart-line", &c.guide_active);
    set_var(html, "--chart-highlight", &c.user_path);
    set_var(html, "--chart-guide", &c.guide);
    set_var(html, "--chart-guide-active", &c.guide_active);
    set_var(html, "--chart-user-path", &c.user_path);
    set_var(html, "--chart-marker", &c.markers);
    set_var(html, "--chart-border", &c.border);
    set_var(html, "--ok", &c.result);
    set_var(html, "--warn", &c.warning);
    set_var(html, "--bg", &c.background);
    set_var(html, "--panel", &c.background);
    set_var(html, "--line", &c.border);
    set_var(html, "--txt", &c.axis_text);
    set_var(html, "--mut", &c.guide);
    set_var(html, "--amb", &c.user_path);
}

pub fn apply_palette(palette: &PaletteChoice, theme: ThemePreference) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(html) = doc
        .document_element()
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let _ = html.set_attribute("data-palette", &palette.id);
    if palette.id == BUILTIN_PALETTE_ID {
        clear_palette_vars(&html);
        return;
    }
    let Some(entry) = palette.entry() else {
        clear_palette_vars(&html);
        return;
    };
    let c = match theme {
        ThemePreference::Light => &entry.light,
        ThemePreference::Dark => &entry.dark,
    };
    apply_palette_colors(&html, c);
}

#[component]
pub fn PaletteThemeEffect(
    theme: RwSignal<ThemePreference>,
    palette: RwSignal<PaletteChoice>,
) -> impl IntoView {
    Effect::new(move |_| {
        crate::theme::apply_theme(theme.get());
        apply_palette(&palette.get(), theme.get());
    });
    view! { <span class="sr-only">"palette"</span> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_json_loads() {
        assert_eq!(default_palette_id(), "default");
        assert_eq!(file().len(), 4);
        assert!(file().contains_key("okabe_ito"));
        assert!(PaletteChoice::menu_options(Locale::En).len() == 5);
    }
}
