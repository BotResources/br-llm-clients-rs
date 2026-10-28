use std::sync::Arc;

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::ClientError;

use crate::run::run_with;
use crate::secret_log::SecretLog;
use crate::secrets_cache::SecretsCache;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{Discard, spec_for, user_request};
use crate::test_server::{TestServer, http};

const LISTED: &str = "LISTED-5151";
const LATE: &str = "LATE-7272";

async fn refused_body(
    dialect: &Arc<ScriptedDialect>,
    log: &SecretLog,
    cache: &SecretsCache,
) -> String {
    let body = format!("a {LISTED} b {LATE} c");
    let server = TestServer::start(move |_| http("500 Bad", &[], body.as_bytes())).await;
    let spec = spec_for(dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new(LISTED).expect("secret")])
        .with_secret_log(log.clone());
    let outcome = run_with(&spec, cache, &user_request(), &mut Discard).await;
    assert!(cache.in_effect(&spec).built().is_some());
    match outcome.into_parts().1 {
        Err(ClientError::ProviderFailed { body, .. }) => body,
        other => panic!("not failed: {other:?}"),
    }
}

#[tokio::test]
async fn given_a_refusal_and_a_log_that_does_not_change_when_run_then_the_secrets_are_built_once() {
    let dialect = Arc::new(ScriptedDialect::default());
    let cache = SecretsCache::default();
    let body = refused_body(&dialect, &SecretLog::new(), &cache).await;
    assert_eq!(body, format!("a [redacted] b {LATE} c"));
    assert_eq!(cache.snapshots_taken(), 1);
}

#[tokio::test]
async fn given_a_secret_recorded_between_the_two_cleanings_when_run_then_the_secrets_are_built_again()
 {
    let log = SecretLog::new();
    let recorder = log.clone();
    let dialect = Arc::new(ScriptedDialect {
        refine: Box::new(move |_| {
            recorder.record(&Secret::new(LATE).expect("secret"));
            None
        }),
        ..ScriptedDialect::default()
    });
    let cache = SecretsCache::default();
    let body = refused_body(&dialect, &log, &cache).await;
    assert_eq!(
        dialect.seen_bodies(),
        vec![format!("a [redacted] b {LATE} c")]
    );
    assert_eq!(body, "a [redacted] b [redacted] c");
    assert_eq!(cache.snapshots_taken(), 2);
}
