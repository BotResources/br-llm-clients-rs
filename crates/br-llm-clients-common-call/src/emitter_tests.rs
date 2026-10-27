use br_llm_clients_core::ClientError;
use br_llm_messages::{BlockStart, DeltaOp, ResponseDelta, ResponseMetadata};

use crate::call_record::CallRecord;
use crate::emitter::Emitter;
use crate::test_fixtures::user_request;
use crate::test_ids::{block_of, reply_entry, reply_turn};
use crate::test_sink::RecordingSink;
use crate::test_stream::kinds;

fn delta(op: DeltaOp) -> ResponseDelta {
    ResponseDelta::new(reply_turn(), reply_entry(), op)
}

fn started() -> ResponseDelta {
    delta(DeltaOp::response_started(ResponseMetadata::new()))
}

fn block_start(n: u64) -> ResponseDelta {
    delta(DeltaOp::block_started(block_of(n), BlockStart::text()))
}

#[tokio::test]
async fn given_a_delta_the_draft_refuses_when_emitted_then_the_reply_is_malformed_and_the_sink_gets_nothing()
 {
    let mut sink = RecordingSink::default();
    let mut record = CallRecord::start();
    let request = user_request();
    let mut emitter = Emitter::new(&request, &mut sink);
    let error = emitter
        .emit(block_start(1), &mut record)
        .await
        .expect_err("refused");
    assert!(matches!(error, ClientError::MalformedResponse { .. }));
    assert!(record.time_to_first_event.is_none());
    drop(emitter);
    assert!(sink.deltas.is_empty());
}

#[tokio::test]
async fn given_deltas_the_draft_accepts_when_emitted_then_the_sink_gets_them_in_order_and_the_deltas_already_sent_stand()
 {
    let mut sink = RecordingSink::default();
    let mut record = CallRecord::start();
    let request = user_request();
    let mut emitter = Emitter::new(&request, &mut sink);
    emitter.emit(started(), &mut record).await.expect("sent");
    emitter
        .emit(block_start(1), &mut record)
        .await
        .expect("sent");
    let refused = emitter.emit(block_start(1), &mut record).await;
    assert!(refused.is_err());
    assert!(record.time_to_first_event.is_some());
    drop(emitter);
    assert_eq!(kinds(&sink.deltas), ["started", "start 1 text"]);
}

#[tokio::test]
async fn given_a_delta_for_another_entry_when_emitted_then_the_draft_refuses_it() {
    let mut sink = RecordingSink::default();
    let mut record = CallRecord::start();
    let request = user_request();
    let mut emitter = Emitter::new(&request, &mut sink);
    let other = ResponseDelta::new(
        reply_turn(),
        br_llm_messages::EntryId::from_uuid(crate::test_ids::uuid_of(0x2001)).expect("entry"),
        DeltaOp::response_started(ResponseMetadata::new()),
    );
    assert!(emitter.emit(other, &mut record).await.is_err());
    drop(emitter);
    assert!(sink.deltas.is_empty());
}
