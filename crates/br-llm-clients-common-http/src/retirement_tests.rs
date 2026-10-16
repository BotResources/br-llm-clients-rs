use super::Retirement;

#[test]
fn given_a_live_connection_then_it_is_neither_retired_nor_ended() {
    let retirement = Retirement::new();
    assert!(!retirement.is_retired());
    assert!(!retirement.is_ended());
}

#[test]
fn given_a_connection_that_ended_then_it_is_retired_and_ended() {
    let retirement = Retirement::new();
    retirement.end();
    assert!(retirement.is_retired());
    assert!(retirement.is_ended());
}

#[test]
fn given_a_connection_that_was_trimmed_then_it_is_retired_and_not_ended() {
    let retirement = Retirement::new();
    retirement.trim();
    assert!(retirement.is_retired());
    assert!(!retirement.is_ended());
}

#[test]
fn given_a_connection_that_was_trimmed_when_it_then_ends_then_it_stays_trimmed() {
    let retirement = Retirement::new();
    retirement.trim();
    retirement.end();
    assert!(!retirement.is_ended());
}

#[test]
fn given_a_connection_that_ended_when_it_is_then_trimmed_then_it_stays_ended() {
    let retirement = Retirement::new();
    retirement.end();
    retirement.trim();
    assert!(retirement.is_ended());
}
