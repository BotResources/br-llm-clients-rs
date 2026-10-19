use crate::run_support::{call_with, sse_with_sentinel};
use crate::test_server::raw;
use crate::test_support::{content_chunk, dialect, finish_chunk, hello};

async fn request_id_with_headers(header_lines: &[u8]) -> Option<String> {
    let mut bytes = b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n".to_vec();
    bytes.extend_from_slice(header_lines);
    let body = sse_with_sentinel(&[content_chunk("a"), finish_chunk("stop")]);
    bytes.extend_from_slice(
        format!(
            "content-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        )
        .as_bytes(),
    );
    bytes.extend_from_slice(&body);
    let called = call_with(dialect(), &hello(), move |_| raw(bytes.clone())).await;
    called.outcome.report().request_id().map(str::to_owned)
}

#[tokio::test]
async fn given_the_request_id_header_when_called_then_the_report_carries_its_value() {
    let id = request_id_with_headers(b"x-request-id: req_5d8f1c\r\n").await;
    assert_eq!(id.as_deref(), Some("req_5d8f1c"));
}

#[tokio::test]
async fn given_the_header_in_any_case_when_called_then_it_is_found() {
    for name in ["X-Request-Id", "X-REQUEST-ID", "x-Request-ID"] {
        let id = request_id_with_headers(format!("{name}: abc\r\n").as_bytes()).await;
        assert_eq!(id.as_deref(), Some("abc"), "{name}");
    }
}

#[tokio::test]
async fn given_no_header_when_called_then_there_is_no_request_id() {
    assert_eq!(request_id_with_headers(b"").await, None);
    assert_eq!(request_id_with_headers(b"x-other-id: abc\r\n").await, None);
}

#[tokio::test]
async fn given_an_empty_value_when_called_then_there_is_no_request_id() {
    assert_eq!(request_id_with_headers(b"x-request-id:\r\n").await, None);
}

#[tokio::test]
async fn given_two_headers_of_that_name_when_called_then_the_first_is_used() {
    let id = request_id_with_headers(b"x-request-id: first\r\nx-request-id: second\r\n").await;
    assert_eq!(id.as_deref(), Some("first"));
}

#[tokio::test]
async fn given_a_value_that_is_not_valid_text_when_called_then_there_is_no_request_id() {
    let id = request_id_with_headers(b"x-request-id: bad\xff\xfevalue\r\n").await;
    assert_eq!(id, None);
}

#[tokio::test]
async fn given_a_refused_call_when_the_header_is_present_then_the_report_still_carries_it() {
    let called =
        crate::run_support::call_refused("500 Oops", &[("x-request-id", "req_failed")], "{}").await;
    assert_eq!(called.outcome.report().request_id(), Some("req_failed"));
}
