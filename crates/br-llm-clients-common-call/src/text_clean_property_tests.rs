use crate::test_secret_reference::{
    assemble, components, occurrences, secrets_of, sets_of_up_to_three, words,
};

fn check_replace(set: &[Vec<u8>], text: &[u8]) {
    let secrets = secrets_of(set);
    let readable = std::str::from_utf8(text).expect("utf8");
    let ranges = secrets.ranges(readable);
    let found = occurrences(text, set);
    let label = format!("{set:?} {readable:?}");
    for occurrence in &found {
        assert!(
            ranges
                .iter()
                .any(|range| range.start <= occurrence.start && occurrence.end <= range.end),
            "{label}: {occurrence:?} is not inside one range of {ranges:?}"
        );
    }
    for pair in ranges.windows(2) {
        assert!(pair[0].end <= pair[1].start, "{label}: {ranges:?}");
    }
    for range in &ranges {
        assert!(
            found
                .iter()
                .any(|occurrence| occurrence.start == range.start)
                && found.iter().any(|occurrence| occurrence.end == range.end),
            "{label}: {range:?} is wider than its occurrences"
        );
        for inside in range.start + 1..range.end {
            assert!(
                found.iter().any(|occurrence| occurrence.start < inside
                    && inside < occurrence.end
                    && range.start <= occurrence.start
                    && occurrence.end <= range.end),
                "{label}: {range:?} splits at {inside}"
            );
        }
    }
    assert_eq!(ranges, components(text, set), "{label}");
    assert_eq!(
        secrets.replace(readable).as_bytes(),
        assemble(text, &ranges),
        "{label}"
    );
}

fn sweep(letters: &[u8], longest_secret: usize, longest_text: usize) -> usize {
    let texts = words(letters, 0, longest_text);
    let mut checked = 0;
    for set in sets_of_up_to_three(&words(letters, 1, longest_secret)) {
        for text in &texts {
            check_replace(&set, text);
            checked += 1;
        }
    }
    checked
}

#[test]
fn given_every_set_of_up_to_three_secrets_and_every_short_text_when_replaced_then_every_occurrence_is_inside_one_placeholder_per_merged_range()
 {
    assert!(sweep(b"xy", 3, 7) > 100_000);
}

#[test]
fn given_secrets_made_of_placeholder_letters_when_replaced_then_the_placeholders_written_in_the_pass_are_not_scanned()
 {
    assert!(sweep(b"rd]", 2, 5) > 100_000);
}
