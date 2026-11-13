use da20_core::cruise::table3_trace_uses_cell;
use da20_core::mt175r::cruise::mt_fig4_trace_uses_cell;
use da20_core::types::{
    MtFig2SourceTrace, MtFig3SourceTrace, MtFig4SourceTrace, Table3SourceTrace, Table4SourceTrace,
};
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

#[component]
pub fn MtFig2Fig3SourcePanel(
    locale: Locale,
    fig2: Option<MtFig2SourceTrace>,
    fig3: Option<MtFig3SourceTrace>,
) -> impl IntoView {
    let c = copy(locale);
    let Some(f2) = fig2 else {
        return view! { <span></span> }.into_any();
    };
    let weights2: String = f2
        .curve_brackets
        .iter()
        .map(|b| format!("{}%={:.3}", b.pct_bhp, b.weight))
        .collect::<Vec<_>>()
        .join(", ");
    let weights3 = fig3
        .as_ref()
        .map(|f3| {
            f3.curve_brackets
                .iter()
                .map(|b| format!("{}%={:.3}", b.pct_bhp, b.weight))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    view! {
        <h4 class="table-source-title">"Fig 2 / Fig 3"</h4>
        <div class="table-source-block">
            <p class="mono table-source-meta">{format!(
                "Fig 2 · {}: {} · RPM {:.1}",
                c.mt_pct_bhp_weight_lbl,
                weights2,
                f2.rpm_result,
            )}</p>
            {fig3.map(|f3| view! {
                <p class="mono table-source-meta">{format!(
                    "Fig 3 · {}: {} · TAS {:.1} kt",
                    c.mt_pct_bhp_weight_lbl,
                    weights3,
                    f3.tas_result_kt,
                )}</p>
            })}
        </div>
    }
    .into_any()
}

#[component]
pub fn MtFig4SourcePanel(locale: Locale, #[prop(into)] trace: MtFig4SourceTrace) -> impl IntoView {
    let c = copy(locale);
    view! {
        <div class="table-source-block">
            <h4 class="table-source-title">{c.table_rows_used_title}</h4>
            <div class="table-scroll">
                {trace
                    .da_rows
                    .iter()
                    .map(|da_row| {
                        view! {
                            <div class="table-source-pa-group">
                                <p class="mono table-source-meta">{format!(
                                    "{} {} ft · {} {:.1} · {} {:.4}",
                                    c.mt_fig4_col_da,
                                    da_row.da_ft as i64,
                                    c.table_source_rpm_sample_lbl,
                                    da_row.rpm_sample,
                                    c.mt_fig4_da_weight_lbl,
                                    da_row.da_weight,
                                )}</p>
                                {da_row
                                    .rpm_rows
                                    .iter()
                                    .map(|rpm_row| {
                                        view! {
                                            <table class="data-table table-source-rpm">
                                                <thead>
                                                    <tr>
                                                        <th>{c.mt_fig4_col_rpm}</th>
                                                        <th>{c.table_source_rpm_weight_lbl}</th>
                                                        <th>{c.cruise_tbl_pct_bhp}</th>
                                                        <th>{c.cruise_tbl_ktas}</th>
                                                        <th>{c.cruise_tbl_gph}</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <tr class="row-source-band">
                                                        <td class="mono">{rpm_row.rpm as i64}</td>
                                                        <td class="mono">{format!("{:.4}", rpm_row.rpm_weight)}</td>
                                                        {rpm_row.cells.first().map(|cell| {
                                                            let used_cls = if cell.used {
                                                                "cell-used-emphasis"
                                                            } else {
                                                                ""
                                                            };
                                                            view! {
                                                                <>
                                                                    <td class=format!("mono {used_cls}")>
                                                                        {if cell.used {
                                                                            format!("{:.2}", cell.percent_bhp)
                                                                        } else {
                                                                            "—".into()
                                                                        }}
                                                                    </td>
                                                                    <td class=format!("mono {used_cls}")>
                                                                        {if cell.used {
                                                                            format!("{:.0}", cell.tas_kt)
                                                                        } else {
                                                                            "—".into()
                                                                        }}
                                                                    </td>
                                                                    <td class=format!("mono {used_cls}")>
                                                                        {if cell.used {
                                                                            format!("{:.1}", cell.gph)
                                                                        } else {
                                                                            "—".into()
                                                                        }}
                                                                    </td>
                                                                </>
                                                            }
                                                        })}
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

pub fn mt_fig4_table_row_highlight(
    trace: Option<&MtFig4SourceTrace>,
    da_ft: f64,
    rpm: f64,
) -> bool {
    let Some(t) = trace else {
        return false;
    };
    t.da_rows.iter().any(|dr| {
        (dr.da_ft - da_ft).abs() < 0.5 && dr.rpm_rows.iter().any(|rr| (rr.rpm - rpm).abs() < 0.5)
    })
}

pub fn mt_fig4_table_cell_highlight(
    trace: Option<&MtFig4SourceTrace>,
    da_ft: f64,
    rpm: f64,
) -> bool {
    trace
        .map(|t| mt_fig4_trace_uses_cell(t, da_ft, rpm))
        .unwrap_or(false)
}

pub fn landing_table_row_highlight(trace: Option<&Table4SourceTrace>, pa: f64) -> bool {
    let Some(t) = trace else {
        return false;
    };
    t.pa_rows.iter().any(|r| (r.pa_ft - pa).abs() < 0.5)
}
