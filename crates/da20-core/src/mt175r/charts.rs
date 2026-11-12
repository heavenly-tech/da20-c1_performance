use crate::interpolation::lerp;
use crate::mt175r::data::{CurveSample, Mt175rData};

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
    sample_curve_f64(&samples, da_ft, |s| s.rpm.unwrap())
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
