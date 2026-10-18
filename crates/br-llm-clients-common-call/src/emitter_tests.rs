use br_llm_clients_core::ClientError;
use br_llm_messages::{BlockKind, StreamEvent};

use crate::call_record::CallRecord;
use crate::emitter::Emitter;
use crate::test_sink::RecordingSink;

fn start() -> StreamEvent {
    StreamEvent::BlockStart {
        index: 0,
        kind: BlockKind::Text,
    }
}

#[tokio::test]
async fn given_an_event_the_draft_refuses_when_emitted_then_the_reply_is_malformed_and_the_sink_gets_nothing()
 {
    let mut sink = RecordingSink::default();
    let mut record = CallRecord::start();
    let mut emitter = Emitter::new(&mut sink);
    let error = emitter
        .emit(StreamEvent::BlockEnd { index: 0 }, &mut record)
        .await
        .expect_err("refused");
    assert!(matches!(error, ClientError::MalformedResponse { .. }));
    assert!(emitter.log().is_empty());
    assert!(record.time_to_first_event.is_none());
    drop(emitter);
    assert!(sink.events.is_empty());
}

#[tokio::test]
async fn given_events_the_draft_accepts_when_emitted_then_the_sink_gets_them_in_order_and_the_events_already_sent_stand()
 {
    let mut sink = RecordingSink::default();
    let mut record = CallRecord::start();
    let mut emitter = Emitter::new(&mut sink);
    emitter.emit(start(), &mut record).await.expect("sent");
    let refused = emitter.emit(start(), &mut record).await;
    assert!(refused.is_err());
    assert_eq!(emitter.log(), [start()]);
    assert!(record.time_to_first_event.is_some());
    drop(emitter);
    assert_eq!(sink.events, vec![start()]);
}
