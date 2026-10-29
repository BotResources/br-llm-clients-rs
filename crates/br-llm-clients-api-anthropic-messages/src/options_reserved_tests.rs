use br_llm_clients_core::{CacheRequirement, OutputMode, ReasoningLevel};
use serde_json::{Map, json};

use crate::options_tests::RESERVED;
use crate::test_fixtures::*;
use crate::{AnthropicMessages, MessagesOptions};

const NOT_RESERVED: [&str; 11] = [
    "metadata",
    "temperature",
    "tool_choice",
    "top_p",
    "top_k",
    "stop_sequences",
    "container",
    "fallbacks",
    "service_tier",
    "display",
    "betas",
];

#[test]
fn given_the_members_that_the_dialect_never_writes_when_set_as_extra_body_then_each_is_accepted() {
    for name in NOT_RESERVED {
        let mut members = Map::new();
        members.insert(name.to_owned(), json!(1));
        let options = MessagesOptions::new()
            .with_extra_body(members)
            .unwrap_or_else(|error| panic!("{name} refused: {error}"));
        assert_eq!(
            body_of(&dialect_options(options), &request(vec![user("hello")]))[name],
            json!(1)
        );
    }
}

#[test]
fn given_the_fullest_request_when_encoded_then_every_member_of_the_body_is_reserved_in_the_extra_body()
 {
    let weather = tool("weather", "", vec![]);
    let request = request_with(
        Some("be brief"),
        vec![user("hello")],
        vec![weather],
        OutputMode::Plain,
    );
    let dialect = AnthropicMessages::with_options(
        &settings(ReasoningLevel::High, CacheRequirement::Required, 2048),
        MessagesOptions::new().with_version_placement(vertex()),
    );
    let body = body_of(&dialect, &request);
    let members: Vec<&String> = body.as_object().expect("object").keys().collect();
    assert_eq!(members.len(), RESERVED.len());
    for member in members {
        assert!(RESERVED.contains(&member.as_str()), "{member}");
        let mut extra_body = Map::new();
        extra_body.insert(member.clone(), json!(1));
        assert!(MessagesOptions::new().with_extra_body(extra_body).is_err());
    }
}
