use br_llm_messages::{MediaType, ProjectedThread};

use crate::capabilities_test_support::{
    ALL_MEDIA_TYPES, caps_with, image_request, plain_request, plain_settings, request_of,
};
use crate::thread_fixtures::{
    Fixture, ai_turn, content_text, finished, human_turn, plain_images, text_block, turn_id,
};
use crate::{Accepted, Capabilities, ClientError, OutputMode};

fn thread_of_image_groups(groups: &[&[MediaType]]) -> ProjectedThread {
    let fixture = Fixture::new();
    let mut turns = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        if index > 0 {
            turns.push(ai_turn(
                turn_id(),
                fixture.agent,
                vec![finished(vec![content_text("seen")])],
            ));
        }
        let mut content = vec![text_block("see")];
        content.extend(plain_images(group));
        turns.push(human_turn(content));
    }
    fixture.project(turns)
}

fn request_of_groups(groups: &[&[MediaType]]) -> crate::ClientRequest {
    request_of(thread_of_image_groups(groups), vec![], OutputMode::Plain)
}

#[test]
fn given_an_image_when_the_model_takes_none_then_image_not_supported_whatever_the_media_types() {
    for media in [
        Accepted::Any,
        Accepted::Only(Vec::new()),
        Accepted::Only(vec![MediaType::Png]),
    ] {
        let capabilities = caps_with(|spec| {
            spec.images = false;
            spec.image_media_types = media.clone();
        });
        assert_eq!(
            capabilities.check(&plain_settings(), &image_request(&[MediaType::Png])),
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
    let request = request_of_groups(&[&[], &[], &[MediaType::Gif]]);
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_a_media_type_the_list_does_not_hold_when_checked_then_it_is_named() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Png]));
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&[MediaType::Gif])),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Gif
        })
    );
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&[MediaType::Png])),
        Ok(())
    );
}

#[test]
fn given_several_bad_images_in_one_entry_when_checked_then_the_first_in_block_order_is_named() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Png]));
    for (order, first) in [
        (
            [MediaType::Png, MediaType::Gif, MediaType::Webp],
            MediaType::Gif,
        ),
        (
            [MediaType::Webp, MediaType::Gif, MediaType::Png],
            MediaType::Webp,
        ),
        (
            [MediaType::Jpeg, MediaType::Png, MediaType::Gif],
            MediaType::Jpeg,
        ),
    ] {
        assert_eq!(
            capabilities.check(&plain_settings(), &image_request(&order)),
            Err(ClientError::ImageMediaTypeNotSupported { media_type: first })
        );
    }
}

#[test]
fn given_bad_images_in_several_turns_when_checked_then_turn_order_comes_before_block_order() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Png]));
    let request = request_of_groups(&[&[MediaType::Png, MediaType::Webp], &[MediaType::Gif]]);
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Webp
        })
    );
}

#[test]
fn given_an_image_after_good_images_in_a_later_turn_when_checked_then_it_is_named() {
    let capabilities =
        caps_with(|spec| spec.image_media_types = Accepted::Only(vec![MediaType::Png]));
    let request = request_of_groups(&[&[MediaType::Png], &[MediaType::Png, MediaType::Jpeg]]);
    assert_eq!(
        capabilities.check(&plain_settings(), &request),
        Err(ClientError::ImageMediaTypeNotSupported {
            media_type: MediaType::Jpeg
        })
    );
}

#[test]
fn given_a_thread_without_any_image_when_checked_against_nothing_then_no_image_rule_applies() {
    let request = request_of_groups(&[&[], &[]]);
    assert_eq!(
        Capabilities::nothing().check(&plain_settings(), &request),
        Ok(())
    );
}

#[test]
fn given_every_media_type_when_the_list_holds_all_of_them_then_every_image_passes() {
    let capabilities = caps_with(|spec| {
        spec.image_media_types = Accepted::Only(ALL_MEDIA_TYPES.to_vec());
    });
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&ALL_MEDIA_TYPES)),
        Ok(())
    );
}
