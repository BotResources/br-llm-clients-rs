use br_llm_clients_core::{MaxOutputTokens, ReasoningLevel};
use br_llm_clients_mock::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, MockClient,
};
use br_llm_messages::ImageMime;

fn takes_core_capabilities(capabilities: br_llm_clients_core::Capabilities) -> bool {
    capabilities.tools()
}

fn takes_mock_capabilities(capabilities: Capabilities) -> bool {
    capabilities.caching()
}

fn takes_core_spec(spec: br_llm_clients_core::CapabilitiesSpec) -> bool {
    spec.tools
}

fn takes_core_accepted(accepted: br_llm_clients_core::Accepted<ReasoningLevel>) -> bool {
    matches!(accepted, br_llm_clients_core::Accepted::Any)
}

fn takes_core_error(error: br_llm_clients_core::CapabilitiesError) -> String {
    error.to_string()
}

#[test]
fn given_capabilities_built_through_the_mock_when_passed_to_core_then_it_is_the_same_type() {
    let through_mock = Capabilities::new(CapabilitiesSpec::everything()).unwrap();
    assert!(takes_core_capabilities(through_mock));
    let through_core =
        br_llm_clients_core::Capabilities::new(br_llm_clients_core::CapabilitiesSpec::everything())
            .unwrap();
    assert!(takes_mock_capabilities(through_core.clone()));
    let client = MockClient::new(settings(), through_core);
    assert_eq!(client.calls(), 0);
}

#[test]
fn given_the_other_reexported_types_when_passed_to_core_then_they_are_the_same_types() {
    assert!(takes_core_spec(CapabilitiesSpec::everything()));
    assert!(takes_core_accepted(Accepted::Any));
    assert_eq!(
        takes_core_error(CapabilitiesError::InconsistentCapabilities),
        "images are supported but the set of image media types is empty"
    );
}

#[test]
fn given_code_of_the_mock_0_4_when_compiled_then_the_listed_items_work_unchanged() {
    let levels = Accepted::Only(vec![ReasoningLevel::Off, ReasoningLevel::Low]);
    let described = match &levels {
        Accepted::Any => "any",
        Accepted::Only(_) => "only",
    };
    assert_eq!(described, "only");
    let spec = CapabilitiesSpec {
        reasoning_levels: levels.clone(),
        max_output_tokens: MaxOutputTokens::new(10).unwrap(),
        tools: false,
        structured_output: true,
        structured_output_with_tools: false,
        caching: false,
        images: true,
        image_media_types: Accepted::Only(vec![ImageMime::Png]),
    };
    let partial = CapabilitiesSpec {
        tools: false,
        ..CapabilitiesSpec::everything()
    };
    assert_ne!(spec, partial);
    let built: Result<Capabilities, CapabilitiesError> = Capabilities::new(spec);
    assert!(built.is_ok());
    plain_data::<CapabilitiesSpec>();
    plain_data::<Capabilities>();
    plain_data::<CapabilitiesError>();
    plain_data::<Accepted<ReasoningLevel>>();
}

fn plain_data<T: Clone + Eq + std::fmt::Debug>() {}

#[test]
fn given_images_without_media_types_when_built_then_the_error_has_its_one_case_and_its_text() {
    let spec = CapabilitiesSpec {
        images: true,
        image_media_types: Accepted::Only(vec![]),
        ..CapabilitiesSpec::everything()
    };
    let error = Capabilities::new(spec).unwrap_err();
    assert_eq!(error, CapabilitiesError::InconsistentCapabilities);
    assert_eq!(
        error.to_string(),
        "images are supported but the set of image media types is empty"
    );
    let CapabilitiesError::InconsistentCapabilities = error.clone();
    let source: &dyn std::error::Error = &error;
    assert!(source.source().is_none());
}

#[test]
fn given_the_operations_core_adds_when_called_through_the_mock_then_they_are_available() {
    assert!(!CapabilitiesSpec::nothing().tools);
    let one = std::num::NonZeroU32::MIN;
    assert_eq!(CapabilitiesSpec::max_output_tokens_of(one).get(), 1);
    let everything = Capabilities::everything();
    assert!(everything.tools());
    assert_eq!(everything.spec(), &CapabilitiesSpec::everything());
    assert!(!Capabilities::nothing().images());
    assert_eq!(
        Capabilities::normalized(CapabilitiesSpec::everything()),
        everything
    );
    assert_eq!(everything.reasoning_levels(), &Accepted::Any);
    assert_eq!(everything.image_media_types(), &Accepted::Any);
    assert_eq!(everything.max_output_tokens().get(), u32::MAX);
    assert!(everything.structured_output());
    assert!(everything.structured_output_with_tools());
    assert!(everything.accepts_structured_output_with_tools());
    assert!(everything.images());
    assert!(everything.caching());
}

fn settings() -> br_llm_clients_core::ClientSettings {
    br_llm_clients_core::ClientSettings::new(
        br_llm_messages::ModelId::new("model").unwrap(),
        ReasoningLevel::Off,
        br_llm_clients_core::CacheRequirement::NotRequired,
        MaxOutputTokens::new(10).unwrap(),
    )
}
