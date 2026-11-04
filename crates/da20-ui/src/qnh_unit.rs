use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

const QNH_UNIT_KEY: &str = "da20_qnh_unit_v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QnhUnit {
    #[default]
    Hpa,
    InHg,
}

impl QnhUnit {
    pub fn load() -> Self {
        LocalStorage::get(QNH_UNIT_KEY).unwrap_or(Self::Hpa)
    }

    pub fn save(self) {
        let _ = LocalStorage::set(QNH_UNIT_KEY, self);
    }

    #[allow(dead_code)]
    pub fn toggle(self) -> Self {
        match self {
            Self::Hpa => Self::InHg,
            Self::InHg => Self::Hpa,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hpa => "hPa",
            Self::InHg => "inHg",
        }
    }
}
