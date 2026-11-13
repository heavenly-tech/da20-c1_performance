//! WCAG contrast + text/chart token separation (hex values must match `style.css`).

#![cfg(test)]

const AA_BODY: f64 = 4.5;
const AA_LARGE_UI: f64 = 3.0;

struct TokenSet {
    colors: &'static [&'static str],
}

struct ThemeSpec {
    name: &'static str,
    text_ui: TokenSet,
    chart: TokenSet,
    /// (foreground hex, background hex, min_ratio, label)
    text_pairs: &'static [(&'static str, &'static str, f64, &'static str)],
    chart_pairs: &'static [(&'static str, &'static str, f64, &'static str)],
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

fn contrast_ratio(fg: &str, bg: &str) -> f64 {
    let lf = relative_luminance(parse_hex(fg).expect("fg hex"));
    let lb = relative_luminance(parse_hex(bg).expect("bg hex"));
    let lighter = lf.max(lb);
    let darker = lf.min(lb);
    (lighter + 0.05) / (darker + 0.05)
}

fn normalize_hex(hex: &str) -> String {
    hex.trim_start_matches('#').to_ascii_lowercase()
}

const LIGHT_TEXT_UI: &[&str] = &[
    "#e6e9ed", "#ffffff", "#0a0a0a", "#30363d", "#8f4b00", "#a31638", "#1a0e00", "#6d777f",
    "#5c656d", "#0057b8", "#fff4e5", "#7a3d00", "#9a3412", "#0f5c38",
];

const LIGHT_CHART: &[&str] = &[
    "#f4f7fa", "#3d4f63", "#6b7580", "#152a40", "#1a5694", "#6b2fa0", "#c56a08", "#d9820a",
    "#0a6b7c", "#5b21b6", "#14532d", "#7c2d12", "#4c1d95", "#9d174d", "#0369a1",
];

const DARK_TEXT_UI: &[&str] = &[
    "#1c2128", "#2a313a", "#f3f5f7", "#c5cdd6", "#ffb020", "#ff7b6b", "#1a1200", "#5a6570",
    "#252c34", "#4a5563", "#6ba3e8", "#2a1e08", "#ffd27a", "#3a2a05", "#5fd4a8", "#ff9a8c",
    "#ffd4ce",
];

const DARK_CHART: &[&str] = &[
    "#222830", "#4a5668", "#7a8798", "#b8c8d8", "#5f9fd8", "#a8c4f0", "#e8a020", "#f0b830",
    "#40c890", "#ff90a0", "#56b4e9", "#e69f00", "#2fb38a", "#ffa8d0", "#ffe8a0",
];

const NIGHT_TEXT_UI: &[&str] = &[
    "#060303", "#0e0606", "#d85858", "#c86868", "#b8650a", "#e06060", "#1a0800", "#5c1818",
    "#120808", "#9e3030", "#140606", "#a07018", "#180808", "#c84848",
];

const NIGHT_CHART: &[&str] = &[
    "#0a0404", "#701818", "#a04040", "#ca6060", "#b83818", "#cc5018", "#904018", "#a84820",
    "#c85820", "#802020", "#582020", "#682828", "#783018", "#903838", "#984020",
];

const THEMES: &[ThemeSpec] = &[
    ThemeSpec {
        name: "light",
        text_ui: TokenSet {
            colors: LIGHT_TEXT_UI,
        },
        chart: TokenSet {
            colors: LIGHT_CHART,
        },
        text_pairs: &[
            ("#0a0a0a", "#e6e9ed", AA_BODY, "text-primary on page bg"),
            ("#30363d", "#e6e9ed", AA_BODY, "text-muted on page bg"),
            ("#0a0a0a", "#ffffff", AA_BODY, "text-primary on surface"),
            ("#30363d", "#ffffff", AA_BODY, "text-muted on surface"),
            ("#7a3d00", "#fff4e5", AA_BODY, "disclaimer text"),
            ("#8f4b00", "#e6e9ed", AA_LARGE_UI, "accent on page (large)"),
            ("#a31638", "#ffffff", AA_BODY, "warning on input"),
        ],
        chart_pairs: &[
            ("#152a40", "#f4f7fa", AA_BODY, "axis label on chart surface"),
            ("#1a5694", "#f4f7fa", AA_LARGE_UI, "trace on chart surface"),
            ("#6b7580", "#f4f7fa", AA_LARGE_UI, "grid on chart surface"),
            ("#3d4f63", "#f4f7fa", AA_LARGE_UI, "chart border tone"),
        ],
    },
    ThemeSpec {
        name: "dark",
        text_ui: TokenSet {
            colors: DARK_TEXT_UI,
        },
        chart: TokenSet { colors: DARK_CHART },
        text_pairs: &[
            ("#f3f5f7", "#1c2128", AA_BODY, "text-primary on page bg"),
            ("#c5cdd6", "#1c2128", AA_BODY, "text-muted on page bg"),
            ("#f3f5f7", "#2a313a", AA_BODY, "text-primary on surface"),
            ("#ffd27a", "#3a2a05", AA_BODY, "disclaimer text"),
            ("#ff7b6b", "#2a313a", AA_BODY, "warning on surface"),
        ],
        chart_pairs: &[
            ("#b8c8d8", "#222830", AA_BODY, "axis label on chart surface"),
            ("#5f9fd8", "#222830", AA_LARGE_UI, "trace on chart surface"),
            ("#7a8798", "#222830", AA_LARGE_UI, "grid on chart surface"),
        ],
    },
    ThemeSpec {
        name: "night",
        text_ui: TokenSet {
            colors: NIGHT_TEXT_UI,
        },
        chart: TokenSet {
            colors: NIGHT_CHART,
        },
        text_pairs: &[
            ("#d85858", "#060303", AA_BODY, "text-primary on page bg"),
            ("#c86868", "#060303", AA_BODY, "text-muted on page bg"),
            ("#e06060", "#0e0606", AA_BODY, "warning on surface"),
        ],
        chart_pairs: &[
            ("#ca6060", "#0a0404", AA_BODY, "axis label on chart surface"),
            ("#b83818", "#0a0404", AA_LARGE_UI, "trace on chart surface"),
            ("#a04040", "#0a0404", AA_LARGE_UI, "grid on chart surface"),
        ],
    },
];

#[test]
fn text_and_chart_token_sets_do_not_share_hex_values() {
    for theme in THEMES {
        let chart_norm: std::collections::HashSet<String> = theme
            .chart
            .colors
            .iter()
            .map(|h| normalize_hex(h))
            .collect();
        for ui in theme.text_ui.colors {
            let u = normalize_hex(ui);
            assert!(
                !chart_norm.contains(&u),
                "theme {}: hex #{u} appears in both text/ui and chart sets",
                theme.name
            );
        }
    }
}

#[test]
fn wcag_contrast_for_token_pairs() {
    for theme in THEMES {
        for (fg, bg, min, label) in theme.text_pairs {
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= *min,
                "theme {} text check failed for {}: {:.2}:1 < {:.1}:1 (fg {fg} on bg {bg})",
                theme.name,
                label,
                ratio,
                min
            );
        }
        for (fg, bg, min, label) in theme.chart_pairs {
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= *min,
                "theme {} chart check failed for {}: {:.2}:1 < {:.1}:1 (fg {fg} on bg {bg})",
                theme.name,
                label,
                ratio,
                min
            );
        }
    }
}

#[test]
fn contrast_helper_sanity() {
    assert!(contrast_ratio("#000000", "#ffffff") > 20.0);
    assert!(contrast_ratio("#777777", "#ffffff") < 4.5);
}
