use std::sync::Arc;
use std::time::Duration;

use br_llm_clients_core::ClientError;
use tokio::time::advance;

use crate::test_decoder::DecoderLog;
use crate::test_decoder::EndScript;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{Busy, spec_for, user_request};
use crate::test_server::{Gate, TestServer, streamed};
use crate::test_sink::RecordingSink;
use crate::test_stream::{body, dialect_ending};

const SECOND: Duration = Duration::from_secs(1);

fn calm() -> Arc<ScriptedDialect> {
    dialect_ending(EndScript::Events(Vec::new()))
}

fn seconds(count: u64) -> Duration {
    Duration::from_secs(count)
}

fn after_payloads(log: &Arc<DecoderLog>, seen: usize, wait: Duration) -> Gate {
    let log = Arc::clone(log);
    Box::pin(async move {
        while log.payload_count() < seen {
            tokio::task::yield_now().await;
        }
        advance(wait).await;
    })
}

struct Script {
    first_wait: Duration,
    first: Vec<u8>,
    later: Vec<(usize, Duration, Vec<u8>)>,
}

fn script(first_wait: Duration, first: Vec<u8>) -> Script {
    Script {
        first_wait,
        first,
        later: Vec::new(),
    }
}

impl Script {
    fn then(mut self, seen: usize, wait: Duration, bytes: Vec<u8>) -> Self {
        self.later.push((seen, wait, bytes));
        self
    }
}

async fn timed(
    dialect: &Arc<ScriptedDialect>,
    script: Script,
    sink: &mut RecordingSink,
) -> br_llm_clients_core::CallOutcome {
    let log = Arc::clone(&dialect.log);
    let server = TestServer::start(move |_| {
        let later = script
            .later
            .iter()
            .map(|(seen, wait, bytes)| (after_payloads(&log, *seen, *wait), bytes.clone()))
            .collect();
        let first_wait = script.first_wait;
        streamed("200 OK", &[], &script.first, later).after(async move {
            advance(first_wait).await;
        })
    })
    .await;
    let spec = spec_for(dialect, server.endpoint("/v1/test"));
    crate::run::run(&spec, &user_request(), sink).await
}

async fn timed_by(
    first_wait: Duration,
    first: Vec<u8>,
    seen: usize,
    wait: Duration,
    rest: Vec<u8>,
    sink: &mut RecordingSink,
) -> br_llm_clients_core::CallOutcome {
    let dialect = calm();
    let plan = script(first_wait, first).then(seen, wait, rest);
    timed(&dialect, plan, sink).await
}

#[tokio::test(start_paused = true)]
async fn given_a_first_event_after_two_seconds_and_an_end_after_five_when_run_then_the_times_are_two_and_five()
 {
    let _busy = Busy::keep();
    let first = body(&[&["start a text", "text a hi"]]);
    let rest = body(&[&["end a", "stop finished"]]);
    let mut sink = RecordingSink::default();
    let outcome = timed_by(seconds(2), first, 1, seconds(3), rest, &mut sink).await;
    let report = outcome.into_parts().0;
    assert_eq!(report.time_to_first_event(), Some(seconds(2)));
    assert_eq!(report.total_duration(), seconds(5));
}

#[tokio::test(start_paused = true)]
async fn given_a_first_payload_that_gives_no_event_when_run_then_the_first_event_time_is_that_of_the_first_sink_call()
 {
    let _busy = Busy::keep();
    let first = b"data:\ndata:\n\n".to_vec();
    let rest = body(&[&["start a text", "text a hi", "end a", "stop finished"]]);
    let mut sink = RecordingSink::default();
    let outcome = timed_by(seconds(2), first, 1, seconds(3), rest, &mut sink).await;
    let report = outcome.into_parts().0;
    assert_eq!(report.time_to_first_event(), Some(seconds(5)));
    assert_eq!(report.total_duration(), seconds(5));
}

#[tokio::test(start_paused = true)]
async fn given_a_sink_that_waits_a_second_per_event_when_run_then_the_first_event_time_stops_before_the_sink_and_the_total_counts_every_wait()
 {
    let _busy = Busy::keep();
    let first = body(&[&["start a text", "text a hi", "end a", "stop finished"]]);
    let mut sink = RecordingSink {
        pause: Some(SECOND),
        ..RecordingSink::default()
    };
    let outcome = timed(&calm(), script(Duration::ZERO, first), &mut sink).await;
    let report = outcome.into_parts().0;
    assert_eq!(sink.deltas.len(), 5);
    assert_eq!(report.time_to_first_event(), Some(Duration::ZERO));
    assert_eq!(report.total_duration(), seconds(5));
}

#[tokio::test(start_paused = true)]
async fn given_a_sink_that_waits_when_the_next_payload_is_ready_then_it_is_decoded_after_the_sink_returned()
 {
    let _busy = Busy::keep();
    let dialect = calm();
    let mut bytes = body(&[&["start a text", "text a one"]]);
    bytes.extend(body(&[&["text a two", "end a"]]));
    bytes.extend(body(&[&["stop finished"]]));
    let mut sink = RecordingSink::watching(&dialect.log);
    sink.pause = Some(SECOND);
    let outcome = timed(&dialect, script(Duration::ZERO, bytes), &mut sink).await;
    assert!(outcome.into_parts().1.is_ok());
    assert_eq!(sink.payloads_seen, vec![1, 1, 1, 2, 2, 3]);
}

#[tokio::test(start_paused = true)]
async fn given_a_failure_after_an_event_when_run_then_the_report_holds_the_first_event_time_and_the_total()
 {
    let _busy = Busy::keep();
    let first = body(&[&["usage 1 2", "start a text", "text a hi"]]);
    let rest = body(&[&["error 500 - - boom"]]);
    let mut sink = RecordingSink::default();
    let outcome = timed_by(seconds(1), first, 1, seconds(4), rest, &mut sink).await;
    let (report, result) = outcome.into_parts();
    assert!(matches!(result, Err(ClientError::ProviderFailed { .. })));
    assert_eq!(report.time_to_first_event(), Some(seconds(1)));
    assert_eq!(report.total_duration(), seconds(5));
    assert_eq!(
        report.usage().map(|usage| usage.output_tokens()),
        Some(Some(2))
    );
}

#[tokio::test(start_paused = true)]
async fn given_a_stream_with_no_event_for_the_sink_when_run_then_the_first_event_time_is_absent() {
    let _busy = Busy::keep();
    let first = b"data:\ndata:\n\n".to_vec();
    let mut sink = RecordingSink::default();
    let outcome = timed(&calm(), script(seconds(1), first), &mut sink).await;
    let (report, result) = outcome.into_parts();
    assert_eq!(result, Err(ClientError::StreamTruncated));
    assert_eq!(report.time_to_first_event(), None);
    assert_eq!(report.total_duration(), seconds(1));
    assert!(report.model().is_none());
    assert!(report.usage().is_none());
}

#[tokio::test(start_paused = true)]
async fn given_a_payload_of_metadata_only_when_run_then_the_start_of_the_reply_counts_as_the_first_event()
 {
    let _busy = Busy::keep();
    let first = body(&[&["usage 1 2", "model m"]]);
    let mut sink = RecordingSink::default();
    let outcome = timed(&calm(), script(seconds(1), first), &mut sink).await;
    let (report, result) = outcome.into_parts();
    assert_eq!(result, Err(ClientError::StreamTruncated));
    assert_eq!(report.time_to_first_event(), Some(seconds(1)));
    assert_eq!(report.total_duration(), seconds(1));
    assert_eq!(report.model().map(|model| model.as_str()), Some("m"));
    assert!(report.usage().is_some());
}

#[tokio::test(start_paused = true)]
async fn given_a_failure_and_no_usage_event_when_run_then_the_report_holds_no_usage_and_no_model() {
    let _busy = Busy::keep();
    let first = body(&[&["start a text", "text a hi", "text zz bad"]]);
    let mut sink = RecordingSink::default();
    let outcome = timed(&calm(), script(Duration::ZERO, first), &mut sink).await;
    let report = outcome.into_parts().0;
    assert!(report.usage().is_none());
    assert!(report.model().is_none());
    assert_eq!(report.time_to_first_event(), Some(Duration::ZERO));
}
