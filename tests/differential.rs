// Project: protext-scraper
// File: tests/differential.rs
// Description: Differential corpus captured from the original offline Python implementation.
// Author: Jan Alexandr Kopřiva
// License: MIT
#[path = "support/differential.rs"]
mod differential;
mod support;
#[test]
fn captured_python_reference_cases() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/differential.json")).unwrap();
    assert_eq!(cases.len(), 97);
    for case in cases {
        let expected = match case.get("intentional_rust") {
            Some(value) => {
                assert!(case["reason"].as_str().is_some_and(|s| !s.is_empty()));
                value
            }
            None => &case["python"],
        };
        assert_eq!(
            &differential::evaluate(&case),
            expected,
            "reference case: {}",
            case["name"]
        );
    }
}
