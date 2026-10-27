use serde_json::{Value, json};

use crate::{CallOutcome, ClientSettings};

const REPORT: &str = r#""report":{"total_duration":{"secs":1,"nanos":0}}"#;

fn outcome_documents(schema: &str, result: &str) -> [String; 2] {
    [
        format!(r#"{{"schema":"{schema}",{REPORT},"result":{result}}}"#),
        format!(r#"{{{REPORT},"result":{result},"schema":"{schema}"}}"#),
    ]
}

fn settings_documents(schema: &str, fields: &str) -> [String; 2] {
    [
        format!(r#"{{"schema":"{schema}",{fields}}}"#),
        format!(r#"{{{fields},"schema":"{schema}"}}"#),
    ]
}

const GOOD_SETTINGS: &str =
    r#""model":"model-x","reasoning":"off","cache":"not_required","max_output_tokens":4096"#;

fn assert_outcome_refused_naming(document: &str, schema: &str) {
    let named = format!("{schema:?}");
    let text = serde_json::from_str::<CallOutcome>(document)
        .unwrap_err()
        .to_string();
    assert!(text.contains(&named), "{document} -> {text}");
    let value: Value = serde_json::from_str(document).unwrap();
    let text = serde_json::from_value::<CallOutcome>(value)
        .unwrap_err()
        .to_string();
    assert!(text.contains(&named), "{document} -> {text}");
}

fn assert_settings_refused_naming(document: &str, schema: &str) {
    let named = format!("{schema:?}");
    let text = serde_json::from_str::<ClientSettings>(document)
        .unwrap_err()
        .to_string();
    assert!(text.contains(&named), "{document} -> {text}");
    let value: Value = serde_json::from_str(document).unwrap();
    let text = serde_json::from_value::<ClientSettings>(value)
        .unwrap_err()
        .to_string();
    assert!(text.contains(&named), "{document} -> {text}");
}

#[test]
fn given_a_previous_schema_outcome_with_a_value_of_the_old_shape_when_loaded_then_the_schema_is_named()
 {
    let old_shapes = [
        r#"{"status":"failure","error":{"type":"reply_unusable","stop_reason":{"type":"end_turn"}}}"#,
        r#"{"status":"failure","error":{"type":"image_media_type_not_supported","media_type":"png"}}"#,
        r#"{"status":"success","step":{"content":[],"stop_reason":{"type":"end_turn"}}}"#,
    ];
    for result in old_shapes {
        for document in outcome_documents("br-llm-clients-core/1", result) {
            assert_outcome_refused_naming(&document, "br-llm-clients-core/1");
        }
    }
}

#[test]
fn given_a_later_schema_outcome_with_an_invalid_value_when_loaded_then_the_schema_is_named() {
    let invalid = [
        r#"{"status":"weird"}"#,
        r#"{"status":"failure","error":{"type":"transport"},"extra":1}"#,
        r#"{"status":"failure","error":{"type":"case_from_the_future"}}"#,
    ];
    for result in invalid {
        for document in outcome_documents("br-llm-clients-core/3", result) {
            assert_outcome_refused_naming(&document, "br-llm-clients-core/3");
        }
    }
}

#[test]
fn given_a_wrong_schema_outcome_with_a_broken_report_when_loaded_then_the_schema_is_named() {
    for document in [
        r#"{"schema":"other/2","report":{"total_duration":"slow"},"result":{"status":"failure","error":{"type":"transport"}}}"#,
        r#"{"report":{"total_duration":"slow"},"result":{"status":"failure","error":{"type":"transport"}},"schema":"other/2"}"#,
    ] {
        assert_outcome_refused_naming(document, "other/2");
    }
}

#[test]
fn given_later_schema_settings_with_an_invalid_value_or_key_when_loaded_then_the_schema_is_named() {
    let invalid = [
        r#""model":"model-x","reasoning":"ultra","cache":"not_required","max_output_tokens":4096"#,
        r#""model":"model-x","reasoning":"off","cache":"not_required","max_output_tokens":4096,"extra":true"#,
        r#""model":"model-x","reasoning":"off","cache":"not_required","max_output_tokens":0"#,
        r#""model":7"#,
    ];
    for fields in invalid {
        for document in settings_documents("br-llm-clients-core/3", fields) {
            assert_settings_refused_naming(&document, "br-llm-clients-core/3");
        }
    }
}

#[test]
fn given_previous_schema_settings_with_an_invalid_value_when_loaded_then_the_schema_is_named() {
    let invalid = [
        r#""model":"model-x","reasoning":"off","cache":"not_required","max_output_tokens":4096,"endpoint":"x""#,
        r#""model":"model-x","reasoning":"max","cache":"sometimes","max_output_tokens":4096"#,
    ];
    for fields in invalid {
        for document in settings_documents("br-llm-clients-core/1", fields) {
            assert_settings_refused_naming(&document, "br-llm-clients-core/1");
        }
    }
}

#[test]
fn given_a_missing_null_numeric_or_repeated_schema_when_loaded_then_refused_about_the_schema() {
    let outcome_tail =
        format!(r#"{REPORT},"result":{{"status":"failure","error":{{"type":"transport"}}}}"#);
    let documents = [
        format!("{{{outcome_tail}}}"),
        format!(r#"{{"schema":null,{outcome_tail}}}"#),
        format!(r#"{{"schema":2,{outcome_tail}}}"#),
        format!(
            r#"{{"schema":"br-llm-clients-core/2","schema":"br-llm-clients-core/2",{outcome_tail}}}"#
        ),
        format!(r#"{{{outcome_tail},"schema":null}}"#),
    ];
    for document in documents {
        let text = serde_json::from_str::<CallOutcome>(&document)
            .unwrap_err()
            .to_string();
        assert!(text.contains("schema"), "{document} -> {text}");
    }
    let settings = [
        format!("{{{GOOD_SETTINGS}}}"),
        format!(r#"{{"schema":null,{GOOD_SETTINGS}}}"#),
        format!(r#"{{"schema":2,{GOOD_SETTINGS}}}"#),
        format!(
            r#"{{"schema":"br-llm-clients-core/2","schema":"br-llm-clients-core/2",{GOOD_SETTINGS}}}"#
        ),
    ];
    for document in settings {
        let text = serde_json::from_str::<ClientSettings>(&document)
            .unwrap_err()
            .to_string();
        assert!(text.contains("schema"), "{document} -> {text}");
    }
}

#[test]
fn given_a_current_schema_document_with_keys_in_any_order_when_loaded_then_it_loads() {
    for document in settings_documents("br-llm-clients-core/2", GOOD_SETTINGS) {
        assert!(serde_json::from_str::<ClientSettings>(&document).is_ok());
    }
    for document in outcome_documents(
        "br-llm-clients-core/2",
        r#"{"status":"failure","error":{"type":"transport"}}"#,
    ) {
        assert!(serde_json::from_str::<CallOutcome>(&document).is_ok());
    }
}

#[test]
fn given_a_current_schema_document_that_is_not_an_object_when_loaded_then_refused() {
    for value in [
        json!(null),
        json!([]),
        json!("br-llm-clients-core/2"),
        json!(2),
    ] {
        assert!(serde_json::from_value::<CallOutcome>(value.clone()).is_err());
        assert!(serde_json::from_value::<ClientSettings>(value).is_err());
    }
}

#[test]
fn given_a_key_given_twice_deep_in_a_current_document_when_loaded_then_refused() {
    let documents = [
        r#"{"schema":"br-llm-clients-core/2","report":{"total_duration":{"secs":1,"nanos":0},"total_duration":{"secs":1,"nanos":0}},"result":{"status":"failure","error":{"type":"transport"}}}"#,
        r#"{"report":{"total_duration":{"secs":1,"nanos":0}},"result":{"status":"failure","error":{"type":"transport","type":"transport"}},"schema":"br-llm-clients-core/2"}"#,
        r#"{"report":{"total_duration":{"secs":1,"secs":1,"nanos":0}},"result":{"status":"failure","error":{"type":"transport"}},"schema":"br-llm-clients-core/2"}"#,
    ];
    for document in documents {
        assert!(
            serde_json::from_str::<CallOutcome>(document).is_err(),
            "{document}"
        );
    }
    let settings = r#"{"model":"a","model":"a","reasoning":"off","cache":"not_required","max_output_tokens":4096,"schema":"br-llm-clients-core/2"}"#;
    assert!(serde_json::from_str::<ClientSettings>(settings).is_err());
}
