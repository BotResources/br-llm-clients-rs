use super::Announced::{self, Awaiting, Usable, ZeroSinceFirst};

fn run(from: Announced, entries: &[Option<u32>]) -> Option<Announced> {
    from.after_run(entries)
}

#[test]
fn given_a_first_run_then_the_announcement_follows_the_rules_of_the_first_settings() {
    assert_eq!(run(Awaiting, &[Some(0)]), Some(ZeroSinceFirst));
    assert_eq!(run(Awaiting, &[Some(5)]), Some(Usable(5)));
    assert_eq!(run(Awaiting, &[None]), Some(Usable(usize::MAX)));
    assert_eq!(
        run(Awaiting, &[Some(u32::MAX)]),
        Some(Usable(4_294_967_295))
    );
}

#[test]
fn given_a_first_zero_then_a_raise_in_the_same_run_never_shows_the_zero() {
    assert_eq!(run(Awaiting, &[Some(0), Some(100)]), Some(Usable(100)));
    assert_eq!(run(Awaiting, &[Some(0), Some(3), Some(0)]), Some(Usable(0)));
    assert_eq!(run(Awaiting, &[Some(0), None]), Some(ZeroSinceFirst));
}

#[test]
fn given_a_usable_first_settings_then_a_zero_in_the_same_run_leaves_it_usable_and_full() {
    assert_eq!(run(Awaiting, &[Some(100), Some(0)]), Some(Usable(0)));
    assert_eq!(run(Awaiting, &[None, Some(0)]), Some(Usable(0)));
}

#[test]
fn given_a_first_zero_then_a_later_raise_makes_it_usable_and_a_later_zero_changes_nothing() {
    assert_eq!(run(ZeroSinceFirst, &[Some(3)]), Some(Usable(3)));
    assert_eq!(run(ZeroSinceFirst, &[Some(0)]), Some(ZeroSinceFirst));
    assert_eq!(run(ZeroSinceFirst, &[None]), None);
}

#[test]
fn given_a_usable_connection_then_a_later_zero_costs_nothing_and_a_missing_entry_keeps_the_limit() {
    assert_eq!(run(Usable(5), &[Some(0)]), Some(Usable(0)));
    assert_eq!(run(Usable(0), &[Some(7)]), Some(Usable(7)));
    assert_eq!(run(Usable(5), &[None]), None);
    assert_eq!(run(Usable(5), &[]), None);
}

#[test]
fn given_an_announcement_then_its_limit_reads_from_the_one_value() {
    assert_eq!(Awaiting.limit(), 0);
    assert_eq!(ZeroSinceFirst.limit(), 0);
    assert_eq!(Usable(4).limit(), 4);
}
