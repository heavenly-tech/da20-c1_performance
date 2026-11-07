use da20_core::takeoff_rounding::TakeoffDistanceRounding;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const KEY: &str = "da20_takeoff_prefs_v1";

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

impl TakeoffPrefs {
    pub fn load() -> Self {
        if cfg!(target_arch = "wasm32") {
            LocalStorage::get(KEY).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(KEY, self);
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
}
