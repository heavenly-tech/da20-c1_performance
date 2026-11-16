//! Cruise performance vs pressure-altitude sweep.

use crate::altitude::isa_troposphere_density_altitude_ft;
use crate::altitude_sweep::{
    build_altitude_series, oat_at_pa_with_lapse, AltitudeSweepRangeError,
    DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
};
use crate::cruise::{compute_cruise, disa_c, CruiseError};
use crate::data::PerformanceData;
use crate::types::{CruiseSolveMode, FlightInputs};

pub const CRUISE_SWEEP_MAX_ROWS: usize = 60;
pub const CRUISE_SWEEP_DEFAULT_STEP_FT: f64 = 1000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CruiseSweepParams {
    pub from_pa_ft: f64,
    pub to_pa_ft: f64,
    pub step_ft: f64,
    pub lapse_rate_c_per_ft: f64,
}

impl Default for CruiseSweepParams {
    fn default() -> Self {
        Self {
            from_pa_ft: 0.0,
            to_pa_ft: 0.0,
            step_ft: CRUISE_SWEEP_DEFAULT_STEP_FT,
            lapse_rate_c_per_ft: DEFAULT_SWEEP_LAPSE_RATE_C_PER_FT,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CruiseSweepRow {
    pub pa_ft: f64,
    pub oat_c: f64,
    pub isa_dev_c: f64,
    pub density_altitude_ft: Option<f64>,
    pub rpm: Option<f64>,
    pub percent_bhp: Option<f64>,
    pub tas_kt: Option<f64>,
    pub fuel_flow_us_gal_h: Option<f64>,
    pub nm_per_gal: Option<f64>,
    pub off_chart: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CruiseSweepResult {
    pub rows: Vec<CruiseSweepRow>,
    pub best_tas_row: Option<usize>,
    pub best_nm_per_gal_row: Option<usize>,
}

pub type CruiseSweepError = AltitudeSweepRangeError;

fn row_off_chart(res: &Result<crate::types::CruiseResult, CruiseError>) -> bool {
    matches!(
        res,
        Err(CruiseError::OutOfChart { .. }) | Err(CruiseError::BhpNotAchievable)
    )
}

fn inputs_at_pa(
    base: &FlightInputs,
    pa_start_ft: f64,
    oat_start_c: f64,
    pa_ft: f64,
    lapse: f64,
) -> FlightInputs {
    let mut inputs = base.clone();
    inputs.cruise_pressure_alt_ft = pa_ft;
    inputs.cruise_oat_c = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, pa_ft, lapse);
    inputs
}

pub fn compute_cruise_altitude_sweep(
    data: &PerformanceData,
    base: &FlightInputs,
    params: &CruiseSweepParams,
) -> Result<CruiseSweepResult, CruiseSweepError> {
    let pa_series = build_altitude_series(
        params.from_pa_ft,
        params.to_pa_ft,
        params.step_ft,
        CRUISE_SWEEP_MAX_ROWS,
    )?;
    let pa_start_ft = base.cruise_pressure_alt_ft;
    let oat_start_c = base.cruise_oat_c;
    let lapse = params.lapse_rate_c_per_ft;
    let hold_bhp = base.cruise_solve_mode == CruiseSolveMode::SetPercentBhp;

    let mut rows = Vec::with_capacity(pa_series.len());
    for pa_ft in pa_series {
        let oat_c = oat_at_pa_with_lapse(oat_start_c, pa_start_ft, pa_ft, lapse);
        let isa_dev_c = disa_c(oat_c, pa_ft);
        let inputs = inputs_at_pa(base, pa_start_ft, oat_start_c, pa_ft, lapse);
        let res = compute_cruise(data, &inputs);

        if row_off_chart(&res) {
            rows.push(CruiseSweepRow {
                pa_ft,
                oat_c,
                isa_dev_c,
                density_altitude_ft: None,
                rpm: None,
                percent_bhp: None,
                tas_kt: None,
                fuel_flow_us_gal_h: None,
                nm_per_gal: None,
                off_chart: true,
            });
            continue;
        }

        let Ok(cr) = res else {
            continue;
        };
        let da = match cr.density_altitude_ft {
            Some(d) => Some(d),
            None => Some(isa_troposphere_density_altitude_ft(pa_ft, oat_c)),
        };
        let gph = if cr.mt_gph_out_of_chart {
            None
        } else {
            Some(cr.fuel_flow_us_gal_h)
        };
        let nm_per_gal = gph.filter(|g| *g > 0.0).map(|g| cr.tas_kt / g);

        rows.push(CruiseSweepRow {
            pa_ft,
            oat_c,
            isa_dev_c,
            density_altitude_ft: da,
            rpm: if hold_bhp {
                cr.rpm
            } else {
                Some(base.cruise_rpm)
            },
            percent_bhp: if hold_bhp {
                Some(base.cruise_percent_bhp)
            } else {
                cr.percent_bhp
            },
            tas_kt: Some(cr.tas_kt),
            fuel_flow_us_gal_h: gph,
            nm_per_gal,
            off_chart: false,
        });
    }

    let mut best_tas: Option<(usize, f64)> = None;
    let mut best_eff: Option<(usize, f64)> = None;
    for (i, row) in rows.iter().enumerate() {
        if row.off_chart {
            continue;
        }
        if let Some(tas) = row.tas_kt {
            if best_tas.map(|(_, v)| tas > v).unwrap_or(true) {
                best_tas = Some((i, tas));
            }
        }
        if let Some(nm) = row.nm_per_gal {
            if best_eff.map(|(_, v)| nm > v).unwrap_or(true) {
                best_eff = Some((i, nm));
            }
        }
    }

    Ok(CruiseSweepResult {
        rows,
        best_tas_row: best_tas.map(|(i, _)| i),
        best_nm_per_gal_row: best_eff.map(|(i, _)| i),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cruise::isa_temp_c_at_pa;
    use crate::data::PerformanceData;
    use crate::types::{CruiseSolveMode, PropellerKind};

    #[test]
    fn start_row_matches_cruise_readout_sensenich() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            propeller: PropellerKind::Sensenich,
            cruise_pressure_alt_ft: 6500.0,
            cruise_rpm: 2600.0,
            cruise_oat_c: isa_temp_c_at_pa(6500.0),
            cruise_solve_mode: CruiseSolveMode::SetRpm,
            ..Default::default()
        };
        let readout = compute_cruise(&data, &inputs).unwrap();
        let sweep = compute_cruise_altitude_sweep(
            &data,
            &inputs,
            &CruiseSweepParams {
                from_pa_ft: 6500.0,
                to_pa_ft: 8500.0,
                step_ft: 1000.0,
                ..Default::default()
            },
        )
        .unwrap();
        let row = &sweep.rows[0];
        assert!((row.tas_kt.unwrap() - readout.tas_kt).abs() < 1e-6);
        assert!((row.fuel_flow_us_gal_h.unwrap() - readout.fuel_flow_us_gal_h).abs() < 1e-6);
    }

    #[test]
    fn start_row_matches_cruise_readout_mt() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            propeller: PropellerKind::Mt175R,
            cruise_pressure_alt_ft: 6000.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 65.0,
            cruise_oat_c: isa_temp_c_at_pa(6000.0),
            ..Default::default()
        };
        let readout = compute_cruise(&data, &inputs).unwrap();
        let sweep = compute_cruise_altitude_sweep(
            &data,
            &inputs,
            &CruiseSweepParams {
                from_pa_ft: 6000.0,
                to_pa_ft: 8000.0,
                step_ft: 1000.0,
                ..Default::default()
            },
        )
        .unwrap();
        let row = &sweep.rows[0];
        assert!((row.tas_kt.unwrap() - readout.tas_kt).abs() < 1e-4);
    }

    #[test]
    fn lapse_zero_constant_oat() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_oat_c: 18.0,
            cruise_pressure_alt_ft: 4000.0,
            ..Default::default()
        };
        let sweep = compute_cruise_altitude_sweep(
            &data,
            &inputs,
            &CruiseSweepParams {
                from_pa_ft: 4000.0,
                to_pa_ft: 6000.0,
                step_ft: 1000.0,
                lapse_rate_c_per_ft: 0.0,
            },
        )
        .unwrap();
        for row in &sweep.rows {
            if !row.off_chart {
                assert!((row.oat_c - 18.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn bhp_not_achievable_marked_off_chart() {
        let data = PerformanceData::production();
        let inputs = FlightInputs {
            cruise_pressure_alt_ft: 10_000.0,
            cruise_solve_mode: CruiseSolveMode::SetPercentBhp,
            cruise_percent_bhp: 95.0,
            cruise_oat_c: isa_temp_c_at_pa(10_000.0),
            ..Default::default()
        };
        let sweep = compute_cruise_altitude_sweep(
            &data,
            &inputs,
            &CruiseSweepParams {
                from_pa_ft: 10_000.0,
                to_pa_ft: 12_000.0,
                step_ft: 1000.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(sweep.rows.iter().any(|r| r.off_chart));
    }
}
