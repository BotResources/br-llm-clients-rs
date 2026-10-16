use crate::authorizer::AuthorizedHeader;
use crate::config_error::ConfigError;
use crate::secret::Secret;

#[test]
fn given_an_empty_text_when_building_a_secret_then_it_is_refused() {
    assert!(matches!(
        Secret::new(""),
        Err(ConfigError::EmptySecret { .. })
    ));
}

#[test]
fn given_a_secret_when_formatting_then_debug_and_display_give_the_same_placeholder() {
    let secret = Secret::new("sk-very-secret").expect("secret");
    assert_eq!(format!("{secret:?}"), "[redacted]");
    assert_eq!(format!("{secret}"), "[redacted]");
    assert_eq!(format!("{secret:#?}"), "[redacted]");
}

#[test]
fn given_a_secret_when_exposed_then_the_value_comes_back() {
    assert_eq!(Secret::new("sk-1").expect("secret").expose(), "sk-1");
}

#[test]
fn given_a_clone_when_exposed_then_it_holds_the_same_value() {
    let secret = Secret::new("sk-1").expect("secret");
    assert_eq!(secret.clone().expose(), "sk-1");
}

#[test]
fn given_a_secret_inside_another_value_when_formatting_then_the_value_is_not_shown() {
    let header =
        AuthorizedHeader::new("authorization", Secret::new("Bearer sk-1").expect("secret"));
    let debug = format!("{header:?}");
    assert!(!debug.contains("sk-1"), "{debug}");
    assert!(debug.contains("authorization"));
}

#[test]
fn given_a_whitespace_secret_when_building_then_it_is_accepted_because_only_empty_is_refused() {
    assert!(Secret::new(" ").is_ok());
}
