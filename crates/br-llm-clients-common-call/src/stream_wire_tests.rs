use std::sync::Arc;

use br_llm_clients_core::ClientError;
use br_llm_messages::ResponseEnding;

use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{spec_for, user_request};
use crate::test_requests::structured_request;
use crate::test_server::{TestServer, http, raw};
use crate::test_sink::RecordingSink;
use crate::test_stream::{body, call_with, dialect_ending, kinds};

const REPLY: [&str; 6] = [
    "usage 3 4",
    "model m-1",
    "start a text",
    "text a hi",
    "end a",
    "stop finished",
];

fn calm() -> Arc<ScriptedDialect> {
    dialect_ending(EndScript::Events(Vec::new()))
}

fn broken(head: &str, sent: &[u8]) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 200 OK\r\n{head}content-length: 100000\r\n\r\n").into_bytes();
    bytes.extend_from_slice(sent);
    bytes
}

#[tokio::test]
async fn given_a_reply_over_the_wire_when_run_then_the_outcome_holds_the_step_and_one_request_was_made()
 {
    let dialect = calm();
    let called = call_with(&dialect, &user_request(), body(&[&REPLY])).await;
    let (report, result) = called.outcome.into_parts();
    let step = result.expect("a step");
    assert_eq!(step.ending(), &ResponseEnding::finished());
    assert_eq!(called.requests, 1);
    assert_eq!(report.usage(), step.usage());
    assert_eq!(report.model().map(|model| model.as_str()), Some("m-1"));
    assert_eq!(
        kinds(&called.sink.deltas),
        vec![
            "started",
            "meta usage=Some(3)/Some(4)",
            "meta model=m-1",
            "start 1 text",
            "text 1 \"hi\"",
            "final 1",
            "done Finished"
        ]
    );
}

#[tokio::test]
async fn given_any_content_type_when_run_then_the_body_is_read_as_a_stream_anyway() {
    for content_type in ["application/json", "text/plain", "text/event-stream"] {
        let dialect = calm();
        let bytes = body(&[&REPLY]);
        let server =
            TestServer::start(move |_| http("200 OK", &[("Content-Type", content_type)], &bytes))
                .await;
        let spec = spec_for(&dialect, server.endpoint("/v1/test"));
        let mut sink = RecordingSink::default();
        let outcome = crate::run::run(&spec, &user_request(), &mut sink).await;
        assert!(outcome.into_parts().1.is_ok(), "{content_type}");
    }
}

#[tokio::test]
async fn given_a_body_that_breaks_mid_stream_when_run_then_it_is_truncated_and_the_earlier_events_stand()
 {
    let dialect = calm();
    let sent = body(&[&["usage 3 4", "model m-1", "start a text", "text a hi"]]);
    let server = TestServer::start(move |_| raw(broken("", &sent))).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let mut sink = RecordingSink::default();
    let (report, result) = crate::run::run(&spec, &user_request(), &mut sink)
        .await
        .into_parts();
    assert_eq!(result, Err(ClientError::StreamTruncated));
    assert_eq!(server.count(), 1);
    assert_eq!(
        kinds(&sink.deltas),
        vec![
            "started",
            "meta usage=Some(3)/Some(4)",
            "meta model=m-1",
            "start 1 text",
            "text 1 \"hi\""
        ]
    );
    assert_eq!(
        report.usage().map(|usage| usage.input_tokens()),
        Some(Some(3))
    );
    assert_eq!(report.model().map(|model| model.as_str()), Some("m-1"));
    assert_eq!(dialect.log.ends(), 0);
}

#[tokio::test]
async fn given_a_body_with_an_encoding_that_is_not_supported_when_run_then_the_reply_is_malformed()
{
    let dialect = calm();
    let server = TestServer::start(|_| {
        http(
            "200 OK",
            &[("Content-Encoding", "unknown-coding")],
            b"data: x\n\n",
        )
    })
    .await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let mut sink = RecordingSink::default();
    let error = crate::run::run(&spec, &user_request(), &mut sink)
        .await
        .into_parts()
        .1
        .expect_err("a failure");
    assert!(
        matches!(error, ClientError::MalformedResponse { .. }),
        "{error:?}"
    );
    assert_eq!(server.count(), 1);
}

#[tokio::test]
async fn given_a_complete_when_the_body_breaks_afterwards_then_the_call_still_succeeds() {
    let dialect = calm();
    let mut sent = body(&[&REPLY]);
    sent.extend(body(&[&["complete"]]));
    let server = TestServer::start(move |_| raw(broken("", &sent))).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let mut sink = RecordingSink::default();
    let outcome = crate::run::run(&spec, &user_request(), &mut sink).await;
    assert!(outcome.into_parts().1.is_ok());
    assert_eq!(dialect.log.ends(), 0);
    assert_eq!(server.count(), 1);
}

#[tokio::test]
async fn given_a_stream_that_fails_when_run_then_the_call_is_not_retried() {
    let dialect = calm();
    let called = call_with(&dialect, &user_request(), body(&[&["error 500 - - boom"]])).await;
    assert!(called.outcome.into_parts().1.is_err());
    assert_eq!(called.requests, 1);
    let empty = call_with(&dialect, &user_request(), Vec::new()).await;
    assert_eq!(
        empty.outcome.into_parts().1,
        Err(ClientError::StreamTruncated)
    );
    assert_eq!(empty.requests, 1);
}

#[tokio::test]
async fn given_a_request_id_in_the_headers_when_a_reply_is_read_then_the_report_holds_it() {
    let dialect = calm();
    let bytes = body(&[&REPLY]);
    let server =
        TestServer::start(move |_| http("200 OK", &[("X-Request-Id", "req-7")], &bytes)).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let mut sink = RecordingSink::default();
    let report = crate::run::run(&spec, &user_request(), &mut sink)
        .await
        .into_parts()
        .0;
    assert_eq!(report.request_id(), Some("req-7"));
}

#[tokio::test]
async fn given_a_secret_in_a_decoder_text_when_run_then_the_error_of_the_outcome_is_cleaned() {
    let dialect = dialect_ending(EndScript::Failure("the key sk-77 broke".to_owned()));
    let secret = br_llm_clients_common_http::Secret::new("sk-77").expect("secret");
    let bytes = body(&[&REPLY]);
    let server = TestServer::start(move |_| http("200 OK", &[], &bytes)).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test")).with_secrets(vec![secret]);
    let mut sink = RecordingSink::default();
    let error = crate::run::run(&spec, &user_request(), &mut sink)
        .await
        .into_parts()
        .1
        .expect_err("a failure");
    assert_eq!(
        error,
        ClientError::MalformedResponse {
            detail: "the key [redacted] broke".to_owned()
        }
    );
}

#[tokio::test]
async fn given_calls_when_run_then_each_reply_gets_its_own_decoder_made_from_the_output_mode_of_its_request()
 {
    let dialect = calm();
    let plain = call_with(&dialect, &user_request(), body(&[&REPLY])).await;
    assert!(plain.outcome.into_parts().1.is_ok());
    assert_eq!(dialect.log.created(), 1);
    let structured = structured_request(false);
    call_with(&dialect, &structured, body(&[&REPLY])).await;
    assert_eq!(dialect.log.created(), 2);
    assert_eq!(
        dialect.seen_modes(),
        vec![user_request().output().clone(), structured.output().clone()]
    );
}

#[tokio::test]
async fn given_a_refusal_when_run_then_no_decoder_is_made() {
    let dialect = calm();
    let server = TestServer::start(|_| http("500 Bad", &[], b"")).await;
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let mut sink = RecordingSink::default();
    crate::run::run(&spec, &user_request(), &mut sink).await;
    assert_eq!(dialect.log.created(), 0);
}
