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

pub fn phase_from_slug(slug: &str) -> Option<Phase> {
    match slug.trim().trim_matches('/') {
        "takeoff" => Some(Phase::Takeoff),
        "climb" => Some(Phase::Climb),
        "cruise" => Some(Phase::Cruise),
        "endurance" => Some(Phase::Endurance),
        "landing" => Some(Phase::Landing),
        "method" => Some(Phase::Manual),
        "manual" => Some(Phase::Manual),
        "readings" => Some(Phase::Takeoff),
        _ => None,
    }
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
        let slug = phase_slug(phase);
        let _ = window.location().set_hash(slug);
    }
}

pub fn redirect_legacy_manual_hash() {
    if let Some(window) = web_sys::window() {
        if let Ok(hash) = window.location().hash() {
            let slug = hash.trim_start_matches('#');
            if slug.eq_ignore_ascii_case("manual") {
                let _ = window.location().set_hash("method");
            } else if slug.eq_ignore_ascii_case("readings") {
                let _ = window.location().set_hash("takeoff");
            }
        }
    }
}
