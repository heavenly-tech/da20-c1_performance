use crate::altitude::isa_troposphere_density_altitude_ft;
use crate::chart_bounds::{limits, ChartOutOfRange, CHART_EDGE_TOL};
use crate::cruise::CruiseError;
use crate::data::PerformanceData;
use crate::interpolation::lerp;
use crate::mt175r::charts::{
    lookup_fig2_pct_at_rpm, lookup_fig2_rpm_at_pct, lookup_fig3_tas_at_pct,
};
use crate::mt175r::data::Mt175rData;
use crate::types::{
    cruise_fairing_multiplier, AfmLookupTag, CalculationWarning, CruiseResult, CruiseSolveMode,
    FlightInputs, MtCurveHighlight, MtFig2SourceTrace, MtFig3SourceTrace, MtFig4DaBracketRow,
    MtFig4PublishedCell, MtFig4RpmBracketRow, MtFig4SourceTrace,
};

const DA_MATCH_TOL: f64 = 0.5;
const RPM_MATCH_TOL: f64 = 0.5;

#[derive(Clone, Copy, Debug)]
struct Triple {
    bhp: f64,
    tas: f64,
    gph: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct LookupFlags {
    rpm_interp: bool,
    da_interp: bool,
}

fn row_valid(r: &[f64]) -> bool {
    r.len() >= 5
}

fn unique_sorted_das(rows: &[Vec<f64>]) -> Vec<f64> {
    let mut v: Vec<f64> = rows.iter().filter(|r| row_valid(r)).map(|r| r[0]).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|a, b| (*a - *b).abs() < DA_MATCH_TOL);
    v
}

fn rows_at_da(rows: &[Vec<f64>], da: f64) -> Vec<&Vec<f64>> {
    let mut v: Vec<_> = rows
        .iter()
        .filter(|r| row_valid(r) && (r[0] - da).abs() < DA_MATCH_TOL)
        .collect();
    v.sort_by(|a, b| a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal));
    v
}

fn lerp_triple(a: Triple, b: Triple, t: f64) -> Triple {
    Triple {
        bhp: lerp(0.0, a.bhp, 1.0, b.bhp, t),
        tas: lerp(0.0, a.tas, 1.0, b.tas, t),
        gph: lerp(0.0, a.gph, 1.0, b.gph, t),
    }
}

fn cell_for_row(row: &[f64]) -> MtFig4PublishedCell {
    MtFig4PublishedCell {
        percent_bhp: row[2],
        tas_kt: row[3],
        gph: row[4],
        weight: 1.0,
        used: true,
    }
}

fn trace_on_da_ladder(ladder: &[&Vec<f64>], rpm: f64) -> Option<Vec<MtFig4RpmBracketRow>> {
    if ladder.is_empty() || !rpm_on_ladder(ladder, rpm) {
        return None;
    }
    if let Some(row) = ladder.iter().find(|r| (r[1] - rpm).abs() < RPM_MATCH_TOL) {
        return Some(vec![MtFig4RpmBracketRow {
            da_ft: row[0],
            rpm: row[1],
            rpm_weight: 1.0,
            cells: vec![cell_for_row(row)],
        }]);
    }
    let mut below_idx = None;
    let mut above_idx = None;
    for (i, r) in ladder.iter().enumerate() {
        if r[1] < rpm - RPM_MATCH_TOL {
            below_idx = Some(i);
        } else if r[1] > rpm + RPM_MATCH_TOL && above_idx.is_none() {
            above_idx = Some(i);
        }
    }
    let (b, a) = (below_idx?, above_idx?);
    let r0 = ladder[b];
    let r1 = ladder[a];
    let t_rpm = (rpm - r0[1]) / (r1[1] - r0[1]);
    Some(vec![
        MtFig4RpmBracketRow {
            da_ft: r0[0],
            rpm: r0[1],
            rpm_weight: 1.0 - t_rpm,
            cells: vec![cell_for_row(r0)],
        },
        MtFig4RpmBracketRow {
            da_ft: r1[0],
            rpm: r1[1],
            rpm_weight: t_rpm,
            cells: vec![cell_for_row(r1)],
        },
    ])
}

fn rpm_ladder_min_max(ladder: &[&Vec<f64>]) -> Option<(f64, f64)> {
    if ladder.is_empty() {
        return None;
    }
    Some((ladder[0][1], ladder[ladder.len() - 1][1]))
}

fn rpm_on_ladder(ladder: &[&Vec<f64>], rpm: f64) -> bool {
    let Some((lo, hi)) = rpm_ladder_min_max(ladder) else {
        return false;
    };
    rpm >= lo - CHART_EDGE_TOL && rpm <= hi + CHART_EDGE_TOL
}

fn rmin_on_tabulated_da(rows: &[Vec<f64>], da_tab: f64) -> Option<f64> {
    let ladder = rows_at_da(rows, da_tab);
    rpm_ladder_min_max(&ladder).map(|(lo, _)| lo)
}

fn rmax_on_tabulated_da(rows: &[Vec<f64>], da_tab: f64) -> Option<f64> {
    let ladder = rows_at_da(rows, da_tab);
    rpm_ladder_min_max(&ladder).map(|(_, hi)| hi)
}

pub fn rmax_at_da(rows: &[Vec<f64>], da: f64) -> Result<f64, CruiseError> {
    let das = unique_sorted_das(rows);
    if let Some(&exact) = das.iter().find(|d| (*d - da).abs() < DA_MATCH_TOL) {
        return rmax_on_tabulated_da(rows, exact).ok_or(CruiseError::OutOfChart {
            axis: "Fig 4 DA",
            min: limits::FIG4_DA_FT.0,
            max: limits::FIG4_DA_FT.1,
        });
    }
    let mut j = 0usize;
    while j + 1 < das.len() && das[j + 1] < da - DA_MATCH_TOL {
        j += 1;
    }
    if j + 1 >= das.len() {
        return Err(CruiseError::OutOfChart {
            axis: "Fig 4 DA",
            min: limits::FIG4_DA_FT.0,
            max: limits::FIG4_DA_FT.1,
        });
    }
    let d0 = das[j];
    let d1 = das[j + 1];
    let r0 = rmax_on_tabulated_da(rows, d0).ok_or(CruiseError::OutOfChart {
        axis: "Fig 4 DA",
        min: limits::FIG4_DA_FT.0,
        max: limits::FIG4_DA_FT.1,
    })?;
    let r1 = rmax_on_tabulated_da(rows, d1).ok_or(CruiseError::OutOfChart {
        axis: "Fig 4 DA",
        min: limits::FIG4_DA_FT.0,
        max: limits::FIG4_DA_FT.1,
    })?;
    Ok(lerp(d0, r0, d1, r1, da))
}

pub fn rmin_at_da(rows: &[Vec<f64>], da: f64) -> Result<f64, CruiseError> {
    let das = unique_sorted_das(rows);
    if let Some(&exact) = das.iter().find(|d| (*d - da).abs() < DA_MATCH_TOL) {
        return rmin_on_tabulated_da(rows, exact).ok_or(CruiseError::OutOfChart {
            axis: "Fig 4 DA",
            min: limits::FIG4_DA_FT.0,
            max: limits::FIG4_DA_FT.1,
        });
    }
    let mut j = 0usize;
    while j + 1 < das.len() && das[j + 1] < da - DA_MATCH_TOL {
        j += 1;
    }
    if j + 1 >= das.len() {
        return Err(CruiseError::OutOfChart {
            axis: "Fig 4 DA",
            min: limits::FIG4_DA_FT.0,
            max: limits::FIG4_DA_FT.1,
        });
    }
    let d0 = das[j];
    let d1 = das[j + 1];
    let r0 = rmin_on_tabulated_da(rows, d0).ok_or(CruiseError::OutOfChart {
        axis: "Fig 4 DA",
        min: limits::FIG4_DA_FT.0,
        max: limits::FIG4_DA_FT.1,
    })?;
    let r1 = rmin_on_tabulated_da(rows, d1).ok_or(CruiseError::OutOfChart {
        axis: "Fig 4 DA",
        min: limits::FIG4_DA_FT.0,
        max: limits::FIG4_DA_FT.1,
    })?;
    Ok(lerp(d0, r0, d1, r1, da))
}

fn normalized_s(rpm: f64, rmin: f64, rpm_max: f64) -> f64 {
    (rpm - rmin) / (rpm_max - rmin)
}

const FIG4_RPM_FLOOR: f64 = 2200.0;

fn rpm_row_at_tabulated_da(rows: &[Vec<f64>], da_tab: f64, s: f64) -> Option<f64> {
    let rmax = rmax_on_tabulated_da(rows, da_tab)?;
    Some(FIG4_RPM_FLOOR + s * (rmax - FIG4_RPM_FLOOR))
}

fn lookup_on_da_ladder(ladder: &[&Vec<f64>], rpm: f64) -> Option<(Triple, LookupFlags)> {
    if ladder.is_empty() || !rpm_on_ladder(ladder, rpm) {
        return None;
    }
    if let Some(row) = ladder.iter().find(|r| (r[1] - rpm).abs() < RPM_MATCH_TOL) {
        return Some((
            Triple {
                bhp: row[2],
                tas: row[3],
                gph: row[4],
            },
            LookupFlags::default(),
        ));
    }
    let mut below_idx = None;
    let mut above_idx = None;
    for (i, r) in ladder.iter().enumerate() {
        if r[1] < rpm - RPM_MATCH_TOL {
            below_idx = Some(i);
        } else if r[1] > rpm + RPM_MATCH_TOL && above_idx.is_none() {
            above_idx = Some(i);
        }
    }
    let (b, a) = (below_idx?, above_idx?);
    let r0 = ladder[b];
    let r1 = ladder[a];
    let t_rpm = (rpm - r0[1]) / (r1[1] - r0[1]);
    let flags = LookupFlags {
        rpm_interp: true,
        ..LookupFlags::default()
    };
    Some((
        lerp_triple(
            Triple {
                bhp: r0[2],
                tas: r0[3],
                gph: r0[4],
            },
            Triple {
                bhp: r1[2],
                tas: r1[3],
                gph: r1[4],
            },
            t_rpm,
        ),
        flags,
    ))
}

fn lookup_at_tabulated_da(
    rows: &[Vec<f64>],
    da_tab: f64,
    rpm_row: f64,
) -> Option<(Triple, LookupFlags)> {
    lookup_on_da_ladder(&rows_at_da(rows, da_tab), rpm_row)
}

pub fn recompute_triple_from_mt_fig4_source(trace: &MtFig4SourceTrace) -> (f64, f64, f64) {
    let mut acc = Triple {
        bhp: 0.0,
        tas: 0.0,
        gph: 0.0,
    };
    for da_row in &trace.da_rows {
        let mut da_tri = Triple {
            bhp: 0.0,
            tas: 0.0,
            gph: 0.0,
        };
        for rpm_row in &da_row.rpm_rows {
            let mut row_tri = Triple {
                bhp: 0.0,
                tas: 0.0,
                gph: 0.0,
            };
            for cell in &rpm_row.cells {
                if cell.used {
                    row_tri.bhp += cell.weight * cell.percent_bhp;
                    row_tri.tas += cell.weight * cell.tas_kt;
                    row_tri.gph += cell.weight * cell.gph;
                }
            }
            da_tri.bhp += rpm_row.rpm_weight * row_tri.bhp;
            da_tri.tas += rpm_row.rpm_weight * row_tri.tas;
            da_tri.gph += rpm_row.rpm_weight * row_tri.gph;
        }
        acc.bhp += da_row.da_weight * da_tri.bhp;
        acc.tas += da_row.da_weight * da_tri.tas;
        acc.gph += da_row.da_weight * da_tri.gph;
    }
    (acc.bhp, acc.tas, acc.gph)
}

fn merge_flags(a: &mut LookupFlags, b: LookupFlags) {
    a.rpm_interp |= b.rpm_interp;
    a.da_interp |= b.da_interp;
}

fn resolve_tag(da: f64, rpm: f64, rows: &[Vec<f64>], f: &LookupFlags) -> AfmLookupTag {
    let exact = !f.rpm_interp
        && !f.da_interp
        && rows.iter().any(|r| {
            row_valid(r) && (r[0] - da).abs() < DA_MATCH_TOL && (r[1] - rpm).abs() < RPM_MATCH_TOL
        });
    if exact {
        AfmLookupTag::Table
    } else {
        AfmLookupTag::Interpolated
    }
}

fn lookup_fig4(
    rows: &[Vec<f64>],
    da: f64,
    rpm: f64,
) -> Result<(Triple, AfmLookupTag, LookupFlags, MtFig4SourceTrace), CruiseError> {
    if da < limits::FIG4_DA_FT.0 - CHART_EDGE_TOL || da > limits::FIG4_DA_FT.1 + CHART_EDGE_TOL {
        return Err(CruiseError::OutOfChart {
            axis: "Fig 4 DA",
            min: limits::FIG4_DA_FT.0,
            max: limits::FIG4_DA_FT.1,
        });
    }

    let das = unique_sorted_das(rows);
    if das.is_empty() {
        return Err(CruiseError::OutOfChart {
            axis: "Fig 4 DA",
            min: limits::FIG4_DA_FT.0,
            max: limits::FIG4_DA_FT.1,
        });
    }

    let rpm_lo = rmin_at_da(rows, da)?;
    let rpm_max = rmax_at_da(rows, da)?;
    if rpm < rpm_lo - CHART_EDGE_TOL || rpm > rpm_max + CHART_EDGE_TOL {
        return Err(CruiseError::OutOfChart {
            axis: "Fig 4 RPM",
            min: rpm_lo,
            max: rpm_max,
        });
    }

    let s = normalized_s(rpm, rpm_lo, rpm_max);
    let mut source = MtFig4SourceTrace {
        query_pa_ft: f64::NAN,
        query_da_ft: da,
        query_rpm: rpm,
        da_rows: Vec::new(),
    };

    let (d0, d1) = if let Some(&exact) = das.iter().find(|d| (*d - da).abs() < DA_MATCH_TOL) {
        (exact, exact)
    } else {
        let mut j = 0usize;
        while j + 1 < das.len() && das[j + 1] < da - DA_MATCH_TOL {
            j += 1;
        }
        (das[j], das[j + 1])
    };

    if (d0 - d1).abs() < DA_MATCH_TOL {
        let rpm_row = rpm_row_at_tabulated_da(rows, d0, s).ok_or(CruiseError::OutOfChart {
            axis: "Fig 4 RPM",
            min: rpm_lo,
            max: rpm_max,
        })?;
        let rpm_rows =
            trace_on_da_ladder(&rows_at_da(rows, d0), rpm_row).ok_or(CruiseError::OutOfChart {
                axis: "Fig 4 RPM",
                min: rpm_lo,
                max: rpm_max,
            })?;
        source.da_rows.push(MtFig4DaBracketRow {
            da_ft: d0,
            rpm_sample: rpm_row,
            da_weight: 1.0,
            rpm_rows,
        });
        let (tri, f) =
            lookup_at_tabulated_da(rows, d0, rpm_row).ok_or(CruiseError::OutOfChart {
                axis: "Fig 4 RPM",
                min: rpm_lo,
                max: rpm_max,
            })?;
        let tag = resolve_tag(da, rpm, rows, &f);
        return Ok((tri, tag, f, source));
    }

    let t = (da - d0) / (d1 - d0);
    let mut combined = LookupFlags::default();
    let mut tri0 = None;
    let mut tri1 = None;
    for (d_tab, w) in [(d0, 1.0 - t), (d1, t)] {
        let rpm_row = rpm_row_at_tabulated_da(rows, d_tab, s).ok_or(CruiseError::OutOfChart {
            axis: "Fig 4 RPM",
            min: rpm_lo,
            max: rpm_max,
        })?;
        let rpm_rows = trace_on_da_ladder(&rows_at_da(rows, d_tab), rpm_row).ok_or(
            CruiseError::OutOfChart {
                axis: "Fig 4 RPM",
                min: rpm_lo,
                max: rpm_max,
            },
        )?;
        source.da_rows.push(MtFig4DaBracketRow {
            da_ft: d_tab,
            rpm_sample: rpm_row,
            da_weight: w,
            rpm_rows,
        });
        let (tri, f) =
            lookup_at_tabulated_da(rows, d_tab, rpm_row).ok_or(CruiseError::OutOfChart {
                axis: "Fig 4 RPM",
                min: rpm_lo,
                max: rpm_max,
            })?;
        merge_flags(&mut combined, f);
        if d_tab == d0 {
            tri0 = Some(tri);
        } else {
            tri1 = Some(tri);
        }
    }
    combined.da_interp = true;
    let tri = lerp_triple(tri0.unwrap(), tri1.unwrap(), t);
    let tag = resolve_tag(da, rpm, rows, &combined);
    Ok((tri, tag, combined, source))
}

fn curve_highlights(brackets: &[crate::mt175r::charts::MtCurveBracket]) -> Vec<MtCurveHighlight> {
    brackets
        .iter()
        .map(|b| MtCurveHighlight {
            pct_bhp: b.pct_bhp,
            weight: b.weight,
        })
        .collect()
}

fn solve_rpm_for_percent_bhp(
    data: &Mt175rData,
    da: f64,
    target_bhp: f64,
) -> Result<(f64, LookupFlags), CruiseError> {
    let fig2 = lookup_fig2_rpm_at_pct(data, da, target_bhp).ok_or(CruiseError::OutOfChart {
        axis: "Fig 2 %BHP",
        min: crate::mt175r::charts::MT_PCT_BHP_MIN,
        max: crate::mt175r::charts::MT_PCT_BHP_MAX,
    })?;
    Ok((
        fig2.rpm,
        LookupFlags {
            rpm_interp: fig2.pct_interp,
            ..LookupFlags::default()
        },
    ))
}

fn doubtful_gph_warnings(trace: &MtFig4SourceTrace) -> Vec<CalculationWarning> {
    const DOUBTFUL: [(f64, f64); 2] = [(10_000.0, 2200.0), (12_000.0, 2200.0)];
    let mut out = Vec::new();
    for (da_tgt, rpm_tgt) in DOUBTFUL {
        for da_row in &trace.da_rows {
            for rpm_row in &da_row.rpm_rows {
                if (da_row.da_ft - da_tgt).abs() < DA_MATCH_TOL
                    && (rpm_row.rpm - rpm_tgt).abs() < RPM_MATCH_TOL
                {
                    let used = rpm_row.cells.iter().any(|c| c.used && c.weight > 1e-9);
                    let weight = da_row.da_weight * rpm_row.rpm_weight;
                    if used && weight > 1e-9 {
                        out.push(CalculationWarning::with_args(
                            "MT_DOUBTFUL_GPH",
                            vec![format!("{da_tgt:.0}"), format!("{rpm_tgt:.0}")],
                        ));
                    }
                }
            }
        }
    }
    out
}

pub fn mt_fig4_trace_uses_cell(trace: &MtFig4SourceTrace, da_ft: f64, rpm: f64) -> bool {
    trace.da_rows.iter().any(|dr| {
        (dr.da_ft - da_ft).abs() < DA_MATCH_TOL
            && dr.rpm_rows.iter().any(|rr| {
                (rr.rpm - rpm).abs() < RPM_MATCH_TOL
                    && rr.cells.iter().any(|c| c.used && c.weight > 1e-9)
            })
    })
}

struct MtCruiseBuild<'a> {
    mt: &'a Mt175rData,
    pa: f64,
    da: f64,
    pct_bhp: f64,
    rpm: f64,
    tas_fig3_kt: f64,
    fig2_source: MtFig2SourceTrace,
    fig3_source: MtFig3SourceTrace,
    extra_flags: LookupFlags,
    chart_interp: bool,
    warnings: Vec<CalculationWarning>,
    fairings: bool,
}

fn mt_fig4_gph_out_of_chart(mt: &Mt175rData, da: f64, rpm: f64) -> bool {
    if da < limits::FIG4_DA_FT.0 - CHART_EDGE_TOL || rpm < FIG4_RPM_FLOOR - CHART_EDGE_TOL {
        return true;
    }
    lookup_fig4(mt.fig4_rows(), da, rpm).is_err()
}

/// Valid range string bounds for partial Fig 4 GPH (floor vs RPM ceiling at this DA).
pub fn fig4_gph_partial_ooc_range(mt: &Mt175rData, da: f64, rpm: f64) -> ChartOutOfRange {
    if da < limits::FIG4_DA_FT.0 - CHART_EDGE_TOL || rpm < FIG4_RPM_FLOOR - CHART_EDGE_TOL {
        return ChartOutOfRange {
            label: "Fig 4 GPH",
            min: limits::FIG4_DA_FT.0,
            max: FIG4_RPM_FLOOR,
        };
    }
    let rows = mt.fig4_rows();
    let min = rmin_at_da(rows, da).unwrap_or(FIG4_RPM_FLOOR);
    let max = rmax_at_da(rows, da).unwrap_or(mt.rpm_max());
    ChartOutOfRange {
        label: "Fig 4 GPH",
        min,
        max,
    }
}

fn build_mt_cruise_result(ctx: MtCruiseBuild<'_>) -> Result<CruiseResult, CruiseError> {
    let m = cruise_fairing_multiplier(ctx.fairings);
    let tas = ctx.tas_fig3_kt * m;
    let mut warnings = ctx.warnings;

    if mt_fig4_gph_out_of_chart(ctx.mt, ctx.da, ctx.rpm) {
        let ooc = fig4_gph_partial_ooc_range(ctx.mt, ctx.da, ctx.rpm);
        warnings.push(ooc.warning());
        return Ok(CruiseResult {
            provenance: ctx.mt.provenance(),
            afm_lookup_tag: AfmLookupTag::Interpolated,
            rpm: Some(ctx.rpm),
            percent_bhp: Some(ctx.pct_bhp),
            tas_chart_kt: Some(ctx.tas_fig3_kt),
            tas_kt: tas,
            fuel_flow_chart_us_gal_h: None,
            fuel_flow_us_gal_h: 0.0,
            table3_source: None,
            mt_fig4_source: None,
            mt_fig2_source: Some(ctx.fig2_source),
            mt_fig3_source: Some(ctx.fig3_source),
            mt_fig4_ktas_kt: None,
            density_altitude_ft: Some(ctx.da),
            mt_gph_out_of_chart: true,
            warnings,
        });
    }

    let rows = ctx.mt.fig4_rows();
    let (tri, _, mut flags, mut mt_source) = lookup_fig4(rows, ctx.da, ctx.rpm)?;
    merge_flags(&mut flags, ctx.extra_flags);
    mt_source.query_pa_ft = ctx.pa;
    let tag = if ctx.chart_interp || flags.rpm_interp || flags.da_interp {
        AfmLookupTag::Interpolated
    } else {
        resolve_tag(ctx.da, ctx.rpm, rows, &flags)
    };
    warnings.extend(doubtful_gph_warnings(&mt_source));

    Ok(CruiseResult {
        provenance: ctx.mt.provenance(),
        afm_lookup_tag: tag,
        rpm: Some(ctx.rpm),
        percent_bhp: Some(ctx.pct_bhp),
        tas_chart_kt: Some(ctx.tas_fig3_kt),
        tas_kt: tas,
        fuel_flow_chart_us_gal_h: Some(tri.gph),
        fuel_flow_us_gal_h: tri.gph,
        table3_source: None,
        mt_fig4_source: Some(mt_source),
        mt_fig2_source: Some(ctx.fig2_source),
        mt_fig3_source: Some(ctx.fig3_source),
        mt_fig4_ktas_kt: Some(tri.tas),
        density_altitude_ft: Some(ctx.da),
        mt_gph_out_of_chart: false,
        warnings,
    })
}

pub fn compute_mt175r_cruise(
    mt: &Mt175rData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    let pa = inputs.cruise_pressure_alt_ft;
    let da = isa_troposphere_density_altitude_ft(pa, inputs.cruise_oat_c);
    let warnings = vec![CalculationWarning::code_only("FIG4_GPH")];

    let (pct_bhp, rpm, fig2_source, extra_flags, chart_interp) = match inputs.cruise_solve_mode {
        CruiseSolveMode::SetRpm => {
            let inv = lookup_fig2_pct_at_rpm(mt, da, inputs.cruise_rpm).ok_or(
                CruiseError::OutOfChart {
                    axis: "Fig 2 RPM",
                    min: FIG4_RPM_FLOOR,
                    max: mt.rpm_max(),
                },
            )?;
            let trace = MtFig2SourceTrace {
                query_da_ft: da,
                query_pct_bhp: None,
                query_rpm: Some(inputs.cruise_rpm),
                rpm_result: inputs.cruise_rpm,
                curve_brackets: curve_highlights(&inv.brackets),
            };
            (
                inv.percent_bhp,
                inputs.cruise_rpm,
                trace,
                LookupFlags {
                    rpm_interp: inv.pct_interp,
                    ..LookupFlags::default()
                },
                inv.pct_interp,
            )
        }
        CruiseSolveMode::SetPercentBhp => {
            let (solved, inv) = solve_rpm_for_percent_bhp(mt, da, inputs.cruise_percent_bhp)?;
            let brackets = lookup_fig2_rpm_at_pct(mt, da, inputs.cruise_percent_bhp)
                .map(|f| curve_highlights(&f.brackets))
                .unwrap_or_default();
            let fig2_interp = brackets.len() > 1;
            let trace = MtFig2SourceTrace {
                query_da_ft: da,
                query_pct_bhp: Some(inputs.cruise_percent_bhp),
                query_rpm: None,
                rpm_result: solved,
                curve_brackets: brackets,
            };
            (inputs.cruise_percent_bhp, solved, trace, inv, fig2_interp)
        }
    };

    let fig3 = lookup_fig3_tas_at_pct(mt, da, pct_bhp).ok_or(CruiseError::OutOfChart {
        axis: "Fig 3 TAS",
        min: crate::mt175r::charts::MT_PCT_BHP_MIN,
        max: crate::mt175r::charts::MT_PCT_BHP_MAX,
    })?;
    let fig3_source = MtFig3SourceTrace {
        query_da_ft: da,
        query_pct_bhp: pct_bhp,
        tas_result_kt: fig3.tas_kt,
        curve_brackets: curve_highlights(&fig3.brackets),
        capped_at_full_throttle: fig3.capped_at_full_throttle,
    };

    build_mt_cruise_result(MtCruiseBuild {
        mt,
        pa,
        da,
        pct_bhp,
        rpm,
        tas_fig3_kt: fig3.tas_kt,
        fig2_source,
        fig3_source,
        extra_flags,
        chart_interp: chart_interp || fig3.pct_interp,
        warnings,
        fairings: inputs.wheel_fairings_installed,
    })
}

static MT_DATA: std::sync::OnceLock<Mt175rData> = std::sync::OnceLock::new();

fn mt_data() -> &'static Mt175rData {
    MT_DATA.get_or_init(Mt175rData::production)
}

pub fn compute_cruise(
    _afm: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    compute_mt175r_cruise(mt_data(), inputs)
}

pub fn effective_cruise_highlight_rpm(inputs: &FlightInputs) -> f64 {
    match inputs.cruise_solve_mode {
        CruiseSolveMode::SetRpm => inputs.cruise_rpm,
        CruiseSolveMode::SetPercentBhp => compute_mt175r_cruise(mt_data(), inputs)
            .ok()
            .and_then(|r| r.rpm)
            .unwrap_or(inputs.cruise_rpm),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn da6000_65pct_bhp_mode() {
        let mt = Mt175rData::production();
        let da = 6000.0;
        let rows = mt.fig4_rows();
        let (rpm, _) = solve_rpm_for_percent_bhp(&mt, da, 65.0).unwrap();
        assert!((rpm - 2440.0).abs() < 5.0, "rpm={rpm}");
        let fig3 = lookup_fig3_tas_at_pct(&mt, da, 65.0).unwrap();
        assert!((fig3.tas_kt - 122.0).abs() < 0.5, "tas={}", fig3.tas_kt);
        let (tri, tag, _, _) = lookup_fig4(rows, da, rpm).unwrap();
        assert!((tri.gph - 6.14).abs() < 1e-2, "gph={}", tri.gph);
        assert_ne!(tri.tas, fig3.tas_kt);
        assert_eq!(tag, AfmLookupTag::Interpolated);
    }

    #[test]
    fn fig4_exact_node_da6000_rpm2400() {
        let mt = Mt175rData::production();
        let rows = mt.fig4_rows();
        let (tri, tag, _, trace) = lookup_fig4(rows, 6000.0, 2400.0).unwrap();
        assert_eq!(tag, AfmLookupTag::Table);
        assert!((tri.bhp - 63.0).abs() < 1e-6);
        assert!((tri.tas - 120.0).abs() < 1e-6);
        assert!((tri.gph - 5.9).abs() < 1e-6);
        assert_eq!(trace.da_rows.len(), 1);
        let (b, t, g) = recompute_triple_from_mt_fig4_source(&trace);
        assert!((b - tri.bhp).abs() < 1e-9);
        assert!((t - tri.tas).abs() < 1e-9);
        assert!((g - tri.gph).abs() < 1e-9);
    }

    #[test]
    fn fig4_normalized_interpolation_between_da_rows() {
        let mt = Mt175rData::production();
        let rows = mt.fig4_rows();
        let da = 6500.0;
        let rpm = 2550.0;
        let rmin = rmin_at_da(rows, da).unwrap();
        let rpm_max = rmax_at_da(rows, da).unwrap();
        let s = normalized_s(rpm, rmin, rpm_max);
        let rpm6 = rpm_row_at_tabulated_da(rows, 6000.0, s).unwrap();
        let rpm8 = rpm_row_at_tabulated_da(rows, 8000.0, s).unwrap();
        let t6 = lookup_at_tabulated_da(rows, 6000.0, rpm6).unwrap().0;
        let t8 = lookup_at_tabulated_da(rows, 8000.0, rpm8).unwrap().0;
        let (blend, _, _, trace) = lookup_fig4(rows, da, rpm).unwrap();
        assert_eq!(trace.da_rows.len(), 2);
        let (b, t, g) = recompute_triple_from_mt_fig4_source(&trace);
        assert!((b - blend.bhp).abs() < 1e-9);
        assert!((t - blend.tas).abs() < 1e-9);
        assert!((g - blend.gph).abs() < 1e-9);
        let expected_bhp = lerp(6000.0, t6.bhp, 8000.0, t8.bhp, da);
        assert!((blend.bhp - expected_bhp).abs() < 1e-6);
    }

    #[test]
    fn rmax_interpolates_9000_to_2600() {
        let mt = Mt175rData::production();
        let rows = mt.fig4_rows();
        let r = rmax_at_da(rows, 9000.0).unwrap();
        assert!((r - 2600.0).abs() < 1e-6, "rmax={r}");
    }

    #[test]
    fn da_edges_out_of_chart() {
        let mt = Mt175rData::production();
        let rows = mt.fig4_rows();
        assert!(lookup_fig4(rows, 1999.9, 2400.0).is_err());
        assert!(lookup_fig4(rows, 12_000.1, 2400.0).is_err());
    }

    #[test]
    fn rpm_edges_out_of_chart() {
        let mt = Mt175rData::production();
        let rows = mt.fig4_rows();
        let hi = rmax_at_da(rows, 6000.0).unwrap() + 0.1;
        assert!(lookup_fig4(rows, 6000.0, hi).is_err());
        assert!(lookup_fig4(rows, 6000.0, 2199.9).is_err());
    }

    #[test]
    fn inverse_matches_forward() {
        let mt = Mt175rData::production();
        let da = 6000.0;
        let target = 58.0;
        let (rpm, _) = solve_rpm_for_percent_bhp(&mt, da, target).unwrap();
        let inv = lookup_fig2_pct_at_rpm(&mt, da, rpm).unwrap();
        assert!((inv.percent_bhp - target).abs() < 0.05);
    }

    #[test]
    fn doubtful_gph_warning_at_10000_2200() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 10_000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(10_000.0),
            cruise_rpm: 2200.0,
            cruise_solve_mode: CruiseSolveMode::SetRpm,
            ..FlightInputs::default()
        };
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!(
            r.warnings
                .iter()
                .any(|w| w.code == "MT_DOUBTFUL_GPH" && w.args == ["10000", "2200"]),
            "{:?}",
            r.warnings
        );
        assert!((r.fuel_flow_us_gal_h - 3.3).abs() < 1e-6);
    }

    #[test]
    fn doubtful_gph_warning_at_12000_2200_via_fig4_lookup() {
        let mt = Mt175rData::production();
        let rows = mt.fig4_rows();
        let (_, _, _, trace) = lookup_fig4(rows, 12_000.0, 2200.0).unwrap();
        let warnings = doubtful_gph_warnings(&trace);
        assert!(
            warnings
                .iter()
                .any(|w| w.code == "MT_DOUBTFUL_GPH" && w.args == ["12000", "2200"]),
            "{warnings:?}",
        );
    }

    #[test]
    fn fig2_ooc_at_12000_rpm2200_below_45_curve() {
        let mt = Mt175rData::production();
        let pa = 12_000.0;
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(pa),
            cruise_rpm: 2200.0,
            cruise_solve_mode: CruiseSolveMode::SetRpm,
            ..FlightInputs::default()
        };
        assert!(compute_mt175r_cruise(&mt, &inputs).is_err());
    }

    #[test]
    fn mt_cruise_without_fairings_reduces_tas_five_percent_not_fuel_flow() {
        use crate::cruise::isa_temp_c_at_pa;

        let mt = Mt175rData::production();
        let cases: &[(f64, f64)] = &[(2000.0, 2400.0), (6000.0, 2440.0), (10_000.0, 2300.0)];
        for &(pa, rpm) in cases {
            let fairings_on = FlightInputs {
                propeller: crate::types::PropellerKind::Mt175R,
                cruise_pressure_alt_ft: pa,
                cruise_oat_c: isa_temp_c_at_pa(pa),
                cruise_rpm: rpm,
                cruise_solve_mode: CruiseSolveMode::SetRpm,
                wheel_fairings_installed: true,
                ..FlightInputs::default()
            };
            let fairings_off = FlightInputs {
                wheel_fairings_installed: false,
                ..fairings_on.clone()
            };
            let on = compute_mt175r_cruise(&mt, &fairings_on).unwrap();
            let off = compute_mt175r_cruise(&mt, &fairings_off).unwrap();
            let chart_tas = off.tas_chart_kt.expect("chart TAS");
            assert!(
                (off.tas_kt - chart_tas * 0.95).abs() < 1e-6,
                "pa={pa} rpm={rpm} off_tas={}",
                off.tas_kt
            );
            assert!(
                (on.tas_kt - chart_tas).abs() < 1e-6,
                "pa={pa} rpm={rpm} on_tas={}",
                on.tas_kt
            );
            let chart_ff = off.fuel_flow_chart_us_gal_h.expect("chart GPH");
            assert!(
                (off.fuel_flow_us_gal_h - chart_ff).abs() < 1e-9,
                "pa={pa} rpm={rpm}"
            );
            assert!(
                (on.fuel_flow_us_gal_h - chart_ff).abs() < 1e-9,
                "pa={pa} rpm={rpm}"
            );
        }
    }

    #[test]
    fn compute_full_inputs_da6000_rpm2440() {
        use crate::cruise::isa_temp_c_at_pa;

        let mt = Mt175rData::production();
        let pa = 6000.0;
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: isa_temp_c_at_pa(pa),
            cruise_rpm: 2440.0,
            ..FlightInputs::default()
        };
        let da = isa_troposphere_density_altitude_ft(pa, inputs.cruise_oat_c);
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!((r.density_altitude_ft.unwrap() - da).abs() <= 1.0);
        assert!((r.percent_bhp.unwrap() - 65.0).abs() < 0.15);
        assert!((r.tas_chart_kt.unwrap() - 122.12).abs() < 0.5);
        assert!(r.mt_fig4_ktas_kt.is_some());
        assert!((r.fuel_flow_us_gal_h - 6.14).abs() < 0.02);
    }

    #[test]
    fn partial_fig4_gph_da2000_pct55() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 2000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(2000.0),
            cruise_percent_bhp: 55.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!(r.mt_gph_out_of_chart);
        assert!((r.rpm.unwrap() - 2188.9).abs() < 1.0);
        assert!((r.tas_chart_kt.unwrap() - 108.58).abs() < 0.4);
        assert!(r.fuel_flow_chart_us_gal_h.is_none());
    }

    #[test]
    fn partial_fig4_gph_da2000_pct45() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 2000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(2000.0),
            cruise_percent_bhp: 45.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!(r.mt_gph_out_of_chart);
        assert!((r.rpm.unwrap() - 2025.0).abs() < 1.0);
        assert!((r.tas_chart_kt.unwrap() - 98.2).abs() < 0.3);
    }

    #[test]
    fn partial_fig4_gph_rpm_ceiling_da12000_pct65() {
        let mt = Mt175rData::production();
        let pa = 12000.0;
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(pa),
            cruise_percent_bhp: 65.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!(r.mt_gph_out_of_chart);
        assert!((r.rpm.unwrap() - 2568.2).abs() < 1.0);
        assert!((r.tas_chart_kt.unwrap() - 127.50).abs() < 0.4);
        assert!(r.fuel_flow_chart_us_gal_h.is_none());
    }

    #[test]
    fn partial_fig4_gph_rpm_ceiling_da8100_pct75() {
        let mt = Mt175rData::production();
        let pa = 8100.0;
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: pa,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(pa),
            cruise_percent_bhp: 75.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!(r.mt_gph_out_of_chart);
        assert!((r.rpm.unwrap() - 2646.1).abs() < 1.0);
        assert!((r.tas_chart_kt.unwrap() - 134.22).abs() < 0.4);
        assert!(r.fuel_flow_chart_us_gal_h.is_none());
    }

    #[test]
    fn fig2_55_peak_scan_at_12000_da() {
        let mt = Mt175rData::production();
        let rpm = lookup_fig2_rpm_at_pct(&mt, 12_000.0, 55.0).unwrap().rpm;
        assert!(
            (rpm - 2398.8).abs() < 0.5,
            "55% @12000 ft DA should be peak_scan ~2398.8 RPM from fig2_orig, got {rpm}"
        );
    }

    #[test]
    fn pct_bhp_below_45_out_of_chart() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 6000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(6000.0),
            cruise_percent_bhp: 44.9,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        assert!(matches!(
            compute_mt175r_cruise(&mt, &inputs),
            Err(CruiseError::OutOfChart { .. })
        ));
    }

    #[test]
    fn pct_bhp_above_75_out_of_chart() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 6000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(6000.0),
            cruise_percent_bhp: 76.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        assert!(matches!(
            compute_mt175r_cruise(&mt, &inputs),
            Err(CruiseError::OutOfChart { .. })
        ));
    }

    #[test]
    fn fig2_75_interpolates_at_8100_da() {
        let mt = Mt175rData::production();
        let rpm = lookup_fig2_rpm_at_pct(&mt, 8100.0, 75.0).unwrap().rpm;
        assert!((rpm - 2646.0).abs() < 3.0, "rpm={rpm}");
    }

    #[test]
    fn fig3_65_at_12000_meets_full_throttle_near_127_5_kt() {
        let mt = Mt175rData::production();
        let fig3 = lookup_fig3_tas_at_pct(&mt, 12_000.0, 65.0).unwrap();
        assert!((fig3.tas_kt - 127.5).abs() < 0.2, "tas={}", fig3.tas_kt);
    }

    #[test]
    fn pct_75_past_fig2_da_endpoint_is_ooc() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 9000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(9000.0),
            cruise_percent_bhp: 75.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            ..FlightInputs::default()
        };
        assert!(compute_mt175r_cruise(&mt, &inputs).is_err());
    }

    #[test]
    fn rpm_above_2800_fig2_ooc() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 2000.0,
            cruise_oat_c: crate::cruise::isa_temp_c_at_pa(2000.0),
            cruise_rpm: 2800.1,
            cruise_solve_mode: CruiseSolveMode::SetRpm,
            ..FlightInputs::default()
        };
        assert!(compute_mt175r_cruise(&mt, &inputs).is_err());
    }

    #[test]
    fn printed_validation_rows_fig2_fig3_gph_chain() {
        let mt = Mt175rData::production();
        let fig_only: &[(f64, f64, f64, f64)] = &[
            (2000.0, 55.0, 2188.9, 108.58),
            (4000.0, 65.0, 2395.5, 120.47),
        ];
        for &(da, pct, exp_rpm, exp_tas) in fig_only {
            let fig2 = lookup_fig2_rpm_at_pct(&mt, da, pct).expect("fig2");
            assert!((fig2.rpm - exp_rpm).abs() < 2.0, "da={da} pct={pct}");
            let fig3 = lookup_fig3_tas_at_pct(&mt, da, pct).expect("fig3");
            assert!((fig3.tas_kt - exp_tas).abs() < 0.4, "da={da} pct={pct}");
        }
        let full_chain: &[(f64, f64)] = &[
            (4000.0, 65.0),
            (6000.0, 65.0),
            (8000.0, 75.0),
            (10_000.0, 65.0),
        ];
        for &(da, pct) in full_chain {
            let fig2 = lookup_fig2_rpm_at_pct(&mt, da, pct).expect("fig2");
            let fig3 = lookup_fig3_tas_at_pct(&mt, da, pct).expect("fig3");
            let inputs = FlightInputs {
                propeller: crate::types::PropellerKind::Mt175R,
                cruise_pressure_alt_ft: da,
                cruise_oat_c: crate::cruise::isa_temp_c_at_pa(da),
                cruise_percent_bhp: pct,
                cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
                ..FlightInputs::default()
            };
            let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
            assert!((r.rpm.unwrap() - fig2.rpm).abs() < 1e-6);
            assert!((r.tas_chart_kt.unwrap() - fig3.tas_kt).abs() < 1e-6);
        }
    }

    #[test]
    fn roundtrip_pct_bhp_rpm_pct_modes() {
        let mt = Mt175rData::production();
        let da = 6500.0;
        for pct in [48.0, 52.5, 60.0, 68.0, 74.0] {
            let (rpm, _) = solve_rpm_for_percent_bhp(&mt, da, pct).unwrap();
            let inv = lookup_fig2_pct_at_rpm(&mt, da, rpm).unwrap();
            assert!((inv.percent_bhp - pct).abs() < 0.15, "pct={pct}");
        }
    }

    #[test]
    fn isa_density_altitude_fig1_example_note_range() {
        let da = isa_troposphere_density_altitude_ft(6000.0, 15.0);
        assert!((da - 7374.0).abs() < 2.0, "da={da}");
    }
}
