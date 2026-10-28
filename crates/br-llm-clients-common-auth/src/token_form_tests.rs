use br_llm_clients_common_http::Secret;

use crate::error::AuthConfigError;
use crate::form_body::encode;
use crate::token_form::TokenForm;

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

#[test]
fn given_an_empty_name_when_added_then_the_form_is_refused() {
    let plain = TokenForm::new().plain("", "VALUE-1").expect_err("plain");
    let hidden = TokenForm::new()
        .secret("", secret("VALUE-2"))
        .expect_err("secret");
    for error in [plain, hidden] {
        assert!(matches!(error, AuthConfigError::InvalidFormField { .. }));
        assert!(!error.to_string().contains("VALUE"));
    }
}

#[test]
fn given_a_name_twice_when_added_then_the_form_is_refused_whatever_the_kind() {
    let form = TokenForm::new().plain("name", "VALUE-1").expect("first");
    let again = form.clone().plain("name", "VALUE-2").expect_err("plain");
    let hidden = form.secret("name", secret("VALUE-3")).expect_err("secret");
    for error in [again, hidden] {
        assert!(matches!(error, AuthConfigError::DuplicateFormField { .. }));
        assert!(error.to_string().contains("`name`"));
        assert!(!error.to_string().contains("VALUE"));
    }
}

#[test]
fn given_names_that_differ_by_case_then_they_are_two_fields() {
    let form = TokenForm::new()
        .plain("a", "1")
        .expect("a")
        .plain("A", "2")
        .expect("A");
    assert_eq!(encode(&form), "a=1&A=2");
}

fn duplicate_text(name: &str) -> String {
    TokenForm::new()
        .plain(name, "v")
        .and_then(|form| form.plain(name, "w"))
        .expect_err("duplicate")
        .to_string()
}

#[test]
fn given_a_name_with_control_characters_when_refused_then_the_text_shows_them_escaped() {
    let text = duplicate_text(&format!("a\nb{}", "n".repeat(100)));
    assert!(text.contains("a\\nb"));
    assert!(!text.contains('\n'));
}

#[test]
fn given_a_name_of_64_characters_when_refused_then_the_text_is_not_cut() {
    let name = "n".repeat(64);
    assert_eq!(
        duplicate_text(&name),
        format!("duplicate form field: the form field `{name}` is given twice")
    );
}

#[test]
fn given_a_name_of_65_characters_when_refused_then_the_text_is_cut_at_64() {
    let name = "n".repeat(65);
    assert_eq!(
        duplicate_text(&name),
        format!(
            "duplicate form field: the form field `{}...` is given twice",
            "n".repeat(64)
        )
    );
}

#[test]
fn given_an_empty_client_id_when_building_client_credentials_then_the_form_is_refused() {
    let error = TokenForm::client_credentials("", secret("sec")).expect_err("empty");
    assert!(matches!(error, AuthConfigError::InvalidFormField { .. }));
    assert!(error.to_string().contains("client_id"));
}

#[test]
fn given_a_ready_made_form_when_extended_then_a_taken_name_is_refused_and_a_new_one_added() {
    let form = TokenForm::client_credentials("id", secret("sec")).expect("form");
    assert!(matches!(
        form.clone().plain("grant_type", "x"),
        Err(AuthConfigError::DuplicateFormField { .. })
    ));
    let extended = form.plain("scope", "s").expect("scope");
    assert!(encode(&extended).ends_with("&scope=s"));
}

#[test]
fn given_a_form_then_secrets_gives_its_secret_values_only() {
    let form = TokenForm::client_credentials("PUBLIC-ID", secret("sec-1"))
        .expect("form")
        .secret("extra", secret("sec-2"))
        .expect("extra")
        .plain("scope", "PLAIN");
    let values: Vec<String> = form
        .expect("scope")
        .secrets()
        .iter()
        .map(|secret| secret.expose().to_owned())
        .collect();
    assert_eq!(values, vec!["sec-1", "sec-2"]);
    assert!(TokenForm::new().secrets().is_empty());
}
