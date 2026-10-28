use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{ClientError, LimitKind, ReasoningLevel};
use br_llm_messages::{MediaType, ResponseEnding, ToolName};

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

fn unusable(ending: ResponseEnding) -> ClientError {
    ClientError::ReplyUnusable { ending }
}

fn unclassified(reason: &str) -> ResponseEnding {
    ResponseEnding::unclassified(reason).expect("ending")
}

#[test]
fn given_a_secret_in_the_reason_of_an_unclassified_ending_when_cleaned_then_it_is_replaced() {
    let cleaned = clean_error(unusable(unclassified("rate TOPSECRET token")), &secrets());
    assert_eq!(cleaned, unusable(unclassified("rate [redacted] token")));
}

#[test]
fn given_a_reason_over_the_cap_when_cleaned_then_the_replacement_comes_before_the_cut() {
    let long = format!("TOPSECRET{}", "a".repeat(70_000));
    let expected = format!("[redacted]{}", "a".repeat(65_526));
    assert_eq!(
        clean_error(unusable(unclassified(&long)), &secrets()),
        unusable(unclassified(&expected))
    );
    let at_the_cut = format!("{}TOPSECRET", "a".repeat(65_533));
    let expected = format!("{}[re", "a".repeat(65_533));
    assert_eq!(
        clean_error(unusable(unclassified(&at_the_cut)), &secrets()),
        unusable(unclassified(&expected))
    );
}

#[test]
fn given_the_endings_without_a_secret_in_a_reply_unusable_when_cleaned_then_they_are_unchanged() {
    let endings = [
        ResponseEnding::finished(),
        ResponseEnding::output_exhaustion(),
        ResponseEnding::stop_sequence(),
        ResponseEnding::matched_stop_sequence("plain sequence").expect("ending"),
        unclassified("plain reason"),
    ];
    for ending in endings {
        let error = unusable(ending);
        assert_eq!(clean_error(error.clone(), &secrets()), error);
    }
}

fn matched(sequence: &str) -> ResponseEnding {
    ResponseEnding::matched_stop_sequence(sequence).expect("ending")
}

#[test]
fn given_a_secret_in_a_matched_stop_sequence_when_cleaned_then_it_is_replaced_and_the_kind_stays() {
    assert_eq!(
        clean_error(unusable(matched("TOPSECRET")), &secrets()),
        unusable(matched("[redacted]"))
    );
    assert_eq!(
        clean_error(unusable(matched("end TOPSECRET end")), &secrets()),
        unusable(matched("end [redacted] end"))
    );
}

#[test]
fn given_a_matched_stop_sequence_over_the_cap_when_cleaned_then_the_replacement_comes_before_the_cut()
 {
    let at_the_cut = format!("{}TOPSECRET", "a".repeat(65_533));
    let expected = format!("{}[re", "a".repeat(65_533));
    assert_eq!(
        clean_error(unusable(matched(&at_the_cut)), &secrets()),
        unusable(matched(&expected))
    );
}
