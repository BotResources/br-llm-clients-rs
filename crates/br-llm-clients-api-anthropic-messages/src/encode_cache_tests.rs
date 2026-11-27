use br_llm_clients_core::{CacheRequirement, ReasoningLevel};
use serde_json::{Value, json};

use crate::test_call::{call_with, hello};
use crate::test_fixtures::*;
use crate::test_server::http;
use crate::test_shapes::every_request_shape;
use crate::test_streams::BASIC;
use crate::{AnthropicMessages, CacheTtl, MessagesOptions};

const LEVELS: [ReasoningLevel; 6] = [
    ReasoningLevel::Off,
    ReasoningLevel::Low,
    ReasoningLevel::Medium,
    ReasoningLevel::High,
    ReasoningLevel::Extra,
    ReasoningLevel::Max,
];

fn with_ttl(level: ReasoningLevel, cache: CacheRequirement, ttl: CacheTtl) -> AnthropicMessages {
    AnthropicMessages::with_options(
        &settings(level, cache, 1024),
        MessagesOptions::new().with_cache_ttl(ttl),
    )
}

fn without_cache_control(mut body: Value) -> Value {
    body.as_object_mut()
        .expect("object")
        .remove("cache_control");
    body
}

#[test]
fn given_the_five_minute_ttl_when_encoded_then_the_bytes_are_those_of_the_default_options() {
    for level in LEVELS {
        for cache in [CacheRequirement::Required, CacheRequirement::NotRequired] {
            let explicit = with_ttl(level, cache, CacheTtl::FiveMinutes);
            for shape in every_request_shape() {
                let by_default = encode(&dialect_with(level, cache), &shape).expect("default");
                let by_ttl = encode(&explicit, &shape).expect("five minutes");
                assert_eq!(by_ttl, by_default, "{level:?} {cache:?}");
            }
        }
    }
}

#[test]
fn given_the_five_minute_ttl_and_required_when_encoded_then_cache_control_is_the_one_of_0_2() {
    let encoded = encode(
        &with_ttl(
            ReasoningLevel::Off,
            CacheRequirement::Required,
            CacheTtl::FiveMinutes,
        ),
        &hello(),
    )
    .expect("encoded");
    let text = String::from_utf8(encoded.body().to_vec()).expect("utf-8");
    assert!(
        text.contains(r#""cache_control":{"type":"ephemeral"}"#),
        "{text}"
    );
    assert!(!text.contains("ttl"), "{text}");
}

#[test]
fn given_the_one_hour_ttl_and_required_when_encoded_then_cache_control_carries_it_and_nothing_else_changes()
 {
    for level in LEVELS {
        let one_hour = with_ttl(level, CacheRequirement::Required, CacheTtl::OneHour);
        let five_minutes = dialect_with(level, CacheRequirement::Required);
        for shape in every_request_shape() {
            let body = body_of(&one_hour, &shape);
            assert_eq!(
                body["cache_control"],
                json!({"type": "ephemeral", "ttl": "1h"}),
                "{level:?}"
            );
            assert_eq!(
                without_cache_control(body),
                without_cache_control(body_of(&five_minutes, &shape)),
                "{level:?}"
            );
        }
    }
}

#[test]
fn given_not_required_when_encoded_with_either_ttl_then_no_cache_control_is_sent() {
    for level in LEVELS {
        let five_minutes = with_ttl(level, CacheRequirement::NotRequired, CacheTtl::FiveMinutes);
        let one_hour = with_ttl(level, CacheRequirement::NotRequired, CacheTtl::OneHour);
        for shape in every_request_shape() {
            let by_one_hour = encode(&one_hour, &shape).expect("one hour");
            assert_eq!(
                by_one_hour,
                encode(&five_minutes, &shape).expect("five minutes")
            );
            let body: Value = serde_json::from_slice(by_one_hour.body()).expect("json");
            assert!(body.get("cache_control").is_none(), "{level:?}");
        }
    }
}

#[test]
fn given_two_ttls_when_set_then_the_second_replaces_the_first() {
    let options = MessagesOptions::new()
        .with_cache_ttl(CacheTtl::OneHour)
        .with_cache_ttl(CacheTtl::FiveMinutes);
    assert_eq!(options, MessagesOptions::new());
    assert_ne!(
        MessagesOptions::new().with_cache_ttl(CacheTtl::OneHour),
        MessagesOptions::new()
    );
}

#[test]
fn given_options_when_debug_is_shown_then_the_cache_lifetime_appears() {
    let default = format!("{:?}", MessagesOptions::new());
    assert!(default.contains("cache_ttl: FiveMinutes"), "{default}");
    let one_hour = format!(
        "{:?}",
        MessagesOptions::new().with_cache_ttl(CacheTtl::OneHour)
    );
    assert!(one_hour.contains("cache_ttl: OneHour"), "{one_hour}");
}

#[tokio::test]
async fn given_the_one_hour_ttl_when_run_then_the_server_gets_it_in_cache_control() {
    let dialect = with_ttl(
        ReasoningLevel::Off,
        CacheRequirement::Required,
        CacheTtl::OneHour,
    );
    let called = call_with(dialect, &hello(), |_| {
        http(
            "200 OK",
            &[("content-type", "text/event-stream")],
            BASIC.as_bytes(),
        )
    })
    .await;
    assert!(called.outcome.result().is_ok());
    let [received] = called.received.as_slice() else {
        panic!("one request expected");
    };
    let sent: Value = serde_json::from_slice(&received.body).expect("json");
    assert_eq!(
        sent["cache_control"],
        json!({"type": "ephemeral", "ttl": "1h"})
    );
}
