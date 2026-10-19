use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::{ClientError, ReasoningLevel};
use serde_json::Value;

use crate::test_recorded::{run_plain, run_with_secrets};
use crate::test_recordings::{BAD_BODY, BAD_KEY, BAD_MODEL, Recording};
use crate::test_requests::dialect;

fn subject() -> impl Dialect {
    dialect(ReasoningLevel::Off)
}

fn error_object(recording: &Recording) -> Value {
    serde_json::from_str::<Value>(recording.body).expect("a JSON body")["error"].clone()
}

#[test]
fn given_the_recorded_refusal_bodies_when_the_code_is_read_then_the_code_wins_over_the_type() {
    let subject = subject();
    for (recording, code) in [
        (&BAD_KEY, "invalid_api_key"),
        (&BAD_MODEL, "model_not_found"),
        (&BAD_BODY, "missing_required_parameter"),
    ] {
        assert_eq!(error_object(recording)["type"], "invalid_request_error");
        assert_eq!(error_object(recording)["code"], code);
        assert_eq!(subject.error_code(recording.body).as_deref(), Some(code));
    }
}

#[test]
fn given_the_recorded_refusal_bodies_when_refined_then_there_is_no_row_and_the_generic_table_decides()
 {
    let subject = subject();
    for recording in [&BAD_KEY, &BAD_MODEL, &BAD_BODY] {
        let code = subject.error_code(recording.body).expect("a code");
        let info = ErrorInfo::new(recording.body)
            .with_status(recording.status)
            .with_code(code);
        assert_eq!(subject.refine_error(&info), None, "{}", recording.name);
    }
}

#[tokio::test]
async fn given_the_recorded_401_when_run_then_it_is_authentication_with_the_request_id_and_no_body()
{
    let called = run_plain(&BAD_KEY).await;
    let report = called.outcome.report().clone();
    let (_, result) = called.outcome.into_parts();
    assert_eq!(result.expect_err("a failure"), ClientError::Authentication);
    assert_eq!(report.request_id(), Some(BAD_KEY.request_id));
    assert_eq!(report.usage(), None);
    assert_eq!(report.model(), None);
}

#[tokio::test]
async fn given_the_recorded_401_when_run_then_nothing_of_the_masked_key_reaches_the_outcome() {
    let outcome = run_plain(&BAD_KEY).await.outcome;
    let shown = format!("{outcome:?}");
    assert!(!shown.contains("sk-"));
    assert!(!shown.contains("inval"));
}

#[tokio::test]
async fn given_the_recorded_404_when_run_then_it_is_provider_rejected_with_the_code_and_the_body_as_received()
 {
    let (_, result) = run_plain(&BAD_MODEL).await.outcome.into_parts();
    assert_eq!(
        result.expect_err("a failure"),
        ClientError::ProviderRejected {
            status: Some(404),
            code: Some("model_not_found".to_owned()),
            body: BAD_MODEL.body.to_owned(),
        }
    );
}

#[tokio::test]
async fn given_the_recorded_400_when_run_then_it_is_provider_rejected_with_the_code_and_the_body_as_received()
 {
    let called = run_plain(&BAD_BODY).await;
    let report = called.outcome.report().clone();
    let (_, result) = called.outcome.into_parts();
    assert_eq!(
        result.expect_err("a failure"),
        ClientError::ProviderRejected {
            status: Some(400),
            code: Some("missing_required_parameter".to_owned()),
            body: BAD_BODY.body.to_owned(),
        }
    );
    assert_eq!(report.request_id(), Some(BAD_BODY.request_id));
}

#[tokio::test]
async fn given_a_secret_inside_the_recorded_refusal_when_run_then_it_is_redacted_in_the_body_and_nowhere_else()
 {
    let outcome = run_with_secrets(&BAD_MODEL, &["gpt-does-not-exist"]).await;
    let (report, result) = outcome.into_parts();
    let ClientError::ProviderRejected { status, code, body } = result.expect_err("a failure")
    else {
        panic!("expected a rejection");
    };
    assert_eq!(status, Some(404));
    assert_eq!(code.as_deref(), Some("model_not_found"));
    assert!(body.contains("The model `[redacted]` does not exist"));
    assert!(!body.contains("gpt-does-not-exist"));
    assert_eq!(report.request_id(), Some(BAD_MODEL.request_id));
}

#[tokio::test]
async fn given_a_secret_that_is_the_code_of_the_recorded_refusal_when_run_then_the_code_is_redacted_too()
 {
    let outcome = run_with_secrets(&BAD_BODY, &["missing_required_parameter"]).await;
    let (_, result) = outcome.into_parts();
    let ClientError::ProviderRejected { code, body, .. } = result.expect_err("a failure") else {
        panic!("expected a rejection");
    };
    assert_eq!(code.as_deref(), Some("[redacted]"));
    assert!(!body.contains("missing_required_parameter"));
}
