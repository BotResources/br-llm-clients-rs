use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{DeltaOp, InterruptionReason, ResponseDelta};

use crate::chunking::Chunking;
use crate::reply::Reply;
use crate::test_support::*;

fn serve_one(reply: Reply) -> (CallOutcome, Vec<ResponseDelta>) {
    let client = mock();
    client.push_reply(reply);
    let (outcome, sink) = run(&client, &request());
    (outcome, sink.deltas())
}

#[test]
fn given_a_cut_when_called_then_the_first_deltas_are_sent_then_the_error_is_given() {
    let response = text_response("hello world");
    let expected = expected_deltas(&request(), &response, &Chunking::none());
    let reply = Reply::cut(response, 2, ClientError::StreamTruncated)
        .usage(usage(9, 1))
        .build()
        .unwrap();
    let (outcome, sent) = serve_one(reply);
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].kind(), expected[0].kind());
    assert_eq!(sent[1].kind(), expected[1].kind());
    assert_eq!(error_of(&outcome), ClientError::StreamTruncated);
    assert_eq!(outcome.report().usage(), Some(&usage(9, 1)));
}

#[test]
fn given_a_cut_when_the_response_has_usage_and_model_then_the_report_never_takes_them_from_it() {
    let response = response_of(
        vec![text_content("hi")],
        br_llm_messages::ResponseEnding::finished(),
        br_llm_messages::ResponseMetadata::new()
            .with_usage(usage(5, 5))
            .with_model(model("response-model")),
    );
    let reply = Reply::cut(response, 1, ClientError::Transport)
        .build()
        .unwrap();
    let (outcome, _) = serve_one(reply);
    assert_eq!(outcome.report().usage(), None);
    assert_eq!(outcome.report().model(), None);
}

#[test]
fn given_a_cut_at_zero_when_called_then_no_delta_is_sent_but_the_identities_are_taken() {
    let source = crate::sequential_id_source::SequentialIdSource::new();
    let client = crate::mock_client::MockClient::builder(settings(), everything_caps())
        .id_source(source.clone())
        .build();
    client.push_reply(
        Reply::cut(text_response("x"), 0, ClientError::Transport)
            .build()
            .unwrap(),
    );
    let (outcome, sink) = run(&client, &request());
    assert!(sink.is_empty());
    assert_eq!(error_of(&outcome), ClientError::Transport);
    assert_eq!(source.given(), 1);
}

#[test]
fn given_a_cut_at_the_full_count_when_called_then_every_delta_but_the_completed_is_sent() {
    let response = text_response("hi");
    let expected = expected_deltas(&request(), &response, &Chunking::none());
    let count = expected.len() - 1;
    let (_, sent) = serve_one(
        Reply::cut(response, count, ClientError::Transport)
            .build()
            .unwrap(),
    );
    assert_eq!(sent.len(), count);
    assert!(
        !sent
            .iter()
            .any(|delta| matches!(delta.op(), DeltaOp::ResponseCompleted { .. }))
    );
}

#[test]
fn given_a_cut_with_chunking_when_called_then_the_count_follows_the_pieces() {
    let response = text_response("abcdef");
    let chunking = Chunking::none().text(2);
    let reply = Reply::cut(response.clone(), 5, ClientError::Transport)
        .chunking(chunking)
        .build()
        .unwrap();
    let (_, sent) = serve_one(reply);
    let expected = expected_deltas(&request(), &response, &chunking);
    assert_eq!(sent, expected[..5]);
    assert!(matches!(sent[2].op(), DeltaOp::TextAppended { text, .. } if text == "ab"));
    assert!(matches!(sent[4].op(), DeltaOp::TextAppended { text, .. } if text == "ef"));
}

#[test]
fn given_a_broken_stream_when_called_then_the_list_is_sent_as_given_completed_and_interrupted_included_then_the_error()
 {
    let list = vec![
        started_delta(),
        delta(DeltaOp::response_interrupted(InterruptionReason::failed())),
        delta(DeltaOp::response_completed(
            br_llm_messages::ResponseEnding::finished(),
        )),
    ];
    let reply = Reply::broken_stream(list.clone(), ClientError::Transport)
        .usage(usage(2, 2))
        .build()
        .unwrap();
    let (outcome, sent) = serve_one(reply);
    assert_eq!(sent, list);
    assert_eq!(error_of(&outcome), ClientError::Transport);
    assert_eq!(outcome.report().usage(), Some(&usage(2, 2)));
}

#[test]
fn given_a_broken_stream_with_request_identities_when_called_then_every_delta_takes_the_turn_and_entry_of_the_request()
 {
    let list = vec![started_delta(), started_delta()];
    let reply = Reply::broken_stream(list, ClientError::Transport)
        .with_request_identities()
        .build()
        .unwrap();
    let client = mock();
    client.push_reply(reply);
    let request_value = request();
    let (_, sink) = run(&client, &request_value);
    assert_eq!(sink.len(), 2);
    for sent in sink.deltas() {
        assert_eq!(sent.turn(), request_value.reply_turn());
        assert_eq!(sent.entry(), request_value.reply_entry());
    }
}

#[test]
fn given_a_broken_stream_without_the_request_identities_when_called_then_the_deltas_keep_the_identities_they_hold()
 {
    let list = vec![started_delta()];
    let reply = Reply::broken_stream(list.clone(), ClientError::Transport)
        .build()
        .unwrap();
    let (_, sent) = serve_one(reply);
    assert_eq!(sent, list);
    assert_ne!(sent[0].entry(), request().reply_entry());
}

#[test]
fn given_a_tool_call_response_with_chunking_when_called_then_the_deltas_fold_to_the_response() {
    let client = mock();
    let reply = Reply::success(calls_response(2))
        .chunking(Chunking::none().tool_arguments(2))
        .build()
        .unwrap();
    client.push_reply(reply);
    let (outcome, sink) = run(&client, &request());
    assert_eq!(sink.fold().unwrap(), response_from(&outcome));
}
