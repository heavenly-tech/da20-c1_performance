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
            summary: "Enter pressure altitude (ft) as for standard pressure (1013.25 hPa); this planner does not apply a QNH correction. Same value drives chart and table axes.",
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
            summary: "ΔISA = OAT − ISA at PA. On each Table 3 PA row, linearly interpolate or extrapolate in RPM along that row's own tabulated RPM ladder after interpolating along ΔISA (−20 / 0 / +10); then blend those values linearly in PA (and ΔISA is already per row). Set-RPM forward mode reads %BHP, KTAS, and GPH from that surface. Set-%BHP inverse mode bisects RPM on the same forward surface (±100 RPM slack beyond each bracketing row's ladder); requested %BHP outside that envelope is not achievable. Wheel fairing note: apply ~5% lower TAS after Table 3 lookup; GPH is not scaled.",
            interpolation: "Tags: Table on exact printed PA/RPM/ΔISA nodes; Interpolated inside the grid; Extrapolated when RPM, PA, or ΔISA is outside tabulated limits (inverse hits off-chart when the solved RPM needs extrapolation).",
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
            limitations: "Runway heading: three-digit magnetic 001–360 only (no designator shortcuts).",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cruise_method_manual_no_exact_row_or_oat_band_claims() {
        let cruise = all_methods()
            .into_iter()
            .find(|e| e.id == "cruise")
            .expect("cruise method entry");
        let joined = format!(
            "{} {} {}",
            cruise.summary, cruise.interpolation, cruise.limitations
        );
        assert!(
            !joined.contains("Exact row match"),
            "manual must not claim exact row match for Table 3"
        );
        assert!(
            !joined.contains("OAT band"),
            "manual must not describe band-snapping OAT selection"
        );
        assert!(
            joined.contains("ΔISA") || joined.contains("dISA"),
            "manual should describe ΔISA"
        );
    }
}
