use std::time::Duration;

use br_llm_clients_core::{CallOutcome, ClientError};
use br_llm_messages::{Step, StopReason, StreamEvent};

use crate::chunking::Chunking;
use crate::derive_events::derive_events;
use crate::reply::Reply;
use crate::test_support::*;

fn serve_one(reply: Reply) -> (CallOutcome, Vec<StreamEvent>) {
    let client = mock();
    client.push_reply(reply);
    let (outcome, sink) = run(&client, &request());
    (outcome, sink.events())
}

fn all_events(step_value: &Step) -> Vec<StreamEvent> {
    derive_events(step_value, &Chunking::none())
        .unwrap()
        .into_events()
}

#[test]
fn given_a_success_when_called_then_the_sink_gets_the_events_and_the_call_gives_the_step() {
    let step_value = step_with_usage("hello", usage(3, 4));
    let (outcome, events) = serve_one(Reply::success(step_value.clone()).build().unwrap());
    assert_eq!(events, all_events(&step_value));
    let given = step_of(&outcome);
    assert_eq!(given.content(), step_value.content());
    assert_eq!(given.stop_reason(), step_value.stop_reason());
}

#[test]
fn given_a_success_when_called_then_the_report_usage_is_the_usage_of_the_step() {
    let (outcome, _) = serve_one(
        Reply::success(step_with_usage("hello", usage(3, 4)))
            .build()
            .unwrap(),
    );
    assert_eq!(outcome.report().usage(), Some(&usage(3, 4)));
}

#[test]
fn given_a_success_without_usage_when_called_then_the_report_has_no_usage() {
    let (outcome, _) = serve_one(Reply::success(text_step("hi")).build().unwrap());
    assert_eq!(outcome.report().usage(), None);
}

#[test]
fn given_a_success_with_no_report_values_when_called_then_only_the_zero_duration_is_present() {
    let (outcome, _) = serve_one(Reply::success(text_step("hi")).build().unwrap());
    let report = outcome.report();
    assert_eq!(report.total_duration(), Duration::ZERO);
    assert_eq!(report.model(), None);
    assert_eq!(report.request_id(), None);
    assert_eq!(report.time_to_first_event(), None);
}

#[test]
fn given_report_values_on_a_reply_when_called_then_the_report_holds_them_unmeasured() {
    let reply = Reply::success(text_step("hi"))
        .model(model("answering-model"))
        .request_id("req-1")
        .time_to_first_event(Duration::from_millis(40))
        .total_duration(Duration::from_millis(900))
        .build()
        .unwrap();
    let (outcome, _) = serve_one(reply);
    let report = outcome.report();
    assert_eq!(report.model(), Some(&model("answering-model")));
    assert_eq!(report.request_id(), Some("req-1"));
    assert_eq!(
        report.time_to_first_event(),
        Some(Duration::from_millis(40))
    );
    assert_eq!(report.total_duration(), Duration::from_millis(900));
}

#[test]
fn given_a_step_without_model_and_a_reply_model_when_called_then_the_step_gets_the_reply_model() {
    let reply = Reply::success(text_step("hi"))
        .model(model("reply-model"))
        .build()
        .unwrap();
    let (outcome, _) = serve_one(reply);
    assert_eq!(step_of(&outcome).model(), Some(&model("reply-model")));
}

#[test]
fn given_a_step_without_model_and_no_reply_model_when_called_then_the_step_gets_the_settings_model_and_the_report_none()
 {
    let (outcome, _) = serve_one(Reply::success(text_step("hi")).build().unwrap());
    assert_eq!(step_of(&outcome).model(), Some(&model("settings-model")));
    assert_eq!(outcome.report().model(), None);
}

#[test]
fn given_a_step_with_its_own_model_when_called_then_it_is_kept_and_may_differ_from_the_report() {
    let step_value = Step::new(
        step_of(&serve_one(Reply::success(text_step("hi")).build().unwrap()).0)
            .content()
            .to_vec(),
        StopReason::EndTurn,
        None,
        Some(model("step-model")),
    )
    .unwrap();
    let reply = Reply::success(step_value)
        .model(model("report-model"))
        .build()
        .unwrap();
    let (outcome, _) = serve_one(reply);
    assert_eq!(step_of(&outcome).model(), Some(&model("step-model")));
    assert_eq!(outcome.report().model(), Some(&model("report-model")));
}

#[test]
fn given_a_failure_when_called_then_no_event_is_sent_and_the_call_gives_the_error() {
    let (outcome, events) = serve_one(
        Reply::failure(ClientError::RateLimited { retry_after: None })
            .build()
            .unwrap(),
    );
    assert!(events.is_empty());
    assert_eq!(
        error_of(&outcome),
        ClientError::RateLimited { retry_after: None }
    );
}

#[test]
fn given_a_failure_with_a_named_usage_when_called_then_the_report_holds_that_usage() {
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
fn given_a_failure_without_named_usage_when_called_then_the_report_has_none() {
    let (outcome, _) = serve_one(Reply::failure(ClientError::Transport).build().unwrap());
    assert_eq!(outcome.report().usage(), None);
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
fn given_a_cut_when_called_then_the_first_events_are_sent_then_the_error_is_given() {
    let step_value = text_step("hello world");
    let events = all_events(&step_value);
    let reply = Reply::cut(step_value, 2, ClientError::StreamTruncated)
        .usage(usage(9, 1))
        .build()
        .unwrap();
    let (outcome, sent) = serve_one(reply);
    assert_eq!(sent, events[..2]);
    assert_eq!(error_of(&outcome), ClientError::StreamTruncated);
    assert_eq!(outcome.report().usage(), Some(&usage(9, 1)));
}

#[test]
fn given_a_cut_when_the_step_has_usage_then_the_report_never_takes_the_usage_of_the_step() {
    let reply = Reply::cut(
        step_with_usage("hi", usage(5, 5)),
        1,
        ClientError::Transport,
    )
    .build()
    .unwrap();
    let (outcome, _) = serve_one(reply);
    assert_eq!(outcome.report().usage(), None);
}

#[test]
fn given_a_cut_at_zero_when_called_then_it_acts_like_a_failure_that_sends_no_event() {
    let (outcome, sent) = serve_one(
        Reply::cut(text_step("x"), 0, ClientError::Transport)
            .build()
            .unwrap(),
    );
    assert!(sent.is_empty());
    assert_eq!(error_of(&outcome), ClientError::Transport);
}

#[test]
fn given_a_cut_at_the_full_count_when_called_then_every_event_but_the_finish_is_sent() {
    let step_value = text_step("hi");
    let events = all_events(&step_value);
    let count = events.len() - 1;
    let (_, sent) = serve_one(
        Reply::cut(step_value, count, ClientError::Transport)
            .build()
            .unwrap(),
    );
    assert_eq!(sent, events[..count]);
}

#[test]
fn given_a_cut_with_chunking_when_called_then_the_count_follows_the_pieces() {
    let step_value = text_step("abcdef");
    let chunking = Chunking::none().text(2);
    let events = derive_events(&step_value, &chunking).unwrap().into_events();
    let reply = Reply::cut(step_value, 3, ClientError::Transport)
        .chunking(chunking)
        .build()
        .unwrap();
    let (_, sent) = serve_one(reply);
    assert_eq!(sent, events[..3]);
}

#[test]
fn given_a_broken_stream_when_called_then_the_list_is_sent_as_given_finish_included_then_the_error()
{
    let list = vec![
        StreamEvent::TextDelta {
            index: 7,
            text: "orphan".to_owned(),
        },
        StreamEvent::Finish {
            stop_reason: StopReason::EndTurn,
            usage: None,
        },
        StreamEvent::BlockEnd { index: 1 },
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
fn given_a_tool_call_step_with_chunking_when_called_then_the_events_fold_to_the_step() {
    let client = mock();
    let reply = Reply::success(calls_step(&["a", "b"]))
        .chunking(Chunking::none().tool_arguments(2))
        .build()
        .unwrap();
    client.push_reply(reply);
    let (outcome, sink) = run(&client, &request());
    let folded = sink.fold(step_of(&outcome).model().cloned()).unwrap();
    assert_eq!(folded, step_of(&outcome));
}
