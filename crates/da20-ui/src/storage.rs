use std::collections::BTreeMap;

use da20_core::pressure::qnh_hpa_to_inhg;
use da20_core::types::FlightInputs;
use da20_core::wind_format::format_wind;
use da20_core::PerformanceData;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const KEY_V2: &str = "da20_performance_readings_v2";
const KEY_V1: &str = "da20_performance_readings_v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ReadingsStore {
    #[serde(default)]
    by_phase: BTreeMap<String, Vec<da20_core::types::SavedReading>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ReadingsStoreV1 {
    readings: Vec<da20_core::types::SavedReading>,
}

impl ReadingsStore {
    pub fn save(&self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(KEY_V2, self);
        }
    }

    pub fn load() -> Self {
        if cfg!(target_arch = "wasm32") {
            if let Ok(store) = LocalStorage::get::<ReadingsStore>(KEY_V2) {
                return store;
            }
            let mut store = ReadingsStore::default();
            if let Ok(legacy) = LocalStorage::get::<ReadingsStoreV1>(KEY_V1) {
                for r in legacy.readings {
                    store.by_phase.entry(r.phase.clone()).or_default().push(r);
                }
                store.save();
            }
            return store;
        }
        ReadingsStore::default()
    }

    pub fn for_phase(&self, phase: &str) -> &[da20_core::types::SavedReading] {
        self.by_phase
            .get(phase)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn find_in_phase(&self, phase: &str, id: &str) -> Option<&da20_core::types::SavedReading> {
        self.by_phase.get(phase)?.iter().find(|r| r.id == id)
    }

    pub fn push_phase(&mut self, phase: &str, reading: da20_core::types::SavedReading) {
        self.by_phase
            .entry(phase.to_string())
            .or_default()
            .push(reading);
        self.save();
    }

    pub fn delete_phase(&mut self, phase: &str, id: &str) {
        if let Some(list) = self.by_phase.get_mut(phase) {
            list.retain(|r| r.id != id);
            if list.is_empty() {
                self.by_phase.remove(phase);
            }
        }
        self.save();
    }

    pub fn clear_phase(&mut self, phase: &str) {
        self.by_phase.remove(phase);
        self.save();
    }

    pub fn export_json_phase(&self, phase: &str) -> String {
        let list = self.for_phase(phase);
        serde_json::to_string_pretty(list).unwrap_or_else(|_| "[]".into())
    }

    pub fn export_csv_phase(&self, phase: &str) -> String {
        export_csv_rows(self.for_phase(phase))
    }
}

fn export_csv_rows(readings: &[da20_core::types::SavedReading]) -> String {
    let mut out = String::from(
        "id,label,phase,created_at,field_elev,qnh_hpa,qnh_inhg,oat,runway,wind,wind_kt,weight,climb_figure,obstacle_m,cruise_oat,takeoff_ground_ft,takeoff_over_obstacle_ft,climb_roc_fpm,cruise_tas,cruise_ff,landing_ground,landing_50ft,endurance_h,range_nm\n",
    );
    for r in readings {
        let i = &r.inputs;
        let wind = format_wind(i.wind_dir_deg, i.wind_speed_kt);
        let climb_fig = format!("{:?}", i.climb_figure);
        let qnh_inhg = format!("{:.2}", qnh_hpa_to_inhg(i.qnh_hpa));
        let t_gr = r.takeoff.as_ref().map(|t| t.ground_roll_ft);
        let t_50 = r.takeoff.as_ref().map(|t| t.distance_over_50ft_ft);
        let roc = r.climb.as_ref().map(|c| c.rate_of_climb_fpm);
        let tas = r.cruise.as_ref().map(|c| c.tas_kt);
        let ff = r.cruise.as_ref().map(|c| c.fuel_flow_us_gal_h);
        let l_gr = r.landing.as_ref().map(|l| l.ground_roll_ft);
        let l_50 = r.landing.as_ref().map(|l| l.distance_over_50ft_ft);
        let end_h = r.endurance.as_ref().map(|e| e.endurance_h);
        let range = r.endurance.as_ref().map(|e| e.range_nm);
        let oat_csv = format!("{:.1}", i.oat_c);
        let cruise_oat_csv = format!("{:.1}", i.cruise_oat_c);
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.id,
            escape_csv(&r.label),
            escape_csv(&r.phase),
            r.created_at_iso,
            i.field_elevation_ft,
            i.qnh_hpa,
            qnh_inhg,
            oat_csv,
            i.runway,
            wind,
            i.wind_speed_kt,
            i.weight_lb,
            escape_csv(&climb_fig),
            i.obstacle_height_m,
            cruise_oat_csv,
            fmt_opt(t_gr),
            fmt_opt(t_50),
            fmt_opt(roc),
            fmt_opt(tas),
            fmt_opt(ff),
            fmt_opt(l_gr),
            fmt_opt(l_50),
            fmt_opt(end_h),
            fmt_opt(range),
        ));
    }
    out
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.1}")).unwrap_or_default()
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[derive(Clone, PartialEq)]
pub struct SavedBundle {
    pub takeoff: Option<da20_core::types::TakeoffResult>,
    pub climb: Option<da20_core::types::ClimbResult>,
    pub cruise: Option<da20_core::types::CruiseResult>,
    pub landing: Option<da20_core::types::LandingResult>,
    pub wind: Option<da20_core::types::WindResult>,
    pub endurance: Option<da20_core::types::EnduranceResult>,
}

pub fn compute_all(data: &PerformanceData, inputs: &FlightInputs) -> SavedBundle {
    SavedBundle {
        takeoff: da20_core::takeoff::compute_takeoff(data, inputs).ok(),
        climb: da20_core::climb::compute_climb(data, inputs, inputs.cruise_pressure_alt_ft).ok(),
        cruise: da20_core::cruise::compute_cruise(data, inputs).ok(),
        landing: da20_core::landing::compute_landing(data, inputs).ok(),
        wind: da20_core::wind::compute_wind(data, inputs).ok(),
        endurance: None,
    }
}

use da20_core::types::SavedReading;

pub fn new_reading(
    label: String,
    phase: String,
    inputs: FlightInputs,
    bundle: SavedBundle,
) -> SavedReading {
    SavedReading {
        id: new_reading_id(),
        label,
        phase,
        created_at_iso: format_iso_now(),
        inputs,
        takeoff: bundle.takeoff,
        climb: bundle.climb,
        cruise: bundle.cruise,
        landing: bundle.landing,
        wind: bundle.wind,
        endurance: bundle.endurance,
    }
}

fn new_reading_id() -> String {
    if cfg!(target_arch = "wasm32") {
        format!("{}", js_sys::Date::now() as u64)
    } else {
        "test-id".into()
    }
}

/// Display a saved readout timestamp in the user's local timezone (not raw UTC `Z`).
pub fn format_readout_timestamp(iso: &str, locale: crate::i18n::Locale) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsValue;
        let loc = match locale {
            crate::i18n::Locale::Es => "es-CL",
            crate::i18n::Locale::En => "en-US",
        };
        js_sys::Date::new(&JsValue::from_str(iso))
            .to_locale_string_with_locale(loc)
            .as_string()
            .unwrap_or_else(|| iso.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = locale;
        iso.replace('T', " ").trim_end_matches('Z').to_string()
    }
}

fn format_iso_now() -> String {
    if cfg!(target_arch = "wasm32") {
        js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_default()
    } else {
        "1970-01-01T00:00:00.000Z".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::types::{defaults, FlightInputs};

    fn sample_reading(phase: &str, id: &str) -> SavedReading {
        SavedReading {
            id: id.into(),
            label: format!("Label {id}"),
            phase: phase.into(),
            created_at_iso: "2026-01-01T00:00:00.000Z".into(),
            inputs: FlightInputs {
                oat_c: 15.0,
                weight_lb: defaults::WEIGHT_LB,
                runway: "190".into(),
                ..Default::default()
            },
            takeoff: None,
            climb: None,
            cruise: None,
            landing: None,
            wind: None,
            endurance: None,
        }
    }

    #[test]
    fn push_and_filter_by_phase() {
        let mut store = ReadingsStore::default();
        store.push_phase("takeoff", sample_reading("takeoff", "a"));
        store.push_phase("climb", sample_reading("climb", "b"));
        store.push_phase("takeoff", sample_reading("takeoff", "c"));
        assert_eq!(store.for_phase("takeoff").len(), 2);
        assert_eq!(store.for_phase("climb").len(), 1);
        assert!(store.for_phase("landing").is_empty());
    }

    #[test]
    fn delete_and_clear_phase() {
        let mut store = ReadingsStore::default();
        store.push_phase("takeoff", sample_reading("takeoff", "a"));
        store.push_phase("takeoff", sample_reading("takeoff", "b"));
        store.delete_phase("takeoff", "a");
        assert_eq!(store.for_phase("takeoff").len(), 1);
        assert_eq!(store.for_phase("takeoff")[0].id, "b");
        store.clear_phase("takeoff");
        assert!(store.for_phase("takeoff").is_empty());
    }

    #[test]
    fn restore_find_in_phase() {
        let mut store = ReadingsStore::default();
        store.push_phase("climb", sample_reading("climb", "x"));
        let found = store.find_in_phase("climb", "x").expect("reading");
        assert_eq!(found.inputs.oat_c, 15.0);
    }

    #[test]
    fn export_json_csv_per_phase() {
        let mut store = ReadingsStore::default();
        store.push_phase("takeoff", sample_reading("takeoff", "1"));
        store.push_phase("climb", sample_reading("climb", "2"));
        let json = store.export_json_phase("takeoff");
        assert!(json.contains("\"id\": \"1\""));
        assert!(!json.contains("\"id\": \"2\""));
        let csv = store.export_csv_phase("takeoff");
        assert!(csv.lines().count() >= 2);
        assert!(csv.contains("1"));
        assert!(csv.contains("takeoff"));
    }

    #[test]
    fn persist_roundtrip_serde() {
        let mut store = ReadingsStore::default();
        store.push_phase("landing", sample_reading("landing", "z"));
        let raw = serde_json::to_string(&store).unwrap();
        let back: ReadingsStore = serde_json::from_str(&raw).unwrap();
        assert_eq!(store, back);
    }
}
