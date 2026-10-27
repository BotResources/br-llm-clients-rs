use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{ClientError, LimitKind, ReasoningLevel};
use br_llm_messages::{MediaType, ToolName};

use crate::error_clean::clean_error;
use crate::text_clean::Secrets;

fn secrets() -> Secrets {
    Secrets::new(vec![Secret::new("TOPSECRET").expect("secret")])
}

fn tool() -> Option<ToolName> {
    Some(ToolName::new("TOPSECRET").expect("tool"))
}

fn with_text(text: &str) -> Vec<ClientError> {
    vec![
        ClientError::InvalidMessages {
            reason: text.to_owned(),
        },
        ClientError::ReasoningNotSupported {
            level: ReasoningLevel::High,
            reason: text.to_owned(),
        },
        ClientError::LimitExceeded {
            kind: LimitKind::Tools,
            description: text.to_owned(),
        },
        ClientError::ToolSchemaRejected {
            tool: tool(),
            reason: text.to_owned(),
        },
        ClientError::OutputShapeRejected {
            reason: text.to_owned(),
        },
        ClientError::MalformedResponse {
            detail: text.to_owned(),
        },
        ClientError::ProviderRejected {
            status: Some(400),
            code: Some(text.to_owned()),
            body: text.to_owned(),
        },
        ClientError::ProviderFailed {
            status: Some(500),
            code: Some(text.to_owned()),
            body: text.to_owned(),
        },
    ]
}

#[test]
fn given_a_secret_in_every_text_of_every_case_when_cleaned_then_each_text_is_replaced() {
    let cleaned: Vec<ClientError> = with_text("a TOPSECRET b")
        .into_iter()
        .map(|error| clean_error(error, &secrets()))
        .collect();
    assert_eq!(cleaned, with_text("a [redacted] b"));
}

#[test]
fn given_a_text_over_the_cap_in_every_case_when_cleaned_then_each_text_is_cut() {
    let cleaned: Vec<ClientError> = with_text(&"a".repeat(70_000))
        .into_iter()
        .map(|error| clean_error(error, &secrets()))
        .collect();
    assert_eq!(cleaned, with_text(&"a".repeat(65_536)));
}

#[test]
fn given_a_code_and_a_body_when_cleaned_then_each_one_is_cleaned_on_its_own() {
    let error = ClientError::ProviderFailed {
        status: None,
        code: None,
        body: "TOPSECRET".to_owned(),
    };
    let expected = ClientError::ProviderFailed {
        status: None,
        code: None,
        body: "[redacted]".to_owned(),
    };
    assert_eq!(clean_error(error, &secrets()), expected);
}

#[test]
fn given_typed_fields_equal_to_a_secret_when_cleaned_then_they_are_not_changed() {
    let error = ClientError::ToolSchemaRejected {
        tool: tool(),
        reason: "r".to_owned(),
    };
    assert_eq!(clean_error(error.clone(), &secrets()), error);
    let level = ClientError::ReasoningNotSupported {
        level: ReasoningLevel::Max,
        reason: "r".to_owned(),
    };
    assert_eq!(clean_error(level.clone(), &secrets()), level);
}

#[test]
fn given_cases_without_a_text_when_cleaned_then_they_are_unchanged() {
    let errors = vec![
        ClientError::Authentication,
        ClientError::Transport,
        ClientError::StreamTruncated,
        ClientError::RateLimited {
            retry_after: Some(std::time::Duration::from_secs(3)),
        },
        ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Gif,
        },
    ];
    for error in errors {
        assert_eq!(clean_error(error.clone(), &secrets()), error);
    }
}
