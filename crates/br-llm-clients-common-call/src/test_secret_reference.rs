use std::ops::Range;

use br_llm_clients_common_http::Secret;

use crate::text_clean::Secrets;

pub(crate) const PLACEHOLDER: &str = "[redacted]";
pub(crate) const NEUTRAL: u8 = b'q';

pub(crate) fn secrets_of(values: &[Vec<u8>]) -> Secrets {
    Secrets::new(
        values
            .iter()
            .map(|value| {
                Secret::new(String::from_utf8(value.clone()).expect("utf8")).expect("secret")
            })
            .collect(),
    )
}

pub(crate) fn words(letters: &[u8], min: usize, max: usize) -> Vec<Vec<u8>> {
    (min..=max)
        .flat_map(|length| words_of_length(letters, length))
        .collect()
}

pub(crate) fn words_of_length(letters: &[u8], length: usize) -> Vec<Vec<u8>> {
    let mut level = vec![Vec::new()];
    for _ in 0..length {
        level = level
            .iter()
            .flat_map(|word: &Vec<u8>| {
                letters.iter().map(move |letter| {
                    let mut next = word.clone();
                    next.push(*letter);
                    next
                })
            })
            .collect();
    }
    level
}

pub(crate) fn sets_of_up_to_three(pool: &[Vec<u8>]) -> Vec<Vec<Vec<u8>>> {
    let mut sets = Vec::new();
    for (first_index, first) in pool.iter().enumerate() {
        sets.push(vec![first.clone()]);
        for (second_index, second) in pool.iter().enumerate().skip(first_index + 1) {
            sets.push(vec![first.clone(), second.clone()]);
            for third in pool.iter().skip(second_index + 1) {
                sets.push(vec![first.clone(), second.clone(), third.clone()]);
            }
        }
    }
    sets
}

pub(crate) fn occurrences(text: &[u8], secrets: &[Vec<u8>]) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    for start in 0..text.len() {
        for secret in secrets {
            if text[start..].starts_with(secret) {
                found.push(start..start + secret.len());
            }
        }
    }
    found
}

pub(crate) fn components(text: &[u8], secrets: &[Vec<u8>]) -> Vec<Range<usize>> {
    let mut sorted = occurrences(text, secrets);
    sorted.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in sorted {
        match merged.last_mut() {
            Some(last) if range.start < last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

pub(crate) fn assemble(text: &[u8], ranges: &[Range<usize>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    for range in ranges {
        out.extend_from_slice(&text[at..range.start]);
        out.extend_from_slice(PLACEHOLDER.as_bytes());
        at = range.end;
    }
    out.extend_from_slice(&text[at..]);
    out
}

pub(crate) fn open_from(cut: &[u8], secrets: &[Vec<u8>]) -> usize {
    (0..cut.len())
        .find(|&from| {
            let rest = &cut[from..];
            secrets
                .iter()
                .any(|secret| secret.len() > rest.len() && secret.starts_with(rest))
        })
        .unwrap_or(cut.len())
}

pub(crate) fn all_tails(letters: &[u8], secrets: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut with_neutral = letters.to_vec();
    with_neutral.push(NEUTRAL);
    let longest = secrets.iter().map(Vec::len).max().unwrap_or(1);
    words_of_length(&with_neutral, longest - 1)
}

pub(crate) fn secret_tails(secrets: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut tails = vec![vec![NEUTRAL]];
    for secret in secrets {
        for split in 1..secret.len() {
            let mut tail = secret[split..].to_vec();
            tail.push(NEUTRAL);
            tails.push(tail);
        }
    }
    tails
}

pub(crate) fn reachable(cut: &[u8], secrets: &[Vec<u8>], tails: &[Vec<u8>]) -> Vec<bool> {
    let mut reached = vec![false; cut.len()];
    for tail in tails {
        let whole = [cut, tail.as_slice()].concat();
        for range in occurrences(&whole, secrets) {
            let end = range.end.min(cut.len());
            if range.start < end {
                reached[range.start..end]
                    .iter_mut()
                    .for_each(|byte| *byte = true);
            }
        }
    }
    reached
}

pub(crate) fn safe_to_keep(cut: &[u8], kept: usize, secrets: &[Vec<u8>], reached: &[bool]) -> bool {
    let mut covered = vec![false; kept];
    for range in occurrences(&cut[..kept], secrets) {
        covered[range].iter_mut().for_each(|byte| *byte = true);
    }
    (0..kept).all(|index| !reached[index] || covered[index])
}

pub(crate) fn best_kept(cut: &[u8], secrets: &[Vec<u8>], reached: &[bool]) -> usize {
    (0..=open_from(cut, secrets))
        .rev()
        .find(|&kept| safe_to_keep(cut, kept, secrets, reached))
        .unwrap_or(0)
}

pub(crate) fn trimmed(secrets: &Secrets, cut: &[u8]) -> Vec<u8> {
    let mut bytes = cut.to_vec();
    secrets.trim_cut(&mut bytes);
    bytes
}

pub(crate) fn check_trim(set: &[Vec<u8>], cut: &[u8], tails: &[Vec<u8>]) -> (usize, usize) {
    let secrets = secrets_of(set);
    let kept = trimmed(&secrets, cut);
    let label = || format!("{:?} {:?}", show_all(set), String::from_utf8_lossy(cut));
    assert!(cut.starts_with(&kept), "{}", label());
    let reached = reachable(cut, set, tails);
    let literal = open_from(cut, set);
    assert!(kept.len() <= literal, "{} keeps {}", label(), kept.len());
    assert!(
        safe_to_keep(cut, kept.len(), set, &reached),
        "{} leaks",
        label()
    );
    assert_eq!(kept.len(), best_kept(cut, set, &reached), "{}", label());
    let cleaned = secrets.replace(std::str::from_utf8(&kept).expect("utf8"));
    assert_eq!(
        cleaned.as_bytes(),
        assemble(&kept, &components(&kept, set)),
        "{}",
        label()
    );
    (kept.len(), literal)
}

fn show_all(set: &[Vec<u8>]) -> Vec<String> {
    set.iter()
        .map(|secret| String::from_utf8_lossy(secret).into_owned())
        .collect()
}
