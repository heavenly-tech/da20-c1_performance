//! Multi-panel hand-plot geometry (Fig 5.4 takeoff strip, Fig 5.2 → 5.5/5.6/5.8 climb strip).

use crate::rev29::engine::{self, lb_to_kg, rate_of_climb_fpm, TakeoffDistancesM};

fn equivalent_altitude_ft(
    data: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
) -> (f64, Vec<CalculationWarning>) {
    engine::equivalent_altitude_ft(data, pa_ft, oat_c).unwrap_or((f64::NAN, vec![]))
}

fn takeoff_distances_m(
    data: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
    weight_kg: f64,
    headwind_kt: f64,
    tailwind_kt: f64,
    obstacle_m: f64,
) -> TakeoffDistancesM {
    engine::takeoff_distances_m(
        data,
        pa_ft,
        oat_c,
        weight_kg,
        headwind_kt,
        tailwind_kt,
        obstacle_m,
    )
    .unwrap_or_else(|_| TakeoffDistancesM {
        base_m: f64::NAN,
        after_weight_m: f64::NAN,
        wind_panel_entry_m: f64::NAN,
        obstacle_panel_entry_m: f64::NAN,
        ground_roll_m: f64::NAN,
        over_obstacle_m: f64::NAN,
        warnings: vec![],
    })
}

fn takeoff_base_m(data: &Rev29File, pa_ft: f64, oat_c: f64) -> (f64, Vec<CalculationWarning>) {
    engine::takeoff_base_m(data, pa_ft, oat_c).unwrap_or((f64::NAN, vec![]))
}

fn weight_factor(data: &Rev29File, weight_kg: f64) -> (f64, Vec<CalculationWarning>) {
    engine::weight_factor(data, weight_kg).unwrap_or((f64::NAN, vec![]))
}

fn wind_panel_hw_m(
    data: &Rev29File,
    entry_m: f64,
    headwind_kt: f64,
) -> (f64, f64, Vec<CalculationWarning>) {
    engine::wind_panel_hw_m(data, entry_m, headwind_kt).unwrap_or((f64::NAN, f64::NAN, vec![]))
}

fn wind_panel_tw_m(
    data: &Rev29File,
    entry_m: f64,
    tailwind_kt: f64,
) -> (f64, f64, Vec<CalculationWarning>) {
    engine::wind_panel_tw_m(data, entry_m, tailwind_kt).unwrap_or((f64::NAN, f64::NAN, vec![]))
}

fn obstacle_add_m(
    data: &Rev29File,
    ground_m: f64,
    obstacle_m: f64,
) -> (f64, f64, Vec<CalculationWarning>) {
    engine::obstacle_add_m(data, ground_m, obstacle_m).unwrap_or((f64::NAN, f64::NAN, vec![]))
}
use crate::rev29::types::Rev29File;
use crate::types::climb_roc_fairing_multiplier;
use crate::types::defaults;
use crate::types::CalculationWarning;
use crate::types::FlightInputs;
use crate::wind::{headwind_tailwind_kt, runway_heading_deg, wind_components};

/// Mockup-aligned strip layout (1110×560 viewBox).
pub const STRIP_TOP: f64 = 40.0;
pub const STRIP_BOTTOM: f64 = 500.0;
pub const DIST_MIN_M: f64 = 150.0;
pub const DIST_MAX_M: f64 = 1000.0;

const PATH_STEPS: usize = 12;

fn wind_panel_hw_distance(
    data: &Rev29File,
    entry_m: f64,
    kt: f64,
) -> (f64, Vec<CalculationWarning>) {
    let (d, _, w) = wind_panel_hw_m(data, entry_m, kt);
    (d, w)
}

fn wind_panel_tw_distance(
    data: &Rev29File,
    entry_m: f64,
    kt: f64,
) -> (f64, Vec<CalculationWarning>) {
    let (d, _, w) = wind_panel_tw_m(data, entry_m, kt);
    (d, w)
}

#[derive(Clone, Copy, Debug)]
pub struct Panel {
    pub x: f64,
    pub w: f64,
}

pub const TAKEOFF_OAT: Panel = Panel { x: 60.0, w: 420.0 };
pub const TAKEOFF_WEIGHT: Panel = Panel { x: 510.0, w: 230.0 };
/// Full wind panel (TW sub-panel + HW sub-panel). Wide enough that a 5 kt
/// tailwind correction reads as a slope (5:20 sub-panels, 80 px per 5 kt).
pub const TAKEOFF_WIND: Panel = Panel { x: 770.0, w: 400.0 };
/// Fig 5.4 tailwind sub-panel (0–5 kt); 0 kt reference on the right edge, away from weight panel.
/// Width 5/25 of the wind panel (same 0–5 kt vs 0–20 kt scale as the AFM chart).
pub const TAKEOFF_TW: Panel = Panel {
    x: TAKEOFF_WIND.x,
    w: TAKEOFF_WIND.w * 5.0 / 25.0,
};
/// Headwind sub-panel (0–20 kt), to the right of the TW sub-panel.
pub const TAKEOFF_HW: Panel = Panel {
    x: TAKEOFF_TW.x + TAKEOFF_TW.w,
    w: TAKEOFF_WIND.w - TAKEOFF_TW.w,
};
pub const TAKEOFF_OBS: Panel = Panel {
    x: 1170.0,
    w: 110.0,
};

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
    TAKEOFF_HW.x + (hw.clamp(0.0, 20.0) / 20.0) * TAKEOFF_HW.w
}

pub fn x_tailwind_kt(tw: f64) -> f64 {
    TAKEOFF_TW.x + TAKEOFF_TW.w - (tw.clamp(0.0, 5.0) / 5.0) * TAKEOFF_TW.w
}

/// Vertical 0 kt tailwind reference (entry from weight panel).
pub fn tailwind_reference_x() -> f64 {
    x_tailwind_kt(0.0)
}

pub fn takeoff_wind_is_tailwind(hw: f64, tw: f64) -> bool {
    tw > 0.05 && tw >= hw
}

pub fn takeoff_wind_panel_entry_x(hw: f64, tw: f64) -> f64 {
    if takeoff_wind_is_tailwind(hw, tw) {
        tailwind_reference_x()
    } else {
        TAKEOFF_HW.x
    }
}

pub fn tailwind_subpanel_x_range() -> (f64, f64) {
    (TAKEOFF_TW.x, TAKEOFF_TW.x + TAKEOFF_TW.w)
}

pub fn tailwind_kt_from_strip_x(x: f64) -> f64 {
    let x = x.clamp(TAKEOFF_TW.x, TAKEOFF_TW.x + TAKEOFF_TW.w);
    (TAKEOFF_TW.x + TAKEOFF_TW.w - x) / TAKEOFF_TW.w * 5.0
}

pub fn headwind_kt_from_strip_x(x: f64) -> f64 {
    let x = x.clamp(TAKEOFF_HW.x, TAKEOFF_HW.x + TAKEOFF_HW.w);
    (x - TAKEOFF_HW.x) / TAKEOFF_HW.w * 20.0
}

/// Map strip x in the Fig 5.4 wind panel to a wind speed/direction back-solve.
/// TW sub-panel (left) → tailwind; HW sub-panel (right) → headwind; shared edge is 0 kt.
pub fn back_solve_takeoff_wind_from_strip_x(
    sx: f64,
    wind_dir_deg: f64,
    runway_heading_deg: f64,
) -> Option<(f64, f64, bool)> {
    if !(TAKEOFF_WIND.x..=TAKEOFF_WIND.x + TAKEOFF_WIND.w).contains(&sx) {
        return None;
    }
    if sx <= tailwind_reference_x() {
        let tw = tailwind_kt_from_strip_x(sx);
        Some(crate::wind::back_solve_wind_for_tailwind(
            wind_dir_deg,
            runway_heading_deg,
            tw,
        ))
    } else {
        let hw = headwind_kt_from_strip_x(sx);
        Some(crate::wind::back_solve_wind_for_headwind(
            wind_dir_deg,
            runway_heading_deg,
            hw,
        ))
    }
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

/// Inverse of [`y_ea_ft`]: EA (ft) at a strip Y (shared EA / ROC panel vertical axis).
pub fn ea_ft_from_strip_y(y: f64) -> f64 {
    let ea_min = -2000.0;
    let ea_max = 16_000.0;
    let h = STRIP_BOTTOM - STRIP_TOP;
    let t = (STRIP_BOTTOM - y.clamp(STRIP_TOP, STRIP_BOTTOM)) / h;
    ea_min + t * (ea_max - ea_min)
}

/// ROC panel horizontal axis span (AFM Fig 5.5/5.6/5.8).
pub const ROC_PANEL_MAX_FPM: f64 = 1100.0;

/// ROC panel horizontal axis: ROC (fpm) → strip x (0–1100 fpm).
pub fn x_roc_fpm(roc: f64) -> f64 {
    CLIMB_ROC.x + (roc.clamp(0.0, ROC_PANEL_MAX_FPM) / ROC_PANEL_MAX_FPM) * CLIMB_ROC.w
}

/// ROC (fpm) read on the ROC panel x-axis at strip x.
pub fn roc_fpm_from_strip_x(x: f64) -> f64 {
    ((x - CLIMB_ROC.x) / CLIMB_ROC.w * ROC_PANEL_MAX_FPM).clamp(0.0, ROC_PANEL_MAX_FPM)
}

/// Y coordinate of the ROC panel x-axis (ROC = 0 baseline at panel bottom).
pub fn roc_panel_x_axis_y() -> f64 {
    STRIP_BOTTOM
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

fn bracket_entry_line_index(lines: &[f64], entry_m: f64) -> usize {
    let mut j = 0usize;
    while j + 1 < lines.len() && lines[j + 1] <= entry_m {
        j += 1;
    }
    j.min(lines.len().saturating_sub(2))
}

fn bracket_hw_line_index(lines: &[f64], entry_m: f64) -> usize {
    bracket_entry_line_index(lines, entry_m)
}

/// Weight-panel point on the interpolated distance-row guide at `kg`.
pub fn weight_panel_point_interp(data: &Rev29File, entry_m: f64, kg: f64) -> (f64, f64) {
    let lines = &data.hw_lines.value;
    if lines.is_empty() {
        return weight_panel_point(data, entry_m, kg);
    }
    if lines.len() == 1 || entry_m <= lines[0] {
        return weight_panel_point(data, lines[0], kg);
    }
    if entry_m >= lines[lines.len() - 1] {
        return weight_panel_point(data, *lines.last().unwrap(), kg);
    }
    let j = bracket_hw_line_index(lines, entry_m);
    let p0 = weight_panel_point(data, lines[j], kg);
    let p1 = weight_panel_point(data, lines[j + 1], kg);
    let t = (entry_m - lines[j]) / (lines[j + 1] - lines[j]);
    (p0.0 + t * (p1.0 - p0.0), p0.1 + t * (p1.1 - p0.1))
}

fn tailwind_panel_point(data: &Rev29File, line_m: f64, tw_kt: f64) -> (f64, f64) {
    let dist = wind_panel_tw_distance(data, line_m, tw_kt).0;
    (x_tailwind_kt(tw_kt), y_dist(dist))
}

/// Tailwind-panel point on the interpolated distance-row guide at `tw_kt`.
pub fn tailwind_panel_point_interp(data: &Rev29File, entry_m: f64, tw_kt: f64) -> (f64, f64) {
    let lines = &data.tw_lines.value;
    if lines.is_empty() {
        return tailwind_panel_point(data, entry_m, tw_kt);
    }
    if lines.len() == 1 || entry_m <= lines[0] {
        return tailwind_panel_point(data, lines[0], tw_kt);
    }
    if entry_m >= lines[lines.len() - 1] {
        return tailwind_panel_point(data, *lines.last().unwrap(), tw_kt);
    }
    let j = bracket_entry_line_index(lines, entry_m);
    let p0 = tailwind_panel_point(data, lines[j], tw_kt);
    let p1 = tailwind_panel_point(data, lines[j + 1], tw_kt);
    let t = (entry_m - lines[j]) / (lines[j + 1] - lines[j]);
    (p0.0 + t * (p1.0 - p0.0), p0.1 + t * (p1.1 - p0.1))
}

fn headwind_panel_point(data: &Rev29File, line_m: f64, hw_kt: f64) -> (f64, f64) {
    let dist = wind_panel_hw_distance(data, line_m, hw_kt).0;
    (x_headwind_kt(hw_kt), y_dist(dist))
}

fn headwind_panel_point_interp(data: &Rev29File, entry_m: f64, hw_kt: f64) -> (f64, f64) {
    let lines = &data.hw_lines.value;
    if lines.is_empty() {
        return headwind_panel_point(data, entry_m, hw_kt);
    }
    if lines.len() == 1 || entry_m <= lines[0] {
        return headwind_panel_point(data, lines[0], hw_kt);
    }
    if entry_m >= lines[lines.len() - 1] {
        return headwind_panel_point(data, *lines.last().unwrap(), hw_kt);
    }
    let j = bracket_entry_line_index(lines, entry_m);
    let p0 = headwind_panel_point(data, lines[j], hw_kt);
    let p1 = headwind_panel_point(data, lines[j + 1], hw_kt);
    let t = (entry_m - lines[j]) / (lines[j + 1] - lines[j]);
    (p0.0 + t * (p1.0 - p0.0), p0.1 + t * (p1.1 - p0.1))
}

fn trace_weight_path_interp(
    data: &Rev29File,
    entry_m: f64,
    from_kg: f64,
    to_kg: f64,
) -> Vec<(f64, f64)> {
    let lo = from_kg.min(to_kg);
    let hi = from_kg.max(to_kg);
    let mut pts = Vec::with_capacity(PATH_STEPS + 1);
    for i in 0..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let kg = lo + t * (hi - lo);
        pts.push(weight_panel_point_interp(data, entry_m, kg));
    }
    if to_kg < from_kg {
        pts.reverse();
    }
    pts
}

fn trace_headwind_guide(
    data: &Rev29File,
    after_weight_m: f64,
    hw_kt: f64,
    entry: (f64, f64),
) -> Vec<(f64, f64)> {
    if hw_kt <= 1e-9 {
        return vec![entry];
    }
    let mut pts = vec![entry];
    for i in 1..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let kt = hw_kt * t;
        pts.push(headwind_panel_point_interp(data, after_weight_m, kt));
    }
    pts
}

fn trace_tailwind_guide(
    data: &Rev29File,
    after_weight_m: f64,
    tw_kt: f64,
    entry: (f64, f64),
) -> Vec<(f64, f64)> {
    if tw_kt <= 1e-9 {
        return vec![entry];
    }
    let mut pts = vec![entry];
    for i in 1..=PATH_STEPS {
        let t = i as f64 / PATH_STEPS as f64;
        let kt = tw_kt * t;
        pts.push(tailwind_panel_point_interp(data, after_weight_m, kt));
    }
    pts
}

/// Chart guide polyline at fixed entry distance (same samples as the hand path wind trace).
pub fn headwind_guide_polyline(data: &Rev29File, entry_line_m: f64) -> Vec<(f64, f64)> {
    wind_guide_polyline(
        data,
        entry_line_m,
        &data.hw_kt.value,
        wind_panel_hw_distance,
        x_headwind_kt,
    )
}

pub fn tailwind_guide_polyline(data: &Rev29File, entry_line_m: f64) -> Vec<(f64, f64)> {
    wind_guide_polyline(
        data,
        entry_line_m,
        &data.tw_kt.value,
        wind_panel_tw_distance,
        x_tailwind_kt,
    )
}

fn wind_guide_polyline(
    data: &Rev29File,
    entry_line_m: f64,
    kt_axis: &[f64],
    sample: fn(&Rev29File, f64, f64) -> (f64, Vec<CalculationWarning>),
    x_map: fn(f64) -> f64,
) -> Vec<(f64, f64)> {
    kt_axis
        .iter()
        .map(|&kt| {
            let dist = sample(data, entry_line_m, kt).0;
            (x_map(kt), y_dist(dist))
        })
        .collect()
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
    let wind_row_m = core.after_weight_m;

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

    let along_weight = trace_weight_path_interp(data, core.base_m, chart_kg, weight_kg);
    append_segment(&mut points, &along_weight);

    // Enter the wind panel on a horizontal at the weight-exit distance. Do not snap
    // vertically onto the 0 kt guide — that step is the jump at the panel edge.
    let wind_entry_x = if takeoff_wind_is_tailwind(hw, tw) {
        tailwind_reference_x()
    } else {
        TAKEOFF_HW.x
    };
    if let Some((lx, ly)) = points.last().copied() {
        if (lx - wind_entry_x).abs() > 0.5 {
            points.push((wind_entry_x, ly));
        }
    }
    let wind_entry = points
        .last()
        .copied()
        .unwrap_or((wind_entry_x, y_dist(wind_row_m)));
    push_turn_marker(&mut markers, wind_entry);

    let wind_trace = if takeoff_wind_is_tailwind(hw, tw) {
        trace_tailwind_guide(data, wind_row_m, tw, wind_entry)
    } else if hw > 0.0 {
        trace_headwind_guide(data, wind_row_m, hw, wind_entry)
    } else {
        vec![wind_entry]
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

    let obs_trace = trace_obstacle_guide(data, core.obstacle_panel_entry_m, obs_m);
    append_segment(&mut points, &obs_trace);
    if let Some(final_pt) = points.last().copied() {
        push_turn_marker(&mut markers, final_pt);
    }

    TakeoffPath {
        points,
        markers,
        chart_ground_m: core.ground_roll_m,
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
    let (core_roc_base, _) = rate_of_climb_fpm(data, key, pa_ft, inputs.oat_c).ok()?;
    let fair_m = climb_roc_fairing_multiplier(inputs.wheel_fairings_installed);
    let core_roc = core_roc_base * fair_m;

    let oat_x = x_oat(inputs.oat_c);
    let y_ea = y_ea_ft(ea);
    let fan_right = CLIMB_EQ.x + CLIMB_EQ.w;
    // Chart line is AFM Fig 5.x read; optional fairing penalty applies in the numeric readout only.
    let chart_roc = chart_roc_fpm_at_ea(data, key, ea);
    let x_on_roc = x_roc_fpm(chart_roc);

    // Fan → EA; horizontal at EA onto ROC line; vertical to ROC x-axis readout.
    let points = vec![
        (oat_x, STRIP_BOTTOM),
        (oat_x, y_ea),
        (fan_right, y_ea),
        (x_on_roc, y_ea),
        (x_on_roc, roc_panel_x_axis_y()),
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

/// ROC (fpm) read on the selected climb figure line at equivalent altitude.
pub fn chart_roc_fpm_at_ea(data: &Rev29File, roc_key: &str, ea_ft: f64) -> f64 {
    let [a, b] = *data.roc.value.get(roc_key).unwrap_or(&[0.0, 0.0]);
    a + b * ea_ft
}

/// Chart-read base roll (OAT/PA panel only).
pub fn chart_read_takeoff_base_m(data: &Rev29File, pa_ft: f64, oat_c: f64) -> f64 {
    takeoff_base_m(data, pa_ft, oat_c).0
}

/// Horizontal PA trace on the Fig 5.4 OAT/PA panel at the chart-read base distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TakeoffOatPaTrace {
    pub y: f64,
    pub oat_x: f64,
    pub pa_ft: f64,
}

/// Fig 5.4 OAT/PA panel: full-width PA isoline (interpolated between printed PA rows).
pub fn takeoff_pa_curve_strip_points(data: &Rev29File, pa_ft: f64) -> Vec<(f64, f64)> {
    data.to_t
        .value
        .iter()
        .filter_map(|&t| {
            engine::takeoff_base_m(data, pa_ft, t)
                .ok()
                .map(|(m, _)| (x_oat(t), y_dist(m)))
        })
        .collect()
}

/// Point on the constant-PA takeoff curve at the current OAT (for vertical cursor).
pub fn takeoff_pa_curve_point_at_oat(data: &Rev29File, pa_ft: f64, oat_c: f64) -> (f64, f64) {
    let x = x_oat(oat_c);
    let pts = takeoff_pa_curve_strip_points(data, pa_ft);
    if pts.is_empty() {
        return (x, STRIP_BOTTOM);
    }
    if pts.len() == 1 {
        return pts[0];
    }
    if x <= pts[0].0 {
        return pts[0];
    }
    if x >= pts.last().unwrap().0 {
        return *pts.last().unwrap();
    }
    for w in pts.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if x >= x0 && x <= x1 {
            let t = if (x1 - x0).abs() < f64::EPSILON {
                0.0
            } else {
                (x - x0) / (x1 - x0)
            };
            return (x, y0 + t * (y1 - y0));
        }
    }
    pts.last().copied().unwrap_or((x, STRIP_BOTTOM))
}

pub fn takeoff_oat_panel_pa_trace(data: &Rev29File, pa_ft: f64, oat_c: f64) -> TakeoffOatPaTrace {
    let (oat_x, y) = takeoff_pa_curve_point_at_oat(data, pa_ft, oat_c);
    TakeoffOatPaTrace { y, oat_x, pa_ft }
}

/// Y on a strip polyline at a fixed strip x (linear between adjacent samples).
pub fn y_on_strip_polyline_at_x(pts: &[(f64, f64)], x: f64) -> f64 {
    if pts.is_empty() {
        return STRIP_BOTTOM;
    }
    if pts.len() == 1 {
        return pts[0].1;
    }
    if x <= pts[0].0 {
        return pts[0].1;
    }
    if x >= pts.last().unwrap().0 {
        return pts.last().unwrap().1;
    }
    for w in pts.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if x >= x0 && x <= x1 {
            let t = if (x1 - x0).abs() < f64::EPSILON {
                0.0
            } else {
                (x - x0) / (x1 - x0)
            };
            return y0 + t * (y1 - y0);
        }
    }
    pts.last().map(|p| p.1).unwrap_or(STRIP_BOTTOM)
}

/// Y where the selected PA isoline meets the right edge of the OAT/PA panel.
pub fn takeoff_pa_isoline_y_at_panel_right(data: &Rev29File, pa_ft: f64) -> f64 {
    let x = TAKEOFF_OAT.x + TAKEOFF_OAT.w;
    y_on_strip_polyline_at_x(&takeoff_pa_curve_strip_points(data, pa_ft), x)
}

pub fn climb_pa_isoline_y_at_panel_right(data: &Rev29File, pa_ft: f64) -> f64 {
    let x = CLIMB_EQ.x + CLIMB_EQ.w;
    y_on_strip_polyline_at_x(&climb_fan_pa_curve_strip_points(data, pa_ft), x)
}

/// Fig 5.2 fan chart: OAT → (x,y) along the **constant chart pressure-altitude** row for `pa_ft`.
pub fn climb_fan_pa_curve_strip_points(data: &Rev29File, pa_ft: f64) -> Vec<(f64, f64)> {
    data.fan_t
        .value
        .iter()
        .filter_map(|&t| {
            engine::equivalent_altitude_ft(data, pa_ft, t)
                .ok()
                .map(|(ea, _)| (x_oat(t), y_ea_ft(ea)))
        })
        .collect()
}

/// Point on the constant-PA fan curve at the current OAT strip position (for guide + vertical cursor).
pub fn climb_fan_pa_curve_point_at_oat(data: &Rev29File, pa_ft: f64, oat_c: f64) -> (f64, f64) {
    let x = x_oat(oat_c);
    let pts = climb_fan_pa_curve_strip_points(data, pa_ft);
    if pts.is_empty() {
        return (x, STRIP_BOTTOM);
    }
    if pts.len() == 1 {
        return pts[0];
    }
    if x <= pts[0].0 {
        return pts[0];
    }
    if x >= pts.last().unwrap().0 {
        return *pts.last().unwrap();
    }
    for w in pts.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if x >= x0 && x <= x1 {
            let t = if (x1 - x0).abs() < f64::EPSILON {
                0.0
            } else {
                (x - x0) / (x1 - x0)
            };
            return (x, y0 + t * (y1 - y0));
        }
    }
    pts.last().copied().unwrap_or((x, STRIP_BOTTOM))
}

/// Fig 5.2 fan chart in **chart coordinates** (OAT °C, EA ft): full constant-PA isoline like the
/// printed curves — every sample in `fan_t`, spanning the left panel.
pub fn climb_fan_pa_curve_chart_points(data: &Rev29File, pa_ft: f64) -> Vec<(f64, f64)> {
    data.fan_t
        .value
        .iter()
        .filter_map(|&t| {
            engine::equivalent_altitude_ft(data, pa_ft, t)
                .ok()
                .map(|(ea, _)| (t, ea))
        })
        .collect()
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

/// Wind-panel guide segment at a fixed chart row distance (AFM `hw_lines` value), not user weight.
pub fn wind_panel_segment(
    data: &Rev29File,
    entry_distance_m: f64,
    headwind_max: f64,
) -> ((f64, f64), (f64, f64)) {
    let y0 = y_dist(entry_distance_m);
    let y1 = y_dist(wind_panel_hw_m(data, entry_distance_m, headwind_max).0);
    ((x_headwind_kt(0.0), y0), (x_headwind_kt(headwind_max), y1))
}

/// Obstacle-panel guide: fixed ground-line entry from AFM `obs_lines`, fixed obstacle height.
pub fn obstacle_guide_ys(data: &Rev29File, ground_line_m: f64, obstacle_h_m: f64) -> (f64, f64) {
    (
        y_dist(ground_line_m),
        y_dist(obstacle_add_m(data, ground_line_m, obstacle_h_m).0),
    )
}

const TAKEOFF_GUIDE_OBS_MAX_M: f64 = 15.0;
const TAKEOFF_GUIDE_LINE_COUNT: usize = 8;
const TAKEOFF_TW_GUIDE_LINE_COUNT: usize = 7;
const TAKEOFF_OBS_GUIDE_COUNT: usize = 6;

/// Digitized Fig 5.4 guide geometry only — no flight inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct TakeoffPanelGuides {
    pub weight_polylines: Vec<Vec<(f64, f64)>>,
    pub headwind_polylines: Vec<Vec<(f64, f64)>>,
    pub tailwind_polylines: Vec<Vec<(f64, f64)>>,
    pub obstacle_ys: Vec<(f64, f64)>,
}

pub fn takeoff_panel_guides(data: &Rev29File) -> TakeoffPanelGuides {
    let weight_polylines = data
        .hw_lines
        .value
        .iter()
        .take(TAKEOFF_GUIDE_LINE_COUNT)
        .map(|&line_m| weight_guide_polyline(data, line_m))
        .collect();
    let headwind_polylines = data
        .hw_lines
        .value
        .iter()
        .take(TAKEOFF_GUIDE_LINE_COUNT)
        .map(|&line_m| headwind_guide_polyline(data, line_m))
        .collect();
    let tailwind_polylines = data
        .tw_lines
        .value
        .iter()
        .take(TAKEOFF_TW_GUIDE_LINE_COUNT)
        .map(|&line_m| tailwind_guide_polyline(data, line_m))
        .collect();
    let obstacle_ys = data
        .obs_lines
        .value
        .iter()
        .take(TAKEOFF_OBS_GUIDE_COUNT)
        .map(|&g_m| obstacle_guide_ys(data, g_m, TAKEOFF_GUIDE_OBS_MAX_M))
        .collect();
    TakeoffPanelGuides {
        weight_polylines,
        headwind_polylines,
        tailwind_polylines,
        obstacle_ys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rev29::engine::{fahrenheit_to_celsius, m_to_ft};
    use crate::types::FlightInputs;

    #[test]
    fn takeoff_panel_guides_are_static_chart_data() {
        let data = Rev29File::production();
        let guides = takeoff_panel_guides(&data);
        assert_eq!(guides, takeoff_panel_guides(&data));
        for (i, &line_m) in data
            .hw_lines
            .value
            .iter()
            .take(TAKEOFF_GUIDE_LINE_COUNT)
            .enumerate()
        {
            assert_eq!(
                guides.headwind_polylines[i],
                headwind_guide_polyline(&data, line_m)
            );
            assert_eq!(
                guides.weight_polylines[i],
                weight_guide_polyline(&data, line_m)
            );
        }
    }

    #[test]
    fn wind_guide_geometry_unaffected_by_weight_scaling_bug() {
        let data = Rev29File::production();
        let guides = takeoff_panel_guides(&data);
        let line_m = data.hw_lines.value[2];
        let (wf, _) = weight_factor(&data, 650.0);
        let (wf_chart, _) = weight_factor(&data, defaults::TAKEOFF_CHART_WEIGHT_KG);
        let wrong = headwind_guide_polyline(&data, line_m * wf / wf_chart);
        assert_ne!(guides.headwind_polylines[2], wrong);
    }

    #[test]
    fn panel_guides_same_while_path_entry_distance_changes_with_weight() {
        let data = Rev29File::production();
        let pa = 1000.0;
        let oat_a = 10.0;
        let oat_b = 35.0;
        let kg_a = lb_to_kg(1400.0);
        let kg_b = lb_to_kg(1700.0);
        assert_eq!(takeoff_panel_guides(&data), takeoff_panel_guides(&data));
        let a = takeoff_distances_m(&data, pa, oat_a, kg_a, 0.0, 0.0, 0.0);
        let b = takeoff_distances_m(&data, pa, oat_b, kg_b, 0.0, 0.0, 0.0);
        assert!((a.after_weight_m - b.after_weight_m).abs() > 1.0);
    }

    #[test]
    fn wind_and_obstacle_corrections_pure_in_entry_and_component() {
        let data = Rev29File::production();
        let entry = 450.0;
        let hw = 8.0;
        let out = wind_panel_hw_m(&data, entry, hw).0;
        assert_eq!(out, wind_panel_hw_m(&data, entry, hw).0);
        let g = 380.0;
        let h = 6.0;
        let obs = obstacle_add_m(&data, g, h).0;
        assert_eq!(obs, obstacle_add_m(&data, g, h).0);
        assert!((obstacle_add_m(&data, g, 0.0).0 - g).abs() < 1e-9);
    }

    #[test]
    fn afm_tailwind_panel_reference_distances() {
        let data = Rev29File::production();
        let (d331, _, _) = wind_panel_tw_m(&data, 300.0, 2.85);
        assert!(
            (d331 - 331.0).abs() <= 1.0,
            "300 m + 2.85 kt TW expected ~331 m, got {d331}"
        );
        let (d583, _, _) = wind_panel_tw_m(&data, 500.0, 4.84);
        assert!(
            (d583 - 583.0).abs() <= 1.0,
            "500 m + 4.84 kt TW expected ~583 m, got {d583}"
        );
    }

    #[test]
    fn path_wind_trace_overlays_guide_at_chart_entry_line() {
        let data = Rev29File::production();
        let entry = 300.0;
        let hw_kt = 20.0;
        let guide = headwind_guide_polyline(&data, entry);
        let anchor = (x_headwind_kt(0.0), y_dist(entry));
        let trace = trace_headwind_guide(&data, entry, hw_kt, anchor);
        assert!(!guide.is_empty() && !trace.is_empty());
        for pt in &trace {
            let on_guide = guide
                .windows(2)
                .any(|w| point_near_segment(*pt, w[0], w[1], 0.75));
            assert!(on_guide, "trace point {:?} off guide", pt);
        }
    }

    fn point_near_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64), eps: f64) -> bool {
        let (px, py) = p;
        let (ax, ay) = a;
        let (bx, by) = b;
        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-9 {
            return (px - ax).hypot(py - ay) <= eps;
        }
        let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let qx = ax + t * dx;
        let qy = ay + t * dy;
        (px - qx).hypot(py - qy) <= eps
    }

    #[test]
    fn climb_fan_pa_curve_point_lies_on_strip_polyline_for_input_pa() {
        let data = Rev29File::production();
        let pa = 2500.0_f64;
        let pts = climb_fan_pa_curve_strip_points(&data, pa);
        assert!(pts.len() >= 2, "fan strip needs ≥2 OAT samples");

        for &t in data.fan_t.value.iter() {
            let (x, y) = climb_fan_pa_curve_point_at_oat(&data, pa, t);
            let (ea, _) = equivalent_altitude_ft(&data, pa, t);
            let want_y = y_ea_ft(ea);
            assert!(
                (x - x_oat(t)).abs() < 1e-6,
                "curve point x should match OAT axis at sample OAT {t}"
            );
            assert!(
                (y - want_y).abs() < 0.05,
                "at OAT {t}°C expected EA row y≈{want_y}, got {y}"
            );
        }

        let t_mid = (data.fan_t.value[0] + data.fan_t.value[1]) * 0.5;
        let p = climb_fan_pa_curve_point_at_oat(&data, pa, t_mid);
        assert!((p.0 - x_oat(t_mid)).abs() < 1e-6);
        let on_curve = pts
            .windows(2)
            .any(|w| point_near_segment(p, w[0], w[1], 0.75));
        assert!(on_curve, "mid-OAT point {:?} off PA fan polyline", p);
    }

    #[test]
    fn climb_fan_pa_live_row_matches_full_strip_isoline() {
        let data = Rev29File::production();
        let pa = 2500.0_f64;
        let full = climb_fan_pa_curve_strip_points(&data, pa);
        assert!(full.len() >= 2);
        let oat = 12.0_f64;
        let cursor = climb_fan_pa_curve_point_at_oat(&data, pa, oat);
        assert!(
            full.iter()
                .any(|&v| (v.0 - cursor.0).hypot(v.1 - cursor.1) < 0.75)
                || full
                    .windows(2)
                    .any(|w| point_near_segment(cursor, w[0], w[1], 0.75))
        );
        assert!((full.first().unwrap().0 - x_oat(*data.fan_t.value.first().unwrap())).abs() < 1e-6);
        assert!((full.last().unwrap().0 - x_oat(*data.fan_t.value.last().unwrap())).abs() < 1e-6);
    }

    #[test]
    fn climb_fan_pa_curve_chart_matches_strip_full_isoline() {
        let data = Rev29File::production();
        let pa = 2500.0_f64;
        let strip_pts = climb_fan_pa_curve_strip_points(&data, pa);
        let chart_pts = climb_fan_pa_curve_chart_points(&data, pa);
        assert_eq!(chart_pts.len(), strip_pts.len());
        for ((t, ea), strip) in chart_pts.iter().zip(strip_pts.iter()) {
            let mapped = (x_oat(*t), y_ea_ft(*ea));
            assert!(
                (mapped.0 - strip.0).abs() < 1e-3 && (mapped.1 - strip.1).abs() < 1e-3,
                "chart-mapped {:?} vs strip {:?}",
                mapped,
                strip
            );
        }
    }

    #[test]
    fn tailwind_panel_continuity_and_magnitude_2_to_5_kt() {
        let data = Rev29File::production();
        let entry = 450.0;
        let d0 = wind_panel_tw_m(&data, entry, 0.0).0;
        assert!((d0 - entry).abs() < 2.0);
        let mut prev = d0;
        let tw_max = *data.tw_kt.value.last().expect("tw kt");
        for tw in [2.0, 3.0, 4.0, tw_max] {
            let d = wind_panel_tw_m(&data, entry, tw).0;
            assert!(d >= prev - 0.5, "TW {tw} kt should not shorten distance");
            let delta = d - prev;
            assert!(delta < 35.0, "per-step jump {delta} m too large at {tw} kt");
            prev = d;
        }
        let d_max = wind_panel_tw_m(&data, entry, tw_max).0;
        assert!(
            d_max - entry < 120.0,
            "total TW correction should stay on chart scale"
        );
    }

    #[test]
    fn takeoff_ground_roll_increases_monotonically_with_tailwind_0_to_5_kt() {
        let data = Rev29File::production();
        let pa = 2500.0;
        let oat = 15.0;
        let kg = 772.0;
        let mut prev = takeoff_distances_m(&data, pa, oat, kg, 0.0, 0.0, 0.0).ground_roll_m;
        let tw_max = *data.tw_kt.value.last().expect("tw kt");
        let steps = (tw_max * 10.0).round() as i32;
        for step in 1..=steps {
            let tw = step as f64 / 10.0;
            let dist = takeoff_distances_m(&data, pa, oat, kg, 0.0, tw, 0.0).ground_roll_m;
            assert!(
                dist >= prev - 1e-9,
                "tailwind {tw} kt: ground roll {dist} m decreased from {prev} m"
            );
            prev = dist;
        }
    }

    #[test]
    fn takeoff_wind_strip_zero_kt_shared_at_tw_hw_boundary() {
        let x = tailwind_reference_x();
        assert!((x - TAKEOFF_HW.x).abs() < 1e-9);
        assert!((tailwind_kt_from_strip_x(x) - headwind_kt_from_strip_x(x)).abs() < 1e-9);
    }

    #[test]
    fn takeoff_tailwind_hand_path_follows_tw_panel_guide_to_core_distance() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            oat_c: 15.0,
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 4.0,
            weight_lb: 1700.0,
            obstacle_height_m: 0.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let (hw, tw) = headwind_and_tailwind(&inputs);
        assert!(takeoff_wind_is_tailwind(hw, tw));
        let path = takeoff_hand_path(&data, &inputs, 2500.0);
        assert!((path.chart_ground_m - path.core_ground_m).abs() < 0.05);
        let core = takeoff_distances_m(
            &data,
            2500.0,
            inputs.oat_c,
            lb_to_kg(inputs.weight_lb),
            hw,
            tw,
            0.0,
        );
        assert!(path.core_ground_m > core.after_weight_m);
    }

    #[test]
    fn takeoff_distance_reasonable_with_tailwind_and_headwind_cases() {
        use crate::data::PerformanceData;
        use crate::takeoff::compute_takeoff;
        let data = PerformanceData::production();
        let tw_case = FlightInputs {
            oat_c: 15.0,
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 4.0,
            weight_lb: 1700.0,
            obstacle_height_m: 0.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let tw = compute_takeoff(&data, &tw_case).expect("tw");
        assert!(
            tw.distance_over_obstacle_raw_ft < 2500.0,
            "raw {} ft",
            tw.distance_over_obstacle_raw_ft
        );
        let hw_case = FlightInputs {
            oat_c: 15.0,
            runway: "360".into(),
            wind_dir_deg: 360.0,
            wind_speed_kt: 8.0,
            weight_lb: 1700.0,
            obstacle_height_m: 0.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let hw = compute_takeoff(&data, &hw_case).expect("hw");
        assert!(
            hw.distance_over_obstacle_raw_ft < 2500.0,
            "raw {} ft (regression: chart-top snap)",
            hw.distance_over_obstacle_raw_ft
        );
    }

    #[test]
    fn tailwind_over_5_kt_is_out_of_chart() {
        let data = Rev29File::production();
        assert!(crate::rev29::engine::wind_panel_tw_m(&data, 400.0, 8.0).is_err());
    }

    #[test]
    fn tailwind_panel_max_kt_on_chart_no_limit_warning() {
        use crate::rev29::engine::TAILWIND_PANEL_MAX_KT;
        let data = Rev29File::production();
        let (_, _, w) = wind_panel_tw_m(&data, 400.0, TAILWIND_PANEL_MAX_KT);
        assert!(
            !w.iter().any(|x| x.code == "TAILWIND_CHART_LIMIT"),
            "5 kt is on the Fig 5.4 TW sub-panel scale"
        );
    }

    #[test]
    fn along_runway_shows_headwind_or_tailwind_not_both() {
        use crate::wind::{along_runway_display_kt, wind_components};
        let hw_c = wind_components(350.0, 8.0, 360.0);
        let (is_hw, kt) = along_runway_display_kt(&hw_c);
        assert!(is_hw && kt > 0.5);
        let tw_c = wind_components(10.0, 5.0, 190.0);
        let (is_hw, kt) = along_runway_display_kt(&tw_c);
        assert!(!is_hw && kt > 0.5);
    }

    #[test]
    fn takeoff_oat_panel_pa_trace_y_matches_engine_base_read() {
        let data = Rev29File::production();
        for pa_ft in [500.0, 2500.0, 5000.0] {
            for oat_c in [-10.0, 15.0, 40.0] {
                let trace = takeoff_oat_panel_pa_trace(&data, pa_ft, oat_c);
                assert_eq!(trace.pa_ft, pa_ft);
                let base = takeoff_base_m(&data, pa_ft, oat_c).0;
                assert!(
                    (trace.y - y_dist(base)).abs() < 0.01,
                    "pa={pa_ft} oat={oat_c}: y {} vs {}",
                    trace.y,
                    y_dist(base)
                );
                assert!((trace.oat_x - x_oat(oat_c)).abs() < 0.01);
            }
        }
    }

    #[test]
    fn takeoff_path_endpoints_match_core_distances() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pressure_altitude_input_ft: 1000.0,
            oat_c: fahrenheit_to_celsius(72.0),
            weight_lb: 1600.0,
            wind_dir_deg: 0.0,
            wind_speed_kt: 4.0,
            runway: "360".into(),
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
    fn climb_chart_readout_matches_roc_line_at_ea() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 3500.0,
                phase: crate::types::ChartPhase::Climb,
            }),
            oat_c: 25.0,
            ..Default::default()
        };
        let key = inputs.climb_figure.roc_key();
        let path = climb_hand_path(&data, &inputs, 3500.0).expect("climb");
        let (ea, _) = equivalent_altitude_ft(&data, 3500.0, inputs.oat_c);
        let y_ea = y_ea_ft(ea);
        let line_roc = chart_roc_fpm_at_ea(&data, key, ea);
        assert!((path.chart_roc_fpm - line_roc).abs() < 0.01);
        assert!((path.chart_ea_ft - ea).abs() < 0.01);
        let hit = path.points[path.points.len() - 2];
        let axis = path.points.last().copied().expect("path end");
        assert!((hit.0 - x_roc_fpm(line_roc)).abs() < 0.5);
        assert!((hit.1 - y_ea).abs() < 0.5);
        assert!((axis.0 - hit.0).abs() < 0.01);
        assert!((axis.1 - roc_panel_x_axis_y()).abs() < 0.5);
        assert!((path.chart_roc_fpm - roc_fpm_from_strip_x(hit.0)).abs() < 0.01);
    }

    #[test]
    fn climb_chart_readout_within_one_percent_of_core() {
        let data = Rev29File::production();
        let cases = [
            (2130.0, 20.0),
            (2500.0, 18.0),
            (3500.0, 25.0),
            (6500.0, 15.0),
        ];
        for (pa_ft, oat_c) in cases {
            let inputs = FlightInputs {
                pa_override: Some(crate::types::PaOverride {
                    pa_ft,
                    phase: crate::types::ChartPhase::Climb,
                }),
                oat_c,
                ..Default::default()
            };
            let path = climb_hand_path(&data, &inputs, pa_ft).expect("climb");
            let (ea, _) = equivalent_altitude_ft(&data, pa_ft, oat_c);
            assert!(
                (path.chart_ea_ft - ea).abs() < 1.0,
                "pa={pa_ft} oat={oat_c}: chart EA {} vs fan EA {ea}",
                path.chart_ea_ft
            );
            assert!(
                (path.chart_roc_fpm - path.core_roc_fpm).abs() / path.core_roc_fpm.max(1.0) < 0.01,
                "pa={pa_ft} oat={oat_c}: chart ROC {} vs core {}",
                path.chart_roc_fpm,
                path.core_roc_fpm
            );
        }
    }

    #[test]
    fn climb_roc_panel_shares_ea_y_mapper_with_fan_chart() {
        let data = Rev29File::production();
        let key = FlightInputs::default().climb_figure.roc_key();
        let pl = crate::rev29::charts::climb_roc_vs_ea(&data, key);
        for (ea, roc) in &pl.points {
            let y = y_ea_ft(*ea);
            let x = x_roc_fpm(*roc);
            assert!((ea_ft_from_strip_y(y) - ea).abs() < 5.0);
            assert!((roc_fpm_from_strip_x(x) - roc).abs() < 1.0);
        }
    }

    #[test]
    fn climb_horizontal_transfer_meets_roc_line() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Climb,
            }),
            oat_c: 18.0,
            ..Default::default()
        };
        let key = inputs.climb_figure.roc_key();
        let path = climb_hand_path(&data, &inputs, 2500.0).expect("climb");
        let (ea, _) = equivalent_altitude_ft(&data, 2500.0, inputs.oat_c);
        let pts = &path.points;
        assert!(pts.len() >= 3);
        let transfer_from = pts[pts.len() - 3];
        let hit = pts[pts.len() - 2];
        let axis = pts[pts.len() - 1];
        assert!(
            (transfer_from.1 - hit.1).abs() < 0.01,
            "transfer onto ROC line must be horizontal (equal y): {transfer_from:?} → {hit:?}"
        );
        assert!(
            hit.0 > transfer_from.0,
            "transfer must move right into the ROC panel"
        );
        assert!(
            (hit.0 - axis.0).abs() < 0.01,
            "x-axis drop must be vertical (equal x): {hit:?} → {axis:?}"
        );
        assert!(
            (axis.1 - roc_panel_x_axis_y()).abs() < 0.5,
            "readout point must sit on the ROC panel x-axis"
        );
        let roc_at_hit = chart_roc_fpm_at_ea(&data, key, ea);
        assert!((path.chart_roc_fpm - roc_at_hit).abs() < 0.01);
        assert!((hit.0 - x_roc_fpm(roc_at_hit)).abs() < 0.5);
        assert!((hit.1 - y_ea_ft(ea)).abs() < 0.5);
        assert!((path.chart_roc_fpm - roc_fpm_from_strip_x(axis.0)).abs() < 0.01);
    }

    #[test]
    fn climb_path_segments_axis_aligned_except_along_roc_line() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Climb,
            }),
            oat_c: 18.0,
            ..Default::default()
        };
        let path = climb_hand_path(&data, &inputs, 2500.0).expect("climb");
        let (ea, _) = equivalent_altitude_ft(&data, 2500.0, inputs.oat_c);
        let pts = &path.points;
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let dx = (b.0 - a.0).abs();
            let dy = (b.1 - a.1).abs();
            let axis_aligned = dx < 0.01 || dy < 0.01;
            assert!(
                axis_aligned,
                "segment ({a:?})→({b:?}) must be horizontal or vertical, not diagonal"
            );
        }
        let end = pts.last().copied().expect("end");
        let hit = pts[pts.len() - 2];
        assert!((end.0 - hit.0).abs() < 0.01);
        assert!((end.1 - roc_panel_x_axis_y()).abs() < 0.5);
        assert!((path.chart_ea_ft - ea).abs() < 1.0);
    }

    #[test]
    fn climb_path_stores_fan_ea_and_intersection_roc() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2000.0,
                phase: crate::types::ChartPhase::Climb,
            }),
            oat_c: fahrenheit_to_celsius(65.0),
            ..Default::default()
        };
        let key = inputs.climb_figure.roc_key();
        let path = climb_hand_path(&data, &inputs, 2000.0).expect("roc");
        let fan_ea = equivalent_altitude_ft(&data, 2000.0, inputs.oat_c).0;
        assert!((path.chart_ea_ft - fan_ea).abs() < 0.01);
        assert!((path.chart_roc_fpm - chart_roc_fpm_at_ea(&data, key, fan_ea)).abs() < 0.01);
        assert!(path.core_roc_fpm > 0.0);
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
        assert!(
            core.ground_roll_m.is_finite(),
            "in-chart inputs required for path geometry test: {inputs:?}"
        );
        let path = takeoff_hand_path(data, &inputs, pa_ft);

        let wt_exit_y = y_dist(core.after_weight_m);
        let wind_x = takeoff_wind_panel_entry_x(hw, tw);
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
                runway: "360".into(),
                obstacle_height_m: 8.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 2500.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
            FlightInputs {
                oat_c: 5.0,
                weight_lb: 1700.0,
                wind_dir_deg: 360.0,
                wind_speed_kt: 4.0,
                runway: "360".into(),
                obstacle_height_m: 8.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 500.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
            FlightInputs {
                oat_c: 21.0,
                weight_lb: 1598.0,
                wind_dir_deg: 360.0,
                wind_speed_kt: 8.0,
                runway: "360".into(),
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
                runway: "090".into(),
                obstacle_height_m: 0.0,
                pa_override: Some(crate::types::PaOverride {
                    pa_ft: 3500.0,
                    phase: crate::types::ChartPhase::Takeoff,
                }),
                ..Default::default()
            },
        ];
        for (idx, inputs) in cases.into_iter().enumerate() {
            let pa = inputs
                .pa_override
                .as_ref()
                .map(|o| o.pa_ft)
                .unwrap_or(2130.0);
            panel_y_continuity(&data, inputs, pa);
            let _ = idx;
        }
    }

    #[test]
    fn tailwind_subpanel_x_disjoint_from_weight_panel() {
        let (tw_lo, tw_hi) = tailwind_subpanel_x_range();
        let wt_hi = TAKEOFF_WEIGHT.x + TAKEOFF_WEIGHT.w;
        assert!(
            tw_lo >= wt_hi - 0.01,
            "TW sub-panel [{tw_lo},{tw_hi}] must not overlap weight panel ending at {wt_hi}"
        );
        for tw in [0.0, 1.0, 2.5, 5.0] {
            let x = x_tailwind_kt(tw);
            assert!(x >= tw_lo && x <= tw_hi + 0.01);
        }
        assert!((tailwind_reference_x() - tw_hi).abs() < 0.01);
    }

    #[test]
    fn takeoff_tailwind_path_x_monotonic_outside_tw_subpanel() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            oat_c: 15.0,
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 4.0,
            weight_lb: 1700.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let (hw, tw) = headwind_and_tailwind(&inputs);
        assert!(takeoff_wind_is_tailwind(hw, tw));
        let path = takeoff_hand_path(&data, &inputs, 2500.0);
        let (tw_lo, tw_hi) = tailwind_subpanel_x_range();
        for w in path.points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let in_tw = |p: (f64, f64)| p.0 >= tw_lo - 0.5 && p.0 <= tw_hi + 0.5;
            if in_tw(a) && in_tw(b) {
                continue;
            }
            assert!(
                b.0 >= a.0 - 0.01,
                "x must not decrease outside TW sub-panel: ({a:?})→({b:?})"
            );
        }
    }

    #[test]
    fn takeoff_runway_190_wind_010_05_tailwind_case() {
        let inputs = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 5.0,
            ..Default::default()
        };
        use crate::wind::{headwind_tailwind_kt, runway_heading_deg, wind_components};
        let rwy = runway_heading_deg(&inputs.runway).expect("rwy");
        let c = wind_components(inputs.wind_dir_deg, inputs.wind_speed_kt, rwy);
        let (hw, tw) = headwind_tailwind_kt(&c);
        assert!(takeoff_wind_is_tailwind(hw, tw), "hw={hw} tw={tw}");
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
            runway: "360".into(),
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

    fn assert_only_oat_riser_vertical(path: &TakeoffPath, case: &str) {
        for (i, w) in path.points.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let vertical = (b.0 - a.0).abs() < 0.01 && (b.1 - a.1).abs() > 0.5;
            if vertical && i != 0 {
                panic!("{case}: unexpected vertical segment {a:?} -> {b:?}");
            }
        }
    }

    #[test]
    fn takeoff_path_only_oat_riser_is_vertical() {
        let data = Rev29File::production();
        let cases = [
            (
                "tailwind",
                FlightInputs {
                    oat_c: 15.0,
                    runway: "190".into(),
                    wind_dir_deg: 10.0,
                    wind_speed_kt: 4.0,
                    weight_lb: 800.0 / 0.45359237,
                    pa_override: Some(crate::types::PaOverride {
                        pa_ft: 2500.0,
                        phase: crate::types::ChartPhase::Takeoff,
                    }),
                    ..Default::default()
                },
            ),
            (
                "zero_hw",
                FlightInputs {
                    runway: "360".into(),
                    wind_dir_deg: 0.0,
                    wind_speed_kt: 0.0,
                    weight_lb: 750.0 / 0.45359237,
                    ..Default::default()
                },
            ),
            (
                "max_hw",
                FlightInputs {
                    runway: "360".into(),
                    wind_dir_deg: 360.0,
                    wind_speed_kt: 20.0,
                    weight_lb: 750.0 / 0.45359237,
                    ..Default::default()
                },
            ),
            (
                "clamp_entry",
                FlightInputs {
                    oat_c: 35.0,
                    weight_lb: 600.0 / 0.45359237,
                    wind_speed_kt: 10.0,
                    wind_dir_deg: 360.0,
                    runway: "360".into(),
                    ..Default::default()
                },
            ),
        ];
        for (name, inputs) in cases {
            let path = takeoff_hand_path(&data, &inputs, 2500.0);
            assert_only_oat_riser_vertical(&path, name);
        }
    }

    #[test]
    fn takeoff_tailwind_trace_follows_interpolated_guides() {
        let data = Rev29File::production();
        let entry = 526.5;
        let kt = 3.0;
        let p = tailwind_panel_point_interp(&data, entry, kt);
        let p_lo = tailwind_panel_point(&data, 500.0, kt);
        let p_hi = tailwind_panel_point(&data, 600.0, kt);
        let t = (entry - 500.0) / 100.0;
        let want_x = p_lo.0 + t * (p_hi.0 - p_lo.0);
        let want_y = p_lo.1 + t * (p_hi.1 - p_lo.1);
        assert!((p.0 - want_x).abs() < 0.01 && (p.1 - want_y).abs() < 0.01);
    }

    #[test]
    fn takeoff_lighter_weight_needs_less_distance_same_wind() {
        use crate::data::PerformanceData;
        use crate::takeoff::compute_takeoff;
        let data = Rev29File::production();
        let pdata = PerformanceData::production();
        let base = FlightInputs {
            oat_c: 20.0,
            runway: "360".into(),
            wind_dir_deg: 0.0,
            wind_speed_kt: 0.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let mut light = base.clone();
        light.weight_lb = 650.0 / 0.45359237;
        let mut heavy = base;
        heavy.weight_lb = 800.0 / 0.45359237;
        let pl = takeoff_hand_path(&data, &light, 2500.0);
        let ph = takeoff_hand_path(&data, &heavy, 2500.0);
        assert!(
            pl.chart_over_obstacle_m <= ph.chart_over_obstacle_m + 1e-6,
            "650 kg obs {} vs 800 kg {}",
            pl.chart_over_obstacle_m,
            ph.chart_over_obstacle_m
        );
        let tl = compute_takeoff(&pdata, &light).unwrap();
        let th = compute_takeoff(&pdata, &heavy).unwrap();
        assert!(tl.distance_over_obstacle_raw_m <= th.distance_over_obstacle_raw_m);
        assert!((pl.chart_over_obstacle_m - tl.distance_over_obstacle_raw_m).abs() < 0.05);
        assert!((ph.chart_over_obstacle_m - th.distance_over_obstacle_raw_m).abs() < 0.05);
    }

    #[test]
    fn takeoff_path_endpoint_y_matches_chart_ground_roll() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            oat_c: 20.0,
            weight_lb: 800.0 / 0.45359237,
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 4.0,
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let path = takeoff_hand_path(&data, &inputs, 2500.0);
        let wind_right = TAKEOFF_WIND.x + TAKEOFF_WIND.w - 0.5;
        let wind_end = path
            .points
            .iter()
            .rfind(|(x, _)| *x >= TAKEOFF_TW.x && *x <= wind_right)
            .copied()
            .expect("wind");
        let want_y = y_dist(path.chart_ground_m);
        assert!(
            (wind_end.1 - want_y).abs() < 0.75,
            "wind exit y {} vs ground roll y {want_y}",
            wind_end.1
        );
    }

    #[test]
    fn takeoff_weight_trace_follows_interpolated_guides() {
        let data = Rev29File::production();
        let entry = 526.5;
        let p = weight_panel_point_interp(&data, entry, 700.0);
        let p_lo = weight_panel_point(&data, 500.0, 700.0);
        let p_hi = weight_panel_point(&data, 600.0, 700.0);
        let t = (entry - 500.0) / 100.0;
        let want_x = p_lo.0 + t * (p_hi.0 - p_lo.0);
        let want_y = p_lo.1 + t * (p_hi.1 - p_lo.1);
        assert!((p.0 - want_x).abs() < 0.01 && (p.1 - want_y).abs() < 0.01);
    }

    #[test]
    fn takeoff_wind_path_ends_at_entered_component_no_flat_tail() {
        let data = Rev29File::production();
        let inputs = FlightInputs {
            oat_c: 15.0,
            weight_lb: 1700.0,
            obstacle_height_m: 0.0,
            wind_dir_deg: 360.0,
            wind_speed_kt: 12.0,
            runway: "360".into(),
            pa_override: Some(crate::types::PaOverride {
                pa_ft: 2500.0,
                phase: crate::types::ChartPhase::Takeoff,
            }),
            ..Default::default()
        };
        let (hw, _) = headwind_and_tailwind(&inputs);
        let path = takeoff_hand_path(&data, &inputs, 2500.0);
        let want_x = x_headwind_kt(hw);
        let wind_right = TAKEOFF_WIND.x + TAKEOFF_WIND.w - 0.5;
        let last_in_wind = path
            .points
            .iter()
            .rfind(|(x, _)| *x >= TAKEOFF_TW.x && *x <= wind_right)
            .copied()
            .expect("wind segment");
        assert!(
            (last_in_wind.0 - want_x).abs() < 0.01,
            "last wind x {} vs entered {want_x}",
            last_in_wind.0
        );
        for w in path.points.windows(2) {
            let (a, b) = (w[0], w[1]);
            if a.0 >= TAKEOFF_TW.x
                && b.0 <= wind_right
                && a.0 >= TAKEOFF_TW.x
                && (b.1 - a.1).abs() < 0.01
                && (b.0 - a.0).abs() > 1.0
            {
                panic!("flat horizontal segment inside wind panel: {a:?} -> {b:?}");
            }
        }
    }

    #[test]
    fn takeoff_pa_curve_spans_oat_panel_and_passes_cursor() {
        let data = Rev29File::production();
        let pa = 2500.0;
        let oat = 15.0;
        let pts = takeoff_pa_curve_strip_points(&data, pa);
        assert!(pts.len() >= 2);
        assert!((pts.first().unwrap().0 - TAKEOFF_OAT.x).abs() < 25.0);
        assert!((pts.last().unwrap().0 - (TAKEOFF_OAT.x + TAKEOFF_OAT.w)).abs() < 25.0);
        let cursor = takeoff_pa_curve_point_at_oat(&data, pa, oat);
        let on_curve = pts
            .windows(2)
            .any(|w| point_near_segment(cursor, w[0], w[1], 0.75));
        assert!(on_curve, "cursor {:?} off PA isoline", cursor);
    }

    #[test]
    fn takeoff_pa_curve_strip_finite_at_high_pa_rows() {
        let data = Rev29File::production();
        for pa in [5999.0, 6000.0, 7000.0, 8000.0, 10_000.0] {
            let pts = takeoff_pa_curve_strip_points(&data, pa);
            assert!(!pts.is_empty(), "PA {pa} ft: no strip points");
            for (x, y) in &pts {
                assert!(
                    x.is_finite() && y.is_finite(),
                    "PA {pa} ft: non-finite {x},{y}"
                );
            }
        }
    }

    #[test]
    fn takeoff_pa_curve_strip_ends_at_row_oat_envelope() {
        let data = Rev29File::production();
        let cases = [
            (6000.0, 55.0),
            (7000.0, 35.0),
            (8000.0, 35.0),
            (10_000.0, 15.0),
        ];
        for (pa, last_oat) in cases {
            let pts = takeoff_pa_curve_strip_points(&data, pa);
            let last_x = pts.last().unwrap().0;
            assert!(
                (last_x - x_oat(last_oat)).abs() < 0.05,
                "PA {pa} ft: last x {last_x} vs OAT {last_oat}"
            );
        }
    }

    #[test]
    fn climb_fan_pa_curve_strip_finite_on_printed_pa_row() {
        let data = Rev29File::production();
        let pts = climb_fan_pa_curve_strip_points(&data, 6000.0);
        assert!(!pts.is_empty());
        for (x, y) in &pts {
            assert!(x.is_finite() && y.is_finite());
        }
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
