use std::time::Duration;

use http::HeaderValue;

use crate::authorizer::{AuthorizationRequest, AuthorizedHeader};
use crate::authorizers::{ApiKeyAuthorizer, BearerAuthorizer};
use crate::body::Collected;
use crate::config::HttpConfig;
use crate::endpoint::Endpoint;
use crate::prepare::prepare;
use crate::request::Request;
use crate::secret::Secret;

const LEAKS: [&str; 5] = [
    "SECRET-KEY",
    "SECRET-BODY",
    "SECRET-QUERY",
    "SECRET-HEADER",
    "SECRET-PEM",
];

fn assert_clean(label: &str, text: &str) {
    for leak in LEAKS {
        assert!(!text.contains(leak), "{label} leaks {leak}: {text}");
    }
}

#[test]
fn given_every_type_that_can_hold_a_secret_or_payload_when_formatted_then_nothing_sensitive_is_printed()
 {
    let secret = Secret::new("SECRET-KEY").expect("secret");
    let endpoint = Endpoint::https("https://h/p?token=SECRET-QUERY")
        .expect("endpoint")
        .with_query_pair("k", "SECRET-QUERY");
    let request = Request::new(endpoint.clone(), "SECRET-BODY")
        .with_header("x-h", "SECRET-HEADER")
        .expect("header");
    let prepared = prepare(request.clone(), &HeaderValue::from_static("a/1")).expect("prepared");
    let view = AuthorizationRequest::new(
        &prepared.path_and_query,
        &prepared.host,
        &prepared.headers,
        &prepared.body,
    );
    let mut config = HttpConfig::new(
        Duration::from_secs(1),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    config.extra_root_certificates = vec!["SECRET-PEM".to_owned()];
    let collected = Collected {
        bytes: b"SECRET-BODY".to_vec(),
        cut: false,
    };
    let header = AuthorizedHeader::new("authorization", secret.clone());
    let api_key = ApiKeyAuthorizer::new("x-api-key", secret.clone()).expect("authorizer");
    let bearer = BearerAuthorizer::new(&secret);
    for (label, text) in [
        ("secret", format!("{secret:?} {secret} {secret:#?}")),
        ("endpoint", format!("{endpoint:?} {endpoint} {endpoint:#?}")),
        ("request", format!("{request:?} {request:#?}")),
        ("authorization request", format!("{view:?} {view:#?}")),
        ("config", format!("{config:?} {config:#?}")),
        ("collected", format!("{collected:?} {collected:#?}")),
        ("authorized header", format!("{header:?} {header:#?}")),
        ("api key", format!("{api_key:?} {api_key:#?}")),
        ("bearer", format!("{bearer:?} {bearer:#?}")),
    ] {
        assert_clean(label, &text);
    }
}
