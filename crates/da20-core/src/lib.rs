//! DA20-C1 Eclipse performance core (AFM DA202-C1 Chapter 5).
//!
//! All POH numeric tables are loaded from JSON; do not hard-code AFM values here.

pub mod altitude;
pub mod climb;
pub mod cruise;
pub mod data;
pub mod endurance;
pub mod equivalent_altitude;
pub mod interpolation;
pub mod landing;
pub mod methods;
pub mod poh_harness;
pub mod pressure;
pub mod provenance;
pub mod rev29;
pub mod takeoff;
pub mod types;
pub mod warnings;
pub mod wind;
pub mod wind_format;

pub use data::PerformanceData;
pub use types::*;
