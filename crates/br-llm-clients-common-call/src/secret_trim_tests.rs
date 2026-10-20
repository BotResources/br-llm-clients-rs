use br_llm_clients_common_http::Secret;

use crate::text_clean::Secrets;

fn secrets(values: &[&str]) -> Secrets {
    Secrets::new(
        values
            .iter()
            .map(|value| Secret::new(*value).expect("secret"))
            .collect(),
    )
}

fn trimmed(values: &[&str], bytes: &[u8]) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    secrets(values).trim_cut(&mut bytes);
    bytes
}

#[test]
fn given_a_suffix_that_starts_a_secret_when_trimmed_then_the_longest_start_is_removed() {
    let secret = "QZXVsecret";
    for start in 1..secret.len() {
        let mut bytes = b"aaaa".to_vec();
        bytes.extend_from_slice(&secret.as_bytes()[..start]);
        assert_eq!(
            trimmed(&[secret], &bytes),
            b"aaaa",
            "start of {start} bytes"
        );
    }
}

#[test]
fn given_a_whole_secret_at_the_end_when_trimmed_then_it_stays_for_the_replacement() {
    assert_eq!(trimmed(&["QZXV"], b"aaaaQZXV"), b"aaaaQZXV");
}

#[test]
fn given_an_end_that_starts_no_secret_when_trimmed_then_nothing_is_removed() {
    assert_eq!(trimmed(&["QZXVsecret"], b"aaaaZX"), b"aaaaZX");
    assert_eq!(trimmed(&["QZXVsecret"], b"aaaaQZXsecre"), b"aaaaQZXsecre");
}

#[test]
fn given_the_shorter_secret_with_the_longer_suffix_when_trimmed_then_both_parts_go() {
    let longer = "QRzzzzzzzzzzz";
    let shorter = "ABQR1234";
    assert_eq!(trimmed(&[longer, shorter], b"aaaaABQR"), b"aaaa");
    assert_eq!(trimmed(&[shorter, longer], b"aaaaABQR"), b"aaaa");
}

#[test]
fn given_two_starts_that_overlap_when_trimmed_then_the_longer_start_goes() {
    let longer = "CDEFYYYYYYYY";
    let shorter = "ABCDXXXX";
    assert_eq!(trimmed(&[longer, shorter], b"aaaaABCD"), b"aaaa");
}

#[test]
fn given_a_body_that_is_only_a_start_of_a_secret_when_trimmed_then_it_becomes_empty() {
    assert_eq!(trimmed(&["QZXVsecret"], b"QZX"), b"");
    assert_eq!(trimmed(&["QZ"], b"QZ"), b"QZ");
}

#[test]
fn given_no_secret_when_trimmed_then_nothing_is_removed() {
    assert_eq!(trimmed(&[], b"aaaaQZ"), b"aaaaQZ");
}

#[test]
fn given_a_cut_inside_a_multibyte_secret_when_trimmed_then_the_partial_character_goes() {
    let secret = "pre-€-post";
    let bytes = [b"aaaa".as_slice(), &secret.as_bytes()[..5]].concat();
    assert_eq!(trimmed(&[secret], &bytes), b"aaaa");
}
