use crate::session_id::{SessionId, SessionIdError};

#[test]
fn given_a_uuid_when_made_then_it_is_kept_as_is() {
    let id = SessionId::new("0190a0b1-0000-7000-8000-000000000001").unwrap();
    assert_eq!(id.as_str(), "0190a0b1-0000-7000-8000-000000000001");
    assert_eq!(id.to_string(), "0190a0b1-0000-7000-8000-000000000001");
}

#[test]
fn given_an_inner_space_when_made_then_it_is_accepted() {
    assert!(SessionId::new("docs/a file.md").is_ok());
}

#[test]
fn given_an_empty_text_when_made_then_it_is_refused() {
    assert_eq!(SessionId::new(""), Err(SessionIdError::Empty));
}

#[test]
fn given_256_bytes_when_made_then_it_is_accepted_and_257_is_refused() {
    assert!(SessionId::new("a".repeat(256)).is_ok());
    assert_eq!(
        SessionId::new("a".repeat(257)),
        Err(SessionIdError::TooLong)
    );
}

#[test]
fn given_an_outer_space_when_made_then_it_is_refused() {
    assert_eq!(SessionId::new(" a"), Err(SessionIdError::OuterSpace));
    assert_eq!(SessionId::new("a "), Err(SessionIdError::OuterSpace));
}

#[test]
fn given_a_control_or_non_ascii_byte_when_made_then_its_index_is_given() {
    assert_eq!(
        SessionId::new("ab\r\nx"),
        Err(SessionIdError::InvalidByte { index: 2 })
    );
    assert_eq!(
        SessionId::new("a\u{e9}"),
        Err(SessionIdError::InvalidByte { index: 1 })
    );
    assert_eq!(
        SessionId::new("a\tb"),
        Err(SessionIdError::InvalidByte { index: 1 })
    );
}

#[test]
fn given_an_error_when_shown_then_it_does_not_hold_the_value() {
    let error = SessionId::new("secret\u{7f}").unwrap_err();
    assert!(!error.to_string().contains("secret"));
}
