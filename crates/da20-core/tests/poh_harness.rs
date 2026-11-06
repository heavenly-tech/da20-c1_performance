use da20_core::data::PerformanceData;
use da20_core::poh_harness::run_rev29_reference_checks;

#[test]
fn rev29_harness_integration() {
    let data = PerformanceData::production();
    let results = run_rev29_reference_checks(&data);
    assert!(results.len() >= 7);
    assert!(results.iter().all(|r| r.passed));
}
