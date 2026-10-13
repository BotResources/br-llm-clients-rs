use br_llm_clients_core::Client;
use br_llm_messages::ImageMime;

use crate::accepted::Accepted;
use crate::build_error::MockBuildError;
use crate::call_log::{CallState, LoggedCall, Served, StateKind};
use crate::call_view::{MatchContext, ReplyContext};
use crate::capabilities::{Capabilities, CapabilitiesSpec};
use crate::capabilities_error::CapabilitiesError;
use crate::chunking::Chunking;
use crate::derive_events::DerivedEvents;
use crate::fold_error::FoldError;
use crate::gate::{Gate, WaitForWaiting};
use crate::log_disabled::LogDisabled;
use crate::mock_builder::MockBuilder;
use crate::mock_client::MockClient;
use crate::mock_waits::{WaitForCalls, WaitForState};
use crate::recording_sink::{RecordingSink, WaitForEvents};
use crate::reply::Reply;
use crate::reply_builder::{
    BrokenStreamKind, CutKind, FailureKind, HangKind, ReplyBuilder, SuccessKind,
};
use crate::reply_source::ReplySource;
use crate::rule::Rule;

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

fn assert_future_traits<T: Send + Sync + Unpin + 'static>() {}

#[test]
fn given_every_public_type_when_compiled_then_it_is_send_and_sync() {
    assert_send_sync::<Accepted<ImageMime>>();
    assert_send_sync::<BrokenStreamKind>();
    assert_send_sync::<CallState>();
    assert_send_sync::<Capabilities>();
    assert_send_sync::<CapabilitiesError>();
    assert_send_sync::<CapabilitiesSpec>();
    assert_send_sync::<Chunking>();
    assert_send_sync::<CutKind>();
    assert_send_sync::<DerivedEvents>();
    assert_send_sync::<FailureKind>();
    assert_send_sync::<FoldError>();
    assert_send_sync::<Gate>();
    assert_send_sync::<HangKind>();
    assert_send_sync::<LogDisabled>();
    assert_send_sync::<LoggedCall>();
    assert_send_sync::<MatchContext<'static>>();
    assert_send_sync::<MockBuildError>();
    assert_send_sync::<MockBuilder>();
    assert_send_sync::<MockClient>();
    assert_send_sync::<RecordingSink>();
    assert_send_sync::<Reply>();
    assert_send_sync::<ReplyBuilder<SuccessKind>>();
    assert_send_sync::<ReplyContext<'static>>();
    assert_send_sync::<ReplySource>();
    assert_send_sync::<Rule>();
    assert_send_sync::<Served>();
    assert_send_sync::<StateKind>();
    assert_send_sync::<SuccessKind>();
    assert_send_sync::<dyn Client>();
}

#[test]
fn given_the_four_wait_futures_when_compiled_then_they_are_send_sync_unpin_and_static() {
    assert_future_traits::<WaitForCalls>();
    assert_future_traits::<WaitForEvents>();
    assert_future_traits::<WaitForState>();
    assert_future_traits::<WaitForWaiting>();
}
