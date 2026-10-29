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

#[test]
fn given_a_start_that_repeats_inside_itself_when_trimmed_then_the_longest_start_is_removed() {
    assert_eq!(trimmed(&["QZQZsecret"], b"aaaaQZQZ"), b"aaaa");
    assert_eq!(trimmed(&["QZQZsecret"], b"aaaaQZQ"), b"aaaa");
}

#[test]
fn given_two_secrets_when_the_longest_start_is_the_longer_ones_then_it_is_removed() {
    assert_eq!(
        trimmed(&["ABCDEFGHIJKL", "EFGHXXXX"], b"aaaaABCDEF"),
        b"aaaa"
    );
    assert_eq!(
        trimmed(&["EFGHXXXX", "ABCDEFGHIJKL"], b"aaaaABCDEF"),
        b"aaaa"
    );
}

#[test]
fn given_a_whole_secret_whose_last_byte_is_its_first_when_it_ends_the_bytes_then_it_goes_with_the_start()
 {
    let secret = "QZXVsecret7788Q";
    let bytes = [b"aaaa".as_slice(), secret.as_bytes()].concat();
    assert_eq!(trimmed(&[secret], &bytes), b"aaaa");
}

#[test]
fn given_a_whole_secret_whose_tail_starts_another_when_it_ends_the_bytes_then_both_go() {
    let first = "abcdefXYZ123";
    let second = "cdefXYZ123ghij";
    let bytes = [b"zzzz".as_slice(), first.as_bytes()].concat();
    assert_eq!(trimmed(&[first, second], &bytes), b"zzzz");
    assert_eq!(trimmed(&[second, first], &bytes), b"zzzz");
}

#[test]
fn given_a_whole_secret_then_a_start_when_trimmed_then_only_the_start_goes() {
    assert_eq!(trimmed(&["QZXV"], b"aaaaQZXVQZ"), b"aaaaQZXV");
}

#[test]
fn given_a_whole_secret_that_starts_a_longer_one_when_trimmed_then_it_goes_with_the_longer_start() {
    assert_eq!(trimmed(&["QZXV", "QZXVQ"], b"aaaaQZXV"), b"aaaa");
    assert_eq!(trimmed(&["QZXVQ", "QZXV"], b"aaaaQZXV"), b"aaaa");
}

#[test]
fn given_a_whole_secret_that_starts_a_longer_one_when_the_rest_continues_past_the_cut_then_no_byte_of_the_longer_one_stays()
 {
    let secrets = secrets(&["xy", "xyzw"]);
    let mut bytes = b"aaaaxyz".to_vec();
    secrets.trim_cut(&mut bytes);
    assert_eq!(
        secrets.replace(&String::from_utf8(bytes).expect("utf8")),
        "aaaa"
    );
}

#[test]
fn given_a_whole_secret_that_overlaps_a_secret_crossing_the_cut_when_trimmed_then_no_byte_of_either_stays()
 {
    assert_eq!(trimmed(&["abc", "bcdz"], b"0000abcd"), b"0000");
    assert_eq!(trimmed(&["bcdz", "abc"], b"0000abcd"), b"0000");
}

#[test]
fn given_a_secret_that_overlaps_itself_at_the_cut_when_trimmed_then_no_byte_of_it_stays() {
    assert_eq!(trimmed(&["aba"], b"abab"), b"");
    assert_eq!(trimmed(&["aa", "aba"], b"aab"), b"");
}

#[test]
fn given_short_secrets_that_cover_the_bytes_of_a_longer_one_when_trimmed_then_they_stay_for_the_replacement()
 {
    let secrets = secrets(&["x", "xxx"]);
    let mut bytes = b"xxxx".to_vec();
    secrets.trim_cut(&mut bytes);
    assert_eq!(bytes, b"xx");
    assert_eq!(
        secrets.replace(&String::from_utf8(bytes).expect("utf8")),
        "[redacted][redacted]"
    );
}

#[test]
fn given_one_byte_secrets_when_trimmed_then_nothing_is_removed() {
    assert_eq!(trimmed(&["k"], b"kkkk"), b"kkkk");
    assert_eq!(trimmed(&["k", "z"], b"akzk"), b"akzk");
}

#[test]
fn given_a_whole_secret_that_a_longer_cut_could_keep_safely_when_trimmed_then_the_suffix_rule_still_bounds_it()
 {
    assert_eq!(trimmed(&["xxx", "zxz", "zzx"], b"xxx"), b"");
}
