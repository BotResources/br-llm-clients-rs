use crate::CapabilitiesError;
use crate::capabilities_test_support::assert_plain_data;

#[test]
fn given_the_error_when_shown_then_the_text_is_exact() {
    assert_eq!(
        CapabilitiesError::InconsistentCapabilities.to_string(),
        "images are supported but the set of image media types is empty"
    );
}

fn assert_std_error<T: std::error::Error>() {}

#[test]
fn given_the_error_when_compiled_then_it_is_a_std_error_and_plain_data() {
    assert_std_error::<CapabilitiesError>();
    assert_plain_data::<CapabilitiesError>();
}
