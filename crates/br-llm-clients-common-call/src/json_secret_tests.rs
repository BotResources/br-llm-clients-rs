use br_llm_clients_common_http::Secret;

use crate::text_clean::Secrets;

const PASSWORD: &str = "p\"a\\s/s+w=o<r>d&'x`y";
const WIDE: &str = "\u{e9}\u{1f600}\n\u{8}\u{c}\r\t\u{1}\u{7f}\u{2028}/\"k";

struct Profile {
    escaped: &'static str,
    upper: bool,
    slash: bool,
    non_ascii: bool,
    quote_unicode: bool,
}

const SERDE: Profile = Profile {
    escaped: "",
    upper: false,
    slash: false,
    non_ascii: false,
    quote_unicode: false,
};
const JACKSON: Profile = Profile {
    upper: true,
    ..SERDE
};
const PHP: Profile = Profile {
    slash: true,
    non_ascii: true,
    ..SERDE
};
const PYTHON: Profile = Profile {
    non_ascii: true,
    ..SERDE
};
const GO: Profile = Profile {
    escaped: "<>&\u{2028}\u{2029}",
    ..SERDE
};
const GSON: Profile = Profile {
    escaped: "<>&='",
    ..SERDE
};
const DOTNET: Profile = Profile {
    escaped: "&'+<>`\u{7f}",
    upper: true,
    non_ascii: true,
    quote_unicode: true,
    ..SERDE
};
const PROFILES: [&Profile; 7] = [&SERDE, &JACKSON, &PHP, &PYTHON, &GO, &GSON, &DOTNET];

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

fn encode(text: &str, profile: &Profile) -> String {
    let mut out = String::new();
    for character in text.chars() {
        let written = match character {
            '"' if profile.quote_unicode => unicode(character, profile.upper),
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            '/' if profile.slash => "\\/".to_owned(),
            '\u{8}' => "\\b".to_owned(),
            '\u{c}' => "\\f".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            '\u{0}'..='\u{1f}' => unicode(character, profile.upper),
            _ if profile.escaped.contains(character) => unicode(character, profile.upper),
            _ if profile.non_ascii && !character.is_ascii() => unicode(character, profile.upper),
            _ => character.to_string(),
        };
        out.push_str(&written);
    }
    out
}

fn secrets(values: &[&str]) -> Secrets {
    Secrets::new(
        values
            .iter()
            .map(|value| Secret::new(*value).expect("secret"))
            .collect(),
    )
}

fn body(form: &str) -> String {
    format!("{{\"error\":{{\"message\":\"Incorrect API key provided: {form}.\"}}}}")
}

fn trimmed(cleaner: &Secrets, bytes: &[u8]) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    cleaner.trim_cut(&mut bytes);
    bytes
}

#[test]
fn given_each_known_encoder_once_twice_and_three_times_when_replaced_then_no_part_is_left() {
    for secret in [PASSWORD, WIDE] {
        let cleaner = secrets(&[secret]);
        for inner in PROFILES {
            let once = body(&encode(secret, inner));
            let clean = body("[redacted]");
            assert_eq!(cleaner.replace(&once), clean, "{once}");
            for outer in PROFILES {
                let twice = encode(&once, outer);
                assert_eq!(cleaner.replace(&twice), encode(&clean, outer), "{twice}");
                let thrice = encode(&twice, &SERDE);
                let expected = encode(&encode(&clean, outer), &SERDE);
                assert_eq!(cleaner.replace(&thrice), expected, "{thrice}");
            }
        }
    }
}

#[test]
fn given_encoders_that_mix_their_escapes_when_replaced_then_no_part_is_left() {
    let cases = [
        ("p<\u{e9}", "p\\u003C\\u00e9"),
        ("a<&b", "a\\u003c&b"),
        ("a\nb\u{8}", "a\\nb\\u0008"),
        ("a</b/c", "a<\\/b/c"),
        ("ab\"c<d", "ab\\\\\\\"c\\u003cd"),
        ("a\"b", "a\\\\\\u0022b"),
        ("ab/c+d", "ab\\\\/c\\u002Bd"),
        ("a\"b=c", "a\\\\\\\"b\\u003dc"),
    ];
    for (secret, text) in cases {
        let cleaner = secrets(&[secret]);
        assert_eq!(
            cleaner.replace(&format!("key {text} end")),
            "key [redacted] end",
            "{text}"
        );
    }
}

#[test]
fn given_a_refusal_cut_inside_an_escaped_secret_when_trimmed_then_no_byte_of_it_is_kept() {
    for secret in [PASSWORD, WIDE] {
        let cleaner = secrets(&[secret]);
        for profile in PROFILES {
            let once = encode(secret, profile);
            for form in [once.clone(), encode(&once, &SERDE)] {
                for end in 1..form.len() {
                    let mut bytes = b"pad ".to_vec();
                    bytes.extend_from_slice(&form.as_bytes()[..end]);
                    assert_eq!(trimmed(&cleaner, &bytes), b"pad ", "{form} cut at {end}");
                }
            }
        }
    }
}

#[test]
fn given_a_cut_body_that_ends_in_an_incomplete_escape_when_trimmed_then_the_escape_is_removed() {
    let cleaner = secrets(&["sk-1"]);
    let tails = [
        "\\",
        "\\u",
        "\\u00",
        "\\u00e",
        "\\ud83d",
        "\\ud83d\\",
        "\\ud83d\\ude0",
        "\\\\",
        "\\\\u00",
    ];
    for tail in tails {
        let bytes = format!("pad {tail}");
        assert_eq!(trimmed(&cleaner, bytes.as_bytes()), b"pad ", "{tail}");
    }
    for whole in ["pad \\q", "pad \\n", "pad \\u00e9", "pad \\udc00x"] {
        assert_eq!(
            trimmed(&cleaner, whole.as_bytes()),
            whole.as_bytes(),
            "{whole}"
        );
    }
    assert_eq!(cleaner.replace("pad \\u00"), "pad \\u00");
    assert_eq!(trimmed(&secrets(&[]), b"pad \\u00"), b"pad \\u00");
}

#[test]
fn given_a_whole_escaped_secret_before_the_cut_when_trimmed_then_it_is_kept_for_the_replacement() {
    let cleaner = secrets(&["a\"b"]);
    assert_eq!(trimmed(&cleaner, b"x a\\\"b y"), b"x a\\\"b y");
    assert_eq!(trimmed(&cleaner, b"x a\\\"b"), b"x a\\\"b");
    assert_eq!(trimmed(&cleaner, b"x a\\u0022b"), b"x a\\u0022b");
    assert_eq!(trimmed(&cleaner, b"x a\\u0022"), b"x ");
    assert_eq!(cleaner.replace("x a\\u0022b y"), "x [redacted] y");
}

#[test]
fn given_an_escaped_secret_in_a_text_when_tested_then_it_appears_in_it() {
    let cleaner = secrets(&["a\"b"]);
    assert!(cleaner.appears_in("req-a\\\"b"));
    assert!(cleaner.appears_in("req-a\\u0022b"));
    assert!(!cleaner.appears_in("req-a'b"));
    assert!(!cleaner.appears_in(""));
}

#[test]
fn given_a_secret_nested_four_times_when_replaced_then_it_is_beyond_the_three_decodings() {
    let cleaner = secrets(&["a\"b"]);
    let mut text = "a\"b".to_owned();
    for _ in 0..3 {
        text = encode(&text, &SERDE);
    }
    assert_eq!(cleaner.replace(&text), "[redacted]");
    let four = encode(&text, &SERDE);
    assert_eq!(cleaner.replace(&four), four);
}

#[test]
fn given_a_raw_secret_that_holds_a_backslash_when_replaced_then_raw_and_escaped_are_found() {
    let cleaner = secrets(&["a\\nb"]);
    assert_eq!(cleaner.replace("x a\\nb y"), "x [redacted] y");
    assert_eq!(cleaner.replace("x a\\\\nb y"), "x [redacted] y");
    assert_eq!(cleaner.replace("x a\nb y"), "x a\nb y");
    let mut bytes = b"x a\\n".to_vec();
    cleaner.trim_cut(&mut bytes);
    assert_eq!(bytes, b"x ");
}

#[test]
fn given_the_last_surrogate_pair_when_replaced_then_it_is_decoded() {
    let cleaner = secrets(&["\u{10ffff}x"]);
    assert_eq!(cleaner.replace("k \\udbff\\udfffx"), "k [redacted]");
    assert_eq!(cleaner.replace("k \\uDBFF\\uDFFFx"), "k [redacted]");
}

#[test]
fn given_an_incomplete_escape_at_the_end_of_a_decoded_text_when_replaced_then_the_next_decoding_reads_it()
 {
    let cleaner = secrets(&["\"\\u00"]);
    assert_eq!(cleaner.replace("\\\\u0022\\u00"), "[redacted]");
}

#[test]
fn given_occurrences_found_in_two_readings_that_touch_when_replaced_then_each_keeps_its_placeholder()
 {
    let cleaner = secrets(&["ab"]);
    assert_eq!(cleaner.replace("ab\\u0061b"), "[redacted][redacted]");
    assert_eq!(cleaner.replace("a\\u0062ab"), "[redacted][redacted]");
}
