use da20_core::data::PerformanceData;
use da20_core::poh_harness::run_worked_examples;

#[test]
fn integration_harness_waits_for_production_examples() {
    let mut data = PerformanceData::synthetic_fixture();
    data.validation.worked_examples.push(da20_core::data::WorkedExample {
        id: "poh-example-1".into(),
        description: Some("TODO".into()),
        inputs: serde_json::json!({}),
        expected: serde_json::json!({}),
        tolerance_key: Some("takeoff_ground_roll_ft".into()),
    });
    let results = run_worked_examples(&data);
    assert_eq!(results.len(), 1);
    assert!(!results[0].passed);
}
