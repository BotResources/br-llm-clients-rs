use std::time::Duration;

use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{ContentBlock, ResponseDelta, ResponseEnding, ResponseMetadata};

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
fn given_a_success_when_called_then_the_sink_gets_every_delta_and_the_call_gives_the_response() {
    let response = text_response_with_usage("hello", usage(3, 4));
    let expected = expected_deltas(&request(), &response, &Chunking::none());
    let client = mock();
    client.push_reply(Reply::success(response.clone()).build().unwrap());
    let request_value = request();
    let (outcome, sink) = run(&client, &request_value);
    let given = response_from(&outcome);
    assert_eq!(given.id(), request_value.reply_entry());
    assert_eq!(given.ending(), response.ending());
    assert_eq!(given.usage(), response.usage());
    assert_eq!(text_of(&outcome), "hello");
    assert_eq!(sink.deltas().len(), expected.len());
    assert_eq!(sink.fold().unwrap(), given);
}

#[test]
fn given_a_success_when_called_then_the_report_usage_is_the_usage_of_the_response() {
    let (outcome, _) = serve_one(
        Reply::success(text_response_with_usage("hello", usage(3, 4)))
            .build()
            .unwrap(),
    );
    assert_eq!(outcome.report().usage(), Some(&usage(3, 4)));
}

#[test]
fn given_a_success_without_usage_when_called_then_the_report_has_no_usage() {
    let (outcome, _) = serve_one(Reply::success(text_response("hi")).build().unwrap());
    assert_eq!(outcome.report().usage(), None);
}

#[test]
fn given_a_success_with_no_report_values_when_called_then_only_the_zero_duration_is_present() {
    let (outcome, _) = serve_one(Reply::success(text_response("hi")).build().unwrap());
    let report = outcome.report();
    assert_eq!(report.total_duration(), Duration::ZERO);
    assert_eq!(report.model(), None);
    assert_eq!(report.request_id(), None);
    assert_eq!(report.time_to_first_event(), None);
}

#[test]
fn given_report_values_on_a_reply_when_called_then_the_report_holds_them_unmeasured() {
    let reply = Reply::success(text_response("hi"))
        .request_id("req-1")
        .time_to_first_event(Duration::from_millis(40))
        .total_duration(Duration::from_millis(900))
        .build()
        .unwrap();
    let (outcome, _) = serve_one(reply);
    let report = outcome.report();
    assert_eq!(report.request_id(), Some("req-1"));
    assert_eq!(
        report.time_to_first_event(),
        Some(Duration::from_millis(40))
    );
    assert_eq!(report.total_duration(), Duration::from_millis(900));
}

#[test]
fn given_a_response_without_model_when_called_then_the_response_and_the_report_have_none_not_the_settings_model()
 {
    let (outcome, deltas) = serve_one(Reply::success(text_response("hi")).build().unwrap());
    assert_eq!(response_from(&outcome).model(), None);
    assert_eq!(outcome.report().model(), None);
    let started = deltas.first().unwrap();
    assert!(
        matches!(started.op(), br_llm_messages::DeltaOp::ResponseStarted { metadata, .. } if metadata.model().is_none())
    );
}

#[test]
fn given_a_response_with_a_model_when_called_then_the_report_model_is_the_model_of_the_response() {
    let response = response_of(
        vec![text_content("hi")],
        ResponseEnding::finished(),
        ResponseMetadata::new().with_model(model("response-model")),
    );
    let (outcome, _) = serve_one(Reply::success(response).build().unwrap());
    assert_eq!(
        response_from(&outcome).model(),
        Some(&model("response-model"))
    );
    assert_eq!(outcome.report().model(), Some(&model("response-model")));
}

#[test]
fn given_a_response_with_a_tool_call_and_the_ending_finished_when_called_then_the_ending_is_tool_use_in_the_deltas_and_in_the_response()
 {
    let response = response_of(
        vec![call_content("search", "{}")],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    );
    assert_eq!(response.ending(), &ResponseEnding::tool_use());
    let with_text = response_of(
        vec![text_content("a"), call_content("search", "{}")],
        ResponseEnding::finished(),
        ResponseMetadata::new(),
    );
    let (outcome, deltas) = serve_one(Reply::success(with_text).build().unwrap());
    assert_eq!(
        response_from(&outcome).ending(),
        &ResponseEnding::tool_use()
    );
    let last = deltas.last().unwrap();
    assert!(matches!(
        last.op(),
        br_llm_messages::DeltaOp::ResponseCompleted { ending, .. } if *ending == ResponseEnding::tool_use()
    ));
}

#[test]
fn given_a_response_without_tool_call_when_called_then_its_own_ending_is_kept() {
    let response = response_of(
        vec![text_content("a")],
        ResponseEnding::output_exhaustion(),
        ResponseMetadata::new(),
    );
    let (outcome, deltas) = serve_one(Reply::success(response).build().unwrap());
    assert_eq!(
        response_from(&outcome).ending(),
        &ResponseEnding::output_exhaustion()
    );
    assert!(matches!(
        deltas.last().unwrap().op(),
        br_llm_messages::DeltaOp::ResponseCompleted { ending, .. } if *ending == ResponseEnding::output_exhaustion()
    ));
}

#[test]
fn given_a_failure_when_called_then_no_delta_is_sent_and_the_call_gives_the_error() {
    let (outcome, deltas) = serve_one(
        Reply::failure(ClientError::RateLimited { retry_after: None })
            .build()
            .unwrap(),
    );
    assert!(deltas.is_empty());
    assert_eq!(
        error_of(&outcome),
        ClientError::RateLimited { retry_after: None }
    );
}

#[test]
fn given_a_failure_with_named_values_when_called_then_the_report_holds_them() {
    let reply = Reply::failure(ClientError::Transport)
        .usage(usage(1, 2))
        .model(model("m"))
        .request_id("r")
        .total_duration(Duration::from_secs(1))
        .build()
        .unwrap();
    let (outcome, _) = serve_one(reply);
    assert_eq!(outcome.report().usage(), Some(&usage(1, 2)));
    assert_eq!(outcome.report().model(), Some(&model("m")));
    assert_eq!(outcome.report().request_id(), Some("r"));
    assert_eq!(outcome.report().total_duration(), Duration::from_secs(1));
}

#[test]
fn given_a_failure_without_named_values_when_called_then_the_report_has_neither_usage_nor_model() {
    let (outcome, _) = serve_one(Reply::failure(ClientError::Transport).build().unwrap());
    assert_eq!(outcome.report().usage(), None);
    assert_eq!(outcome.report().model(), None);
}

#[test]
fn given_any_client_error_when_used_in_a_failure_then_the_mock_does_not_judge_it() {
    for error in [
        ClientError::ModelNotFound,
        ClientError::ReasoningNotSupported {
            level: br_llm_clients_core::ReasoningLevel::Max,
            reason: "x".to_owned(),
        },
        ClientError::ContentBlocked,
        ClientError::ProviderFailed {
            status: Some(500),
            code: None,
            body: "b".to_owned(),
        },
    ] {
        let (outcome, _) = serve_one(Reply::failure(error.clone()).build().unwrap());
        assert_eq!(error_of(&outcome), error);
    }
}

#[test]
fn given_a_success_with_blocks_when_called_then_the_blocks_keep_their_order_and_kinds() {
    let response = response_of(
        vec![text_content("a"), call_content("search", "{}")],
        ResponseEnding::tool_use(),
        ResponseMetadata::new(),
    );
    let (outcome, _) = serve_one(Reply::success(response).build().unwrap());
    let kinds: Vec<_> = response_from(&outcome)
        .blocks()
        .iter()
        .map(ContentBlock::kind)
        .collect();
    assert_eq!(
        kinds,
        [
            br_llm_messages::ContentBlockKind::Text,
            br_llm_messages::ContentBlockKind::RuntimeToolCall
        ]
    );
}
