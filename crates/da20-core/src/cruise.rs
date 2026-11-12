use crate::data::PerformanceData;
use crate::interpolation::lerp;
use crate::provenance::provenance_for;
use crate::types::{
    cruise_fairing_multiplier, AfmLookupTag, CalculationWarning, CruiseResult, CruiseSolveMode,
    FlightInputs,
};

pub const CRUISE_BHP_EXTRAP_NOTE: &str =
    "Requested %BHP is outside the range spanned by tabulated RPM levels at this PA/ΔISA; RPM solved by linear extrapolation along the %BHP–RPM curve (not clamped).";

pub const CRUISE_TABLE3_GPH_NOTE: &str = "Table 3 GPH values are transcribed as printed in AFM Rev 29 (includes known oddities, e.g. 2000 ft / 2700 RPM ISA 6.8 GPH vs −20 °C 7.7 GPH; 4000 ft / 2800 RPM +10 °C 6.8 GPH).";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CruiseError {
    #[error("cruise row not found")]
    NotFound,
}

pub fn isa_temp_c_at_pa(pa_ft: f64) -> f64 {
    15.0 - 0.0019812 * pa_ft
}

/// ΔISA = OAT − ISA at pressure altitude (°C vs °C).
pub fn disa_c(oat_c: f64, pa_ft: f64) -> f64 {
    oat_c - isa_temp_c_at_pa(pa_ft)
}

const RPM_MATCH_TOL: f64 = 0.5;
const PA_MATCH_TOL: f64 = 0.5;
const DISA_NODE_EPS: f64 = 1e-4;

#[derive(Clone, Copy, Debug)]
struct Triple {
    bhp: f64,
    tas: f64,
    gph: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct LookupFlags {
    rpm_interp: bool,
    rpm_extrap: bool,
    pa_interp: bool,
    pa_extrap: bool,
    disa_interp: bool,
    disa_extrap: bool,
    /// True when PA is outside every altitude band where this RPM appears (no row at that PA).
    rpm_band_gap: bool,
    /// True when target %BHP lies outside the band covered by tabulated RPM rows (inverse solve).
    bhp_extrap: bool,
}

fn row_valid(r: &[f64]) -> bool {
    r.len() >= 11
}

fn unique_sorted_rpms(rows: &[Vec<f64>]) -> Vec<f64> {
    let mut v: Vec<f64> = rows.iter().filter(|r| row_valid(r)).map(|r| r[1]).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|a, b| (*a - *b).abs() < RPM_MATCH_TOL);
    v
}

fn rows_at_rpm(rows: &[Vec<f64>], rpm: f64) -> Vec<&Vec<f64>> {
    let mut v: Vec<_> = rows
        .iter()
        .filter(|r| row_valid(r) && (r[1] - rpm).abs() < RPM_MATCH_TOL)
        .collect();
    v.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
    v
}

fn pas_for_rpm(rows: &[Vec<f64>], rpm: f64) -> Option<Vec<f64>> {
    let at = rows_at_rpm(rows, rpm);
    if at.is_empty() {
        return None;
    }
    let mut p: Vec<f64> = at.iter().map(|r| r[0]).collect();
    p.sort_by(|a, b| a.partial_cmp(b).unwrap());
    p.dedup_by(|a, b| (*a - *b).abs() < PA_MATCH_TOL);
    Some(p)
}

fn lerp_triple(a: Triple, b: Triple, t: f64) -> Triple {
    Triple {
        bhp: lerp(0.0, a.bhp, 1.0, b.bhp, t),
        tas: lerp(0.0, a.tas, 1.0, b.tas, t),
        gph: lerp(0.0, a.gph, 1.0, b.gph, t),
    }
}

/// Bilinear in (PA × ΔISA) at one RPM band (one row per PA).
fn interpolate_triple_along_disa(row: &[f64], disa: f64) -> (Triple, bool, bool) {
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
        return (t_m20, false, false);
    }
    if abs_ex(disa, 0.0) {
        return (isa, false, false);
    }
    if abs_ex(disa, 10.0) {
        return (t_p10, false, false);
    }

    if disa < -20.0 {
        let t = (disa + 20.0) / 20.0;
        (lerp_triple(t_m20, isa, t), false, true)
    } else if disa < 0.0 {
        let t = (disa + 20.0) / 20.0;
        (lerp_triple(t_m20, isa, t), true, false)
    } else if disa < 10.0 {
        let t = disa / 10.0;
        (lerp_triple(isa, t_p10, t), true, false)
    } else {
        let t = disa / 10.0;
        (lerp_triple(isa, t_p10, t), false, true)
    }
}

/// Interpolate in PA between rows at fixed RPM, each row first resolved in ΔISA.
fn surface_at_fixed_rpm(
    rows: &[Vec<f64>],
    rpm: f64,
    pa: f64,
    disa: f64,
) -> Option<(Triple, LookupFlags)> {
    let at_rpm = rows_at_rpm(rows, rpm);
    if at_rpm.is_empty() {
        return None;
    }

    let mut flags = LookupFlags::default();

    if at_rpm.len() == 1 {
        let only_pa = at_rpm[0][0];
        let (tri, di, de) = interpolate_triple_along_disa(at_rpm[0], disa);
        flags.disa_interp |= di;
        flags.disa_extrap |= de;
        if (pa - only_pa).abs() > PA_MATCH_TOL {
            flags.pa_extrap = true;
        }
        return Some((tri, flags));
    }

    let pas: Vec<f64> = at_rpm.iter().map(|r| r[0]).collect();
    let pa_min = pas[0];
    let pa_max = *pas.last().unwrap();

    if pa < pa_min - PA_MATCH_TOL {
        flags.pa_extrap = true;
        if at_rpm.len() >= 2 {
            let r0 = at_rpm[0];
            let r1 = at_rpm[1];
            let p0 = r0[0];
            let p1 = r1[0];
            let t = (pa - p0) / (p1 - p0);
            let (t0, di0, de0) = interpolate_triple_along_disa(r0, disa);
            let (t1, di1, de1) = interpolate_triple_along_disa(r1, disa);
            flags.disa_interp |= di0 || di1;
            flags.disa_extrap |= de0 || de1;
            return Some((lerp_triple(t0, t1, t), flags));
        }
        let row = at_rpm[0];
        let (tri, di, de) = interpolate_triple_along_disa(row, disa);
        flags.disa_interp |= di;
        flags.disa_extrap |= de;
        return Some((tri, flags));
    }
    if pa > pa_max + PA_MATCH_TOL {
        flags.pa_extrap = true;
        let n = at_rpm.len();
        if n >= 2 {
            let r0 = at_rpm[n - 2];
            let r1 = at_rpm[n - 1];
            let p0 = r0[0];
            let p1 = r1[0];
            let t = (pa - p0) / (p1 - p0);
            let (t0, di0, de0) = interpolate_triple_along_disa(r0, disa);
            let (t1, di1, de1) = interpolate_triple_along_disa(r1, disa);
            flags.disa_interp |= di0 || di1;
            flags.disa_extrap |= de0 || de1;
            return Some((lerp_triple(t0, t1, t), flags));
        }
        let row = at_rpm[n - 1];
        let (tri, di, de) = interpolate_triple_along_disa(row, disa);
        flags.disa_interp |= di;
        flags.disa_extrap |= de;
        return Some((tri, flags));
    }

    let mut j = 0usize;
    while j + 1 < at_rpm.len() && at_rpm[j + 1][0] < pa - PA_MATCH_TOL {
        j += 1;
    }
    let r0 = at_rpm[j];
    let r1 = at_rpm[(j + 1).min(at_rpm.len() - 1)];
    let p0 = r0[0];
    let p1 = r1[0];

    if (p1 - p0).abs() < PA_MATCH_TOL {
        let (tri, di, de) = interpolate_triple_along_disa(r0, disa);
        flags.disa_interp |= di;
        flags.disa_extrap |= de;
        return Some((tri, flags));
    }

    let t = (pa - p0) / (p1 - p0);
    let (t0, di0, de0) = interpolate_triple_along_disa(r0, disa);
    let (t1, di1, de1) = interpolate_triple_along_disa(r1, disa);
    flags.disa_interp |= di0 || di1;
    flags.disa_extrap |= de0 || de1;

    let interior_pa = t > DISA_NODE_EPS && t < 1.0 - DISA_NODE_EPS;
    if interior_pa {
        flags.pa_interp = true;
    }

    Some((lerp_triple(t0, t1, t), flags))
}

fn merge_flags(a: &mut LookupFlags, b: LookupFlags) {
    a.rpm_interp |= b.rpm_interp;
    a.rpm_extrap |= b.rpm_extrap;
    a.pa_interp |= b.pa_interp;
    a.pa_extrap |= b.pa_extrap;
    a.disa_interp |= b.disa_interp;
    a.disa_extrap |= b.disa_extrap;
    a.rpm_band_gap |= b.rpm_band_gap;
    a.bhp_extrap |= b.bhp_extrap;
}

fn percent_bhp_at_rpm(
    rows: &[Vec<f64>],
    pa: f64,
    disa: f64,
    rpm: f64,
) -> Result<(f64, LookupFlags), CruiseError> {
    let (tri, _, f) = lookup_table3(rows, pa, rpm, disa)?;
    Ok((tri.bhp, f))
}

fn bhp_between(b0: f64, b1: f64, target: f64) -> bool {
    let lo = b0.min(b1) - 1e-6;
    let hi = b0.max(b1) + 1e-6;
    target >= lo && target <= hi
}

fn push_solution(
    best: &mut Option<(f64, LookupFlags)>,
    rpm: f64,
    flags: LookupFlags,
    hint_rpm: f64,
) {
    let replace = |cur: f64, cand: f64| {
        let d_cur = (cur - hint_rpm).abs();
        let d_cand = (cand - hint_rpm).abs();
        if (d_cand - d_cur).abs() < 1e-6 {
            cand > cur
        } else {
            d_cand < d_cur
        }
    };
    if best.as_ref().is_none_or(|(r, _)| replace(*r, rpm)) {
        *best = Some((rpm, flags));
    }
}

/// Invert %BHP → RPM by inverting the same piecewise-linear RPM segments as `lookup_table3`.
fn solve_rpm_for_percent_bhp(
    rows: &[Vec<f64>],
    pa: f64,
    disa: f64,
    target_bhp: f64,
    hint_rpm: f64,
) -> Result<(f64, LookupFlags), CruiseError> {
    let levels = unique_sorted_rpms(rows);
    if levels.is_empty() {
        return Err(CruiseError::NotFound);
    }

    let mut candidates: Option<(f64, LookupFlags)> = None;
    for &rpm in &levels {
        let (bhp, f) = percent_bhp_at_rpm(rows, pa, disa, rpm)?;
        if (bhp - target_bhp).abs() < 1e-4 {
            push_solution(&mut candidates, rpm, f, hint_rpm);
        }
    }
    if let Some(hit) = candidates {
        return Ok(hit);
    }

    let mut best: Option<(f64, LookupFlags)> = None;
    for w in levels.windows(2) {
        let r0 = w[0];
        let r1 = w[1];
        let (b0, f0) = percent_bhp_at_rpm(rows, pa, disa, r0)?;
        let (b1, f1) = percent_bhp_at_rpm(rows, pa, disa, r1)?;
        if !bhp_between(b0, b1, target_bhp) {
            continue;
        }
        let denom = b1 - b0;
        let t = if denom.abs() < 1e-9 {
            0.5
        } else {
            (target_bhp - b0) / denom
        };
        let rpm = lerp(0.0, r0, 1.0, r1, t);
        let mut merged = LookupFlags::default();
        merge_flags(&mut merged, f0);
        merge_flags(&mut merged, f1);
        merged.rpm_interp = (r1 - r0).abs() > RPM_MATCH_TOL;
        push_solution(&mut best, rpm, merged, hint_rpm);
    }
    if let Some(hit) = best {
        return Ok(hit);
    }

    let r_lo = levels[0];
    let r_hi = *levels.last().unwrap();
    let (b_lo, f_lo) = percent_bhp_at_rpm(rows, pa, disa, r_lo)?;
    let (b_hi, f_hi) = percent_bhp_at_rpm(rows, pa, disa, r_hi)?;
    let band_min = b_lo.min(b_hi);

    let inv_flags = LookupFlags {
        bhp_extrap: true,
        rpm_extrap: true,
        ..Default::default()
    };

    if levels.len() < 2 {
        let mut merged = f_lo;
        merge_flags(&mut merged, inv_flags);
        return Ok((r_lo, merged));
    }

    let below_band = target_bhp < band_min;
    let (rpm_a, rpm_b, b_a, b_b, fa, fb) = if below_band {
        if b_lo <= b_hi {
            let r1 = levels[1];
            let (b1, f1) = percent_bhp_at_rpm(rows, pa, disa, r1)?;
            (r_lo, r1, b_lo, b1, f_lo, f1)
        } else {
            let n = levels.len();
            let ra = levels[n - 2];
            let (ba, fa) = percent_bhp_at_rpm(rows, pa, disa, ra)?;
            (ra, r_hi, ba, b_hi, fa, f_hi)
        }
    } else if b_lo <= b_hi {
        let n = levels.len();
        let ra = levels[n - 2];
        let (ba, fa) = percent_bhp_at_rpm(rows, pa, disa, ra)?;
        (ra, r_hi, ba, b_hi, fa, f_hi)
    } else {
        let r1 = levels[1];
        let (b1, f1) = percent_bhp_at_rpm(rows, pa, disa, r1)?;
        (r_lo, r1, b_lo, b1, f_lo, f1)
    };

    let denom = b_b - b_a;
    let t = if denom.abs() < 1e-9 {
        if below_band {
            -1.0
        } else {
            2.0
        }
    } else {
        (target_bhp - b_a) / denom
    };
    let solved = lerp(0.0, rpm_a, 1.0, rpm_b, t);
    let mut merged = LookupFlags::default();
    merge_flags(&mut merged, fa);
    merge_flags(&mut merged, fb);
    merged.rpm_interp = true;
    merge_flags(&mut merged, inv_flags);
    Ok((solved, merged))
}

fn surface_with_rpm_gap_flag(
    rows: &[Vec<f64>],
    rpm: f64,
    pa: f64,
    disa: f64,
) -> Option<(Triple, LookupFlags)> {
    let (tri, mut f) = surface_at_fixed_rpm(rows, rpm, pa, disa)?;
    if let Some(pas) = pas_for_rpm(rows, rpm) {
        let pa_min = *pas.first().unwrap();
        let pa_max = *pas.last().unwrap();
        if pa < pa_min - PA_MATCH_TOL || pa > pa_max + PA_MATCH_TOL {
            f.rpm_band_gap = true;
        }
    }
    Some((tri, f))
}

fn lookup_table3(
    rows: &[Vec<f64>],
    pa: f64,
    rpm: f64,
    disa: f64,
) -> Result<(Triple, AfmLookupTag, LookupFlags), CruiseError> {
    let levels = unique_sorted_rpms(rows);
    if levels.is_empty() {
        return Err(CruiseError::NotFound);
    }

    let nearest_level = |r: f64| -> Option<f64> {
        levels
            .iter()
            .copied()
            .min_by(|a, b| (a - r).abs().partial_cmp(&(b - r).abs()).unwrap())
    };

    let exact_rpm_level = levels
        .iter()
        .find(|l| (*l - rpm).abs() < RPM_MATCH_TOL)
        .copied();

    if let Some(rpm_tab) = exact_rpm_level {
        let (tri, f) =
            surface_with_rpm_gap_flag(rows, rpm_tab, pa, disa).ok_or(CruiseError::NotFound)?;
        let tag = resolve_tag(pa, rpm_tab, disa, rows, &f);
        return Ok((tri, tag, f));
    }

    let below = levels
        .iter()
        .filter(|l| **l < rpm)
        .max_by(|a, b| a.partial_cmp(b).unwrap());
    let above = levels
        .iter()
        .filter(|l| **l > rpm)
        .min_by(|a, b| a.partial_cmp(b).unwrap());

    let (r0, r1, t_rpm, rpm_extrap) = match (below, above) {
        (Some(&b), Some(&a)) => (b, a, (rpm - b) / (a - b), false),
        (None, Some(&_a)) => {
            if levels.len() < 2 {
                return Err(CruiseError::NotFound);
            }
            let b = levels[0];
            let a = levels[1];
            (b, a, (rpm - b) / (a - b), true)
        }
        (Some(&_b), None) => {
            let n = levels.len();
            if n < 2 {
                return Err(CruiseError::NotFound);
            }
            let b = levels[n - 2];
            let a = levels[n - 1];
            (b, a, (rpm - b) / (a - b), true)
        }
        _ => return Err(CruiseError::NotFound),
    };

    let s0 = surface_with_rpm_gap_flag(rows, r0, pa, disa).ok_or(CruiseError::NotFound)?;
    let s1 = surface_with_rpm_gap_flag(rows, r1, pa, disa).ok_or(CruiseError::NotFound)?;

    let mut f = LookupFlags::default();
    merge_flags(&mut f, s0.1);
    merge_flags(&mut f, s1.1);
    f.rpm_interp = (r1 - r0).abs() > RPM_MATCH_TOL;
    f.rpm_extrap |= rpm_extrap;

    let tri = lerp_triple(s0.0, s1.0, t_rpm);

    if let Some(near) = nearest_level(rpm) {
        if pas_for_rpm(rows, near).is_none_or(|pas| {
            let mn = *pas.first().unwrap();
            let mx = *pas.last().unwrap();
            pa < mn - PA_MATCH_TOL || pa > mx + PA_MATCH_TOL
        }) {
            f.rpm_band_gap = true;
        }
    }

    let tag = resolve_tag(pa, rpm, disa, rows, &f);
    Ok((tri, tag, f))
}

fn resolve_tag(pa: f64, rpm: f64, disa: f64, rows: &[Vec<f64>], f: &LookupFlags) -> AfmLookupTag {
    if f.rpm_band_gap || f.rpm_extrap || f.disa_extrap || f.pa_extrap || f.bhp_extrap {
        return AfmLookupTag::Extrapolated;
    }

    let exact_disa = (disa + 20.0).abs() < DISA_NODE_EPS
        || disa.abs() < DISA_NODE_EPS
        || (disa - 10.0).abs() < DISA_NODE_EPS;

    let at_rpm: Vec<_> = rows_at_rpm(rows, rpm);
    let exact_pa = at_rpm.iter().any(|r| (r[0] - pa).abs() < PA_MATCH_TOL);

    let exact = exact_pa
        && exact_disa
        && at_rpm
            .iter()
            .any(|r| (r[0] - pa).abs() < PA_MATCH_TOL && (r[1] - rpm).abs() < RPM_MATCH_TOL)
        && !f.rpm_interp
        && !f.pa_interp
        && !f.disa_interp;

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
    let (tri, _, mut flags) = lookup_table3(rows, pa, rpm, disa)?;
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
        warnings,
    })
}

pub fn compute_cruise(
    data: &PerformanceData,
    inputs: &FlightInputs,
) -> Result<CruiseResult, CruiseError> {
    let pa = inputs.cruise_pressure_alt_ft;
    let rows = &data.inner().cruise_table.value.rows;
    let disa = disa_c(inputs.cruise_oat_c, pa);
    let mut warnings = vec![CalculationWarning::code_only("TABLE3_GPH")];

    let (rpm, extra_flags) = match inputs.cruise_solve_mode {
        CruiseSolveMode::SetRpm => (inputs.cruise_rpm, LookupFlags::default()),
        CruiseSolveMode::SetPercentBhp => {
            let (solved, inv) = solve_rpm_for_percent_bhp(
                rows,
                pa,
                disa,
                inputs.cruise_percent_bhp,
                inputs.cruise_rpm,
            )?;
            if inv.bhp_extrap {
                warnings.push(CalculationWarning::with_args(
                    "CRUISE_BHP_OUT_OF_RANGE",
                    vec![format!("{:.2}", inputs.cruise_percent_bhp)],
                ));
            }
            (solved, inv)
        }
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

/// RPM row to highlight in the full Table 3 view (solved RPM in %BHP mode).
pub fn effective_cruise_highlight_rpm(data: &PerformanceData, inputs: &FlightInputs) -> f64 {
    match inputs.cruise_solve_mode {
        CruiseSolveMode::SetRpm => inputs.cruise_rpm,
        CruiseSolveMode::SetPercentBhp => compute_cruise(data, inputs)
            .ok()
            .and_then(|r| r.rpm)
            .unwrap_or(inputs.cruise_rpm),
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
            (r.percent_bhp.unwrap() - 56.25).abs() < 0.01,
            "bhp={}",
            r.percent_bhp.unwrap()
        );
        assert!((r.tas_chart_kt.unwrap() - 116.0).abs() < 0.01);
        assert!((r.fuel_flow_us_gal_h - 5.525).abs() < 0.01);
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
    fn round_trip_rpm_bhp_rpm_grid() {
        let data = PerformanceData::production();
        let rows = &data.inner().cruise_table.value.rows;
        let pas: Vec<f64> = rows.iter().filter(|r| row_valid(r)).map(|r| r[0]).collect();
        let pas: Vec<f64> = {
            let mut v = pas;
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v.dedup_by(|a, b| (*a - *b).abs() < PA_MATCH_TOL);
            v
        };
        let rpms = unique_sorted_rpms(rows);
        for &pa in &pas {
            for pa_mid in [pa - 250.0, pa, pa + 250.0] {
                if pa_mid < 0.0 {
                    continue;
                }
                let disa = 0.0;
                let oat = isa_temp_c_at_pa(pa_mid);
                for &rpm in &rpms {
                    let forward = FlightInputs {
                        cruise_pressure_alt_ft: pa_mid,
                        cruise_rpm: rpm,
                        cruise_oat_c: oat,
                        cruise_solve_mode: CruiseSolveMode::SetRpm,
                        wheel_fairings_installed: true,
                        ..FlightInputs::default()
                    };
                    let r_fwd = compute_cruise(&data, &forward).expect("forward");
                    let bhp = r_fwd.percent_bhp.unwrap();
                    let rpms_lv = unique_sorted_rpms(rows);
                    let b_lo = percent_bhp_at_rpm(rows, pa_mid, disa, rpms_lv[0])
                        .expect("bhp lo")
                        .0;
                    let b_hi = percent_bhp_at_rpm(rows, pa_mid, disa, *rpms_lv.last().unwrap())
                        .expect("bhp hi")
                        .0;
                    let band_min = b_lo.min(b_hi);
                    let band_max = b_lo.max(b_hi);
                    if bhp < band_min - 0.01 || bhp > band_max + 0.01 {
                        continue;
                    }
                    let inverse = FlightInputs {
                        cruise_pressure_alt_ft: pa_mid,
                        cruise_oat_c: oat,
                        cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
                        cruise_percent_bhp: bhp,
                        cruise_rpm: rpm,
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
        assert!((r.rpm.unwrap() - 2600.0).abs() < 0.5);
        assert!((r.percent_bhp.unwrap() - 56.25).abs() < 0.01);
        assert!((r.tas_chart_kt.unwrap() - 116.0).abs() < 0.01);
        assert!((r.fuel_flow_us_gal_h - 5.525).abs() < 0.01);
        assert_eq!(r.afm_lookup_tag, AfmLookupTag::Interpolated);
    }

    #[test]
    fn unreachable_bhp_not_clamped_and_tag_extrapolated() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 5.0,
            wheel_fairings_installed: true,
            ..FlightInputs::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        assert_eq!(r.afm_lookup_tag, AfmLookupTag::Extrapolated);
        let rows = &data.inner().cruise_table.value.rows;
        let disa = disa_c(inputs.cruise_oat_c, inputs.cruise_pressure_alt_ft);
        let levels = unique_sorted_rpms(rows);
        let (b_lo, _) =
            percent_bhp_at_rpm(rows, inputs.cruise_pressure_alt_ft, disa, levels[0]).unwrap();
        let (b_hi, _) = percent_bhp_at_rpm(
            rows,
            inputs.cruise_pressure_alt_ft,
            disa,
            *levels.last().unwrap(),
        )
        .unwrap();
        let band_min = b_lo.min(b_hi);
        assert!(
            5.0 < band_min - 0.5,
            "test assumes 5% BHP below band min {band_min}"
        );
        let solved = r.rpm.unwrap();
        let anchor = if b_lo <= b_hi {
            levels[0]
        } else {
            *levels.last().unwrap()
        };
        assert!(
            (solved - anchor).abs() > 50.0,
            "expected extrapolated RPM away from anchor {anchor}, got {solved}"
        );
        assert!(r
            .warnings
            .iter()
            .any(|w| w.code == "CRUISE_BHP_OUT_OF_RANGE"));
        assert!(r.tas_kt.is_finite() && r.tas_kt > 0.0, "tas={}", r.tas_kt);
        assert!(
            r.fuel_flow_us_gal_h.is_finite() && r.fuel_flow_us_gal_h > 0.0,
            "gph={}",
            r.fuel_flow_us_gal_h
        );
        assert!(r.percent_bhp.unwrap().is_finite());
    }

    #[test]
    fn lookup_high_rpm_extrapolation_finite_at_6500ft() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 6500.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_rpm: 3000.0,
            cruise_solve_mode: CruiseSolveMode::SetRpm,
            wheel_fairings_installed: true,
            ..FlightInputs::default()
        };
        let r = compute_cruise(&data, &inputs).unwrap();
        assert!(r.tas_chart_kt.unwrap().is_finite());
        assert!(r.fuel_flow_us_gal_h.is_finite());
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
