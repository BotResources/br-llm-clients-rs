use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bytes::Bytes;
use hyper::body::Body;

use crate::payload::Payload;

fn payload(bytes: &'static [u8]) -> (Payload, Arc<AtomicBool>) {
    let taken = Arc::new(AtomicBool::new(false));
    (
        Payload::new(Bytes::from_static(bytes), Arc::clone(&taken)),
        taken,
    )
}

#[test]
fn given_a_body_nobody_asked_about_then_the_request_is_not_marked_written() {
    let (_payload, taken) = payload(b"abc");
    assert!(!taken.load(Ordering::SeqCst));
}

#[test]
fn given_the_size_of_a_body_asked_when_the_library_takes_the_request_then_it_counts_as_written() {
    for bytes in [&b"abc"[..], &b""[..]] {
        let (payload, taken) = payload(bytes);
        let hint = payload.size_hint();
        assert_eq!(hint.exact(), Some(bytes.len() as u64));
        assert!(taken.load(Ordering::SeqCst), "{} bytes", bytes.len());
    }
}

#[test]
fn given_the_end_of_a_body_asked_when_the_library_takes_the_request_then_it_counts_as_written() {
    for bytes in [&b"abc"[..], &b""[..]] {
        let (payload, taken) = payload(bytes);
        assert_eq!(payload.is_end_stream(), bytes.is_empty());
        assert!(taken.load(Ordering::SeqCst), "{} bytes", bytes.len());
    }
}
