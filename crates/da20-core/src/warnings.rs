use crate::types::CalculationWarning;

/// One warning per unique `(code, args)` per computation pass.
pub fn dedupe_warnings(warnings: Vec<CalculationWarning>) -> Vec<CalculationWarning> {
    let mut out = Vec::with_capacity(warnings.len());
    for w in warnings {
        if !out
            .iter()
            .any(|o: &CalculationWarning| o.code == w.code && o.args == w.args)
        {
            out.push(w);
        }
    }
    out
}

pub fn takeoff_off_chart(warnings: &[CalculationWarning]) -> bool {
    warnings
        .iter()
        .any(|w| matches!(w.code.as_str(), "CHART_CLAMP_LOW" | "CHART_CLAMP_HIGH"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_identical_warnings() {
        let w = CalculationWarning::with_args(
            "CHART_CLAMP_LOW",
            vec!["panel".into(), "206".into(), "300".into()],
        );
        let out = dedupe_warnings(vec![w.clone(), w.clone(), w]);
        assert_eq!(out.len(), 1);
    }
}
