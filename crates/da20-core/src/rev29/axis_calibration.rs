//! Endpoint calibration for digitized AFM axes (source-overlay pixel space).
//!
//! Axes whose stored endpoint sits within [`SNAP_TOLERANCE_PX`] of a published gridline
//! are snapped onto that gridline. Tailwind kt uses proportional rescale from 0 so the
//! last knot lands exactly on the 5 kt gridline without changing curve samples (`tw_m`).

use crate::chart_bounds::limits;

pub const SNAP_TOLERANCE_PX: f64 = 1.0;

const M_TO_FT: f64 = 3.280839895;

/// Linear map between data value and horizontal/vertical pixel on the source AFM overlay
/// (Rev 29 Fig 5.4 JPEG extracts on PDF pp. 145–146; climb Fig 5.2 p. 142; Table 4 p. 153).
#[derive(Clone, Copy, Debug)]
pub struct LinearAxisPx {
    pub label: &'static str,
    pub data_min: f64,
    pub data_max_grid: f64,
    pub pixel_at_min: f64,
    pub pixel_at_max: f64,
    /// `true`: value rises with pixel coordinate; `false`: value falls with pixel.
    pub increasing: bool,
}

impl LinearAxisPx {
    pub fn pixel_for(&self, value: f64) -> f64 {
        let span = self.data_max_grid - self.data_min;
        if span.abs() < f64::EPSILON {
            return self.pixel_at_max;
        }
        let t = ((value - self.data_min) / span).clamp(0.0, 1.0);
        if self.increasing {
            self.pixel_at_min + t * (self.pixel_at_max - self.pixel_at_min)
        } else {
            self.pixel_at_max - t * (self.pixel_at_max - self.pixel_at_min)
        }
    }

    pub fn endpoint_snap_action(&self, endpoint: f64) -> EndpointAction {
        let px_end = self.pixel_for(endpoint);
        let px_grid = self.pixel_for(self.data_max_grid);
        if (px_end - px_grid).abs() <= SNAP_TOLERANCE_PX {
            return EndpointAction::Snap {
                old_endpoint: endpoint,
                new_endpoint: self.data_max_grid,
            };
        }
        EndpointAction::Left {
            old_endpoint: endpoint,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EndpointAction {
    Snap {
        old_endpoint: f64,
        new_endpoint: f64,
    },
    Left {
        old_endpoint: f64,
    },
    /// Proportional rescale from axis origin (tailwind kt).
    RescaleProportional {
        old_endpoint: f64,
        new_endpoint: f64,
        scale: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AxisAuditRow {
    pub axis: &'static str,
    pub old_endpoint: f64,
    pub new_endpoint: f64,
    pub action: &'static str,
}

/// Published overlay pixel anchors (sub-pixel grid-line peaks on source JPEGs).
pub mod overlay {
    use super::LinearAxisPx;
    use crate::chart_bounds::limits;

    pub const TW_KT: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 tailwind (tw_kt)",
        data_min: 0.0,
        data_max_grid: limits::FIG54_TAILWIND_KT.1,
        pixel_at_min: 489.0,
        pixel_at_max: 442.0,
        increasing: false,
    };

    pub const HW_KT: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 headwind (hw_kt)",
        data_min: 0.0,
        data_max_grid: limits::FIG54_HEADWIND_KT.1,
        pixel_at_min: 489.0,
        pixel_at_max: 792.0,
        increasing: true,
    };

    pub const WEIGHT_KG: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 weight (wt_kg)",
        data_min: limits::FIG54_WEIGHT_KG.0,
        data_max_grid: limits::FIG54_WEIGHT_KG.1,
        pixel_at_min: 318.0,
        pixel_at_max: 57.0,
        increasing: false,
    };

    pub const TO_OAT_C: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 OAT (to_T)",
        data_min: limits::FIG54_OAT_C.0,
        data_max_grid: limits::FIG54_OAT_C.1,
        pixel_at_min: 41.0,
        pixel_at_max: 619.0,
        increasing: true,
    };

    pub const TO_PA_FT: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 pressure altitude (to_PA)",
        data_min: limits::FIG54_PA_FT.0,
        data_max_grid: limits::FIG54_PA_FT.1,
        pixel_at_min: 0.0,
        pixel_at_max: 0.0,
        increasing: true,
    };

    pub const FAN_OAT_C: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.2 OAT (fan_T)",
        data_min: limits::FIG52_OAT_C.0,
        data_max_grid: limits::FIG52_OAT_C.1,
        pixel_at_min: 63.0,
        pixel_at_max: 705.0,
        increasing: true,
    };

    pub const FAN_PA_FT: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.2 pressure altitude (fan_PA)",
        data_min: limits::FIG52_PA_FT.0,
        data_max_grid: limits::FIG52_PA_FT.1,
        pixel_at_min: 662.0,
        pixel_at_max: 38.0,
        increasing: false,
    };

    pub const LAND_PA_FT: LinearAxisPx = LinearAxisPx {
        label: "Table 4 pressure altitude (land_alt)",
        data_min: limits::TABLE4_PA_FT.0,
        data_max_grid: limits::TABLE4_PA_FT.1,
        pixel_at_min: 0.0,
        pixel_at_max: 0.0,
        increasing: true,
    };

    /// Obstacle height on sheet 2 (horizontal axis, m); 15 m aligns with the AFM screen line.
    pub const OBSTACLE_HEIGHT_M: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 obstacle height (m)",
        data_min: 0.0,
        data_max_grid: 15.0,
        pixel_at_min: 841.0,
        pixel_at_max: 961.0,
        increasing: true,
    };

    /// Same panel, ft tick at 50 ft (49.2126 m = 15 m; digitized overlay peak ~49.2 ft).
    pub const OBSTACLE_HEIGHT_FT_HINT: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 obstacle height (50 ft hint)",
        data_min: 0.0,
        data_max_grid: 50.0,
        pixel_at_min: 841.0,
        pixel_at_max: 961.0,
        increasing: true,
    };

    pub const HW_LINES_M: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 HW panel distance (hw_lines)",
        data_min: 300.0,
        data_max_grid: 1000.0,
        pixel_at_min: 612.0,
        pixel_at_max: 40.0,
        increasing: false,
    };

    pub const TW_LINES_M: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 TW panel distance (tw_lines)",
        data_min: 200.0,
        data_max_grid: 800.0,
        pixel_at_min: 612.0,
        pixel_at_max: 40.0,
        increasing: false,
    };

    pub const OBS_LINES_M: LinearAxisPx = LinearAxisPx {
        label: "Fig 5.4 obstacle panel ground line (obs_lines)",
        data_min: 200.0,
        data_max_grid: 800.0,
        pixel_at_min: 612.0,
        pixel_at_max: 40.0,
        increasing: false,
    };
}

/// Rescale tailwind kt knots proportionally so the former endpoint maps to `target_max`.
pub fn rescale_tailwind_knots(axis: &mut [f64], target_max: f64) -> Option<AxisAuditRow> {
    if axis.is_empty() {
        return None;
    }
    let old_end = *axis.last().unwrap();
    if (old_end - target_max).abs() < 1e-9 {
        return None;
    }
    if old_end <= 0.0 {
        return None;
    }
    let scale = target_max / old_end;
    for v in axis.iter_mut() {
        *v *= scale;
    }
    axis[axis.len() - 1] = target_max;
    Some(AxisAuditRow {
        axis: overlay::TW_KT.label,
        old_endpoint: old_end,
        new_endpoint: target_max,
        action: "rescale proportional",
    })
}

fn audit_value_axis(spec: &LinearAxisPx, endpoint: f64) -> AxisAuditRow {
    match spec.endpoint_snap_action(endpoint) {
        EndpointAction::Snap {
            old_endpoint,
            new_endpoint,
        } => AxisAuditRow {
            axis: spec.label,
            old_endpoint,
            new_endpoint,
            action: "snapped",
        },
        EndpointAction::Left { old_endpoint } => AxisAuditRow {
            axis: spec.label,
            old_endpoint,
            new_endpoint: old_endpoint,
            action: "left",
        },
        EndpointAction::RescaleProportional { .. } => unreachable!(),
    }
}

fn snap_endpoint_if_within_px(axis: &mut [f64], spec: &LinearAxisPx) -> AxisAuditRow {
    if axis.is_empty() {
        return AxisAuditRow {
            axis: spec.label,
            old_endpoint: f64::NAN,
            new_endpoint: f64::NAN,
            action: "left",
        };
    }
    let end = *axis.last().unwrap();
    let row = audit_value_axis(spec, end);
    if row.action == "snapped" {
        axis[axis.len() - 1] = row.new_endpoint;
    }
    row
}

/// Apply endpoint calibration to Rev 29 takeoff/climb/landing axes after JSON load.
#[allow(clippy::too_many_arguments)]
pub fn calibrate_rev29_axes(
    tw_kt: &mut [f64],
    hw_kt: &mut [f64],
    wt_kg: &mut [f64],
    to_t: &mut [f64],
    to_pa: &mut [f64],
    fan_t: &mut [f64],
    fan_pa: &mut [f64],
    land_alt: &mut [f64],
    hw_lines: &mut [f64],
    tw_lines: &mut [f64],
    obs_lines: &mut [f64],
) -> Vec<AxisAuditRow> {
    let mut rows = Vec::new();

    if let Some(row) = rescale_tailwind_knots(tw_kt, limits::FIG54_TAILWIND_KT.1) {
        rows.push(row);
    } else {
        let end = tw_kt.last().copied().unwrap_or(f64::NAN);
        rows.push(AxisAuditRow {
            axis: overlay::TW_KT.label,
            old_endpoint: end,
            new_endpoint: end,
            action: "left",
        });
    }

    rows.push(snap_endpoint_if_within_px(hw_kt, &overlay::HW_KT));
    rows.push(snap_endpoint_if_within_px(wt_kg, &overlay::WEIGHT_KG));
    rows.push(snap_endpoint_if_within_px(to_t, &overlay::TO_OAT_C));
    rows.push(snap_endpoint_if_within_px(to_pa, &overlay::TO_PA_FT));
    rows.push(snap_endpoint_if_within_px(fan_t, &overlay::FAN_OAT_C));
    rows.push(snap_endpoint_if_within_px(fan_pa, &overlay::FAN_PA_FT));
    rows.push(snap_endpoint_if_within_px(land_alt, &overlay::LAND_PA_FT));
    rows.push(snap_endpoint_if_within_px(hw_lines, &overlay::HW_LINES_M));
    rows.push(snap_endpoint_if_within_px(tw_lines, &overlay::TW_LINES_M));
    rows.push(snap_endpoint_if_within_px(obs_lines, &overlay::OBS_LINES_M));

    let m15 = 15.0;
    rows.push(audit_value_axis(&overlay::OBSTACLE_HEIGHT_M, m15));
    let ft_from_m15 = m15 * M_TO_FT;
    rows.push(audit_value_axis(
        &overlay::OBSTACLE_HEIGHT_FT_HINT,
        (ft_from_m15 * 10.0).round() / 10.0,
    ));

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_rescale_maps_old_endpoint_to_five_kt() {
        let mut kt = vec![0.0, 0.76, 1.81, 2.85, 3.9, 4.84];
        let row = rescale_tailwind_knots(&mut kt, 5.0).unwrap();
        assert_eq!(row.new_endpoint, 5.0);
        assert!((kt[1] - 0.7851239669421487).abs() < 1e-9);
        assert_eq!(kt.last().copied().unwrap(), 5.0);
    }

    #[test]
    fn five_kt_tw_in_chart_five_point_one_out() {
        use crate::rev29::Rev29File;
        let data = Rev29File::production();
        let entry = 400.0;
        assert!(
            crate::rev29::engine::wind_panel_tw_m(&data, entry, 5.0).is_ok(),
            "5.0 kt TW must interpolate on calibrated tw_kt"
        );
        assert!(
            crate::rev29::engine::wind_panel_tw_m(&data, entry, 5.1).is_err(),
            "5.1 kt TW is past the 5 kt gridline"
        );
    }

    #[test]
    fn eight_hundred_kg_on_chart() {
        use crate::rev29::Rev29File;
        let data = Rev29File::production();
        assert!(crate::rev29::engine::weight_factor(&data, 800.0).is_ok());
        assert!(crate::rev29::engine::weight_factor(&data, 801.0).is_err());
    }
}
