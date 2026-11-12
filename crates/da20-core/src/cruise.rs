use crate::data::PerformanceData;
use crate::interpolation::lerp;
use crate::provenance::provenance_for;
use crate::types::{
    cruise_fairing_multiplier, AfmLookupTag, CalculationWarning, CruiseResult, CruiseSolveMode,
    FlightInputs,
};

pub const CRUISE_TABLE3_GPH_NOTE: &str = "Table 3 GPH values are transcribed as printed in AFM Rev 29 (includes known oddities, e.g. 2000 ft / 2700 RPM ISA 6.8 GPH vs −20 °C 7.7 GPH; 4000 ft / 2800 RPM +10 °C 6.8 GPH).";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CruiseError {
    #[error("cruise row not found")]
    NotFound,
    #[error("requested %BHP not achievable within Table 3 RPM coverage at this PA/ΔISA")]
    BhpNotAchievable,
}

/// RPM search slack beyond each bracketing PA row's own tabulated RPM ladder.
const BHP_INVERSE_RPM_SLACK: f64 = 100.0;

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
    /// True when target %BHP lies outside the band covered by tabulated RPM rows (inverse solve).
    bhp_extrap: bool,
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

/// Interpolate in RPM on one PA row's own tabulated RPM ladder (ΔISA per node first).
fn lookup_on_pa_ladder(ladder: &[&Vec<f64>], rpm: f64, disa: f64) -> Option<(Triple, LookupFlags)> {
    if ladder.is_empty() {
        return None;
    }

    if let Some(row) = ladder.iter().find(|r| (r[1] - rpm).abs() < RPM_MATCH_TOL) {
        let (tri, di, de) = interpolate_triple_along_disa(row, disa);
        let mut flags = LookupFlags::default();
        flags.disa_interp |= di;
        flags.disa_extrap |= de;
        return Some((tri, flags));
    }

    if ladder.len() == 1 {
        let row = ladder[0];
        let (tri, di, de) = interpolate_triple_along_disa(row, disa);
        let mut flags = LookupFlags::default();
        flags.disa_interp |= di;
        flags.disa_extrap |= de;
        flags.rpm_extrap = true;
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

    let (i0, i1, t_rpm, rpm_extrap) = match (below_idx, above_idx) {
        (Some(b), Some(a)) => (
            b,
            a,
            (rpm - ladder[b][1]) / (ladder[a][1] - ladder[b][1]),
            false,
        ),
        (None, Some(_)) => {
            if ladder.len() < 2 {
                return None;
            }
            (
                0,
                1,
                (rpm - ladder[0][1]) / (ladder[1][1] - ladder[0][1]),
                true,
            )
        }
        (Some(_b), None) => {
            let n = ladder.len();
            (
                n - 2,
                n - 1,
                (rpm - ladder[n - 2][1]) / (ladder[n - 1][1] - ladder[n - 2][1]),
                true,
            )
        }
        _ => return None,
    };

    let r0 = ladder[i0];
    let r1 = ladder[i1];

    let (t0, di0, de0) = interpolate_triple_along_disa(r0, disa);
    let (t1, di1, de1) = interpolate_triple_along_disa(r1, disa);
    let mut flags = LookupFlags::default();
    flags.disa_interp |= di0 || di1;
    flags.disa_extrap |= de0 || de1;
    flags.rpm_interp = (r1[1] - r0[1]).abs() > RPM_MATCH_TOL;
    flags.rpm_extrap |= rpm_extrap;
    Some((lerp_triple(t0, t1, t_rpm), flags))
}

fn lookup_at_tabulated_pa(
    rows: &[Vec<f64>],
    pa_tab: f64,
    rpm: f64,
    disa: f64,
) -> Option<(Triple, LookupFlags)> {
    lookup_on_pa_ladder(&rows_at_pa(rows, pa_tab), rpm, disa)
}

fn tabulated_rpm_bounds_for_pas(rows: &[Vec<f64>], pas: &[f64]) -> Option<(f64, f64)> {
    let mut rpm_min = f64::INFINITY;
    let mut rpm_max = f64::NEG_INFINITY;
    for &pa in pas {
        let ladder = rows_at_pa(rows, pa);
        if ladder.is_empty() {
            continue;
        }
        rpm_min = rpm_min.min(ladder[0][1]);
        rpm_max = rpm_max.max(ladder[ladder.len() - 1][1]);
    }
    if rpm_min.is_finite() && rpm_max.is_finite() {
        Some((rpm_min, rpm_max))
    } else {
        None
    }
}

fn lookup_table3(
    rows: &[Vec<f64>],
    pa: f64,
    rpm: f64,
    disa: f64,
) -> Result<(Triple, AfmLookupTag, LookupFlags), CruiseError> {
    let pas = unique_sorted_pas(rows);
    if pas.is_empty() {
        return Err(CruiseError::NotFound);
    }

    let pa_min = pas[0];
    let pa_max = *pas.last().unwrap();

    let blend_from = |p0: f64, p1: f64, t: f64| -> Result<(Triple, LookupFlags), CruiseError> {
        let s0 = lookup_at_tabulated_pa(rows, p0, rpm, disa).ok_or(CruiseError::NotFound)?;
        let s1 = lookup_at_tabulated_pa(rows, p1, rpm, disa).ok_or(CruiseError::NotFound)?;
        let mut f = LookupFlags::default();
        merge_flags(&mut f, s0.1);
        merge_flags(&mut f, s1.1);
        if t > DISA_NODE_EPS && t < 1.0 - DISA_NODE_EPS {
            f.pa_interp = true;
        }
        Ok((lerp_triple(s0.0, s1.0, t), f))
    };

    if pa < pa_min - PA_MATCH_TOL {
        if pas.len() < 2 {
            return Err(CruiseError::NotFound);
        }
        let (tri, mut f) = blend_from(pas[0], pas[1], (pa - pas[0]) / (pas[1] - pas[0]))?;
        f.pa_extrap = true;
        let tag = resolve_tag(pa, rpm, disa, rows, &f);
        return Ok((tri, tag, f));
    }
    if pa > pa_max + PA_MATCH_TOL {
        if pas.len() < 2 {
            return Err(CruiseError::NotFound);
        }
        let n = pas.len();
        let (tri, mut f) = blend_from(
            pas[n - 2],
            pas[n - 1],
            (pa - pas[n - 2]) / (pas[n - 1] - pas[n - 2]),
        )?;
        f.pa_extrap = true;
        let tag = resolve_tag(pa, rpm, disa, rows, &f);
        return Ok((tri, tag, f));
    }

    if let Some(&exact) = pas.iter().find(|p| (*p - pa).abs() < PA_MATCH_TOL) {
        let (tri, f) =
            lookup_at_tabulated_pa(rows, exact, rpm, disa).ok_or(CruiseError::NotFound)?;
        let tag = resolve_tag(pa, rpm, disa, rows, &f);
        return Ok((tri, tag, f));
    }

    let mut j = 0usize;
    while j + 1 < pas.len() && pas[j + 1] < pa - PA_MATCH_TOL {
        j += 1;
    }
    let p0 = pas[j];
    let p1 = pas[(j + 1).min(pas.len() - 1)];
    let t = (pa - p0) / (p1 - p0);
    let (tri, f) = blend_from(p0, p1, t)?;
    let tag = resolve_tag(pa, rpm, disa, rows, &f);
    Ok((tri, tag, f))
}

fn merge_flags(a: &mut LookupFlags, b: LookupFlags) {
    a.rpm_interp |= b.rpm_interp;
    a.rpm_extrap |= b.rpm_extrap;
    a.pa_interp |= b.pa_interp;
    a.pa_extrap |= b.pa_extrap;
    a.disa_interp |= b.disa_interp;
    a.disa_extrap |= b.disa_extrap;
    a.bhp_extrap |= b.bhp_extrap;
}

fn pa_bracket_for_lookup(rows: &[Vec<f64>], pa: f64) -> Vec<f64> {
    let pas = unique_sorted_pas(rows);
    if pas.is_empty() {
        return Vec::new();
    }
    if pa < pas[0] - PA_MATCH_TOL {
        return vec![pas[0], pas.get(1).copied().unwrap_or(pas[0])];
    }
    if pa > *pas.last().unwrap() + PA_MATCH_TOL {
        let n = pas.len();
        return vec![pas[n - 2], pas[n - 1]];
    }
    if let Some(&exact) = pas.iter().find(|p| (*p - pa).abs() < PA_MATCH_TOL) {
        return vec![exact];
    }
    let mut j = 0usize;
    while j + 1 < pas.len() && pas[j + 1] < pa - PA_MATCH_TOL {
        j += 1;
    }
    vec![pas[j], pas[(j + 1).min(pas.len() - 1)]]
}

#[cfg(test)]
fn rpm_in_inverse_search_band(rows: &[Vec<f64>], pa: f64, rpm: f64) -> bool {
    let bracket = pa_bracket_for_lookup(rows, pa);
    let Some((lo, hi)) = tabulated_rpm_bounds_for_pas(rows, &bracket) else {
        return false;
    };
    rpm >= lo - BHP_INVERSE_RPM_SLACK - 1e-6 && rpm <= hi + BHP_INVERSE_RPM_SLACK + 1e-6
}

/// Bisection inverse on the monotone forward %BHP(RPM) surface (±100 RPM slack on bracketing PA ladders).
fn solve_rpm_for_percent_bhp(
    rows: &[Vec<f64>],
    pa: f64,
    disa: f64,
    target_bhp: f64,
    hint_rpm: f64,
) -> Result<(f64, LookupFlags), CruiseError> {
    let bracket_pas = pa_bracket_for_lookup(rows, pa);
    let Some((rpm_tab_min, rpm_tab_max)) = tabulated_rpm_bounds_for_pas(rows, &bracket_pas) else {
        return Err(CruiseError::NotFound);
    };
    let rpm_lo = rpm_tab_min - BHP_INVERSE_RPM_SLACK;
    let rpm_hi = rpm_tab_max + BHP_INVERSE_RPM_SLACK;

    let sample = |rpm: f64| -> Result<(f64, LookupFlags), CruiseError> {
        let (tri, _, f) = lookup_table3(rows, pa, rpm, disa)?;
        Ok((tri.bhp, f))
    };

    let (b_lo, f_lo) = sample(rpm_lo)?;
    let (b_hi, f_hi) = sample(rpm_hi)?;
    let band_min = b_lo.min(b_hi);
    let band_max = b_lo.max(b_hi);

    if target_bhp < band_min - 1e-4 || target_bhp > band_max + 1e-4 {
        return Err(CruiseError::BhpNotAchievable);
    }

    let (b_at_min, _) = sample(rpm_tab_min)?;
    let (b_at_max, _) = sample(rpm_tab_max)?;
    let in_tab_band =
        target_bhp >= b_at_min.min(b_at_max) - 1e-4 && target_bhp <= b_at_min.max(b_at_max) + 1e-4;

    let mut inv_flags = LookupFlags::default();
    if !in_tab_band {
        inv_flags.bhp_extrap = true;
        inv_flags.rpm_extrap = true;
    }

    let increasing = b_hi >= b_lo - 1e-9;
    let mut lo = rpm_lo;
    let mut hi = rpm_hi;
    if hint_rpm >= rpm_lo && hint_rpm <= rpm_hi {
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
    let (_, f_end) = sample(rpm)?;
    merge_flags(&mut inv_flags, f_end);
    merge_flags(&mut inv_flags, f_lo);
    merge_flags(&mut inv_flags, f_hi);
    Ok((rpm, inv_flags))
}

fn resolve_tag(pa: f64, rpm: f64, disa: f64, rows: &[Vec<f64>], f: &LookupFlags) -> AfmLookupTag {
    if f.rpm_extrap || f.disa_extrap || f.pa_extrap || f.bhp_extrap {
        return AfmLookupTag::Extrapolated;
    }

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
        CruiseSolveMode::SetPercentBhp => match solve_rpm_for_percent_bhp(
            rows,
            pa,
            disa,
            inputs.cruise_percent_bhp,
            inputs.cruise_rpm,
        ) {
            Ok((solved, inv)) => {
                if inv.bhp_extrap {
                    warnings.push(CalculationWarning::with_args(
                        "CRUISE_BHP_OUT_OF_RANGE",
                        vec![format!("{:.2}", inputs.cruise_percent_bhp)],
                    ));
                }
                (solved, inv)
            }
            Err(CruiseError::BhpNotAchievable) => {
                warnings.push(CalculationWarning::with_args(
                    "CRUISE_BHP_NOT_ACHIEVABLE",
                    vec![format!("{:.2}", inputs.cruise_percent_bhp)],
                ));
                return Err(CruiseError::BhpNotAchievable);
            }
            Err(e) => return Err(e),
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
                    if !rpm_in_inverse_search_band(rows, pa_mid, rpm) {
                        continue;
                    }
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
        let disas = [-30.0, -20.0, 0.0, 10.0, 20.0];
        for pa in (0..=12_000).step_by(250) {
            let pa = pa as f64;
            for &disa in &disas {
                let oat = isa_temp_c_at_pa(pa) + disa;
                let mut prev_bhp = f64::NEG_INFINITY;
                let mut prev_tas = f64::NEG_INFINITY;
                let mut prev_gph = f64::NEG_INFINITY;
                for rpm in (2300..=2800).step_by(10) {
                    let rpm = rpm as f64;
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
                }
            }
        }
    }

    #[test]
    fn monotonic_6500_isa_2400_vs_2450() {
        let data = PerformanceData::production();
        let oat = isa_temp_c_at_pa(6500.0);
        let mut prev = 0.0f64;
        for rpm in [2400.0, 2450.0, 2600.0] {
            let inputs = FlightInputs {
                cruise_pressure_alt_ft: 6500.0,
                cruise_rpm: rpm,
                cruise_oat_c: oat,
                wheel_fairings_installed: true,
                ..FlightInputs::default()
            };
            let r = compute_cruise(&data, &inputs).unwrap();
            let bhp = r.percent_bhp.unwrap();
            assert!(bhp >= prev - 1e-6, "rpm={rpm} bhp={bhp} prev={prev}");
            prev = bhp;
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
