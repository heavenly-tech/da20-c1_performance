use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Rev29File {
    #[serde(rename = "_meta")]
    pub meta: Rev29Meta,
    pub worked_examples: serde_json::Value,
    #[serde(rename = "fan_T")]
    pub fan_t: FanField<Vec<f64>>,
    #[serde(rename = "fan_PA")]
    pub fan_pa: FanField<Vec<f64>>,
    #[serde(rename = "fan_EA")]
    pub fan_ea: FanField<Vec<Vec<f64>>>,
    pub roc: RocField,
    #[serde(rename = "to_T")]
    pub to_t: FanField<Vec<f64>>,
    #[serde(rename = "to_PA")]
    pub to_pa: FanField<Vec<f64>>,
    #[serde(rename = "to_base_m")]
    pub to_base_m: FanField<Vec<Vec<Option<f64>>>>,
    #[serde(rename = "wt_kg")]
    pub wt_kg: FanField<Vec<f64>>,
    #[serde(rename = "wt_f")]
    pub wt_f: FanField<Vec<f64>>,
    pub hw_lines: FanField<Vec<f64>>,
    pub hw_kt: FanField<Vec<f64>>,
    pub hw_m: FanField<Vec<Vec<f64>>>,
    pub tw_lines: FanField<Vec<f64>>,
    pub tw_kt: FanField<Vec<f64>>,
    pub tw_m: FanField<Vec<Vec<f64>>>,
    pub obs_lines: FanField<Vec<f64>>,
    pub obs_slope_m_per_m: FanField<Vec<f64>>,
    pub land_alt: FanField<Vec<f64>>,
    pub land_50ft: FanField<Vec<f64>>,
    pub land_roll: FanField<Vec<f64>>,
    pub cruise_table: CruiseTableField,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rev29Meta {
    pub doc: String,
    pub revision: u32,
    pub revision_date: String,
    pub source_url: String,
    pub data_status: Option<String>,
    pub technical_publications_url: Option<String>,
    pub afm_official_url: Option<String>,
    pub amm_official_url: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceMeta {
    pub afm_page: String,
    #[serde(default)]
    pub pdf_page: Option<serde_json::Value>,
    pub figure: String,
    #[serde(default)]
    pub page_revision: Option<String>,
    #[serde(default)]
    pub manual_revision: Option<u32>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FanField<T> {
    pub value: T,
    pub source: SourceMeta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RocField {
    pub value: HashMap<String, [f64; 2]>,
    pub source: SourceMeta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CruiseTableField {
    pub value: CruiseTableValue,
    pub source: SourceMeta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CruiseTableValue {
    pub cols: Vec<String>,
    pub rows: Vec<Vec<f64>>,
}

impl Rev29File {
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn production() -> Self {
        Self::from_json_str(include_str!("../../../../data/da20c1_ch5_data_rev29.json"))
            .expect("embedded Rev 29 AFM Ch.5 data must parse")
    }

    pub fn is_production(&self) -> bool {
        self.meta.data_status.as_deref() == Some("production")
    }
}
