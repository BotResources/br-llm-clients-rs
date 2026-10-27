use std::num::NonZeroU32;

use br_llm_messages::MediaType;

use crate::capabilities_test_support::{AmbiguousIfDefault, assert_plain_data};
use crate::{Accepted, CapabilitiesSpec, MaxOutputTokens, ReasoningLevel};

fn non_zero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

#[test]
fn given_a_non_zero_number_when_made_a_max_output_tokens_then_the_number_is_the_same() {
    for number in [1, 8192, u32::MAX] {
        assert_eq!(
            CapabilitiesSpec::max_output_tokens_of(non_zero(number)).get(),
            number
        );
    }
}

#[test]
fn given_max_output_tokens_of_when_compared_to_the_public_constructor_then_the_values_are_equal() {
    assert_eq!(
        CapabilitiesSpec::max_output_tokens_of(non_zero(8192)),
        MaxOutputTokens::new(8192).unwrap()
    );
}

#[test]
fn given_everything_when_built_then_every_field_is_the_most_open_value() {
    let spec = CapabilitiesSpec::everything();
    assert_eq!(spec.reasoning_levels, Accepted::Any);
    assert_eq!(spec.max_output_tokens.get(), u32::MAX);
    assert_eq!(
        spec.max_output_tokens,
        CapabilitiesSpec::max_output_tokens_of(NonZeroU32::MAX)
    );
    assert!(spec.tools);
    assert!(spec.structured_output);
    assert!(spec.structured_output_with_tools);
    assert!(spec.caching);
    assert!(spec.images);
    assert_eq!(spec.image_media_types, Accepted::Any);
}

#[test]
fn given_nothing_when_built_then_no_optional_feature_is_declared_and_no_output_limit() {
    let spec = CapabilitiesSpec::nothing();
    assert_eq!(
        spec.reasoning_levels,
        Accepted::Only(vec![ReasoningLevel::Off])
    );
    assert_eq!(spec.max_output_tokens.get(), u32::MAX);
    assert!(!spec.tools);
    assert!(!spec.structured_output);
    assert!(!spec.structured_output_with_tools);
    assert!(!spec.caching);
    assert!(!spec.images);
    assert_eq!(spec.image_media_types, Accepted::Only(Vec::new()));
}

#[test]
fn given_the_two_starting_points_when_asked_if_inconsistent_then_neither_is() {
    assert!(!CapabilitiesSpec::everything().is_inconsistent());
    assert!(!CapabilitiesSpec::nothing().is_inconsistent());
}

#[test]
fn given_every_combination_of_images_and_media_types_when_asked_then_only_images_with_an_empty_only_is_inconsistent()
 {
    let sets: [Accepted<MediaType>; 3] = [
        Accepted::Any,
        Accepted::Only(Vec::new()),
        Accepted::Only(vec![MediaType::Png]),
    ];
    for images in [true, false] {
        for set in &sets {
            let spec = CapabilitiesSpec {
                images,
                image_media_types: set.clone(),
                ..CapabilitiesSpec::everything()
            };
            let expected = images && *set == Accepted::Only(Vec::new());
            assert_eq!(spec.is_inconsistent(), expected, "{images} {set:?}");
        }
    }
}

#[test]
fn given_two_specs_that_differ_in_one_field_when_compared_then_they_are_not_equal() {
    let base = CapabilitiesSpec::everything();
    let variants = [
        CapabilitiesSpec {
            reasoning_levels: Accepted::Only(Vec::new()),
            ..base.clone()
        },
        CapabilitiesSpec {
            max_output_tokens: MaxOutputTokens::new(1).unwrap(),
            ..base.clone()
        },
        CapabilitiesSpec {
            tools: false,
            ..base.clone()
        },
        CapabilitiesSpec {
            structured_output: false,
            ..base.clone()
        },
        CapabilitiesSpec {
            structured_output_with_tools: false,
            ..base.clone()
        },
        CapabilitiesSpec {
            caching: false,
            ..base.clone()
        },
        CapabilitiesSpec {
            images: false,
            ..base.clone()
        },
        CapabilitiesSpec {
            image_media_types: Accepted::Only(Vec::new()),
            ..base.clone()
        },
    ];
    for variant in &variants {
        assert_ne!(*variant, base);
    }
}

#[test]
fn given_a_spec_when_compiled_then_it_is_plain_data() {
    assert_plain_data::<CapabilitiesSpec>();
}

#[test]
fn given_a_spec_when_compiled_then_it_has_no_default_value() {
    <CapabilitiesSpec as AmbiguousIfDefault<_>>::proof();
}
