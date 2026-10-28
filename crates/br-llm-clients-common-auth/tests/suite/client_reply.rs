use std::time::Duration;

use br_llm_clients_common_auth::{FormTokenClient, TokenForm};
use br_llm_clients_common_http::{AuthorizerError, Endpoint, Secret};
use tokio::net::TcpListener;

use crate::support::server::{Answer, Server, http_client, json, short_body};

const LIMIT: usize = 64 * 1024;

fn form() -> TokenForm {
    TokenForm::refresh_token(Secret::new("rt-in").expect("secret"))
}

fn client(server: &Server) -> FormTokenClient {
    FormTokenClient::new(
        http_client(Duration::from_secs(5)),
        server.endpoint("/token"),
    )
}

fn padded(total: usize) -> String {
    let head = r#"{"access_token":"at","pad":""#;
    let tail = r#""}"#;
    let fill = total - head.len() - tail.len();
    format!("{head}{}{tail}", "x".repeat(fill))
}

#[tokio::test]
async fn given_a_full_reply_when_fetched_then_the_parts_are_read_and_other_members_ignored() {
    let body = r#"{"access_token":"at-1","token_type":"bEaReR","expires_in":3599,"refresh_token":"rt-1","scope":"a b","ext":{"k":[1]}}"#;
    let server = Server::start(vec![json(200, body)]).await;
    let reply = client(&server).fetch(&form()).await.expect("reply");
    assert_eq!(reply.access_token.secret().expose(), "at-1");
    assert_eq!(
        reply.access_token.valid_for(),
        Some(Duration::from_secs(3599))
    );
    assert_eq!(reply.refresh_token.expect("refresh").expose(), "rt-1");
}

#[tokio::test]
async fn given_a_reply_with_only_a_token_in_any_2xx_when_fetched_then_there_is_no_validity_and_no_refresh_token()
 {
    for status in [200, 201, 202, 299] {
        let server = Server::start(vec![json(status, r#"{"access_token":"at"}"#)]).await;
        let reply = client(&server).fetch(&form()).await.expect("reply");
        assert_eq!(reply.access_token.valid_for(), None, "status {status}");
        assert!(reply.refresh_token.is_none(), "status {status}");
    }
}

#[tokio::test]
async fn given_expires_in_as_digits_in_a_text_then_it_is_the_validity() {
    let server = Server::start(vec![json(
        200,
        r#"{"access_token":"at","expires_in":"3600"}"#,
    )])
    .await;
    let reply = client(&server).fetch(&form()).await.expect("reply");
    assert_eq!(
        reply.access_token.valid_for(),
        Some(Duration::from_secs(3600))
    );
}

#[tokio::test]
async fn given_a_malformed_success_then_it_is_a_fetch_failure_with_the_fixed_text_of_its_flaw() {
    let cases = [
        ("[]", "a body that is not a JSON object"),
        (
            r#"{"access_token":""}"#,
            "an access_token that is missing or empty",
        ),
        (
            r#"{"access_token":"a","token_type":"mac"}"#,
            "a token_type that is not Bearer",
        ),
        (
            r#"{"access_token":"a","expires_in":1.5}"#,
            "an expires_in that is not a whole number of seconds",
        ),
        (
            r#"{"access_token":"a","refresh_token":""}"#,
            "a refresh_token that is empty or not a text",
        ),
    ];
    for (body, flaw) in cases {
        let server = Server::start(vec![json(200, body)]).await;
        let error = client(&server).fetch(&form()).await.expect_err("malformed");
        assert_eq!(
            error,
            AuthorizerError::fetch_failed(format!("the token endpoint answered 200 with {flaw}")),
            "body {body}"
        );
    }
}

#[tokio::test]
async fn given_a_body_of_exactly_64_kib_then_it_is_read_and_one_byte_more_is_a_fetch_failure() {
    let exact = Server::start(vec![json(200, &padded(LIMIT))]).await;
    let reply = client(&exact).fetch(&form()).await.expect("reply");
    assert_eq!(reply.access_token.secret().expose(), "at");
    let over = Server::start(vec![json(200, &padded(LIMIT + 1))]).await;
    let error = client(&over).fetch(&form()).await.expect_err("too long");
    assert_eq!(
        error,
        AuthorizerError::fetch_failed(
            "the token endpoint answered 200 with a body of more than 64 KiB"
        )
    );
}

fn chunked(body: &[u8]) -> Answer {
    let mut bytes =
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n".to_vec();
    for chunk in body.chunks(1000) {
        bytes.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
        bytes.extend_from_slice(chunk);
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"0\r\n\r\n");
    Answer::Raw(bytes)
}

#[tokio::test]
async fn given_a_chunked_body_with_no_length_then_the_64_kib_limit_still_holds() {
    let exact = Server::start(vec![chunked(padded(LIMIT).as_bytes())]).await;
    let reply = client(&exact).fetch(&form()).await.expect("reply");
    assert_eq!(reply.access_token.secret().expose(), "at");
    let over = Server::start(vec![chunked(padded(LIMIT + 1).as_bytes())]).await;
    assert_eq!(
        client(&over).fetch(&form()).await.expect_err("too long"),
        AuthorizerError::fetch_failed(
            "the token endpoint answered 200 with a body of more than 64 KiB"
        )
    );
}

#[tokio::test]
async fn given_a_body_that_ends_early_then_it_is_a_fetch_failure_that_names_the_stage() {
    let server = Server::start(vec![short_body(200, 100, br#"{"access_to"#)]).await;
    let error = client(&server)
        .fetch(&form())
        .await
        .expect_err("interrupted");
    assert_eq!(
        error,
        AuthorizerError::fetch_failed("the token answer could not be read")
    );
}

#[tokio::test]
async fn given_a_server_that_closes_without_an_answer_then_it_is_a_fetch_failure_that_names_the_stage()
 {
    let server = Server::start(vec![Answer::Close]).await;
    let error = client(&server).fetch(&form()).await.expect_err("closed");
    assert_eq!(
        error,
        AuthorizerError::fetch_failed("the token request could not be sent")
    );
}

#[tokio::test]
async fn given_a_port_where_nothing_listens_then_it_is_a_fetch_failure_that_names_the_stage() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("address").port();
    drop(listener);
    let endpoint =
        Endpoint::plain_http(&format!("http://127.0.0.1:{port}/token")).expect("endpoint");
    let client = FormTokenClient::new(http_client(Duration::from_secs(5)), endpoint);
    let error = client.fetch(&form()).await.expect_err("refused");
    assert_eq!(
        error,
        AuthorizerError::fetch_failed("the token request could not be sent")
    );
}

#[tokio::test]
async fn given_a_server_that_never_answers_then_the_http_timeout_is_a_fetch_failure() {
    let server = Server::start(vec![Answer::Hang]).await;
    let client = FormTokenClient::new(
        http_client(Duration::from_millis(150)),
        server.endpoint("/token"),
    );
    let error = client.fetch(&form()).await.expect_err("timeout");
    assert_eq!(
        error,
        AuthorizerError::fetch_failed("the token request could not be sent")
    );
}
