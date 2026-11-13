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
            summary: "Interpolation on digitized PA × OAT grid (fan_PA, fan_T, fan_EA).",
            interpolation: "Linear on OAT per PA row, then linear on PA; out of chart outside PA 0–16000 ft or OAT −20..60 °C.",
            limitations: "Requires complete digitized Fig 5.2 grid from Rev 29 JSON.",
        },
        MethodEntry {
            id: "takeoff",
            title: "Takeoff distance (Fig 5.4)",
            afm_reference: "AFM pages 5-9 / 5-10, Figure 5.4",
            summary: "Chain: base ground roll (OAT×PA) → weight factor → Fig 5.4 wind panel (HW/TW) → obstacle slope from sheet 2. Distances in meters, displayed in feet.",
            interpolation: "Linear on each panel only; out of chart outside PA 0–10000 ft, OAT −20..55 °C, weight 600–800 kg, headwind >20 kt, tailwind >5 kt, or panel entry/obstacle off printed lines.",
            limitations: TAKEOFF_AFM_ROUNDING_NOTE,
        },
        MethodEntry {
            id: "climb",
            title: "Climb ROC (Fig 5.5 / 5.6 / 5.8 via Fig 5.2)",
            afm_reference: "AFM pages 5-11, 5-12, 5-16",
            summary: "ROC (fpm) = a + b × EA_ft from digitized linear fits (roc.*). Planner shows Fig 5.5 (cruise flaps) ROC by default.",
            interpolation: "EA from Fig 5.2 (same hull); ROC coefficients fixed per figure.",
            limitations: CLIMB_DERIVED_ESTIMATE_NOTE,
        },
        MethodEntry {
            id: "cruise_mt175r",
            title: "MT 175 R cruise (AFMS E-1487)",
            afm_reference: "AFMS E-1487 Figures 1–4",
            summary: "PA + OAT → density altitude (ISA troposphere formula; Fig 1 example ≈7374 ft vs 7391 printed NWS). Chain: Fig 2 RPM/%BHP → Fig 3 TAS (max gross, no weight correction; cap at Full Throttle) → Fig 4 GPH only. Fig 4 KTAS comparison only. Wheel fairings ~−5% TAS; GPH not scaled.",
            interpolation: "Tabla or Interpolado tags only. Off chart below 45% BHP, past 75% endpoint, right of Full Throttle/2800 RPM, Fig 2 RPM < 2200 or DA < 2000 ft for GPH/endurance/range.",
            limitations: "Fig 4 GPH transcribed as printed; two doubtful cells at DA 10000/12000 RPM 2200.",
        },
        MethodEntry {
            id: "cruise",
            title: "Cruise performance (Table 3)",
            afm_reference: "AFM pages 5-14 / 5-15, Table 3",
            summary: "ΔISA = OAT − ISA at PA. Interpolation only: rmin(PA) is linear in PA between each row's first tabulated RPM; valid RPM is [rmin(PA), 2800]. Normalize s = (RPM−rmin)/(2800−rmin); on each bracketing row use RPM_row = rmin_row + s(2800−rmin_row), interpolate ΔISA then RPM on that ladder, then blend in PA. Set-%BHP inverts on [rmin(PA), 2800] only. Fairings: ~−5% TAS after lookup; GPH not scaled.",
            interpolation: "Tags: Tabla (printed node) or Interpolado only. Out of chart if PA, ΔISA, or RPM are outside the hull (e.g. 6500 ft: RPM < 2512.5).",
            limitations: CRUISE_TABLE3_GPH_NOTE,
        },
        MethodEntry {
            id: "landing",
            title: "Landing distance (Table 4)",
            afm_reference: "AFM page 5-17, Table 4",
            summary: "Linear interpolation on pressure altitude for ground roll and 50 ft distances (table values in ft). Optional wind panel correction.",
            interpolation: "1D on land_alt axes; out of chart outside PA 0–7000 ft.",
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
