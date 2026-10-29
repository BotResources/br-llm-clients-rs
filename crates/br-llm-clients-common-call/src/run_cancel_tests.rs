use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use br_llm_clients_common_http::Secret;
use br_llm_clients_core::{BoxFuture, StreamSink};
use br_llm_messages::ResponseDelta;
use tokio::sync::Notify;

use crate::call_spec::CallSpec;
use crate::run::run;
use crate::secret_log::SecretLog;
use crate::test_decoder::EndScript;
use crate::test_fixtures::{run_call, spec_for, user_request};
use crate::test_server::{TestServer, closed_endpoint, http, streamed};
use crate::test_stream::{body, dialect_ending};

const LOGGED: &str = "LOGGED-1234";
const LISTED: &str = "LISTED-5678";

struct Signalling {
    seen: Arc<AtomicUsize>,
    reached: Arc<Notify>,
    at: usize,
}

impl StreamSink for Signalling {
    fn send<'a>(&'a mut self, _delta: ResponseDelta) -> BoxFuture<'a, ()> {
        let count = self.seen.fetch_add(1, Ordering::SeqCst).saturating_add(1);
        if count == self.at {
            self.reached.notify_one();
        }
        Box::pin(async {})
    }
}

fn contents(log: &SecretLog) -> Vec<String> {
    log.snapshot()
        .iter()
        .map(|secret| secret.expose().to_owned())
        .collect()
}

fn full_reply() -> Vec<u8> {
    body(&[&["start a text", "text a hi", "end a", "stop finished"]])
}

fn stalled(first: Vec<u8>) -> crate::test_server::Wire {
    streamed(
        "200 OK",
        &[],
        &first,
        vec![(Box::pin(std::future::pending()), b"x".to_vec())],
    )
}

async fn server_stalling_first_call() -> TestServer {
    TestServer::start(|index| {
        if index == 0 {
            stalled(body(&[&["start a text", "text a hi"]]))
        } else {
            http("200 OK", &[], &full_reply())
        }
    })
    .await
}

fn logged_spec(server: &TestServer, log: &SecretLog) -> CallSpec {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new(LISTED).expect("secret")])
        .with_secret_log(log.clone())
}

#[tokio::test]
async fn given_a_run_dropped_while_the_body_is_stalled_when_dropped_then_it_gives_no_outcome_and_sends_nothing_more()
 {
    let server = server_stalling_first_call().await;
    let spec = logged_spec(&server, &SecretLog::new());
    let seen = Arc::new(AtomicUsize::new(0));
    let reached = Arc::new(Notify::new());
    let mut sink = Signalling {
        seen: Arc::clone(&seen),
        reached: Arc::clone(&reached),
        at: 3,
    };
    let request = user_request();
    let mut call = Box::pin(run(&spec, &request, &mut sink));
    tokio::select! {
        _ = &mut call => panic!("the call ended while its body was stalled"),
        () = reached.notified() => {}
    }
    drop(call);
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    assert_eq!(seen.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn given_a_run_dropped_mid_stream_when_the_log_and_the_spec_are_used_again_then_both_work() {
    let server = server_stalling_first_call().await;
    let log = SecretLog::new();
    log.record(&Secret::new(LOGGED).expect("secret"));
    let spec = logged_spec(&server, &log);
    let seen = Arc::new(AtomicUsize::new(0));
    let reached = Arc::new(Notify::new());
    let mut sink = Signalling {
        seen,
        reached: Arc::clone(&reached),
        at: 3,
    };
    let request = user_request();
    let mut call = Box::pin(run(&spec, &request, &mut sink));
    tokio::select! {
        _ = &mut call => panic!("the call ended while its body was stalled"),
        () = reached.notified() => {}
    }
    drop(call);
    assert_eq!(contents(&log), vec![LOGGED.to_owned()]);
    log.record(&Secret::new("LATER-9999").expect("secret"));
    assert_eq!(
        contents(&log),
        vec![LOGGED.to_owned(), "LATER-9999".to_owned()]
    );
    let second = run_call(&spec, &request).await;
    let (_, result) = second.into_parts();
    assert!(result.is_ok());
    assert_eq!(server.count(), 2);
}

#[tokio::test]
async fn given_a_success_a_refusal_and_a_failure_when_run_then_the_log_is_neither_filled_nor_emptied()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let text = format!("echo {LISTED} {LOGGED}");
    let reply = body(&[&[
        "start a text",
        &format!("text a {text}"),
        "end a",
        "stop finished",
    ]]);
    let refusal = text.clone().into_bytes();
    let server = TestServer::start(move |index| match index {
        0 => http("200 OK", &[], &reply),
        _ => http("500 Bad", &[], &refusal),
    })
    .await;
    let log = SecretLog::new();
    for value in [LOGGED, "OTHER-4321"] {
        log.record(&Secret::new(value).expect("secret"));
    }
    let before = contents(&log);
    let spec = spec_for(&dialect, server.endpoint("/v1/test"))
        .with_secrets(vec![Secret::new(LISTED).expect("secret")])
        .with_secret_log(log.clone());
    let success = run_call(&spec, &user_request()).await;
    assert!(success.into_parts().1.is_ok());
    assert_eq!(contents(&log), before);
    let refused = run_call(&spec, &user_request()).await;
    assert!(refused.into_parts().1.is_err());
    assert_eq!(contents(&log), before);
    let broken = spec_for(&dialect, closed_endpoint())
        .with_secrets(vec![Secret::new(LISTED).expect("secret")])
        .with_secret_log(log.clone());
    let failed = run_call(&broken, &user_request()).await;
    assert!(failed.into_parts().1.is_err());
    assert_eq!(contents(&log), before);
}
