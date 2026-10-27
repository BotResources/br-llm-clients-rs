use br_llm_messages::{IncomingEntry, MediaType, TurnEntry};

use crate::capabilities_test_support::{caps_with, plain_settings};
use crate::thread_fixtures::{
    Fixture, agent_mail, agent_request, ai_turn, as_turn_entry, content_image, finished, human,
    human_turn, plain_image, text_block, turn_id,
};
use crate::{Accepted, Capabilities, ClientError, ClientRequest, OutputMode};

type Make = fn(Vec<br_llm_messages::PlainBlock>) -> IncomingEntry;

const KINDS: [(&str, Make); 3] = [
    ("human message", human),
    ("agent request", agent_request),
    ("agent mail", agent_mail),
];

fn refuses_images() -> Capabilities {
    caps_with(|spec| spec.images = false)
}

fn only_png() -> Capabilities {
    caps_with(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Png]))
}

fn check_ai_turn(capabilities: &Capabilities, entries: Vec<TurnEntry>) -> Result<(), ClientError> {
    let fixture = Fixture::new();
    let reply = turn_id();
    let thread = fixture.project(vec![
        human_turn(vec![text_block("go")]),
        ai_turn(reply, fixture.agent, entries),
    ]);
    let request = ClientRequest::new(
        None,
        thread,
        reply,
        crate::thread_fixtures::entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap();
    capabilities.check(&plain_settings(), &request)
}

#[test]
fn given_an_image_in_an_incoming_entry_of_an_ai_turn_when_the_model_takes_none_then_image_not_supported()
 {
    for (kind, make) in KINDS {
        let entry = as_turn_entry(make(vec![plain_image(MediaType::Png)]));
        assert_eq!(
            check_ai_turn(&refuses_images(), vec![entry]),
            Err(ClientError::ImageNotSupported),
            "{kind}"
        );
    }
}

#[test]
fn given_an_incoming_entry_of_an_ai_turn_without_image_when_the_model_takes_none_then_it_passes() {
    for (kind, make) in KINDS {
        let entry = as_turn_entry(make(vec![text_block("plain")]));
        assert_eq!(
            check_ai_turn(&refuses_images(), vec![entry]),
            Ok(()),
            "{kind}"
        );
    }
}

#[test]
fn given_a_bad_media_type_in_an_incoming_entry_of_an_ai_turn_when_checked_then_it_is_named() {
    for (kind, make) in KINDS {
        let entry = as_turn_entry(make(vec![
            plain_image(MediaType::Png),
            plain_image(MediaType::Gif),
        ]));
        assert_eq!(
            check_ai_turn(&only_png(), vec![entry]),
            Err(ClientError::ImageMediaTypeNotSupported {
                media_type: MediaType::Gif
            }),
            "{kind}"
        );
    }
}

#[test]
fn given_an_incoming_entry_before_a_response_image_when_checked_then_the_entry_image_wins() {
    for (kind, make) in KINDS {
        let entry = as_turn_entry(make(vec![plain_image(MediaType::Gif)]));
        let response = finished(vec![content_image(MediaType::Webp)]);
        assert_eq!(
            check_ai_turn(&only_png(), vec![entry, response]),
            Err(ClientError::ImageMediaTypeNotSupported {
                media_type: MediaType::Gif
            }),
            "{kind}"
        );
    }
}

#[test]
fn given_an_incoming_entry_after_a_response_image_when_checked_then_the_response_image_wins() {
    for (kind, make) in KINDS {
        let entry = as_turn_entry(make(vec![plain_image(MediaType::Gif)]));
        let response = finished(vec![content_image(MediaType::Webp)]);
        assert_eq!(
            check_ai_turn(&only_png(), vec![response, entry]),
            Err(ClientError::ImageMediaTypeNotSupported {
                media_type: MediaType::Webp
            }),
            "{kind}"
        );
    }
}
