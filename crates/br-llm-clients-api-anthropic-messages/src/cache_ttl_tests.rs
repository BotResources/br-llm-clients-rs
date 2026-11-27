use std::collections::HashSet;

use crate::CacheTtl;

#[test]
fn given_each_cache_ttl_when_written_then_it_is_5m_or_1h() {
    assert_eq!(CacheTtl::FiveMinutes.as_str(), "5m");
    assert_eq!(CacheTtl::OneHour.as_str(), "1h");
}

#[test]
fn given_the_default_cache_ttl_when_read_then_it_is_five_minutes() {
    assert_eq!(CacheTtl::default(), CacheTtl::FiveMinutes);
}

#[test]
fn given_each_written_form_when_parsed_then_it_gives_its_cache_ttl() {
    for ttl in [CacheTtl::FiveMinutes, CacheTtl::OneHour] {
        assert_eq!(CacheTtl::parse(ttl.as_str()), Some(ttl));
    }
}

#[test]
fn given_a_text_that_is_not_exactly_a_written_form_when_parsed_then_it_gives_nothing() {
    for text in [
        "", " ", "5M", "1H", " 1h", "1h ", "5 m", "60m", "300s", "3600", "2h", "1d", "one hour",
        "\"1h\"",
    ] {
        assert_eq!(CacheTtl::parse(text), None, "{text:?}");
    }
}

#[test]
fn given_the_cache_ttls_when_hashed_and_copied_then_they_stay_distinct() {
    let ttl = CacheTtl::OneHour;
    let copy = ttl;
    assert_eq!(ttl, copy);
    let set: HashSet<CacheTtl> = [CacheTtl::FiveMinutes, CacheTtl::OneHour, copy].into();
    assert_eq!(set.len(), 2);
}
