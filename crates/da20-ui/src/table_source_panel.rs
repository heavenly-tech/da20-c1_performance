use da20_core::cruise::table3_trace_uses_cell;
use da20_core::types::{Table3SourceTrace, Table4SourceTrace};
use leptos::prelude::*;

use crate::i18n::{copy, Locale};

const DISA_COLS: [f64; 3] = [-20.0, 0.0, 10.0];

#[component]
pub fn Table3SourcePanel(locale: Locale, #[prop(into)] trace: Table3SourceTrace) -> impl IntoView {
    let c = copy(locale);
    view! {
        <div class="table-source-block">
            <h4 class="table-source-title">{c.table_rows_used_title}</h4>
            <div class="table-scroll">
                {trace
                    .pa_rows
                    .iter()
                    .map(|pa_row| {
                        view! {
                            <div class="table-source-pa-group">
                                <p class="mono table-source-meta">{format!(
                                    "{} {} ft · {} {:.1} · {} {:.4}",
                                    c.table_source_pa_lbl,
                                    pa_row.pa_ft as i64,
                                    c.table_source_rpm_sample_lbl,
                                    pa_row.rpm_sample,
                                    c.table_source_pa_weight_lbl,
                                    pa_row.pa_weight,
                                )}</p>
                                {pa_row
                                    .rpm_rows
                                    .iter()
                                    .map(|rpm_row| {
                                        view! {
                                            <table class="data-table table-source-rpm">
                                                <thead>
                                                    <tr>
                                                        <th>{c.table_source_rpm_lbl}</th>
                                                        <th>{c.table_source_rpm_weight_lbl}</th>
                                                        <th colspan=3>{c.cruise_tbl_group_disa_m20}</th>
                                                        <th colspan=3>{c.cruise_tbl_group_isa}</th>
                                                        <th colspan=3>{c.cruise_tbl_group_disa_p10}</th>
                                                    </tr>
                                                    <tr>
                                                        <th></th>
                                                        <th></th>
                                                        {(0..3).map(|_| {
                                                            view! {
                                                                <>
                                                                    <th>{c.cruise_tbl_pct_bhp}</th>
                                                                    <th>{c.cruise_tbl_ktas}</th>
                                                                    <th>{c.cruise_tbl_gph}</th>
                                                                </>
                                                            }
                                                        }).collect_view()}
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <tr class="row-source-band">
                                                        <td class="mono">{rpm_row.rpm as i64}</td>
                                                        <td class="mono">{format!("{:.4}", rpm_row.rpm_weight)}</td>
                                                        {DISA_COLS
                                                            .into_iter()
                                                            .map(|disa| {
                                                                let cell = rpm_row.cells.iter().find(|c| {
                                                                    (c.disa_c - disa).abs() < 1e-4
                                                                });
                                                                let (b, t, g, w, used) = cell
                                                                    .map(|c| {
                                                                        (
                                                                            c.percent_bhp,
                                                                            c.tas_kt,
                                                                            c.gph,
                                                                            c.disa_weight,
                                                                            c.disa_used,
                                                                        )
                                                                    })
                                                                    .unwrap_or((0.0, 0.0, 0.0, 0.0, false));
                                                                let used_cls = if used {
                                                                    "cell-used-emphasis"
                                                                } else {
                                                                    ""
                                                                };
                                                                view! {
                                                                    <>
                                                                        <td class=format!("mono {used_cls}")>
                                                                            {if used { format!("{b:.2}") } else { "—".into() }}
                                                                        </td>
                                                                        <td class=format!("mono {used_cls}")>
                                                                            {if used { format!("{t:.0}") } else { "—".into() }}
                                                                        </td>
                                                                        <td class=format!("mono {used_cls}")>
                                                                            {if used {
                                                format!("{g:.1} ({w:.3})")
                                            } else {
                                                "—".into()
                                            }}
                                                                        </td>
                                                                    </>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </tr>
                                                </tbody>
                                            </table>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn Table4SourcePanel(locale: Locale, #[prop(into)] trace: Table4SourceTrace) -> impl IntoView {
    let c = copy(locale);
    view! {
        <div class="table-source-block">
            <h4 class="table-source-title">{c.table_rows_used_title}</h4>
            <div class="table-scroll">
                <table class="data-table table-source-rpm">
                    <thead>
                        <tr>
                            <th>{c.table_source_pa_lbl}</th>
                            <th>{c.table_source_pa_weight_lbl}</th>
                            <th>{c.table_source_landing_roll_ft}</th>
                            <th>{c.table_source_landing_over_50ft_ft}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {trace
                            .pa_rows
                            .iter()
                            .map(|r| {
                                view! {
                                    <tr class="row-source-band">
                                        <td class="mono cell-used-emphasis">{r.pa_ft as i64}</td>
                                        <td class="mono cell-used-emphasis">{format!("{:.4}", r.pa_weight)}</td>
                                        <td class="mono cell-used-emphasis">{r.ground_roll_ft as i64}</td>
                                        <td class="mono cell-used-emphasis">{r.distance_over_50ft_ft as i64}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

pub fn cruise_table_row_highlight(trace: Option<&Table3SourceTrace>, pa: f64, rpm: f64) -> bool {
    let Some(t) = trace else {
        return false;
    };
    t.pa_rows.iter().any(|pr| {
        (pr.pa_ft - pa).abs() < 0.5 && pr.rpm_rows.iter().any(|rr| (rr.rpm - rpm).abs() < 0.5)
    })
}

pub fn cruise_table_cell_highlight(
    trace: Option<&Table3SourceTrace>,
    pa: f64,
    rpm: f64,
    disa_c: f64,
) -> bool {
    trace
        .map(|t| table3_trace_uses_cell(t, pa, rpm, disa_c))
        .unwrap_or(false)
}

pub fn landing_table_row_highlight(trace: Option<&Table4SourceTrace>, pa: f64) -> bool {
    let Some(t) = trace else {
        return false;
    };
    t.pa_rows.iter().any(|r| (r.pa_ft - pa).abs() < 0.5)
}
