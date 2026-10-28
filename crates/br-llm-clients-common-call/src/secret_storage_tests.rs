use br_llm_clients_common_http::Secret;

use crate::secret_automaton::reserved;
use crate::text_clean::Secrets;

const PREFIX: &str = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.";

fn token(seed: usize, size: usize) -> String {
    let letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut state =
        0x9E37_79B9_7F4A_7C15_u64 ^ (seed as u64 + 1).wrapping_mul(0x2545_F491_4F6C_DD1D);
    let mut text = PREFIX.to_owned();
    while text.len() < size {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        text.push(char::from(letters[(state % 64) as usize]));
    }
    text
}

fn secrets(values: &[&str], node_limit: usize) -> Secrets {
    Secrets::with_node_limit(
        values
            .iter()
            .map(|value| Secret::new(*value).expect("secret"))
            .collect(),
        node_limit,
    )
}

#[test]
fn given_1024_secrets_of_2_kib_with_a_shared_prefix_when_built_then_the_storage_is_nine_bytes_per_secret_byte()
 {
    let tokens: Vec<String> = (0..1024).map(|seed| token(seed, 2048)).collect();
    let total: usize = tokens.iter().map(String::len).sum();
    let secrets = Secrets::new(
        tokens
            .iter()
            .map(|value| Secret::new(value.as_str()).expect("secret"))
            .collect(),
    );
    let text = format!("{PREFIX} and {} and {PREFIX}", tokens[700]);
    assert_eq!(
        secrets.replace(&text),
        format!("{PREFIX} and [redacted] and {PREFIX}")
    );
    let storage = secrets.built().expect("built").storage_bytes();
    let per_secret = 2 * (size_of::<(u32, u8, u32)>() + size_of::<(u32, u32)>());
    let fixed = 256 * size_of::<u32>() + tokens.len() * per_secret;
    assert!(storage <= 9 * (total + 1) + fixed, "{storage} for {total}");
    assert!(storage <= 12 * total, "{storage} for {total}");
}

#[test]
fn given_secrets_that_fit_the_node_limit_when_cleaned_then_they_are_replaced() {
    let fitting = secrets(&["abc", "de"], 6);
    assert_eq!(fitting.replace("xabcx de"), "x[redacted]x [redacted]");
    let mut bytes = b"xxab".to_vec();
    fitting.trim_cut(&mut bytes);
    assert_eq!(bytes, b"xx");
}

#[test]
fn given_secrets_over_the_node_limit_when_cleaned_then_every_text_is_wholly_redacted() {
    let over = secrets(&["abc", "de"], 5);
    assert_eq!(over.replace("xabcx de"), "[redacted]");
    assert_eq!(over.replace("plain"), "[redacted]");
    assert_eq!(over.replace(""), "");
    let mut bytes = b"plain".to_vec();
    over.trim_cut(&mut bytes);
    assert!(bytes.is_empty());
    assert!(over.built().is_none());
}

#[test]
fn given_a_reservation_that_cannot_be_made_when_reserved_then_it_is_refused_without_a_panic() {
    assert!(reserved::<u32>(usize::MAX).is_none());
    assert!(reserved::<u32>(4).is_some_and(|reserved| reserved.capacity() >= 4));
}
