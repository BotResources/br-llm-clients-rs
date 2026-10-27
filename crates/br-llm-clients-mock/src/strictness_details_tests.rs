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
