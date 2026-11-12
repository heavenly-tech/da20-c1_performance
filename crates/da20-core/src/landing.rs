use crate::afm_tag::landing_tag_from_pa;
use crate::altitude::effective_pressure_altitude_ft;
use crate::chart_bounds::ChartOutOfRange;
use crate::chart_bounds::{self, limits, CHART_EDGE_TOL};
use crate::data::PerformanceData;
use crate::provenance::provenance_for;
use crate::rev29::engine::landing_lookup_ft;
use crate::rev29::types::Rev29File;
use crate::types::{
    CalculationWarning, FlightInputs, LandingResult, Table4PaBracketRow, Table4SourceTrace,
};

pub const LANDING_TABLE_50FT_M_NOTE: &str =
    "Table 4 lists 1360 ft (≈415 m rounded) at sea level; AFM narrative text cites 414 m for the same condition.";

pub const LANDING_NO_CORRECTIONS_NOTE: &str =
    "Table 4 has no wind, temperature, or weight corrections — values are pressure-altitude lookup only.";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LandingError {
    #[error(transparent)]
    OutOfChart(ChartOutOfRange),
}

pub fn table4_source_trace(
    data: &Rev29File,
    pa: f64,
) -> Result<Table4SourceTrace, ChartOutOfRange> {
    chart_bounds::require_in_range(
        "Table 4 PA",
        limits::TABLE4_PA_FT.0,
        limits::TABLE4_PA_FT.1,
        pa,
        CHART_EDGE_TOL,
    )?;
    let alts = &data.land_alt.value;
    let roll = &data.land_roll.value;
    let over = &data.land_50ft.value;
    if alts.is_empty() {
        return Ok(Table4SourceTrace {
            query_pa_ft: pa,
            pa_rows: vec![],
        });
    }
    if alts.len() == 1 {
        return Ok(Table4SourceTrace {
            query_pa_ft: pa,
            pa_rows: vec![Table4PaBracketRow {
                pa_ft: alts[0],
                pa_weight: 1.0,
                ground_roll_ft: roll[0],
                distance_over_50ft_ft: over[0],
            }],
        });
    }
    const PA_MATCH: f64 = 0.5;
    if let Some(&exact) = alts.iter().find(|a| (**a - pa).abs() < PA_MATCH) {
        let i = alts
            .iter()
            .position(|a| (*a - exact).abs() < 1e-9)
            .unwrap_or(0);
        return Ok(Table4SourceTrace {
            query_pa_ft: pa,
            pa_rows: vec![Table4PaBracketRow {
                pa_ft: exact,
                pa_weight: 1.0,
                ground_roll_ft: roll[i],
                distance_over_50ft_ft: over[i],
            }],
        });
    }
    let mut j = 0usize;
    while j + 1 < alts.len() && alts[j + 1] < pa - PA_MATCH {
        j += 1;
    }
    if j + 1 >= alts.len() {
        return Err(ChartOutOfRange {
            label: "Table 4 PA",
            min: limits::TABLE4_PA_FT.0,
            max: limits::TABLE4_PA_FT.1,
        });
    }
    let p0 = alts[j];
    let p1 = alts[j + 1];
    let t = (pa - p0) / (p1 - p0);
    Ok(Table4SourceTrace {
        query_pa_ft: pa,
        pa_rows: vec![
            Table4PaBracketRow {
                pa_ft: p0,
                pa_weight: 1.0 - t,
                ground_roll_ft: roll[j],
                distance_over_50ft_ft: over[j],
            },
            Table4PaBracketRow {
                pa_ft: p1,
                pa_weight: t,
                ground_roll_ft: roll[j + 1],
                distance_over_50ft_ft: over[j + 1],
            },
        ],
    })
}

pub fn table4_trace_uses_pa(trace: &Table4SourceTrace, pa_ft: f64) -> bool {
    trace.pa_rows.iter().any(|r| (r.pa_ft - pa_ft).abs() < 0.5)
}

pub fn recompute_landing_from_table4_source(trace: &Table4SourceTrace) -> (f64, f64) {
    let mut roll = 0.0;
    let mut over = 0.0;
    for row in &trace.pa_rows {
        roll += row.pa_weight * row.ground_roll_ft;
        over += row.pa_weight * row.distance_over_50ft_ft;
    }
    (roll, over)
}

pub fn compute_landing(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<LandingResult, LandingError> {
    let pa = effective_pressure_altitude_ft(inputs, Some(crate::types::ChartPhase::Landing));
    let (roll_ft, over_50_ft, mut warnings) =
        landing_lookup_ft(data.inner(), pa).map_err(LandingError::OutOfChart)?;
    let table4_source = table4_source_trace(data.inner(), pa).ok();

    warnings.push(CalculationWarning::code_only("LANDING_TABLE_50FT"));
    warnings.push(CalculationWarning::code_only("LANDING_NO_WIND"));
    let afm_lookup_tag = landing_tag_from_pa(pa, &data.inner().land_alt.value);

    Ok(LandingResult {
        provenance: provenance_for(data.inner(), data.landing_source()),
        afm_lookup_tag,
        ground_roll_ft: roll_ft,
        distance_over_50ft_ft: over_50_ft,
        wind_correction_ft: 0.0,
        table4_source,
        warnings,
    })
}

pub fn table_landing_50ft_ft(data: &PerformanceData, pa_ft: f64) -> f64 {
    landing_lookup_ft(data.inner(), pa_ft)
        .map(|t| t.1)
        .unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;

    #[test]
    fn table4_source_recomputes_interpolated_pa() {
        let data = PerformanceData::production();
        let pa = 2130.0;
        let r = compute_landing(
            &data,
            &FlightInputs {
                pressure_altitude_input_ft: pa,
                ..Default::default()
            },
        )
        .unwrap();
        let trace = r.table4_source.as_ref().unwrap();
        assert_eq!(trace.pa_rows.len(), 2);
        let (roll, over) = recompute_landing_from_table4_source(trace);
        assert!((roll - r.ground_roll_ft).abs() < 1e-9);
        assert!((over - r.distance_over_50ft_ft).abs() < 1e-9);
    }

    #[test]
    fn table4_sea_level_exact_lookup() {
        let data = PerformanceData::production();
        assert_eq!(table_landing_50ft_ft(&data, 0.0), 1360.0);
        let m = 1360.0 * 0.3048;
        assert!((m - 414.528_f64).abs() < 0.01);
    }
}
