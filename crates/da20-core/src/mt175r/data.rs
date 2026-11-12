use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct Mt175rFile {
    pub max_propeller_speed_rpm: f64,
    figures: Value,
    pub source: Mt175rSourceMeta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Fig4CruiseTable {
    pub rows: Vec<Fig4PrintedRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Fig4PrintedRow {
    pub da_ft: f64,
    pub rpm: f64,
    pub pct_bhp: f64,
    pub ktas: f64,
    pub gph: f64,
}

#[derive(Debug, Clone)]
pub struct CurveSample {
    pub da_ft: f64,
    pub rpm: Option<f64>,
    pub tas_kt: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PowerCurve {
    pub pct_bhp: f64,
    pub samples: Vec<CurveSample>,
    pub lin_a: Option<f64>,
    pub lin_b: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SpeedCurve {
    pub pct_bhp: f64,
    pub samples: Vec<CurveSample>,
    pub quad_coef: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mt175rSourceMeta {
    pub document: String,
    pub propeller: String,
    pub issue: String,
    pub pages: String,
    pub approval_note: String,
}

pub struct Mt175rData {
    inner: Mt175rFile,
    fig4_rows: Vec<Vec<f64>>,
    fig2_curves: std::collections::HashMap<String, PowerCurve>,
    fig3_curves: std::collections::HashMap<String, SpeedCurve>,
}

fn parse_sample(row: &Value) -> Option<CurveSample> {
    Some(CurveSample {
        da_ft: row.get("da_ft")?.as_f64()?,
        rpm: row.get("rpm").and_then(|v| v.as_f64()),
        tas_kt: row.get("tas_kt").and_then(|v| v.as_f64()),
    })
}

fn parse_power_curves(fig: &Value) -> std::collections::HashMap<String, PowerCurve> {
    let mut out = std::collections::HashMap::new();
    let Some(curves) = fig.get("curves").and_then(|v| v.as_object()) else {
        return out;
    };
    for (name, c) in curves {
        let pct = c.get("pct_bhp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let samples: Vec<CurveSample> = c
            .get("samples")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(parse_sample).collect())
            .unwrap_or_default();
        let (lin_a, lin_b) = c
            .get("fit")
            .and_then(|f| {
                let a = f.get("a")?.as_f64()?;
                let b = f.get("b")?.as_f64()?;
                Some((a, b))
            })
            .map(|(a, b)| (Some(a), Some(b)))
            .unwrap_or((None, None));
        out.insert(
            name.clone(),
            PowerCurve {
                pct_bhp: pct,
                samples,
                lin_a,
                lin_b,
            },
        );
    }
    out
}

fn parse_speed_curves(fig: &Value) -> std::collections::HashMap<String, SpeedCurve> {
    let mut out = std::collections::HashMap::new();
    let Some(curves) = fig.get("curves").and_then(|v| v.as_object()) else {
        return out;
    };
    for (name, c) in curves {
        let pct = c.get("pct_bhp").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let samples: Vec<CurveSample> = c
            .get("samples")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(parse_sample).collect())
            .unwrap_or_default();
        let quad_coef = c.get("fit").and_then(|f| {
            let coef = f.get("coef")?.as_array()?;
            if coef.len() >= 3 {
                Some([
                    coef[0].as_f64()?,
                    coef[1].as_f64()?,
                    coef[2].as_f64()?,
                ])
            } else {
                None
            }
        });
        out.insert(
            name.clone(),
            SpeedCurve {
                pct_bhp: pct,
                samples,
                quad_coef,
            },
        );
    }
    out
}

impl Mt175rData {
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        let inner: Mt175rFile = serde_json::from_str(s)?;
        let fig4: Fig4CruiseTable = serde_json::from_value(
            inner
                .figures
                .get("fig4_cruise_table")
                .cloned()
                .unwrap_or(Value::Null),
        )?;
        let fig4_rows = fig4
            .rows
            .iter()
            .map(|r| vec![r.da_ft, r.rpm, r.pct_bhp, r.ktas, r.gph])
            .collect();
        let fig2 = inner.figures.get("fig2_power").cloned().unwrap_or(Value::Null);
        let fig3 = inner
            .figures
            .get("fig3_speed_power")
            .cloned()
            .unwrap_or(Value::Null);
        Ok(Self {
            fig2_curves: parse_power_curves(&fig2),
            fig3_curves: parse_speed_curves(&fig3),
            inner,
            fig4_rows,
        })
    }

    pub fn production() -> Self {
        Self::from_json_str(include_str!("../../../../data/mt175r/mt175r_cruise_data.json"))
            .expect("embedded MT175R supplement data must parse")
    }

    pub fn inner(&self) -> &Mt175rFile {
        &self.inner
    }

    pub fn fig4_rows(&self) -> &[Vec<f64>] {
        &self.fig4_rows
    }

    pub fn fig2_curves(&self) -> &std::collections::HashMap<String, PowerCurve> {
        &self.fig2_curves
    }

    pub fn fig3_curves(&self) -> &std::collections::HashMap<String, SpeedCurve> {
        &self.fig3_curves
    }

    pub fn rpm_max(&self) -> f64 {
        self.inner.max_propeller_speed_rpm
    }

    pub fn provenance(&self) -> crate::types::ProvenanceBlock {
        let s = &self.inner.source;
        crate::types::ProvenanceBlock {
            afm_document_number: s.document.clone(),
            afm_revision: "01".into(),
            afm_date: s.issue.clone(),
            data_status: s.approval_note.clone(),
            citation: crate::types::Citation {
                figure_or_table: "Figure 4 (MT AFMS E-1487)".into(),
                page: Some(7),
                afm_revision: None,
                notes: Some(s.propeller.clone()),
            },
        }
    }
}
