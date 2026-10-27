use std::time::Duration;

use serde_json::{Value, json};

use crate::call_outcome_stored_tests::rich_success;
use crate::test_support::{text_response, usage};
use crate::{CallOutcome, CallReport, ClientError};

fn report() -> CallReport {
    CallReport::new(Duration::from_millis(900))
}

fn failure() -> CallOutcome {
    CallOutcome::failure(report(), ClientError::Transport)
}

fn refusal_text(json: &str) -> String {
    serde_json::from_str::<CallOutcome>(json)
        .unwrap_err()
        .to_string()
}

#[test]
fn given_a_document_of_the_previous_schema_when_loaded_then_refused_naming_the_schema_found() {
    let previous = r#"{"schema":"br-llm-clients-core/1","report":{"total_duration":{"secs":1,"nanos":0}},"result":{"status":"success","step":{"content":[],"stop_reason":{"type":"end_turn"}}}}"#;
    assert!(refusal_text(previous).contains("br-llm-clients-core/1"));
    let as_value: Value = serde_json::from_str(previous).unwrap();
    let error = serde_json::from_value::<CallOutcome>(as_value).unwrap_err();
    assert!(error.to_string().contains("br-llm-clients-core/1"));
}

#[test]
fn given_the_current_shape_under_another_schema_when_loaded_then_refused_naming_the_schema() {
    for schema in [
        "br-llm-clients-core/1",
        "br-llm-clients-core/3",
        "other/2",
        "",
    ] {
        let mut json = serde_json::to_value(failure()).unwrap();
        json["schema"] = json!(schema);
        let error = serde_json::from_value::<CallOutcome>(json).unwrap_err();
        assert!(
            error.to_string().contains(&format!("{schema:?}")),
            "{schema}"
        );
    }
}

#[test]
fn given_a_stored_outcome_missing_a_part_when_loaded_then_refused() {
    for missing in ["schema", "report", "result"] {
        let mut json = serde_json::to_value(failure()).unwrap();
        json.as_object_mut().unwrap().remove(missing);
        assert!(
            serde_json::from_value::<CallOutcome>(json).is_err(),
            "{missing}"
        );
    }
}

#[test]
fn given_a_stored_outcome_with_an_unknown_key_at_any_level_when_loaded_then_refused() {
    let mut top = serde_json::to_value(failure()).unwrap();
    top["host"] = json!("example.org");
    assert!(serde_json::from_value::<CallOutcome>(top).is_err());

    let mut in_report = serde_json::to_value(failure()).unwrap();
    in_report["report"]["host"] = json!("example.org");
    assert!(serde_json::from_value::<CallOutcome>(in_report).is_err());

    let mut in_result = serde_json::to_value(failure()).unwrap();
    in_result["result"]["host"] = json!("example.org");
    assert!(serde_json::from_value::<CallOutcome>(in_result).is_err());

    let mut in_usage = serde_json::to_value(CallOutcome::failure(
        report().with_usage(usage(1, 1)),
        ClientError::Transport,
    ))
    .unwrap();
    in_usage["report"]["usage"]["cost"] = json!(3);
    assert!(serde_json::from_value::<CallOutcome>(in_usage).is_err());
}

#[test]
fn given_a_result_with_the_wrong_parts_for_its_status_when_loaded_then_refused() {
    let response = serde_json::to_value(text_response("hi", None)).unwrap();
    let error = json!({ "type": "transport" });
    let cases = [
        json!({ "status": "success" }),
        json!({ "status": "success", "error": error }),
        json!({ "status": "success", "response": response, "error": error }),
        json!({ "status": "failure" }),
        json!({ "status": "failure", "response": response }),
        json!({ "status": "failure", "response": response, "error": error }),
        json!({ "response": response }),
        json!({ "status": "partial", "response": response }),
        json!({ "status": "failure", "error": error, "step": {} }),
    ];
    for result in cases {
        let mut json = serde_json::to_value(failure()).unwrap();
        json["result"] = result.clone();
        assert!(
            serde_json::from_value::<CallOutcome>(json).is_err(),
            "{result}"
        );
    }
}

#[test]
fn given_a_stored_success_whose_usages_differ_when_loaded_then_refused() {
    let response = text_response("hi", Some(usage(10, 5)));
    let outcome = CallOutcome::success(report().with_usage(usage(10, 5)), response).unwrap();
    let mut json = serde_json::to_value(&outcome).unwrap();
    json["report"]["usage"]["output_tokens"] = json!(6);
    assert!(serde_json::from_value::<CallOutcome>(json.clone()).is_err());
    json["report"].as_object_mut().unwrap().remove("usage");
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_response_breaking_a_messages_rule_when_loaded_then_refused() {
    let mut json =
        serde_json::to_value(CallOutcome::success(report(), text_response("hi", None)).unwrap())
            .unwrap();
    assert!(serde_json::from_value::<CallOutcome>(json.clone()).is_ok());
    json["result"]["response"]["ending"] = json!({ "type": "tool_use" });
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_tool_call_whose_name_br_llm_messages_refuses_when_loaded_then_refused() {
    let mut json = serde_json::to_value(rich_success()).unwrap();
    let blocks = json["result"]["response"]["content"]
        .as_array_mut()
        .unwrap();
    let call = blocks.last_mut().unwrap();
    assert_eq!(call["type"], json!("runtime_tool_call"));
    call["name"] = json!("fs.read");
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_error_case_unknown_to_this_version_when_loaded_then_refused() {
    let mut json = serde_json::to_value(failure()).unwrap();
    json["result"]["error"] = json!({ "type": "teapot" });
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_report_without_total_duration_when_loaded_then_refused() {
    let mut json = serde_json::to_value(failure()).unwrap();
    json["report"] = json!({});
    assert!(serde_json::from_value::<CallOutcome>(json).is_err());
}

#[test]
fn given_a_stored_report_with_a_null_or_empty_item_when_loaded_then_refused() {
    for item in [
        ("usage", json!(null)),
        ("usage", json!({})),
        ("model", json!(null)),
        ("model", json!("")),
        ("request_id", json!(null)),
        ("time_to_first_event", json!(null)),
    ] {
        let mut json = serde_json::to_value(report()).unwrap();
        json[item.0] = item.1.clone();
        assert!(
            serde_json::from_value::<CallReport>(json).is_err(),
            "{item:?}"
        );
    }
}

#[test]
fn given_a_key_given_twice_at_any_level_when_loaded_then_refused() {
    let response = serde_json::to_string(&text_response("hi", None)).unwrap();
    let report = r#"{"total_duration":{"secs":1,"nanos":0}}"#;
    let documents = [
        format!(
            r#"{{"schema":"br-llm-clients-core/2","report":{report},"result":{{"status":"success","status":"failure","error":{{"type":"transport"}}}}}}"#
        ),
        format!(
            r#"{{"schema":"br-llm-clients-core/2","report":{report},"result":{{"status":"success","response":{response},"response":{response}}}}}"#
        ),
        format!(
            r#"{{"schema":"br-llm-clients-core/2","report":{report},"result":{{"status":"failure","error":{{"type":"transport"}},"error":{{"type":"transport"}}}}}}"#
        ),
        format!(
            r#"{{"schema":"br-llm-clients-core/2","schema":"br-llm-clients-core/2","report":{report},"result":{{"status":"failure","error":{{"type":"transport"}}}}}}"#
        ),
        format!(
            r#"{{"schema":"br-llm-clients-core/2","report":{report},"report":{report},"result":{{"status":"failure","error":{{"type":"transport"}}}}}}"#
        ),
    ];
    for document in documents {
        assert!(
            serde_json::from_str::<CallOutcome>(&document).is_err(),
            "{document}"
        );
    }
}
