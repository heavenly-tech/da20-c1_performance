//! Solve pressure altitude (fan row) for a target equivalent altitude at a given OAT (Fig 5.2).

use crate::rev29::types::Rev29File;

fn ea_at_pa_row(data: &Rev29File, row_idx: usize, oat_c: f64) -> f64 {
    let fan_t = &data.fan_t.value;
    let fan_ea = &data.fan_ea.value;
    let row = &fan_ea[row_idx];
    let xs: Vec<f64> = fan_t.to_vec();
    let ys: Vec<f64> = row.to_vec();
    crate::rev29::engine::linear_interp_quiet(&xs, &ys, oat_c)
}

/// Pressure altitude (ft) on the Fig 5.2 fan chart such that EA matches `ea_target` at `oat_c`.
/// Assumes EA increases with PA along digitized rows (AFM fan chart).
pub fn pressure_altitude_for_equivalent_altitude(
    data: &Rev29File,
    ea_target: f64,
    oat_c: f64,
) -> f64 {
    let fan_pa = &data.fan_pa.value;
    let n = fan_pa.len();
    match n {
        0 => 0.0,
        1 => fan_pa[0],
        _ => {
            let mut eas: Vec<f64> = (0..n).map(|j| ea_at_pa_row(data, j, oat_c)).collect();
            // Enforce monotonic non-decreasing EA vs PA index (clip inversions for robustness).
            for i in 1..n {
                if eas[i] < eas[i - 1] {
                    eas[i] = eas[i - 1];
                }
            }
            let ea_clamped = ea_target.clamp(eas[0], eas[n - 1]);
            match eas
                .windows(2)
                .position(|w| ea_clamped >= w[0] && ea_clamped <= w[1])
            {
                Some(j) => crate::interpolation::lerp(
                    eas[j],
                    fan_pa[j],
                    eas[j + 1],
                    fan_pa[j + 1],
                    ea_clamped,
                ),
                None => {
                    if ea_clamped <= eas[0] {
                        fan_pa[0]
                    } else {
                        fan_pa[n - 1]
                    }
                }
            }
        }
    }
}
