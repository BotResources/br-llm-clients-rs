use br_llm_clients_core::{ClientError, OutputMode, ReasoningLevel};
use br_llm_messages::MediaType;

use crate::Accepted;
use crate::strictness::check;
use crate::test_support::*;

#[test]
fn given_images_of_several_media_types_when_some_are_refused_then_the_first_refused_in_reading_order_is_named()
 {
    let capabilities = caps(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Gif]));
    let images = request_of(
        vec![
            crate::fixtures::plain_image(MediaType::Gif),
            crate::fixtures::plain_image(MediaType::Webp),
            crate::fixtures::plain_image(MediaType::Png),
        ],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(
        check(&capabilities, &settings(), &images),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Webp
        })
    );
}

#[test]
fn given_a_level_not_in_the_capabilities_when_called_then_the_reason_names_the_model_of_the_settings_and_not_the_mock()
 {
    let client = crate::MockClient::new(
        settings(),
        caps(|spec| spec.reasoning_levels = Accepted::Only(vec![])),
    );
    let ClientError::ReasoningNotSupported { level, reason } =
        error_of(&run(&client, &request()).0)
    else {
        panic!("reasoning refusal expected");
    };
    assert_eq!(level, ReasoningLevel::Medium);
    assert!(reason.contains("settings-model"));
    assert!(!reason.to_lowercase().contains("mock"));
}

#[test]
fn given_a_history_ending_with_an_assistant_message_when_called_then_the_mock_accepts_it() {
    let agent = crate::fixtures::agent_id();
    let history = crate::fixtures::project(
        agent,
        vec![
            crate::fixtures::human_turn(vec![crate::fixtures::text_block("first")]),
            crate::fixtures::answered_turn(agent, "last"),
        ],
    );
    let accepted = br_llm_clients_core::ClientRequest::new(
        None,
        history,
        crate::fixtures::turn_id(),
        crate::fixtures::entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap();
    let client = mock();
    client.push_reply(crate::Reply::text("fine").build().unwrap());
    let (outcome, _) = run(&client, &accepted);
    assert_eq!(text_of(&outcome), "fine");
}
