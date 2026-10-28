use br_llm_clients_common_http::Secret;

use crate::form_body::{encode, percent_encode};
use crate::token_form::TokenForm;

fn encoded(text: &str) -> String {
    let mut out = String::new();
    percent_encode(text, &mut out);
    out
}

fn secret(value: &str) -> Secret {
    Secret::new(value).expect("secret")
}

#[test]
fn given_unreserved_characters_when_encoded_then_they_stay() {
    assert_eq!(encoded("AZaz09-._~"), "AZaz09-._~");
}

#[test]
fn given_every_other_ascii_character_when_encoded_then_each_byte_is_percent_and_upper_hex() {
    for byte in 0u8..128 {
        let character = char::from(byte);
        if character.is_ascii_alphanumeric() || "-._~".contains(character) {
            continue;
        }
        let text = character.to_string();
        assert_eq!(encoded(&text), format!("%{byte:02X}"), "byte {byte}");
    }
}

#[test]
fn given_the_vectors_that_trip_encoders_then_they_are_exact() {
    assert_eq!(encoded(" "), "%20");
    assert_eq!(encoded("*"), "%2A");
    assert_eq!(encoded("+"), "%2B");
    assert_eq!(encoded("&"), "%26");
    assert_eq!(encoded("="), "%3D");
    assert_eq!(encoded("%"), "%25");
    assert_eq!(encoded("\u{e9}"), "%C3%A9");
    assert_eq!(encoded("\u{1F512}"), "%F0%9F%94%92");
    assert_eq!(encoded(""), "");
}

#[test]
fn given_a_form_when_encoded_then_the_fields_are_in_the_order_of_the_calls_joined_by_and() {
    let form = TokenForm::new()
        .plain("b", "2")
        .expect("b")
        .secret("a", secret("1 1"))
        .expect("a")
        .plain("c", "")
        .expect("c");
    assert_eq!(encode(&form), "b=2&a=1%201&c=");
    assert_eq!(encode(&TokenForm::new()), "");
}

#[test]
fn given_the_ready_made_forms_then_the_fields_and_their_order_are_fixed() {
    assert_eq!(
        encode(&TokenForm::client_credentials("id", secret("sec")).expect("form")),
        "grant_type=client_credentials&client_id=id&client_secret=sec"
    );
    assert_eq!(
        encode(&TokenForm::jwt_bearer(secret("a.b.c"))),
        "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=a.b.c"
    );
    assert_eq!(
        encode(&TokenForm::refresh_token(secret("rt"))),
        "grant_type=refresh_token&refresh_token=rt"
    );
}
