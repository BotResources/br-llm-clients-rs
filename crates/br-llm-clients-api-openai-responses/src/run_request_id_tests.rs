use crate::test_events::{completed, created, usage_object};
use crate::test_requests::hello;
use crate::test_run::{call, sse};
use crate::test_server::http;

fn stream() -> Vec<u8> {
    sse(&[created("m"), completed("m", usage_object(1, 1))])
}

async fn request_id_with(headers: &[(&str, &str)]) -> Option<String> {
    let called = call(&hello(), http("200 OK", headers, &stream())).await;
    called.outcome.report().request_id().map(str::to_owned)
}

#[tokio::test]
async fn given_the_x_request_id_header_when_run_then_the_report_holds_the_value_as_received() {
    assert_eq!(
        request_id_with(&[("x-request-id", "req_2f8a91")]).await,
        Some("req_2f8a91".to_owned())
    );
}

#[tokio::test]
async fn given_the_header_in_another_case_when_run_then_it_is_found() {
    for name in ["X-Request-Id", "X-REQUEST-ID", "x-Request-ID"] {
        assert_eq!(
            request_id_with(&[(name, "Req_MiXeD-1")]).await,
            Some("Req_MiXeD-1".to_owned()),
            "{name}"
        );
    }
}

#[tokio::test]
async fn given_no_header_when_run_then_there_is_no_request_id() {
    assert_eq!(
        request_id_with(&[("content-type", "text/event-stream")]).await,
        None
    );
}

#[tokio::test]
async fn given_an_empty_header_value_when_run_then_there_is_no_request_id() {
    assert_eq!(request_id_with(&[("x-request-id", "")]).await, None);
}

#[tokio::test]
async fn given_other_request_id_like_headers_when_run_then_only_x_request_id_counts() {
    assert_eq!(
        request_id_with(&[
            ("request-id", "a"),
            ("x-client-request-id", "b"),
            ("openai-request-id", "c")
        ])
        .await,
        None
    );
}

#[tokio::test]
async fn given_a_refusal_with_the_header_when_run_then_the_report_holds_the_request_id() {
    let wire = http(
        "500 Internal Server Error",
        &[("x-request-id", "req_err_1")],
        b"{}",
    );
    let called = call(&hello(), wire).await;
    assert_eq!(called.outcome.report().request_id(), Some("req_err_1"));
}
