//! Multi-panel hand-plot strips (Fig 5.4 takeoff, Fig 5.2 → ROC climb).

use std::collections::HashSet;

use da20_core::altitude::{effective_pressure_altitude_ft, set_pa_override};
use da20_core::climb::compute_climb;
use da20_core::rev29::charts::{climb_roc_vs_ea, equivalent_altitude_curves, takeoff_base_curves};
use da20_core::rev29::engine::lb_to_kg;
use da20_core::rev29::hand_plot::{
    chart_read_takeoff_base_m, climb_hand_path, headwind_and_tailwind, takeoff_hand_path,
    takeoff_panel_guides, x_ea_ft, x_oat, x_weight_kg, y_dist, y_ea_ft, y_roc_fpm, CLIMB_EQ,
    CLIMB_ROC, DIST_MAX_M, DIST_MIN_M, STRIP_BOTTOM, STRIP_TOP, TAKEOFF_OAT, TAKEOFF_OBS,
    TAKEOFF_WEIGHT, TAKEOFF_WIND,
};
use da20_core::takeoff::compute_takeoff;
use da20_core::types::{defaults, ChartPhase, FlightInputs};
use da20_core::PerformanceData;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::chart_guides::{guide_line_label, LabelAnchor};
use crate::chart_inputs::round_after_chart_drag;
use crate::i18n::{
    copy, format_strip_chart_roll_obs, format_takeoff_conservative_estimate,
    format_takeoff_over_obstacle_result, Locale,
};
use crate::phase_readouts::OVERLAY_STROKE_COLORS;
use crate::storage::ReadingsStore;

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
    #[prop(optional)] store: Option<RwSignal<ReadingsStore>>,
    #[prop(optional)] phase_key: Option<String>,
    #[prop(optional)] overlay_ids: Option<RwSignal<HashSet<String>>>,
) -> impl IntoView {
    let dragging = RwSignal::new(false);

    let on_pointer_down = move |ev: web_sys::PointerEvent| {
        dragging.set(true);
        if let Some((sx, sy)) = svg_coords(&ev) {
            apply_hit(kind, sx, sy, data.get(), inputs);
        }
        capture_pointer(&ev);
    };
    let on_pointer_up = move |ev: web_sys::PointerEvent| {
        if dragging.get() {
            inputs.update(round_after_chart_drag);
        }
        dragging.set(false);
        release_pointer(&ev);
    };
    let on_pointer_move = move |ev: web_sys::PointerEvent| {
        if !dragging.get() {
            return;
        }
        if let Some((sx, sy)) = svg_coords(&ev) {
            apply_hit(kind, sx, sy, data.get(), inputs);
        }
    };

    view! {
        <div class="strip-wrap">
            <p class="hint">{move || copy(locale.get()).plot_hint}</p>
            <svg
                class="plot-svg strip-svg"
                viewBox="0 0 1110 560"
                on:pointerdown=on_pointer_down
                on:pointerup=on_pointer_up
                on:pointerleave=move |_| {
                    if dragging.get() {
                        inputs.update(round_after_chart_drag);
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
            <p class="cursor-readout mono">
                {move || readout_line(locale.get(), kind, data.get(), inputs.get())}
            </p>
        </div>
    }
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
            let path = takeoff_hand_path(inner, &inputs, pa);
            let core = compute_takeoff(&data, &inputs)
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
            format!("{}: {chart} | {core}", c.plot_readout_chart,)
        }
        StripKind::Climb => {
            let path = climb_hand_path(inner, &inputs, pa);
            let core = compute_climb(&data, &inputs, inputs.cruise_pressure_alt_ft)
                .map(|r| {
                    format!(
                        "{:.0} fpm @ EA {:.0} ft",
                        r.rate_of_climb_fpm, r.equivalent_altitude_ft
                    )
                })
                .unwrap_or_else(|_| "—".into());
            match path {
                Some(p) => {
                    let chart = format!("{:.0} fpm @ EA {:.0} ft", p.chart_roc_fpm, p.chart_ea_ft);
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
    let (hw, _tw) = headwind_and_tailwind(&inputs);
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

    let hw_line_m = inner.hw_lines.value.clone();
    let weight_guides =
        guides
            .weight_polylines
            .iter()
            .zip(hw_line_m.iter())
            .map(|(poly, &line_m)| {
                let pts: String = poly
                    .iter()
                    .map(|(x, y)| format!("{x:.1},{y:.1}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let (lx, ly) = poly
                    .last()
                    .copied()
                    .unwrap_or((TAKEOFF_WEIGHT.x, STRIP_TOP));
                view! {
                    <>
                        <polyline points=pts class="chart-stroke-guide" opacity="0.7"/>
                        {guide_line_label(
                            lx + 3.0,
                            ly,
                            format!("{line_m:.0} m"),
                            LabelAnchor::Start,
                        )}
                    </>
                }
            });

    let headwind_guides =
        guides
            .headwind_polylines
            .iter()
            .zip(hw_line_m.iter())
            .map(|(poly, &line_m)| {
                let pts: String = poly
                    .iter()
                    .map(|(x, y)| format!("{x:.1},{y:.1}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let (lx, ly) = poly.last().copied().unwrap_or((TAKEOFF_WIND.x, STRIP_TOP));
                view! {
                    <>
                        <polyline points=pts class="chart-stroke-guide" opacity="0.7"/>
                        {guide_line_label(
                            lx + 3.0,
                            ly,
                            format!("{line_m:.0} m"),
                            LabelAnchor::Start,
                        )}
                    </>
                }
            });

    let tw_line_m = inner.tw_lines.value.clone();
    let tailwind_guides =
        guides
            .tailwind_polylines
            .iter()
            .zip(tw_line_m.iter())
            .map(|(poly, &line_m)| {
                let pts: String = poly
                    .iter()
                    .map(|(x, y)| format!("{x:.1},{y:.1}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let (lx, ly) = poly.first().copied().unwrap_or((TAKEOFF_WIND.x, STRIP_TOP));
                view! {
                    <>
                        <polyline points=pts class="chart-stroke-guide" opacity="0.7"
                            stroke-dasharray="6 4"/>
                        {guide_line_label(
                            lx - 3.0,
                            ly,
                            format!("{line_m:.0} m"),
                            LabelAnchor::End,
                        )}
                    </>
                }
            });

    let obs_line_m = inner.obs_lines.value.clone();
    let obs_guides = guides
        .obstacle_ys
        .iter()
        .zip(obs_line_m.iter())
        .map(|((y0, y15), &g_m)| {
            view! {
                <>
                    <line
                        x1=TAKEOFF_OBS.x y1=*y0
                        x2={TAKEOFF_OBS.x + TAKEOFF_OBS.w} y2=*y15
                        class="chart-stroke-guide" opacity="0.7"
                    />
                    {guide_line_label(
                        TAKEOFF_OBS.x + TAKEOFF_OBS.w + 3.0,
                        *y15,
                        format!("{:.0} m", defaults::OBSTACLE_HEIGHT_MAX_M),
                        LabelAnchor::Start,
                    )}
                    {guide_line_label(
                        TAKEOFF_OBS.x - 3.0,
                        *y0,
                        format!("{g_m:.0} m"),
                        LabelAnchor::End,
                    )}
                </>
            }
        });

    let path_pts: String = path
        .points
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");

    view! {
        {panel_backgrounds(locale)}
        {grid_lines()}
        {curve_paths.collect_view()}
        {weight_guides.collect_view()}
        {headwind_guides.collect_view()}
        {tailwind_guides.collect_view()}
        {obs_guides.collect_view()}
        <line x1=oat_x y1=STRIP_TOP x2=oat_x y2=STRIP_BOTTOM class="chart-stroke-guide-active" stroke-dasharray="5 4"/>
        <line x1=x_weight_kg(kg) y1=STRIP_TOP x2=x_weight_kg(kg) y2=STRIP_BOTTOM class="chart-stroke-guide-active" stroke-dasharray="4 3"/>
        <polyline points=path_pts class="chart-stroke-user-path"/>
        {saved_path_overlays(StripKind::Takeoff, data.clone(), store, phase_key, overlay_ids)}
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
            TAKEOFF_WIND.x + TAKEOFF_WIND.w + 3.0,
            STRIP_TOP + 14.0,
            format!("{:.0} kt", inner.hw_kt.value.last().copied().unwrap_or(20.0)),
            LabelAnchor::Start,
        )}
        <text x="70" y="28" fill="var(--chart-label)" font-size="11">
            {c.strip_fig54_base}" "{format!("{base:.0}")}" m"
        </text>
        <text x="520" y="28" fill="var(--chart-label)" font-size="11">
            {c.strip_fig54_wt}" "{format!("{kg:.0}")}" kg · "{c.strip_fig54_hw}" "{format!("{hw:.1}")}" kt"
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
        .map(|(ea, roc)| format!("{},{}", x_ea_ft(*ea), y_roc_fpm(*roc)))
        .collect::<Vec<_>>()
        .join(" ");

    let path_pts = path.as_ref().map(|p| {
        p.points
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ")
    });

    let oat_x = x_oat(inputs.oat_c);

    let roc_label = match inputs.climb_figure {
        da20_core::types::ClimbFigure::Fig55CruiseFlaps => c.strip_fig55_roc,
        da20_core::types::ClimbFigure::Fig56TakeoffFlaps => c.strip_fig56_roc,
        da20_core::types::ClimbFigure::Fig58BalkedLanding => c.strip_fig58_roc,
    };

    view! {
        <rect x=CLIMB_EQ.x y=STRIP_TOP width=CLIMB_EQ.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        <rect x=CLIMB_ROC.x y=STRIP_TOP width=CLIMB_ROC.w height=STRIP_BOTTOM-STRIP_TOP class="chart-panel"/>
        <text x={CLIMB_EQ.x + 8.0} y="36" fill="var(--chart-label)" font-size="11">{c.strip_fig52_ea}</text>
        <text x={CLIMB_ROC.x + 8.0} y="36" fill="var(--chart-label)" font-size="11">{roc_label}</text>
        {eq_paths.collect_view()}
        <polyline points=roc_pts class="chart-stroke-line"/>
        {guide_line_label(
            x_ea_ft(0.0),
            y_roc_fpm(roc_pl.points.first().map(|(_, r)| *r).unwrap_or(0.0)),
            "0 ft".to_string(),
            LabelAnchor::End,
        )}
        {guide_line_label(
            x_ea_ft(14_000.0),
            y_roc_fpm(roc_pl.points.last().map(|(_, r)| *r).unwrap_or(0.0)),
            "14000 ft".to_string(),
            LabelAnchor::Start,
        )}
        <line x1=oat_x y1=STRIP_TOP x2=oat_x y2=STRIP_BOTTOM class="chart-stroke-guide-active" stroke-dasharray="5 4"/>
        {path_pts.map(|pts| view! {
            <>
                {saved_path_overlays(StripKind::Climb, data.clone(), store, phase_key.clone(), overlay_ids)}
                <polyline points=pts class="chart-stroke-user-path"/>
            </>
        })}
        {path.map(|p| p.markers.iter().map(|(x,y)| view! {
            <circle cx=*x cy=*y r="6" class="chart-marker"/>
        }).collect_view())}
    }
}

fn saved_path_overlays(
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
                    let color = OVERLAY_STROKE_COLORS[idx % OVERLAY_STROKE_COLORS.len()];
                    Some(view! {
                        <polyline
                            points=pts
                            fill="none"
                            stroke=color
                            stroke-width="2.5"
                            opacity="0.88"
                            stroke-dasharray="7 4"
                        />
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
        <text x=TAKEOFF_OAT.x+TAKEOFF_OAT.w/2.0 y="538" fill="var(--chart-label)" font-size="12" text-anchor="middle">{c.strip_panel_oat_pa}</text>
        <text x=TAKEOFF_WEIGHT.x+TAKEOFF_WEIGHT.w/2.0 y="538" fill="var(--chart-label)" font-size="12" text-anchor="middle">{c.strip_panel_weight}</text>
        <text x=TAKEOFF_WIND.x+TAKEOFF_WIND.w/2.0 y="538" fill="var(--chart-label)" font-size="12" text-anchor="middle">{c.strip_panel_hw}</text>
        <text x=TAKEOFF_OBS.x+TAKEOFF_OBS.w/2.0 y="538" fill="var(--chart-label)" font-size="12" text-anchor="middle">{c.strip_panel_obstacle}</text>
    }
}

fn grid_lines() -> impl IntoView {
    view! {
        {(0..=8).map(|i| {
            let m = DIST_MIN_M + (DIST_MAX_M - DIST_MIN_M) * i as f64 / 8.0;
            let y = y_dist(m);
            view! {
                <>
                    <line x1=TAKEOFF_OAT.x y1=y x2=TAKEOFF_OBS.x+TAKEOFF_OBS.w y2=y class="chart-stroke-grid"/>
                    {guide_line_label(
                        TAKEOFF_OAT.x - 3.0,
                        y,
                        format!("{m:.0} m"),
                        LabelAnchor::End,
                    )}
                </>
            }
        }).collect_view()}
    }
}

fn apply_hit(
    kind: StripKind,
    sx: f64,
    sy: f64,
    data: PerformanceData,
    inputs: RwSignal<FlightInputs>,
) {
    let inner = data.inner();
    match kind {
        StripKind::Takeoff => {
            if (TAKEOFF_OAT.x..TAKEOFF_OAT.x + TAKEOFF_OAT.w).contains(&sx) {
                let oat = (sx - TAKEOFF_OAT.x) / TAKEOFF_OAT.w * 75.0 - 20.0;
                let ny = 1.0 - (sy - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP);
                let pa = lerp_axis(&inner.to_pa.value, ny);
                inputs.update(|i| {
                    i.oat_c = oat;
                    set_pa_override(i, pa, ChartPhase::Takeoff);
                });
            } else if (TAKEOFF_WEIGHT.x..TAKEOFF_WEIGHT.x + TAKEOFF_WEIGHT.w).contains(&sx) {
                let kg = defaults::TAKEOFF_CHART_WEIGHT_KG
                    - (sx - TAKEOFF_WEIGHT.x) / TAKEOFF_WEIGHT.w
                        * (defaults::TAKEOFF_CHART_WEIGHT_KG - 600.0);
                inputs.update(|i| i.weight_lb = kg / 0.45359237);
            } else if (TAKEOFF_WIND.x..TAKEOFF_WIND.x + TAKEOFF_WIND.w).contains(&sx) {
                let hw = (sx - TAKEOFF_WIND.x) / TAKEOFF_WIND.w * 20.0;
                inputs.update(|i| {
                    let Some(rwy) = da20_core::wind::runway_heading_deg(&i.runway) else {
                        return;
                    };
                    let (dir, speed, clamp) =
                        da20_core::wind::back_solve_wind_for_headwind(i.wind_dir_deg, rwy, hw);
                    i.wind_dir_deg = dir;
                    i.wind_speed_kt = speed;
                    i.wind_strip_crosswind_clamped = clamp;
                });
            } else if sx >= TAKEOFF_OBS.x {
                let obs = (sx - TAKEOFF_OBS.x) / TAKEOFF_OBS.w * 15.0;
                inputs.update(|i| i.obstacle_height_m = obs.clamp(0.0, 15.0));
            }
        }
        StripKind::Climb => {
            if (CLIMB_EQ.x..CLIMB_EQ.x + CLIMB_EQ.w).contains(&sx) {
                let oat = (sx - CLIMB_EQ.x) / CLIMB_EQ.w * 75.0 - 20.0;
                let ny = 1.0 - (sy - STRIP_TOP) / (STRIP_BOTTOM - STRIP_TOP);
                let pa = lerp_axis(&inner.fan_pa.value, ny);
                inputs.update(|i| {
                    i.oat_c = oat;
                    set_pa_override(i, pa, ChartPhase::Climb);
                });
            }
        }
    }
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

const STRIP_VIEWBOX_W: f64 = 1110.0;
const STRIP_VIEWBOX_H: f64 = 560.0;

/// Map pointer client coordinates within an element bounding box to strip SVG viewBox space.
fn pointer_to_strip_viewbox(
    client_x: f64,
    client_y: f64,
    rect_left: f64,
    rect_top: f64,
    rect_width: f64,
    rect_height: f64,
) -> (f64, f64) {
    let nx = (client_x - rect_left) / rect_width;
    let ny = (client_y - rect_top) / rect_height;
    (nx * STRIP_VIEWBOX_W, ny * STRIP_VIEWBOX_H)
}

fn svg_coords(ev: &web_sys::PointerEvent) -> Option<(f64, f64)> {
    let target = ev.current_target()?.dyn_into::<web_sys::Element>().ok()?;
    let rect = target.get_bounding_client_rect();
    Some(pointer_to_strip_viewbox(
        ev.client_x() as f64,
        ev.client_y() as f64,
        rect.left(),
        rect.top(),
        rect.width(),
        rect.height(),
    ))
}

fn capture_pointer(ev: &web_sys::PointerEvent) {
    if let Some(el) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let _ = el.set_pointer_capture(ev.pointer_id());
    }
}

fn release_pointer(ev: &web_sys::PointerEvent) {
    if let Some(el) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let _ = el.release_pointer_capture(ev.pointer_id());
    }
}

#[cfg(test)]
mod strip_drag_tests {
    use super::*;
    use da20_core::types::defaults;

    #[test]
    fn pointer_to_viewbox_maps_element_center() {
        let (sx, sy) = pointer_to_strip_viewbox(200.0, 90.0, 100.0, 40.0, 200.0, 100.0);
        assert!((sx - 555.0).abs() < 0.01);
        assert!((sy - 280.0).abs() < 0.01);
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

        let sx_wind_mid = TAKEOFF_WIND.x + TAKEOFF_WIND.w * 0.5;
        let hw = (sx_wind_mid - TAKEOFF_WIND.x) / TAKEOFF_WIND.w * 20.0;
        assert!((hw - 10.0).abs() < 0.01);
    }
}
