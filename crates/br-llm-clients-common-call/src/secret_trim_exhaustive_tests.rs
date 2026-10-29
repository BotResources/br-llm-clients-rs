use crate::test_secret_reference::{all_tails, check_trim, sets_of_up_to_three, words};

fn sweep(letters: &[u8], longest_secret: usize, longest_cut: usize) -> (usize, usize) {
    let pool = words(letters, 1, longest_secret);
    let cuts = words(letters, 0, longest_cut);
    let mut checked = 0_usize;
    let mut below_literal = 0_usize;
    for set in sets_of_up_to_three(&pool) {
        let tails = all_tails(letters, &set);
        for cut in &cuts {
            let (kept, literal) = check_trim(&set, cut, &tails);
            if kept < literal {
                below_literal += 1;
            }
            checked += 1;
        }
    }
    (checked, below_literal)
}

#[test]
fn given_every_set_of_up_to_three_secrets_over_two_letters_when_every_cut_is_trimmed_then_no_byte_of_any_occurrence_stays_and_the_most_is_kept()
 {
    let (checked, below_literal) = sweep(b"xy", 3, 7);
    assert!(checked > 100_000, "{checked}");
    assert!(below_literal > 0, "{below_literal}");
}

#[test]
fn given_every_set_of_up_to_three_secrets_over_three_letters_when_every_cut_is_trimmed_then_no_byte_of_any_occurrence_stays_and_the_most_is_kept()
 {
    let (checked, below_literal) = sweep(b"xyz", 2, 5);
    assert!(checked > 100_000, "{checked}");
    assert!(below_literal > 0, "{below_literal}");
}
