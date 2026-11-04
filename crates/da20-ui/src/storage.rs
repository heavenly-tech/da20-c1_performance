use da20_core::types::FlightInputs;
use da20_core::PerformanceData;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const KEY: &str = "da20_performance_readings_v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReadingsStore {
    pub readings: Vec<da20_core::types::SavedReading>,
}

impl ReadingsStore {
    pub fn load() -> Self {
        LocalStorage::get(KEY).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = LocalStorage::set(KEY, self);
    }

    pub fn push(&mut self, reading: da20_core::types::SavedReading) {
        self.readings.push(reading);
        self.save();
    }

    pub fn delete(&mut self, id: &str) {
        self.readings.retain(|r| r.id != id);
        self.save();
    }

    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&self.readings).unwrap_or_else(|_| "[]".into())
    }

    pub fn export_csv(&self) -> String {
        let mut out = String::from("id,label,created_at,field_elev,qnh,oat,runway,wind_dir,wind_kt,weight,takeoff_ground_ft,takeoff_50ft_ft,cruise_tas,cruise_ff,landing_ground,landing_50ft\n");
        for r in &self.readings {
            let i = &r.inputs;
            let t_gr = r.takeoff.as_ref().map(|t| t.ground_roll_ft);
            let t_50 = r.takeoff.as_ref().map(|t| t.distance_over_50ft_ft);
            let tas = r.cruise.as_ref().map(|c| c.tas_kt);
            let ff = r.cruise.as_ref().map(|c| c.fuel_flow_us_gal_h);
            let l_gr = r.landing.as_ref().map(|l| l.ground_roll_ft);
            let l_50 = r.landing.as_ref().map(|l| l.distance_over_50ft_ft);
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                r.id,
                escape_csv(&r.label),
                r.created_at_iso,
                i.field_elevation_ft,
                i.qnh_hpa,
                i.oat_c,
                i.runway,
                i.wind_dir_deg,
                i.wind_speed_kt,
                i.weight_lb,
                fmt_opt(t_gr),
                fmt_opt(t_50),
                fmt_opt(tas),
                fmt_opt(ff),
                fmt_opt(l_gr),
                fmt_opt(l_50),
            ));
        }
        out
    }
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
}

pub fn compute_all(data: &PerformanceData, inputs: &FlightInputs) -> SavedBundle {
    SavedBundle {
        takeoff: da20_core::takeoff::compute_takeoff(data, inputs).ok(),
        climb: da20_core::climb::compute_climb(data, inputs, inputs.cruise_pressure_alt_ft).ok(),
        cruise: da20_core::cruise::compute_cruise(data, inputs).ok(),
        landing: da20_core::landing::compute_landing(data, inputs).ok(),
        wind: da20_core::wind::compute_wind(data, inputs).ok(),
    }
}

use da20_core::types::SavedReading;

pub fn new_reading(label: String, inputs: FlightInputs, bundle: SavedBundle) -> SavedReading {
    SavedReading {
        id: format!("{}", js_sys::Date::now() as u64),
        label,
        created_at_iso: format_iso_now(),
        inputs,
        takeoff: bundle.takeoff,
        climb: bundle.climb,
        cruise: bundle.cruise,
        landing: bundle.landing,
        wind: bundle.wind,
    }
}

fn format_iso_now() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}
