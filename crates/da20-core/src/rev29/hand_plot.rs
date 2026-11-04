//! Multi-panel hand-plot geometry (Fig 5.4 takeoff strip, Fig 5.2 → 5.5/5.6/5.8 climb strip).

use crate::rev29::engine::{
    equivalent_altitude_ft, lb_to_kg, obstacle_add_m, rate_of_climb_fpm, takeoff_base_m,
    takeoff_distances_m, weight_factor, wind_panel_hw_m, wind_panel_tw_m,
};
use crate::rev29::types::Rev29File;
use crate::types::defaults;
use crate::types::FlightInputs;
use crate::wind::runway_heading_deg;

/// Mockup-aligned strip layout (1110×560 viewBox).
pub const STRIP_TOP: f64 = 40.0;
pub const STRIP_BOTTOM: f64 = 500.0;
pub const DIST_MIN_M: f64 = 150.0;
pub const DIST_MAX_M: f64 = 1000.0;

const PATH_STEPS: usize = 12;

#[derive(Clone, Copy, Debug)]
pub struct Panel {
    pub x: f64,
    pub w: f64,
}

pub const TAKEOFF_OAT: Panel = Panel { x: 60.0, w: 420.0 };
pub const TAKEOFF_WEIGHT: Panel = Panel { x: 510.0, w: 230.0 };
pub const TAKEOFF_WIND: Panel = Panel { x: 770.0, w: 150.0 };
pub const TAKEOFF_OBS: Panel = Panel { x: 950.0, w: 110.0 };

pub const CLIMB_EQ: Panel = Panel { x: 60.0, w: 480.0 };
pub const CLIMB_ROC: Panel = Panel { x: 560.0, w: 490.0 };

pub fn y_dist(m: f64) -> f64 {
    STRIP_BOTTOM
        - (m.clamp(DIST_MIN_M, DIST_MAX_M) - DIST_MIN_M) / (DIST_MAX_M - DIST_MIN_M)
            * (STRIP_BOTTOM - STRIP_TOP)
}

pub fn x_oat(oat_c: f64) -> f64 {
    TAKEOFF_OAT.x + (oat_c + 20.0) / 75.0 * TAKEOFF_OAT.w
}

pub fn x_weight_kg(kg: f64) -> f64 {
    TAKEOFF_WEIGHT.x + (kg - 600.0) / 200.0 * TAKEOFF_WEIGHT.w
}

pub fn x_headwind_kt(hw: f64) -> f64 {
    TAKEOFF_WIND.x + (hw.clamp(0.0, 20.0) / 20.0) * TAKEOFF_WIND.w
}

pub fn x_tailwind_kt(tw: f64) -> f64 {
    TAKEOFF_WIND.x - (tw.clamp(0.0, 5.0) / 5.0) * (TAKEOFF_WIND.w * 0.25)
}

pub fn x_obstacle_m(obs_m: f64) -> f64 {
    TAKEOFF_OBS.x
        + (obs_m.clamp(0.0, defaults::OBSTACLE_HEIGHT_MAX_M) / defaults::OBSTACLE_HEIGHT_MAX_M)
            * TAKEOFF_OBS.w
}

pub fn y_ea_ft(ea: f64) -> f64 {
    let ea_min = -2000.0;
    let ea_max = 16_000.0;
    STRIP_BOTTOM
        - (ea.clamp(ea_min, ea_max) - ea_min) / (ea_max - ea_min) * (STRIP_BOTTOM - STRIP_TOP)
}

pub fn x_ea_ft(ea: f64) -> f64 {
    CLIMB_ROC.x + (ea.clamp(0.0, 14_000.0) / 14_000.0) * CLIMB_ROC.w
}

pub fn y_roc_fpm(roc: f64) -> f64 {
    STRIP_BOTTOM - (roc.clamp(0.0, 1000.0) / 1000.0) * (STRIP_BOTTOM - STRIP_TOP)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakeoffPath {
    pub points: Vec<(f64, f64)>,
    pub chart_ground_m: f64,
    pub chart_over_obstacle_m: f64,
    pub core_ground_m: f64,
    pub core_over_obstacle_m: f64,
}

pub fn headwind_and_tailwind(inputs: &FlightInputs) -> (f64, f64) {
    let rwy = runway_heading_deg(&inputs.runway).unwrap_or(0.0);
    let angle = (inputs.wind_dir_deg - rwy).to_radians();
    let hw = inputs.wind_speed_kt * angle.cos();
    (hw.max(0.0), (-hw).max(0.0))
}

fn nearest_weight_guide_line(data: &Rev29File, base_m: f64, ref_kg: f64) -> f64 {
    let (wf_ref, _) = weight_factor(data, ref_kg);
    data.hw_lines
        .value
        .iter()
        .copied()
        .min_by(|a, b| {
            ((*a * wf_ref) - base_m)
                .abs()
                .partial_cmp(&((*b * wf_ref) - base_m).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(data.hw_lines.value[0])
}

fn trace_weight_guide(data: &Rev29File, line_m: f64, from_kg: f64, to_kg: f64) -> Vec<(f64, f64)> {
    let lo = from_kg.min(to_kg);
    let hi = from_kg.max(to_kg);
    let mut pts = Vec::with_capacity(PATH_STEPS + 1);
    for i in 0..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let kg = lo + t * (hi - lo);
        let (wf, _) = weight_factor(data, kg);
        pts.push((x_weight_kg(kg), y_dist(line_m * wf)));
    }
    if to_kg < from_kg {
        pts.reverse();
    }
    pts
}

fn trace_headwind_guide(data: &Rev29File, after_weight_m: f64, hw_kt: f64) -> Vec<(f64, f64)> {
    let mut pts = Vec::with_capacity(PATH_STEPS + 1);
    for i in 0..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let hw = hw_kt * t;
        let dist = wind_panel_hw_m(data, after_weight_m, hw).0;
        pts.push((x_headwind_kt(hw), y_dist(dist)));
    }
    pts
}

fn trace_tailwind_guide(data: &Rev29File, after_weight_m: f64, tw_kt: f64) -> Vec<(f64, f64)> {
    let mut pts = Vec::with_capacity(PATH_STEPS + 1);
    for i in 0..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let tw = tw_kt * t;
        let dist = wind_panel_tw_m(data, after_weight_m, tw).0;
        pts.push((x_tailwind_kt(tw), y_dist(dist)));
    }
    pts
}

fn trace_obstacle_guide(data: &Rev29File, ground_m: f64, obs_m: f64) -> Vec<(f64, f64)> {
    let mut pts = Vec::with_capacity(PATH_STEPS + 1);
    for i in 0..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let h = obs_m * t;
        let dist = obstacle_add_m(data, ground_m, h).0;
        pts.push((x_obstacle_m(h), y_dist(dist)));
    }
    pts
}

fn append_segment(points: &mut Vec<(f64, f64)>, segment: &[(f64, f64)]) {
    for (x, y) in segment {
        if points
            .last()
            .map(|(lx, ly)| (*lx - x).abs() < 0.01 && (*ly - y).abs() < 0.01)
            != Some(true)
        {
            points.push((*x, *y));
        }
    }
}

pub fn takeoff_hand_path(data: &Rev29File, inputs: &FlightInputs, pa_ft: f64) -> TakeoffPath {
    let weight_kg = lb_to_kg(inputs.weight_lb);
    let ref_kg = defaults::TAKEOFF_REF_WEIGHT_KG;
    let (hw, tw) = headwind_and_tailwind(inputs);
    let obs_m = inputs
        .obstacle_height_m
        .clamp(0.0, defaults::OBSTACLE_HEIGHT_MAX_M);
    let core = takeoff_distances_m(data, pa_ft, inputs.oat_c, weight_kg, hw, tw, obs_m);

    let oat_x = x_oat(inputs.oat_c);
    let y_base = y_dist(core.base_m);
    let guide_line = nearest_weight_guide_line(data, core.base_m, ref_kg);

    let mut points = vec![(oat_x, STRIP_BOTTOM), (oat_x, y_base)];

    let ref_x = x_weight_kg(ref_kg);
    points.push((ref_x, y_base));

    let along_weight = trace_weight_guide(data, guide_line, ref_kg, weight_kg);
    append_segment(&mut points, &along_weight);

    let weight_exit_x = TAKEOFF_WEIGHT.x + TAKEOFF_WEIGHT.w;
    let after_wt_y = y_dist(core.after_weight_m);
    if let Some((lx, ly)) = points.last().copied() {
        if (lx - weight_exit_x).abs() > 0.5 {
            points.push((weight_exit_x, ly));
        }
        if (ly - after_wt_y).abs() > 0.5 {
            points.push((weight_exit_x, after_wt_y));
        }
    }

    let wind_entry_x = if tw > 0.0 && hw <= 0.0 {
        x_tailwind_kt(0.0)
    } else {
        x_headwind_kt(0.0)
    };
    if points.last().map(|(x, _)| (*x - wind_entry_x).abs() > 0.5) == Some(true) {
        points.push((wind_entry_x, after_wt_y));
    }

    let wind_trace = if hw > 0.0 {
        trace_headwind_guide(data, core.after_weight_m, hw)
    } else if tw > 0.0 {
        trace_tailwind_guide(data, core.after_weight_m, tw)
    } else {
        vec![(x_headwind_kt(0.0), y_dist(core.after_weight_m))]
    };
    append_segment(&mut points, &wind_trace);

    let obs_entry_x = x_obstacle_m(0.0);
    if let Some((_, yg)) = points.last().copied() {
        if (points.last().unwrap().0 - obs_entry_x).abs() > 0.5 {
            points.push((obs_entry_x, yg));
        }
    }

    let obs_trace = trace_obstacle_guide(data, core.ground_roll_m, obs_m);
    append_segment(&mut points, &obs_trace);

    let chart_ground = if hw > 0.0 {
        wind_panel_hw_m(data, core.after_weight_m, hw).0
    } else if tw > 0.0 {
        wind_panel_tw_m(data, core.after_weight_m, tw).0
    } else {
        core.after_weight_m
    };

    TakeoffPath {
        points,
        chart_ground_m: chart_ground,
        chart_over_obstacle_m: core.over_obstacle_m,
        core_ground_m: core.ground_roll_m,
        core_over_obstacle_m: core.over_obstacle_m,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimbPath {
    pub points: Vec<(f64, f64)>,
    pub chart_ea_ft: f64,
    pub chart_roc_fpm: f64,
    pub core_roc_fpm: f64,
}

pub fn climb_hand_path(data: &Rev29File, inputs: &FlightInputs, pa_ft: f64) -> Option<ClimbPath> {
    let key = inputs.climb_figure.roc_key();
    let (ea, _) = equivalent_altitude_ft(data, pa_ft, inputs.oat_c);
    let (core_roc, _) = rate_of_climb_fpm(data, key, pa_ft, inputs.oat_c)?;
    let [a, b] = *data.roc.value.get(key)?;
    let chart_roc = a + b * ea;

    let oat_x = x_oat(inputs.oat_c);
    let y_ea = y_ea_ft(ea);
    let ea_x = x_ea_ft(ea);
    let y_roc = y_roc_fpm(chart_roc);

    let points = vec![
        (oat_x, STRIP_BOTTOM),
        (oat_x, y_ea),
        (ea_x, y_ea),
        (ea_x, y_roc),
    ];

    Some(ClimbPath {
        points,
        chart_ea_ft: ea,
        chart_roc_fpm: chart_roc,
        core_roc_fpm: core_roc,
    })
}

/// Chart-read base roll (OAT/PA panel only).
pub fn chart_read_takeoff_base_m(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    takeoff_base_m(data, pa_ft, oat_c).0
}

/// Sample wind-panel ground distance at a given headwind (follow guide line).
pub fn chart_read_wind_ground_m(data: &Rev29File, after_weight_m: f64, headwind_kt: f64) -> f64 {
    wind_panel_hw_m(data, after_weight_m, headwind_kt).0
}

/// Weight-panel Y at guide line for distance row and weight kg.
pub fn weight_panel_point(data: &Rev29File, line_m: f64, kg: f64) -> (f64, f64) {
    let (wf, _) = weight_factor(data, kg);
    (x_weight_kg(kg), y_dist(line_m * wf))
}

/// Wind-panel guide segment endpoints at fixed base distance (after weight).
pub fn wind_panel_segment(
    data: &Rev29File,
    after_weight_m: f64,
    headwind_max: f64,
) -> ((f64, f64), (f64, f64)) {
    let y0 = y_dist(after_weight_m);
    let y1 = y_dist(wind_panel_hw_m(data, after_weight_m, headwind_max).0);
    ((x_headwind_kt(0.0), y0), (x_headwind_kt(headwind_max), y1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rev29::engine::{fahrenheit_to_celsius, m_to_ft, takeoff_distances_m};
    use crate::types::FlightInputs;

    #[test]
    fn takeoff_path_endpoints_match_core_distances() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            field_elevation_ft: 1000.0,
            oat_c: fahrenheit_to_celsius(72.0),
            weight_lb: 1600.0,
            wind_dir_deg: 0.0,
            wind_speed_kt: 4.0,
            runway: "36".into(),
            obstacle_height_m: 5.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };

        let path = takeoff_hand_path(&data, &inputs, 1000.0);
        assert!(path.points.len() >= 8);
        let last = path.points.last().unwrap();
        assert!((y_dist(path.core_over_obstacle_m) - last.1).abs() < 0.5);
        assert!((path.chart_ground_m - path.core_ground_m).abs() < 0.01);
    }

    #[test]
    fn afm_obstacle_example_near_1200_ft() {
        let data = Rev29File::production();
        let d = takeoff_distances_m(
            &data,
            1000.0,
            fahrenheit_to_celsius(72.0),
            725.0,
            4.0,
            0.0,
            5.0,
        );
        let ft = m_to_ft(d.over_obstacle_m);
        assert!(
            (ft - 1200.0).abs() < 60.0,
            "expected ~1200 ft over 5 m obstacle, got {ft:.0}"
        );
    }

    #[test]
    fn weight_panel_trace_moves_along_guide() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            weight_lb: 1764.0,
            ..Default::default()
        };
        let path = takeoff_hand_path(&data, &inputs, 2130.0);
        let ref_x = x_weight_kg(defaults::TAKEOFF_REF_WEIGHT_KG);
        let wt_x = x_weight_kg(lb_to_kg(inputs.weight_lb));
        let weight_pts: Vec<_> = path
            .points
            .iter()
            .filter(|(x, _)| *x >= ref_x.min(wt_x) - 1.0 && *x <= ref_x.max(wt_x) + 1.0)
            .collect();
        assert!(weight_pts.len() >= 3);
        let xs: Vec<f64> = weight_pts.iter().map(|(x, _)| *x).collect();
        assert!(xs.windows(2).any(|w| (w[1] - w[0]).abs() > 1.0));
    }

    #[test]
    fn chart_read_base_matches_engine() {
        let data = Rev29File::production();
        let base = chart_read_takeoff_base_m(&data, 1000.0, 21.0);
        let (engine_base, _) = takeoff_base_m(&data, 1000.0, 21.0);
        assert!((base - engine_base).abs() < 0.01);
    }

    #[test]
    fn climb_path_roc_matches_core() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2000.0,
                phase: crate::types::ChartPhase::Climb,
            }),
            oat_c: fahrenheit_to_celsius(65.0),
            ..Default::default()
        };
        let path = climb_hand_path(&data, &inputs, 2000.0).expect("roc");
        assert!((path.chart_roc_fpm - path.core_roc_fpm).abs() < 1.0);
        assert!(
            (path.chart_ea_ft - equivalent_altitude_ft(&data, 2000.0, inputs.oat_c).0).abs() < 1.0
        );
    }

    #[test]
    fn y_dist_monotonic() {
        assert!(y_dist(200.0) > y_dist(800.0));
    }
}
