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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_identical_warnings() {
        let w = CalculationWarning::with_args(
            "CHART_OUT_OF_RANGE",
            vec!["Fig 5.4 PA".into(), "0".into(), "10000".into()],
        );
        let out = dedupe_warnings(vec![w.clone(), w.clone(), w]);
        assert_eq!(out.len(), 1);
    }
}
