use std::time::Duration;

use br_llm_clients_core::ClientError;

use crate::classify::classify_refusal;
use crate::error_info::ErrorInfo;
use crate::test_dialect::{Refine, ScriptedDialect};

fn dialect(refine: Refine) -> ScriptedDialect {
    ScriptedDialect {
        refine,
        ..ScriptedDialect::default()
    }
}

fn plain() -> ScriptedDialect {
    ScriptedDialect::default()
}

fn after(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn refusal(status: u16) -> ErrorInfo {
    ErrorInfo::new("body")
        .with_status(status)
        .with_code("c")
        .with_retry_after(after(7))
}

fn rejected(status: u16) -> ClientError {
    ClientError::ProviderRejected {
        status: Some(status),
        code: Some("c".to_owned()),
        body: "body".to_owned(),
    }
}

fn failed(status: u16) -> ClientError {
    ClientError::ProviderFailed {
        status: Some(status),
        code: Some("c".to_owned()),
        body: "body".to_owned(),
    }
}

#[test]
fn given_the_named_statuses_when_classified_then_each_has_its_case() {
    let rows = [
        (401, ClientError::Authentication),
        (403, ClientError::PermissionDenied),
        (
            429,
            ClientError::RateLimited {
                retry_after: Some(after(7)),
            },
        ),
        (
            503,
            ClientError::Unavailable {
                retry_after: Some(after(7)),
            },
        ),
        (
            529,
            ClientError::Unavailable {
                retry_after: Some(after(7)),
            },
        ),
    ];
    for (status, expected) in rows {
        assert_eq!(classify_refusal(&plain(), refusal(status)), expected);
    }
}

#[test]
fn given_the_other_client_statuses_when_classified_then_the_request_is_rejected() {
    for status in [400, 402, 404, 409, 422, 428, 499] {
        assert_eq!(
            classify_refusal(&plain(), refusal(status)),
            rejected(status)
        );
    }
}

#[test]
fn given_any_other_refusal_status_when_classified_then_the_provider_failed() {
    for status in [100, 301, 302, 399, 500, 501, 502, 504, 530, 599, 600] {
        assert_eq!(classify_refusal(&plain(), refusal(status)), failed(status));
    }
}

#[test]
fn given_no_delay_when_classified_then_a_rate_limit_and_an_unavailable_carry_none() {
    let info = ErrorInfo::new("b").with_status(429);
    assert_eq!(
        classify_refusal(&plain(), info),
        ClientError::RateLimited { retry_after: None }
    );
    let info = ErrorInfo::new("b").with_status(503);
    assert_eq!(
        classify_refusal(&plain(), info),
        ClientError::Unavailable { retry_after: None }
    );
}

#[test]
fn given_a_dialect_answer_when_classified_then_it_wins_over_the_generic_table() {
    let dialect = dialect(Box::new(|_| Some(ClientError::ModelNotFound)));
    for status in [401, 404, 429, 500] {
        assert_eq!(
            classify_refusal(&dialect, refusal(status)),
            ClientError::ModelNotFound
        );
    }
}

#[test]
fn given_a_dialect_answer_of_nothing_when_classified_then_the_generic_table_applies() {
    let dialect = dialect(Box::new(|_| None));
    assert_eq!(
        classify_refusal(&dialect, refusal(401)),
        ClientError::Authentication
    );
}

#[test]
fn given_a_dialect_that_reads_the_info_when_classified_then_it_gets_the_info_unchanged() {
    let dialect = plain();
    let info = refusal(404);
    classify_refusal(&dialect, info.clone());
    assert_eq!(dialect.seen_infos(), vec![info]);
}

#[test]
fn given_a_rate_limit_with_no_delay_from_the_dialect_when_classified_then_the_delay_is_set() {
    let dialect = dialect(Box::new(|_| {
        Some(ClientError::RateLimited { retry_after: None })
    }));
    assert_eq!(
        classify_refusal(&dialect, refusal(400)),
        ClientError::RateLimited {
            retry_after: Some(after(7))
        }
    );
    let dialect = self::dialect(Box::new(|_| {
        Some(ClientError::Unavailable { retry_after: None })
    }));
    assert_eq!(
        classify_refusal(&dialect, refusal(400)),
        ClientError::Unavailable {
            retry_after: Some(after(7))
        }
    );
}

#[test]
fn given_a_delay_from_the_dialect_when_classified_then_the_dialect_delay_stays() {
    let dialect = dialect(Box::new(|_| {
        Some(ClientError::RateLimited {
            retry_after: Some(after(1)),
        })
    }));
    assert_eq!(
        classify_refusal(&dialect, refusal(400)),
        ClientError::RateLimited {
            retry_after: Some(after(1))
        }
    );
}

#[test]
fn given_no_delay_in_the_info_when_classified_then_no_delay_is_made_up() {
    let dialect = dialect(Box::new(|_| {
        Some(ClientError::RateLimited { retry_after: None })
    }));
    let info = ErrorInfo::new("b").with_status(400);
    assert_eq!(
        classify_refusal(&dialect, info),
        ClientError::RateLimited { retry_after: None }
    );
}

#[test]
fn given_another_case_from_the_dialect_when_classified_then_no_delay_is_added() {
    let dialect = dialect(Box::new(|_| Some(ClientError::QuotaExceeded)));
    assert_eq!(
        classify_refusal(&dialect, refusal(429)),
        ClientError::QuotaExceeded
    );
}
