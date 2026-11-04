use da20_core::pressure::qnh_hpa_to_inhg;
use da20_core::types::FlightInputs;
use da20_core::wind_format::format_wind;
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
        let mut out = String::from(
            "id,label,phase,created_at,field_elev,qnh_hpa,qnh_inhg,oat,runway,wind,wind_kt,weight,climb_figure,obstacle_m,cruise_oat,takeoff_ground_ft,takeoff_over_obstacle_ft,climb_roc_fpm,cruise_tas,cruise_ff,landing_ground,landing_50ft,endurance_h,range_nm\n",
        );
        for r in &self.readings {
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
        id: format!("{}", js_sys::Date::now() as u64),
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

fn format_iso_now() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}
