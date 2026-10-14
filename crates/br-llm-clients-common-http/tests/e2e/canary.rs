use br_llm_clients_common_http::{ApiKeyAuthorizer, Endpoint, Request, Secret, SendError};

use crate::support::pki::Pki;
use crate::support::probe::go_away_with;
use crate::support::raw::RawServer;
use crate::support::{guarded, trusting_client};

async fn error_for(key: &str, debug: String, last_stream: u32) -> SendError {
    let acceptor = Pki::trusted().acceptor(&[b"h2"]);
    let server = RawServer::start(move |stream, _| {
        let (acceptor, debug) = (acceptor.clone(), debug.clone());
        async move {
            if let Ok(tls) = acceptor.accept(stream).await {
                go_away_with(tls, last_stream, debug.as_bytes()).await;
            }
        }
    })
    .await;
    let endpoint = Endpoint::https(&format!(
        "https://127.0.0.1:{}/x?token=QUERY-CANARY",
        server.addr.port()
    ))
    .expect("endpoint");
    let authorizer =
        ApiKeyAuthorizer::new("x-api-key", Secret::new(key).expect("secret")).expect("authorizer");
    let request = Request::new(endpoint, "BODY-CANARY");
    guarded(trusting_client(&Pki::trusted().ca_pem).send(request, &authorizer))
        .await
        .unwrap_err()
}

fn assert_text_is_ours(error: &SendError, secrets: &[&str]) {
    let texts = [
        error.to_string(),
        format!("{error:?}"),
        format!("{error:#?}"),
    ];
    for text in texts {
        assert!(
            text.chars().count() <= 512,
            "{} chars",
            text.chars().count()
        );
        for secret in secrets {
            assert!(!text.contains(secret), "leaked {secret}: {text}");
        }
        assert!(
            !text.contains("BODY-CANARY") && !text.contains("QUERY-CANARY"),
            "{text}"
        );
    }
}

#[tokio::test]
async fn given_goaway_debug_data_that_echoes_the_credential_then_no_error_text_holds_it() {
    let key = "sk-CANARY-7731";
    let debug = format!("echo x-api-key: {key} body: BODY-CANARY token=QUERY-CANARY");
    for last_stream in [0u32, 1] {
        let error = error_for(key, debug.clone(), last_stream).await;
        assert_text_is_ours(&error, &[key, "echo x-api-key"]);
    }
}

#[tokio::test]
async fn given_goaway_debug_data_with_the_credential_escaped_then_no_error_text_holds_it() {
    let key = "sk\"CANARY\\7731";
    let debug = format!("echo key={key}");
    let error = error_for(key, debug, 1).await;
    assert_text_is_ours(&error, &["CANARY", "echo key"]);
}

#[tokio::test]
async fn given_sixteen_kilobytes_of_goaway_debug_data_then_the_error_text_stays_short() {
    let error = error_for("sk-CANARY-7731", "A".repeat(16_000), 1).await;
    assert_text_is_ours(&error, &["sk-CANARY-7731", &"A".repeat(100)]);
}
