use std::collections::HashMap;

use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

use crate::i18n::{copy, Locale};
use crate::theme::ThemePreference;

const PALETTE_STORE_KEY: &str = "da20_palette_v2";
const PALETTES_JSON: &str = include_str!("../../../data/palettes.json");

pub const DEFAULT_LIGHT_PALETTE: &str = "mono_amber";
pub const DEFAULT_DARK_PALETTE: &str = "vfr_chart";
/// Brighter hand-path / markers on VFR dark (colour-blind vs blue guides).
const VFR_DARK_USER_PATH: &str = "#ffc8f5";

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

pub fn default_palette_id(theme: ThemePreference) -> &'static str {
    match theme {
        ThemePreference::Light => DEFAULT_LIGHT_PALETTE,
        ThemePreference::Dark | ThemePreference::Night => DEFAULT_DARK_PALETTE,
    }
}

fn normalize_palette_id(id: &str, theme: ThemePreference) -> String {
    if file().contains_key(id) {
        id.to_string()
    } else {
        default_palette_id(theme).to_string()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct PerThemePalettes {
    light: Option<String>,
    dark: Option<String>,
    #[serde(default)]
    night: Option<String>,
}

impl PerThemePalettes {
    fn load() -> Self {
        LocalStorage::get(PALETTE_STORE_KEY).unwrap_or_default()
    }

    fn save(&self) {
        let _ = LocalStorage::set(PALETTE_STORE_KEY, self);
    }

    fn choice_for(&self, theme: ThemePreference) -> PaletteChoice {
        let stored = match theme {
            ThemePreference::Light => self.light.as_deref(),
            ThemePreference::Dark => self.dark.as_deref(),
            ThemePreference::Night => self.night.as_deref(),
        };
        PaletteChoice {
            id: normalize_palette_id(stored.unwrap_or(default_palette_id(theme)), theme),
        }
    }

    fn set(&mut self, theme: ThemePreference, id: String) {
        let id = normalize_palette_id(&id, theme);
        match theme {
            ThemePreference::Light => self.light = Some(id),
            ThemePreference::Dark => self.dark = Some(id),
            ThemePreference::Night => self.night = Some(id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteChoice {
    pub id: String,
}

impl PaletteChoice {
    pub fn for_theme(theme: ThemePreference) -> Self {
        PerThemePalettes::load().choice_for(theme)
    }

    pub fn save_for_theme(&self, theme: ThemePreference) {
        let mut store = PerThemePalettes::load();
        store.set(theme, self.id.clone());
        store.save();
    }

    pub fn menu_options(locale: Locale) -> Vec<(String, String)> {
        PALETTE_ORDER
            .iter()
            .filter_map(|&id| {
                file().get(id).map(|entry| {
                    (
                        id.to_string(),
                        palette_display_name(locale, id, &entry.name),
                    )
                })
            })
            .collect()
    }

    fn entry(&self) -> Option<&PaletteEntry> {
        file().get(&self.id)
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

fn set_var(html: &web_sys::HtmlElement, name: &str, value: &str) {
    let _ = html.style().set_property(name, value);
}

/// Palette JSON only overrides chart tokens — never page text / UI colours.
fn apply_palette_colors(html: &web_sys::HtmlElement, c: &PaletteColors) {
    set_var(html, "--chart-surface-bg", &c.background);
    set_var(html, "--chart-grid", &c.grid);
    set_var(html, "--chart-axis-label", &c.axis_text);
    set_var(html, "--chart-trace-line", &c.guide_active);
    set_var(html, "--chart-trace-highlight", &c.user_path);
    set_var(html, "--chart-guide", &c.guide);
    set_var(html, "--chart-guide-active", &c.guide_active);
    set_var(html, "--chart-user-path", &c.user_path);
    set_var(html, "--chart-marker", &c.markers);
    set_var(html, "--chart-border", &c.border);
    set_var(html, "--chart-result", &c.result);
    set_var(html, "--chart-warning", &c.warning);
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
    // Night cockpit theme: chart colours come only from CSS tokens (red/amber).
    if theme == ThemePreference::Night {
        return;
    }
    let Some(entry) = palette.entry() else {
        return;
    };
    let c = match theme {
        ThemePreference::Light => &entry.light,
        ThemePreference::Dark => &entry.dark,
        ThemePreference::Night => return,
    };
    apply_palette_colors(&html, c);
    if palette.id == "vfr_chart" && theme == ThemePreference::Dark {
        set_var(&html, "--chart-user-path", VFR_DARK_USER_PATH);
        set_var(&html, "--chart-marker", VFR_DARK_USER_PATH);
        set_var(&html, "--chart-trace-highlight", VFR_DARK_USER_PATH);
    }
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
        assert_eq!(file().len(), 4);
        assert!(file().contains_key("okabe_ito"));
        assert_eq!(default_palette_id(ThemePreference::Light), "mono_amber");
        assert_eq!(default_palette_id(ThemePreference::Dark), "vfr_chart");
        assert_eq!(PaletteChoice::menu_options(Locale::En).len(), 4);
    }

    #[test]
    fn per_theme_defaults() {
        assert_eq!(
            normalize_palette_id("unknown", ThemePreference::Light),
            "mono_amber"
        );
        assert_eq!(
            normalize_palette_id("unknown", ThemePreference::Dark),
            "vfr_chart"
        );
    }
}
