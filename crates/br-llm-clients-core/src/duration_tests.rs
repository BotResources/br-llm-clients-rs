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

fn load_error(kind: &str, retry_after: serde_json::Value) -> Result<crate::ClientError, String> {
    serde_json::from_value::<crate::ClientError>(json!({"type": kind, "retry_after": retry_after}))
        .map_err(|error| error.to_string())
}

fn load_failure(kind: &str, retry_after: serde_json::Value) -> Result<crate::CallOutcome, String> {
    serde_json::from_value::<crate::CallOutcome>(json!({
        "schema": "br-llm-clients-core/2",
        "report": {"total_duration": {"secs": 0, "nanos": 900_000_000}},
        "result": {
            "status": "failure",
            "error": {"type": kind, "retry_after": retry_after}
        }
    }))
    .map_err(|error| error.to_string())
}

#[test]
fn given_a_retry_after_with_nanos_of_a_second_or_more_when_loaded_then_refused() {
    for kind in ["rate_limited", "unavailable"] {
        for nanos in [1_000_000_000_u64, 1_500_000_000, 4_294_967_295] {
            let stored = json!({"secs": 1, "nanos": nanos});
            assert!(
                load_error(kind, stored.clone()).is_err(),
                "{kind} alone {nanos}"
            );
            assert!(
                load_failure(kind, stored).is_err(),
                "{kind} in a failure {nanos}"
            );
        }
    }
}

#[test]
fn given_a_retry_after_with_nanos_below_a_second_when_loaded_then_it_comes_back_as_stored() {
    let stored = json!({"secs": 1, "nanos": 999_999_999});
    let expected = Duration::new(1, 999_999_999);
    assert_eq!(
        load_error("rate_limited", stored.clone()),
        Ok(crate::ClientError::RateLimited {
            retry_after: Some(expected)
        })
    );
    assert_eq!(
        load_error("unavailable", stored.clone()),
        Ok(crate::ClientError::Unavailable {
            retry_after: Some(expected)
        })
    );
    for (kind, error) in [
        (
            "rate_limited",
            crate::ClientError::RateLimited {
                retry_after: Some(expected),
            },
        ),
        (
            "unavailable",
            crate::ClientError::Unavailable {
                retry_after: Some(expected),
            },
        ),
    ] {
        let outcome = load_failure(kind, stored.clone()).unwrap();
        assert_eq!(
            outcome,
            crate::CallOutcome::failure(CallReport::new(Duration::from_millis(900)), error.clone())
        );
        let again = serde_json::to_value(&outcome).unwrap();
        assert_eq!(again["result"]["error"]["retry_after"], stored);
        assert_eq!(
            serde_json::from_value::<crate::CallOutcome>(again).unwrap(),
            outcome
        );
    }
}
