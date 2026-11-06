//! Polyline samples for hand-plot SVG (from digitized Rev 29 JSON).

use crate::rev29::engine::{
    equivalent_altitude_ft, landing_lookup_ft, rate_of_climb_fpm, takeoff_base_m,
};
use crate::rev29::types::Rev29File;
use crate::types::FlightInputs;

pub struct Polyline {
    pub points: Vec<(f64, f64)>,
}

pub fn takeoff_base_curves(data: &Rev29File) -> Vec<Polyline> {
    let to_t = &data.to_t.value;
    let to_pa = &data.to_pa.value;
    let to_base = &data.to_base_m.value;
    to_base
        .iter()
        .zip(to_pa.iter())
        .map(|(row, _pa)| {
            let mut points = Vec::new();
            for (t, v) in to_t.iter().zip(row.iter()) {
                if let Some(m) = v {
                    points.push((*t, *m));
                }
            }
            Polyline { points }
        })
        .filter(|p| !p.points.is_empty())
        .collect()
}

pub fn equivalent_altitude_curves(data: &Rev29File) -> Vec<Polyline> {
    let fan_t = &data.fan_t.value;
    let fan_pa = &data.fan_pa.value;
    let fan_ea = &data.fan_ea.value;
    fan_ea
        .iter()
        .zip(fan_pa.iter())
        .map(|(row, &_pa)| Polyline {
            points: fan_t
                .iter()
                .zip(row.iter())
                .map(|(&t, &ea)| (t, ea))
                .collect(),
        })
        .collect()
}

pub fn climb_roc_vs_ea(data: &Rev29File, roc_key: &str) -> Polyline {
    let [a, b] = data.roc.value.get(roc_key).copied().unwrap_or([0.0, 0.0]);
    let ea_min = 0.0;
    let ea_max = 14_000.0;
    let steps = 20;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let ea = ea_min + (ea_max - ea_min) * i as f64 / steps as f64;
        points.push((ea, a + b * ea));
    }
    Polyline { points }
}

pub fn landing_table_curves(data: &Rev29File) -> (Polyline, Polyline) {
    let alts = &data.land_alt.value;
    let roll: Vec<_> = alts
        .iter()
        .map(|&a| {
            (
                a,
                data.land_roll.value[alts.iter().position(|x| *x == a).unwrap_or(0)],
            )
        })
        .collect();
    let dist50: Vec<_> = alts
        .iter()
        .enumerate()
        .map(|(i, &a)| (a, data.land_50ft.value[i]))
        .collect();
    (Polyline { points: roll }, Polyline { points: dist50 })
}

pub fn wind_headwind_vs_angle(speed_kt: f64) -> Polyline {
    let mut points = Vec::new();
    for deg in (0..=90i32).step_by(3) {
        let rad = (deg as f64).to_radians();
        points.push((deg as f64, speed_kt * rad.cos()));
    }
    Polyline { points }
}

pub fn sample_takeoff_base_m(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    takeoff_base_m(data, pa_ft, oat_c).0
}

pub fn sample_equivalent_altitude_ft(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    equivalent_altitude_ft(data, pa_ft, oat_c).0
}

pub fn sample_roc_fpm(data: &Rev29File, key: &str, pa_ft: f64, oat_c: f64) -> f64 {
    rate_of_climb_fpm(data, key, pa_ft, oat_c)
        .map(|(v, _)| v)
        .unwrap_or(0.0)
}

/// Chart ROC (fpm) at chart PA and OAT (AFM figure value; optional fairing penalty is not applied here).
pub fn sample_roc_fpm_with_inputs(
    data: &Rev29File,
    key: &str,
    pa_ft: f64,
    inputs: &FlightInputs,
) -> f64 {
    sample_roc_fpm(data, key, pa_ft, inputs.oat_c)
}

pub fn sample_landing_ft(data: &Rev29File, pa_ft: f64) -> (f64, f64) {
    let (r, o, _) = landing_lookup_ft(data, pa_ft);
    (r, o)
}

pub fn axis_bounds(values: &[f64]) -> (f64, f64) {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min, max)
}

#[allow(clippy::too_many_arguments)]
pub fn map_to_svg(
    x: f64,
    y: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
) -> (f64, f64) {
    let nx = if (x_max - x_min).abs() < f64::EPSILON {
        0.5
    } else {
        (x - x_min) / (x_max - x_min)
    };
    let ny = if (y_max - y_min).abs() < f64::EPSILON {
        0.5
    } else {
        (y - y_min) / (y_max - y_min)
    };
    (
        left + nx.clamp(0.0, 1.0) * width,
        top + (1.0 - ny.clamp(0.0, 1.0)) * height,
    )
}
