use crate::chart_bounds::CHART_EDGE_TOL;
use crate::interpolation::lerp;
use crate::mt175r::data::{CurveSample, Mt175rData};

/// Printed %BHP curves on AFMS Fig 2 / Fig 3.
pub const PRINTED_PCT_CURVES: [i32; 4] = [45, 55, 65, 75];

pub const FIG2_PCT75_DA_MAX_FT: f64 = 8270.0;
pub const FIG3_PCT75_DA_MAX_FT: f64 = 8500.0;
pub const MT_MAX_PROP_RPM: f64 = 2800.0;
pub const MT_PCT_BHP_MIN: f64 = 45.0;
pub const MT_PCT_BHP_MAX: f64 = 75.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MtCurveBracket {
    pub pct_bhp: i32,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fig2RpmLookup {
    pub rpm: f64,
    pub percent_bhp: f64,
    pub brackets: Vec<MtCurveBracket>,
    pub pct_interp: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fig2PctLookup {
    pub percent_bhp: f64,
    pub brackets: Vec<MtCurveBracket>,
    pub pct_interp: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fig3TasLookup {
    pub tas_kt: f64,
    pub brackets: Vec<MtCurveBracket>,
    pub pct_interp: bool,
    pub full_throttle_cap_kt: Option<f64>,
    pub capped_at_full_throttle: bool,
}

fn fig2_curve_valid_at_da(pct_bhp: i32, da_ft: f64) -> bool {
    if pct_bhp == 75 {
        (-CHART_EDGE_TOL..=FIG2_PCT75_DA_MAX_FT + CHART_EDGE_TOL).contains(&da_ft)
    } else {
        (-CHART_EDGE_TOL..=12_000.0 + CHART_EDGE_TOL).contains(&da_ft)
    }
}

fn fig3_curve_valid_at_da(pct_bhp: i32, da_ft: f64) -> bool {
    if pct_bhp == 75 {
        (-CHART_EDGE_TOL..=FIG3_PCT75_DA_MAX_FT + CHART_EDGE_TOL).contains(&da_ft)
    } else {
        (-CHART_EDGE_TOL..=12_000.0 + CHART_EDGE_TOL).contains(&da_ft)
    }
}

fn pct_brackets(target_pct: f64) -> Option<(i32, i32, f64)> {
    if target_pct < MT_PCT_BHP_MIN - 1e-9 || target_pct > MT_PCT_BHP_MAX + 1e-9 {
        return None;
    }
    for &p in &PRINTED_PCT_CURVES {
        if (target_pct - p as f64).abs() < 1e-9 {
            return Some((p, p, 0.0));
        }
    }
    for w in PRINTED_PCT_CURVES.windows(2) {
        let lo = w[0] as f64;
        let hi = w[1] as f64;
        if target_pct > lo + 1e-9 && target_pct < hi - 1e-9 {
            let t = (target_pct - lo) / (hi - lo);
            return Some((w[0], w[1], t));
        }
    }
    None
}

fn brackets_vec(lo: i32, hi: i32, t: f64) -> Vec<MtCurveBracket> {
    if lo == hi {
        vec![MtCurveBracket {
            pct_bhp: lo,
            weight: 1.0,
        }]
    } else {
        vec![
            MtCurveBracket {
                pct_bhp: lo,
                weight: 1.0 - t,
            },
            MtCurveBracket {
                pct_bhp: hi,
                weight: t,
            },
        ]
    }
}

/// Linear interpolation of a digitized chart curve sample set in density altitude.
pub fn sample_curve_f64(
    samples: &[CurveSample],
    da_ft: f64,
    value: impl Fn(&CurveSample) -> f64,
) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted: Vec<_> = samples.iter().collect();
    sorted.sort_by(|a, b| {
        a.da_ft
            .partial_cmp(&b.da_ft)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if da_ft < sorted[0].da_ft - 1e-6 || da_ft > sorted[sorted.len() - 1].da_ft + 1e-6 {
        return None;
    }
    if let Some(exact) = sorted.iter().find(|s| (s.da_ft - da_ft).abs() < 0.5) {
        return Some(value(exact));
    }
    for pair in sorted.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if da_ft >= a.da_ft && da_ft <= b.da_ft {
            let t = (da_ft - a.da_ft) / (b.da_ft - a.da_ft);
            return Some(lerp(0.0, value(a), 1.0, value(b), t));
        }
    }
    None
}

fn fig2_curve(data: &Mt175rData, pct_bhp: i32) -> Option<&crate::mt175r::data::PowerCurve> {
    data.fig2_curves().get(&format!("{pct_bhp}%"))
}

fn fig3_curve(data: &Mt175rData, pct_bhp: i32) -> Option<&crate::mt175r::data::SpeedCurve> {
    data.fig3_curves().get(&format!("{pct_bhp}%"))
}

/// Fig 2: RPM at a density altitude on a printed %BHP curve (interpolate-only on samples).
pub fn fig2_rpm_at_da(data: &Mt175rData, pct_bhp: i32, da_ft: f64) -> Option<f64> {
    let curve = fig2_curve(data, pct_bhp)?;
    let samples: Vec<CurveSample> = curve
        .samples
        .iter()
        .filter(|s| s.rpm.is_some())
        .cloned()
        .collect();
    if let Some(v) = sample_curve_f64(&samples, da_ft, |s| s.rpm.unwrap()) {
        return Some(v);
    }
    if let Some((end_da, end_rpm)) = fig2_printed_endpoint(pct_bhp) {
        if (da_ft - end_da).abs() < 0.5 {
            return Some(end_rpm);
        }
    }
    None
}

/// Full-throttle TAS (kt) at density altitude on Fig 3 (interpolate-only).
pub fn fig3_full_throttle_tas_at_da(data: &Mt175rData, da_ft: f64) -> Option<f64> {
    let curve = data.fig3_curves().get("Full Throttle")?;
    sample_curve_f64(&curve.samples, da_ft, |s| s.tas_kt.unwrap())
}

/// Fig 2 forward: DA + %BHP → RPM with nearest-curve weights (interpolate only).
pub fn lookup_fig2_rpm_at_pct(
    data: &Mt175rData,
    da_ft: f64,
    target_pct_bhp: f64,
) -> Option<Fig2RpmLookup> {
    let (lo, hi, t) = pct_brackets(target_pct_bhp)?;
    if !fig2_curve_valid_at_da(lo, da_ft) || (lo != hi && !fig2_curve_valid_at_da(hi, da_ft)) {
        return None;
    }
    let rpm_lo = fig2_rpm_at_da(data, lo, da_ft)?;
    let rpm = if lo == hi {
        rpm_lo
    } else {
        let rpm_hi = fig2_rpm_at_da(data, hi, da_ft)?;
        lerp(lo as f64, rpm_lo, hi as f64, rpm_hi, target_pct_bhp)
    };
    if rpm > MT_MAX_PROP_RPM + CHART_EDGE_TOL {
        return None;
    }
    let rpm_45 = fig2_rpm_at_da(data, 45, da_ft)?;
    if rpm < rpm_45 - CHART_EDGE_TOL {
        return None;
    }
    Some(Fig2RpmLookup {
        rpm,
        percent_bhp: target_pct_bhp,
        brackets: brackets_vec(lo, hi, t),
        pct_interp: lo != hi,
    })
}

/// Fig 2 inverse: DA + RPM → %BHP with curve weights.
pub fn lookup_fig2_pct_at_rpm(data: &Mt175rData, da_ft: f64, rpm: f64) -> Option<Fig2PctLookup> {
    if rpm > MT_MAX_PROP_RPM + CHART_EDGE_TOL {
        return None;
    }
    let mut pairs: Vec<(i32, f64)> = Vec::new();
    for &p in &PRINTED_PCT_CURVES {
        if !fig2_curve_valid_at_da(p, da_ft) {
            continue;
        }
        if let Some(r) = fig2_rpm_at_da(data, p, da_ft) {
            pairs.push((p, r));
        }
    }
    pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if pairs.is_empty() {
        return None;
    }
    let rpm_45 = pairs[0].1;
    if rpm < rpm_45 - CHART_EDGE_TOL {
        return None;
    }
    if let Some(exact) = pairs.iter().find(|(_, r)| (*r - rpm).abs() < 0.5) {
        return Some(Fig2PctLookup {
            percent_bhp: exact.0 as f64,
            brackets: vec![MtCurveBracket {
                pct_bhp: exact.0,
                weight: 1.0,
            }],
            pct_interp: false,
        });
    }
    for w in pairs.windows(2) {
        let (plo, rlo) = w[0];
        let (phi, rhi) = w[1];
        if rpm >= rlo - CHART_EDGE_TOL && rpm <= rhi + CHART_EDGE_TOL {
            let t = (rpm - rlo) / (rhi - rlo);
            let pct = lerp(rlo, plo as f64, rhi, phi as f64, rpm);
            return Some(Fig2PctLookup {
                percent_bhp: pct,
                brackets: brackets_vec(plo, phi, t),
                pct_interp: true,
            });
        }
    }
    if rpm > pairs.last()?.1 + CHART_EDGE_TOL {
        return None;
    }
    None
}

/// Fig 3: DA + %BHP → TAS (kt), capped at Full Throttle envelope.
pub fn lookup_fig3_tas_at_pct(
    data: &Mt175rData,
    da_ft: f64,
    target_pct_bhp: f64,
) -> Option<Fig3TasLookup> {
    let (lo, hi, t) = pct_brackets(target_pct_bhp)?;
    if !fig3_curve_valid_at_da(lo, da_ft) || (lo != hi && !fig3_curve_valid_at_da(hi, da_ft)) {
        return None;
    }
    let tas_lo = fig3_tas_at_da(data, lo, da_ft)?;
    let raw_tas = if lo == hi {
        tas_lo
    } else {
        let tas_hi = fig3_tas_at_da(data, hi, da_ft)?;
        lerp(lo as f64, tas_lo, hi as f64, tas_hi, target_pct_bhp)
    };
    let ft_cap = fig3_full_throttle_tas_at_da(data, da_ft);
    let (tas_kt, capped) = match ft_cap {
        Some(ft) if raw_tas > ft + CHART_EDGE_TOL => (ft, true),
        _ => (raw_tas, false),
    };
    Some(Fig3TasLookup {
        tas_kt,
        brackets: brackets_vec(lo, hi, t),
        pct_interp: lo != hi,
        full_throttle_cap_kt: ft_cap,
        capped_at_full_throttle: capped,
    })
}

/// Fig 3: TAS (kt) at a density altitude on a printed %BHP curve (interpolate-only on samples).
pub fn fig3_tas_at_da(data: &Mt175rData, pct_bhp: i32, da_ft: f64) -> Option<f64> {
    let curve = fig3_curve(data, pct_bhp)?;
    let samples: Vec<CurveSample> = curve
        .samples
        .iter()
        .filter(|s| s.tas_kt.is_some())
        .cloned()
        .collect();
    sample_curve_f64(&samples, da_ft, |s| s.tas_kt.unwrap())
}

/// Full-throttle sample helper for curves stored under the Fig 3 map.
pub fn fig3_named_tas_at_da(data: &Mt175rData, name: &str, da_ft: f64) -> Option<f64> {
    let curve = data.fig3_curves().get(name)?;
    sample_curve_f64(&curve.samples, da_ft, |s| s.tas_kt.unwrap())
}

/// Printed endpoints beyond digitized polylines (AFMS fig. 2, read from originals).
fn fig2_printed_endpoint(pct_bhp: i32) -> Option<(f64, f64)> {
    match pct_bhp {
        45 => Some((12_000.0, 2235.0)),
        55 => Some((12_000.0, 2391.0)),
        75 => Some((8270.0, 2650.0)),
        _ => None,
    }
}

/// Points along a Fig 2 curve for plotting (sorted by DA ascending).
pub fn fig2_curve_chart_points(data: &Mt175rData, pct_bhp: i32) -> Vec<(f64, f64)> {
    let Some(curve) = fig2_curve(data, pct_bhp) else {
        return vec![];
    };
    let mut pts: Vec<(f64, f64)> = curve
        .samples
        .iter()
        .filter_map(|s| Some((s.da_ft, s.rpm?)))
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if pct_bhp == 75 {
        pts.retain(|p| p.0 <= 8270.0 + 1.0);
    }
    if let Some(end) = fig2_printed_endpoint(pct_bhp) {
        let needs = pts.last().map(|p| p.0 < end.0 - 1.0).unwrap_or(true);
        if needs && (pct_bhp == 75 || pts.is_empty() || (pts.last().unwrap().0 - end.0).abs() > 1.0)
        {
            pts.push(end);
        }
    }
    pts
}

/// Full-throttle TAS envelope (Fig 3) for display.
pub fn fig3_full_throttle_chart_points(data: &Mt175rData) -> Vec<(f64, f64)> {
    let Some(curve) = data.fig3_curves().get("Full Throttle") else {
        return vec![];
    };
    let mut pts: Vec<(f64, f64)> = curve
        .samples
        .iter()
        .filter_map(|s| Some((s.da_ft, s.tas_kt?)))
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if !pts.iter().any(|p| (p.0 - 8500.0).abs() < 1.0) {
        pts.push((8500.0, 134.2));
    }
    pts
}

/// Points along a Fig 3 curve for plotting (sorted by DA ascending).
pub fn fig3_curve_chart_points(data: &Mt175rData, pct_bhp: i32) -> Vec<(f64, f64)> {
    let Some(curve) = fig3_curve(data, pct_bhp) else {
        return vec![];
    };
    let mut pts: Vec<(f64, f64)> = curve
        .samples
        .iter()
        .filter_map(|s| Some((s.da_ft, s.tas_kt?)))
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fig2_fig3_da6000_pct65_match_validation() {
        let data = Mt175rData::production();
        let rpm = fig2_rpm_at_da(&data, 65, 6000.0).unwrap();
        let tas = fig3_tas_at_da(&data, 65, 6000.0).unwrap();
        assert!((rpm - 2438.7).abs() < 2.0, "rpm={rpm}");
        assert!((tas - 122.12).abs() < 0.5, "tas={tas}");
    }

    #[test]
    fn lookup_fig2_pct_interp_between_55_and_65() {
        let data = Mt175rData::production();
        let rpm = lookup_fig2_rpm_at_pct(&data, 6000.0, 60.0).unwrap().rpm;
        let inv = lookup_fig2_pct_at_rpm(&data, 6000.0, rpm).unwrap();
        assert!((inv.percent_bhp - 60.0).abs() < 0.05);
        assert_eq!(inv.brackets.len(), 2);
    }

    #[test]
    fn fig3_seventy_five_meets_full_throttle_at_8500_ft_134_2_kt() {
        let data = Mt175rData::production();
        let tas_75 = fig3_tas_at_da(&data, 75, 8500.0).expect("75% curve at DA 8500");
        let ft = data
            .fig3_curves()
            .get("Full Throttle")
            .expect("Full Throttle");
        let tas_ft = sample_curve_f64(&ft.samples, 8500.0, |s| s.tas_kt.expect("tas"))
            .expect("Full Throttle at DA 8500");
        assert!((tas_75 - 134.2).abs() < 0.05, "75% tas={tas_75}");
        assert!((tas_ft - 134.2).abs() < 0.05, "full throttle tas={tas_ft}");
        assert!((tas_75 - tas_ft).abs() < 0.05, "75%={tas_75} ft={tas_ft}");
    }
}
