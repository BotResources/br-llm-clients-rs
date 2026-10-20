#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod build_error;
mod call;
mod call_context;
mod call_log;
mod call_view;
mod chunking;
mod claim;
mod codes;
mod derive_events;
mod fold_error;
mod gate;
mod gate_pass;
mod log;
mod log_disabled;
mod mock_builder;
mod mock_client;
mod mock_waits;
mod outcomes;
mod plan;
mod recording_sink;
mod reply;
mod reply_builder;
mod reply_check;
mod reply_source;
mod report_values;
mod rule;
mod script;
mod serve;
mod shared;
mod state_counts;
mod step_check;
mod strictness;
mod sync;
mod tool_call_id;
mod tracker;
mod usage_rules;

#[cfg(test)]
mod call_hang_tests;
#[cfg(test)]
mod call_reply_tests;
#[cfg(test)]
mod capabilities_tests;
#[cfg(test)]
mod chunking_tests;
#[cfg(test)]
mod derive_events_tests;
#[cfg(test)]
mod first_poll_tests;
#[cfg(test)]
mod gate_state_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod log_switch_tests;
#[cfg(test)]
mod log_tests;
#[cfg(test)]
mod parallel_tests;
#[cfg(test)]
mod recording_sink_tests;
#[cfg(test)]
mod reentrancy_tests;
#[cfg(test)]
mod refusal_tests;
#[cfg(test)]
mod reply_build_gate_tests;
#[cfg(test)]
mod reply_build_tests;
#[cfg(test)]
mod reply_build_usage_tests;
#[cfg(test)]
mod reply_function_tests;
#[cfg(test)]
mod rule_tests;
#[cfg(test)]
mod script_tests;
#[cfg(test)]
mod send_sync_tests;
#[cfg(test)]
mod step_check_tests;
#[cfg(test)]
mod strictness_order_tests;
#[cfg(test)]
mod strictness_request_tests;
#[cfg(test)]
mod strictness_settings_tests;
#[cfg(test)]
mod sync_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tool_call_id_tests;
#[cfg(test)]
mod waker_drop_tests;
#[cfg(test)]
mod waker_panic_tests;
#[cfg(test)]
mod waker_replace_tests;
#[cfg(test)]
mod waker_support;

pub use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec};
pub use build_error::MockBuildError;
pub use call_log::{CallState, LoggedCall, Served, StateKind};
pub use call_view::{MatchContext, ReplyContext};
pub use chunking::Chunking;
pub use codes::{CODE_INVALID_REPLY, CODE_SCRIPT_EXHAUSTED};
pub use derive_events::{DerivedEvents, derive_events};
pub use fold_error::FoldError;
pub use gate::{Gate, WaitForWaiting};
pub use log_disabled::LogDisabled;
pub use mock_builder::MockBuilder;
pub use mock_client::MockClient;
pub use mock_waits::{WaitForCalls, WaitForState};
pub use recording_sink::{RecordingSink, WaitForEvents};
pub use reply::Reply;
pub use reply_builder::{
    BrokenStreamKind, CutKind, FailureKind, HangKind, NamesUsage, ReplyBuilder, ReplyKind,
    Reported, SuccessKind,
};
pub use reply_source::ReplySource;
pub use rule::Rule;
pub use tool_call_id::tool_call_id;
