use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{
    ALL_MIMES, assistant_text, caps_with, image_request, plain_request, plain_settings, request_of,
    user_images, user_result_message, user_text,
};
use crate::{Accepted, Capabilities, ClientError, OutputMode};

#[test]
fn given_an_image_when_the_model_takes_none_then_image_not_supported_whatever_the_media_types() {
    for media in [
        Accepted::Any,
        Accepted::Only(Vec::new()),
        Accepted::Only(vec![ImageMime::Png]),
    ] {
        let capabilities = caps_with(|spec| {
            spec.images = false;
            spec.image_media_types = media.clone();
        });
        assert_eq!(
            capabilities.check(&plain_settings(), &image_request(&[ImageMime::Png])),
            Err(ClientError::ImageNotSupported)
        );
        assert_eq!(
            capabilities.check(&plain_settings(), &plain_request()),
            Ok(())
        );
    }
}

#[test]
fn given_an_image_late_in_the_conversation_when_the_model_takes_none_then_it_is_still_found() {
    let capabilities = caps_with(|spec| spec.images = false);
    let request = request_of(
        vec![
            user_text("a"),
            assistant_text("b"),
            user_images(&[ImageMime::Gif]),
        ],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_a_media_type_the_list_does_not_hold_when_checked_then_it_is_named() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![ImageMime::Png]));
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&[ImageMime::Gif])),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Gif
        })
    );
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&[ImageMime::Png])),
        Ok(())
    );
}

#[test]
fn given_several_bad_images_in_one_message_when_checked_then_the_first_in_block_order_is_named() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![ImageMime::Png]));
    for (order, first) in [
        (
            [ImageMime::Png, ImageMime::Gif, ImageMime::Webp],
            ImageMime::Gif,
        ),
        (
            [ImageMime::Webp, ImageMime::Gif, ImageMime::Png],
            ImageMime::Webp,
        ),
        (
            [ImageMime::Jpeg, ImageMime::Png, ImageMime::Gif],
            ImageMime::Jpeg,
        ),
    ] {
        assert_eq!(
            capabilities.check(&plain_settings(), &image_request(&order)),
            Err(ClientError::ImageMediaTypeNotSupported { media_type: first })
        );
    }
}

#[test]
fn given_bad_images_in_several_messages_when_checked_then_message_order_comes_before_block_order() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![ImageMime::Png]));
    let request = request_of(
        vec![
            user_images(&[ImageMime::Png, ImageMime::Webp]),
            assistant_text("seen"),
            user_images(&[ImageMime::Gif]),
        ],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Webp
        })
    );
}

#[test]
fn given_an_image_after_good_images_in_a_later_user_message_when_checked_then_it_is_named() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![ImageMime::Png]));
    let request = request_of(
        vec![
            user_images(&[ImageMime::Png]),
            assistant_text("seen"),
            user_images(&[ImageMime::Png, ImageMime::Jpeg]),
        ],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Jpeg
        })
    );
}

#[test]
fn given_a_user_message_with_text_and_results_and_no_image_when_checked_then_no_image_rule_applies()
{
    let capabilities = Capabilities::nothing();
    let request = request_of(
        vec![user_text("a"), user_result_message("t1")],
        vec![],
        OutputMode::Plain,
    );
    assert_eq!(capabilities.check(&plain_settings(), &request), Ok(()));
}

#[test]
fn given_every_media_type_when_the_list_holds_all_of_them_then_every_image_passes() {
    let capabilities = caps_with(|spec| {
        spec.image_media_types = Accepted::Only(ALL_MIMES.to_vec());
    });
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&ALL_MIMES)),
        Ok(())
    );
}
