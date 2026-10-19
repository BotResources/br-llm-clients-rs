use br_llm_messages::AssistantBlock;

use crate::payload::Payload;
use crate::test_decoder::EndScript;
use crate::test_fixtures::user_request;
use crate::test_stream::{body, dialect_ending, drive, drive_one, frame, kinds};

const STREAM: &str = concat!(
    ": a comment\r\n",
    "event: delta\r\n",
    "data: start a text\r\n",
    "data: text a h\u{e9}llo\r\n",
    "\r\n",
    "data: start b tool search call_1\r",
    "data: args b {\"q\":\r",
    "data: args b \"caf\u{e9}\"}\r",
    "\r",
    "event: tail\n",
    "data: end a\n",
    "data: end b\n",
    "data: usage 3 4\n",
    "data: model m-1\n",
    "data: stop tool\n",
    "\n",
);

#[tokio::test]
async fn given_a_stream_when_cut_at_every_byte_boundary_then_events_payloads_and_step_match_one_chunk()
 {
    let bytes = STREAM.as_bytes().to_vec();
    let reference_dialect = dialect_ending(EndScript::Events(Vec::new()));
    let reference = drive_one(&reference_dialect, &user_request(), bytes.clone()).await;
    let step = reference.result.clone().expect("a step");
    assert!(!reference.sink.events.is_empty());
    assert_eq!(step.content().len(), 2);
    let payloads = reference_dialect.log.payloads();
    assert_eq!(payloads.len(), 3);
    for cut in 1..bytes.len() {
        let dialect = dialect_ending(EndScript::Events(Vec::new()));
        let chunks = [bytes[..cut].to_vec(), bytes[cut..].to_vec()];
        let driven = drive(&dialect, &user_request(), &chunks).await;
        assert_eq!(driven.result, Ok(step.clone()), "cut at {cut}");
        assert_eq!(driven.sink.events, reference.sink.events, "cut at {cut}");
        assert_eq!(dialect.log.payloads(), payloads, "cut at {cut}");
    }
}

#[tokio::test]
async fn given_a_stream_when_sent_one_byte_at_a_time_then_the_events_match_one_chunk() {
    let bytes = STREAM.as_bytes().to_vec();
    let reference = drive_one(
        &dialect_ending(EndScript::Events(Vec::new())),
        &user_request(),
        bytes.clone(),
    )
    .await;
    let chunks: Vec<Vec<u8>> = bytes.iter().map(|byte| vec![*byte]).collect();
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let driven = drive(&dialect, &user_request(), &chunks).await;
    assert_eq!(driven.result, reference.result);
    assert_eq!(driven.sink.events, reference.sink.events);
}

#[tokio::test]
async fn given_the_frames_of_a_stream_when_read_then_each_payload_keeps_its_event_name_and_data() {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    drive_one(&dialect, &user_request(), STREAM.as_bytes().to_vec()).await;
    let payloads = dialect.log.payloads();
    assert_eq!(
        payloads[0],
        Payload::new(Some("delta".to_owned()), "start a text\ntext a h\u{e9}llo")
    );
    assert_eq!(payloads[1].event(), None);
    assert_eq!(payloads[2].event(), Some("tail"));
    let silent = dialect_ending(EndScript::Events(Vec::new()));
    drive_one(
        &silent,
        &user_request(),
        body(&[&["start a text", "text a x", "end a", "stop end_turn"]]),
    )
    .await;
    assert_eq!(silent.log.payloads()[0].event(), None);
}

#[tokio::test]
async fn given_a_stream_when_read_then_the_decoder_sees_one_payload_only_after_the_sink_took_the_events_of_the_one_before()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let mut bytes = frame(&["start a text", "text a one"]);
    bytes.extend(frame(&["text a two", "end a"]));
    bytes.extend(frame(&["stop end_turn"]));
    let driven = drive_one(&dialect, &user_request(), bytes).await;
    assert_eq!(driven.sink.payloads_seen, vec![1, 1, 2, 2, 3]);
    assert_eq!(
        kinds(&driven.sink.events),
        vec![
            "start 0 Text",
            "text 0 \"one\"",
            "text 0 \"two\"",
            "end 0",
            "finish end_turn false"
        ]
    );
}

#[tokio::test]
async fn given_a_stream_split_in_the_middle_of_a_multibyte_character_when_read_then_the_text_is_whole()
 {
    let dialect = dialect_ending(EndScript::Events(Vec::new()));
    let bytes = body(&[&[
        "start a text",
        "text a \u{1f600}x",
        "end a",
        "stop end_turn",
    ]]);
    let cut = bytes
        .windows(4)
        .position(|window| window == "\u{1f600}".as_bytes())
        .expect("the character")
        + 2;
    let chunks = [bytes[..cut].to_vec(), bytes[cut..].to_vec()];
    let driven = drive(&dialect, &user_request(), &chunks).await;
    let step = driven.result.expect("a step");
    let Some(AssistantBlock::Text { text }) = step.content().first() else {
        panic!("not text");
    };
    assert_eq!(text.as_str(), "\u{1f600}x");
}
