use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{
    ALL_MIMES, AmbiguousIfDefault, caps_with, image_request, plain_settings,
};
use crate::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, ClientError, MaxOutputTokens,
    ReasoningLevel,
};

fn level_sets() -> Vec<Accepted<ReasoningLevel>> {
    vec![
        Accepted::Any,
        Accepted::Only(Vec::new()),
        Accepted::Only(vec![ReasoningLevel::Off]),
        Accepted::Only(vec![ReasoningLevel::Low, ReasoningLevel::Max]),
    ]
}

fn media_sets() -> Vec<Accepted<ImageMime>> {
    vec![
        Accepted::Any,
        Accepted::Only(Vec::new()),
        Accepted::Only(vec![ImageMime::Png]),
        Accepted::Only(vec![ImageMime::Png, ImageMime::Gif]),
    ]
}

fn every_spec() -> Vec<CapabilitiesSpec> {
    let mut specs = Vec::new();
    for bits in 0..16u8 {
        for images in [true, false] {
            for levels in level_sets() {
                for media in media_sets() {
                    for tokens in [1, u32::MAX] {
                        specs.push(CapabilitiesSpec {
                            reasoning_levels: levels.clone(),
                            max_output_tokens: MaxOutputTokens::new(tokens).unwrap(),
                            tools: bits & 1 != 0,
                            structured_output: bits & 2 != 0,
                            structured_output_with_tools: bits & 4 != 0,
                            caching: bits & 8 != 0,
                            images,
                            image_media_types: media.clone(),
                        });
                    }
                }
            }
        }
    }
    specs
}

#[test]
fn given_every_spec_when_built_with_new_then_only_the_inconsistent_spec_is_refused() {
    for spec in every_spec() {
        let inconsistent = spec.images && spec.image_media_types == Accepted::Only(Vec::new());
        match Capabilities::new(spec.clone()) {
            Err(error) => {
                assert!(inconsistent, "{spec:?}");
                assert_eq!(error, CapabilitiesError::InconsistentCapabilities);
            }
            Ok(capabilities) => {
                assert!(!inconsistent, "{spec:?}");
                assert_eq!(capabilities.spec(), &spec);
            }
        }
    }
}

#[test]
fn given_every_consistent_spec_when_normalized_then_the_value_is_the_one_of_new() {
    for spec in every_spec() {
        if let Ok(built) = Capabilities::new(spec.clone()) {
            assert_eq!(Capabilities::normalized(spec), built);
        }
    }
}

#[test]
fn given_every_inconsistent_spec_when_normalized_then_images_is_false_and_nothing_else_changes() {
    let mut seen = 0;
    for spec in every_spec() {
        if Capabilities::new(spec.clone()).is_ok() {
            continue;
        }
        seen += 1;
        let normalized = Capabilities::normalized(spec.clone());
        assert!(!normalized.images());
        assert_eq!(
            normalized.spec(),
            &CapabilitiesSpec {
                images: false,
                ..spec
            }
        );
    }
    assert!(seen > 0);
}

#[test]
fn given_every_spec_when_normalized_then_the_result_is_accepted_by_new_unchanged() {
    for spec in every_spec() {
        let normalized = Capabilities::normalized(spec);
        assert_eq!(Capabilities::new(normalized.spec().clone()), Ok(normalized));
    }
}

#[test]
fn given_the_three_images_cases_when_normalized_then_only_images_with_an_empty_only_changes() {
    let cases = [
        (true, Accepted::Any, true),
        (true, Accepted::Only(Vec::new()), false),
        (true, Accepted::Only(vec![ImageMime::Png]), true),
        (false, Accepted::Any, false),
        (false, Accepted::Only(Vec::new()), false),
        (false, Accepted::Only(vec![ImageMime::Png]), false),
    ];
    for (images, media, expected_images) in cases {
        let capabilities = Capabilities::normalized(CapabilitiesSpec {
            images,
            image_media_types: media.clone(),
            ..CapabilitiesSpec::everything()
        });
        assert_eq!(capabilities.images(), expected_images, "{images} {media:?}");
        assert_eq!(capabilities.image_media_types(), &media);
    }
}

#[test]
fn given_a_normalized_inconsistent_spec_when_a_call_holds_an_image_then_it_is_refused_with_image_not_supported()
 {
    let capabilities = caps_with(|spec| spec.image_media_types = Accepted::Only(Vec::new()));
    assert_eq!(
        capabilities.check(&plain_settings(), &image_request(&[ImageMime::Png])),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_the_starting_points_when_built_as_capabilities_then_they_are_the_normalized_specs() {
    assert_eq!(
        Capabilities::everything(),
        Capabilities::normalized(CapabilitiesSpec::everything())
    );
    assert_eq!(
        Capabilities::nothing(),
        Capabilities::normalized(CapabilitiesSpec::nothing())
    );
    assert_eq!(
        Capabilities::everything().spec(),
        &CapabilitiesSpec::everything()
    );
    assert_eq!(Capabilities::nothing().spec(), &CapabilitiesSpec::nothing());
    assert_eq!(
        Capabilities::everything(),
        Capabilities::new(CapabilitiesSpec::everything()).unwrap()
    );
    assert_eq!(
        Capabilities::nothing(),
        Capabilities::new(CapabilitiesSpec::nothing()).unwrap()
    );
}

#[test]
fn given_every_spec_when_read_through_the_accessors_then_the_fields_are_as_stored() {
    for spec in every_spec() {
        let capabilities = Capabilities::normalized(spec);
        let stored = capabilities.spec();
        assert_eq!(capabilities.reasoning_levels(), &stored.reasoning_levels);
        assert_eq!(capabilities.image_media_types(), &stored.image_media_types);
        assert_eq!(capabilities.max_output_tokens(), stored.max_output_tokens);
        assert_eq!(capabilities.tools(), stored.tools);
        assert_eq!(capabilities.structured_output(), stored.structured_output);
        assert_eq!(
            capabilities.structured_output_with_tools(),
            stored.structured_output_with_tools
        );
        assert_eq!(capabilities.caching(), stored.caching);
        assert_eq!(capabilities.images(), stored.images);
    }
}

#[test]
fn given_the_three_booleans_when_combined_then_structured_output_with_tools_needs_all_three() {
    for bits in 0..8u8 {
        let (tools, structured, with_tools) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        let capabilities = caps_with(|spec| {
            spec.tools = tools;
            spec.structured_output = structured;
            spec.structured_output_with_tools = with_tools;
        });
        assert_eq!(
            capabilities.accepts_structured_output_with_tools(),
            tools && structured && with_tools,
            "{tools} {structured} {with_tools}"
        );
        assert_eq!(capabilities.structured_output_with_tools(), with_tools);
    }
}

#[test]
fn given_a_capabilities_when_cloned_compared_and_shown_then_equality_follows_every_field() {
    let capabilities = Capabilities::nothing();
    assert_eq!(capabilities.clone(), capabilities);
    assert_ne!(capabilities, Capabilities::everything());
    assert_ne!(
        capabilities,
        caps_with(|spec| {
            *spec = CapabilitiesSpec {
                caching: true,
                ..CapabilitiesSpec::nothing()
            }
        })
    );
    let shown = format!("{capabilities:?}");
    assert!(shown.contains("Capabilities"));
    assert!(shown.contains("max_output_tokens"));
    assert!(shown.contains("image_media_types"));
}

#[test]
fn given_a_capabilities_when_compiled_then_it_has_no_default_value() {
    <Capabilities as AmbiguousIfDefault<_>>::proof();
}

#[test]
fn given_the_error_when_shown_then_the_text_is_exact() {
    assert_eq!(
        CapabilitiesError::InconsistentCapabilities.to_string(),
        "images are supported but the set of image media types is empty"
    );
}

#[test]
fn given_the_error_when_used_then_it_is_a_std_error_with_debug_clone_and_equality() {
    let error = CapabilitiesError::InconsistentCapabilities;
    let as_error: &dyn std::error::Error = &error;
    assert!(as_error.source().is_none());
    assert_eq!(error.clone(), error);
    assert!(format!("{error:?}").contains("InconsistentCapabilities"));
}

#[test]
fn given_the_image_media_types_when_all_are_listed_then_new_accepts_a_spec_that_takes_images() {
    let spec = CapabilitiesSpec {
        image_media_types: Accepted::Only(ALL_MIMES.to_vec()),
        ..CapabilitiesSpec::everything()
    };
    assert!(Capabilities::new(spec).is_ok());
}
