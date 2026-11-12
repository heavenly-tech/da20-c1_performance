//! Solve pressure altitude (fan row) for a target equivalent altitude at a given OAT (Fig 5.2).

use crate::chart_bounds::{limits, CHART_EDGE_TOL};
use crate::rev29::types::Rev29File;

fn ea_at_pa_row(data: &Rev29File, row_idx: usize, oat_c: f64) -> Option<f64> {
    let fan_t = &data.fan_t.value;
    let fan_ea = &data.fan_ea.value;
    let row = &fan_ea[row_idx];
    let xs: Vec<f64> = fan_t.to_vec();
    let ys: Vec<f64> = row.to_vec();
    crate::rev29::engine::linear_interp(&xs, &ys, oat_c, "Fig 5.2 OAT")
        .ok()
        .map(|(v, _)| v)
}

/// Pressure altitude (ft) on the Fig 5.2 fan chart such that EA matches `ea_target` at `oat_c`.
/// Returns `None` when OAT or EA is outside the digitized chart hull (no clamped PA).
pub fn pressure_altitude_for_equivalent_altitude(
    data: &Rev29File,
    ea_target: f64,
    oat_c: f64,
) -> Option<f64> {
    if oat_c < limits::FIG52_OAT_C.0 - CHART_EDGE_TOL
        || oat_c > limits::FIG52_OAT_C.1 + CHART_EDGE_TOL
    {
        return None;
    }

    let fan_pa = &data.fan_pa.value;
    let n = fan_pa.len();
    match n {
        0 => None,
        1 => Some(fan_pa[0]),
        _ => {
            let eas: Vec<f64> = (0..n)
                .map(|j| ea_at_pa_row(data, j, oat_c))
                .collect::<Option<Vec<_>>>()?;
            let mut eas = eas;
            for i in 1..n {
                if eas[i] < eas[i - 1] {
                    eas[i] = eas[i - 1];
                }
            }
            let ea_min = eas[0];
            let ea_max = eas[n - 1];
            if ea_target < ea_min - CHART_EDGE_TOL || ea_target > ea_max + CHART_EDGE_TOL {
                return None;
            }
            eas.windows(2)
                .position(|w| {
                    ea_target >= w[0] - CHART_EDGE_TOL && ea_target <= w[1] + CHART_EDGE_TOL
                })
                .map(|j| {
                    crate::interpolation::lerp(
                        eas[j],
                        fan_pa[j],
                        eas[j + 1],
                        fan_pa[j + 1],
                        ea_target,
                    )
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rev29::engine::equivalent_altitude_ft;
    use crate::rev29::Rev29File;

    fn sample() -> Rev29File {
        Rev29File::production()
    }

    #[test]
    fn inverse_matches_forward_at_mid_chart() {
        let data = sample();
        let pa = 6000.0;
        let oat = 15.0;
        let ea = equivalent_altitude_ft(&data, pa, oat).unwrap().0;
        let back = pressure_altitude_for_equivalent_altitude(&data, ea, oat).unwrap();
        assert!((back - pa).abs() < 1.0, "back={back} pa={pa}");
    }

    #[test]
    fn inverse_none_when_ea_below_chart_hull() {
        let data = sample();
        let oat = 15.0;
        let n = data.fan_pa.value.len();
        let eas_min = ea_at_pa_row(&data, 0, oat).unwrap();
        let eas_max = ea_at_pa_row(&data, n - 1, oat).unwrap();
        assert!(pressure_altitude_for_equivalent_altitude(&data, eas_min - 100.0, oat).is_none());
        assert!(pressure_altitude_for_equivalent_altitude(&data, eas_max + 100.0, oat).is_none());
    }
}
