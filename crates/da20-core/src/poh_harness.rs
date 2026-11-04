//! POH worked-example regression harness (library module + integration tests).

use crate::data::{PerformanceData, WorkedExample};
use crate::types::FlightInputs;

#[derive(Debug, Clone)]
pub struct HarnessResult {
    pub example_id: String,
    pub passed: bool,
    pub message: String,
}

pub fn run_worked_examples(data: &PerformanceData) -> Vec<HarnessResult> {
    data.validation
        .worked_examples
        .iter()
        .map(|ex| evaluate_example(data, ex))
        .collect()
}

fn evaluate_example(_data: &PerformanceData, ex: &WorkedExample) -> HarnessResult {
    HarnessResult {
        example_id: ex.id.clone(),
        passed: false,
        message: "Awaiting POH worked-example expected values and tolerances in production data."
            .into(),
    }
}

pub fn run_all(_data: &PerformanceData, _inputs: &FlightInputs) -> Vec<HarnessResult> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_empty_for_synthetic() {
        let data = PerformanceData::synthetic_fixture();
        assert!(run_worked_examples(&data).is_empty());
    }
}
