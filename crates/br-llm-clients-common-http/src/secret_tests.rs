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

#[test]
fn given_a_text_holding_the_value_when_asked_then_the_secret_appears() {
    let secret = Secret::new("sk-abc").expect("secret");
    assert!(secret.appears_in("sk-abc"));
    assert!(secret.appears_in("bad header sk-abc here"));
    assert!(secret.appears_in("Bearer sk-abc"));
    assert!(secret.appears_in("sk-abcsk-abc"));
}

#[test]
fn given_a_part_a_changed_case_or_a_re_encoded_value_when_asked_then_it_does_not_appear() {
    let secret = Secret::new("sk-abc+/=").expect("secret");
    assert!(!secret.appears_in("sk-ab"));
    assert!(!secret.appears_in("k-abc+/="));
    assert!(!secret.appears_in("SK-ABC+/="));
    assert!(!secret.appears_in("sk-abc%2B%2F%3D"));
    assert!(!secret.appears_in("sk-abc+/"));
    assert!(!secret.appears_in("sk- abc+/="));
    assert!(!secret.appears_in("c2stYWJjKy89"));
}

#[test]
fn given_an_empty_text_when_asked_then_the_secret_does_not_appear() {
    assert!(!Secret::new("sk-1").expect("secret").appears_in(""));
    assert!(!Secret::new(" ").expect("secret").appears_in(""));
}

#[test]
fn given_a_non_ascii_value_when_asked_then_the_match_is_exact_on_bytes() {
    let secret = Secret::new("clé-ü").expect("secret");
    assert!(secret.appears_in("x clé-ü y"));
    assert!(!secret.appears_in("x cle-u y"));
    assert!(!secret.appears_in("x clé-Ü y"));
}

#[test]
fn given_a_text_cleaned_by_the_second_barrier_when_asked_then_the_secret_no_longer_appears() {
    let secret = Secret::new("sk-9").expect("secret");
    let text = "failed with sk-9 in it";
    let cleaned = crate::redact::Redactor::new([secret.expose()]).clean(text);
    assert!(secret.appears_in(text));
    assert!(!secret.appears_in(&cleaned));
}
