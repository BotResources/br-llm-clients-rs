use std::time::Duration;

use serde_json::json;

use crate::CallReport;

fn load(total: serde_json::Value, first: Option<serde_json::Value>) -> Result<CallReport, String> {
    let mut report = json!({ "total_duration": total });
    if let Some(first) = first {
        report["time_to_first_event"] = first;
    }
    serde_json::from_value::<CallReport>(report).map_err(|error| error.to_string())
}

#[test]
fn given_nanos_below_a_second_when_loaded_then_the_duration_comes_back_as_stored() {
    let report = load(
        json!({"secs": 7, "nanos": 999_999_999}),
        Some(json!({"secs": 0, "nanos": 0})),
    )
    .unwrap();
    assert_eq!(report.total_duration(), Duration::new(7, 999_999_999));
    assert_eq!(report.time_to_first_event(), Some(Duration::ZERO));
}

#[test]
fn given_nanos_of_a_second_or_more_when_loaded_then_refused_in_both_durations() {
    for nanos in [1_000_000_000_u64, 2_000_000_000, 4_294_967_295] {
        assert!(
            load(json!({"secs": 0, "nanos": nanos}), None).is_err(),
            "total {nanos}"
        );
        assert!(
            load(
                json!({"secs": 1, "nanos": 0}),
                Some(json!({"secs": 0, "nanos": nanos}))
            )
            .is_err(),
            "first {nanos}"
        );
    }
}

#[test]
fn given_a_duration_with_a_negative_or_unknown_part_when_loaded_then_refused() {
    for total in [
        json!({"secs": 1, "nanos": -1}),
        json!({"secs": -1, "nanos": 0}),
        json!({"secs": 1, "nanos": 0, "extra": 0}),
        json!({"secs": 1}),
    ] {
        assert!(load(total.clone(), None).is_err(), "{total}");
    }
}
