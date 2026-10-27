use br_llm_messages::{
    ContentBlock, IncomingEntry, MediaType, ProjectedThread, Turn, TurnEntry, TurnId,
};

use crate::capabilities_test_support::{caps_with, plain_settings};
use crate::thread_fixtures::{
    Fixture, agent_id, agent_mail, agent_request, ai_turn, content_image, content_text, entry_id,
    finished, human_turn, input_turn, plain_image, plain_images, provider_record_block,
    provider_result_blocks, response, text_block, tool_call, tool_results, turn_id,
};
use crate::{Accepted, Capabilities, ClientError, ClientRequest, OutputMode};

const PNG: MediaType = MediaType::Png;

fn refuses_images() -> Capabilities {
    caps_with(|spec| spec.images = false)
}

fn only_png() -> Capabilities {
    caps_with(|spec| spec.image_media_types = Accepted::Only(vec![PNG]))
}

fn request_replying_in(thread: ProjectedThread, reply: TurnId) -> ClientRequest {
    ClientRequest::new(None, thread, reply, entry_id(), vec![], OutputMode::Plain).unwrap()
}

fn is_read(capabilities: &Capabilities, thread: ProjectedThread, reply: TurnId) -> bool {
    let request = request_replying_in(thread, reply);
    match capabilities.check(&plain_settings(), &request) {
        Ok(()) => false,
        Err(ClientError::ImageNotSupported) => true,
        Err(other) => panic!("a case outside the image rules: {other:?}"),
    }
}

fn first_bad(thread: ProjectedThread, reply: TurnId) -> Option<MediaType> {
    let request = request_replying_in(thread, reply);
    match only_png().check(&plain_settings(), &request) {
        Err(ClientError::ImageMediaTypeNotSupported { media_type }) => Some(media_type),
        _ => None,
    }
}

fn some_turn() -> TurnId {
    turn_id()
}

#[test]
fn given_an_image_in_a_human_message_when_checked_then_it_is_read() {
    let thread = Fixture::new().project(vec![human_turn(vec![plain_image(PNG)])]);
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_in_an_agent_request_when_checked_then_it_is_read() {
    let thread = Fixture::new().project(vec![input_turn(vec![agent_request(vec![plain_image(
        PNG,
    )])])]);
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_in_an_agent_mail_when_checked_then_it_is_read() {
    let thread = Fixture::new().project(vec![input_turn(vec![agent_mail(vec![plain_image(PNG)])])]);
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_in_the_output_of_another_agent_when_checked_then_it_is_read() {
    let fixture = Fixture::new();
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(
            turn_id(),
            agent_id(),
            vec![finished(vec![content_image(PNG)])],
        ),
    ]);
    let projected_as_output = thread.turns().iter().any(|turn| match turn {
        Turn::InputTurn(input) => input
            .entries()
            .iter()
            .any(|entry| matches!(entry, IncomingEntry::AgentOutput(_))),
        _ => false,
    });
    assert!(projected_as_output);
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_in_a_partial_output_of_another_agent_when_checked_then_it_is_read() {
    let thread = Fixture::new().project_with_partial_output(crate::thread_fixtures::image(PNG));
    let projected_as_partial = thread.turns().iter().any(|turn| match turn {
        Turn::InputTurn(input) => input
            .entries()
            .iter()
            .any(|entry| matches!(entry, IncomingEntry::PartialAgentOutput(_))),
        _ => false,
    });
    assert!(projected_as_partial);
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_in_a_runtime_tool_result_when_checked_then_it_is_read() {
    let fixture = Fixture::new();
    let call = tool_call("search");
    let reply = response(
        vec![ContentBlock::RuntimeToolCall(call.clone())],
        br_llm_messages::ResponseEnding::tool_use(),
    );
    let results = tool_results(&[(&call, vec![plain_image(PNG)])]);
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(
            turn_id(),
            fixture.agent,
            vec![
                TurnEntry::ModelResponse(reply),
                TurnEntry::RuntimeToolResults(results),
            ],
        ),
    ]);
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_block_in_a_model_response_of_any_turn_when_checked_then_it_is_read() {
    let fixture = Fixture::new();
    let turn = turn_id();
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(
            turn,
            fixture.agent,
            vec![finished(vec![content_image(PNG)])],
        ),
    ]);
    assert!(is_read(&refuses_images(), thread.clone(), turn));
    assert!(is_read(&refuses_images(), thread, some_turn()));
}

fn thread_with_provider_blocks(blocks: Vec<ContentBlock>) -> (ProjectedThread, TurnId) {
    let fixture = Fixture::new();
    let turn = turn_id();
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(turn, fixture.agent, vec![finished(blocks)]),
    ]);
    (thread, turn)
}

#[test]
fn given_an_image_in_a_provider_tool_result_when_the_turn_is_the_reply_turn_then_it_is_read() {
    let (thread, turn) = thread_with_provider_blocks(provider_result_blocks(PNG));
    assert!(is_read(&refuses_images(), thread, turn));
}

#[test]
fn given_an_image_in_a_provider_tool_result_when_the_turn_is_another_turn_then_it_is_not_read() {
    let (thread, _) = thread_with_provider_blocks(provider_result_blocks(PNG));
    assert!(!is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_an_image_in_a_provider_execution_record_when_the_turn_is_the_reply_turn_then_it_is_read() {
    let (thread, turn) = thread_with_provider_blocks(vec![provider_record_block(PNG)]);
    assert!(is_read(&refuses_images(), thread, turn));
}

#[test]
fn given_an_image_in_a_provider_execution_record_when_the_turn_is_another_turn_then_it_is_not_read()
{
    let (thread, _) = thread_with_provider_blocks(vec![provider_record_block(PNG)]);
    assert!(!is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_the_same_thread_when_the_reply_turn_changes_then_only_the_reply_turn_reads_its_provider_blocks()
 {
    let (thread, turn) = thread_with_provider_blocks(provider_result_blocks(PNG));
    let capabilities = refuses_images();
    assert!(is_read(&capabilities, thread.clone(), turn));
    assert!(!is_read(&capabilities, thread, some_turn()));
}

#[test]
fn given_a_thread_without_an_image_when_checked_then_none_is_read() {
    let fixture = Fixture::new();
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(
            turn_id(),
            fixture.agent,
            vec![finished(vec![content_text("done")])],
        ),
    ]);
    assert!(!is_read(&refuses_images(), thread, some_turn()));
}

#[test]
fn given_images_in_a_response_when_checked_then_block_order_decides_between_image_and_provider_blocks()
 {
    let mut blocks = provider_result_blocks(MediaType::Webp);
    blocks.push(content_image(MediaType::Gif));
    let (thread, turn) = thread_with_provider_blocks(blocks);
    assert_eq!(first_bad(thread, turn), Some(MediaType::Webp));

    let mut blocks = vec![content_image(MediaType::Gif)];
    blocks.extend(provider_result_blocks(MediaType::Webp));
    let (thread, turn) = thread_with_provider_blocks(blocks);
    assert_eq!(first_bad(thread, turn), Some(MediaType::Gif));
}

#[test]
fn given_images_in_several_results_when_checked_then_result_order_then_block_order_decides() {
    let fixture = Fixture::new();
    let (first, second) = (tool_call("first"), tool_call("second"));
    let reply = response(
        vec![
            ContentBlock::RuntimeToolCall(first.clone()),
            ContentBlock::RuntimeToolCall(second.clone()),
        ],
        br_llm_messages::ResponseEnding::tool_use(),
    );
    let results = tool_results(&[
        (
            &second,
            plain_images(&[PNG, MediaType::Webp, MediaType::Gif]),
        ),
        (&first, plain_images(&[MediaType::Jpeg])),
    ]);
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(
            turn_id(),
            fixture.agent,
            vec![
                TurnEntry::ModelResponse(reply),
                TurnEntry::RuntimeToolResults(results),
            ],
        ),
    ]);
    assert_eq!(first_bad(thread, some_turn()), Some(MediaType::Webp));
}

#[test]
fn given_images_in_several_kinds_of_entry_when_checked_then_thread_order_decides() {
    let fixture = Fixture::new();
    let call = tool_call("search");
    let reply = response(
        vec![ContentBlock::RuntimeToolCall(call.clone())],
        br_llm_messages::ResponseEnding::tool_use(),
    );
    let results = tool_results(&[(&call, plain_images(&[MediaType::Gif]))]);
    let thread = fixture.project(vec![
        human_turn(vec![plain_image(PNG)]),
        ai_turn(
            turn_id(),
            fixture.agent,
            vec![
                TurnEntry::ModelResponse(reply),
                TurnEntry::RuntimeToolResults(results),
            ],
        ),
        human_turn(vec![plain_image(MediaType::Webp)]),
    ]);
    assert_eq!(first_bad(thread, some_turn()), Some(MediaType::Gif));
}

#[test]
fn given_a_request_built_from_one_thread_when_the_check_reads_images_then_the_request_gives_the_reply_turn()
 {
    let (thread, turn) = thread_with_provider_blocks(provider_result_blocks(PNG));
    let request = request_replying_in(thread, turn);
    assert_eq!(request.reply_turn(), turn);
    assert_eq!(
        refuses_images().check(&plain_settings(), &request),
        Err(ClientError::ImageNotSupported)
    );
}
