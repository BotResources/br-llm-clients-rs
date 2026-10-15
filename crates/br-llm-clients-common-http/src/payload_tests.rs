use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bytes::Bytes;
use hyper::body::Body;

use crate::payload::Payload;

fn payload(bytes: &'static [u8]) -> (Payload, Arc<AtomicBool>) {
    let written = Arc::new(AtomicBool::new(false));
    (
        Payload::new(Bytes::from_static(bytes), Arc::clone(&written)),
        written,
    )
}

#[test]
fn given_a_body_nobody_asked_about_then_the_request_is_not_marked_written() {
    let (_payload, written) = payload(b"abc");
    assert!(!written.load(Ordering::SeqCst));
}

#[test]
fn given_the_size_of_a_body_asked_when_the_library_takes_the_request_then_it_counts_as_written() {
    for bytes in [&b"abc"[..], &b""[..]] {
        let (payload, written) = payload(bytes);
        let hint = payload.size_hint();
        assert_eq!(hint.exact(), Some(bytes.len() as u64));
        assert!(written.load(Ordering::SeqCst), "{} bytes", bytes.len());
    }
}

#[test]
fn given_the_end_of_a_body_asked_when_the_library_takes_the_request_then_it_counts_as_written() {
    for bytes in [&b"abc"[..], &b""[..]] {
        let (payload, written) = payload(bytes);
        assert_eq!(payload.is_end_stream(), bytes.is_empty());
        assert!(written.load(Ordering::SeqCst), "{} bytes", bytes.len());
    }
}
