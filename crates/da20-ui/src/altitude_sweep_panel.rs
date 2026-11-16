//! Shared PA altitude sweep UI (climb ROC and cruise).

use da20_core::altitude::effective_pressure_altitude_ft;
use da20_core::altitude_sweep::{AltitudeSweepRangeError, DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT};
use da20_core::climb_sweep::{
    compute_climb_roc_sweep, ClimbSweepParams, ClimbSweepResult, ClimbSweepStopReason,
    CLIMB_SWEEP_DEFAULT_STEP_FT,
};
use da20_core::cruise_sweep::{
    compute_cruise_altitude_sweep, CruiseSweepParams, CruiseSweepResult,
    CRUISE_SWEEP_DEFAULT_STEP_FT,
};
use da20_core::types::{ChartPhase, CruiseSolveMode, FlightInputs};
use da20_core::PerformanceData;
use leptos::prelude::*;

use crate::i18n::{copy, Locale};
use crate::inputs::NumField;

#[derive(Clone, Copy, Debug, Default)]
pub struct PaSweepDraft {
    pub from: f64,
    pub to: f64,
    pub step: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct LapseDraft {
    pub per_1000_ft: f64,
}

impl Default for LapseDraft {
    fn default() -> Self {
        Self {
            per_1000_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT * 1000.0,
        }
    }
}

pub fn lapse_c_per_ft(d: LapseDraft) -> f64 {
    d.per_1000_ft / 1000.0
}

pub fn format_min_sec(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as i64;
    let min = total / 60;
    let sec = total % 60;
    format!("{min}:{sec:02}")
}

pub fn format_altitude_sweep_err(locale: Locale, err: AltitudeSweepRangeError) -> String {
    match locale {
        Locale::Es => match err {
            AltitudeSweepRangeError::NonFinite => "Valores no finitos.".into(),
            AltitudeSweepRangeError::ZeroStep => "El paso no puede ser cero.".into(),
            AltitudeSweepRangeError::RangeDirection => "Paso incompatible con desde/hasta.".into(),
            AltitudeSweepRangeError::EmptyRange => "Rango vacío.".into(),
            AltitudeSweepRangeError::TooManyRows { max } => {
                format!("Demasiadas filas (máx. {max}).")
            }
        },
        Locale::En => match err {
            AltitudeSweepRangeError::NonFinite => "Non-finite values.".into(),
            AltitudeSweepRangeError::ZeroStep => "Step cannot be zero.".into(),
            AltitudeSweepRangeError::RangeDirection => "Step sign does not match from/to.".into(),
            AltitudeSweepRangeError::EmptyRange => "Empty range.".into(),
            AltitudeSweepRangeError::TooManyRows { max } => {
                format!("Too many rows (max {max}).")
            }
        },
    }
}

#[component]
fn AltitudePaSweepFields(
    locale: RwSignal<Locale>,
    range: RwSignal<PaSweepDraft>,
    lapse: RwSignal<LapseDraft>,
    pa_min: f64,
    pa_max: f64,
) -> impl IntoView {
    view! {
        <div class="sweep-range">
            <NumField label=move || copy(locale.get()).takeoff_sweep_start.to_string()
                min=pa_min
                max=pa_max
                get=move || range.get().from
                set=move |v| range.update(|d| d.from = v)/>
            <NumField label=move || copy(locale.get()).takeoff_sweep_end.to_string()
                min=pa_min
                max=pa_max
                get=move || range.get().to
                set=move |v| range.update(|d| d.to = v)/>
            <NumField label=move || copy(locale.get()).takeoff_sweep_step.to_string()
                get=move || range.get().step
                set=move |v| range.update(|d| d.step = v)/>
        </div>
        <NumField label=move || copy(locale.get()).altitude_sweep_lapse_label.to_string()
            get=move || lapse.get().per_1000_ft
            set=move |v| lapse.update(|d| d.per_1000_ft = v)/>
        <p class="lab results-muted">{move || copy(locale.get()).altitude_sweep_lapse_note.to_string()}</p>
    }
}

#[component]
pub fn ClimbRocSweepPanel(
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    let range = RwSignal::new(PaSweepDraft::default());
    let lapse = RwSignal::new(LapseDraft::default());
    let error = RwSignal::new(Option::<String>::None);
    let result = RwSignal::new(Option::<ClimbSweepResult>::None);

    let init_defaults = move || {
        let i = inputs.get_untracked();
        let from = effective_pressure_altitude_ft(&i, Some(ChartPhase::Climb));
        let to = i.cruise_pressure_alt_ft;
        let step = if to >= from {
            CLIMB_SWEEP_DEFAULT_STEP_FT
        } else {
            -CLIMB_SWEEP_DEFAULT_STEP_FT
        };
        range.set(PaSweepDraft { from, to, step });
    };
    init_defaults();

    let run = move |_| {
        error.set(None);
        result.set(None);
        let r = range.get();
        let params = ClimbSweepParams {
            from_pa_ft: r.from,
            to_pa_ft: r.to,
            step_ft: r.step,
            lapse_rate_c_per_ft: lapse_c_per_ft(lapse.get()),
        };
        match compute_climb_roc_sweep(&data.get(), &inputs.get(), &params) {
            Ok(res) => result.set(Some(res)),
            Err(e) => error.set(Some(format_altitude_sweep_err(locale.get(), e))),
        }
    };

    view! {
        <div class="card sweep-card">
            <h3>{move || copy(locale.get()).climb_sweep_title.to_string()}</h3>
            <AltitudePaSweepFields locale=locale range=range lapse=lapse
                pa_min=da20_core::chart_bounds::limits::FIG52_PA_FT.0
                pa_max=da20_core::chart_bounds::limits::FIG52_PA_FT.1/>
            <button type="button" class="btn g" on:click=run>
                {move || copy(locale.get()).takeoff_sweep_run.to_string()}
            </button>
            {move || error.get().map(|e| view! { <p class="lab field-error">{e}</p> })}
            <ClimbRocSweepTable locale=locale result=result/>
        </div>
    }
}

#[component]
fn ClimbRocSweepTable(
    locale: RwSignal<Locale>,
    result: RwSignal<Option<ClimbSweepResult>>,
) -> impl IntoView {
    view! {
        {move || {
            let Some(res) = result.get() else {
                return ().into_any();
            };
            let c = copy(locale.get());
            let oob = c.altitude_sweep_off_chart;
            let has_oob = res.rows.iter().any(|r| r.rate_of_climb_fpm.is_none());
            view! {
                <div class="sweep-table-wrap">
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
                    {res.total_time_s.map(|t| {
                        let label = format!("{}: {}", c.climb_sweep_total_time, format_min_sec(t));
                        view! { <p class="lab readout-text">{label}</p> }
                    })}
                    {res.stop_reason.map(|reason| {
                        let msg = match reason {
                            ClimbSweepStopReason::ServiceCeiling => c.climb_sweep_service_ceiling,
                            ClimbSweepStopReason::OutOfChart => c.altitude_sweep_off_chart,
                        };
                        view! { <p class="lab warn">{msg}</p> }
                    })}
                    {has_oob.then(|| view! {
                        <p class="lab results-muted">{c.takeoff_sweep_oob_footnote}</p>
                    })}
                </div>
            }.into_any()
        }}
    }
}

#[component]
pub fn CruiseAltitudeSweepPanel(
    locale: RwSignal<Locale>,
    data: RwSignal<PerformanceData>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    let range = RwSignal::new(PaSweepDraft::default());
    let lapse = RwSignal::new(LapseDraft::default());
    let error = RwSignal::new(Option::<String>::None);
    let result = RwSignal::new(Option::<CruiseSweepResult>::None);

    let init_defaults = move || {
        let i = inputs.get_untracked();
        let from = i.cruise_pressure_alt_ft;
        let to = (from + 4000.0).min(da20_core::chart_bounds::limits::TABLE3_PA_FT.1);
        let step = CRUISE_SWEEP_DEFAULT_STEP_FT;
        range.set(PaSweepDraft { from, to, step });
    };
    init_defaults();

    let run = move |_| {
        error.set(None);
        result.set(None);
        let r = range.get();
        let params = CruiseSweepParams {
            from_pa_ft: r.from,
            to_pa_ft: r.to,
            step_ft: r.step,
            lapse_rate_c_per_ft: lapse_c_per_ft(lapse.get()),
        };
        match compute_cruise_altitude_sweep(&data.get(), &inputs.get(), &params) {
            Ok(res) => result.set(Some(res)),
            Err(e) => error.set(Some(format_altitude_sweep_err(locale.get(), e))),
        }
    };

    view! {
        <div class="card sweep-card">
            <h3>{move || copy(locale.get()).cruise_sweep_title.to_string()}</h3>
            <AltitudePaSweepFields locale=locale range=range lapse=lapse
                pa_min=da20_core::chart_bounds::limits::TABLE3_PA_FT.0
                pa_max=da20_core::chart_bounds::limits::TABLE3_PA_FT.1/>
            <button type="button" class="btn g" on:click=run>
                {move || copy(locale.get()).takeoff_sweep_run.to_string()}
            </button>
            {move || error.get().map(|e| view! { <p class="lab field-error">{e}</p> })}
            <CruiseSweepTable locale=locale result=result inputs=inputs/>
        </div>
    }
}

#[component]
fn CruiseSweepTable(
    locale: RwSignal<Locale>,
    result: RwSignal<Option<CruiseSweepResult>>,
    inputs: RwSignal<FlightInputs>,
) -> impl IntoView {
    view! {
        {move || {
            let Some(res) = result.get() else {
                return ().into_any();
            };
            let c = copy(locale.get());
            let oob = c.altitude_sweep_off_chart;
            let hold_bhp = inputs.get().cruise_solve_mode == CruiseSolveMode::SetPercentBhp;
            let best_tas = res.best_tas_row;
            let best_nm = res.best_nm_per_gal_row;
            let has_oob = res.rows.iter().any(|r| r.off_chart);
            view! {
                <div class="sweep-table-wrap">
                    <table class="sweep-table mono">
                        <thead>
                            <tr>
                                <th>{c.cruise_sweep_col_pa}</th>
                                <th>{c.cruise_sweep_col_oat}</th>
                                <th>{c.cruise_sweep_col_isa}</th>
                                <th>{c.cruise_sweep_col_da}</th>
                                {hold_bhp.then(|| view! { <th>{c.cruise_sweep_col_rpm}</th> })}
                                {(!hold_bhp).then(|| view! { <th>{c.cruise_sweep_col_bhp}</th> })}
                                <th>{c.cruise_sweep_col_tas}</th>
                                <th>{c.cruise_sweep_col_gph}</th>
                                <th>{c.cruise_sweep_col_nm_gal}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {res.rows.iter().enumerate().map(|(idx, row)| {
                                let marker = if Some(idx) == best_tas {
                                    format!(" [{}]", c.cruise_sweep_best_tas)
                                } else if Some(idx) == best_nm {
                                    format!(" [{}]", c.cruise_sweep_best_efficiency)
                                } else {
                                    String::new()
                                };
                                if row.off_chart {
                                    return view! {
                                        <tr>
                                            <td>{format!("{:.0}", row.pa_ft.round())}</td>
                                            <td>{format!("{:.1}", row.oat_c)}</td>
                                            <td>{format!("{:.1}", row.isa_dev_c)}</td>
                                            <td colspan="4">{oob}{marker}</td>
                                        </tr>
                                    }.into_any();
                                }
                                let rpm = row.rpm.map(|v| format!("{:.0}", v.round()));
                                let bhp = row.percent_bhp.map(|v| format!("{:.1}", v));
                                let tas = row.tas_kt.map(|v| format!("{:.0}", v.round()));
                                let gph = row.fuel_flow_us_gal_h.map(|v| format!("{:.1}", v));
                                let nm = row.nm_per_gal.map(|v| format!("{:.2}", v));
                                view! {
                                    <tr>
                                        <td>{format!("{:.0}", row.pa_ft.round())}</td>
                                        <td>{format!("{:.1}", row.oat_c)}</td>
                                        <td>{format!("{:.1}", row.isa_dev_c)}</td>
                                        <td>{row.density_altitude_ft.map(|v| format!("{:.0}", v.round())).unwrap_or_else(|| oob.into())}</td>
                                        {hold_bhp.then(|| view! {
                                            <td>{rpm.unwrap_or_else(|| "—".into())}</td>
                                        })}
                                        {(!hold_bhp).then(|| view! {
                                            <td>{bhp.unwrap_or_else(|| "—".into())}</td>
                                        })}
                                        <td>{tas.unwrap_or_else(|| oob.into())}{marker}</td>
                                        <td>{gph.unwrap_or_else(|| "—".into())}</td>
                                        <td>{nm.unwrap_or_else(|| "—".into())}</td>
                                    </tr>
                                }.into_any()
                            }).collect_view()}
                        </tbody>
                    </table>
                    {has_oob.then(|| view! {
                        <p class="lab results-muted">{c.takeoff_sweep_oob_footnote}</p>
                    })}
                </div>
            }.into_any()
        }}
    }
}
