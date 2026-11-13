use serde::{Deserialize, Serialize};

/// How a numeric result relates to the published AFM grid (Table 3 nodes, chart ticks, etc.).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AfmLookupTag {
    /// Published grid point (exact PA × RPM × temp column, or exact table altitude, etc.).
    Table,
    /// Between published points (interpolation within chart/table range).
    #[default]
    Interpolated,
}

/// Default flight-planning inputs for SCTB (Santa Clara County).
pub mod defaults {
    pub const FIELD_ELEVATION_FT: f64 = 2130.0;
    pub const QNH_HPA: f64 = 1013.0;
    pub const OAT_C: f64 = 20.0;
    pub const RUNWAY: &str = "190";
    pub const WIND_DIR_DEG: f64 = 200.0;
    pub const WIND_SPEED_KT: f64 = 8.0;
    pub const WEIGHT_LB: f64 = 1764.0;
    pub const CRUISE_PRESSURE_ALT_FT: f64 = 6500.0;
    pub const CRUISE_RPM: f64 = 2600.0;
    /// Fig 5.4 weight panel reference (AFM worked example, kg).
    pub const TAKEOFF_REF_WEIGHT_KG: f64 = 725.0;
    /// Fig 5.4 sheet 1 weight panel chart datum (kg), where wt_f = 1.0.
    pub const TAKEOFF_CHART_WEIGHT_KG: f64 = 800.0;
    /// Obstacle panel x-axis (AFM Fig 5.4 sheet 2, p. 5-10).
    pub const OBSTACLE_HEIGHT_MAX_M: f64 = 15.0;
    pub const OBSTACLE_HEIGHT_DEFAULT_M: f64 = 15.0;
    pub const OBSTACLE_PRESET_AFM_EXAMPLE_M: f64 = 5.0;
    /// AFM §1.8 — total / usable / unusable (US gal), standard tanks.
    pub const FUEL_TOTAL_US_GAL: f64 = 24.5;
    pub const FUEL_USABLE_US_GAL: f64 = 24.0;
    pub const FUEL_UNUSABLE_US_GAL: f64 = 0.5;
    /// Placard alternate configurations (usable US gal).
    pub const FUEL_USABLE_PLACARD_A_US_GAL: f64 = 21.3;
    pub const FUEL_USABLE_PLACARD_B_US_GAL: f64 = 19.8;
    pub const FUEL_RESERVE_US_GAL: f64 = 4.5;
}

/// Chart phase for scoped pressure-altitude override from hand plots.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChartPhase {
    Takeoff,
    Climb,
    Landing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PaOverride {
    pub pa_ft: f64,
    pub phase: ChartPhase,
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

/// Table 3 cruise solve mode: hold RPM or hold %BHP and solve RPM inversely.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CruiseSolveMode {
    #[default]
    SetRpm,
    SetPercentBhp,
}

/// Installed propeller / cruise data source (AFM Table 3 vs MT AFMS Fig 4).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PropellerKind {
    #[default]
    Sensenich,
    Mt175R,
}

/// Performance penalty multipliers when optional wheel fairings are not installed (AFM note).
pub fn climb_roc_fairing_multiplier(installed: bool) -> f64 {
    if installed {
        1.0
    } else {
        0.97
    }
}

/// TAS multiplier when optional wheel fairings are not installed (AFM: ~5% lower maximum cruising speed).
/// Fuel flow from Table 3 is not multiplied by this (penalty applies to speed, not the tabulated GPH).
pub fn cruise_fairing_multiplier(installed: bool) -> f64 {
    if installed {
        1.0
    } else {
        0.95
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlightInputs {
    /// Chart pressure altitude (ft) at **standard pressure** (1013.25 hPa) — no QNH correction.
    #[serde(alias = "field_elevation_ft")]
    pub pressure_altitude_input_ft: f64,
    pub oat_c: f64,
    pub runway: String,
    pub wind_dir_deg: f64,
    pub wind_speed_kt: f64,
    pub weight_lb: f64,
    pub cruise_pressure_alt_ft: f64,
    pub cruise_rpm: f64,
    #[serde(default)]
    pub cruise_solve_mode: CruiseSolveMode,
    /// Target % BHP when `cruise_solve_mode` is `SetPercentBhp`.
    #[serde(default = "default_cruise_percent_bhp")]
    pub cruise_percent_bhp: f64,
    /// OAT (°C) at cruise pressure altitude for Table 3 (independent of aerodrome OAT).
    #[serde(default = "default_cruise_oat_c")]
    pub cruise_oat_c: f64,
    /// Obstacle height for Fig 5.4 sheet 2 (AFM example uses 5 m / 16 ft).
    #[serde(default = "default_obstacle_m")]
    pub obstacle_height_m: f64,
    /// Hand-plot PA override, scoped to one chart phase.
    #[serde(default)]
    pub pa_override: Option<PaOverride>,
    /// Legacy field (deserialized only); migrated into `pa_override` on load.
    #[serde(default, skip_serializing)]
    pub direct_pressure_altitude_ft: Option<f64>,
    #[serde(default)]
    pub climb_figure: ClimbFigure,
    /// Standard wheel fairings installed (AFM baseline performance).
    #[serde(default = "default_wheel_fairings_installed")]
    pub wheel_fairings_installed: bool,
    /// Usable fuel for derived endurance (AFM Sec 1 default).
    #[serde(default = "default_fuel_usable")]
    pub fuel_usable_us_gal: f64,
    #[serde(default = "default_fuel_reserve")]
    pub fuel_reserve_us_gal: f64,
    /// UI-only: set when strip wind drag hits pure-crosswind clamp (not persisted).
    #[serde(default, skip_serializing)]
    pub wind_strip_crosswind_clamped: bool,
    #[serde(default)]
    pub propeller: PropellerKind,
}

fn default_fuel_usable() -> f64 {
    defaults::FUEL_USABLE_US_GAL
}

fn default_fuel_reserve() -> f64 {
    defaults::FUEL_RESERVE_US_GAL
}

fn default_obstacle_m() -> f64 {
    defaults::OBSTACLE_HEIGHT_DEFAULT_M
}

fn default_cruise_oat_c() -> f64 {
    crate::cruise::isa_temp_c_at_pa(defaults::CRUISE_PRESSURE_ALT_FT)
}

fn default_cruise_percent_bhp() -> f64 {
    56.25
}

fn default_wheel_fairings_installed() -> bool {
    true
}

impl Default for FlightInputs {
    fn default() -> Self {
        Self {
            pressure_altitude_input_ft: defaults::FIELD_ELEVATION_FT,
            oat_c: defaults::OAT_C,
            runway: defaults::RUNWAY.to_string(),
            wind_dir_deg: defaults::WIND_DIR_DEG,
            wind_speed_kt: defaults::WIND_SPEED_KT,
            weight_lb: defaults::WEIGHT_LB,
            cruise_pressure_alt_ft: defaults::CRUISE_PRESSURE_ALT_FT,
            cruise_rpm: defaults::CRUISE_RPM,
            cruise_solve_mode: CruiseSolveMode::default(),
            cruise_percent_bhp: default_cruise_percent_bhp(),
            cruise_oat_c: default_cruise_oat_c(),
            obstacle_height_m: defaults::OBSTACLE_HEIGHT_DEFAULT_M,
            pa_override: None,
            direct_pressure_altitude_ft: None,
            climb_figure: ClimbFigure::default(),
            wheel_fairings_installed: default_wheel_fairings_installed(),
            fuel_usable_us_gal: defaults::FUEL_USABLE_US_GAL,
            fuel_reserve_us_gal: defaults::FUEL_RESERVE_US_GAL,
            wind_strip_crosswind_clamped: false,
            propeller: PropellerKind::default(),
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
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub message: String,
}

impl CalculationWarning {
    pub fn code_only(code: &str) -> Self {
        Self {
            code: code.into(),
            args: vec![],
            message: String::new(),
        }
    }

    pub fn with_args(code: &str, args: Vec<String>) -> Self {
        Self {
            code: code.into(),
            args,
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TakeoffResult {
    pub provenance: ProvenanceBlock,
    #[serde(default)]
    pub afm_lookup_tag: AfmLookupTag,
    pub pressure_altitude_ft: f64,
    pub ground_roll_ft: f64,
    pub distance_over_obstacle_raw_m: f64,
    pub distance_over_obstacle_raw_ft: f64,
    pub distance_over_obstacle_conservative_m: f64,
    pub distance_over_obstacle_conservative_ft: f64,
    pub distance_over_50ft_ft: f64,
    pub wind_correction_ft: f64,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClimbResult {
    pub provenance: ProvenanceBlock,
    #[serde(default)]
    pub afm_lookup_tag: AfmLookupTag,
    pub equivalent_altitude_ft: f64,
    /// AFM chart ROC before optional wheel-fairing penalty (`rate_of_climb_fpm` includes that penalty).
    /// Omitted in v3 saved readouts; use `None` and show only `rate_of_climb_fpm` with the −3% note.
    #[serde(default)]
    pub rate_of_climb_chart_fpm: Option<f64>,
    pub rate_of_climb_fpm: f64,
    pub time_to_altitude_min: Option<f64>,
    pub fuel_to_altitude_us_gal: Option<f64>,
    pub distance_to_altitude_nm: Option<f64>,
    pub warnings: Vec<CalculationWarning>,
}

/// One printed Table 3 cell (ΔISA column) with optional blend weight in a lookup trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table3PublishedCell {
    pub disa_c: f64,
    pub percent_bhp: f64,
    pub tas_kt: f64,
    pub gph: f64,
    #[serde(default)]
    pub disa_weight: f64,
    #[serde(default)]
    pub disa_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table3RpmBracketRow {
    pub pa_ft: f64,
    pub rpm: f64,
    pub rpm_weight: f64,
    pub cells: Vec<Table3PublishedCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table3PaBracketRow {
    pub pa_ft: f64,
    pub rpm_sample: f64,
    pub pa_weight: f64,
    pub rpm_rows: Vec<Table3RpmBracketRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Table3SourceTrace {
    pub query_pa_ft: f64,
    pub query_rpm: f64,
    pub query_disa_c: f64,
    pub pa_rows: Vec<Table3PaBracketRow>,
}

/// One Fig 4 cell with blend weight in an MT lookup trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MtFig4PublishedCell {
    pub percent_bhp: f64,
    pub tas_kt: f64,
    pub gph: f64,
    #[serde(default)]
    pub weight: f64,
    #[serde(default)]
    pub used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MtFig4RpmBracketRow {
    pub da_ft: f64,
    pub rpm: f64,
    pub rpm_weight: f64,
    pub cells: Vec<MtFig4PublishedCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MtFig4DaBracketRow {
    pub da_ft: f64,
    pub rpm_sample: f64,
    pub da_weight: f64,
    pub rpm_rows: Vec<MtFig4RpmBracketRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MtFig4SourceTrace {
    pub query_pa_ft: f64,
    pub query_da_ft: f64,
    pub query_rpm: f64,
    pub da_rows: Vec<MtFig4DaBracketRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MtCurveHighlight {
    pub pct_bhp: i32,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MtFig2SourceTrace {
    pub query_da_ft: f64,
    pub query_pct_bhp: Option<f64>,
    pub query_rpm: Option<f64>,
    pub rpm_result: f64,
    pub curve_brackets: Vec<MtCurveHighlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MtFig3SourceTrace {
    pub query_da_ft: f64,
    pub query_pct_bhp: f64,
    pub tas_result_kt: f64,
    pub curve_brackets: Vec<MtCurveHighlight>,
    #[serde(default)]
    pub capped_at_full_throttle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table4PaBracketRow {
    pub pa_ft: f64,
    pub pa_weight: f64,
    pub ground_roll_ft: f64,
    pub distance_over_50ft_ft: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Table4SourceTrace {
    pub query_pa_ft: f64,
    pub pa_rows: Vec<Table4PaBracketRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CruiseResult {
    pub provenance: ProvenanceBlock,
    #[serde(default)]
    pub afm_lookup_tag: AfmLookupTag,
    /// Tabulated/solved RPM used for this Table 3 lookup.
    #[serde(default)]
    pub rpm: Option<f64>,
    /// % BHP from Table 3 (chart value; not scaled by wheel-fairing TAS note).
    #[serde(default)]
    pub percent_bhp: Option<f64>,
    /// Table 3 TAS (kt) before optional fairing penalty (`tas_kt` includes penalty when fairings off).
    #[serde(default)]
    pub tas_chart_kt: Option<f64>,
    pub tas_kt: f64,
    /// Table 3 fuel flow (US gal/h); not scaled by the fairing note (see `cruise_fairing_multiplier`).
    #[serde(default)]
    pub fuel_flow_chart_us_gal_h: Option<f64>,
    pub fuel_flow_us_gal_h: f64,
    #[serde(default)]
    pub table3_source: Option<Table3SourceTrace>,
    #[serde(default)]
    pub mt_fig4_source: Option<MtFig4SourceTrace>,
    #[serde(default)]
    pub mt_fig2_source: Option<MtFig2SourceTrace>,
    #[serde(default)]
    pub mt_fig3_source: Option<MtFig3SourceTrace>,
    /// Fig 4 KTAS from the table for MT comparison only (never blended into `tas_*`).
    #[serde(default)]
    pub mt_fig4_ktas_kt: Option<f64>,
    /// ISA troposphere density altitude used for MT Fig 4 (Sensenich cruise leaves this unset).
    #[serde(default)]
    pub density_altitude_ft: Option<f64>,
    /// Fig 4 GPH unavailable (DA below 2000 ft or RPM below 2200) while Fig 2/3 remain valid.
    #[serde(default)]
    pub mt_gph_out_of_chart: bool,
    pub warnings: Vec<CalculationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LandingResult {
    pub provenance: ProvenanceBlock,
    #[serde(default)]
    pub afm_lookup_tag: AfmLookupTag,
    pub ground_roll_ft: f64,
    pub distance_over_50ft_ft: f64,
    pub wind_correction_ft: f64,
    #[serde(default)]
    pub table4_source: Option<Table4SourceTrace>,
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
    #[serde(default)]
    pub afm_lookup_tag: AfmLookupTag,
    pub fuel_flow_us_gal_h: f64,
    pub tas_kt: f64,
    /// Table 3 TAS (kt) before fairing penalty, when applicable.
    #[serde(default)]
    pub tas_chart_kt: Option<f64>,
    pub endurance_h: f64,
    pub range_nm: f64,
    /// Range at Table 3 TAS (NM); `range_nm` uses penalized TAS when fairings off.
    #[serde(default)]
    pub range_chart_nm: Option<f64>,
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
