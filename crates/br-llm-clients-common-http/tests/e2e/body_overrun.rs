use br_llm_clients_common_http::{BodyError, NoAuthorizer, Response, SendError};

use crate::support::frames::{
    DATA, END_HEADERS, END_STREAM, HEADERS, frame, goaway_frame, response_head,
};
use crate::support::h2_body::{BodyScript, BodyServer, End};
use crate::support::pki::Pki;
use crate::support::raw_h2::{Answer, Policy, RawH2};
use crate::support::{guarded, post, trusting_client};

const TWO_HUNDRED: &[u8] = &[b'x'; 200];

async fn outcome(
    declared: Option<usize>,
    frames: &'static [&'static [u8]],
) -> Result<(), BodyError> {
    let server = BodyServer::start(BodyScript {
        declared,
        frames,
        end: End::Fin,
    })
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let mut response = guarded(client.send(post(&server.endpoint(), "x"), &NoAuthorizer))
        .await
        .expect("response");
    drain(&mut response).await
}

async fn drain(response: &mut Response) -> Result<(), BodyError> {
    loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(_)) => {}
            Ok(None) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn interrupted(outcome: &Result<(), BodyError>) -> bool {
    matches!(outcome, Err(BodyError::Interrupted { .. }))
}

fn protocol(outcome: &Result<(), BodyError>) -> bool {
    matches!(outcome, Err(BodyError::Protocol { .. }))
}

#[tokio::test]
async fn given_an_http2_frame_that_crosses_the_declared_length_while_bytes_were_missing_then_it_is_interrupted()
 {
    let cases: [(&str, usize, &'static [&'static [u8]]); 6] = [
        ("5 then 10 of 10", 10, &[b"01234", b"0123456789"]),
        ("8 then 5 of 10", 10, &[b"01234567", b"01234"]),
        ("one 15 of 10", 10, &[b"0123456789abcde"]),
        ("one 200 of 100", 100, &[TWO_HUNDRED]),
        ("10 then 200 of 100", 100, &[b"0123456789", TWO_HUNDRED]),
        ("0 then 15 of 10", 10, &[b"", b"0123456789abcde"]),
    ];
    for (name, declared, frames) in cases {
        let outcome = outcome(Some(declared), frames).await;
        assert!(interrupted(&outcome), "{name}: {outcome:?}");
    }
}

#[tokio::test]
async fn given_an_http2_body_with_more_bytes_after_its_whole_declared_length_then_it_is_a_protocol_failure()
 {
    let cases: [(&str, usize, &'static [&'static [u8]]); 2] = [
        ("10 then 5 of 10", 10, &[b"0123456789", b"01234"]),
        ("5 of 0", 0, &[b"01234"]),
    ];
    for (name, declared, frames) in cases {
        let outcome = outcome(Some(declared), frames).await;
        assert!(protocol(&outcome), "{name}: {outcome:?}");
    }
}

fn five_bytes_then_a_goaway_that_excludes_the_stream(stream: u32) -> Vec<u8> {
    let mut wire = response_head(stream, None, false);
    wire.extend_from_slice(&frame(DATA, 0, stream, b"01234"));
    wire.extend_from_slice(&goaway_frame(0, 0, b""));
    wire
}

fn five_of_ten_bytes_then_a_goaway_that_excludes_the_stream(stream: u32) -> Vec<u8> {
    let mut wire = response_head(stream, Some("10"), false);
    wire.extend_from_slice(&frame(DATA, 0, stream, b"01234"));
    wire.extend_from_slice(&goaway_frame(0, 0, b""));
    wire
}

async fn goaway_outcome(build: fn(u32) -> Vec<u8>) -> (Vec<u8>, Result<(), BodyError>) {
    let server = RawH2::start(move |_| Policy {
        answer: Answer::Frames(build),
        ..Policy::healthy()
    })
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let mut response = guarded(client.send(post(&server.endpoint(), "x"), &NoAuthorizer))
        .await
        .map_err(|error: SendError| error)
        .expect("response");
    let mut bytes = Vec::new();
    loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
            Ok(None) => return (bytes, Ok(())),
            Err(error) => return (bytes, Err(error)),
        }
    }
}

#[tokio::test]
async fn given_a_goaway_with_no_error_that_excludes_the_stream_and_no_declared_length_then_the_cut_is_not_seen()
 {
    let (bytes, outcome) = goaway_outcome(five_bytes_then_a_goaway_that_excludes_the_stream).await;
    assert_eq!(bytes, b"01234");
    assert_eq!(outcome, Ok(()));
}

#[tokio::test]
async fn given_a_goaway_with_no_error_that_excludes_the_stream_and_a_declared_length_then_it_is_interrupted()
 {
    let (_, outcome) =
        goaway_outcome(five_of_ten_bytes_then_a_goaway_that_excludes_the_stream).await;
    assert!(interrupted(&outcome), "{outcome:?}");
}

fn five_of_ten_bytes_then_trailers_with_a_pseudo_header(stream: u32) -> Vec<u8> {
    let mut wire = response_head(stream, Some("10"), false);
    wire.extend_from_slice(&frame(DATA, 0, stream, b"01234"));
    wire.extend_from_slice(&frame(HEADERS, END_STREAM | END_HEADERS, stream, &[0x88]));
    wire
}

#[tokio::test]
async fn given_a_stream_level_rule_broken_while_declared_bytes_are_missing_then_it_is_interrupted()
{
    let (_, outcome) = goaway_outcome(five_of_ten_bytes_then_trailers_with_a_pseudo_header).await;
    assert!(interrupted(&outcome), "{outcome:?}");
}
