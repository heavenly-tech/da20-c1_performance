//! Built-in methods manual entries (shown in UI).

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
            afm_reference: "AFM performance section — field elevation & altimeter setting",
            summary: "PA (ft) = field elevation + (1013.25 − QNH_hPa) × 30. Used as the vertical axis for takeoff/landing chart entry.",
            interpolation: "N/A — direct formula from inputs.",
            limitations: "Uses standard approximate hPa→ft conversion; verify against AFM worked examples when data arrives.",
        },
        MethodEntry {
            id: "density_altitude",
            title: "Density altitude",
            afm_reference: "Planning aid (not a separate AFM chart in Ch.5)",
            summary: "DA (ft) = PA + 120 × (OAT − ISA_temperature_at_PA).",
            interpolation: "N/A",
            limitations: "Displayed for situational awareness; climb/cruise lookups use OAT and PA per AFM charts.",
        },
        MethodEntry {
            id: "equivalent_altitude",
            title: "Equivalent altitude (Fig 5.2)",
            afm_reference: "DA202-C1 AFM Figure 5.2",
            summary: "Bilinear interpolation on digitized PA × OAT grid to obtain equivalent altitude for climb chart entry.",
            interpolation: "Bilinear with clamping at grid edges; extrapolation disabled by default.",
            limitations: "Requires complete digitized Fig 5.2 grid from approved data file.",
        },
        MethodEntry {
            id: "takeoff",
            title: "Takeoff distance (Fig 5.4)",
            afm_reference: "DA202-C1 AFM Figure 5.4",
            summary: "Select chart at PA (interpolate between adjacent pressure-altitude curves if present). Enter at OAT for ground roll and 50 ft distance; add wind correction from Fig 5.3 model.",
            interpolation: "1D on temperature per chart; PA chart selection via clamped index interpolation.",
            limitations: "Weight must match chart note (1764 lb in standard data); corrections for other weights await AFM data.",
        },
        MethodEntry {
            id: "climb",
            title: "Climb performance (Fig 5.5 / 5.6 / 5.8)",
            afm_reference: "DA202-C1 AFM Figures 5.5, 5.6, 5.8 via Fig 5.2",
            summary: "Compute equivalent altitude from Fig 5.2 at departure PA/OAT; read ROC from 5.5; time/fuel/distance from 5.8 to target equivalent altitude.",
            interpolation: "1D on equivalent altitude axes with clamping.",
            limitations: "Segment availability depends on digitized curves in JSON data file.",
        },
        MethodEntry {
            id: "cruise",
            title: "Cruise performance (Table 3)",
            afm_reference: "DA202-C1 AFM Table 3",
            summary: "Lookup TAS and fuel flow by pressure altitude, RPM, and OAT.",
            interpolation: "Bilinear on PA × OAT per RPM column, then RPM interpolation when multiple RPM columns exist.",
            limitations: "Table structure must match AFM layout in production data file.",
        },
        MethodEntry {
            id: "landing",
            title: "Landing distance (Table 4)",
            afm_reference: "DA202-C1 AFM Table 4",
            summary: "Bilinear interpolation on PA × OAT for ground roll and 50 ft distance; wind correction from Fig 5.3 model.",
            interpolation: "Bilinear with clamping.",
            limitations: "Dry, paved, zero slope assumptions per AFM; see limitations in approved AFM.",
        },
        MethodEntry {
            id: "wind",
            title: "Wind components & correction (Fig 5.3)",
            afm_reference: "DA202-C1 AFM Figure 5.3",
            summary: "Resolve wind into headwind/crosswind along runway heading; apply headwind component to distance correction grid from data file.",
            interpolation: "1D clamped on headwind component axis.",
            limitations: "Runway heading derived from runway number × 10°; verify with actual magnetic direction for operation.",
        },
    ]
}
