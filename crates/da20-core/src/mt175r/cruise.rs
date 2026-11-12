use crate::altitude::nws_density_altitude_ft;
use crate::chart_bounds::{limits, CHART_EDGE_TOL};
use crate::cruise::CruiseError;
use crate::data::PerformanceData;
use crate::interpolation::lerp;
use crate::mt175r::data::Mt175rData;
use crate::types::{
    cruise_fairing_multiplier, AfmLookupTag, CalculationWarning, CruiseResult, CruiseSolveMode,
    FlightInputs, MtFig4DaBracketRow, MtFig4PublishedCell, MtFig4RpmBracketRow, MtFig4SourceTrace,
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

fn solve_rpm_for_percent_bhp(
    rows: &[Vec<f64>],
    da: f64,
    target_bhp: f64,
    hint_rpm: f64,
) -> Result<(f64, LookupFlags), CruiseError> {
    let rpm_lo = rmin_at_da(rows, da)?;
    let rpm_hi = rmax_at_da(rows, da)?;

    let sample = |rpm: f64| -> Result<(f64, LookupFlags), CruiseError> {
        let (tri, _, f, _) = lookup_fig4(rows, da, rpm)?;
        Ok((tri.bhp, f))
    };

    let (b_lo, f_lo) = sample(rpm_lo)?;
    let (b_hi, f_hi) = sample(rpm_hi)?;
    let band_min = b_lo.min(b_hi);
    let band_max = b_lo.max(b_hi);

    if target_bhp < band_min - 1e-4 || target_bhp > band_max + 1e-4 {
        return Err(CruiseError::BhpNotAchievable);
    }

    let mut inv_flags = LookupFlags::default();
    merge_flags(&mut inv_flags, f_lo);
    merge_flags(&mut inv_flags, f_hi);

    let increasing = b_hi >= b_lo - 1e-9;
    let mut lo = rpm_lo;
    let mut hi = rpm_hi;
    if hint_rpm >= rpm_lo - CHART_EDGE_TOL
        && hint_rpm <= rpm_hi + CHART_EDGE_TOL
        && lookup_fig4(rows, da, hint_rpm).is_ok()
    {
        let (b_h, f_h) = sample(hint_rpm)?;
        if (b_h - target_bhp).abs() < 1e-4 {
            merge_flags(&mut inv_flags, f_h);
            return Ok((hint_rpm, inv_flags));
        }
        if increasing {
            if b_h < target_bhp {
                lo = hint_rpm;
            } else {
                hi = hint_rpm;
            }
        } else if b_h > target_bhp {
            lo = hint_rpm;
        } else {
            hi = hint_rpm;
        }
    }

    for _ in 0..80 {
        let mid = (lo + hi) * 0.5;
        let (b_mid, f_mid) = sample(mid)?;
        merge_flags(&mut inv_flags, f_mid);
        if (b_mid - target_bhp).abs() < 1e-4 {
            return Ok((mid, inv_flags));
        }
        if increasing {
            if b_mid < target_bhp {
                lo = mid;
            } else {
                hi = mid;
            }
        } else if b_mid > target_bhp {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(((lo + hi) * 0.5, inv_flags))
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
    rpm: f64,
    extra_flags: LookupFlags,
    warnings: Vec<CalculationWarning>,
    fairings: bool,
}

fn build_mt_cruise_result(ctx: MtCruiseBuild<'_>) -> Result<CruiseResult, CruiseError> {
    let rows = ctx.mt.fig4_rows();
    let (tri, _, mut flags, mut mt_source) = lookup_fig4(rows, ctx.da, ctx.rpm)?;
    merge_flags(&mut flags, ctx.extra_flags);
    mt_source.query_pa_ft = ctx.pa;
    let tag = resolve_tag(ctx.da, ctx.rpm, rows, &flags);
    let mut warnings = ctx.warnings;
    warnings.extend(doubtful_gph_warnings(&mt_source));

    let m = cruise_fairing_multiplier(ctx.fairings);
    let tas = tri.tas * m;

    Ok(CruiseResult {
        provenance: ctx.mt.provenance(),
        afm_lookup_tag: tag,
        rpm: Some(ctx.rpm),
        percent_bhp: Some(tri.bhp),
        tas_chart_kt: Some(tri.tas),
        tas_kt: tas,
        fuel_flow_chart_us_gal_h: Some(tri.gph),
        fuel_flow_us_gal_h: tri.gph,
        table3_source: None,
        mt_fig4_source: Some(mt_source),
        density_altitude_ft: Some(ctx.da),
        warnings,
    })
}

pub fn compute_mt175r_cruise(
    mt: &Mt175rData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    let pa = inputs.cruise_pressure_alt_ft;
    let da = nws_density_altitude_ft(pa, inputs.cruise_oat_c);
    let rows = mt.fig4_rows();
    let mut warnings = vec![CalculationWarning::code_only("FIG4_GPH")];

    let (rpm, extra_flags) = match inputs.cruise_solve_mode {
        CruiseSolveMode::SetRpm => (inputs.cruise_rpm, LookupFlags::default()),
        CruiseSolveMode::SetPercentBhp => {
            match solve_rpm_for_percent_bhp(rows, da, inputs.cruise_percent_bhp, inputs.cruise_rpm)
            {
                Ok((solved, inv)) => (solved, inv),
                Err(CruiseError::BhpNotAchievable) => {
                    warnings.push(CalculationWarning::with_args(
                        "CRUISE_BHP_NOT_ACHIEVABLE",
                        vec![format!("{:.2}", inputs.cruise_percent_bhp)],
                    ));
                    return Err(CruiseError::BhpNotAchievable);
                }
                Err(e @ CruiseError::OutOfChart { .. }) => return Err(e),
            }
        }
    };

    build_mt_cruise_result(MtCruiseBuild {
        mt,
        pa,
        da,
        rpm,
        extra_flags,
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
        let (rpm, _) = solve_rpm_for_percent_bhp(rows, da, 65.0, 2400.0).unwrap();
        assert!((rpm - 2440.0).abs() < 1e-3, "rpm={rpm}");
        let (tri, tag, _, _) = lookup_fig4(rows, da, rpm).unwrap();
        assert!((tri.bhp - 65.0).abs() < 1e-2, "bhp={}", tri.bhp);
        assert!((tri.tas - 122.4).abs() < 1e-2, "tas={}", tri.tas);
        assert!((tri.gph - 6.14).abs() < 1e-2, "gph={}", tri.gph);
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
        let rows = mt.fig4_rows();
        let da = 6000.0;
        let target = 58.0;
        let (rpm, _) = solve_rpm_for_percent_bhp(rows, da, target, 2400.0).unwrap();
        let (tri, _, _, _) = lookup_fig4(rows, da, rpm).unwrap();
        assert!((tri.bhp - target).abs() < 0.05);
    }

    #[test]
    fn doubtful_gph_warning_at_10000_2200() {
        let mt = Mt175rData::production();
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 10_000.0,
            // OAT chosen so NWS DA matches the Fig 4 row at 10000 ft (ISA overshoots DA).
            cruise_oat_c: -4.954249,
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
    fn doubtful_gph_warning_at_12000_2200() {
        let mt = Mt175rData::production();
        let pa = 12_000.0;
        let inputs = FlightInputs {
            propeller: crate::types::PropellerKind::Mt175R,
            cruise_pressure_alt_ft: pa,
            // OAT chosen so NWS DA matches the Fig 4 row at 12000 ft (ISA gives DA above chart).
            cruise_oat_c: -8.916752156701492,
            cruise_rpm: 2200.0,
            cruise_solve_mode: CruiseSolveMode::SetRpm,
            ..FlightInputs::default()
        };
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!(
            r.warnings
                .iter()
                .any(|w| w.code == "MT_DOUBTFUL_GPH" && w.args == ["12000", "2200"]),
            "{:?}",
            r.warnings
        );
        assert!((r.fuel_flow_us_gal_h - 1.7).abs() < 1e-6);
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
    fn printed_example_pa6000_oat15_da() {
        let da = nws_density_altitude_ft(6000.0, 15.0);
        assert!((da - 7391.0).abs() <= 5.0, "da={da}");
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
        let da = nws_density_altitude_ft(pa, inputs.cruise_oat_c);
        let r = compute_mt175r_cruise(&mt, &inputs).unwrap();
        assert!((r.density_altitude_ft.unwrap() - da).abs() <= 1.0);
        assert!((r.percent_bhp.unwrap() - 65.0).abs() < 0.15);
        assert!((r.tas_chart_kt.unwrap() - 122.4).abs() < 0.1);
        assert!((r.fuel_flow_us_gal_h - 6.14).abs() < 0.02);
    }
}
