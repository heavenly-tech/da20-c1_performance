use crate::chart_bounds::limits;
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
    rpm >= lo - RPM_MATCH_TOL && rpm <= hi + RPM_MATCH_TOL
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

fn lookup_table3(
    rows: &[Vec<f64>],
    pa: f64,
    rpm: f64,
    disa: f64,
) -> Result<(Triple, AfmLookupTag, LookupFlags), CruiseError> {
    if disa < limits::TABLE3_DISA_C.0 - DISA_NODE_EPS
        || disa > limits::TABLE3_DISA_C.1 + DISA_NODE_EPS
    {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 ΔISA",
            min: limits::TABLE3_DISA_C.0,
            max: limits::TABLE3_DISA_C.1,
        });
    }
    if pa < limits::TABLE3_PA_FT.0 - PA_MATCH_TOL || pa > limits::TABLE3_PA_FT.1 + PA_MATCH_TOL {
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
    if rpm < rpm_lo - RPM_MATCH_TOL || rpm > TABLE3_RPM_MAX + RPM_MATCH_TOL {
        return Err(CruiseError::OutOfChart {
            axis: "Table 3 RPM",
            min: rpm_lo,
            max: TABLE3_RPM_MAX,
        });
    }

    let s = normalized_s(rpm, rpm_lo);

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
        let (tri, f) =
            lookup_at_tabulated_pa(rows, p0, rpm_row, disa).ok_or(CruiseError::OutOfChart {
                axis: "Table 3 RPM",
                min: rpm_lo,
                max: TABLE3_RPM_MAX,
            })?;
        let tag = resolve_tag(pa, rpm, disa, rows, &f);
        return Ok((tri, tag, f));
    }

    let t = (pa - p0) / (p1 - p0);
    let (tri, f) = blend_from(p0, p1, t)?;
    let tag = resolve_tag(pa, rpm, disa, rows, &f);
    Ok((tri, tag, f))
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

    let mut inv_flags = LookupFlags::default();
    merge_flags(&mut inv_flags, f_lo);
    merge_flags(&mut inv_flags, f_hi);

    let increasing = b_hi >= b_lo - 1e-9;
    let mut lo = rpm_lo;
    let mut hi = rpm_hi;
    if hint_rpm >= rpm_lo - RPM_MATCH_TOL
        && hint_rpm <= rpm_hi + RPM_MATCH_TOL
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
                while (rpm_i as f64) <= TABLE3_RPM_MAX + RPM_MATCH_TOL {
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
