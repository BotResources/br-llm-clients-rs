use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{AmbiguousIfDefault, assert_plain_data};
use crate::{Accepted, ReasoningLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Colour {
    Red,
    Green,
    Blue,
}

const EVERY_COLOUR: [Colour; 3] = [Colour::Red, Colour::Green, Colour::Blue];

#[test]
fn given_any_when_asked_then_every_value_is_accepted() {
    for colour in EVERY_COLOUR {
        assert!(Accepted::Any.accepts(&colour));
    }
}

#[test]
fn given_only_when_asked_then_a_listed_value_is_accepted_and_every_other_value_is_refused() {
    let accepted = Accepted::Only(vec![Colour::Red, Colour::Blue]);
    assert!(accepted.accepts(&Colour::Red));
    assert!(accepted.accepts(&Colour::Blue));
    assert!(!accepted.accepts(&Colour::Green));
}

#[test]
fn given_an_empty_only_when_asked_then_nothing_is_accepted() {
    let accepted: Accepted<Colour> = Accepted::Only(Vec::new());
    for colour in EVERY_COLOUR {
        assert!(!accepted.accepts(&colour));
    }
}

#[test]
fn given_the_forms_when_asked_if_empty_then_only_an_empty_only_is_an_empty_set() {
    assert!(!Accepted::<Colour>::Any.is_empty_set());
    assert!(Accepted::<Colour>::Only(Vec::new()).is_empty_set());
    assert!(!Accepted::Only(vec![Colour::Red]).is_empty_set());
    assert!(!Accepted::Only(vec![Colour::Red, Colour::Red]).is_empty_set());
}

#[test]
fn given_an_accepted_when_compiled_then_it_is_plain_data() {
    assert_plain_data::<Accepted<Colour>>();
    assert_plain_data::<Accepted<ReasoningLevel>>();
    assert_plain_data::<Accepted<ImageMime>>();
}

#[test]
fn given_an_accepted_when_cloned_then_the_clone_is_equal() {
    let accepted = Accepted::Only(vec![Colour::Green]);
    assert_eq!(accepted.clone(), accepted);
    assert_eq!(Accepted::<Colour>::Any.clone(), Accepted::Any);
}

#[test]
fn given_accepted_when_compiled_then_it_has_no_default_value() {
    <Accepted<Colour> as AmbiguousIfDefault<_>>::proof();
}
