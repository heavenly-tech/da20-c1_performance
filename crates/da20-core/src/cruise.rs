use crate::chart_bounds::{limits, CHART_EDGE_TOL};
use crate::data::PerformanceData;
use crate::interpolation::lerp;
use crate::provenance::provenance_for;
use crate::types::{
    cruise_fairing_multiplier, AfmLookupTag, CalculationWarning, CruiseResult, CruiseSolveMode,
    FlightInputs, PropellerKind, Table3PaBracketRow, Table3PublishedCell, Table3RpmBracketRow,
    Table3SourceTrace,
};

pub const CRUISE_TABLE3_GPH_NOTE: &str = "Table 3 GPH values are transcribed as printed in AFM Rev 29 (includes known oddities, e.g. 2000 ft / 2700 RPM ISA 6.8 GPH vs −20 °C 7.7 GPH; 4000 ft / 2800 RPM +10 °C 6.8 GPH).";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CruiseError {
    #[error("requested %BHP not achievable within Table 3 RPM coverage at this PA/ΔISA")]
    BhpNotAchievable,
    #[error("{axis} out of chart ({min}..{max})")]
    OutOfChart {
        axis: &'static str,
        min: f64,
        max: f64,
    },
}

pub fn isa_temp_c_at_pa(pa_ft: f64) -> f64 {
    15.0 - 0.0019812 * pa_ft
}

/// ΔISA = OAT − ISA at pressure altitude (°C vs °C).
pub fn disa_c(oat_c: f64, pa_ft: f64) -> f64 {
    oat_c - isa_temp_c_at_pa(pa_ft)
}

/// Match tabulated Table 3 row keys (integer PA/RPM nodes in JSON).
const RPM_MATCH_TOL: f64 = 0.5;
const PA_MATCH_TOL: f64 = 0.5;
const DISA_NODE_EPS: f64 = 1e-4;
/// Table 3 upper RPM limit (all rows).
pub const TABLE3_RPM_MAX: f64 = 2800.0;

#[derive(Clone, Copy, Debug)]
struct Triple {
    bhp: f64,
    tas: f64,
    gph: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct LookupFlags {
    rpm_interp: bool,
    pa_interp: bool,
    disa_interp: bool,
}

fn row_valid(r: &[f64]) -> bool {
    r.len() >= 11
}

fn unique_sorted_pas(rows: &[Vec<f64>]) -> Vec<f64> {
    let mut v: Vec<f64> = rows.iter().filter(|r| row_valid(r)).map(|r| r[0]).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|a, b| (*a - *b).abs() < PA_MATCH_TOL);
    v
}

fn rows_at_pa(rows: &[Vec<f64>], pa: f64) -> Vec<&Vec<f64>> {
    let mut v: Vec<_> = rows
        .iter()
        .filter(|r| row_valid(r) && (r[0] - pa).abs() < PA_MATCH_TOL)
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

fn disa_cells_for_row(row: &[f64], disa: f64) -> Option<Vec<Table3PublishedCell>> {
    let col = |disa_node: f64, off: usize| Table3PublishedCell {
        disa_c: disa_node,
        percent_bhp: row[off],
        tas_kt: row[off + 1],
        gph: row[off + 2],
        disa_weight: 0.0,
        disa_used: false,
    };
    let abs_ex = |d: f64, tgt: f64| (d - tgt).abs() < DISA_NODE_EPS;
    if abs_ex(disa, -20.0) {
        let mut c = col(-20.0, 2);
        c.disa_weight = 1.0;
        c.disa_used = true;
        return Some(vec![c]);
    }
    if abs_ex(disa, 0.0) {
        let mut c = col(0.0, 5);
        c.disa_weight = 1.0;
        c.disa_used = true;
        return Some(vec![c]);
    }
    if abs_ex(disa, 10.0) {
        let mut c = col(10.0, 8);
        c.disa_weight = 1.0;
        c.disa_used = true;
        return Some(vec![c]);
    }
    if disa < 0.0 {
        let t = (disa + 20.0) / 20.0;
        let mut c0 = col(-20.0, 2);
        let mut c1 = col(0.0, 5);
        c0.disa_weight = 1.0 - t;
        c1.disa_weight = t;
        c0.disa_used = true;
        c1.disa_used = true;
        Some(vec![c0, c1])
    } else {
        let t = disa / 10.0;
        let mut c0 = col(0.0, 5);
        let mut c1 = col(10.0, 8);
        c0.disa_weight = 1.0 - t;
        c1.disa_weight = t;
        c0.disa_used = true;
        c1.disa_used = true;
        Some(vec![c0, c1])
    }
}

fn trace_on_pa_ladder(
    ladder: &[&Vec<f64>],
    rpm: f64,
    disa: f64,
) -> Option<Vec<Table3RpmBracketRow>> {
    if ladder.is_empty() || !rpm_on_ladder(ladder, rpm) {
        return None;
    }
    if let Some(row) = ladder.iter().find(|r| (r[1] - rpm).abs() < RPM_MATCH_TOL) {
        let cells = disa_cells_for_row(row, disa)?;
        return Some(vec![Table3RpmBracketRow {
            pa_ft: row[0],
            rpm: row[1],
            rpm_weight: 1.0,
            cells,
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
        Table3RpmBracketRow {
            pa_ft: r0[0],
            rpm: r0[1],
            rpm_weight: 1.0 - t_rpm,
            cells: disa_cells_for_row(r0, disa)?,
        },
        Table3RpmBracketRow {
            pa_ft: r1[0],
            rpm: r1[1],
            rpm_weight: t_rpm,
            cells: disa_cells_for_row(r1, disa)?,
        },
    ])
}

fn trace_at_tabulated_pa(
    rows: &[Vec<f64>],
    pa_tab: f64,
    rpm_row: f64,
    disa: f64,
) -> Option<Vec<Table3RpmBracketRow>> {
    trace_on_pa_ladder(&rows_at_pa(rows, pa_tab), rpm_row, disa)
}

/// ΔISA strictly between printed columns −20 / 0 / +10 °C (C1).
fn interpolate_triple_along_disa(row: &[f64], disa: f64) -> Option<(Triple, bool)> {
    if disa < limits::TABLE3_DISA_C.0 - DISA_NODE_EPS
        || disa > limits::TABLE3_DISA_C.1 + DISA_NODE_EPS
    {
        return None;
    }
    let t_m20 = Triple {
        bhp: row[2],
        tas: row[3],
        gph: row[4],
    };
    let isa = Triple {
        bhp: row[5],
        tas: row[6],
        gph: row[7],
    };
    let t_p10 = Triple {
        bhp: row[8],
        tas: row[9],
        gph: row[10],
    };

    let abs_ex = |d: f64, tgt: f64| (d - tgt).abs() < DISA_NODE_EPS;

    if abs_ex(disa, -20.0) {
        return Some((t_m20, false));
    }
    if abs_ex(disa, 0.0) {
        return Some((isa, false));
    }
    if abs_ex(disa, 10.0) {
        return Some((t_p10, false));
    }

    if disa < 0.0 {
        let t = (disa + 20.0) / 20.0;
        Some((lerp_triple(t_m20, isa, t), true))
    } else {
        let t = disa / 10.0;
        Some((lerp_triple(isa, t_p10, t), true))
    }
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

/// First (minimum) RPM on a tabulated PA row's ladder.
fn rmin_on_tabulated_pa(rows: &[Vec<f64>], pa_tab: f64) -> Option<f64> {
    let ladder = rows_at_pa(rows, pa_tab);
    rpm_ladder_min_max(&ladder).map(|(lo, _)| lo)
}

/// Linear blend of each row's minimum RPM across PA (C2 domain floor).
pub fn rmin_at_pa(rows: &[Vec<f64>], pa: f64) -> Result<f64, CruiseError> {
    let pas = unique_sorted_pas(rows);
    if let Some(&exact) = pas.iter().find(|p| (*p - pa).abs() < PA_MATCH_TOL) {
        return rmin_on_tabulated_pa(rows, exact).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 PA",
            min: limits::TABLE3_PA_FT.0,
            max: limits::TABLE3_PA_FT.1,
        });
    }
    let mut j = 0usize;
    while j + 1 < pas.len() && pas[j + 1] < pa - PA_MATCH_TOL {
        j += 1;
    }
    if j + 1 >= pas.len() {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 PA",
            min: limits::TABLE3_PA_FT.0,
            max: limits::TABLE3_PA_FT.1,
        });
    }
    let p0 = pas[j];
    let p1 = pas[j + 1];
    let r0 = rmin_on_tabulated_pa(rows, p0).ok_or(CruiseError::OutOfChart {
        axis: "Table 3 PA",
        min: limits::TABLE3_PA_FT.0,
        max: limits::TABLE3_PA_FT.1,
    })?;
    let r1 = rmin_on_tabulated_pa(rows, p1).ok_or(CruiseError::OutOfChart {
        axis: "Table 3 PA",
        min: limits::TABLE3_PA_FT.0,
        max: limits::TABLE3_PA_FT.1,
    })?;
    Ok(lerp(p0, r0, p1, r1, pa))
}

fn normalized_s(rpm: f64, rmin: f64) -> f64 {
    (rpm - rmin) / (TABLE3_RPM_MAX - rmin)
}

fn rpm_row_at_tabulated_pa(rows: &[Vec<f64>], pa_tab: f64, s: f64) -> Option<f64> {
    let rmin = rmin_on_tabulated_pa(rows, pa_tab)?;
    Some(rmin + s * (TABLE3_RPM_MAX - rmin))
}

/// Interpolate in RPM on one PA row's ladder (no extrapolation beyond ladder ends).
fn lookup_on_pa_ladder(ladder: &[&Vec<f64>], rpm: f64, disa: f64) -> Option<(Triple, LookupFlags)> {
    if ladder.is_empty() || !rpm_on_ladder(ladder, rpm) {
        return None;
    }

    if let Some(row) = ladder.iter().find(|r| (r[1] - rpm).abs() < RPM_MATCH_TOL) {
        let (tri, di) = interpolate_triple_along_disa(row, disa)?;
        let mut flags = LookupFlags::default();
        flags.disa_interp |= di;
        return Some((tri, flags));
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
    let (t0, di0) = interpolate_triple_along_disa(r0, disa)?;
    let (t1, di1) = interpolate_triple_along_disa(r1, disa)?;
    let mut flags = LookupFlags::default();
    flags.disa_interp |= di0 || di1;
    flags.rpm_interp = true;
    Some((lerp_triple(t0, t1, t_rpm), flags))
}

fn lookup_at_tabulated_pa(
    rows: &[Vec<f64>],
    pa_tab: f64,
    rpm_row: f64,
    disa: f64,
) -> Option<(Triple, LookupFlags)> {
    lookup_on_pa_ladder(&rows_at_pa(rows, pa_tab), rpm_row, disa)
}

pub fn table3_trace_uses_cell(
    trace: &Table3SourceTrace,
    pa_ft: f64,
    rpm: f64,
    disa_c: f64,
) -> bool {
    let pa_match = |a: f64, b: f64| (a - b).abs() < PA_MATCH_TOL;
    let rpm_match = |a: f64, b: f64| (a - b).abs() < RPM_MATCH_TOL;
    let disa_match = |a: f64, b: f64| (a - b).abs() < DISA_NODE_EPS;
    trace.pa_rows.iter().any(|pr| {
        pa_match(pr.pa_ft, pa_ft)
            && pr.rpm_rows.iter().any(|rr| {
                rpm_match(rr.rpm, rpm)
                    && rr
                        .cells
                        .iter()
                        .any(|c| c.disa_used && disa_match(c.disa_c, disa_c))
            })
    })
}

pub fn recompute_triple_from_table3_source(trace: &Table3SourceTrace) -> (f64, f64, f64) {
    let mut acc = Triple {
        bhp: 0.0,
        tas: 0.0,
        gph: 0.0,
    };
    for pa_row in &trace.pa_rows {
        let mut pa_tri = Triple {
            bhp: 0.0,
            tas: 0.0,
            gph: 0.0,
        };
        for rpm_row in &pa_row.rpm_rows {
            let mut row_tri = Triple {
                bhp: 0.0,
                tas: 0.0,
                gph: 0.0,
            };
            for cell in &rpm_row.cells {
                if cell.disa_used {
                    row_tri.bhp += cell.disa_weight * cell.percent_bhp;
                    row_tri.tas += cell.disa_weight * cell.tas_kt;
                    row_tri.gph += cell.disa_weight * cell.gph;
                }
            }
            pa_tri.bhp += rpm_row.rpm_weight * row_tri.bhp;
            pa_tri.tas += rpm_row.rpm_weight * row_tri.tas;
            pa_tri.gph += rpm_row.rpm_weight * row_tri.gph;
        }
        acc.bhp += pa_row.pa_weight * pa_tri.bhp;
        acc.tas += pa_row.pa_weight * pa_tri.tas;
        acc.gph += pa_row.pa_weight * pa_tri.gph;
    }
    (acc.bhp, acc.tas, acc.gph)
}

fn lookup_table3(
    rows: &[Vec<f64>],
    pa: f64,
    rpm: f64,
    disa: f64,
) -> Result<(Triple, AfmLookupTag, LookupFlags, Table3SourceTrace), CruiseError> {
    if disa < limits::TABLE3_DISA_C.0 - DISA_NODE_EPS
        || disa > limits::TABLE3_DISA_C.1 + DISA_NODE_EPS
    {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 ΔISA",
            min: limits::TABLE3_DISA_C.0,
            max: limits::TABLE3_DISA_C.1,
        });
    }
    if pa < limits::TABLE3_PA_FT.0 - CHART_EDGE_TOL || pa > limits::TABLE3_PA_FT.1 + CHART_EDGE_TOL
    {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 PA",
            min: limits::TABLE3_PA_FT.0,
            max: limits::TABLE3_PA_FT.1,
        });
    }

    let pas = unique_sorted_pas(rows);
    if pas.is_empty() {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 PA",
            min: limits::TABLE3_PA_FT.0,
            max: limits::TABLE3_PA_FT.1,
        });
    }

    let rpm_lo = rmin_at_pa(rows, pa)?;
    if rpm < rpm_lo - CHART_EDGE_TOL || rpm > TABLE3_RPM_MAX + CHART_EDGE_TOL {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        });
    }

    let s = normalized_s(rpm, rpm_lo);
    let mut source = Table3SourceTrace {
        query_pa_ft: pa,
        query_rpm: rpm,
        query_disa_c: disa,
        pa_rows: Vec::new(),
    };

    let (p0, p1) = if let Some(&exact) = pas.iter().find(|p| (*p - pa).abs() < PA_MATCH_TOL) {
        (exact, exact)
    } else {
        let mut j = 0usize;
        while j + 1 < pas.len() && pas[j + 1] < pa - PA_MATCH_TOL {
            j += 1;
        }
        (pas[j], pas[j + 1])
    };

    let blend_from = |p0: f64, p1: f64, t: f64| -> Result<(Triple, LookupFlags), CruiseError> {
        let rpm0 = rpm_row_at_tabulated_pa(rows, p0, s).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        })?;
        let rpm1 = rpm_row_at_tabulated_pa(rows, p1, s).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        })?;
        let s0 = lookup_at_tabulated_pa(rows, p0, rpm0, disa).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        })?;
        let s1 = lookup_at_tabulated_pa(rows, p1, rpm1, disa).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        })?;
        let mut f = LookupFlags::default();
        merge_flags(&mut f, s0.1);
        merge_flags(&mut f, s1.1);
        if t > DISA_NODE_EPS && t < 1.0 - DISA_NODE_EPS {
            f.pa_interp = true;
        }
        Ok((lerp_triple(s0.0, s1.0, t), f))
    };

    if (p0 - p1).abs() < PA_MATCH_TOL {
        let rpm_row = rpm_row_at_tabulated_pa(rows, p0, s).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        })?;
        let rpm_rows =
            trace_at_tabulated_pa(rows, p0, rpm_row, disa).ok_or(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                min: rpm_lo,
                max: TABLE3_RPM_MAX,
            })?;
        source.pa_rows.push(Table3PaBracketRow {
            pa_ft: p0,
            rpm_sample: rpm_row,
            pa_weight: 1.0,
            rpm_rows,
        });
        let (tri, f) =
            lookup_at_tabulated_pa(rows, p0, rpm_row, disa).ok_or(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                min: rpm_lo,
                max: TABLE3_RPM_MAX,
            })?;
        let tag = resolve_tag(pa, rpm, disa, rows, &f);
        return Ok((tri, tag, f, source));
    }

    let t = (pa - p0) / (p1 - p0);
    for (p_tab, w) in [(p0, 1.0 - t), (p1, t)] {
        let rpm_row = rpm_row_at_tabulated_pa(rows, p_tab, s).ok_or(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        })?;
        let rpm_rows =
            trace_at_tabulated_pa(rows, p_tab, rpm_row, disa).ok_or(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                min: rpm_lo,
                max: TABLE3_RPM_MAX,
            })?;
        source.pa_rows.push(Table3PaBracketRow {
            pa_ft: p_tab,
            rpm_sample: rpm_row,
            pa_weight: w,
            rpm_rows,
        });
    }
    let (tri, f) = blend_from(p0, p1, t)?;
    let tag = resolve_tag(pa, rpm, disa, rows, &f);
    Ok((tri, tag, f, source))
}

fn merge_flags(a: &mut LookupFlags, b: LookupFlags) {
    a.rpm_interp |= b.rpm_interp;
    a.pa_interp |= b.pa_interp;
    a.disa_interp |= b.disa_interp;
}

/// Bisection inverse on the strict forward %BHP(RPM) surface (C4: no RPM slack).
fn solve_rpm_for_percent_bhp(
    rows: &[Vec<f64>],
    pa: f64,
    disa: f64,
    target_bhp: f64,
    hint_rpm: f64,
) -> Result<(f64, LookupFlags), CruiseError> {
    let rpm_lo = rmin_at_pa(rows, pa)?;
    let rpm_hi = TABLE3_RPM_MAX;

    let sample = |rpm: f64| -> Result<(f64, LookupFlags), CruiseError> {
        let (tri, _, f, _) = lookup_table3(rows, pa, rpm, disa)?;
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
        && lookup_table3(rows, pa, hint_rpm, disa).is_ok()
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
    let rpm = (lo + hi) * 0.5;
    Ok((rpm, inv_flags))
}

fn resolve_tag(pa: f64, rpm: f64, disa: f64, rows: &[Vec<f64>], f: &LookupFlags) -> AfmLookupTag {
    let exact_disa = (disa + 20.0).abs() < DISA_NODE_EPS
        || disa.abs() < DISA_NODE_EPS
        || (disa - 10.0).abs() < DISA_NODE_EPS;

    let exact = exact_disa
        && !f.rpm_interp
        && !f.pa_interp
        && !f.disa_interp
        && rows.iter().any(|r| {
            row_valid(r) && (r[0] - pa).abs() < PA_MATCH_TOL && (r[1] - rpm).abs() < RPM_MATCH_TOL
        });

    if exact {
        AfmLookupTag::Table
    } else {
        AfmLookupTag::Interpolated
    }
}

#[allow(clippy::too_many_arguments)]
fn build_cruise_result(
    data: &PerformanceData,
    pa: f64,
    rpm: f64,
    disa: f64,
    rows: &[Vec<f64>],
    extra_flags: LookupFlags,
    warnings: Vec<CalculationWarning>,
    fairings: bool,
) -> Result<CruiseResult, CruiseError> {
    let (tri, _, mut flags, table3_source) = lookup_table3(rows, pa, rpm, disa)?;
    merge_flags(&mut flags, extra_flags);
    let tag = resolve_tag(pa, rpm, disa, rows, &flags);

    let m = cruise_fairing_multiplier(fairings);
    let tas = tri.tas * m;

    Ok(CruiseResult {
        provenance: provenance_for(data.inner(), data.cruise_source()),
        afm_lookup_tag: tag,
        rpm: Some(rpm),
        percent_bhp: Some(tri.bhp),
        tas_chart_kt: Some(tri.tas),
        tas_kt: tas,
        fuel_flow_chart_us_gal_h: Some(tri.gph),
        fuel_flow_us_gal_h: tri.gph,
        table3_source: Some(table3_source),
        mt_fig4_source: None,
        density_altitude_ft: None,
        warnings,
    })
}

fn compute_cruise_sensenich(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    let pa = inputs.cruise_pressure_alt_ft;
    let rows = &data.inner().cruise_table.value.rows;
    let disa = disa_c(inputs.cruise_oat_c, pa);
    let mut warnings = vec![CalculationWarning::code_only("TABLE3_GPH")];

    let (rpm, extra_flags) = match inputs.cruise_solve_mode {
        CruiseSolveMode::SetRpm => (inputs.cruise_rpm, LookupFlags::default()),
        CruiseSolveMode::SetPercentBhp => match solve_rpm_for_percent_bhp(
            rows,
            pa,
            disa,
            inputs.cruise_percent_bhp,
            inputs.cruise_rpm,
        ) {
            Ok((solved, inv)) => (solved, inv),
            Err(CruiseError::BhpNotAchievable) => {
                warnings.push(CalculationWarning::with_args(
                    "CRUISE_BHP_NOT_ACHIEVABLE",
                    vec![format!("{:.2}", inputs.cruise_percent_bhp)],
                ));
                return Err(CruiseError::BhpNotAchievable);
            }
            Err(e @ CruiseError::OutOfChart { .. }) => return Err(e),
        },
    };

    build_cruise_result(
        data,
        pa,
        rpm,
        disa,
        rows,
        extra_flags,
        warnings,
        inputs.wheel_fairings_installed,
    )
}

pub fn compute_cruise(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    match inputs.propeller {
        PropellerKind::Mt175R => crate::mt175r::cruise::compute_cruise(data, inputs),
        PropellerKind::Sensenich => compute_cruise_sensenich(data, inputs),
    }
}

/// RPM row to highlight in the full Table 3 view (solved RPM in %BHP mode).
pub fn effective_cruise_highlight_rpm(data: &PerformanceData, inputs: &FlightInputs) -> f64 {
    match inputs.propeller {
        PropellerKind::Mt175R => crate::mt175r::cruise::effective_cruise_highlight_rpm(inputs),
        PropellerKind::Sensenich => match inputs.cruise_solve_mode {
            CruiseSolveMode::SetRpm => inputs.cruise_rpm,
            CruiseSolveMode::SetPercentBhp => compute_cruise_sensenich(data, inputs)
                .ok()
                .and_then(|r| r.rpm)
                .unwrap_or(inputs.cruise_rpm),
        },
    }
}

/// Cell tag for a published Table 3 row at a temperature column (ΔISA −20 / 0 / +10).
pub fn cruise_cell_tag_for_column(_pa: f64, _rpm: f64, col_disa: f64) -> AfmLookupTag {
    if (col_disa + 20.0).abs() < DISA_NODE_EPS
        || col_disa.abs() < DISA_NODE_EPS
        || (col_disa - 10.0).abs() < DISA_NODE_EPS
    {
        AfmLookupTag::Table
    } else {
        AfmLookupTag::Interpolated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PerformanceData;

    #[test]
    fn normalized_6500_isa_2600_row_samples_and_blend() {
        let data = PerformanceData::production();
        let rows = &data.inner().cruise_table.value.rows;
        let pa = 6500.0;
        let rpm = 2600.0;
        let disa = 0.0;
        let rmin = rmin_at_pa(rows, pa).unwrap();
        let s = normalized_s(rpm, rmin);
        let rpm6 = rpm_row_at_tabulated_pa(rows, 6000.0, s).unwrap();
        let rpm8 = rpm_row_at_tabulated_pa(rows, 8000.0, s).unwrap();
        assert!((rmin - 2512.5).abs() < 1e-9);
        assert!((s - 87.5 / 287.5).abs() < 1e-9);
        assert!((rpm6 - 2_591.3043478261).abs() < 1e-6);
        assert!((rpm8 - 2_626.0869565217).abs() < 1e-6);
        let t6 = lookup_at_tabulated_pa(rows, 6000.0, rpm6, disa).unwrap().0;
        let t8 = lookup_at_tabulated_pa(rows, 8000.0, rpm8, disa).unwrap().0;
        assert!((t6.bhp - 56.4782608696).abs() < 1e-6);
        assert!((t6.tas - 115.5652173913).abs() < 1e-6);
        assert!((t6.gph - 5.5565217391).abs() < 1e-6);
        assert!((t8.bhp - 55.5652173913).abs() < 1e-6);
        assert!((t8.tas - 117.3043478261).abs() < 1e-6);
        assert!((t8.gph - 5.4304347826).abs() < 1e-6);
        let (blend, _, _, _) = lookup_table3(rows, pa, rpm, disa).unwrap();
        assert!((blend.bhp - 56.25).abs() < 1e-9);
        assert!((blend.tas - 116.0).abs() < 1e-9);
        assert!((blend.gph - 5.525).abs() < 1e-9);
    }

    #[test]
    fn worked_example_6500ft_2600rpm_isa() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            wheel_fairings_installed: true,
            ..FlightInputs::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        assert!(
            (r.percent_bhp.unwrap() - 56.25).abs() < 1e-6,
            "bhp={}",
            r.percent_bhp.unwrap()
        );
        assert!((r.tas_chart_kt.unwrap() - 116.0).abs() < 1e-6);
        assert!((r.fuel_flow_us_gal_h - 5.525).abs() < 1e-6);
        assert_eq!(r.afm_lookup_tag, AfmLookupTag::Interpolated);
    }

    #[test]
    fn rpm_2650_at_6500_not_10000_2650_row() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2650.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            wheel_fairings_installed: true,
            ..FlightInputs::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        let row_10k = data
            .inner()
            .cruise_table
            .value
            .rows
            .iter()
            .find(|row| {
                row.len() >= 11 && (row[0] - 10000.0).abs() < 1.0 && (row[1] - 2650.0).abs() < 1.0
            })
            .expect("10000/2650 row");
        // TAS can match the 10k row by numerical coincidence; GPH/%BHP at 6500 ft still differ if we
        // interpolate 6000–8000 rather than snapping to the 10k published row.
        let gph_10k_isa = row_10k[7];
        let bhp_10k_isa = row_10k[5];
        assert!(
            (r.fuel_flow_us_gal_h - gph_10k_isa).abs() > 0.05
                || (r.percent_bhp.unwrap() - bhp_10k_isa).abs() > 0.25,
            "should not use 10000 ft row fuel/BHP alone (got {:.2} gph, {:.2} %BHP vs row {:.1} / {:.1})",
            r.fuel_flow_us_gal_h,
            r.percent_bhp.unwrap(),
            gph_10k_isa,
            bhp_10k_isa
        );
    }

    #[test]
    fn cruise_ignores_aerodrome_oat() {
        let data = PerformanceData::production();
        let mut inputs = FlightInputs {
            oat_c: 40.0,
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            ..FlightInputs::default()
        };
        let r1 = compute_cruise(&data, &inputs).unwrap();
        inputs.oat_c = -10.0;
        let r2 = compute_cruise(&data, &inputs).unwrap();
        assert!((r1.tas_kt - r2.tas_kt).abs() < 1e-9);
    }

    #[test]
    fn round_trip_rpm_bhp_on_inverse_band() {
        let data = PerformanceData::production();
        let rows = &data.inner().cruise_table.value.rows;
        let pas = unique_sorted_pas(rows);
        for &pa in &pas {
            for pa_mid in [pa - 250.0, pa, pa + 250.0] {
                if pa_mid < 0.0 {
                    continue;
                }
                let oat = isa_temp_c_at_pa(pa_mid);
                let ladder: Vec<f64> = rows_at_pa(rows, pa).into_iter().map(|r| r[1]).collect();
                for &rpm in &ladder {
                    let forward = FlightInputs {
                        cruise_pressure_alt_ft: pa_mid,
                        cruise_rpm: rpm,
                        cruise_oat_c: oat,
                        cruise_solve_mode: CruiseSolveMode::SetRpm,
                        wheel_fairings_installed: true,
                        ..FlightInputs::default()
                    };
                    let Ok(r_fwd) = compute_cruise(&data, &forward) else {
                        continue;
                    };
                    let bhp = r_fwd.percent_bhp.unwrap();
                    let inverse = FlightInputs {
                        cruise_pressure_alt_ft: pa_mid,
                        cruise_oat_c: oat,
                        cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
                        cruise_percent_bhp: bhp,
                        wheel_fairings_installed: true,
                        ..FlightInputs::default()
                    };
                    let r_inv = compute_cruise(&data, &inverse).expect("inverse");
                    assert!(
                        (r_inv.rpm.unwrap() - rpm).abs() < 1.0,
                        "pa={pa_mid} rpm={rpm} bhp={bhp} got={}",
                        r_inv.rpm.unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn table3_printed_nodes_reproduce_exactly() {
        let data = PerformanceData::production();
        let rows = &data.inner().cruise_table.value.rows;
        for row in rows {
            if !row_valid(row) {
                continue;
            }
            let pa = row[0];
            let rpm = row[1];
            for (disa, bhp, tas, gph) in [
                (-20.0, row[2], row[3], row[4]),
                (0.0, row[5], row[6], row[7]),
                (10.0, row[8], row[9], row[10]),
            ] {
                let oat = isa_temp_c_at_pa(pa) + disa;
                let inputs = FlightInputs {
                    cruise_pressure_alt_ft: pa,
                    cruise_rpm: rpm,
                    cruise_oat_c: oat,
                    cruise_solve_mode: CruiseSolveMode::SetRpm,
                    wheel_fairings_installed: true,
                    ..FlightInputs::default()
                };
                let r = compute_cruise(&data, &inputs).unwrap();
                assert_eq!(
                    r.afm_lookup_tag,
                    AfmLookupTag::Table,
                    "pa={pa} rpm={rpm} disa={disa}"
                );
                assert!((r.percent_bhp.unwrap() - bhp).abs() < 1e-6);
                assert!((r.tas_chart_kt.unwrap() - tas).abs() < 1e-6);
                assert!((r.fuel_flow_us_gal_h - gph).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn bhp_tas_gph_non_decreasing_in_rpm_property() {
        let data = PerformanceData::production();
        let disas = [-20.0, 0.0, 10.0];
        for pa in (2_000..=12_000).step_by(250) {
            let pa = pa as f64;
            for &disa in &disas {
                let oat = isa_temp_c_at_pa(pa) + disa;
                let mut prev_bhp = f64::NEG_INFINITY;
                let mut prev_tas = f64::NEG_INFINITY;
                let mut prev_gph = f64::NEG_INFINITY;
                let rows = &data.inner().cruise_table.value.rows;
                let Ok(rpm_floor) = rmin_at_pa(rows, pa) else {
                    continue;
                };
                let mut rpm_i = rpm_floor.ceil() as i32;
                if (rpm_i as f64) < rpm_floor - RPM_MATCH_TOL {
                    rpm_i += 1;
                }
                while (rpm_i as f64) <= TABLE3_RPM_MAX + CHART_EDGE_TOL {
                    let rpm = rpm_i as f64;
                    let inputs = FlightInputs {
                        cruise_pressure_alt_ft: pa,
                        cruise_rpm: rpm,
                        cruise_oat_c: oat,
                        cruise_solve_mode: CruiseSolveMode::SetRpm,
                        wheel_fairings_installed: true,
                        ..FlightInputs::default()
                    };
                    let Ok(r) = compute_cruise(&data, &inputs) else {
                        continue;
                    };
                    let bhp = r.percent_bhp.unwrap();
                    let tas = r.tas_chart_kt.unwrap();
                    let gph = r.fuel_flow_us_gal_h;
                    assert!(
                        bhp + 1e-6 >= prev_bhp,
                        "BHP decreased pa={pa} disa={disa} rpm={rpm}: {bhp} < {prev_bhp}"
                    );
                    assert!(
                        tas + 1e-6 >= prev_tas,
                        "TAS decreased pa={pa} disa={disa} rpm={rpm}: {tas} < {prev_tas}"
                    );
                    if (-20.0..=10.0).contains(&disa) {
                        assert!(
                            gph + 1e-6 >= prev_gph,
                            "GPH decreased pa={pa} disa={disa} rpm={rpm}: {gph} < {prev_gph}"
                        );
                        prev_gph = gph;
                    }
                    prev_bhp = bhp;
                    prev_tas = tas;
                    rpm_i += 10;
                }
            }
        }
    }

    #[test]
    fn rmin_interpolation_at_6500_ft() {
        let data = PerformanceData::production();
        let rows = &data.inner().cruise_table.value.rows;
        let rmin = rmin_at_pa(rows, 6500.0).unwrap();
        assert!((rmin - 2512.5).abs() < 1e-6, "rmin={rmin}");
    }

    #[test]
    fn rpm_boundary_6500_ft_2512_5_valid_2500_invalid() {
        let data = PerformanceData::production();
        let oat = isa_temp_c_at_pa(6500.0);
        let ok = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2512.5,
            cruise_oat_c: oat,
            ..Default::default()
        };
        compute_cruise(&data, &ok).unwrap();
        let bad = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2500.0,
            cruise_oat_c: oat,
            ..Default::default()
        };
        assert!(compute_cruise(&data, &bad).is_err());
    }

    #[test]
    fn monotonic_6500_isa_valid_rpm_band() {
        let data = PerformanceData::production();
        let oat = isa_temp_c_at_pa(6500.0);
        for rpm in [2400.0, 2450.0] {
            let inputs = FlightInputs {
                cruise_pressure_alt_ft: 6500.0,
                cruise_rpm: rpm,
                cruise_oat_c: oat,
                ..Default::default()
            };
            assert!(compute_cruise(&data, &inputs).is_err());
        }
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: oat,
            ..Default::default()
        };
        compute_cruise(&data, &inputs).unwrap();
    }

    #[test]
    fn worked_example_bhp_mode_6500ft_isa() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 56.25,
            wheel_fairings_installed: true,
            ..FlightInputs::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        assert!((r.rpm.unwrap() - 2600.0).abs() < 1e-6);
        assert!((r.percent_bhp.unwrap() - 56.25).abs() < 1e-6);
        assert!((r.tas_chart_kt.unwrap() - 116.0).abs() < 1e-6);
        assert!((r.fuel_flow_us_gal_h - 5.525).abs() < 1e-6);
        assert_eq!(r.afm_lookup_tag, AfmLookupTag::Interpolated);
    }

    #[test]
    fn unreachable_bhp_five_percent_not_achievable() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 5.0,
            wheel_fairings_installed: true,
            ..FlightInputs::default()
        };
        assert_eq!(
            compute_cruise(&data, &inputs),
            Err(CruiseError::BhpNotAchievable)
        );
    }

    #[test]
    fn table3_pa_edges_inclusive_with_float_epsilon() {
        let data = PerformanceData::production();
        let oat_at = |pa: f64| isa_temp_c_at_pa(pa);
        for (pa, rpm, expect_ok) in [
            (1999.9, 2600.0, false),
            (2000.0, 2600.0, true),
            (12000.0, 2700.0, true),
            (12000.1, 2700.0, false),
        ] {
            let inputs = FlightInputs {
                cruise_pressure_alt_ft: pa,
                cruise_rpm: rpm,
                cruise_oat_c: oat_at(pa),
                ..Default::default()
            };
            let res = compute_cruise(&data, &inputs);
            assert_eq!(res.is_ok(), expect_ok, "pa={pa}");
        }
    }

    #[test]
    fn table3_rpm_edges_just_outside_rmin_and_2800() {
        let data = PerformanceData::production();
        let rows = &data.inner().cruise_table.value.rows;
        let pa = 6500.0;
        let oat = isa_temp_c_at_pa(pa);
        let rmin = rmin_at_pa(rows, pa).unwrap();
        let below = FlightInputs {
            cruise_pressure_alt_ft: pa,
            cruise_rpm: rmin - 0.1,
            cruise_oat_c: oat,
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&data, &below),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                ..
            })
        ));
        let above = FlightInputs {
            cruise_pressure_alt_ft: pa,
            cruise_rpm: TABLE3_RPM_MAX + 0.1,
            cruise_oat_c: oat,
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&data, &above),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                ..
            })
        ));
    }

    #[test]
    fn rpm_3000_at_6500_out_of_chart() {
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_rpm: 3000.0,
            ..Default::default()
        };
        assert!(matches!(
            compute_cruise(&PerformanceData::production(), &inputs),
            Err(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                ..
            })
        ));
    }

    #[test]
    fn table3_source_recomputes_6500_isa_2600() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            ..Default::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        let trace = r.table3_source.as_ref().expect("trace");
        assert_eq!(trace.pa_rows.len(), 2);
        let (bhp, tas, gph) = recompute_triple_from_table3_source(trace);
        assert!((bhp - r.percent_bhp.unwrap()).abs() < 1e-9);
        assert!((tas - r.tas_chart_kt.unwrap()).abs() < 1e-9);
        assert!((gph - r.fuel_flow_us_gal_h).abs() < 1e-9);
    }

    #[test]
    fn table3_source_recomputes_disa_between_columns() {
        let data = PerformanceData::production();
        let pa = 6500.0;
        let disa = -5.0;
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: pa,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(pa) + disa,
            ..Default::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        let trace = r.table3_source.as_ref().unwrap();
        let (bhp, tas, gph) = recompute_triple_from_table3_source(trace);
        assert!((bhp - r.percent_bhp.unwrap()).abs() < 1e-9);
        assert!((tas - r.tas_chart_kt.unwrap()).abs() < 1e-9);
        assert!((gph - r.fuel_flow_us_gal_h).abs() < 1e-9);
    }

    #[test]
    fn table3_source_recomputes_exact_node() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6000.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6000.0),
            ..Default::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        let trace = r.table3_source.as_ref().unwrap();
        assert_eq!(trace.pa_rows.len(), 1);
        assert_eq!(trace.pa_rows[0].rpm_rows.len(), 1);
        let (bhp, _, _) = recompute_triple_from_table3_source(trace);
        assert!((bhp - r.percent_bhp.unwrap()).abs() < 1e-9);
    }

    #[test]
    fn table3_source_recomputes_bhp_mode() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 56.25,
            ..Default::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        let trace = r.table3_source.as_ref().unwrap();
        let (bhp, tas, gph) = recompute_triple_from_table3_source(trace);
        assert!((bhp - r.percent_bhp.unwrap()).abs() < 1e-9);
        assert!((tas - r.tas_chart_kt.unwrap()).abs() < 1e-9);
        assert!((gph - r.fuel_flow_us_gal_h).abs() < 1e-9);
    }

    #[test]
    fn cruise_without_fairings_reduces_tas_five_percent_not_fuel_flow() {
        let data = PerformanceData::production();
        let inputs_on = FlightInputs::default();
        let on = compute_cruise(&data, &inputs_on).unwrap();
        let inputs_off = FlightInputs {
            wheel_fairings_installed: false,
            ..FlightInputs::default()
        };
        let off = compute_cruise(&data, &inputs_off).unwrap();
        let chart_tas = off.tas_chart_kt.expect("chart TAS");
        assert!((off.tas_kt - chart_tas * 0.95).abs() < 1e-6);
        let chart_ff = off.fuel_flow_chart_us_gal_h.expect("chart FF");
        assert!((off.fuel_flow_us_gal_h - chart_ff).abs() < 1e-9);
        assert!((on.tas_kt - chart_tas).abs() < 1e-6);
        assert!((on.fuel_flow_us_gal_h - chart_ff).abs() < 1e-6);
    }
}
