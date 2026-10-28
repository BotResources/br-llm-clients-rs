use br_llm_clients_core::{CacheRequirement, ClientSettings, MaxOutputTokens, ReasoningLevel};
use br_llm_clients_mock::{
    Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec, MockClient,
};
use br_llm_messages::{MediaType, ModelName};

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
        CapabilitiesError::InconsistentCapabilities.to_string()
    );
}

#[test]
fn given_a_spec_written_field_by_field_when_built_then_the_mock_accepts_it() {
    let spec = CapabilitiesSpec {
        reasoning_levels: Accepted::Only(vec![ReasoningLevel::Off, ReasoningLevel::Low]),
        max_output_tokens: MaxOutputTokens::new(10).unwrap(),
        tools: false,
        structured_output: true,
        structured_output_with_tools: false,
        caching: false,
        images: true,
        image_media_types: Accepted::Only(vec![MediaType::Png]),
    };
    let partial = CapabilitiesSpec {
        tools: false,
        ..CapabilitiesSpec::everything()
    };
    assert_ne!(spec, partial);
    let built: Result<Capabilities, CapabilitiesError> = Capabilities::new(spec);
    assert!(built.is_ok());
}

#[test]
fn given_images_without_media_types_when_built_then_the_error_is_inconsistent_capabilities() {
    let spec = CapabilitiesSpec {
        images: true,
        image_media_types: Accepted::Only(vec![]),
        ..CapabilitiesSpec::everything()
    };
    assert_eq!(
        Capabilities::new(spec).unwrap_err(),
        CapabilitiesError::InconsistentCapabilities
    );
}

#[test]
fn given_the_operations_of_core_when_called_through_the_mock_then_they_are_available() {
    let everything = Capabilities::new(CapabilitiesSpec::everything()).unwrap();
    assert!(everything.tools());
    assert_eq!(everything.spec(), &CapabilitiesSpec::everything());
    assert_eq!(everything.reasoning_levels(), &Accepted::Any);
    assert_eq!(everything.image_media_types(), &Accepted::Any);
    assert_eq!(everything.max_output_tokens().get(), u32::MAX);
    assert!(everything.structured_output());
    assert!(everything.structured_output_with_tools());
    assert!(everything.images());
    assert!(everything.caching());
}

fn settings() -> ClientSettings {
    ClientSettings::new(
        ModelName::new("model").unwrap(),
        ReasoningLevel::Off,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(10).unwrap(),
    )
}
