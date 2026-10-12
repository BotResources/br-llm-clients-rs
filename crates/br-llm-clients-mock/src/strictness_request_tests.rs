use br_llm_clients_core::{ClientError, OutputMode};
use br_llm_messages::ImageMime;

use crate::accepted::Accepted;
use crate::strictness::{ModeClass, check, check_request, classify_mode};
use crate::test_support::*;

#[test]
fn given_the_known_output_modes_when_classified_then_none_is_unknown() {
    assert_eq!(classify_mode(&OutputMode::Plain), ModeClass::Plain);
    assert_eq!(classify_mode(&shape()), ModeClass::Structured);
}

#[test]
fn given_an_output_mode_unknown_to_the_mock_when_checked_then_it_is_refused_at_the_structured_position()
 {
    let capabilities = everything_caps();
    let unknown = check_request(&capabilities, &request(), ModeClass::Unknown).unwrap_err();
    assert!(is_unknown_value(&unknown, "OutputMode"));
    let no_tools = caps(|spec| spec.tools = false);
    let with_tool = request_of(vec![user_text("hi")], vec![tool()], OutputMode::Plain);
    assert_eq!(
        check_request(&no_tools, &with_tool, ModeClass::Unknown),
        Err(ClientError::ToolsNotSupported)
    );
    let images = request_of(vec![user_image(ImageMime::Png)], vec![], OutputMode::Plain);
    let no_images = caps(|spec| spec.images = false);
    assert!(is_unknown_value(
        &check_request(&no_images, &images, ModeClass::Unknown).unwrap_err(),
        "OutputMode"
    ));
}

#[test]
fn given_invalid_messages_when_checked_then_the_check_of_core_decides_and_comes_first() {
    let capabilities = caps(|spec| spec.tools = false);
    let bad = request_of(
        vec![assistant_text("first"), user_text("then")],
        vec![tool()],
        OutputMode::Plain,
    );
    let error = check_request(&capabilities, &bad, ModeClass::Plain).unwrap_err();
    assert_eq!(Some(error), bad.check_messages().err());
}

#[test]
fn given_the_rules_of_core_when_checked_then_the_examples_of_the_intent_hold() {
    let capabilities = everything_caps();
    let accepted = request_of(
        vec![
            user_text("go"),
            assistant_call("a"),
            assistant_text("aside"),
            user_result("a"),
        ],
        vec![tool()],
        OutputMode::Plain,
    );
    assert_eq!(
        check_request(&capabilities, &accepted, ModeClass::Plain),
        Ok(())
    );
    let refused = request_of(
        vec![
            user_text("go"),
            assistant_call("a"),
            user_text("between"),
            user_result("a"),
        ],
        vec![tool()],
        OutputMode::Plain,
    );
    assert!(matches!(
        check_request(&capabilities, &refused, ModeClass::Plain),
        Err(ClientError::InvalidMessages { .. })
    ));
    let repeated = request_of(
        vec![
            user_text("go"),
            assistant_call("a"),
            user_result("a"),
            assistant_call("a"),
            user_result("a"),
        ],
        vec![tool()],
        OutputMode::Plain,
    );
    assert!(matches!(
        check_request(&capabilities, &repeated, ModeClass::Plain),
        Err(ClientError::InvalidMessages { .. })
    ));
}

#[test]
fn given_the_rules_left_to_each_library_when_checked_then_the_mock_accepts_them() {
    let capabilities = caps(|spec| spec.tools = false);
    let lenient = request_of(
        vec![
            user_text("go"),
            assistant_call("a"),
            user_result("a"),
            assistant_text("one"),
            assistant_text("two"),
        ],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(
        check_request(&capabilities, &lenient, ModeClass::Plain),
        Ok(())
    );
}

#[test]
fn given_tools_on_a_model_without_tools_when_checked_then_tools_not_supported() {
    let capabilities = caps(|spec| spec.tools = false);
    let with_tool = request_of(vec![user_text("hi")], vec![tool()], OutputMode::Plain);
    assert_eq!(
        check(&capabilities, &settings(), &with_tool),
        Err(ClientError::ToolsNotSupported)
    );
}

#[test]
fn given_the_name_of_a_tool_call_that_is_not_declared_when_checked_then_nothing_is_refused() {
    let undeclared = request_of(
        vec![user_text("go"), assistant_call("a"), user_result("a")],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(check(&everything_caps(), &settings(), &undeclared), Ok(()));
}

#[test]
fn given_a_structured_mode_when_checked_then_each_missing_capability_is_its_own_error() {
    let structured = request_of(vec![user_text("hi")], vec![], shape());
    let without = caps(|spec| spec.structured_output = false);
    assert_eq!(
        check(&without, &settings(), &structured),
        Err(ClientError::StructuredOutputNotSupported)
    );
    let with_tools = request_of(vec![user_text("hi")], vec![tool()], shape());
    let not_together = caps(|spec| spec.structured_output_with_tools = false);
    assert_eq!(
        check(&not_together, &settings(), &with_tools),
        Err(ClientError::StructuredOutputWithToolsNotSupported)
    );
    assert_eq!(check(&not_together, &settings(), &structured), Ok(()));
    assert_eq!(check(&everything_caps(), &settings(), &with_tools), Ok(()));
}

#[test]
fn given_a_structured_request_with_tools_when_tools_are_missing_then_tools_come_before_structured()
{
    let with_tools = request_of(vec![user_text("hi")], vec![tool()], shape());
    let nothing = caps(|spec| {
        spec.tools = false;
        spec.structured_output = false;
    });
    assert_eq!(
        check(&nothing, &settings(), &with_tools),
        Err(ClientError::ToolsNotSupported)
    );
}

#[test]
fn given_an_image_on_a_model_without_images_when_checked_then_image_not_supported() {
    let capabilities = caps(|spec| spec.images = false);
    let with_image = request_of(vec![user_image(ImageMime::Png)], vec![], OutputMode::Plain);
    assert_eq!(
        check(&capabilities, &settings(), &with_image),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_images_of_several_types_when_one_is_refused_then_the_first_such_image_is_named() {
    let capabilities = caps(|spec| {
        spec.image_media_types = Accepted::Only(vec![ImageMime::Png]);
    });
    let messages = vec![
        user_image(ImageMime::Png),
        assistant_text("ok"),
        user_image(ImageMime::Gif),
        user_image(ImageMime::Webp),
    ];
    let request_value = request_of(messages, vec![], OutputMode::Plain);
    assert_eq!(
        check(&capabilities, &settings(), &request_value),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Gif
        })
    );
}

#[test]
fn given_two_images_in_one_message_when_the_second_is_refused_then_the_second_is_named() {
    let capabilities = caps(|spec| {
        spec.image_media_types = Accepted::Only(vec![ImageMime::Png]);
    });
    let message = br_llm_messages::WireMessage::User {
        content: vec![
            br_llm_messages::WireUserBlock::Image(br_llm_messages::Image {
                mime: ImageMime::Png,
                data: br_llm_messages::Base64Data::new("aGVsbG8=").unwrap(),
            }),
            br_llm_messages::WireUserBlock::Image(br_llm_messages::Image {
                mime: ImageMime::Jpeg,
                data: br_llm_messages::Base64Data::new("aGVsbG8=").unwrap(),
            }),
        ],
    };
    let request_value = request_of(vec![message], vec![], OutputMode::Plain);
    assert_eq!(
        check(&capabilities, &settings(), &request_value),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Jpeg
        })
    );
}
