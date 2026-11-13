use da20_core::chart_bounds::limits;
use da20_core::cruise::compute_cruise;
use da20_core::cruise::isa_temp_c_at_pa;
use da20_core::takeoff_rounding::TakeoffDistanceRounding;
use da20_core::types::{
    defaults, ChartPhase, ClimbFigure, CruiseSolveMode, FlightInputs, PaOverride, PropellerKind,
};
use da20_core::PerformanceData;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

use crate::ground_oat::estimated_oat_c_from_ground;

pub const TAKEOFF_PREFS_STORAGE_KEY: &str = "da20_takeoff_prefs_v1";
pub const FLIGHT_PREFS_STORAGE_KEY: &str = "da20_flight_prefs_v2";
pub const FLIGHT_PREFS_STORAGE_KEY_V1: &str = "da20_flight_prefs_v1";

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

fn default_tab_weight_lb() -> f64 {
    defaults::WEIGHT_LB
}

fn default_runway() -> String {
    defaults::RUNWAY.to_string()
}

fn default_wind_dir() -> f64 {
    defaults::WIND_DIR_DEG
}

fn default_wind_speed() -> f64 {
    defaults::WIND_SPEED_KT
}

fn default_obstacle_m() -> f64 {
    defaults::OBSTACLE_HEIGHT_DEFAULT_M
}

fn default_cruise_pressure_alt_ft() -> f64 {
    defaults::CRUISE_PRESSURE_ALT_FT
}

fn default_cruise_rpm() -> f64 {
    defaults::CRUISE_RPM
}

fn default_fuel_usable() -> f64 {
    defaults::FUEL_USABLE_US_GAL
}

fn default_fuel_reserve() -> f64 {
    defaults::FUEL_RESERVE_US_GAL
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TakeoffTabPrefs {
    #[serde(default = "default_ground_elevation_ft")]
    pub pressure_altitude_ft: f64,
    #[serde(default = "default_ground_oat_c")]
    pub oat_c: f64,
    #[serde(default = "default_tab_weight_lb")]
    pub weight_lb: f64,
    #[serde(default = "default_runway")]
    pub runway: String,
    #[serde(default = "default_wind_dir")]
    pub wind_dir_deg: f64,
    #[serde(default = "default_wind_speed")]
    pub wind_speed_kt: f64,
    #[serde(default = "default_obstacle_m")]
    pub obstacle_height_m: f64,
}

impl Default for TakeoffTabPrefs {
    fn default() -> Self {
        Self {
            pressure_altitude_ft: default_ground_elevation_ft(),
            oat_c: default_ground_oat_c(),
            weight_lb: default_tab_weight_lb(),
            runway: default_runway(),
            wind_dir_deg: default_wind_dir(),
            wind_speed_kt: default_wind_speed(),
            obstacle_height_m: default_obstacle_m(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClimbTabPrefs {
    #[serde(default = "default_ground_elevation_ft")]
    pub pressure_altitude_ft: f64,
    #[serde(default = "default_ground_oat_c")]
    pub oat_c: f64,
    #[serde(default = "default_tab_weight_lb")]
    pub weight_lb: f64,
    #[serde(default)]
    pub climb_figure: ClimbFigure,
    #[serde(default)]
    pub pa_override: Option<PaOverride>,
    /// Target pressure altitude for climb time-to-altitude (not cruise-tab PA).
    #[serde(default = "default_cruise_pressure_alt_ft")]
    pub climb_target_altitude_ft: f64,
}

impl Default for ClimbTabPrefs {
    fn default() -> Self {
        Self {
            pressure_altitude_ft: default_ground_elevation_ft(),
            oat_c: default_ground_oat_c(),
            weight_lb: default_tab_weight_lb(),
            climb_figure: ClimbFigure::default(),
            pa_override: None,
            climb_target_altitude_ft: default_cruise_pressure_alt_ft(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LandingTabPrefs {
    #[serde(default = "default_ground_elevation_ft")]
    pub pressure_altitude_ft: f64,
    #[serde(default = "default_ground_oat_c")]
    pub oat_c: f64,
    #[serde(default)]
    pub pa_override: Option<PaOverride>,
    #[serde(default = "default_landing_ground_idle_1000_rpm")]
    pub landing_ground_idle_1000_rpm: bool,
}

impl Default for LandingTabPrefs {
    fn default() -> Self {
        Self {
            pressure_altitude_ft: default_ground_elevation_ft(),
            oat_c: default_ground_oat_c(),
            pa_override: None,
            landing_ground_idle_1000_rpm: default_landing_ground_idle_1000_rpm(),
        }
    }
}

/// Per-tab cruise / endurance planning (ground lapse, PA, RPM, weight, solve mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CruiseTabPrefs {
    #[serde(default = "default_ground_elevation_ft")]
    pub ground_elevation_ft: f64,
    #[serde(default = "default_ground_oat_c")]
    pub ground_oat_c: f64,
    #[serde(default = "default_cruise_pressure_alt_ft")]
    pub cruise_pressure_alt_ft: f64,
    #[serde(default = "default_tab_weight_lb")]
    pub weight_lb: f64,
    #[serde(default = "default_cruise_rpm")]
    pub cruise_rpm: f64,
    #[serde(default)]
    pub cruise_oat_manual: bool,
    #[serde(default)]
    pub cruise_solve_saved: Option<CruiseSolveMode>,
    #[serde(default)]
    pub cruise_percent_bhp_saved: Option<f64>,
    #[serde(default)]
    pub cruise_oat_c_saved: Option<f64>,
}

impl Default for CruiseTabPrefs {
    fn default() -> Self {
        Self {
            ground_elevation_ft: default_ground_elevation_ft(),
            ground_oat_c: default_ground_oat_c(),
            cruise_pressure_alt_ft: default_cruise_pressure_alt_ft(),
            weight_lb: default_tab_weight_lb(),
            cruise_rpm: default_cruise_rpm(),
            cruise_oat_manual: false,
            cruise_solve_saved: None,
            cruise_percent_bhp_saved: None,
            cruise_oat_c_saved: None,
        }
    }
}

impl CruiseTabPrefs {
    pub fn mark_cruise_solve_saved(&mut self, mode: CruiseSolveMode) {
        self.cruise_solve_saved = Some(mode);
    }

    pub fn mark_cruise_percent_saved(&mut self, pct: f64) {
        self.cruise_percent_bhp_saved = Some(pct);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnduranceTabPrefs {
    #[serde(flatten)]
    pub cruise: CruiseTabPrefs,
    #[serde(default = "default_fuel_usable")]
    pub fuel_usable_us_gal: f64,
    #[serde(default = "default_fuel_reserve")]
    pub fuel_reserve_us_gal: f64,
}

impl Default for EnduranceTabPrefs {
    fn default() -> Self {
        Self {
            cruise: CruiseTabPrefs::default(),
            fuel_usable_us_gal: default_fuel_usable(),
            fuel_reserve_us_gal: default_fuel_reserve(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlightPrefs {
    #[serde(default)]
    pub propeller: PropellerKind,
    #[serde(default = "default_wheel_fairings_installed")]
    pub wheel_fairings_installed: bool,
    #[serde(default)]
    pub takeoff: TakeoffTabPrefs,
    #[serde(default)]
    pub climb: ClimbTabPrefs,
    #[serde(default)]
    pub landing: LandingTabPrefs,
    #[serde(default, alias = "cruise_planning")]
    pub cruise: CruiseTabPrefs,
    #[serde(default)]
    pub endurance: EnduranceTabPrefs,
    #[serde(default, rename = "cruise_oat_manual", skip_serializing)]
    legacy_cruise_oat_manual: bool,
    #[serde(default, rename = "cruise_solve_saved", skip_serializing)]
    legacy_cruise_solve_saved: Option<CruiseSolveMode>,
    #[serde(default, rename = "cruise_percent_bhp_saved", skip_serializing)]
    legacy_cruise_percent_bhp_saved: Option<f64>,
    #[serde(default, rename = "cruise_oat_c_saved", skip_serializing)]
    legacy_cruise_oat_c_saved: Option<f64>,
    #[serde(
        default = "default_landing_ground_idle_1000_rpm",
        rename = "landing_ground_idle_1000_rpm",
        skip_serializing
    )]
    legacy_landing_ground_idle_1000_rpm: bool,
    /// Legacy v2 root keys (deserialized from older saves; not written back).
    #[serde(
        default = "default_ground_elevation_ft",
        rename = "ground_elevation_ft",
        skip_serializing
    )]
    legacy_ground_elevation_ft: f64,
    #[serde(
        default = "default_ground_oat_c",
        rename = "ground_oat_c",
        skip_serializing
    )]
    legacy_ground_oat_c: f64,
}

impl Default for FlightPrefs {
    fn default() -> Self {
        Self {
            propeller: PropellerKind::default(),
            wheel_fairings_installed: default_wheel_fairings_installed(),
            takeoff: TakeoffTabPrefs::default(),
            climb: ClimbTabPrefs::default(),
            landing: LandingTabPrefs::default(),
            cruise: CruiseTabPrefs::default(),
            endurance: EnduranceTabPrefs::default(),
            legacy_cruise_oat_manual: false,
            legacy_cruise_solve_saved: None,
            legacy_cruise_percent_bhp_saved: None,
            legacy_cruise_oat_c_saved: None,
            legacy_landing_ground_idle_1000_rpm: default_landing_ground_idle_1000_rpm(),
            legacy_ground_elevation_ft: default_ground_elevation_ft(),
            legacy_ground_oat_c: default_ground_oat_c(),
        }
    }
}

fn default_landing_ground_idle_1000_rpm() -> bool {
    true
}

impl FlightPrefs {
    pub(crate) fn finish_migration(&mut self) {
        self.migrate_legacy_ground_fields();
        self.migrate_legacy_root_tab_fields();
        self.seed_endurance_from_cruise_if_needed();
    }

    fn migrate_legacy_root_tab_fields(&mut self) {
        if self.legacy_cruise_oat_manual
            || self.legacy_cruise_solve_saved.is_some()
            || self.legacy_cruise_percent_bhp_saved.is_some()
            || self.legacy_cruise_oat_c_saved.is_some()
        {
            self.cruise.cruise_oat_manual = self.legacy_cruise_oat_manual;
            if let Some(m) = self.legacy_cruise_solve_saved {
                self.cruise.cruise_solve_saved = Some(m);
            }
            if let Some(p) = self.legacy_cruise_percent_bhp_saved {
                self.cruise.cruise_percent_bhp_saved = Some(p);
            }
            if let Some(o) = self.legacy_cruise_oat_c_saved {
                self.cruise.cruise_oat_c_saved = Some(o);
            }
        }
        if !self.legacy_landing_ground_idle_1000_rpm {
            self.landing.landing_ground_idle_1000_rpm = false;
        }
    }

    fn seed_endurance_from_cruise_if_needed(&mut self) {
        if self.endurance.cruise == CruiseTabPrefs::default() {
            self.endurance.cruise = self.cruise.clone();
        }
    }

    /// Split pre-tab-isolation `ground_*` root fields into per-tab prefs (one-time per load).
    pub(crate) fn migrate_legacy_ground_fields(&mut self) {
        let legacy_elev = self.legacy_ground_elevation_ft;
        let legacy_oat = self.legacy_ground_oat_c;
        let takeoff_pristine = self.takeoff == TakeoffTabPrefs::default();
        let cruise_ground_pristine = self.cruise.ground_elevation_ft
            == default_ground_elevation_ft()
            && self.cruise.ground_oat_c == default_ground_oat_c();
        let legacy_differs =
            legacy_elev != default_ground_elevation_ft() || legacy_oat != default_ground_oat_c();
        if takeoff_pristine && legacy_differs {
            self.takeoff.pressure_altitude_ft = legacy_elev;
            self.takeoff.oat_c = legacy_oat;
        }
        if cruise_ground_pristine && legacy_differs {
            self.cruise.ground_elevation_ft = legacy_elev;
            self.cruise.ground_oat_c = legacy_oat;
        }
        if self.climb == ClimbTabPrefs::default() && takeoff_pristine && legacy_differs {
            self.climb.pressure_altitude_ft = legacy_elev;
            self.climb.oat_c = legacy_oat;
        }
        if self.landing == LandingTabPrefs::default() && takeoff_pristine && legacy_differs {
            self.landing.pressure_altitude_ft = legacy_elev;
            self.landing.oat_c = legacy_oat;
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

pub fn apply_estimated_cruise_oat(tab: &CruiseTabPrefs, inputs: &mut FlightInputs) {
    let est = estimated_oat_c_from_ground(
        tab.ground_elevation_ft,
        tab.ground_oat_c,
        inputs.cruise_pressure_alt_ft,
    );
    inputs.cruise_oat_c = clamp_cruise_oat_for_propeller(inputs, est);
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
            if let Ok(mut p) = LocalStorage::get::<Self>(FLIGHT_PREFS_STORAGE_KEY) {
                p.finish_migration();
                return p;
            }
            if let Ok(legacy) = LocalStorage::get::<FlightPrefsV1>(FLIGHT_PREFS_STORAGE_KEY_V1) {
                let mut upgraded = upgrade_from_v1(legacy);
                upgraded.finish_migration();
                upgraded.save();
                return upgraded;
            }
            Self::default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        if cfg!(target_arch = "wasm32") {
            let _ = LocalStorage::set(FLIGHT_PREFS_STORAGE_KEY, self.clone());
        }
    }

    pub fn apply_global_fields(&self, inputs: &mut FlightInputs) {
        inputs.propeller = self.propeller;
        inputs.wheel_fairings_installed = self.wheel_fairings_installed;
    }
}

pub fn hydrate_cruise_tab_fields(
    tab: &CruiseTabPrefs,
    data: &PerformanceData,
    global: &FlightPrefs,
    inputs: &mut FlightInputs,
) {
    inputs.cruise_pressure_alt_ft = tab.cruise_pressure_alt_ft;
    inputs.cruise_rpm = tab.cruise_rpm;
    inputs.weight_lb = tab.weight_lb;
    inputs.cruise_solve_mode = tab
        .cruise_solve_saved
        .unwrap_or(CruiseSolveMode::SetPercentBhp);
    inputs.cruise_percent_bhp = tab
        .cruise_percent_bhp_saved
        .unwrap_or_else(|| resolved_default_cruise_percent_bhp(data));
    if tab.cruise_oat_manual {
        if let Some(oat) = tab.cruise_oat_c_saved {
            inputs.cruise_oat_c = clamp_cruise_oat_for_propeller(inputs, oat);
        }
    } else {
        apply_estimated_cruise_oat(tab, inputs);
    }
    inputs.propeller = global.propeller;
    inputs.wheel_fairings_installed = global.wheel_fairings_installed;
}

pub fn persist_cruise_tab_from_inputs(tab: &mut CruiseTabPrefs, inputs: &FlightInputs) {
    tab.cruise_pressure_alt_ft = inputs.cruise_pressure_alt_ft;
    tab.cruise_rpm = inputs.cruise_rpm;
    tab.weight_lb = inputs.weight_lb;
    if tab.cruise_oat_manual {
        tab.cruise_oat_c_saved = Some(inputs.cruise_oat_c);
    }
    tab.cruise_solve_saved = Some(inputs.cruise_solve_mode);
    if inputs.cruise_solve_mode == CruiseSolveMode::SetPercentBhp {
        tab.cruise_percent_bhp_saved = Some(inputs.cruise_percent_bhp);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FlightPrefsV1 {
    #[serde(default)]
    propeller: PropellerKind,
    #[serde(default = "default_wheel_fairings_installed")]
    wheel_fairings_installed: bool,
}

pub fn upgrade_from_v1(v1: FlightPrefsV1) -> FlightPrefs {
    FlightPrefs {
        propeller: v1.propeller,
        wheel_fairings_installed: v1.wheel_fairings_installed,
        ..FlightPrefs::default()
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn upgrade_flight_prefs_from_v1_json(json: &str) -> Result<FlightPrefs, serde_json::Error> {
    let v1: FlightPrefsV1 = serde_json::from_str(json)?;
    Ok(upgrade_from_v1(v1))
}

pub fn persist_manual_cruise_oat(tab: &mut CruiseTabPrefs, inputs: &FlightInputs) {
    if tab.cruise_oat_manual {
        tab.cruise_oat_c_saved = Some(inputs.cruise_oat_c);
    }
}

pub fn snapshot_and_save_flight_prefs(inputs: &FlightInputs, prefs: &mut FlightPrefs) {
    prefs.propeller = inputs.propeller;
    prefs.wheel_fairings_installed = inputs.wheel_fairings_installed;
    prefs.save();
}

pub fn persist_takeoff_tab_from_inputs(prefs: &mut FlightPrefs, inputs: &FlightInputs) {
    prefs.takeoff = TakeoffTabPrefs {
        pressure_altitude_ft: inputs.pressure_altitude_input_ft,
        oat_c: inputs.oat_c,
        weight_lb: inputs.weight_lb,
        runway: inputs.runway.clone(),
        wind_dir_deg: inputs.wind_dir_deg,
        wind_speed_kt: inputs.wind_speed_kt,
        obstacle_height_m: inputs.obstacle_height_m,
    };
    prefs.save();
}

pub fn persist_climb_tab_from_inputs(prefs: &mut FlightPrefs, inputs: &FlightInputs) {
    prefs.climb.pressure_altitude_ft = inputs.pressure_altitude_input_ft;
    prefs.climb.oat_c = inputs.oat_c;
    prefs.climb.weight_lb = inputs.weight_lb;
    prefs.climb.climb_figure = inputs.climb_figure;
    prefs.climb.pa_override = inputs.pa_override.filter(|o| o.phase == ChartPhase::Climb);
    prefs.climb.climb_target_altitude_ft = inputs.cruise_pressure_alt_ft;
    prefs.save();
}

pub fn persist_landing_tab_from_inputs(prefs: &mut FlightPrefs, inputs: &FlightInputs) {
    prefs.landing.pressure_altitude_ft = inputs.pressure_altitude_input_ft;
    prefs.landing.oat_c = inputs.oat_c;
    prefs.landing.pa_override = inputs
        .pa_override
        .filter(|o| o.phase == ChartPhase::Landing);
    prefs.landing.landing_ground_idle_1000_rpm = inputs.landing_ground_idle_1000_rpm;
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
        let mut prefs = FlightPrefs {
            propeller: PropellerKind::Mt175R,
            wheel_fairings_installed: false,
            ..FlightPrefs::default()
        };
        prefs.cruise.cruise_oat_manual = true;
        prefs.cruise.cruise_solve_saved = Some(CruiseSolveMode::SetRpm);
        prefs.cruise.cruise_percent_bhp_saved = Some(70.0);
        prefs.cruise.cruise_oat_c_saved = Some(12.5);
        prefs.takeoff.pressure_altitude_ft = 2000.0;
        prefs.takeoff.oat_c = 25.0;
        prefs.cruise.ground_elevation_ft = 2000.0;
        prefs.cruise.ground_oat_c = 25.0;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.takeoff, prefs.takeoff);
        assert_eq!(back.cruise, prefs.cruise);
        assert_eq!(back.propeller, prefs.propeller);
    }

    #[test]
    fn default_cruise_mode_without_saved_state() {
        let data = PerformanceData::production();
        let prefs = FlightPrefs::default();
        let mut inputs = FlightInputs::default();
        hydrate_cruise_tab_fields(&prefs.cruise, &data, &prefs, &mut inputs);
        assert_eq!(inputs.cruise_solve_mode, CruiseSolveMode::SetPercentBhp);
        let pct = resolved_default_cruise_percent_bhp(&data);
        assert!((inputs.cruise_percent_bhp - pct).abs() < f64::EPSILON);
    }

    #[test]
    fn saved_rpm_mode_persists() {
        let data = PerformanceData::production();
        let mut prefs = FlightPrefs::default();
        prefs.cruise.cruise_solve_saved = Some(CruiseSolveMode::SetRpm);
        prefs.cruise.cruise_percent_bhp_saved = Some(72.0);
        let mut inputs = FlightInputs::default();
        hydrate_cruise_tab_fields(&prefs.cruise, &data, &prefs, &mut inputs);
        assert_eq!(inputs.cruise_solve_mode, CruiseSolveMode::SetRpm);
        assert!((inputs.cruise_percent_bhp - 72.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cruise_ground_estimate_updates_cruise_oat_only() {
        let mut prefs = FlightPrefs::default();
        prefs.cruise.ground_elevation_ft = 2000.0;
        prefs.cruise.ground_oat_c = 25.0;
        prefs.cruise.cruise_oat_manual = false;
        let mut inputs = FlightInputs {
            cruise_pressure_alt_ft: 8000.0,
            ..Default::default()
        };
        apply_estimated_cruise_oat(&prefs.cruise, &mut inputs);
        let raw = estimated_oat_c_from_ground(2000.0, 25.0, inputs.cruise_pressure_alt_ft);
        let expected = clamp_cruise_oat_for_propeller(&inputs, raw);
        assert!((inputs.cruise_oat_c - expected).abs() < f64::EPSILON);
        assert_eq!(prefs.takeoff.oat_c, defaults::OAT_C);
    }

    #[test]
    fn manual_cruise_oat_override_sticks() {
        let mut prefs = FlightPrefs::default();
        prefs.cruise.cruise_oat_manual = true;
        prefs.cruise.cruise_oat_c_saved = Some(5.0);
        let mut inputs = FlightInputs {
            cruise_pressure_alt_ft: defaults::CRUISE_PRESSURE_ALT_FT,
            ..Default::default()
        };
        hydrate_cruise_tab_fields(
            &prefs.cruise,
            &PerformanceData::production(),
            &prefs,
            &mut inputs,
        );
        assert!((inputs.cruise_oat_c - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn v1_prefs_upgrade_keeps_propeller_and_fairings() {
        let json = r#"{"propeller":"Mt175R","wheel_fairings_installed":false}"#;
        let upgraded = upgrade_flight_prefs_from_v1_json(json).expect("v1 json");
        assert_eq!(upgraded.propeller, PropellerKind::Mt175R);
        assert!(!upgraded.wheel_fairings_installed);
        assert!(upgraded.landing.landing_ground_idle_1000_rpm);
        assert!(!upgraded.cruise.cruise_oat_manual);
        assert_eq!(upgraded.cruise.cruise_solve_saved, None);
    }

    #[test]
    fn manual_cruise_oat_saved_round_trips_through_hydrate() {
        let data = PerformanceData::production();
        let mut prefs = FlightPrefs::default();
        prefs.cruise.cruise_oat_manual = true;
        prefs.cruise.cruise_oat_c_saved = Some(-8.5);
        let json = serde_json::to_string(&prefs).expect("serialize");
        let loaded: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        let mut inputs = FlightInputs::default();
        hydrate_cruise_tab_fields(&loaded.cruise, &data, &loaded, &mut inputs);
        assert!((inputs.cruise_oat_c - (-8.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn endurance_cruise_seeded_from_cruise_on_finish_migration() {
        let mut prefs = FlightPrefs::default();
        prefs.cruise.cruise_pressure_alt_ft = 7200.0;
        prefs.cruise.weight_lb = 1700.0;
        prefs.cruise.cruise_percent_bhp_saved = Some(65.0);
        prefs.endurance = EnduranceTabPrefs::default();
        prefs.finish_migration();
        assert_eq!(prefs.endurance.cruise.cruise_pressure_alt_ft, 7200.0);
        assert_eq!(prefs.endurance.cruise.weight_lb, 1700.0);
        assert_eq!(prefs.endurance.cruise.cruise_percent_bhp_saved, Some(65.0));
    }

    #[test]
    fn endurance_cruise_not_overwritten_when_already_customized() {
        let mut prefs = FlightPrefs::default();
        prefs.cruise.cruise_pressure_alt_ft = 7200.0;
        prefs.endurance.cruise.cruise_pressure_alt_ft = 8100.0;
        prefs.finish_migration();
        assert_eq!(prefs.endurance.cruise.cruise_pressure_alt_ft, 8100.0);
    }

    #[test]
    fn legacy_v2_ground_keys_migrate_into_tabs() {
        let json = r#"{"ground_elevation_ft":1500.0,"ground_oat_c":18.0}"#;
        let mut prefs: FlightPrefs = serde_json::from_str(json).expect("legacy v2");
        prefs.finish_migration();
        assert_eq!(prefs.takeoff.pressure_altitude_ft, 1500.0);
        assert_eq!(prefs.takeoff.oat_c, 18.0);
        assert_eq!(prefs.cruise.ground_elevation_ft, 1500.0);
        assert_eq!(prefs.cruise.ground_oat_c, 18.0);
    }

    #[test]
    fn default_landing_ground_idle_on_without_saved_state() {
        let data = PerformanceData::production();
        let prefs = FlightPrefs::default();
        let mut inputs = FlightInputs::default();
        crate::phase_state::hydrate_tab_inputs(
            crate::phases::Phase::Landing,
            &prefs,
            &data,
            &mut inputs,
        );
        assert!(inputs.landing_ground_idle_1000_rpm);
        inputs.pressure_altitude_input_ft = 0.0;
        let r = da20_core::landing::compute_landing(&data, &inputs).unwrap();
        assert_eq!(r.ground_roll_ft.round() as i64, 707);
        assert_eq!(r.distance_over_50ft_ft.round() as i64, 1428);
    }

    #[test]
    fn saved_landing_ground_idle_off_persists() {
        let data = PerformanceData::production();
        let mut prefs = FlightPrefs::default();
        prefs.landing.landing_ground_idle_1000_rpm = false;
        let json = serde_json::to_string(&prefs).expect("serialize");
        let back: FlightPrefs = serde_json::from_str(&json).expect("deserialize");
        assert!(!back.landing.landing_ground_idle_1000_rpm);
        let mut inputs = FlightInputs::default();
        crate::phase_state::hydrate_tab_inputs(
            crate::phases::Phase::Landing,
            &back,
            &data,
            &mut inputs,
        );
        assert!(!inputs.landing_ground_idle_1000_rpm);
        let r = da20_core::landing::compute_landing(
            &data,
            &FlightInputs {
                pressure_altitude_input_ft: 0.0,
                landing_ground_idle_1000_rpm: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(r.ground_roll_ft.round() as i64, 661);
        assert_eq!(r.distance_over_50ft_ft.round() as i64, 1360);
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
