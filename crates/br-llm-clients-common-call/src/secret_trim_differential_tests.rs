use crate::test_secret_reference::{check_trim, secret_tails};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).expect("small")
    }

    fn word(&mut self, letters: &[u8], min: usize, max: usize) -> Vec<u8> {
        let length = min + self.below(max - min + 1);
        (0..length)
            .map(|_| letters[self.below(letters.len())])
            .collect()
    }
}

fn random_set(rng: &mut Rng, letters: &[u8]) -> Vec<Vec<u8>> {
    let count = 1 + rng.below(4);
    let mut set: Vec<Vec<u8>> = (0..count).map(|_| rng.word(letters, 1, 5)).collect();
    if rng.below(2) == 0 {
        let mut longer = set[0].clone();
        longer.extend(rng.word(letters, 1, 3));
        set.push(longer);
    }
    if rng.below(4) == 0 && set[0].len() > 1 {
        let mut overlap = set[0][1..].to_vec();
        overlap.extend(rng.word(letters, 0, 2));
        set.push(overlap);
    }
    set.sort();
    set.dedup();
    set
}

fn every_cut(set: &[Vec<u8>], body: &[u8]) -> (usize, usize) {
    let tails = secret_tails(set);
    let mut below_literal = 0;
    for end in 0..=body.len() {
        let (kept, literal) = check_trim(set, &body[..end], &tails);
        if kept < literal {
            below_literal += 1;
        }
    }
    (body.len() + 1, below_literal)
}

#[test]
fn given_random_secret_sets_and_bodies_when_every_cut_is_trimmed_then_the_trim_matches_the_reference()
 {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut checked = 0;
    let mut below_literal = 0;
    for round in 0..4_000 {
        let letters: &[u8] = match round % 3 {
            0 => b"xyzw",
            1 => b"xyz",
            _ => b"xed]",
        };
        let set = random_set(&mut rng, letters);
        let body = rng.word(letters, 0, 12);
        let (cases, below) = every_cut(&set, &body);
        checked += cases;
        below_literal += below;
    }
    assert!(checked > 20_000, "{checked}");
    assert!(below_literal > 0, "{below_literal}");
}

#[test]
fn given_secrets_made_of_the_placeholder_when_every_cut_is_trimmed_then_the_trim_matches_the_reference()
 {
    let sets: Vec<Vec<Vec<u8>>> = vec![
        vec![b"[redacted]".to_vec()],
        vec![b"[redacted]x".to_vec(), b"x".to_vec()],
        vec![b"red".to_vec(), b"[".to_vec()],
        vec![b"d]".to_vec(), b"x".to_vec()],
        ["r", "e", "d", "a", "c", "t"]
            .iter()
            .map(|letter| letter.as_bytes().to_vec())
            .collect(),
    ];
    let mut rng = Rng(7);
    let mut checked = 0;
    for set in &sets {
        for _ in 0..300 {
            checked += every_cut(set, &rng.word(b"[redact]x", 0, 12)).0;
        }
        checked += every_cut(set, b"[redacted][redacted]x").0;
    }
    assert!(checked > 10_000, "{checked}");
}
