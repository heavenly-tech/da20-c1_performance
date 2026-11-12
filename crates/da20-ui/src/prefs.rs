use da20_core::takeoff_rounding::TakeoffDistanceRounding;
use da20_core::types::{FlightInputs, PropellerKind};
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

pub const TAKEOFF_PREFS_STORAGE_KEY: &str = "da20_takeoff_prefs_v1";
pub const FLIGHT_PREFS_STORAGE_KEY: &str = "da20_flight_prefs_v1";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlightPrefs {
    #[serde(default)]
    pub propeller: PropellerKind,
    #[serde(default = "default_wheel_fairings_installed")]
    pub wheel_fairings_installed: bool,
}

impl Default for FlightPrefs {
    fn default() -> Self {
        Self {
            propeller: PropellerKind::default(),
            wheel_fairings_installed: default_wheel_fairings_installed(),
        }
    }
}

impl FlightPrefs {
    pub fn load() -> Self {
        if cfg!(target_arch = "wasm32") {
            LocalStorage::get(FLIGHT_PREFS_STORAGE_KEY).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(FLIGHT_PREFS_STORAGE_KEY, self);
        }
    }

    pub fn from_inputs(inputs: &FlightInputs) -> Self {
        Self {
            propeller: inputs.propeller,
            wheel_fairings_installed: inputs.wheel_fairings_installed,
        }
    }
}

pub fn save_flight_prefs_from_inputs(inputs: &FlightInputs) {
    FlightPrefs::from_inputs(inputs).save();
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
        assert_eq!(FLIGHT_PREFS_STORAGE_KEY, "da20_flight_prefs_v1");
        let prefs = FlightPrefs {
            propeller: PropellerKind::Mt175R,
            wheel_fairings_installed: false,
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, prefs);
    }

    #[test]
    fn prefs_storage_key_and_kg_round_trip() {
        assert_eq!(TAKEOFF_PREFS_STORAGE_KEY, "da20_takeoff_prefs_v1");
        let prefs = TakeoffPrefs {
            weight: WeightUnit::Kg,
            obstacle: ObstacleUnit::Feet,
            distance_rounding: TakeoffDistanceRounding::Ceil10Meters,
        };
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: TakeoffPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, prefs);
        let missing_weight = r#"{"obstacle":"Meters","distance_rounding":"Raw"}"#;
        let defaults: TakeoffPrefs = serde_json::from_str(missing_weight).expect("default weight");
        assert_eq!(defaults.weight, WeightUnit::Lb);
    }
}
