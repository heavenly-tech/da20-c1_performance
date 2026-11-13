use std::collections::BTreeMap;

use da20_core::chart_bounds::ChartOutOfRange;
use da20_core::climb::ClimbError;
use da20_core::cruise::CruiseError;
use da20_core::landing::LandingError;
use da20_core::takeoff::TakeoffError;
use da20_core::types::{climb_roc_fairing_multiplier, cruise_fairing_multiplier, FlightInputs};
use da20_core::wind_format::format_wind;
use da20_core::PerformanceData;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const KEY_V4: &str = "da20_performance_readings_v4";
const KEY_V3: &str = "da20_performance_readings_v3";
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

/// v3 → v4: backfill `climb.rate_of_climb_chart_fpm` when missing (default = no separate chart field).
fn backfill_climb_chart_roc(store: &mut ReadingsStore) {
    for readings in store.by_phase.values_mut() {
        for r in readings.iter_mut() {
            if let Some(ref mut c) = r.climb {
                if c.rate_of_climb_chart_fpm.is_none() {
                    let m = climb_roc_fairing_multiplier(r.inputs.wheel_fairings_installed);
                    c.rate_of_climb_chart_fpm = Some(if m < 1.0 - 1e-9 {
                        c.rate_of_climb_fpm / m
                    } else {
                        c.rate_of_climb_fpm
                    });
                }
            }
        }
    }
}

/// Backfill Table 3 chart TAS / fuel flow when missing (v4 saved before separate `tas_chart_kt`).
fn backfill_cruise_chart_values(store: &mut ReadingsStore) {
    for readings in store.by_phase.values_mut() {
        for r in readings.iter_mut() {
            if let Some(ref mut c) = r.cruise {
                let m = cruise_fairing_multiplier(r.inputs.wheel_fairings_installed);
                if c.tas_chart_kt.is_none() {
                    c.tas_chart_kt = Some(if m < 1.0 - 1e-9 {
                        c.tas_kt / m
                    } else {
                        c.tas_kt
                    });
                }
                if c.fuel_flow_chart_us_gal_h.is_none() {
                    c.fuel_flow_chart_us_gal_h = Some(c.fuel_flow_us_gal_h);
                }
            }
        }
    }
}

fn backfill_endurance_chart_range(store: &mut ReadingsStore) {
    for readings in store.by_phase.values_mut() {
        for r in readings.iter_mut() {
            let Some(ref cr) = r.cruise else {
                continue;
            };
            let Some(ref mut e) = r.endurance else {
                continue;
            };
            if e.tas_chart_kt.is_none() {
                e.tas_chart_kt = cr.tas_chart_kt;
            }
            if e.range_chart_nm.is_none() && e.endurance_h > 0.0 {
                let tas_c = e.tas_chart_kt.or(cr.tas_chart_kt).unwrap_or(e.tas_kt);
                e.range_chart_nm = Some(e.endurance_h * tas_c);
            }
        }
    }
}

fn backfill_fairing_chart_fields(store: &mut ReadingsStore) {
    backfill_climb_chart_roc(store);
    backfill_cruise_chart_values(store);
    backfill_endurance_chart_range(store);
}

/// Migrate saved readout JSON: `field_elevation_ft` → `pressure_altitude_input_ft`; drop `qnh_hpa`.
fn migrate_legacy_readings_json(mut v: serde_json::Value) -> Option<serde_json::Value> {
    let by_phase = v.get_mut("by_phase")?.as_object_mut()?;
    for (_, readings_val) in by_phase.iter_mut() {
        let readings = readings_val.as_array_mut()?;
        for reading in readings {
            let inputs = reading.get_mut("inputs")?.as_object_mut()?;
            if let Some(elev) = inputs.remove("field_elevation_ft") {
                inputs.insert("pressure_altitude_input_ft".to_string(), elev);
            }
            inputs.remove("qnh_hpa");
        }
    }
    Some(v)
}

impl ReadingsStore {
    pub fn save(&self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(KEY_V4, self);
        }
    }

    pub fn load() -> Self {
        if cfg!(target_arch = "wasm32") {
            if let Ok(mut store) = LocalStorage::get::<ReadingsStore>(KEY_V4) {
                backfill_fairing_chart_fields(&mut store);
                store.save();
                return store;
            }
            if let Ok(mut store) = LocalStorage::get::<ReadingsStore>(KEY_V3) {
                backfill_fairing_chart_fields(&mut store);
                LocalStorage::delete(KEY_V3);
                store.save();
                return store;
            }
            if let Ok(v) = LocalStorage::get::<serde_json::Value>(KEY_V2) {
                if let Some(migrated) = migrate_legacy_readings_json(v) {
                    if let Ok(mut store) = serde_json::from_value::<ReadingsStore>(migrated) {
                        LocalStorage::delete(KEY_V2);
                        backfill_fairing_chart_fields(&mut store);
                        store.save();
                        return store;
                    }
                }
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
        "id,label,phase,created_at,pa_ft,oat,runway,wind,wind_kt,weight,climb_figure,obstacle_m,cruise_oat,takeoff_ground_ft,takeoff_over_obstacle_ft,climb_roc_fpm,cruise_tas,cruise_ff,landing_ground,landing_50ft,endurance_h,range_nm\n",
    );
    for r in readings {
        let i = &r.inputs;
        let wind = format_wind(i.wind_dir_deg, i.wind_speed_kt);
        let climb_fig = format!("{:?}", i.climb_figure);
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
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.id,
            escape_csv(&r.label),
            escape_csv(&r.phase),
            r.created_at_iso,
            i.pressure_altitude_input_ft,
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
    pub cruise_bhp_not_achievable: bool,
    pub cruise_out_of_chart: bool,
    pub takeoff_out_of_chart: bool,
    pub climb_out_of_chart: bool,
    pub landing_out_of_chart: bool,
    pub takeoff_ooc: Option<ChartOutOfRange>,
    pub climb_ooc: Option<ChartOutOfRange>,
    pub cruise_ooc: Option<ChartOutOfRange>,
    pub landing_ooc: Option<ChartOutOfRange>,
    pub landing: Option<da20_core::types::LandingResult>,
    pub wind: Option<da20_core::types::WindResult>,
    pub endurance: Option<da20_core::types::EnduranceResult>,
}

pub fn compute_all_isolated(
    data: &PerformanceData,
    prefs: &crate::prefs::FlightPrefs,
) -> SavedBundle {
    use crate::phase_state::{
        assemble_climb_inputs, assemble_cruise_inputs, assemble_endurance_inputs,
        assemble_landing_inputs, assemble_takeoff_inputs,
    };
    use da20_core::endurance::compute_endurance;

    let takeoff_in = assemble_takeoff_inputs(prefs);
    let climb_in = assemble_climb_inputs(prefs);
    let cruise_in = assemble_cruise_inputs(prefs, data);
    let landing_in = assemble_landing_inputs(prefs);
    let endurance_in = assemble_endurance_inputs(prefs, data);

    let takeoff_res = da20_core::takeoff::compute_takeoff(data, &takeoff_in);
    let climb_res =
        da20_core::climb::compute_climb(data, &climb_in, prefs.climb.climb_target_altitude_ft);
    let cruise_res = da20_core::cruise::compute_cruise(data, &cruise_in);
    let landing_res = da20_core::landing::compute_landing(data, &landing_in);

    let takeoff_ooc = takeoff_ooc_from_err(takeoff_res.as_ref().err());
    let climb_ooc = climb_ooc_from_err(climb_res.as_ref().err());
    let cruise_ooc = cruise_ooc_from_err(cruise_res.as_ref().err());
    let landing_ooc = landing_ooc_from_err(landing_res.as_ref().err());
    let cruise_bhp_not_achievable = matches!(&cruise_res, Err(CruiseError::BhpNotAchievable));

    SavedBundle {
        takeoff: takeoff_res.ok(),
        takeoff_out_of_chart: takeoff_ooc.is_some(),
        takeoff_ooc,
        climb: climb_res.ok(),
        climb_out_of_chart: climb_ooc.is_some(),
        climb_ooc,
        cruise: cruise_res.ok(),
        cruise_bhp_not_achievable,
        cruise_out_of_chart: cruise_ooc.is_some(),
        cruise_ooc,
        landing: landing_res.ok(),
        landing_out_of_chart: landing_ooc.is_some(),
        landing_ooc,
        wind: da20_core::wind::compute_wind(data, &takeoff_in).ok(),
        endurance: compute_endurance(data, &endurance_in).ok(),
    }
}

pub fn compute_all(data: &PerformanceData, inputs: &FlightInputs) -> SavedBundle {
    let takeoff_res = da20_core::takeoff::compute_takeoff(data, inputs);
    let climb_res = da20_core::climb::compute_climb(data, inputs, inputs.cruise_pressure_alt_ft);
    let cruise_res = da20_core::cruise::compute_cruise(data, inputs);
    let landing_res = da20_core::landing::compute_landing(data, inputs);

    let takeoff_ooc = takeoff_ooc_from_err(takeoff_res.as_ref().err());
    let climb_ooc = climb_ooc_from_err(climb_res.as_ref().err());
    let cruise_ooc = cruise_ooc_from_err(cruise_res.as_ref().err());
    let landing_ooc = landing_ooc_from_err(landing_res.as_ref().err());
    let cruise_bhp_not_achievable = matches!(&cruise_res, Err(CruiseError::BhpNotAchievable));

    SavedBundle {
        takeoff: takeoff_res.ok(),
        takeoff_out_of_chart: takeoff_ooc.is_some(),
        takeoff_ooc,
        climb: climb_res.ok(),
        climb_out_of_chart: climb_ooc.is_some(),
        climb_ooc,
        cruise: cruise_res.ok(),
        cruise_bhp_not_achievable,
        cruise_out_of_chart: cruise_ooc.is_some(),
        cruise_ooc,
        landing: landing_res.ok(),
        landing_out_of_chart: landing_ooc.is_some(),
        landing_ooc,
        wind: da20_core::wind::compute_wind(data, inputs).ok(),
        endurance: None,
    }
}

fn takeoff_ooc_from_err(err: Option<&TakeoffError>) -> Option<ChartOutOfRange> {
    match err {
        Some(TakeoffError::OutOfChart(e)) => Some(*e),
        _ => None,
    }
}

fn climb_ooc_from_err(err: Option<&ClimbError>) -> Option<ChartOutOfRange> {
    match err {
        Some(ClimbError::OutOfChart(e)) => Some(*e),
        _ => None,
    }
}

fn cruise_ooc_from_err(err: Option<&CruiseError>) -> Option<ChartOutOfRange> {
    match err {
        Some(CruiseError::OutOfChart { axis, min, max }) => Some(ChartOutOfRange {
            label: axis,
            min: *min,
            max: *max,
        }),
        _ => None,
    }
}

fn landing_ooc_from_err(err: Option<&LandingError>) -> Option<ChartOutOfRange> {
    match err {
        Some(LandingError::OutOfChart(e)) => Some(*e),
        _ => None,
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
            .to_locale_string(loc, &JsValue::UNDEFINED)
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
    use crate::i18n::Locale;
    use da20_core::climb::compute_climb;
    use da20_core::types::{defaults, FlightInputs};
    use da20_core::PerformanceData;

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
    fn migrate_v2_json_drops_qnh_and_renames_field() {
        let raw = r#"{
            "by_phase": {
                "takeoff": [{
                    "id": "1",
                    "label": "t",
                    "phase": "takeoff",
                    "created_at_iso": "2026-01-01T00:00:00.000Z",
                    "inputs": {
                        "field_elevation_ft": 3000.0,
                        "qnh_hpa": 980.0,
                        "oat_c": 20.0,
                        "runway": "190",
                        "wind_dir_deg": 200.0,
                        "wind_speed_kt": 8.0,
                        "weight_lb": 1764.0,
                        "cruise_pressure_alt_ft": 6500.0,
                        "cruise_rpm": 2600.0,
                        "cruise_oat_c": 10.0,
                        "obstacle_height_m": 5.0,
                        "pa_override": null,
                        "climb_figure": "Fig55CruiseFlaps",
                        "wheel_fairings_installed": true,
                        "fuel_usable_us_gal": 24.0,
                        "fuel_reserve_us_gal": 4.5
                    },
                    "takeoff": null,
                    "climb": null,
                    "cruise": null,
                    "landing": null,
                    "wind": null,
                    "endurance": null
                }]
            }
        }"#;
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        let migrated = migrate_legacy_readings_json(v).expect("migrate");
        let store: ReadingsStore = serde_json::from_value(migrated).expect("parse store");
        let i = &store.for_phase("takeoff")[0].inputs;
        assert_eq!(i.pressure_altitude_input_ft, 3000.0);
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
        assert!(csv.contains("pa_ft"));
    }

    #[test]
    fn persist_roundtrip_serde() {
        let mut store = ReadingsStore::default();
        store.push_phase("landing", sample_reading("landing", "z"));
        let raw = serde_json::to_string(&store).unwrap();
        let back: ReadingsStore = serde_json::from_str(&raw).unwrap();
        assert_eq!(store, back);
    }

    /// v4 schema: `climb.rate_of_climb_chart_fpm` optional; migration backfills from penalized ROC.
    #[test]
    fn climb_saved_reading_without_chart_roc_backfills_on_load() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            wheel_fairings_installed: false,
            ..Default::default()
        };
        let mut climbing = compute_climb(&data, &inputs, inputs.cruise_pressure_alt_ft).unwrap();
        climbing.rate_of_climb_chart_fpm = None;

        let mut store = ReadingsStore::default();
        store.by_phase.insert(
            "climb".into(),
            vec![SavedReading {
                id: "c1".into(),
                label: String::new(),
                phase: "climb".into(),
                created_at_iso: "2026-01-01T00:00:00.000Z".into(),
                inputs,
                takeoff: None,
                climb: Some(climbing),
                cruise: None,
                landing: None,
                wind: None,
                endurance: None,
            }],
        );

        backfill_fairing_chart_fields(&mut store);
        let cl = store
            .for_phase("climb")
            .first()
            .and_then(|r| r.climb.as_ref())
            .expect("climb block");
        let chart = cl.rate_of_climb_chart_fpm.expect("backfilled chart ROC");
        assert!((chart * 0.97 - cl.rate_of_climb_fpm).abs() < 1e-6);
    }

    #[test]
    fn readout_timestamp_native_strips_utc_z_suffix() {
        let s = format_readout_timestamp("2026-01-01T12:30:00.000Z", Locale::En);
        assert!(!s.ends_with('Z'));
        assert!(s.contains("2026"));
    }

    #[test]
    fn tab_prefs_edit_reload_matches_compute_all_isolated() {
        use crate::inputs::{
            shared_airfield_set_oat_c, shared_airfield_set_pressure_altitude, AirfieldInputsMode,
        };
        use crate::phase_state::{assemble_takeoff_inputs, hydrate_tab_inputs};
        use crate::phases::Phase;
        use crate::prefs::FlightPrefs;
        use da20_core::takeoff::compute_takeoff;
        use da20_core::types::ChartPhase;

        let data = PerformanceData::production();
        let mut prefs = FlightPrefs::default();
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Takeoff, &prefs, &data, &mut inputs);
        shared_airfield_set_pressure_altitude(
            AirfieldInputsMode::Takeoff,
            Some(ChartPhase::Takeoff),
            &mut prefs,
            &mut inputs,
            4200.0,
        );
        shared_airfield_set_oat_c(AirfieldInputsMode::Takeoff, &mut prefs, &mut inputs, 12.0);
        crate::prefs::persist_takeoff_tab_from_inputs(&mut prefs, &inputs);

        let roll_direct = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs))
            .unwrap()
            .ground_roll_ft;
        let bundle_live = compute_all_isolated(&data, &prefs);
        assert!(
            (bundle_live.takeoff.as_ref().unwrap().ground_roll_ft - roll_direct).abs()
                < f64::EPSILON
        );

        let json = serde_json::to_string(&prefs).expect("serialize prefs");
        let loaded: FlightPrefs = serde_json::from_str(&json).expect("deserialize prefs");
        let bundle_reload = compute_all_isolated(&data, &loaded);
        assert_eq!(
            bundle_live.takeoff.as_ref().unwrap().ground_roll_ft,
            bundle_reload.takeoff.as_ref().unwrap().ground_roll_ft
        );
    }
}
