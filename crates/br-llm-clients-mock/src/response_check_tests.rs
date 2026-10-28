use br_llm_clients_core::ClientError;
use br_llm_messages::ResponseEnding;

use crate::response_support::*;
use crate::test_support::*;

fn is_malformed(result: Result<br_llm_messages::ModelResponse, ClientError>) -> bool {
    matches!(result, Err(ClientError::MalformedResponse { .. }))
}

#[test]
fn given_a_response_without_content_when_it_ends_with_filtering_or_refusal_then_content_blocked() {
    for ending in [ResponseEnding::filtering(), ResponseEnding::refusal()] {
        let response = response_ending(vec![thinking_content()], ending);
        assert_eq!(
            serve(&plain_request(), response).unwrap_err(),
            ClientError::ContentBlocked
        );
    }
}

#[test]
fn given_a_response_without_content_when_it_ends_otherwise_then_reply_unusable_carries_the_ending()
{
    for ending in [
        ResponseEnding::finished(),
        ResponseEnding::output_exhaustion(),
        ResponseEnding::stop_sequence(),
    ] {
        let response = response_ending(vec![thinking_content()], ending.clone());
        assert_eq!(
            serve(&plain_request(), response).unwrap_err(),
            ClientError::ReplyUnusable { ending }
        );
    }
}

#[test]
fn given_a_response_with_only_an_image_when_served_then_the_image_is_not_content() {
    let blocked = response_ending(vec![image_content()], ResponseEnding::refusal());
    assert_eq!(
        serve(&plain_request(), blocked).unwrap_err(),
        ClientError::ContentBlocked
    );
    let unusable = response_ending(vec![image_content()], ResponseEnding::finished());
    assert!(matches!(
        serve(&plain_request(), unusable),
        Err(ClientError::ReplyUnusable { .. })
    ));
}

#[test]
fn given_a_response_with_only_a_block_of_a_provider_tool_when_served_then_it_counts_as_content() {
    for ending in [ResponseEnding::finished(), ResponseEnding::refusal()] {
        let response = response_ending(vec![provider_content()], ending);
        assert!(serve(&plain_request(), response).is_ok());
    }
}

#[test]
fn given_a_plain_request_when_the_response_holds_a_structured_block_then_malformed() {
    let response = response_ending(
        vec![structured_content(r#"{"a":1}"#)],
        ResponseEnding::finished(),
    );
    assert!(is_malformed(serve(&plain_request(), response)));
    let mixed = response_ending(
        vec![text_content("t"), structured_content("1")],
        ResponseEnding::finished(),
    );
    assert!(is_malformed(serve(&plain_request(), mixed)));
}

#[test]
fn given_a_plain_request_when_the_response_holds_text_or_calls_then_it_is_given() {
    let text = response_ending(vec![text_content("t")], ResponseEnding::finished());
    assert!(serve(&plain_request(), text).is_ok());
    let calls = response_ending(
        vec![call_content("search", "{}")],
        ResponseEnding::tool_use(),
    );
    assert!(serve(&tools_request(), calls).is_ok());
}

#[test]
fn given_a_structured_request_when_the_response_holds_one_structured_block_then_it_is_given() {
    let response = response_ending(
        vec![thinking_content(), structured_content(r#"{"a":1}"#)],
        ResponseEnding::finished(),
    );
    assert!(serve(&structured_request(), response).is_ok());
}

#[test]
fn given_a_structured_request_when_the_response_breaks_the_shape_then_malformed() {
    let cases = [
        vec![text_content("only text")],
        vec![structured_content("1"), text_content("t")],
        vec![structured_content("1"), structured_content("2")],
        vec![structured_content("1"), call_content("search", "{}")],
        vec![call_content("search", "{}")],
        vec![provider_content()],
    ];
    for blocks in cases {
        let response = response_ending(blocks, ResponseEnding::finished());
        assert!(is_malformed(serve(&structured_request(), response)));
    }
}

#[test]
fn given_a_structured_request_when_there_is_no_structured_block_and_no_call_and_the_ending_blocks_then_content_blocked_whatever_the_text()
 {
    for ending in [ResponseEnding::filtering(), ResponseEnding::refusal()] {
        let response = response_ending(vec![text_content("sorry")], ending);
        assert_eq!(
            serve(&structured_request(), response).unwrap_err(),
            ClientError::ContentBlocked
        );
    }
}

#[test]
fn given_a_structured_request_when_the_response_holds_no_content_then_the_missing_content_comes_before_the_shape()
 {
    let unusable = response_ending(vec![thinking_content()], ResponseEnding::finished());
    assert_eq!(
        serve(&structured_request(), unusable).unwrap_err(),
        ClientError::ReplyUnusable {
            ending: ResponseEnding::finished()
        }
    );
    let blocked = response_ending(vec![image_content()], ResponseEnding::refusal());
    assert_eq!(
        serve(&structured_request(), blocked).unwrap_err(),
        ClientError::ContentBlocked
    );
}

#[test]
fn given_a_structured_request_when_a_structured_block_or_a_call_is_present_then_a_blocking_ending_does_not_block()
 {
    let value = response_ending(vec![structured_content("1")], ResponseEnding::refusal());
    assert!(serve(&structured_request(), value).is_ok());
    let calls = response_ending(
        vec![call_content("search", "{}")],
        ResponseEnding::refusal(),
    );
    assert!(serve(&structured_with_tools_request(), calls).is_ok());
}

#[test]
fn given_a_structured_request_with_tools_when_the_response_holds_calls_and_text_then_it_is_given() {
    let calls = response_ending(
        vec![
            text_content("let me look"),
            thinking_content(),
            call_content("search", "{}"),
        ],
        ResponseEnding::tool_use(),
    );
    assert!(serve(&structured_with_tools_request(), calls).is_ok());
    let value = response_ending(vec![structured_content("1")], ResponseEnding::finished());
    assert!(serve(&structured_with_tools_request(), value).is_ok());
}

#[test]
fn given_a_structured_request_with_tools_when_the_response_holds_a_value_and_a_call_then_malformed()
{
    let response = response_ending(
        vec![structured_content("1"), call_content("search", "{}")],
        ResponseEnding::tool_use(),
    );
    assert!(is_malformed(serve(
        &structured_with_tools_request(),
        response
    )));
}

#[test]
fn given_capabilities_that_refuse_structured_with_tools_when_the_check_runs_then_calls_do_not_replace_the_value()
 {
    let capabilities = caps(|spec| spec.structured_output_with_tools = false);
    let calls = response_ending(
        vec![call_content("search", "{}")],
        ResponseEnding::tool_use(),
    );
    assert!(matches!(
        crate::response_check::check(&capabilities, &structured_with_tools_request(), &calls),
        Err(ClientError::MalformedResponse { .. })
    ));
    assert_eq!(
        crate::response_check::check(&everything_caps(), &structured_with_tools_request(), &calls),
        Ok(())
    );
}

#[test]
fn given_a_structured_request_when_a_structured_block_comes_beside_an_image_or_a_provider_block_then_it_is_given()
 {
    for beside in [image_content(), provider_content()] {
        let response = response_ending(
            vec![beside, structured_content(r#"{"a":1}"#)],
            ResponseEnding::finished(),
        );
        assert!(serve(&structured_request(), response).is_ok());
    }
}

#[test]
fn given_runtime_tool_calls_when_they_come_beside_an_image_or_a_provider_block_then_they_are_given()
{
    for beside in [image_content(), provider_content()] {
        let calls = response_ending(
            vec![beside, call_content("search", "{}")],
            ResponseEnding::tool_use(),
        );
        assert!(serve(&tools_request(), calls.clone()).is_ok());
        assert!(serve(&structured_with_tools_request(), calls).is_ok());
    }
}
