//! WCAG contrast + text/chart token separation — parsed from `style.css` (source of truth).

#![cfg(test)]

use std::collections::{HashMap, HashSet};

const STYLE_CSS: &str = include_str!("../style.css");

const AA_BODY: f64 = 4.5;
const AA_UI_NON_TEXT: f64 = 3.0;

#[derive(Clone, Copy)]
struct ContrastCheck {
    fg: &'static str,
    bg: &'static str,
    min: f64,
    label: &'static str,
}

fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r, g, b])
}

fn channel_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(rgb: [u8; 3]) -> f64 {
    let r = channel_linear(f64::from(rgb[0]) / 255.0);
    let g = channel_linear(f64::from(rgb[1]) / 255.0);
    let b = channel_linear(f64::from(rgb[2]) / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    let lf = relative_luminance(parse_hex(fg).expect("fg hex"));
    let lb = relative_luminance(parse_hex(bg).expect("bg hex"));
    let lighter = lf.max(lb);
    let darker = lf.min(lb);
    (lighter + 0.05) / (darker + 0.05)
}

fn normalize_hex(hex: &str) -> String {
    hex.trim_start_matches('#').to_ascii_lowercase()
}

fn extract_theme_block(css: &str, theme: &str) -> String {
    let needle = format!("[data-theme=\"{theme}\"]");
    let start = css
        .find(&needle)
        .unwrap_or_else(|| panic!("missing theme block {needle}"));
    let rest = &css[start..];
    let open = rest.find('{').expect("theme block brace");
    let mut depth = 0i32;
    let mut end = open;
    for (i, ch) in rest[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    rest[..end].to_string()
}

fn parse_variables(block: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in block.lines() {
        let line = line.trim();
        if !line.starts_with("--") {
            continue;
        }
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let value = rest.trim().trim_end_matches(';').trim();
        map.insert(name.trim().to_string(), value.to_string());
    }
    map
}

fn resolve_var(map: &HashMap<String, String>, name: &str, depth: usize) -> Option<String> {
    if depth > 24 {
        return None;
    }
    let value = map.get(name)?;
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return Some(format!("#{hex}"));
    }
    if let Some(inner) = value.strip_prefix("var(").and_then(|s| s.strip_suffix(')')) {
        let var = inner.trim();
        return resolve_var(map, var, depth + 1);
    }
    None
}

fn resolve_key(map: &HashMap<String, String>, key: &str) -> Option<String> {
    let name = if key.starts_with("--") {
        key.to_string()
    } else {
        format!("--{key}")
    };
    resolve_var(map, &name, 0)
}

fn is_text_ui_var(name: &str) -> bool {
    name.starts_with("--text-")
        || name.starts_with("--ui-")
        || matches!(
            name,
            "--disc-bg"
                | "--disc-fg"
                | "--disc-border"
                | "--caution-fg"
                | "--caution-border"
                | "--ooc-fg"
                | "--badge-not-tcca-fg"
                | "--badge-not-tcca-border"
                | "--selection-fg"
        )
}

fn is_chart_var(name: &str) -> bool {
    name.starts_with("--chart-") || name.starts_with("--trace-")
}

const CHART_NON_TEXT: &[ContrastCheck] = &[
    ContrastCheck {
        fg: "chart-guide",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "chart guide",
    },
    ContrastCheck {
        fg: "chart-guide-active",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "chart guide active",
    },
    ContrastCheck {
        fg: "chart-marker",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "chart marker",
    },
    ContrastCheck {
        fg: "chart-border",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "chart border",
    },
    ContrastCheck {
        fg: "trace-1",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "overlay trace 1",
    },
    ContrastCheck {
        fg: "trace-2",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "overlay trace 2",
    },
    ContrastCheck {
        fg: "trace-3",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "overlay trace 3",
    },
    ContrastCheck {
        fg: "trace-4",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "overlay trace 4",
    },
    ContrastCheck {
        fg: "trace-5",
        bg: "chart-surface-bg",
        min: AA_UI_NON_TEXT,
        label: "overlay trace 5",
    },
];

fn contrast_checks(theme: &str) -> &'static [ContrastCheck] {
    match theme {
        "light" => &[
            ContrastCheck {
                fg: "text-primary",
                bg: "ui-page-bg",
                min: AA_BODY,
                label: "body on page",
            },
            ContrastCheck {
                fg: "text-muted",
                bg: "ui-page-bg",
                min: AA_BODY,
                label: "muted on page",
            },
            ContrastCheck {
                fg: "text-primary",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "body on surface",
            },
            ContrastCheck {
                fg: "ui-btn-primary-fg",
                bg: "ui-btn-primary-bg",
                min: AA_BODY,
                label: "primary button",
            },
            ContrastCheck {
                fg: "ui-btn-secondary-fg",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "secondary button (kg/lb, toggles off)",
            },
            ContrastCheck {
                fg: "ui-btn-toggle-on-fg",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "toggle on (propeller/fairing)",
            },
            ContrastCheck {
                fg: "ui-disc-fg",
                bg: "ui-disc-bg",
                min: AA_BODY,
                label: "disclaimer",
            },
            ContrastCheck {
                fg: "text-warning",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "warning / out-of-chart on surface",
            },
            ContrastCheck {
                fg: "ui-border",
                bg: "ui-page-bg",
                min: AA_UI_NON_TEXT,
                label: "page border (non-text UI)",
            },
            ContrastCheck {
                fg: "ui-input-border",
                bg: "ui-input-bg",
                min: AA_UI_NON_TEXT,
                label: "input border",
            },
            ContrastCheck {
                fg: "ui-focus-ring",
                bg: "ui-page-bg",
                min: AA_UI_NON_TEXT,
                label: "focus ring on page",
            },
            ContrastCheck {
                fg: "chart-axis-label",
                bg: "chart-surface-bg",
                min: AA_BODY,
                label: "chart axis label",
            },
            ContrastCheck {
                fg: "chart-trace-line",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart trace",
            },
            ContrastCheck {
                fg: "chart-grid",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart grid",
            },
            ContrastCheck {
                fg: "chart-result",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart result trace",
            },
            ContrastCheck {
                fg: "chart-warning",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart warning trace",
            },
        ],
        "dark" => &[
            ContrastCheck {
                fg: "text-primary",
                bg: "ui-page-bg",
                min: AA_BODY,
                label: "body on page",
            },
            ContrastCheck {
                fg: "text-muted",
                bg: "ui-page-bg",
                min: AA_BODY,
                label: "muted on page",
            },
            ContrastCheck {
                fg: "ui-btn-primary-fg",
                bg: "ui-btn-primary-bg",
                min: AA_BODY,
                label: "primary button",
            },
            ContrastCheck {
                fg: "ui-btn-secondary-fg",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "secondary button",
            },
            ContrastCheck {
                fg: "ui-btn-toggle-on-fg",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "toggle on",
            },
            ContrastCheck {
                fg: "ui-disc-fg",
                bg: "ui-disc-bg",
                min: AA_BODY,
                label: "disclaimer",
            },
            ContrastCheck {
                fg: "ui-border",
                bg: "ui-page-bg",
                min: AA_UI_NON_TEXT,
                label: "page border",
            },
            ContrastCheck {
                fg: "ui-input-border",
                bg: "ui-input-bg",
                min: AA_UI_NON_TEXT,
                label: "input border",
            },
            ContrastCheck {
                fg: "ui-focus-ring",
                bg: "ui-page-bg",
                min: AA_UI_NON_TEXT,
                label: "focus ring",
            },
            ContrastCheck {
                fg: "chart-axis-label",
                bg: "chart-surface-bg",
                min: AA_BODY,
                label: "chart axis label",
            },
            ContrastCheck {
                fg: "chart-trace-line",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart trace",
            },
            ContrastCheck {
                fg: "chart-grid",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart grid",
            },
            ContrastCheck {
                fg: "chart-result",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart result trace",
            },
            ContrastCheck {
                fg: "chart-warning",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart warning trace",
            },
        ],
        "night" => &[
            ContrastCheck {
                fg: "text-primary",
                bg: "ui-page-bg",
                min: AA_BODY,
                label: "body on page",
            },
            ContrastCheck {
                fg: "text-muted",
                bg: "ui-page-bg",
                min: AA_BODY,
                label: "muted on page",
            },
            ContrastCheck {
                fg: "ui-btn-primary-fg",
                bg: "ui-btn-primary-bg",
                min: AA_BODY,
                label: "primary button",
            },
            ContrastCheck {
                fg: "ui-btn-secondary-fg",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "secondary button",
            },
            ContrastCheck {
                fg: "ui-btn-toggle-on-fg",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "toggle on",
            },
            ContrastCheck {
                fg: "ui-disc-fg",
                bg: "ui-disc-bg",
                min: AA_BODY,
                label: "disclaimer",
            },
            ContrastCheck {
                fg: "text-warning",
                bg: "ui-surface-bg",
                min: AA_BODY,
                label: "out-of-chart",
            },
            ContrastCheck {
                fg: "ui-border",
                bg: "ui-page-bg",
                min: AA_UI_NON_TEXT,
                label: "page border",
            },
            ContrastCheck {
                fg: "ui-input-border",
                bg: "ui-input-bg",
                min: AA_UI_NON_TEXT,
                label: "input border",
            },
            ContrastCheck {
                fg: "ui-focus-ring",
                bg: "ui-page-bg",
                min: AA_UI_NON_TEXT,
                label: "focus ring",
            },
            ContrastCheck {
                fg: "chart-axis-label",
                bg: "chart-surface-bg",
                min: AA_BODY,
                label: "chart axis label",
            },
            ContrastCheck {
                fg: "chart-trace-line",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart trace",
            },
            ContrastCheck {
                fg: "chart-grid",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart grid",
            },
            ContrastCheck {
                fg: "chart-result",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart result trace",
            },
            ContrastCheck {
                fg: "chart-warning",
                bg: "chart-surface-bg",
                min: AA_UI_NON_TEXT,
                label: "chart warning trace",
            },
        ],
        _ => &[],
    }
}

const NIGHT_SCROLLBAR: ContrastCheck = ContrastCheck {
    fg: "ui-scrollbar-thumb",
    bg: "ui-scrollbar-track",
    min: AA_UI_NON_TEXT,
    label: "scrollbar thumb",
};

fn iter_checks(theme: &str) -> impl Iterator<Item = ContrastCheck> {
    let base = contrast_checks(theme).iter().copied();
    let chart = CHART_NON_TEXT.iter().copied();
    let night = (theme == "night").then_some(std::iter::once(NIGHT_SCROLLBAR));
    base.chain(chart).chain(night.into_iter().flatten())
}

fn themes() -> [&'static str; 3] {
    ["light", "dark", "night"]
}

#[test]
fn text_and_chart_token_sets_do_not_share_hex_values() {
    for theme in themes() {
        let block = extract_theme_block(STYLE_CSS, theme);
        let map = parse_variables(&block);
        let mut text_hex = HashSet::new();
        let mut chart_hex = HashSet::new();
        for name in map.keys() {
            if is_text_ui_var(name) {
                if let Some(h) = resolve_var(&map, name, 0) {
                    text_hex.insert(normalize_hex(&h));
                }
            }
            if is_chart_var(name) {
                if let Some(h) = resolve_var(&map, name, 0) {
                    chart_hex.insert(normalize_hex(&h));
                }
            }
        }
        for t in &text_hex {
            assert!(
                !chart_hex.contains(t),
                "theme {theme}: hex #{t} in both text/ui and chart sets"
            );
        }
    }
}

#[test]
fn wcag_contrast_from_style_css() {
    for theme in themes() {
        let block = extract_theme_block(STYLE_CSS, theme);
        let map = parse_variables(&block);
        for check in iter_checks(theme) {
            let fg = resolve_key(&map, check.fg)
                .unwrap_or_else(|| panic!("theme {theme}: could not resolve --{}", check.fg));
            let bg = resolve_key(&map, check.bg)
                .unwrap_or_else(|| panic!("theme {theme}: could not resolve --{}", check.bg));
            let ratio = contrast_ratio(&fg, &bg);
            assert!(
                ratio >= check.min,
                "theme {theme} failed {}: {ratio:.2}:1 < {:.1}:1 ({fg} on {bg})",
                check.label,
                check.min
            );
        }
    }
}

#[test]
fn contrast_helper_sanity() {
    assert!(contrast_ratio("#000000", "#ffffff") > 20.0);
    assert!(contrast_ratio("#777777", "#ffffff") < 4.5);
}

#[test]
fn palette_json_backgrounds_are_not_night_chart_surface() {
    const PALETTES_JSON: &str = include_str!("../../../data/palettes.json");
    const NIGHT_CHART_SURFACE: &str = "#0a0404";
    let file: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(PALETTES_JSON).expect("palettes.json");
    for (id, entry) in &file {
        let light = entry
            .get("light")
            .and_then(|v| v.get("background"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{id} light background"));
        let dark = entry
            .get("dark")
            .and_then(|v| v.get("background"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{id} dark background"));
        assert_ne!(
            light.to_ascii_lowercase(),
            NIGHT_CHART_SURFACE,
            "palette {id} light"
        );
        assert_ne!(
            dark.to_ascii_lowercase(),
            NIGHT_CHART_SURFACE,
            "palette {id} dark"
        );
    }
}
