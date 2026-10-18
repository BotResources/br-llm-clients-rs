use std::sync::Arc;
use std::time::Duration;

use tokio::time::advance;

use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{Busy, assistant_first_request, run_call, spec_for, user_request};
use crate::test_server::{TestServer, http};

async fn slow_server(wait: Duration) -> TestServer {
    TestServer::start(move |_| {
        http("500 Bad", &[], b"").after(async move {
            advance(wait).await;
        })
    })
    .await
}

#[tokio::test(start_paused = true)]
async fn given_a_server_that_waits_when_run_then_the_total_duration_is_the_wait() {
    let _busy = Busy::keep();
    let server = slow_server(Duration::from_secs(5)).await;
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let report = run_call(&spec, &user_request()).await.into_parts().0;
    assert_eq!(report.total_duration(), Duration::from_secs(5));
    assert_eq!(report.time_to_first_event(), None);
}

#[tokio::test(start_paused = true)]
async fn given_a_call_that_is_not_polled_yet_when_time_passes_then_the_call_starts_at_its_first_poll()
 {
    let _busy = Busy::keep();
    let server = slow_server(Duration::from_secs(5)).await;
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let request = user_request();
    let call = run_call(&spec, &request);
    advance(Duration::from_secs(100)).await;
    let report = call.await.into_parts().0;
    assert_eq!(report.total_duration(), Duration::from_secs(5));
}

#[tokio::test(start_paused = true)]
async fn given_a_failure_before_anything_is_sent_when_run_then_the_total_duration_is_zero() {
    let server = slow_server(Duration::from_secs(5)).await;
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, server.endpoint("/v1/test"));
    let report = run_call(&spec, &assistant_first_request())
        .await
        .into_parts()
        .0;
    assert_eq!(report.total_duration(), Duration::ZERO);
    assert_eq!(report.request_id(), None);
}
