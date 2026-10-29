use br_llm_clients_common_http::Secret;

use crate::text_clean::Secrets;

const LETTERS: [char; 12] = [
    'a',
    'u',
    '0',
    'D',
    '"',
    '\\',
    '/',
    '<',
    '\n',
    '\u{1}',
    '\u{e9}',
    '\u{1f600}',
];

struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0 % bound as u64).expect("small")
    }
}

fn unicode(character: char, upper: bool) -> String {
    let mut units = [0_u16; 2];
    character
        .encode_utf16(&mut units)
        .iter()
        .map(|unit| {
            if upper {
                format!("\\u{unit:04X}")
            } else {
                format!("\\u{unit:04x}")
            }
        })
        .collect()
}

fn writings(character: char) -> Vec<String> {
    let mut all = vec![unicode(character, false), unicode(character, true)];
    match character {
        '"' | '\\' | '/' => all.push(format!("\\{character}")),
        '\n' => all.push("\\n".to_owned()),
        _ => {}
    }
    if !matches!(character, '"' | '\\' | '\u{0}'..='\u{1f}') {
        all.push(character.to_string());
    }
    all
}

fn encode(rng: &mut Rng, text: &str) -> String {
    text.chars()
        .map(|character| {
            let all = writings(character);
            all[rng.below(all.len())].clone()
        })
        .collect()
}

#[test]
fn given_random_secrets_written_with_mixed_escapes_up_to_three_times_when_cleaned_then_no_byte_is_left()
 {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut checked = 0;
    for _ in 0..600 {
        let length = 1 + rng.below(4);
        let secret: String = (0..length)
            .map(|_| LETTERS[rng.below(LETTERS.len())])
            .collect();
        let mut form = secret.clone();
        for _ in 0..1 + rng.below(3) {
            form = encode(&mut rng, &form);
        }
        let cleaner = Secrets::new(vec![Secret::new(secret.as_str()).expect("secret")]);
        let text = format!("#( {form} )#");
        assert_eq!(
            cleaner.replace(&text),
            "#( [redacted] )#",
            "{secret:?} {form}"
        );
        for end in 1..form.len() {
            let mut bytes = b"#( ".to_vec();
            bytes.extend_from_slice(&form.as_bytes()[..end]);
            cleaner.trim_cut(&mut bytes);
            assert_eq!(bytes, b"#( ", "{secret:?} {form} cut at {end}");
            checked += 1;
        }
    }
    assert!(checked > 50_000, "{checked}");
}
