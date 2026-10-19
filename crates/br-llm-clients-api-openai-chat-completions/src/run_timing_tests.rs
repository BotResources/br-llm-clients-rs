use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use br_llm_clients_common_call::run;
use br_llm_clients_core::CallReport;
use serde_json::json;
use tokio::time::advance;

use crate::run_support::{spec_for, sse, sse_with_sentinel};
use crate::test_server::{Gate, TestServer, streamed};
use crate::test_sink::RecordingSink;
use crate::test_support::{content_chunk, dialect, finish_chunk, hello, tool_chunk};

const WAIT: Duration = Duration::from_secs(5);

struct Busy(tokio::task::JoinHandle<()>);

impl Busy {
    fn keep() -> Self {
        Self(tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        }))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn after_events(seen: &Arc<AtomicUsize>, count: usize) -> Gate {
    let seen = Arc::clone(seen);
    Box::pin(async move {
        while seen.load(Ordering::SeqCst) < count {
            tokio::task::yield_now().await;
        }
        advance(WAIT).await;
    })
}

fn after_quiet_time() -> Gate {
    Box::pin(async move {
        for _ in 0..500 {
            tokio::task::yield_now().await;
        }
        advance(WAIT).await;
    })
}

async fn timed(first: Vec<u8>, rest: Vec<u8>, gate_after_events: Option<usize>) -> CallReport {
    let seen = Arc::new(AtomicUsize::new(0));
    let gate_seen = Arc::clone(&seen);
    let server = TestServer::start(move |_| {
        let gate = match gate_after_events {
            Some(count) => after_events(&gate_seen, count),
            None => after_quiet_time(),
        };
        streamed(
            "200 OK",
            &[("content-type", "text/event-stream")],
            &first,
            vec![(gate, rest.clone())],
        )
    })
    .await;
    let spec = spec_for(dialect(), &server);
    let mut sink = RecordingSink::counting(&seen);
    let outcome = run(&spec, &hello(), &mut sink).await;
    assert!(outcome.result().is_ok());
    outcome.into_parts().0
}

fn begin_call() -> String {
    tool_chunk(
        json!([{"index": 0, "id": "c1", "type": "function", "function": {"name": "f", "arguments": "{}"}}]),
        None,
    )
}

#[tokio::test(start_paused = true)]
async fn given_text_before_a_pause_when_called_then_the_text_reaches_the_sink_before_the_stream_ends()
 {
    let _busy = Busy::keep();
    let first = sse(&[content_chunk("Hello")]);
    let rest = sse_with_sentinel(&[finish_chunk("stop")]);
    let report = timed(first, rest, Some(2)).await;
    assert_eq!(report.time_to_first_event(), Some(Duration::ZERO));
    assert_eq!(report.total_duration(), WAIT);
}

#[tokio::test(start_paused = true)]
async fn given_a_tool_call_before_a_pause_when_called_then_nothing_reaches_the_sink_before_the_finish()
 {
    let _busy = Busy::keep();
    let first = sse(&[begin_call()]);
    let rest = sse_with_sentinel(&[finish_chunk("tool_calls")]);
    let report = timed(first, rest, None).await;
    assert_eq!(report.time_to_first_event(), Some(WAIT));
    assert_eq!(report.total_duration(), WAIT);
}

#[tokio::test(start_paused = true)]
async fn given_text_then_a_tool_call_before_a_pause_when_called_then_only_the_text_is_early() {
    let _busy = Busy::keep();
    let first = sse(&[content_chunk("Checking."), begin_call()]);
    let rest = sse_with_sentinel(&[finish_chunk("tool_calls")]);
    let report = timed(first, rest, Some(2)).await;
    assert_eq!(report.time_to_first_event(), Some(Duration::ZERO));
    assert_eq!(report.total_duration(), WAIT);
}
