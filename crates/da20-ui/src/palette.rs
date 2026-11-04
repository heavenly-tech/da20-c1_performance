use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

use crate::theme::ThemePreference;

const PALETTE_KEY: &str = "da20_palette_v1";
const PALETTES_JSON: &str = include_str!("../../../data/palettes.json");

#[derive(Debug, Deserialize)]
struct PalettesFile {
    default_palette_id: String,
    palettes: Vec<PaletteDef>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaletteDef {
    id: String,
    name_en: String,
    name_es: String,
    light: PaletteColors,
    dark: PaletteColors,
}

#[derive(Debug, Clone, Deserialize)]
struct PaletteColors {
    background: String,
    grid: String,
    axis_text: String,
    chart_line: String,
    chart_highlight: String,
    guide: String,
    guide_active: String,
    user_path: String,
    markers: String,
    result: String,
    warning: String,
    border: String,
    ui_bg: String,
    ui_panel: String,
    ui_line: String,
    ui_txt: String,
    ui_mut: String,
    ui_amb: String,
}

static FILE: std::sync::OnceLock<PalettesFile> = std::sync::OnceLock::new();

fn file() -> &'static PalettesFile {
    FILE.get_or_init(|| serde_json::from_str(PALETTES_JSON).expect("data/palettes.json must parse"))
}

pub fn default_palette_id() -> &'static str {
    &file().default_palette_id
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

    pub fn menu_options() -> Vec<(String, String, String)> {
        file()
            .palettes
            .iter()
            .map(|p| (p.id.clone(), p.name_en.clone(), p.name_es.clone()))
            .collect()
    }

    fn def(&self) -> Option<&PaletteDef> {
        file()
            .palettes
            .iter()
            .find(|p| p.id == self.id)
            .or_else(|| file().palettes.first())
    }

    #[allow(dead_code)]
    pub fn label(&self, locale: crate::i18n::Locale) -> String {
        self.def()
            .map(|p| match locale {
                crate::i18n::Locale::Es => p.name_es.clone(),
                crate::i18n::Locale::En => p.name_en.clone(),
            })
            .unwrap_or_else(|| self.id.clone())
    }
}

fn set_var(html: &web_sys::HtmlElement, name: &str, value: &str) {
    let _ = html.style().set_property(name, value);
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
    let Some(def) = palette.def() else {
        return;
    };
    let c = match theme {
        ThemePreference::Light => &def.light,
        ThemePreference::Dark => &def.dark,
    };
    set_var(&html, "--chart-bg", &c.background);
    set_var(&html, "--chart-grid", &c.grid);
    set_var(&html, "--chart-label", &c.axis_text);
    set_var(&html, "--chart-line", &c.chart_line);
    set_var(&html, "--chart-highlight", &c.chart_highlight);
    set_var(&html, "--chart-guide", &c.guide);
    set_var(&html, "--chart-guide-active", &c.guide_active);
    set_var(&html, "--chart-user-path", &c.user_path);
    set_var(&html, "--chart-marker", &c.markers);
    set_var(&html, "--chart-border", &c.border);
    set_var(&html, "--ok", &c.result);
    set_var(&html, "--warn", &c.warning);
    set_var(&html, "--bg", &c.ui_bg);
    set_var(&html, "--panel", &c.ui_panel);
    set_var(&html, "--line", &c.ui_line);
    set_var(&html, "--txt", &c.ui_txt);
    set_var(&html, "--mut", &c.ui_mut);
    set_var(&html, "--amb", &c.ui_amb);
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
        assert_eq!(default_palette_id(), "diamond");
        assert!(PaletteChoice::menu_options().len() >= 4);
    }
}
