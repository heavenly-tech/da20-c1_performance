//! Built-in methods manual entries (shown in UI).

use crate::climb::CLIMB_DERIVED_ESTIMATE_NOTE;
use crate::cruise::CRUISE_TABLE3_GPH_NOTE;
use crate::landing::LANDING_TABLE_50FT_M_NOTE;
use crate::takeoff::TAKEOFF_AFM_ROUNDING_NOTE;

#[derive(Debug, Clone)]
pub struct MethodEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub afm_reference: &'static str,
    pub summary: &'static str,
    pub interpolation: &'static str,
    pub limitations: &'static str,
}

pub fn all_methods() -> Vec<MethodEntry> {
    vec![
        MethodEntry {
            id: "pressure_altitude",
            title: "Pressure altitude",
            afm_reference: "AFM DA202-C1 Rev 29 — performance inputs",
            summary: "PA (ft) = field elevation + (1013.25 − QNH_hPa) × 30. Used as the vertical axis for chart/table entry.",
            interpolation: "N/A — direct formula from inputs.",
            limitations: "Approximate hPa→ft conversion; verify against AFM examples.",
        },
        MethodEntry {
            id: "equivalent_altitude",
            title: "Equivalent altitude (Fig 5.2)",
            afm_reference: "AFM page 5-6, Figure 5.2 (digitized grid Rev 27, unchanged Rev 29)",
            summary: "Bilinear-style interpolation on digitized PA × OAT grid (fan_PA, fan_T, fan_EA).",
            interpolation: "Linear on temperature per PA row, then linear on PA.",
            limitations: "Requires complete digitized Fig 5.2 grid from Rev 29 JSON.",
        },
        MethodEntry {
            id: "takeoff",
            title: "Takeoff distance (Fig 5.4)",
            afm_reference: "AFM pages 5-9 / 5-10, Figure 5.4",
            summary: "Chain: base ground roll (OAT×PA) → weight factor → Fig 5.4 wind panel (HW/TW) → obstacle slope from sheet 2. Distances in meters, displayed in feet.",
            interpolation: "Linear on each panel axis (see validate_rev29 reference script).",
            limitations: TAKEOFF_AFM_ROUNDING_NOTE,
        },
        MethodEntry {
            id: "climb",
            title: "Climb ROC (Fig 5.5 / 5.6 / 5.8 via Fig 5.2)",
            afm_reference: "AFM pages 5-11, 5-12, 5-16",
            summary: "ROC (fpm) = a + b × EA_ft from digitized linear fits (roc.*). Planner shows Fig 5.5 (cruise flaps) ROC by default.",
            interpolation: "Linear EA from Fig 5.2; ROC coefficients fixed per figure.",
            limitations: CLIMB_DERIVED_ESTIMATE_NOTE,
        },
        MethodEntry {
            id: "cruise",
            title: "Cruise performance (Table 3)",
            afm_reference: "AFM pages 5-14 / 5-15, Table 3",
            summary: "Exact row match on PA and RPM; OAT band selects dISA−20 / ISA / dISA+10 column group.",
            interpolation: "Band selection by OAT−ISA at PA (no cross-band interpolation).",
            limitations: CRUISE_TABLE3_GPH_NOTE,
        },
        MethodEntry {
            id: "landing",
            title: "Landing distance (Table 4)",
            afm_reference: "AFM page 5-17, Table 4",
            summary: "Linear interpolation on pressure altitude for ground roll and 50 ft distances (table values in ft). Optional wind panel correction.",
            interpolation: "1D on land_alt axes.",
            limitations: LANDING_TABLE_50FT_M_NOTE,
        },
        MethodEntry {
            id: "wind",
            title: "Wind components (Fig 5.3)",
            afm_reference: "AFM page 5-8, Figure 5.3",
            summary: "Headwind/crosswind from wind direction, speed, and runway heading (trigonometric). Takeoff/landing distance wind panels use Fig 5.4 sheet 1.",
            interpolation: "N/A for components; panel interpolation for distance corrections.",
            limitations: "Runway heading = runway number × 10° unless overridden.",
        },
    ]
}
