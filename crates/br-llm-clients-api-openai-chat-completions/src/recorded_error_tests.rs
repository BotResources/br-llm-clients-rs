use br_llm_clients_common_call::ErrorInfo;
use br_llm_clients_core::ClientError;

use crate::Dialect;
use crate::test_recordings::{BAD_BODY, BAD_KEY, BAD_MODEL, Recording, call_recorded};
use crate::test_support::{dialect, hello};

fn info_of(recording: Recording, code: &str) -> ErrorInfo {
    ErrorInfo::new(recording.body)
        .with_status(recording.status())
        .with_code(code)
}

#[test]
fn given_the_recorded_error_bodies_when_read_then_the_code_is_the_code_member_and_never_the_type() {
    let dialect = dialect();
    assert_eq!(
        dialect.error_code(BAD_KEY.body).as_deref(),
        Some("invalid_api_key")
    );
    assert_eq!(
        dialect.error_code(BAD_MODEL.body).as_deref(),
        Some("model_not_found")
    );
    assert_eq!(
        dialect.error_code(BAD_BODY.body).as_deref(),
        Some("empty_array")
    );
}

#[test]
fn given_the_recorded_error_codes_when_refined_then_the_dialect_has_no_opinion() {
    let dialect = dialect();
    for (recording, code) in [
        (BAD_KEY, "invalid_api_key"),
        (BAD_MODEL, "model_not_found"),
        (BAD_BODY, "empty_array"),
    ] {
        assert_eq!(
            dialect.refine_error(&info_of(recording, code)),
            None,
            "{}",
            recording.name
        );
    }
}

#[tokio::test]
async fn given_the_recorded_401_when_called_then_the_failure_is_authentication_with_the_request_id_and_no_text()
 {
    let called = call_recorded(BAD_KEY, dialect(), &hello()).await;
    assert_eq!(called.error(), &ClientError::Authentication);
    assert_eq!(
        called.outcome.report().request_id(),
        Some(BAD_KEY.request_id())
    );
    assert!(!format!("{:?}", called.outcome).contains("sk-inval"));
}

#[tokio::test]
async fn given_the_recorded_404_when_called_then_the_failure_is_a_rejection_with_the_status_the_code_and_the_whole_body()
 {
    let called = call_recorded(BAD_MODEL, dialect(), &hello()).await;
    assert_eq!(
        called.error(),
        &ClientError::ProviderRejected {
            status: Some(404),
            code: Some("model_not_found".to_owned()),
            body: BAD_MODEL.body.to_owned(),
        }
    );
    assert_eq!(
        called.outcome.report().request_id(),
        Some(BAD_MODEL.request_id())
    );
}

#[tokio::test]
async fn given_the_recorded_400_when_called_then_the_failure_is_a_rejection_with_the_status_the_code_and_the_whole_body()
 {
    let called = call_recorded(BAD_BODY, dialect(), &hello()).await;
    assert_eq!(
        called.error(),
        &ClientError::ProviderRejected {
            status: Some(400),
            code: Some("empty_array".to_owned()),
            body: BAD_BODY.body.to_owned(),
        }
    );
    assert_eq!(
        called.outcome.report().request_id(),
        Some(BAD_BODY.request_id())
    );
}
