use crate::Accepted;
use crate::capabilities_test_support::AmbiguousIfDefault;

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
fn given_the_forms_when_compared_then_they_are_equal_only_when_the_same_form_and_list() {
    assert_eq!(Accepted::<Colour>::Any, Accepted::Any);
    assert_ne!(Accepted::<Colour>::Any, Accepted::Only(Vec::new()));
    assert_ne!(Accepted::Any, Accepted::Only(EVERY_COLOUR.to_vec()));
    assert_eq!(
        Accepted::Only(vec![Colour::Red]),
        Accepted::Only(vec![Colour::Red])
    );
    assert_ne!(
        Accepted::Only(vec![Colour::Red]),
        Accepted::Only(vec![Colour::Green])
    );
}

#[test]
fn given_an_accepted_when_cloned_and_shown_then_the_clone_is_equal_and_debug_names_the_form() {
    let accepted = Accepted::Only(vec![Colour::Green]);
    assert_eq!(accepted.clone(), accepted);
    assert!(format!("{accepted:?}").contains("Only"));
    assert!(format!("{:?}", Accepted::<Colour>::Any).contains("Any"));
}

fn assert_eq_bound<T: Eq>() {}

#[test]
fn given_a_type_with_eq_when_wrapped_then_accepted_is_eq() {
    assert_eq_bound::<Accepted<Colour>>();
}

#[test]
fn given_accepted_when_compiled_then_it_has_no_default_value() {
    <Accepted<Colour> as AmbiguousIfDefault<_>>::proof();
}
