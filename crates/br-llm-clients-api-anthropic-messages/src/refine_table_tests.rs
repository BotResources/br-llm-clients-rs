use br_llm_clients_common_call::{Dialect, ErrorInfo};
use br_llm_clients_core::{ClientError, LimitKind};

use crate::test_fixtures::dialect;

const STATUSES: [Option<u16>; 6] = [None, Some(400), Some(401), Some(404), Some(500), Some(529)];

fn info(code: Option<&str>, status: Option<u16>) -> ErrorInfo {
    let mut info = ErrorInfo::new(format!("text of {code:?} {status:?}"));
    if let Some(code) = code {
        info = info.with_code(code);
    }
    if let Some(status) = status {
        info = info.with_status(status);
    }
    info
}

fn refined(code: Option<&str>, status: Option<u16>) -> Option<ClientError> {
    dialect().refine_error(&info(code, status))
}

#[test]
fn given_a_code_row_when_refined_with_any_or_no_status_then_the_case_follows_the_table() {
    for status in STATUSES {
        assert_eq!(
            refined(Some("overloaded_error"), status),
            Some(ClientError::Unavailable { retry_after: None })
        );
        assert_eq!(
            refined(Some("rate_limit_error"), status),
            Some(ClientError::RateLimited { retry_after: None })
        );
        assert_eq!(
            refined(Some("billing_error"), status),
            Some(ClientError::QuotaExceeded)
        );
        assert_eq!(
            refined(Some("authentication_error"), status),
            Some(ClientError::Authentication)
        );
        assert_eq!(
            refined(Some("permission_error"), status),
            Some(ClientError::PermissionDenied)
        );
    }
}

#[test]
fn given_request_too_large_when_refined_then_it_is_a_limit_of_kind_other_with_the_text() {
    for status in STATUSES {
        let info = info(Some("request_too_large"), status);
        assert_eq!(
            dialect().refine_error(&info),
            Some(ClientError::LimitExceeded {
                kind: LimitKind::Other,
                description: info.text().to_owned(),
            })
        );
    }
}

#[test]
fn given_no_code_and_status_402_when_refined_then_it_is_quota_exceeded() {
    assert_eq!(refined(None, Some(402)), Some(ClientError::QuotaExceeded));
}

#[test]
fn given_no_code_and_status_413_when_refined_then_it_is_a_limit_of_kind_other_with_the_text() {
    let info = info(None, Some(413));
    assert_eq!(
        dialect().refine_error(&info),
        Some(ClientError::LimitExceeded {
            kind: LimitKind::Other,
            description: info.text().to_owned(),
        })
    );
}

#[test]
fn given_the_402_and_413_rows_when_refined_then_they_are_not_swapped() {
    assert_ne!(refined(None, Some(402)), refined(None, Some(413)));
    assert!(matches!(
        refined(None, Some(402)),
        Some(ClientError::QuotaExceeded)
    ));
    assert!(matches!(
        refined(None, Some(413)),
        Some(ClientError::LimitExceeded { .. })
    ));
}

#[test]
fn given_a_status_row_with_another_code_when_refined_then_nothing_is_decided() {
    assert_eq!(refined(Some("invalid_request_error"), Some(402)), None);
    assert_eq!(refined(Some("api_error"), Some(413)), None);
}

#[test]
fn given_codes_and_statuses_outside_the_table_when_refined_then_the_generic_table_applies() {
    for code in [
        "api_error",
        "not_found_error",
        "timeout_error",
        "conflict_error",
        "invalid_request_error",
        "brand_new_error",
    ] {
        for status in STATUSES {
            assert_eq!(refined(Some(code), status), None, "{code} {status:?}");
        }
    }
    for status in [
        None,
        Some(400),
        Some(401),
        Some(403),
        Some(404),
        Some(429),
        Some(500),
        Some(503),
        Some(529),
    ] {
        assert_eq!(refined(None, status), None, "{status:?}");
    }
}

#[test]
fn given_a_code_with_a_different_case_when_refined_then_it_is_compared_as_it_is() {
    assert_eq!(refined(Some("Overloaded_Error"), None), None);
}
