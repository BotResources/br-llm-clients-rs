use std::fmt::Debug;

use br_llm_messages::MediaType;

use crate::{
    Accepted, BuildError, CallOutcome, CallReport, Capabilities, CapabilitiesSpec, ClientError,
    ClientRequest, ClientSettings, OutputMode, ReasoningLevel, ToolSpec,
};

trait AmbiguousIfEq<Marker> {
    fn proof() {}
}
impl<T: ?Sized> AmbiguousIfEq<()> for T {}
impl<T: ?Sized + Eq> AmbiguousIfEq<u8> for T {}

fn assert_eq_type<T: Eq>() {}

fn assert_data<T: Debug + Clone + PartialEq>() {}

fn assert_send_sync<T: Send + Sync + 'static>() {}

fn assert_error<T: std::error::Error + std::fmt::Display>() {}

#[test]
fn given_the_types_with_eq_when_compiled_then_they_have_it() {
    assert_eq_type::<ClientSettings>();
    assert_eq_type::<BuildError>();
    assert_eq_type::<CapabilitiesSpec>();
    assert_eq_type::<Capabilities>();
    assert_eq_type::<Accepted<ReasoningLevel>>();
    assert_eq_type::<Accepted<MediaType>>();
}

#[test]
fn given_the_types_that_hold_a_usage_or_a_block_when_compiled_then_they_have_no_eq() {
    <CallReport as AmbiguousIfEq<_>>::proof();
    <CallOutcome as AmbiguousIfEq<_>>::proof();
    <ClientError as AmbiguousIfEq<_>>::proof();
    <ClientRequest as AmbiguousIfEq<_>>::proof();
}

#[test]
fn given_the_data_types_when_compiled_then_they_have_debug_clone_and_partial_eq() {
    assert_data::<ClientRequest>();
    assert_data::<ClientSettings>();
    assert_data::<CallOutcome>();
    assert_data::<CallReport>();
    assert_data::<ClientError>();
    assert_data::<ToolSpec>();
    assert_data::<OutputMode>();
}

#[test]
fn given_the_call_types_when_compiled_then_they_are_send_sync_and_static() {
    assert_send_sync::<ClientRequest>();
    assert_send_sync::<CallOutcome>();
    assert_send_sync::<CallReport>();
    assert_send_sync::<ClientError>();
    assert_send_sync::<ClientSettings>();
}

#[test]
fn given_the_two_error_types_when_compiled_then_they_are_std_errors_with_display() {
    assert_error::<ClientError>();
    assert_error::<BuildError>();
}
