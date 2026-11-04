//! Multi-panel hand-plot geometry (Fig 5.4 takeoff strip, Fig 5.2 → 5.5/5.6/5.8 climb strip).

use crate::rev29::engine::{
    equivalent_altitude_ft, lb_to_kg, obstacle_add_m, rate_of_climb_fpm, takeoff_base_m,
    takeoff_distances_m, weight_factor, wind_panel_hw_m, wind_panel_tw_m,
};
use crate::rev29::types::Rev29File;
use crate::types::defaults;
use crate::types::FlightInputs;
use crate::wind::{headwind_tailwind_kt, runway_heading_deg, wind_components};

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

const WEIGHT_PANEL_KG_MIN: f64 = 600.0;

pub fn x_weight_kg(kg: f64) -> f64 {
    let kg = kg.clamp(WEIGHT_PANEL_KG_MIN, defaults::TAKEOFF_CHART_WEIGHT_KG);
    // AFM Fig 5.4: chart reference weight (800 kg) at the left of the weight panel.
    TAKEOFF_WEIGHT.x
        + (defaults::TAKEOFF_CHART_WEIGHT_KG - kg)
            / (defaults::TAKEOFF_CHART_WEIGHT_KG - WEIGHT_PANEL_KG_MIN)
            * TAKEOFF_WEIGHT.w
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
    /// Panel / turn markers only (not sampled along curved guide segments).
    pub markers: Vec<(f64, f64)>,
    pub chart_ground_m: f64,
    pub chart_over_obstacle_m: f64,
    pub core_ground_m: f64,
    pub core_over_obstacle_m: f64,
}

pub fn headwind_and_tailwind(inputs: &FlightInputs) -> (f64, f64) {
    let rwy = runway_heading_deg(&inputs.runway).unwrap_or(0.0);
    let c = wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy);
    headwind_tailwind_kt(&c)
}

fn weight_distance_m(data: &Rev29File, base_m: f64, weight_kg: f64) -> f64 {
    let chart_kg = defaults::TAKEOFF_CHART_WEIGHT_KG;
    let (wf, _) = weight_factor(data, weight_kg);
    let (wf_chart, _) = weight_factor(data, chart_kg);
    base_m * wf / wf_chart
}

fn trace_weight_path(data: &Rev29File, base_m: f64, from_kg: f64, to_kg: f64) -> Vec<(f64, f64)> {
    let lo = from_kg.min(to_kg);
    let hi = from_kg.max(to_kg);
    let mut pts = Vec::with_capacity(PATH_STEPS + 1);
    for i in 0..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let kg = lo + t * (hi - lo);
        let dist = weight_distance_m(data, base_m, kg);
        pts.push((x_weight_kg(kg), y_dist(dist)));
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

fn push_turn_marker(markers: &mut Vec<(f64, f64)>, p: (f64, f64)) {
    const EPS: f64 = 0.6;
    if markers
        .iter()
        .all(|m| (m.0 - p.0).abs() >= EPS || (m.1 - p.1).abs() >= EPS)
    {
        markers.push(p);
    }
}

pub fn takeoff_hand_path(data: &Rev29File, inputs: &FlightInputs, pa_ft: f64) -> TakeoffPath {
    let weight_kg = lb_to_kg(inputs.weight_lb);
    let (hw, tw) = headwind_and_tailwind(inputs);
    let obs_m = inputs
        .obstacle_height_m
        .clamp(0.0, defaults::OBSTACLE_HEIGHT_MAX_M);
    let core = takeoff_distances_m(data, pa_ft, inputs.oat_c, weight_kg, hw, tw, obs_m);

    let oat_x = x_oat(inputs.oat_c);
    let y_on_pa_curve = y_dist(core.base_m);
    let weight_ref_x = TAKEOFF_WEIGHT.x;
    let chart_kg = defaults::TAKEOFF_CHART_WEIGHT_KG;
    let after_wt_y = y_dist(core.after_weight_m);

    let mut markers = Vec::with_capacity(8);
    push_turn_marker(&mut markers, (oat_x, STRIP_BOTTOM));
    push_turn_marker(&mut markers, (oat_x, y_on_pa_curve));
    push_turn_marker(&mut markers, (weight_ref_x, y_on_pa_curve));

    // (1) OAT vertical to PA curve; (2) horizontal to weight-panel reference (800 kg, left edge);
    // (3) weight guides to aircraft weight; (4) horizontal to wind panel, then wind guides.
    let mut points = vec![
        (oat_x, STRIP_BOTTOM),
        (oat_x, y_on_pa_curve),
        (weight_ref_x, y_on_pa_curve),
    ];

    let along_weight = trace_weight_path(data, core.base_m, chart_kg, weight_kg);
    append_segment(&mut points, &along_weight);

    let wind_panel_left = if tw > 0.0 && hw <= 0.0 {
        x_tailwind_kt(0.0)
    } else {
        TAKEOFF_WIND.x
    };
    if let Some((lx, ly)) = points.last().copied() {
        if (ly - after_wt_y).abs() > 0.5 {
            points.push((lx, after_wt_y));
        }
        if (lx - wind_panel_left).abs() > 0.5 {
            points.push((wind_panel_left, after_wt_y));
        }
    }
    push_turn_marker(&mut markers, (wind_panel_left, after_wt_y));

    let wind_trace = if hw > 0.0 {
        trace_headwind_guide(data, core.after_weight_m, hw)
    } else if tw > 0.0 {
        trace_tailwind_guide(data, core.after_weight_m, tw)
    } else {
        vec![(x_headwind_kt(0.0), y_dist(core.after_weight_m))]
    };
    append_segment(&mut points, &wind_trace);
    if let Some(wind_exit) = points.last().copied() {
        push_turn_marker(&mut markers, wind_exit);
    }

    let obs_entry_x = x_obstacle_m(0.0);
    if let Some((_, yg)) = points.last().copied() {
        if (points.last().unwrap().0 - obs_entry_x).abs() > 0.5 {
            points.push((obs_entry_x, yg));
            push_turn_marker(&mut markers, (obs_entry_x, yg));
        }
    }

    let obs_trace = trace_obstacle_guide(data, core.ground_roll_m, obs_m);
    append_segment(&mut points, &obs_trace);
    if let Some(final_pt) = points.last().copied() {
        push_turn_marker(&mut markers, final_pt);
    }

    let chart_ground = if hw > 0.0 {
        wind_panel_hw_m(data, core.after_weight_m, hw).0
    } else if tw > 0.0 {
        wind_panel_tw_m(data, core.after_weight_m, tw).0
    } else {
        core.after_weight_m
    };

    TakeoffPath {
        points,
        markers,
        chart_ground_m: chart_ground,
        chart_over_obstacle_m: core.over_obstacle_m,
        core_ground_m: core.ground_roll_m,
        core_over_obstacle_m: core.over_obstacle_m,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimbPath {
    pub points: Vec<(f64, f64)>,
    pub markers: Vec<(f64, f64)>,
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
    let markers = points.clone();

    Some(ClimbPath {
        points,
        markers,
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

/// Sample points along a weight-panel guide (800 kg → 600 kg, same wf curve as the hand path).
pub fn weight_guide_polyline(data: &Rev29File, line_m: f64) -> Vec<(f64, f64)> {
    (0..=PATH_STEPS)
        .map(|i| {
            let t = i as f64 / PATH_STEPS as f64;
            let kg = defaults::TAKEOFF_CHART_WEIGHT_KG
                - t * (defaults::TAKEOFF_CHART_WEIGHT_KG - WEIGHT_PANEL_KG_MIN);
            weight_panel_point(data, line_m, kg)
        })
        .collect()
}

/// Weight-panel Y at guide line for distance row and weight kg.
pub fn weight_panel_point(data: &Rev29File, line_m: f64, kg: f64) -> (f64, f64) {
    let chart_kg = defaults::TAKEOFF_CHART_WEIGHT_KG;
    let (wf, _) = weight_factor(data, kg);
    let (wf_chart, _) = weight_factor(data, chart_kg);
    (x_weight_kg(kg), y_dist(line_m * wf / wf_chart))
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
        let d = takeoff_distances_m(&data, 1000.0, 21.0, 725.0, 4.0, 0.0, 5.0);
        let ft = m_to_ft(d.over_obstacle_m);
        assert!(
            (ft - 1154.0).abs() / 1154.0 <= 0.015,
            "expected ~1154 ft raw chart, got {ft:.0}"
        );
        assert_eq!(crate::takeoff::afm_conservative_distance_ft(ft), 1200.0);
    }

    #[test]
    fn takeoff_path_follows_pilot_reading_order() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 1000.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            oat_c: 21.0,
            weight_lb: 1598.0,
            ..Default::default()
        };
        let path = takeoff_hand_path(&data, &inputs, 1000.0);
        let oat_x = x_oat(21.0);
        assert_eq!(path.points[0], (oat_x, STRIP_BOTTOM));
        assert!((path.points[1].0 - oat_x).abs() < 0.01);
        assert_eq!(path.points[2].0, TAKEOFF_WEIGHT.x);
        assert!((path.points[2].1 - path.points[1].1).abs() < 0.01);
        let core = takeoff_distances_m(
            &data,
            1000.0,
            21.0,
            lb_to_kg(inputs.weight_lb),
            0.0,
            0.0,
            inputs.obstacle_height_m,
        );
        let wt_y = path
            .points
            .iter()
            .find(|(x, _)| (x - x_weight_kg(lb_to_kg(inputs.weight_lb))).abs() < 1.0)
            .map(|(_, y)| *y)
            .unwrap_or(0.0);
        assert!((wt_y - y_dist(core.after_weight_m)).abs() < 0.5);
    }

    #[test]
    fn weight_panel_trace_moves_along_guide() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            weight_lb: 1598.0,
            ..Default::default()
        };
        let path = takeoff_hand_path(&data, &inputs, 2130.0);
        let chart_x = x_weight_kg(defaults::TAKEOFF_CHART_WEIGHT_KG);
        let wt_x = x_weight_kg(lb_to_kg(inputs.weight_lb));
        let weight_pts: Vec<_> = path
            .points
            .iter()
            .filter(|(x, _)| *x >= chart_x.min(wt_x) - 1.0 && *x <= chart_x.max(wt_x) + 1.0)
            .collect();
        assert!(weight_pts.len() >= 3);
        let xs: Vec<f64> = weight_pts.iter().map(|(x, _)| *x).collect();
        assert!(xs.windows(2).any(|w| (w[1] - w[0]).abs() > 1.0));
        let core = takeoff_distances_m(
            &data,
            2130.0,
            inputs.oat_c,
            lb_to_kg(inputs.weight_lb),
            0.0,
            0.0,
            inputs.obstacle_height_m,
        );
        let last_in_panel = weight_pts.last().unwrap();
        assert!((last_in_panel.1 - y_dist(core.after_weight_m)).abs() < 0.5);
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

    fn panel_y_continuity(data: &Rev29File, inputs: FlightInputs, pa_ft: f64) {
        let (hw, tw) = headwind_and_tailwind(&inputs);
        let weight_kg = lb_to_kg(inputs.weight_lb);
        let obs_m = inputs
            .obstacle_height_m
            .clamp(0.0, defaults::OBSTACLE_HEIGHT_MAX_M);
        let core = takeoff_distances_m(data, pa_ft, inputs.oat_c, weight_kg, hw, tw, obs_m);
        let path = takeoff_hand_path(data, &inputs, pa_ft);

        let wt_exit_y = y_dist(core.after_weight_m);
        let wind_x = if tw > 0.0 && hw <= 0.0 {
            x_tailwind_kt(0.0)
        } else {
            TAKEOFF_WIND.x
        };
        let wind_entry = path
            .points
            .iter()
            .find(|(x, _)| (*x - wind_x).abs() < 1.5)
            .unwrap_or_else(|| panic!("no wind entry for {inputs:?}"));
        assert!(
            (wind_entry.1 - wt_exit_y).abs() < 0.6,
            "wind entry y {} vs weight exit y {wt_exit_y}",
            wind_entry.1
        );

        let ground_y = y_dist(core.ground_roll_m);
        let obs_entry = path
            .points
            .iter()
            .find(|(x, _)| *x >= TAKEOFF_OBS.x - 0.5)
            .unwrap_or_else(|| panic!("no obstacle entry"));
        assert!(
            (obs_entry.1 - ground_y).abs() < 0.6,
            "obstacle entry y {} vs wind exit y {ground_y}",
            obs_entry.1
        );
    }

    #[test]
    fn takeoff_path_wind_and_obstacle_entries_match_exits() {
        let data = Rev29File::production();
        let cases = [
            FlightInputs {
                oat_c: 35.0,
                weight_lb: 1500.0,
                wind_dir_deg: 270.0,
                wind_speed_kt: 12.0,
                runway: "36".into(),
                obstacle_height_m: 8.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 2500.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
            FlightInputs {
                oat_c: 5.0,
                weight_lb: 1800.0,
                wind_dir_deg: 90.0,
                wind_speed_kt: 6.0,
                runway: "19".into(),
                obstacle_height_m: 12.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 500.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
            FlightInputs {
                oat_c: 21.0,
                weight_lb: 1598.0,
                wind_dir_deg: 180.0,
                wind_speed_kt: 8.0,
                runway: "36".into(),
                obstacle_height_m: 5.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 1000.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
            FlightInputs {
                oat_c: 28.0,
                weight_lb: 1650.0,
                wind_dir_deg: 360.0,
                wind_speed_kt: 3.0,
                runway: "09".into(),
                obstacle_height_m: 0.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 3500.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
        ];
        for inputs in cases {
            let pa = inputs
                .pa_override
                .as_ref()
                .map(|o| o.pa_ft)
                .unwrap_or(2130.0);
            panel_y_continuity(&data, inputs, pa);
        }
    }

    #[test]
    fn takeoff_markers_only_at_turn_points() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            oat_c: 35.0,
            weight_lb: 1500.0,
            wind_dir_deg: 270.0,
            wind_speed_kt: 12.0,
            runway: "36".into(),
            obstacle_height_m: 8.0,
            ..Default::default()
        };
        let path = takeoff_hand_path(&data, &inputs, 2500.0);
        assert!(
            path.points.len() > path.markers.len(),
            "sampled polyline {} pts vs {} turn markers",
            path.points.len(),
            path.markers.len()
        );
        assert!(
            path.markers.len() >= 5 && path.markers.len() <= 8,
            "unexpected marker count {}",
            path.markers.len()
        );
    }

    #[test]
    fn climb_markers_match_turn_points() {
        let data = Rev29File::production();
        let path = climb_hand_path(
            &data,
            &FlightInputs {
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 2000.0,
                    phase: crate::types::ChartPhase::Climb,
                }),
                ..Default::default()
            },
            2000.0,
        )
        .expect("climb");
        assert_eq!(path.markers.len(), path.points.len());
    }
}
