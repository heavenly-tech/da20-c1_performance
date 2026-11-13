//! Multi-panel hand-plot strips (Fig 5.4 takeoff, Fig 5.2 → ROC climb).

use std::collections::HashSet;

use da20_core::altitude::effective_pressure_altitude_ft;
use da20_core::chart_bounds::limits;
use da20_core::climb::compute_climb;
use da20_core::rev29::charts::{climb_roc_vs_ea, equivalent_altitude_curves, takeoff_base_curves};
use da20_core::rev29::engine::lb_to_kg;
use da20_core::rev29::hand_plot::{
    back_solve_takeoff_wind_from_strip_x, chart_read_takeoff_base_m,
    climb_fan_pa_curve_point_at_oat, climb_fan_pa_curve_strip_points, climb_hand_path,
    climb_pa_isoline_y_at_panel_right, headwind_and_tailwind, tailwind_reference_x,
    takeoff_hand_path, takeoff_pa_curve_point_at_oat, takeoff_pa_curve_strip_points,
    takeoff_pa_isoline_y_at_panel_right, takeoff_panel_guides, takeoff_wind_is_tailwind,
    x_headwind_kt, x_oat, x_obstacle_m, x_roc_fpm, x_tailwind_kt, x_weight_kg, y_dist, y_ea_ft,
    CLIMB_EQ, CLIMB_ROC, STRIP_BOTTOM, STRIP_TOP, TAKEOFF_HW, TAKEOFF_OAT, TAKEOFF_OBS, TAKEOFF_TW,
    TAKEOFF_WEIGHT, TAKEOFF_WIND,
};
use da20_core::rev29::types::Rev29File;
use da20_core::takeoff::compute_takeoff;
use da20_core::types::{defaults, AfmLookupTag, ChartPhase, FlightInputs};
use da20_core::wind::runway_heading_status;
use da20_core::PerformanceData;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::chart_axes::{
    climb_strip_axis_ticks, climb_strip_major_grid, climb_strip_panel_titles,
    takeoff_strip_major_grid, takeoff_strip_panel_titles, takeoff_strip_x_axis_ticks,
};
use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::chart_inputs::round_after_chart_drag;
use crate::i18n::{
    copy, format_climb_readout_interactive_pair, format_out_of_chart, format_strip_chart_roll_obs,
    format_takeoff_conservative_estimate, format_takeoff_over_obstacle_result,
    format_takeoff_strip_pa_trace_label, Locale,
};
use crate::inputs::{
    shared_airfield_set_oat_c, shared_airfield_set_pressure_altitude,
    takeoff_apply_wind_from_strip, takeoff_set_obstacle_height_m, takeoff_set_weight_lb,
    AirfieldInputsMode,
};
use crate::phase_readouts::overlay_trace_var;
use crate::prefs::FlightPrefs;
use crate::storage::compute_all;
use crate::storage::ReadingsStore;

const STRIP_INPUT_CURSOR_DASH: &str = "5 4";
const OBSTACLE_GUIDE_LABEL_MIN_DY: f64 = 12.0;

/// Skip obstacle-panel `{NNN} m` labels that would stack on the same strip y.
pub(crate) fn obstacle_guide_label_y_allowed(y: f64, placed: &mut Vec<f64>) -> bool {
    if placed
        .iter()
        .any(|p| (p - y).abs() < OBSTACLE_GUIDE_LABEL_MIN_DY)
    {
        return false;
    }
    placed.push(y);
    true
}

fn strip_input_cursor(x: f64) -> impl IntoView {
    view! {
        <line
            x1=x
            y1=STRIP_TOP
            x2=x
            y2=STRIP_BOTTOM
            class="chart-stroke-guide-active"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
    }
}

const STRIP_PA_LABEL_ABOVE_DY: f64 = 10.0;

fn strip_pa_label_at_panel_edge(
    x: f64,
    y: f64,
    label: String,
    anchor: LabelAnchor,
) -> impl IntoView {
    guide_line_label(x, y - STRIP_PA_LABEL_ABOVE_DY, label, anchor)
}

/// Fig 5.4: selected PA isoline (full panel width) + vertical OAT cursor; label at panel edge.
fn strip_takeoff_pa_curve_guide(
    inner: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
    label: String,
) -> impl IntoView {
    let cursor = takeoff_pa_curve_point_at_oat(inner, pa_ft, oat_c);
    let pts: String = takeoff_pa_curve_strip_points(inner, pa_ft)
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    let (cx, _cy) = cursor;
    let iso = takeoff_pa_curve_strip_points(inner, pa_ft);
    let (edge_x, edge_y) = iso.last().copied().unwrap_or((
        TAKEOFF_OAT.x + TAKEOFF_OAT.w,
        takeoff_pa_isoline_y_at_panel_right(inner, pa_ft),
    ));
    view! {
        <polyline
            points=pts
            fill="none"
            class="chart-stroke-pa-isoline"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        <line
            x1=cx
            y1=STRIP_TOP
            x2=cx
            y2=STRIP_BOTTOM
            class="chart-stroke-guide-active"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        {strip_pa_label_at_panel_edge(
            edge_x,
            edge_y,
            label,
            LabelAnchor::End,
        )}
    }
}

/// Fig 5.2: PA row polyline from chart left **through the cursor** + vertical EA cursor + label at cursor.
fn strip_climb_ea_cursor_with_pa_row(
    inner: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
    label: String,
) -> impl IntoView {
    let cursor = climb_fan_pa_curve_point_at_oat(inner, pa_ft, oat_c);
    let pts: String = climb_fan_pa_curve_strip_points(inner, pa_ft)
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    let (cx, _cy) = cursor;
    let iso = climb_fan_pa_curve_strip_points(inner, pa_ft);
    let (edge_x, edge_y) = iso.last().copied().unwrap_or((
        CLIMB_EQ.x + CLIMB_EQ.w,
        climb_pa_isoline_y_at_panel_right(inner, pa_ft),
    ));
    view! {
        <polyline
            points=pts
            fill="none"
            class="chart-stroke-guide-active"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        <line
            x1=cx
            y1=STRIP_TOP
            x2=cx
            y2=STRIP_BOTTOM
            class="chart-stroke-guide-active"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        {strip_pa_label_at_panel_edge(edge_x, edge_y, label, LabelAnchor::End)}
    }
}

fn strip_climb_pa_curve_guide_overlay(
    inner: &Rev29File,
    pa_ft: f64,
    oat_c: f64,
    label: String,
    color: &'static str,
) -> impl IntoView {
    let cursor = climb_fan_pa_curve_point_at_oat(inner, pa_ft, oat_c);
    let pts: String = climb_fan_pa_curve_strip_points(inner, pa_ft)
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    let (cx, _cy) = cursor;
    let (edge_x, edge_y) = climb_fan_pa_curve_strip_points(inner, pa_ft)
        .last()
        .copied()
        .unwrap_or((
            CLIMB_EQ.x + CLIMB_EQ.w,
            climb_pa_isoline_y_at_panel_right(inner, pa_ft),
        ));
    view! {
        <polyline
            points=pts
            fill="none"
            stroke=color
            stroke-width="2.5"
            opacity="0.88"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        <line
            x1=cx
            y1=STRIP_TOP
            x2=cx
            y2=STRIP_BOTTOM
            stroke=color
            stroke-width="2.5"
            opacity="0.88"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        <text
            x=edge_x
            y=edge_y - STRIP_PA_LABEL_ABOVE_DY
            fill=color
            font-size="9"
            text-anchor="end"
            dominant-baseline="middle"
            opacity="0.92"
        >
            {label}
        </text>
    }
}

fn strip_takeoff_pa_curve_guide_overlay(
    inner: &Rev29File,
    pa_ft: f64,
    _oat_c: f64,
    label: String,
    color: &'static str,
) -> impl IntoView {
    let pts: String = takeoff_pa_curve_strip_points(inner, pa_ft)
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    let (edge_x, edge_y) = takeoff_pa_curve_strip_points(inner, pa_ft)
        .last()
        .copied()
        .unwrap_or((
            TAKEOFF_OAT.x + TAKEOFF_OAT.w,
            takeoff_pa_isoline_y_at_panel_right(inner, pa_ft),
        ));
    view! {
        <polyline
            points=pts
            fill="none"
            stroke=color
            stroke-width="2.5"
            opacity="0.88"
            stroke-dasharray=STRIP_INPUT_CURSOR_DASH
        />
        <text
            x=edge_x
            y=edge_y - STRIP_PA_LABEL_ABOVE_DY
            fill=color
            font-size="9"
            text-anchor="end"
            dominant-baseline="middle"
            opacity="0.92"
        >
            {label}
        </text>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StripKind {
    Takeoff,
    Climb,
}

#[component]
pub fn StripHandPlot(
    locale: RwSignal<Locale>,
    kind: StripKind,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
    #[prop(optional)] flight_prefs: Option<RwSignal<crate::prefs::FlightPrefs>>,
    #[prop(optional)] sync_phase: Option<crate::phases::Phase>,
    #[prop(optional)] store: Option<RwSignal<ReadingsStore>>,
    #[prop(optional)] phase_key: Option<String>,
    #[prop(optional)] overlay_ids: Option<RwSignal<HashSet<String>>>,
) -> impl IntoView {
    let dragging = RwSignal::new(false);

    let apply_strip_pointer = move |sx: f64, sy: f64| {
        let perf = data.get();
        if let (Some(fp), Some(phase)) = (flight_prefs, sync_phase) {
            let (mode, chart_phase) = crate::inputs::strip_airfield_target(phase);
            fp.update(|p| {
                inputs.update(|i| {
                    apply_strip_hit(kind, sx, sy, &perf, mode, chart_phase, i, p);
                });
            });
        } else {
            inputs.update(|i| {
                apply_strip_hit_nprefs(kind, sx, sy, &perf, i);
            });
        }
    };

    let on_pointer_down = move |ev: web_sys::PointerEvent| {
        dragging.set(true);
        if let Some((sx, sy)) = svg_coords(&ev) {
            apply_strip_pointer(sx, sy);
        }
        capture_pointer(&ev);
    };
    let on_pointer_up = move |ev: web_sys::PointerEvent| {
        if dragging.get() {
            inputs.update(round_after_chart_drag);
            if let (Some(fp), Some(phase)) = (flight_prefs, sync_phase) {
                fp.update(|p| {
                    crate::phase_state::persist_tab_inputs(phase, p, &inputs.get_untracked());
                    p.save();
                });
            }
        }
        dragging.set(false);
        release_pointer(&ev);
    };
    let on_pointer_move = move |ev: web_sys::PointerEvent| {
        if !dragging.get() {
            return;
        }
        if let Some((sx, sy)) = svg_coords(&ev) {
            apply_strip_pointer(sx, sy);
        }
    };

    view! {
        <div class="strip-wrap">
            <p class="hint">{move || copy(locale.get()).plot_hint}</p>
            <svg
                class="plot-svg strip-svg"
                viewBox=format!("0 0 {STRIP_VIEWBOX_W} {STRIP_VIEWBOX_H}")
                on:pointerdown=on_pointer_down
                on:pointerup=on_pointer_up
                on:pointerleave=move |_| {
                    if dragging.get() {
                        inputs.update(round_after_chart_drag);
                        if let (Some(fp), Some(phase)) = (flight_prefs, sync_phase) {
                            fp.update(|p| {
                                crate::phase_state::persist_tab_inputs(
                                    phase,
                                    p,
                                    &inputs.get_untracked(),
                                );
                                p.save();
                            });
                        }
                    }
                    dragging.set(false);
                }
                on:pointermove=on_pointer_move
            >
                {move || match kind {
                    StripKind::Takeoff => render_takeoff_strip(
                        locale.get(),
                        data.get(),
                        inputs.get(),
                        store,
                        phase_key.clone(),
                        overlay_ids,
                    )
                    .into_any(),
                    StripKind::Climb => render_climb_strip(
                        locale.get(),
                        data.get(),
                        inputs.get(),
                        store,
                        phase_key.clone(),
                        overlay_ids,
                    )
                    .into_any(),
                }}
            </svg>
            <p class="cursor-readout readout-text"
                class:results-muted=move || {
                    kind == StripKind::Takeoff
                        && runway_heading_status(&inputs.get().runway)
                            != da20_core::wind::RunwayHeadingStatus::Valid
                }>
                {move || readout_line(locale.get(), kind, data.get(), inputs.get())}
            </p>
        </div>
    }
}

/// Points of the Fig 5.4 user polyline. The strip view formats this list into
/// the `<polyline points>` attribute, so tests of this function cover the path
/// that is drawn.
pub fn takeoff_rendered_path_points(
    inner: &Rev29File,
    inputs: &FlightInputs,
    pa_ft: f64,
) -> Vec<(f64, f64)> {
    takeoff_hand_path(inner, inputs, pa_ft).points
}

pub fn format_svg_polyline(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn readout_line(
    locale: Locale,
    kind: StripKind,
    data: PerformanceData,
    inputs: FlightInputs,
) -> String {
    let c = copy(locale);
    let pa = effective_pressure_altitude_ft(
        &inputs,
        Some(match kind {
            StripKind::Takeoff => ChartPhase::Takeoff,
            StripKind::Climb => ChartPhase::Climb,
        }),
    );
    let inner = data.inner();
    match kind {
        StripKind::Takeoff => {
            if runway_heading_status(&inputs.runway) != da20_core::wind::RunwayHeadingStatus::Valid
            {
                return format!("{}: — | {}: —", c.plot_readout_chart, c.plot_readout_core);
            }
            let bundle = compute_all(&data, &inputs);
            if let Some(ooc) = &bundle.takeoff_ooc {
                let msg = format_out_of_chart(locale, ooc);
                return format!(
                    "{}: {msg} | {}: —",
                    c.plot_readout_chart, c.plot_readout_core
                );
            }
            let path = takeoff_hand_path(inner, &inputs, pa);
            let takeoff = compute_takeoff(&data, &inputs);
            let core = takeoff
                .as_ref()
                .map(|t| {
                    format!(
                        "{} | {}",
                        format_takeoff_over_obstacle_result(
                            locale,
                            t.distance_over_obstacle_raw_ft,
                            t.distance_over_obstacle_raw_m,
                        ),
                        format_takeoff_conservative_estimate(
                            locale,
                            t.distance_over_obstacle_conservative_ft,
                            t.distance_over_obstacle_conservative_m,
                        ),
                    )
                })
                .unwrap_or_else(|_| "—".into());
            let chart = format_strip_chart_roll_obs(
                locale,
                path.chart_ground_m,
                path.chart_over_obstacle_m,
            );
            format!(
                "{}: {chart} | {}: {core}",
                c.plot_readout_chart, c.plot_readout_core
            )
        }
        StripKind::Climb => {
            let bundle = compute_all(&data, &inputs);
            if let Some(ooc) = &bundle.climb_ooc {
                let msg = format_out_of_chart(locale, ooc);
                return format!(
                    "{}: {msg} | {}: —",
                    c.plot_readout_chart, c.plot_readout_core
                );
            }
            let path = climb_hand_path(inner, &inputs, pa);
            let core = compute_climb(&data, &inputs, inputs.cruise_pressure_alt_ft)
                .map(|r| {
                    let (_, penalized_line) = format_climb_readout_interactive_pair(
                        locale,
                        r.rate_of_climb_fpm,
                        r.rate_of_climb_chart_fpm,
                        inputs.wheel_fairings_installed,
                        r.afm_lookup_tag,
                    );
                    format!("{penalized_line} @ EA {:.0} ft", r.equivalent_altitude_ft)
                })
                .unwrap_or_else(|_| "—".into());
            match path {
                Some(p) => {
                    let (chart_roc_line, _) = format_climb_readout_interactive_pair(
                        locale,
                        p.chart_roc_fpm,
                        Some(p.chart_roc_fpm),
                        inputs.wheel_fairings_installed,
                        AfmLookupTag::Interpolated,
                    );
                    let chart = format!("{chart_roc_line} @ EA {:.0} ft", p.chart_ea_ft);
                    format!(
                        "{}: {chart} | {}: {core}",
                        c.plot_readout_chart, c.plot_readout_core
                    )
                }
                None => format!(
                    "{}: — | {}: {core}",
                    c.plot_readout_chart, c.plot_readout_core
                ),
            }
        }
    }
}

fn render_takeoff_strip(
    locale: Locale,
    data: PerformanceData,
    inputs: FlightInputs,
    store: Option<RwSignal<ReadingsStore>>,
    phase_key: Option<String>,
    overlay_ids: Option<RwSignal<HashSet<String>>>,
) -> impl IntoView {
    let c = copy(locale);
    let inner = data.inner();
    let pa = effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Takeoff));
    let path = takeoff_hand_path(inner, &inputs, pa);
    let base = chart_read_takeoff_base_m(inner, pa, inputs.oat_c);
    let oat_x = x_oat(inputs.oat_c);
    let (hw, tw) = headwind_and_tailwind(&inputs);
    let rwy_valid =
        runway_heading_status(&inputs.runway) == da20_core::wind::RunwayHeadingStatus::Valid;
    let tailwind_case = rwy_valid && takeoff_wind_is_tailwind(hw, tw);
    let wind_kt = if tailwind_case { tw } else { hw };
    let (wind_caption_lbl, wind_caption_kt) =
        takeoff_strip_wind_caption_parts(locale, hw, tw, rwy_valid);
    let kg = lb_to_kg(inputs.weight_lb);
    let guides = takeoff_panel_guides(inner);

    let base_curves = takeoff_base_curves(inner);
    let pa_axis = inner.to_pa.value.clone();
    let curve_paths = base_curves.iter().enumerate().map(|(idx, pl)| {
        let pts: String = pl
            .points
            .iter()
            .map(|(t, m)| {
                let sx = x_oat(*t);
                let sy = y_dist(*m);
                format!("{sx:.1},{sy:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let pa_label = pa_axis.get(idx).copied().unwrap_or(0.0);
        let (lx, ly) = pl
            .points
            .last()
            .map(|(t, m)| (x_oat(*t), y_dist(*m)))
            .unwrap_or((TAKEOFF_OAT.x, STRIP_TOP));
        view! {
            <>
                <polyline points=pts class="chart-stroke-line"/>
                {guide_line_label(
                    lx + 4.0,
                    ly,
                    format!("{pa_label:.0} ft"),
                    LabelAnchor::Start,
                )}
            </>
        }
    });

    let weight_guides = guides.weight_polylines.iter().map(|poly| {
        let pts: String = poly
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ");
        view! {
            <polyline points=pts class="chart-stroke-guide" opacity="0.7"/>
        }
    });

    let headwind_guides = guides.headwind_polylines.iter().map(|poly| {
        let pts: String = poly
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ");
        view! {
            <polyline points=pts class="chart-stroke-guide" opacity="0.7"/>
        }
    });

    let tailwind_guides = guides.tailwind_polylines.iter().map(|poly| {
        let pts: String = poly
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ");
        view! {
            <polyline points=pts class="chart-stroke-guide" opacity="0.7" stroke-dasharray="6 4"/>
        }
    });

    let obs_line_m = inner.obs_lines.value.clone();
    let obs_guide_count = guides.obstacle_ys.len();
    let mut obs_label_ys: Vec<f64> = Vec::new();
    let obs_guides = guides
        .obstacle_ys
        .iter()
        .zip(obs_line_m.iter())
        .enumerate()
        .map(|(idx, ((y0, y15), &g_m))| {
            let show_left = obstacle_guide_label_y_allowed(*y0, &mut obs_label_ys);
            let show_right = idx + 1 == obs_guide_count
                && obstacle_guide_label_y_allowed(*y15, &mut obs_label_ys);
            view! {
                <>
                    <line
                        x1=TAKEOFF_OBS.x y1=*y0
                        x2={TAKEOFF_OBS.x + TAKEOFF_OBS.w} y2=*y15
                        class="chart-stroke-guide" opacity="0.7"
                    />
                    {show_right.then(|| view! {
                        {guide_line_label(
                            TAKEOFF_OBS.x + TAKEOFF_OBS.w + 3.0,
                            *y15,
                            format!("{:.0} m", defaults::OBSTACLE_HEIGHT_MAX_M),
                            LabelAnchor::Start,
                        )}
                    })}
                    {show_left.then(|| view! {
                        {guide_line_label(
                            TAKEOFF_OBS.x - 3.0,
                            *y0,
                            format!("{g_m:.0} m"),
                            LabelAnchor::End,
                        )}
                    })}
                </>
            }
        });

    let path_pts = format_svg_polyline(&takeoff_rendered_path_points(inner, &inputs, pa));

    let pa_trace_label = format_takeoff_strip_pa_trace_label(locale, pa);

    view! {
        {panel_backgrounds(locale)}
        {takeoff_strip_major_grid()}
        {curve_paths.collect_view()}
        {strip_takeoff_pa_curve_guide(inner, pa, inputs.oat_c, pa_trace_label)}
        {weight_guides.collect_view()}
        <g class:results-muted=tailwind_case>
            {headwind_guides.collect_view()}
        </g>
        {tailwind_guides.collect_view()}
        {tailwind_case.then(|| view! {
            <>
                <rect x=TAKEOFF_TW.x y=STRIP_TOP width=TAKEOFF_TW.w height=STRIP_BOTTOM-STRIP_TOP
                    class="chart-panel" opacity="0.35"/>
                <line x1=tailwind_reference_x() y1=STRIP_TOP x2=tailwind_reference_x() y2=STRIP_BOTTOM
                    class="chart-stroke-guide" stroke-dasharray="5 4"/>
            </>
        })}
        {obs_guides.collect_view()}
        {strip_input_cursor(oat_x)}
        {strip_input_cursor(x_weight_kg(kg))}
        {strip_input_cursor(x_headwind_kt(hw))}
        {strip_input_cursor(x_tailwind_kt(tw))}
        {strip_input_cursor(x_obstacle_m(inputs.obstacle_height_m))}
        <polyline points=path_pts class="chart-stroke-user-path"/>
        {saved_path_overlays(
            locale,
            StripKind::Takeoff,
            data.clone(),
            store,
            phase_key,
            overlay_ids,
        )}
        {path.markers.iter().map(|(x,y)| view! {
            <circle cx=*x cy=*y r="6" class="chart-marker"/>
        }).collect_view()}
        {guide_line_label(
            x_weight_kg(defaults::TAKEOFF_CHART_WEIGHT_KG),
            STRIP_TOP + 14.0,
            format!("{:.0} kg", defaults::TAKEOFF_CHART_WEIGHT_KG),
            LabelAnchor::Middle,
        )}
        {guide_line_label(
            x_weight_kg(600.0),
            STRIP_TOP + 14.0,
            "600 kg".to_string(),
            LabelAnchor::Middle,
        )}
        {guide_line_label(
            if tailwind_case {
                tailwind_reference_x() + 3.0
            } else {
                TAKEOFF_HW.x + TAKEOFF_HW.w + 3.0
            },
            STRIP_TOP + 14.0,
            format!("{wind_kt:.0} kt"),
            LabelAnchor::Start,
        )}
        {takeoff_strip_x_axis_ticks(c)}
        <text x="70" y="28" fill="var(--chart-label)" font-size="11"
            opacity=move || if rwy_valid { 1.0 } else { 0.45 }>
            {c.strip_fig54_base}" "{format!("{base:.0}")}" m"
        </text>
        <text x="520" y="28" fill="var(--chart-label)" font-size="11"
            opacity=move || if rwy_valid { 1.0 } else { 0.45 }>
            {c.strip_fig54_wt}" "{format!("{kg:.0}")}" kg · "{wind_caption_lbl}" "{format!("{wind_caption_kt:.1}")}" kt"
        </text>
    }
}

fn render_climb_strip(
    locale: Locale,
    data: PerformanceData,
    inputs: FlightInputs,
    store: Option<RwSignal<ReadingsStore>>,
    phase_key: Option<String>,
    overlay_ids: Option<RwSignal<HashSet<String>>>,
) -> impl IntoView {
    let c = copy(locale);
    let inner = data.inner();
    let pa = effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Climb));
    let path = climb_hand_path(inner, &inputs, pa);
    let eq_curves = equivalent_altitude_curves(inner);
    let fan_pa = inner.fan_pa.value.clone();
    let eq_paths = eq_curves.iter().enumerate().map(|(idx, pl)| {
        let pts: String = pl
            .points
            .iter()
            .map(|(t, ea)| format!("{},{}", x_oat(*t), y_ea_ft(*ea)))
            .collect::<Vec<_>>()
            .join(" ");
        let pa_label = fan_pa.get(idx).copied().unwrap_or(0.0);
        let (lx, ly) = pl
            .points
            .last()
            .map(|(t, ea)| (x_oat(*t), y_ea_ft(*ea)))
            .unwrap_or((CLIMB_EQ.x, STRIP_TOP));
        view! {
            <>
                <polyline points=pts class="chart-stroke-line"/>
                {guide_line_label(
                    lx + 4.0,
                    ly,
                    format!("{pa_label:.0} ft"),
                    LabelAnchor::Start,
                )}
            </>
        }
    });

    let roc_pl = climb_roc_vs_ea(inner, inputs.climb_figure.roc_key());
    let roc_pts: String = roc_pl
        .points
        .iter()
        .map(|(ea, roc)| format!("{},{}", x_roc_fpm(*roc), y_ea_ft(*ea)))
        .collect::<Vec<_>>()
        .join(" ");

    let path_pts = path.as_ref().map(|p| {
        p.points
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    });

    let roc_label = match inputs.climb_figure {
        da20_core::types::ClimbFigure::Fig55CruiseFlaps => c.strip_fig55_roc,
        da20_core::types::ClimbFigure::Fig56TakeoffFlaps => c.strip_fig56_roc,
        da20_core::types::ClimbFigure::Fig58BalkedLanding => c.strip_fig58_roc,
    };
    let pa_label = format_takeoff_strip_pa_trace_label(locale, pa);
    let oat_x = x_oat(inputs.oat_c);

    view! {
        <rect x=CLIMB_EQ.x y=STRIP_TOP width=CLIMB_EQ.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        <rect x=CLIMB_ROC.x y=STRIP_TOP width=CLIMB_ROC.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        {climb_strip_panel_titles(c, roc_label)}
        {climb_strip_major_grid()}
        {eq_paths.collect_view()}
        <polyline points=roc_pts class="chart-stroke-line"/>
        {strip_climb_ea_cursor_with_pa_row(inner, pa, inputs.oat_c, pa_label)}
        {strip_input_cursor(oat_x)}
        {path.as_ref().and_then(|p| p.points.last().copied()).map(|(roc_x, _)| view! {
            {strip_input_cursor(roc_x)}
        })}
        {path_pts.map(|pts| view! {
            <>
                {saved_path_overlays(
                    locale,
                    StripKind::Climb,
                    data.clone(),
                    store,
                    phase_key.clone(),
                    overlay_ids,
                )}
                <polyline points=pts class="chart-stroke-user-path"/>
            </>
        })}
        {path.map(|p| p.markers.iter().map(|(x,y)| view! {
            <circle cx=*x cy=*y r="6" class="chart-marker"/>
        }).collect_view())}
        {climb_strip_axis_ticks(c)}
    }
}

fn saved_path_overlays(
    locale: Locale,
    kind: StripKind,
    data: PerformanceData,
    store: Option<RwSignal<ReadingsStore>>,
    phase_key: Option<String>,
    overlay_ids: Option<RwSignal<HashSet<String>>>,
) -> impl IntoView {
    let (store, phase_key, overlay_ids) = match (store, phase_key, overlay_ids) {
        (Some(s), Some(pk), Some(ids)) => (s, pk, ids),
        _ => {
            return ().into_any();
        }
    };
    view! {
        {move || {
            let inner = data.inner();
            let selected = overlay_ids.get();
            store
                .get()
                .for_phase(&phase_key)
                .iter()
                .enumerate()
                .filter(|(_, r)| selected.contains(&r.id))
                .filter_map(|(idx, r)| {
                    let chart_phase = match kind {
                        StripKind::Takeoff => ChartPhase::Takeoff,
                        StripKind::Climb => ChartPhase::Climb,
                    };
                    let pa = effective_pressure_altitude_ft(&r.inputs, Some(chart_phase));
                    let points = match kind {
                        StripKind::Takeoff => takeoff_hand_path(inner, &r.inputs, pa).points,
                        StripKind::Climb => climb_hand_path(inner, &r.inputs, pa)?.points,
                    };
                    let pts: String = points
                        .iter()
                        .map(|(x, y)| format!("{x:.1},{y:.1}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let color = overlay_trace_var(idx);
                    let dash = crate::theme::overlay_trace_dash(idx);
                    let pa_guide = match kind {
                        StripKind::Takeoff => {
                            let label = format_takeoff_strip_pa_trace_label(locale, pa);
                            strip_takeoff_pa_curve_guide_overlay(
                                inner,
                                pa,
                                r.inputs.oat_c,
                                label,
                                color,
                            )
                            .into_any()
                        }
                        StripKind::Climb => {
                            let label = format_takeoff_strip_pa_trace_label(locale, pa);
                            strip_climb_pa_curve_guide_overlay(
                                inner,
                                pa,
                                r.inputs.oat_c,
                                label,
                                color,
                            )
                            .into_any()
                        }
                    };
                    Some(view! {
                        <>
                            {pa_guide}
                            <polyline
                                points=pts
                                fill="none"
                                stroke=color
                                stroke-width="2.5"
                                opacity="0.88"
                                stroke-dasharray=dash
                            />
                        </>
                    })
                })
                .collect_view()
        }}
    }
    .into_any()
}

fn panel_backgrounds(locale: Locale) -> impl IntoView {
    let c = copy(locale);
    view! {
        <rect x=TAKEOFF_OAT.x y=STRIP_TOP width=TAKEOFF_OAT.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        <rect x=TAKEOFF_WEIGHT.x y=STRIP_TOP width=TAKEOFF_WEIGHT.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        <rect x=TAKEOFF_WIND.x y=STRIP_TOP width=TAKEOFF_WIND.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        <rect x=TAKEOFF_OBS.x y=STRIP_TOP width=TAKEOFF_OBS.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        {takeoff_strip_panel_titles(c)}
    }
}

/// Strip drag with tab prefs — same setters as the numeric fields (live `compute_all_isolated`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_strip_hit(
    kind: StripKind,
    sx: f64,
    sy: f64,
    data: &PerformanceData,
    mode: AirfieldInputsMode,
    chart_phase: Option<ChartPhase>,
    inputs: &mut FlightInputs,
    prefs: &mut FlightPrefs,
) {
    let inner = data.inner();
    match kind {
        StripKind::Takeoff => {
            if (TAKEOFF_OAT.x..TAKEOFF_OAT.x + TAKEOFF_OAT.w).contains(&sx) {
                let oat = ((sx - TAKEOFF_OAT.x) / TAKEOFF_OAT.w * 75.0 - 20.0)
                    .clamp(limits::FIG54_OAT_C.0, limits::FIG54_OAT_C.1);
                let ny = 1.0 - (sy - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP);
                let pa = lerp_axis(&inner.to_pa.value, ny)
                    .clamp(limits::FIG54_PA_FT.0, limits::FIG54_PA_FT.1);
                shared_airfield_set_pressure_altitude(mode, chart_phase, prefs, inputs, pa);
                shared_airfield_set_oat_c(mode, prefs, inputs, oat);
            } else if (TAKEOFF_WEIGHT.x..TAKEOFF_WEIGHT.x + TAKEOFF_WEIGHT.w).contains(&sx) {
                let kg = defaults::TAKEOFF_CHART_WEIGHT_KG
                    - (sx - TAKEOFF_WEIGHT.x) / TAKEOFF_WEIGHT.w
                        * (defaults::TAKEOFF_CHART_WEIGHT_KG - 600.0);
                takeoff_set_weight_lb(prefs, inputs, kg / 0.45359237);
            } else if (TAKEOFF_WIND.x..=TAKEOFF_WIND.x + TAKEOFF_WIND.w).contains(&sx) {
                let Some(rwy) = da20_core::wind::runway_heading_deg(&inputs.runway) else {
                    return;
                };
                let Some((dir, speed, clamp)) =
                    back_solve_takeoff_wind_from_strip_x(sx, inputs.wind_dir_deg, rwy)
                else {
                    return;
                };
                takeoff_apply_wind_from_strip(prefs, inputs, dir, speed, clamp);
            } else if (TAKEOFF_OBS.x..TAKEOFF_OBS.x + TAKEOFF_OBS.w).contains(&sx) {
                let obs = (sx - TAKEOFF_OBS.x) / TAKEOFF_OBS.w * defaults::OBSTACLE_HEIGHT_MAX_M;
                takeoff_set_obstacle_height_m(prefs, inputs, obs);
            }
        }
        StripKind::Climb => {
            if (CLIMB_EQ.x..CLIMB_EQ.x + CLIMB_EQ.w).contains(&sx) {
                let oat = ((sx - CLIMB_EQ.x) / CLIMB_EQ.w * 75.0 - 20.0)
                    .clamp(limits::FIG52_OAT_C.0, limits::FIG52_OAT_C.1);
                let ny = 1.0 - (sy - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP);
                let pa = lerp_axis(&inner.fan_pa.value, ny)
                    .clamp(limits::FIG52_PA_FT.0, limits::FIG52_PA_FT.1);
                shared_airfield_set_pressure_altitude(mode, chart_phase, prefs, inputs, pa);
                shared_airfield_set_oat_c(mode, prefs, inputs, oat);
            }
        }
    }
}

/// Tests and legacy paths without tab prefs (inputs signal only).
fn apply_strip_hit_nprefs(
    kind: StripKind,
    sx: f64,
    sy: f64,
    data: &PerformanceData,
    inputs: &mut FlightInputs,
) {
    let mut prefs = FlightPrefs::default();
    let (mode, chart_phase) = match kind {
        StripKind::Takeoff => (AirfieldInputsMode::Takeoff, Some(ChartPhase::Takeoff)),
        StripKind::Climb => (AirfieldInputsMode::Climb, Some(ChartPhase::Climb)),
    };
    apply_strip_hit(kind, sx, sy, data, mode, chart_phase, inputs, &mut prefs);
}

fn lerp_axis(axis: &[f64], n: f64) -> f64 {
    if axis.is_empty() {
        return 0.0;
    }
    if axis.len() == 1 {
        return axis[0];
    }
    let f = n.clamp(0.0, 1.0) * (axis.len() - 1) as f64;
    let i = f.floor() as usize;
    let j = (i + 1).min(axis.len() - 1);
    axis[i] + (f - i as f64) * (axis[j] - axis[i])
}

const STRIP_VIEWBOX_W: f64 = 1320.0;
const STRIP_VIEWBOX_H: f64 = 560.0;

/// Map client coordinates to strip viewBox when the SVG uses default `meet` (same geometry as CTM⁻¹).
pub(crate) fn client_to_strip_viewbox_meet(
    client_x: f64,
    client_y: f64,
    rect_left: f64,
    rect_top: f64,
    rect_width: f64,
    rect_height: f64,
) -> (f64, f64) {
    let scale = (rect_width / STRIP_VIEWBOX_W).min(rect_height / STRIP_VIEWBOX_H);
    let rendered_w = STRIP_VIEWBOX_W * scale;
    let rendered_h = STRIP_VIEWBOX_H * scale;
    let ox = rect_left + (rect_width - rendered_w) * 0.5;
    let oy = rect_top + (rect_height - rendered_h) * 0.5;
    let sx = (client_x - ox) / scale;
    let sy = (client_y - oy) / scale;
    (sx, sy)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn strip_viewbox_to_client_meet(
    sx: f64,
    sy: f64,
    rect_left: f64,
    rect_top: f64,
    rect_width: f64,
    rect_height: f64,
) -> (f64, f64) {
    let scale = (rect_width / STRIP_VIEWBOX_W).min(rect_height / STRIP_VIEWBOX_H);
    let rendered_w = STRIP_VIEWBOX_W * scale;
    let rendered_h = STRIP_VIEWBOX_H * scale;
    let ox = rect_left + (rect_width - rendered_w) * 0.5;
    let oy = rect_top + (rect_height - rendered_h) * 0.5;
    (ox + sx * scale, oy + sy * scale)
}

#[cfg(target_arch = "wasm32")]
fn client_to_svg_viewbox_via_ctm(
    svg: &web_sys::SvgElement,
    client_x: f64,
    client_y: f64,
) -> Option<(f64, f64)> {
    use wasm_bindgen::JsCast;
    let root = svg.dyn_ref::<web_sys::SvgsvgElement>()?;
    let graphics = root.dyn_ref::<web_sys::SvgGraphicsElement>()?;
    let ctm = graphics.get_screen_ctm()?;
    let inv = ctm.inverse().ok()?;
    let pt = root.create_svg_point();
    pt.set_x(client_x as f32);
    pt.set_y(client_y as f32);
    let out = pt.matrix_transform(&inv);
    Some((f64::from(out.x()), f64::from(out.y())))
}

pub(crate) fn svg_pointer_coords(ev: &web_sys::PointerEvent) -> Option<(f64, f64)> {
    svg_coords(ev)
}

fn svg_coords(ev: &web_sys::PointerEvent) -> Option<(f64, f64)> {
    let target = ev.current_target()?.dyn_into::<web_sys::Element>().ok()?;
    let client_x = ev.client_x() as f64;
    let client_y = ev.client_y() as f64;
    #[cfg(target_arch = "wasm32")]
    if let Ok(svg) = target.clone().dyn_into::<web_sys::SvgElement>() {
        if let Some(pt) = client_to_svg_viewbox_via_ctm(&svg, client_x, client_y) {
            return Some(pt);
        }
    }
    let rect = target.get_bounding_client_rect();
    Some(client_to_strip_viewbox_meet(
        client_x,
        client_y,
        rect.left(),
        rect.top(),
        rect.width(),
        rect.height(),
    ))
}

pub(crate) fn capture_pointer(ev: &web_sys::PointerEvent) {
    if let Some(el) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let _ = el.set_pointer_capture(ev.pointer_id());
    }
}

pub(crate) fn release_pointer(ev: &web_sys::PointerEvent) {
    if let Some(el) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let _ = el.release_pointer_capture(ev.pointer_id());
    }
}

/// ES/EN strip caption wind label + kt for Fig 5.4 header line.
pub fn takeoff_strip_wind_caption_parts(
    locale: Locale,
    hw: f64,
    tw: f64,
    rwy_valid: bool,
) -> (&'static str, f64) {
    let c = copy(locale);
    if !rwy_valid {
        return (c.strip_fig54_hw, 0.0);
    }
    if takeoff_wind_is_tailwind(hw, tw) {
        (c.strip_fig54_tw, tw)
    } else {
        (c.strip_fig54_hw, hw)
    }
}

#[cfg(test)]
mod strip_drag_tests {
    use super::*;
    use da20_core::rev29::hand_plot::{headwind_kt_from_strip_x, tailwind_kt_from_strip_x};
    use da20_core::types::defaults;

    #[test]
    fn pointer_meet_mapping_round_trips_known_viewbox_pixels() {
        let rect_left = 12.0;
        let rect_top = 24.0;
        let rect_w = 920.0;
        let rect_h = 360.0;
        for (sx, sy) in [
            (
                TAKEOFF_OAT.x + TAKEOFF_OAT.w * 0.25,
                (STRIP_TOP + STRIP_BOTTOM) * 0.5,
            ),
            (x_weight_kg(720.0), STRIP_TOP + 40.0),
            (x_headwind_kt(10.0), STRIP_BOTTOM - 30.0),
            (x_tailwind_kt(3.0), STRIP_BOTTOM - 30.0),
        ] {
            let (cx, cy) =
                strip_viewbox_to_client_meet(sx, sy, rect_left, rect_top, rect_w, rect_h);
            let (sx2, sy2) =
                client_to_strip_viewbox_meet(cx, cy, rect_left, rect_top, rect_w, rect_h);
            assert!(
                (sx2 - sx).abs() < 1e-6 && (sy2 - sy).abs() < 1e-6,
                "round-trip failed for ({sx},{sy})"
            );
        }
    }

    #[test]
    fn pointer_meet_mapping_oat_pa_from_oat_panel_pixel() {
        let rect = (0.0, 0.0, 900.0, 360.0);
        let sx = TAKEOFF_OAT.x + TAKEOFF_OAT.w * 0.5;
        let sy = 300.0;
        let (cx, cy) = strip_viewbox_to_client_meet(sx, sy, rect.0, rect.1, rect.2, rect.3);
        let (sx2, sy2) = client_to_strip_viewbox_meet(cx, cy, rect.0, rect.1, rect.2, rect.3);
        assert!((sx2 - sx).abs() < 1e-6 && (sy2 - sy).abs() < 1e-6);
        let oat = (sx2 - TAKEOFF_OAT.x) / TAKEOFF_OAT.w * 75.0 - 20.0;
        assert!((oat - 17.5).abs() < 0.01);
        let ny = 1.0 - (sy2 - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP);
        assert!((ny - (1.0 - (300.0 - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP))).abs() < 1e-6);
    }

    #[test]
    fn full_rect_stretch_mapping_differs_from_meet_when_letterboxed() {
        let rect_w = 1400.0;
        let rect_h = 700.0;
        let sx = 300.0;
        let sy = 80.0;
        let (cx, cy) = strip_viewbox_to_client_meet(sx, sy, 0.0, 0.0, rect_w, rect_h);
        let stretched_x = cx / rect_w * STRIP_VIEWBOX_W;
        let stretched_y = cy / rect_h * STRIP_VIEWBOX_H;
        assert!(
            (stretched_x - sx).abs() > 0.5 || (stretched_y - sy).abs() > 0.5,
            "stretch mapping should mis-read a point when the element aspect differs from the viewBox"
        );
        let (sx2, sy2) = client_to_strip_viewbox_meet(cx, cy, 0.0, 0.0, rect_w, rect_h);
        assert!((sx2 - sx).abs() < 1e-6 && (sy2 - sy).abs() < 1e-6);
    }

    #[test]
    fn pointer_to_viewbox_maps_element_center_when_aspect_matches() {
        let (sx, sy) = client_to_strip_viewbox_meet(200.0, 90.0, 100.0, 40.0, 200.0, 100.0);
        assert!((sx - STRIP_VIEWBOX_W / 2.0).abs() < 0.01);
        assert!((sy - STRIP_VIEWBOX_H / 2.0).abs() < 0.01);
    }

    #[test]
    fn takeoff_strip_hit_regions_map_oat_weight_wind() {
        let sx_oat = TAKEOFF_OAT.x + TAKEOFF_OAT.w * 0.5;
        let oat = (sx_oat - TAKEOFF_OAT.x) / TAKEOFF_OAT.w * 75.0 - 20.0;
        assert!((oat - 17.5).abs() < 0.01);

        let kg_at_left = defaults::TAKEOFF_CHART_WEIGHT_KG
            - (TAKEOFF_WEIGHT.x - TAKEOFF_WEIGHT.x) / TAKEOFF_WEIGHT.w
                * (defaults::TAKEOFF_CHART_WEIGHT_KG - 600.0);
        assert!((kg_at_left - 800.0).abs() < 0.01);

        let sx_hw = TAKEOFF_HW.x + TAKEOFF_HW.w * 0.5;
        let hw = headwind_kt_from_strip_x(sx_hw);
        assert!((hw - 10.0).abs() < 0.01);

        let sx_tw_ref = tailwind_reference_x();
        assert!((tailwind_kt_from_strip_x(sx_tw_ref) - 0.0).abs() < 0.01);
    }

    #[test]
    fn takeoff_wind_strip_drag_back_solves_from_tw_subpanel() {
        let rwy = 190.0;
        let dir = 10.0;
        let sx = TAKEOFF_TW.x + TAKEOFF_TW.w * 0.25;
        let (d, s, _) = back_solve_takeoff_wind_from_strip_x(sx, dir, rwy).expect("tw drag");
        assert_eq!(d, dir);
        assert!(s > 0.0);
        let tw = tailwind_kt_from_strip_x(sx);
        let comp = da20_core::wind::wind_components(d, s, rwy);
        let (_, tw_out) = da20_core::wind::headwind_tailwind_kt(&comp);
        assert!((tw_out - tw).abs() < 0.15, "tw_out={tw_out} target={tw}");
    }

    #[test]
    fn takeoff_strip_pa_drag_matches_typed_setter_and_compute() {
        use crate::inputs::{
            shared_airfield_set_oat_c, shared_airfield_set_pressure_altitude, AirfieldInputsMode,
        };
        use crate::phase_state::{assemble_takeoff_inputs, hydrate_tab_inputs};
        use crate::phases::Phase;
        use crate::prefs::FlightPrefs;
        use da20_core::takeoff::compute_takeoff;
        use da20_core::types::ChartPhase;

        let data = PerformanceData::production();
        let inner = data.inner();
        let sx = TAKEOFF_OAT.x + TAKEOFF_OAT.w * 0.4;
        let sy = STRIP_TOP + (STRIP_BOTTOM - STRIP_TOP) * 0.35;
        let oat = ((sx - TAKEOFF_OAT.x) / TAKEOFF_OAT.w * 75.0 - 20.0)
            .clamp(limits::FIG54_OAT_C.0, limits::FIG54_OAT_C.1);
        let ny = 1.0 - (sy - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP);
        let pa = super::lerp_axis(&inner.to_pa.value, ny)
            .clamp(limits::FIG54_PA_FT.0, limits::FIG54_PA_FT.1);

        let mut prefs_drag = FlightPrefs::default();
        let mut inputs_drag = FlightInputs::default();
        hydrate_tab_inputs(Phase::Takeoff, &prefs_drag, &data, &mut inputs_drag);
        apply_strip_hit(
            StripKind::Takeoff,
            sx,
            sy,
            &data,
            AirfieldInputsMode::Takeoff,
            Some(ChartPhase::Takeoff),
            &mut inputs_drag,
            &mut prefs_drag,
        );

        let mut prefs_type = FlightPrefs::default();
        let mut inputs_type = FlightInputs::default();
        hydrate_tab_inputs(Phase::Takeoff, &prefs_type, &data, &mut inputs_type);
        shared_airfield_set_pressure_altitude(
            AirfieldInputsMode::Takeoff,
            Some(ChartPhase::Takeoff),
            &mut prefs_type,
            &mut inputs_type,
            pa,
        );
        shared_airfield_set_oat_c(
            AirfieldInputsMode::Takeoff,
            &mut prefs_type,
            &mut inputs_type,
            oat,
        );

        assert_eq!(
            prefs_drag.takeoff.pressure_altitude_ft,
            prefs_type.takeoff.pressure_altitude_ft
        );
        assert!((prefs_drag.takeoff.oat_c - prefs_type.takeoff.oat_c).abs() < f64::EPSILON);

        let roll_drag = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs_drag))
            .unwrap()
            .ground_roll_ft;
        let roll_type = compute_takeoff(&data, &assemble_takeoff_inputs(&prefs_type))
            .unwrap()
            .ground_roll_ft;
        assert!((roll_drag - roll_type).abs() < f64::EPSILON);
    }

    #[test]
    fn climb_strip_pa_drag_leaves_takeoff_tab_unchanged() {
        use crate::phase_state::hydrate_tab_inputs;
        use crate::phases::Phase;
        use crate::prefs::FlightPrefs;

        let data = PerformanceData::production();
        let mut prefs = FlightPrefs::default();
        prefs.takeoff.pressure_altitude_ft = 2130.0;
        prefs.takeoff.oat_c = 20.0;
        let mut inputs = FlightInputs::default();
        hydrate_tab_inputs(Phase::Climb, &prefs, &data, &mut inputs);
        let sx = CLIMB_EQ.x + CLIMB_EQ.w * 0.5;
        let sy = STRIP_TOP + (STRIP_BOTTOM - STRIP_TOP) * 0.2;
        apply_strip_hit(
            StripKind::Climb,
            sx,
            sy,
            &data,
            AirfieldInputsMode::Climb,
            Some(ChartPhase::Climb),
            &mut inputs,
            &mut prefs,
        );
        assert_eq!(prefs.takeoff.pressure_altitude_ft, 2130.0);
        assert_eq!(prefs.takeoff.oat_c, 20.0);
        assert!(prefs.climb.pa_override.is_some());
        assert!((prefs.climb.pa_override.as_ref().unwrap().pa_ft - 2130.0).abs() > 100.0);
    }

    #[test]
    fn takeoff_input_cursors_track_hw_tw_obstacle() {
        use da20_core::types::FlightInputs;
        let inputs = FlightInputs {
            wind_speed_kt: 8.0,
            wind_dir_deg: 0.0,
            runway: "360".into(),
            obstacle_height_m: 7.5,
            ..Default::default()
        };
        let (hw, tw) = headwind_and_tailwind(&inputs);
        assert!((x_headwind_kt(hw) - TAKEOFF_HW.x - hw / 20.0 * TAKEOFF_HW.w).abs() < 0.01);
        assert!(
            (x_tailwind_kt(tw) - (TAKEOFF_TW.x + TAKEOFF_TW.w - tw / 5.0 * TAKEOFF_TW.w)).abs()
                < 0.01
        );
        assert!(
            (x_obstacle_m(inputs.obstacle_height_m)
                - TAKEOFF_OBS.x
                - inputs.obstacle_height_m / defaults::OBSTACLE_HEIGHT_MAX_M * TAKEOFF_OBS.w)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn takeoff_strip_caption_shows_tailwind_for_runway_190_wind_010() {
        use da20_core::types::FlightInputs;
        let inputs = FlightInputs {
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 5.0,
            ..Default::default()
        };
        let (hw, tw) = headwind_and_tailwind(&inputs);
        let (lbl, kt) = takeoff_strip_wind_caption_parts(Locale::Es, hw, tw, true);
        assert_eq!(lbl, copy(Locale::Es).strip_fig54_tw);
        assert!((kt - tw).abs() < 0.01);
        assert!((kt - 5.0).abs() < 0.01);
        assert_eq!(lbl, "Viento de cola");
    }

    #[test]
    fn takeoff_pa_curve_strip_spans_full_oat_panel_width() {
        use da20_core::rev29::hand_plot::takeoff_pa_curve_strip_points;

        let data = PerformanceData::production();
        let inner = data.inner();
        let pts = takeoff_pa_curve_strip_points(inner, 2500.0);
        assert!(pts.len() >= 2);
        let left = pts.first().unwrap().0;
        let right = pts.last().unwrap().0;
        assert!((left - TAKEOFF_OAT.x).abs() < 1.0);
        assert!((right - (TAKEOFF_OAT.x + TAKEOFF_OAT.w)).abs() < 1.0);
    }

    #[test]
    fn takeoff_oat_pa_trace_y_matches_effective_pa_engine_read() {
        use da20_core::rev29::hand_plot::takeoff_oat_panel_pa_trace;
        use da20_core::types::{ChartPhase, PaOverride};
        use da20_core::PerformanceData;

        let data = PerformanceData::production();
        let inputs = FlightInputs {
            pa_override: Some(PaOverride {
                pa_ft: 2500.0,
                phase: ChartPhase::Takeoff,
            }),
            oat_c: 20.0,
            ..Default::default()
        };
        let pa = effective_pressure_altitude_ft(&inputs, Some(ChartPhase::Takeoff));
        let trace = takeoff_oat_panel_pa_trace(data.inner(), pa, inputs.oat_c);
        assert!((pa - 2500.0).abs() < 0.01);
        assert!(
            (trace.y - y_dist(chart_read_takeoff_base_m(data.inner(), pa, inputs.oat_c))).abs()
                < 0.01
        );
    }

    fn assert_rendered_path_only_oat_riser_vertical(points: &[(f64, f64)], case: &str) {
        let polyline = format_svg_polyline(points);
        assert!(polyline.contains(','), "{case}: empty polyline");
        for (i, w) in points.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let vertical = (b.0 - a.0).abs() < 0.01 && (b.1 - a.1).abs() > 0.5;
            if vertical && i != 0 {
                panic!("{case}: rendered vertical segment {i}: {a:?} -> {b:?}");
            }
        }
    }

    #[test]
    fn takeoff_rendered_polyline_has_no_vertical_jumps() {
        use da20_core::rev29::hand_plot::tailwind_reference_x;
        use da20_core::types::{ChartPhase, FlightInputs, PaOverride};

        let data = PerformanceData::production();
        let pa = PaOverride {
            pa_ft: 2500.0,
            phase: ChartPhase::Takeoff,
        };
        let cases = [
            (
                "tw-800",
                FlightInputs {
                    oat_c: 20.0,
                    weight_lb: 800.0 / 0.45359237,
                    runway: "190".into(),
                    wind_dir_deg: 10.0,
                    wind_speed_kt: 4.0,
                    pa_override: Some(pa),
                    ..Default::default()
                },
            ),
            (
                "tw-650",
                FlightInputs {
                    oat_c: 20.0,
                    weight_lb: 650.0 / 0.45359237,
                    runway: "190".into(),
                    wind_dir_deg: 10.0,
                    wind_speed_kt: 4.0,
                    pa_override: Some(pa),
                    ..Default::default()
                },
            ),
            (
                "calm-800",
                FlightInputs {
                    oat_c: 20.0,
                    weight_lb: 800.0 / 0.45359237,
                    runway: "360".into(),
                    wind_dir_deg: 360.0,
                    wind_speed_kt: 0.0,
                    pa_override: Some(pa),
                    ..Default::default()
                },
            ),
            (
                "calm-650",
                FlightInputs {
                    oat_c: 20.0,
                    weight_lb: 650.0 / 0.45359237,
                    runway: "360".into(),
                    wind_dir_deg: 360.0,
                    wind_speed_kt: 0.0,
                    pa_override: Some(pa),
                    ..Default::default()
                },
            ),
            (
                "hw20-750",
                FlightInputs {
                    oat_c: 20.0,
                    weight_lb: 750.0 / 0.45359237,
                    runway: "360".into(),
                    wind_dir_deg: 360.0,
                    wind_speed_kt: 20.0,
                    pa_override: Some(pa),
                    ..Default::default()
                },
            ),
        ];
        for (name, inputs) in cases {
            let pts = takeoff_rendered_path_points(data.inner(), &inputs, 2500.0);
            assert_rendered_path_only_oat_riser_vertical(&pts, name);
            let end = *pts.last().unwrap();
            let path = takeoff_hand_path(data.inner(), &inputs, 2500.0);
            assert!(
                (end.1 - y_dist(path.chart_over_obstacle_m)).abs() < 0.75,
                "{name}: endpoint y {} vs obstacle {}",
                end.1,
                y_dist(path.chart_over_obstacle_m)
            );
        }

        let tw = FlightInputs {
            oat_c: 20.0,
            weight_lb: 800.0 / 0.45359237,
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 4.0,
            pa_override: Some(pa),
            ..Default::default()
        };
        let pts = takeoff_rendered_path_points(data.inner(), &tw, 2500.0);
        let core = takeoff_hand_path(data.inner(), &tw, 2500.0);
        let mut sloped = false;
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let in_tw = a.0 >= TAKEOFF_TW.x - 0.1
                && b.0 >= TAKEOFF_TW.x - 0.1
                && a.0 <= tailwind_reference_x() + 0.1
                && b.0 <= tailwind_reference_x() + 0.1;
            if in_tw && b.0 < a.0 - 0.5 && (b.1 - a.1).abs() > 0.2 {
                sloped = true;
            }
        }
        assert!(
            sloped,
            "tailwind leg must slope from 0 kt toward entered TW"
        );
        let tw_x = x_tailwind_kt(4.0);
        let at_tw = pts
            .iter()
            .rfind(|(x, _)| *x >= TAKEOFF_TW.x - 0.5 && *x <= tw_x + 0.5)
            .copied()
            .expect("tw panel end");
        let ground_y = y_dist(core.chart_ground_m);
        assert!(
            (at_tw.1 - ground_y).abs() < 1.5,
            "rendered TW end y {} vs ground-roll y {ground_y}",
            at_tw.1
        );
    }

    #[test]
    fn takeoff_wind_subpanels_render_at_five_to_twenty() {
        let ratio = TAKEOFF_TW.w / TAKEOFF_HW.w;
        assert!((ratio - 0.25).abs() < 1e-9, "TW:HW {ratio}");
        assert!((TAKEOFF_TW.w + TAKEOFF_HW.w - TAKEOFF_WIND.w).abs() < 1e-9);
        assert!(
            (TAKEOFF_WIND.x + TAKEOFF_WIND.w - TAKEOFF_OBS.x).abs() < 1e-6,
            "wind panel must meet the obstacle panel"
        );
        assert!((x_tailwind_kt(5.0) - TAKEOFF_TW.x).abs() < 1e-6);
        assert!((x_tailwind_kt(0.0) - (TAKEOFF_TW.x + TAKEOFF_TW.w)).abs() < 1e-6);
        assert!((x_headwind_kt(20.0) - (TAKEOFF_HW.x + TAKEOFF_HW.w)).abs() < 1e-6);
    }

    #[test]
    fn takeoff_strip_readout_ooc_never_shows_nan() {
        use da20_core::types::FlightInputs;

        let data = PerformanceData::production();
        let inputs = FlightInputs {
            weight_lb: 2500.0,
            ..Default::default()
        };
        let line = readout_line(Locale::Es, StripKind::Takeoff, data, inputs);
        assert!(
            !line.contains("NaN"),
            "readout must not format NaN distances: {line}"
        );
        assert!(
            line.contains("Fuera") || line.contains("carta") || line.contains('—'),
            "expected OOC or placeholder: {line}"
        );
    }

    #[test]
    fn takeoff_strip_readout_matches_solver_and_weight_direction() {
        use da20_core::takeoff::compute_takeoff;
        use da20_core::types::{ChartPhase, FlightInputs, PaOverride};

        let data = PerformanceData::production();
        let pa = Some(PaOverride {
            pa_ft: 2500.0,
            phase: ChartPhase::Takeoff,
        });
        let light = FlightInputs {
            oat_c: 20.0,
            weight_lb: 650.0 / 0.45359237,
            runway: "360".into(),
            wind_dir_deg: 360.0,
            wind_speed_kt: 0.0,
            pa_override: pa,
            ..Default::default()
        };
        let heavy_tw = FlightInputs {
            oat_c: 20.0,
            weight_lb: 800.0 / 0.45359237,
            runway: "190".into(),
            wind_dir_deg: 10.0,
            wind_speed_kt: 4.0,
            pa_override: pa,
            ..Default::default()
        };
        let light_line = readout_line(Locale::En, StripKind::Takeoff, data.clone(), light.clone());
        let heavy_line = readout_line(
            Locale::En,
            StripKind::Takeoff,
            data.clone(),
            heavy_tw.clone(),
        );
        let light_takeoff = compute_takeoff(&data, &light).unwrap();
        let heavy_takeoff = compute_takeoff(&data, &heavy_tw).unwrap();
        let light_path = takeoff_hand_path(data.inner(), &light, 2500.0);
        let heavy_path = takeoff_hand_path(data.inner(), &heavy_tw, 2500.0);
        assert!(
            light_takeoff.distance_over_obstacle_raw_m
                <= heavy_takeoff.distance_over_obstacle_raw_m
        );
        assert!(light_path.chart_ground_m <= heavy_path.chart_ground_m);
        assert!(light_line.contains(&format!("{:.0}", light_path.chart_ground_m.round())));
        assert!(light_line.contains(&format!(
            "{:.0}",
            light_takeoff.distance_over_obstacle_raw_ft.round()
        )));
        assert!(heavy_line.contains(&format!("{:.0}", heavy_path.chart_over_obstacle_m.round())));
        assert_ne!(light_line, heavy_line);
    }

    #[test]
    fn pa_label_y_is_isoline_end_not_cursor() {
        use da20_core::rev29::hand_plot::{
            climb_fan_pa_curve_strip_points, climb_pa_isoline_y_at_panel_right,
            takeoff_pa_curve_point_at_oat, takeoff_pa_curve_strip_points,
            takeoff_pa_isoline_y_at_panel_right,
        };
        let data = PerformanceData::production();
        let pts = takeoff_pa_curve_strip_points(data.inner(), 2500.0);
        let y_end = takeoff_pa_isoline_y_at_panel_right(data.inner(), 2500.0);
        let right = pts.last().copied().unwrap();
        assert!((right.0 - (TAKEOFF_OAT.x + TAKEOFF_OAT.w)).abs() < 1.0);
        assert!((y_end - right.1).abs() < 0.05);
        let cursor = takeoff_pa_curve_point_at_oat(data.inner(), 2500.0, 20.0);
        assert!(
            (cursor.0 - right.0).abs() > 20.0,
            "cursor must not be the panel-edge anchor"
        );
        let climb_pts = climb_fan_pa_curve_strip_points(data.inner(), 2500.0);
        let climb_y = climb_pa_isoline_y_at_panel_right(data.inner(), 2500.0);
        assert!((climb_y - climb_pts.last().unwrap().1).abs() < 0.05);
    }
}
