//! AFM §5.3.1 Figure 5.1 airspeed calibration (KIAS → KCAS at max weight).

use crate::interpolation::lerp;
use crate::types::ClimbFigure;

#[derive(Clone, Copy)]
struct Table1Curve {
    kias: &'static [f64],
    kcas: &'static [f64],
}

/// Flaps CRUISE (Fig 5.1 upper curve).
const FLAPS_CRUISE: Table1Curve = Table1Curve {
    kias: &[42.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0, 80.0, 90.0, 100.0],
    kcas: &[52.0, 58.0, 62.0, 66.0, 70.0, 75.0, 79.0, 83.0, 92.0, 101.0],
};

/// Flaps T/O.
const FLAPS_TO: Table1Curve = Table1Curve {
    kias: &[
        38.0, 45.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0, 80.0, 85.0, 90.0,
    ],
    kcas: &[
        48.0, 53.0, 57.0, 61.0, 65.0, 69.0, 73.0, 77.0, 81.0, 85.0, 89.0,
    ],
};

/// Flaps LDG (balked landing ROC chart).
const FLAPS_LDG: Table1Curve = Table1Curve {
    kias: &[34.0, 40.0, 45.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0],
    kcas: &[44.0, 48.0, 52.0, 55.0, 59.0, 64.0, 68.0, 72.0, 76.0],
};

fn curve_for_figure(figure: ClimbFigure) -> &'static Table1Curve {
    match figure {
        ClimbFigure::Fig55CruiseFlaps => &FLAPS_CRUISE,
        ClimbFigure::Fig56TakeoffFlaps => &FLAPS_TO,
        ClimbFigure::Fig58BalkedLanding => &FLAPS_LDG,
    }
}

/// Linear interpolation on AFM Fig 5.1 (Table 1) for the flap setting matching the ROC figure.
pub fn table1_kcas_from_kias(kias: f64, figure: ClimbFigure) -> Option<f64> {
    let c = curve_for_figure(figure);
    if kias < c.kias[0] || kias > *c.kias.last()? {
        return None;
    }
    for i in 0..c.kias.len().saturating_sub(1) {
        let lo_i = c.kias[i];
        let hi_i = c.kias[i + 1];
        if kias >= lo_i - 1e-9 && kias <= hi_i + 1e-9 {
            let t = if (hi_i - lo_i).abs() < 1e-9 {
                0.0
            } else {
                (kias - lo_i) / (hi_i - lo_i)
            };
            return Some(lerp(0.0, c.kcas[i], 1.0, c.kcas[i + 1], t));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table1_anchor_points_flaps_cruise() {
        assert!(
            (table1_kcas_from_kias(75.0, ClimbFigure::Fig55CruiseFlaps).unwrap() - 79.0).abs()
                < 1e-6
        );
    }

    #[test]
    fn table1_65_kias_flaps_to_equals_69_kcas() {
        let cas = table1_kcas_from_kias(65.0, ClimbFigure::Fig56TakeoffFlaps).expect("65 in range");
        assert!((cas - 69.0).abs() < 1e-6, "kcas={cas}");
    }

    #[test]
    fn table1_68_kias_flaps_to_interpolates_714() {
        let cas = table1_kcas_from_kias(68.0, ClimbFigure::Fig56TakeoffFlaps).unwrap();
        assert!((cas - 71.4).abs() < 0.05, "kcas={cas}");
    }

    #[test]
    fn table1_52_kias_ldg_flaps() {
        let cas = table1_kcas_from_kias(52.0, ClimbFigure::Fig58BalkedLanding).unwrap();
        assert!((cas - 56.6).abs() < 0.1, "kcas={cas}");
    }
}
