use serde::{Deserialize, Serialize};

/// Default flight-planning inputs for SCTB (Santa Clara County).
pub mod defaults {
    pub const FIELD_ELEVATION_FT: f64 = 2130.0;
    pub const QNH_HPA: f64 = 1013.0;
    pub const OAT_C: f64 = 20.0;
    pub const RUNWAY: &str = "19";
    pub const WIND_DIR_DEG: f64 = 200.0;
    pub const WIND_SPEED_KT: f64 = 8.0;
    pub const WEIGHT_LB: f64 = 1764.0;
    pub const CRUISE_PRESSURE_ALT_FT: f64 = 6500.0;
    pub const CRUISE_RPM: f64 = 2600.0;
    /// AFM Section 1 — usable fuel (US gal).
    pub const FUEL_USABLE_US_GAL: f64 = 23.5;
    pub const FUEL_RESERVE_US_GAL: f64 = 4.5;
}

/// AFM climb chart selection (flap configuration / flight phase).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ClimbFigure {
    /// Figure 5.5 — cruise flaps, normal climb.
    #[default]
    Fig55CruiseFlaps,
    /// Figure 5.6 — takeoff flaps.
    Fig56TakeoffFlaps,
    /// Figure 5.8 — balked landing.
    Fig58BalkedLanding,
}

impl ClimbFigure {
    pub fn roc_key(self) -> &'static str {
        match self {
            Self::Fig55CruiseFlaps => "cruise_5_5",
            Self::Fig56TakeoffFlaps => "takeoff_5_6",
            Self::Fig58BalkedLanding => "balked_5_8",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlightInputs {
    pub field_elevation_ft: f64,
    pub qnh_hpa: f64,
    pub oat_c: f64,
    pub runway: String,
    pub wind_dir_deg: f64,
    pub wind_speed_kt: f64,
    pub weight_lb: f64,
    pub cruise_pressure_alt_ft: f64,
    pub cruise_rpm: f64,
    /// Obstacle height for Fig 5.4 sheet 2 (AFM example uses 5 m / 16 ft).
    #[serde(default = "default_obstacle_m")]
    pub obstacle_height_m: f64,
    /// When set (hand-plot / chart entry), used as PA instead of elev+QNH.
    #[serde(default)]
    pub direct_pressure_altitude_ft: Option<f64>,
    #[serde(default)]
    pub climb_figure: ClimbFigure,
    /// Usable fuel for derived endurance (AFM Sec 1 default).
    #[serde(default = "default_fuel_usable")]
    pub fuel_usable_us_gal: f64,
    #[serde(default = "default_fuel_reserve")]
    pub fuel_reserve_us_gal: f64,
}

fn default_fuel_usable() -> f64 {
    defaults::FUEL_USABLE_US_GAL
}

fn default_fuel_reserve() -> f64 {
    defaults::FUEL_RESERVE_US_GAL
}

fn default_obstacle_m() -> f64 {
    5.0
}

impl Default for FlightInputs {
    fn default() -> Self {
        Self {
            field_elevation_ft: defaults::FIELD_ELEVATION_FT,
            qnh_hpa: defaults::QNH_HPA,
            oat_c: defaults::OAT_C,
            runway: defaults::RUNWAY.to_string(),
            wind_dir_deg: defaults::WIND_DIR_DEG,
            wind_speed_kt: defaults::WIND_SPEED_KT,
            weight_lb: defaults::WEIGHT_LB,
            cruise_pressure_alt_ft: defaults::CRUISE_PRESSURE_ALT_FT,
            cruise_rpm: defaults::CRUISE_RPM,
            obstacle_height_m: 5.0,
            direct_pressure_altitude_ft: None,
            climb_figure: ClimbFigure::default(),
            fuel_usable_us_gal: defaults::FUEL_USABLE_US_GAL,
            fuel_reserve_us_gal: defaults::FUEL_RESERVE_US_GAL,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Citation {
    pub figure_or_table: String,
    pub page: Option<u32>,
    pub afm_revision: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceBlock {
    pub afm_document_number: String,
    pub afm_revision: String,
    pub afm_date: String,
    pub data_status: String,
    pub citation: Citation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalculationWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TakeoffResult {
    pub provenance: ProvenanceBlock,
    pub pressure_altitude_ft: f64,
    pub ground_roll_ft: f64,
    pub distance_over_50ft_ft: f64,
    pub wind_correction_ft: f64,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClimbResult {
    pub provenance: ProvenanceBlock,
    pub equivalent_altitude_ft: f64,
    pub rate_of_climb_fpm: f64,
    pub time_to_altitude_min: Option<f64>,
    pub fuel_to_altitude_us_gal: Option<f64>,
    pub distance_to_altitude_nm: Option<f64>,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CruiseResult {
    pub provenance: ProvenanceBlock,
    pub tas_kt: f64,
    pub fuel_flow_us_gal_h: f64,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LandingResult {
    pub provenance: ProvenanceBlock,
    pub ground_roll_ft: f64,
    pub distance_over_50ft_ft: f64,
    pub wind_correction_ft: f64,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindComponents {
    pub headwind_kt: f64,
    pub crosswind_kt: f64,
    pub along_runway_headwind_kt: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindResult {
    pub provenance: ProvenanceBlock,
    pub components: WindComponents,
    pub takeoff_landing_correction_ft: f64,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnduranceResult {
    pub provenance: ProvenanceBlock,
    pub fuel_flow_us_gal_h: f64,
    pub tas_kt: f64,
    pub endurance_h: f64,
    pub range_nm: f64,
    pub usable_us_gal: f64,
    pub reserve_us_gal: f64,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedReading {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub phase: String,
    pub created_at_iso: String,
    pub inputs: FlightInputs,
    pub takeoff: Option<TakeoffResult>,
    pub climb: Option<ClimbResult>,
    pub cruise: Option<CruiseResult>,
    pub landing: Option<LandingResult>,
    pub wind: Option<WindResult>,
    #[serde(default)]
    pub endurance: Option<EnduranceResult>,
}
