//! Climb segment calculator (time, still-air distance, optional fuel estimate).

use da20_core::chart_bounds::limits;
use da20_core::climb_calculator::{
    climb_chart_max_weight_lb, compute_climb_calculator, ClimbCalculatorParams,
    ClimbCalculatorResult, CLIMB_CALC_DEFAULT_STEP_FT,
};
use da20_core::climb_sweep::{ClimbSweepStopReason, CLIMB_SWEEP_DEFAULT_TO_PA_FT};
use da20_core::types::FlightInputs;
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::altitude_sweep_panel::{
    format_altitude_sweep_err, format_min_sec, lapse_c_per_ft, round_lapse_display, LapseDraft,
};
use crate::i18n::{
    copy, format_climb_figure_placard, format_climb_fuel_omit_reason, format_climb_vy_held, Locale,
};
use crate::inputs::NumField;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimbCalculatorDraft {
    pub from_pa_ft: f64,
    pub to_pa_ft: f64,
    pub oat_at_from_c: f64,
}

impl Default for ClimbCalculatorDraft {
    fn default() -> Self {
        Self {
            from_pa_ft: 0.0,
            to_pa_ft: CLIMB_SWEEP_DEFAULT_TO_PA_FT,
            oat_at_from_c: 15.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CalcRunFingerprint {
    weight_lb: f64,
    climb_figure: da20_core::types::ClimbFigure,
    fairings: bool,
    from_pa_ft: f64,
    to_pa_ft: f64,
    oat_at_from_c: f64,
    lapse_per_1000_ft: f64,
}

fn current_fingerprint(
    inputs: &FlightInputs,
    draft: ClimbCalculatorDraft,
    lapse: LapseDraft,
) -> CalcRunFingerprint {
    CalcRunFingerprint {
        weight_lb: inputs.weight_lb,
        climb_figure: inputs.climb_figure,
        fairings: inputs.wheel_fairings_installed,
        from_pa_ft: draft.from_pa_ft,
        to_pa_ft: draft.to_pa_ft,
        oat_at_from_c: draft.oat_at_from_c,
        lapse_per_1000_ft: lapse.per_1000_ft,
    }
}

#[component]
pub fn ClimbCalculatorPanel(
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    let draft = RwSignal::new(ClimbCalculatorDraft::default());
    let lapse = RwSignal::new(LapseDraft::default());
    let from_dirty = RwSignal::new(false);
    let oat_dirty = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let result = RwSignal::new(Option::<ClimbCalculatorResult>::None);
    let run_fingerprint = RwSignal::new(Option::<CalcRunFingerprint>::None);
    let breakdown_open = RwSignal::new(false);

    let sync_from_tab = move || {
        let i = inputs.get_untracked();
        if !from_dirty.get_untracked() {
            draft.update(|d| d.from_pa_ft = i.pressure_altitude_input_ft);
        }
        if !oat_dirty.get_untracked() {
            draft.update(|d| d.oat_at_from_c = i.oat_c);
        }
    };
    sync_from_tab();

    Effect::new(move |_| {
        let i = inputs.get();
        draft.update(|d| {
            if !from_dirty.get_untracked() {
                d.from_pa_ft = i.pressure_altitude_input_ft;
            }
            if !oat_dirty.get_untracked() {
                d.oat_at_from_c = i.oat_c;
            }
        });
    });

    let stale = Memo::new(move |_| {
        let Some(fp) = run_fingerprint.get() else {
            return false;
        };
        let cur = current_fingerprint(&inputs.get(), draft.get(), lapse.get());
        cur != fp
    });

    let run = move |_| {
        error.set(None);
        result.set(None);
        run_fingerprint.set(None);
        breakdown_open.set(false);
        let d = draft.get();
        let params = ClimbCalculatorParams {
            from_pa_ft: d.from_pa_ft,
            to_pa_ft: d.to_pa_ft,
            oat_at_from_c: d.oat_at_from_c,
            step_ft: CLIMB_CALC_DEFAULT_STEP_FT,
            lapse_rate_c_per_ft: lapse_c_per_ft(lapse.get()),
        };
        match compute_climb_calculator(&data.get(), &inputs.get(), &params) {
            Ok(res) => {
                run_fingerprint.set(Some(current_fingerprint(
                    &inputs.get_untracked(),
                    d,
                    lapse.get_untracked(),
                )));
                result.set(Some(res));
            }
            Err(e) => {
                let msg = match e {
                    da20_core::climb_calculator::ClimbCalculatorError::InvalidRange => {
                        copy(locale.get()).climb_calc_range_err.to_string()
                    }
                    da20_core::climb_calculator::ClimbCalculatorError::InvalidLapse => {
                        copy(locale.get()).climb_calc_invalid_lapse.to_string()
                    }
                    da20_core::climb_calculator::ClimbCalculatorError::Sweep(se) => {
                        format_altitude_sweep_err(locale.get(), se)
                    }
                };
                error.set(Some(msg));
            }
        }
    };

    view! {
        <div class="card sweep-card">
            <h3>{move || copy(locale.get()).climb_calc_title.to_string()}</h3>
            <p class="lab results-muted">{move || copy(locale.get()).climb_calc_max_weight_note.to_string()}</p>
            <div class="sweep-range">
                <NumField label=move || copy(locale.get()).altitude_sweep_from_ft.to_string()
                    min=limits::FIG52_PA_FT.0
                    max=limits::FIG52_PA_FT.1
                    get=move || draft.get().from_pa_ft
                    set=move |v| {
                        from_dirty.set(true);
                        draft.update(|d| d.from_pa_ft = v);
                    }/>
                <NumField label=move || copy(locale.get()).altitude_sweep_to_ft.to_string()
                    min=limits::FIG52_PA_FT.0
                    max=limits::FIG52_PA_FT.1
                    get=move || draft.get().to_pa_ft
                    set=move |v| draft.update(|d| d.to_pa_ft = v)/>
                <NumField label=move || copy(locale.get()).climb_calc_oat_from.to_string()
                    get=move || draft.get().oat_at_from_c
                    set=move |v| {
                        oat_dirty.set(true);
                        draft.update(|d| d.oat_at_from_c = v);
                    }/>
            </div>
            <NumField label=move || copy(locale.get()).altitude_sweep_lapse_label.to_string()
                get=move || round_lapse_display(lapse.get().per_1000_ft)
                set=move |v| lapse.update(|d| d.per_1000_ft = round_lapse_display(v))/>
            <p class="lab results-muted">{move || copy(locale.get()).altitude_sweep_lapse_note.to_string()}</p>
            <button type="button" class="btn g" on:click=run>
                {move || copy(locale.get()).climb_calc_run.to_string()}
            </button>
            {move || error.get().map(|e| view! { <p class="lab field-error">{e}</p> })}
            <ClimbCalculatorResults locale=locale result=result stale=stale breakdown_open=breakdown_open/>
        </div>
    }
}

#[component]
fn ClimbCalculatorResults(
    locale: RwSignal<Locale>,
    result: RwSignal<Option<ClimbCalculatorResult>>,
    stale: Memo<bool>,
    breakdown_open: RwSignal<bool>,
) -> impl IntoView {
    view! {
        {move || {
            let Some(res) = result.get() else {
                return ().into_any();
            };
            let c = copy(locale.get());
            let oob = c.altitude_sweep_off_chart;
            let chart_wt = climb_chart_max_weight_lb();
            let fig_lbl = format_climb_figure_placard(locale.get(), res.climb_figure);
            let fair_lbl = if res.wheel_fairings_installed {
                c.climb_calc_fairings_on
            } else {
                c.climb_calc_fairings_off
            };
            let assumptions = format!(
                "{}: {} · {:.0} lb / {:.0} kg · {}",
                c.climb_calc_assumptions,
                fig_lbl,
                res.weight_lb,
                res.weight_lb / 2.2046226218,
                fair_lbl,
            );
            let vy_line =
                format_climb_vy_held(locale.get(), res.vy_kias, res.vy_cas_kt, res.vy_policy);
            view! {
                {stale.get().then(|| view! {
                    <p class="lab warn">{c.climb_calc_stale}</p>
                })}
                <p class="lab results-muted">{assumptions}</p>
                <p class="lab results-muted">{vy_line}</p>
                <p class="lab results-muted">
                    {format!("{:.0} lb chart datum", chart_wt.round())}
                </p>
                <div class="climb-calc-results" class:results-stale=move || stale.get()>
                    {res.total_time_s.map(|t| {
                        let label = format!("{}: {}", c.climb_calc_total_time, format_min_sec(t));
                        view! { <p class="lab readout-text">{label}</p> }
                    })}
                    {res.still_air_distance_nm.map(|nm| {
                        let label = format!("{}: {:.1} NM", c.climb_calc_still_air, nm);
                        view! { <p class="lab readout-text">{label}</p> }
                    })}
                    {res.average_roc_fpm.map(|roc| {
                        let label = format!("{}: {:.0} fpm", c.climb_calc_avg_roc, roc);
                        view! { <p class="lab readout-text">{label}</p> }
                    })}
                    {res.fuel_us_gal.map(|gal| {
                        let label = format!("{}: {:.2} US gal ({})", c.climb_calc_fuel_est, gal, c.climb_calc_fuel_est_note);
                        view! { <p class="lab readout-text results-muted">{label}</p> }
                    })}
                    {res.fuel_omit_reason.map(|r| {
                        let line = format_climb_fuel_omit_reason(locale.get(), r);
                        view! { <p class="lab results-muted">{line}</p> }
                    })}
                    {match res.stop_reason {
                        Some(ClimbSweepStopReason::OutOfChart) => res.out_of_chart_at_pa_ft.map(|pa| {
                            let label = format!("{} ({:.0} ft)", oob, pa);
                            view! { <p class="lab warn">{label}</p> }.into_any()
                        }),
                        Some(ClimbSweepStopReason::ServiceCeiling) => {
                            Some(view! { <p class="lab warn">{c.climb_sweep_service_ceiling}</p> }.into_any())
                        }
                        None => None,
                    }}
                </div>
                <button
                    type="button"
                    class="btn g climb-calc-breakdown-toggle"
                    aria-controls="climb-calc-breakdown"
                    aria-expanded=move || breakdown_open.get()
                    on:click=move |_| breakdown_open.update(|open| *open = !*open)
                >
                    {move || {
                        if breakdown_open.get() {
                            c.climb_calc_hide_breakdown.to_string()
                        } else {
                            c.climb_calc_show_breakdown.to_string()
                        }
                    }}
                </button>
                <Show when=move || breakdown_open.get()>
                    <div id="climb-calc-breakdown" class="sweep-table-wrap" tabindex="-1">
                        <table class="sweep-table mono">
                            <thead>
                                <tr>
                                    <th>{c.climb_sweep_col_pa}</th>
                                    <th>{c.climb_sweep_col_oat}</th>
                                    <th>{c.climb_sweep_col_isa}</th>
                                    <th>{c.climb_sweep_col_da}</th>
                                    <th>{c.climb_sweep_col_roc}</th>
                                    <th>{c.climb_sweep_col_time}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {res.rows.iter().map(|row| {
                                    let roc = row.rate_of_climb_fpm.map(|v| format!("{:.0}", v.round()));
                                    let da = row.density_altitude_ft.map(|v| format!("{:.0}", v.round()));
                                    let t = row.cumulative_time_s.map(format_min_sec);
                                    view! {
                                        <tr>
                                            <td>{format!("{:.0}", row.pa_ft.round())}</td>
                                            <td>{format!("{:.1}", row.oat_c)}</td>
                                            <td>{format!("{:.1}", row.isa_dev_c)}</td>
                                            <td>{da.unwrap_or_else(|| oob.into())}</td>
                                            <td>{roc.unwrap_or_else(|| oob.into())}</td>
                                            <td>{t.unwrap_or_else(|| "—".into())}</td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                </Show>
            }.into_any()
        }}
    }
}
