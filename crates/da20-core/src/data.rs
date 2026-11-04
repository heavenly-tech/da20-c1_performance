use serde::{Deserialize, Serialize};

use crate::types::Citation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceData {
    pub meta: Meta,
    pub equivalent_altitude: EquivalentAltitudeSection,
    pub takeoff: TakeoffSection,
    pub climb: ClimbSection,
    pub cruise: CruiseSection,
    pub landing: LandingSection,
    pub wind: WindSection,
    #[serde(default)]
    pub validation: ValidationSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Meta {
    pub afm_document_number: String,
    pub afm_revision: String,
    pub afm_date: String,
    pub data_status: String,
    pub afm_official_url: String,
    pub amm_official_url: String,
    #[serde(default)]
    pub notes: Option<String>,
}

impl Meta {
    pub fn is_synthetic(&self) -> bool {
        self.data_status == "synthetic_fixture"
            || self.afm_document_number.starts_with("SYNTHETIC")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquivalentAltitudeSection {
    pub citation: Citation,
    pub grid: AltitudeGrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AltitudeGrid {
    pub pressure_altitude_ft: Vec<f64>,
    pub rows: Vec<EquivalentAltitudeRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquivalentAltitudeRow {
    pub oat_c: f64,
    pub equivalent_altitude_ft: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TakeoffSection {
    pub citation: Citation,
    pub weight_lb: f64,
    pub charts: Vec<TakeoffChart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TakeoffChart {
    pub pressure_altitude_ft: f64,
    pub temperature_c: Vec<f64>,
    pub ground_roll_ft: Vec<f64>,
    pub distance_50ft_ft: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClimbSection {
    pub citations: ClimbCitations,
    pub segments: Vec<ClimbSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClimbCitations {
    pub figure_5_5: Citation,
    pub figure_5_6: Citation,
    pub figure_5_8: Citation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClimbSegment {
    pub figure: String,
    pub equivalent_altitude_ft: Vec<f64>,
    #[serde(default)]
    pub rate_of_climb_fpm: Vec<f64>,
    #[serde(default)]
    pub time_min: Vec<f64>,
    #[serde(default)]
    pub fuel_us_gal: Vec<f64>,
    #[serde(default)]
    pub distance_nm: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CruiseSection {
    pub citation: Citation,
    pub table: CruiseTable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CruiseTable {
    pub pressure_altitude_ft: Vec<f64>,
    pub rpm: Vec<f64>,
    pub rows: Vec<CruiseRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CruiseRow {
    pub oat_c: f64,
    pub tas_kt: Vec<f64>,
    pub fuel_flow_us_gal_h: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LandingSection {
    pub citation: Citation,
    pub table: LandingTable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LandingTable {
    pub pressure_altitude_ft: Vec<f64>,
    pub rows: Vec<LandingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LandingRow {
    pub oat_c: f64,
    pub ground_roll_ft: Vec<f64>,
    pub distance_50ft_ft: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindSection {
    pub citation: Citation,
    pub correction_model: WindCorrectionModel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindCorrectionModel {
    pub kind: String,
    #[serde(default)]
    pub grid: Option<WindGrid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindGrid {
    pub headwind_component_kt: Vec<f64>,
    pub correction_ft: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ValidationSection {
    #[serde(default)]
    pub tolerances: std::collections::HashMap<String, f64>,
    #[serde(default)]
    pub worked_examples: Vec<WorkedExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkedExample {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub inputs: serde_json::Value,
    pub expected: serde_json::Value,
    #[serde(default)]
    pub tolerance_key: Option<String>,
}

impl PerformanceData {
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn synthetic_fixture() -> Self {
        let s = include_str!("../../../data/fixtures/SYNTHETIC_performance_data.json");
        Self::from_json_str(s).expect("embedded SYNTHETIC fixture must parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_fixture_loads() {
        let d = PerformanceData::synthetic_fixture();
        assert!(d.meta.is_synthetic());
    }
}
