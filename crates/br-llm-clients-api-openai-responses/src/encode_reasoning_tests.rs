use br_llm_clients_core::{ClientError, ReasoningLevel};
use serde_json::json;

use crate::test_requests::{dialect_with, encode_body, hello};
use crate::{Options, ReasoningParameter, ReasoningSummary};

const LEVELS: [(ReasoningLevel, &str); 5] = [
    (ReasoningLevel::Low, "low"),
    (ReasoningLevel::Medium, "medium"),
    (ReasoningLevel::High, "high"),
    (ReasoningLevel::Extra, "xhigh"),
    (ReasoningLevel::Max, "max"),
];

fn body_at(level: ReasoningLevel, options: Options) -> serde_json::Value {
    encode_body(&dialect_with(level, options), &hello()).expect("encoded")
}

fn omitted() -> Options {
    Options {
        reasoning_parameter: ReasoningParameter::Omitted,
        ..Options::default()
    }
}

fn without_summary() -> Options {
    Options {
        reasoning_summary: ReasoningSummary::None,
        ..Options::default()
    }
}

#[test]
fn given_off_when_encoded_then_the_effort_is_none_alone_with_no_summary_and_no_include() {
    let body = body_at(ReasoningLevel::Off, Options::default());
    assert_eq!(body["reasoning"], json!({ "effort": "none" }));
    assert!(body.get("include").is_none());
}

#[test]
fn given_each_level_above_off_when_encoded_then_the_effort_follows_the_table() {
    for (level, effort) in LEVELS {
        let body = body_at(level, Options::default());
        assert_eq!(body["reasoning"]["effort"], json!(effort), "{level:?}");
    }
}

#[test]
fn given_each_level_above_off_when_encoded_then_summary_auto_and_the_encrypted_include_are_sent() {
    for (level, _) in LEVELS {
        let body = body_at(level, Options::default());
        assert_eq!(body["reasoning"]["summary"], json!("auto"), "{level:?}");
        assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
    }
}

#[test]
fn given_each_summary_option_when_encoded_then_the_summary_value_follows_the_table() {
    let table = [
        (ReasoningSummary::Auto, json!("auto")),
        (ReasoningSummary::Concise, json!("concise")),
        (ReasoningSummary::Detailed, json!("detailed")),
    ];
    for (summary, wire) in table {
        for (level, effort) in LEVELS {
            let options = Options {
                reasoning_summary: summary,
                ..Options::default()
            };
            let body = body_at(level, options);
            assert_eq!(
                body["reasoning"],
                json!({ "effort": effort, "summary": wire }),
                "{level:?} {summary:?}"
            );
            assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
        }
    }
}

#[test]
fn given_any_summary_option_and_off_when_encoded_then_the_effort_is_sent_alone() {
    for summary in [
        ReasoningSummary::None,
        ReasoningSummary::Auto,
        ReasoningSummary::Concise,
        ReasoningSummary::Detailed,
    ] {
        let options = Options {
            reasoning_summary: summary,
            ..Options::default()
        };
        let body = body_at(ReasoningLevel::Off, options);
        assert_eq!(
            body["reasoning"],
            json!({ "effort": "none" }),
            "{summary:?}"
        );
        assert!(body.get("include").is_none());
    }
}

#[test]
fn given_no_summary_option_when_encoded_then_the_summary_field_is_absent_but_include_stays() {
    for (level, effort) in LEVELS {
        let body = body_at(level, without_summary());
        assert_eq!(body["reasoning"], json!({ "effort": effort }), "{level:?}");
        assert_eq!(body["include"], json!(["reasoning.encrypted_content"]));
    }
}

#[test]
fn given_no_level_when_encoded_then_minimal_is_never_sent() {
    for level in [ReasoningLevel::Off]
        .into_iter()
        .chain(LEVELS.into_iter().map(|(level, _)| level))
    {
        let body = body_at(level, Options::default());
        assert_ne!(body["reasoning"]["effort"], json!("minimal"));
    }
}

#[test]
fn given_the_parameter_omitted_and_off_when_encoded_then_nothing_about_reasoning_is_sent() {
    let body = body_at(ReasoningLevel::Off, omitted());
    assert!(body.get("reasoning").is_none());
    assert!(body.get("include").is_none());
}

#[test]
fn given_the_parameter_omitted_and_a_level_above_off_when_encoded_then_the_level_is_refused() {
    for (level, _) in LEVELS {
        let refusal = encode_body(&dialect_with(level, omitted()), &hello()).expect_err("refused");
        assert!(
            matches!(&refusal, ClientError::ReasoningNotSupported { level: named, .. } if *named == level),
            "{level:?}"
        );
    }
}

#[test]
fn given_the_parameter_omitted_and_every_option_when_encoded_then_the_body_never_holds_reasoning_or_include()
 {
    let options = Options {
        reasoning_summary: ReasoningSummary::Auto,
        ..omitted()
    };
    let body = body_at(ReasoningLevel::Off, options);
    assert!(body.get("reasoning").is_none() && body.get("include").is_none());
}

#[test]
fn given_a_refused_level_when_encoded_twice_then_the_refusal_comes_on_every_call() {
    let dialect = dialect_with(ReasoningLevel::High, omitted());
    for _ in 0..2 {
        assert!(matches!(
            crate::test_requests::encoded(&dialect, &hello()),
            Err(ClientError::ReasoningNotSupported { .. })
        ));
    }
}

#[test]
fn given_any_level_and_any_token_limit_when_encoded_then_the_pair_is_never_refused() {
    use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens};
    use br_llm_messages::ModelName;
    for level in [ReasoningLevel::Off, ReasoningLevel::Max] {
        for tokens in [1, 100, u32::MAX] {
            let settings = ClientSettings::new(
                ModelName::new("m").expect("model"),
                level,
                CacheRequirement::NotRequired,
                MaxOutputTokens::new(tokens).expect("tokens"),
            );
            let dialect = crate::OpenAiResponses::new(&settings, Options::default());
            let encoded = crate::test_requests::encoded(&dialect, &hello()).expect("never refused");
            let body: serde_json::Value = serde_json::from_slice(encoded.body()).expect("json");
            assert_eq!(body["max_output_tokens"], json!(tokens));
        }
    }
}
