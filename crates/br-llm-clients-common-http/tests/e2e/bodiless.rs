use br_llm_clients_common_http::{BodyError, NoAuthorizer, Response, SendError};

use crate::support::frames::{
    DATA, END_STREAM, STATUS_204, STATUS_304, frame, response_head_with_status, trailers,
};
use crate::support::pki::Pki;
use crate::support::raw_h2::{Answer, Policy, RawH2};
use crate::support::{guarded, post, trusting_client};

const STATUS_200: u8 = 0x88;

fn ends_with_headers(status: u8, stream: u32) -> Vec<u8> {
    response_head_with_status(stream, status, Some("5"), true)
}

fn ends_with_empty_data(status: u8, stream: u32) -> Vec<u8> {
    let mut wire = response_head_with_status(stream, status, Some("5"), false);
    wire.extend_from_slice(&frame(DATA, END_STREAM, stream, &[]));
    wire
}

fn ends_with_trailers(status: u8, stream: u32) -> Vec<u8> {
    let mut wire = response_head_with_status(stream, status, Some("5"), false);
    wire.extend_from_slice(&trailers(stream));
    wire
}

macro_rules! shapes {
    ($status:expr) => {
        [
            |stream| ends_with_headers($status, stream),
            |stream| ends_with_empty_data($status, stream),
            |stream| ends_with_trailers($status, stream),
        ]
    };
}

const SHAPE_NAMES: [&str; 3] = [
    "END_STREAM on HEADERS",
    "HEADERS then DATA(0, END_STREAM)",
    "HEADERS then trailers",
];

async fn response_for(build: fn(u32) -> Vec<u8>) -> Result<(Response, RawH2), SendError> {
    let server = RawH2::start(move |_| Policy {
        answer: Answer::Frames(build),
        ..Policy::healthy()
    })
    .await;
    let client = trusting_client(&Pki::trusted().ca_pem);
    let response = guarded(client.send(post(&server.endpoint(), "x"), &NoAuthorizer)).await?;
    Ok((response, server))
}

async fn streamed(build: fn(u32) -> Vec<u8>) -> Result<Vec<u8>, String> {
    let (mut response, _server) = response_for(build).await.map_err(|e| format!("{e:?}"))?;
    let mut bytes = Vec::new();
    loop {
        match guarded(response.body_mut().next_chunk()).await {
            Ok(Some(chunk)) => bytes.extend_from_slice(&chunk),
            Ok(None) => return Ok(bytes),
            Err(error) => return Err(format!("{error:?}")),
        }
    }
}

async fn collected(build: fn(u32) -> Vec<u8>) -> Result<(Vec<u8>, bool), String> {
    let (mut response, _server) = response_for(build).await.map_err(|e| format!("{e:?}"))?;
    let outcome = guarded(response.body_mut().collect(100)).await;
    outcome
        .map(|collected| (collected.bytes, collected.cut))
        .map_err(|error| format!("{error:?}"))
}

#[tokio::test]
async fn given_a_204_or_a_304_with_a_declared_length_then_it_is_a_response_without_a_body() {
    let statuses = [
        (STATUS_204, shapes!(STATUS_204)),
        (STATUS_304, shapes!(STATUS_304)),
    ];
    for (status, builds) in statuses {
        for (build, name) in builds.into_iter().zip(SHAPE_NAMES) {
            let label = format!("status byte {status:#x}, {name}");
            assert_eq!(streamed(build).await, Ok(Vec::new()), "{label}, stream");
            assert_eq!(
                collected(build).await,
                Ok((Vec::new(), false)),
                "{label}, collect"
            );
            let (response, _server) = response_for(build).await.expect(&label);
            assert!(
                response.status() == 204 || response.status() == 304,
                "{label}"
            );
        }
    }
}

#[tokio::test]
async fn given_a_200_with_the_same_frames_then_it_keeps_its_failure_kinds() {
    let [headers, empty_data, with_trailers] = shapes!(STATUS_200);
    let outcome = response_for(headers).await.map(|_| ());
    assert!(
        matches!(outcome, Err(SendError::Protocol { .. })),
        "{outcome:?}"
    );
    for (build, name) in [
        (empty_data, SHAPE_NAMES[1]),
        (with_trailers, SHAPE_NAMES[2]),
    ] {
        let (mut response, _server) = response_for(build).await.expect(name);
        let outcome = guarded(response.body_mut().collect(100)).await;
        assert!(
            matches!(outcome, Err(BodyError::Interrupted { .. })),
            "{name}: {outcome:?}"
        );
    }
}
