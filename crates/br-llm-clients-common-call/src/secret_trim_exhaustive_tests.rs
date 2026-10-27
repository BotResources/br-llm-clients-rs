use br_llm_clients_common_http::Secret;

use crate::text_clean::Secrets;

const PLACEHOLDER: &str = "[redacted]";
const LETTERS: [char; 2] = ['x', 'y'];

fn words(min: usize, max: usize) -> Vec<String> {
    let mut all = Vec::new();
    let mut level = vec![String::new()];
    for length in 0..=max {
        if length >= min {
            all.extend(level.iter().cloned());
        }
        level = level
            .iter()
            .flat_map(|word| LETTERS.iter().map(move |letter| format!("{word}{letter}")))
            .collect();
    }
    all
}

fn longest_at(text: &str, at: usize, secrets: &[String]) -> Option<usize> {
    secrets
        .iter()
        .filter(|secret| text[at..].starts_with(secret.as_str()))
        .map(String::len)
        .max()
}

fn matches_of(text: &str, secrets: &[String]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < text.len() {
        match longest_at(text, at, secrets) {
            Some(length) => {
                found.push((at, at + length));
                at += length;
            }
            None => at += 1,
        }
    }
    found
}

fn reference_replace(text: &str, secrets: &[String]) -> String {
    let mut out = String::new();
    let mut at = 0;
    for (start, end) in matches_of(text, secrets) {
        out.push_str(&text[at..start]);
        out.push_str(PLACEHOLDER);
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

fn secrets_of(values: &[String]) -> Secrets {
    Secrets::new(
        values
            .iter()
            .map(|value| Secret::new(value.as_str()).expect("secret"))
            .collect(),
    )
}

fn secret_sets(pool: &[String]) -> Vec<Vec<String>> {
    let mut sets: Vec<Vec<String>> = pool.iter().map(|secret| vec![secret.clone()]).collect();
    for (index, first) in pool.iter().enumerate() {
        for second in &pool[index + 1..] {
            sets.push(vec![first.clone(), second.clone()]);
        }
    }
    sets
}

#[test]
fn given_every_small_body_and_secret_set_when_cut_at_every_position_then_no_part_of_a_crossing_secret_stays()
 {
    let pool = words(2, 4);
    let bodies = words(0, 7);
    let mut checked = 0_usize;
    for set in secret_sets(&pool) {
        let secrets = secrets_of(&set);
        for body in &bodies {
            let whole = matches_of(body, &set);
            for cut in 1..body.len() {
                let mut bytes = body.as_bytes()[..cut].to_vec();
                secrets.trim_cut(&mut bytes);
                let output = secrets.replace(&String::from_utf8(bytes).expect("utf8"));
                for secret in &set {
                    assert!(
                        !output.contains(secret.as_str()),
                        "{set:?} {body} {cut}: {output}"
                    );
                }
                let reference_end = whole
                    .iter()
                    .find(|(start, end)| *start < cut && cut < *end)
                    .map_or(cut, |(start, _)| *start);
                let allowed = reference_replace(&body[..reference_end], &set);
                assert!(
                    allowed.starts_with(&output),
                    "{set:?} {body} {cut}: {output} is not a start of {allowed}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 100_000, "{checked}");
}
