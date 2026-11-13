use da20_core::chart_bounds::limits;
use da20_core::cruise::compute_cruise;
use da20_core::cruise::isa_temp_c_at_pa;
use da20_core::takeoff_rounding::TakeoffDistanceRounding;
use da20_core::types::{defaults, CruiseSolveMode, FlightInputs, PropellerKind};
use da20_core::PerformanceData;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

use crate::ground_oat::estimated_oat_c_from_ground;

pub const TAKEOFF_PREFS_STORAGE_KEY: &str = "da20_takeoff_prefs_v1";
pub const FLIGHT_PREFS_STORAGE_KEY: &str = "da20_flight_prefs_v2";

pub const PREFERRED_DEFAULT_PERCENT_BHP: f64 = 65.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightUnit {
    Kg,
    Lb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObstacleUnit {
    Meters,
    Feet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeoffPrefs {
    #[serde(default = "default_weight_unit")]
    pub weight: WeightUnit,
    #[serde(default = "default_obstacle_unit")]
    pub obstacle: ObstacleUnit,
    #[serde(default)]
    pub distance_rounding: TakeoffDistanceRounding,
}

fn default_weight_unit() -> WeightUnit {
    WeightUnit::Lb
}

fn default_obstacle_unit() -> ObstacleUnit {
    ObstacleUnit::Meters
}

impl Default for TakeoffPrefs {
    fn default() -> Self {
        Self {
            weight: default_weight_unit(),
            obstacle: default_obstacle_unit(),
            distance_rounding: TakeoffDistanceRounding::Raw,
        }
    }
}

fn default_wheel_fairings_installed() -> bool {
    true
}

fn default_ground_elevation_ft() -> f64 {
    defaults::FIELD_ELEVATION_FT
}

fn default_ground_oat_c() -> f64 {
    defaults::OAT_C
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlightPrefs {
    #[serde(default)]
    pub propeller: PropellerKind,
    #[serde(default = "default_wheel_fairings_installed")]
    pub wheel_fairings_installed: bool,
    #[serde(default = "default_ground_elevation_ft")]
    pub ground_elevation_ft: f64,
    #[serde(default = "default_ground_oat_c")]
    pub ground_oat_c: f64,
    /// When false, cruise OAT follows the ground lapse estimate.
    #[serde(default)]
    pub cruise_oat_manual: bool,
    /// Set after the user picks RPM vs %BHP; `None` → default %BHP mode.
    #[serde(default)]
    pub cruise_solve_saved: Option<CruiseSolveMode>,
    #[serde(default)]
    pub cruise_percent_bhp_saved: Option<f64>,
    #[serde(default)]
    pub landing_ground_idle_1000_rpm: bool,
}

impl Default for FlightPrefs {
    fn default() -> Self {
        Self {
            propeller: PropellerKind::default(),
            wheel_fairings_installed: default_wheel_fairings_installed(),
            ground_elevation_ft: default_ground_elevation_ft(),
            ground_oat_c: default_ground_oat_c(),
            cruise_oat_manual: false,
            cruise_solve_saved: None,
            cruise_percent_bhp_saved: None,
            landing_ground_idle_1000_rpm: false,
        }
    }
}

pub fn clamp_cruise_oat_for_propeller(inputs: &FlightInputs, oat: f64) -> f64 {
    if inputs.propeller == PropellerKind::Mt175R {
        oat.clamp(limits::FIG52_OAT_C.0, limits::FIG52_OAT_C.1)
    } else {
        let isa = isa_temp_c_at_pa(inputs.cruise_pressure_alt_ft);
        let (dmin, dmax) = limits::TABLE3_DISA_C;
        oat.clamp(isa + dmin, isa + dmax)
    }
}

pub fn apply_estimated_cruise_oat(inputs: &mut FlightInputs, prefs: &FlightPrefs) {
    let est = estimated_oat_c_from_ground(
        prefs.ground_elevation_ft,
        prefs.ground_oat_c,
        inputs.cruise_pressure_alt_ft,
    );
    inputs.cruise_oat_c = clamp_cruise_oat_for_propeller(inputs, est);
}

pub fn sync_ground_to_takeoff_inputs(inputs: &mut FlightInputs, prefs: &FlightPrefs) {
    inputs.pressure_altitude_input_ft = prefs.ground_elevation_ft;
    inputs.oat_c = prefs.ground_oat_c;
    if !prefs.cruise_oat_manual {
        apply_estimated_cruise_oat(inputs, prefs);
    }
}

/// Picks the preferred 65 %BHP when both propellers are in-chart at default PA/OAT; otherwise the first in-chart candidate.
pub fn resolved_default_cruise_percent_bhp(data: &PerformanceData) -> f64 {
    let candidates = [
        PREFERRED_DEFAULT_PERCENT_BHP,
        60.0,
        70.0,
        55.0,
        75.0,
        50.0,
        80.0,
        56.25,
    ];
    let mut inputs = FlightInputs {
        cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
        ..Default::default()
    };
    for &pct in &candidates {
        inputs.cruise_percent_bhp = pct;
        let mut ok = true;
        for prop in [PropellerKind::Sensenich, PropellerKind::Mt175R] {
            inputs.propeller = prop;
            if compute_cruise(data, &inputs).is_err() {
                ok = false;
                break;
            }
        }
        if ok {
            return pct;
        }
    }
    PREFERRED_DEFAULT_PERCENT_BHP
}

impl FlightPrefs {
    pub fn load() -> Self {
        if cfg!(target_arch = "wasm32") {
            if let Ok(p) = LocalStorage::get::<Self>(FLIGHT_PREFS_STORAGE_KEY) {
                return p;
            }
            if let Ok(legacy) = LocalStorage::get::<FlightPrefsV1>(FLIGHT_PREFS_STORAGE_KEY) {
                return legacy.into();
            }
            Self::default()
        } else {
            Self::default()
        }
    }

    pub fn save(self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(FLIGHT_PREFS_STORAGE_KEY, self);
        }
    }

    pub fn apply_to_inputs(self, data: &PerformanceData, inputs: &mut FlightInputs) {
        inputs.propeller = self.propeller;
        inputs.wheel_fairings_installed = self.wheel_fairings_installed;
        sync_ground_to_takeoff_inputs(inputs, &self);
        inputs.cruise_solve_mode = self
            .cruise_solve_saved
            .unwrap_or(CruiseSolveMode::SetPercentBhp);
        inputs.cruise_percent_bhp = self
            .cruise_percent_bhp_saved
            .unwrap_or_else(|| resolved_default_cruise_percent_bhp(data));
        if !self.cruise_oat_manual {
            apply_estimated_cruise_oat(inputs, &self);
        }
        inputs.landing_ground_idle_1000_rpm = self.landing_ground_idle_1000_rpm;
    }

    pub fn mark_cruise_solve_saved(&mut self, mode: CruiseSolveMode) {
        self.cruise_solve_saved = Some(mode);
    }

    pub fn mark_cruise_percent_saved(&mut self, pct: f64) {
        self.cruise_percent_bhp_saved = Some(pct);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct FlightPrefsV1 {
    #[serde(default)]
    propeller: PropellerKind,
    #[serde(default = "default_wheel_fairings_installed")]
    wheel_fairings_installed: bool,
}

impl From<FlightPrefsV1> for FlightPrefs {
    fn from(v1: FlightPrefsV1) -> Self {
        Self {
            propeller: v1.propeller,
            wheel_fairings_installed: v1.wheel_fairings_installed,
            ..Self::default()
        }
    }
}

pub fn snapshot_and_save_flight_prefs(inputs: &FlightInputs, prefs: &mut FlightPrefs) {
    prefs.propeller = inputs.propeller;
    prefs.wheel_fairings_installed = inputs.wheel_fairings_installed;
    prefs.landing_ground_idle_1000_rpm = inputs.landing_ground_idle_1000_rpm;
    prefs.save();
}

impl TakeoffPrefs {
    pub fn load() -> Self {
        if cfg!(target_arch = "wasm32") {
            LocalStorage::get(TAKEOFF_PREFS_STORAGE_KEY).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(TAKEOFF_PREFS_STORAGE_KEY, self);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use da20_core::rev29::engine::{kg_to_lb, lb_to_kg};

    #[test]
    fn kg_lb_roundtrip() {
        let kg = 725.0;
        let lb = kg_to_lb(kg);
        assert!((lb_to_kg(lb) - kg).abs() < 1e-6);
    }

    #[test]
    fn prefs_default_units() {
        let p = TakeoffPrefs::default();
        assert_eq!(p.weight, WeightUnit::Lb);
        assert_eq!(p.obstacle, ObstacleUnit::Meters);
    }

    #[test]
    fn flight_prefs_roundtrip_json() {
        let prefs = FlightPrefs {
            propeller: PropellerKind::Mt175R,
            wheel_fairings_installed: false,
            ground_elevation_ft: 2000.0,
            ground_oat_c: 25.0,
            cruise_oat_manual: true,
            cruise_solve_saved: Some(CruiseSolveMode::SetRpm),
            cruise_percent_bhp_saved: Some(70.0),
            landing_ground_idle_1000_rpm: true,
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, prefs);
    }

    #[test]
    fn default_cruise_mode_without_saved_state() {
        let data = PerformanceData::production();
        let prefs = FlightPrefs::default();
        let mut inputs = FlightInputs::default();
        prefs.apply_to_inputs(&data, &mut inputs);
        assert_eq!(inputs.cruise_solve_mode, CruiseSolveMode::SetPercentBhp);
        let pct = resolved_default_cruise_percent_bhp(&data);
        assert!((inputs.cruise_percent_bhp - pct).abs() < f64::EPSILON);
    }

    #[test]
    fn saved_rpm_mode_persists() {
        let data = PerformanceData::production();
        let prefs = FlightPrefs {
            cruise_solve_saved: Some(CruiseSolveMode::SetRpm),
            cruise_percent_bhp_saved: Some(72.0),
            ..FlightPrefs::default()
        };
        let mut inputs = FlightInputs::default();
        prefs.apply_to_inputs(&data, &mut inputs);
        assert_eq!(inputs.cruise_solve_mode, CruiseSolveMode::SetRpm);
        assert!((inputs.cruise_percent_bhp - 72.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ground_estimate_syncs_takeoff_and_cruise_oat() {
        let prefs = FlightPrefs {
            ground_elevation_ft: 2000.0,
            ground_oat_c: 25.0,
            cruise_oat_manual: false,
            ..FlightPrefs::default()
        };
        let mut inputs = FlightInputs {
            propeller: PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 8000.0,
            ..Default::default()
        };
        sync_ground_to_takeoff_inputs(&mut inputs, &prefs);
        assert_eq!(inputs.pressure_altitude_input_ft, 2000.0);
        assert_eq!(inputs.oat_c, 25.0);
        assert!((inputs.cruise_oat_c - 13.12).abs() < 0.02);
    }

    #[test]
    fn manual_cruise_oat_override_sticks() {
        let prefs = FlightPrefs {
            cruise_oat_manual: true,
            ground_elevation_ft: 2000.0,
            ground_oat_c: 25.0,
            ..FlightPrefs::default()
        };
        let mut inputs = FlightInputs {
            cruise_oat_c: 5.0,
            ..Default::default()
        };
        sync_ground_to_takeoff_inputs(&mut inputs, &prefs);
        assert!((inputs.cruise_oat_c - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn preferred_default_percent_bhp_in_chart_both_propellers() {
        let data = PerformanceData::production();
        let pct = resolved_default_cruise_percent_bhp(&data);
        let mut inputs = FlightInputs {
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: pct,
            ..Default::default()
        };
        for prop in [PropellerKind::Sensenich, PropellerKind::Mt175R] {
            inputs.propeller = prop;
            compute_cruise(&data, &inputs).expect("in chart");
        }
        assert!((pct - PREFERRED_DEFAULT_PERCENT_BHP).abs() < f64::EPSILON);
    }
}
