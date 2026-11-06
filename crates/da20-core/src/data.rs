use serde::Deserialize;

use crate::rev29::types::Rev29File;
use crate::types::Citation;

/// Runtime performance dataset (AFM DA202-C1 Rev 29 Chapter 5 JSON).
#[derive(Debug, Clone, Deserialize)]
pub struct PerformanceData {
    #[serde(flatten)]
    inner: Rev29File,
}

impl PerformanceData {
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        Ok(Self {
            inner: Rev29File::from_json_str(s)?,
        })
    }

    pub fn production() -> Self {
        Self {
            inner: Rev29File::production(),
        }
    }

    pub fn synthetic_fixture() -> Self {
        Self::production()
    }

    pub fn inner(&self) -> &Rev29File {
        &self.inner
    }

    pub fn meta(&self) -> &crate::rev29::types::Rev29Meta {
        &self.inner.meta
    }

    pub fn is_synthetic(&self) -> bool {
        false
    }

    pub fn takeoff_source(&self) -> &crate::rev29::types::SourceMeta {
        &self.inner.to_base_m.source
    }

    pub fn climb_roc_source(&self) -> &crate::rev29::types::SourceMeta {
        &self.inner.roc.source
    }

    pub fn cruise_source(&self) -> &crate::rev29::types::SourceMeta {
        &self.inner.cruise_table.source
    }

    pub fn landing_source(&self) -> &crate::rev29::types::SourceMeta {
        &self.inner.land_roll.source
    }

    pub fn wind_source(&self) -> &crate::rev29::types::SourceMeta {
        &self.inner.hw_m.source
    }

    pub fn equivalent_altitude_source(&self) -> &crate::rev29::types::SourceMeta {
        &self.inner.fan_ea.source
    }

    pub fn takeoff_citation(&self) -> Citation {
        crate::provenance::citation_from_source(&self.inner.to_base_m.source)
    }
}
