//! Hash URL slugs for phase tabs (`#/takeoff`, `#/method`, …).

use crate::phases::Phase;

pub fn phase_slug(phase: Phase) -> &'static str {
    match phase {
        Phase::Takeoff => "takeoff",
        Phase::Climb => "climb",
        Phase::Cruise => "cruise",
        Phase::Endurance => "endurance",
        Phase::Landing => "landing",
        Phase::Manual => "method",
    }
}

/// Normalize `#/manual`, `#manual`, `manual`, etc. to a bare slug.
pub fn normalize_hash_slug(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('#')
        .trim()
        .trim_start_matches('/')
        .trim()
        .to_ascii_lowercase()
}

pub fn phase_from_slug(slug: &str) -> Option<Phase> {
    match normalize_hash_slug(slug).as_str() {
        "takeoff" => Some(Phase::Takeoff),
        "climb" => Some(Phase::Climb),
        "cruise" => Some(Phase::Cruise),
        "endurance" => Some(Phase::Endurance),
        "landing" => Some(Phase::Landing),
        "method" | "manual" => Some(Phase::Manual),
        "readings" => Some(Phase::Takeoff),
        _ => None,
    }
}

/// If the hash is a legacy slug, return the canonical `#/…` fragment (with leading slash).
pub fn legacy_hash_redirect_target(raw_hash: &str) -> Option<&'static str> {
    match normalize_hash_slug(raw_hash).as_str() {
        "manual" => Some("/method"),
        "readings" => Some("/takeoff"),
        _ => None,
    }
}

pub fn hash_fragment_for_phase(phase: Phase) -> String {
    format!("/{}", phase_slug(phase))
}

pub fn read_phase_from_location() -> Option<Phase> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    let slug = hash.trim_start_matches('#');
    if slug.is_empty() {
        return None;
    }
    phase_from_slug(slug)
}

pub fn write_phase_to_location(phase: Phase) {
    if let Some(window) = web_sys::window() {
        let fragment = hash_fragment_for_phase(phase);
        let _ = window.location().set_hash(&fragment);
    }
}

pub fn redirect_legacy_manual_hash() {
    if let Some(window) = web_sys::window() {
        if let Ok(hash) = window.location().hash() {
            if let Some(target) = legacy_hash_redirect_target(&hash) {
                let _ = window.location().set_hash(target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_slug_strips_hash_and_leading_slash() {
        assert_eq!(normalize_hash_slug("#/manual"), "manual");
        assert_eq!(normalize_hash_slug("#manual"), "manual");
        assert_eq!(normalize_hash_slug("/method"), "method");
        assert_eq!(normalize_hash_slug("  #/takeoff  "), "takeoff");
    }

    #[test]
    fn phase_from_slug_accepts_slashed_and_legacy() {
        assert_eq!(phase_from_slug("/manual"), Some(Phase::Manual));
        assert_eq!(phase_from_slug("#/method"), Some(Phase::Manual));
        assert_eq!(phase_from_slug("manual"), Some(Phase::Manual));
        assert_eq!(phase_from_slug("method"), Some(Phase::Manual));
        assert_eq!(phase_from_slug("/readings"), Some(Phase::Takeoff));
    }

    #[test]
    fn legacy_redirect_targets() {
        assert_eq!(legacy_hash_redirect_target("#/manual"), Some("/method"));
        assert_eq!(legacy_hash_redirect_target("manual"), Some("/method"));
        assert_eq!(legacy_hash_redirect_target("#/readings"), Some("/takeoff"));
        assert_eq!(legacy_hash_redirect_target("takeoff"), None);
    }

    #[test]
    fn hash_fragment_for_phase_uses_leading_slash() {
        assert_eq!(hash_fragment_for_phase(Phase::Manual), "/method");
        assert_eq!(hash_fragment_for_phase(Phase::Takeoff), "/takeoff");
    }
}
