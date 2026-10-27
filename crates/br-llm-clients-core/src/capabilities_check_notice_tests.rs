use br_llm_messages::{MediaType, RuntimeNotice, TurnEntry};

use crate::capabilities_test_support::{caps_with, plain_settings, request_of};
use crate::test_support::tool;
use crate::thread_fixtures::{
    Fixture, ai_turn, entry_id, finished, human, human_turn, input_turn, plain_image, text_block,
    turn_id,
};
use crate::{Accepted, ClientError, ClientRequest, OutputMode};

fn notice_entry() -> RuntimeNotice {
    RuntimeNotice::new(entry_id(), "the runtime says so").unwrap()
}

#[test]
fn given_runtime_notices_in_an_input_turn_and_an_ai_turn_when_checked_then_they_are_ignored() {
    let fixture = Fixture::new();
    let reply = turn_id();
    let thread = fixture.project(vec![
        input_turn(vec![human(vec![text_block("go")]), notice_entry().into()]),
        ai_turn(
            reply,
            fixture.agent,
            vec![
                finished(vec![crate::thread_fixtures::content_text("hi")]),
                TurnEntry::RuntimeNotice(notice_entry()),
            ],
        ),
    ]);
    let request =
        ClientRequest::new(None, thread, reply, entry_id(), vec![], OutputMode::Plain).unwrap();
    let nothing = caps_with(|spec| spec.images = false);
    assert_eq!(nothing.check(&plain_settings(), &request), Ok(()));
}

#[test]
fn given_a_runtime_notice_before_an_image_when_checked_then_the_image_is_still_found() {
    let fixture = Fixture::new();
    let thread = fixture.project(vec![
        input_turn(vec![
            notice_entry().into(),
            human(vec![plain_image(MediaType::Gif)]),
        ]),
        human_turn(vec![text_block("more")]),
    ]);
    let request = request_of(thread, vec![], OutputMode::Plain);
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Png]));
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Gif
        })
    );
}

#[test]
fn given_a_tool_named_with_256_characters_when_the_request_is_built_and_checked_then_it_is_kept() {
    let name = "t".repeat(256);
    let thread = Fixture::new().project(vec![human_turn(vec![text_block("go")])]);
    let request = request_of(thread, vec![tool(&name)], OutputMode::Plain);
    assert_eq!(request.tools()[0].name().as_str(), "t".repeat(256));
    assert_eq!(
        crate::Capabilities::everything().check(&plain_settings(), &request),
        Ok(())
    );
}
